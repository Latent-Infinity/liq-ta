//! Average Directional Index (ADX) indicator.
//!
//! The Average Directional Index is a trend strength indicator that measures the
//! strength of a trend, regardless of its direction. It was developed by J. Welles
//! Wilder Jr. and is commonly used to determine if a market is trending or ranging.
//!
//! # Algorithm
//!
//! This implementation computes ADX with O(n) time complexity:
//!
//! 1. Calculate True Range (TR) for each bar
//! 2. Calculate Directional Movement (+DM and -DM) for each bar
//! 3. Apply Wilder's smoothing to TR, +DM, and -DM
//! 4. Calculate +DI and -DI from smoothed values
//! 5. Calculate DX from +DI and -DI
//! 6. Apply Wilder's smoothing to DX to get ADX
//!
//! # Mathematical Conventions (PRD §4.6, §4.8)
//!
//! - **Wilder's Smoothing**: Uses α = 1/period for all smoothing operations
//! - **Initialization**: First smoothed values use SMA of first `period` values
//!
//! # Formula
//!
//! ```text
//! True Range[i] = max(High[i] - Low[i], |High[i] - Close[i-1]|, |Low[i] - Close[i-1]|)
//!
//! +DM[i] = High[i] - High[i-1]  if positive and > -DM, else 0
//! -DM[i] = Low[i-1] - Low[i]    if positive and > +DM, else 0
//!
//! +DI = 100 × (Smoothed +DM / Smoothed TR)
//! -DI = 100 × (Smoothed -DM / Smoothed TR)
//!
//! DX = 100 × |+DI - -DI| / (+DI + -DI)
//!
//! ADX = Wilder smoothing of DX
//! ```
//!
//! # Output
//!
//! Returns `AdxOutput` with three fields:
//! - `adx`: The Average Directional Index (0-100, trend strength)
//! - `plus_di`: The positive directional indicator (0-100)
//! - `minus_di`: The negative directional indicator (0-100)
//!
//! # Interpretation
//!
//! - ADX < 20: Weak trend or range-bound market
//! - ADX 20-40: Developing trend
//! - ADX 40-60: Strong trend
//! - ADX > 60: Very strong trend
//! - +DI > -DI: Bullish directional movement
//! - -DI > +DI: Bearish directional movement
//!
//! # NaN Handling
//!
//! The first `2 * period - 1` values are NaN:
//! - First `period` values for smoothing TR, +DM, -DM
//! - Additional `period - 1` values for smoothing DX to get ADX
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::adx::adx;
//!
//! let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35, 49.92, 50.19, 50.12, 50.50, 50.80];
//! let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86, 49.50, 49.87, 49.20, 49.80, 50.10];
//! let close = vec![48.16_f64, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32, 49.91, 50.13, 49.53, 50.20, 50.60];
//!
//! let result = adx(&high, &low, &close, 5).unwrap();
//!
//! // ADX, +DI, and -DI values start after the lookback period
//! assert!(result.adx[8].is_nan());  // Still in lookback
//! assert!(!result.adx[9].is_nan()); // First valid ADX value
//! ```

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Output structure for ADX indicator containing ADX, +DI, and -DI.
#[derive(Debug, Clone)]
pub struct AdxOutput<T> {
    /// Average Directional Index values (0-100 range, measures trend strength).
    pub adx: Vec<T>,
    /// Positive Directional Indicator values (0-100 range).
    pub plus_di: Vec<T>,
    /// Negative Directional Indicator values (0-100 range).
    pub minus_di: Vec<T>,
}

/// Returns the lookback period for ADX.
///
/// The lookback is the number of NaN values at the start of the output.
/// For ADX, this is `2 * period - 1` because:
/// - First `period` values are needed for smoothing TR, +DM, -DM
/// - Additional `period - 1` values are needed for smoothing DX
///
/// # Example
///
/// ```
/// use fast_ta::indicators::adx::adx_lookback;
///
/// assert_eq!(adx_lookback(14), 27);  // 2 * 14 - 1 = 27
/// assert_eq!(adx_lookback(5), 9);    // 2 * 5 - 1 = 9
/// ```
#[inline]
#[must_use]
pub const fn adx_lookback(period: usize) -> usize {
    2 * period - 1
}

