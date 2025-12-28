//! BOP (Balance of Power) indicator.
//!
//! Balance of Power measures the strength of buyers vs sellers by comparing
//! the closing price position within the day's range.
//!
//! # Formula
//!
//! ```text
//! BOP = (close - open) / (high - low)
//! ```
//!
//! # Range
//!
//! BOP ranges from -1 to +1:
//! - +1: Close at high (maximum buying pressure)
//! - -1: Close at low (maximum selling pressure)
//! - 0: Close at midpoint of range
//!
//! # Lookback
//!
//! No lookback period (calculated per bar).

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Computes the lookback period for BOP.
#[inline]
#[must_use]
pub const fn bop_lookback() -> usize {
    0
}

/// Returns the minimum input length required for BOP calculation.
#[inline]
#[must_use]
pub const fn bop_min_len() -> usize {
    1
}

/// Computes BOP (Balance of Power) and stores results in output.
///
/// BOP = (close - open) / (high - low)
///
/// # Arguments
///
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn bop_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    output: &mut [T],
) -> Result<()> {
    let n = open.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    // Validate all arrays have same length
    if high.len() != n || low.len() != n || close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "OHLC arrays must have same length: open={}, high={}, low={}, close={}",
                n,
                high.len(),
                low.len(),
                close.len()
            ),
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "bop",
            required: n,
            actual: output.len(),
        });
    }

    // Calculate BOP for each bar
    // Note: NaN-precheck pattern is NOT beneficial here because:
    // - Loop body is very simple (2 subs + 1 div)
    // - 4 pre-scans would add more overhead than inline NaN checks
    for i in 0..n {
        if is_invalid(open[i])
            || is_invalid(high[i])
            || is_invalid(low[i])
            || is_invalid(close[i])
        {
            output[i] = T::nan();
            continue;
        }
        let range = high[i] - low[i];
        if range == T::zero() {
            // When high == low, use 0 (no directional pressure)
            output[i] = T::zero();
        } else {
            output[i] = (close[i] - open[i]) / range;
        }
    }

    Ok(())
}

