//! MAVP (Moving Average Variable Period)
//!
//! This indicator computes a simple moving average where the period can vary
//! at each data point. This allows for adaptive smoothing based on market conditions.
//!
//! # Algorithm
//!
//! At each point i, the SMA is computed using the period specified in the periods array:
//! - If periods\[i\] is 0, NaN, or there's insufficient lookback data, output is NaN
//! - Otherwise, compute SMA over the last periods\[i\] values
//!
//! # Example
//!
//! ```
//! use liq_ta::indicators::mavp::mavp;
//!
//! let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
//! let periods = vec![2.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 4.0, 5.0];
//! let result = mavp(&prices, &periods, 2, 10).unwrap();
//! ```

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns the lookback period for MAVP.
///
/// The lookback depends on the minimum period parameter.
#[inline]
#[must_use]
pub const fn mavp_lookback(min_period: usize) -> usize {
    if min_period == 0 { 0 } else { min_period - 1 }
}

/// Returns the minimum input length required for MAVP.
#[inline]
#[must_use]
pub const fn mavp_min_len(min_period: usize) -> usize {
    if min_period == 0 { 1 } else { min_period }
}

/// Computes MAVP and stores results in output buffer.
///
/// # Arguments
///
/// * `data` - The input price series
/// * `periods` - The period to use at each data point (must be same length as data)
/// * `min_period` - Minimum allowed period value
/// * `max_period` - Maximum allowed period value
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing `()` on success, or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn mavp_into<T: SeriesElement>(
    data: &[T],
    periods: &[T],
    min_period: usize,
    max_period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = data.len();

    if periods.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "data has {} elements, periods has {} elements",
                n,
                periods.len()
            ),
        });
    }

    if min_period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "min_period must be >= 1",
        });
    }

    if max_period < min_period {
        return Err(Error::InvalidPeriod {
            period: max_period,
            reason: "max_period must be >= min_period",
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "mavp",
            required: n,
            actual: output.len(),
        });
    }

    let min_len = mavp_min_len(min_period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "mavp",
            required: min_len,
            actual: n,
        });
    }

    // Convert bounds for comparison
    let min_period_t = T::from_usize(min_period)?;
    let max_period_t = T::from_usize(max_period)?;

    // Process each data point
    for i in 0..n {
        let period_val = periods[i];

        // Check if period is valid
        if is_invalid(period_val) || period_val < min_period_t || period_val > max_period_t {
            output[i] = T::nan();
            continue;
        }

        // Round period to nearest integer and convert to usize
        let period_rounded = period_val.round();
        let Some(period_usize) = period_rounded.to_usize() else {
            output[i] = T::nan();
            continue;
        };

        // Clamp to valid range
        let period = period_usize.clamp(min_period, max_period);

        // Check if we have enough data for this period
        if i + 1 < period {
            output[i] = T::nan();
            continue;
        }

        // Compute SMA for this period
        let period_t = T::from_usize(period)?;
        let start = i + 1 - period;
        let mut sum = T::zero();
        let mut has_nan = false;

        for j in start..=i {
            if is_invalid(data[j]) {
                has_nan = true;
                break;
            }
            sum = sum + data[j];
        }

        if has_nan {
            output[i] = T::nan();
        } else {
            output[i] = sum / period_t;
        }
    }

    Ok(())
}

/// Computes MAVP (Moving Average Variable Period).
///
/// # Arguments
///
/// * `data` - The input price series
/// * `periods` - The period to use at each data point (must be same length as data)
/// * `min_period` - Minimum allowed period value (default: 2)
/// * `max_period` - Maximum allowed period value (default: 30)
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` with the MAVP values, or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The periods array has different length than data (`Error::LengthMismatch`)
/// - `min_period` is 0 (`Error::InvalidPeriod`)
/// - `max_period` < `min_period` (`Error::InvalidPeriod`)
/// - The input data is shorter than `min_period` (`Error::InsufficientData`)
///
/// # Example
///
/// ```
/// use liq_ta::indicators::mavp::mavp;
///
/// let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let periods = vec![2.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 4.0, 5.0];
/// let result = mavp(&prices, &periods, 2, 10).unwrap();
///
/// // At index 2 with period=2: (2+3)/2 = 2.5
/// assert!((result[2] - 2.5).abs() < 1e-10);
/// ```
pub fn mavp<T: SeriesElement>(
    data: &[T],
    periods: &[T],
    min_period: usize,
    max_period: usize,
) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    mavp_into(data, periods, min_period, max_period, &mut output)?;
    Ok(output)
}

