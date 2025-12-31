//! MAMA (MESA Adaptive Moving Average) indicator.
//!
//! MAMA is an adaptive moving average developed by John Ehlers that uses
//! the Hilbert Transform to measure the rate of change of phase, then
//! uses this to adapt the smoothing factor.
//!
//! # Outputs
//!
//! - MAMA: The MESA Adaptive Moving Average
//! - FAMA: Following Adaptive Moving Average (smoothed MAMA)
//!
//! # Parameters
//!
//! - `fast_limit`: Maximum alpha (fastest adaptation), typically 0.5
//! - `slow_limit`: Minimum alpha (slowest adaptation), typically 0.05
//!
//! # Lookback
//!
//! The lookback period is 32 bars (warm-up period for the Hilbert Transform).

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use std::f64::consts::PI;

/// Output structure for MAMA indicator.
#[derive(Debug, Clone)]
pub struct MamaOutput<T> {
    /// MESA Adaptive Moving Average
    pub mama: Vec<T>,
    /// Following Adaptive Moving Average
    pub fama: Vec<T>,
}

/// Computes the lookback period for MAMA.
#[inline]
#[must_use]
pub const fn mama_lookback() -> usize {
    32
}

/// Returns the minimum input length required for MAMA calculation.
#[inline]
#[must_use]
pub const fn mama_min_len() -> usize {
    33
}

/// Computes MAMA with default parameters and stores results in output arrays.
///
/// Uses default parameters: `fast_limit=0.5`, `slow_limit=0.05`
///
/// # Arguments
///
/// * `data` - Input price data
/// * `mama_out` - Pre-allocated output slice for MAMA values
/// * `fama_out` - Pre-allocated output slice for FAMA values
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
pub fn mama_into<T: SeriesElement>(
    data: &[T],
    mama_out: &mut [T],
    fama_out: &mut [T],
) -> Result<()> {
    let fast_limit = T::from_f64(0.5)?;
    let slow_limit = T::from_f64(0.05)?;
    mama_full_into(data, fast_limit, slow_limit, mama_out, fama_out)
}

