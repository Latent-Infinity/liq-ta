//! Double Exponential Moving Average (DEMA) indicator.
//!
//! DEMA reduces the lag inherent in traditional moving averages by applying
//! a combination of single and double-smoothed EMAs.
//!
//! # Algorithm
//!
//! DEMA uses a two-step smoothing process:
//! 1. Calculate EMA of the input data
//! 2. Calculate EMA of the first EMA
//! 3. Apply the DEMA formula: 2 × EMA1 - EMA2
//!
//! # Formula
//!
//! ```text
//! DEMA = 2 × EMA(data, period) - EMA(EMA(data, period), period)
//! ```
//!
//! This combination reduces lag while maintaining smoothness.
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::dema::dema;
//!
//! let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
//! let result = dema(&data, 3).unwrap();
//!
//! // DEMA has lookback of 2*(period-1)
//! assert!(result[0].is_nan());
//! assert!(result[1].is_nan());
//! assert!(result[2].is_nan());
//! assert!(result[3].is_nan());
//! assert!(!result[4].is_nan()); // First valid value
//! ```

use crate::error::{Error, Result};
use crate::indicators::ema::{ema, ema_into};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns the lookback period for DEMA.
///
/// DEMA requires two sequential EMA calculations, so the lookback is
/// `2 * (period - 1)`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::dema::dema_lookback;
///
/// assert_eq!(dema_lookback(5), 8);  // 2 * (5-1) = 8
/// assert_eq!(dema_lookback(14), 26); // 2 * (14-1) = 26
/// ```
#[inline]
#[must_use]
pub const fn dema_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        2 * (period - 1)
    }
}

/// Returns the minimum input length required for DEMA.
///
/// This is the smallest input size that will produce at least one valid output.
/// For DEMA, this is `2 * period - 1`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::dema::dema_min_len;
///
/// assert_eq!(dema_min_len(5), 9);  // 2*5 - 1 = 9
/// assert_eq!(dema_min_len(14), 27); // 2*14 - 1 = 27
/// ```
#[inline]
#[must_use]
pub const fn dema_min_len(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        2 * period - 1
    }
}

/// Computes the Double Exponential Moving Average (DEMA) of a data series.
///
/// Returns a vector of the same length as the input, where the first
/// `2 * (period - 1)` values are NaN (insufficient lookback data) and
/// subsequent values contain the DEMA.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods for EMA calculations
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` with the DEMA values, or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than `2 * period - 1` (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for intermediate EMA and output vectors
///
/// # Example
///
/// ```
/// use fast_ta::indicators::dema::dema;
///
/// let data = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
/// let result = dema(&data, 3).unwrap();
///
/// // First 4 values are NaN (lookback = 2*(3-1) = 4)
/// assert!(result[0].is_nan());
/// assert!(result[3].is_nan());
/// assert!(!result[4].is_nan());
/// ```
#[inline]
#[must_use = "this returns a Result with the DEMA values, which should be used"]
pub fn dema<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    // Validate period
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "dema period must be at least 1",
        });
    }

    // Validate data length
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    let min_len = dema_min_len(period);
    if data.len() < min_len {
        return Err(Error::InsufficientData {
            required: min_len,
            actual: data.len(),
            indicator: "dema",
        });
    }

    // Calculate EMA of input
    let ema1 = ema(data, period)?;

    // Calculate EMA of EMA (second smoothing)
    // We need to manually compute EMA2 starting from the first valid EMA1 value
    // to avoid NaN propagation issues
    let ema1_lookback = period - 1;
    let two = T::from_usize(2)?;
    let lookback = dema_lookback(period);

    let mut result = vec![T::nan(); data.len()];

    // Compute EMA2 manually, starting from valid EMA1 values
    let alpha = T::from_usize(2)? / T::from_usize(period + 1)?;
    let one_minus_alpha = T::one() - alpha;

    // Seed EMA2 with the first valid EMA1 value
    let mut ema2 = ema1[ema1_lookback];

    // First valid DEMA is at 2*(period-1)
    for i in ema1_lookback..data.len() {
        if !is_invalid(ema1[i]) {
            if i == ema1_lookback {
                // Seed value - EMA2 equals EMA1 at this point
                ema2 = ema1[i];
            } else {
                // Standard EMA update
                ema2 = alpha * ema1[i] + one_minus_alpha * ema2;
            }

            // DEMA is valid starting at lookback
            if i >= lookback {
                result[i] = two * ema1[i] - ema2;
            }
        }
    }

    Ok(result)
}

