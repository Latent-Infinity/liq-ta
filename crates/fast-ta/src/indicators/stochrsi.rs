//! STOCHRSI (Stochastic RSI) indicator.
//!
//! Stochastic RSI applies the Stochastic oscillator formula to RSI values
//! instead of price data, providing a more sensitive momentum indicator.
//!
//! # Formula
//!
//! ```text
//! RSI = standard RSI calculation
//! StochRSI = (RSI - Lowest RSI over period) / (Highest RSI over period - Lowest RSI over period)
//! FastK = StochRSI
//! FastD = SMA of FastK over d_period
//! ```
//!
//! # Range
//!
//! `StochRSI` ranges from 0 to 1 (or 0 to 100 if scaled):
//! - > 0.80: Overbought
//! - < 0.20: Oversold
//!
//! # Lookback
//!
//! The lookback period is `rsi_period + stoch_period - 1` for `FastK`,
//! plus additional `d_period - 1` for `FastD`.

use crate::error::{Error, Result};
use crate::indicators::rsi::{rsi_into, rsi_lookback};
use crate::indicators::sma::sma_from_idx_into;
use crate::kernels::{rolling_max_nan_propagating, rolling_min_nan_propagating};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Output structure for Stochastic RSI
#[derive(Debug, Clone)]
pub struct StochRsiOutput<T> {
    /// `FastK` line (raw stochastic of RSI)
    pub fastk: Vec<T>,
    /// `FastD` line (SMA of `FastK`)
    pub fastd: Vec<T>,
}

/// Computes the lookback period for `StochRSI` `FastK`.
#[inline]
#[must_use]
pub const fn stochrsi_k_lookback(rsi_period: usize, stoch_period: usize) -> usize {
    rsi_lookback(rsi_period) + stoch_period - 1
}

/// Computes the lookback period for `StochRSI` `FastD`.
#[inline]
#[must_use]
pub const fn stochrsi_d_lookback(rsi_period: usize, stoch_period: usize, d_period: usize) -> usize {
    stochrsi_k_lookback(rsi_period, stoch_period) + d_period - 1
}

/// Returns the minimum input length required for `StochRSI` calculation.
#[inline]
#[must_use]
pub const fn stochrsi_min_len(rsi_period: usize, stoch_period: usize, d_period: usize) -> usize {
    stochrsi_d_lookback(rsi_period, stoch_period, d_period) + 1
}

