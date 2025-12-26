//! AROON indicator.
//!
//! Aroon is a trend-following indicator that measures the time since a high or low
//! occurred within a specified period.
//!
//! # Formula
//!
//! ```text
//! Aroon Up   = ((period - periods since highest high) / period) * 100
//! Aroon Down = ((period - periods since lowest low) / period) * 100
//! ```
//!
//! # Range
//!
//! Both Aroon Up and Aroon Down range from 0 to 100:
//! - 100: High/Low occurred at the most recent bar
//! - 0: High/Low occurred `period` bars ago
//!
//! # Interpretation
//!
//! - Aroon Up > 70: Strong uptrend
//! - Aroon Down > 70: Strong downtrend
//! - Crossovers indicate trend changes
//!
//! # Lookback
//!
//! The lookback period is `period`.

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Output structure for AROON indicator.
#[derive(Debug, Clone)]
pub struct AroonOutput<T> {
    /// Aroon Up values (0-100)
    pub aroon_up: Vec<T>,
    /// Aroon Down values (0-100)
    pub aroon_down: Vec<T>,
}

/// Computes the lookback period for AROON.
#[inline]
#[must_use]
pub const fn aroon_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for AROON calculation.
#[inline]
#[must_use]
pub const fn aroon_min_len(period: usize) -> usize {
    period + 1
}

/// Computes AROON and stores results in output slices.
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `period` - Lookback period
/// * `aroon_up_output` - Pre-allocated output slice for Aroon Up
/// * `aroon_down_output` - Pre-allocated output slice for Aroon Down
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn aroon_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
    aroon_up_output: &mut [T],
    aroon_down_output: &mut [T],
) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    if low.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "high and low arrays must have same length: high={}, low={}",
                n,
                low.len()
            ),
        });
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    let min_len = aroon_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "aroon",
            required: min_len,
            actual: n,
        });
    }

    if aroon_up_output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "aroon",
            required: n,
            actual: aroon_up_output.len(),
        });
    }

    if aroon_down_output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "aroon",
            required: n,
            actual: aroon_down_output.len(),
        });
    }

    let lookback = aroon_lookback(period);
    let period_t = T::from_usize(period)?;
    let hundred = T::from_f64(100.0)?;

    // Fill lookback period with NaN
    for i in 0..lookback {
        aroon_up_output[i] = T::nan();
        aroon_down_output[i] = T::nan();
    }

    // Calculate AROON for each bar after lookback
    for i in lookback..n {
        let start = i - period;
        let end = i + 1; // inclusive of current bar

        // Find index of highest high and lowest low in the window, propagating NaNs.
        let mut nan_in_window = false;
        let mut highest_idx = start;
        let mut lowest_idx = start;
        let mut highest_val = high[start];
        let mut lowest_val = low[start];

        if is_invalid(highest_val) || is_invalid(lowest_val) {
            nan_in_window = true;
        } else {
            for j in (start + 1)..end {
                let high_val = high[j];
                let low_val = low[j];
                if is_invalid(high_val) || is_invalid(low_val) {
                    nan_in_window = true;
                    break;
                }
                if high_val >= highest_val {
                    highest_val = high_val;
                    highest_idx = j;
                }
                if low_val <= lowest_val {
                    lowest_val = low_val;
                    lowest_idx = j;
                }
            }
        }

        if nan_in_window {
            aroon_up_output[i] = T::nan();
            aroon_down_output[i] = T::nan();
            continue;
        }

        // Periods since highest high and lowest low
        let periods_since_high = i - highest_idx;
        let periods_since_low = i - lowest_idx;

        // Aroon Up = ((period - periods_since_high) / period) * 100
        let aroon_up = ((period_t - T::from_usize(periods_since_high)?) / period_t) * hundred;
        // Aroon Down = ((period - periods_since_low) / period) * 100
        let aroon_down = ((period_t - T::from_usize(periods_since_low)?) / period_t) * hundred;

        aroon_up_output[i] = aroon_up;
        aroon_down_output[i] = aroon_down;
    }

    Ok(())
}

