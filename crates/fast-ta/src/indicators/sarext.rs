//! SAREXT (Extended Parabolic SAR) indicator.
//!
//! SAREXT is an extended version of the Parabolic SAR that allows separate
//! acceleration factor parameters for long and short positions, plus control
//! over the initial SAR value and offset on reversal.
//!
//! # Parameters
//!
//! - `start_value` - Initial SAR value (0.0 means use default based on initial trend)
//! - `offset_on_reverse` - Offset applied on trend reversal (typically 0.0)
//! - `af_init_long` - Initial acceleration factor for long positions
//! - `af_long` - Acceleration factor increment for long positions
//! - `af_max_long` - Maximum acceleration factor for long positions
//! - `af_init_short` - Initial acceleration factor for short positions
//! - `af_short` - Acceleration factor increment for short positions
//! - `af_max_short` - Maximum acceleration factor for short positions
//!
//! # Default Parameters
//!
//! All acceleration factors default to 0.02 (init/step) and 0.20 (max).

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// SAREXT parameters for extended Parabolic SAR calculation.
#[derive(Debug, Clone, Copy)]
pub struct SarExtParams<T> {
    /// Initial SAR value (0.0 means auto-detect based on initial trend)
    pub start_value: T,
    /// Offset applied on trend reversal
    pub offset_on_reverse: T,
    /// Initial acceleration factor for long positions
    pub af_init_long: T,
    /// Acceleration factor increment for long positions
    pub af_long: T,
    /// Maximum acceleration factor for long positions
    pub af_max_long: T,
    /// Initial acceleration factor for short positions
    pub af_init_short: T,
    /// Acceleration factor increment for short positions
    pub af_short: T,
    /// Maximum acceleration factor for short positions
    pub af_max_short: T,
}

impl<T: SeriesElement> Default for SarExtParams<T> {
    fn default() -> Self {
        Self {
            start_value: T::zero(),
            offset_on_reverse: T::zero(),
            af_init_long: T::from_f64(0.02).unwrap_or_else(|_| T::zero()),
            af_long: T::from_f64(0.02).unwrap_or_else(|_| T::zero()),
            af_max_long: T::from_f64(0.20).unwrap_or_else(|_| T::zero()),
            af_init_short: T::from_f64(0.02).unwrap_or_else(|_| T::zero()),
            af_short: T::from_f64(0.02).unwrap_or_else(|_| T::zero()),
            af_max_short: T::from_f64(0.20).unwrap_or_else(|_| T::zero()),
        }
    }
}

/// Computes the lookback period for SAREXT.
#[inline]
#[must_use]
pub const fn sarext_lookback() -> usize {
    1
}

/// Returns the minimum input length required for SAREXT calculation.
#[inline]
#[must_use]
pub const fn sarext_min_len() -> usize {
    2
}

/// Computes Extended Parabolic SAR with default parameters and stores results in output.
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
pub fn sarext_into<T: SeriesElement>(high: &[T], low: &[T], output: &mut [T]) -> Result<()> {
    sarext_full_into(high, low, SarExtParams::default(), output)
}

