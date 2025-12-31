//! `HT_TRENDLINE` (Hilbert Transform - Instantaneous Trendline) indicator.
//!
//! The Hilbert Transform Trendline uses signal processing techniques to
//! compute an adaptive trendline based on the dominant cycle period in the data.
//!
//! This implementation is based on John Ehlers' work on applying the Hilbert
//! Transform to financial market data.
//!
//! # Algorithm
//!
//! 1. Compute smoothed price using a weighted moving average
//! 2. Apply Hilbert Transform to extract in-phase (I) and quadrature (Q) components
//! 3. Estimate the dominant cycle period from the phase relationship
//! 4. Use the period to compute an adaptive smoothed trendline
//!
//! The trendline adapts its smoothing length based on the detected dominant cycle,
//! making it more responsive when cycles are short and smoother when cycles are long.
//!
//! # Interpretation
//!
//! - When price is above the trendline, the market is in an uptrend
//! - When price is below the trendline, the market is in a downtrend
//! - Crossovers can be used as potential entry/exit signals
//! - The trendline works best in trending markets; use `HT_TRENDMODE` to filter
//!
//! # Lookback
//!
//! The lookback period is 63 bars (warm-up period for the Hilbert Transform).
//!
//! # Performance
//!
//! This indicator uses the shared [`ht_core::hilbert_transform`] computation,
//! which has O(n) time complexity and allocates working arrays internally.

use super::ht_core::{hilbert_transform, ht_lookback, ht_min_len};
use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Returns the lookback period for `HT_TRENDLINE`.
///
/// The Hilbert Transform requires a warm-up period of 63 bars.
#[inline]
#[must_use]
pub const fn ht_trendline_lookback() -> usize {
    ht_lookback()
}

/// Returns the minimum input length required for `HT_TRENDLINE` calculation.
#[inline]
#[must_use]
pub const fn ht_trendline_min_len() -> usize {
    ht_min_len()
}

/// Computes Hilbert Transform Trendline and stores results in output.
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
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn ht_trendline_into<T: SeriesElement>(data: &[T], output: &mut [T]) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = data.len();
    let lookback = ht_trendline_lookback();
    let min_len = ht_trendline_min_len();

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "ht_trendline",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "ht_trendline",
            required: n,
            actual: output.len(),
        });
    }

    // Use shared Hilbert Transform computation
    let state = hilbert_transform(data)?;

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Copy trendline from shared HT computation
    for i in lookback..n {
        output[i] = state.trendline[i];
    }

    Ok(())
}

/// Computes Hilbert Transform Trendline.
///
/// # Arguments
///
/// * `data` - Input price data (typically close or HL2)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of trendline values
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::ht_trendline;
///
/// let mut prices: Vec<f64> = Vec::with_capacity(100);
/// for x in 1..=100 {
///     prices.push(50.0 + (x as f64 * 0.1).sin() * 10.0);
/// }
/// let result = ht_trendline(&prices).unwrap();
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
pub fn ht_trendline<T: SeriesElement>(data: &[T]) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    ht_trendline_into(data, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_ht_trendline_lookback() {
        assert_eq!(ht_trendline_lookback(), 63);
    }

    #[test]
    fn test_ht_trendline_min_len() {
        assert_eq!(ht_trendline_min_len(), 64);
    }

    #[test]
    fn test_ht_trendline_empty_input() {
        let data: Vec<f64> = vec![];
        let result = ht_trendline(&data);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_ht_trendline_insufficient_data() {
        let data: Vec<f64> = vec![1.0; 50];
        let result = ht_trendline(&data);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_ht_trendline_output_length() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_trendline(&data).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_ht_trendline_nan_count() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_trendline(&data).unwrap();

        let lookback = ht_trendline_lookback();
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, lookback);
    }

    #[test]
    fn test_ht_trendline_valid_values() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_trendline(&data).unwrap();

        let lookback = ht_trendline_lookback();
        for i in lookback..result.len() {
            assert!(result[i].is_finite(), "result[{}] should be finite", i);
        }
    }

    #[test]
    fn test_ht_trendline_trending_data() {
        // Linear uptrend
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_trendline(&data).unwrap();

        let lookback = ht_trendline_lookback();
        // Trendline should generally increase in uptrend
        let mut increasing_count = 0;
        for i in (lookback + 1)..result.len() {
            if result[i] > result[i - 1] {
                increasing_count += 1;
            }
        }
        // Most values should be increasing
        let total = result.len() - lookback - 1;
        assert!(
            increasing_count > total / 2,
            "Trendline should mostly increase in uptrend: {} of {} increasing",
            increasing_count,
            total
        );
    }

    #[test]
    fn test_ht_trendline_cyclic_data() {
        // Sinusoidal data to test cycle detection
        let data: Vec<f64> = (0..200)
            .map(|x| 50.0 + (x as f64 * 0.2).sin() * 10.0)
            .collect();
        let result = ht_trendline(&data).unwrap();

        let lookback = ht_trendline_lookback();
        for i in lookback..result.len() {
            assert!(result[i].is_finite());
            // Trendline should be within reasonable range of the data
            assert!(result[i] > 30.0 && result[i] < 70.0);
        }
    }

    #[test]
    fn test_ht_trendline_constant_data() {
        let data: Vec<f64> = vec![50.0; 100];
        let result = ht_trendline(&data).unwrap();

        let lookback = ht_trendline_lookback();
        for i in lookback..result.len() {
            assert!(result[i].is_finite());
            // For constant data, trendline should be close to the constant value
            assert!(
                (result[i] - 50.0).abs() < 1.0,
                "result[{}]={} should be close to 50.0",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_ht_trendline_into() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; data.len()];
        ht_trendline_into(&data, &mut output).unwrap();

        let lookback = ht_trendline_lookback();
        for i in 0..lookback {
            assert!(output[i].is_nan());
        }
        for i in lookback..output.len() {
            assert!(output[i].is_finite());
        }
    }

    #[test]
    fn test_ht_trendline_into_buffer_too_small() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 50]; // Too small
        let result = ht_trendline_into(&data, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_ht_trendline_f32() {
        let data: Vec<f32> = (1..=100).map(|x| x as f32).collect();
        let result = ht_trendline(&data).unwrap();

        assert_eq!(result.len(), data.len());
        let lookback = ht_trendline_lookback();
        for i in 0..lookback {
            assert!(result[i].is_nan());
        }
        for i in lookback..result.len() {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_ht_trendline_minimum_length() {
        let data: Vec<f64> = (1..=64).map(|x| x as f64).collect();
        let result = ht_trendline(&data).unwrap();

        assert_eq!(result.len(), 64);
        // First 63 should be NaN, last one valid
        for i in 0..63 {
            assert!(result[i].is_nan());
        }
        assert!(result[63].is_finite());
    }
}