/// Computes `StochRSI` and stores results in output slices.
///
/// # Arguments
///
/// * `data` - Price data (typically closing prices)
/// * `rsi_period` - Period for RSI calculation (typically 14)
/// * `stoch_period` - Period for Stochastic calculation (typically 14)
/// * `k_period` - Smoothing period for K (typically 1 for `FastK`, 3 for `SlowK`)
/// * `d_period` - Period for D line (typically 3)
/// * `fastk` - Pre-allocated output slice for `FastK`
/// * `fastd` - Pre-allocated output slice for `FastD`
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn stochrsi_into<T: SeriesElement + 'static>(
    data: &[T],
    rsi_period: usize,
    stoch_period: usize,
    k_period: usize,
    d_period: usize,
    fastk: &mut [T],
    fastd: &mut [T],
) -> Result<()> {
    let n = data.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    if rsi_period == 0 {
        return Err(Error::InvalidPeriod {
            period: rsi_period,
            reason: "rsi_period must be at least 1",
        });
    }

    if stoch_period == 0 {
        return Err(Error::InvalidPeriod {
            period: stoch_period,
            reason: "stoch_period must be at least 1",
        });
    }

    if k_period == 0 {
        return Err(Error::InvalidPeriod {
            period: k_period,
            reason: "k_period must be at least 1",
        });
    }

    if d_period == 0 {
        return Err(Error::InvalidPeriod {
            period: d_period,
            reason: "d_period must be at least 1",
        });
    }

    // We need enough data for RSI calculation
    let rsi_min = rsi_period + 1;
    if n < rsi_min {
        return Err(Error::InsufficientData {
            indicator: "stochrsi",
            required: rsi_min,
            actual: n,
        });
    }

    if fastk.len() < n || fastd.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "stochrsi",
            required: n,
            actual: fastk.len().min(fastd.len()),
        });
    }

    // First calculate RSI
    let mut rsi_values = vec![T::zero(); n];
    rsi_into(data, rsi_period, &mut rsi_values)?;

    let k_lookback = stochrsi_k_lookback(rsi_period, stoch_period);
    let d_lookback = stochrsi_d_lookback(rsi_period, stoch_period, d_period);

    // Fill lookback with NaN
    for i in 0..n {
        fastk[i] = T::nan();
        fastd[i] = T::nan();
    }

    // Calculate raw StochRSI using O(n) rolling extrema
    // Optimization: For f64/f32, allocate uninitialized memory (Section 5.4)
    use std::any::TypeId;
    let mut raw_stochrsi = if TypeId::of::<T>() == TypeId::of::<f64>() {
        let mut v: Vec<T> = Vec::with_capacity(n);
        unsafe { v.set_len(n); }
        v
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let mut v: Vec<T> = Vec::with_capacity(n);
        unsafe { v.set_len(n); }
        v
    } else {
        vec![T::nan(); n]
    };
    let rsi_lb = rsi_lookback(rsi_period);

    // Use rolling_extrema kernel for O(n) min/max computation
    let min_rsi = rolling_min_nan_propagating(&rsi_values, stoch_period)?;
    let max_rsi = rolling_max_nan_propagating(&rsi_values, stoch_period)?;

    // Compute StochRSI from rolling min/max
    let stoch_start = rsi_lb + stoch_period - 1;
    let half = T::from_f64(0.5)?;
    for i in stoch_start..n {
        let min_val = min_rsi[i];
        let max_val = max_rsi[i];

        if is_invalid(min_val) || is_invalid(max_val) {
            raw_stochrsi[i] = T::nan();
        } else {
            let range = max_val - min_val;
            if range == T::zero() {
                // If range is zero, StochRSI is typically set to 50 (or 0.5)
                raw_stochrsi[i] = half;
            } else {
                raw_stochrsi[i] = (rsi_values[i] - min_val) / range;
            }
        }
    }

    // Apply k_period smoothing (SMA) to get FastK
    if k_period == 1 {
        // No smoothing needed - copy raw_stochrsi to fastk
        for i in k_lookback..n {
            fastk[i] = raw_stochrsi[i];
        }
    } else {
        // Use shared SMA implementation for k_period smoothing
        // raw_stochrsi is valid from stoch_start onwards
        sma_from_idx_into(&raw_stochrsi, k_period, stoch_start, fastk)?;
    }

    // Calculate FastD (SMA of FastK)
    if d_period == 1 {
        // No smoothing needed - copy fastk to fastd
        for i in d_lookback..n {
            fastd[i] = fastk[i];
        }
    } else {
        // Use shared SMA implementation for d_period smoothing
        // fastk is valid from k_lookback onwards (when k_period == 1)
        // or from stoch_start + k_period - 1 onwards (when k_period > 1)
        // The SMA will correctly propagate NaN for any invalid values
        sma_from_idx_into(fastk, d_period, k_lookback, fastd)?;
    }

    Ok(())
}