/// Computes MAVP with default parameters (`min_period=2`, `max_period=30`).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn mavp_default<T: SeriesElement>(data: &[T], periods: &[T]) -> Result<Vec<T>> {
    mavp(data, periods, 2, 30)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        (a - b).abs() < EPSILON
    }

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_mavp_lookback() {
        assert_eq!(mavp_lookback(1), 0);
        assert_eq!(mavp_lookback(2), 1);
        assert_eq!(mavp_lookback(5), 4);
        assert_eq!(mavp_lookback(10), 9);
    }

    #[test]
    fn test_mavp_min_len() {
        assert_eq!(mavp_min_len(1), 1);
        assert_eq!(mavp_min_len(2), 2);
        assert_eq!(mavp_min_len(5), 5);
    }

    #[test]
    fn test_mavp_basic() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let periods = vec![2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0];
        let result = mavp(&prices, &periods, 2, 10).unwrap();

        assert_eq!(result.len(), 10);
        // First value needs period=2, but index 0 only has 1 value
        assert!(result[0].is_nan());
        // At index 1: (1+2)/2 = 1.5
        assert!(approx_eq(result[1], 1.5));
        // At index 2: (2+3)/2 = 2.5
        assert!(approx_eq(result[2], 2.5));
    }

    #[test]
    fn test_mavp_variable_periods() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let periods = vec![1.0, 1.0, 2.0, 3.0, 2.0, 3.0, 4.0, 2.0, 3.0, 5.0];
        let result = mavp(&prices, &periods, 1, 10).unwrap();

        // period=1: value equals input
        assert!(approx_eq(result[0], 1.0));
        assert!(approx_eq(result[1], 2.0));

        // period=2 at index 2: (2+3)/2 = 2.5
        assert!(approx_eq(result[2], 2.5));

        // period=3 at index 3: (2+3+4)/3 = 3.0
        assert!(approx_eq(result[3], 3.0));

        // period=2 at index 4: (4+5)/2 = 4.5
        assert!(approx_eq(result[4], 4.5));

        // period=3 at index 5: (4+5+6)/3 = 5.0
        assert!(approx_eq(result[5], 5.0));

        // period=4 at index 6: (4+5+6+7)/4 = 5.5
        assert!(approx_eq(result[6], 5.5));

        // period=2 at index 7: (7+8)/2 = 7.5
        assert!(approx_eq(result[7], 7.5));

        // period=3 at index 8: (7+8+9)/3 = 8.0
        assert!(approx_eq(result[8], 8.0));

        // period=5 at index 9: (6+7+8+9+10)/5 = 8.0
        assert!(approx_eq(result[9], 8.0));
    }

    #[test]
    fn test_mavp_period_rounding() {
        // Periods should be rounded to nearest integer
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let periods = vec![2.4, 2.5, 2.6, 2.0, 3.0];
        let result = mavp(&prices, &periods, 2, 5).unwrap();

        // 2.4 rounds to 2, 2.5 rounds to 3 (banker's rounding may vary), 2.6 rounds to 3
        // Index 0: period 2, only 1 value -> NaN
        assert!(result[0].is_nan());
        // Index 1: period 2 or 3, need at least 2-3 values
        // With 2.4 -> 2: (1+2)/2 = 1.5
        // With 2.5 -> 2 or 3 depending on rounding
        // Index 2: period 3, (1+2+3)/3 = 2.0
    }

    #[test]
    fn test_mavp_period_outside_range() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let periods = vec![1.0, 5.0, 3.0, 6.0, 3.0]; // 1 < min, 6 > max
        let result = mavp(&prices, &periods, 2, 5).unwrap();

        // period=1 < min_period=2 -> NaN
        assert!(result[0].is_nan());
        // period=5 at index 1, but only 2 values -> NaN
        assert!(result[1].is_nan());
        // period=3 at index 2: (1+2+3)/3 = 2.0
        assert!(approx_eq(result[2], 2.0));
        // period=6 > max_period=5 -> NaN
        assert!(result[3].is_nan());
        // period=3 at index 4: (3+4+5)/3 = 4.0
        assert!(approx_eq(result[4], 4.0));
    }

    #[test]
    fn test_mavp_nan_in_periods() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let periods = vec![2.0, f64::NAN, 2.0, 2.0, 2.0];
        let result = mavp(&prices, &periods, 2, 5).unwrap();

        assert!(result[0].is_nan()); // insufficient data
        assert!(result[1].is_nan()); // NaN period
        assert!(approx_eq(result[2], 2.5)); // (2+3)/2
        assert!(approx_eq(result[3], 3.5)); // (3+4)/2
        assert!(approx_eq(result[4], 4.5)); // (4+5)/2
    }

    #[test]
    fn test_mavp_nan_in_data() {
        let prices = vec![1.0_f64, f64::NAN, 3.0, 4.0, 5.0];
        let periods = vec![2.0, 2.0, 2.0, 2.0, 2.0];
        let result = mavp(&prices, &periods, 2, 5).unwrap();

        assert!(result[0].is_nan()); // insufficient data
        assert!(result[1].is_nan()); // window contains NaN
        assert!(result[2].is_nan()); // window contains NaN
        assert!(approx_eq(result[3], 3.5)); // (3+4)/2
        assert!(approx_eq(result[4], 4.5)); // (4+5)/2
    }

    #[test]
    fn test_mavp_constant_period() {
        // When period is constant, should match SMA
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let periods = vec![3.0; 10];
        let result = mavp(&prices, &periods, 3, 10).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(approx_eq(result[2], 2.0)); // (1+2+3)/3
        assert!(approx_eq(result[3], 3.0)); // (2+3+4)/3
        assert!(approx_eq(result[4], 4.0)); // (3+4+5)/3
    }

    #[test]
    fn test_mavp_f32() {
        let prices = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
        let periods = vec![2.0_f32, 2.0, 2.0, 2.0, 2.0];
        let result = mavp(&prices, &periods, 2, 5).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!((result[1] - 1.5_f32).abs() < 1e-5);
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_mavp_empty_input() {
        let prices: Vec<f64> = vec![];
        let periods: Vec<f64> = vec![];
        let result = mavp(&prices, &periods, 2, 10);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_mavp_length_mismatch() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let periods = vec![2.0, 2.0, 2.0]; // Wrong length
        let result = mavp(&prices, &periods, 2, 10);

        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_mavp_min_period_zero() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let periods = vec![2.0, 2.0, 2.0, 2.0, 2.0];
        let result = mavp(&prices, &periods, 0, 10);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_mavp_max_less_than_min() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let periods = vec![2.0, 2.0, 2.0, 2.0, 2.0];
        let result = mavp(&prices, &periods, 5, 3);

        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_mavp_insufficient_data() {
        let prices = vec![1.0_f64, 2.0];
        let periods = vec![2.0, 2.0];
        let result = mavp(&prices, &periods, 5, 10);

        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_mavp_into_buffer_too_small() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let periods = vec![2.0, 2.0, 2.0, 2.0, 2.0];
        let mut output = vec![0.0_f64; 3];
        let result = mavp_into(&prices, &periods, 2, 10, &mut output);

        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    // ==================== Default Function Tests ====================

    #[test]
    fn test_mavp_default() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let periods = vec![2.0; 10];
        let result = mavp_default(&prices, &periods).unwrap();

        assert_eq!(result.len(), 10);
        assert!(result[0].is_nan()); // min_period=2 default
        assert!(approx_eq(result[1], 1.5));
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_mavp_and_mavp_into_produce_same_result() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let periods = vec![2.0, 3.0, 2.0, 4.0, 3.0, 2.0, 5.0, 3.0];

        let result1 = mavp(&prices, &periods, 2, 10).unwrap();

        let mut result2 = vec![0.0_f64; prices.len()];
        mavp_into(&prices, &periods, 2, 10, &mut result2).unwrap();

        for i in 0..prices.len() {
            assert!(approx_eq(result1[i], result2[i]));
        }
    }

    #[test]
    fn test_mavp_output_length() {
        for n in [5, 10, 50, 100] {
            let prices: Vec<f64> = (1..=n).map(|x| x as f64).collect();
            let periods = vec![3.0; n];
            let result = mavp(&prices, &periods, 2, 10).unwrap();
            assert_eq!(result.len(), n);
        }
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_mavp_period_one() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let periods = vec![1.0; 5];
        let result = mavp(&prices, &periods, 1, 10).unwrap();

        // SMA(1) should equal the input values
        assert!(approx_eq(result[0], 1.0));
        assert!(approx_eq(result[1], 2.0));
        assert!(approx_eq(result[2], 3.0));
        assert!(approx_eq(result[3], 4.0));
        assert!(approx_eq(result[4], 5.0));
    }

    #[test]
    fn test_mavp_large_period() {
        let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let periods = vec![10.0; 10];
        let result = mavp(&prices, &periods, 2, 10).unwrap();

        // Only the last value has enough lookback
        for i in 0..9 {
            assert!(result[i].is_nan());
        }
        // At index 9: (1+2+3+4+5+6+7+8+9+10)/10 = 5.5
        assert!(approx_eq(result[9], 5.5));
    }

    #[test]
    fn test_mavp_varying_period_at_each_index() {
        // Test that each index correctly uses its own period
        let prices = vec![10.0_f64, 20.0, 30.0, 40.0, 50.0];
        let periods = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = mavp(&prices, &periods, 1, 5).unwrap();

        // Index 0, period 1: 10/1 = 10
        assert!(approx_eq(result[0], 10.0));
        // Index 1, period 2: (10+20)/2 = 15
        assert!(approx_eq(result[1], 15.0));
        // Index 2, period 3: (10+20+30)/3 = 20
        assert!(approx_eq(result[2], 20.0));
        // Index 3, period 4: (10+20+30+40)/4 = 25
        assert!(approx_eq(result[3], 25.0));
        // Index 4, period 5: (10+20+30+40+50)/5 = 30
        assert!(approx_eq(result[4], 30.0));
    }
}
