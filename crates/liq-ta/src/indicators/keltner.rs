//! Keltner Channel indicator.
//!
//! Keltner Channels use an EMA center line plus/minus a multiple of ATR.

use crate::error::{Error, Result};
use crate::indicators::atr::{atr, atr_into, atr_lookback, atr_min_len};
use crate::indicators::ema::{ema, ema_into, ema_lookback, ema_min_len};
use crate::traits::SeriesElement;

/// Output structure for Keltner Channels.
#[derive(Debug, Clone)]
pub struct KeltnerChannelOutput<T> {
    /// Upper channel: EMA(close) + multiplier * ATR.
    pub upper: Vec<T>,
    /// Middle channel: EMA(close).
    pub middle: Vec<T>,
    /// Lower channel: EMA(close) - multiplier * ATR.
    pub lower: Vec<T>,
}

/// Returns the lookback period for Keltner Channels.
///
/// This is the maximum lookback of EMA and ATR for the same period.
#[inline]
#[must_use]
pub const fn keltner_channel_lookback(period: usize) -> usize {
    let ema_lb = ema_lookback(period);
    let atr_lb = atr_lookback(period);
    if ema_lb > atr_lb { ema_lb } else { atr_lb }
}

/// Returns the minimum input length required for Keltner Channels.
///
/// This is the maximum minimum length of EMA and ATR for the same period.
#[inline]
#[must_use]
pub const fn keltner_channel_min_len(period: usize) -> usize {
    let ema_min = ema_min_len(period);
    let atr_min = atr_min_len(period);
    if ema_min > atr_min { ema_min } else { atr_min }
}

/// Computes Keltner Channels.
///
/// # Errors
///
/// Returns an error if:
/// - input lengths do not match
/// - period is invalid
/// - multiplier is not positive finite
/// - data length is insufficient
#[must_use = "this returns a Result with Keltner Channel output, which should be used"]
pub fn keltner_channel<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    atr_multiplier: f64,
) -> Result<KeltnerChannelOutput<T>> {
    validate_inputs(high, low, close, period, atr_multiplier)?;

    let n = close.len();
    let mut upper = vec![T::nan(); n];
    let mut middle = ema(close, period)?;
    let mut lower = vec![T::nan(); n];
    let atr_values = atr(high, low, close, period)?;

    let lookback = keltner_channel_lookback(period);
    let mult = T::from_f64(atr_multiplier)?;

    for i in 0..n {
        if i < lookback {
            middle[i] = T::nan();
            continue;
        }
        if !middle[i].is_finite() || !atr_values[i].is_finite() {
            middle[i] = T::nan();
            continue;
        }
        let width = mult * atr_values[i];
        upper[i] = middle[i] + width;
        lower[i] = middle[i] - width;
    }

    Ok(KeltnerChannelOutput {
        upper,
        middle,
        lower,
    })
}

/// Computes Keltner Channels into pre-allocated output buffers.
///
/// Returns the number of valid values (non-lookback positions).
///
/// # Errors
///
/// Returns an error if validation fails or any output buffer is too small.
#[must_use = "this returns a Result with the count of valid Keltner values"]
pub fn keltner_channel_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    atr_multiplier: f64,
    upper_out: &mut [T],
    middle_out: &mut [T],
    lower_out: &mut [T],
) -> Result<usize> {
    validate_inputs(high, low, close, period, atr_multiplier)?;

    let n = close.len();
    if upper_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: upper_out.len(),
            indicator: "keltner_channel (upper)",
        });
    }
    if middle_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: middle_out.len(),
            indicator: "keltner_channel (middle)",
        });
    }
    if lower_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: lower_out.len(),
            indicator: "keltner_channel (lower)",
        });
    }

    ema_into(close, period, middle_out)?;
    let mut atr_values = vec![T::nan(); n];
    atr_into(high, low, close, period, &mut atr_values)?;

    let lookback = keltner_channel_lookback(period);
    let mult = T::from_f64(atr_multiplier)?;

    for i in 0..n {
        if i < lookback || !middle_out[i].is_finite() || !atr_values[i].is_finite() {
            upper_out[i] = T::nan();
            middle_out[i] = T::nan();
            lower_out[i] = T::nan();
            continue;
        }
        let width = mult * atr_values[i];
        upper_out[i] = middle_out[i] + width;
        lower_out[i] = middle_out[i] - width;
    }

    Ok(n.saturating_sub(lookback))
}

