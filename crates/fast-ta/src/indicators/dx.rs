//! DX (Directional Movement Index) and related indicators.
//!
//! This module provides:
//! - DX: Directional Movement Index (0-100)
//! - ADXR: ADX Rating (smoothed ADX average)
//! - `PLUS_DM`: Plus Directional Movement
//! - `MINUS_DM`: Minus Directional Movement
//!
//! These are components used in calculating ADX but can be useful on their own.

use crate::error::{Error, Result};
use crate::indicators::adx::{adx, adx_lookback};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

// =============================================================================
// ADXR (ADX Rating)
// =============================================================================

/// Computes the lookback period for ADXR.
///
/// ADXR is the average of current ADX and ADX from `period` bars ago.
/// Lookback = ADX lookback + period
#[inline]
#[must_use]
pub const fn adxr_lookback(period: usize) -> usize {
    adx_lookback(period) + period
}

/// Returns the minimum input length required for ADXR calculation.
#[inline]
#[must_use]
pub const fn adxr_min_len(period: usize) -> usize {
    adxr_lookback(period) + 1
}

/// Computes ADXR and stores results in output slice.
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - ADX period (typically 14)
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
pub fn adxr_into<T: SeriesElement>(
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

    let min_len = adxr_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "adxr",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "adxr",
            required: n,
            actual: output.len(),
        });
    }

    // First compute ADX
    let adx_result = adx(high, low, close, period)?;
    let adx_values = &adx_result.adx;

    let lookback = adxr_lookback(period);
    let two = T::from_f64(2.0)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // ADXR = (ADX[i] + ADX[i - period]) / 2
    for i in lookback..n {
        let current_adx = adx_values[i];
        let past_adx = adx_values[i - period];

        if !is_invalid(current_adx) && !is_invalid(past_adx) {
            output[i] = (current_adx + past_adx) / two;
        } else {
            output[i] = T::nan();
        }
    }

    Ok(())
}

/// Computes ADXR (Average Directional Movement Rating).
///
/// ADXR is the average of current ADX and ADX from `period` bars ago.
/// It provides a smoother version of ADX.
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Period for ADX calculation (typically 14)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - ADXR values (range 0 to 100)
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn adxr<T: SeriesElement>(high: &[T], low: &[T], close: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::zero(); high.len()];
    adxr_into(high, low, close, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// DX (Directional Movement Index)
// =============================================================================

/// Computes the lookback period for DX.
/// Same as DI lookback since DX is computed from +DI and -DI.
#[inline]
#[must_use]
pub const fn dx_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for DX calculation.
#[inline]
#[must_use]
pub const fn dx_min_len(period: usize) -> usize {
    dx_lookback(period) + 1
}

/// Computes DX and stores results in output slice.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn dx_into<T: SeriesElement>(
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

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    // We need enough data for DI calculation
    let min_len = dx_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "dx",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "dx",
            required: n,
            actual: output.len(),
        });
    }

    // Compute ADX which gives us +DI and -DI
    // Note: This needs 2*period data for ADX, but for just DX we only need period+1
    // We'll handle the case where we have less than 2*period data
    if n >= 2 * period {
        let adx_result = adx(high, low, close, period)?;
        let plus_di = &adx_result.plus_di;
        let minus_di = &adx_result.minus_di;
        let hundred = T::from_f64(100.0)?;

        // Fill lookback with NaN
        for i in 0..period {
            output[i] = T::nan();
        }

        // DX = 100 * |+DI - -DI| / (+DI + -DI)
        for i in period..n {
            if !is_invalid(plus_di[i]) && !is_invalid(minus_di[i]) {
                let di_sum = plus_di[i] + minus_di[i];
                let di_diff = (plus_di[i] - minus_di[i]).abs();
                if di_sum > T::zero() {
                    output[i] = hundred * di_diff / di_sum;
                } else {
                    output[i] = T::zero();
                }
            } else {
                output[i] = T::nan();
            }
        }
    } else {
        // Not enough data for full ADX calculation
        return Err(Error::InsufficientData {
            indicator: "dx",
            required: 2 * period,
            actual: n,
        });
    }

    Ok(())
}

