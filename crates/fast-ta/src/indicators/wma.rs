//! Weighted Moving Average (WMA) indicator.
//!
//! The Weighted Moving Average assigns linearly decreasing weights to older prices,
//! giving more importance to recent data compared to a simple moving average.
//!
//! # Algorithm
//!
//! This implementation uses an O(n) approach where:
//! 1. Initial weighted sum is computed for the first `period` elements
//! 2. For subsequent elements, we update using the rolling formula:
//!    - Add new value with weight `period`
//!    - Subtract the sum of the previous window (each value loses one weight unit)
//!    - Subtract the oldest value (exits window entirely)
//!
//! # NaN Handling
//!
//! Unlike SMA which uses a `nan_count` approach, WMA uses `has_nan` + window rescan
//! because the weighted sum formula requires ALL positions to have values.
//! The rolling update `weighted_sum = weighted_sum - simple_sum + new * period`
//! implicitly adjusts weights for ALL window positions - partial sums with NaN holes
//! cannot be maintained. When NaN exits the window, full O(period) recomputation is
//! required. See `docs/nan-audit-results.md` Phase 4.1 for detailed analysis.
//!
//! # Formula
//!
//! ```text
//! WMA = (P₁ × n + P₂ × (n-1) + P₃ × (n-2) + ... + Pₙ × 1) / (n × (n+1) / 2)
//! ```
//!
//! Where `P₁` is the most recent price (highest weight) and `Pₙ` is the oldest.
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::wma::wma;
//!
//! let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
//! let result = wma(&data, 3).unwrap();
//!
//! // First 2 values are NaN (period-1 lookback)
//! assert!(result[0].is_nan());
//! assert!(result[1].is_nan());
//!
//! // WMA[2] = (1×1 + 2×2 + 3×3) / 6 = 14/6 ≈ 2.333
//! assert!((result[2] - 2.333333).abs() < 1e-5);
//! ```

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Check if a value is invalid (NaN or Infinity).
/// Both NaN and Infinity must propagate through indicators per IEEE 754 policy.
///
/// Note: We cannot use just `.is_nan()` because Infinity must also propagate.
/// Using `.is_finite()` checks for both NaN and ±Infinity in a single operation.
#[inline]
fn is_invalid<T: SeriesElement>(value: T) -> bool {
    !value.is_finite()
}

/// Returns the lookback period for WMA.
///
/// The lookback is the number of NaN values at the start of the output.
/// For WMA, this is `period - 1`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::wma::wma_lookback;
///
/// assert_eq!(wma_lookback(5), 4);
/// assert_eq!(wma_lookback(14), 13);
/// ```
#[inline]
#[must_use]
pub const fn wma_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for WMA.
///
/// This is the smallest input size that will produce at least one valid output.
/// For WMA, this equals the period.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::wma::wma_min_len;
///
/// assert_eq!(wma_min_len(5), 5);
/// assert_eq!(wma_min_len(14), 14);
/// ```
#[inline]
#[must_use]
pub const fn wma_min_len(period: usize) -> usize {
    period
}

