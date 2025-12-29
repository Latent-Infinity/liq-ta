//! MFI (Money Flow Index) indicator.
//!
//! The Money Flow Index is a volume-weighted version of RSI that measures
//! buying and selling pressure using both price and volume.
//!
//! # Formula
//!
//! ```text
//! Typical Price = (High + Low + Close) / 3
//! Raw Money Flow = Typical Price * Volume
//! Money Flow Ratio = Positive Money Flow / Negative Money Flow
//! MFI = 100 - (100 / (1 + Money Flow Ratio))
//! ```
//!
//! Where:
//! - Positive Money Flow = sum of Raw MF when TP > previous TP
//! - Negative Money Flow = sum of Raw MF when TP < previous TP
//!
//! # Range
//!
//! MFI ranges from 0 to 100:
//! - > 80: Overbought
//! - < 20: Oversold
//!
//! # Lookback
//!
//! The lookback period is `period`.
//!
//! # NaN Handling
//!
//! Per indicator-standards.md, any NaN or Inf within the rolling window yields
//! NaN output at that position. MFI is a rolling window indicator, so once the
//! NaN value exits the window, subsequent outputs recover to valid values.
//!
//! The window includes:
//! - The current `period` bars for money flow calculation
//! - The bar immediately before the window (for price comparison)

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Computes the lookback period for MFI.
#[inline]
#[must_use]
pub const fn mfi_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for MFI calculation.
#[inline]
#[must_use]
pub const fn mfi_min_len(period: usize) -> usize {
    period + 1
}

/// Computes MFI and stores results in output slice.
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `period` - Lookback period (typically 14)
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
pub fn mfi_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    if low.len() != n || close.len() != n || volume.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "Arrays must have same length: high={}, low={}, close={}, volume={}",
                n,
                low.len(),
                close.len(),
                volume.len()
            ),
        });
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    let min_len = mfi_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "mfi",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "mfi",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = mfi_lookback(period);
    let three = T::from_f64(3.0)?;
    let hundred = T::from_f64(100.0)?;
    let one = T::from_f64(1.0)?;

    // Calculate typical prices and raw money flow
    // IEEE 754: NaN in any of high/low/close propagates to tp[i]
    let mut tp = vec![T::zero(); n];
    let mut raw_mf = vec![T::zero(); n];
    for i in 0..n {
        tp[i] = (high[i] + low[i] + close[i]) / three;
        raw_mf[i] = tp[i] * volume[i];
    }

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate MFI for each bar after lookback
    // Per indicator-standards.md: any NaN within a rolling window yields NaN output
    for i in lookback..n {
        let start = i - period + 1;

        // Check for NaN/Inf in the window (including comparison element at start-1)
        // We need tp[start-1] for the first comparison, so check from start-1 to i
        let mut has_nan = !tp[start - 1].is_finite() || !raw_mf[start - 1].is_finite();
        if !has_nan {
            for j in start..=i {
                if !tp[j].is_finite() || !raw_mf[j].is_finite() {
                    has_nan = true;
                    break;
                }
            }
        }

        if has_nan {
            output[i] = T::nan();
            continue;
        }

        let mut positive_mf = T::zero();
        let mut negative_mf = T::zero();

        for j in start..=i {
            if tp[j] > tp[j - 1] {
                positive_mf = positive_mf + raw_mf[j];
            } else if tp[j] < tp[j - 1] {
                negative_mf = negative_mf + raw_mf[j];
            }
            // If TP unchanged, money flow is neither positive nor negative
        }

        if negative_mf == T::zero() {
            // All positive or no flow - MFI = 100
            output[i] = hundred;
        } else if positive_mf == T::zero() {
            // All negative - MFI = 0
            output[i] = T::zero();
        } else {
            let mfr = positive_mf / negative_mf;
            output[i] = hundred - (hundred / (one + mfr));
        }
    }

    Ok(())
}

