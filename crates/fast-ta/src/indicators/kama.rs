//! KAMA (Kaufman Adaptive Moving Average) indicator.
//!
//! KAMA is an adaptive moving average that adjusts its smoothing based on
//! market efficiency. It responds quickly during trending markets and slowly
//! during sideways markets.
//!
//! # Formula
//!
//! 1. Efficiency Ratio (ER) = |Price - Price\[n ago\]| / Sum(|Price\[i\] - Price\[i-1\]|)
//! 2. Smoothing Constant (SC) = \[ER * (`fast_sc` - `slow_sc`) + `slow_sc`\]^2
//!    where `fast_sc` = `2/(fast_period+1)`, `slow_sc` = `2/(slow_period+1)`
//! 3. KAMA = KAMA\[prev\] + SC * (Price - KAMA\[prev\])
//!
//! # Default Parameters
//!
//! - period: 10 (for efficiency ratio calculation)
//! - `fast_period`: 2 (fast EMA smoothing)
//! - `slow_period`: 30 (slow EMA smoothing)
//!
//! # Lookback
//!
//! The lookback period is `period - 1`.

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Computes the lookback period for KAMA.
///
/// The lookback is `period - 1`.
#[inline]
#[must_use]
pub const fn kama_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for KAMA calculation.
#[inline]
#[must_use]
pub const fn kama_min_len(period: usize) -> usize {
    if period == 0 {
        1
    } else {
        period
    }
}

/// Computes KAMA with default fast/slow periods (2/30) and stores results in output.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The efficiency ratio period (typically 10)
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
pub fn kama_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    kama_full_into(data, period, 2, 30, output)
}

/// Computes KAMA with custom fast/slow periods and stores results in output.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The efficiency ratio period
/// * `fast_period` - Fast EMA period (typically 2)
/// * `slow_period` - Slow EMA period (typically 30)
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
pub fn kama_full_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    fast_period: usize,
    slow_period: usize,
    output: &mut [T],
) -> Result<()> {
    // Validate inputs
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if fast_period == 0 {
        return Err(Error::InvalidPeriod {
            period: fast_period,
            reason: "fast_period must be at least 1",
        });
    }

    if slow_period == 0 {
        return Err(Error::InvalidPeriod {
            period: slow_period,
            reason: "slow_period must be at least 1",
        });
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            indicator: "kama",
            required: period,
            actual: data.len(),
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            indicator: "kama",
            required: data.len(),
            actual: output.len(),
        });
    }

    // Type specialization for f64/f32
    use std::any::TypeId;
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let data_f64: &[f64] = unsafe { std::mem::transmute(data) };
        let output_f64: &mut [f64] = unsafe { std::mem::transmute(output) };
        return kama_full_into_f64(data_f64, period, fast_period, slow_period, output_f64);
    }

    if TypeId::of::<T>() == TypeId::of::<f32>() {
        let data_f32: &[f32] = unsafe { std::mem::transmute(data) };
        let output_f32: &mut [f32] = unsafe { std::mem::transmute(output) };
        return kama_full_into_f32(data_f32, period, fast_period, slow_period, output_f32);
    }

    // Generic fallback
    let n = data.len();
    let lookback = kama_lookback(period);

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate smoothing constants
    let two = T::from_usize(2)?;
    let fast_sc = two / T::from_usize(fast_period + 1)?;
    let slow_sc = two / T::from_usize(slow_period + 1)?;
    let sc_diff = fast_sc - slow_sc;

    // Initialize KAMA with first valid value
    let mut kama = data[lookback];
    output[lookback] = kama;

    // Early exit if only one valid output
    if lookback + 1 >= n {
        return Ok(());
    }

    // Initialize rolling volatility for first window (compute on-the-fly)
    let mut volatility = T::zero();
    let start = lookback + 2 - period;
    for j in start..=(lookback + 1) {
        volatility = volatility + (data[j] - data[j - 1]).abs();
    }

    // First iteration
    if lookback + 1 < n {
        let i = lookback + 1;
        let change = (data[i] - data[i - period]).abs();
        let er = if volatility > T::zero() {
            change / volatility
        } else {
            T::zero()
        };
        let sc_raw = er * sc_diff + slow_sc;
        let sc = sc_raw * sc_raw;
        kama = kama + sc * (data[i] - kama);
        output[i] = kama;
    }

    // Steady-state loop
    for i in (lookback + 2)..n {
        // Rolling volatility update (compute on-the-fly)
        let old_change = (data[i - period] - data[i - period - 1]).abs();
        let new_change = (data[i] - data[i - 1]).abs();
        volatility = volatility - old_change + new_change;

        let change = (data[i] - data[i - period]).abs();

        let er = if volatility > T::zero() {
            change / volatility
        } else {
            T::zero()
        };

        let sc_raw = er * sc_diff + slow_sc;
        let sc = sc_raw * sc_raw;

        kama = kama + sc * (data[i] - kama);
        output[i] = kama;
    }

    Ok(())
}