/// Returns the minimum input length required for ADX.
///
/// This is the smallest input size that will produce at least one valid ADX output.
/// For ADX, this is `2 * period` (lookback + 1).
///
/// # Example
///
/// ```
/// use fast_ta::indicators::adx::adx_min_len;
///
/// assert_eq!(adx_min_len(14), 28);  // 2 * 14 = 28
/// assert_eq!(adx_min_len(5), 10);   // 2 * 5 = 10
/// ```
#[inline]
#[must_use]
pub const fn adx_min_len(period: usize) -> usize {
    2 * period
}

/// Returns the lookback period for +DI and -DI.
///
/// The DI lookback is `period` because we need `period` values to compute
/// the initial smoothed TR, +DM, and -DM.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::adx::di_lookback;
///
/// assert_eq!(di_lookback(14), 14);
/// assert_eq!(di_lookback(5), 5);
/// ```
#[inline]
#[must_use]
pub const fn di_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for +DI and -DI.
///
/// This is the smallest input size that will produce at least one valid output.
/// For DI indicators, this equals `period + 1`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::adx::di_min_len;
///
/// assert_eq!(di_min_len(14), 15);
/// assert_eq!(di_min_len(5), 6);
/// ```
#[inline]
#[must_use]
pub const fn di_min_len(period: usize) -> usize {
    period + 1
}

/// Computes the Average Directional Index (ADX) with +DI and -DI.
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `close` - The close prices series
/// * `period` - The number of periods for smoothing (commonly 14)
///
/// # Returns
///
/// A `Result` containing `AdxOutput` with ADX, +DI, and -DI vectors.
/// The first `2 * period - 1` values are NaN.
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The series have different lengths (`Error::LengthMismatch`)
/// - The input data is shorter than `2 * period` (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for the three output vectors
///
/// # Example
///
/// ```
/// use fast_ta::indicators::adx::adx;
///
/// let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35, 49.92, 50.19, 50.12];
/// let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86, 49.50, 49.87, 49.20];
/// let close = vec![48.16_f64, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32, 49.91, 50.13, 49.53];
///
/// let result = adx(&high, &low, &close, 5).unwrap();
///
/// // First 2*5-1 = 9 values are NaN
/// for i in 0..9 {
///     assert!(result.adx[i].is_nan());
/// }
/// // ADX values start from index 9
/// assert!(!result.adx[9].is_nan());
/// ```
#[must_use = "this returns a Result with ADX output, which should be used"]
pub fn adx<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Result<AdxOutput<T>> {
    validate_adx_inputs(high, low, close, period)?;

    let n = high.len();

    // Optimization: For f64/f32, allocate uninitialized memory (Section 5.4)
    // Avoids double-write tax since compute_adx_core fills all elements (lookback NaNs + computed values)
    use std::any::TypeId;

    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let high_f64: &[f64] = unsafe { std::mem::transmute(high) };
        let low_f64: &[f64] = unsafe { std::mem::transmute(low) };
        let close_f64: &[f64] = unsafe { std::mem::transmute(close) };

        let mut adx_out: Vec<f64> = Vec::with_capacity(n);
        let mut plus_di: Vec<f64> = Vec::with_capacity(n);
        let mut minus_di: Vec<f64> = Vec::with_capacity(n);
        unsafe {
            adx_out.set_len(n);
            plus_di.set_len(n);
            minus_di.set_len(n);
        }

        compute_adx_core(high_f64, low_f64, close_f64, period, &mut adx_out, &mut plus_di, &mut minus_di)?;

        Ok(AdxOutput {
            adx: unsafe { std::mem::transmute(adx_out) },
            plus_di: unsafe { std::mem::transmute(plus_di) },
            minus_di: unsafe { std::mem::transmute(minus_di) },
        })
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let high_f32: &[f32] = unsafe { std::mem::transmute(high) };
        let low_f32: &[f32] = unsafe { std::mem::transmute(low) };
        let close_f32: &[f32] = unsafe { std::mem::transmute(close) };

        let mut adx_out: Vec<f32> = Vec::with_capacity(n);
        let mut plus_di: Vec<f32> = Vec::with_capacity(n);
        let mut minus_di: Vec<f32> = Vec::with_capacity(n);
        unsafe {
            adx_out.set_len(n);
            plus_di.set_len(n);
            minus_di.set_len(n);
        }

        compute_adx_core(high_f32, low_f32, close_f32, period, &mut adx_out, &mut plus_di, &mut minus_di)?;

        Ok(AdxOutput {
            adx: unsafe { std::mem::transmute(adx_out) },
            plus_di: unsafe { std::mem::transmute(plus_di) },
            minus_di: unsafe { std::mem::transmute(minus_di) },
        })
    } else {
        // Generic fallback: safe initialization
        let mut adx_out = vec![T::nan(); n];
        let mut plus_di = vec![T::nan(); n];
        let mut minus_di = vec![T::nan(); n];

        compute_adx_core(high, low, close, period, &mut adx_out, &mut plus_di, &mut minus_di)?;

        Ok(AdxOutput {
            adx: adx_out,
            plus_di,
            minus_di,
        })
    }
}

