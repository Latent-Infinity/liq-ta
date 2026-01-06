//! CCI (Commodity Channel Index) indicator.
//!
//! The Commodity Channel Index measures the deviation of an asset's price
//! from its statistical mean, helping identify cyclical trends.
//!
//! # Formula
//!
//! ```text
//! Typical Price = (High + Low + Close) / 3
//! CCI = (TP - SMA(TP, period)) / (0.015 * Mean Deviation)
//! ```
//!
//! Where Mean Deviation is the average absolute deviation from the SMA.
//!
//! # Interpretation
//!
//! - CCI > +100: Overbought, potential selling opportunity
//! - CCI < -100: Oversold, potential buying opportunity
//! - Zero-line crossovers can signal trend changes
//!
//! # Lookback
//!
//! The lookback period is `period - 1`.
//!
//! # Precision Behavior
//!
//! When `PrecisionMode::High` is active and input type is `f32`:
//! - Typical price calculation uses `f64`
//! - Rolling SMA sum uses `f64` accumulator
//! - Mean deviation calculation uses `f64`
//! - Final CCI division performed in `f64`
//!
//! **Tolerance**: hybrid(rel=1e-4, abs=0.1) when comparing f32 High mode to f64 reference.
//! CCI is unbounded but typically ranges ±200.

use crate::error::{Error, Result};
use crate::precision::{current_precision_mode, PrecisionMode};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns true if we should use f64 precision for the given type.
#[inline]
fn use_f64_precision<T: 'static>() -> bool {
    use std::any::TypeId;
    TypeId::of::<T>() == TypeId::of::<f32>() && current_precision_mode() == PrecisionMode::High
}

/// Computes the lookback period for CCI.
#[inline]
#[must_use]
pub const fn cci_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for CCI calculation.
#[inline]
#[must_use]
pub const fn cci_min_len(period: usize) -> usize {
    period
}

/// Computes CCI and stores results in output slice.
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period (typically 20)
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
pub fn cci_into<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    if low.len() != n || close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "HLC arrays must have same length: high={}, low={}, close={}",
                n,
                low.len(),
                close.len()
            ),
        });
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    let min_len = cci_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "cci",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "cci",
            required: n,
            actual: output.len(),
        });
    }

    if use_f64_precision::<T>() {
        cci_core_f64(high, low, close, period, output)
    } else {
        cci_core_native(high, low, close, period, output)
    }
}

/// Core CCI computation using native precision.
fn cci_core_native<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();
    let lookback = cci_lookback(period);
    let inv_period = T::from_f64(1.0 / period as f64)?;
    let inv_three = T::from_f64(1.0 / 3.0)?;
    let inv_constant = T::from_f64(1.0 / 0.015)?;
    let zero = T::zero();

    // Calculate typical prices
    // Optimization: Use uninitialized memory for f64/f32 (Section 5.4)
    use std::any::TypeId;
    let mut tp = if TypeId::of::<T>() == TypeId::of::<f64>() || TypeId::of::<T>() == TypeId::of::<f32>() {
        let mut v: Vec<T> = Vec::with_capacity(n);
        unsafe { v.set_len(n); }
        v
    } else {
        vec![zero; n]
    };
    let mut invalid_flags = vec![false; n];
    for i in 0..n {
        if is_invalid(high[i]) || is_invalid(low[i]) || is_invalid(close[i]) {
            tp[i] = T::nan();
            invalid_flags[i] = true;
        } else {
            tp[i] = (high[i] + low[i] + close[i]) * inv_three;
        }
    }

    // Fill lookback period with NaN
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Initialize rolling sum for SMA
    let mut tp_sum = zero;
    let mut invalid_count = 0usize;
    for i in 0..period {
        if invalid_flags[i] {
            invalid_count += 1;
        } else {
            tp_sum = tp_sum + tp[i];
        }
    }

    // Calculate first CCI value (at index = lookback = period - 1)
    if invalid_count > 0 {
        output[lookback] = T::nan();
    } else {
        let tp_sma = tp_sum * inv_period;

        // Calculate mean deviation for first window
        // Optimization: Use slice iterator
        let window = &tp[0..period];
        let mut deviation_sum = zero;
        for &tp_val in window {
            let diff = tp_val - tp_sma;
            deviation_sum = deviation_sum + diff.abs();
        }
        let mean_deviation = deviation_sum * inv_period;

        // CCI = (TP - SMA) / (0.015 * Mean Deviation)
        // Rewritten as: (TP - SMA) * (1/0.015) / Mean Deviation
        if mean_deviation == zero {
            output[lookback] = zero;
        } else {
            output[lookback] = (tp[lookback] - tp_sma) * inv_constant / mean_deviation;
        }
    }

    // Rolling calculation for remaining values
    for i in (lookback + 1)..n {
        // Update rolling sum for SMA
        let old_idx = i - period;
        if invalid_flags[old_idx] {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            tp_sum = tp_sum - tp[old_idx];
        }
        if invalid_flags[i] {
            invalid_count += 1;
        } else {
            tp_sum = tp_sum + tp[i];
        }

        if invalid_count > 0 {
            output[i] = T::nan();
            continue;
        }

        let tp_sma = tp_sum * inv_period;

        // Mean deviation still requires iterating over window
        // (unavoidable since deviations depend on current window's mean)
        // Optimization: Use slice iterator for better auto-vectorization hints
        let start = i + 1 - period;
        let window = &tp[start..=i];
        let mut deviation_sum = zero;

        // Use iterator to allow compiler optimization
        for &tp_val in window {
            let diff = tp_val - tp_sma;
            deviation_sum = deviation_sum + diff.abs();
        }
        let mean_deviation = deviation_sum * inv_period;

        if mean_deviation == zero {
            output[i] = zero;
        } else {
            output[i] = (tp[i] - tp_sma) * inv_constant / mean_deviation;
        }
    }

    Ok(())
}

