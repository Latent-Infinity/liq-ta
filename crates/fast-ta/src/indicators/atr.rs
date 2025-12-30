//! Average True Range (ATR) indicator.
//!
//! The Average True Range is a volatility indicator that measures the degree of
//! price volatility by decomposing the entire range of a security's price for a
//! given period. It was developed by J. Welles Wilder Jr.
//!
//! # Algorithm
//!
//! This implementation computes ATR with O(n) time complexity using Wilder's
//! smoothing method:
//!
//! 1. Calculate True Range (TR) for each bar:
//!    - TR = max(High - Low, |High - Previous Close|, |Low - Previous Close|)
//! 2. Apply Wilder's smoothing to the True Range values:
//!    - First ATR = SMA of first `period` TR values
//!    - Subsequent ATR values use Wilder's formula
//!
//! # Mathematical Conventions (PRD §4.6, §4.8)
//!
//! - **Wilder's Smoothing**: Uses α = 1/period, which is equivalent to standard
//!   EMA with period 2n-1. This produces a slower-responding average than
//!   standard EMA with the same period.
//! - **Initialization**: The first ATR value is the simple average (SMA) of the
//!   first `period` True Range values, per Wilder's original method (PRD §4.6).
//!
//! # Formula
//!
//! ```text
//! True Range[i] = max(
//!     High[i] - Low[i],                    // Current range
//!     |High[i] - Close[i-1]|,              // Gap up from previous close
//!     |Low[i] - Close[i-1]|                // Gap down from previous close
//! )
//!
//! First ATR = SMA(TR[1..period])
//!
//! Subsequent:
//! ATR[i] = (ATR[i-1] * (period-1) + TR[i]) / period
//! ```
//!
//! # Gap Handling
//!
//! ATR correctly handles overnight gaps and opening gaps:
//! - If the market opens significantly above the previous close, the true range
//!   includes the gap (High - Previous Close).
//! - If the market opens significantly below the previous close, the true range
//!   includes the gap (Previous Close - Low).
//!
//! # NaN Handling
//!
//! - The first `period` values are NaN (insufficient data for initial calculation)
//! - ATR requires previous close for TR calculation, so index 0 has no TR value
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::atr::{atr, true_range};
//!
//! let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35, 49.92, 50.19];
//! let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86, 49.50, 49.87];
//! let close = vec![48.16_f64, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32, 49.91, 50.13];
//!
//! // Calculate True Range
//! let tr = true_range(&high, &low, &close).unwrap();
//!
//! // Calculate ATR with 5-period
//! let result = atr(&high, &low, &close, 5).unwrap();
//!
//! // First 5 values are NaN
//! assert!(result[0].is_nan());
//! assert!(result[4].is_nan());
//!
//! // ATR values start from index 5
//! assert!(!result[5].is_nan());
//! ```

use crate::error::{Error, Result};
use crate::precision::{current_precision_mode, PrecisionMode};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns true if we should use f64 precision for the given type.
///
/// Uses f64 precision when:
/// - Input type is f32 AND PrecisionMode is High
#[inline]
fn use_f64_precision<T: 'static>() -> bool {
    use std::any::TypeId;
    TypeId::of::<T>() == TypeId::of::<f32>() && current_precision_mode() == PrecisionMode::High
}

/// Returns the lookback period for ATR.
///
/// The lookback is the number of NaN values at the start of the output.
/// For ATR, this is `period` (because True Range starts at index 1,
/// then we need `period` TR values for the initial SMA).
///
/// # Example
///
/// ```
/// use fast_ta::indicators::atr::atr_lookback;
///
/// assert_eq!(atr_lookback(14), 14);
/// assert_eq!(atr_lookback(5), 5);
/// ```
#[inline]
#[must_use]
pub const fn atr_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for ATR.
///
/// This is the smallest input size that will produce at least one valid output.
/// For ATR, this is `period + 1` (one extra for the initial True Range).
///
/// # Example
///
/// ```
/// use fast_ta::indicators::atr::atr_min_len;
///
/// assert_eq!(atr_min_len(14), 15);
/// assert_eq!(atr_min_len(5), 6);
/// ```
#[inline]
#[must_use]
pub const fn atr_min_len(period: usize) -> usize {
    period + 1
}

/// Returns the lookback period for True Range.
///
/// True Range has a lookback of 1 (first value is NaN because
/// it requires the previous close).
///
/// # Example
///
/// ```
/// use fast_ta::indicators::atr::true_range_lookback;
///
/// assert_eq!(true_range_lookback(), 1);
/// ```
#[inline]
#[must_use]
pub const fn true_range_lookback() -> usize {
    1
}

