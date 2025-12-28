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

    // For period 1, TRIMA equals the input
    if period == 1 {
        output[..data.len()].copy_from_slice(data);
        return Ok(());
    }

    let n = data.len();
    let lookback = trima_lookback(period);

    // Fill lookback period with NaN
    output[..lookback].fill(T::nan());

    // Check for NaN in data - if any, use slow path
    let has_nan = data.iter().take(n).any(|&v| is_invalid(v));
    if has_nan {
        return trima_into_slow(data, period, output, lookback);
    }

    // Fast path: TA-Lib style incremental algorithm
    // Uses numerator, numeratorSub, numeratorAdd for O(1) per-element updates
    trima_into_fast(data, period, output, lookback)
}

/// Fast path for TRIMA using TA-Lib's incremental algorithm.
/// Computes in O(1) per element instead of SMA of SMA.
fn trima_into_fast<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
    lookback: usize,
) -> Result<()> {
    let n = data.len();

    if period % 2 == 1 {
        // Odd period logic
        // Triangular weights: 1+2+3+...+(n/2+1)+...+2+1 = (n/2+1)^2
        let i = period / 2;
        let factor = 1.0 / ((i + 1) * (i + 1)) as f64;

        let mut trailing_idx = 0usize;
        let mut middle_idx = trailing_idx + i;
        let mut today_idx = middle_idx + i;

        // Initialize numerator and numeratorSub
        let mut numerator = 0.0_f64;
        let mut numerator_sub = 0.0_f64;

        // Build initial numeratorSub and numerator from trailing to middle (descending)
        for j in (trailing_idx..=middle_idx).rev() {
            let temp_real = data[j].to_f64().unwrap_or(0.0);
            numerator_sub += temp_real;
            numerator += numerator_sub;
        }

        // Build numeratorAdd and add to numerator from (middle+1) to today
        let mut numerator_add = 0.0_f64;
        for j in (middle_idx + 1)..=today_idx {
            let temp_real = data[j].to_f64().unwrap_or(0.0);
            numerator_add += temp_real;
            numerator += numerator_add;
        }

        // Write first output
        let mut temp_real = data[trailing_idx].to_f64().unwrap_or(0.0);
        output[lookback] = T::from_f64(numerator * factor)?;
        trailing_idx += 1;
        middle_idx += 1;
        today_idx += 1;

        // Main loop - O(1) per iteration
        while today_idx < n {
            // Step 1: Remove old trailing values
            numerator -= numerator_sub;
            numerator_sub -= temp_real;
            temp_real = data[middle_idx].to_f64().unwrap_or(0.0);
            numerator_sub += temp_real;

            // Step 2: Add new leading values
            numerator += numerator_add;
            numerator_add -= temp_real;
            temp_real = data[today_idx].to_f64().unwrap_or(0.0);
            numerator_add += temp_real;

            // Step 3: Add newest value
            numerator += temp_real;

            // Step 4: Output and advance
            temp_real = data[trailing_idx].to_f64().unwrap_or(0.0);
            output[today_idx] = T::from_f64(numerator * factor)?;

            trailing_idx += 1;
            middle_idx += 1;
            today_idx += 1;
        }
    } else {
        // Even period logic
        // Triangular weights: 1+2+...+n/2+n/2+...+2+1 = n/2 * (n/2 + 1)
        let i = period / 2;
        let factor = 1.0 / (i * (i + 1)) as f64;

        let mut trailing_idx = 0usize;
        let mut middle_idx = trailing_idx + i - 1;
        let mut today_idx = middle_idx + i;

        // Initialize numerator and numeratorSub
        let mut numerator = 0.0_f64;
        let mut numerator_sub = 0.0_f64;

        // Build initial numeratorSub and numerator from trailing to middle (descending)
        for j in (trailing_idx..=middle_idx).rev() {
            let temp_real = data[j].to_f64().unwrap_or(0.0);
            numerator_sub += temp_real;
            numerator += numerator_sub;
        }

        // Build numeratorAdd and add to numerator from (middle+1) to today
        let mut numerator_add = 0.0_f64;
        for j in (middle_idx + 1)..=today_idx {
            let temp_real = data[j].to_f64().unwrap_or(0.0);
            numerator_add += temp_real;
            numerator += numerator_add;
        }

        // Write first output
        let mut temp_real = data[trailing_idx].to_f64().unwrap_or(0.0);
        output[lookback] = T::from_f64(numerator * factor)?;
        trailing_idx += 1;
        middle_idx += 1;
        today_idx += 1;

        // Main loop - O(1) per iteration (slightly different from odd)
        while today_idx < n {
            // Step 1: Remove old trailing values
            numerator -= numerator_sub;
            numerator_sub -= temp_real;
            temp_real = data[middle_idx].to_f64().unwrap_or(0.0);
            numerator_sub += temp_real;

            // Step 2: Even period differs - subtract middle from add first
            numerator_add -= temp_real;
            numerator += numerator_add;
            temp_real = data[today_idx].to_f64().unwrap_or(0.0);
            numerator_add += temp_real;

            // Step 3: Add newest value
            numerator += temp_real;

            // Step 4: Output and advance
            temp_real = data[trailing_idx].to_f64().unwrap_or(0.0);
            output[today_idx] = T::from_f64(numerator * factor)?;

            trailing_idx += 1;
            middle_idx += 1;
            today_idx += 1;
        }
    }

    Ok(())
}