/// Computes MFI (Money Flow Index).
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `period` - Lookback period (typically 14)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - MFI values (range 0 to 100)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::mfi;
///
/// let high = vec![25.0_f64, 26.0, 27.0, 28.0, 27.5, 27.0];
/// let low = vec![23.0_f64, 24.0, 25.0, 26.0, 25.5, 25.0];
/// let close = vec![24.0_f64, 25.0, 26.0, 27.0, 26.5, 26.0];
/// let volume = vec![1000.0_f64, 1100.0, 1200.0, 1300.0, 1400.0, 1500.0];
///
/// let result = mfi(&high, &low, &close, &volume, 3).unwrap();
/// assert!(result[3].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn mfi<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    period: usize,
) -> Result<Vec<T>> {
    let mut output = vec![T::zero(); high.len()];
    mfi_into(high, low, close, volume, period, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_mfi_lookback() {
        assert_eq!(mfi_lookback(5), 5);
        assert_eq!(mfi_lookback(14), 14);
    }

    #[test]
    fn test_mfi_min_len() {
        assert_eq!(mfi_min_len(5), 6);
        assert_eq!(mfi_min_len(14), 15);
    }

    #[test]
    fn test_mfi_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];
        let volume: Vec<f64> = vec![];
        let result = mfi(&high, &low, &close, &volume, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_mfi_invalid_period() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_mfi_insufficient_data() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let volume: Vec<f64> = vec![1000.0; 5];
        let result = mfi(&high, &low, &close, &volume, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_mfi_length_mismatch() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 5);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_mfi_output_length() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume: Vec<f64> = vec![1000.0; 10];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();
        assert_eq!(result.len(), high.len());
    }

    #[test]
    fn test_mfi_lookback_nan() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume: Vec<f64> = vec![1000.0; 10];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        // First 5 values should be NaN (lookback = period)
        for i in 0..5 {
            assert!(result[i].is_nan(), "mfi[{}] should be NaN", i);
        }

        // Values after lookback should be finite
        for i in 5..result.len() {
            assert!(result[i].is_finite(), "mfi[{}] should be finite", i);
        }
    }

    #[test]
    fn test_mfi_range() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 14.0, 13.0, 12.0, 11.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 13.0, 12.0, 11.0, 10.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 13.5, 12.5, 11.5, 10.5];
        let volume: Vec<f64> = vec![1000.0; 10];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        for i in 5..result.len() {
            assert!(
                result[i] >= 0.0 && result[i] <= 100.0,
                "mfi[{}] = {} should be in [0, 100]",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_mfi_all_positive() {
        // All prices increasing - all positive money flow
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        // With all positive flow, MFI = 100
        assert!(
            (result[5] - 100.0).abs() < 1e-10,
            "mfi should be 100 with all positive flow"
        );
    }

    #[test]
    fn test_mfi_all_negative() {
        // All prices decreasing - all negative money flow
        let high: Vec<f64> = vec![15.0, 14.0, 13.0, 12.0, 11.0, 10.0];
        let low: Vec<f64> = vec![14.0, 13.0, 12.0, 11.0, 10.0, 9.0];
        let close: Vec<f64> = vec![14.5, 13.5, 12.5, 11.5, 10.5, 9.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        // With all negative flow, MFI = 0
        assert!(
            (result[5] - 0.0).abs() < 1e-10,
            "mfi should be 0 with all negative flow"
        );
    }

    #[test]
    fn test_mfi_into() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let mut output = vec![0.0_f64; 6];

        mfi_into(&high, &low, &close, &volume, 5, &mut output).unwrap();

        assert!((output[5] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_mfi_into_buffer_too_small() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let mut output = vec![0.0_f64; 3]; // Too small

        let result = mfi_into(&high, &low, &close, &volume, 5, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_mfi_f32() {
        let high: Vec<f32> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f32> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f32> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f32> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        assert!((result[5] - 100.0_f32).abs() < 1e-5);
    }

    #[test]
    fn test_mfi_varying_volume() {
        // Test with varying volumes to ensure volume weighting works
        let high: Vec<f64> = vec![10.0, 11.0, 10.0, 11.0, 10.0, 11.0];
        let low: Vec<f64> = vec![9.0, 10.0, 9.0, 10.0, 9.0, 10.0];
        let close: Vec<f64> = vec![9.5, 10.5, 9.5, 10.5, 9.5, 10.5];
        // Higher volume on up days should give higher MFI
        let volume: Vec<f64> = vec![1000.0, 2000.0, 1000.0, 2000.0, 1000.0, 2000.0];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        // With 2x volume on up days vs down days, positive MF > negative MF
        assert!(
            result[5] > 50.0,
            "mfi should be > 50 with higher volume on up days"
        );
    }

    // NaN handling tests - per indicator-standards.md:
    // "any NaN within a rolling window yields NaN output at that position"

    #[test]
    fn test_mfi_nan_in_high_propagates() {
        // NaN in high should propagate to output for affected window positions
        let high: Vec<f64> = vec![10.0, 11.0, f64::NAN, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume: Vec<f64> = vec![1000.0; 10];
        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // NaN at index 2 affects output at indices 3, 4, 5 (window includes index 2)
        // Also affects index 2 comparisons for index 3
        assert!(result[3].is_nan(), "mfi[3] should be NaN");
        assert!(result[4].is_nan(), "mfi[4] should be NaN");
        assert!(result[5].is_nan(), "mfi[5] should be NaN");

        // Index 6 is outside the window of NaN at 2, should be finite
        assert!(result[6].is_finite(), "mfi[6] should be finite");
    }

    #[test]
    fn test_mfi_nan_in_low_propagates() {
        // NaN in low should propagate
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, f64::NAN, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // NaN at index 3 affects outputs starting at index 3
        assert!(result[3].is_nan(), "mfi[3] should be NaN");
        assert!(result[4].is_nan(), "mfi[4] should be NaN");
        assert!(result[5].is_nan(), "mfi[5] should be NaN");
    }

    #[test]
    fn test_mfi_nan_in_close_propagates() {
        // NaN in close should propagate
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, f64::NAN, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // NaN at index 4 affects output at index 4 and beyond within window
        assert!(result[4].is_nan(), "mfi[4] should be NaN");
        assert!(result[5].is_nan(), "mfi[5] should be NaN");
    }

    #[test]
    fn test_mfi_nan_in_volume_propagates() {
        // NaN in volume should propagate
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, f64::NAN];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0, 1000.0, 1000.0, 1000.0, 1000.0, f64::NAN];
        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // NaN at index 5 affects output at index 5
        assert!(result[5].is_nan(), "mfi[5] should be NaN");
    }

    #[test]
    fn test_mfi_inf_in_high_propagates() {
        // Inf should also propagate like NaN
        let high: Vec<f64> = vec![10.0, 11.0, f64::INFINITY, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // Inf at index 2 affects output at indices 3, 4, 5
        assert!(result[3].is_nan(), "mfi[3] should be NaN due to Inf in window");
        assert!(result[4].is_nan(), "mfi[4] should be NaN due to Inf in window");
        assert!(result[5].is_nan(), "mfi[5] should be NaN due to Inf in window");
    }

    #[test]
    fn test_mfi_recovery_after_nan() {
        // Once NaN exits the window, output should recover
        let mut high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        high[1] = f64::NAN;
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume: Vec<f64> = vec![1000.0; 10];
        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // NaN at index 1 affects indices 2, 3, 4 (window of size 3)
        assert!(result[2].is_nan(), "mfi[2] should be NaN");
        assert!(result[3].is_nan(), "mfi[3] should be NaN");
        assert!(result[4].is_nan(), "mfi[4] should be NaN");

        // Index 5 is outside the window, should be finite
        assert!(result[5].is_finite(), "mfi[5] should be finite after NaN exits window");
    }

    #[test]
    fn test_mfi_comparison_nan_in_first_element() {
        // NaN in first element should affect comparisons
        let high: Vec<f64> = vec![f64::NAN, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // NaN at index 0 (used for comparison at index 1) affects indices 1, 2, 3
        assert!(result[1].is_nan(), "mfi[1] should be NaN due to NaN in comparison element");
        assert!(result[2].is_nan(), "mfi[2] should be NaN");
        assert!(result[3].is_nan(), "mfi[3] should be NaN");

        // Index 4 is outside window
        assert!(result[4].is_finite(), "mfi[4] should be finite");
    }
}