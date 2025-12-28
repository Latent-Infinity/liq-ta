//! MIDPRICE indicator.
//!
//! MIDPRICE calculates the midpoint of the high-low price range over a specified period.
//! It uses separate high and low price series.
//!
//! # Formula
//!
//! MIDPRICE = (Highest(high, period) + Lowest(low, period)) / 2
//!
//! # Lookback
//!
//! The lookback period is `period - 1`.
//!
//! # Performance
//!
//! Uses O(n) monotonic deque algorithm instead of naive O(n×period) approach.

use crate::error::{Error, Result};
use crate::kernels::rolling_extrema::MonotonicDeque;
use crate::traits::SeriesElement;

/// Computes the lookback period for MIDPRICE.
///
/// The lookback is `period - 1`, representing the number of data points
/// needed before the first valid MIDPRICE value can be calculated.
///
/// # Arguments
///
/// * `period` - The MIDPRICE period
///
/// # Returns
///
/// The lookback period (period - 1)
#[inline]
#[must_use]
pub const fn midprice_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for MIDPRICE calculation.
///
/// This is the lookback period plus 1.
///
/// # Arguments
///
/// * `period` - The MIDPRICE period
#[inline]
#[must_use]
pub const fn midprice_min_len(period: usize) -> usize {
    if period == 0 {
        1
    } else {
        period
    }
}

