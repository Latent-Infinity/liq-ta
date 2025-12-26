//! KAMA (Kaufman Adaptive Moving Average) indicator.
//!
//! KAMA is an adaptive moving average that adjusts its smoothing based on
//! market efficiency. It responds quickly during trending markets and slowly
//! during sideways markets.
//!
//! # Formula
//!
//! 1. Efficiency Ratio (ER) = |Price - Price\[n ago\]| / Sum(|Price\[i\] - Price\[i-1\]|)
//! 2. Smoothing Constant (SC) = \[ER * (`fast_sc` - `slow_sc`) + `slow_sc`\]^2
//!    where `fast_sc` = `2/(fast_period+1)`, `slow_sc` = `2/(slow_period+1)`
//! 3. KAMA = KAMA\[prev\] + SC * (Price - KAMA\[prev\])
//!
//! # Default Parameters
//!
//! - period: 10 (for efficiency ratio calculation)
//! - `fast_period`: 2 (fast EMA smoothing)
//! - `slow_period`: 30 (slow EMA smoothing)
//!
//! # Lookback
//!
//! The lookback period is `period - 1`.

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Computes the lookback period for KAMA.
///
/// The lookback is `period - 1`.
#[inline]
#[must_use]
pub const fn kama_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for KAMA calculation.
#[inline]
#[must_use]
pub const fn kama_min_len(period: usize) -> usize {
    if period == 0 {
        1
    } else {
        period
    }
}

/// Computes KAMA with default fast/slow periods (2/30) and stores results in output.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The efficiency ratio period (typically 10)
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
pub fn kama_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    kama_full_into(data, period, 2, 30, output)
}

/// Computes KAMA with custom fast/slow periods and stores results in output.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The efficiency ratio period
/// * `fast_period` - Fast EMA period (typically 2)
/// * `slow_period` - Slow EMA period (typically 30)
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
pub fn kama_full_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    fast_period: usize,
    slow_period: usize,
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

    if fast_period == 0 {
        return Err(Error::InvalidPeriod {
            period: fast_period,
            reason: "fast_period must be at least 1",
        });
    }

    if slow_period == 0 {
        return Err(Error::InvalidPeriod {
            period: slow_period,
            reason: "slow_period must be at least 1",
        });
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            indicator: "kama",
            required: period,
            actual: data.len(),
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            indicator: "kama",
            required: data.len(),
            actual: output.len(),
        });
    }

    let lookback = kama_lookback(period);
    let n = data.len();
    let zero = T::zero();

    // Fill lookback period with NaN
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Calculate smoothing constants
    let two = T::from_usize(2)?;
    let fast_sc = two / T::from_usize(fast_period + 1)?;
    let slow_sc = two / T::from_usize(slow_period + 1)?;
    let sc_diff = fast_sc - slow_sc;

    // Pre-compute all absolute changes (volatility components)
    // abs_changes[i] = |data[i] - data[i-1]| for i >= 1
    let mut abs_changes = vec![zero; n];
    for i in 1..n {
        abs_changes[i] = (data[i] - data[i - 1]).abs();
    }

    // Initialize KAMA with first valid value
    let mut kama = data[lookback];
    output[lookback] = kama;

    // Early exit if only one valid output
    if lookback + 1 >= n {
        return Ok(());
    }

    // Initialize volatility for first calculation (at lookback + 1)
    // Volatility window for i = lookback + 1 = period is [1, period]
    let mut volatility = zero;
    for j in 1..=period {
        volatility = volatility + abs_changes[j];
    }

    // Calculate KAMA for remaining values
    for i in (lookback + 1)..n {
        // Calculate change (direction)
        let change = (data[i] - data[i - period]).abs();

        // Calculate Efficiency Ratio
        let er = if volatility > zero {
            change / volatility
        } else {
            zero
        };

        // Calculate Smoothing Constant
        let sc_raw = er * sc_diff + slow_sc;
        let sc = sc_raw * sc_raw;

        // Update KAMA
        kama = kama + sc * (data[i] - kama);
        output[i] = kama;

        // Update volatility for next iteration (if not last)
        if i + 1 < n {
            // Window slides: remove oldest, add newest
            // Current window was [i - period + 1, i]
            // Next window is [i - period + 2, i + 1]
            let old_idx = i - period + 1;
            volatility = volatility - abs_changes[old_idx] + abs_changes[i + 1];
        }
    }

    Ok(())
}

