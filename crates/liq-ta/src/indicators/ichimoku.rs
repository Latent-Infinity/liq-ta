//! Ichimoku Kinko Hyo indicator.
//!
//! This module provides the standard five Ichimoku lines:
//! Tenkan-sen, Kijun-sen, Senkou Span A, Senkou Span B, and Chikou Span.

use crate::error::{Error, Result};
use crate::kernels::rolling_extrema::{rolling_max, rolling_min};
use crate::traits::SeriesElement;

/// Output structure for Ichimoku Kinko Hyo.
#[derive(Debug, Clone)]
pub struct IchimokuOutput<T> {
    /// Tenkan-sen (conversion line).
    pub tenkan: Vec<T>,
    /// Kijun-sen (base line).
    pub kijun: Vec<T>,
    /// Senkou Span A (leading span A), shifted forward by displacement.
    pub senkou_a: Vec<T>,
    /// Senkou Span B (leading span B), shifted forward by displacement.
    pub senkou_b: Vec<T>,
    /// Chikou Span (lagging span), shifted backward by displacement.
    pub chikou: Vec<T>,
}

/// Returns the lookback period for Ichimoku.
///
/// This reflects the latest of the rolling windows required by Tenkan/Kijun/Senkou B.
#[inline]
#[must_use]
pub const fn ichimoku_lookback(
    tenkan_period: usize,
    kijun_period: usize,
    senkou_b_period: usize,
) -> usize {
    if tenkan_period == 0 || kijun_period == 0 || senkou_b_period == 0 {
        0
    } else {
        let mut max_period = tenkan_period;
        if kijun_period > max_period {
            max_period = kijun_period;
        }
        if senkou_b_period > max_period {
            max_period = senkou_b_period;
        }
        max_period - 1
    }
}

/// Returns the minimum input length required for Ichimoku.
#[inline]
#[must_use]
pub const fn ichimoku_min_len(
    tenkan_period: usize,
    kijun_period: usize,
    senkou_b_period: usize,
) -> usize {
    let mut max_period = tenkan_period;
    if kijun_period > max_period {
        max_period = kijun_period;
    }
    if senkou_b_period > max_period {
        max_period = senkou_b_period;
    }
    max_period
}

/// Computes Ichimoku Kinko Hyo.
///
/// # Errors
///
/// Returns an error if:
/// - periods are invalid
/// - input lengths do not match
/// - data length is insufficient
#[must_use = "this returns a Result with Ichimoku output, which should be used"]
pub fn ichimoku<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    tenkan_period: usize,
    kijun_period: usize,
    senkou_b_period: usize,
    displacement: usize,
) -> Result<IchimokuOutput<T>> {
    validate_inputs(
        high,
        low,
        close,
        tenkan_period,
        kijun_period,
        senkou_b_period,
    )?;

    let n = close.len();
    let mut output = IchimokuOutput {
        tenkan: vec![T::nan(); n],
        kijun: vec![T::nan(); n],
        senkou_a: vec![T::nan(); n],
        senkou_b: vec![T::nan(); n],
        chikou: vec![T::nan(); n],
    };

    compute_ichimoku_core(
        high,
        low,
        close,
        tenkan_period,
        kijun_period,
        senkou_b_period,
        displacement,
        &mut output,
    )?;

    Ok(output)
}

