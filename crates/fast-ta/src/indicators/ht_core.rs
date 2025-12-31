//! Hilbert Transform Core Algorithm
//!
//! This module contains the shared Hilbert Transform computation used by all HT_* indicators.
//! The algorithm is based on John Ehlers' work on applying the Hilbert Transform to financial
//! market data, as described in his book "Rocket Science for Traders" (2001).
//!
//! # Algorithm Overview
//!
//! The Hilbert Transform is a signal processing technique that creates a 90-degree phase shift
//! of an input signal. In technical analysis, it's used to:
//!
//! 1. **Extract the dominant cycle** - Identify the primary periodic component in price data
//! 2. **Measure instantaneous phase** - Track where we are in the current market cycle
//! 3. **Distinguish trends from cycles** - Determine if price is trending or oscillating
//!
//! ## Mathematical Foundation
//!
//! The discrete Hilbert Transform approximation uses a weighted FIR filter with coefficients
//! designed to approximate the ideal Hilbert Transform response:
//!
//! - Coefficients `a = 0.0962` and `b = 0.5769` are derived from optimal filter design
//! - A 4-bar weighted moving average (WMA) smooths the input before transformation
//! - The homodyne discriminator extracts the dominant cycle period from phase relationships
//!
//! ## Processing Steps
//!
//! 1. **Smoothing**: Apply 4-bar WMA with weights [4, 3, 2, 1] to reduce noise
//! 2. **Detrending**: Remove trend component using the Hilbert filter
//! 3. **Quadrature**: Generate in-phase (I) and quadrature (Q) components
//! 4. **Phase Advance**: Apply 90-degree phase shift using j-operator approximation
//! 5. **Homodyne Discriminator**: Extract dominant cycle period from I/Q correlation
//! 6. **Period Smoothing**: Apply exponential smoothing to stabilize period estimates
//!
//! # Outputs
//!
//! The Hilbert Transform computes several values that different indicators use:
//! - `smooth_period` - The smoothed dominant cycle period (used by `HT_DCPERIOD`)
//! - `phase` - The instantaneous phase angle in degrees (used by `HT_DCPHASE`)
//! - `i1`, `q1` - In-phase and quadrature components (used by `HT_PHASOR`)
//! - `sine`, `lead_sine` - Sine wave outputs (used by `HT_SINE`)
//! - `trend_mode` - 1 for trend, 0 for cycle (used by `HT_TRENDMODE`)
//! - `trendline` - Adaptive trendline (used by `HT_TRENDLINE`)
//!
//! # Performance
//!
//! - **Time Complexity**: O(n) where n is the input length
//! - **Space Complexity**: O(n) for working arrays (17 arrays of length n)
//! - The algorithm requires a 63-bar warm-up period for stable output
//!
//! # References
//!
//! - Ehlers, John F. "Rocket Science for Traders" (2001)
//! - Ehlers, John F. "MESA and Trading Market Cycles" (1992)

use crate::traits::SeriesElement;
use std::f64::consts::PI;

/// Hilbert Transform state containing all computed values.
#[derive(Debug)]
pub struct HilbertState<T> {
    /// Dominant cycle period
    pub period: Vec<T>,
    /// Smoothed dominant cycle period
    pub smooth_period: Vec<T>,
    /// Instantaneous phase in degrees
    pub phase: Vec<T>,
    /// In-phase component (raw)
    pub i1: Vec<T>,
    /// Quadrature component (raw)
    pub q1: Vec<T>,
    /// Sine wave
    pub sine: Vec<T>,
    /// Lead sine wave (45 degrees ahead)
    pub lead_sine: Vec<T>,
    /// Trend mode: 1 for trend, 0 for cycle
    pub trend_mode: Vec<T>,
    /// Adaptive trendline
    pub trendline: Vec<T>,
}