/// Computes ADX into pre-allocated output buffers.
///
/// This variant allows reusing existing buffers to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `close` - The close prices series
/// * `period` - The number of periods for smoothing
/// * `adx_out` - Pre-allocated buffer for ADX values
/// * `plus_di_out` - Pre-allocated buffer for +DI values
/// * `minus_di_out` - Pre-allocated buffer for -DI values
///
/// # Returns
///
/// A `Result` containing the number of valid ADX values computed (n - 2*period + 1),
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The series have different lengths (`Error::LengthMismatch`)
/// - The input data is shorter than `2 * period` (`Error::InsufficientData`)
/// - Any output buffer is shorter than the input data
///
/// # Example
///
/// ```
/// use fast_ta::indicators::adx::adx_into;
///
/// let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35, 49.92, 50.19, 50.12];
/// let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86, 49.50, 49.87, 49.20];
/// let close = vec![48.16_f64, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32, 49.91, 50.13, 49.53];
///
/// let mut adx_out = vec![0.0_f64; 11];
/// let mut plus_di = vec![0.0_f64; 11];
/// let mut minus_di = vec![0.0_f64; 11];
///
/// let valid_count = adx_into(&high, &low, &close, 5, &mut adx_out, &mut plus_di, &mut minus_di).unwrap();
/// assert_eq!(valid_count, 2); // 11 - 9 = 2 valid ADX values
/// ```
#[must_use = "this returns a Result with the count of valid ADX values"]
pub fn adx_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    adx_out: &mut [T],
    plus_di_out: &mut [T],
    minus_di_out: &mut [T],
) -> Result<usize> {
    validate_adx_inputs(high, low, close, period)?;

    let n = high.len();

    if adx_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: adx_out.len(),
            indicator: "adx",
        });
    }
    if plus_di_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: plus_di_out.len(),
            indicator: "adx (+DI)",
        });
    }
    if minus_di_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: minus_di_out.len(),
            indicator: "adx (-DI)",
        });
    }

    // Initialize lookback period with NaN using efficient slice.fill()
    let lookback = adx_lookback(period);
    adx_out[..lookback.min(n)].fill(T::nan());
    let di_lb = di_lookback(period);
    plus_di_out[..di_lb.min(n)].fill(T::nan());
    minus_di_out[..di_lb.min(n)].fill(T::nan());

    compute_adx_core(high, low, close, period, adx_out, plus_di_out, minus_di_out)?;

    // Return count of valid ADX values
    Ok(n.saturating_sub(lookback))
}

/// Validates OHLC inputs have matching lengths and are not empty.
#[inline]
fn validate_ohlc_inputs<T: SeriesElement>(high: &[T], low: &[T], close: &[T]) -> Result<()> {
    if high.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = high.len();

    if low.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", n, low.len()),
        });
    }

    if close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, close has {}", n, close.len()),
        });
    }

    Ok(())
}