/// f64-specialized KAMA implementation - eliminates trait overhead
#[inline]
fn kama_full_into_f64(
    data: &[f64],
    period: usize,
    fast_period: usize,
    slow_period: usize,
    output: &mut [f64],
) -> Result<()> {
    let n = data.len();
    let lookback = period - 1;

    // Fill lookback period with NaN
    output[..lookback].fill(f64::NAN);

    // Calculate smoothing constants (no trait overhead)
    let fast_sc = 2.0 / (fast_period + 1) as f64;
    let slow_sc = 2.0 / (slow_period + 1) as f64;
    let sc_diff = fast_sc - slow_sc;

    // Initialize KAMA
    let mut kama = data[lookback];
    output[lookback] = kama;

    if lookback + 1 >= n {
        return Ok(());
    }

    // Initialize rolling volatility for first window (compute on-the-fly)
    let mut volatility = 0.0_f64;
    let start = lookback + 2 - period;
    for j in start..=(lookback + 1) {
        volatility += (data[j] - data[j - 1]).abs();
    }

    // First iteration (no volatility update needed)
    if lookback + 1 < n {
        let i = lookback + 1;
        let change = (data[i] - data[i - period]).abs();
        let er = if volatility > 0.0 {
            change / volatility
        } else {
            0.0
        };
        let sc_raw = sc_diff.mul_add(er, slow_sc);
        let sc = sc_raw * sc_raw;
        kama = (data[i] - kama).mul_add(sc, kama);
        output[i] = kama;
    }

    // Steady-state loop with 4x unrolling
    let main_loop_end = lookback + 2 + ((n - (lookback + 2)) / 4) * 4;

    let mut i = lookback + 2;
    while i < main_loop_end {
        // Unroll 4 iterations - process 4 samples per loop iteration
        // This improves ILP, reduces branch overhead, and helps prefetching

        // Iteration 1
        let old_change_0 = (data[i - period] - data[i - period - 1]).abs();
        let new_change_0 = (data[i] - data[i - 1]).abs();
        volatility = volatility - old_change_0 + new_change_0;
        let change_0 = (data[i] - data[i - period]).abs();
        let er_0 = if volatility > 0.0 { change_0 / volatility } else { 0.0 };
        let sc_raw_0 = sc_diff.mul_add(er_0, slow_sc);
        let sc_0 = sc_raw_0 * sc_raw_0;
        kama = (data[i] - kama).mul_add(sc_0, kama);
        output[i] = kama;

        // Iteration 2
        let old_change_1 = (data[i + 1 - period] - data[i + 1 - period - 1]).abs();
        let new_change_1 = (data[i + 1] - data[i]).abs();
        volatility = volatility - old_change_1 + new_change_1;
        let change_1 = (data[i + 1] - data[i + 1 - period]).abs();
        let er_1 = if volatility > 0.0 { change_1 / volatility } else { 0.0 };
        let sc_raw_1 = sc_diff.mul_add(er_1, slow_sc);
        let sc_1 = sc_raw_1 * sc_raw_1;
        kama = (data[i + 1] - kama).mul_add(sc_1, kama);
        output[i + 1] = kama;

        // Iteration 3
        let old_change_2 = (data[i + 2 - period] - data[i + 2 - period - 1]).abs();
        let new_change_2 = (data[i + 2] - data[i + 1]).abs();
        volatility = volatility - old_change_2 + new_change_2;
        let change_2 = (data[i + 2] - data[i + 2 - period]).abs();
        let er_2 = if volatility > 0.0 { change_2 / volatility } else { 0.0 };
        let sc_raw_2 = sc_diff.mul_add(er_2, slow_sc);
        let sc_2 = sc_raw_2 * sc_raw_2;
        kama = (data[i + 2] - kama).mul_add(sc_2, kama);
        output[i + 2] = kama;

        // Iteration 4
        let old_change_3 = (data[i + 3 - period] - data[i + 3 - period - 1]).abs();
        let new_change_3 = (data[i + 3] - data[i + 2]).abs();
        volatility = volatility - old_change_3 + new_change_3;
        let change_3 = (data[i + 3] - data[i + 3 - period]).abs();
        let er_3 = if volatility > 0.0 { change_3 / volatility } else { 0.0 };
        let sc_raw_3 = sc_diff.mul_add(er_3, slow_sc);
        let sc_3 = sc_raw_3 * sc_raw_3;
        kama = (data[i + 3] - kama).mul_add(sc_3, kama);
        output[i + 3] = kama;

        i += 4;
    }

    // Handle remaining iterations (0-3 samples)
    while i < n {
        let old_change = (data[i - period] - data[i - period - 1]).abs();
        let new_change = (data[i] - data[i - 1]).abs();
        volatility = volatility - old_change + new_change;
        let change = (data[i] - data[i - period]).abs();
        let er = if volatility > 0.0 { change / volatility } else { 0.0 };
        let sc_raw = sc_diff.mul_add(er, slow_sc);
        let sc = sc_raw * sc_raw;
        kama = (data[i] - kama).mul_add(sc, kama);
        output[i] = kama;
        i += 1;
    }

    Ok(())
}