/// Computes the True Range (TR) for a series of OHLC data.
///
/// True Range measures the greatest of:
/// - Current High minus Current Low
/// - Absolute value of Current High minus Previous Close
/// - Absolute value of Current Low minus Previous Close
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `close` - The close prices series
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` with the True Range values.
/// The first value (index 0) is NaN as there's no previous close.
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The series have different lengths (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for the output vector
///
/// # Example
///
/// ```
/// use fast_ta::indicators::atr::true_range;
///
/// let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.0];
/// let low = vec![9.0_f64, 10.0, 11.0, 10.5, 11.0];
/// let close = vec![9.5_f64, 10.5, 11.5, 11.0, 11.8];
///
/// let tr = true_range(&high, &low, &close).unwrap();
///
/// assert!(tr[0].is_nan());  // No previous close
/// assert!(!tr[1].is_nan()); // Valid TR
/// ```
#[must_use = "this returns a Result with the True Range values, which should be used"]
pub fn true_range<T: SeriesElement + 'static>(high: &[T], low: &[T], close: &[T]) -> Result<Vec<T>> {
    // Validate inputs
    validate_ohlc_inputs(high, low, close)?;

    let n = high.len();
    let mut tr = vec![T::nan(); n];

    // Use specialized f64 fast path when possible
    use std::any::TypeId;
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        // Safety: We've checked the type is f64
        let h = unsafe { std::slice::from_raw_parts(high.as_ptr() as *const f64, high.len()) };
        let l = unsafe { std::slice::from_raw_parts(low.as_ptr() as *const f64, low.len()) };
        let c = unsafe { std::slice::from_raw_parts(close.as_ptr() as *const f64, close.len()) };
        let out = unsafe { std::slice::from_raw_parts_mut(tr.as_mut_ptr() as *mut f64, tr.len()) };
        true_range_f64_optimized(h, l, c, out);
        return Ok(tr);
    }

    // Scalar path for other types - handles NaN/infinity correctly via compute_true_range
    for i in 1..n {
        tr[i] = compute_true_range(high[i], low[i], close[i - 1]);
    }

    Ok(tr)
}

/// Optimized f64 true range computation (TA-Lib style).
/// Uses local variables and incremental max for better compiler optimization.
/// Assumes data is clean (no NaN/Inf) for maximum performance - use generic path if needed.
#[inline]
fn true_range_f64_optimized(high: &[f64], low: &[f64], close: &[f64], output: &mut [f64]) {
    let n = high.len();

    // First value is NaN (no previous close)
    output[0] = f64::NAN;

    // Tight loop with local variables (TA-Lib approach)
    // Note: No NaN checking for performance - NaN will propagate naturally via arithmetic
    for i in 1..n {
        let temp_high = high[i];
        let temp_low = low[i];
        let prev_close = close[i - 1];

        // Compute max(hl, hc, lc) incrementally like TA-Lib
        let mut greatest = temp_high - temp_low;

        let val2 = (prev_close - temp_high).abs();
        if val2 > greatest {
            greatest = val2;
        }

        let val3 = (prev_close - temp_low).abs();
        if val3 > greatest {
            greatest = val3;
        }

        output[i] = greatest;
    }
}

/// Computes the True Range into a pre-allocated output buffer.
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `close` - The close prices series
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid TR values (n - 1).
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The series have different lengths (`Error::InsufficientData`)
/// - The output buffer is shorter than the input data
///
/// # Example
///
/// ```
/// use fast_ta::indicators::atr::true_range_into;
///
/// let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.0];
/// let low = vec![9.0_f64, 10.0, 11.0, 10.5, 11.0];
/// let close = vec![9.5_f64, 10.5, 11.5, 11.0, 11.8];
/// let mut output = vec![0.0_f64; 5];
///
/// let valid_count = true_range_into(&high, &low, &close, &mut output).unwrap();
/// assert_eq!(valid_count, 4); // n - 1 valid values
/// ```
#[must_use = "this returns a Result with the count of valid True Range values"]
pub fn true_range_into<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    output: &mut [T],
) -> Result<usize> {
    // Validate inputs
    validate_ohlc_inputs(high, low, close)?;

    let n = high.len();

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "true_range",
        });
    }

    // Use specialized f64 fast path when possible
    use std::any::TypeId;
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        // Safety: We've checked the type is f64
        let h = unsafe { std::slice::from_raw_parts(high.as_ptr() as *const f64, high.len()) };
        let l = unsafe { std::slice::from_raw_parts(low.as_ptr() as *const f64, low.len()) };
        let c = unsafe { std::slice::from_raw_parts(close.as_ptr() as *const f64, close.len()) };
        let out = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut f64, n) };
        true_range_f64_optimized(h, l, c, out);
        return Ok(n - 1);
    }

    // First value is NaN
    output[0] = T::nan();

    // Compute TR for remaining values
    for i in 1..n {
        output[i] = compute_true_range(high[i], low[i], close[i - 1]);
    }

    // Return count of valid values (all except first)
    Ok(n - 1)
}

/// Computes the Average True Range (ATR) using Wilder's smoothing.
///
/// ATR measures market volatility by calculating the average of True Range values
/// over a specified period, using Wilder's smoothing method.
///
/// # Precision Behavior
///
/// When `PrecisionMode::High` is active and input type is `f32`:
/// - ATR smoothing state is maintained in `f64` to prevent drift accumulation
/// - Particularly important for long periods (> 14) where cumulative error builds
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `close` - The close prices series
/// * `period` - The number of periods for the ATR calculation (commonly 14)
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` with the ATR values, or an error if validation fails.
/// The first `period` values are NaN (insufficient data for initial calculation).
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The series have different lengths (`Error::InsufficientData`)
/// - The input data is shorter than period + 1 (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for the output vector (plus O(n) temporary for TR)
///
/// # Gap Handling
///
/// ATR correctly handles overnight gaps:
/// - A gap up from previous close is reflected in the True Range
/// - A gap down from previous close is reflected in the True Range
///
/// # Example
///
/// ```
/// use fast_ta::indicators::atr::atr;
///
/// let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35, 49.92, 50.19, 50.12];
/// let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86, 49.50, 49.87, 49.20];
/// let close = vec![48.16_f64, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32, 49.91, 50.13, 49.53];
///
/// let result = atr(&high, &low, &close, 5).unwrap();
///
/// // First 5 values are NaN
/// for i in 0..5 {
///     assert!(result[i].is_nan());
/// }
///
/// // ATR values start from index 5
/// assert!(!result[5].is_nan());
/// assert!(result[5] > 0.0); // ATR is always positive
/// ```
#[must_use = "this returns a Result with the ATR values, which should be used"]
pub fn atr<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Result<Vec<T>> {
    // Validate inputs
    validate_atr_inputs(high, low, close, period)?;

    let n = high.len();

    // Initialize result vector with NaN
    let mut result = vec![T::nan(); n];

    // Compute ATR values into the result vector
    compute_atr_core(high, low, close, period, &mut result)?;

    Ok(result)
}