/// Computes DX (Directional Movement Index).
///
/// DX measures the difference between +DI and -DI relative to their sum.
/// It's the basis for ADX.
///
/// # Formula
///
/// DX = 100 * |+DI - -DI| / (+DI + -DI)
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn dx<T: SeriesElement>(high: &[T], low: &[T], close: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::zero(); high.len()];
    dx_into(high, low, close, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// PLUS_DM and MINUS_DM (Directional Movement)
// =============================================================================

/// Computes the lookback period for directional movement.
#[inline]
#[must_use]
pub const fn dm_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for DM calculation.
#[inline]
#[must_use]
pub const fn dm_min_len(period: usize) -> usize {
    dm_lookback(period) + 1
}

/// Computes `PLUS_DM` (Plus Directional Movement) with Wilder smoothing.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn plus_dm_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    if low.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", n, low.len()),
        });
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    let min_len = dm_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "plus_dm",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "plus_dm",
            required: n,
            actual: output.len(),
        });
    }

    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..period {
        output[i] = T::nan();
    }

    // Calculate initial sum of +DM for the first period
    let mut sum_plus_dm = T::zero();
    let mut nan_active = false;
    for i in 1..=period {
        if is_invalid(high[i])
            || is_invalid(high[i - 1])
            || is_invalid(low[i])
            || is_invalid(low[i - 1])
        {
            nan_active = true;
            continue;
        }
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];

        let plus_dm = if up_move > down_move && up_move > T::zero() {
            up_move
        } else {
            T::zero()
        };
        sum_plus_dm = sum_plus_dm + plus_dm;
    }

    // First smoothed value
    let mut smoothed_plus_dm = sum_plus_dm;
    if nan_active || is_invalid(smoothed_plus_dm) {
        nan_active = true;
        smoothed_plus_dm = T::nan();
        output[period] = T::nan();
    } else {
        output[period] = smoothed_plus_dm;
    }

    // Continue with Wilder smoothing
    for i in (period + 1)..n {
        if nan_active
            || is_invalid(high[i])
            || is_invalid(high[i - 1])
            || is_invalid(low[i])
            || is_invalid(low[i - 1])
        {
            nan_active = true;
            output[i] = T::nan();
            continue;
        }
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];

        let plus_dm = if up_move > down_move && up_move > T::zero() {
            up_move
        } else {
            T::zero()
        };

        // Wilder smoothing: smoothed = prev - prev/period + current
        smoothed_plus_dm = smoothed_plus_dm - smoothed_plus_dm / period_t + plus_dm;
        output[i] = smoothed_plus_dm;
    }

    Ok(())
}

/// Computes `PLUS_DM` (Plus Directional Movement).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn plus_dm<T: SeriesElement>(high: &[T], low: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::zero(); high.len()];
    plus_dm_into(high, low, period, &mut output)?;
    Ok(output)
}

/// Computes `MINUS_DM` (Minus Directional Movement) with Wilder smoothing.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn minus_dm_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    if low.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", n, low.len()),
        });
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    let min_len = dm_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "minus_dm",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "minus_dm",
            required: n,
            actual: output.len(),
        });
    }

    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..period {
        output[i] = T::nan();
    }

    // Calculate initial sum of -DM for the first period
    let mut sum_minus_dm = T::zero();
    let mut nan_active = false;
    for i in 1..=period {
        if is_invalid(high[i])
            || is_invalid(high[i - 1])
            || is_invalid(low[i])
            || is_invalid(low[i - 1])
        {
            nan_active = true;
            continue;
        }
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];

        let minus_dm = if down_move > up_move && down_move > T::zero() {
            down_move
        } else {
            T::zero()
        };
        sum_minus_dm = sum_minus_dm + minus_dm;
    }

    // First smoothed value
    let mut smoothed_minus_dm = sum_minus_dm;
    if nan_active || is_invalid(smoothed_minus_dm) {
        nan_active = true;
        smoothed_minus_dm = T::nan();
        output[period] = T::nan();
    } else {
        output[period] = smoothed_minus_dm;
    }

    // Continue with Wilder smoothing
    for i in (period + 1)..n {
        if nan_active
            || is_invalid(high[i])
            || is_invalid(high[i - 1])
            || is_invalid(low[i])
            || is_invalid(low[i - 1])
        {
            nan_active = true;
            output[i] = T::nan();
            continue;
        }
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];

        let minus_dm = if down_move > up_move && down_move > T::zero() {
            down_move
        } else {
            T::zero()
        };

        // Wilder smoothing: smoothed = prev - prev/period + current
        smoothed_minus_dm = smoothed_minus_dm - smoothed_minus_dm / period_t + minus_dm;
        output[i] = smoothed_minus_dm;
    }

    Ok(())
}

