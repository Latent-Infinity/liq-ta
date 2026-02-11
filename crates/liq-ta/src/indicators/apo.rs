//! APO (Absolute Price Oscillator) and PPO (Percentage Price Oscillator) indicators.
//!
//! These oscillators measure the difference between two exponential moving averages:
//! - APO: Absolute difference (`fast_ema` - `slow_ema`)
//! - PPO: Percentage difference ((`fast_ema` - `slow_ema`) / `slow_ema`) * 100
//!
//! # Formulas
//!
//! ```text
//! APO = EMA(price, fast_period) - EMA(price, slow_period)
//! PPO = ((EMA(price, fast_period) - EMA(price, slow_period)) / EMA(price, slow_period)) * 100
//! ```
//!
//! # Lookback
//!
//! Lookback = `slow_period` - 1 (same as MACD line lookback)

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

// =============================================================================
// APO - Absolute Price Oscillator
// =============================================================================

/// Computes the lookback period for APO.
#[inline]
#[must_use]
pub const fn apo_lookback(slow_period: usize) -> usize {
    if slow_period == 0 { 0 } else { slow_period - 1 }
}

/// Returns the minimum input length required for APO calculation.
#[inline]
#[must_use]
pub const fn apo_min_len(slow_period: usize) -> usize {
    slow_period
}

/// Computes APO (Absolute Price Oscillator) and stores results in output.
///
/// APO = `fast_ema` - `slow_ema`
///
/// # Arguments
///
/// * `data` - Input price data
/// * `fast_period` - Fast EMA period (typically 12)
/// * `slow_period` - Slow EMA period (typically 26)
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn apo_into<T: SeriesElement>(
    data: &[T],
    fast_period: usize,
    slow_period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if fast_period == 0 || slow_period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be at least 1",
        });
    }

    if fast_period >= slow_period {
        return Err(Error::InvalidPeriod {
            period: fast_period,
            reason: "fast_period must be less than slow_period",
        });
    }

    let n = data.len();
    let min_len = apo_min_len(slow_period);

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "apo",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "apo",
            required: n,
            actual: output.len(),
        });
    }

    // Calculate fast and slow EMAs
    let fast_alpha = T::from_f64(2.0 / (fast_period as f64 + 1.0))?;
    let slow_alpha = T::from_f64(2.0 / (slow_period as f64 + 1.0))?;
    let one = T::one();

    let slow_lookback = slow_period - 1;

    // Calculate initial SMA for fast EMA
    let mut fast_sum = T::zero();
    for i in 0..fast_period {
        fast_sum = fast_sum + data[i];
    }
    let mut fast_ema = fast_sum / T::from_usize(fast_period)?;

    // Advance fast EMA to slow_period - 1 so it aligns with slow EMA start.
    for i in fast_period..slow_period {
        fast_ema = fast_alpha * data[i] + (one - fast_alpha) * fast_ema;
    }

    // Calculate initial SMA for slow EMA
    let mut slow_sum = T::zero();
    for i in 0..slow_period {
        slow_sum = slow_sum + data[i];
    }
    let mut slow_ema = slow_sum / T::from_usize(slow_period)?;

    // Fill lookback with NaN
    for i in 0..slow_lookback {
        output[i] = T::nan();
    }

    // First valid value at slow_lookback
    output[slow_lookback] = fast_ema - slow_ema;

    // Continue from slow_lookback + 1
    for i in (slow_lookback + 1)..n {
        fast_ema = fast_alpha * data[i] + (one - fast_alpha) * fast_ema;
        slow_ema = slow_alpha * data[i] + (one - slow_alpha) * slow_ema;
        output[i] = fast_ema - slow_ema;
    }

    Ok(())
}

/// Computes APO (Absolute Price Oscillator).
///
/// APO = `fast_ema` - `slow_ema`
///
/// # Arguments
///
/// * `data` - Input price data
/// * `fast_period` - Fast EMA period (default: 12)
/// * `slow_period` - Slow EMA period (default: 26)
///
/// # Example
///
/// ```
/// use liq_ta::indicators::apo;
///
/// let mut prices: Vec<f64> = Vec::with_capacity(30);
/// for x in 1..=30 {
///     prices.push(x as f64);
/// }
/// let result = apo(&prices, 12, 26).unwrap();
/// assert!(result[25].is_finite()); // First valid value at index 25
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn apo<T: SeriesElement>(data: &[T], fast_period: usize, slow_period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    apo_into(data, fast_period, slow_period, &mut output)?;
    Ok(output)
}

// =============================================================================
// PPO - Percentage Price Oscillator
// =============================================================================

/// Computes the lookback period for PPO.
#[inline]
#[must_use]
pub const fn ppo_lookback(slow_period: usize) -> usize {
    if slow_period == 0 { 0 } else { slow_period - 1 }
}

/// Returns the minimum input length required for PPO calculation.
#[inline]
#[must_use]
pub const fn ppo_min_len(slow_period: usize) -> usize {
    slow_period
}

