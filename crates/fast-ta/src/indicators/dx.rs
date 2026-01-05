//! DX (Directional Movement Index) and related indicators.
//!
//! This module provides:
//! - DX: Directional Movement Index (0-100)
//! - ADXR: ADX Rating (smoothed ADX average)
//! - `PLUS_DM`: Plus Directional Movement
//! - `MINUS_DM`: Minus Directional Movement
//!
//! These are components used in calculating ADX but can be useful on their own.
//!
//! # Optimization Note
//!
//! DX shares the +DI/-DI computation with ADX but without the final ADX smoothing step.
//! This implementation computes +DI and -DI directly, avoiding the overhead of full ADX
//! computation and reducing the minimum data requirement from `2*period` to `period+1`.

use crate::error::{Error, Result};
use crate::indicators::adx::{adx, adx_lookback};
use crate::traits::SeriesElement;

// =============================================================================
// Shared DI computation helpers (inline for DX optimization)
// =============================================================================

/// Computes True Range for a single bar.
/// Returns NaN if any input is non-finite (NaN or Infinity).
#[inline]
fn compute_true_range<T: SeriesElement>(high: T, low: T, prev_close: T) -> T {
    // Per project policy: non-finite inputs (NaN or Infinity) produce NaN output
    if !high.is_finite() || !low.is_finite() || !prev_close.is_finite() {
        return T::nan();
    }

    let hl = high - low;
    let hc = (high - prev_close).abs();
    let lc = (low - prev_close).abs();

    hl.max(hc).max(lc)
}