/// Computes the Double Exponential Moving Average into a pre-allocated output buffer.
///
/// This variant allows reusing an existing buffer to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods for EMA calculations
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid DEMA values computed,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than `2 * period - 1` (`Error::InsufficientData`)
/// - The output buffer is shorter than the input data (`Error::BufferTooSmall`)
///
/// # Example
///
/// ```
/// use fast_ta::indicators::dema::dema_into;
///
/// let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let mut output = vec![0.0_f64; 10];
/// let valid_count = dema_into(&data, 3, &mut output).unwrap();
///
/// assert_eq!(valid_count, 6); // 10 - 4 = 6 valid values
/// ```
#[inline]
#[must_use = "this returns a Result with the count of valid DEMA values"]
pub fn dema_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize> {
    // Validate period
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "dema period must be at least 1",
        });
    }

    // Validate data length
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    let min_len = dema_min_len(period);
    if data.len() < min_len {
        return Err(Error::InsufficientData {
            required: min_len,
            actual: data.len(),
            indicator: "dema",
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "dema",
        });
    }

    // Calculate EMA of input
    let mut ema1 = vec![T::nan(); data.len()];
    ema_into(data, period, &mut ema1)?;

    // Calculate EMA2 manually to avoid NaN propagation
    let ema1_lookback = period - 1;
    let two = T::from_usize(2)?;
    let lookback = dema_lookback(period);

    // Fill lookback with NaN
    for item in output.iter_mut().take(lookback) {
        *item = T::nan();
    }

    // Compute EMA2 manually
    let alpha = T::from_usize(2)? / T::from_usize(period + 1)?;
    let one_minus_alpha = T::one() - alpha;
    let mut ema2 = ema1[ema1_lookback];

    for i in ema1_lookback..data.len() {
        if !is_invalid(ema1[i]) {
            if i == ema1_lookback {
                ema2 = ema1[i];
            } else {
                ema2 = alpha * ema1[i] + one_minus_alpha * ema2;
            }

            if i >= lookback {
                output[i] = two * ema1[i] - ema2;
            }
        } else if i >= lookback {
            output[i] = T::nan();
        }
    }

    Ok(data.len() - lookback)
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

    // ==================== Lookback and Min Len Tests ====================

    #[test]
    fn test_dema_lookback() {
        assert_eq!(dema_lookback(1), 0); // 2*(1-1) = 0
        assert_eq!(dema_lookback(3), 4); // 2*(3-1) = 4
        assert_eq!(dema_lookback(5), 8); // 2*(5-1) = 8
        assert_eq!(dema_lookback(14), 26); // 2*(14-1) = 26
    }

    #[test]
    fn test_dema_min_len() {
        assert_eq!(dema_min_len(1), 1); // 2*1 - 1 = 1
        assert_eq!(dema_min_len(3), 5); // 2*3 - 1 = 5
        assert_eq!(dema_min_len(5), 9); // 2*5 - 1 = 9
        assert_eq!(dema_min_len(14), 27); // 2*14 - 1 = 27
    }

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_dema_basic() {
        // Create a sequence where we can verify DEMA behavior
        let data: Vec<f64> = (1..=10).map(|x| x as f64).collect();
        let result = dema(&data, 3).unwrap();

        assert_eq!(result.len(), 10);

        // First 4 values should be NaN (lookback = 2*(3-1) = 4)
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());

        // Values from index 4 onwards should be valid
        assert!(!result[4].is_nan());
        assert!(!result[5].is_nan());
        assert!(!result[9].is_nan());
    }

    #[test]
    fn test_dema_f32() {
        let data: Vec<f32> = (1..=10).map(|x| x as f32).collect();
        let result = dema(&data, 3).unwrap();

        assert_eq!(result.len(), 10);
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
    }

    #[test]
    fn test_dema_period_one() {
        // DEMA(1) should equal the input values (no lag reduction possible)
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = dema(&data, 1).unwrap();

        assert_eq!(result.len(), 5);
        // With period 1, EMA = data, so DEMA = 2*data - data = data
        for i in 0..5 {
            assert!(approx_eq(result[i], data[i], EPSILON));
        }
    }

    #[test]
    fn test_dema_constant_values() {
        // DEMA of constant values should equal the constant
        let data = vec![5.0_f64; 20];
        let result = dema(&data, 5).unwrap();

        // After lookback, all values should be 5.0
        for i in dema_lookback(5)..result.len() {
            assert!(approx_eq(result[i], 5.0, EPSILON));
        }
    }

    #[test]
    fn test_dema_reduces_lag() {
        // For a trending sequence, DEMA should be closer to current price than EMA
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let dema_result = dema(&data, 5).unwrap();
        let ema_result = ema(&data, 5).unwrap();

        // At the end of an uptrend, DEMA should be higher than EMA
        // (less lag means closer to current values)
        let last_idx = data.len() - 1;
        assert!(dema_result[last_idx] > ema_result[last_idx]);
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_dema_minimum_length() {
        // Test with exactly minimum required length
        let data: Vec<f64> = (1..=5).map(|x| x as f64).collect();
        let result = dema(&data, 3).unwrap();

        assert_eq!(result.len(), 5);
        // Only last value should be valid (lookback = 4)
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
    }

    #[test]
    fn test_dema_negative_values() {
        let data = vec![-5.0_f64, -4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0];
        let result = dema(&data, 3).unwrap();

        // Should handle negative values correctly
        assert!(!result[4].is_nan());
        assert!(!result[9].is_nan());
    }

    #[test]
    fn test_dema_large_values() {
        let data: Vec<f64> = (1..=10).map(|x| x as f64 * 1e15).collect();
        let result = dema(&data, 3).unwrap();

        assert!(!result[4].is_nan());
        assert!(result[4] > 0.0);
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_dema_empty_input() {
        let data: Vec<f64> = vec![];
        let result = dema(&data, 3);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_dema_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = dema(&data, 0);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_dema_insufficient_data() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0]; // 4 elements
        let result = dema(&data, 3); // Needs 5 elements (2*3-1)

        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                required: 5,
                actual: 4,
                ..
            })
        ));
    }

    // ==================== dema_into Tests ====================

    #[test]
    fn test_dema_into_basic() {
        let data: Vec<f64> = (1..=10).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 10];
        let valid_count = dema_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 6); // 10 - 4 = 6
        assert!(output[3].is_nan());
        assert!(!output[4].is_nan());
    }

    #[test]
    fn test_dema_into_buffer_reuse() {
        let data1: Vec<f64> = (1..=10).map(|x| x as f64).collect();
        let data2: Vec<f64> = (10..=19).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 10];

        dema_into(&data1, 3, &mut output).unwrap();
        let val1 = output[9];

        dema_into(&data2, 3, &mut output).unwrap();
        let val2 = output[9];

        // Values should be different
        assert!((val1 - val2).abs() > EPSILON);
    }

    #[test]
    fn test_dema_into_insufficient_output() {
        let data: Vec<f64> = (1..=10).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 5]; // Too short
        let result = dema_into(&data, 3, &mut output);

        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_dema_into_f32() {
        let data: Vec<f32> = (1..=10).map(|x| x as f32).collect();
        let mut output = vec![0.0_f32; 10];
        let valid_count = dema_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 6);
        assert!(!output[4].is_nan());
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_dema_and_dema_into_produce_same_result() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let result1 = dema(&data, 5).unwrap();

        let mut result2 = vec![0.0_f64; data.len()];
        dema_into(&data, 5, &mut result2).unwrap();

        for i in 0..data.len() {
            assert!(approx_eq(result1[i], result2[i], EPSILON));
        }
    }

    #[test]
    fn test_dema_valid_count() {
        let data = vec![1.0_f64; 100];
        let mut output = vec![0.0_f64; 100];

        let valid_count = dema_into(&data, 10, &mut output).unwrap();
        // Lookback = 2*(10-1) = 18, so valid = 100 - 18 = 82
        assert_eq!(valid_count, 82);
    }

    // ==================== Property-Based-Like Tests ====================

    #[test]
    fn test_dema_output_length_equals_input_length() {
        for len in [10, 20, 50, 100] {
            for period in [2, 3, 5] {
                let min_len = dema_min_len(period);
                if len >= min_len {
                    let data: Vec<f64> = (0..len).map(|x| x as f64).collect();
                    let result = dema(&data, period).unwrap();
                    assert_eq!(result.len(), len);
                }
            }
        }
    }

    #[test]
    fn test_dema_nan_count() {
        // First 2*(period-1) values should be NaN
        for period in 2..=5 {
            let len = 50;
            let data: Vec<f64> = (0..len).map(|x| x as f64).collect();
            let result = dema(&data, period).unwrap();

            let expected_nan = dema_lookback(period);
            let nan_count = result.iter().filter(|x| x.is_nan()).count();
            assert_eq!(nan_count, expected_nan);
        }
    }

    // ==================== DEMA Formula Verification ====================

    #[test]
    fn test_dema_formula_verification() {
        // Verify DEMA = 2*EMA1 - EMA2 where EMA2 is computed on valid EMA1 values
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let period = 5;

        let ema1 = ema(&data, period).unwrap();
        let dema_result = dema(&data, period).unwrap();

        // Compute EMA2 manually as done in the implementation
        let alpha = 2.0 / (period as f64 + 1.0);
        let ema1_lookback = period - 1;
        let mut ema2 = ema1[ema1_lookback];

        let lookback = dema_lookback(period);
        for i in ema1_lookback..data.len() {
            if i == ema1_lookback {
                ema2 = ema1[i];
            } else {
                ema2 = alpha * ema1[i] + (1.0 - alpha) * ema2;
            }

            if i >= lookback {
                let expected = 2.0 * ema1[i] - ema2;
                assert!(approx_eq(dema_result[i], expected, EPSILON));
            }
        }
    }
}
