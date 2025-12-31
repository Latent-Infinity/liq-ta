//! `HT_PHASOR` (Hilbert Transform - Phasor Components)
//!
//! This indicator uses the Hilbert Transform to compute the in-phase (I) and
//! quadrature (Q) components of the price data.
//!
//! # Algorithm
//!
//! The Hilbert Transform creates a complex analytic signal from real price data:
//!
//! ```text
//! Analytic Signal = I(t) + j·Q(t)
//! ```
//!
//! Where:
//! - **I (In-phase)**: The original signal after detrending (3-bar delay)
//! - **Q (Quadrature)**: The 90-degree phase-shifted signal
//!
//! These components form a phasor that rotates in the complex plane, with the
//! rotation speed indicating the instantaneous frequency of the dominant cycle.
//!
//! # Interpretation
//!
//! - **Phasor magnitude** (`sqrt(I² + Q²)`): Indicates cycle amplitude
//! - **Phasor angle** (`atan(Q/I)`): Indicates cycle phase
//! - **Rotation direction**: Counter-clockwise = cycle progressing normally
//! - **Rotation speed**: Fast = short cycle, Slow = long cycle or trend
//!
//! # Use Cases
//!
//! - Building custom cycle indicators
//! - Analyzing cycle strength via phasor magnitude
//! - Detecting phase relationships between different time series
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

/// Output structure for `HT_PHASOR` containing in-phase and quadrature components.
#[derive(Debug, Clone)]
pub struct HtPhasorOutput<T> {
    /// In-phase component
    pub inphase: Vec<T>,
    /// Quadrature component
    pub quadrature: Vec<T>,
}

/// Returns the lookback period for `HT_PHASOR`.
#[inline]
#[must_use]
pub const fn ht_phasor_lookback() -> usize {
    ht_lookback()
}

/// Returns the minimum input length required for `HT_PHASOR`.
#[inline]
#[must_use]
pub const fn ht_phasor_min_len() -> usize {
    ht_min_len()
}

/// Computes `HT_PHASOR` and stores results in output buffers.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn ht_phasor_into<T: SeriesElement>(
    data: &[T],
    inphase_out: &mut [T],
    quadrature_out: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = data.len();
    let lookback = ht_phasor_lookback();
    let min_len = ht_phasor_min_len();

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "ht_phasor",
            required: min_len,
            actual: n,
        });
    }

    if inphase_out.len() < n || quadrature_out.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "ht_phasor",
            required: n,
            actual: inphase_out.len().min(quadrature_out.len()),
        });
    }

    let state = hilbert_transform(data)?;

    for i in 0..lookback {
        inphase_out[i] = T::nan();
        quadrature_out[i] = T::nan();
    }

    for i in lookback..n {
        inphase_out[i] = state.i1[i];
        quadrature_out[i] = state.q1[i];
    }

    Ok(())
}

/// Computes `HT_PHASOR`.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn ht_phasor<T: SeriesElement>(data: &[T]) -> Result<HtPhasorOutput<T>> {
    let n = data.len();
    if n == 0 {
        return Err(Error::EmptyInput);
    }

    let min_len = ht_phasor_min_len();
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "ht_phasor",
            required: min_len,
            actual: n,
        });
    }

    let state = hilbert_transform(data)?;
    let lookback = ht_phasor_lookback();

    let mut inphase = vec![T::nan(); n];
    let mut quadrature = vec![T::nan(); n];

    for i in lookback..n {
        inphase[i] = state.i1[i];
        quadrature[i] = state.q1[i];
    }

    Ok(HtPhasorOutput {
        inphase,
        quadrature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ht_phasor_lookback() {
        assert_eq!(ht_phasor_lookback(), 63);
    }

    #[test]
    fn test_ht_phasor_min_len() {
        assert_eq!(ht_phasor_min_len(), 64);
    }

    #[test]
    fn test_ht_phasor_empty_input() {
        let data: Vec<f64> = vec![];
        let result = ht_phasor(&data);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_ht_phasor_insufficient_data() {
        let data: Vec<f64> = vec![1.0; 50];
        let result = ht_phasor(&data);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_ht_phasor_output_length() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_phasor(&data).unwrap();
        assert_eq!(result.inphase.len(), data.len());
        assert_eq!(result.quadrature.len(), data.len());
    }

    #[test]
    fn test_ht_phasor_nan_count() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_phasor(&data).unwrap();

        let lookback = ht_phasor_lookback();
        let nan_count_i = result.inphase.iter().filter(|x| x.is_nan()).count();
        let nan_count_q = result.quadrature.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count_i, lookback);
        assert_eq!(nan_count_q, lookback);
    }

    #[test]
    fn test_ht_phasor_valid_values() {
        let data: Vec<f64> = (1..=200)
            .map(|x| 50.0 + (x as f64 * 0.1).sin() * 10.0)
            .collect();
        let result = ht_phasor(&data).unwrap();

        let lookback = ht_phasor_lookback();
        for i in lookback..result.inphase.len() {
            assert!(result.inphase[i].is_finite());
            assert!(result.quadrature[i].is_finite());
        }
    }

    #[test]
    fn test_ht_phasor_into_buffer_too_small() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let mut inphase = vec![0.0_f64; 50];
        let mut quadrature = vec![0.0_f64; 100];
        let result = ht_phasor_into(&data, &mut inphase, &mut quadrature);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_ht_phasor_f32() {
        let data: Vec<f32> = (1..=100).map(|x| x as f32).collect();
        let result = ht_phasor(&data).unwrap();
        assert_eq!(result.inphase.len(), data.len());
        assert_eq!(result.quadrature.len(), data.len());
    }
}
