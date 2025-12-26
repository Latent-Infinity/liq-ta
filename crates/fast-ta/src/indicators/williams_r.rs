//! Williams %R indicator.
//!
//! Williams %R is a momentum indicator that measures overbought and oversold levels.
//! It was developed by Larry Williams and is similar to the Stochastic oscillator,
//! but expressed on a negative scale from -100 to 0.
//!
//! # Algorithm
//!
//! Williams %R compares the closing price to the highest high and lowest low
//! over a lookback period:
//!
//! ```text
//! %R = -100 × (Highest High - Close) / (Highest High - Lowest Low)
//! ```
//!
//! # Interpretation
//!
//! - %R = 0: Close is at the highest high (overbought)
//! - %R = -100: Close is at the lowest low (oversold)
//! - %R = -50: Close is at the midpoint of the range
//! - %R > -20: Overbought territory
//! - %R < -80: Oversold territory
//!
//! # Edge Cases
//!
//! - When Highest High == Lowest Low (range = 0), %R = -50 (midpoint)
//!
//! # NaN Handling
//!
//! The first `period - 1` values are NaN (insufficient lookback data).
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::williams_r::williams_r;
//!
//! let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35];
//! let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86];
//! let close = vec![48.16_f64, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32];
//!
//! let result = williams_r(&high, &low, &close, 5).unwrap();
//!
//! // First 4 values are NaN
//! assert!(result[3].is_nan());
//!
//! // Williams %R values start from index 4
//! assert!(!result[4].is_nan());
//! assert!(result[4] >= -100.0 && result[4] <= 0.0);
//! ```

use crate::error::{Error, Result};
use crate::kernels::rolling_extrema::{rolling_max, rolling_min};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns the lookback period for Williams %R.
///
/// The lookback is the number of NaN values at the start of the output.
/// For Williams %R, this is `period - 1`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::williams_r::williams_r_lookback;
///
/// assert_eq!(williams_r_lookback(14), 13);
/// assert_eq!(williams_r_lookback(5), 4);
/// ```
#[inline]
#[must_use]
pub const fn williams_r_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for Williams %R.
///
/// This is the smallest input size that will produce at least one valid output.
/// For Williams %R, this equals the period.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::williams_r::williams_r_min_len;
///
/// assert_eq!(williams_r_min_len(14), 14);
/// assert_eq!(williams_r_min_len(5), 5);
/// ```
#[inline]
#[must_use]
pub const fn williams_r_min_len(period: usize) -> usize {
    period
}

/// Computes Williams %R for OHLC price data.
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `close` - The close prices series
/// * `period` - The lookback period (commonly 14)
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` with Williams %R values in range [-100, 0].
/// The first `period - 1` values are NaN.
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The series have different lengths (`Error::LengthMismatch`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for the output vector
///
/// # Example
///
/// ```
/// use fast_ta::indicators::williams_r::williams_r;
///
/// let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35];
/// let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86];
/// let close = vec![48.16_f64, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32];
///
/// let result = williams_r(&high, &low, &close, 5).unwrap();
///
/// // Values are in [-100, 0] range
/// for i in 4..result.len() {
///     assert!(result[i] >= -100.0 && result[i] <= 0.0);
/// }
/// ```
#[must_use = "this returns a Result with Williams %R values, which should be used"]
pub fn williams_r<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Result<Vec<T>> {
    validate_inputs(high, low, close, period)?;

    let n = high.len();
    let mut result = vec![T::nan(); n];

    compute_williams_r_core(high, low, close, period, &mut result)?;

    Ok(result)
}

/// Computes Williams %R into a pre-allocated output buffer.
///
/// This variant allows reusing an existing buffer to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `close` - The close prices series
/// * `period` - The lookback period
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid Williams %R values computed (n - period + 1),
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The series have different lengths (`Error::LengthMismatch`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
/// - The output buffer is shorter than the input data
///
/// # Example
///
/// ```
/// use fast_ta::indicators::williams_r::williams_r_into;
///
/// let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35];
/// let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86];
/// let close = vec![48.16_f64, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32];
/// let mut output = vec![0.0_f64; 8];
///
/// let valid_count = williams_r_into(&high, &low, &close, 5, &mut output).unwrap();
/// assert_eq!(valid_count, 4); // 8 - 4 = 4 valid values
/// ```
#[must_use = "this returns a Result with the count of valid Williams %R values"]
pub fn williams_r_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    validate_inputs(high, low, close, period)?;

    let n = high.len();

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "williams_r",
        });
    }

    // Initialize lookback period with NaN
    let lookback = williams_r_lookback(period);
    for i in 0..lookback.min(n) {
        output[i] = T::nan();
    }

    compute_williams_r_core(high, low, close, period, output)?;

    Ok(n.saturating_sub(lookback))
}

