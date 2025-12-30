//! T3 (Tillson T3 Moving Average) indicator.
//!
//! T3 is a smoothed moving average developed by Tim Tillson that uses
//! a combination of six EMAs with a volume factor to reduce lag while
//! maintaining smoothness.
//!
//! # Formula
//!
//! T3 = c1*e6 + c2*e5 + c3*e4 + c4*e3
//!
//! where:
//! - e1 = EMA(data, period)
//! - e2 = EMA(e1, period)
//! - e3 = EMA(e2, period)
//! - e4 = EMA(e3, period)
//! - e5 = EMA(e4, period)
//! - e6 = EMA(e5, period)
//!
//! Coefficients (v = volume factor, typically 0.7):
//! - c1 = -v^3
//! - c2 = 3*v^2 + 3*v^3
//! - c3 = -6*v^2 - 3*v - 3*v^3
//! - c4 = 1 + 3*v + v^3 + 3*v^2
//!
//! # Lookback
//!
//! The lookback period is `6 * (period - 1)`.

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Computes the lookback period for T3.
///
/// The lookback is `6 * (period - 1)` due to six sequential EMAs.
#[inline]
#[must_use]
pub const fn t3_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        6 * (period - 1)
    }
}

/// Returns the minimum input length required for T3 calculation.
#[inline]
#[must_use]
pub const fn t3_min_len(period: usize) -> usize {
    if period == 0 {
        1
    } else {
        t3_lookback(period) + 1
    }
}

/// Computes T3 with default volume factor (0.7) and stores results in output.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The EMA period
/// * `output` - Pre-allocated output slice
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn t3_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    // Default volume factor is 0.7
    let vfactor = T::from_usize(7)? / T::from_usize(10)?;
    t3_full_into(data, period, vfactor, output)
}