/// Computes the Weighted Moving Average (WMA) of a data series.
///
/// Returns a vector of the same length as the input, where the first `period - 1`
/// values are NaN (insufficient lookback data) and subsequent values contain the
/// weighted moving average.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods to average over
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` with the WMA values, or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for the output vector
///
/// # NaN Handling
///
/// - The first `period - 1` elements of the output are NaN
/// - If any input value in the current window contains NaN, it will propagate to the output
///
/// # Example
///
/// ```
/// use fast_ta::indicators::wma::wma;
///
/// let data = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];
/// let result = wma(&data, 3).unwrap();
///
/// assert!(result[0].is_nan());
/// assert!(result[1].is_nan());
/// // WMA[2] = (10×1 + 11×2 + 12×3) / 6 = 68/6 ≈ 11.333
/// assert!((result[2] - 11.333333).abs() < 1e-5);
/// ```
#[inline]
#[must_use = "this returns a Result with the WMA values, which should be used"]
pub fn wma<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    // Validate inputs
    crate::traits::validate_indicator_input(data, period, "wma")?;

    // Weight sum: n + (n-1) + ... + 1 = n*(n+1)/2
    let weight_sum = T::from_usize(period * (period + 1) / 2)?;

    // Initialize result vector with NaN
    let mut result = vec![T::nan(); data.len()];

    // Compute initial weighted sum for the first window
    // Weights: oldest=1, ..., newest=period
    let mut weighted_sum = T::zero();
    let mut simple_sum = T::zero(); // Sum of all values in window (for rolling update)
    let mut has_nan = false;

    for (i, &value) in data.iter().take(period).enumerate() {
        if is_invalid(value) {
            has_nan = true;
        }
        let weight = T::from_usize(i + 1)?; // Weight 1 for oldest, period for newest
        weighted_sum = weighted_sum + value * weight;
        simple_sum = simple_sum + value;
    }

    // Set the first valid WMA value
    if !has_nan {
        result[period - 1] = weighted_sum / weight_sum;
    }

    // Rolling update for remaining elements
    let period_t = T::from_usize(period)?;

    for i in period..data.len() {
        let new_value = data[i];
        let old_value = data[i - period];

        // Check if NaN/Inf is entering or exiting the window
        let nan_entering = is_invalid(new_value);
        let nan_exiting = is_invalid(old_value);

        if nan_entering {
            has_nan = true;
        }

        if has_nan {
            // Window had NaN/Inf - check if it's clear now
            if nan_exiting && !nan_entering {
                // The exiting value was NaN/Inf - check if window is now clean
                has_nan = data[i - period + 1..=i].iter().any(|v| is_invalid(*v));

                if !has_nan {
                    // Window is now clean - recompute sums from scratch
                    weighted_sum = T::zero();
                    simple_sum = T::zero();
                    for (j, &val) in data[i - period + 1..=i].iter().enumerate() {
                        let weight = T::from_usize(j + 1).unwrap();
                        weighted_sum = weighted_sum + val * weight;
                        simple_sum = simple_sum + val;
                    }
                }
            }
        } else {
            // Normal rolling update (no NaN in window)
            // Rolling formula:
            // new_weighted_sum = weighted_sum - simple_sum + new_value * period
            // new_simple_sum = simple_sum - old_value + new_value
            weighted_sum = weighted_sum - simple_sum + new_value * period_t;
            simple_sum = simple_sum - old_value + new_value;
        }

        if has_nan {
            result[i] = T::nan();
        } else {
            result[i] = weighted_sum / weight_sum;
        }
    }

    Ok(result)
}