/// Computes AROON (Aroon Up and Aroon Down).
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `period` - Lookback period (typically 25)
///
/// # Returns
///
/// * `Ok(AroonOutput)` - Aroon Up and Aroon Down values (0-100)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::aroon;
///
/// let high = vec![12.0, 13.0, 14.0, 15.0, 14.5, 14.0, 13.5, 13.0, 12.5, 12.0, 11.5];
/// let low = vec![10.0, 11.0, 12.0, 13.0, 12.5, 12.0, 11.5, 11.0, 10.5, 10.0, 9.5];
///
/// let result = aroon(&high, &low, 5).unwrap();
/// // After lookback, values are between 0 and 100
/// assert!(result.aroon_up[5] >= 0.0 && result.aroon_up[5] <= 100.0);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn aroon<T: SeriesElement>(high: &[T], low: &[T], period: usize) -> Result<AroonOutput<T>> {
    let n = high.len();
    let mut aroon_up = vec![T::zero(); n];
    let mut aroon_down = vec![T::zero(); n];
    aroon_into(high, low, period, &mut aroon_up, &mut aroon_down)?;
    Ok(AroonOutput {
        aroon_up,
        aroon_down,
    })
}

/// Computes the lookback period for AROONOSC.
#[inline]
#[must_use]
pub const fn aroonosc_lookback(period: usize) -> usize {
    aroon_lookback(period)
}

/// Returns the minimum input length required for AROONOSC calculation.
#[inline]
#[must_use]
pub const fn aroonosc_min_len(period: usize) -> usize {
    aroon_min_len(period)
}

/// Computes AROONOSC (Aroon Oscillator) and stores results in output slice.
///
/// AROONOSC = Aroon Up - Aroon Down
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `period` - Lookback period
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn aroonosc_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    if low.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "high and low arrays must have same length: high={}, low={}",
                n,
                low.len()
            ),
        });
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    let min_len = aroonosc_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "aroonosc",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "aroonosc",
            required: n,
            actual: output.len(),
        });
    }

    // Calculate AROON first
    let mut aroon_up = vec![T::zero(); n];
    let mut aroon_down = vec![T::zero(); n];
    aroon_into(high, low, period, &mut aroon_up, &mut aroon_down)?;

    // AROONOSC = Aroon Up - Aroon Down
    for i in 0..n {
        output[i] = aroon_up[i] - aroon_down[i];
    }

    Ok(())
}