/// Validates ADX inputs.
fn validate_adx_inputs<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    validate_ohlc_inputs(high, low, close)?;

    // ADX needs at least 2 * period data points
    let min_len = adx_min_len(period);
    if high.len() < min_len {
        return Err(Error::InsufficientData {
            required: min_len,
            actual: high.len(),
            indicator: "adx",
        });
    }

    Ok(())
}

/// Computes True Range for a single bar.
/// Uses IEEE 754 NaN propagation - if any input is NaN, result is NaN.
#[inline]
fn compute_true_range<T: SeriesElement>(high: T, low: T, prev_close: T) -> T {
    let hl = high - low;
    let hc = (high - prev_close).abs();
    let lc = (low - prev_close).abs();

    // IEEE 754: max(NaN, x) = NaN, so NaN propagates naturally
    hl.max(hc).max(lc)
}

/// Computes directional movement (+DM and -DM) for a single bar.
/// Uses IEEE 754 NaN propagation - if any input is NaN, result is NaN.
#[inline]
fn compute_directional_movement<T: SeriesElement>(
    high: T,
    prev_high: T,
    low: T,
    prev_low: T,
) -> (T, T) {
    let up_move = high - prev_high;
    let down_move = prev_low - low;

    // IEEE 754: NaN comparisons return false, so we need to check for NaN
    // to ensure proper propagation. This is cheaper than 4 input checks.
    if !up_move.is_finite() || !down_move.is_finite() {
        return (T::nan(), T::nan());
    }

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

/// Core ADX computation algorithm.
fn compute_adx_core<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    adx_out: &mut [T],
    plus_di_out: &mut [T],
    minus_di_out: &mut [T],
) -> Result<()> {
    let n = high.len();

    // Fill lookback periods with NaN
    let adx_lb = adx_lookback(period);
    for i in 0..adx_lb.min(n) {
        adx_out[i] = T::nan();
    }
    let di_lb = di_lookback(period);
    for i in 0..di_lb.min(n) {
        plus_di_out[i] = T::nan();
        minus_di_out[i] = T::nan();
    }

    let period_t = T::from_usize(period)?;
    let alpha = T::one() / period_t; // Wilder smoothing factor (1/period) for difference form
    let hundred = T::hundred();

    // Step 1: Calculate initial sum of TR, +DM, -DM for the first period
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

    // Initial smoothed values (SMA of first period)
    let mut smoothed_tr = sum_tr;
    let mut smoothed_plus_dm = sum_plus_dm;
    let mut smoothed_minus_dm = sum_minus_dm;

    // Track if NaN has been encountered - once set, all subsequent outputs are NaN
    // This is the "nan_active flag pattern" for cumulative indicators:
    // - ADX has "infinite memory" through Wilder smoothing (recursive state)
    // - Once NaN enters, IEEE 754 would propagate it through all calculations anyway
    // - The flag allows early-exit to skip expensive calculations (+8-41% performance)
    // - Use is_finite() to detect NaN and Infinity in one check
    let mut nan_active = !smoothed_tr.is_finite() || !smoothed_plus_dm.is_finite();

    // Calculate first +DI and -DI at index = period
    // Use IEEE 754 propagation where possible, with explicit checks for clarity
    let plus_di_val = if nan_active {
        T::nan()
    } else if smoothed_tr > T::zero() {
        hundred * smoothed_plus_dm / smoothed_tr
    } else {
        T::zero()
    };
    let minus_di_val = if nan_active {
        T::nan()
    } else if smoothed_tr > T::zero() {
        hundred * smoothed_minus_dm / smoothed_tr
    } else {
        T::zero()
    };

    plus_di_out[period] = plus_di_val;
    minus_di_out[period] = minus_di_val;

    // Calculate first DX - check result instead of inputs
    let first_dx = if !plus_di_val.is_finite() {
        nan_active = true;
        T::nan()
    } else {
        let di_sum = plus_di_val + minus_di_val;
        let di_diff = (plus_di_val - minus_di_val).abs();
        if di_sum > T::zero() {
            hundred * di_diff / di_sum
        } else {
            T::zero()
        }
    };

    // Continue with Wilder smoothing for +DI and -DI, accumulating DX values
    let mut dx_sum = first_dx;

    for i in (period + 1)..(2 * period) {
        let tr = compute_true_range(high[i], low[i], close[i - 1]);
        let (plus_dm, minus_dm) =
            compute_directional_movement(high[i], high[i - 1], low[i], low[i - 1]);

        // Early exit if NaN is active or new inputs are invalid
        // Check tr and plus_dm only (minus_dm has same validity as plus_dm from same computation)
        if nan_active || !tr.is_finite() || !plus_dm.is_finite() {
            nan_active = true;
            plus_di_out[i] = T::nan();
            minus_di_out[i] = T::nan();
            dx_sum = T::nan();
            continue;
        }

        // Wilder smoothing using difference form (section 5.3)
        // Reduces critical path latency by eliminating divisions
        smoothed_tr = (tr - smoothed_tr).mul_add(alpha, smoothed_tr);
        smoothed_plus_dm = (plus_dm - smoothed_plus_dm).mul_add(alpha, smoothed_plus_dm);
        smoothed_minus_dm = (minus_dm - smoothed_minus_dm).mul_add(alpha, smoothed_minus_dm);

        // Calculate current +DI and -DI
        let plus_di = if smoothed_tr > T::zero() {
            hundred * smoothed_plus_dm / smoothed_tr
        } else {
            T::zero()
        };
        let minus_di = if smoothed_tr > T::zero() {
            hundred * smoothed_minus_dm / smoothed_tr
        } else {
            T::zero()
        };

        plus_di_out[i] = plus_di;
        minus_di_out[i] = minus_di;

        // Calculate current DX and accumulate
        let di_sum = plus_di + minus_di;
        let di_diff = (plus_di - minus_di).abs();
        let dx = if di_sum > T::zero() {
            hundred * di_diff / di_sum
        } else {
            T::zero()
        };

        dx_sum = dx_sum + dx;
    }

    // Initial ADX = SMA of DX for first period values
    // This is at index 2 * period - 1
    let adx_start = 2 * period - 1;
    let mut prev_adx = dx_sum / period_t;
    if nan_active || !prev_adx.is_finite() {
        nan_active = true;
        prev_adx = T::nan();
    }
    adx_out[adx_start] = prev_adx;

    // Step 2: Continue computing +DI, -DI, and apply Wilder smoothing to ADX
    for i in (2 * period)..n {
        let tr = compute_true_range(high[i], low[i], close[i - 1]);
        let (plus_dm, minus_dm) =
            compute_directional_movement(high[i], high[i - 1], low[i], low[i - 1]);

        // Early exit if NaN is active or new inputs are invalid
        if nan_active || !tr.is_finite() || !plus_dm.is_finite() {
            nan_active = true;
            plus_di_out[i] = T::nan();
            minus_di_out[i] = T::nan();
            adx_out[i] = T::nan();
            prev_adx = T::nan();
            continue;
        }

        // Wilder smoothing using difference form (section 5.3)
        smoothed_tr = (tr - smoothed_tr).mul_add(alpha, smoothed_tr);
        smoothed_plus_dm = (plus_dm - smoothed_plus_dm).mul_add(alpha, smoothed_plus_dm);
        smoothed_minus_dm = (minus_dm - smoothed_minus_dm).mul_add(alpha, smoothed_minus_dm);

        // Calculate current +DI and -DI
        let plus_di = if smoothed_tr > T::zero() {
            hundred * smoothed_plus_dm / smoothed_tr
        } else {
            T::zero()
        };
        let minus_di = if smoothed_tr > T::zero() {
            hundred * smoothed_minus_dm / smoothed_tr
        } else {
            T::zero()
        };

        plus_di_out[i] = plus_di;
        minus_di_out[i] = minus_di;

        // Calculate current DX
        let di_sum = plus_di + minus_di;
        let di_diff = (plus_di - minus_di).abs();
        let dx = if di_sum > T::zero() {
            hundred * di_diff / di_sum
        } else {
            T::zero()
        };

        // Wilder smoothing for ADX using difference form (section 5.3)
        let adx_val = (dx - prev_adx).mul_add(alpha, prev_adx);
        adx_out[i] = adx_val;
        prev_adx = adx_val;
    }

    Ok(())
}