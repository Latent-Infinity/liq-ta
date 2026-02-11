//! SAR (Parabolic Stop and Reverse) indicator.
//!
//! The Parabolic SAR is a trend-following indicator that provides potential
//! entry and exit points. It appears as dots above or below the price
//! depending on the trend direction.
//!
//! # Formula
//!
//! SAR(i) = SAR(i-1) + AF × (EP - SAR(i-1))
//!
//! Where:
//! - AF = Acceleration Factor, starts at `af_start` and increases by `af_step`
//!   each time a new extreme point is made, up to `af_max`
//! - EP = Extreme Point, the highest high in an uptrend or lowest low in a downtrend
//!
//! # Default Parameters
//!
//! - `af_start` = 0.02 (initial acceleration factor)
//! - `af_step` = 0.02 (acceleration factor increment)
//! - `af_max` = 0.20 (maximum acceleration factor)
//!
//! # Lookback
//!
//! The lookback period is 1 (need at least 2 bars to start).

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Computes the lookback period for SAR.
///
/// SAR requires 1 prior bar to start calculation.
#[inline]
#[must_use]
pub const fn sar_lookback() -> usize {
    1
}

/// Returns the minimum input length required for SAR calculation.
#[inline]
#[must_use]
pub const fn sar_min_len() -> usize {
    2
}

/// Computes Parabolic SAR with default parameters and stores results in output.
///
/// Uses default parameters: `af_start=0.02`, `af_step=0.02`, `af_max=0.20`
///
/// # Arguments
///
/// * `high` - High price data
/// * `low` - Low price data
/// * `output` - Pre-allocated output slice
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn sar_into<T: SeriesElement>(high: &[T], low: &[T], output: &mut [T]) -> Result<()> {
    let af_start = T::from_f64(0.02)?;
    let af_step = T::from_f64(0.02)?;
    let af_max = T::from_f64(0.20)?;
    sar_full_into(high, low, af_start, af_step, af_max, output)
}

/// Computes Parabolic SAR with custom parameters and stores results in output.
///
/// # Arguments
///
/// * `high` - High price data
/// * `low` - Low price data
/// * `af_start` - Initial acceleration factor (typically 0.02)
/// * `af_step` - Acceleration factor increment (typically 0.02)
/// * `af_max` - Maximum acceleration factor (typically 0.20)
/// * `output` - Pre-allocated output slice
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn sar_full_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    af_start: T,
    af_step: T,
    af_max: T,
    output: &mut [T],
) -> Result<()> {
    // Validate inputs
    if high.is_empty() || low.is_empty() {
        return Err(Error::EmptyInput);
    }

    if high.len() != low.len() {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", high.len(), low.len()),
        });
    }

    let n = high.len();
    let min_len = sar_min_len();

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "sar",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "sar",
            required: n,
            actual: output.len(),
        });
    }

    // First value is NaN (lookback period)
    output[0] = T::nan();

    if is_invalid(high[0]) || is_invalid(low[0]) || is_invalid(high[1]) || is_invalid(low[1]) {
        for out in output.iter_mut().skip(1) {
            *out = T::nan();
        }
        return Ok(());
    }

    // Determine initial trend by comparing first two bars
    // If high[1] > high[0], start with uptrend; otherwise downtrend
    let mut is_uptrend = high[1] > high[0];

    // Initialize SAR and EP based on initial trend
    let mut sar: T;
    let mut ep: T;
    let mut af = af_start;

    if is_uptrend {
        // In uptrend, SAR starts below price at the low of first bar
        sar = low[0];
        ep = high[1];
    } else {
        // In downtrend, SAR starts above price at the high of first bar
        sar = high[0];
        ep = low[1];
    }

    output[1] = sar;

    let mut nan_active = false;
    for i in 2..n {
        if nan_active || is_invalid(high[i]) || is_invalid(low[i]) {
            output[i] = T::nan();
            nan_active = true;
            continue;
        }
        // Calculate new SAR
        let mut new_sar = sar + af * (ep - sar);

        if is_uptrend {
            // In uptrend, SAR cannot be above the prior two lows
            if new_sar > low[i - 1] {
                new_sar = low[i - 1];
            }
            if new_sar > low[i - 2] {
                new_sar = low[i - 2];
            }

            // Check for trend reversal
            if low[i] < new_sar {
                // Reversal to downtrend
                is_uptrend = false;
                new_sar = ep; // SAR becomes the previous EP
                ep = low[i];
                af = af_start;
            } else {
                // Continue uptrend
                if high[i] > ep {
                    ep = high[i];
                    af = af + af_step;
                    if af > af_max {
                        af = af_max;
                    }
                }
            }
        } else {
            // In downtrend, SAR cannot be below the prior two highs
            if new_sar < high[i - 1] {
                new_sar = high[i - 1];
            }
            if new_sar < high[i - 2] {
                new_sar = high[i - 2];
            }

            // Check for trend reversal
            if high[i] > new_sar {
                // Reversal to uptrend
                is_uptrend = true;
                new_sar = ep; // SAR becomes the previous EP
                ep = high[i];
                af = af_start;
            } else {
                // Continue downtrend
                if low[i] < ep {
                    ep = low[i];
                    af = af + af_step;
                    if af > af_max {
                        af = af_max;
                    }
                }
            }
        }

        sar = new_sar;
        output[i] = sar;
    }

    Ok(())
}