/// Computes AROONOSC (Aroon Oscillator).
///
/// AROONOSC = Aroon Up - Aroon Down
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `period` - Lookback period (typically 25)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Aroon Oscillator values (range -100 to +100)
/// * `Err(Error)` if inputs are invalid
///
/// # Range
///
/// AROONOSC ranges from -100 to +100:
/// - +100: Strong uptrend (Aroon Up = 100, Aroon Down = 0)
/// - -100: Strong downtrend (Aroon Up = 0, Aroon Down = 100)
/// - 0: Balanced market
///
/// # Example
///
/// ```
/// use fast_ta::indicators::aroonosc;
///
/// let high = vec![12.0, 13.0, 14.0, 15.0, 14.5, 14.0, 13.5, 13.0, 12.5, 12.0, 11.5];
/// let low = vec![10.0, 11.0, 12.0, 13.0, 12.5, 12.0, 11.5, 11.0, 10.5, 10.0, 9.5];
///
/// let result = aroonosc(&high, &low, 5).unwrap();
/// // Values are between -100 and +100
/// assert!(result[5] >= -100.0 && result[5] <= 100.0);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn aroonosc<T: SeriesElement>(high: &[T], low: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::zero(); high.len()];
    aroonosc_into(high, low, period, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_aroon_lookback() {
        assert_eq!(aroon_lookback(5), 5);
        assert_eq!(aroon_lookback(14), 14);
        assert_eq!(aroon_lookback(25), 25);
    }

    #[test]
    fn test_aroon_min_len() {
        assert_eq!(aroon_min_len(5), 6);
        assert_eq!(aroon_min_len(14), 15);
        assert_eq!(aroon_min_len(25), 26);
    }

    #[test]
    fn test_aroon_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let result = aroon(&high, &low, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_aroon_invalid_period() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = aroon(&high, &low, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_aroon_insufficient_data() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let result = aroon(&high, &low, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_aroon_length_mismatch() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0];
        let result = aroon(&high, &low, 5);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_aroon_output_length() {
        let high: Vec<f64> = vec![
            10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
        ];
        let low: Vec<f64> = vec![
            9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0,
        ];
        let result = aroon(&high, &low, 5).unwrap();
        assert_eq!(result.aroon_up.len(), high.len());
        assert_eq!(result.aroon_down.len(), high.len());
    }

    #[test]
    fn test_aroon_lookback_nan() {
        let high: Vec<f64> = vec![
            10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
        ];
        let low: Vec<f64> = vec![
            9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0,
        ];
        let result = aroon(&high, &low, 5).unwrap();

        // First 5 values should be NaN (lookback = period)
        for i in 0..5 {
            assert!(result.aroon_up[i].is_nan(), "aroon_up[{}] should be NaN", i);
            assert!(
                result.aroon_down[i].is_nan(),
                "aroon_down[{}] should be NaN",
                i
            );
        }

        // Values after lookback should be finite
        for i in 5..result.aroon_up.len() {
            assert!(
                result.aroon_up[i].is_finite(),
                "aroon_up[{}] should be finite",
                i
            );
            assert!(
                result.aroon_down[i].is_finite(),
                "aroon_down[{}] should be finite",
                i
            );
        }
    }

    #[test]
    fn test_aroon_range() {
        let high: Vec<f64> = vec![
            10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
        ];
        let low: Vec<f64> = vec![
            9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0,
        ];
        let result = aroon(&high, &low, 5).unwrap();

        for i in 5..result.aroon_up.len() {
            assert!(
                result.aroon_up[i] >= 0.0 && result.aroon_up[i] <= 100.0,
                "aroon_up[{}] = {} should be in [0, 100]",
                i,
                result.aroon_up[i]
            );
            assert!(
                result.aroon_down[i] >= 0.0 && result.aroon_down[i] <= 100.0,
                "aroon_down[{}] = {} should be in [0, 100]",
                i,
                result.aroon_down[i]
            );
        }
    }

    #[test]
    fn test_aroon_strong_uptrend() {
        // Monotonically increasing prices - Aroon Up should be 100
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = aroon(&high, &low, 5).unwrap();

        // At index 5, highest high is at current bar (0 periods ago)
        // Aroon Up = ((5 - 0) / 5) * 100 = 100
        assert!(
            (result.aroon_up[5] - 100.0).abs() < 1e-10,
            "aroon_up should be 100 in uptrend"
        );

        // Lowest low is at beginning (5 periods ago)
        // Aroon Down = ((5 - 5) / 5) * 100 = 0
        assert!(
            (result.aroon_down[5] - 0.0).abs() < 1e-10,
            "aroon_down should be 0 in uptrend"
        );
    }

    #[test]
    fn test_aroon_strong_downtrend() {
        // Monotonically decreasing prices - Aroon Down should be 100
        let high: Vec<f64> = vec![20.0, 19.0, 18.0, 17.0, 16.0, 15.0];
        let low: Vec<f64> = vec![19.0, 18.0, 17.0, 16.0, 15.0, 14.0];
        let result = aroon(&high, &low, 5).unwrap();

        // At index 5, lowest low is at current bar (0 periods ago)
        // Aroon Down = ((5 - 0) / 5) * 100 = 100
        assert!(
            (result.aroon_down[5] - 100.0).abs() < 1e-10,
            "aroon_down should be 100 in downtrend"
        );

        // Highest high is at beginning (5 periods ago)
        // Aroon Up = ((5 - 5) / 5) * 100 = 0
        assert!(
            (result.aroon_up[5] - 0.0).abs() < 1e-10,
            "aroon_up should be 0 in downtrend"
        );
    }

    #[test]
    fn test_aroon_calculation() {
        // Create a specific pattern to verify calculation
        let high: Vec<f64> = vec![10.0, 15.0, 12.0, 13.0, 11.0, 14.0];
        let low: Vec<f64> = vec![8.0, 13.0, 10.0, 11.0, 9.0, 12.0];
        let result = aroon(&high, &low, 5).unwrap();

        // At index 5:
        // Window is [10,15,12,13,11,14] for highs
        // Highest high is 15 at index 1 (4 periods ago)
        // Aroon Up = ((5 - 4) / 5) * 100 = 20
        assert!(
            (result.aroon_up[5] - 20.0).abs() < 1e-10,
            "aroon_up[5] = {}",
            result.aroon_up[5]
        );

        // Window is [8,13,10,11,9,12] for lows
        // Lowest low is 8 at index 0 (5 periods ago)
        // Aroon Down = ((5 - 5) / 5) * 100 = 0
        assert!(
            (result.aroon_down[5] - 0.0).abs() < 1e-10,
            "aroon_down[5] = {}",
            result.aroon_down[5]
        );
    }

    #[test]
    fn test_aroon_high_at_end_of_period() {
        // High at current bar should give Aroon Up = 100
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 11.0, 10.0, 13.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 10.0, 9.0, 12.0];
        let result = aroon(&high, &low, 5).unwrap();

        // At index 5, highest is 13 at index 5 (0 periods ago)
        // Aroon Up = ((5 - 0) / 5) * 100 = 100
        assert!((result.aroon_up[5] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_aroon_low_at_end_of_period() {
        // Low at current bar should give Aroon Down = 100
        let high: Vec<f64> = vec![15.0, 14.0, 13.0, 14.0, 15.0, 12.0];
        let low: Vec<f64> = vec![14.0, 13.0, 12.0, 13.0, 14.0, 11.0];
        let result = aroon(&high, &low, 5).unwrap();

        // At index 5, lowest is 11 at index 5 (0 periods ago)
        // Aroon Down = ((5 - 0) / 5) * 100 = 100
        assert!((result.aroon_down[5] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_aroon_into() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let mut aroon_up = vec![0.0_f64; 6];
        let mut aroon_down = vec![0.0_f64; 6];

        aroon_into(&high, &low, 5, &mut aroon_up, &mut aroon_down).unwrap();

        assert!((aroon_up[5] - 100.0).abs() < 1e-10);
        assert!((aroon_down[5] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_aroon_into_buffer_too_small() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let mut aroon_up = vec![0.0_f64; 3]; // Too small
        let mut aroon_down = vec![0.0_f64; 6];

        let result = aroon_into(&high, &low, 5, &mut aroon_up, &mut aroon_down);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_aroon_f32() {
        let high: Vec<f32> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f32> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = aroon(&high, &low, 5).unwrap();

        assert!((result.aroon_up[5] - 100.0_f32).abs() < 1e-5);
        assert!((result.aroon_down[5] - 0.0_f32).abs() < 1e-5);
    }

    #[test]
    fn test_aroon_period_1() {
        let high: Vec<f64> = vec![10.0, 11.0, 10.0, 12.0, 11.0];
        let low: Vec<f64> = vec![9.0, 10.0, 9.0, 11.0, 10.0];
        let result = aroon(&high, &low, 1).unwrap();

        // With period 1, we compare current bar to previous bar
        // At index 1: window is [10.0, 11.0] for highs
        // Highest is at index 1 (0 periods ago) -> Aroon Up = 100
        assert!((result.aroon_up[1] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_aroon_multiple_same_values() {
        // When there are multiple equal highs/lows, should use the most recent
        let high: Vec<f64> = vec![10.0, 15.0, 15.0, 15.0, 15.0, 15.0];
        let low: Vec<f64> = vec![9.0, 14.0, 14.0, 14.0, 14.0, 14.0];
        let result = aroon(&high, &low, 5).unwrap();

        // At index 5, there are multiple 15s. Most recent is at index 5.
        // Aroon Up = ((5 - 0) / 5) * 100 = 100
        assert!((result.aroon_up[5] - 100.0).abs() < 1e-10);
    }

    // AROONOSC tests
    #[test]
    fn test_aroonosc_lookback() {
        assert_eq!(aroonosc_lookback(5), 5);
        assert_eq!(aroonosc_lookback(14), 14);
    }

    #[test]
    fn test_aroonosc_min_len() {
        assert_eq!(aroonosc_min_len(5), 6);
        assert_eq!(aroonosc_min_len(14), 15);
    }

    #[test]
    fn test_aroonosc_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let result = aroonosc(&high, &low, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_aroonosc_invalid_period() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = aroonosc(&high, &low, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_aroonosc_output_length() {
        let high: Vec<f64> = vec![
            10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
        ];
        let low: Vec<f64> = vec![
            9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0,
        ];
        let result = aroonosc(&high, &low, 5).unwrap();
        assert_eq!(result.len(), high.len());
    }

    #[test]
    fn test_aroonosc_lookback_nan() {
        let high: Vec<f64> = vec![
            10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
        ];
        let low: Vec<f64> = vec![
            9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0,
        ];
        let result = aroonosc(&high, &low, 5).unwrap();

        // First 5 values should be NaN
        for i in 0..5 {
            assert!(result[i].is_nan(), "aroonosc[{}] should be NaN", i);
        }
    }

    #[test]
    fn test_aroonosc_range() {
        let high: Vec<f64> = vec![
            10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
        ];
        let low: Vec<f64> = vec![
            9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0,
        ];
        let result = aroonosc(&high, &low, 5).unwrap();

        for i in 5..result.len() {
            assert!(
                result[i] >= -100.0 && result[i] <= 100.0,
                "aroonosc[{}] = {} should be in [-100, 100]",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_aroonosc_strong_uptrend() {
        // Monotonically increasing - Aroon Up = 100, Aroon Down = 0
        // AROONOSC = 100 - 0 = 100
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = aroonosc(&high, &low, 5).unwrap();

        assert!(
            (result[5] - 100.0).abs() < 1e-10,
            "aroonosc should be 100 in uptrend"
        );
    }

    #[test]
    fn test_aroonosc_strong_downtrend() {
        // Monotonically decreasing - Aroon Up = 0, Aroon Down = 100
        // AROONOSC = 0 - 100 = -100
        let high: Vec<f64> = vec![20.0, 19.0, 18.0, 17.0, 16.0, 15.0];
        let low: Vec<f64> = vec![19.0, 18.0, 17.0, 16.0, 15.0, 14.0];
        let result = aroonosc(&high, &low, 5).unwrap();

        assert!(
            (result[5] - (-100.0)).abs() < 1e-10,
            "aroonosc should be -100 in downtrend"
        );
    }

    #[test]
    fn test_aroonosc_calculation() {
        // Use same data as test_aroon_calculation
        let high: Vec<f64> = vec![10.0, 15.0, 12.0, 13.0, 11.0, 14.0];
        let low: Vec<f64> = vec![8.0, 13.0, 10.0, 11.0, 9.0, 12.0];
        let result = aroonosc(&high, &low, 5).unwrap();

        // Aroon Up = 20, Aroon Down = 0
        // AROONOSC = 20 - 0 = 20
        assert!(
            (result[5] - 20.0).abs() < 1e-10,
            "aroonosc[5] = {}",
            result[5]
        );
    }

    #[test]
    fn test_aroonosc_into() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let mut output = vec![0.0_f64; 6];

        aroonosc_into(&high, &low, 5, &mut output).unwrap();

        assert!((output[5] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_aroonosc_into_buffer_too_small() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let mut output = vec![0.0_f64; 3]; // Too small

        let result = aroonosc_into(&high, &low, 5, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_aroonosc_f32() {
        let high: Vec<f32> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f32> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = aroonosc(&high, &low, 5).unwrap();

        assert!((result[5] - 100.0_f32).abs() < 1e-5);
    }
}