fn validate_inputs<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    atr_multiplier: f64,
) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if !atr_multiplier.is_finite() || atr_multiplier <= 0.0 {
        return Err(Error::LengthMismatch {
            description: "atr_multiplier must be a positive finite number".to_string(),
        });
    }
    if high.is_empty() {
        return Err(Error::EmptyInput);
    }
    let n = high.len();
    if low.len() != n || close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "high has {n} elements, low has {}, close has {}",
                low.len(),
                close.len()
            ),
        });
    }
    let required = keltner_channel_min_len(period);
    if n < required {
        return Err(Error::InsufficientData {
            required,
            actual: n,
            indicator: "keltner_channel",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    fn sample_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);
        for i in 0..n {
            let c = 100.0 + i as f64;
            high.push(c + 1.5);
            low.push(c - 1.5);
            close.push(c);
        }
        (high, low, close)
    }

    fn assert_series_close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for i in 0..actual.len() {
            let a = actual[i];
            let e = expected[i];
            if a.is_nan() || e.is_nan() {
                assert!(
                    a.is_nan() && e.is_nan(),
                    "index {i}: actual={a:?}, expected={e:?}"
                );
            } else {
                assert!((a - e).abs() < 1e-10, "index {i}: actual={a}, expected={e}");
            }
        }
    }

    #[test]
    fn test_keltner_lookback_and_min_len_match_components() {
        let period = 20;
        assert_eq!(
            keltner_channel_lookback(period),
            ema_lookback(period).max(atr_lookback(period))
        );
        assert_eq!(
            keltner_channel_min_len(period),
            ema_min_len(period).max(atr_min_len(period))
        );
    }

    #[test]
    fn test_keltner_rejects_invalid_period() {
        let (high, low, close) = sample_ohlc(10);
        let err =
            keltner_channel(&high, &low, &close, 0, 2.0).expect_err("expected invalid period");
        assert!(matches!(err, Error::InvalidPeriod { period: 0, .. }));
    }

    #[test]
    fn test_keltner_rejects_invalid_multiplier() {
        let (high, low, close) = sample_ohlc(10);
        let err =
            keltner_channel(&high, &low, &close, 3, 0.0).expect_err("expected invalid multiplier");
        assert!(matches!(err, Error::LengthMismatch { .. }));

        let err = keltner_channel(&high, &low, &close, 3, f64::INFINITY)
            .expect_err("expected invalid multiplier");
        assert!(matches!(err, Error::LengthMismatch { .. }));
    }

    #[test]
    fn test_keltner_rejects_empty_and_mismatched_input() {
        let err =
            keltner_channel::<f64>(&[], &[], &[], 3, 2.0).expect_err("expected empty input error");
        assert!(matches!(err, Error::EmptyInput));

        let (high, mut low, close) = sample_ohlc(10);
        low.pop();
        let err = keltner_channel(&high, &low, &close, 3, 2.0)
            .expect_err("expected length mismatch error");
        assert!(matches!(err, Error::LengthMismatch { .. }));
    }

    #[test]
    fn test_keltner_rejects_insufficient_data() {
        let (high, low, close) = sample_ohlc(5);
        let err =
            keltner_channel(&high, &low, &close, 10, 2.0).expect_err("expected insufficient data");
        assert!(matches!(
            err,
            Error::InsufficientData {
                indicator: "keltner_channel",
                ..
            }
        ));
    }

    #[test]
    fn test_keltner_basic_shape_and_ordering() {
        let (high, low, close) = sample_ohlc(40);
        let period = 10;
        let out =
            keltner_channel(&high, &low, &close, period, 2.0).expect("keltner should compute");
        let lookback = keltner_channel_lookback(period);

        assert_eq!(out.upper.len(), close.len());
        assert_eq!(out.middle.len(), close.len());
        assert_eq!(out.lower.len(), close.len());

        for i in 0..lookback {
            assert!(out.upper[i].is_nan());
            assert!(out.middle[i].is_nan());
            assert!(out.lower[i].is_nan());
        }

        for i in lookback..close.len() {
            assert!(out.upper[i].is_finite());
            assert!(out.middle[i].is_finite());
            assert!(out.lower[i].is_finite());
            assert!(out.upper[i] >= out.middle[i]);
            assert!(out.middle[i] >= out.lower[i]);
        }
    }

    #[test]
    fn test_keltner_nan_propagation_after_lookback() {
        let (high, low, mut close) = sample_ohlc(40);
        close[25] = f64::NAN;

        let out = keltner_channel(&high, &low, &close, 10, 2.0).expect("keltner should compute");
        let lookback = keltner_channel_lookback(10);

        assert!(out.middle[lookback..].iter().any(|v| v.is_nan()));
        for i in lookback..close.len() {
            if out.middle[i].is_nan() {
                assert!(out.upper[i].is_nan());
                assert!(out.lower[i].is_nan());
            }
        }
    }

    #[test]
    fn test_keltner_into_rejects_small_upper_buffer() {
        let (high, low, close) = sample_ohlc(20);
        let n = close.len();
        let mut upper = vec![0.0; n - 1];
        let mut middle = vec![0.0; n];
        let mut lower = vec![0.0; n];
        let err = keltner_channel_into(
            &high,
            &low,
            &close,
            5,
            2.0,
            &mut upper,
            &mut middle,
            &mut lower,
        )
        .expect_err("expected small upper buffer error");
        assert!(matches!(
            err,
            Error::BufferTooSmall {
                indicator: "keltner_channel (upper)",
                ..
            }
        ));
    }

    #[test]
    fn test_keltner_into_rejects_small_middle_buffer() {
        let (high, low, close) = sample_ohlc(20);
        let n = close.len();
        let mut upper = vec![0.0; n];
        let mut middle = vec![0.0; n - 1];
        let mut lower = vec![0.0; n];
        let err = keltner_channel_into(
            &high,
            &low,
            &close,
            5,
            2.0,
            &mut upper,
            &mut middle,
            &mut lower,
        )
        .expect_err("expected small middle buffer error");
        assert!(matches!(
            err,
            Error::BufferTooSmall {
                indicator: "keltner_channel (middle)",
                ..
            }
        ));
    }

    #[test]
    fn test_keltner_into_rejects_small_lower_buffer() {
        let (high, low, close) = sample_ohlc(20);
        let n = close.len();
        let mut upper = vec![0.0; n];
        let mut middle = vec![0.0; n];
        let mut lower = vec![0.0; n - 1];
        let err = keltner_channel_into(
            &high,
            &low,
            &close,
            5,
            2.0,
            &mut upper,
            &mut middle,
            &mut lower,
        )
        .expect_err("expected small lower buffer error");
        assert!(matches!(
            err,
            Error::BufferTooSmall {
                indicator: "keltner_channel (lower)",
                ..
            }
        ));
    }

    #[test]
    fn test_keltner_into_matches_allocating_variant() {
        let (high, low, close) = sample_ohlc(50);
        let period = 14;
        let n = close.len();
        let mut upper = vec![1.0; n];
        let mut middle = vec![1.0; n];
        let mut lower = vec![1.0; n];

        let valid = keltner_channel_into(
            &high,
            &low,
            &close,
            period,
            1.5,
            &mut upper,
            &mut middle,
            &mut lower,
        )
        .expect("keltner into should compute");
        let direct =
            keltner_channel(&high, &low, &close, period, 1.5).expect("keltner should compute");

        assert_eq!(valid, n - keltner_channel_lookback(period));
        assert_series_close(&upper, &direct.upper);
        assert_series_close(&middle, &direct.middle);
        assert_series_close(&lower, &direct.lower);
    }
}