/// Computes MIDPRICE and stores results in the provided output slice.
///
/// MIDPRICE is the average of the highest high and lowest low over a period.
///
/// # Arguments
///
/// * `high` - High price data
/// * `low` - Low price data
/// * `period` - The MIDPRICE period (must be >= 1)
/// * `output` - Pre-allocated output slice (must have length >= `high.len()`)
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(Error)` if period is invalid, data insufficient, or arrays mismatch
///
/// # NaN Handling
///
/// The first `period - 1` elements of the output will be NaN.
/// If any high/low value in the window is NaN, the output is NaN.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn midprice_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
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

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if high.len() < period {
        return Err(Error::InsufficientData {
            indicator: "midprice",
            required: period,
            actual: high.len(),
        });
    }

    if output.len() < high.len() {
        return Err(Error::BufferTooSmall {
            indicator: "midprice",
            required: high.len(),
            actual: output.len(),
        });
    }

    let lookback = midprice_lookback(period);
    let n = high.len();

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // For period 1, MIDPRICE = (high + low) / 2
    if period == 1 {
        let two = T::from_usize(2)?;
        for i in 0..n {
            let h = high[i];
            let l = low[i];
            if !h.is_finite() || !l.is_finite() {
                output[i] = T::nan();
            } else {
                output[i] = (h + l) / two;
            }
        }
        return Ok(());
    }

    let two = T::from_usize(2)?;

    // Use O(n) monotonic deques for rolling max (on high) and min (on low)
    let mut max_deque: MonotonicDeque<T> = MonotonicDeque::new(period);
    let mut min_deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    // Ring buffer for tracking invalid value indices (for NaN/Infinity propagation)
    let mut invalid_buf = vec![0usize; period];
    let mut invalid_head = 0usize;
    let mut invalid_tail = 0usize;
    let mut invalid_len = 0usize;
    let wrap = |idx: usize| idx % period;

    // Single pass through data
    for i in 0..n {
        let h = high[i];
        let l = low[i];

        // Track non-finite values for propagation policy
        if !h.is_finite() || !l.is_finite() {
            invalid_buf[invalid_tail] = i;
            invalid_tail = wrap(invalid_tail + 1);
            invalid_len += 1;
        }

        // Update deques with current elements
        max_deque.push_max(i, high);
        min_deque.push_min(i, low);

        // Remove expired invalid indices from the window
        if i >= period {
            let window_start = i + 1 - period;
            while invalid_len > 0 && invalid_buf[invalid_head] < window_start {
                invalid_head = wrap(invalid_head + 1);
                invalid_len -= 1;
            }
        }

        // Output valid values after lookback period
        if i >= lookback {
            // If any invalid value is still in the window, output NaN (propagation policy)
            if invalid_len > 0 {
                output[i] = T::nan();
            } else {
                let highest_high = max_deque.get_extremum(high);
                let lowest_low = min_deque.get_extremum(low);

                // If either deque is empty (all values were NaN), output NaN
                if !highest_high.is_finite() || !lowest_low.is_finite() {
                    output[i] = T::nan();
                } else {
                    output[i] = (highest_high + lowest_low) / two;
                }
            }
        }
    }

    Ok(())
}

/// Computes MIDPRICE (midpoint of high-low range over a period).
///
/// MIDPRICE is the average of the highest high and lowest low over a period.
///
/// # Arguments
///
/// * `high` - High price data
/// * `low` - Low price data
/// * `period` - The MIDPRICE period (must be >= 1)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of MIDPRICE values with same length as input
/// * `Err(Error)` if period is invalid, data insufficient, or arrays mismatch
///
/// # NaN Handling
///
/// The first `period - 1` elements will be NaN.
/// If any high/low value in the window is NaN, the output is NaN.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::midprice;
///
/// let high: Vec<f64> = vec![11.0, 12.0, 13.0, 12.0, 11.0, 10.0, 11.0, 12.0, 13.0, 14.0];
/// let low: Vec<f64> = vec![9.0, 10.0, 11.0, 10.0, 9.0, 8.0, 9.0, 10.0, 11.0, 12.0];
/// let result = midprice(&high, &low, 5).unwrap();
/// // First 4 values are NaN, then MIDPRICE values
/// assert!(result[4].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn midprice<T: SeriesElement>(high: &[T], low: &[T], period: usize) -> Result<Vec<T>> {
    let len = high.len();
    let mut output = vec![T::nan(); len];
    midprice_into(high, low, period, &mut output)?;
    Ok(output)
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
    fn test_midprice_lookback() {
        assert_eq!(midprice_lookback(1), 0);
        assert_eq!(midprice_lookback(2), 1);
        assert_eq!(midprice_lookback(5), 4);
        assert_eq!(midprice_lookback(10), 9);
        assert_eq!(midprice_lookback(0), 0);
    }

    #[test]
    fn test_midprice_min_len() {
        assert_eq!(midprice_min_len(1), 1);
        assert_eq!(midprice_min_len(2), 2);
        assert_eq!(midprice_min_len(5), 5);
        assert_eq!(midprice_min_len(10), 10);
    }

    #[test]
    fn test_midprice_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let result = midprice(&high, &low, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_midprice_length_mismatch() {
        let high: Vec<f64> = vec![1.0, 2.0, 3.0];
        let low: Vec<f64> = vec![0.5, 1.5];
        let result = midprice(&high, &low, 2);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_midprice_zero_period() {
        let high: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let low: Vec<f64> = vec![0.5, 1.5, 2.5, 3.5, 4.5];
        let result = midprice(&high, &low, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_midprice_insufficient_data() {
        let high: Vec<f64> = vec![1.0, 2.0, 3.0];
        let low: Vec<f64> = vec![0.5, 1.5, 2.5];
        let result = midprice(&high, &low, 5);
        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                indicator: "midprice",
                required: 5,
                actual: 3,
            })
        ));
    }

    #[test]
    fn test_midprice_period_one() {
        let high: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let low: Vec<f64> = vec![8.0, 18.0, 28.0, 38.0, 48.0];
        let result = midprice(&high, &low, 1).unwrap();

        // MIDPRICE with period 1 = (high + low) / 2
        assert_eq!(result.len(), high.len());
        assert!(approx_eq(result[0], 9.0, EPSILON)); // (10+8)/2
        assert!(approx_eq(result[1], 19.0, EPSILON)); // (20+18)/2
        assert!(approx_eq(result[2], 29.0, EPSILON)); // (30+28)/2
    }

    #[test]
    fn test_midprice_output_length_equals_input_length() {
        let high: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let low: Vec<f64> = vec![0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5];
        let result = midprice(&high, &low, 5).unwrap();
        assert_eq!(result.len(), high.len());
    }

    #[test]
    fn test_midprice_nan_count() {
        let high: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let low: Vec<f64> = vec![0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5];
        let period = 5;
        let result = midprice(&high, &low, period).unwrap();

        // Count NaN values - should be period - 1 = 4
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, period - 1);
    }

    #[test]
    fn test_midprice_valid_count() {
        let high: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let low: Vec<f64> = vec![0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5];
        let period = 5;
        let result = midprice(&high, &low, period).unwrap();

        // Valid values start at index period - 1
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        assert_eq!(valid_count, high.len() - (period - 1));
    }

    #[test]
    fn test_midprice_basic() {
        // High: [10, 12, 11, 13, 14] -> highest = 14
        // Low:  [8, 9, 8, 10, 11] -> lowest = 8
        // MIDPRICE = (14 + 8) / 2 = 11
        let high: Vec<f64> = vec![10.0, 12.0, 11.0, 13.0, 14.0, 15.0, 14.0];
        let low: Vec<f64> = vec![8.0, 9.0, 8.0, 10.0, 11.0, 12.0, 11.0];
        let result = midprice(&high, &low, 5).unwrap();

        // First 4 values should be NaN
        for i in 0..4 {
            assert!(result[i].is_nan());
        }

        // At index 4: highest_high=14, lowest_low=8, midprice=11
        assert!(approx_eq(result[4], 11.0, EPSILON));

        // At index 5: window [12,11,13,14,15], [9,8,10,11,12]
        // highest_high=15, lowest_low=8, midprice=11.5
        assert!(approx_eq(result[5], 11.5, EPSILON));

        // At index 6: window [11,13,14,15,14], [8,10,11,12,11]
        // highest_high=15, lowest_low=8, midprice=11.5
        assert!(approx_eq(result[6], 11.5, EPSILON));
    }

    #[test]
    fn test_midprice_constant_values() {
        // MIDPRICE of constant high/low should be (high+low)/2
        let high: Vec<f64> = vec![50.0; 10];
        let low: Vec<f64> = vec![40.0; 10];
        let result = midprice(&high, &low, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 45.0, EPSILON));
        }
    }

    #[test]
    fn test_midprice_period_two() {
        let high: Vec<f64> = vec![10.0, 20.0, 15.0, 25.0, 30.0];
        let low: Vec<f64> = vec![5.0, 15.0, 10.0, 20.0, 25.0];
        let result = midprice(&high, &low, 2).unwrap();

        // First 1 value should be NaN
        assert!(result[0].is_nan());

        // Window 0-1: highest_high=20, lowest_low=5, midprice=12.5
        assert!(approx_eq(result[1], 12.5, EPSILON));
        // Window 1-2: highest_high=20, lowest_low=10, midprice=15
        assert!(approx_eq(result[2], 15.0, EPSILON));
        // Window 2-3: highest_high=25, lowest_low=10, midprice=17.5
        assert!(approx_eq(result[3], 17.5, EPSILON));
        // Window 3-4: highest_high=30, lowest_low=20, midprice=25
        assert!(approx_eq(result[4], 25.0, EPSILON));
    }

    #[test]
    fn test_midprice_f32() {
        let high: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let low: Vec<f32> = vec![0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5];
        let result = midprice(&high, &low, 5).unwrap();

        assert_eq!(result.len(), high.len());

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
    fn test_midprice_into_f32() {
        let high: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let low: Vec<f32> = vec![0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5];
        let mut output = vec![0.0_f32; high.len()];
        midprice_into(&high, &low, 5, &mut output).unwrap();

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
    fn test_midprice_into_insufficient_output() {
        let high: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let low: Vec<f64> = vec![0.5, 1.5, 2.5, 3.5, 4.5];
        let mut output: Vec<f64> = vec![0.0; 3]; // Too small
        let result = midprice_into(&high, &low, 3, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_midprice_minimum_length() {
        // Test with exactly the minimum required data
        let high: Vec<f64> = vec![10.0, 12.0, 11.0, 13.0, 14.0];
        let low: Vec<f64> = vec![8.0, 9.0, 8.0, 10.0, 11.0];
        let result = midprice(&high, &low, 5).unwrap();

        assert_eq!(result.len(), 5);
        // First 4 are NaN
        for i in 0..4 {
            assert!(result[i].is_nan());
        }
        // Only last value is valid: highest_high=14, lowest_low=8, midprice=11
        assert!(result[4].is_finite());
        assert!(approx_eq(result[4], 11.0, EPSILON));
    }

    #[test]
    fn test_midprice_negative_values() {
        let high: Vec<f64> = vec![-5.0, -3.0, -4.0, -2.0, -1.0];
        let low: Vec<f64> = vec![-10.0, -8.0, -9.0, -7.0, -6.0];
        let result = midprice(&high, &low, 5).unwrap();

        // highest_high = -1, lowest_low = -10, midprice = -5.5
        assert!(approx_eq(result[4], -5.5, EPSILON));
    }

    #[test]
    fn test_midprice_large_values() {
        let high: Vec<f64> = vec![1e15, 2e15, 3e15, 4e15, 5e15, 6e15, 7e15, 8e15, 9e15, 1e16];
        let low: Vec<f64> = vec![
            0.5e15, 1.5e15, 2.5e15, 3.5e15, 4.5e15, 5.5e15, 6.5e15, 7.5e15, 8.5e15, 9.5e15,
        ];
        let result = midprice(&high, &low, 5).unwrap();

        // Should handle large values without overflow
        for i in 4..10 {
            assert!(result[i].is_finite());
        }
    }
}