/// Computes `StochRSI` (Stochastic RSI).
///
/// # Arguments
///
/// * `data` - Price data (typically closing prices)
/// * `rsi_period` - Period for RSI calculation (typically 14)
/// * `stoch_period` - Period for Stochastic calculation (typically 14)
/// * `k_period` - Smoothing period for K (typically 1 for `FastK`)
/// * `d_period` - Period for D line (typically 3)
///
/// # Returns
///
/// * `Ok(StochRsiOutput)` - `FastK` and `FastD` values (range 0 to 1)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::stochrsi;
///
/// let prices = vec![44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0,
///                   45.5, 46.0, 46.5, 47.0, 46.5, 46.0, 45.5, 45.0, 44.5, 45.0];
///
/// let result = stochrsi(&prices, 5, 5, 1, 3).unwrap();
/// // FastK is the raw StochRSI, FastD is the smoothed version
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn stochrsi<T: SeriesElement + 'static>(
    data: &[T],
    rsi_period: usize,
    stoch_period: usize,
    k_period: usize,
    d_period: usize,
) -> Result<StochRsiOutput<T>> {
    // Optimization: For f64/f32, allocate uninitialized memory (Section 5.4)
    use std::any::TypeId;

    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let data_f64: &[f64] = unsafe { std::mem::transmute(data) };
        let mut fastk: Vec<f64> = Vec::with_capacity(data.len());
        let mut fastd: Vec<f64> = Vec::with_capacity(data.len());
        unsafe {
            fastk.set_len(data.len());
            fastd.set_len(data.len());
        }

        stochrsi_into(data_f64, rsi_period, stoch_period, k_period, d_period, &mut fastk, &mut fastd)?;

        Ok(StochRsiOutput {
            fastk: unsafe { std::mem::transmute(fastk) },
            fastd: unsafe { std::mem::transmute(fastd) },
        })
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let data_f32: &[f32] = unsafe { std::mem::transmute(data) };
        let mut fastk: Vec<f32> = Vec::with_capacity(data.len());
        let mut fastd: Vec<f32> = Vec::with_capacity(data.len());
        unsafe {
            fastk.set_len(data.len());
            fastd.set_len(data.len());
        }

        stochrsi_into(data_f32, rsi_period, stoch_period, k_period, d_period, &mut fastk, &mut fastd)?;

        Ok(StochRsiOutput {
            fastk: unsafe { std::mem::transmute(fastk) },
            fastd: unsafe { std::mem::transmute(fastd) },
        })
    } else {
        // Generic fallback: safe initialization
        let mut fastk = vec![T::nan(); data.len()];
        let mut fastd = vec![T::nan(); data.len()];
        stochrsi_into(data, rsi_period, stoch_period, k_period, d_period, &mut fastk, &mut fastd)?;
        Ok(StochRsiOutput { fastk, fastd })
    }
}

