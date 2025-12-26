//! Triangular Moving Average (TRIMA) indicator.
//!
//! TRIMA is a double-smoothed moving average that applies heavier weighting to
//! the middle of the price series. It's computed as an SMA of an SMA.
//!
//! # Formula
//!
//! For odd period n:
//! - SMA1 period = (n+1)/2
//! - SMA2 period = (n+1)/2
//!
//! For even period n:
//! - SMA1 period = n/2 + 1
//! - SMA2 period = n/2
//!
//! TRIMA = SMA(SMA(data, `SMA1_period`), `SMA2_period`)
//!
//! # Lookback
//!
//! The lookback period is `period - 1`.

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Computes the lookback period for TRIMA.
///
/// The lookback is `period - 1`, representing the number of data points
/// needed before the first valid TRIMA value can be calculated.
///
/// # Arguments
///
/// * `period` - The TRIMA period
///
/// # Returns
///
/// The lookback period (period - 1)
#[inline]
#[must_use]
pub const fn trima_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for TRIMA calculation.
///
/// This is the lookback period plus 1.
///
/// # Arguments
///
/// * `period` - The TRIMA period
#[inline]
#[must_use]
pub const fn trima_min_len(period: usize) -> usize {
    if period == 0 {
        1
    } else {
        period
    }
}

/// Computes Triangular Moving Average (TRIMA) and stores results in the provided output slice.
///
/// TRIMA is a double-smoothed moving average that gives more weight to the middle
/// of the data range, resulting in a smoother line than SMA.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The TRIMA period (must be >= 1)
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
pub fn trima_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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
            indicator: "trima",
            required: period,
            actual: data.len(),
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            indicator: "trima",
            required: data.len(),
            actual: output.len(),
        });
    }

    let lookback = trima_lookback(period);

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // For period 1, TRIMA equals the input
    if period == 1 {
        for i in 0..data.len() {
            output[i] = data[i];
        }
        return Ok(());
    }

    // Calculate the periods for the two SMAs
    // For odd: both = (period+1)/2
    // For even: first = period/2 + 1, second = period/2
    let (sma1_period, sma2_period) = if period % 2 == 1 {
        let p = period.div_ceil(2);
        (p, p)
    } else {
        (period / 2 + 1, period / 2)
    };

    // Compute first SMA
    let sma1_len = data.len() - sma1_period + 1;
    let mut sma1 = vec![T::nan(); sma1_len];

    // First SMA value
    let mut sum = T::zero();
    let mut invalid_count = 0usize;
    for i in 0..sma1_period {
        if is_invalid(data[i]) {
            invalid_count += 1;
        } else {
            sum = sum + data[i];
        }
    }
    let period1_t = T::from_usize(sma1_period)?;
    sma1[0] = if invalid_count > 0 {
        T::nan()
    } else {
        sum / period1_t
    };

    // Subsequent SMA1 values using rolling sum
    for i in 1..sma1_len {
        let old_value = data[i - 1];
        let new_value = data[i + sma1_period - 1];
        if is_invalid(old_value) {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            sum = sum - old_value;
        }
        if is_invalid(new_value) {
            invalid_count += 1;
        } else {
            sum = sum + new_value;
        }
        sma1[i] = if invalid_count > 0 {
            T::nan()
        } else {
            sum / period1_t
        };
    }

    // Compute second SMA of SMA1
    if sma1.len() < sma2_period {
        // Not enough SMA1 values for second smoothing
        for i in lookback..data.len() {
            output[i] = T::nan();
        }
        return Ok(());
    }

    let sma2_len = sma1.len() - sma2_period + 1;

    // First SMA2 value
    let mut sum2 = T::zero();
    let mut invalid_count2 = 0usize;
    for i in 0..sma2_period {
        if is_invalid(sma1[i]) {
            invalid_count2 += 1;
        } else {
            sum2 = sum2 + sma1[i];
        }
    }
    let period2_t = T::from_usize(sma2_period)?;

    // The first valid TRIMA is at index (sma1_period - 1) + (sma2_period - 1) = period - 1
    output[lookback] = if invalid_count2 > 0 {
        T::nan()
    } else {
        sum2 / period2_t
    };

    // Subsequent TRIMA values
    for i in 1..sma2_len {
        let old_value = sma1[i - 1];
        let new_value = sma1[i + sma2_period - 1];
        if is_invalid(old_value) {
            invalid_count2 = invalid_count2.saturating_sub(1);
        } else {
            sum2 = sum2 - old_value;
        }
        if is_invalid(new_value) {
            invalid_count2 += 1;
        } else {
            sum2 = sum2 + new_value;
        }
        output[lookback + i] = if invalid_count2 > 0 {
            T::nan()
        } else {
            sum2 / period2_t
        };
    }

    Ok(())
}