/// Computes Ichimoku into pre-allocated output vectors.
///
/// Returns `(valid_tenkan, valid_kijun)` counts.
///
/// # Errors
///
/// Returns an error if validation fails or output vectors are too small.
#[must_use = "this returns a Result with valid Ichimoku counts"]
pub fn ichimoku_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    tenkan_period: usize,
    kijun_period: usize,
    senkou_b_period: usize,
    displacement: usize,
    output: &mut IchimokuOutput<T>,
) -> Result<(usize, usize)> {
    validate_inputs(
        high,
        low,
        close,
        tenkan_period,
        kijun_period,
        senkou_b_period,
    )?;

    let n = close.len();
    if output.tenkan.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.tenkan.len(),
            indicator: "ichimoku (tenkan)",
        });
    }
    if output.kijun.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.kijun.len(),
            indicator: "ichimoku (kijun)",
        });
    }
    if output.senkou_a.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.senkou_a.len(),
            indicator: "ichimoku (senkou_a)",
        });
    }
    if output.senkou_b.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.senkou_b.len(),
            indicator: "ichimoku (senkou_b)",
        });
    }
    if output.chikou.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.chikou.len(),
            indicator: "ichimoku (chikou)",
        });
    }

    output.tenkan[..n].fill(T::nan());
    output.kijun[..n].fill(T::nan());
    output.senkou_a[..n].fill(T::nan());
    output.senkou_b[..n].fill(T::nan());
    output.chikou[..n].fill(T::nan());

    compute_ichimoku_core(
        high,
        low,
        close,
        tenkan_period,
        kijun_period,
        senkou_b_period,
        displacement,
        output,
    )?;

    let tenkan_valid = n.saturating_sub(if tenkan_period == 0 {
        0
    } else {
        tenkan_period - 1
    });
    let kijun_valid = n.saturating_sub(if kijun_period == 0 {
        0
    } else {
        kijun_period - 1
    });
    Ok((tenkan_valid, kijun_valid))
}

