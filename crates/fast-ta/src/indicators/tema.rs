//! Triple Exponential Moving Average (TEMA) indicator.
//!
//! TEMA further reduces lag compared to DEMA by applying triple smoothing
//! with a correction formula.
//!
//! # Algorithm
//!
//! TEMA uses a three-step smoothing process:
//! 1. Calculate EMA1 = EMA(data, period)
//! 2. Calculate EMA2 = EMA(EMA1, period)
//! 3. Calculate EMA3 = EMA(EMA2, period)
//! 4. Apply the TEMA formula: 3×EMA1 - 3×EMA2 + EMA3
//!
//! # Formula
//!
//! ```text
//! TEMA = 3 × EMA1 - 3 × EMA2 + EMA3
//! ```
//!
//! This combination provides even less lag than DEMA while maintaining smoothness.
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::tema::tema;
//!
//! let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
//! let result = tema(&data, 3).unwrap();
//!
//! // TEMA has lookback of 3*(period-1)
//! assert!(result[0].is_nan());
//! assert!(result[5].is_nan());
//! assert!(!result[6].is_nan()); // First valid value at index 6
//! ```

use crate::error::{Error, Result};
use crate::indicators::ema::{ema, ema_into};
use crate::traits::{validate_period, SeriesElement, ValidatedInput};

/// Returns the lookback period for TEMA.
///
/// TEMA requires three sequential EMA calculations, so the lookback is
/// `3 * (period - 1)`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::tema::tema_lookback;
///
/// assert_eq!(tema_lookback(5), 12);  // 3 * (5-1) = 12
/// assert_eq!(tema_lookback(14), 39); // 3 * (14-1) = 39
/// ```
#[inline]
#[must_use]
pub const fn tema_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        3 * (period - 1)
    }
}

/// Returns the minimum input length required for TEMA.
///
/// This is the smallest input size that will produce at least one valid output.
/// For TEMA, this is `3 * period - 2`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::tema::tema_min_len;
///
/// assert_eq!(tema_min_len(5), 13);  // 3*5 - 2 = 13
/// assert_eq!(tema_min_len(14), 40); // 3*14 - 2 = 40
/// ```
#[inline]
#[must_use]
pub const fn tema_min_len(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        3 * period - 2
    }
}

/// Computes the Triple Exponential Moving Average (TEMA) of a data series.
///
/// Returns a vector of the same length as the input, where the first
/// `3 * (period - 1)` values are NaN (insufficient lookback data) and
/// subsequent values contain the TEMA.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods for EMA calculations
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` with the TEMA values, or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than `3 * period - 2` (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for intermediate EMA and output vectors
///
/// # Example
///
/// ```
/// use fast_ta::indicators::tema::tema;
///
/// let mut data: Vec<f64> = Vec::with_capacity(15);
/// for x in 1..=15 {
///     data.push(x as f64);
/// }
/// let result = tema(&data, 3).unwrap();
///
/// // First 6 values are NaN (lookback = 3*(3-1) = 6)
/// assert!(result[5].is_nan());
/// assert!(!result[6].is_nan());
/// ```
#[inline]
#[must_use = "this returns a Result with the TEMA values, which should be used"]
pub fn tema<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    // Validate inputs using shared utilities
    validate_period(period)?;
    data.validate_not_empty()?;
    let min_len = tema_min_len(period);
    data.validate_min_length(min_len, "tema")?;

    // Calculate EMA of input
    let ema1 = ema(data, period)?;

    // Calculate EMA2 and EMA3 manually to avoid NaN propagation
    let ema1_lookback = period - 1;
    let three = T::from_usize(3)?;
    let lookback = tema_lookback(period);

    let mut result = vec![T::nan(); data.len()];

    // Compute EMA2 and EMA3 manually
    let alpha = T::from_usize(2)? / T::from_usize(period + 1)?;
    let one_minus_alpha = T::one() - alpha;

    // Seed EMA2 with the first valid EMA1 value
    let mut ema2 = ema1[ema1_lookback];
    // Seed EMA3 with EMA2's initial value (which equals EMA1's first valid)
    let mut ema3 = ema2;

    // Track when EMA3 becomes valid: needs period-1 EMA1 values then period-1 EMA2 values
    let ema3_valid_from = 2 * ema1_lookback;

    for i in ema1_lookback..data.len() {
        if !ema1[i].is_nan() {
            if i == ema1_lookback {
                // Seed values
                ema2 = ema1[i];
                ema3 = ema1[i];
            } else {
                // Update EMA2
                ema2 = alpha * ema1[i] + one_minus_alpha * ema2;

                // Update EMA3 (only after EMA2 has been running for period-1 steps)
                if i >= ema3_valid_from {
                    ema3 = alpha * ema2 + one_minus_alpha * ema3;
                } else if i == ema3_valid_from - 1 {
                    // Seed EMA3 with current EMA2 value just before it becomes valid
                    ema3 = ema2;
                }
            }

            // TEMA is valid starting at lookback
            if i >= lookback {
                result[i] = three * ema1[i] - three * ema2 + ema3;
            }
        }
    }

    Ok(result)
}

