//! `HT_DCPERIOD` (Hilbert Transform - Dominant Cycle Period)
//!
//! This indicator uses the Hilbert Transform to compute the dominant cycle period
//! in the price data. The dominant cycle period is useful for adaptive indicators
//! that adjust their parameters based on market conditions.
//!
//! # Algorithm
//!
//! The dominant cycle period is extracted using the Hilbert Transform's homodyne
//! discriminator. The algorithm:
//!
//! 1. Applies a smoothing filter to reduce noise
//! 2. Uses the Hilbert Transform to generate in-phase (I) and quadrature (Q) components
//! 3. Correlates successive I/Q samples to detect the instantaneous frequency
//! 4. Converts frequency to period (Period = 2π / ω)
//! 5. Applies exponential smoothing to stabilize the estimate
//!
//! # Interpretation
//!
//! - **Period range**: Output is constrained to [6, 50] bars
//! - **Short periods** (6-15): Indicates fast market cycles, potentially choppy conditions
//! - **Long periods** (30-50): Indicates slower, more trending market behavior
//! - **Adaptive use**: Use the period to dynamically adjust other indicator lengths
//!
//! # Lookback
//!
//! The lookback period is 63 bars (warm-up period for the Hilbert Transform).
//!
//! # Performance
//!
//! Uses the shared [`crate::indicators::ht_core::hilbert_transform`] computation with O(n)
//! complexity.

use super::ht_core::{hilbert_transform, ht_lookback, ht_min_len};
use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Returns the lookback period for `HT_DCPERIOD`.
#[inline]
#[must_use]
pub const fn ht_dcperiod_lookback() -> usize {
    ht_lookback()
}

/// Returns the minimum input length required for `HT_DCPERIOD`.
#[inline]
#[must_use]
pub const fn ht_dcperiod_min_len() -> usize {
    ht_min_len()
}

/// Computes `HT_DCPERIOD` and stores results in output.
///
/// # Arguments
///
/// * `data` - Input price data (typically close or HL2)
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
pub fn ht_dcperiod_into<T: SeriesElement>(data: &[T], output: &mut [T]) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = data.len();
    let lookback = ht_dcperiod_lookback();
    let min_len = ht_dcperiod_min_len();

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "ht_dcperiod",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "ht_dcperiod",
            required: n,
            actual: output.len(),
        });
    }

    // Compute Hilbert Transform
    let state = hilbert_transform(data)?;

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Copy smooth_period to output
    for i in lookback..n {
        output[i] = state.smooth_period[i];
    }

    Ok(())
}

/// Computes `HT_DCPERIOD`.
///
/// # Arguments
///
/// * `data` - Input price data (typically close or HL2)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of dominant cycle period values
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use liq_ta::indicators::ht_dcperiod;
///
/// let mut prices: Vec<f64> = Vec::with_capacity(100);
/// for x in 1..=100 {
///     prices.push(50.0 + (x as f64 * 0.1).sin() * 10.0);
/// }
/// let result = ht_dcperiod(&prices).unwrap();
/// assert!(result[0].is_nan()); // First 63 values are NaN
/// assert!(result[63].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn ht_dcperiod<T: SeriesElement>(data: &[T]) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    ht_dcperiod_into(data, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ht_dcperiod_lookback() {
        assert_eq!(ht_dcperiod_lookback(), 63);
    }

    #[test]
    fn test_ht_dcperiod_min_len() {
        assert_eq!(ht_dcperiod_min_len(), 64);
    }

    #[test]
    fn test_ht_dcperiod_empty_input() {
        let data: Vec<f64> = vec![];
        let result = ht_dcperiod(&data);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_ht_dcperiod_insufficient_data() {
        let data: Vec<f64> = vec![1.0; 50];
        let result = ht_dcperiod(&data);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_ht_dcperiod_output_length() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_dcperiod(&data).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_ht_dcperiod_nan_count() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_dcperiod(&data).unwrap();

        let lookback = ht_dcperiod_lookback();
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, lookback);
    }

    #[test]
    fn test_ht_dcperiod_valid_range() {
        let data: Vec<f64> = (1..=200)
            .map(|x| 50.0 + (x as f64 * 0.1).sin() * 10.0)
            .collect();
        let result = ht_dcperiod(&data).unwrap();

        let lookback = ht_dcperiod_lookback();
        for i in lookback..result.len() {
            assert!(result[i].is_finite());
            // Period should be in valid range [6, 50]
            assert!(
                result[i] >= 6.0 && result[i] <= 50.0,
                "period at {} is {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_ht_dcperiod_into() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; data.len()];
        ht_dcperiod_into(&data, &mut output).unwrap();

        let lookback = ht_dcperiod_lookback();
        for i in 0..lookback {
            assert!(output[i].is_nan());
        }
        for i in lookback..output.len() {
            assert!(output[i].is_finite());
        }
    }

    #[test]
    fn test_ht_dcperiod_into_buffer_too_small() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 50];
        let result = ht_dcperiod_into(&data, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_ht_dcperiod_f32() {
        let data: Vec<f32> = (1..=100).map(|x| x as f32).collect();
        let result = ht_dcperiod(&data).unwrap();

        assert_eq!(result.len(), data.len());
        let lookback = ht_dcperiod_lookback();
        for i in lookback..result.len() {
            assert!(result[i].is_finite());
        }
    }
}