/// Computes the Average True Range into a pre-allocated output buffer.
///
/// This variant allows reusing an existing buffer to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `close` - The close prices series
/// * `period` - The number of periods for the ATR calculation
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid ATR values computed (n - period),
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The series have different lengths (`Error::InsufficientData`)
/// - The input data is shorter than period + 1 (`Error::InsufficientData`)
/// - The output buffer is shorter than the input data
///
/// # Example
///
/// ```
/// use fast_ta::indicators::atr::atr_into;
///
/// let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35, 49.92, 50.19, 50.12];
/// let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86, 49.50, 49.87, 49.20];
/// let close = vec![48.16_f64, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32, 49.91, 50.13, 49.53];
/// let mut output = vec![0.0_f64; 11];
///
/// let valid_count = atr_into(&high, &low, &close, 5, &mut output).unwrap();
/// assert_eq!(valid_count, 6); // 11 - 5 = 6 valid values
/// ```
#[must_use = "this returns a Result with the count of valid ATR values"]
pub fn atr_into<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    // Validate inputs
    validate_atr_inputs(high, low, close, period)?;

    let n = high.len();

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "atr",
        });
    }

    // Initialize lookback period with NaN
    for item in output.iter_mut().take(period) {
        *item = T::nan();
    }

    // Compute ATR values
    compute_atr_core(high, low, close, period, output)?;

    // Return count of valid (non-NaN) values
    Ok(n - period)
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

/// Validates ATR inputs.
fn validate_atr_inputs<T: SeriesElement>(
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

    // ATR needs at least period + 1 data points:
    // - We need TR values starting from index 1 (first TR at index 1)
    // - We need `period` TR values for the initial SMA
    // - So we need indices 1..=period to have TR values
    // - This means we need at least period + 1 data points
    if high.len() < period + 1 {
        return Err(Error::InsufficientData {
            required: period + 1,
            actual: high.len(),
            indicator: "atr",
        });
    }

    Ok(())
}

/// Computes True Range for a single bar.
///
/// TR = max(High - Low, |High - Prev Close|, |Low - Prev Close|)
#[inline]
fn compute_true_range<T: SeriesElement>(high: T, low: T, prev_close: T) -> T {
    if is_invalid(high) || is_invalid(low) || is_invalid(prev_close) {
        return T::nan();
    }

    let hl = high - low;
    let hc = (high - prev_close).abs();
    let lc = (low - prev_close).abs();

    // Return maximum of the three
    hl.max(hc).max(lc)
}

/// Core ATR computation algorithm using Wilder's smoothing.
///
/// This function assumes all validation has been done and output is properly sized.
/// It fills the output slice with ATR values starting at index `period`.
///
/// Dispatches to f64 precision path for f32 inputs in High precision mode.
fn compute_atr_core<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if use_f64_precision::<T>() {
        compute_atr_core_f64(high, low, close, period, output)
    } else {
        compute_atr_core_native(high, low, close, period, output)
    }
}

/// Core ATR computation using native precision.
fn compute_atr_core_native<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();
    let period_t = T::from_usize(period)?;
    let period_minus_one_t = T::from_usize(period - 1)?;

    // Step 1: Calculate initial sum of True Range values for the first period
    // TR starts at index 1 (needs previous close)
    // We sum TR[1] through TR[period] for the initial SMA
    let mut sum_tr = T::zero();
    for i in 1..=period {
        let tr = compute_true_range(high[i], low[i], close[i - 1]);
        sum_tr = sum_tr + tr;
    }

    // Calculate initial ATR (SMA of first period TR values)
    let mut atr_prev = sum_tr / period_t;
    output[period] = atr_prev;

    // Step 2: Apply Wilder's smoothing for remaining values
    // Wilder's formula: ATR = (prev_ATR * (period-1) + TR) / period
    for i in (period + 1)..n {
        let tr = compute_true_range(high[i], low[i], close[i - 1]);
        let atr_current = (atr_prev * period_minus_one_t + tr) / period_t;
        output[i] = atr_current;
        atr_prev = atr_current;
    }

    Ok(())
}