/// Computes the Triple Exponential Moving Average into a pre-allocated output buffer.
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
/// A `Result` containing the number of valid TEMA values computed,
/// or an error if validation fails.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::tema::tema_into;
///
/// let mut data: Vec<f64> = Vec::with_capacity(15);
/// for x in 1..=15 {
///     data.push(x as f64);
/// }
/// let mut output = vec![0.0_f64; 15];
/// let valid_count = tema_into(&data, 3, &mut output).unwrap();
///
/// assert_eq!(valid_count, 9); // 15 - 6 = 9 valid values
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
#[inline]
#[must_use = "this returns a Result with the count of valid TEMA values"]
pub fn tema_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize> {
    // Validate inputs using shared utilities
    validate_period(period)?;
    data.validate_not_empty()?;
    let min_len = tema_min_len(period);
    data.validate_min_length(min_len, "tema")?;

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "tema",
        });
    }

    // Calculate EMA of input
    let mut ema1 = vec![T::nan(); data.len()];
    ema_into(data, period, &mut ema1)?;

    // Calculate EMA2 and EMA3 manually
    let ema1_lookback = period - 1;
    let three = T::from_usize(3)?;
    let lookback = tema_lookback(period);

    // Fill lookback with NaN
    for item in output.iter_mut().take(lookback) {
        *item = T::nan();
    }

    let alpha = T::from_usize(2)? / T::from_usize(period + 1)?;
    let one_minus_alpha = T::one() - alpha;

    let mut ema2 = ema1[ema1_lookback];
    let mut ema3 = ema2;

    let ema2_valid_from = ema1_lookback;

    for i in ema1_lookback..data.len() {
        if !ema1[i].is_nan() {
            if i == ema1_lookback {
                ema2 = ema1[i];
                ema3 = ema1[i];
            } else {
                ema2 = alpha * ema1[i] + one_minus_alpha * ema2;

                if i >= ema2_valid_from + ema1_lookback {
                    ema3 = alpha * ema2 + one_minus_alpha * ema3;
                } else if i == ema2_valid_from + ema1_lookback - 1 {
                    ema3 = ema2;
                }
            }

            if i >= lookback {
                output[i] = three * ema1[i] - three * ema2 + ema3;
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
    use crate::indicators::dema::dema;
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
    fn test_tema_lookback() {
        assert_eq!(tema_lookback(1), 0); // 3*(1-1) = 0
        assert_eq!(tema_lookback(3), 6); // 3*(3-1) = 6
        assert_eq!(tema_lookback(5), 12); // 3*(5-1) = 12
        assert_eq!(tema_lookback(14), 39); // 3*(14-1) = 39
    }

    #[test]
    fn test_tema_min_len() {
        assert_eq!(tema_min_len(1), 1); // 3*1 - 2 = 1
        assert_eq!(tema_min_len(3), 7); // 3*3 - 2 = 7
        assert_eq!(tema_min_len(5), 13); // 3*5 - 2 = 13
        assert_eq!(tema_min_len(14), 40); // 3*14 - 2 = 40
    }

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_tema_basic() {
        let data: Vec<f64> = (1..=15).map(|x| x as f64).collect();
        let result = tema(&data, 3).unwrap();

        assert_eq!(result.len(), 15);

        // First 6 values should be NaN (lookback = 3*(3-1) = 6)
        for i in 0..6 {
            assert!(result[i].is_nan(), "Expected NaN at index {}", i);
        }

        // Values from index 6 onwards should be valid
        for i in 6..15 {
            assert!(!result[i].is_nan(), "Expected valid at index {}", i);
        }
    }

    #[test]
    fn test_tema_f32() {
        let data: Vec<f32> = (1..=15).map(|x| x as f32).collect();
        let result = tema(&data, 3).unwrap();

        assert_eq!(result.len(), 15);
        assert!(result[5].is_nan());
        assert!(!result[6].is_nan());
    }

    #[test]
    fn test_tema_period_one() {
        // TEMA(1) should equal the input values
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = tema(&data, 1).unwrap();

        assert_eq!(result.len(), 5);
        for i in 0..5 {
            assert!(approx_eq(result[i], data[i], EPSILON));
        }
    }

    #[test]
    fn test_tema_constant_values() {
        // TEMA of constant values should equal the constant
        let data = vec![5.0_f64; 25];
        let result = tema(&data, 5).unwrap();

        // After lookback, all values should be 5.0
        for i in tema_lookback(5)..result.len() {
            assert!(approx_eq(result[i], 5.0, EPSILON));
        }
    }

    #[test]
    fn test_tema_reduces_lag_more_than_dema() {
        // For a trending sequence, TEMA should be closer to current price than DEMA
        let data: Vec<f64> = (1..=30).map(|x| x as f64).collect();
        let tema_result = tema(&data, 5).unwrap();
        let dema_result = dema(&data, 5).unwrap();
        let ema_result = ema(&data, 5).unwrap();

        let last_idx = data.len() - 1;

        // TEMA > DEMA > EMA for uptrending data (less lag = closer to current)
        assert!(tema_result[last_idx] > dema_result[last_idx]);
        assert!(dema_result[last_idx] > ema_result[last_idx]);
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_tema_minimum_length() {
        // Test with exactly minimum required length for period 3
        // min_len = 3*3 - 2 = 7
        let data: Vec<f64> = (1..=7).map(|x| x as f64).collect();
        let result = tema(&data, 3).unwrap();

        assert_eq!(result.len(), 7);
        // Only last value should be valid (lookback = 6)
        assert!(result[5].is_nan());
        assert!(!result[6].is_nan());
    }

    #[test]
    fn test_tema_negative_values() {
        let data: Vec<f64> = (-10..=10).map(|x| x as f64).collect();
        let result = tema(&data, 3).unwrap();

        // Should handle negative values correctly
        let lookback = tema_lookback(3);
        assert!(!result[lookback].is_nan());
        assert!(!result[result.len() - 1].is_nan());
    }

    #[test]
    fn test_tema_large_values() {
        let data: Vec<f64> = (1..=15).map(|x| x as f64 * 1e15).collect();
        let result = tema(&data, 3).unwrap();

        assert!(!result[6].is_nan());
        assert!(result[6] > 0.0);
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_tema_empty_input() {
        let data: Vec<f64> = vec![];
        let result = tema(&data, 3);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_tema_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = tema(&data, 0);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_tema_insufficient_data() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0]; // 6 elements
        let result = tema(&data, 3); // Needs 7 elements (3*3-2)

        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                required: 7,
                actual: 6,
                ..
            })
        ));
    }

    // ==================== tema_into Tests ====================

    #[test]
    fn test_tema_into_basic() {
        let data: Vec<f64> = (1..=15).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 15];
        let valid_count = tema_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 9); // 15 - 6 = 9
        assert!(output[5].is_nan());
        assert!(!output[6].is_nan());
    }

    #[test]
    fn test_tema_into_buffer_reuse() {
        let data1: Vec<f64> = (1..=15).map(|x| x as f64).collect();
        let data2: Vec<f64> = (15..=29).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 15];

        tema_into(&data1, 3, &mut output).unwrap();
        let val1 = output[14];

        tema_into(&data2, 3, &mut output).unwrap();
        let val2 = output[14];

        // Values should be different
        assert!((val1 - val2).abs() > EPSILON);
    }

    #[test]
    fn test_tema_into_insufficient_output() {
        let data: Vec<f64> = (1..=15).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 5]; // Too short
        let result = tema_into(&data, 3, &mut output);

        assert!(matches!(
            result,
            Err(Error::BufferTooSmall {
                required: 15,
                actual: 5,
                ..
            })
        ));
    }

    #[test]
    fn test_tema_into_empty_input() {
        let data: Vec<f64> = vec![];
        let mut output = vec![0.0_f64; 10];
        let result = tema_into(&data, 3, &mut output);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_tema_into_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let mut output = vec![0.0_f64; 3];
        let result = tema_into(&data, 0, &mut output);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_tema_into_insufficient_data() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0]; // 6 elements
        let mut output = vec![0.0_f64; 6];
        let result = tema_into(&data, 3, &mut output); // Needs 7 elements

        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                required: 7,
                actual: 6,
                ..
            })
        ));
    }

    // ==================== Comparison Tests ====================

    #[test]
    fn test_tema_vs_tema_into() {
        let data: Vec<f64> = (1..=30).map(|x| x as f64).collect();
        let result = tema(&data, 5).unwrap();

        let mut output = vec![0.0_f64; 30];
        tema_into(&data, 5, &mut output).unwrap();

        for i in 0..30 {
            if result[i].is_nan() {
                assert!(output[i].is_nan());
            } else {
                assert!(approx_eq(result[i], output[i], EPSILON));
            }
        }
    }

    #[test]
    fn test_tema_consistency() {
        // Test that TEMA values are consistent across different input sizes
        let data1: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let data2: Vec<f64> = (1..=30).map(|x| x as f64).collect();

        let result1 = tema(&data1, 3).unwrap();
        let result2 = tema(&data2, 3).unwrap();

        // First 20 values should match
        for i in 0..20 {
            if result1[i].is_nan() {
                assert!(result2[i].is_nan());
            } else {
                assert!(approx_eq(result1[i], result2[i], EPSILON));
            }
        }
    }
}