/// Slow path for TRIMA with NaN propagation using SMA of SMA.
fn trima_into_slow<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
    _lookback: usize,
) -> Result<()> {
    // Calculate the periods for the two SMAs
    let (sma1_period, sma2_period) = if period % 2 == 1 {
        let p = period.div_ceil(2);
        (p, p)
    } else {
        (period / 2 + 1, period / 2)
    };

    let n = data.len();

    // Check if we have enough data for second SMA
    if n < sma1_period {
        return Ok(());
    }
    let sma1_valid_len = n - sma1_period + 1;
    if sma1_valid_len < sma2_period {
        return Ok(());
    }

    // Pre-compute reciprocals
    let inv_period1 = T::one() / T::from_usize(sma1_period)?;
    let inv_period2 = T::one() / T::from_usize(sma2_period)?;

    // Ring buffer for SMA1 values
    let mut sma1_ring = vec![T::zero(); sma2_period];
    let mut invalid1_ring = vec![0usize; sma2_period];
    let mut ring_idx = 0usize;

    // SMA1 rolling state
    let mut sum1 = T::zero();
    let mut invalid_count1 = 0usize;

    // SMA2 rolling state
    let mut sum2 = T::zero();
    let mut invalid_count2 = 0usize;
    let mut sma1_count = 0usize;

    // Initialize SMA1 sum for first window
    for i in 0..sma1_period {
        if is_invalid(data[i]) {
            invalid_count1 += 1;
        } else {
            sum1 = sum1 + data[i];
        }
    }

    // Process all data positions where SMA1 is valid
    for data_idx in (sma1_period - 1)..n {
        let sma1_val = if invalid_count1 == 0 {
            sum1 * inv_period1
        } else {
            T::nan()
        };

        let old_sma1 = sma1_ring[ring_idx];
        let old_invalid1 = invalid1_ring[ring_idx];

        sma1_ring[ring_idx] = sma1_val;
        invalid1_ring[ring_idx] = invalid_count1;

        if sma1_count >= sma2_period {
            if old_invalid1 > 0 {
                invalid_count2 -= 1;
            } else {
                sum2 = sum2 - old_sma1;
            }
        }

        if invalid_count1 > 0 {
            invalid_count2 += 1;
        } else {
            sum2 = sum2 + sma1_val;
        }

        sma1_count += 1;

        if sma1_count >= sma2_period {
            if invalid_count2 == 0 {
                output[data_idx] = sum2 * inv_period2;
            } else {
                output[data_idx] = T::nan();
            }
        }

        ring_idx = (ring_idx + 1) % sma2_period;

        if data_idx + 1 < n {
            let new_value = data[data_idx + 1];
            let old_value = data[data_idx + 1 - sma1_period];

            if is_invalid(new_value) {
                invalid_count1 += 1;
            } else {
                sum1 = sum1 + new_value;
            }

            if is_invalid(old_value) {
                invalid_count1 -= 1;
            } else {
                sum1 = sum1 - old_value;
            }
        }
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

    #[test]
    fn test_trima_infinity_propagation() {
        // Test that infinity in the input produces NaN when in the window
        let mut data: Vec<f64> = (0..120).map(|i| 100.0 + i as f64).collect();
        data[25] = f64::INFINITY;

        let result = trima(&data, 5).unwrap();

        // Infinity at index 25 should produce NaN at indices where it's in the window
        // With period 5, the effective window size spans multiple indices
        assert!(
            result[25].is_nan(),
            "result[25] should be NaN when infinity is in window, got {}",
            result[25]
        );

        // Values before infinity enters the window should be valid
        assert!(result[20].is_finite(), "result[20] should be finite");

        // Values after infinity leaves the window should recover
        assert!(result[35].is_finite(), "result[35] should recover to finite");
    }
}
