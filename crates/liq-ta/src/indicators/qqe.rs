//! Quantitative Qualitative Estimation (QQE) indicator.
//!
//! QQE is based on a smoothed RSI and adaptive volatility bands.

use crate::error::{Error, Result};
use crate::indicators::rsi::rsi;
use crate::traits::SeriesElement;

/// Output structure for QQE.
#[derive(Debug, Clone)]
pub struct QqeOutput<T> {
    /// Smoothed RSI line.
    pub qqe: Vec<T>,
    /// Upper dynamic band.
    pub upper_band: Vec<T>,
    /// Lower dynamic band.
    pub lower_band: Vec<T>,
}

/// Returns the lookback period for QQE.
#[inline]
#[must_use]
pub const fn qqe_lookback(
    rsi_period: usize,
    smoothing_period: usize,
    wilders_period: usize,
) -> usize {
    if rsi_period == 0 || smoothing_period == 0 || wilders_period == 0 {
        0
    } else {
        rsi_period + smoothing_period + wilders_period - 1
    }
}

/// Returns the minimum input length required for QQE.
#[inline]
#[must_use]
pub const fn qqe_min_len(
    rsi_period: usize,
    smoothing_period: usize,
    wilders_period: usize,
) -> usize {
    qqe_lookback(rsi_period, smoothing_period, wilders_period) + 1
}

/// Computes QQE.
///
/// # Errors
///
/// Returns an error if:
/// - any period is invalid
/// - `factor` is not positive finite
/// - input is empty or too short
#[must_use = "this returns a Result with QQE output, which should be used"]
pub fn qqe<T: SeriesElement>(
    data: &[T],
    rsi_period: usize,
    smoothing_period: usize,
    wilders_period: usize,
    factor: f64,
) -> Result<QqeOutput<T>> {
    validate_inputs(data, rsi_period, smoothing_period, wilders_period, factor)?;

    let n = data.len();
    let mut qqe_line = vec![T::nan(); n];
    let mut upper_band = vec![T::nan(); n];
    let mut lower_band = vec![T::nan(); n];

    compute_qqe_core(
        data,
        rsi_period,
        smoothing_period,
        wilders_period,
        factor,
        &mut qqe_line,
        &mut upper_band,
        &mut lower_band,
    )?;

    Ok(QqeOutput {
        qqe: qqe_line,
        upper_band,
        lower_band,
    })
}

/// Computes QQE into pre-allocated output buffers.
///
/// Returns the number of valid values (non-lookback positions).
///
/// # Errors
///
/// Returns an error if validation fails or output buffers are too small.
#[must_use = "this returns a Result with the count of valid QQE values"]
pub fn qqe_into<T: SeriesElement>(
    data: &[T],
    rsi_period: usize,
    smoothing_period: usize,
    wilders_period: usize,
    factor: f64,
    qqe_out: &mut [T],
    upper_band_out: &mut [T],
    lower_band_out: &mut [T],
) -> Result<usize> {
    validate_inputs(data, rsi_period, smoothing_period, wilders_period, factor)?;

    let n = data.len();
    if qqe_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: qqe_out.len(),
            indicator: "qqe (line)",
        });
    }
    if upper_band_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: upper_band_out.len(),
            indicator: "qqe (upper_band)",
        });
    }
    if lower_band_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: lower_band_out.len(),
            indicator: "qqe (lower_band)",
        });
    }

    qqe_out[..n].fill(T::nan());
    upper_band_out[..n].fill(T::nan());
    lower_band_out[..n].fill(T::nan());

    compute_qqe_core(
        data,
        rsi_period,
        smoothing_period,
        wilders_period,
        factor,
        qqe_out,
        upper_band_out,
        lower_band_out,
    )?;

    Ok(n.saturating_sub(qqe_lookback(rsi_period, smoothing_period, wilders_period)))
}

fn validate_inputs<T: SeriesElement>(
    data: &[T],
    rsi_period: usize,
    smoothing_period: usize,
    wilders_period: usize,
    factor: f64,
) -> Result<()> {
    if rsi_period == 0 {
        return Err(Error::InvalidPeriod {
            period: rsi_period,
            reason: "rsi_period must be at least 1",
        });
    }
    if smoothing_period == 0 {
        return Err(Error::InvalidPeriod {
            period: smoothing_period,
            reason: "smoothing_period must be at least 1",
        });
    }
    if wilders_period == 0 {
        return Err(Error::InvalidPeriod {
            period: wilders_period,
            reason: "wilders_period must be at least 1",
        });
    }
    if !factor.is_finite() || factor <= 0.0 {
        return Err(Error::LengthMismatch {
            description: "factor must be a positive finite number".to_string(),
        });
    }
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    let required = qqe_min_len(rsi_period, smoothing_period, wilders_period);
    if data.len() < required {
        return Err(Error::InsufficientData {
            required,
            actual: data.len(),
            indicator: "qqe",
        });
    }
    Ok(())
}