/// Computes Triangular Moving Average (TRIMA).
///
/// TRIMA is a double-smoothed moving average that gives more weight to the middle
/// of the data range, resulting in a smoother line than SMA.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The TRIMA period (must be >= 1)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of TRIMA values with same length as input
/// * `Err(Error)` if period is invalid or data insufficient
///
/// # NaN Handling
///
/// The first `period - 1` elements will be NaN.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::trima;
///
/// let prices = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
/// let result = trima(&prices, 5).unwrap();
/// // First 4 values are NaN, then TRIMA values
/// assert!(result[4].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn trima<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    trima_into(data, period, &mut output)?;
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
    fn test_trima_lookback() {
        assert_eq!(trima_lookback(1), 0);
        assert_eq!(trima_lookback(2), 1);
        assert_eq!(trima_lookback(5), 4);
        assert_eq!(trima_lookback(10), 9);
        assert_eq!(trima_lookback(0), 0);
    }

    #[test]
    fn test_trima_min_len() {
        assert_eq!(trima_min_len(1), 1);
        assert_eq!(trima_min_len(2), 2);
        assert_eq!(trima_min_len(5), 5);
        assert_eq!(trima_min_len(10), 10);
    }

    #[test]
    fn test_trima_empty_input() {
        let data: Vec<f64> = vec![];
        let result = trima(&data, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_trima_zero_period() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = trima(&data, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_trima_insufficient_data() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0];
        let result = trima(&data, 5);
        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                indicator: "trima",
                required: 5,
                actual: 3,
            })
        ));
    }

    #[test]
    fn test_trima_period_one() {
        let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = trima(&data, 1).unwrap();
        // TRIMA with period 1 equals input
        assert_eq!(result.len(), data.len());
        for i in 0..data.len() {
            assert!(approx_eq(result[i], data[i], EPSILON));
        }
    }

    #[test]
    fn test_trima_output_length_equals_input_length() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = trima(&data, 5).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_trima_nan_count() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;
        let result = trima(&data, period).unwrap();

        // Count NaN values - should be period - 1 = 4
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, period - 1);
    }

    #[test]
    fn test_trima_valid_count() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;
        let result = trima(&data, period).unwrap();

        // Valid values start at index period - 1
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        assert_eq!(valid_count, data.len() - (period - 1));
    }

    #[test]
    fn test_trima_basic_odd_period() {
        // Period 5 (odd): SMA1_period = 3, SMA2_period = 3
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let result = trima(&data, 5).unwrap();

        // First 4 values should be NaN
        for i in 0..4 {
            assert!(result[i].is_nan());
        }

        // For odd period 5:
        // SMA1 of data with period 3: [2, 3, 4, 5, 6]
        // SMA2 of SMA1 with period 3: [3, 4, 5]
        // These go at indices 4, 5, 6
        assert!(result[4].is_finite());
        assert!(result[5].is_finite());
        assert!(result[6].is_finite());

        // Expected: SMA of [2,3,4] = 3, SMA of [3,4,5] = 4, SMA of [4,5,6] = 5
        assert!(approx_eq(result[4], 3.0, EPSILON));
        assert!(approx_eq(result[5], 4.0, EPSILON));
        assert!(approx_eq(result[6], 5.0, EPSILON));
    }

    #[test]
    fn test_trima_basic_even_period() {
        // Period 4 (even): SMA1_period = 3, SMA2_period = 2
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let result = trima(&data, 4).unwrap();

        // First 3 values should be NaN
        for i in 0..3 {
            assert!(result[i].is_nan());
        }

        // For even period 4:
        // SMA1 of data with period 3: [2, 3, 4, 5, 6]
        // SMA2 of SMA1 with period 2: [2.5, 3.5, 4.5, 5.5]
        // These go at indices 3, 4, 5, 6
        assert!(result[3].is_finite());
        assert!(result[4].is_finite());

        // Expected: SMA of [2,3] = 2.5, SMA of [3,4] = 3.5, etc.
        assert!(approx_eq(result[3], 2.5, EPSILON));
        assert!(approx_eq(result[4], 3.5, EPSILON));
    }

    #[test]
    fn test_trima_smoother_than_sma() {
        // TRIMA should be smoother than SMA due to double smoothing
        use crate::indicators::sma;

        let data: Vec<f64> = vec![10.0, 12.0, 11.0, 13.0, 12.0, 14.0, 13.0, 15.0, 14.0, 16.0];
        let period = 5;

        let trima_result = trima(&data, period).unwrap();
        let sma_result = sma(&data, period).unwrap();

        // Both should have same number of valid values
        let trima_valid: Vec<f64> = trima_result
            .iter()
            .filter(|x| !x.is_nan())
            .cloned()
            .collect();
        let sma_valid: Vec<f64> = sma_result.iter().filter(|x| !x.is_nan()).cloned().collect();

        // Calculate variance of changes
        let trima_changes: Vec<f64> = trima_valid
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .collect();
        let sma_changes: Vec<f64> = sma_valid.windows(2).map(|w| (w[1] - w[0]).abs()).collect();

        let trima_avg_change: f64 = trima_changes.iter().sum::<f64>() / trima_changes.len() as f64;
        let sma_avg_change: f64 = sma_changes.iter().sum::<f64>() / sma_changes.len() as f64;

        // TRIMA should have smaller average changes (smoother)
        assert!(
            trima_avg_change <= sma_avg_change,
            "TRIMA avg change {} should be <= SMA avg change {}",
            trima_avg_change,
            sma_avg_change
        );
    }

    #[test]
    fn test_trima_period_two() {
        // Period 2 (even): SMA1_period = 2, SMA2_period = 1
        let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = trima(&data, 2).unwrap();

        // First 1 value should be NaN
        assert!(result[0].is_nan());

        // For period 2: SMA1 = [15, 25, 35, 45], SMA2 with period 1 = same
        assert!(approx_eq(result[1], 15.0, EPSILON));
        assert!(approx_eq(result[2], 25.0, EPSILON));
        assert!(approx_eq(result[3], 35.0, EPSILON));
        assert!(approx_eq(result[4], 45.0, EPSILON));
    }

    #[test]
    fn test_trima_period_three() {
        // Period 3 (odd): SMA1_period = 2, SMA2_period = 2
        let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = trima(&data, 3).unwrap();

        // First 2 values should be NaN
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());

        // For period 3: SMA1 of period 2 = [15, 25, 35, 45]
        // SMA2 of period 2 = [20, 30, 40]
        assert!(approx_eq(result[2], 20.0, EPSILON));
        assert!(approx_eq(result[3], 30.0, EPSILON));
        assert!(approx_eq(result[4], 40.0, EPSILON));
    }

    #[test]
    fn test_trima_f32() {
        let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = trima(&data, 5).unwrap();

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
    fn test_trima_into_f32() {
        let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut output = vec![0.0_f32; data.len()];
        trima_into(&data, 5, &mut output).unwrap();

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
    fn test_trima_into_insufficient_output() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mut output: Vec<f64> = vec![0.0; 3]; // Too small
        let result = trima_into(&data, 3, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_trima_minimum_length() {
        // Test with exactly the minimum required data
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = trima(&data, 5).unwrap();

        assert_eq!(result.len(), 5);
        // First 4 are NaN
        for i in 0..4 {
            assert!(result[i].is_nan());
        }
        // Only last value is valid
        assert!(result[4].is_finite());
    }

    #[test]
    fn test_trima_negative_values() {
        let data: Vec<f64> = vec![-5.0, -4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0];
        let result = trima(&data, 5).unwrap();

        // Should handle negative values correctly
        for i in 4..10 {
            assert!(result[i].is_finite());
        }

        // TRIMA of linear data should follow the trend
        assert!(result[5] > result[4]);
        assert!(result[6] > result[5]);
    }

    #[test]
    fn test_trima_constant_values() {
        // TRIMA of constant values should equal that constant
        let data: Vec<f64> = vec![42.0; 10];
        let result = trima(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 42.0, EPSILON));
        }
    }

    #[test]
    fn test_trima_large_values() {
        let data: Vec<f64> = vec![1e15, 2e15, 3e15, 4e15, 5e15, 6e15, 7e15, 8e15, 9e15, 1e16];
        let result = trima(&data, 5).unwrap();

        // Should handle large values without overflow
        for i in 4..10 {
            assert!(result[i].is_finite());
        }
    }
}
