//! `HT_SINE` (Hilbert Transform - Sine Wave)
//!
//! This indicator uses the Hilbert Transform to compute sine wave values
//! based on the dominant cycle phase. Outputs both sine and lead_sine
//! (45 degrees ahead).
//!
//! # Algorithm
//!
//! Once the instantaneous phase is computed by the Hilbert Transform, sine waves
//! are generated:
//!
//! ```text
//! Sine = sin(Phase)
//! Lead Sine = sin(Phase + 45°)
//! ```
//!
//! The lead sine is 45 degrees (1/8 of a cycle) ahead, providing an early warning
//! of phase changes.
//!
//! # Interpretation
//!
//! - **Crossover signals**: When sine crosses lead_sine from below, it may indicate
//!   a cycle bottom; crossing from above may indicate a cycle top
//! - **Value range**: Both outputs are bounded to [-1, +1]
//! - **Cycle mode**: Crossovers are most reliable when `HT_TRENDMODE` = 0 (cycling)
//! - **Trending markets**: Crossovers may generate false signals in strong trends
//!
//! # Trading Strategy
//!
//! A common approach:
//! 1. Use `HT_TRENDMODE` to filter for cycling markets
//! 2. Buy when sine crosses above lead_sine
//! 3. Sell when sine crosses below lead_sine
//!
//! # Lookback
//!
//! The lookback period is 63 bars (warm-up period for the Hilbert Transform).
//!
//! # Performance
//!
//! Uses the shared [`ht_core::hilbert_transform`] computation with O(n) complexity.

use super::ht_core::{hilbert_transform, ht_lookback, ht_min_len};
use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Output structure for `HT_SINE` containing sine and `lead_sine`.
#[derive(Debug, Clone)]
pub struct HtSineOutput<T> {
    /// Sine wave
    pub sine: Vec<T>,
    /// Lead sine wave (45 degrees ahead)
    pub lead_sine: Vec<T>,
}

/// Returns the lookback period for `HT_SINE`.
#[inline]
#[must_use]
pub const fn ht_sine_lookback() -> usize {
    ht_lookback()
}

/// Returns the minimum input length required for `HT_SINE`.
#[inline]
#[must_use]
pub const fn ht_sine_min_len() -> usize {
    ht_min_len()
}

/// Computes `HT_SINE` and stores results in output buffers.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn ht_sine_into<T: SeriesElement>(
    data: &[T],
    sine_out: &mut [T],
    lead_sine_out: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = data.len();
    let lookback = ht_sine_lookback();
    let min_len = ht_sine_min_len();

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "ht_sine",
            required: min_len,
            actual: n,
        });
    }

    if sine_out.len() < n || lead_sine_out.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "ht_sine",
            required: n,
            actual: sine_out.len().min(lead_sine_out.len()),
        });
    }

    let state = hilbert_transform(data)?;

    for i in 0..lookback {
        sine_out[i] = T::nan();
        lead_sine_out[i] = T::nan();
    }

    for i in lookback..n {
        sine_out[i] = state.sine[i];
        lead_sine_out[i] = state.lead_sine[i];
    }

    Ok(())
}

/// Computes `HT_SINE`.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn ht_sine<T: SeriesElement>(data: &[T]) -> Result<HtSineOutput<T>> {
    let n = data.len();
    if n == 0 {
        return Err(Error::EmptyInput);
    }

    let min_len = ht_sine_min_len();
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "ht_sine",
            required: min_len,
            actual: n,
        });
    }

    let state = hilbert_transform(data)?;
    let lookback = ht_sine_lookback();

    let mut sine = vec![T::nan(); n];
    let mut lead_sine = vec![T::nan(); n];

    for i in lookback..n {
        sine[i] = state.sine[i];
        lead_sine[i] = state.lead_sine[i];
    }

    Ok(HtSineOutput { sine, lead_sine })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ht_sine_lookback() {
        assert_eq!(ht_sine_lookback(), 63);
    }

    #[test]
    fn test_ht_sine_min_len() {
        assert_eq!(ht_sine_min_len(), 64);
    }

    #[test]
    fn test_ht_sine_empty_input() {
        let data: Vec<f64> = vec![];
        let result = ht_sine(&data);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_ht_sine_insufficient_data() {
        let data: Vec<f64> = vec![1.0; 50];
        let result = ht_sine(&data);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_ht_sine_output_length() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_sine(&data).unwrap();
        assert_eq!(result.sine.len(), data.len());
        assert_eq!(result.lead_sine.len(), data.len());
    }

    #[test]
    fn test_ht_sine_nan_count() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_sine(&data).unwrap();

        let lookback = ht_sine_lookback();
        let nan_count_s = result.sine.iter().filter(|x| x.is_nan()).count();
        let nan_count_l = result.lead_sine.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count_s, lookback);
        assert_eq!(nan_count_l, lookback);
    }

    #[test]
    fn test_ht_sine_value_range() {
        let data: Vec<f64> = (1..=200)
            .map(|x| 50.0 + (x as f64 * 0.1).sin() * 10.0)
            .collect();
        let result = ht_sine(&data).unwrap();

        let lookback = ht_sine_lookback();
        for i in lookback..result.sine.len() {
            if !result.sine[i].is_nan() {
                // Sine values should be in [-1, 1]
                assert!(
                    result.sine[i] >= -1.0 && result.sine[i] <= 1.0,
                    "sine at {} is {}",
                    i,
                    result.sine[i]
                );
                assert!(
                    result.lead_sine[i] >= -1.0 && result.lead_sine[i] <= 1.0,
                    "lead_sine at {} is {}",
                    i,
                    result.lead_sine[i]
                );
            }
        }
    }

    #[test]
    fn test_ht_sine_into_buffer_too_small() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let mut sine = vec![0.0_f64; 50];
        let mut lead_sine = vec![0.0_f64; 100];
        let result = ht_sine_into(&data, &mut sine, &mut lead_sine);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_ht_sine_f32() {
        let data: Vec<f32> = (1..=100).map(|x| x as f32).collect();
        let result = ht_sine(&data).unwrap();
        assert_eq!(result.sine.len(), data.len());
        assert_eq!(result.lead_sine.len(), data.len());
    }
}