/// Computes T3 with custom volume factor and stores results in output.
///
/// This implementation fuses all 6 EMA passes into a single loop for
/// improved cache efficiency and reduced memory allocation.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The EMA period
/// * `vfactor` - Volume factor (typically 0.7)
/// * `output` - Pre-allocated output slice
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn t3_full_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    vfactor: T,
    output: &mut [T],
) -> Result<()> {
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

    let lookback = t3_lookback(period);
    let min_len = t3_min_len(period);

    if data.len() < min_len {
        return Err(Error::InsufficientData {
            indicator: "t3",
            required: min_len,
            actual: data.len(),
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            indicator: "t3",
            required: data.len(),
            actual: output.len(),
        });
    }

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // For period 1, T3 equals the input (no smoothing)
    if period == 1 {
        for i in 0..data.len() {
            output[i] = data[i];
        }
        return Ok(());
    }

    // EMA smoothing factor
    let alpha = T::from_usize(2)? / T::from_usize(period + 1)?;
    let one_minus_alpha = T::one() - alpha;

    let ema_lookback = period - 1;
    let n = data.len();

    // Use scalar EMA state variables instead of arrays
    // Each EMA level starts being valid at (level * ema_lookback)
    let start1 = ema_lookback;
    let start2 = 2 * ema_lookback;
    let start3 = 3 * ema_lookback;
    let start4 = 4 * ema_lookback;
    let start5 = 5 * ema_lookback;
    // start6 = lookback = 6 * ema_lookback

    // Initialize EMA1 at its start point
    let mut e1 = data[start1];

    // Advance EMA1 to start2 and initialize EMA2
    for i in (start1 + 1)..=start2 {
        e1 = alpha * data[i] + one_minus_alpha * e1;
    }
    let mut e2 = e1;

    // Advance EMAs to start3 and initialize EMA3
    for i in (start2 + 1)..=start3 {
        e1 = alpha * data[i] + one_minus_alpha * e1;
        e2 = alpha * e1 + one_minus_alpha * e2;
    }
    let mut e3 = e2;

    // Advance EMAs to start4 and initialize EMA4
    for i in (start3 + 1)..=start4 {
        e1 = alpha * data[i] + one_minus_alpha * e1;
        e2 = alpha * e1 + one_minus_alpha * e2;
        e3 = alpha * e2 + one_minus_alpha * e3;
    }
    let mut e4 = e3;

    // Advance EMAs to start5 and initialize EMA5
    for i in (start4 + 1)..=start5 {
        e1 = alpha * data[i] + one_minus_alpha * e1;
        e2 = alpha * e1 + one_minus_alpha * e2;
        e3 = alpha * e2 + one_minus_alpha * e3;
        e4 = alpha * e3 + one_minus_alpha * e4;
    }
    let mut e5 = e4;

    // Advance EMAs to lookback and initialize EMA6
    for i in (start5 + 1)..lookback {
        e1 = alpha * data[i] + one_minus_alpha * e1;
        e2 = alpha * e1 + one_minus_alpha * e2;
        e3 = alpha * e2 + one_minus_alpha * e3;
        e4 = alpha * e3 + one_minus_alpha * e4;
        e5 = alpha * e4 + one_minus_alpha * e5;
    }

    // At lookback, update all EMAs
    e1 = alpha * data[lookback] + one_minus_alpha * e1;
    e2 = alpha * e1 + one_minus_alpha * e2;
    e3 = alpha * e2 + one_minus_alpha * e3;
    e4 = alpha * e3 + one_minus_alpha * e4;
    e5 = alpha * e4 + one_minus_alpha * e5;
    let mut e6 = e5;

    // Calculate coefficients after initialization (TA-Lib style - keeps values in registers)
    let temp_real = vfactor * vfactor;  // v^2
    let c1 = T::zero() - (temp_real * vfactor);  // -v^3
    let three = T::from_usize(3)?;
    let six = T::from_usize(6)?;
    let c2 = three * (temp_real - c1);  // 3(v^2 + v^3)
    let c3 = T::zero() - six * temp_real - three * (vfactor - c1);  // -6v^2 - 3v - 3v^3
    let c4 = T::one() + three * vfactor - c1 + three * temp_real;  // 1 + 3v + v^3 + 3v^2

    // First valid T3 value
    output[lookback] = c1 * e6 + c2 * e5 + c3 * e4 + c4 * e3;

    // Main loop: update all 6 EMAs and compute T3 in single pass
    for i in (lookback + 1)..n {
        e1 = alpha * data[i] + one_minus_alpha * e1;
        e2 = alpha * e1 + one_minus_alpha * e2;
        e3 = alpha * e2 + one_minus_alpha * e3;
        e4 = alpha * e3 + one_minus_alpha * e4;
        e5 = alpha * e4 + one_minus_alpha * e5;
        e6 = alpha * e5 + one_minus_alpha * e6;
        output[i] = c1 * e6 + c2 * e5 + c3 * e4 + c4 * e3;
    }

    Ok(())
}

/// Computes T3 with default volume factor (0.7).
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The EMA period
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of T3 values
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::t3;
///
/// let mut prices: Vec<f64> = Vec::with_capacity(20);
/// for x in 1..=20 {
///     prices.push(x as f64);
/// }
/// let result = t3(&prices, 3).unwrap();
/// // First 12 values are NaN (lookback = 6 * (3-1) = 12)
/// assert!(result[12].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn t3<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    t3_into(data, period, &mut output)?;
    Ok(output)
}

