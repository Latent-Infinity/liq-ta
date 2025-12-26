//! Simple Moving Average (SMA) indicator.
//!
//! The Simple Moving Average is a trend-following indicator that smooths price data
//! by creating a constantly updated average price. The SMA calculates the arithmetic
//! mean of a given set of values over a specified period.
//!
//! # Algorithm
//!
//! This implementation uses an O(n) rolling sum approach where:
//! 1. Initial sum is computed for the first `period` elements
//! 2. For each subsequent element, we add the new value and subtract the oldest value
//! 3. This maintains the rolling sum with O(1) operations per element
//!
//! # Formula
//!
//! ```text
//! SMA = (P1 + P2 + ... + Pn) / n
//! ```
//!
//! Where `P` is the price and `n` is the period.
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::sma::sma;
//!
//! let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
//! let result = sma(&data, 3).unwrap();
//!
//! // First 2 values are NaN (period-1 lookback)
//! assert!(result[0].is_nan());
//! assert!(result[1].is_nan());
//!
//! // SMA starts from index 2 (period - 1)
//! assert!((result[2] - 2.0).abs() < 1e-10); // (1+2+3)/3 = 2.0
//! assert!((result[3] - 3.0).abs() < 1e-10); // (2+3+4)/3 = 3.0
//! assert!((result[4] - 4.0).abs() < 1e-10); // (3+4+5)/3 = 4.0
//! ```

use crate::error::{Error, Result};
use crate::kernels::simd;
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Helper to compute initial sum and NaN count using SIMD for f64.
///
/// The SIMD `sum_and_count_f64` kernel properly handles NaN by tracking count
/// of valid values, providing 2-4x speedup even with NaN in the data.
#[inline]
fn compute_initial_sum<T: SeriesElement + 'static>(data: &[T], period: usize) -> (T, usize) {
    use std::any::TypeId;
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        // SAFETY: We verified T is f64 via TypeId
        let data_f64: &[f64] = unsafe { &*(data as *const [T] as *const [f64]) };
        let (sum, count) = simd::sum_and_count_f64(&data_f64[..period]);
        let nan_count = period - count;
        // SAFETY: We verified T is f64 via TypeId
        let sum_t: T = unsafe { *(&sum as *const f64 as *const T) };
        return (sum_t, nan_count);
    }
    // Fallback to scalar for f32 (no SIMD kernel yet)
    let mut sum = T::zero();
    let mut nan_count = 0usize;
    for &value in data.iter().take(period) {
        if is_invalid(value) {
            nan_count += 1;
        } else {
            sum = sum + value;
        }
    }
    (sum, nan_count)
}

/// Returns the lookback period for SMA.
///
/// The lookback is the number of NaN values at the start of the output.
/// For SMA, this is `period - 1`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::sma::sma_lookback;
///
/// assert_eq!(sma_lookback(5), 4);
/// assert_eq!(sma_lookback(14), 13);
/// ```
#[inline]
#[must_use]
pub const fn sma_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for SMA.
///
/// This is the smallest input size that will produce at least one valid output.
/// For SMA, this equals the period.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::sma::sma_min_len;
///
/// assert_eq!(sma_min_len(5), 5);
/// assert_eq!(sma_min_len(14), 14);
/// ```
#[inline]
#[must_use]
pub const fn sma_min_len(period: usize) -> usize {
    period
}