/// Core CCI computation using f64 precision for f32 inputs.
fn cci_core_f64<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();
    let lookback = cci_lookback(period);
    let inv_period = 1.0 / period as f64;
    let inv_three = 1.0 / 3.0;
    let inv_constant = 1.0 / 0.015;

    // Calculate typical prices in f64
    // Optimization: Use uninitialized memory (Section 5.4)
    let mut tp: Vec<f64> = Vec::with_capacity(n);
    unsafe { tp.set_len(n); }
    let mut invalid_flags = vec![false; n];
    for i in 0..n {
        if is_invalid(high[i]) || is_invalid(low[i]) || is_invalid(close[i]) {
            tp[i] = f64::NAN;
            invalid_flags[i] = true;
        } else {
            let h = high[i].to_f64().unwrap_or(0.0);
            let l = low[i].to_f64().unwrap_or(0.0);
            let c = close[i].to_f64().unwrap_or(0.0);
            tp[i] = (h + l + c) * inv_three;
        }
    }

    // Fill lookback period with NaN
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Initialize rolling sum for SMA in f64
    let mut tp_sum: f64 = 0.0;
    let mut invalid_count = 0usize;
    for i in 0..period {
        if invalid_flags[i] {
            invalid_count += 1;
        } else {
            tp_sum += tp[i];
        }
    }

    // Calculate first CCI value (at index = lookback = period - 1)
    if invalid_count > 0 {
        output[lookback] = T::nan();
    } else {
        let tp_sma = tp_sum * inv_period;

        // Calculate mean deviation for first window in f64
        // Optimization: Use slice iterator
        let window = &tp[0..period];
        let mut deviation_sum: f64 = 0.0;
        for &tp_val in window {
            let diff = tp_val - tp_sma;
            deviation_sum += diff.abs();
        }
        let mean_deviation = deviation_sum * inv_period;

        // CCI = (TP - SMA) * (1/0.015) / Mean Deviation
        let cci_val = if mean_deviation == 0.0 {
            0.0
        } else {
            (tp[lookback] - tp_sma) * inv_constant / mean_deviation
        };
        output[lookback] = T::from_f64(cci_val)?;
    }

    // Rolling calculation for remaining values
    for i in (lookback + 1)..n {
        // Update rolling sum for SMA
        let old_idx = i - period;
        if invalid_flags[old_idx] {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            tp_sum -= tp[old_idx];
        }
        if invalid_flags[i] {
            invalid_count += 1;
        } else {
            tp_sum += tp[i];
        }

        if invalid_count > 0 {
            output[i] = T::nan();
            continue;
        }

        let tp_sma = tp_sum * inv_period;

        // Mean deviation in f64
        // Optimization: Use slice iterator for better auto-vectorization
        let start = i + 1 - period;
        let window = &tp[start..=i];
        let mut deviation_sum: f64 = 0.0;

        // Use iterator to allow compiler optimization
        for &tp_val in window {
            let diff = tp_val - tp_sma;
            deviation_sum += diff.abs();
        }
        let mean_deviation = deviation_sum * inv_period;

        let cci_val = if mean_deviation == 0.0 {
            0.0
        } else {
            (tp[i] - tp_sma) * inv_constant / mean_deviation
        };
        output[i] = T::from_f64(cci_val)?;
    }

    Ok(())
}