/// Computes all Hilbert Transform values for the given data.
///
/// This is the core computation shared by all HT_* indicators. It performs
/// a single pass through the data, computing all intermediate and output
/// values that the individual indicator functions extract.
///
/// # Arguments
///
/// * `data` - Input price data (typically close prices or HL2 average)
///
/// # Returns
///
/// A `HilbertState` containing all computed values:
/// - Dominant cycle period and smoothed period
/// - Instantaneous phase (0-360 degrees)
/// - In-phase (I) and Quadrature (Q) phasor components
/// - Sine and lead sine (45 degrees ahead) waves
/// - Trend mode (1 = trend, 0 = cycle)
/// - Adaptive trendline
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
///
/// # Performance
///
/// - Allocates 17 working arrays of length `n`
/// - Single pass O(n) algorithm after array allocation
/// - Consider reusing `HilbertState` outputs across multiple indicator calls
pub fn hilbert_transform<T: SeriesElement>(data: &[T]) -> crate::error::Result<HilbertState<T>> {
    let n = data.len();
    if n == 0 {
        return Err(crate::error::Error::EmptyInput);
    }

    // Initialize all output vectors
    let mut period = vec![T::zero(); n];
    let mut smooth_period = vec![T::zero(); n];
    let mut phase = vec![T::nan(); n];
    let mut i1 = vec![T::zero(); n];
    let mut q1 = vec![T::zero(); n];
    let mut sine = vec![T::nan(); n];
    let mut lead_sine = vec![T::nan(); n];
    let mut trend_mode = vec![T::zero(); n];
    let mut trendline = vec![T::zero(); n];

    // Working arrays
    let mut smooth = vec![T::zero(); n];
    let mut detrender = vec![T::zero(); n];
    let mut ji = vec![T::zero(); n];
    let mut jq = vec![T::zero(); n];
    let mut i2 = vec![T::zero(); n];
    let mut q2 = vec![T::zero(); n];
    let mut re = vec![T::zero(); n];
    let mut im = vec![T::zero(); n];

    // Constants
    let two_pi = T::from_f64(2.0 * PI)?;
    let deg_per_rad = T::from_f64(180.0 / PI)?;
    let a = T::from_f64(0.0962)?;
    let b = T::from_f64(0.5769)?;
    let smooth_coef = T::from_f64(0.2)?;
    let period_smooth = T::from_f64(0.33)?;
    let six = T::from_f64(6.0)?;
    let min_period = T::from_f64(6.0)?;
    let max_period = T::from_f64(50.0)?;

    // WMA coefficients
    let c1 = T::from_f64(4.0)?;
    let c2 = T::from_f64(3.0)?;
    let c3 = T::from_f64(2.0)?;
    let c4 = T::one();
    let c_sum = T::from_f64(10.0)?;

    // Initialize periods
    for i in 0..n.min(12) {
        period[i] = six;
        smooth_period[i] = six;
    }

    // For trend mode calculation
    let mut trend_line_prev = T::zero();
    let mut it_trend = vec![T::zero(); n];

    // Main calculation loop
    // Note: Loop starts at i=6, guaranteeing sufficient history for:
    // - 4-bar WMA (needs i >= 3)
    // - 6-bar Hilbert filter lookback (needs i >= 6)
    for i in 6..n {
        // Compute smoothed price (4-bar WMA with weights [4, 3, 2, 1])
        smooth[i] =
            (c1 * data[i] + c2 * data[i - 1] + c3 * data[i - 2] + c4 * data[i - 3]) / c_sum;

        // Adaptive period adjustment coefficient (Ehlers' formula)
        let adj_prev_period = T::from_f64(0.075)? * period[i - 1] + T::from_f64(0.54)?;

        // Detrender: removes trend using Hilbert filter coefficients
        detrender[i] =
            (a * smooth[i] + b * smooth[i - 2] - b * smooth[i - 4] - a * smooth[i - 6])
                * adj_prev_period;

        // Compute InPhase and Quadrature components
        // Q1 uses same filter on detrended signal
        q1[i] = (a * detrender[i] + b * detrender[i - 2]
            - b * detrender[i - 4]
            - a * detrender[i - 6])
            * adj_prev_period;
        // I1 is 3-bar delayed detrender (90-degree shift approximation)
        i1[i] = detrender[i - 3];

        // Advance the phase of I1 and Q1 by 90 degrees (j-operator)
        ji[i] = (a * i1[i] + b * i1[i - 2] - b * i1[i - 4] - a * i1[i - 6]) * adj_prev_period;
        jq[i] = (a * q1[i] + b * q1[i - 2] - b * q1[i - 4] - a * q1[i - 6]) * adj_prev_period;

        // Phasor addition for 3-bar averaging
        i2[i] = i1[i] - jq[i];
        q2[i] = q1[i] + ji[i];

        // Smooth the I and Q components (EMA with alpha=0.2)
        i2[i] = smooth_coef * i2[i] + (T::one() - smooth_coef) * i2[i - 1];
        q2[i] = smooth_coef * q2[i] + (T::one() - smooth_coef) * q2[i - 1];

        // Homodyne Discriminator: extracts instantaneous frequency
        re[i] = i2[i] * i2[i - 1] + q2[i] * q2[i - 1];
        im[i] = i2[i] * q2[i - 1] - q2[i] * i2[i - 1];

        re[i] = smooth_coef * re[i] + (T::one() - smooth_coef) * re[i - 1];
        im[i] = smooth_coef * im[i] + (T::one() - smooth_coef) * im[i - 1];

        // Compute period from phase difference
        if im[i] != T::zero() && re[i] != T::zero() {
            period[i] = two_pi / (im[i] / re[i]).atan();
        } else {
            period[i] = period[i - 1];
        }

        // Limit period to reasonable range [6, 50] bars
        if period[i] > max_period {
            period[i] = max_period;
        }
        if period[i] < min_period {
            period[i] = min_period;
        }

        // Smooth the period (EMA with alpha=0.33)
        smooth_period[i] =
            period_smooth * period[i] + (T::one() - period_smooth) * smooth_period[i - 1];

        // Compute phase from I/Q ratio (arctangent)
        if i1[i] != T::zero() {
            phase[i] = (q1[i] / i1[i]).atan() * deg_per_rad;
        } else if q1[i] > T::zero() {
            phase[i] = T::from_f64(90.0)?;
        } else if q1[i] < T::zero() {
            phase[i] = T::from_f64(-90.0)?;
        } else {
            phase[i] = phase[i - 1];
        }

        // Adjust phase to 0-360 range
        if phase[i] < T::zero() {
            phase[i] = phase[i] + T::from_f64(360.0)?;
        }

        // Compute sine waves from phase
        let phase_rad = phase[i] / deg_per_rad;
        sine[i] = phase_rad.sin();
        lead_sine[i] = (phase_rad + T::from_f64(PI / 4.0)?).sin(); // 45 degrees ahead

        // Compute adaptive trendline using dominant cycle period
        let dc_period = smooth_period[i];
        let half_period = (dc_period / T::from_f64(2.0)?).floor();
        let hp_int = half_period.to_f64().unwrap_or(3.0) as usize;
        let hp_int = hp_int.max(1).min(25);

        // WMA-style trendline with period-adaptive length
        // Note: hp_int is clamped to [1, 25], and i >= 6, so i >= hp_int for most cases
        if i >= hp_int {
            let mut sum = T::zero();
            let mut weight_sum = T::zero();
            for j in 0..hp_int {
                let weight = T::from_usize(hp_int - j)?;
                sum = sum + weight * data[i - j];
                weight_sum = weight_sum + weight;
            }
            trendline[i] = if weight_sum > T::zero() {
                sum / weight_sum
            } else {
                data[i]
            };
        } else {
            trendline[i] = data[i];
        }

        // Compute IT Trend for trend mode detection
        it_trend[i] = (T::from_f64(2.0)? * trendline[i] + trendline[i - 1]) / T::from_f64(3.0)?;

        // Trend Mode: 1 if trending, 0 if cycling
        // Based on rate of phase change (slow = trend, fast = cycle)
        let trend_diff = (trendline[i] - trend_line_prev).abs();
        let delta_phase = if !phase[i].is_nan() && !phase[i - 1].is_nan() {
            let diff = phase[i] - phase[i - 1];
            if diff < T::zero() {
                diff + T::from_f64(360.0)?
            } else {
                diff
            }
        } else {
            T::zero()
        };

        // If phase is changing slowly (< 15 degrees per bar), we're in a trend
        if delta_phase < T::from_f64(15.0)? && trend_diff > T::zero() {
            trend_mode[i] = T::one();
        } else {
            trend_mode[i] = T::zero();
        }

        trend_line_prev = trendline[i];
    }

    Ok(HilbertState {
        period,
        smooth_period,
        phase,
        i1,
        q1,
        sine,
        lead_sine,
        trend_mode,
        trendline,
    })
}