fn validate_inputs<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    tenkan_period: usize,
    kijun_period: usize,
    senkou_b_period: usize,
) -> Result<()> {
    if tenkan_period == 0 {
        return Err(Error::InvalidPeriod {
            period: tenkan_period,
            reason: "tenkan_period must be at least 1",
        });
    }
    if kijun_period == 0 {
        return Err(Error::InvalidPeriod {
            period: kijun_period,
            reason: "kijun_period must be at least 1",
        });
    }
    if senkou_b_period == 0 {
        return Err(Error::InvalidPeriod {
            period: senkou_b_period,
            reason: "senkou_b_period must be at least 1",
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
    let required = ichimoku_min_len(tenkan_period, kijun_period, senkou_b_period);
    if n < required {
        return Err(Error::InsufficientData {
            required,
            actual: n,
            indicator: "ichimoku",
        });
    }
    Ok(())
}

fn compute_ichimoku_core<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    tenkan_period: usize,
    kijun_period: usize,
    senkou_b_period: usize,
    displacement: usize,
    output: &mut IchimokuOutput<T>,
) -> Result<()> {
    let n = close.len();
    let two = T::two();

    let tenkan_high = rolling_max(high, tenkan_period)?;
    let tenkan_low = rolling_min(low, tenkan_period)?;
    let kijun_high = rolling_max(high, kijun_period)?;
    let kijun_low = rolling_min(low, kijun_period)?;
    let span_b_high = rolling_max(high, senkou_b_period)?;
    let span_b_low = rolling_min(low, senkou_b_period)?;

    for i in 0..n {
        if tenkan_high[i].is_finite() && tenkan_low[i].is_finite() {
            output.tenkan[i] = (tenkan_high[i] + tenkan_low[i]) / two;
        }
        if kijun_high[i].is_finite() && kijun_low[i].is_finite() {
            output.kijun[i] = (kijun_high[i] + kijun_low[i]) / two;
        }
    }

    for i in 0..n {
        let shifted = i.saturating_add(displacement);
        if shifted < n && output.tenkan[i].is_finite() && output.kijun[i].is_finite() {
            output.senkou_a[shifted] = (output.tenkan[i] + output.kijun[i]) / two;
        }
        if shifted < n && span_b_high[i].is_finite() && span_b_low[i].is_finite() {
            output.senkou_b[shifted] = (span_b_high[i] + span_b_low[i]) / two;
        }
        let lagged_source = i.saturating_add(displacement);
        if lagged_source < n && close[lagged_source].is_finite() {
            output.chikou[i] = close[lagged_source];
        }
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
            let base = 100.0 + (i as f64 * 0.5);
            high.push(base + 2.0);
            low.push(base - 2.0);
            close.push(base + ((i % 3) as f64 * 0.1));
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
    fn test_ichimoku_lookback_and_min_len() {
        assert_eq!(ichimoku_lookback(9, 26, 52), 51);
        assert_eq!(ichimoku_lookback(0, 26, 52), 0);
        assert_eq!(ichimoku_min_len(9, 26, 52), 52);
        assert_eq!(ichimoku_min_len(5, 8, 3), 8);
    }

    #[test]
    fn test_ichimoku_rejects_invalid_periods() {
        let (high, low, close) = sample_ohlc(60);
        let err =
            ichimoku(&high, &low, &close, 0, 26, 52, 26).expect_err("expected invalid period");
        assert!(matches!(err, Error::InvalidPeriod { period: 0, .. }));

        let err = ichimoku(&high, &low, &close, 9, 0, 52, 26).expect_err("expected invalid period");
        assert!(matches!(err, Error::InvalidPeriod { period: 0, .. }));

        let err = ichimoku(&high, &low, &close, 9, 26, 0, 26).expect_err("expected invalid period");
        assert!(matches!(err, Error::InvalidPeriod { period: 0, .. }));
    }

    #[test]
    fn test_ichimoku_rejects_empty_mismatch_and_short_data() {
        let err = ichimoku::<f64>(&[], &[], &[], 9, 26, 52, 26).expect_err("expected empty input");
        assert!(matches!(err, Error::EmptyInput));

        let (high, mut low, close) = sample_ohlc(60);
        low.pop();
        let err =
            ichimoku(&high, &low, &close, 9, 26, 52, 26).expect_err("expected length mismatch");
        assert!(matches!(err, Error::LengthMismatch { .. }));

        let (high, low, close) = sample_ohlc(20);
        let err =
            ichimoku(&high, &low, &close, 9, 26, 52, 26).expect_err("expected insufficient data");
        assert!(matches!(
            err,
            Error::InsufficientData {
                indicator: "ichimoku",
                ..
            }
        ));
    }

    #[test]
    fn test_ichimoku_alignment_and_shift_behavior() {
        let (high, low, close) = sample_ohlc(80);
        let tenkan = 9;
        let kijun = 26;
        let span_b = 52;
        let displacement = 26;
        let out = ichimoku(&high, &low, &close, tenkan, kijun, span_b, displacement)
            .expect("ichimoku should compute");

        assert_eq!(out.tenkan.len(), close.len());
        assert_eq!(out.kijun.len(), close.len());
        assert_eq!(out.senkou_a.len(), close.len());
        assert_eq!(out.senkou_b.len(), close.len());
        assert_eq!(out.chikou.len(), close.len());

        for i in 0..(tenkan - 1) {
            assert!(out.tenkan[i].is_nan());
        }
        for i in 0..(kijun - 1) {
            assert!(out.kijun[i].is_nan());
        }
        for i in 0..(close.len() - displacement) {
            assert_eq!(out.chikou[i], close[i + displacement]);
        }
        for i in (close.len() - displacement)..close.len() {
            assert!(out.chikou[i].is_nan());
        }
    }

    #[test]
    fn test_ichimoku_large_displacement_keeps_shifted_outputs_nan() {
        let (high, low, close) = sample_ohlc(70);
        let out = ichimoku(&high, &low, &close, 9, 26, 52, 500).expect("ichimoku should compute");

        assert!(out.senkou_a.iter().all(|v| v.is_nan()));
        assert!(out.senkou_b.iter().all(|v| v.is_nan()));
        assert!(out.chikou.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn test_ichimoku_chikou_skips_non_finite_source() {
        let (high, low, mut close) = sample_ohlc(70);
        let displacement = 5;
        close[40] = f64::NAN;
        let out =
            ichimoku(&high, &low, &close, 9, 26, 52, displacement).expect("ichimoku computes");
        assert!(out.chikou[40 - displacement].is_nan());
    }

    #[test]
    fn test_ichimoku_into_rejects_small_tenkan_buffer() {
        let (high, low, close) = sample_ohlc(60);
        let n = close.len();
        let mut output = IchimokuOutput {
            tenkan: vec![0.0; n - 1],
            kijun: vec![0.0; n],
            senkou_a: vec![0.0; n],
            senkou_b: vec![0.0; n],
            chikou: vec![0.0; n],
        };
        let err = ichimoku_into(&high, &low, &close, 9, 26, 52, 26, &mut output)
            .expect_err("expected small tenkan buffer");
        assert!(matches!(
            err,
            Error::BufferTooSmall {
                indicator: "ichimoku (tenkan)",
                ..
            }
        ));
    }

    #[test]
    fn test_ichimoku_into_rejects_small_kijun_buffer() {
        let (high, low, close) = sample_ohlc(60);
        let n = close.len();
        let mut output = IchimokuOutput {
            tenkan: vec![0.0; n],
            kijun: vec![0.0; n - 1],
            senkou_a: vec![0.0; n],
            senkou_b: vec![0.0; n],
            chikou: vec![0.0; n],
        };
        let err = ichimoku_into(&high, &low, &close, 9, 26, 52, 26, &mut output)
            .expect_err("expected small kijun buffer");
        assert!(matches!(
            err,
            Error::BufferTooSmall {
                indicator: "ichimoku (kijun)",
                ..
            }
        ));
    }

    #[test]
    fn test_ichimoku_into_rejects_small_senkou_a_buffer() {
        let (high, low, close) = sample_ohlc(60);
        let n = close.len();
        let mut output = IchimokuOutput {
            tenkan: vec![0.0; n],
            kijun: vec![0.0; n],
            senkou_a: vec![0.0; n - 1],
            senkou_b: vec![0.0; n],
            chikou: vec![0.0; n],
        };
        let err = ichimoku_into(&high, &low, &close, 9, 26, 52, 26, &mut output)
            .expect_err("expected small senkou_a buffer");
        assert!(matches!(
            err,
            Error::BufferTooSmall {
                indicator: "ichimoku (senkou_a)",
                ..
            }
        ));
    }

    #[test]
    fn test_ichimoku_into_rejects_small_senkou_b_buffer() {
        let (high, low, close) = sample_ohlc(60);
        let n = close.len();
        let mut output = IchimokuOutput {
            tenkan: vec![0.0; n],
            kijun: vec![0.0; n],
            senkou_a: vec![0.0; n],
            senkou_b: vec![0.0; n - 1],
            chikou: vec![0.0; n],
        };
        let err = ichimoku_into(&high, &low, &close, 9, 26, 52, 26, &mut output)
            .expect_err("expected small senkou_b buffer");
        assert!(matches!(
            err,
            Error::BufferTooSmall {
                indicator: "ichimoku (senkou_b)",
                ..
            }
        ));
    }

    #[test]
    fn test_ichimoku_into_rejects_small_chikou_buffer() {
        let (high, low, close) = sample_ohlc(60);
        let n = close.len();
        let mut output = IchimokuOutput {
            tenkan: vec![0.0; n],
            kijun: vec![0.0; n],
            senkou_a: vec![0.0; n],
            senkou_b: vec![0.0; n],
            chikou: vec![0.0; n - 1],
        };
        let err = ichimoku_into(&high, &low, &close, 9, 26, 52, 26, &mut output)
            .expect_err("expected small chikou buffer");
        assert!(matches!(
            err,
            Error::BufferTooSmall {
                indicator: "ichimoku (chikou)",
                ..
            }
        ));
    }

    #[test]
    fn test_ichimoku_into_matches_allocating_variant() {
        let (high, low, close) = sample_ohlc(90);
        let n = close.len();
        let tenkan_period = 9;
        let kijun_period = 26;
        let senkou_b_period = 52;
        let displacement = 26;
        let mut output = IchimokuOutput {
            tenkan: vec![1.0; n],
            kijun: vec![1.0; n],
            senkou_a: vec![1.0; n],
            senkou_b: vec![1.0; n],
            chikou: vec![1.0; n],
        };

        let (tenkan_valid, kijun_valid) = ichimoku_into(
            &high,
            &low,
            &close,
            tenkan_period,
            kijun_period,
            senkou_b_period,
            displacement,
            &mut output,
        )
        .expect("ichimoku_into should compute");
        let direct = ichimoku(
            &high,
            &low,
            &close,
            tenkan_period,
            kijun_period,
            senkou_b_period,
            displacement,
        )
        .expect("ichimoku should compute");

        assert_eq!(tenkan_valid, n - (tenkan_period - 1));
        assert_eq!(kijun_valid, n - (kijun_period - 1));
        assert_series_close(&output.tenkan, &direct.tenkan);
        assert_series_close(&output.kijun, &direct.kijun);
        assert_series_close(&output.senkou_a, &direct.senkou_a);
        assert_series_close(&output.senkou_b, &direct.senkou_b);
        assert_series_close(&output.chikou, &direct.chikou);
    }
}