/// Computes CCI (Commodity Channel Index).
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period (typically 20)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - CCI values (typically ranges from -300 to +300)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::cci;
///
/// let high = vec![25.0_f64, 26.0, 27.0, 28.0, 27.5, 27.0, 26.5, 26.0, 25.5, 25.0];
/// let low = vec![23.0_f64, 24.0, 25.0, 26.0, 25.5, 25.0, 24.5, 24.0, 23.5, 23.0];
/// let close = vec![24.0_f64, 25.0, 26.0, 27.0, 26.5, 26.0, 25.5, 25.0, 24.5, 24.0];
///
/// let result = cci(&high, &low, &close, 5).unwrap();
/// // First 4 values are NaN (lookback = period - 1)
/// assert!(result[4].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn cci<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Result<Vec<T>> {
    // Optimization: For f64/f32, allocate uninitialized memory (Section 5.4)
    use std::any::TypeId;

    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let high_f64: &[f64] = unsafe { std::mem::transmute(high) };
        let low_f64: &[f64] = unsafe { std::mem::transmute(low) };
        let close_f64: &[f64] = unsafe { std::mem::transmute(close) };

        let mut output: Vec<f64> = Vec::with_capacity(high.len());
        unsafe { output.set_len(high.len()); }

        cci_into(high_f64, low_f64, close_f64, period, &mut output)?;
        Ok(unsafe { std::mem::transmute(output) })
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let high_f32: &[f32] = unsafe { std::mem::transmute(high) };
        let low_f32: &[f32] = unsafe { std::mem::transmute(low) };
        let close_f32: &[f32] = unsafe { std::mem::transmute(close) };

        let mut output: Vec<f32> = Vec::with_capacity(high.len());
        unsafe { output.set_len(high.len()); }

        cci_into(high_f32, low_f32, close_f32, period, &mut output)?;
        Ok(unsafe { std::mem::transmute(output) })
    } else {
        // Generic fallback: safe initialization
        let mut output = vec![T::zero(); high.len()];
        cci_into(high, low, close, period, &mut output)?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_cci_lookback() {
        assert_eq!(cci_lookback(1), 0);
        assert_eq!(cci_lookback(14), 13);
        assert_eq!(cci_lookback(20), 19);
    }

    #[test]
    fn test_cci_min_len() {
        assert_eq!(cci_min_len(1), 1);
        assert_eq!(cci_min_len(14), 14);
        assert_eq!(cci_min_len(20), 20);
    }

    #[test]
    fn test_cci_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];
        let result = cci(&high, &low, &close, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_cci_invalid_period() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let result = cci(&high, &low, &close, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_cci_insufficient_data() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5];
        let result = cci(&high, &low, &close, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_cci_length_mismatch() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let result = cci(&high, &low, &close, 5);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_cci_output_length() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let result = cci(&high, &low, &close, 5).unwrap();
        assert_eq!(result.len(), high.len());
    }

    #[test]
    fn test_cci_lookback_nan() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let result = cci(&high, &low, &close, 5).unwrap();

        // First 4 values should be NaN (lookback = period - 1 = 4)
        for i in 0..4 {
            assert!(result[i].is_nan(), "cci[{}] should be NaN", i);
        }

        // Values after lookback should be finite
        for i in 4..result.len() {
            assert!(result[i].is_finite(), "cci[{}] should be finite", i);
        }
    }

    #[test]
    fn test_cci_constant_prices() {
        // When prices are constant, CCI should be 0 (or near 0)
        let high: Vec<f64> = vec![10.0; 10];
        let low: Vec<f64> = vec![10.0; 10];
        let close: Vec<f64> = vec![10.0; 10];
        let result = cci(&high, &low, &close, 5).unwrap();

        // All non-NaN values should be 0 when prices are constant
        for i in 4..result.len() {
            assert!(
                (result[i] - 0.0).abs() < 1e-10,
                "cci[{}] should be 0 for constant prices",
                i
            );
        }
    }

    #[test]
    fn test_cci_uptrend() {
        // In a strong uptrend, CCI should be positive
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let result = cci(&high, &low, &close, 5).unwrap();

        // In uptrend, later CCI values should be positive
        for i in 4..result.len() {
            assert!(
                result[i] > 0.0,
                "cci[{}] = {} should be positive in uptrend",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_cci_downtrend() {
        // In a strong downtrend, CCI should be negative
        let high: Vec<f64> = vec![19.0, 18.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 10.0];
        let low: Vec<f64> = vec![18.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 10.0, 9.0];
        let close: Vec<f64> = vec![18.5, 17.5, 16.5, 15.5, 14.5, 13.5, 12.5, 11.5, 10.5, 9.5];
        let result = cci(&high, &low, &close, 5).unwrap();

        // In downtrend, later CCI values should be negative
        for i in 4..result.len() {
            assert!(
                result[i] < 0.0,
                "cci[{}] = {} should be negative in downtrend",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_cci_into() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let mut output = vec![0.0_f64; 5];

        cci_into(&high, &low, &close, 5, &mut output).unwrap();

        // Check that result is finite at last position
        assert!(output[4].is_finite());
    }

    #[test]
    fn test_cci_into_buffer_too_small() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let mut output = vec![0.0_f64; 3]; // Too small

        let result = cci_into(&high, &low, &close, 5, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_cci_f32() {
        let high: Vec<f32> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f32> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close: Vec<f32> = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let result = cci(&high, &low, &close, 5).unwrap();

        assert!(result[4].is_finite());
    }

    #[test]
    fn test_cci_period_1() {
        // With period 1, CCI depends on a single bar
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let result = cci(&high, &low, &close, 1).unwrap();

        // With period 1, SMA = current TP, so deviation = 0, so CCI = 0
        for i in 0..result.len() {
            assert!(
                (result[i] - 0.0).abs() < 1e-10,
                "cci[{}] should be 0 with period 1",
                i
            );
        }
    }
}