fn compute_qqe_core<T: SeriesElement>(
    data: &[T],
    rsi_period: usize,
    smoothing_period: usize,
    wilders_period: usize,
    factor: f64,
    qqe_out: &mut [T],
    upper_band_out: &mut [T],
    lower_band_out: &mut [T],
) -> Result<()> {
    let n = data.len();
    let smoothed_rsi = {
        let rsi_values = rsi(data, rsi_period)?;
        let mut smoothed = vec![T::nan(); n];
        ema_with_nan_prefix(&rsi_values, smoothing_period, &mut smoothed)?;
        smoothed
    };

    let mut delta = vec![T::nan(); n];
    for i in 1..n {
        if smoothed_rsi[i].is_finite() && smoothed_rsi[i - 1].is_finite() {
            delta[i] = (smoothed_rsi[i] - smoothed_rsi[i - 1]).abs();
        }
    }

    let mut smoothed_delta = vec![T::nan(); n];
    ema_with_nan_prefix(&delta, wilders_period, &mut smoothed_delta)?;

    let factor_t = T::from_f64(factor)?;
    for i in 0..n {
        qqe_out[i] = smoothed_rsi[i];
        if smoothed_rsi[i].is_finite() && smoothed_delta[i].is_finite() {
            let width = factor_t * smoothed_delta[i];
            upper_band_out[i] = smoothed_rsi[i] + width;
            lower_band_out[i] = smoothed_rsi[i] - width;
        }
    }

    Ok(())
}