/// Computes MAMA with custom parameters and stores results in output arrays.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `fast_limit` - Maximum alpha (typically 0.5)
/// * `slow_limit` - Minimum alpha (typically 0.05)
/// * `mama_out` - Pre-allocated output slice for MAMA values
/// * `fama_out` - Pre-allocated output slice for FAMA values
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
pub fn mama_full_into<T: SeriesElement>(
    data: &[T],
    fast_limit: T,
    slow_limit: T,
    mama_out: &mut [T],
    fama_out: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = data.len();
    let lookback = mama_lookback();
    let min_len = mama_min_len();

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "mama",
            required: min_len,
            actual: n,
        });
    }

    if mama_out.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "mama",
            required: n,
            actual: mama_out.len(),
        });
    }

    if fama_out.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "mama (fama)",
            required: n,
            actual: fama_out.len(),
        });
    }

    // Fill lookback period with NaN
    for i in 0..lookback {
        mama_out[i] = T::nan();
        fama_out[i] = T::nan();
    }

    // Constants
    let two_pi = T::from_f64(2.0 * PI)?;

    // Hilbert Transform coefficients
    let a = T::from_f64(0.0962)?;
    let b = T::from_f64(0.5769)?;

    // State variables
    let mut smooth = vec![T::zero(); n];
    let mut detrender = vec![T::zero(); n];
    let mut i1 = vec![T::zero(); n];
    let mut q1 = vec![T::zero(); n];
    let mut ji = vec![T::zero(); n];
    let mut jq = vec![T::zero(); n];
    let mut i2 = vec![T::zero(); n];
    let mut q2 = vec![T::zero(); n];
    let mut re = vec![T::zero(); n];
    let mut im = vec![T::zero(); n];
    let mut period = vec![T::zero(); n];
    let mut smooth_period = vec![T::zero(); n];
    let mut phase = vec![T::zero(); n];

    // MAMA and FAMA state
    let mut mama = data[0];
    let mut fama = data[0];

    // Initial period estimate
    let six = T::from_usize(6)?;
    for i in 0..n.min(12) {
        period[i] = six;
        smooth_period[i] = six;
    }

    // WMA coefficients
    let c1 = T::from_f64(4.0)?;
    let c2 = T::from_f64(3.0)?;
    let c3 = T::from_f64(2.0)?;
    let c4 = T::one();
    let c_sum = T::from_f64(10.0)?;

    // Main calculation loop
    for i in 6..n {
        // Compute smoothed price (4-bar WMA)
        // Note: i >= 6 here, so we always have at least 4 bars of history
        smooth[i] =
            (c1 * data[i] + c2 * data[i - 1] + c3 * data[i - 2] + c4 * data[i - 3]) / c_sum;

        // Compute Hilbert Transform components
        // Note: Loop starts at i=6, so we always have sufficient history for Hilbert Transform
        let adj_prev_period = T::from_f64(0.075)? * period[i - 1] + T::from_f64(0.54)?;

        // Detrender
        detrender[i] =
            (a * smooth[i] + b * smooth[i - 2] - b * smooth[i - 4] - a * smooth[i - 6])
                * adj_prev_period;

        // Compute InPhase and Quadrature components
        q1[i] = (a * detrender[i] + b * detrender[i - 2]
            - b * detrender[i - 4]
            - a * detrender[i - 6])
            * adj_prev_period;
        i1[i] = detrender[i - 3];

        // Advance the phase of I1 and Q1 by 90 degrees
        ji[i] = (a * i1[i] + b * i1[i - 2] - b * i1[i - 4] - a * i1[i - 6]) * adj_prev_period;
        jq[i] = (a * q1[i] + b * q1[i - 2] - b * q1[i - 4] - a * q1[i - 6]) * adj_prev_period;

        // Phasor addition for 3-bar averaging
        i2[i] = i1[i] - jq[i];
        q2[i] = q1[i] + ji[i];

        // Smooth the I and Q components
        let smooth_coef = T::from_f64(0.2)?;
        i2[i] = smooth_coef * i2[i] + (T::one() - smooth_coef) * i2[i - 1];
        q2[i] = smooth_coef * q2[i] + (T::one() - smooth_coef) * q2[i - 1];

        // Homodyne Discriminator
        re[i] = i2[i] * i2[i - 1] + q2[i] * q2[i - 1];
        im[i] = i2[i] * q2[i - 1] - q2[i] * i2[i - 1];

        re[i] = smooth_coef * re[i] + (T::one() - smooth_coef) * re[i - 1];
        im[i] = smooth_coef * im[i] + (T::one() - smooth_coef) * im[i - 1];

        // Compute period
        if im[i] != T::zero() && re[i] != T::zero() {
            period[i] = two_pi / (im[i] / re[i]).atan();
        }

        // Limit period to reasonable range
        let min_period = T::from_f64(6.0)?;
        let max_period = T::from_f64(50.0)?;
        if period[i] > max_period {
            period[i] = max_period;
        }
        if period[i] < min_period {
            period[i] = min_period;
        }

        // Smooth the period
        let period_smooth = T::from_f64(0.33)?;
        smooth_period[i] =
            period_smooth * period[i] + (T::one() - period_smooth) * smooth_period[i - 1];

        // Compute phase
        if i1[i] != T::zero() {
            phase[i] = (q1[i] / i1[i]).atan();
        }

        // Compute delta phase
        let mut delta_phase = phase[i - 1] - phase[i];
        if delta_phase < T::from_f64(1.0)? {
            delta_phase = T::from_f64(1.0)?;
        }

        // Compute alpha
        let mut alpha = fast_limit / delta_phase;
        if alpha < slow_limit {
            alpha = slow_limit;
        }
        if alpha > fast_limit {
            alpha = fast_limit;
        }

        // Update MAMA and FAMA
        mama = alpha * data[i] + (T::one() - alpha) * mama;
        let fama_alpha = T::from_f64(0.5)? * alpha;
        fama = fama_alpha * mama + (T::one() - fama_alpha) * fama;

        // Store output after lookback
        if i >= lookback {
            mama_out[i] = mama;
            fama_out[i] = fama;
        }
    }

    Ok(())
}

/// Computes MAMA with default parameters.
///
/// Uses default parameters: `fast_limit=0.5`, `slow_limit=0.05`
///
/// # Arguments
///
/// * `data` - Input price data
///
/// # Returns
///
/// * `Ok(MamaOutput)` - MAMA and FAMA values
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::mama;
///
/// let mut prices: Vec<f64> = Vec::with_capacity(100);
/// for x in 1..=100 {
///     prices.push(50.0 + (x as f64 * 0.1).sin() * 10.0);
/// }
/// let result = mama(&prices).unwrap();
/// assert!(result.mama[0].is_nan()); // First 32 values are NaN
/// assert!(result.mama[32].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn mama<T: SeriesElement>(data: &[T]) -> Result<MamaOutput<T>> {
    let mut mama_out = vec![T::nan(); data.len()];
    let mut fama_out = vec![T::nan(); data.len()];
    mama_into(data, &mut mama_out, &mut fama_out)?;
    Ok(MamaOutput {
        mama: mama_out,
        fama: fama_out,
    })
}