/// f32-specialized KAMA implementation - uses f64 accumulators for precision
#[inline]
fn kama_full_into_f32(
    data: &[f32],
    period: usize,
    fast_period: usize,
    slow_period: usize,
    output: &mut [f32],
) -> Result<()> {
    let n = data.len();
    let lookback = period - 1;

    // Fill lookback period with NaN
    output[..lookback].fill(f32::NAN);

    // Calculate smoothing constants using f64 for precision
    let fast_sc = 2.0 / (fast_period + 1) as f64;
    let slow_sc = 2.0 / (slow_period + 1) as f64;
    let sc_diff = fast_sc - slow_sc;

    // Initialize KAMA
    let mut kama = data[lookback] as f64;
    output[lookback] = kama as f32;

    if lookback + 1 >= n {
        return Ok(());
    }

    // Initialize rolling volatility for first window (compute on-the-fly with f64 precision)
    let mut volatility = 0.0_f64;
    let start = lookback + 2 - period;
    for j in start..=(lookback + 1) {
        volatility += (data[j] as f64 - data[j - 1] as f64).abs();
    }

    // First iteration (no volatility update needed)
    if lookback + 1 < n {
        let i = lookback + 1;
        let change = (data[i] as f64 - data[i - period] as f64).abs();
        let er = if volatility > 0.0 {
            change / volatility
        } else {
            0.0
        };
        let sc_raw = sc_diff.mul_add(er, slow_sc);
        let sc = sc_raw * sc_raw;
        kama = (data[i] as f64 - kama).mul_add(sc, kama);
        output[i] = kama as f32;
    }

    // Steady-state loop with 4x unrolling
    let main_loop_end = lookback + 2 + ((n - (lookback + 2)) / 4) * 4;

    let mut i = lookback + 2;
    while i < main_loop_end {
        // Unroll 4 iterations - process 4 samples per loop iteration
        // This improves ILP, reduces branch overhead, and helps prefetching

        // Iteration 1
        let old_change_0 = (data[i - period] as f64 - data[i - period - 1] as f64).abs();
        let new_change_0 = (data[i] as f64 - data[i - 1] as f64).abs();
        volatility = volatility - old_change_0 + new_change_0;
        let change_0 = (data[i] as f64 - data[i - period] as f64).abs();
        let er_0 = if volatility > 0.0 { change_0 / volatility } else { 0.0 };
        let sc_raw_0 = sc_diff.mul_add(er_0, slow_sc);
        let sc_0 = sc_raw_0 * sc_raw_0;
        kama = (data[i] as f64 - kama).mul_add(sc_0, kama);
        output[i] = kama as f32;

        // Iteration 2
        let old_change_1 = (data[i + 1 - period] as f64 - data[i + 1 - period - 1] as f64).abs();
        let new_change_1 = (data[i + 1] as f64 - data[i] as f64).abs();
        volatility = volatility - old_change_1 + new_change_1;
        let change_1 = (data[i + 1] as f64 - data[i + 1 - period] as f64).abs();
        let er_1 = if volatility > 0.0 { change_1 / volatility } else { 0.0 };
        let sc_raw_1 = sc_diff.mul_add(er_1, slow_sc);
        let sc_1 = sc_raw_1 * sc_raw_1;
        kama = (data[i + 1] as f64 - kama).mul_add(sc_1, kama);
        output[i + 1] = kama as f32;

        // Iteration 3
        let old_change_2 = (data[i + 2 - period] as f64 - data[i + 2 - period - 1] as f64).abs();
        let new_change_2 = (data[i + 2] as f64 - data[i + 1] as f64).abs();
        volatility = volatility - old_change_2 + new_change_2;
        let change_2 = (data[i + 2] as f64 - data[i + 2 - period] as f64).abs();
        let er_2 = if volatility > 0.0 { change_2 / volatility } else { 0.0 };
        let sc_raw_2 = sc_diff.mul_add(er_2, slow_sc);
        let sc_2 = sc_raw_2 * sc_raw_2;
        kama = (data[i + 2] as f64 - kama).mul_add(sc_2, kama);
        output[i + 2] = kama as f32;

        // Iteration 4
        let old_change_3 = (data[i + 3 - period] as f64 - data[i + 3 - period - 1] as f64).abs();
        let new_change_3 = (data[i + 3] as f64 - data[i + 2] as f64).abs();
        volatility = volatility - old_change_3 + new_change_3;
        let change_3 = (data[i + 3] as f64 - data[i + 3 - period] as f64).abs();
        let er_3 = if volatility > 0.0 { change_3 / volatility } else { 0.0 };
        let sc_raw_3 = sc_diff.mul_add(er_3, slow_sc);
        let sc_3 = sc_raw_3 * sc_raw_3;
        kama = (data[i + 3] as f64 - kama).mul_add(sc_3, kama);
        output[i + 3] = kama as f32;

        i += 4;
    }

    // Handle remaining iterations (0-3 samples)
    while i < n {
        let old_change = (data[i - period] as f64 - data[i - period - 1] as f64).abs();
        let new_change = (data[i] as f64 - data[i - 1] as f64).abs();
        volatility = volatility - old_change + new_change;
        let change = (data[i] as f64 - data[i - period] as f64).abs();
        let er = if volatility > 0.0 { change / volatility } else { 0.0 };
        let sc_raw = sc_diff.mul_add(er, slow_sc);
        let sc = sc_raw * sc_raw;
        kama = (data[i] as f64 - kama).mul_add(sc, kama);
        output[i] = kama as f32;
        i += 1;
    }

    Ok(())
}