/// Computes T3 with custom volume factor.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The EMA period
/// * `vfactor` - Volume factor (typically 0.7)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of T3 values
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn t3_full<T: SeriesElement>(data: &[T], period: usize, vfactor: T) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    t3_full_into(data, period, vfactor, &mut output)?;
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
    fn test_t3_lookback() {
        assert_eq!(t3_lookback(1), 0);
        assert_eq!(t3_lookback(2), 6);
        assert_eq!(t3_lookback(3), 12);
        assert_eq!(t3_lookback(5), 24);
        assert_eq!(t3_lookback(0), 0);
    }

    #[test]
    fn test_t3_min_len() {
        assert_eq!(t3_min_len(1), 1);
        assert_eq!(t3_min_len(2), 7);
        assert_eq!(t3_min_len(3), 13);
        assert_eq!(t3_min_len(5), 25);
    }

    #[test]
    fn test_t3_empty_input() {
        let data: Vec<f64> = vec![];
        let result = t3(&data, 3);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_t3_zero_period() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = t3(&data, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_t3_insufficient_data() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0];
        let result = t3(&data, 3);
        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                indicator: "t3",
                ..
            })
        ));
    }

    #[test]
    fn test_t3_output_length_equals_input_length() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let result = t3(&data, 3).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_t3_nan_count() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let period = 3;
        let result = t3(&data, period).unwrap();

        let expected_lookback = 6 * (period - 1);
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, expected_lookback);
    }

    #[test]
    fn test_t3_valid_count() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let period = 3;
        let result = t3(&data, period).unwrap();

        let lookback = t3_lookback(period);
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        assert_eq!(valid_count, data.len() - lookback);
    }

    #[test]
    fn test_t3_period_one() {
        // With period 1, T3 should equal input (no smoothing)
        let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = t3(&data, 1).unwrap();

        for i in 0..data.len() {
            assert!(approx_eq(result[i], data[i], EPSILON));
        }
    }

    #[test]
    fn test_t3_constant_values() {
        // T3 of constant values should stay constant
        let data: Vec<f64> = vec![42.0; 20];
        let result = t3(&data, 3).unwrap();

        for i in 12..20 {
            assert!(approx_eq(result[i], 42.0, EPSILON));
        }
    }

    #[test]
    fn test_t3_trending_market() {
        // T3 should follow trend direction
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let result = t3(&data, 3).unwrap();

        // T3 should increase in uptrend
        let valid_start = t3_lookback(3);
        for i in (valid_start + 1)..result.len() {
            assert!(
                result[i] > result[i - 1],
                "T3 should increase: result[{}]={} > result[{}]={}",
                i,
                result[i],
                i - 1,
                result[i - 1]
            );
        }
    }

    #[test]
    fn test_t3_f32() {
        let data: Vec<f32> = (1..=20).map(|x| x as f32).collect();
        let result = t3(&data, 3).unwrap();

        assert_eq!(result.len(), data.len());

        // First 12 should be NaN
        for i in 0..12 {
            assert!(result[i].is_nan());
        }

        // Rest should be valid
        for i in 12..20 {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_t3_into_f32() {
        let data: Vec<f32> = (1..=20).map(|x| x as f32).collect();
        let mut output = vec![0.0_f32; data.len()];
        t3_into(&data, 3, &mut output).unwrap();

        for i in 0..12 {
            assert!(output[i].is_nan());
        }

        for i in 12..20 {
            assert!(output[i].is_finite());
        }
    }

    #[test]
    fn test_t3_into_insufficient_output() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let mut output: Vec<f64> = vec![0.0; 10]; // Too small
        let result = t3_into(&data, 3, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_t3_full_custom_vfactor() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();

        // Different vfactors should produce different results
        let result_07 = t3_full(&data, 3, 0.7).unwrap();
        let result_09 = t3_full(&data, 3, 0.9).unwrap();

        // Both should be valid but different
        assert!(result_07[15].is_finite());
        assert!(result_09[15].is_finite());
        // With higher vfactor, T3 reacts differently
        assert!((result_07[15] - result_09[15]).abs() > 0.001);
    }

    #[test]
    fn test_t3_minimum_length() {
        let data: Vec<f64> = (1..=13).map(|x| x as f64).collect();
        let result = t3(&data, 3).unwrap();

        assert_eq!(result.len(), 13);
        // First 12 are NaN
        for i in 0..12 {
            assert!(result[i].is_nan());
        }
        // Only last value is valid
        assert!(result[12].is_finite());
    }

    #[test]
    fn test_t3_negative_values() {
        let data: Vec<f64> = (-10..=10).map(|x| x as f64).collect();
        let result = t3(&data, 3).unwrap();

        // Should handle negative values correctly
        let lookback = t3_lookback(3);
        for i in lookback..result.len() {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_t3_large_values() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64 * 1e6).collect();
        let result = t3(&data, 3).unwrap();

        let lookback = t3_lookback(3);
        for i in lookback..result.len() {
            assert!(result[i].is_finite());
        }
    }
}