/// Simple `StochRSI` with common defaults (rsi=14, stoch=14, k=1, d=3).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn stochrsi_default<T: SeriesElement>(data: &[T]) -> Result<StochRsiOutput<T>> {
    stochrsi(data, 14, 14, 1, 3)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_stochrsi_k_lookback() {
        // rsi_lookback(14) = 14, + 14 - 1 = 27
        assert_eq!(stochrsi_k_lookback(14, 14), 27);
        // rsi_lookback(5) = 5, + 5 - 1 = 9
        assert_eq!(stochrsi_k_lookback(5, 5), 9);
    }

    #[test]
    fn test_stochrsi_d_lookback() {
        // k_lookback(14, 14) = 27, + 3 - 1 = 29
        assert_eq!(stochrsi_d_lookback(14, 14, 3), 29);
    }

    #[test]
    fn test_stochrsi_min_len() {
        assert_eq!(stochrsi_min_len(14, 14, 3), 30);
        // d_lookback(5, 5, 3) = 9 + 2 = 11, + 1 = 12
        assert_eq!(stochrsi_min_len(5, 5, 3), 12);
    }

    #[test]
    fn test_stochrsi_empty_input() {
        let data: Vec<f64> = vec![];
        let result = stochrsi(&data, 5, 5, 1, 3);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_stochrsi_invalid_rsi_period() {
        let data: Vec<f64> = vec![10.0; 20];
        let result = stochrsi(&data, 0, 5, 1, 3);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_stochrsi_invalid_stoch_period() {
        let data: Vec<f64> = vec![10.0; 20];
        let result = stochrsi(&data, 5, 0, 1, 3);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_stochrsi_invalid_k_period() {
        let data: Vec<f64> = vec![10.0; 20];
        let result = stochrsi(&data, 5, 5, 0, 3);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_stochrsi_invalid_d_period() {
        let data: Vec<f64> = vec![10.0; 20];
        let result = stochrsi(&data, 5, 5, 1, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_stochrsi_output_length() {
        let data: Vec<f64> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];
        let result = stochrsi(&data, 5, 5, 1, 3).unwrap();
        assert_eq!(result.fastk.len(), data.len());
        assert_eq!(result.fastd.len(), data.len());
    }

    #[test]
    fn test_stochrsi_lookback_nan() {
        let data: Vec<f64> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];
        let result = stochrsi(&data, 5, 5, 1, 3).unwrap();

        let k_lookback = stochrsi_k_lookback(5, 5);
        let d_lookback = stochrsi_d_lookback(5, 5, 3);

        // FastK should be NaN for lookback period
        for i in 0..k_lookback {
            assert!(result.fastk[i].is_nan(), "fastk[{}] should be NaN", i);
        }

        // FastD should be NaN for its lookback period
        for i in 0..d_lookback {
            assert!(result.fastd[i].is_nan(), "fastd[{}] should be NaN", i);
        }

        // Values after lookback should be finite
        for i in k_lookback..result.fastk.len() {
            assert!(result.fastk[i].is_finite(), "fastk[{}] should be finite", i);
        }
        for i in d_lookback..result.fastd.len() {
            assert!(result.fastd[i].is_finite(), "fastd[{}] should be finite", i);
        }
    }

    #[test]
    fn test_stochrsi_range() {
        let data: Vec<f64> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];
        let result = stochrsi(&data, 5, 5, 1, 3).unwrap();

        let k_lookback = stochrsi_k_lookback(5, 5);
        for i in k_lookback..result.fastk.len() {
            assert!(
                result.fastk[i] >= 0.0 && result.fastk[i] <= 1.0,
                "fastk[{}] = {} should be in [0, 1]",
                i,
                result.fastk[i]
            );
        }

        let d_lookback = stochrsi_d_lookback(5, 5, 3);
        for i in d_lookback..result.fastd.len() {
            assert!(
                result.fastd[i] >= 0.0 && result.fastd[i] <= 1.0,
                "fastd[{}] = {} should be in [0, 1]",
                i,
                result.fastd[i]
            );
        }
    }

    #[test]
    fn test_stochrsi_into() {
        let data: Vec<f64> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];
        let mut fastk = vec![0.0_f64; 20];
        let mut fastd = vec![0.0_f64; 20];

        stochrsi_into(&data, 5, 5, 1, 3, &mut fastk, &mut fastd).unwrap();

        let k_lookback = stochrsi_k_lookback(5, 5);
        assert!(fastk[k_lookback].is_finite());
    }

    #[test]
    fn test_stochrsi_into_buffer_too_small() {
        let data: Vec<f64> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];
        let mut fastk = vec![0.0_f64; 10]; // Too small
        let mut fastd = vec![0.0_f64; 20];

        let result = stochrsi_into(&data, 5, 5, 1, 3, &mut fastk, &mut fastd);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_stochrsi_f32() {
        let data: Vec<f32> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];
        let result = stochrsi(&data, 5, 5, 1, 3).unwrap();

        let k_lookback = stochrsi_k_lookback(5, 5);
        assert!(result.fastk[k_lookback].is_finite());
    }

    #[test]
    fn test_stochrsi_default() {
        let data: Vec<f64> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0, 45.5, 46.0, 45.5, 45.0, 44.5, 44.0, 43.5, 44.0,
            44.5, 45.0,
        ];
        let result = stochrsi_default(&data).unwrap();
        assert_eq!(result.fastk.len(), data.len());
    }

    #[test]
    fn test_stochrsi_constant_prices() {
        // Constant prices should give RSI = 50 (no gains or losses after initial)
        // StochRSI with constant RSI = 0.5 (middle of range)
        let data: Vec<f64> = vec![10.0; 20];
        let result = stochrsi(&data, 5, 5, 1, 3).unwrap();

        let k_lookback = stochrsi_k_lookback(5, 5);
        for i in k_lookback..result.fastk.len() {
            assert!(
                result.fastk[i] >= 0.0 && result.fastk[i] <= 1.0,
                "fastk[{}] = {} should be in [0, 1]",
                i,
                result.fastk[i]
            );
        }
    }

    #[test]
    fn test_stochrsi_with_smoothing() {
        let data: Vec<f64> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];

        // Test with k_period = 3 (SlowK)
        let result = stochrsi(&data, 5, 5, 3, 3).unwrap();
        let k_lookback = stochrsi_k_lookback(5, 5) + 3 - 1;
        for i in k_lookback..result.fastk.len() {
            assert!(result.fastk[i].is_finite(), "fastk[{}] should be finite", i);
        }
    }
}