/// Computes BOP (Balance of Power).
///
/// BOP = (close - open) / (high - low)
///
/// # Arguments
///
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of BOP values (range -1 to +1)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::bop;
///
/// let open  = vec![10.0_f64, 11.0, 12.0];
/// let high  = vec![12.0_f64, 13.0, 14.0];
/// let low   = vec![ 9.0_f64, 10.0, 11.0];
/// let close = vec![11.0_f64, 12.0, 13.0];
///
/// let result = bop(&open, &high, &low, &close).unwrap();
/// // First bar: (11 - 10) / (12 - 9) = 1/3 ≈ 0.333
/// assert!((result[0] - (1.0_f64/3.0)).abs() < 1e-10);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn bop<T: SeriesElement>(open: &[T], high: &[T], low: &[T], close: &[T]) -> Result<Vec<T>> {
    let mut output = vec![T::zero(); open.len()];
    bop_into(open, high, low, close, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_bop_lookback() {
        assert_eq!(bop_lookback(), 0);
    }

    #[test]
    fn test_bop_min_len() {
        assert_eq!(bop_min_len(), 1);
    }

    #[test]
    fn test_bop_empty_input() {
        let open: Vec<f64> = vec![];
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];
        let result = bop(&open, &high, &low, &close);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_bop_length_mismatch() {
        let open = vec![10.0, 11.0];
        let high = vec![12.0];
        let low = vec![9.0, 10.0];
        let close = vec![11.0, 12.0];
        let result = bop(&open, &high, &low, &close);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_bop_output_length() {
        let open = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let high = vec![12.0, 13.0, 14.0, 15.0, 16.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close = vec![11.0, 12.0, 13.0, 14.0, 15.0];
        let result = bop(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), open.len());
    }

    #[test]
    fn test_bop_basic_calculation() {
        let open: Vec<f64> = vec![10.0];
        let high: Vec<f64> = vec![12.0];
        let low: Vec<f64> = vec![9.0];
        let close: Vec<f64> = vec![11.0];
        let result = bop(&open, &high, &low, &close).unwrap();

        // BOP = (11 - 10) / (12 - 9) = 1/3
        assert!((result[0] - (1.0_f64 / 3.0_f64)).abs() < 1e-10);
    }

    #[test]
    fn test_bop_close_at_high() {
        // Close at high: maximum buying pressure
        let open: Vec<f64> = vec![10.0];
        let high: Vec<f64> = vec![12.0];
        let low: Vec<f64> = vec![9.0];
        let close: Vec<f64> = vec![12.0]; // close == high
        let result = bop(&open, &high, &low, &close).unwrap();

        // BOP = (12 - 10) / (12 - 9) = 2/3
        assert!((result[0] - (2.0_f64 / 3.0_f64)).abs() < 1e-10);
    }

    #[test]
    fn test_bop_close_at_low() {
        // Close at low: maximum selling pressure
        let open: Vec<f64> = vec![10.0];
        let high: Vec<f64> = vec![12.0];
        let low: Vec<f64> = vec![9.0];
        let close: Vec<f64> = vec![9.0]; // close == low
        let result = bop(&open, &high, &low, &close).unwrap();

        // BOP = (9 - 10) / (12 - 9) = -1/3
        assert!((result[0] - (-1.0_f64 / 3.0_f64)).abs() < 1e-10);
    }

    #[test]
    fn test_bop_open_at_low_close_at_high() {
        // Strong bullish bar: open at low, close at high
        let open: Vec<f64> = vec![9.0];
        let high: Vec<f64> = vec![12.0];
        let low: Vec<f64> = vec![9.0];
        let close: Vec<f64> = vec![12.0];
        let result = bop(&open, &high, &low, &close).unwrap();

        // BOP = (12 - 9) / (12 - 9) = 1.0
        assert!((result[0] - 1.0_f64).abs() < 1e-10);
    }

    #[test]
    fn test_bop_open_at_high_close_at_low() {
        // Strong bearish bar: open at high, close at low
        let open: Vec<f64> = vec![12.0];
        let high: Vec<f64> = vec![12.0];
        let low: Vec<f64> = vec![9.0];
        let close: Vec<f64> = vec![9.0];
        let result = bop(&open, &high, &low, &close).unwrap();

        // BOP = (9 - 12) / (12 - 9) = -1.0
        assert!((result[0] - (-1.0_f64)).abs() < 1e-10);
    }

    #[test]
    fn test_bop_doji_high_eq_low() {
        // Doji: high == low (no range)
        let open: Vec<f64> = vec![10.0];
        let high: Vec<f64> = vec![10.0];
        let low: Vec<f64> = vec![10.0];
        let close: Vec<f64> = vec![10.0];
        let result = bop(&open, &high, &low, &close).unwrap();

        // When high == low, BOP = 0
        assert!((result[0] - 0.0_f64).abs() < 1e-10);
    }

    #[test]
    fn test_bop_multiple_bars() {
        let open: Vec<f64> = vec![10.0, 11.0, 12.0];
        let high: Vec<f64> = vec![12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0];
        let close: Vec<f64> = vec![11.0, 12.0, 13.0];
        let result = bop(&open, &high, &low, &close).unwrap();

        // Bar 0: (11 - 10) / (12 - 9) = 1/3
        assert!((result[0] - (1.0_f64 / 3.0_f64)).abs() < 1e-10);
        // Bar 1: (12 - 11) / (13 - 10) = 1/3
        assert!((result[1] - (1.0_f64 / 3.0_f64)).abs() < 1e-10);
        // Bar 2: (13 - 12) / (14 - 11) = 1/3
        assert!((result[2] - (1.0_f64 / 3.0_f64)).abs() < 1e-10);
    }

    #[test]
    fn test_bop_into() {
        let open = vec![10.0, 11.0, 12.0];
        let high = vec![12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![11.0, 12.0, 13.0];
        let mut output = vec![0.0_f64; 3];

        bop_into(&open, &high, &low, &close, &mut output).unwrap();

        assert!((output[0] - (1.0 / 3.0)).abs() < 1e-10);
    }

    #[test]
    fn test_bop_into_buffer_too_small() {
        let open = vec![10.0, 11.0, 12.0];
        let high = vec![12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![11.0, 12.0, 13.0];
        let mut output = vec![0.0_f64; 2]; // Too small

        let result = bop_into(&open, &high, &low, &close, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_bop_f32() {
        let open: Vec<f32> = vec![10.0, 11.0, 12.0];
        let high: Vec<f32> = vec![12.0, 13.0, 14.0];
        let low: Vec<f32> = vec![9.0, 10.0, 11.0];
        let close: Vec<f32> = vec![11.0, 12.0, 13.0];
        let result = bop(&open, &high, &low, &close).unwrap();

        assert!((result[0] - (1.0_f32 / 3.0_f32)).abs() < 1e-5);
    }

    #[test]
    fn test_bop_no_nan_values() {
        // BOP has no lookback, so all values should be finite
        let open: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let high: Vec<f64> = vec![12.0, 13.0, 14.0, 15.0, 16.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close: Vec<f64> = vec![11.0, 12.0, 13.0, 14.0, 15.0];
        let result = bop(&open, &high, &low, &close).unwrap();

        for (i, val) in result.iter().enumerate() {
            assert!(val.is_finite(), "BOP[{}] should be finite", i);
        }
    }

    #[test]
    fn test_bop_range() {
        // BOP should always be between -1 and +1
        let open = vec![10.0, 12.0, 9.0, 12.0, 10.0];
        let high = vec![12.0, 13.0, 14.0, 15.0, 10.0];
        let low = vec![9.0, 10.0, 9.0, 12.0, 10.0];
        let close = vec![11.0, 10.0, 14.0, 12.0, 10.0];
        let result = bop(&open, &high, &low, &close).unwrap();

        for (i, val) in result.iter().enumerate() {
            assert!(
                *val >= -1.0 && *val <= 1.0,
                "BOP[{}] = {} should be in range [-1, 1]",
                i,
                val
            );
        }
    }
}