/// Computes the Simple Moving Average (SMA) of a data series.
///
/// Returns a vector of the same length as the input, where the first `period - 1`
/// values are NaN (insufficient lookback data) and subsequent values contain the
/// moving average.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods to average over
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` with the SMA values, or an error if validation fails.
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
/// - Uses SIMD acceleration for f64 when the `simd` feature is enabled (default)
///
/// # NaN Handling
///
/// - The first `period - 1` elements of the output are NaN
/// - If any input value in the current window contains NaN, it will propagate to the output
///
/// # Example
///
/// ```
/// use fast_ta::indicators::sma::sma;
///
/// let data = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];
/// let result = sma(&data, 3).unwrap();
///
/// assert!(result[0].is_nan());
/// assert!(result[1].is_nan());
/// assert!((result[2] - 11.0).abs() < 1e-10);
/// ```
#[inline]
#[must_use = "this returns a Result with the SMA values, which should be used"]
pub fn sma<T: SeriesElement + 'static>(data: &[T], period: usize) -> Result<Vec<T>> {
    // Validate inputs
    crate::traits::validate_indicator_input(data, period, "sma")?;

    // Use inverse multiply instead of division for speed
    let inv_period = T::from_f64(1.0 / period as f64)?;

    // Initialize result vector with NaN
    let mut result = vec![T::nan(); data.len()];

    // Compute initial sum using SIMD when available for f64
    let (mut sum, nan_count) = compute_initial_sum(data, period);

    // Fast path: if no NaN in initial window, check if any NaN in rest of data
    // If no NaN at all, use optimized loop without NaN checks
    if nan_count == 0 {
        result[period - 1] = sum * inv_period;

        // Check if rest of data has any NaN - if not, use fast path
        let has_nan = data[period..].iter().any(|&x| is_invalid(x));

        if !has_nan {
            // Fast path: no NaN checking needed
            for i in period..data.len() {
                let new_value = data[i];
                let old_value = data[i - period];
                sum = sum + new_value - old_value;
                result[i] = sum * inv_period;
            }
            return Ok(result);
        }
    }

    // Slow path: need to track NaN count
    let mut nan_count = nan_count;
    for i in period..data.len() {
        let new_value = data[i];
        let old_value = data[i - period];

        if is_invalid(new_value) {
            nan_count += 1;
        } else {
            sum = sum + new_value;
        }

        if is_invalid(old_value) {
            nan_count -= 1;
        } else {
            sum = sum - old_value;
        }

        if nan_count == 0 {
            result[i] = sum * inv_period;
        } else {
            result[i] = T::nan();
        }
    }

    Ok(result)
}