/// Computes KAMA with default fast/slow periods (2/30).
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The efficiency ratio period (typically 10)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of KAMA values
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::kama;
///
/// let prices: Vec<f64> = vec![10.0, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 13.5, 14.0, 13.5, 14.5, 15.0];
/// let result = kama(&prices, 10).unwrap();
/// assert!(result[9].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn kama<T: SeriesElement + 'static>(data: &[T], period: usize) -> Result<Vec<T>> {
    use std::any::TypeId;

    // Wrapper optimization: avoid initializing Vec for f64/f32
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let mut output: Vec<f64> = Vec::with_capacity(data.len());
        unsafe { output.set_len(data.len()); }
        kama_into(data, period, unsafe { std::mem::transmute(output.as_mut_slice()) })?;
        return Ok(unsafe { std::mem::transmute(output) });
    }

    if TypeId::of::<T>() == TypeId::of::<f32>() {
        let mut output: Vec<f32> = Vec::with_capacity(data.len());
        unsafe { output.set_len(data.len()); }
        kama_into(data, period, unsafe { std::mem::transmute(output.as_mut_slice()) })?;
        return Ok(unsafe { std::mem::transmute(output) });
    }

    // Generic path: safe initialization
    let mut output = vec![T::nan(); data.len()];
    kama_into(data, period, &mut output)?;
    Ok(output)
}