/// Computes Extended Parabolic SAR with custom parameters and stores results in output.
///
/// # Arguments
///
/// * `high` - High price data
/// * `low` - Low price data
/// * `params` - SAREXT parameters
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
pub fn sarext_full_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    params: SarExtParams<T>,
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
    let min_len = sarext_min_len();

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "sarext",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "sarext",
            required: n,
            actual: output.len(),
        });
    }

    // First value is NaN (lookback period)
    output[0] = T::nan();

    if is_invalid(high[0])
        || is_invalid(low[0])
        || is_invalid(high[1])
        || is_invalid(low[1])
    {
        for out in output.iter_mut().skip(1) {
            *out = T::nan();
        }
        return Ok(());
    }

    // Determine initial trend by comparing first two bars
    let mut is_long = high[1] > high[0];

    // Initialize SAR and EP based on initial trend
    let mut sar: T;
    let mut ep: T;
    let mut af: T;

    if is_long {
        // Start in long position
        if params.start_value == T::zero() {
            sar = low[0];
        } else {
            sar = params.start_value;
        }
        ep = high[1];
        af = params.af_init_long;
    } else {
        // Start in short position
        if params.start_value == T::zero() {
            sar = high[0];
        } else {
            sar = params.start_value;
        }
        ep = low[1];
        af = params.af_init_short;
    }

    // Output SAR with sign convention: positive = long, negative = short
    if is_long {
        output[1] = sar;
    } else {
        output[1] = T::zero() - sar;
    }

    let mut nan_active = false;
    for i in 2..n {
        if nan_active || is_invalid(high[i]) || is_invalid(low[i]) {
            output[i] = T::nan();
            nan_active = true;
            continue;
        }
        // Calculate new SAR
        let mut new_sar = sar + af * (ep - sar);

        if is_long {
            // In long position, SAR cannot be above the prior two lows
            if new_sar > low[i - 1] {
                new_sar = low[i - 1];
            }
            if new_sar > low[i - 2] {
                new_sar = low[i - 2];
            }

            // Check for reversal to short
            if low[i] < new_sar {
                // Reversal to short position
                is_long = false;
                new_sar = ep + params.offset_on_reverse;
                ep = low[i];
                af = params.af_init_short;
            } else {
                // Continue long
                if high[i] > ep {
                    ep = high[i];
                    af = af + params.af_long;
                    if af > params.af_max_long {
                        af = params.af_max_long;
                    }
                }
            }
        } else {
            // In short position, SAR cannot be below the prior two highs
            if new_sar < high[i - 1] {
                new_sar = high[i - 1];
            }
            if new_sar < high[i - 2] {
                new_sar = high[i - 2];
            }

            // Check for reversal to long
            if high[i] > new_sar {
                // Reversal to long position
                is_long = true;
                new_sar = ep - params.offset_on_reverse;
                ep = high[i];
                af = params.af_init_long;
            } else {
                // Continue short
                if low[i] < ep {
                    ep = low[i];
                    af = af + params.af_short;
                    if af > params.af_max_short {
                        af = params.af_max_short;
                    }
                }
            }
        }

        sar = new_sar;

        // Output with sign convention
        if is_long {
            output[i] = sar;
        } else {
            output[i] = T::zero() - sar;
        }
    }

    Ok(())
}

/// Computes Extended Parabolic SAR with default parameters.
///
/// # Arguments
///
/// * `high` - High price data
/// * `low` - Low price data
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of SAREXT values (positive = long, negative = short)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::sarext;
///
/// let high: Vec<f64> = vec![10.0, 11.0, 12.0, 11.5, 11.0, 10.5, 10.0, 9.5];
/// let low: Vec<f64> = vec![9.0, 10.0, 11.0, 10.5, 10.0, 9.5, 9.0, 8.5];
/// let result = sarext(&high, &low).unwrap();
/// assert!(result[0].is_nan());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn sarext<T: SeriesElement>(high: &[T], low: &[T]) -> Result<Vec<T>> {
    let n = high.len().max(low.len());
    let mut output = vec![T::nan(); n];
    sarext_into(high, low, &mut output)?;
    Ok(output)
}