/// Computes directional movement (+DM and -DM) for a single bar.
/// Returns (NaN, NaN) if any input is non-finite (NaN or Infinity).
#[inline]
fn compute_directional_movement<T: SeriesElement>(
    high: T,
    prev_high: T,
    low: T,
    prev_low: T,
) -> (T, T) {
    // Per project policy: non-finite inputs (NaN or Infinity) produce NaN output
    if !high.is_finite() || !prev_high.is_finite() || !low.is_finite() || !prev_low.is_finite() {
        return (T::nan(), T::nan());
    }

    let up_move = high - prev_high;
    let down_move = prev_low - low;

    let plus_dm = if up_move > down_move && up_move > T::zero() {
        up_move
    } else {
        T::zero()
    };

    let minus_dm = if down_move > up_move && down_move > T::zero() {
        down_move
    } else {
        T::zero()
    };

    (plus_dm, minus_dm)
}

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

    // Fill lookback with NaN using efficient slice.fill()
    output[..lookback].fill(T::nan());

    // ADXR = (ADX[i] + ADX[i - period]) / 2
    // IEEE 754: NaN + x = NaN, NaN / 2 = NaN, so NaN propagates naturally
    for i in lookback..n {
        let current_adx = adx_values[i];
        let past_adx = adx_values[i - period];
        output[i] = (current_adx + past_adx) / two;
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
/// This implementation computes +DI and -DI directly using shared computation
/// patterns with ADX, avoiding the overhead of the full ADX smoothing step.
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

    if low.len() != n || close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "high has {} elements, low has {}, close has {}",
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

    // We need enough data for DI calculation (period + 1)
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

    let period_t = T::from_usize(period)?;
    let hundred = T::hundred();
    let alpha = T::one() / period_t; // Wilder smoothing factor (1/period) for difference form

    // Fill lookback with NaN using efficient slice.fill()
    output[..period].fill(T::nan());

    // Compute initial sums of TR, +DM, -DM for the first period
    // TR and DM start at index 1 (needs previous bar)
    let mut sum_tr = T::zero();
    let mut sum_plus_dm = T::zero();
    let mut sum_minus_dm = T::zero();

    for i in 1..=period {
        let tr = compute_true_range(high[i], low[i], close[i - 1]);
        let (plus_dm, minus_dm) =
            compute_directional_movement(high[i], high[i - 1], low[i], low[i - 1]);
        sum_tr = sum_tr + tr;
        sum_plus_dm = sum_plus_dm + plus_dm;
        sum_minus_dm = sum_minus_dm + minus_dm;
    }

    // Initialize smoothed values
    let mut smoothed_tr = sum_tr;
    let mut smoothed_plus_dm = sum_plus_dm;
    let mut smoothed_minus_dm = sum_minus_dm;

    // Compute first DX at index = period
    // Note: NaN comparisons are always false, so we must check is_finite() explicitly
    // to ensure NaN propagates rather than defaulting to zero
    let plus_di = if smoothed_tr.is_finite() && smoothed_tr > T::zero() {
        hundred * smoothed_plus_dm / smoothed_tr
    } else if !smoothed_tr.is_finite() || !smoothed_plus_dm.is_finite() {
        T::nan()
    } else {
        T::zero()
    };
    let minus_di = if smoothed_tr.is_finite() && smoothed_tr > T::zero() {
        hundred * smoothed_minus_dm / smoothed_tr
    } else if !smoothed_tr.is_finite() || !smoothed_minus_dm.is_finite() {
        T::nan()
    } else {
        T::zero()
    };

    // DX = 100 * |+DI - -DI| / (+DI + -DI)
    // IEEE 754: NaN + x = NaN, NaN.abs() = NaN, so di_sum and di_diff propagate NaN
    let di_sum = plus_di + minus_di;
    let di_diff = (plus_di - minus_di).abs();
    output[period] = if di_sum.is_finite() && di_sum > T::zero() {
        hundred * di_diff / di_sum
    } else if !di_sum.is_finite() {
        T::nan()
    } else {
        T::zero()
    };

    // Continue with Wilder smoothing for subsequent values
    for i in (period + 1)..n {
        let tr = compute_true_range(high[i], low[i], close[i - 1]);
        let (plus_dm, minus_dm) =
            compute_directional_movement(high[i], high[i - 1], low[i], low[i - 1]);

        // Wilder smoothing using difference form (section 5.3)
        // Reduces critical path latency compared to standard form
        // IEEE 754: NaN propagates through arithmetic operations
        smoothed_tr = (tr - smoothed_tr).mul_add(alpha, smoothed_tr);
        smoothed_plus_dm = (plus_dm - smoothed_plus_dm).mul_add(alpha, smoothed_plus_dm);
        smoothed_minus_dm = (minus_dm - smoothed_minus_dm).mul_add(alpha, smoothed_minus_dm);

        // Compute DI values with explicit NaN handling
        let plus_di = if smoothed_tr.is_finite() && smoothed_tr > T::zero() {
            hundred * smoothed_plus_dm / smoothed_tr
        } else if !smoothed_tr.is_finite() || !smoothed_plus_dm.is_finite() {
            T::nan()
        } else {
            T::zero()
        };
        let minus_di = if smoothed_tr.is_finite() && smoothed_tr > T::zero() {
            hundred * smoothed_minus_dm / smoothed_tr
        } else if !smoothed_tr.is_finite() || !smoothed_minus_dm.is_finite() {
            T::nan()
        } else {
            T::zero()
        };

        // DX = 100 * |+DI - -DI| / (+DI + -DI)
        let di_sum = plus_di + minus_di;
        let di_diff = (plus_di - minus_di).abs();
        output[i] = if di_sum.is_finite() && di_sum > T::zero() {
            hundred * di_diff / di_sum
        } else if !di_sum.is_finite() {
            T::nan()
        } else {
            T::zero()
        };
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
    let alpha = T::one() / period_t; // Wilder smoothing factor for difference form

    // Fill lookback with NaN using efficient slice.fill()
    output[..period].fill(T::nan());

    // Calculate initial sum of +DM for the first period
    let mut sum_plus_dm = T::zero();
    for i in 1..=period {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];

        // Handle NaN: if up_move or down_move is NaN, comparisons fail and plus_dm = 0
        // But we want NaN propagation. Check for finite before comparison.
        let plus_dm = if up_move.is_finite() && down_move.is_finite() {
            if up_move > down_move && up_move > T::zero() {
                up_move
            } else {
                T::zero()
            }
        } else {
            T::nan()
        };
        sum_plus_dm = sum_plus_dm + plus_dm;
    }

    // First smoothed value - NaN propagates through sum
    let mut smoothed_plus_dm = sum_plus_dm;
    output[period] = smoothed_plus_dm;

    // Continue with Wilder smoothing
    // Once smoothed_plus_dm is NaN, it stays NaN through Wilder smoothing
    for i in (period + 1)..n {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];

        // Handle NaN with single check on computed values
        let plus_dm = if up_move.is_finite() && down_move.is_finite() {
            if up_move > down_move && up_move > T::zero() {
                up_move
            } else {
                T::zero()
            }
        } else {
            T::nan()
        };

        // Wilder smoothing using difference form (section 5.3)
        // IEEE 754: if smoothed_plus_dm is NaN or plus_dm is NaN, result is NaN
        smoothed_plus_dm = (plus_dm - smoothed_plus_dm).mul_add(alpha, smoothed_plus_dm);
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
    let alpha = T::one() / period_t; // Wilder smoothing factor for difference form

    // Fill lookback with NaN using efficient slice.fill()
    output[..period].fill(T::nan());

    // Calculate initial sum of -DM for the first period
    let mut sum_minus_dm = T::zero();
    for i in 1..=period {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];

        // Handle NaN: if up_move or down_move is NaN, comparisons fail
        // Check for finite before comparison for proper NaN propagation
        let minus_dm = if up_move.is_finite() && down_move.is_finite() {
            if down_move > up_move && down_move > T::zero() {
                down_move
            } else {
                T::zero()
            }
        } else {
            T::nan()
        };
        sum_minus_dm = sum_minus_dm + minus_dm;
    }

    // First smoothed value - NaN propagates through sum
    let mut smoothed_minus_dm = sum_minus_dm;
    output[period] = smoothed_minus_dm;

    // Continue with Wilder smoothing
    // Once smoothed_minus_dm is NaN, it stays NaN through Wilder smoothing
    for i in (period + 1)..n {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];

        // Handle NaN with single check on computed values
        let minus_dm = if up_move.is_finite() && down_move.is_finite() {
            if down_move > up_move && down_move > T::zero() {
                down_move
            } else {
                T::zero()
            }
        } else {
            T::nan()
        };

        // Wilder smoothing using difference form (section 5.3)
        // IEEE 754: if smoothed_minus_dm is NaN or minus_dm is NaN, result is NaN
        smoothed_minus_dm = (minus_dm - smoothed_minus_dm).mul_add(alpha, smoothed_minus_dm);
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