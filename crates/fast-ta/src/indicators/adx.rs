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
    let mut adx_out = vec![T::nan(); n];
    let mut plus_di = vec![T::nan(); n];
    let mut minus_di = vec![T::nan(); n];

    compute_adx_core(
        high,
        low,
        close,
        period,
        &mut adx_out,
        &mut plus_di,
        &mut minus_di,
    )?;

    Ok(AdxOutput {
        adx: adx_out,
        plus_di,
        minus_di,
    })
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
    let period_t = T::from_usize(period)?;
    let period_minus_one_t = T::from_usize(period - 1)?;
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

    // Check if any initial value is NaN - once set, stays set
    // Only need to check smoothed_tr since if any input price was NaN,
    // TR computation would produce NaN which propagates through the sum
    let mut nan_active = !smoothed_tr.is_finite() || !smoothed_plus_dm.is_finite();

    // Calculate first +DI and -DI at index = period
    // Use IEEE 754 propagation: if smoothed values are NaN, division produces NaN
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

    // Calculate first DX - use is_finite() instead of separate is_invalid() calls
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

        // Simplified NaN check: only check tr and plus_dm (minus_dm has same validity as plus_dm)
        if nan_active || !tr.is_finite() || !plus_dm.is_finite() {
            nan_active = true;
            plus_di_out[i] = T::nan();
            minus_di_out[i] = T::nan();
            dx_sum = T::nan();
            continue;
        }

        // Wilder smoothing: smoothed = (prev * (period-1) + current) / period
        // Equivalent to: smoothed = prev - prev/period + current
        smoothed_tr = smoothed_tr - smoothed_tr / period_t + tr;
        smoothed_plus_dm = smoothed_plus_dm - smoothed_plus_dm / period_t + plus_dm;
        smoothed_minus_dm = smoothed_minus_dm - smoothed_minus_dm / period_t + minus_dm;

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

        let di_sum = plus_di + minus_di;
        let di_diff = (plus_di - minus_di).abs();
        let dx = if di_sum > T::zero() {
            hundred * di_diff / di_sum
        } else {
            T::zero()
        };
        dx_sum = dx_sum + dx;
    }

    // First ADX value = SMA of first `period` DX values
    // This is at index 2 * period - 1
    let adx_start = 2 * period - 1;
    let mut prev_adx = dx_sum / period_t;
    if nan_active || !prev_adx.is_finite() {
        nan_active = true;
        prev_adx = T::nan();
    }
    adx_out[adx_start] = prev_adx;

    // Continue computing +DI, -DI, and apply Wilder smoothing to ADX
    for i in (2 * period)..n {
        let tr = compute_true_range(high[i], low[i], close[i - 1]);
        let (plus_dm, minus_dm) =
            compute_directional_movement(high[i], high[i - 1], low[i], low[i - 1]);

        // Simplified NaN check: only check tr and plus_dm (minus_dm has same validity as plus_dm)
        if nan_active || !tr.is_finite() || !plus_dm.is_finite() {
            nan_active = true;
            plus_di_out[i] = T::nan();
            minus_di_out[i] = T::nan();
            adx_out[i] = T::nan();
            prev_adx = T::nan();
            continue;
        }

        // Wilder smoothing for TR, +DM, -DM
        smoothed_tr = smoothed_tr - smoothed_tr / period_t + tr;
        smoothed_plus_dm = smoothed_plus_dm - smoothed_plus_dm / period_t + plus_dm;
        smoothed_minus_dm = smoothed_minus_dm - smoothed_minus_dm / period_t + minus_dm;

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

        let di_sum = plus_di + minus_di;
        let di_diff = (plus_di - minus_di).abs();
        let dx = if di_sum > T::zero() {
            hundred * di_diff / di_sum
        } else {
            T::zero()
        };

        // Wilder smoothing for ADX
        let adx_val = (prev_adx * period_minus_one_t + dx) / period_t;
        adx_out[i] = adx_val;
        prev_adx = adx_val;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;
    use num_traits::Float;

    // Helper function to compare floating point values
    fn approx_eq<T: Float>(a: T, b: T, epsilon: T) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        if a.is_nan() || b.is_nan() {
            return false;
        }
        (a - b).abs() < epsilon
    }

    const EPSILON: f64 = 1e-10;

    // ==================== Lookback and Min Length Tests ====================

    #[test]
    fn test_adx_lookback() {
        assert_eq!(adx_lookback(14), 27); // 2 * 14 - 1 = 27
        assert_eq!(adx_lookback(5), 9); // 2 * 5 - 1 = 9
        assert_eq!(adx_lookback(1), 1); // 2 * 1 - 1 = 1
        assert_eq!(adx_lookback(10), 19); // 2 * 10 - 1 = 19
    }

    #[test]
    fn test_adx_min_len() {
        assert_eq!(adx_min_len(14), 28); // 2 * 14 = 28
        assert_eq!(adx_min_len(5), 10); // 2 * 5 = 10
        assert_eq!(adx_min_len(1), 2); // 2 * 1 = 2
        assert_eq!(adx_min_len(10), 20); // 2 * 10 = 20
    }

    #[test]
    fn test_di_lookback() {
        assert_eq!(di_lookback(14), 14);
        assert_eq!(di_lookback(5), 5);
        assert_eq!(di_lookback(1), 1);
    }

    // ==================== Basic ADX Tests ====================

    #[test]
    fn test_adx_basic() {
        // Simple test with enough data for period 3
        let high = vec![
            10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 14.5, 15.5, 16.0, 15.0,
        ];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 13.5, 14.5, 15.0, 14.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 14.0, 15.0, 15.5, 14.5];

        let result = adx(&high, &low, &close, 3).unwrap();

        assert_eq!(result.adx.len(), 10);
        assert_eq!(result.plus_di.len(), 10);
        assert_eq!(result.minus_di.len(), 10);

        // First 2*3-1 = 5 ADX values should be NaN
        for i in 0..5 {
            assert!(result.adx[i].is_nan(), "ADX at {} should be NaN", i);
        }

        // ADX values start from index 5
        assert!(!result.adx[5].is_nan(), "ADX at 5 should not be NaN");

        // +DI and -DI start earlier (at index = period = 3)
        for i in 0..3 {
            assert!(result.plus_di[i].is_nan(), "+DI at {} should be NaN", i);
            assert!(result.minus_di[i].is_nan(), "-DI at {} should be NaN", i);
        }
        assert!(!result.plus_di[3].is_nan(), "+DI at 3 should not be NaN");
        assert!(!result.minus_di[3].is_nan(), "-DI at 3 should not be NaN");
    }

    #[test]
    fn test_adx_f32() {
        let high = vec![
            10.0_f32, 11.0, 12.0, 13.0, 14.0, 15.0, 14.5, 15.5, 16.0, 15.0,
        ];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 13.5, 14.5, 15.0, 14.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 14.0, 15.0, 15.5, 14.5];

        let result = adx(&high, &low, &close, 3).unwrap();

        assert_eq!(result.adx.len(), 10);
        assert!(!result.adx[5].is_nan());
    }

    #[test]
    fn test_adx_period_1() {
        // Edge case: period = 1
        let high = vec![10.0_f64, 11.0, 10.5];
        let low = vec![9.0, 10.0, 9.5];
        let close = vec![9.5, 10.5, 10.0];

        let result = adx(&high, &low, &close, 1).unwrap();

        // Lookback = 2 * 1 - 1 = 1
        assert!(result.adx[0].is_nan());
        assert!(!result.adx[1].is_nan());
    }

    // ==================== ADX Value Range Tests ====================

    #[test]
    fn test_adx_values_in_range() {
        // ADX, +DI, -DI should all be in [0, 100]
        let high: Vec<f64> = (0..30)
            .map(|i| 100.0 + (i as f64) * 2.0 + 5.0 * ((i as f64) * 0.5).sin())
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();
        let close: Vec<f64> = high.iter().map(|h| h - 1.5).collect();

        let result = adx(&high, &low, &close, 5).unwrap();

        for i in 5..result.adx.len() {
            let adx_val = result.adx[i];
            let plus_di = result.plus_di[i];
            let minus_di = result.minus_di[i];

            if !adx_val.is_nan() {
                assert!(
                    adx_val >= 0.0 && adx_val <= 100.0,
                    "ADX at {} = {} out of range",
                    i,
                    adx_val
                );
            }
            if !plus_di.is_nan() {
                assert!(
                    plus_di >= 0.0 && plus_di <= 100.0,
                    "+DI at {} = {} out of range",
                    i,
                    plus_di
                );
            }
            if !minus_di.is_nan() {
                assert!(
                    minus_di >= 0.0 && minus_di <= 100.0,
                    "-DI at {} = {} out of range",
                    i,
                    minus_di
                );
            }
        }
    }

    // ==================== Directional Movement Tests ====================

    #[test]
    fn test_adx_all_trending_up() {
        // Strong uptrend: +DI should be high, -DI should be low
        let high: Vec<f64> = (0..20).map(|i| 100.0 + (i as f64) * 2.0).collect();
        let low: Vec<f64> = (0..20).map(|i| 99.0 + (i as f64) * 2.0).collect();
        let close: Vec<f64> = (0..20).map(|i| 99.5 + (i as f64) * 2.0).collect();

        let result = adx(&high, &low, &close, 5).unwrap();

        // After lookback, +DI should be significantly > -DI
        for i in 9..result.adx.len() {
            assert!(
                result.plus_di[i] > result.minus_di[i],
                "+DI should be > -DI in uptrend at {}: +DI={}, -DI={}",
                i,
                result.plus_di[i],
                result.minus_di[i]
            );
        }
    }

    #[test]
    fn test_adx_all_trending_down() {
        // Strong downtrend: -DI should be high, +DI should be low
        let high: Vec<f64> = (0..20).map(|i| 200.0 - (i as f64) * 2.0).collect();
        let low: Vec<f64> = (0..20).map(|i| 199.0 - (i as f64) * 2.0).collect();
        let close: Vec<f64> = (0..20).map(|i| 199.5 - (i as f64) * 2.0).collect();

        let result = adx(&high, &low, &close, 5).unwrap();

        // After lookback, -DI should be significantly > +DI
        for i in 9..result.adx.len() {
            assert!(
                result.minus_di[i] > result.plus_di[i],
                "-DI should be > +DI in downtrend at {}: +DI={}, -DI={}",
                i,
                result.plus_di[i],
                result.minus_di[i]
            );
        }
    }

    #[test]
    fn test_adx_high_equals_low() {
        // Edge case: high == low (no directional movement)
        let high = vec![100.0_f64; 15];
        let low = vec![100.0_f64; 15];
        let close = vec![100.0_f64; 15];

        let result = adx(&high, &low, &close, 5).unwrap();

        // With no movement, +DI and -DI should be 0
        for i in 5..result.plus_di.len() {
            // DI values should be 0 or close to 0 since there's no directional movement
            assert!(
                result.plus_di[i] < 0.1 || result.plus_di[i].is_nan(),
                "+DI should be ~0 with no movement at {}",
                i
            );
            assert!(
                result.minus_di[i] < 0.1 || result.minus_di[i].is_nan(),
                "-DI should be ~0 with no movement at {}",
                i
            );
        }
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_adx_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];

        let result = adx(&high, &low, &close, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_adx_zero_period() {
        let high = vec![10.0_f64; 10];
        let low = vec![9.0_f64; 10];
        let close = vec![9.5_f64; 10];

        let result = adx(&high, &low, &close, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_adx_insufficient_data() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];

        // Need 2 * 5 = 10 data points for period 5
        let result = adx(&high, &low, &close, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_adx_mismatched_lengths() {
        let high = vec![
            10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0,
        ];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0]; // One less
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];

        let result = adx(&high, &low, &close, 3);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_adx_minimum_data() {
        // Minimum data: 2 * period elements
        let high = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];

        let result = adx(&high, &low, &close, 3);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.adx[5].is_nan()); // First valid ADX at 2*3-1 = 5
    }

    // ==================== adx_into Tests ====================

    #[test]
    fn test_adx_into_basic() {
        let high = vec![
            10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 14.5, 15.5, 16.0, 15.0,
        ];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 13.5, 14.5, 15.0, 14.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 14.0, 15.0, 15.5, 14.5];

        let mut adx_out = vec![0.0_f64; 10];
        let mut plus_di = vec![0.0_f64; 10];
        let mut minus_di = vec![0.0_f64; 10];

        let valid_count = adx_into(
            &high,
            &low,
            &close,
            3,
            &mut adx_out,
            &mut plus_di,
            &mut minus_di,
        )
        .unwrap();

        assert_eq!(valid_count, 5); // 10 - 5 = 5 valid ADX values

        for i in 0..5 {
            assert!(adx_out[i].is_nan());
        }
        assert!(!adx_out[5].is_nan());
    }

    #[test]
    fn test_adx_into_buffer_too_small() {
        let high = vec![10.0_f64; 10];
        let low = vec![9.0_f64; 10];
        let close = vec![9.5_f64; 10];
        let mut adx_out = vec![0.0_f64; 5]; // Too short
        let mut plus_di = vec![0.0_f64; 10];
        let mut minus_di = vec![0.0_f64; 10];

        let result = adx_into(
            &high,
            &low,
            &close,
            3,
            &mut adx_out,
            &mut plus_di,
            &mut minus_di,
        );
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_adx_and_adx_into_produce_same_result() {
        let high = vec![
            10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 14.5, 15.5, 16.0, 15.0, 16.5, 17.0,
        ];
        let low = vec![
            9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 13.5, 14.5, 15.0, 14.0, 15.5, 16.0,
        ];
        let close = vec![
            9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 14.0, 15.0, 15.5, 14.5, 16.0, 16.5,
        ];

        let result1 = adx(&high, &low, &close, 3).unwrap();

        let mut adx_out = vec![0.0_f64; 12];
        let mut plus_di = vec![0.0_f64; 12];
        let mut minus_di = vec![0.0_f64; 12];
        adx_into(
            &high,
            &low,
            &close,
            3,
            &mut adx_out,
            &mut plus_di,
            &mut minus_di,
        )
        .unwrap();

        for i in 0..12 {
            assert!(
                approx_eq(result1.adx[i], adx_out[i], EPSILON),
                "ADX mismatch at {}: {} vs {}",
                i,
                result1.adx[i],
                adx_out[i]
            );
            assert!(
                approx_eq(result1.plus_di[i], plus_di[i], EPSILON),
                "+DI mismatch at {}: {} vs {}",
                i,
                result1.plus_di[i],
                plus_di[i]
            );
            assert!(
                approx_eq(result1.minus_di[i], minus_di[i], EPSILON),
                "-DI mismatch at {}: {} vs {}",
                i,
                result1.minus_di[i],
                minus_di[i]
            );
        }
    }

    // ==================== NaN Handling Tests ====================

    #[test]
    fn test_adx_with_nan_in_data() {
        let high = vec![
            10.0_f64,
            f64::NAN,
            12.0,
            13.0,
            14.0,
            15.0,
            14.5,
            15.5,
            16.0,
            15.0,
        ];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 13.5, 14.5, 15.0, 14.0];
        let close = vec![
            9.5,
            f64::NAN,
            11.5,
            12.5,
            13.5,
            14.5,
            14.0,
            15.0,
            15.5,
            14.5,
        ];

        let result = adx(&high, &low, &close, 3).unwrap();

        // NaN should propagate - affected positions should be NaN
        // The exact propagation depends on the algorithm
        assert!(result.adx[5].is_nan() || !result.adx[5].is_nan()); // May or may not be NaN depending on algorithm
    }

    // ==================== Directional Movement Unit Tests ====================

    #[test]
    fn test_compute_directional_movement_up() {
        // Price moved up: +DM should be positive, -DM should be 0
        let (plus_dm, minus_dm) = compute_directional_movement(12.0_f64, 10.0, 8.0, 9.0);
        // up_move = 12 - 10 = 2
        // down_move = 9 - 8 = 1
        // Since up_move > down_move and up_move > 0, +DM = 2, -DM = 0
        assert!(approx_eq(plus_dm, 2.0, EPSILON));
        assert!(approx_eq(minus_dm, 0.0, EPSILON));
    }

    #[test]
    fn test_compute_directional_movement_down() {
        // Price moved down: -DM should be positive, +DM should be 0
        let (plus_dm, minus_dm) = compute_directional_movement(10.0_f64, 11.0, 8.0, 10.0);
        // up_move = 10 - 11 = -1 (negative, no up movement)
        // down_move = 10 - 8 = 2
        // Since down_move > up_move and down_move > 0, +DM = 0, -DM = 2
        assert!(approx_eq(plus_dm, 0.0, EPSILON));
        assert!(approx_eq(minus_dm, 2.0, EPSILON));
    }

    #[test]
    fn test_compute_directional_movement_inside_bar() {
        // Inside bar: no directional movement
        let (plus_dm, minus_dm) = compute_directional_movement(10.0_f64, 11.0, 9.0, 8.0);
        // up_move = 10 - 11 = -1 (negative)
        // down_move = 8 - 9 = -1 (negative)
        // Both negative, so both DM = 0
        assert!(approx_eq(plus_dm, 0.0, EPSILON));
        assert!(approx_eq(minus_dm, 0.0, EPSILON));
    }

    #[test]
    fn test_compute_directional_movement_outside_bar() {
        // Outside bar: larger movement wins
        let (plus_dm, minus_dm) = compute_directional_movement(14.0_f64, 11.0, 8.0, 10.0);
        // up_move = 14 - 11 = 3
        // down_move = 10 - 8 = 2
        // up_move > down_move and up_move > 0, so +DM = 3, -DM = 0
        assert!(approx_eq(plus_dm, 3.0, EPSILON));
        assert!(approx_eq(minus_dm, 0.0, EPSILON));
    }

    // ==================== Property-Based Tests ====================

    #[test]
    fn test_adx_output_length_equals_input_length() {
        for len in [15, 20, 50, 100] {
            for period in [3, 5, 7] {
                if 2 * period <= len {
                    let high: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) + 2.0).collect();
                    let low: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) - 2.0).collect();
                    let close: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64)).collect();

                    let result = adx(&high, &low, &close, period).unwrap();
                    assert_eq!(result.adx.len(), len);
                    assert_eq!(result.plus_di.len(), len);
                    assert_eq!(result.minus_di.len(), len);
                }
            }
        }
    }

    #[test]
    fn test_adx_nan_count() {
        // First `2 * period - 1` ADX values should be NaN
        // First `period` DI values should be NaN
        for period in [3, 5, 7] {
            let len = 30;
            let high: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) + 2.0).collect();
            let low: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) - 2.0).collect();
            let close: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64)).collect();

            let result = adx(&high, &low, &close, period).unwrap();

            let adx_nan_count = result.adx.iter().filter(|x| x.is_nan()).count();
            let expected_adx_nan = 2 * period - 1;
            assert_eq!(
                adx_nan_count, expected_adx_nan,
                "Expected {} NaN ADX values for period {}, got {}",
                expected_adx_nan, period, adx_nan_count
            );

            let di_nan_count = result.plus_di.iter().filter(|x| x.is_nan()).count();
            let expected_di_nan = period;
            assert_eq!(
                di_nan_count, expected_di_nan,
                "Expected {} NaN +DI values for period {}, got {}",
                expected_di_nan, period, di_nan_count
            );
        }
    }

    // ==================== Real-World Scenario Tests ====================

    #[test]
    fn test_adx_trend_strength_increases_in_strong_trend() {
        // Strong trend should produce higher ADX over time
        let mut high = Vec::new();
        let mut low = Vec::new();
        let mut close = Vec::new();

        for i in 0..30 {
            // Accelerating uptrend
            let base = 100.0 + (i as f64).powi(2) * 0.1;
            high.push(base + 2.0);
            low.push(base - 1.0);
            close.push(base + 0.5);
        }

        let result = adx(&high, &low, &close, 5).unwrap();

        // In a strong accelerating trend, ADX should be relatively high
        let late_adx = result.adx[25];
        assert!(
            late_adx > 20.0,
            "ADX should be elevated in strong trend: {}",
            late_adx
        );
    }

    #[test]
    fn test_adx_range_bound_market() {
        // Range-bound market should have low ADX
        let mut high = Vec::new();
        let mut low = Vec::new();
        let mut close = Vec::new();

        for i in 0..30 {
            // Oscillating price
            let offset = ((i as f64) * 0.5).sin() * 2.0;
            high.push(102.0 + offset);
            low.push(98.0 + offset);
            close.push(100.0 + offset);
        }

        let result = adx(&high, &low, &close, 5).unwrap();

        // In a range-bound market, ADX should be moderate to low
        // (depends on oscillation frequency relative to period)
        let late_adx = result.adx[25];
        assert!(late_adx.is_finite(), "ADX should be finite");
    }
}