/// Computes the Weighted Moving Average into a pre-allocated output buffer.
///
/// This variant allows reusing an existing buffer to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods to average over
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid WMA values computed,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
/// - The output buffer is shorter than the input data (`Error::BufferTooSmall`)
///
/// # Example
///
/// ```
/// use fast_ta::indicators::wma::wma_into;
///
/// let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
/// let mut output = vec![0.0_f64; 5];
/// let valid_count = wma_into(&data, 3, &mut output).unwrap();
///
/// assert_eq!(valid_count, 3);
/// assert!(output[0].is_nan());
/// assert!((output[2] - 2.333333).abs() < 1e-5);
/// ```
#[inline]
#[must_use = "this returns a Result with the count of valid WMA values"]
pub fn wma_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize> {
    // Validate inputs
    crate::traits::validate_indicator_input(data, period, "wma")?;

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "wma",
        });
    }

    // Weight sum: n*(n+1)/2
    let weight_sum = T::from_usize(period * (period + 1) / 2)?;

    // Initialize lookback period with NaN
    for item in output.iter_mut().take(period - 1) {
        *item = T::nan();
    }

    // Compute initial weighted sum
    let mut weighted_sum = T::zero();
    let mut simple_sum = T::zero();
    let mut has_nan = false;

    for (i, &value) in data.iter().take(period).enumerate() {
        if is_invalid(value) {
            has_nan = true;
        }
        let weight = T::from_usize(i + 1)?;
        weighted_sum = weighted_sum + value * weight;
        simple_sum = simple_sum + value;
    }

    // Set first valid value
    if has_nan {
        output[period - 1] = T::nan();
    } else {
        output[period - 1] = weighted_sum / weight_sum;
    }

    // Rolling update
    let period_t = T::from_usize(period)?;

    for i in period..data.len() {
        let new_value = data[i];
        let old_value = data[i - period];

        let nan_entering = is_invalid(new_value);
        let nan_exiting = is_invalid(old_value);

        if nan_entering {
            has_nan = true;
        }

        if has_nan {
            if nan_exiting && !nan_entering {
                has_nan = data[i - period + 1..=i].iter().any(|v| is_invalid(*v));

                if !has_nan {
                    weighted_sum = T::zero();
                    simple_sum = T::zero();
                    for (j, &val) in data[i - period + 1..=i].iter().enumerate() {
                        let weight = T::from_usize(j + 1).unwrap();
                        weighted_sum = weighted_sum + val * weight;
                        simple_sum = simple_sum + val;
                    }
                }
            }
        } else {
            weighted_sum = weighted_sum - simple_sum + new_value * period_t;
            simple_sum = simple_sum - old_value + new_value;
        }

        if has_nan {
            output[i] = T::nan();
        } else {
            output[i] = weighted_sum / weight_sum;
        }
    }

    Ok(data.len() - period + 1)
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
        (a - b).abs() < epsilon
    }

    const EPSILON: f64 = 1e-10;
    const EPSILON_F32: f32 = 1e-5;

    // ==================== Lookback and Min Len Tests ====================

    #[test]
    fn test_wma_lookback() {
        assert_eq!(wma_lookback(1), 0);
        assert_eq!(wma_lookback(5), 4);
        assert_eq!(wma_lookback(14), 13);
        assert_eq!(wma_lookback(20), 19);
    }

    #[test]
    fn test_wma_min_len() {
        assert_eq!(wma_min_len(1), 1);
        assert_eq!(wma_min_len(5), 5);
        assert_eq!(wma_min_len(14), 14);
    }

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_wma_basic() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = wma(&data, 3).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());

        // WMA[2] = (1×1 + 2×2 + 3×3) / 6 = (1 + 4 + 9) / 6 = 14/6 ≈ 2.333
        assert!(approx_eq(result[2], 14.0 / 6.0, EPSILON));

        // WMA[3] = (2×1 + 3×2 + 4×3) / 6 = (2 + 6 + 12) / 6 = 20/6 ≈ 3.333
        assert!(approx_eq(result[3], 20.0 / 6.0, EPSILON));

        // WMA[4] = (3×1 + 4×2 + 5×3) / 6 = (3 + 8 + 15) / 6 = 26/6 ≈ 4.333
        assert!(approx_eq(result[4], 26.0 / 6.0, EPSILON));
    }

    #[test]
    fn test_wma_f32() {
        let data = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
        let result = wma(&data, 3).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(approx_eq(result[2], 14.0_f32 / 6.0, EPSILON_F32));
    }

    #[test]
    fn test_wma_period_one() {
        // WMA(1) should equal the input values (weight is just 1/1)
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = wma(&data, 1).unwrap();

        assert_eq!(result.len(), 5);
        assert!(approx_eq(result[0], 1.0, EPSILON));
        assert!(approx_eq(result[1], 2.0, EPSILON));
        assert!(approx_eq(result[2], 3.0, EPSILON));
        assert!(approx_eq(result[3], 4.0, EPSILON));
        assert!(approx_eq(result[4], 5.0, EPSILON));
    }

    #[test]
    fn test_wma_period_two() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = wma(&data, 2).unwrap();

        // WMA[1] = (1×1 + 2×2) / 3 = 5/3 ≈ 1.667
        assert!(result[0].is_nan());
        assert!(approx_eq(result[1], 5.0 / 3.0, EPSILON));

        // WMA[2] = (2×1 + 3×2) / 3 = 8/3 ≈ 2.667
        assert!(approx_eq(result[2], 8.0 / 3.0, EPSILON));

        // WMA[3] = (3×1 + 4×2) / 3 = 11/3 ≈ 3.667
        assert!(approx_eq(result[3], 11.0 / 3.0, EPSILON));

        // WMA[4] = (4×1 + 5×2) / 3 = 14/3 ≈ 4.667
        assert!(approx_eq(result[4], 14.0 / 3.0, EPSILON));
    }

    #[test]
    fn test_wma_period_equals_length() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = wma(&data, 5).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());

        // WMA[4] = (1×1 + 2×2 + 3×3 + 4×4 + 5×5) / 15 = (1+4+9+16+25)/15 = 55/15 ≈ 3.667
        assert!(approx_eq(result[4], 55.0 / 15.0, EPSILON));
    }

    #[test]
    fn test_wma_single_element_period_one() {
        let data = vec![42.0_f64];
        let result = wma(&data, 1).unwrap();

        assert_eq!(result.len(), 1);
        assert!(approx_eq(result[0], 42.0, EPSILON));
    }

    // ==================== Comparison with SMA ====================

    #[test]
    fn test_wma_emphasizes_recent_values() {
        // For an increasing sequence, WMA should be higher than SMA
        // because recent values (which are larger) have more weight
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let wma_result = wma(&data, 5).unwrap();

        // At index 4, SMA = (1+2+3+4+5)/5 = 3.0
        // WMA = (1×1 + 2×2 + 3×3 + 4×4 + 5×5) / 15 = 55/15 ≈ 3.667
        // WMA > SMA for increasing data
        let sma_val = 3.0;
        assert!(wma_result[4] > sma_val);
    }

    #[test]
    fn test_wma_constant_values() {
        // WMA of constant values should equal the constant
        let data = vec![5.0_f64; 10];
        let result = wma(&data, 3).unwrap();

        for i in 2..result.len() {
            assert!(approx_eq(result[i], 5.0, EPSILON));
        }
    }

    // ==================== NaN Handling Tests ====================

    #[test]
    fn test_wma_with_nan_in_initial_window() {
        let data = vec![1.0_f64, f64::NAN, 3.0, 4.0, 5.0];
        let result = wma(&data, 3).unwrap();

        // All values up to and including the first valid window should be NaN
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());

        // At index 3, window is data[1..=3] = [NaN, 3.0, 4.0] - still has NaN!
        assert!(result[3].is_nan());

        // After NaN exits the window (at index 4), window is data[2..=4] = [3.0, 4.0, 5.0]
        // WMA[4] = (3×1 + 4×2 + 5×3) / 6 = 26/6
        assert!(!result[4].is_nan());
        assert!(approx_eq(result[4], 26.0 / 6.0, EPSILON));
    }

    #[test]
    fn test_wma_with_nan_in_middle() {
        let data = vec![1.0_f64, 2.0, 3.0, f64::NAN, 5.0, 6.0];
        let result = wma(&data, 3).unwrap();

        // First window [1, 2, 3] is valid
        assert!(!result[2].is_nan());
        assert!(approx_eq(result[2], 14.0 / 6.0, EPSILON));

        // Windows containing NaN are NaN
        assert!(result[3].is_nan());
        assert!(result[4].is_nan());

        // After NaN exits at index 5, window is [5.0, 6.0, ?] -- we need 3 elements
        // At index 5, window is data[3..=5] which contains NaN at index 3
        // So result[5] should still be NaN
        assert!(result[5].is_nan());
    }

    #[test]
    fn test_wma_with_nan_at_end() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, f64::NAN];
        let result = wma(&data, 3).unwrap();

        // First valid window at index 2
        assert!(!result[2].is_nan());
        assert!(approx_eq(result[2], 14.0 / 6.0, EPSILON));

        // Window at index 3: [2, 3, 4] - no NaN
        assert!(!result[3].is_nan());
        assert!(approx_eq(result[3], 20.0 / 6.0, EPSILON));

        // Window at index 4: [3, 4, NaN] - contains NaN
        assert!(result[4].is_nan());
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_wma_empty_input() {
        let data: Vec<f64> = vec![];
        let result = wma(&data, 3);

        assert!(result.is_err());
    }

    #[test]
    fn test_wma_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = wma(&data, 0);

        assert!(result.is_err());
    }

    #[test]
    fn test_wma_period_too_large() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = wma(&data, 5);

        assert!(result.is_err());
    }

    // ==================== wma_into Tests ====================

    #[test]
    fn test_wma_into_basic() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f64; 5];
        let valid_count = wma_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!(approx_eq(output[2], 14.0 / 6.0, EPSILON));
    }

    #[test]
    fn test_wma_into_buffer_too_small() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f64; 3];
        let result = wma_into(&data, 3, &mut output);

        assert!(result.is_err());
    }

    #[test]
    fn test_wma_into_with_nan() {
        let data = vec![1.0_f64, 2.0, 3.0, f64::NAN, 5.0, 6.0];
        let mut output = vec![0.0_f64; 6];
        let valid_count = wma_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 4);
assert!(output[2].is_nan() == false);
        assert!(output[3].is_nan());
        assert!(output[4].is_nan());
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_wma_large_period() {
        let data: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let result = wma(&data, 50).unwrap();

        assert_eq!(result.len(), 100);
        for i in 0..49 {
            assert!(result[i].is_nan());
        }
        assert!(!result[49].is_nan());
    }

    #[test]
    fn test_wma_very_small_values() {
        let data = vec![1e-10_f64, 2e-10, 3e-10, 4e-10, 5e-10];
        let result = wma(&data, 3).unwrap();

        assert!(!result[2].is_nan());
        // Result should be proportional to input
        assert!(result[2] > 0.0 && result[2] < 5e-10);
    }
}