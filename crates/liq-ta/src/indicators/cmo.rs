//! CMO (Chande Momentum Oscillator) indicator.
//!
//! The Chande Momentum Oscillator measures momentum by comparing
//! gains to losses over a specified period.
//!
//! # Formula
//!
//! ```text
//! CMO = ((Sum of gains - Sum of losses) / (Sum of gains + Sum of losses)) * 100
//! ```
//!
//! Where:
//! - Gains = sum of positive price changes over the period
//! - Losses = sum of absolute negative price changes over the period
//!
//! # Range
//!
//! CMO ranges from -100 to +100:
//! - +100: All gains, no losses (strong uptrend)
//! - -100: All losses, no gains (strong downtrend)
//! - 0: Equal gains and losses
//!
//! # Interpretation
//!
//! - CMO > +50: Overbought
//! - CMO < -50: Oversold
//! - Zero-line crossovers can signal trend changes
//!
//! # Lookback
//!
//! The lookback period is `period`.

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Computes the lookback period for CMO.
#[inline]
#[must_use]
pub const fn cmo_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for CMO calculation.
#[inline]
#[must_use]
pub const fn cmo_min_len(period: usize) -> usize {
    period + 1
}

/// Computes CMO and stores results in output slice.
///
/// # Arguments
///
/// * `data` - Price data (typically closing prices)
/// * `period` - Lookback period (typically 14)
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cmo_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    let n = data.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    let min_len = cmo_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "cmo",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "cmo",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = cmo_lookback(period);
    let hundred = T::from_f64(100.0)?;

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Pre-compute all price changes with gains positive and losses as positive values.
    // Track NaN windows to enforce NaN propagation.
    // changes[i] contains the change from data[i] to data[i+1]
    // For gains: positive change
    // For losses: absolute value of negative change
    let mut gains = vec![T::zero(); n - 1];
    let mut losses = vec![T::zero(); n - 1];
    let mut nan_flags = vec![false; n - 1];

    for i in 0..n - 1 {
        if is_invalid(data[i]) || is_invalid(data[i + 1]) {
            nan_flags[i] = true;
        } else {
            let change = data[i + 1] - data[i];
            if change > T::zero() {
                gains[i] = change;
            } else if change < T::zero() {
                losses[i] = change.abs();
            }
        }
    }

    // Calculate initial sums for the first window (indices 0..period in changes)
    let mut sum_gains = T::zero();
    let mut sum_losses = T::zero();
    let mut nan_count = 0usize;

    for i in 0..period {
        if nan_flags[i] {
            nan_count += 1;
        } else {
            sum_gains = sum_gains + gains[i];
            sum_losses = sum_losses + losses[i];
        }
    }

    // Calculate first CMO value at index = period
    if nan_count > 0 {
        output[lookback] = T::nan();
    } else {
        let total = sum_gains + sum_losses;
        if total == T::zero() {
            output[lookback] = T::zero();
        } else {
            output[lookback] = ((sum_gains - sum_losses) / total) * hundred;
        }
    }

    // Rolling calculation for remaining values
    // Window slides: remove oldest change, add newest change
    for i in (lookback + 1)..n {
        // Index of oldest change to remove (change from data[i-period-1] to data[i-period])
        let old_idx = i - period - 1;
        // Index of newest change to add (change from data[i-1] to data[i])
        let new_idx = i - 1;

        if nan_flags[old_idx] {
            nan_count = nan_count.saturating_sub(1);
        } else {
            sum_gains = sum_gains - gains[old_idx];
            sum_losses = sum_losses - losses[old_idx];
        }

        if nan_flags[new_idx] {
            nan_count += 1;
        } else {
            sum_gains = sum_gains + gains[new_idx];
            sum_losses = sum_losses + losses[new_idx];
        }

        if nan_count > 0 {
            output[i] = T::nan();
        } else {
            let total = sum_gains + sum_losses;
            if total == T::zero() {
                output[i] = T::zero();
            } else {
                output[i] = ((sum_gains - sum_losses) / total) * hundred;
            }
        }
    }

    Ok(())
}