/// Computes `MINUS_DM` (Minus Directional Movement).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn minus_dm<T: SeriesElement>(high: &[T], low: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::zero(); high.len()];
    minus_dm_into(high, low, period, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    // ADXR Tests
    #[test]
    fn test_adxr_lookback() {
        // adx_lookback(14) = 27, + 14 = 41
        assert_eq!(adxr_lookback(14), 41);
        assert_eq!(adxr_lookback(5), 14); // 9 + 5
    }

    #[test]
    fn test_adxr_min_len() {
        assert_eq!(adxr_min_len(14), 42);
        assert_eq!(adxr_min_len(5), 15);
    }

    #[test]
    fn test_adxr_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];
        let result = adxr(&high, &low, &close, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_adxr_output_length() {
        let n = 20;
        let high: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let low: Vec<f64> = (0..n).map(|i| 98.0 + i as f64).collect();
        let close: Vec<f64> = (0..n).map(|i| 99.0 + i as f64).collect();
        let result = adxr(&high, &low, &close, 5).unwrap();
        assert_eq!(result.len(), n);
    }

    #[test]
    fn test_adxr_lookback_nan() {
        let n = 20;
        let high: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let low: Vec<f64> = (0..n).map(|i| 98.0 + i as f64).collect();
        let close: Vec<f64> = (0..n).map(|i| 99.0 + i as f64).collect();
        let result = adxr(&high, &low, &close, 5).unwrap();

        let lookback = adxr_lookback(5);
        for i in 0..lookback {
            assert!(result[i].is_nan(), "adxr[{}] should be NaN", i);
        }
        for i in lookback..n {
            assert!(result[i].is_finite(), "adxr[{}] should be finite", i);
        }
    }

    // DX Tests
    #[test]
    fn test_dx_lookback() {
        assert_eq!(dx_lookback(14), 14);
        assert_eq!(dx_lookback(5), 5);
    }

    #[test]
    fn test_dx_min_len() {
        assert_eq!(dx_min_len(14), 15);
        assert_eq!(dx_min_len(5), 6);
    }

    #[test]
    fn test_dx_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];
        let result = dx(&high, &low, &close, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_dx_output_length() {
        let n = 20;
        let high: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let low: Vec<f64> = (0..n).map(|i| 98.0 + i as f64).collect();
        let close: Vec<f64> = (0..n).map(|i| 99.0 + i as f64).collect();
        let result = dx(&high, &low, &close, 5).unwrap();
        assert_eq!(result.len(), n);
    }

    #[test]
    fn test_dx_range() {
        let n = 20;
        let high: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let low: Vec<f64> = (0..n).map(|i| 98.0 + i as f64).collect();
        let close: Vec<f64> = (0..n).map(|i| 99.0 + i as f64).collect();
        let result = dx(&high, &low, &close, 5).unwrap();

        for i in dx_lookback(5)..n {
            if result[i].is_finite() {
                assert!(
                    result[i] >= 0.0 && result[i] <= 100.0,
                    "dx[{}] = {} should be in [0, 100]",
                    i,
                    result[i]
                );
            }
        }
    }

    // PLUS_DM Tests
    #[test]
    fn test_plus_dm_lookback() {
        assert_eq!(dm_lookback(14), 14);
        assert_eq!(dm_lookback(5), 5);
    }

    #[test]
    fn test_plus_dm_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let result = plus_dm(&high, &low, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_plus_dm_output_length() {
        let n = 20;
        let high: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let low: Vec<f64> = (0..n).map(|i| 98.0 + i as f64).collect();
        let result = plus_dm(&high, &low, 5).unwrap();
        assert_eq!(result.len(), n);
    }

    #[test]
    fn test_plus_dm_uptrend() {
        // In uptrend, plus_dm should be positive
        let high: Vec<f64> = (0..15).map(|i| 100.0 + i as f64 * 2.0).collect();
        let low: Vec<f64> = (0..15).map(|i| 99.0 + i as f64 * 2.0).collect();
        let result = plus_dm(&high, &low, 5).unwrap();

        for i in dm_lookback(5)..result.len() {
            assert!(
                result[i] > 0.0,
                "plus_dm[{}] should be positive in uptrend",
                i
            );
        }
    }

    // MINUS_DM Tests
    #[test]
    fn test_minus_dm_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let result = minus_dm(&high, &low, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_minus_dm_output_length() {
        let n = 20;
        let high: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let low: Vec<f64> = (0..n).map(|i| 98.0 + i as f64).collect();
        let result = minus_dm(&high, &low, 5).unwrap();
        assert_eq!(result.len(), n);
    }

    #[test]
    fn test_minus_dm_downtrend() {
        // In downtrend, minus_dm should be positive
        let high: Vec<f64> = (0..15).map(|i| 200.0 - i as f64 * 2.0).collect();
        let low: Vec<f64> = (0..15).map(|i| 199.0 - i as f64 * 2.0).collect();
        let result = minus_dm(&high, &low, 5).unwrap();

        for i in dm_lookback(5)..result.len() {
            assert!(
                result[i] > 0.0,
                "minus_dm[{}] should be positive in downtrend",
                i
            );
        }
    }

    #[test]
    fn test_dm_length_mismatch() {
        let high: Vec<f64> = vec![10.0; 15];
        let low: Vec<f64> = vec![9.0; 10]; // Different length
        let result = plus_dm(&high, &low, 5);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_dm_invalid_period() {
        let high: Vec<f64> = vec![10.0; 15];
        let low: Vec<f64> = vec![9.0; 15];
        let result = plus_dm(&high, &low, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }
}