/// Computes PPO (Percentage Price Oscillator) and stores results in output.
///
/// PPO = ((`fast_ema` - `slow_ema`) / `slow_ema`) * 100
///
/// # Arguments
///
/// * `data` - Input price data
/// * `fast_period` - Fast EMA period (typically 12)
/// * `slow_period` - Slow EMA period (typically 26)
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn ppo_into<T: SeriesElement>(
    data: &[T],
    fast_period: usize,
    slow_period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if fast_period == 0 || slow_period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be at least 1",
        });
    }

    if fast_period >= slow_period {
        return Err(Error::InvalidPeriod {
            period: fast_period,
            reason: "fast_period must be less than slow_period",
        });
    }

    let n = data.len();
    let min_len = ppo_min_len(slow_period);

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "ppo",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "ppo",
            required: n,
            actual: output.len(),
        });
    }

    let hundred = T::from_f64(100.0)?;

    // Calculate fast and slow EMAs
    let fast_alpha = T::from_f64(2.0 / (fast_period as f64 + 1.0))?;
    let slow_alpha = T::from_f64(2.0 / (slow_period as f64 + 1.0))?;
    let one = T::one();

    let slow_lookback = slow_period - 1;

    // Fill lookback with NaN
    for i in 0..slow_lookback {
        output[i] = T::nan();
    }

    // Calculate initial SMAs
    let mut fast_sum = T::zero();
    for i in 0..fast_period {
        fast_sum = fast_sum + data[i];
    }
    let mut fast_ema = fast_sum / T::from_usize(fast_period)?;

    // Continue fast EMA from fast_period to slow_period
    for i in fast_period..slow_period {
        fast_ema = fast_alpha * data[i] + (one - fast_alpha) * fast_ema;
    }

    let mut slow_sum = T::zero();
    for i in 0..slow_period {
        slow_sum = slow_sum + data[i];
    }
    let mut slow_ema = slow_sum / T::from_usize(slow_period)?;

    // First valid value
    if slow_ema == T::zero() {
        output[slow_lookback] = T::nan();
    } else {
        output[slow_lookback] = ((fast_ema - slow_ema) / slow_ema) * hundred;
    }

    // Continue calculating
    for i in (slow_lookback + 1)..n {
        fast_ema = fast_alpha * data[i] + (one - fast_alpha) * fast_ema;
        slow_ema = slow_alpha * data[i] + (one - slow_alpha) * slow_ema;
        if slow_ema == T::zero() {
            output[i] = T::nan();
        } else {
            output[i] = ((fast_ema - slow_ema) / slow_ema) * hundred;
        }
    }

    Ok(())
}