/// Computes Parabolic SAR with default parameters.
///
/// Uses default parameters: `af_start=0.02`, `af_step=0.02`, `af_max=0.20`
///
/// # Arguments
///
/// * `high` - High price data
/// * `low` - Low price data
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of SAR values
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use liq_ta::indicators::sar;
///
/// let high: Vec<f64> = vec![10.0, 11.0, 12.0, 11.5, 11.0, 10.5, 10.0, 9.5];
/// let low: Vec<f64> = vec![9.0, 10.0, 11.0, 10.5, 10.0, 9.5, 9.0, 8.5];
/// let result = sar(&high, &low).unwrap();
/// assert!(result[0].is_nan()); // First value is NaN
/// assert!(result[1].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn sar<T: SeriesElement>(high: &[T], low: &[T]) -> Result<Vec<T>> {
    let n = high.len().max(low.len());
    let mut output = vec![T::nan(); n];
    sar_into(high, low, &mut output)?;
    Ok(output)
}

/// Computes Parabolic SAR with custom parameters.
///
/// # Arguments
///
/// * `high` - High price data
/// * `low` - Low price data
/// * `af_start` - Initial acceleration factor (typically 0.02)
/// * `af_step` - Acceleration factor increment (typically 0.02)
/// * `af_max` - Maximum acceleration factor (typically 0.20)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of SAR values
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn sar_full<T: SeriesElement>(
    high: &[T],
    low: &[T],
    af_start: T,
    af_step: T,
    af_max: T,
) -> Result<Vec<T>> {
    let n = high.len().max(low.len());
    let mut output = vec![T::nan(); n];
    sar_full_into(high, low, af_start, af_step, af_max, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_sar_lookback() {
        assert_eq!(sar_lookback(), 1);
    }

    #[test]
    fn test_sar_min_len() {
        assert_eq!(sar_min_len(), 2);
    }

    #[test]
    fn test_sar_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let result = sar(&high, &low);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_sar_length_mismatch() {
        let high: Vec<f64> = vec![1.0, 2.0, 3.0];
        let low: Vec<f64> = vec![0.5, 1.5];
        let result = sar(&high, &low);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_sar_insufficient_data() {
        let high: Vec<f64> = vec![1.0];
        let low: Vec<f64> = vec![0.5];
        let result = sar(&high, &low);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_sar_output_length() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 11.5, 11.0, 10.5, 10.0, 9.5];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 10.5, 10.0, 9.5, 9.0, 8.5];
        let result = sar(&high, &low).unwrap();
        assert_eq!(result.len(), high.len());
    }

    #[test]
    fn test_sar_first_value_nan() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 11.5, 11.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 10.5, 10.0];
        let result = sar(&high, &low).unwrap();
        assert!(result[0].is_nan());
    }

    #[test]
    fn test_sar_valid_values() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = sar(&high, &low).unwrap();

        // All values after lookback should be finite
        for i in 1..result.len() {
            assert!(result[i].is_finite(), "result[{}] should be finite", i);
        }
    }

    #[test]
    fn test_sar_uptrend() {
        // Clear uptrend - SAR should be below price
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
        let result = sar(&high, &low).unwrap();

        // In uptrend, SAR should be below the low prices
        for i in 1..result.len() {
            assert!(
                result[i] <= low[i],
                "In uptrend, SAR[{}]={} should be <= low[{}]={}",
                i,
                result[i],
                i,
                low[i]
            );
        }
    }

    #[test]
    fn test_sar_downtrend() {
        // Clear downtrend - SAR should be above price
        let high: Vec<f64> = vec![17.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 10.0];
        let low: Vec<f64> = vec![16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 10.0, 9.0];
        let result = sar(&high, &low).unwrap();

        // In downtrend, SAR should be above the high prices (after initial setup)
        for i in 2..result.len() {
            assert!(
                result[i] >= high[i],
                "In downtrend, SAR[{}]={} should be >= high[{}]={}",
                i,
                result[i],
                i,
                high[i]
            );
        }
    }

    #[test]
    fn test_sar_with_reversal() {
        // Uptrend then reversal to downtrend
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 12.0, 11.0, 10.0, 9.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 11.0, 10.0, 9.0, 8.0];
        let result = sar(&high, &low).unwrap();

        // All values should be finite after lookback
        for i in 1..result.len() {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_sar_custom_parameters() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];

        let result_default = sar(&high, &low).unwrap();
        let result_custom = sar_full(&high, &low, 0.01, 0.01, 0.10).unwrap();

        // Different parameters should produce different results
        let mut differs = false;
        for i in 1..result_default.len() {
            if (result_default[i] - result_custom[i]).abs() > 1e-10 {
                differs = true;
                break;
            }
        }
        assert!(
            differs,
            "Different AF parameters should produce different results"
        );
    }

    #[test]
    fn test_sar_into() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let mut output = vec![0.0_f64; high.len()];

        sar_into(&high, &low, &mut output).unwrap();

        assert!(output[0].is_nan());
        for i in 1..output.len() {
            assert!(output[i].is_finite());
        }
    }

    #[test]
    fn test_sar_into_buffer_too_small() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let mut output = vec![0.0_f64; 3]; // Too small

        let result = sar_into(&high, &low, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_sar_f32() {
        let high: Vec<f32> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f32> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = sar(&high, &low).unwrap();

        assert_eq!(result.len(), high.len());
        assert!(result[0].is_nan());
        for i in 1..result.len() {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_sar_minimum_length() {
        let high: Vec<f64> = vec![10.0, 11.0];
        let low: Vec<f64> = vec![9.0, 10.0];
        let result = sar(&high, &low).unwrap();

        assert_eq!(result.len(), 2);
        assert!(result[0].is_nan());
        assert!(result[1].is_finite());
    }

    #[test]
    fn test_sar_acceleration_factor_caps_at_max() {
        // Long uptrend to trigger max AF
        let high: Vec<f64> = (0..20).map(|x| 10.0 + x as f64).collect();
        let low: Vec<f64> = (0..20).map(|x| 9.0 + x as f64).collect();
        let result = sar(&high, &low).unwrap();

        // Should complete without error and produce valid results
        for i in 1..result.len() {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_sar_sideways_market() {
        // Sideways/choppy market
        let high: Vec<f64> = vec![10.0, 11.0, 10.5, 11.0, 10.5, 11.0, 10.5, 11.0];
        let low: Vec<f64> = vec![9.0, 10.0, 9.5, 10.0, 9.5, 10.0, 9.5, 10.0];
        let result = sar(&high, &low).unwrap();

        // All values should be finite after lookback
        for i in 1..result.len() {
            assert!(result[i].is_finite());
        }
    }
}
