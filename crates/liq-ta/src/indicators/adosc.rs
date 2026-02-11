//! Chaikin Accumulation/Distribution Oscillator (ADOSC)
//!
//! The Chaikin A/D Oscillator is the difference between a fast and slow EMA of the
//! Accumulation/Distribution Line. It is used to measure momentum in the A/D Line.
//!
//! # Formula
//!
//! ```text
//! AD = Accumulation/Distribution Line
//! ADOSC = EMA(AD, fast_period) - EMA(AD, slow_period)
//! ```
//!
//! # Default Parameters
//!
//! - Fast Period: 3
//! - Slow Period: 10
//!
//! # Example
//!
//! ```
//! use liq_ta::indicators::adosc;
//!
//! let high = [25.0_f64, 26.0, 25.5, 26.5, 27.0, 26.5, 27.5, 28.0, 27.5, 28.5, 29.0, 28.5];
//! let low = [24.0_f64, 24.5, 24.0, 25.0, 25.5, 25.0, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0];
//! let close = [24.5_f64, 25.5, 24.5, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0, 28.0, 28.5, 28.0];
//! let volume = [1000.0_f64, 1500.0, 1200.0, 1800.0, 2000.0, 1600.0, 2200.0, 2500.0, 2000.0, 2800.0, 3000.0, 2400.0];
//!
//! let result = adosc(&high, &low, &close, &volume, 3, 10).unwrap();
//! assert_eq!(result.len(), 12);
//! ```

use super::ad::ad_into;
use super::ema::{ema_into, ema_lookback};
use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Returns the lookback period for ADOSC.
///
/// The lookback is based on the slow period EMA.
#[inline]
#[must_use]
pub const fn adosc_lookback(slow_period: usize) -> usize {
    ema_lookback(slow_period)
}

/// Returns the minimum data length required for ADOSC.
#[inline]
#[must_use]
pub const fn adosc_min_len(slow_period: usize) -> usize {
    adosc_lookback(slow_period) + 1
}

/// Computes ADOSC (Chaikin A/D Oscillator) into a pre-allocated output buffer.
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `fast_period` - Fast EMA period (typically 3)
/// * `slow_period` - Slow EMA period (typically 10)
/// * `output` - Pre-allocated output buffer
///
/// # Errors
///
/// Returns an error if:
/// - Any input is empty
/// - Input lengths don't match
/// - Period is 0
/// - `fast_period` >= `slow_period`
/// - Output buffer is too small
pub fn adosc_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    fast_period: usize,
    slow_period: usize,
    output: &mut [T],
) -> Result<()> {
    let len = high.len();

    // Validate inputs
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if low.len() != len || close.len() != len || volume.len() != len {
        return Err(Error::LengthMismatch {
            description: format!(
                "high has {} elements, low has {}, close has {}, volume has {}",
                len,
                low.len(),
                close.len(),
                volume.len()
            ),
        });
    }
    if fast_period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "fast_period must be at least 1",
        });
    }
    if slow_period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "slow_period must be at least 1",
        });
    }
    if fast_period >= slow_period {
        return Err(Error::InvalidPeriod {
            period: fast_period,
            reason: "fast_period must be less than slow_period",
        });
    }

    let min_len = adosc_min_len(slow_period);
    if len < min_len {
        return Err(Error::InsufficientData {
            required: min_len,
            actual: len,
            indicator: "adosc",
        });
    }
    if output.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: output.len(),
            indicator: "adosc",
        });
    }

    // Compute AD
    let mut ad_values = vec![T::zero(); len];
    ad_into(high, low, close, volume, &mut ad_values)?;

    // Compute fast and slow EMAs of AD
    let mut fast_ema = vec![T::nan(); len];
    let mut slow_ema = vec![T::nan(); len];

    ema_into(&ad_values, fast_period, &mut fast_ema)?;
    ema_into(&ad_values, slow_period, &mut slow_ema)?;

    // ADOSC = fast_ema - slow_ema
    let lookback = adosc_lookback(slow_period);

    // Fill NaN for lookback period
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Compute oscillator values
    for i in lookback..len {
        output[i] = fast_ema[i] - slow_ema[i];
    }

    Ok(())
}