/// Returns the standard Hilbert Transform lookback period.
#[inline]
#[must_use]
pub const fn ht_lookback() -> usize {
    63
}

/// Returns the minimum data length required for Hilbert Transform.
#[inline]
#[must_use]
pub const fn ht_min_len() -> usize {
    64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hilbert_transform_empty() {
        let data: Vec<f64> = vec![];
        let result = hilbert_transform(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_hilbert_transform_basic() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = hilbert_transform(&data).unwrap();

        assert_eq!(result.period.len(), 100);
        assert_eq!(result.smooth_period.len(), 100);
        assert_eq!(result.phase.len(), 100);
        assert_eq!(result.sine.len(), 100);
        assert_eq!(result.lead_sine.len(), 100);
        assert_eq!(result.trend_mode.len(), 100);
        assert_eq!(result.trendline.len(), 100);
    }

    #[test]
    fn test_hilbert_transform_period_range() {
        let data: Vec<f64> = (1..=200)
            .map(|x| 50.0 + (x as f64 * 0.1).sin() * 10.0)
            .collect();
        let result = hilbert_transform(&data).unwrap();

        // After warmup, period should be in valid range
        for i in 64..result.period.len() {
            assert!(
                result.smooth_period[i] >= 6.0,
                "period at {} is {}",
                i,
                result.smooth_period[i]
            );
            assert!(
                result.smooth_period[i] <= 50.0,
                "period at {} is {}",
                i,
                result.smooth_period[i]
            );
        }
    }

    #[test]
    fn test_hilbert_transform_sine_range() {
        let data: Vec<f64> = (1..=200)
            .map(|x| 50.0 + (x as f64 * 0.1).sin() * 10.0)
            .collect();
        let result = hilbert_transform(&data).unwrap();

        // After warmup, sine should be in [-1, 1]
        for i in 64..result.sine.len() {
            if !result.sine[i].is_nan() {
                assert!(result.sine[i] >= -1.0 && result.sine[i] <= 1.0);
                assert!(result.lead_sine[i] >= -1.0 && result.lead_sine[i] <= 1.0);
            }
        }
    }

    #[test]
    fn test_hilbert_transform_trend_mode() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = hilbert_transform(&data).unwrap();

        // Trend mode should be 0 or 1
        for i in 0..result.trend_mode.len() {
            assert!(result.trend_mode[i] == 0.0 || result.trend_mode[i] == 1.0);
        }
    }
}