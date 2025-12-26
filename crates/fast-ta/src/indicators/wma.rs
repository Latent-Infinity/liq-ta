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
use crate::utils::is_invalid;

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

        // Check if NaN is entering or exiting the window
        let nan_entering = is_invalid(new_value);
        let nan_exiting = is_invalid(old_value);

        if nan_entering {
            has_nan = true;
        }

        if has_nan {
            // Window had NaN - check if it's clear now
            if nan_exiting && !nan_entering {
                // The exiting value was NaN - check if window is now NaN-free
                has_nan = data[i - period + 1..=i].iter().any(|&v| is_invalid(v));

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
                has_nan = data[i - period + 1..=i].iter().any(|&v| is_invalid(v));

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

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_wma_with_nan_in_data() {
        let data = vec![1.0_f64, 2.0, f64::NAN, 4.0, 5.0, 6.0];
        let result = wma(&data, 3).unwrap();

        assert!(result[0].is_nan()); // lookback
        assert!(result[1].is_nan()); // lookback
        assert!(result[2].is_nan()); // window contains NaN
        assert!(result[3].is_nan()); // window contains NaN
        assert!(result[4].is_nan()); // window contains NaN

        // WMA[5] = (4×1 + 5×2 + 6×3) / 6 = (4 + 10 + 18) / 6 = 32/6 ≈ 5.333
        assert!(approx_eq(result[5], 32.0 / 6.0, EPSILON));
    }

    #[test]
    fn test_wma_negative_values() {
        let data = vec![-5.0_f64, -3.0, -1.0, 1.0, 3.0, 5.0];
        let result = wma(&data, 3).unwrap();

        // WMA[2] = (-5×1 + -3×2 + -1×3) / 6 = (-5 - 6 - 3) / 6 = -14/6 ≈ -2.333
        assert!(approx_eq(result[2], -14.0 / 6.0, EPSILON));

        // WMA[5] = (1×1 + 3×2 + 5×3) / 6 = (1 + 6 + 15) / 6 = 22/6 ≈ 3.667
        assert!(approx_eq(result[5], 22.0 / 6.0, EPSILON));
    }

    #[test]
    fn test_wma_large_values() {
        let data = vec![1e15_f64, 2e15, 3e15, 4e15, 5e15];
        let result = wma(&data, 3).unwrap();

        // WMA[2] = (1e15×1 + 2e15×2 + 3e15×3) / 6 = 14e15/6
        assert!(approx_eq(result[2], 14e15 / 6.0, 1e5));
    }

    #[test]
    fn test_wma_small_values() {
        let data = vec![1e-15_f64, 2e-15, 3e-15, 4e-15, 5e-15];
        let result = wma(&data, 3).unwrap();

        assert!(approx_eq(result[2], 14e-15 / 6.0, 1e-25));
    }

    #[test]
    fn test_wma_infinity_handling() {
        let data = vec![1.0_f64, f64::INFINITY, 3.0, 4.0, 5.0];
        let result = wma(&data, 3).unwrap();

        assert!(result[2].is_nan()); // Window contains infinity
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_wma_empty_input() {
        let data: Vec<f64> = vec![];
        let result = wma(&data, 3);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_wma_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = wma(&data, 0);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_wma_period_exceeds_length() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = wma(&data, 5);

        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                required: 5,
                actual: 3,
                ..
            })
        ));
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
        assert!(approx_eq(output[3], 20.0 / 6.0, EPSILON));
        assert!(approx_eq(output[4], 26.0 / 6.0, EPSILON));
    }

    #[test]
    fn test_wma_into_buffer_reuse() {
        let data1 = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let data2 = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
        let mut output = vec![0.0_f64; 5];

        wma_into(&data1, 3, &mut output).unwrap();
        assert!(approx_eq(output[2], 14.0 / 6.0, EPSILON));

        wma_into(&data2, 3, &mut output).unwrap();
        // WMA[2] for [5,4,3] = (5×1 + 4×2 + 3×3) / 6 = (5+8+9)/6 = 22/6
        assert!(approx_eq(output[2], 22.0 / 6.0, EPSILON));
    }

    #[test]
    fn test_wma_into_insufficient_output() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f64; 3]; // Too short
        let result = wma_into(&data, 3, &mut output);

        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_wma_into_empty_input() {
        let data: Vec<f64> = vec![];
        let mut output = vec![0.0_f64; 5];
        let result = wma_into(&data, 3, &mut output);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_wma_into_f32() {
        let data = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f32; 5];
        let valid_count = wma_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(approx_eq(output[2], 14.0_f32 / 6.0, EPSILON_F32));
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_wma_and_wma_into_produce_same_result() {
        let data = vec![10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        let result1 = wma(&data, 4).unwrap();

        let mut result2 = vec![0.0_f64; data.len()];
        wma_into(&data, 4, &mut result2).unwrap();

        for i in 0..data.len() {
            assert!(approx_eq(result1[i], result2[i], EPSILON));
        }
    }

    #[test]
    fn test_wma_valid_count() {
        let data = vec![1.0_f64; 100];
        let mut output = vec![0.0_f64; 100];

        let valid_count = wma_into(&data, 10, &mut output).unwrap();
        assert_eq!(valid_count, 91); // 100 - 10 + 1

        let valid_count = wma_into(&data, 1, &mut output).unwrap();
        assert_eq!(valid_count, 100); // All values valid

        let valid_count = wma_into(&data, 100, &mut output).unwrap();
        assert_eq!(valid_count, 1); // Only last value valid
    }

    // ==================== Property-Based-Like Tests ====================

    #[test]
    fn test_wma_output_length_equals_input_length() {
        for len in [5, 10, 50, 100] {
            for period in [1, 2, 5] {
                if period <= len {
                    let data: Vec<f64> = (0..len).map(|x| x as f64).collect();
                    let result = wma(&data, period).unwrap();
                    assert_eq!(result.len(), len);
                }
            }
        }
    }

    #[test]
    fn test_wma_nan_count() {
        // First (period - 1) values should be NaN
        for period in 1..=10 {
            let data: Vec<f64> = (0..20).map(|x| x as f64).collect();
            let result = wma(&data, period).unwrap();

            let nan_count = result.iter().filter(|x| x.is_nan()).count();
            assert_eq!(nan_count, period - 1);
        }
    }

    #[test]
    fn test_wma_weight_distribution() {
        // Test that recent values have more weight
        // For data [0, 0, 0, 0, 100] with period 5:
        // WMA = (0×1 + 0×2 + 0×3 + 0×4 + 100×5) / 15 = 500/15 ≈ 33.33
        // SMA would be 100/5 = 20
        // WMA > SMA because the 100 has the highest weight

        let data = vec![0.0_f64, 0.0, 0.0, 0.0, 100.0];
        let result = wma(&data, 5).unwrap();

        // WMA = 500/15 ≈ 33.33
        assert!(approx_eq(result[4], 500.0 / 15.0, EPSILON));
        assert!(result[4] > 20.0); // WMA > SMA
    }

    // ==================== TA-Lib Reference Values ====================

    #[test]
    fn test_wma_talib_reference() {
        // Test values that can be cross-checked with TA-Lib
        // Using period=5 on a known sequence
        let data = vec![
            22.27_f64, 22.19, 22.08, 22.17, 22.18, 22.13, 22.23, 22.43, 22.24, 22.29,
        ];
        let result = wma(&data, 5).unwrap();

        // WMA[4] = (22.27×1 + 22.19×2 + 22.08×3 + 22.17×4 + 22.18×5) / 15
        //        = (22.27 + 44.38 + 66.24 + 88.68 + 110.9) / 15
        //        = 332.47 / 15 ≈ 22.1647
        assert!(approx_eq(result[4], 332.47 / 15.0, 1e-4));

        // First 4 should be NaN
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
    }
}