/// Computes ADOSC (Chaikin A/D Oscillator) and returns a newly allocated vector.
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `fast_period` - Fast EMA period (typically 3)
/// * `slow_period` - Slow EMA period (typically 10)
///
/// # Returns
///
/// A vector containing the ADOSC values.
///
/// # Errors
///
/// Returns an error if:
/// - Any input is empty
/// - Input lengths don't match
/// - Period is 0
/// - `fast_period` >= `slow_period`
///
/// # Example
///
/// ```
/// use liq_ta::indicators::adosc;
///
/// let high = [25.0_f64, 26.0, 25.5, 26.5, 27.0, 26.5, 27.5, 28.0, 27.5, 28.5, 29.0, 28.5];
/// let low = [24.0_f64, 24.5, 24.0, 25.0, 25.5, 25.0, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0];
/// let close = [24.5_f64, 25.5, 24.5, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0, 28.0, 28.5, 28.0];
/// let volume = [1000.0_f64, 1500.0, 1200.0, 1800.0, 2000.0, 1600.0, 2200.0, 2500.0, 2000.0, 2800.0, 3000.0, 2400.0];
///
/// let result = adosc(&high, &low, &close, &volume, 3, 10).unwrap();
/// assert_eq!(result.len(), 12);
/// ```
pub fn adosc<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    fast_period: usize,
    slow_period: usize,
) -> Result<Vec<T>> {
    let len = high.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }

    let mut output = vec![T::nan(); len];
    adosc_into(
        high,
        low,
        close,
        volume,
        fast_period,
        slow_period,
        &mut output,
    )?;
    Ok(output)
}