/// Core ATR computation using f64 precision for smoothing state.
///
/// Uses f64 for the ATR state to prevent drift accumulation over long periods.
/// TR computation is done in f64, smoothing state maintained in f64, output cast to T.
fn compute_atr_core_f64<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();
    let period_f64 = period as f64;
    let period_minus_one_f64 = (period - 1) as f64;

    // Step 1: Calculate initial sum of True Range values in f64
    let mut sum_tr: f64 = 0.0;
    for i in 1..=period {
        let tr = compute_true_range(high[i], low[i], close[i - 1]);
        if is_invalid(tr) {
            // If any TR in initial window is NaN, propagate it
            output[period] = T::nan();
            // Continue with NaN state
            let mut atr_prev_f64 = f64::NAN;
            for j in (period + 1)..n {
                let tr_j = compute_true_range(high[j], low[j], close[j - 1]);
                let tr_f64 = tr_j.to_f64().unwrap_or(f64::NAN);
                atr_prev_f64 = (atr_prev_f64 * period_minus_one_f64 + tr_f64) / period_f64;
                output[j] = T::from_f64(atr_prev_f64)?;
            }
            return Ok(());
        }
        sum_tr += tr.to_f64().unwrap_or(0.0);
    }

    // Calculate initial ATR (SMA of first period TR values) in f64
    let mut atr_prev_f64 = sum_tr / period_f64;
    output[period] = T::from_f64(atr_prev_f64)?;

    // Step 2: Apply Wilder's smoothing with f64 state
    for i in (period + 1)..n {
        let tr = compute_true_range(high[i], low[i], close[i - 1]);
        let tr_f64 = tr.to_f64().unwrap_or(f64::NAN);

        // Wilder's formula in f64: ATR = (prev_ATR * (period-1) + TR) / period
        atr_prev_f64 = (atr_prev_f64 * period_minus_one_f64 + tr_f64) / period_f64;
        output[i] = T::from_f64(atr_prev_f64)?;
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
    const EPSILON_F32: f32 = 1e-5;
    // Looser epsilon for ATR calculations involving multiple operations
    const ATR_EPSILON: f64 = 1e-6;

    // ==================== True Range Tests ====================

    #[test]
    fn test_true_range_basic() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.0];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.0];
        let close = vec![9.5, 10.5, 11.5, 11.0, 11.8];

        let tr = true_range(&high, &low, &close).unwrap();

        assert_eq!(tr.len(), 5);
        assert!(tr[0].is_nan()); // No previous close

        // TR[1] = max(11-10, |11-9.5|, |10-9.5|) = max(1, 1.5, 0.5) = 1.5
        assert!(approx_eq(tr[1], 1.5, EPSILON));

        // TR[2] = max(12-11, |12-10.5|, |11-10.5|) = max(1, 1.5, 0.5) = 1.5
        assert!(approx_eq(tr[2], 1.5, EPSILON));

        // TR[3] = max(11.5-10.5, |11.5-11.5|, |10.5-11.5|) = max(1, 0, 1) = 1
        assert!(approx_eq(tr[3], 1.0, EPSILON));

        // TR[4] = max(12-11, |12-11|, |11-11|) = max(1, 1, 0) = 1
        assert!(approx_eq(tr[4], 1.0, EPSILON));
    }

    #[test]
    fn test_true_range_with_gap_up() {
        // Gap up scenario: previous close = 50, next day opens at 55
        let high = vec![52.0_f64, 58.0]; // Gap up - high is well above previous close
        let low = vec![48.0, 54.0]; // Gap up - low is above previous close too
        let close = vec![50.0, 57.0];

        let tr = true_range(&high, &low, &close).unwrap();

        // TR[1] = max(58-54, |58-50|, |54-50|) = max(4, 8, 4) = 8
        // The gap up is captured by |High - Prev Close|
        assert!(approx_eq(tr[1], 8.0, EPSILON));
    }

    #[test]
    fn test_true_range_with_gap_down() {
        // Gap down scenario: previous close = 50, next day opens at 45
        let high = vec![52.0_f64, 47.0]; // Gap down - high is below previous close
        let low = vec![48.0, 43.0]; // Gap down - low is well below previous close
        let close = vec![50.0, 45.0];

        let tr = true_range(&high, &low, &close).unwrap();

        // TR[1] = max(47-43, |47-50|, |43-50|) = max(4, 3, 7) = 7
        // The gap down is captured by |Low - Prev Close|
        assert!(approx_eq(tr[1], 7.0, EPSILON));
    }

    #[test]
    fn test_true_range_inside_bar() {
        // Inside bar: current bar is entirely within previous bar range
        let high = vec![52.0_f64, 51.0]; // Lower high
        let low = vec![48.0, 49.0]; // Higher low
        let close = vec![50.0, 50.0];

        let tr = true_range(&high, &low, &close).unwrap();

        // TR[1] = max(51-49, |51-50|, |49-50|) = max(2, 1, 1) = 2
        // Just the current bar's range
        assert!(approx_eq(tr[1], 2.0, EPSILON));
    }

    #[test]
    fn test_true_range_outside_bar() {
        // Outside bar: current bar engulfs previous close
        let high = vec![51.0_f64, 54.0]; // Higher high
        let low = vec![49.0, 47.0]; // Lower low
        let close = vec![50.0, 51.0];

        let tr = true_range(&high, &low, &close).unwrap();

        // TR[1] = max(54-47, |54-50|, |47-50|) = max(7, 4, 3) = 7
        assert!(approx_eq(tr[1], 7.0, EPSILON));
    }

    #[test]
    fn test_true_range_f32() {
        let high = vec![10.0_f32, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];

        let tr = true_range(&high, &low, &close).unwrap();

        assert_eq!(tr.len(), 3);
        assert!(tr[0].is_nan());
        assert!(approx_eq(tr[1], 1.5_f32, EPSILON_F32));
    }

    #[test]
    fn test_true_range_into() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.0];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.0];
        let close = vec![9.5, 10.5, 11.5, 11.0, 11.8];
        let mut output = vec![0.0_f64; 5];

        let valid_count = true_range_into(&high, &low, &close, &mut output).unwrap();

        assert_eq!(valid_count, 4);
        assert!(output[0].is_nan());
        assert!(approx_eq(output[1], 1.5, EPSILON));
    }

    #[test]
    fn test_true_range_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];

        let result = true_range(&high, &low, &close);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_true_range_mismatched_lengths() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0]; // One less element
        let close = vec![9.5, 10.5, 11.5];

        let result = true_range(&high, &low, &close);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    // ==================== ATR Basic Tests ====================

    #[test]
    fn test_atr_basic() {
        let high = vec![
            48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35, 49.92, 50.19,
        ];
        let low = vec![
            47.79, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86, 49.50, 49.87,
        ];
        let close = vec![
            48.16, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32, 49.91, 50.13,
        ];

        let result = atr(&high, &low, &close, 5).unwrap();

        assert_eq!(result.len(), 10);

        // First 5 values should be NaN
        for i in 0..5 {
            assert!(result[i].is_nan(), "ATR at {} should be NaN", i);
        }

        // ATR values start from index 5
        assert!(!result[5].is_nan());
        assert!(result[5] > 0.0); // ATR should be positive
    }

    #[test]
    fn test_atr_f32() {
        let high = vec![
            48.70_f32, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35, 49.92, 50.19,
        ];
        let low = vec![
            47.79, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86, 49.50, 49.87,
        ];
        let close = vec![
            48.16, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32, 49.91, 50.13,
        ];

        let result = atr(&high, &low, &close, 5).unwrap();

        assert_eq!(result.len(), 10);
        assert!(!result[5].is_nan());
    }

    #[test]
    fn test_atr_period_one() {
        // ATR(1) should equal the True Range (no smoothing)
        let high = vec![10.0_f64, 12.0, 11.0, 13.0, 12.5];
        let low = vec![9.0, 10.5, 10.0, 11.5, 11.0];
        let close = vec![9.5, 11.0, 10.5, 12.0, 11.5];

        let result = atr(&high, &low, &close, 1).unwrap();

        // First value is NaN
        assert!(result[0].is_nan());

        // TR[1] = max(12-10.5, |12-9.5|, |10.5-9.5|) = max(1.5, 2.5, 1) = 2.5
        // ATR(1)[1] = TR[1] = 2.5
        assert!(approx_eq(result[1], 2.5, ATR_EPSILON));
    }

    #[test]
    fn test_atr_constant_range() {
        // Constant range bars - ATR should equal the range
        let high = vec![110.0_f64; 15];
        let low = vec![100.0_f64; 15];
        let close = vec![105.0_f64; 15];

        let result = atr(&high, &low, &close, 5).unwrap();

        // Each TR = 10 (high - low, since no gaps)
        // ATR should equal 10 after warmup
        for i in 5..result.len() {
            assert!(
                approx_eq(result[i], 10.0, ATR_EPSILON),
                "ATR at {} should be 10.0, got {}",
                i,
                result[i]
            );
        }
    }

    // ==================== ATR Known Value Tests ====================

    #[test]
    fn test_atr_known_calculation() {
        // Manual verification with simple data
        // Using period 3 for easier manual calculation
        let high = vec![12.0_f64, 13.0, 14.0, 13.5, 14.5];
        let low = vec![10.0, 11.0, 12.0, 12.0, 13.0];
        let close = vec![11.0, 12.0, 13.0, 13.0, 14.0];

        let result = atr(&high, &low, &close, 3).unwrap();

        // TR[1] = max(13-11, |13-11|, |11-11|) = max(2, 2, 0) = 2
        // TR[2] = max(14-12, |14-12|, |12-12|) = max(2, 2, 0) = 2
        // TR[3] = max(13.5-12, |13.5-13|, |12-13|) = max(1.5, 0.5, 1) = 1.5

        // ATR[3] = SMA(TR[1..3]) = (2 + 2 + 1.5) / 3 = 5.5 / 3 = 1.8333...
        assert!(
            approx_eq(result[3], 5.5 / 3.0, ATR_EPSILON),
            "Expected ATR[3] = {}, got {}",
            5.5 / 3.0,
            result[3]
        );

        // TR[4] = max(14.5-13, |14.5-13|, |13-13|) = max(1.5, 1.5, 0) = 1.5
        // ATR[4] = (ATR[3] * 2 + TR[4]) / 3 = (1.8333 * 2 + 1.5) / 3 = 5.1666 / 3 = 1.7222...
        let expected_atr_4 = (result[3] * 2.0 + 1.5) / 3.0;
        assert!(
            approx_eq(result[4], expected_atr_4, ATR_EPSILON),
            "Expected ATR[4] = {}, got {}",
            expected_atr_4,
            result[4]
        );
    }

    #[test]
    fn test_atr_14_period_standard() {
        // Standard 14-period ATR test
        let n = 30;
        let high: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) + 2.0).collect();
        let low: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) - 2.0).collect();
        let close: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64)).collect();

        let result = atr(&high, &low, &close, 14).unwrap();

        // First 14 values should be NaN
        for i in 0..14 {
            assert!(result[i].is_nan());
        }

        // Values should be positive and reasonable
        for i in 14..result.len() {
            assert!(!result[i].is_nan());
            assert!(result[i] > 0.0);
        }
    }

    // ==================== ATR Gap Handling Tests ====================

    #[test]
    fn test_atr_with_gap_up() {
        // Test that gaps are properly reflected in ATR
        let high = vec![50.0_f64, 52.0, 60.0, 62.0, 61.0, 63.0]; // Big gap at index 2
        let low = vec![48.0, 50.0, 57.0, 59.0, 59.0, 61.0];
        let close = vec![49.0, 51.0, 59.0, 60.0, 60.0, 62.0];

        let result = atr(&high, &low, &close, 3).unwrap();

        // TR[2] should capture the gap: max(60-57, |60-51|, |57-51|) = max(3, 9, 6) = 9
        // ATR[3] should be elevated due to the gap
        assert!(result[3] > 3.0, "ATR should be elevated due to gap up");
    }

    #[test]
    fn test_atr_with_gap_down() {
        // Test that gaps are properly reflected in ATR
        let high = vec![52.0_f64, 50.0, 43.0, 42.0, 43.0, 42.0]; // Big gap down at index 2
        let low = vec![48.0, 48.0, 40.0, 40.0, 41.0, 40.0];
        let close = vec![51.0, 49.0, 41.0, 41.0, 42.0, 41.0];

        let result = atr(&high, &low, &close, 3).unwrap();

        // TR[2] should capture the gap: max(43-40, |43-49|, |40-49|) = max(3, 6, 9) = 9
        // ATR[3] should be elevated due to the gap
        assert!(result[3] > 3.0, "ATR should be elevated due to gap down");
    }

    #[test]
    fn test_atr_overnight_gap_scenario() {
        // Simulate a typical overnight gap scenario
        let high = vec![
            100.0_f64, 101.0, 102.0, // Normal trading
            105.0, // Gap up on news
            106.0, 105.5, // Continue after gap
        ];
        let low = vec![99.0, 100.0, 101.0, 103.0, 104.0, 104.0];
        let close = vec![100.0, 100.5, 101.5, 104.0, 105.0, 104.5];

        let result = atr(&high, &low, &close, 3).unwrap();

        // The gap at index 3 should inflate the ATR
        // TR[3] = max(105-103, |105-101.5|, |103-101.5|) = max(2, 3.5, 1.5) = 3.5
        assert!(
            result[3] > result[4].min(result[5]),
            "Gap should cause ATR spike"
        );
    }

    // ==================== ATR Wilder Smoothing Tests ====================

    #[test]
    fn test_atr_wilder_smoothing_behavior() {
        // Verify ATR uses Wilder's smoothing (slower response than SMA)
        // After a volatility spike, ATR should gradually decay

        let mut high = vec![100.0_f64; 20];
        let mut low = vec![99.0_f64; 20];
        let close = vec![99.5_f64; 20];

        // Create a volatility spike at index 5
        high[5] = 110.0;
        low[5] = 90.0;

        let result = atr(&high, &low, &close, 3).unwrap();

        // ATR should spike after the volatile bar
        let spike_atr = result[5];
        assert!(spike_atr > 1.0, "ATR should spike on volatile bar");

        // ATR should gradually decrease after the spike
        for i in 6..10 {
            assert!(
                result[i] < result[i - 1],
                "ATR should decrease after spike: ATR[{}]={} should be < ATR[{}]={}",
                i,
                result[i],
                i - 1,
                result[i - 1]
            );
        }
    }

    #[test]
    fn test_atr_smoothing_memory_effect() {
        // Wilder's smoothing has a longer memory than SMA
        // A spike should take time to fully decay

        let mut high = vec![100.0_f64; 30];
        let mut low = vec![99.0_f64; 30];
        let close = vec![99.5_f64; 30];

        // Create a massive spike
        high[10] = 150.0;
        low[10] = 50.0;

        let result = atr(&high, &low, &close, 5).unwrap();

        // The spike's effect should still be visible many bars later
        // Normal TR = 1.0, so ATR should be above 1 for a while
        assert!(
            result[20] > 1.1,
            "Wilder smoothing should have memory effect"
        );
    }

    // ==================== ATR Edge Case Tests ====================

    #[test]
    fn test_atr_with_nan_in_data() {
        let high = vec![10.0_f64, f64::NAN, 12.0, 11.5, 12.0, 12.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.0, 11.5];
        let close = vec![9.5, f64::NAN, 11.5, 11.0, 11.8, 12.0];

        let result = atr(&high, &low, &close, 3).unwrap();

        // NaN should propagate through the calculation
        assert!(
            result[3].is_nan(),
            "ATR should be NaN when input contains NaN"
        );
    }

    #[test]
    fn test_atr_negative_prices() {
        // ATR should work with negative values (unusual but valid for some instruments)
        let high = vec![-5.0_f64, -4.0, -3.0, -4.5, -3.5, -2.5];
        let low = vec![-7.0, -6.0, -5.0, -6.0, -5.0, -4.0];
        let close = vec![-6.0, -5.0, -4.0, -5.0, -4.0, -3.0];

        let result = atr(&high, &low, &close, 3).unwrap();

        // ATR should still be positive (it's a measure of range)
        assert!(
            result[3] > 0.0,
            "ATR should be positive even with negative prices"
        );
    }

    #[test]
    fn test_atr_large_values() {
        let high: Vec<f64> = (0..15).map(|i| 1e10 + (i as f64) * 1e8).collect();
        let low: Vec<f64> = (0..15).map(|i| 1e10 + (i as f64) * 1e8 - 5e7).collect();
        let close: Vec<f64> = (0..15).map(|i| 1e10 + (i as f64) * 1e8 - 2e7).collect();

        let result = atr(&high, &low, &close, 5).unwrap();

        // Should handle large values without overflow
        for i in 5..result.len() {
            assert!(!result[i].is_nan());
            assert!(!result[i].is_infinite());
            assert!(result[i] > 0.0);
        }
    }

    #[test]
    fn test_atr_small_values() {
        let high: Vec<f64> = (0..15).map(|i| 1e-10 + (i as f64) * 1e-12).collect();
        let low: Vec<f64> = (0..15)
            .map(|i| 1e-10 + (i as f64) * 1e-12 - 5e-13)
            .collect();
        let close: Vec<f64> = (0..15)
            .map(|i| 1e-10 + (i as f64) * 1e-12 - 2e-13)
            .collect();

        let result = atr(&high, &low, &close, 5).unwrap();

        // Should handle small values
        for i in 5..result.len() {
            assert!(!result[i].is_nan());
            assert!(result[i] > 0.0);
        }
    }

    #[test]
    fn test_atr_zero_range_bars() {
        // Doji bars with zero range
        let high = vec![100.0_f64; 10];
        let low = vec![100.0_f64; 10];
        let close = vec![100.0_f64; 10];

        let result = atr(&high, &low, &close, 5).unwrap();

        // Zero range bars should give zero ATR
        for i in 5..result.len() {
            assert!(
                approx_eq(result[i], 0.0, ATR_EPSILON),
                "ATR should be 0 for zero-range bars"
            );
        }
    }

    // ==================== ATR Error Handling Tests ====================

    #[test]
    fn test_atr_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];

        let result = atr(&high, &low, &close, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_atr_zero_period() {
        let high = vec![10.0_f64; 10];
        let low = vec![9.0_f64; 10];
        let close = vec![9.5_f64; 10];

        let result = atr(&high, &low, &close, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_atr_insufficient_data() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];

        let result = atr(&high, &low, &close, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_atr_mismatched_lengths() {
        let high = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0]; // One less
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];

        let result = atr(&high, &low, &close, 3);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_atr_minimum_data() {
        // Minimum data: period + 1 elements
        let high = vec![10.0_f64, 11.0, 12.0, 13.0];
        let low = vec![9.0, 10.0, 11.0, 12.0];
        let close = vec![9.5, 10.5, 11.5, 12.5];

        let result = atr(&high, &low, &close, 3);
        assert!(result.is_ok());

        let atr_values = result.unwrap();
        assert!(atr_values[3] > 0.0);
    }

    // ==================== atr_into Tests ====================

    #[test]
    fn test_atr_into_basic() {
        let high = vec![
            48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35, 49.92, 50.19,
        ];
        let low = vec![
            47.79, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86, 49.50, 49.87,
        ];
        let close = vec![
            48.16, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32, 49.91, 50.13,
        ];
        let mut output = vec![0.0_f64; 10];

        let valid_count = atr_into(&high, &low, &close, 5, &mut output).unwrap();

        assert_eq!(valid_count, 5); // 10 - 5 = 5 valid values

        for i in 0..5 {
            assert!(output[i].is_nan());
        }
        assert!(!output[5].is_nan());
    }

    #[test]
    fn test_atr_into_buffer_reuse() {
        let high1 = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low1 = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close1 = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];

        // Second set with higher volatility
        let high2 = vec![10.0_f64, 15.0, 12.0, 17.0, 14.0, 19.0];
        let low2 = vec![5.0, 10.0, 7.0, 12.0, 9.0, 14.0];
        let close2 = vec![7.0, 12.0, 9.0, 14.0, 11.0, 16.0];

        let mut output = vec![0.0_f64; 6];

        atr_into(&high1, &low1, &close1, 3, &mut output).unwrap();
        let first_atr = output[4];

        atr_into(&high2, &low2, &close2, 3, &mut output).unwrap();
        let second_atr = output[4];

        // Second set has higher volatility, so ATR should be higher
        assert!(
            second_atr > first_atr,
            "Higher volatility should give higher ATR"
        );
    }

    #[test]
    fn test_atr_into_insufficient_output() {
        let high = vec![10.0_f64; 10];
        let low = vec![9.0_f64; 10];
        let close = vec![9.5_f64; 10];
        let mut output = vec![0.0_f64; 5]; // Too short

        let result = atr_into(&high, &low, &close, 3, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_atr_into_f32() {
        let high = vec![10.0_f32, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let mut output = vec![0.0_f32; 6];

        let valid_count = atr_into(&high, &low, &close, 3, &mut output).unwrap();
        assert_eq!(valid_count, 3);
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_atr_and_atr_into_produce_same_result() {
        let high = vec![
            48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35, 49.92, 50.19,
        ];
        let low = vec![
            47.79, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86, 49.50, 49.87,
        ];
        let close = vec![
            48.16, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32, 49.91, 50.13,
        ];

        let result1 = atr(&high, &low, &close, 5).unwrap();

        let mut result2 = vec![0.0_f64; 10];
        atr_into(&high, &low, &close, 5, &mut result2).unwrap();

        for i in 0..10 {
            assert!(
                approx_eq(result1[i], result2[i], EPSILON),
                "Mismatch at index {}: {} vs {}",
                i,
                result1[i],
                result2[i]
            );
        }
    }

    #[test]
    fn test_atr_valid_count() {
        let high = vec![10.0_f64; 100];
        let low = vec![9.0_f64; 100];
        let close = vec![9.5_f64; 100];
        let mut output = vec![0.0_f64; 100];

        let valid_count = atr_into(&high, &low, &close, 10, &mut output).unwrap();
        assert_eq!(valid_count, 90); // 100 - 10 = 90

        let valid_count = atr_into(&high, &low, &close, 1, &mut output).unwrap();
        assert_eq!(valid_count, 99); // 100 - 1 = 99

        let valid_count = atr_into(&high, &low, &close, 99, &mut output).unwrap();
        assert_eq!(valid_count, 1); // 100 - 99 = 1
    }

    // ==================== Property-Based Tests ====================

    #[test]
    fn test_atr_output_length_equals_input_length() {
        for len in [10, 20, 50, 100] {
            for period in [3, 5, 14] {
                if period < len {
                    let high: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) + 2.0).collect();
                    let low: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) - 2.0).collect();
                    let close: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64)).collect();

                    let result = atr(&high, &low, &close, period).unwrap();
                    assert_eq!(result.len(), len);
                }
            }
        }
    }

    #[test]
    fn test_atr_nan_count() {
        // First `period` values should be NaN
        for period in [3, 5, 14] {
            let len = 30;
            let high: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) + 2.0).collect();
            let low: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) - 2.0).collect();
            let close: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64)).collect();

            let result = atr(&high, &low, &close, period).unwrap();

            let nan_count = result.iter().filter(|x| x.is_nan()).count();
            assert_eq!(
                nan_count, period,
                "Expected {} NaN values for period {}",
                period, period
            );
        }
    }

    #[test]
    fn test_atr_always_positive() {
        // ATR should always be non-negative
        let high: Vec<f64> = (0..50)
            .map(|i| 100.0 + 10.0 * ((i as f64) * 0.5).sin())
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 2.0).collect();
        let close: Vec<f64> = high.iter().map(|h| h - 1.0).collect();

        let result = atr(&high, &low, &close, 14).unwrap();

        for (i, &val) in result.iter().enumerate() {
            if !val.is_nan() {
                assert!(val >= 0.0, "ATR at index {} should be >= 0, got {}", i, val);
            }
        }
    }

    #[test]
    fn test_atr_bounded_by_max_true_range() {
        // ATR should not exceed the maximum True Range in the lookback window
        let high = vec![
            100.0_f64, 101.0, 102.0, 103.0, 104.0, 200.0, 106.0, 107.0, 108.0, 109.0,
        ];
        let low = vec![
            99.0, 100.0, 101.0, 102.0, 103.0, 50.0, 105.0, 106.0, 107.0, 108.0,
        ];
        let close = vec![
            99.5, 100.5, 101.5, 102.5, 103.5, 150.0, 105.5, 106.5, 107.5, 108.5,
        ];

        let result = atr(&high, &low, &close, 3).unwrap();

        // At index 6 (after the huge spike at 5), ATR should be elevated but reasonable
        // The spike TR = max(200-50, |200-103.5|, |50-103.5|) = max(150, 96.5, 53.5) = 150
        assert!(
            result[6] < 150.0,
            "ATR should be smoothed, not equal to max TR"
        );
        assert!(result[6] > 1.0, "ATR should be elevated from the spike");
    }

    // ==================== True Range Computation Tests ====================

    #[test]
    fn test_compute_true_range_high_minus_low() {
        // When current bar engulfs previous close, use High - Low
        let tr = compute_true_range(110.0_f64, 90.0_f64, 100.0_f64);
        assert!(approx_eq(tr, 20.0, EPSILON)); // 110 - 90 = 20
    }

    #[test]
    fn test_compute_true_range_gap_up() {
        // Gap up: Low is above previous close
        let tr = compute_true_range(120.0_f64, 110.0_f64, 100.0_f64);
        // max(120-110=10, |120-100|=20, |110-100|=10) = 20
        assert!(approx_eq(tr, 20.0, EPSILON));
    }

    #[test]
    fn test_compute_true_range_gap_down() {
        // Gap down: High is below previous close
        let tr = compute_true_range(90.0_f64, 80.0_f64, 100.0_f64);
        // max(90-80=10, |90-100|=10, |80-100|=20) = 20
        assert!(approx_eq(tr, 20.0, EPSILON));
    }

    // ==================== Real-World Scenario Tests ====================

    #[test]
    fn test_atr_increasing_volatility() {
        // Simulate increasing volatility - ATR should increase
        let mut high = Vec::new();
        let mut low = Vec::new();
        let mut close = Vec::new();

        for i in 0..20 {
            let range = 1.0 + (i as f64) * 0.5; // Increasing range
            high.push(100.0 + range);
            low.push(100.0 - range);
            close.push(100.0);
        }

        let result = atr(&high, &low, &close, 5).unwrap();

        // ATR should generally increase
        let early_atr = result[7];
        let late_atr = result[18];
        assert!(
            late_atr > early_atr,
            "ATR should increase with increasing volatility"
        );
    }

    #[test]
    fn test_atr_decreasing_volatility() {
        // Simulate decreasing volatility - ATR should decrease
        let mut high = Vec::new();
        let mut low = Vec::new();
        let mut close = Vec::new();

        for i in 0..20 {
            let range = 10.0 - (i as f64) * 0.4; // Decreasing range
            let range = range.max(0.5); // Minimum range
            high.push(100.0 + range);
            low.push(100.0 - range);
            close.push(100.0);
        }

        let result = atr(&high, &low, &close, 5).unwrap();

        // ATR should generally decrease
        let early_atr = result[7];
        let late_atr = result[18];
        assert!(
            late_atr < early_atr,
            "ATR should decrease with decreasing volatility"
        );
    }

    #[test]
    fn test_atr_volatility_spike_and_decay() {
        // Normal volatility, spike, then return to normal
        let mut high = vec![101.0_f64; 30];
        let mut low = vec![99.0_f64; 30];
        let close = vec![100.0_f64; 30];

        // Create a spike at index 15
        high[15] = 120.0;
        low[15] = 80.0;

        let result = atr(&high, &low, &close, 5).unwrap();

        // Before spike
        let pre_spike = result[14];
        // At/after spike
        let spike_effect = result[16];
        // Long after spike
        let post_spike = result[28];

        assert!(spike_effect > pre_spike, "ATR should spike");
        assert!(post_spike < spike_effect, "ATR should decay after spike");
        assert!(
            post_spike > pre_spike * 0.9,
            "Some memory effect should remain"
        );
    }
}