/// Computes KAMA with custom fast/slow periods.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The efficiency ratio period
/// * `fast_period` - Fast EMA period (typically 2)
/// * `slow_period` - Slow EMA period (typically 30)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of KAMA values
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn kama_full<T: SeriesElement + 'static>(
    data: &[T],
    period: usize,
    fast_period: usize,
    slow_period: usize,
) -> Result<Vec<T>> {
    use std::any::TypeId;

    // Wrapper optimization: avoid initializing Vec for f64/f32
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let mut output: Vec<f64> = Vec::with_capacity(data.len());
        unsafe { output.set_len(data.len()); }
        kama_full_into(data, period, fast_period, slow_period, unsafe { std::mem::transmute(output.as_mut_slice()) })?;
        return Ok(unsafe { std::mem::transmute(output) });
    }

    if TypeId::of::<T>() == TypeId::of::<f32>() {
        let mut output: Vec<f32> = Vec::with_capacity(data.len());
        unsafe { output.set_len(data.len()); }
        kama_full_into(data, period, fast_period, slow_period, unsafe { std::mem::transmute(output.as_mut_slice()) })?;
        return Ok(unsafe { std::mem::transmute(output) });
    }

    // Generic path: safe initialization
    let mut output = vec![T::nan(); data.len()];
    kama_full_into(data, period, fast_period, slow_period, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;
    use num_traits::Float;

    fn approx_eq<T: Float>(a: T, b: T, epsilon: T) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        (a - b).abs() < epsilon
    }

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_kama_lookback() {
        assert_eq!(kama_lookback(1), 0);
        assert_eq!(kama_lookback(2), 1);
        assert_eq!(kama_lookback(10), 9);
        assert_eq!(kama_lookback(0), 0);
    }

    #[test]
    fn test_kama_min_len() {
        assert_eq!(kama_min_len(1), 1);
        assert_eq!(kama_min_len(2), 2);
        assert_eq!(kama_min_len(10), 10);
    }

    #[test]
    fn test_kama_empty_input() {
        let data: Vec<f64> = vec![];
        let result = kama(&data, 10);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_kama_zero_period() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = kama(&data, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_kama_insufficient_data() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0];
        let result = kama(&data, 10);
        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                indicator: "kama",
                required: 10,
                actual: 3,
            })
        ));
    }

    #[test]
    fn test_kama_output_length_equals_input_length() {
        let data: Vec<f64> = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let result = kama(&data, 10).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_kama_nan_count() {
        let data: Vec<f64> = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let period = 10;
        let result = kama(&data, period).unwrap();

        // Count NaN values - should be period - 1 = 9
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, period - 1);
    }

    #[test]
    fn test_kama_valid_count() {
        let data: Vec<f64> = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let period = 10;
        let result = kama(&data, period).unwrap();

        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        assert_eq!(valid_count, data.len() - (period - 1));
    }

    #[test]
    fn test_kama_first_value_equals_data() {
        // First KAMA value should equal the data value at that point
        let data: Vec<f64> = vec![
            10.0, 11.0, 12.0, 11.0, 12.0, 13.0, 12.0, 13.0, 14.0, 13.0, 14.0, 15.0,
        ];
        let result = kama(&data, 10).unwrap();

        // First valid KAMA should be at index 9 (lookback = 9)
        assert!(approx_eq(result[9], data[9], EPSILON));
    }

    #[test]
    fn test_kama_trending_market() {
        // In a strongly trending market (high ER), KAMA should follow price closely
        let data: Vec<f64> = vec![
            10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0,
            24.0,
        ];
        let result = kama(&data, 10).unwrap();

        // KAMA should increase with the trend
        for i in 10..result.len() {
            assert!(result[i] > result[i - 1], "KAMA should increase in uptrend");
        }
    }

    #[test]
    fn test_kama_sideways_market() {
        // In a sideways market (low ER), KAMA should be relatively flat
        let data: Vec<f64> = vec![
            10.0, 11.0, 10.0, 11.0, 10.0, 11.0, 10.0, 11.0, 10.0, 11.0, 10.0, 11.0, 10.0, 11.0,
            10.0,
        ];
        let result = kama(&data, 10).unwrap();

        // Calculate KAMA range after lookback
        let valid_kama: Vec<f64> = result.iter().filter(|x| !x.is_nan()).cloned().collect();
        let kama_range = valid_kama.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            - valid_kama.iter().cloned().fold(f64::INFINITY, f64::min);

        // KAMA range should be smaller than price range in sideways market
        assert!(kama_range < 1.0, "KAMA should be flat in sideways market");
    }

    #[test]
    fn test_kama_constant_values() {
        // KAMA of constant values should stay constant
        let data: Vec<f64> = vec![42.0; 15];
        let result = kama(&data, 10).unwrap();

        for i in 9..15 {
            assert!(approx_eq(result[i], 42.0, EPSILON));
        }
    }

    #[test]
    fn test_kama_period_one() {
        // With period 1, KAMA should equal the input (ER = 0 when no change period)
        let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = kama(&data, 1).unwrap();

        // First value should equal input
        assert!(approx_eq(result[0], data[0], EPSILON));
        // All values should be valid
        for i in 0..data.len() {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_kama_f32() {
        let data: Vec<f32> = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let result = kama(&data, 10).unwrap();

        assert_eq!(result.len(), data.len());

        // First 9 should be NaN
        for i in 0..9 {
            assert!(result[i].is_nan());
        }

        // Rest should be valid
        for i in 9..12 {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_kama_into_f32() {
        let data: Vec<f32> = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let mut output = vec![0.0_f32; data.len()];
        kama_into(&data, 10, &mut output).unwrap();

        for i in 0..9 {
            assert!(output[i].is_nan());
        }

        for i in 9..12 {
            assert!(output[i].is_finite());
        }
    }

    #[test]
    fn test_kama_into_insufficient_output() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut output: Vec<f64> = vec![0.0; 5]; // Too small
        let result = kama_into(&data, 5, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_kama_full_custom_periods() {
        let data: Vec<f64> = vec![
            10.0, 11.0, 12.0, 11.0, 12.0, 13.0, 12.0, 13.0, 14.0, 13.0, 14.0, 15.0,
        ];

        // With faster periods, KAMA should react more quickly
        let result_fast = kama_full(&data, 10, 2, 10).unwrap();
        let result_slow = kama_full(&data, 10, 2, 50).unwrap();

        // Both should have same number of NaN
        let fast_valid: Vec<f64> = result_fast
            .iter()
            .filter(|x| !x.is_nan())
            .cloned()
            .collect();
        let slow_valid: Vec<f64> = result_slow
            .iter()
            .filter(|x| !x.is_nan())
            .cloned()
            .collect();

        assert_eq!(fast_valid.len(), slow_valid.len());
    }
}