/// Computes ADOSC with default parameters (fast=3, slow=10).
///
/// # Example
///
/// ```
/// use liq_ta::indicators::adosc_default;
///
/// let high = [25.0_f64, 26.0, 25.5, 26.5, 27.0, 26.5, 27.5, 28.0, 27.5, 28.5, 29.0, 28.5];
/// let low = [24.0_f64, 24.5, 24.0, 25.0, 25.5, 25.0, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0];
/// let close = [24.5_f64, 25.5, 24.5, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0, 28.0, 28.5, 28.0];
/// let volume = [1000.0_f64, 1500.0, 1200.0, 1800.0, 2000.0, 1600.0, 2200.0, 2500.0, 2000.0, 2800.0, 3000.0, 2400.0];
///
/// let result = adosc_default(&high, &low, &close, &volume).unwrap();
/// assert_eq!(result.len(), 12);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn adosc_default<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
) -> Result<Vec<T>> {
    adosc(high, low, close, volume, 3, 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        (a - b).abs() < tol
    }

    #[test]
    fn test_adosc_lookback() {
        assert_eq!(adosc_lookback(10), 9); // EMA lookback = period - 1
    }

    #[test]
    fn test_adosc_min_len() {
        assert_eq!(adosc_min_len(10), 10); // lookback + 1
    }

    #[test]
    fn test_adosc_empty_input() {
        let high: [f64; 0] = [];
        let low: [f64; 0] = [];
        let close: [f64; 0] = [];
        let volume: [f64; 0] = [];
        let result = adosc(&high, &low, &close, &volume, 3, 10);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_adosc_length_mismatch() {
        let high = [25.0_f64, 26.0];
        let low = [24.0_f64];
        let close = [24.5_f64, 25.5];
        let volume = [1000.0_f64, 1500.0];
        let result = adosc(&high, &low, &close, &volume, 3, 10);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_adosc_invalid_fast_period() {
        let high = [25.0_f64; 20];
        let low = [24.0_f64; 20];
        let close = [24.5_f64; 20];
        let volume = [1000.0_f64; 20];
        let result = adosc(&high, &low, &close, &volume, 0, 10);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_adosc_invalid_slow_period() {
        let high = [25.0_f64; 20];
        let low = [24.0_f64; 20];
        let close = [24.5_f64; 20];
        let volume = [1000.0_f64; 20];
        let result = adosc(&high, &low, &close, &volume, 3, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_adosc_fast_ge_slow() {
        let high = [25.0_f64; 20];
        let low = [24.0_f64; 20];
        let close = [24.5_f64; 20];
        let volume = [1000.0_f64; 20];
        let result = adosc(&high, &low, &close, &volume, 10, 3);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));

        // Also test equal periods
        let result2 = adosc(&high, &low, &close, &volume, 5, 5);
        assert!(matches!(result2, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_adosc_insufficient_data() {
        let high = [25.0_f64; 5];
        let low = [24.0_f64; 5];
        let close = [24.5_f64; 5];
        let volume = [1000.0_f64; 5];
        let result = adosc(&high, &low, &close, &volume, 3, 10);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_adosc_basic() {
        // Create test data with trending AD
        let high = [
            25.0_f64, 26.0, 25.5, 26.5, 27.0, 26.5, 27.5, 28.0, 27.5, 28.5, 29.0, 28.5,
        ];
        let low = [
            24.0_f64, 24.5, 24.0, 25.0, 25.5, 25.0, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0,
        ];
        let close = [
            24.5_f64, 25.5, 24.5, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0, 28.0, 28.5, 28.0,
        ];
        let volume = [
            1000.0_f64, 1500.0, 1200.0, 1800.0, 2000.0, 1600.0, 2200.0, 2500.0, 2000.0, 2800.0,
            3000.0, 2400.0,
        ];

        let result = adosc(&high, &low, &close, &volume, 3, 10).unwrap();
        assert_eq!(result.len(), 12);

        // First 9 values (lookback for slow_period=10) should be NaN
        for i in 0..9 {
            assert!(result[i].is_nan(), "Expected NaN at index {i}");
        }

        // Values after lookback should be valid
        for i in 9..12 {
            assert!(!result[i].is_nan(), "Expected valid value at index {i}");
        }
    }

    #[test]
    fn test_adosc_default() {
        let high = [
            25.0_f64, 26.0, 25.5, 26.5, 27.0, 26.5, 27.5, 28.0, 27.5, 28.5, 29.0, 28.5,
        ];
        let low = [
            24.0_f64, 24.5, 24.0, 25.0, 25.5, 25.0, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0,
        ];
        let close = [
            24.5_f64, 25.5, 24.5, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0, 28.0, 28.5, 28.0,
        ];
        let volume = [
            1000.0_f64, 1500.0, 1200.0, 1800.0, 2000.0, 1600.0, 2200.0, 2500.0, 2000.0, 2800.0,
            3000.0, 2400.0,
        ];

        let result = adosc_default(&high, &low, &close, &volume).unwrap();
        let result2 = adosc(&high, &low, &close, &volume, 3, 10).unwrap();

        // Results should match
        for i in 0..result.len() {
            assert!(approx_eq(result[i], result2[i], 1e-10));
        }
    }

    #[test]
    fn test_adosc_into_buffer_too_small() {
        let high = [25.0_f64; 15];
        let low = [24.0_f64; 15];
        let close = [24.5_f64; 15];
        let volume = [1000.0_f64; 15];
        let mut output = [0.0_f64; 5];

        let result = adosc_into(&high, &low, &close, &volume, 3, 10, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_adosc_into_success() {
        let high = [
            25.0_f64, 26.0, 25.5, 26.5, 27.0, 26.5, 27.5, 28.0, 27.5, 28.5, 29.0, 28.5,
        ];
        let low = [
            24.0_f64, 24.5, 24.0, 25.0, 25.5, 25.0, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0,
        ];
        let close = [
            24.5_f64, 25.5, 24.5, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0, 28.0, 28.5, 28.0,
        ];
        let volume = [
            1000.0_f64, 1500.0, 1200.0, 1800.0, 2000.0, 1600.0, 2200.0, 2500.0, 2000.0, 2800.0,
            3000.0, 2400.0,
        ];
        let mut output = [0.0_f64; 12];

        adosc_into(&high, &low, &close, &volume, 3, 10, &mut output).unwrap();

        // Check that we get expected NaN pattern
        for i in 0..9 {
            assert!(output[i].is_nan());
        }
        for i in 9..12 {
            assert!(!output[i].is_nan());
        }
    }

    #[test]
    fn test_adosc_f32() {
        let high = [
            25.0_f32, 26.0, 25.5, 26.5, 27.0, 26.5, 27.5, 28.0, 27.5, 28.5, 29.0, 28.5,
        ];
        let low = [
            24.0_f32, 24.5, 24.0, 25.0, 25.5, 25.0, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0,
        ];
        let close = [
            24.5_f32, 25.5, 24.5, 26.0, 26.5, 26.0, 27.0, 27.5, 27.0, 28.0, 28.5, 28.0,
        ];
        let volume = [
            1000.0_f32, 1500.0, 1200.0, 1800.0, 2000.0, 1600.0, 2200.0, 2500.0, 2000.0, 2800.0,
            3000.0, 2400.0,
        ];

        let result = adosc(&high, &low, &close, &volume, 3, 10).unwrap();
        assert_eq!(result.len(), 12);
    }

    #[test]
    fn test_adosc_nan_lookback_count() {
        let high = [25.0_f64; 20];
        let low = [24.0_f64; 20];
        let close = [24.5_f64; 20];
        let volume = [1000.0_f64; 20];

        let result = adosc(&high, &low, &close, &volume, 3, 10).unwrap();

        let nan_count: usize = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, adosc_lookback(10));
    }

    #[test]
    fn test_adosc_output_length() {
        let high = [25.0_f64; 100];
        let low = [24.0_f64; 100];
        let close = [24.5_f64; 100];
        let volume = [1000.0_f64; 100];

        let result = adosc(&high, &low, &close, &volume, 3, 10).unwrap();
        assert_eq!(result.len(), 100);
    }
}