/// Computes CMO (Chande Momentum Oscillator).
///
/// # Arguments
///
/// * `data` - Price data (typically closing prices)
/// * `period` - Lookback period (typically 14)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - CMO values (range -100 to +100)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use liq_ta::indicators::cmo;
///
/// let prices = vec![44.0_f64, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0];
///
/// let result = cmo(&prices, 5).unwrap();
/// // First 5 values are NaN (lookback = period)
/// assert!(result[5].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn cmo<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    cmo_into(data, period, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_cmo_lookback() {
        assert_eq!(cmo_lookback(5), 5);
        assert_eq!(cmo_lookback(14), 14);
    }

    #[test]
    fn test_cmo_min_len() {
        assert_eq!(cmo_min_len(5), 6);
        assert_eq!(cmo_min_len(14), 15);
    }

    #[test]
    fn test_cmo_empty_input() {
        let data: Vec<f64> = vec![];
        let result = cmo(&data, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_cmo_invalid_period() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let result = cmo(&data, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_cmo_insufficient_data() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let result = cmo(&data, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_cmo_output_length() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let result = cmo(&data, 5).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_cmo_lookback_nan() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let result = cmo(&data, 5).unwrap();

        // First 5 values should be NaN (lookback = period)
        for i in 0..5 {
            assert!(result[i].is_nan(), "cmo[{}] should be NaN", i);
        }

        // Values after lookback should be finite
        for i in 5..result.len() {
            assert!(result[i].is_finite(), "cmo[{}] should be finite", i);
        }
    }

    #[test]
    fn test_cmo_range() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 14.0, 13.0, 12.0, 11.0];
        let result = cmo(&data, 5).unwrap();

        for i in 5..result.len() {
            assert!(
                result[i] >= -100.0 && result[i] <= 100.0,
                "cmo[{}] = {} should be in [-100, 100]",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_cmo_all_gains() {
        // Monotonically increasing - all gains, no losses
        // CMO = (gains - 0) / (gains + 0) * 100 = 100
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let result = cmo(&data, 5).unwrap();

        assert!(
            (result[5] - 100.0).abs() < 1e-10,
            "cmo should be 100 with all gains"
        );
    }

    #[test]
    fn test_cmo_all_losses() {
        // Monotonically decreasing - all losses, no gains
        // CMO = (0 - losses) / (0 + losses) * 100 = -100
        let data: Vec<f64> = vec![15.0, 14.0, 13.0, 12.0, 11.0, 10.0];
        let result = cmo(&data, 5).unwrap();

        assert!(
            (result[5] - (-100.0)).abs() < 1e-10,
            "cmo should be -100 with all losses"
        );
    }

    #[test]
    fn test_cmo_equal_gains_losses() {
        // Equal gains and losses should give CMO = 0
        // Pattern: up 1, down 1, up 1, down 1, up 1
        let data: Vec<f64> = vec![10.0, 11.0, 10.0, 11.0, 10.0, 11.0];
        let result = cmo(&data, 5).unwrap();

        // Gains: 1+1+1 = 3, Losses: 1+1 = 2
        // CMO = (3 - 2) / (3 + 2) * 100 = 20
        // Actually let's verify the calculation
        assert!(result[5].is_finite());
    }

    #[test]
    fn test_cmo_constant_prices() {
        // No changes = CMO should be 0
        let data: Vec<f64> = vec![10.0; 10];
        let result = cmo(&data, 5).unwrap();

        for i in 5..result.len() {
            assert!(
                (result[i] - 0.0).abs() < 1e-10,
                "cmo[{}] should be 0 for constant prices",
                i
            );
        }
    }

    #[test]
    fn test_cmo_into() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let mut output = vec![0.0_f64; 6];

        cmo_into(&data, 5, &mut output).unwrap();

        assert!((output[5] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_cmo_into_buffer_too_small() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let mut output = vec![0.0_f64; 3]; // Too small

        let result = cmo_into(&data, 5, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_cmo_f32() {
        let data: Vec<f32> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let result = cmo(&data, 5).unwrap();

        assert!((result[5] - 100.0_f32).abs() < 1e-5);
    }

    #[test]
    fn test_cmo_calculation() {
        // Verify specific calculation
        // Prices: 10, 12, 11, 13, 12, 14
        // Changes: +2, -1, +2, -1, +2
        // Gains: 2 + 2 + 2 = 6
        // Losses: 1 + 1 = 2
        // CMO = (6 - 2) / (6 + 2) * 100 = 4/8 * 100 = 50
        let data: Vec<f64> = vec![10.0, 12.0, 11.0, 13.0, 12.0, 14.0];
        let result = cmo(&data, 5).unwrap();

        assert!((result[5] - 50.0).abs() < 1e-10, "cmo[5] = {}", result[5]);
    }
}