/// Validates input data.
fn validate_inputs<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if high.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = high.len();

    if low.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", n, low.len()),
        });
    }

    if close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, close has {}", n, close.len()),
        });
    }

    if n < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: n,
            indicator: "williams_r",
        });
    }

    Ok(())
}

/// Core Williams %R computation.
fn compute_williams_r_core<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let neg_hundred = T::from_i32(-100)?;
    let neg_fifty = T::from_i32(-50)?;

    // Calculate rolling highest high and lowest low
    let highest_high = rolling_max(high, period)?;
    let lowest_low = rolling_min(low, period)?;

    // Calculate Williams %R for each valid position
    let lookback = williams_r_lookback(period);
    for i in lookback..close.len() {
        let hh = highest_high[i];
        let ll = lowest_low[i];
        let c = close[i];

        if is_invalid(hh) || is_invalid(ll) || is_invalid(c) {
            output[i] = T::nan();
            continue;
        }

        let range = hh - ll;

        if range <= T::zero() {
            // When high == low (no range), return midpoint (-50)
            output[i] = neg_fifty;
        } else {
            // %R = -100 × (HH - Close) / (HH - LL)
            output[i] = neg_hundred * (hh - c) / range;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;
    use num_traits::Float;

    fn approx_eq<T: Float>(a: T, b: T, epsilon: T) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        if a.is_nan() || b.is_nan() {
            return false;
        }
        (a - b).abs() < epsilon
    }

    const EPSILON: f64 = 1e-10;

    // ==================== Lookback and Min Length Tests ====================

    #[test]
    fn test_williams_r_lookback() {
        assert_eq!(williams_r_lookback(14), 13);
        assert_eq!(williams_r_lookback(5), 4);
        assert_eq!(williams_r_lookback(1), 0);
        assert_eq!(williams_r_lookback(0), 0);
    }

    #[test]
    fn test_williams_r_min_len() {
        assert_eq!(williams_r_min_len(14), 14);
        assert_eq!(williams_r_min_len(5), 5);
        assert_eq!(williams_r_min_len(1), 1);
    }

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_williams_r_basic() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.5, 13.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.5, 12.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 12.0, 13.0];

        let result = williams_r(&high, &low, &close, 3).unwrap();

        assert_eq!(result.len(), 8);

        // First 2 values should be NaN (period - 1 = 2)
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());

        // Values from index 2 onwards should be valid and in [-100, 0]
        for i in 2..result.len() {
            assert!(!result[i].is_nan(), "Value at {} should not be NaN", i);
            assert!(
                result[i] >= -100.0 && result[i] <= 0.0,
                "Value at {} = {} should be in [-100, 0]",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_williams_r_f32() {
        let high = vec![10.0_f32, 11.0, 12.0, 11.5, 12.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0];

        let result = williams_r(&high, &low, &close, 3).unwrap();

        assert_eq!(result.len(), 5);
        assert!(!result[2].is_nan());
    }

    #[test]
    fn test_williams_r_period_1() {
        let high = vec![10.0_f64, 11.0, 10.5];
        let low = vec![9.0, 10.0, 9.5];
        let close = vec![9.5, 10.5, 10.0];

        let result = williams_r(&high, &low, &close, 1).unwrap();

        // With period 1, lookback = 0, all values valid
        for i in 0..result.len() {
            assert!(!result[i].is_nan());
        }
    }

    // ==================== Known Value Tests ====================

    #[test]
    fn test_williams_r_close_at_highest_high() {
        // Close at highest high should give %R = 0
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5];
        let close = vec![9.5, 10.5, 12.0, 11.0, 12.5]; // Close at index 4 = high

        let result = williams_r(&high, &low, &close, 3).unwrap();

        // At index 4, close = 12.5, highest high over [2,3,4] = max(12.0, 11.5, 12.5) = 12.5
        // %R = -100 × (12.5 - 12.5) / (12.5 - 10.5) = 0
        assert!(
            approx_eq(result[4], 0.0, EPSILON),
            "Expected %R = 0 when close at highest high, got {}",
            result[4]
        );
    }

    #[test]
    fn test_williams_r_close_at_lowest_low() {
        // Close at lowest low should give %R = -100
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 10.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 9.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 9.5]; // Close at index 4 = low

        let result = williams_r(&high, &low, &close, 3).unwrap();

        // At index 4, close = 9.5, highest high = max(12.0, 11.5, 10.5) = 12.0
        // lowest low = min(11.0, 10.5, 9.5) = 9.5
        // %R = -100 × (12.0 - 9.5) / (12.0 - 9.5) = -100
        assert!(
            approx_eq(result[4], -100.0, EPSILON),
            "Expected %R = -100 when close at lowest low, got {}",
            result[4]
        );
    }

    #[test]
    fn test_williams_r_close_at_midpoint() {
        // Close at midpoint should give %R = -50
        let high = vec![10.0_f64, 10.0, 10.0];
        let low = vec![8.0, 8.0, 8.0];
        let close = vec![9.0, 9.0, 9.0]; // Close at midpoint

        let result = williams_r(&high, &low, &close, 3).unwrap();

        // HH = 10, LL = 8, Close = 9 (midpoint)
        // %R = -100 × (10 - 9) / (10 - 8) = -100 × 1/2 = -50
        assert!(
            approx_eq(result[2], -50.0, EPSILON),
            "Expected %R = -50 when close at midpoint, got {}",
            result[2]
        );
    }

    #[test]
    fn test_williams_r_high_equals_low() {
        // Edge case: high == low (no range) should give %R = -50
        let high = vec![10.0_f64, 10.0, 10.0, 10.0, 10.0];
        let low = vec![10.0, 10.0, 10.0, 10.0, 10.0];
        let close = vec![10.0, 10.0, 10.0, 10.0, 10.0];

        let result = williams_r(&high, &low, &close, 3).unwrap();

        for i in 2..result.len() {
            assert!(
                approx_eq(result[i], -50.0, EPSILON),
                "Expected %R = -50 when high == low, got {} at {}",
                result[i],
                i
            );
        }
    }

    // ==================== Value Range Tests ====================

    #[test]
    fn test_williams_r_values_in_range() {
        let high: Vec<f64> = (0..30)
            .map(|i| 100.0 + (i as f64) * 2.0 + 5.0 * ((i as f64) * 0.5).sin())
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();
        let close: Vec<f64> = high.iter().map(|h| h - 1.5).collect();

        let result = williams_r(&high, &low, &close, 5).unwrap();

        for i in 4..result.len() {
            assert!(
                result[i] >= -100.0 && result[i] <= 0.0,
                "%R at {} = {} out of range [-100, 0]",
                i,
                result[i]
            );
        }
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_williams_r_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];

        let result = williams_r(&high, &low, &close, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_williams_r_zero_period() {
        let high = vec![10.0_f64; 10];
        let low = vec![9.0_f64; 10];
        let close = vec![9.5_f64; 10];

        let result = williams_r(&high, &low, &close, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_williams_r_insufficient_data() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];

        let result = williams_r(&high, &low, &close, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_williams_r_mismatched_lengths() {
        let high = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0, 12.0]; // One less
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5];

        let result = williams_r(&high, &low, &close, 3);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_williams_r_minimum_data() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];

        let result = williams_r(&high, &low, &close, 3);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output[2].is_nan());
    }

    // ==================== williams_r_into Tests ====================

    #[test]
    fn test_williams_r_into_basic() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.5, 13.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.5, 12.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 12.0, 13.0];
        let mut output = vec![0.0_f64; 8];

        let valid_count = williams_r_into(&high, &low, &close, 3, &mut output).unwrap();

        assert_eq!(valid_count, 6); // 8 - 2 = 6 valid values

        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!(!output[2].is_nan());
    }

    #[test]
    fn test_williams_r_into_buffer_too_small() {
        let high = vec![10.0_f64; 10];
        let low = vec![9.0_f64; 10];
        let close = vec![9.5_f64; 10];
        let mut output = vec![0.0_f64; 5]; // Too short

        let result = williams_r_into(&high, &low, &close, 3, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_williams_r_and_williams_r_into_produce_same_result() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.5, 13.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.5, 12.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 12.0, 13.0];

        let result1 = williams_r(&high, &low, &close, 3).unwrap();

        let mut result2 = vec![0.0_f64; 8];
        williams_r_into(&high, &low, &close, 3, &mut result2).unwrap();

        for i in 0..8 {
            assert!(
                approx_eq(result1[i], result2[i], EPSILON),
                "Mismatch at {}: {} vs {}",
                i,
                result1[i],
                result2[i]
            );
        }
    }

    // ==================== NaN Handling Tests ====================

    #[test]
    fn test_williams_r_with_nan_in_data() {
        let high = vec![10.0_f64, f64::NAN, 12.0, 11.5, 12.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5];
        let close = vec![9.5, f64::NAN, 11.5, 11.0, 12.0];

        let result = williams_r(&high, &low, &close, 3).unwrap();

        // NaN in rolling window may propagate
        assert_eq!(result.len(), 5);
    }

    // ==================== Property-Based Tests ====================

    #[test]
    fn test_williams_r_output_length_equals_input_length() {
        for len in [10, 20, 50, 100] {
            for period in [3, 5, 14] {
                if period <= len {
                    let high: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) + 2.0).collect();
                    let low: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) - 2.0).collect();
                    let close: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64)).collect();

                    let result = williams_r(&high, &low, &close, period).unwrap();
                    assert_eq!(result.len(), len);
                }
            }
        }
    }

    #[test]
    fn test_williams_r_nan_count() {
        for period in [3, 5, 14] {
            let len = 30;
            let high: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) + 2.0).collect();
            let low: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) - 2.0).collect();
            let close: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64)).collect();

            let result = williams_r(&high, &low, &close, period).unwrap();

            let nan_count = result.iter().filter(|x| x.is_nan()).count();
            let expected = period - 1;
            assert_eq!(
                nan_count, expected,
                "Expected {} NaN values for period {}, got {}",
                expected, period, nan_count
            );
        }
    }

    // ==================== Real-World Scenario Tests ====================

    #[test]
    fn test_williams_r_uptrend() {
        // In a strong uptrend, close tends to be near the high
        // So %R should be closer to 0
        let mut high = Vec::new();
        let mut low = Vec::new();
        let mut close = Vec::new();

        for i in 0..20 {
            high.push(100.0 + (i as f64) * 2.0 + 1.0);
            low.push(100.0 + (i as f64) * 2.0 - 1.0);
            close.push(100.0 + (i as f64) * 2.0 + 0.8); // Close near high
        }

        let result = williams_r(&high, &low, &close, 5).unwrap();

        // In uptrend with close near high, %R should be > -20 (overbought zone)
        for i in 10..result.len() {
            assert!(
                result[i] > -30.0,
                "%R should be elevated in uptrend, got {} at {}",
                result[i],
                i
            );
        }
    }

    #[test]
    fn test_williams_r_downtrend() {
        // In a strong downtrend, close tends to be near the low
        // So %R should be closer to -100
        let mut high = Vec::new();
        let mut low = Vec::new();
        let mut close = Vec::new();

        for i in 0..20 {
            high.push(200.0 - (i as f64) * 2.0 + 1.0);
            low.push(200.0 - (i as f64) * 2.0 - 1.0);
            close.push(200.0 - (i as f64) * 2.0 - 0.8); // Close near low
        }

        let result = williams_r(&high, &low, &close, 5).unwrap();

        // In downtrend with close near low, %R should be < -70 (oversold zone)
        for i in 10..result.len() {
            assert!(
                result[i] < -70.0,
                "%R should be depressed in downtrend, got {} at {}",
                result[i],
                i
            );
        }
    }

    #[test]
    fn test_williams_r_range_bound() {
        // In a range-bound market, %R should oscillate around -50
        let high = vec![100.0_f64; 20];
        let low = vec![98.0_f64; 20];
        let close = vec![99.0_f64; 20]; // Midpoint

        let result = williams_r(&high, &low, &close, 5).unwrap();

        for i in 4..result.len() {
            assert!(
                approx_eq(result[i], -50.0, EPSILON),
                "%R should be -50 in range-bound, got {} at {}",
                result[i],
                i
            );
        }
    }
}