/// Computes PPO (Percentage Price Oscillator).
///
/// PPO = ((`fast_ema` - `slow_ema`) / `slow_ema`) * 100
///
/// # Arguments
///
/// * `data` - Input price data
/// * `fast_period` - Fast EMA period (default: 12)
/// * `slow_period` - Slow EMA period (default: 26)
///
/// # Example
///
/// ```
/// use liq_ta::indicators::ppo;
///
/// let mut prices: Vec<f64> = Vec::with_capacity(30);
/// for x in 1..=30 {
///     prices.push(x as f64);
/// }
/// let result = ppo(&prices, 12, 26).unwrap();
/// assert!(result[25].is_finite()); // First valid value at index 25
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn ppo<T: SeriesElement>(data: &[T], fast_period: usize, slow_period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    ppo_into(data, fast_period, slow_period, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    // =========================================================================
    // APO Tests
    // =========================================================================

    #[test]
    fn test_apo_lookback() {
        assert_eq!(apo_lookback(26), 25);
        assert_eq!(apo_lookback(12), 11);
    }

    #[test]
    fn test_apo_min_len() {
        assert_eq!(apo_min_len(26), 26);
        assert_eq!(apo_min_len(12), 12);
    }

    #[test]
    fn test_apo_empty_input() {
        let data: Vec<f64> = vec![];
        let result = apo(&data, 12, 26);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_apo_invalid_period_zero() {
        let data: Vec<f64> = (1..=30).map(|x| x as f64).collect();
        let result = apo(&data, 0, 26);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_apo_invalid_fast_ge_slow() {
        let data: Vec<f64> = (1..=30).map(|x| x as f64).collect();
        let result = apo(&data, 26, 12);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));

        let result2 = apo(&data, 12, 12);
        assert!(matches!(result2, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_apo_insufficient_data() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let result = apo(&data, 12, 26);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_apo_output_length() {
        let data: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        let result = apo(&data, 12, 26).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_apo_nan_count() {
        let data: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        let result = apo(&data, 12, 26).unwrap();
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, apo_lookback(26));
    }

    #[test]
    fn test_apo_constant_price() {
        let data: Vec<f64> = vec![50.0; 50];
        let result = apo(&data, 12, 26).unwrap();

        // For constant price, both EMAs equal the price, so APO = 0
        for i in apo_lookback(26)..result.len() {
            assert!((result[i] - 0.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_apo_uptrend() {
        let data: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        let result = apo(&data, 12, 26).unwrap();

        // In uptrend, fast EMA > slow EMA, so APO > 0
        for i in (apo_lookback(26) + 5)..result.len() {
            assert!(
                result[i] > 0.0,
                "APO[{}] = {} should be positive",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_apo_downtrend() {
        let data: Vec<f64> = (1..=50).rev().map(|x| x as f64).collect();
        let result = apo(&data, 12, 26).unwrap();

        // In downtrend, fast EMA < slow EMA, so APO < 0
        for i in (apo_lookback(26) + 5)..result.len() {
            assert!(
                result[i] < 0.0,
                "APO[{}] = {} should be negative",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_apo_into() {
        let data: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; data.len()];
        apo_into(&data, 12, 26, &mut output).unwrap();

        let lookback = apo_lookback(26);
        for i in 0..lookback {
            assert!(output[i].is_nan());
        }
        for i in lookback..output.len() {
            assert!(output[i].is_finite());
        }
    }

    #[test]
    fn test_apo_into_buffer_too_small() {
        let data: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 20]; // Too small
        let result = apo_into(&data, 12, 26, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    // =========================================================================
    // PPO Tests
    // =========================================================================

    #[test]
    fn test_ppo_lookback() {
        assert_eq!(ppo_lookback(26), 25);
        assert_eq!(ppo_lookback(12), 11);
    }

    #[test]
    fn test_ppo_min_len() {
        assert_eq!(ppo_min_len(26), 26);
        assert_eq!(ppo_min_len(12), 12);
    }

    #[test]
    fn test_ppo_empty_input() {
        let data: Vec<f64> = vec![];
        let result = ppo(&data, 12, 26);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_ppo_invalid_period_zero() {
        let data: Vec<f64> = (1..=30).map(|x| x as f64).collect();
        let result = ppo(&data, 0, 26);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_ppo_invalid_fast_ge_slow() {
        let data: Vec<f64> = (1..=30).map(|x| x as f64).collect();
        let result = ppo(&data, 26, 12);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_ppo_output_length() {
        let data: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        let result = ppo(&data, 12, 26).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_ppo_nan_count() {
        let data: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        let result = ppo(&data, 12, 26).unwrap();
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, ppo_lookback(26));
    }

    #[test]
    fn test_ppo_constant_price() {
        let data: Vec<f64> = vec![50.0; 50];
        let result = ppo(&data, 12, 26).unwrap();

        // For constant price, both EMAs equal the price, so PPO = 0
        for i in ppo_lookback(26)..result.len() {
            assert!((result[i] - 0.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_ppo_uptrend() {
        let data: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        let result = ppo(&data, 12, 26).unwrap();

        // In uptrend, fast EMA > slow EMA, so PPO > 0
        for i in (ppo_lookback(26) + 5)..result.len() {
            assert!(
                result[i] > 0.0,
                "PPO[{}] = {} should be positive",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_ppo_downtrend() {
        let data: Vec<f64> = (1..=50).rev().map(|x| x as f64).collect();
        let result = ppo(&data, 12, 26).unwrap();

        // In downtrend, fast EMA < slow EMA, so PPO < 0
        for i in (ppo_lookback(26) + 5)..result.len() {
            assert!(
                result[i] < 0.0,
                "PPO[{}] = {} should be negative",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_ppo_into() {
        let data: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; data.len()];
        ppo_into(&data, 12, 26, &mut output).unwrap();

        let lookback = ppo_lookback(26);
        for i in 0..lookback {
            assert!(output[i].is_nan());
        }
        for i in lookback..output.len() {
            assert!(output[i].is_finite());
        }
    }

    // =========================================================================
    // APO/PPO Relationship Tests
    // =========================================================================

    #[test]
    fn test_apo_ppo_relationship() {
        // PPO should equal (APO / slow_ema) * 100
        // For now just verify they have the same sign
        let data: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        let apo_result = apo(&data, 12, 26).unwrap();
        let ppo_result = ppo(&data, 12, 26).unwrap();

        for i in apo_lookback(26)..data.len() {
            // Same sign test
            assert_eq!(
                apo_result[i].signum(),
                ppo_result[i].signum(),
                "APO and PPO should have same sign at index {}",
                i
            );
        }
    }

    #[test]
    fn test_apo_f32() {
        let data: Vec<f32> = (1..=50).map(|x| x as f32).collect();
        let result = apo(&data, 12, 26).unwrap();
        assert!(result[apo_lookback(26)].is_finite());
    }

    #[test]
    fn test_ppo_f32() {
        let data: Vec<f32> = (1..=50).map(|x| x as f32).collect();
        let result = ppo(&data, 12, 26).unwrap();
        assert!(result[ppo_lookback(26)].is_finite());
    }
}
