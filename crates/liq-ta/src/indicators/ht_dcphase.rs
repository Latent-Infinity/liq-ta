//! `HT_DCPHASE` (Hilbert Transform - Dominant Cycle Phase)
//!
//! This indicator uses the Hilbert Transform to compute the instantaneous phase
//! of the dominant cycle in the price data.
//!
//! # Algorithm
//!
//! The phase is computed from the arctangent of the ratio between the quadrature (Q)
//! and in-phase (I) components produced by the Hilbert Transform:
//!
//! ```text
//! Phase = atan(Q / I) × (180 / π)
//! ```
//!
//! The result is normalized to the range [0, 360] degrees, representing where we are
//! in the current market cycle.
//!
//! # Interpretation
//!
//! - **0°/360°**: Cycle trough (potential buy point)
//! - **90°**: Rising phase (upward momentum)
//! - **180°**: Cycle peak (potential sell point)
//! - **270°**: Falling phase (downward momentum)
//! - **Rate of change**: Fast phase changes indicate cycling; slow changes indicate trending
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

/// Returns the lookback period for `HT_DCPHASE`.
#[inline]
#[must_use]
pub const fn ht_dcphase_lookback() -> usize {
    ht_lookback()
}

/// Returns the minimum input length required for `HT_DCPHASE`.
#[inline]
#[must_use]
pub const fn ht_dcphase_min_len() -> usize {
    ht_min_len()
}

/// Computes `HT_DCPHASE` and stores results in output.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn ht_dcphase_into<T: SeriesElement>(data: &[T], output: &mut [T]) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = data.len();
    let lookback = ht_dcphase_lookback();
    let min_len = ht_dcphase_min_len();

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "ht_dcphase",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "ht_dcphase",
            required: n,
            actual: output.len(),
        });
    }

    let state = hilbert_transform(data)?;

    for i in 0..lookback {
        output[i] = T::nan();
    }

    for i in lookback..n {
        output[i] = state.phase[i];
    }

    Ok(())
}

/// Computes `HT_DCPHASE`.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn ht_dcphase<T: SeriesElement>(data: &[T]) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    ht_dcphase_into(data, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ht_dcphase_lookback() {
        assert_eq!(ht_dcphase_lookback(), 63);
    }

    #[test]
    fn test_ht_dcphase_min_len() {
        assert_eq!(ht_dcphase_min_len(), 64);
    }

    #[test]
    fn test_ht_dcphase_empty_input() {
        let data: Vec<f64> = vec![];
        let result = ht_dcphase(&data);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_ht_dcphase_insufficient_data() {
        let data: Vec<f64> = vec![1.0; 50];
        let result = ht_dcphase(&data);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_ht_dcphase_output_length() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_dcphase(&data).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_ht_dcphase_nan_count() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_dcphase(&data).unwrap();

        let lookback = ht_dcphase_lookback();
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, lookback);
    }

    #[test]
    fn test_ht_dcphase_valid_range() {
        let data: Vec<f64> = (1..=200)
            .map(|x| 50.0 + (x as f64 * 0.1).sin() * 10.0)
            .collect();
        let result = ht_dcphase(&data).unwrap();

        let lookback = ht_dcphase_lookback();
        for i in lookback..result.len() {
            if !result[i].is_nan() {
                // Phase should be in [0, 360]
                assert!(
                    result[i] >= 0.0 && result[i] <= 360.0,
                    "phase at {} is {}",
                    i,
                    result[i]
                );
            }
        }
    }

    #[test]
    fn test_ht_dcphase_into_buffer_too_small() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 50];
        let result = ht_dcphase_into(&data, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_ht_dcphase_f32() {
        let data: Vec<f32> = (1..=100).map(|x| x as f32).collect();
        let result = ht_dcphase(&data).unwrap();
        assert_eq!(result.len(), data.len());
    }
}
