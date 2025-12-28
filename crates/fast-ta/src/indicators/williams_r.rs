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
//! # Precision Behavior
//!
//! When `PrecisionMode::High` is active and input type is `f32`:
//! - %R calculation (division by range) performed in `f64`
//! - Prevents precision loss when range is very small
//!
//! **Tolerance**: abs(0.01) when comparing f32 High mode to f64 reference.
//! Williams %R is bounded -100 to 0, so absolute tolerance is appropriate.
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
use crate::kernels::rolling_extrema::MonotonicDeque;
use crate::precision::{current_precision_mode, PrecisionMode};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns true if we should use f64 precision for the given type.
///
/// Uses f64 when:
/// - Input type is f32 AND PrecisionMode is High
#[inline]
fn use_f64_precision<T: 'static>() -> bool {
    use std::any::TypeId;
    TypeId::of::<T>() == TypeId::of::<f32>() && current_precision_mode() == PrecisionMode::High
}

/// Computes %R = -100 × (highest_high - close) / range with appropriate precision.
///
/// For f32 inputs in High precision mode, the calculation is performed in f64.
#[inline]
fn compute_williams_r_value<T: SeriesElement + 'static>(
    highest_high: T,
    close: T,
    range: T,
    neg_hundred: T,
) -> Result<T> {
    if use_f64_precision::<T>() {
        let hh_f64 = highest_high.to_f64().unwrap_or(0.0);
        let c_f64 = close.to_f64().unwrap_or(0.0);
        let range_f64 = range.to_f64().unwrap_or(1.0);
        // Match original: neg_hundred * (hh - c) / range
        let wr = -100.0 * (hh_f64 - c_f64) / range_f64;
        T::from_f64(wr)
    } else {
        // Match original: neg_hundred * (hh - c) / range
        Ok(neg_hundred * (highest_high - close) / range)
    }
}

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

/// Core Williams %R computation using monotonic deque for O(n) complexity.
///
/// This implementation uses MonotonicDeque directly to avoid intermediate
/// allocations and compute everything in a single pass over the data.
fn compute_williams_r_core<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let neg_hundred = T::from_i32(-100)?;
    let neg_fifty = T::from_i32(-50)?;
    let lookback = williams_r_lookback(period);
    let n = close.len();

    // Initialize monotonic deques for tracking rolling max/min
    let mut max_deque: MonotonicDeque<T> = MonotonicDeque::new(period);
    let mut min_deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    // Single pass computation
    for i in 0..n {
        // Update deques with current values
        max_deque.push_max(i, high);
        min_deque.push_min(i, low);

        if i < lookback {
            // Lookback period: output NaN
            output[i] = T::nan();
        } else {
            // Get rolling highest high and lowest low
            let hh = max_deque.get_extremum(high);
            let ll = min_deque.get_extremum(low);
            let c = close[i];

            if is_invalid(hh) || is_invalid(ll) || is_invalid(c) {
                output[i] = T::nan();
            } else {
                let range = hh - ll;

                if range <= T::zero() {
                    // When high == low (no range), return midpoint (-50)
                    output[i] = neg_fifty;
                } else {
                    // %R = -100 × (HH - Close) / (HH - LL)
                    output[i] = compute_williams_r_value(hh, c, range, neg_hundred)?;
                }
            }
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
        let low: Vec<f64> = high.iter().map(|&h| h - 5.0 - ((h * 0.1).sin().abs() * 2.0)).collect();
        let close: Vec<f64> = high
            .iter()
            .zip(low.iter())
            .map(|(&h, &l)| l + (h - l) * 0.5)
            .collect();

        let result = williams_r(&high, &low, &close, 14).unwrap();

        assert_eq!(result.len(), 30);

        // First 13 values should be NaN
        for i in 0..13 {
            assert!(
                result[i].is_nan(),
                "Expected NaN at index {}, got {}",
                i,
                result[i]
            );
        }

        // All other values should be in [-100, 0]
        for i in 13..result.len() {
            assert!(
                !result[i].is_nan(),
                "Expected valid value at index {}, got NaN",
                i
            );
            assert!(
                result[i] >= -100.0 && result[i] <= 0.0,
                "Value at {} = {} should be in [-100, 0]",
                i,
                result[i]
            );
        }
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_williams_r_empty_input() {
        let empty: Vec<f64> = vec![];
        let result = williams_r(&empty, &empty, &empty, 5);

        assert!(result.is_err());
    }

    #[test]
    fn test_williams_r_zero_period() {
        let high = vec![10.0_f64];
        let low = vec![9.0];
        let close = vec![9.5];

        let result = williams_r(&high, &low, &close, 0);

        assert!(result.is_err());
    }

    #[test]
    fn test_williams_r_length_mismatch() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0];
        let close = vec![9.5, 10.5, 11.5];

        let result = williams_r(&high, &low, &close, 2);

        assert!(result.is_err());
    }

    #[test]
    fn test_williams_r_insufficient_data() {
        let high = vec![10.0_f64, 11.0];
        let low = vec![9.0, 10.0];
        let close = vec![9.5, 10.5];

        let result = williams_r(&high, &low, &close, 5);

        assert!(result.is_err());
    }

    #[test]
    fn test_williams_r_into_buffer_too_small() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];
        let mut output = vec![0.0; 2]; // Too small

        let result = williams_r_into(&high, &low, &close, 2, &mut output);

        assert!(result.is_err());
    }

    #[test]
    fn test_williams_r_into_valid() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0];
        let mut output = vec![0.0; 5];

        let valid_count = williams_r_into(&high, &low, &close, 3, &mut output).unwrap();

        assert_eq!(valid_count, 3); // 5 - 2 = 3

        // First 2 values should be NaN
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());

        // Last 3 values should be valid and in range
        for i in 2..5 {
            assert!(!output[i].is_nan());
            assert!(output[i] >= -100.0 && output[i] <= 0.0);
        }
    }

    // ==================== NaN Handling Tests ====================

    #[test]
    fn test_williams_r_nan_in_high() {
        let high = vec![10.0_f64, 11.0, f64::NAN, 11.5, 12.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0];

        let result = williams_r(&high, &low, &close, 3).unwrap();

        // Result should still be computed, NaN in input propagates
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_williams_r_nan_in_low() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5];
        let low = vec![9.0, 10.0, f64::NAN, 10.5, 11.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0];

        let result = williams_r(&high, &low, &close, 3).unwrap();

        // Result should still be computed, NaN in input propagates
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_williams_r_nan_in_close() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5];
        let close = vec![9.5, 10.5, f64::NAN, 11.0, 12.0];

        let result = williams_r(&high, &low, &close, 3).unwrap();

        // Result should still be computed, NaN in input propagates
        assert_eq!(result.len(), 5);
    }
}