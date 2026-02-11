//! MIDPOINT indicator.
//!
//! MIDPOINT calculates the midpoint of the price range over a specified period.
//! It's computed as (highest + lowest) / 2.
//!
//! # Formula
//!
//! MIDPOINT = (Highest(data, period) + Lowest(data, period)) / 2
//!
//! # Lookback
//!
//! The lookback period is `period - 1`.
//!
//! # Complexity
//!
//! - Time: O(n) for n elements using monotonic deque algorithm
//! - Space: O(k) for the deques, where k is the period

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Computes the lookback period for MIDPOINT.
///
/// The lookback is `period - 1`, representing the number of data points
/// needed before the first valid MIDPOINT value can be calculated.
///
/// # Arguments
///
/// * `period` - The MIDPOINT period
///
/// # Returns
///
/// The lookback period (period - 1)
#[inline]
#[must_use]
pub const fn midpoint_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

/// Returns the minimum input length required for MIDPOINT calculation.
///
/// This is the lookback period plus 1.
///
/// # Arguments
///
/// * `period` - The MIDPOINT period
#[inline]
#[must_use]
pub const fn midpoint_min_len(period: usize) -> usize {
    if period == 0 { 1 } else { period }
}

/// Computes MIDPOINT and stores results in the provided output slice.
///
/// MIDPOINT is the average of the highest and lowest values over a period.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The MIDPOINT period (must be >= 1)
/// * `output` - Pre-allocated output slice (must have length >= `data.len()`)
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(Error)` if period is invalid or data insufficient
///
/// # NaN Handling
///
/// The first `period - 1` elements of the output will be NaN.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn midpoint_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    use std::any::TypeId;

    // Validate inputs
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            indicator: "midpoint",
            required: period,
            actual: data.len(),
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            indicator: "midpoint",
            required: data.len(),
            actual: output.len(),
        });
    }

    let n = data.len();

    // VHGW dispatch for f64: use VHGW for large n
    // Threshold: n >= 1000 (bench-derived, adjust based on benchmarks)
    const VHGW_THRESHOLD: usize = 1000;
    if TypeId::of::<T>() == TypeId::of::<f64>() && n >= VHGW_THRESHOLD {
        let data_f64: &[f64] = unsafe { std::mem::transmute(data) };
        let output_f64: &mut [f64] = unsafe { std::mem::transmute(output) };
        crate::kernels::rolling_midpoint_vhgw_f64(data_f64, period, output_f64)?;
        return Ok(());
    }

    let lookback = midpoint_lookback(period);

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // For period 1, MIDPOINT equals the input (highest = lowest = data[i])
    // Normalize non-finite values to NaN per indicator policy.
    if period == 1 {
        for i in 0..n {
            output[i] = if data[i].is_finite() {
                data[i]
            } else {
                T::nan()
            };
        }
        return Ok(());
    }

    // For f64/f32: use specialized path with * 0.5 instead of / 2
    // This allows compiler to use multiply instead of divide (faster)
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let data_f64: &[f64] = unsafe { std::mem::transmute(data) };
        let output_f64: &mut [f64] = unsafe { std::mem::transmute(output) };
        return midpoint_deque_f64(data_f64, period, lookback, output_f64);
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let data_f32: &[f32] = unsafe { std::mem::transmute(data) };
        let output_f32: &mut [f32] = unsafe { std::mem::transmute(output) };
        return midpoint_deque_f32(data_f32, period, lookback, output_f32);
    }

    // Generic fallback path with division
    let two = T::from_usize(2)?;
    let mut max_deque: crate::kernels::rolling_extrema::MonotonicDeque<T> =
        crate::kernels::rolling_extrema::MonotonicDeque::new(period);
    let mut min_deque: crate::kernels::rolling_extrema::MonotonicDeque<T> =
        crate::kernels::rolling_extrema::MonotonicDeque::new(period);

    for i in 0..n {
        max_deque.push_max(i, data);
        min_deque.push_min(i, data);

        if i >= lookback {
            let highest = max_deque.get_extremum(data);
            let lowest = min_deque.get_extremum(data);
            output[i] = (highest + lowest) / two;
        }
    }

    Ok(())
}

/// Specialized f64 deque path with multiplication optimization
#[inline]
fn midpoint_deque_f64(
    data: &[f64],
    period: usize,
    lookback: usize,
    output: &mut [f64],
) -> Result<()> {
    let n = data.len();
    let mut max_deque: crate::kernels::rolling_extrema::MonotonicDeque<f64> =
        crate::kernels::rolling_extrema::MonotonicDeque::new(period);
    let mut min_deque: crate::kernels::rolling_extrema::MonotonicDeque<f64> =
        crate::kernels::rolling_extrema::MonotonicDeque::new(period);

    // Warmup loop: only update deques, no output
    for i in 0..lookback {
        max_deque.push_max(i, data);
        min_deque.push_min(i, data);
    }

    // Steady-state loop: update deques and write output (no branch)
    for i in lookback..n {
        max_deque.push_max(i, data);
        min_deque.push_min(i, data);

        let highest = max_deque.get_extremum(data);
        let lowest = min_deque.get_extremum(data);
        output[i] = (highest + lowest) * 0.5;
    }

    Ok(())
}

/// Specialized f32 deque path with multiplication optimization
#[inline]
fn midpoint_deque_f32(
    data: &[f32],
    period: usize,
    lookback: usize,
    output: &mut [f32],
) -> Result<()> {
    let n = data.len();
    let mut max_deque: crate::kernels::rolling_extrema::MonotonicDeque<f32> =
        crate::kernels::rolling_extrema::MonotonicDeque::new(period);
    let mut min_deque: crate::kernels::rolling_extrema::MonotonicDeque<f32> =
        crate::kernels::rolling_extrema::MonotonicDeque::new(period);

    // Warmup loop: only update deques, no output
    for i in 0..lookback {
        max_deque.push_max(i, data);
        min_deque.push_min(i, data);
    }

    // Steady-state loop: update deques and write output (no branch)
    for i in lookback..n {
        max_deque.push_max(i, data);
        min_deque.push_min(i, data);

        let highest = max_deque.get_extremum(data);
        let lowest = min_deque.get_extremum(data);
        output[i] = (highest + lowest) * 0.5;
    }

    Ok(())
}

/// Computes MIDPOINT (midpoint of price range over a period).
///
/// MIDPOINT is the average of the highest and lowest values over a period.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The MIDPOINT period (must be >= 1)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of MIDPOINT values with same length as input
/// * `Err(Error)` if period is invalid or data insufficient
///
/// # NaN Handling
///
/// The first `period - 1` elements will be NaN.
///
/// # Example
///
/// ```
/// use liq_ta::indicators::midpoint;
///
/// let prices = vec![10.0_f64, 11.0, 12.0, 11.0, 10.0, 9.0, 10.0, 11.0, 12.0, 13.0];
/// let result = midpoint(&prices, 5).unwrap();
/// // First 4 values are NaN, then MIDPOINT values
/// assert!(result[4].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn midpoint<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    use std::any::TypeId;

    // Optimized allocation for f64/f32: avoid double-initialization
    // midpoint_into() writes all elements, so this is pure double-write tax
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let data_f64: &[f64] = unsafe { std::mem::transmute(data) };
        let mut output: Vec<f64> = Vec::with_capacity(data.len());
        unsafe {
            output.set_len(data.len());
        }
        midpoint_into(data_f64, period, &mut output)?;
        Ok(unsafe { std::mem::transmute(output) })
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let data_f32: &[f32] = unsafe { std::mem::transmute(data) };
        let mut output: Vec<f32> = Vec::with_capacity(data.len());
        unsafe {
            output.set_len(data.len());
        }
        midpoint_into(data_f32, period, &mut output)?;
        Ok(unsafe { std::mem::transmute(output) })
    } else {
        // Generic fallback: initialize to NaN
        let mut output = vec![T::nan(); data.len()];
        midpoint_into(data, period, &mut output)?;
        Ok(output)
    }
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

    #[test]
    fn test_midpoint_lookback() {
        assert_eq!(midpoint_lookback(1), 0);
        assert_eq!(midpoint_lookback(2), 1);
        assert_eq!(midpoint_lookback(5), 4);
        assert_eq!(midpoint_lookback(10), 9);
        assert_eq!(midpoint_lookback(0), 0);
    }

    #[test]
    fn test_midpoint_min_len() {
        assert_eq!(midpoint_min_len(1), 1);
        assert_eq!(midpoint_min_len(2), 2);
        assert_eq!(midpoint_min_len(5), 5);
        assert_eq!(midpoint_min_len(10), 10);
    }

    #[test]
    fn test_midpoint_empty_input() {
        let data: Vec<f64> = vec![];
        let result = midpoint(&data, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_midpoint_zero_period() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = midpoint(&data, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_midpoint_insufficient_data() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0];
        let result = midpoint(&data, 5);
        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                indicator: "midpoint",
                required: 5,
                actual: 3,
            })
        ));
    }

    #[test]
    fn test_midpoint_period_one() {
        let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = midpoint(&data, 1).unwrap();
        // MIDPOINT with period 1 equals input
        assert_eq!(result.len(), data.len());
        for i in 0..data.len() {
            assert!(approx_eq(result[i], data[i], EPSILON));
        }
    }

    #[test]
    fn test_midpoint_output_length_equals_input_length() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = midpoint(&data, 5).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_midpoint_nan_count() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;
        let result = midpoint(&data, period).unwrap();

        // Count NaN values - should be period - 1 = 4
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, period - 1);
    }

    #[test]
    fn test_midpoint_valid_count() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;
        let result = midpoint(&data, period).unwrap();

        // Valid values start at index period - 1
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        assert_eq!(valid_count, data.len() - (period - 1));
    }

    #[test]
    fn test_midpoint_basic() {
        // For window [1, 2, 3, 4, 5]: highest=5, lowest=1, midpoint=3
        // For window [2, 3, 4, 5, 6]: highest=6, lowest=2, midpoint=4
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let result = midpoint(&data, 5).unwrap();

        // First 4 values should be NaN
        for i in 0..4 {
            assert!(result[i].is_nan());
        }

        // Expected midpoints
        assert!(approx_eq(result[4], 3.0, EPSILON)); // (5+1)/2 = 3
        assert!(approx_eq(result[5], 4.0, EPSILON)); // (6+2)/2 = 4
        assert!(approx_eq(result[6], 5.0, EPSILON)); // (7+3)/2 = 5
    }

    #[test]
    fn test_midpoint_constant_values() {
        // MIDPOINT of constant values should equal that constant
        let data: Vec<f64> = vec![42.0; 10];
        let result = midpoint(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 42.0, EPSILON));
        }
    }

    #[test]
    fn test_midpoint_with_highs_and_lows() {
        // Test with distinct highs and lows
        let data: Vec<f64> = vec![10.0, 5.0, 15.0, 8.0, 12.0];
        let result = midpoint(&data, 5).unwrap();

        // Window [10, 5, 15, 8, 12]: highest=15, lowest=5, midpoint=10
        assert!(approx_eq(result[4], 10.0, EPSILON));
    }

    #[test]
    fn test_midpoint_period_two() {
        let data: Vec<f64> = vec![10.0, 20.0, 15.0, 25.0, 30.0];
        let result = midpoint(&data, 2).unwrap();

        // First 1 value should be NaN
        assert!(result[0].is_nan());

        // Window [10, 20]: (20+10)/2 = 15
        assert!(approx_eq(result[1], 15.0, EPSILON));
        // Window [20, 15]: (20+15)/2 = 17.5
        assert!(approx_eq(result[2], 17.5, EPSILON));
        // Window [15, 25]: (25+15)/2 = 20
        assert!(approx_eq(result[3], 20.0, EPSILON));
        // Window [25, 30]: (30+25)/2 = 27.5
        assert!(approx_eq(result[4], 27.5, EPSILON));
    }

    #[test]
    fn test_midpoint_f32() {
        let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = midpoint(&data, 5).unwrap();

        assert_eq!(result.len(), data.len());

        // First 4 should be NaN
        for i in 0..4 {
            assert!(result[i].is_nan());
        }

        // Rest should be valid
        for i in 4..10 {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_midpoint_into_f32() {
        let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut output = vec![0.0_f32; data.len()];
        midpoint_into(&data, 5, &mut output).unwrap();

        // First 4 should be NaN
        for i in 0..4 {
            assert!(output[i].is_nan());
        }

        // Rest should be valid
        for i in 4..10 {
            assert!(output[i].is_finite());
        }
    }

    #[test]
    fn test_midpoint_into_insufficient_output() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mut output: Vec<f64> = vec![0.0; 3]; // Too small
        let result = midpoint_into(&data, 3, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_midpoint_minimum_length() {
        // Test with exactly the minimum required data
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = midpoint(&data, 5).unwrap();

        assert_eq!(result.len(), 5);
        // First 4 are NaN
        for i in 0..4 {
            assert!(result[i].is_nan());
        }
        // Only last value is valid: (5+1)/2 = 3
        assert!(result[4].is_finite());
        assert!(approx_eq(result[4], 3.0, EPSILON));
    }

    #[test]
    fn test_midpoint_negative_values() {
        let data: Vec<f64> = vec![-10.0, -5.0, -15.0, -8.0, -12.0];
        let result = midpoint(&data, 5).unwrap();

        // Window [-10, -5, -15, -8, -12]: highest=-5, lowest=-15, midpoint=-10
        assert!(approx_eq(result[4], -10.0, EPSILON));
    }

    #[test]
    fn test_midpoint_large_values() {
        let data: Vec<f64> = vec![1e15, 2e15, 3e15, 4e15, 5e15, 6e15, 7e15, 8e15, 9e15, 1e16];
        let result = midpoint(&data, 5).unwrap();

        // Should handle large values without overflow
        for i in 4..10 {
            assert!(result[i].is_finite());
        }
    }
}