/// Computes KAMA with default fast/slow periods (2/30).
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The efficiency ratio period (typically 10)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of KAMA values
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::kama;
///
/// let prices: Vec<f64> = vec![10.0, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 13.5, 14.0, 13.5, 14.5, 15.0];
/// let result = kama(&prices, 10).unwrap();
/// assert!(result[9].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn kama<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    kama_into(data, period, &mut output)?;
    Ok(output)
}

/// Computes KAMA with custom fast/slow periods.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The efficiency ratio period
/// * `fast_period` - Fast EMA period (typically 2)
/// * `slow_period` - Slow EMA period (typically 30)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of KAMA values
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn kama_full<T: SeriesElement>(
    data: &[T],
    period: usize,
    fast_period: usize,
    slow_period: usize,
) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    kama_full_into(data, period, fast_period, slow_period, &mut output)?;
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
    fn test_kama_lookback() {
        assert_eq!(kama_lookback(1), 0);
        assert_eq!(kama_lookback(2), 1);
        assert_eq!(kama_lookback(10), 9);
        assert_eq!(kama_lookback(0), 0);
    }

    #[test]
    fn test_kama_min_len() {
        assert_eq!(kama_min_len(1), 1);
        assert_eq!(kama_min_len(2), 2);
        assert_eq!(kama_min_len(10), 10);
    }

    #[test]
    fn test_kama_empty_input() {
        let data: Vec<f64> = vec![];
        let result = kama(&data, 10);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_kama_zero_period() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = kama(&data, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_kama_insufficient_data() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0];
        let result = kama(&data, 10);
        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                indicator: "kama",
                required: 10,
                actual: 3,
            })
        ));
    }

    #[test]
    fn test_kama_output_length_equals_input_length() {
        let data: Vec<f64> = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let result = kama(&data, 10).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_kama_nan_count() {
        let data: Vec<f64> = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let period = 10;
        let result = kama(&data, period).unwrap();

        // Count NaN values - should be period - 1 = 9
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, period - 1);
    }

    #[test]
    fn test_kama_valid_count() {
        let data: Vec<f64> = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let period = 10;
        let result = kama(&data, period).unwrap();

        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        assert_eq!(valid_count, data.len() - (period - 1));
    }

    #[test]
    fn test_kama_first_value_equals_data() {
        // First KAMA value should equal the data value at that point
        let data: Vec<f64> = vec![
            10.0, 11.0, 12.0, 11.0, 12.0, 13.0, 12.0, 13.0, 14.0, 13.0, 14.0, 15.0,
        ];
        let result = kama(&data, 10).unwrap();

        // First valid KAMA should be at index 9 (lookback = 9)
        assert!(approx_eq(result[9], data[9], EPSILON));
    }

    #[test]
    fn test_kama_trending_market() {
        // In a strongly trending market (high ER), KAMA should follow price closely
        let data: Vec<f64> = vec![
            10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0,
            24.0,
        ];
        let result = kama(&data, 10).unwrap();

        // KAMA should increase with the trend
        for i in 10..result.len() {
            assert!(result[i] > result[i - 1], "KAMA should increase in uptrend");
        }
    }

    #[test]
    fn test_kama_sideways_market() {
        // In a sideways market (low ER), KAMA should be relatively flat
        let data: Vec<f64> = vec![
            10.0, 11.0, 10.0, 11.0, 10.0, 11.0, 10.0, 11.0, 10.0, 11.0, 10.0, 11.0, 10.0, 11.0,
            10.0,
        ];
        let result = kama(&data, 10).unwrap();

        // Calculate KAMA range after lookback
        let valid_kama: Vec<f64> = result.iter().filter(|x| !x.is_nan()).cloned().collect();
        let kama_range = valid_kama.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            - valid_kama.iter().cloned().fold(f64::INFINITY, f64::min);

        // KAMA range should be smaller than price range in sideways market
        assert!(kama_range < 1.0, "KAMA should be flat in sideways market");
    }

    #[test]
    fn test_kama_constant_values() {
        // KAMA of constant values should stay constant
        let data: Vec<f64> = vec![42.0; 15];
        let result = kama(&data, 10).unwrap();

        for i in 9..15 {
            assert!(approx_eq(result[i], 42.0, EPSILON));
        }
    }

    #[test]
    fn test_kama_period_one() {
        // With period 1, KAMA should equal the input (ER = 0 when no change period)
        let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = kama(&data, 1).unwrap();

        // First value should equal input
        assert!(approx_eq(result[0], data[0], EPSILON));
        // All values should be valid
        for i in 0..data.len() {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_kama_f32() {
        let data: Vec<f32> = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let result = kama(&data, 10).unwrap();

        assert_eq!(result.len(), data.len());

        // First 9 should be NaN
        for i in 0..9 {
            assert!(result[i].is_nan());
        }

        // Rest should be valid
        for i in 9..12 {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_kama_into_f32() {
        let data: Vec<f32> = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let mut output = vec![0.0_f32; data.len()];
        kama_into(&data, 10, &mut output).unwrap();

        for i in 0..9 {
            assert!(output[i].is_nan());
        }

        for i in 9..12 {
            assert!(output[i].is_finite());
        }
    }

    #[test]
    fn test_kama_into_insufficient_output() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut output: Vec<f64> = vec![0.0; 5]; // Too small
        let result = kama_into(&data, 5, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_kama_full_custom_periods() {
        let data: Vec<f64> = vec![
            10.0, 11.0, 12.0, 11.0, 12.0, 13.0, 12.0, 13.0, 14.0, 13.0, 14.0, 15.0,
        ];

        // With faster periods, KAMA should react more quickly
        let result_fast = kama_full(&data, 10, 2, 10).unwrap();
        let result_slow = kama_full(&data, 10, 2, 50).unwrap();

        // Both should have same number of NaN
        let fast_valid: Vec<f64> = result_fast
            .iter()
            .filter(|x| !x.is_nan())
            .cloned()
            .collect();
        let slow_valid: Vec<f64> = result_slow
            .iter()
            .filter(|x| !x.is_nan())
            .cloned()
            .collect();

        assert_eq!(fast_valid.len(), slow_valid.len());
    }

    #[test]
    fn test_kama_minimum_length() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = kama(&data, 10).unwrap();

        assert_eq!(result.len(), 10);
        // First 9 are NaN
        for i in 0..9 {
            assert!(result[i].is_nan());
        }
        // Only last value is valid
        assert!(result[9].is_finite());
    }

    #[test]
    fn test_kama_negative_values() {
        let data: Vec<f64> = vec![
            -10.0, -9.0, -8.0, -7.0, -6.0, -5.0, -4.0, -3.0, -2.0, -1.0, 0.0, 1.0,
        ];
        let result = kama(&data, 10).unwrap();

        // Should handle negative values correctly
        for i in 9..12 {
            assert!(result[i].is_finite());
        }

        // KAMA should increase with uptrend
        assert!(result[10] > result[9]);
        assert!(result[11] > result[10]);
    }

    #[test]
    fn test_kama_large_values() {
        let data: Vec<f64> = vec![
            1e15, 2e15, 3e15, 4e15, 5e15, 6e15, 7e15, 8e15, 9e15, 1e16, 1.1e16, 1.2e16,
        ];
        let result = kama(&data, 10).unwrap();

        for i in 9..12 {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_kama_full_invalid_fast_period() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = kama_full(&data, 10, 0, 30);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_kama_full_invalid_slow_period() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = kama_full(&data, 10, 2, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }
}