/// Computes MAMA with custom parameters.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `fast_limit` - Maximum alpha (typically 0.5)
/// * `slow_limit` - Minimum alpha (typically 0.05)
///
/// # Returns
///
/// * `Ok(MamaOutput)` - MAMA and FAMA values
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn mama_full<T: SeriesElement>(
    data: &[T],
    fast_limit: T,
    slow_limit: T,
) -> Result<MamaOutput<T>> {
    let mut mama_out = vec![T::nan(); data.len()];
    let mut fama_out = vec![T::nan(); data.len()];
    mama_full_into(data, fast_limit, slow_limit, &mut mama_out, &mut fama_out)?;
    Ok(MamaOutput {
        mama: mama_out,
        fama: fama_out,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_mama_lookback() {
        assert_eq!(mama_lookback(), 32);
    }

    #[test]
    fn test_mama_min_len() {
        assert_eq!(mama_min_len(), 33);
    }

    #[test]
    fn test_mama_empty_input() {
        let data: Vec<f64> = vec![];
        let result = mama(&data);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_mama_insufficient_data() {
        let data: Vec<f64> = vec![1.0; 20];
        let result = mama(&data);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_mama_output_length() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = mama(&data).unwrap();
        assert_eq!(result.mama.len(), data.len());
        assert_eq!(result.fama.len(), data.len());
    }

    #[test]
    fn test_mama_nan_count() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = mama(&data).unwrap();

        let lookback = mama_lookback();
        let mama_nan_count = result.mama.iter().filter(|x| x.is_nan()).count();
        let fama_nan_count = result.fama.iter().filter(|x| x.is_nan()).count();
        assert_eq!(mama_nan_count, lookback);
        assert_eq!(fama_nan_count, lookback);
    }

    #[test]
    fn test_mama_valid_values() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = mama(&data).unwrap();

        let lookback = mama_lookback();
        for i in lookback..result.mama.len() {
            assert!(result.mama[i].is_finite(), "mama[{}] should be finite", i);
            assert!(result.fama[i].is_finite(), "fama[{}] should be finite", i);
        }
    }

    #[test]
    fn test_mama_fama_relationship() {
        // FAMA is a smoothed version of MAMA, so they should be related
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = mama(&data).unwrap();

        let lookback = mama_lookback();
        // Both should follow the general trend
        for i in lookback..result.mama.len() {
            // Both should be positive for positive input data
            assert!(result.mama[i] > 0.0);
            assert!(result.fama[i] > 0.0);
        }
    }

    #[test]
    fn test_mama_cyclic_data() {
        // Sinusoidal data to test cycle adaptation
        let data: Vec<f64> = (0..200)
            .map(|x| 50.0 + (x as f64 * 0.2).sin() * 10.0)
            .collect();
        let result = mama(&data).unwrap();

        let lookback = mama_lookback();
        for i in lookback..result.mama.len() {
            assert!(result.mama[i].is_finite());
            assert!(result.fama[i].is_finite());
            // Values should be within reasonable range
            assert!(result.mama[i] > 30.0 && result.mama[i] < 70.0);
            assert!(result.fama[i] > 30.0 && result.fama[i] < 70.0);
        }
    }

    #[test]
    fn test_mama_custom_limits() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();

        let result_default = mama(&data).unwrap();
        let result_custom = mama_full(&data, 0.3, 0.1).unwrap();

        // Different limits should produce different results
        let lookback = mama_lookback();
        let mut differs = false;
        for i in lookback..result_default.mama.len() {
            if (result_default.mama[i] - result_custom.mama[i]).abs() > 1e-10 {
                differs = true;
                break;
            }
        }
        assert!(differs, "Different limits should produce different results");
    }

    #[test]
    fn test_mama_into() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let mut mama_out = vec![0.0_f64; data.len()];
        let mut fama_out = vec![0.0_f64; data.len()];

        mama_into(&data, &mut mama_out, &mut fama_out).unwrap();

        let lookback = mama_lookback();
        for i in 0..lookback {
            assert!(mama_out[i].is_nan());
            assert!(fama_out[i].is_nan());
        }
        for i in lookback..data.len() {
            assert!(mama_out[i].is_finite());
            assert!(fama_out[i].is_finite());
        }
    }

    #[test]
    fn test_mama_into_buffer_too_small() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let mut mama_out = vec![0.0_f64; 50]; // Too small
        let mut fama_out = vec![0.0_f64; data.len()];

        let result = mama_into(&data, &mut mama_out, &mut fama_out);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_mama_f32() {
        let data: Vec<f32> = (1..=100).map(|x| x as f32).collect();
        let result = mama(&data).unwrap();

        assert_eq!(result.mama.len(), data.len());
        assert_eq!(result.fama.len(), data.len());

        let lookback = mama_lookback();
        for i in 0..lookback {
            assert!(result.mama[i].is_nan());
            assert!(result.fama[i].is_nan());
        }
        for i in lookback..data.len() {
            assert!(result.mama[i].is_finite());
            assert!(result.fama[i].is_finite());
        }
    }

    #[test]
    fn test_mama_minimum_length() {
        let data: Vec<f64> = (1..=33).map(|x| x as f64).collect();
        let result = mama(&data).unwrap();

        assert_eq!(result.mama.len(), 33);
        assert_eq!(result.fama.len(), 33);
        // First 32 should be NaN, last one valid
        for i in 0..32 {
            assert!(result.mama[i].is_nan());
            assert!(result.fama[i].is_nan());
        }
        assert!(result.mama[32].is_finite());
        assert!(result.fama[32].is_finite());
    }
}