/// Computes Extended Parabolic SAR with custom parameters.
///
/// # Arguments
///
/// * `high` - High price data
/// * `low` - Low price data
/// * `params` - SAREXT parameters
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of SAREXT values (positive = long, negative = short)
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn sarext_full<T: SeriesElement>(
    high: &[T],
    low: &[T],
    params: SarExtParams<T>,
) -> Result<Vec<T>> {
    let n = high.len().max(low.len());
    let mut output = vec![T::nan(); n];
    sarext_full_into(high, low, params, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_sarext_lookback() {
        assert_eq!(sarext_lookback(), 1);
    }

    #[test]
    fn test_sarext_min_len() {
        assert_eq!(sarext_min_len(), 2);
    }

    #[test]
    fn test_sarext_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let result = sarext(&high, &low);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_sarext_length_mismatch() {
        let high: Vec<f64> = vec![1.0, 2.0, 3.0];
        let low: Vec<f64> = vec![0.5, 1.5];
        let result = sarext(&high, &low);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_sarext_insufficient_data() {
        let high: Vec<f64> = vec![1.0];
        let low: Vec<f64> = vec![0.5];
        let result = sarext(&high, &low);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_sarext_output_length() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 11.5, 11.0, 10.5, 10.0, 9.5];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 10.5, 10.0, 9.5, 9.0, 8.5];
        let result = sarext(&high, &low).unwrap();
        assert_eq!(result.len(), high.len());
    }

    #[test]
    fn test_sarext_first_value_nan() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 11.5, 11.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 10.5, 10.0];
        let result = sarext(&high, &low).unwrap();
        assert!(result[0].is_nan());
    }

    #[test]
    fn test_sarext_uptrend_positive() {
        // Clear uptrend - values should be positive (long position)
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = sarext(&high, &low).unwrap();

        // In long position, values should be positive
        for i in 1..result.len() {
            assert!(
                result[i] > 0.0,
                "In uptrend, result[{}]={} should be positive",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_sarext_downtrend_negative() {
        // Clear downtrend - values should be negative (short position)
        let high: Vec<f64> = vec![17.0, 16.0, 15.0, 14.0, 13.0, 12.0];
        let low: Vec<f64> = vec![16.0, 15.0, 14.0, 13.0, 12.0, 11.0];
        let result = sarext(&high, &low).unwrap();

        // In short position, values should be negative (after initial setup)
        for i in 2..result.len() {
            assert!(
                result[i] < 0.0,
                "In downtrend, result[{}]={} should be negative",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_sarext_custom_params() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];

        let default_result = sarext(&high, &low).unwrap();

        let custom_params = SarExtParams {
            start_value: 0.0,
            offset_on_reverse: 0.0,
            af_init_long: 0.01,
            af_long: 0.01,
            af_max_long: 0.10,
            af_init_short: 0.03,
            af_short: 0.03,
            af_max_short: 0.30,
        };
        let custom_result = sarext_full(&high, &low, custom_params).unwrap();

        // Different parameters should produce different results
        let mut differs = false;
        for i in 1..default_result.len() {
            if (default_result[i].abs() - custom_result[i].abs()).abs() > 1e-10 {
                differs = true;
                break;
            }
        }
        assert!(
            differs,
            "Different parameters should produce different results"
        );
    }

    #[test]
    fn test_sarext_with_start_value() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];

        let params = SarExtParams {
            start_value: 8.0,
            offset_on_reverse: 0.0,
            af_init_long: 0.02,
            af_long: 0.02,
            af_max_long: 0.20,
            af_init_short: 0.02,
            af_short: 0.02,
            af_max_short: 0.20,
        };
        let result = sarext_full(&high, &low, params).unwrap();

        // First valid value should start from specified start_value
        assert!((result[1] - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_sarext_into() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let mut output = vec![0.0_f64; high.len()];

        sarext_into(&high, &low, &mut output).unwrap();

        assert!(output[0].is_nan());
        for i in 1..output.len() {
            assert!(output[i].is_finite());
        }
    }

    #[test]
    fn test_sarext_into_buffer_too_small() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let mut output = vec![0.0_f64; 3]; // Too small

        let result = sarext_into(&high, &low, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_sarext_f32() {
        let high: Vec<f32> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f32> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = sarext(&high, &low).unwrap();

        assert_eq!(result.len(), high.len());
        assert!(result[0].is_nan());
        for i in 1..result.len() {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_sarext_params_default() {
        let params: SarExtParams<f64> = SarExtParams::default();
        assert_eq!(params.start_value, 0.0);
        assert_eq!(params.offset_on_reverse, 0.0);
        assert!((params.af_init_long - 0.02).abs() < 1e-10);
        assert!((params.af_long - 0.02).abs() < 1e-10);
        assert!((params.af_max_long - 0.20).abs() < 1e-10);
        assert!((params.af_init_short - 0.02).abs() < 1e-10);
        assert!((params.af_short - 0.02).abs() < 1e-10);
        assert!((params.af_max_short - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_sarext_minimum_length() {
        let high: Vec<f64> = vec![10.0, 11.0];
        let low: Vec<f64> = vec![9.0, 10.0];
        let result = sarext(&high, &low).unwrap();

        assert_eq!(result.len(), 2);
        assert!(result[0].is_nan());
        assert!(result[1].is_finite());
    }
}