/// Computes the Simple Moving Average into a pre-allocated output buffer.
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
/// A `Result` containing the number of valid SMA values computed (`data.len()` - period + 1),
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
/// - The output buffer is shorter than the input data
///
/// # Performance
///
/// Uses SIMD acceleration for f64 when the `simd` feature is enabled (default).
///
/// # Example
///
/// ```
/// use fast_ta::indicators::sma::sma_into;
///
/// let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
/// let mut output = vec![0.0_f64; 5];
/// let valid_count = sma_into(&data, 3, &mut output).unwrap();
///
/// assert_eq!(valid_count, 3);
/// assert!(output[0].is_nan());
/// assert!((output[2] - 2.0).abs() < 1e-10);
/// ```
#[inline]
#[must_use = "this returns a Result with the count of valid SMA values"]
pub fn sma_into<T: SeriesElement + 'static>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    // Validate inputs
    crate::traits::validate_indicator_input(data, period, "sma")?;

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "sma",
        });
    }

    // Use inverse multiply instead of division for speed
    let inv_period = T::from_f64(1.0 / period as f64)?;

    // Initialize lookback period with NaN
    for item in output.iter_mut().take(period - 1) {
        *item = T::nan();
    }

    // Compute initial sum using SIMD when available for f64
    let (mut sum, nan_count) = compute_initial_sum(data, period);

    // Fast path: if no NaN in initial window, check if any NaN in rest of data
    if nan_count == 0 {
        output[period - 1] = sum * inv_period;

        // Check if rest of data has any NaN - if not, use fast path
        let has_nan = data[period..].iter().any(|&x| is_invalid(x));

        if !has_nan {
            // Fast path: no NaN checking needed
            for i in period..data.len() {
                let new_value = data[i];
                let old_value = data[i - period];
                sum = sum + new_value - old_value;
                output[i] = sum * inv_period;
            }
            return Ok(data.len() - period + 1);
        }
    } else {
        output[period - 1] = T::nan();
    }

    // Slow path: need to track NaN count
    let mut nan_count = nan_count;
    for i in period..data.len() {
        let new_value = data[i];
        let old_value = data[i - period];

        if is_invalid(new_value) {
            nan_count += 1;
        } else {
            sum = sum + new_value;
        }

        if is_invalid(old_value) {
            nan_count -= 1;
        } else {
            sum = sum - old_value;
        }

        if nan_count == 0 {
            output[i] = sum * inv_period;
        } else {
            output[i] = T::nan();
        }
    }

    // Return count of valid (non-NaN) values
    Ok(data.len() - period + 1)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;
    use num_traits::Float;

    // Helper function to compare floating point values
    fn approx_eq<T: Float>(a: T, b: T, epsilon: T) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        (a - b).abs() < epsilon
    }

    const EPSILON: f64 = 1e-10;
    const EPSILON_F32: f32 = 1e-5;

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_sma_basic() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&data, 3).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(approx_eq(result[2], 2.0, EPSILON)); // (1+2+3)/3
        assert!(approx_eq(result[3], 3.0, EPSILON)); // (2+3+4)/3
        assert!(approx_eq(result[4], 4.0, EPSILON)); // (3+4+5)/3
    }

    #[test]
    fn test_sma_f32() {
        let data = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&data, 3).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(approx_eq(result[2], 2.0_f32, EPSILON_F32));
        assert!(approx_eq(result[3], 3.0_f32, EPSILON_F32));
        assert!(approx_eq(result[4], 4.0_f32, EPSILON_F32));
    }

    #[test]
    fn test_sma_period_one() {
        // SMA(1) should equal the input values
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&data, 1).unwrap();

        assert_eq!(result.len(), 5);
        assert!(approx_eq(result[0], 1.0, EPSILON));
        assert!(approx_eq(result[1], 2.0, EPSILON));
        assert!(approx_eq(result[2], 3.0, EPSILON));
        assert!(approx_eq(result[3], 4.0, EPSILON));
        assert!(approx_eq(result[4], 5.0, EPSILON));
    }

    #[test]
    fn test_sma_period_equals_length() {
        // Period equals data length - only one valid output
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&data, 5).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
        assert!(approx_eq(result[4], 3.0, EPSILON)); // (1+2+3+4+5)/5 = 15/5 = 3
    }

    #[test]
    fn test_sma_single_element_period_one() {
        let data = vec![42.0_f64];
        let result = sma(&data, 1).unwrap();

        assert_eq!(result.len(), 1);
        assert!(approx_eq(result[0], 42.0, EPSILON));
    }

    // ==================== Reference Value Tests ====================

    #[test]
    fn test_sma_known_values() {
        // Test against known/expected SMA values
        let data = vec![
            22.27_f64, 22.19, 22.08, 22.17, 22.18, 22.13, 22.23, 22.43, 22.24, 22.29,
        ];
        let result = sma(&data, 5).unwrap();

        // Expected values calculated manually:
        // SMA[4] = (22.27 + 22.19 + 22.08 + 22.17 + 22.18) / 5 = 22.178
        // SMA[5] = (22.19 + 22.08 + 22.17 + 22.18 + 22.13) / 5 = 22.15
        // etc.

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
        assert!(approx_eq(result[4], 22.178, 1e-6));
        assert!(approx_eq(result[5], 22.15, 1e-6));
    }

    #[test]
    fn test_sma_constant_values() {
        // SMA of constant values should equal the constant
        let data = vec![5.0_f64; 10];
        let result = sma(&data, 3).unwrap();

        for i in 2..result.len() {
            assert!(approx_eq(result[i], 5.0, EPSILON));
        }
    }

    #[test]
    fn test_sma_linear_sequence() {
        // For a linear sequence 1,2,3,4,5,6,7,8,9,10 with period 3
        // SMA should be at the center of each window
        let data: Vec<f64> = (1..=10).map(|x| x as f64).collect();
        let result = sma(&data, 3).unwrap();

        // For odd-period SMA of a linear sequence, result equals middle value
        assert!(approx_eq(result[2], 2.0, EPSILON)); // Center of 1,2,3
        assert!(approx_eq(result[3], 3.0, EPSILON)); // Center of 2,3,4
        assert!(approx_eq(result[9], 9.0, EPSILON)); // Center of 8,9,10
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_sma_with_nan_in_data() {
        // NaN in the middle of the data should propagate
        let data = vec![1.0_f64, 2.0, f64::NAN, 4.0, 5.0, 6.0];
        let result = sma(&data, 3).unwrap();

        // Windows containing NaN should produce NaN output
        assert!(result[0].is_nan()); // lookback
        assert!(result[1].is_nan()); // lookback
        assert!(result[2].is_nan()); // window contains NaN
        assert!(result[3].is_nan()); // window contains NaN
        assert!(result[4].is_nan()); // window contains NaN
        assert!(approx_eq(result[5], 5.0, EPSILON)); // (4+5+6)/3 - NaN rolled out
    }

    #[test]
    fn test_sma_negative_values() {
        let data = vec![-5.0_f64, -3.0, -1.0, 1.0, 3.0, 5.0];
        let result = sma(&data, 3).unwrap();

        assert!(approx_eq(result[2], -3.0, EPSILON)); // (-5-3-1)/3
        assert!(approx_eq(result[3], -1.0, EPSILON)); // (-3-1+1)/3
        assert!(approx_eq(result[4], 1.0, EPSILON)); // (-1+1+3)/3
        assert!(approx_eq(result[5], 3.0, EPSILON)); // (1+3+5)/3
    }

    #[test]
    fn test_sma_large_values() {
        // Test with very large values to check for overflow issues
        let data = vec![1e15_f64, 2e15, 3e15, 4e15, 5e15];
        let result = sma(&data, 3).unwrap();

        assert!(approx_eq(result[2], 2e15, 1e5)); // Larger epsilon for large values
        assert!(approx_eq(result[3], 3e15, 1e5));
        assert!(approx_eq(result[4], 4e15, 1e5));
    }

    #[test]
    fn test_sma_small_values() {
        // Test with very small values
        let data = vec![1e-15_f64, 2e-15, 3e-15, 4e-15, 5e-15];
        let result = sma(&data, 3).unwrap();

        assert!(approx_eq(result[2], 2e-15, 1e-25));
        assert!(approx_eq(result[3], 3e-15, 1e-25));
        assert!(approx_eq(result[4], 4e-15, 1e-25));
    }

    #[test]
    fn test_sma_alternating_values() {
        // Test with alternating values
        let data = vec![1.0_f64, -1.0, 1.0, -1.0, 1.0, -1.0];
        let result = sma(&data, 2).unwrap();

        // (1 + -1) / 2 = 0 for all pairs
        assert!(result[0].is_nan());
        assert!(approx_eq(result[1], 0.0, EPSILON));
        assert!(approx_eq(result[2], 0.0, EPSILON));
        assert!(approx_eq(result[3], 0.0, EPSILON));
    }

    #[test]
    fn test_sma_infinity_handling() {
        // Test with infinity values
        let data = vec![1.0_f64, f64::INFINITY, 3.0, 4.0, 5.0];
        let result = sma(&data, 3).unwrap();

        assert!(result[2].is_nan()); // Window contains infinity
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_sma_empty_input() {
        let data: Vec<f64> = vec![];
        let result = sma(&data, 3);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_sma_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = sma(&data, 0);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_sma_period_exceeds_length() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = sma(&data, 5);

        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                required: 5,
                actual: 3,
                ..
            })
        ));
    }

    // ==================== sma_into Tests ====================

    #[test]
    fn test_sma_into_basic() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f64; 5];
        let valid_count = sma_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!(approx_eq(output[2], 2.0, EPSILON));
        assert!(approx_eq(output[3], 3.0, EPSILON));
        assert!(approx_eq(output[4], 4.0, EPSILON));
    }

    #[test]
    fn test_sma_into_buffer_reuse() {
        // Test that we can reuse the same buffer
        let data1 = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let data2 = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
        let mut output = vec![0.0_f64; 5];

        sma_into(&data1, 3, &mut output).unwrap();
        assert!(approx_eq(output[2], 2.0, EPSILON));

        sma_into(&data2, 3, &mut output).unwrap();
        assert!(approx_eq(output[2], 4.0, EPSILON)); // (5+4+3)/3
    }

    #[test]
    fn test_sma_into_insufficient_output() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f64; 3]; // Too short
        let result = sma_into(&data, 3, &mut output);

        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_sma_into_empty_input() {
        let data: Vec<f64> = vec![];
        let mut output = vec![0.0_f64; 5];
        let result = sma_into(&data, 3, &mut output);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_sma_into_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let mut output = vec![0.0_f64; 3];
        let result = sma_into(&data, 0, &mut output);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_sma_into_f32() {
        let data = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f32; 5];
        let valid_count = sma_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(approx_eq(output[2], 2.0_f32, EPSILON_F32));
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_sma_and_sma_into_produce_same_result() {
        let data = vec![10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        let result1 = sma(&data, 4).unwrap();

        let mut result2 = vec![0.0_f64; data.len()];
        sma_into(&data, 4, &mut result2).unwrap();

        for i in 0..data.len() {
            assert!(approx_eq(result1[i], result2[i], EPSILON));
        }
    }

    #[test]
    fn test_sma_valid_count() {
        let data = vec![1.0_f64; 100];
        let mut output = vec![0.0_f64; 100];

        let valid_count = sma_into(&data, 10, &mut output).unwrap();
        assert_eq!(valid_count, 91); // 100 - 10 + 1

        let valid_count = sma_into(&data, 1, &mut output).unwrap();
        assert_eq!(valid_count, 100); // All values valid

        let valid_count = sma_into(&data, 100, &mut output).unwrap();
        assert_eq!(valid_count, 1); // Only last value valid
    }

    // ==================== Property-Based-Like Tests ====================

    #[test]
    fn test_sma_output_length_equals_input_length() {
        for len in [5, 10, 50, 100] {
            for period in [1, 2, 5] {
                if period <= len {
                    let data: Vec<f64> = (0..len).map(|x| x as f64).collect();
                    let result = sma(&data, period).unwrap();
                    assert_eq!(result.len(), len);
                }
            }
        }
    }

    #[test]
    fn test_sma_nan_count() {
        // First (period - 1) values should be NaN
        for period in 1..=10 {
            let data: Vec<f64> = (0..20).map(|x| x as f64).collect();
            let result = sma(&data, period).unwrap();

            let nan_count = result.iter().filter(|x| x.is_nan()).count();
            assert_eq!(nan_count, period - 1);
        }
    }

    #[test]
    fn test_sma_rolling_property() {
        // Verify the rolling sum property: SMA[i+1] = SMA[i] + (new - old) / period
        let data: Vec<f64> = (0..10).map(|x| (x * 2) as f64).collect();
        let period = 3;
        let result = sma(&data, period).unwrap();

        for i in period..data.len() {
            let expected_diff = (data[i] - data[i - period]) / (period as f64);
            let actual_diff = result[i] - result[i - 1];
            assert!(approx_eq(expected_diff, actual_diff, EPSILON));
        }
    }

    #[test]
    fn test_sma_bounded_by_input_range() {
        // SMA should always be within the range of input values in the window
        let data = vec![10.0_f64, 20.0, 5.0, 25.0, 15.0, 30.0, 8.0, 22.0];
        let result = sma(&data, 3).unwrap();

        for i in 2..data.len() {
            if !result[i].is_nan() {
                let window_min = data[i - 2..=i]
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min);
                let window_max = data[i - 2..=i]
                    .iter()
                    .cloned()
                    .fold(f64::NEG_INFINITY, f64::max);
                assert!(result[i] >= window_min);
                assert!(result[i] <= window_max);
            }
        }
    }

    // ==================== SIMD Integration Tests ====================

    mod simd_tests {
        use super::*;

        #[test]
        fn test_sma_f64_with_simd_large_period() {
            // Test with large period that exercises SIMD code path
            let data: Vec<f64> = (0..1000).map(|x| x as f64).collect();
            let result = sma(&data, 100).unwrap();

            // First 99 should be NaN
            for i in 0..99 {
                assert!(result[i].is_nan());
            }

            // Verify first SMA value: sum(0..100) / 100 = 4950 / 100 = 49.5
            assert!(approx_eq(result[99], 49.5, EPSILON));
        }

        #[test]
        fn test_sma_f64_with_simd_nan_handling() {
            // Verify SIMD path handles NaN correctly
            let data = vec![1.0_f64, 2.0, f64::NAN, 4.0, 5.0, 6.0];
            let result = sma(&data, 3).unwrap();

            // Windows containing NaN should produce NaN output
            assert!(result[2].is_nan()); // window contains NaN
            assert!(result[3].is_nan()); // window contains NaN
            assert!(result[4].is_nan()); // window contains NaN
            assert!(approx_eq(result[5], 5.0, EPSILON)); // (4+5+6)/3
        }
    }
}