fn ema_with_nan_prefix<T: SeriesElement>(
    input: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if input.len() != output.len() {
        return Err(Error::LengthMismatch {
            description: format!(
                "input has {} elements, output has {}",
                input.len(),
                output.len()
            ),
        });
    }

    let n = input.len();
    output.fill(T::nan());

    if n == 0 {
        return Ok(());
    }

    let start = match input.iter().position(|value| value.is_finite()) {
        Some(idx) => idx,
        None => return Ok(()),
    };

    if start + period > n {
        return Ok(());
    }

    let mut sum = T::zero();
    for value in input.iter().skip(start).take(period) {
        if !value.is_finite() {
            return Ok(());
        }
        sum = sum + *value;
    }

    let period_t = T::from_usize(period)?;
    let mut prev = sum / period_t;
    let seed_idx = start + period - 1;
    output[seed_idx] = prev;

    let alpha = T::two() / T::from_usize(period + 1)?;
    let one = T::one();
    for i in (seed_idx + 1)..n {
        if !input[i].is_finite() {
            output[i] = T::nan();
            continue;
        }
        prev = alpha * input[i] + (one - alpha) * prev;
        output[i] = prev;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    fn sample_series(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| 100.0 + i as f64 * 0.2 + ((i % 5) as f64 * 0.1))
            .collect()
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
    fn test_qqe_lookback_and_min_len() {
        assert_eq!(qqe_lookback(14, 5, 14), 32);
        assert_eq!(qqe_min_len(14, 5, 14), 33);
        assert_eq!(qqe_lookback(0, 5, 14), 0);
    }

    #[test]
    fn test_qqe_rejects_invalid_periods() {
        let data = sample_series(40);
        let err = qqe(&data, 0, 5, 14, 4.236).expect_err("expected invalid period");
        assert!(matches!(err, Error::InvalidPeriod { period: 0, .. }));
        let err = qqe(&data, 14, 0, 14, 4.236).expect_err("expected invalid period");
        assert!(matches!(err, Error::InvalidPeriod { period: 0, .. }));
        let err = qqe(&data, 14, 5, 0, 4.236).expect_err("expected invalid period");
        assert!(matches!(err, Error::InvalidPeriod { period: 0, .. }));
    }

    #[test]
    fn test_qqe_rejects_invalid_factor_empty_and_short_data() {
        let data = sample_series(40);
        let err = qqe(&data, 14, 5, 14, 0.0).expect_err("expected invalid factor");
        assert!(matches!(err, Error::LengthMismatch { .. }));
        let err = qqe(&data, 14, 5, 14, f64::NAN).expect_err("expected invalid factor");
        assert!(matches!(err, Error::LengthMismatch { .. }));

        let err = qqe::<f64>(&[], 14, 5, 14, 4.236).expect_err("expected empty input");
        assert!(matches!(err, Error::EmptyInput));

        let short = sample_series(20);
        let err = qqe(&short, 14, 5, 14, 4.236).expect_err("expected insufficient data");
        assert!(matches!(
            err,
            Error::InsufficientData {
                indicator: "qqe",
                ..
            }
        ));
    }

    #[test]
    fn test_qqe_output_shape_and_band_relationship() {
        let data = sample_series(120);
        let rsi_period = 14;
        let smoothing_period = 5;
        let wilders_period = 14;
        let out =
            qqe(&data, rsi_period, smoothing_period, wilders_period, 4.236).expect("qqe computes");
        let lookback = qqe_lookback(rsi_period, smoothing_period, wilders_period);

        assert_eq!(out.qqe.len(), data.len());
        assert_eq!(out.upper_band.len(), data.len());
        assert_eq!(out.lower_band.len(), data.len());

        for i in 0..lookback {
            assert!(out.upper_band[i].is_nan());
            assert!(out.lower_band[i].is_nan());
        }

        for i in lookback..data.len() {
            if out.qqe[i].is_finite()
                && out.upper_band[i].is_finite()
                && out.lower_band[i].is_finite()
            {
                assert!(out.upper_band[i] >= out.qqe[i]);
                assert!(out.qqe[i] >= out.lower_band[i]);
            }
        }
    }

    #[test]
    fn test_qqe_into_rejects_small_line_buffer() {
        let data = sample_series(50);
        let n = data.len();
        let mut line = vec![0.0; n - 1];
        let mut upper = vec![0.0; n];
        let mut lower = vec![0.0; n];
        let err = qqe_into(&data, 14, 5, 14, 4.236, &mut line, &mut upper, &mut lower)
            .expect_err("expected small line buffer");
        assert!(matches!(
            err,
            Error::BufferTooSmall {
                indicator: "qqe (line)",
                ..
            }
        ));
    }

    #[test]
    fn test_qqe_into_rejects_small_upper_buffer() {
        let data = sample_series(50);
        let n = data.len();
        let mut line = vec![0.0; n];
        let mut upper = vec![0.0; n - 1];
        let mut lower = vec![0.0; n];
        let err = qqe_into(&data, 14, 5, 14, 4.236, &mut line, &mut upper, &mut lower)
            .expect_err("expected small upper buffer");
        assert!(matches!(
            err,
            Error::BufferTooSmall {
                indicator: "qqe (upper_band)",
                ..
            }
        ));
    }

    #[test]
    fn test_qqe_into_rejects_small_lower_buffer() {
        let data = sample_series(50);
        let n = data.len();
        let mut line = vec![0.0; n];
        let mut upper = vec![0.0; n];
        let mut lower = vec![0.0; n - 1];
        let err = qqe_into(&data, 14, 5, 14, 4.236, &mut line, &mut upper, &mut lower)
            .expect_err("expected small lower buffer");
        assert!(matches!(
            err,
            Error::BufferTooSmall {
                indicator: "qqe (lower_band)",
                ..
            }
        ));
    }

    #[test]
    fn test_qqe_into_matches_allocating_variant() {
        let data = sample_series(120);
        let n = data.len();
        let mut line = vec![1.0; n];
        let mut upper = vec![1.0; n];
        let mut lower = vec![1.0; n];
        let valid = qqe_into(&data, 14, 5, 14, 4.236, &mut line, &mut upper, &mut lower)
            .expect("qqe_into computes");
        let direct = qqe(&data, 14, 5, 14, 4.236).expect("qqe computes");

        assert_eq!(valid, n - qqe_lookback(14, 5, 14));
        assert_series_close(&line, &direct.qqe);
        assert_series_close(&upper, &direct.upper_band);
        assert_series_close(&lower, &direct.lower_band);
    }

    #[test]
    fn test_ema_with_nan_prefix_rejects_zero_period() {
        let input = vec![1.0, 2.0, 3.0];
        let mut out = vec![0.0; 3];
        let err = ema_with_nan_prefix(&input, 0, &mut out).expect_err("expected invalid period");
        assert!(matches!(err, Error::InvalidPeriod { period: 0, .. }));
    }

    #[test]
    fn test_ema_with_nan_prefix_rejects_length_mismatch() {
        let input = vec![1.0, 2.0, 3.0];
        let mut out = vec![0.0; 2];
        let err = ema_with_nan_prefix(&input, 2, &mut out).expect_err("expected mismatch");
        assert!(matches!(err, Error::LengthMismatch { .. }));
    }

    #[test]
    fn test_ema_with_nan_prefix_handles_empty_and_no_finite_data() {
        let input: Vec<f64> = Vec::new();
        let mut out: Vec<f64> = Vec::new();
        ema_with_nan_prefix(&input, 2, &mut out).expect("empty input should be ok");
        assert!(out.is_empty());

        let input = vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
        let mut out = vec![0.0; input.len()];
        ema_with_nan_prefix(&input, 2, &mut out).expect("no finite data should be ok");
        assert!(out.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn test_ema_with_nan_prefix_handles_start_window_and_seed_nan() {
        let input = vec![f64::NAN, 1.0];
        let mut out = vec![0.0; input.len()];
        ema_with_nan_prefix(&input, 2, &mut out).expect("insufficient tail should be ok");
        assert!(out.iter().all(|v| v.is_nan()));

        let input = vec![1.0, f64::NAN, 3.0];
        let mut out = vec![0.0; input.len()];
        ema_with_nan_prefix(&input, 2, &mut out).expect("nan in seed window should be ok");
        assert!(out.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn test_ema_with_nan_prefix_marks_non_finite_input_points() {
        let input = vec![1.0, 2.0, 3.0, f64::NAN, 5.0];
        let mut out = vec![0.0; input.len()];
        ema_with_nan_prefix(&input, 2, &mut out).expect("ema should compute");

        assert!(out[0].is_nan());
        assert!(out[1].is_finite());
        assert!(out[2].is_finite());
        assert!(out[3].is_nan());
        assert!(out[4].is_finite());
    }
}
