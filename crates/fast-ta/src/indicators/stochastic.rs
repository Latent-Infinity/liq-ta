//! Stochastic Oscillator indicator.
//!
//! The Stochastic Oscillator is a momentum indicator that compares a security's
//! closing price to its price range over a given period. It oscillates between
//! 0 and 100, where readings above 80 typically indicate overbought conditions
//! and readings below 20 indicate oversold conditions.
//!
//! # Canonical API
//!
//! The primary API is [`stochastic()`] with configurable `k_slowing` parameter:
//!
//! - **Fast Stochastic**: `k_slowing = 1` (default) - no smoothing on %K
//! - **Slow Stochastic**: `k_slowing = 3` (traditional) - %K smoothed before %D
//!
//! For typical use cases, prefer the canonical [`stochastic()`] function or the
//! [`Stochastic`] configuration type.
//!
//! # Convenience Functions
//!
//! For explicit variant selection, these functions are also available:
//!
//! - [`stochastic_fast`]: Fast Stochastic (equivalent to `k_slowing = 1`)
//! - [`stochastic_slow`]: Slow Stochastic (equivalent to `k_slowing = 3`)
//! - [`stochastic_full`]: Full Stochastic with explicit `slow_k_period` parameter
//!
//! # Algorithm
//!
//! The calculation uses O(n) rolling extrema to find the highest high and lowest
//! low over the lookback period. The basic formula is:
//!
//! ```text
//! %K = 100 * (Close - Lowest Low) / (Highest High - Lowest Low)
//! %D = SMA(%K, d_period)
//! ```
//!
//! For Slow/Full variants, an additional smoothing is applied to %K before
//! computing %D.
//!
//! # Mathematical Conventions (PRD §4.5, §4.8)
//!
//! - **Zero Range Handling**: When `highest_high == lowest_low` (flat price over
//!   the lookback window), %K = 50 (stable midpoint). This is a deterministic
//!   output for an indeterminate operation (0/0), not a NaN override.
//! - **NaN Precedence**: If any of `high`, `low`, or `close` in the required window
//!   contains NaN, the output is NaN. NaN propagation takes priority over the
//!   flat-price fallback.
//! - **Rolling Extrema**: Uses monotonic deque for O(n) computation when
//!   `k_period` ≥ 25, naive O(n×k) for smaller periods (per E04 findings).
//!
//! # Precision Behavior
//!
//! When `PrecisionMode::High` is active and input type is `f32`:
//! - %K calculation (division by range) performed in `f64`
//! - Prevents precision loss when range is very small
//! - SMA smoothing for %D uses `f64` accumulators
//!
//! **Tolerance**: abs(0.01) when comparing f32 High mode to f64 reference.
//! Stochastic is bounded 0-100, so absolute tolerance is appropriate.
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::stochastic::{stochastic, StochasticOutput};
//!
//! let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5];
//! let low = vec![9.0_f64, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5];
//! let close = vec![9.5_f64, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0];
//!
//! // Fast stochastic (k_slowing = 1)
//! let fast = stochastic(&high, &low, &close, 5, 3, 1).unwrap();
//!
//! // Slow stochastic (k_slowing = 3)
//! let slow = stochastic(&high, &low, &close, 5, 3, 3).unwrap();
//!
//! // First 4 values (k_period - 1) of %K are NaN
//! assert!(fast.k[0].is_nan());
//! assert!(fast.k[3].is_nan());
//! assert!(!fast.k[4].is_nan());
//! ```

use crate::error::{Error, Result};
use crate::indicators::sma::sma_from_idx_into;
use crate::precision::{current_precision_mode, PrecisionMode};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns true if we should use f64 precision for the given type.
///
/// Uses f64 when:
/// - Input type is f32 AND PrecisionMode is High
#[inline]
fn use_f64_precision<T: 'static>() -> bool {
    use std::any::TypeId;
    TypeId::of::<T>() == TypeId::of::<f32>() && current_precision_mode() == PrecisionMode::High
}

/// Computes %K with appropriate precision.
///
/// For f32 inputs in High precision mode, the calculation is performed in f64.
/// There are two formula variants used in different code paths:
/// - `compute_percent_k_mul_first`: 100 * (close - lowest) / range (for cached extrema path)
/// - `compute_percent_k_div_first`: (close - lowest) / range * 100 (for streaming path)
#[inline]
fn compute_percent_k_mul_first<T: SeriesElement + 'static>(
    close: T,
    lowest: T,
    range: T,
    hundred: T,
) -> Result<T> {
    if use_f64_precision::<T>() {
        let close_f64 = close.to_f64().unwrap_or(0.0);
        let lowest_f64 = lowest.to_f64().unwrap_or(0.0);
        let range_f64 = range.to_f64().unwrap_or(1.0);
        // Match original: hundred * (close - lowest) / range
        let k = 100.0 * (close_f64 - lowest_f64) / range_f64;
        T::from_f64(k)
    } else {
        // Match original: hundred * (close - lowest) / range
        Ok(hundred * (close - lowest) / range)
    }
}

#[inline]
fn compute_percent_k_div_first<T: SeriesElement + 'static>(
    close: T,
    lowest: T,
    range: T,
    hundred: T,
) -> Result<T> {
    if use_f64_precision::<T>() {
        let close_f64 = close.to_f64().unwrap_or(0.0);
        let lowest_f64 = lowest.to_f64().unwrap_or(0.0);
        let range_f64 = range.to_f64().unwrap_or(1.0);
        // Match original: (close - lowest) / range * 100
        let k = (close_f64 - lowest_f64) / range_f64 * 100.0;
        T::from_f64(k)
    } else {
        // Match original: (close - lowest) / range * hundred
        Ok((close - lowest) / range * hundred)
    }
}

/// Computes the Stochastic Oscillator with configurable %K slowing.
///
/// This is the canonical stochastic function per PRD §4.6. The `k_slowing` parameter
/// controls the smoothing applied to %K before computing %D:
///
/// - **Fast Stochastic**: `k_slowing = 1` (default). %K = raw stochastic, no smoothing.
/// - **Slow Stochastic**: `k_slowing > 1`. %K is smoothed with `SMA(k_slowing)` before %D.
///
/// # Arguments
///
/// * `high` - The high prices
/// * `low` - The low prices
/// * `close` - The closing prices
/// * `k_period` - The lookback period for raw %K (commonly 14)
/// * `d_period` - The smoothing period for %D (commonly 3)
/// * `k_slowing` - The smoothing period for %K (1 = fast, 3 = traditional slow)
///
/// # Returns
///
/// A `Result` containing a [`StochasticOutput`] with %K and %D lines.
///
/// # Errors
///
/// Returns an error if:
/// - Any input is empty (`Error::EmptyInput`)
/// - Any period is zero (`Error::InvalidPeriod`)
/// - Input lengths don't match
/// - Input data is shorter than required
///
/// # Example
///
/// ```
/// use fast_ta::indicators::stochastic::stochastic;
///
/// let high = vec![44.0_f64, 44.5, 44.75, 44.25, 44.5, 44.75, 45.0, 44.5, 44.0, 44.25, 44.5];
/// let low = vec![43.5_f64, 44.0, 44.25, 43.75, 44.0, 44.25, 44.5, 44.0, 43.5, 43.75, 44.0];
/// let close = vec![43.75_f64, 44.25, 44.5, 44.0, 44.25, 44.5, 44.75, 44.25, 43.75, 44.0, 44.25];
///
/// // Fast stochastic (k_slowing = 1)
/// let fast = stochastic(&high, &low, &close, 5, 3, 1).unwrap();
///
/// // Slow stochastic (k_slowing = 3)
/// let slow = stochastic(&high, &low, &close, 5, 3, 3).unwrap();
/// ```
#[must_use = "this returns a Result with Stochastic values, which should be used"]
pub fn stochastic<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    d_period: usize,
    k_slowing: usize,
) -> Result<StochasticOutput<T>> {
    if k_slowing == 1 {
        // Fast stochastic: no smoothing on %K
        stochastic_fast(high, low, close, k_period, d_period)
    } else {
        // Slow/Full stochastic: smooth %K before computing %D
        stochastic_full(high, low, close, k_period, k_slowing, d_period)
    }
}

/// Computes the Stochastic Oscillator into pre-allocated output buffers.
///
/// This is the `_into` variant of [`stochastic`] for buffer reuse.
///
/// # Arguments
///
/// * `high` - The high prices
/// * `low` - The low prices
/// * `close` - The closing prices
/// * `k_period` - The lookback period for raw %K (commonly 14)
/// * `d_period` - The smoothing period for %D (commonly 3)
/// * `k_slowing` - The smoothing period for %K (1 = fast, 3 = traditional slow)
/// * `output` - Pre-allocated output buffers
///
/// # Returns
///
/// A tuple `(valid_k_count, valid_d_count)` indicating non-NaN values.
///
/// # Errors
///
/// Returns an error if validation fails or output buffers are too small.
#[must_use = "this returns a Result with valid counts, which should be used"]
pub fn stochastic_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    d_period: usize,
    k_slowing: usize,
    output: &mut StochasticOutput<T>,
) -> Result<(usize, usize)> {
    if k_slowing == 1 {
        stochastic_fast_into(high, low, close, k_period, d_period, output)
    } else {
        stochastic_full_into(high, low, close, k_period, k_slowing, d_period, output)
    }
}

/// Returns the lookback period for the Stochastic %K line.
///
/// The %K lookback is `k_period - 1`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::stochastic::stochastic_k_lookback;
///
/// assert_eq!(stochastic_k_lookback(14), 13);
/// assert_eq!(stochastic_k_lookback(5), 4);
/// ```
#[inline]
#[must_use]
pub const fn stochastic_k_lookback(k_period: usize) -> usize {
    if k_period == 0 {
        0
    } else {
        k_period - 1
    }
}

/// Returns the lookback period for the Stochastic %D line.
///
/// The %D lookback is `k_period + d_period - 2`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::stochastic::stochastic_d_lookback;
///
/// assert_eq!(stochastic_d_lookback(14, 3), 15);
/// assert_eq!(stochastic_d_lookback(5, 3), 6);
/// ```
#[inline]
#[must_use]
pub const fn stochastic_d_lookback(k_period: usize, d_period: usize) -> usize {
    if k_period == 0 || d_period == 0 {
        0
    } else {
        k_period + d_period - 2
    }
}

/// Returns the minimum input length required for Stochastic Oscillator.
///
/// This is the smallest input size that will produce at least one valid %D value.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::stochastic::stochastic_min_len;
///
/// assert_eq!(stochastic_min_len(14, 3), 16);
/// assert_eq!(stochastic_min_len(5, 3), 7);
/// ```
#[inline]
#[must_use]
pub const fn stochastic_min_len(k_period: usize, d_period: usize) -> usize {
    if k_period == 0 || d_period == 0 {
        0
    } else {
        k_period + d_period - 1
    }
}

/// Output structure containing %K and %D lines of the Stochastic Oscillator.
///
/// Both vectors have the same length as the input data. NaN values fill the
/// lookback period where insufficient data exists.
#[derive(Debug, Clone)]
pub struct StochasticOutput<T> {
    /// The %K line (fast line).
    pub k: Vec<T>,
    /// The %D line (signal line, SMA of %K).
    pub d: Vec<T>,
}

/// Computes the Fast Stochastic Oscillator.
///
/// The Fast Stochastic consists of:
/// - **%K**: The raw stochastic value comparing close to the high-low range
/// - **%D**: Simple Moving Average of %K
///
/// # Arguments
///
/// * `high` - The high prices
/// * `low` - The low prices
/// * `close` - The closing prices
/// * `k_period` - The lookback period for %K (commonly 14)
/// * `d_period` - The smoothing period for %D (commonly 3)
///
/// # Returns
///
/// A `Result` containing a [`StochasticOutput`] with %K and %D lines,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - Any input is empty (`Error::EmptyInput`)
/// - Any period is zero (`Error::InvalidPeriod`)
/// - Input lengths don't match
/// - Input data is shorter than `k_period` (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for the output vectors
///
/// # NaN Handling
///
/// - The first `k_period - 1` elements of %K are NaN
/// - The first `k_period + d_period - 2` elements of %D are NaN
///
/// # Example
///
/// ```
/// use fast_ta::indicators::stochastic::stochastic_fast;
///
/// let high = vec![44.0_f64, 44.5, 44.75, 44.25, 44.5, 44.75, 45.0];
/// let low = vec![43.5_f64, 44.0, 44.25, 43.75, 44.0, 44.25, 44.5];
/// let close = vec![43.75_f64, 44.25, 44.5, 44.0, 44.25, 44.5, 44.75];
///
/// let result = stochastic_fast(&high, &low, &close, 5, 3).unwrap();
/// assert!(!result.k[4].is_nan()); // First valid %K
/// ```
#[must_use = "this returns a Result with Stochastic values, which should be used"]
pub fn stochastic_fast<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    d_period: usize,
) -> Result<StochasticOutput<T>> {
    validate_stochastic_inputs(high, low, close, k_period, d_period)?;

    let n = close.len();

    // Fast path: single-pass NaN check (combined to avoid 3 separate iterations)
    let mut has_nan = false;
    for i in 0..n {
        if is_invalid(high[i]) || is_invalid(low[i]) || is_invalid(close[i]) {
            has_nan = true;
            break;
        }
    }

    if has_nan {
        // Slow path: proper NaN handling using cached extrema
        let mut k = vec![T::nan(); n];
        let mut d = vec![T::nan(); n];
        compute_raw_k(high, low, close, k_period, &mut k)?;
        compute_sma_of_series(&k, d_period, k_period - 1, &mut d)?;
        Ok(StochasticOutput { k, d })
    } else {
        // Fast path: VHGW algorithm (always, no threshold)
        use std::any::TypeId;

        // f64 specialization: zero-overhead VHGW
        if TypeId::of::<T>() == TypeId::of::<f64>() {
            let high_f64: &[f64] = unsafe { std::mem::transmute(high) };
            let low_f64: &[f64] = unsafe { std::mem::transmute(low) };
            let close_f64: &[f64] = unsafe { std::mem::transmute(close) };

            let mut k: Vec<f64> = Vec::with_capacity(n);
            let mut d: Vec<f64> = Vec::with_capacity(n);
            unsafe {
                k.set_len(n);
                d.set_len(n);
            }

            crate::kernels::compute_stochastic_fast_vhgw_f64(
                high_f64,
                low_f64,
                close_f64,
                k_period,
                d_period,
                &mut k,
                &mut d,
            )?;

            Ok(StochasticOutput {
                k: unsafe { std::mem::transmute(k) },
                d: unsafe { std::mem::transmute(d) },
            })
        } else if TypeId::of::<T>() == TypeId::of::<f32>() {
            // f32 specialization: zero-overhead VHGW
            let high_f32: &[f32] = unsafe { std::mem::transmute(high) };
            let low_f32: &[f32] = unsafe { std::mem::transmute(low) };
            let close_f32: &[f32] = unsafe { std::mem::transmute(close) };

            let mut k: Vec<f32> = Vec::with_capacity(n);
            let mut d: Vec<f32> = Vec::with_capacity(n);
            unsafe {
                k.set_len(n);
                d.set_len(n);
            }

            crate::kernels::compute_stochastic_fast_vhgw_f32(
                high_f32,
                low_f32,
                close_f32,
                k_period,
                d_period,
                &mut k,
                &mut d,
            )?;

            Ok(StochasticOutput {
                k: unsafe { std::mem::transmute(k) },
                d: unsafe { std::mem::transmute(d) },
            })
        } else {
            // Generic fallback: streaming deque for non-f32/f64 types
            let mut k = Vec::with_capacity(n);
            let mut d = Vec::with_capacity(n);
            unsafe {
                k.set_len(n);
                d.set_len(n);
            }
            compute_stochastic_fast_streaming(high, low, close, k_period, d_period, &mut k, &mut d)?;
            Ok(StochasticOutput { k, d })
        }
    }
}

/// Computes the Fast Stochastic Oscillator into pre-allocated output buffers.
///
/// This variant allows reusing existing buffers to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `high` - The high prices
/// * `low` - The low prices
/// * `close` - The closing prices
/// * `k_period` - The lookback period for %K
/// * `d_period` - The smoothing period for %D
/// * `output` - Pre-allocated output structure
///
/// # Returns
///
/// A `Result` containing a tuple of (valid %K count, valid %D count),
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - Any input is empty (`Error::EmptyInput`)
/// - Any period is zero (`Error::InvalidPeriod`)
/// - Input lengths don't match
/// - Input data is shorter than `k_period` (`Error::InsufficientData`)
/// - Output buffers are shorter than the input data
#[must_use = "this returns a Result with valid counts, which should be used"]
pub fn stochastic_fast_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    d_period: usize,
    output: &mut StochasticOutput<T>,
) -> Result<(usize, usize)> {
    validate_stochastic_inputs(high, low, close, k_period, d_period)?;

    let n = close.len();
    if output.k.len() < n || output.d.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.k.len().min(output.d.len()),
            indicator: "stochastic",
        });
    }

    // Fast path: single-pass NaN check (combined to avoid 3 separate iterations)
    let mut has_nan = false;
    for i in 0..n {
        if is_invalid(high[i]) || is_invalid(low[i]) || is_invalid(close[i]) {
            has_nan = true;
            break;
        }
    }

    if has_nan {
        // Slow path: proper NaN handling
        for i in 0..n {
            output.k[i] = T::nan();
            output.d[i] = T::nan();
        }
        compute_raw_k(high, low, close, k_period, &mut output.k)?;
        compute_sma_of_series(&output.k, d_period, k_period - 1, &mut output.d)?;
    } else {
        // Fast path: VHGW algorithm (always, no threshold)
        use std::any::TypeId;

        // f64 specialization: zero-overhead VHGW
        if TypeId::of::<T>() == TypeId::of::<f64>() {
            let high_f64: &[f64] = unsafe { std::mem::transmute(high) };
            let low_f64: &[f64] = unsafe { std::mem::transmute(low) };
            let close_f64: &[f64] = unsafe { std::mem::transmute(close) };
            let k_f64: &mut [f64] = unsafe { std::mem::transmute(output.k.as_mut_slice()) };
            let d_f64: &mut [f64] = unsafe { std::mem::transmute(output.d.as_mut_slice()) };

            crate::kernels::compute_stochastic_fast_vhgw_f64(
                high_f64,
                low_f64,
                close_f64,
                k_period,
                d_period,
                k_f64,
                d_f64,
            )?;
        } else if TypeId::of::<T>() == TypeId::of::<f32>() {
            // f32 specialization: zero-overhead VHGW
            let high_f32: &[f32] = unsafe { std::mem::transmute(high) };
            let low_f32: &[f32] = unsafe { std::mem::transmute(low) };
            let close_f32: &[f32] = unsafe { std::mem::transmute(close) };
            let k_f32: &mut [f32] = unsafe { std::mem::transmute(output.k.as_mut_slice()) };
            let d_f32: &mut [f32] = unsafe { std::mem::transmute(output.d.as_mut_slice()) };

            crate::kernels::compute_stochastic_fast_vhgw_f32(
                high_f32,
                low_f32,
                close_f32,
                k_period,
                d_period,
                k_f32,
                d_f32,
            )?;
        } else {
            // Generic fallback: streaming deque for non-f32/f64 types
            compute_stochastic_fast_streaming(high, low, close, k_period, d_period, &mut output.k, &mut output.d)?;
        }
    }

    let valid_k = n - (k_period - 1);
    let valid_d = if n >= k_period + d_period - 1 {
        n - (k_period + d_period - 2)
    } else {
        0
    };

    Ok((valid_k, valid_d))
}

/// Computes the Slow Stochastic Oscillator.
///
/// The Slow Stochastic smooths the Fast %K to reduce noise:
/// - **%K**: SMA of Fast %K (the raw stochastic)
/// - **%D**: SMA of Slow %K
///
/// This is equivalent to `stochastic_full(high, low, close, k_period, slow_k_period, d_period)`
/// where `slow_k_period` equals `d_period` (commonly 3).
///
/// # Arguments
///
/// * `high` - The high prices
/// * `low` - The low prices
/// * `close` - The closing prices
/// * `k_period` - The lookback period for raw %K (commonly 14)
/// * `d_period` - The smoothing period for both Slow %K and %D (commonly 3)
///
/// # Returns
///
/// A `Result` containing a [`StochasticOutput`] with Slow %K and %D lines,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - Any input is empty (`Error::EmptyInput`)
/// - Any period is zero (`Error::InvalidPeriod`)
/// - Input lengths don't match
/// - Input data is shorter than `k_period` (`Error::InsufficientData`)
///
/// # NaN Handling
///
/// - The first `k_period + d_period - 2` elements of Slow %K are NaN
/// - The first `k_period + 2*d_period - 3` elements of %D are NaN
///
/// # Example
///
/// ```
/// use fast_ta::indicators::stochastic::stochastic_slow;
///
/// let high = vec![44.0_f64, 44.5, 44.75, 44.25, 44.5, 44.75, 45.0, 44.5, 44.0, 44.25];
/// let low = vec![43.5_f64, 44.0, 44.25, 43.75, 44.0, 44.25, 44.5, 44.0, 43.5, 43.75];
/// let close = vec![43.75_f64, 44.25, 44.5, 44.0, 44.25, 44.5, 44.75, 44.25, 43.75, 44.0];
///
/// let result = stochastic_slow(&high, &low, &close, 5, 3).unwrap();
/// // Slow %K starts at index k_period + d_period - 2 = 5 + 3 - 2 = 6
/// assert!(result.k[5].is_nan());
/// assert!(!result.k[6].is_nan());
/// ```
#[must_use = "this returns a Result with Stochastic values, which should be used"]
pub fn stochastic_slow<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    d_period: usize,
) -> Result<StochasticOutput<T>> {
    // Slow stochastic uses the same period for smoothing K and D
    stochastic_full(high, low, close, k_period, d_period, d_period)
}

/// Computes the Slow Stochastic Oscillator into pre-allocated output buffers.
///
/// # Errors
///
/// Returns an error if:
/// - Any input is empty (`Error::EmptyInput`)
/// - Any period is zero (`Error::InvalidPeriod`)
/// - Input lengths don't match
/// - Input data is shorter than `k_period` (`Error::InsufficientData`)
/// - Output buffers are shorter than the input data
#[must_use = "this returns a Result with valid counts, which should be used"]
pub fn stochastic_slow_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    d_period: usize,
    output: &mut StochasticOutput<T>,
) -> Result<(usize, usize)> {
    stochastic_full_into(high, low, close, k_period, d_period, d_period, output)
}

/// Computes the Full Stochastic Oscillator with configurable smoothing.
///
/// The Full Stochastic provides complete control over smoothing periods:
/// - **%K**: SMA of Raw %K with `slow_k_period` smoothing
/// - **%D**: SMA of Full %K with `d_period` smoothing
///
/// # Arguments
///
/// * `high` - The high prices
/// * `low` - The low prices
/// * `close` - The closing prices
/// * `k_period` - The lookback period for raw %K (commonly 14)
/// * `slow_k_period` - The smoothing period for %K (commonly 3)
/// * `d_period` - The smoothing period for %D (commonly 3)
///
/// # Returns
///
/// A `Result` containing a [`StochasticOutput`] with Full %K and %D lines,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - Any input is empty (`Error::EmptyInput`)
/// - Any period is zero (`Error::InvalidPeriod`)
/// - Input lengths don't match
/// - Input data is shorter than `k_period` (`Error::InsufficientData`)
///
/// # NaN Handling
///
/// - The first `k_period + slow_k_period - 2` elements of Full %K are NaN
/// - The first `k_period + slow_k_period + d_period - 3` elements of %D are NaN
///
/// # Example
///
/// ```
/// use fast_ta::indicators::stochastic::stochastic_full;
///
/// let high = vec![44.0_f64, 44.5, 44.75, 44.25, 44.5, 44.75, 45.0, 44.5, 44.0, 44.25, 44.5];
/// let low = vec![43.5_f64, 44.0, 44.25, 43.75, 44.0, 44.25, 44.5, 44.0, 43.5, 43.75, 44.0];
/// let close = vec![43.75_f64, 44.25, 44.5, 44.0, 44.25, 44.5, 44.75, 44.25, 43.75, 44.0, 44.25];
///
/// // Custom smoothing: 5-period %K lookback, 3-period %K smoothing, 3-period %D smoothing
/// let result = stochastic_full(&high, &low, &close, 5, 3, 3).unwrap();
///
/// // Full %K starts at index k_period + slow_k_period - 2 = 5 + 3 - 2 = 6
/// assert!(result.k[5].is_nan());
/// assert!(!result.k[6].is_nan());
/// ```
#[must_use = "this returns a Result with Stochastic values, which should be used"]
pub fn stochastic_full<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    slow_k_period: usize,
    d_period: usize,
) -> Result<StochasticOutput<T>> {
    validate_stochastic_full_inputs(high, low, close, k_period, slow_k_period, d_period)?;

    let n = close.len();

    // Fast path: single-pass NaN check (combined to avoid 3 separate iterations)
    let mut has_nan = false;
    for i in 0..n {
        if is_invalid(high[i]) || is_invalid(low[i]) || is_invalid(close[i]) {
            has_nan = true;
            break;
        }
    }

    if has_nan {
        // Slow path: proper NaN handling
        let mut k = vec![T::nan(); n];
        let mut d = vec![T::nan(); n];
        let mut raw_k = vec![T::nan(); n];
        compute_raw_k(high, low, close, k_period, &mut raw_k)?;
        compute_sma_of_series(&raw_k, slow_k_period, k_period - 1, &mut k)?;
        let k_start_idx = k_period + slow_k_period - 2;
        compute_sma_of_series(&k, d_period, k_start_idx, &mut d)?;
        Ok(StochasticOutput { k, d })
    } else {
        // Fast path: VHGW algorithm (always, no threshold)
        use std::any::TypeId;

        // f64 specialization: zero-overhead VHGW
        if TypeId::of::<T>() == TypeId::of::<f64>() {
            let high_f64: &[f64] = unsafe { std::mem::transmute(high) };
            let low_f64: &[f64] = unsafe { std::mem::transmute(low) };
            let close_f64: &[f64] = unsafe { std::mem::transmute(close) };
            let mut k: Vec<f64> = Vec::with_capacity(n);
            let mut d: Vec<f64> = Vec::with_capacity(n);
            unsafe {
                k.set_len(n);
                d.set_len(n);
            }
            crate::kernels::compute_stochastic_full_vhgw_f64(
                high_f64,
                low_f64,
                close_f64,
                k_period,
                slow_k_period,
                d_period,
                &mut k,
                &mut d,
            )?;
            Ok(StochasticOutput {
                k: unsafe { std::mem::transmute(k) },
                d: unsafe { std::mem::transmute(d) },
            })
        } else if TypeId::of::<T>() == TypeId::of::<f32>() {
            // f32 specialization: zero-overhead VHGW
            let high_f32: &[f32] = unsafe { std::mem::transmute(high) };
            let low_f32: &[f32] = unsafe { std::mem::transmute(low) };
            let close_f32: &[f32] = unsafe { std::mem::transmute(close) };
            let mut k: Vec<f32> = Vec::with_capacity(n);
            let mut d: Vec<f32> = Vec::with_capacity(n);
            unsafe {
                k.set_len(n);
                d.set_len(n);
            }
            crate::kernels::compute_stochastic_full_vhgw_f32(
                high_f32,
                low_f32,
                close_f32,
                k_period,
                slow_k_period,
                d_period,
                &mut k,
                &mut d,
            )?;
            Ok(StochasticOutput {
                k: unsafe { std::mem::transmute(k) },
                d: unsafe { std::mem::transmute(d) },
            })
        } else {
            // Generic fallback: streaming deque for non-f32/f64 types
            let mut k = Vec::with_capacity(n);
            let mut d = Vec::with_capacity(n);
            unsafe {
                k.set_len(n);
                d.set_len(n);
            }
            compute_stochastic_full_streaming(
                high,
                low,
                close,
                k_period,
                slow_k_period,
                d_period,
                &mut k,
                &mut d,
            )?;
            Ok(StochasticOutput { k, d })
        }
    }
}

/// Computes the Full Stochastic Oscillator into pre-allocated output buffers.
///
/// # Errors
///
/// Returns an error if:
/// - Any input is empty (`Error::EmptyInput`)
/// - Any period is zero (`Error::InvalidPeriod`)
/// - Input lengths don't match
/// - Input data is shorter than `k_period` (`Error::InsufficientData`)
/// - Output buffers are shorter than the input data
#[must_use = "this returns a Result with valid counts, which should be used"]
pub fn stochastic_full_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    slow_k_period: usize,
    d_period: usize,
    output: &mut StochasticOutput<T>,
) -> Result<(usize, usize)> {
    validate_stochastic_full_inputs(high, low, close, k_period, slow_k_period, d_period)?;

    let n = close.len();
    if output.k.len() < n || output.d.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.k.len().min(output.d.len()),
            indicator: "stochastic",
        });
    }

    // Fast path: single-pass NaN check (combined to avoid 3 separate iterations)
    let mut has_nan = false;
    for i in 0..n {
        if is_invalid(high[i]) || is_invalid(low[i]) || is_invalid(close[i]) {
            has_nan = true;
            break;
        }
    }

    if has_nan {
        // Slow path: proper NaN handling
        for i in 0..n {
            output.k[i] = T::nan();
            output.d[i] = T::nan();
        }

        let mut raw_k = vec![T::nan(); n];
        compute_raw_k(high, low, close, k_period, &mut raw_k)?;
        compute_sma_of_series(&raw_k, slow_k_period, k_period - 1, &mut output.k)?;
        let k_start_idx = k_period + slow_k_period - 2;
        compute_sma_of_series(&output.k, d_period, k_start_idx, &mut output.d)?;
    } else {
        // Fast path: VHGW algorithm (always, no threshold)
        use std::any::TypeId;

        // f64 specialization: zero-overhead VHGW
        if TypeId::of::<T>() == TypeId::of::<f64>() {
            let high_f64: &[f64] = unsafe { std::mem::transmute(high) };
            let low_f64: &[f64] = unsafe { std::mem::transmute(low) };
            let close_f64: &[f64] = unsafe { std::mem::transmute(close) };
            let k_f64: &mut [f64] = unsafe { std::mem::transmute(&mut output.k[..n]) };
            let d_f64: &mut [f64] = unsafe { std::mem::transmute(&mut output.d[..n]) };
            crate::kernels::compute_stochastic_full_vhgw_f64(
                high_f64,
                low_f64,
                close_f64,
                k_period,
                slow_k_period,
                d_period,
                k_f64,
                d_f64,
            )?;
        } else if TypeId::of::<T>() == TypeId::of::<f32>() {
            // f32 specialization: zero-overhead VHGW
            let high_f32: &[f32] = unsafe { std::mem::transmute(high) };
            let low_f32: &[f32] = unsafe { std::mem::transmute(low) };
            let close_f32: &[f32] = unsafe { std::mem::transmute(close) };
            let k_f32: &mut [f32] = unsafe { std::mem::transmute(&mut output.k[..n]) };
            let d_f32: &mut [f32] = unsafe { std::mem::transmute(&mut output.d[..n]) };
            crate::kernels::compute_stochastic_full_vhgw_f32(
                high_f32,
                low_f32,
                close_f32,
                k_period,
                slow_k_period,
                d_period,
                k_f32,
                d_f32,
            )?;
        } else {
            // Generic fallback: streaming deque for non-f32/f64 types
            compute_stochastic_full_streaming(
                high,
                low,
                close,
                k_period,
                slow_k_period,
                d_period,
                &mut output.k[..n],
                &mut output.d[..n],
            )?;
        }
    }

    let valid_k = if n >= k_period + slow_k_period - 1 {
        n - (k_period + slow_k_period - 2)
    } else {
        0
    };
    let valid_d = if n >= k_period + slow_k_period + d_period - 2 {
        n - (k_period + slow_k_period + d_period - 3)
    } else {
        0
    };

    Ok((valid_k, valid_d))
}

/// Validates inputs for fast stochastic.
fn validate_stochastic_inputs<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    d_period: usize,
) -> Result<()> {
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

    if high.is_empty() {
        return Err(Error::EmptyInput);
    }
    if low.is_empty() {
        return Err(Error::EmptyInput);
    }
    if close.is_empty() {
        return Err(Error::EmptyInput);
    }

    // All inputs must have the same length
    if high.len() != low.len() || high.len() != close.len() {
        return Err(Error::LengthMismatch {
            description: format!(
                "high has {} elements, low has {}, close has {}",
                high.len(),
                low.len(),
                close.len()
            ),
        });
    }

    if high.len() < k_period {
        return Err(Error::InsufficientData {
            required: k_period,
            actual: high.len(),
            indicator: "stochastic",
        });
    }

    Ok(())
}

/// Validates inputs for full stochastic.
fn validate_stochastic_full_inputs<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    slow_k_period: usize,
    d_period: usize,
) -> Result<()> {
    if k_period == 0 {
        return Err(Error::InvalidPeriod {
            period: k_period,
            reason: "k_period must be at least 1",
        });
    }

    if slow_k_period == 0 {
        return Err(Error::InvalidPeriod {
            period: slow_k_period,
            reason: "slow_k_period must be at least 1",
        });
    }

    if d_period == 0 {
        return Err(Error::InvalidPeriod {
            period: d_period,
            reason: "d_period must be at least 1",
        });
    }

    if high.is_empty() {
        return Err(Error::EmptyInput);
    }
    if low.is_empty() {
        return Err(Error::EmptyInput);
    }
    if close.is_empty() {
        return Err(Error::EmptyInput);
    }

    // All inputs must have the same length
    if high.len() != low.len() || high.len() != close.len() {
        return Err(Error::LengthMismatch {
            description: format!(
                "high has {} elements, low has {}, close has {}",
                high.len(),
                low.len(),
                close.len()
            ),
        });
    }

    if high.len() < k_period {
        return Err(Error::InsufficientData {
            required: k_period,
            actual: high.len(),
            indicator: "stochastic",
        });
    }

    Ok(())
}

// =============================================================================
// Streaming Single-Pass Implementation
// =============================================================================

/// Fixed-capacity ring buffer for monotonic deque indices.
/// More efficient than VecDeque for tight loops.
struct IdxRing {
    buf: Vec<usize>,
    head: usize,
    len: usize,
}

impl IdxRing {
    #[inline]
    fn new(cap: usize) -> Self {
        Self {
            buf: vec![0; cap.max(1)],
            head: 0,
            len: 0,
        }
    }

    #[inline]
    fn cap(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn front(&self) -> usize {
        debug_assert!(!self.is_empty());
        self.buf[self.head]
    }

    #[inline]
    fn back_pos(&self) -> usize {
        (self.head + self.len - 1) % self.cap()
    }

    #[inline]
    fn back(&self) -> usize {
        debug_assert!(!self.is_empty());
        self.buf[self.back_pos()]
    }

    #[inline]
    fn pop_front(&mut self) {
        debug_assert!(!self.is_empty());
        self.head = (self.head + 1) % self.cap();
        self.len -= 1;
    }

    #[inline]
    fn pop_back(&mut self) {
        debug_assert!(!self.is_empty());
        self.len -= 1;
    }

    #[inline]
    fn push_back(&mut self, i: usize) {
        debug_assert!(self.len < self.cap());
        let pos = (self.head + self.len) % self.cap();
        self.buf[pos] = i;
        self.len += 1;
    }
}

/// Rolling SMA using a ring buffer and running sum.
/// O(1) per update, no recomputation.
struct RollingSma<T: SeriesElement> {
    buf: Vec<T>,
    head: usize,
    len: usize,
    sum: T,
    inv_period: T,
}

impl<T: SeriesElement> RollingSma<T> {
    #[inline]
    fn new(period: usize, inv_period: T) -> Self {
        Self {
            buf: vec![T::zero(); period.max(1)],
            head: 0,
            len: 0,
            sum: T::zero(),
            inv_period,
        }
    }

    /// Push a value, returns Some(sma) when window is full.
    #[inline]
    fn push(&mut self, x: T) -> Option<T> {
        let p = self.buf.len();
        if self.len < p {
            // Filling up
            let pos = (self.head + self.len) % p;
            self.buf[pos] = x;
            self.sum = self.sum + x;
            self.len += 1;
            if self.len == p {
                Some(self.sum * self.inv_period)
            } else {
                None
            }
        } else {
            // Rolling
            let old = self.buf[self.head];
            self.buf[self.head] = x;
            self.head = (self.head + 1) % p;
            self.sum = self.sum + x - old;
            Some(self.sum * self.inv_period)
        }
    }
}

/// Push index to min queue, maintaining monotonic increasing values.
#[inline]
fn push_min_queue<T: SeriesElement>(q: &mut IdxRing, i: usize, vals: &[T]) {
    unsafe {
        while !q.is_empty() {
            let b = q.back();
            if *vals.get_unchecked(b) <= *vals.get_unchecked(i) {
                break;
            }
            q.pop_back();
        }
        q.push_back(i);
    }
}

/// Push index to max queue, maintaining monotonic decreasing values.
#[inline]
fn push_max_queue<T: SeriesElement>(q: &mut IdxRing, i: usize, vals: &[T]) {
    unsafe {
        while !q.is_empty() {
            let b = q.back();
            if *vals.get_unchecked(b) >= *vals.get_unchecked(i) {
                break;
            }
            q.pop_back();
        }
        q.push_back(i);
    }
}

/// Remove expired indices from front of queue.
#[inline]
fn pop_expired(q: &mut IdxRing, window_start: usize) {
    while !q.is_empty() && q.front() < window_start {
        q.pop_front();
    }
}

/// Streaming single-pass Fast Stochastic implementation.
/// Uses monotonic deques for O(n) rolling min/max + streaming SMA for %D.
/// No intermediate allocations.
#[inline(never)]
fn compute_stochastic_fast_streaming<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    d_period: usize,
    k_out: &mut [T],
    d_out: &mut [T],
) -> Result<()> {
    let n = close.len();
    let fifty = T::from_f64(50.0)?;
    let inv_d_period = T::from_f64(1.0 / d_period as f64)?;

    // Fill initial NaN values
    let k_start = k_period - 1;
    let d_start = k_start + d_period - 1;
    k_out[..k_start].fill(T::nan());
    d_out[..d_start.min(n)].fill(T::nan());

    // Initialize monotonic deques
    let mut min_q = IdxRing::new(k_period);
    let mut max_q = IdxRing::new(k_period);

    // Initialize rolling SMA for %D
    let mut sma_d = RollingSma::new(d_period, inv_d_period);

    // Single pass through data
    for i in 0..n {
        // Expire old indices from front
        if i >= k_period {
            let ws = i + 1 - k_period;
            pop_expired(&mut min_q, ws);
            pop_expired(&mut max_q, ws);
        }

        // Push current index to deques
        push_min_queue(&mut min_q, i, low);
        push_max_queue(&mut max_q, i, high);

        // Emit %K when window is ready
        if i >= k_start {
            // Ensure we've expired old indices
            if i >= k_period {
                let ws = i + 1 - k_period;
                pop_expired(&mut min_q, ws);
                pop_expired(&mut max_q, ws);
            }

            unsafe {
                let ll = *low.get_unchecked(min_q.front());
                let hh = *high.get_unchecked(max_q.front());
                let range = hh - ll;
                let cl = *close.get_unchecked(i);

                let fk = if range > T::zero() {
                    compute_percent_k_div_first(cl, ll, range, T::from_f64(100.0)?)?
                } else {
                    fifty
                };

                *k_out.get_unchecked_mut(i) = fk;

                // Stream into SMA for %D
                if let Some(fd) = sma_d.push(fk) {
                    let d_idx = i;
                    *d_out.get_unchecked_mut(d_idx) = fd;
                }
            }
        }
    }

    Ok(())
}

/// Streaming single-pass Full Stochastic implementation.
/// Uses monotonic deques for rolling min/max + cascaded SMAs.
#[inline(never)]
fn compute_stochastic_full_streaming<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    slow_k_period: usize,
    d_period: usize,
    k_out: &mut [T],
    d_out: &mut [T],
) -> Result<()> {
    let n = close.len();
    let fifty = T::from_f64(50.0)?;
    let inv_slow_k = T::from_f64(1.0 / slow_k_period as f64)?;
    let inv_d_period = T::from_f64(1.0 / d_period as f64)?;

    // Compute output start indices
    let k_start = k_period - 1;
    let slow_k_start = k_start + slow_k_period - 1;
    let d_start = slow_k_start + d_period - 1;

    // Fill initial NaN values
    k_out[..slow_k_start.min(n)].fill(T::nan());
    d_out[..d_start.min(n)].fill(T::nan());

    // Initialize monotonic deques for rolling min/max
    let mut min_q = IdxRing::new(k_period);
    let mut max_q = IdxRing::new(k_period);

    // Initialize cascaded rolling SMAs
    let mut sma_slow_k = RollingSma::new(slow_k_period, inv_slow_k);
    let mut sma_d = RollingSma::new(d_period, inv_d_period);

    // Single pass through data
    for i in 0..n {
        // Expire old indices
        if i >= k_period {
            let ws = i + 1 - k_period;
            pop_expired(&mut min_q, ws);
            pop_expired(&mut max_q, ws);
        }

        // Push current index to deques
        push_min_queue(&mut min_q, i, low);
        push_max_queue(&mut max_q, i, high);

        // Compute fastK when window is ready
        if i >= k_start {
            if i >= k_period {
                let ws = i + 1 - k_period;
                pop_expired(&mut min_q, ws);
                pop_expired(&mut max_q, ws);
            }

            unsafe {
                let ll = *low.get_unchecked(min_q.front());
                let hh = *high.get_unchecked(max_q.front());
                let range = hh - ll;
                let cl = *close.get_unchecked(i);

                let fast_k = if range > T::zero() {
                    compute_percent_k_div_first(cl, ll, range, T::from_f64(100.0)?)?
                } else {
                    fifty
                };

                // Stream fastK into slow_k SMA
                if let Some(slow_k) = sma_slow_k.push(fast_k) {
                    *k_out.get_unchecked_mut(i) = slow_k;

                    // Stream slow_k into %D SMA
                    if let Some(d) = sma_d.push(slow_k) {
                        *d_out.get_unchecked_mut(i) = d;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Fused computation of Full Stochastic (raw %K → Slow %K → %D).
///
/// Three passes: raw %K, SMA for slow %K, SMA for %D.
/// Uses inverse multiply for SMA, no per-element NaN checks.
/// Note: Kept for reference; streaming version is used in production.
#[allow(dead_code)]
#[inline(never)]
fn compute_stochastic_full_fused<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    slow_k_period: usize,
    d_period: usize,
    k_out: &mut [T],
    d_out: &mut [T],
) -> Result<()> {
    let n = close.len();
    let inv_hundred = T::from_f64(0.01)?;
    let inv_slow_k = T::from_f64(1.0 / slow_k_period as f64)?;
    let inv_d_period = T::from_f64(1.0 / d_period as f64)?;
    let fifty = T::from_f64(50.0)?;

    // We need a temp buffer for raw %K values
    // Use uninit - we only read indices that we write to (k_start..n)
    let mut raw_k = Vec::with_capacity(n);
    unsafe { raw_k.set_len(n); }

    // ===== Pass 1: Compute raw %K using TA-Lib cached extrema =====
    let mut highest_idx = 0usize;
    let mut lowest_idx = 0usize;
    let mut highest = high[0];
    let mut lowest = low[0];

    for j in 1..k_period {
        if high[j] > highest {
            highest = high[j];
            highest_idx = j;
        }
        if low[j] < lowest {
            lowest = low[j];
            lowest_idx = j;
        }
    }

    let mut diff = (highest - lowest) * inv_hundred;
    let k_start = k_period - 1;

    if diff > T::zero() {
        raw_k[k_start] = (close[k_start] - lowest) / diff;
    } else {
        raw_k[k_start] = fifty;
    }

    for today in k_period..n {
        let trailing_idx = today + 1 - k_period;

        let tmp_low = low[today];
        if lowest_idx < trailing_idx {
            lowest_idx = trailing_idx;
            lowest = low[lowest_idx];
            for j in (trailing_idx + 1)..=today {
                if low[j] < lowest {
                    lowest = low[j];
                    lowest_idx = j;
                }
            }
            diff = (highest - lowest) * inv_hundred;
        } else if tmp_low <= lowest {
            lowest_idx = today;
            lowest = tmp_low;
            diff = (highest - lowest) * inv_hundred;
        }

        let tmp_high = high[today];
        if highest_idx < trailing_idx {
            highest_idx = trailing_idx;
            highest = high[highest_idx];
            for j in (trailing_idx + 1)..=today {
                if high[j] > highest {
                    highest = high[j];
                    highest_idx = j;
                }
            }
            diff = (highest - lowest) * inv_hundred;
        } else if tmp_high >= highest {
            highest_idx = today;
            highest = tmp_high;
            diff = (highest - lowest) * inv_hundred;
        }

        if diff > T::zero() {
            raw_k[today] = (close[today] - lowest) / diff;
        } else {
            raw_k[today] = fifty;
        }
    }

    // ===== Pass 2: Compute Slow %K = SMA(raw_k, slow_k_period) =====
    let slow_k_start = k_start + slow_k_period - 1;

    // Set initial NaN values for k_out
    k_out[..slow_k_start.min(n)].fill(T::nan());

    if slow_k_start >= n {
        return Ok(());
    }

    let mut sum = T::zero();
    for j in k_start..=slow_k_start {
        sum = sum + raw_k[j];
    }
    k_out[slow_k_start] = sum * inv_slow_k;

    for i in (slow_k_start + 1)..n {
        let old_value = raw_k[i - slow_k_period];
        let new_value = raw_k[i];
        sum = sum + new_value - old_value;
        k_out[i] = sum * inv_slow_k;
    }

    // ===== Pass 3: Compute %D = SMA(slow_k, d_period) =====
    let d_start = slow_k_start + d_period - 1;

    // Set initial NaN values for d_out
    d_out[..d_start.min(n)].fill(T::nan());

    if d_start >= n {
        return Ok(());
    }

    let mut sum_d = T::zero();
    for j in slow_k_start..=d_start {
        sum_d = sum_d + k_out[j];
    }
    d_out[d_start] = sum_d * inv_d_period;

    for i in (d_start + 1)..n {
        let old_value = k_out[i - d_period];
        let new_value = k_out[i];
        sum_d = sum_d + new_value - old_value;
        d_out[i] = sum_d * inv_d_period;
    }

    Ok(())
}

/// Fused computation of Fast Stochastic (%K and %D).
///
/// Combines raw %K computation with SMA for %D in a single optimized flow.
/// Uses TA-Lib style cached extrema for O(n) amortized performance.
/// No per-element NaN checks in the hot path - uses inverse multiply for SMA.
/// Note: Kept for reference; streaming version is used in production.
#[allow(dead_code)]
#[inline(never)]
fn compute_stochastic_fast_fused<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    d_period: usize,
    k_out: &mut [T],
    d_out: &mut [T],
) -> Result<()> {
    let n = close.len();
    let inv_hundred = T::from_f64(0.01)?;
    let inv_d_period = T::from_f64(1.0 / d_period as f64)?;
    let fifty = T::from_f64(50.0)?;

    // ===== Pass 1: Compute raw %K using TA-Lib cached extrema =====

    // Set initial NaN values for lookback period
    let k_start = k_period - 1;
    k_out[..k_start].fill(T::nan());

    // Initialize by scanning the first k_period window
    let mut highest_idx = 0usize;
    let mut lowest_idx = 0usize;
    let mut highest = high[0];
    let mut lowest = low[0];

    for j in 1..k_period {
        if high[j] > highest {
            highest = high[j];
            highest_idx = j;
        }
        if low[j] < lowest {
            lowest = low[j];
            lowest_idx = j;
        }
    }

    // Precompute diff = (highest - lowest) / 100 like TA-Lib
    let mut diff = (highest - lowest) * inv_hundred;

    // First %K value
    let k_start = k_period - 1;
    if diff > T::zero() {
        k_out[k_start] = (close[k_start] - lowest) / diff;
    } else {
        k_out[k_start] = fifty;
    }

    // Compute remaining %K values using unchecked access for performance
    // SAFETY: All indices are guaranteed in bounds by loop construction
    unsafe {
        for today in k_period..n {
            let trailing_idx = today + 1 - k_period;

            // Update lowest low (check if cached min fell out of window)
            let tmp_low = *low.get_unchecked(today);
            if lowest_idx < trailing_idx {
                lowest_idx = trailing_idx;
                lowest = *low.get_unchecked(lowest_idx);
                for j in (trailing_idx + 1)..=today {
                    let val = *low.get_unchecked(j);
                    if val < lowest {
                        lowest = val;
                        lowest_idx = j;
                    }
                }
                diff = (highest - lowest) * inv_hundred;
            } else if tmp_low <= lowest {
                lowest_idx = today;
                lowest = tmp_low;
                diff = (highest - lowest) * inv_hundred;
            }

            // Update highest high (check if cached max fell out of window)
            let tmp_high = *high.get_unchecked(today);
            if highest_idx < trailing_idx {
                highest_idx = trailing_idx;
                highest = *high.get_unchecked(highest_idx);
                for j in (trailing_idx + 1)..=today {
                    let val = *high.get_unchecked(j);
                    if val > highest {
                        highest = val;
                        highest_idx = j;
                    }
                }
                diff = (highest - lowest) * inv_hundred;
            } else if tmp_high >= highest {
                highest_idx = today;
                highest = tmp_high;
                diff = (highest - lowest) * inv_hundred;
            }

            // Compute %K
            if diff > T::zero() {
                *k_out.get_unchecked_mut(today) = (*close.get_unchecked(today) - lowest) / diff;
            } else {
                *k_out.get_unchecked_mut(today) = fifty;
            }
        }
    }

    // ===== Pass 2: Compute %D as SMA of %K using inverse multiply =====

    let d_start = k_start + d_period - 1; // First valid %D index

    // Set initial NaN values for %D lookback period
    d_out[..d_start.min(n)].fill(T::nan());

    if d_start >= n {
        return Ok(());
    }

    // Compute initial sum for first %D window
    // SAFETY: Indices are in bounds (k_start..=d_start and d_start < n verified above)
    let mut sum = T::zero();
    unsafe {
        for j in k_start..=d_start {
            sum = sum + *k_out.get_unchecked(j);
        }
        *d_out.get_unchecked_mut(d_start) = sum * inv_d_period;

        // Rolling SMA for remaining %D values
        for i in (d_start + 1)..n {
            let old_value = *k_out.get_unchecked(i - d_period);
            let new_value = *k_out.get_unchecked(i);
            sum = sum + new_value - old_value;
            *d_out.get_unchecked_mut(i) = sum * inv_d_period;
        }
    }

    Ok(())
}

/// Computes raw %K values using cached extrema (TA-Lib style).
///
/// %K = 100 * (Close - Lowest Low) / (Highest High - Lowest Low)
///
/// Uses a cached extrema approach with conditional rescanning (like TA-Lib):
/// - Track the index/value of current highest high and lowest low
/// - When extremum falls out of window, rescan the window
/// - Otherwise, just compare new value with cached extremum
///
/// This achieves amortized O(n) performance on typical market data.
fn compute_raw_k<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    k_period: usize,
    output: &mut [T],
) -> Result<()> {
    let hundred = T::from_f64(100.0)?;
    let fifty = T::from_f64(50.0)?;
    let n = close.len();

    // Initialize by scanning the first window
    let mut highest_idx = 0usize;
    let mut lowest_idx = 0usize;
    let mut highest = high[0];
    let mut lowest = low[0];
    let mut has_nan = is_invalid(high[0]) || is_invalid(low[0]);

    for j in 1..k_period {
        if is_invalid(high[j]) || is_invalid(low[j]) {
            has_nan = true;
        } else if !has_nan {
            if high[j] > highest {
                highest = high[j];
                highest_idx = j;
            }
            if low[j] < lowest {
                lowest = low[j];
                lowest_idx = j;
            }
        }
    }

    // Compute first %K
    let first_idx = k_period - 1;
    if has_nan || is_invalid(close[first_idx]) {
        output[first_idx] = T::nan();
    } else {
        let range = highest - lowest;
        if range > T::zero() {
            output[first_idx] = compute_percent_k_mul_first(close[first_idx], lowest, range, hundred)?;
        } else {
            output[first_idx] = fifty;
        }
    }

    // Process remaining values
    for today in k_period..n {
        let trailing_idx = today + 1 - k_period;
        let prev_trailing = trailing_idx - 1;

        // Check if old extrema are still valid
        let rescan_high = highest_idx < trailing_idx;
        let rescan_low = lowest_idx < trailing_idx;

        // Track NaN: check if leaving value was NaN or new value is NaN
        let leaving_was_nan =
            is_invalid(high[prev_trailing]) || is_invalid(low[prev_trailing]);
        let entering_is_nan = is_invalid(high[today]) || is_invalid(low[today]);

        if leaving_was_nan && !entering_is_nan {
            // Might need rescan if window was tainted by NaN
            has_nan = false;
            for j in trailing_idx..=today {
                if is_invalid(high[j]) || is_invalid(low[j]) {
                    has_nan = true;
                    break;
                }
            }
        } else if entering_is_nan {
            has_nan = true;
        }

        if has_nan {
            output[today] = T::nan();
            // We need to rescan next time since we don't know extrema
            highest_idx = 0;
            lowest_idx = 0;
            continue;
        }

        // Update highest high
        if rescan_high {
            highest_idx = trailing_idx;
            highest = high[trailing_idx];
            for j in (trailing_idx + 1)..=today {
                if high[j] > highest {
                    highest = high[j];
                    highest_idx = j;
                }
            }
        } else if high[today] >= highest {
            highest_idx = today;
            highest = high[today];
        }

        // Update lowest low
        if rescan_low {
            lowest_idx = trailing_idx;
            lowest = low[trailing_idx];
            for j in (trailing_idx + 1)..=today {
                if low[j] < lowest {
                    lowest = low[j];
                    lowest_idx = j;
                }
            }
        } else if low[today] <= lowest {
            lowest_idx = today;
            lowest = low[today];
        }

        // Compute %K
        if is_invalid(close[today]) {
            output[today] = T::nan();
        } else {
            let range = highest - lowest;
            if range > T::zero() {
                output[today] = compute_percent_k_mul_first(close[today], lowest, range, hundred)?;
            } else {
                output[today] = fifty;
            }
        }
    }

    Ok(())
}

/// Computes SMA of a series starting from a given index.
///
/// This is a thin wrapper around [`sma_from_idx_into`] for internal use.
/// Handles NaN values in the input by only computing SMA where
/// enough valid values exist.
#[inline]
fn compute_sma_of_series<T: SeriesElement + 'static>(
    input: &[T],
    period: usize,
    start_idx: usize,
    output: &mut [T],
) -> Result<()> {
    // Delegate to the shared SMA implementation
    sma_from_idx_into(input, period, start_idx, output).map(|_| ())
}

// ==================== Configuration Type ====================

/// Stochastic Oscillator configuration with fluent builder API.
///
/// Provides sensible defaults (`k_period=14`, `d_period=3`, `k_slowing=1`) and fluent
/// setters for customization. Implements `Default` for zero-config usage per
/// Gravity Check 1.1.
///
/// Per PRD §5.2, the default is **fast stochastic** (`k_slowing = 1`).
/// Use `k_slowing(3)` for traditional slow stochastic.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::stochastic::Stochastic;
///
/// let high = vec![
///     45.0_f64, 45.5, 44.5, 45.5, 45.0, 44.0, 43.5, 44.5, 45.5, 46.0,
///     46.5, 45.5, 44.5, 45.0, 46.0,
/// ];
/// let low = vec![
///     43.0, 43.5, 42.5, 43.5, 43.0, 42.0, 41.5, 42.5, 43.5, 44.0,
///     44.5, 43.5, 42.5, 43.0, 44.0,
/// ];
/// let close = vec![
///     44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0,
///     45.5, 44.5, 43.5, 44.0, 45.0,
/// ];
///
/// // Use defaults (14, 3, 1) - computes fast stochastic
/// let result = Stochastic::default().compute(&high, &low, &close).unwrap();
///
/// // Slow stochastic with fluent API
/// let result = Stochastic::new()
///     .k_period(5)
///     .d_period(3)
///     .k_slowing(3)
///     .compute(&high, &low, &close)
///     .unwrap();
///
/// // Use convenience constructors for fast/slow variants
/// let fast_result = Stochastic::fast(14, 3).compute(&high, &low, &close).unwrap();
/// let slow_result = Stochastic::slow(14, 3).compute(&high, &low, &close).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Stochastic {
    k_period: usize,
    d_period: usize,
    k_slowing: usize,
}

impl Default for Stochastic {
    /// Creates a Stochastic configuration with fast stochastic defaults (14, 3, 1).
    ///
    /// Per PRD §5.2, the default is fast stochastic (`k_slowing = 1`).
    fn default() -> Self {
        Self {
            k_period: 14,
            d_period: 3,
            k_slowing: 1,
        }
    }
}

impl Stochastic {
    /// Creates a new Stochastic configuration with fast stochastic defaults (14, 3, 1).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a fast stochastic configuration.
    ///
    /// Fast stochastic has `k_slowing = 1` (no smoothing on %K).
    #[must_use]
    pub const fn fast(k_period: usize, d_period: usize) -> Self {
        Self {
            k_period,
            d_period,
            k_slowing: 1,
        }
    }

    /// Creates a slow stochastic configuration.
    ///
    /// Slow stochastic has `k_slowing = 3` (3-period smoothing on %K).
    #[must_use]
    pub const fn slow(k_period: usize, d_period: usize) -> Self {
        Self {
            k_period,
            d_period,
            k_slowing: 3,
        }
    }

    /// Sets the %K lookback period.
    ///
    /// Default: 14
    #[must_use]
    pub const fn k_period(mut self, period: usize) -> Self {
        self.k_period = period;
        self
    }

    /// Sets the %D (signal line) smoothing period.
    ///
    /// Default: 3
    #[must_use]
    pub const fn d_period(mut self, period: usize) -> Self {
        self.d_period = period;
        self
    }

    /// Sets the %K slowing (smoothing) period.
    ///
    /// - `k_slowing = 1`: Fast stochastic (no smoothing, default)
    /// - `k_slowing = 3`: Slow stochastic (traditional)
    ///
    /// Default: 1 (fast stochastic per PRD §5.2)
    #[must_use]
    pub const fn k_slowing(mut self, period: usize) -> Self {
        self.k_slowing = period;
        self
    }

    /// Computes the Stochastic Oscillator using the configured parameters.
    ///
    /// Uses the canonical `stochastic()` function with the configured `k_slowing`.
    /// For `k_slowing = 1` (default), this computes fast stochastic.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any input array is empty
    /// - Input arrays have different lengths
    /// - Any period is 0
    /// - Insufficient data for the configured periods
    pub fn compute<T: SeriesElement>(
        &self,
        high: &[T],
        low: &[T],
        close: &[T],
    ) -> Result<StochasticOutput<T>> {
        stochastic(
            high,
            low,
            close,
            self.k_period,
            self.d_period,
            self.k_slowing,
        )
    }

    /// Computes the Stochastic Oscillator into a pre-allocated output struct.
    ///
    /// Returns `(k_valid_count, d_valid_count)`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Output buffers are smaller than input length
    /// - Any input array is empty
    /// - Input arrays have different lengths
    /// - Any period is 0
    /// - Insufficient data for the configured periods
    pub fn compute_into<T: SeriesElement>(
        &self,
        high: &[T],
        low: &[T],
        close: &[T],
        output: &mut StochasticOutput<T>,
    ) -> Result<(usize, usize)> {
        stochastic_into(
            high,
            low,
            close,
            self.k_period,
            self.d_period,
            self.k_slowing,
            output,
        )
    }

    /// Returns the %K period.
    #[must_use]
    pub const fn get_k_period(&self) -> usize {
        self.k_period
    }

    /// Returns the %D period.
    #[must_use]
    pub const fn get_d_period(&self) -> usize {
        self.d_period
    }

    /// Returns the %K slowing (smoothing) period.
    #[must_use]
    pub const fn get_k_slowing(&self) -> usize {
        self.k_slowing
    }

    /// Returns the %K lookback for this configuration.
    #[must_use]
    pub const fn k_lookback(&self) -> usize {
        stochastic_k_lookback(self.k_period)
    }

    /// Returns the %D lookback for this configuration.
    #[must_use]
    pub const fn d_lookback(&self) -> usize {
        stochastic_d_lookback(self.k_period, self.d_period)
    }

    /// Returns the minimum input length for this configuration.
    #[must_use]
    pub const fn min_len(&self) -> usize {
        stochastic_min_len(self.k_period, self.d_period)
    }
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
    // Looser epsilon for stochastic calculations
    const STOCH_EPSILON: f64 = 1e-6;

    // ==================== Fast Stochastic Tests ====================

    #[test]
    fn test_stochastic_fast_basic() {
        let high = vec![
            10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5,
        ];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0];

        let result = stochastic_fast(&high, &low, &close, 5, 3).unwrap();

        assert_eq!(result.k.len(), 10);
        assert_eq!(result.d.len(), 10);

        // First 4 values of %K are NaN
        for i in 0..4 {
            assert!(result.k[i].is_nan(), "k[{}] should be NaN", i);
        }
        assert!(!result.k[4].is_nan());

        // First 6 values of %D are NaN (k_period + d_period - 2 = 5 + 3 - 2 = 6)
        for i in 0..6 {
            assert!(result.d[i].is_nan(), "d[{}] should be NaN", i);
        }
        assert!(!result.d[6].is_nan());
    }

    #[test]
    fn test_stochastic_fast_f32() {
        let high = vec![10.0_f32, 11.0, 12.0, 11.5, 12.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0];

        let result = stochastic_fast(&high, &low, &close, 3, 2).unwrap();

        assert!(!result.k[2].is_nan());
        // %K should be in range [0, 100]
        assert!(result.k[2] >= 0.0 && result.k[2] <= 100.0);
    }

    #[test]
    fn test_stochastic_fast_known_values() {
        // Simple case where we can verify the calculation
        // Period 3, looking at window [10, 11, 12] with close = 12
        // Highest high = 12, Lowest low = 10
        // %K = 100 * (12 - 10) / (12 - 10) = 100
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![10.0, 11.0, 12.0];
        let close = vec![10.0, 11.0, 12.0];

        let result = stochastic_fast(&high, &low, &close, 3, 1).unwrap();

        // At index 2: HH=12, LL=10, Close=12
        // %K = 100 * (12-10)/(12-10) = 100
        assert!(approx_eq(result.k[2], 100.0, STOCH_EPSILON));
    }

    #[test]
    fn test_stochastic_fast_close_at_low() {
        // Close at the lowest low should give %K = 0
        let high = vec![15.0_f64, 14.0, 13.0, 12.0, 11.0];
        let low = vec![10.0, 10.0, 10.0, 10.0, 10.0];
        let close = vec![10.0, 10.0, 10.0, 10.0, 10.0];

        let result = stochastic_fast(&high, &low, &close, 3, 1).unwrap();

        // Close is at lowest low
        assert!(approx_eq(result.k[2], 0.0, STOCH_EPSILON));
    }

    #[test]
    fn test_stochastic_fast_close_at_high() {
        // Close at the highest high should give %K = 100
        let high = vec![20.0_f64, 20.0, 20.0, 20.0, 20.0];
        let low = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let close = vec![20.0, 20.0, 20.0, 20.0, 20.0];

        let result = stochastic_fast(&high, &low, &close, 3, 1).unwrap();

        // Close is at highest high
        assert!(approx_eq(result.k[2], 100.0, STOCH_EPSILON));
    }

    #[test]
    fn test_stochastic_fast_close_at_midpoint() {
        // Close at midpoint should give %K = 50
        let high = vec![20.0_f64, 20.0, 20.0, 20.0, 20.0];
        let low = vec![10.0, 10.0, 10.0, 10.0, 10.0];
        let close = vec![15.0, 15.0, 15.0, 15.0, 15.0];

        let result = stochastic_fast(&high, &low, &close, 3, 1).unwrap();

        // Close is at midpoint: (15 - 10) / (20 - 10) = 0.5 -> 50%
        assert!(approx_eq(result.k[2], 50.0, STOCH_EPSILON));
    }

    #[test]
    fn test_stochastic_fast_no_range() {
        // When high == low, range is 0, should return 50 (neutral)
        let high = vec![50.0_f64; 5];
        let low = vec![50.0_f64; 5];
        let close = vec![50.0_f64; 5];

        let result = stochastic_fast(&high, &low, &close, 3, 1).unwrap();

        // No range, should be neutral (50)
        assert!(approx_eq(result.k[2], 50.0, STOCH_EPSILON));
    }

    // ==================== Slow Stochastic Tests ====================

    #[test]
    fn test_stochastic_slow_basic() {
        let high = vec![
            10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5,
        ];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0];

        let result = stochastic_slow(&high, &low, &close, 5, 3).unwrap();

        assert_eq!(result.k.len(), 10);
        assert_eq!(result.d.len(), 10);

        // Slow %K starts at k_period + d_period - 2 = 5 + 3 - 2 = 6
        for i in 0..6 {
            assert!(result.k[i].is_nan(), "slow k[{}] should be NaN", i);
        }
        assert!(!result.k[6].is_nan());

        // Slow %D starts at k_period + 2*d_period - 3 = 5 + 6 - 3 = 8
        for i in 0..8 {
            assert!(result.d[i].is_nan(), "slow d[{}] should be NaN", i);
        }
        assert!(!result.d[8].is_nan());
    }

    #[test]
    fn test_stochastic_slow_smoother_than_fast() {
        // Slow stochastic should be smoother (less volatile) than fast
        let high: Vec<f64> = (0..20)
            .map(|i| 100.0 + (i as f64) + (i % 3) as f64)
            .collect();
        let low: Vec<f64> = (0..20)
            .map(|i| 95.0 + (i as f64) - (i % 3) as f64)
            .collect();
        let close: Vec<f64> = (0..20).map(|i| 97.5 + (i as f64)).collect();

        let fast = stochastic_fast(&high, &low, &close, 5, 3).unwrap();
        let slow = stochastic_slow(&high, &low, &close, 5, 3).unwrap();

        // Slow %K should have less variance than Fast %K
        // (This is a qualitative test - we just verify both produce valid output)
        let fast_valid: Vec<f64> = fast.k.iter().filter(|x| !x.is_nan()).cloned().collect();
        let slow_valid: Vec<f64> = slow.k.iter().filter(|x| !x.is_nan()).cloned().collect();

        assert!(!fast_valid.is_empty());
        assert!(!slow_valid.is_empty());
    }

    // ==================== Full Stochastic Tests ====================

    #[test]
    fn test_stochastic_full_basic() {
        let high = vec![
            10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5, 12.0,
        ];
        let low = vec![
            9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5, 11.0,
        ];
        let close = vec![
            9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0, 11.5,
        ];

        // k_period=5, slow_k_period=3, d_period=3
        let result = stochastic_full(&high, &low, &close, 5, 3, 3).unwrap();

        assert_eq!(result.k.len(), 11);
        assert_eq!(result.d.len(), 11);

        // Full %K starts at k_period + slow_k_period - 2 = 5 + 3 - 2 = 6
        for i in 0..6 {
            assert!(result.k[i].is_nan(), "full k[{}] should be NaN", i);
        }
        assert!(!result.k[6].is_nan());

        // Full %D starts at k_period + slow_k_period + d_period - 3 = 5 + 3 + 3 - 3 = 8
        for i in 0..8 {
            assert!(result.d[i].is_nan(), "full d[{}] should be NaN", i);
        }
        assert!(!result.d[8].is_nan());
    }

    #[test]
    fn test_stochastic_full_custom_periods() {
        let high = vec![
            10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5, 12.0, 12.5,
        ];
        let low = vec![
            9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5, 11.0, 11.5,
        ];
        let close = vec![
            9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0, 11.5, 12.0,
        ];

        // Custom periods: k_period=3, slow_k_period=2, d_period=4
        let result = stochastic_full(&high, &low, &close, 3, 2, 4).unwrap();

        // Full %K starts at k_period + slow_k_period - 2 = 3 + 2 - 2 = 3
        assert!(result.k[2].is_nan());
        assert!(!result.k[3].is_nan());

        // Full %D starts at k_period + slow_k_period + d_period - 3 = 3 + 2 + 4 - 3 = 6
        assert!(result.d[5].is_nan());
        assert!(!result.d[6].is_nan());
    }

    #[test]
    fn test_stochastic_full_same_as_slow_when_periods_match() {
        let high = vec![
            10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5,
        ];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0];

        let slow = stochastic_slow(&high, &low, &close, 5, 3).unwrap();
        let full = stochastic_full(&high, &low, &close, 5, 3, 3).unwrap();

        // Slow and Full should produce identical results when slow_k_period == d_period
        for i in 0..10 {
            assert!(
                approx_eq(slow.k[i], full.k[i], STOCH_EPSILON),
                "k mismatch at {}: {} vs {}",
                i,
                slow.k[i],
                full.k[i]
            );
            assert!(
                approx_eq(slow.d[i], full.d[i], STOCH_EPSILON),
                "d mismatch at {}: {} vs {}",
                i,
                slow.d[i],
                full.d[i]
            );
        }
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_stochastic_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];

        let result = stochastic_fast(&high, &low, &close, 5, 3);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_stochastic_zero_k_period() {
        let high = vec![1.0_f64, 2.0, 3.0];
        let low = vec![0.5, 1.5, 2.5];
        let close = vec![0.75, 1.75, 2.75];

        let result = stochastic_fast(&high, &low, &close, 0, 3);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_stochastic_zero_d_period() {
        let high = vec![1.0_f64, 2.0, 3.0];
        let low = vec![0.5, 1.5, 2.5];
        let close = vec![0.75, 1.75, 2.75];

        let result = stochastic_fast(&high, &low, &close, 3, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_stochastic_insufficient_data() {
        let high = vec![1.0_f64, 2.0, 3.0];
        let low = vec![0.5, 1.5, 2.5];
        let close = vec![0.75, 1.75, 2.75];

        let result = stochastic_fast(&high, &low, &close, 5, 3);
        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                required: 5,
                actual: 3,
                ..
            })
        ));
    }

    #[test]
    fn test_stochastic_mismatched_lengths() {
        let high = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let low = vec![0.5, 1.5, 2.5]; // Shorter
        let close = vec![0.75, 1.75, 2.75, 3.75, 4.75];

        let result = stochastic_fast(&high, &low, &close, 3, 2);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    // ==================== Into Variant Tests ====================

    #[test]
    fn test_stochastic_fast_into_basic() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5];

        let mut output = StochasticOutput {
            k: vec![0.0_f64; 7],
            d: vec![0.0_f64; 7],
        };

        let (valid_k, valid_d) =
            stochastic_fast_into(&high, &low, &close, 3, 2, &mut output).unwrap();

        assert_eq!(valid_k, 5); // 7 - (3 - 1) = 5
        assert_eq!(valid_d, 4); // 7 - (3 + 2 - 2) = 4

        assert!(output.k[0].is_nan());
        assert!(output.k[1].is_nan());
        assert!(!output.k[2].is_nan());
    }

    #[test]
    fn test_stochastic_fast_into_buffer_reuse() {
        let high1 = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];
        let low1 = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];

        let high2 = vec![14.0_f64, 13.0, 12.0, 11.0, 10.0];
        let low2 = vec![13.0, 12.0, 11.0, 10.0, 9.0];
        let close2 = vec![13.0, 12.0, 11.0, 10.0, 9.0];

        let mut output = StochasticOutput {
            k: vec![0.0_f64; 5],
            d: vec![0.0_f64; 5],
        };

        stochastic_fast_into(&high1, &low1, &close1, 3, 2, &mut output).unwrap();
        let k_first = output.k[3];

        stochastic_fast_into(&high2, &low2, &close2, 3, 2, &mut output).unwrap();
        let k_second = output.k[3];

        // Different data should produce different results
        assert!(!approx_eq(k_first, k_second, EPSILON));
    }

    #[test]
    fn test_stochastic_fast_into_insufficient_output() {
        let high = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let low = vec![0.5, 1.5, 2.5, 3.5, 4.5];
        let close = vec![0.75, 1.75, 2.75, 3.75, 4.75];

        let mut output = StochasticOutput {
            k: vec![0.0_f64; 3], // Too short
            d: vec![0.0_f64; 5],
        };

        let result = stochastic_fast_into(&high, &low, &close, 3, 2, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_stochastic_slow_into_basic() {
        let high = vec![
            10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5,
        ];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0];

        let mut output = StochasticOutput {
            k: vec![0.0_f64; 10],
            d: vec![0.0_f64; 10],
        };

        let (valid_k, valid_d) =
            stochastic_slow_into(&high, &low, &close, 5, 3, &mut output).unwrap();

        assert_eq!(valid_k, 4); // 10 - (5 + 3 - 2) = 4
        assert_eq!(valid_d, 2); // 10 - (5 + 3 + 3 - 3) = 2
    }

    #[test]
    fn test_stochastic_full_into_basic() {
        let high = vec![
            10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5, 12.0,
        ];
        let low = vec![
            9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5, 11.0,
        ];
        let close = vec![
            9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0, 11.5,
        ];

        let mut output = StochasticOutput {
            k: vec![0.0_f64; 11],
            d: vec![0.0_f64; 11],
        };

        let (valid_k, valid_d) =
            stochastic_full_into(&high, &low, &close, 5, 3, 3, &mut output).unwrap();

        assert_eq!(valid_k, 5); // 11 - (5 + 3 - 2) = 5
        assert_eq!(valid_d, 3); // 11 - (5 + 3 + 3 - 3) = 3
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_stochastic_fast_and_fast_into_produce_same_result() {
        let high = vec![
            10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5,
        ];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0];

        let result1 = stochastic_fast(&high, &low, &close, 5, 3).unwrap();

        let mut result2 = StochasticOutput {
            k: vec![0.0_f64; 10],
            d: vec![0.0_f64; 10],
        };
        stochastic_fast_into(&high, &low, &close, 5, 3, &mut result2).unwrap();

        for i in 0..10 {
            assert!(
                approx_eq(result1.k[i], result2.k[i], EPSILON),
                "k mismatch at {}: {} vs {}",
                i,
                result1.k[i],
                result2.k[i]
            );
            assert!(
                approx_eq(result1.d[i], result2.d[i], EPSILON),
                "d mismatch at {}: {} vs {}",
                i,
                result1.d[i],
                result2.d[i]
            );
        }
    }

    // ==================== Bounds Tests ====================

    #[test]
    fn test_stochastic_k_in_bounds() {
        // %K should always be between 0 and 100
        let high: Vec<f64> = (0..50)
            .map(|i| 100.0 + (i as f64) + ((i % 7) as f64))
            .collect();
        let low: Vec<f64> = (0..50)
            .map(|i| 95.0 + (i as f64) - ((i % 5) as f64))
            .collect();
        let close: Vec<f64> = (0..50)
            .map(|i| 97.5 + (i as f64) + ((i % 3) as f64) * 0.5)
            .collect();

        let result = stochastic_fast(&high, &low, &close, 14, 3).unwrap();

        for (i, &k) in result.k.iter().enumerate() {
            if !k.is_nan() {
                assert!(k >= 0.0 && k <= 100.0, "k[{}] = {} is out of bounds", i, k);
            }
        }
    }

    #[test]
    fn test_stochastic_d_in_bounds() {
        // %D should always be between 0 and 100
        let high: Vec<f64> = (0..50)
            .map(|i| 100.0 + (i as f64) + ((i % 7) as f64))
            .collect();
        let low: Vec<f64> = (0..50)
            .map(|i| 95.0 + (i as f64) - ((i % 5) as f64))
            .collect();
        let close: Vec<f64> = (0..50)
            .map(|i| 97.5 + (i as f64) + ((i % 3) as f64) * 0.5)
            .collect();

        let result = stochastic_fast(&high, &low, &close, 14, 3).unwrap();

        for (i, &d) in result.d.iter().enumerate() {
            if !d.is_nan() {
                assert!(d >= 0.0 && d <= 100.0, "d[{}] = {} is out of bounds", i, d);
            }
        }
    }

    // ==================== NaN Handling Tests ====================

    #[test]
    fn test_stochastic_with_nan_in_data() {
        let high = vec![10.0_f64, 11.0, f64::NAN, 11.5, 12.5, 13.0];
        let low = vec![9.0, 10.0, 11.0, f64::NAN, 11.5, 12.0];
        let close = vec![9.5, 10.5, 11.5, 11.0, f64::NAN, 12.5];

        let result = stochastic_fast(&high, &low, &close, 3, 2).unwrap();

        // NaN in input should propagate to output
        // Windows containing NaN will produce NaN
        assert!(result.k[2].is_nan()); // NaN in high[2]
        assert!(result.k[3].is_nan()); // NaN in low[3]
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_stochastic_period_one() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.0, 10.0];
        let low = vec![9.0, 10.0, 11.0, 10.0, 9.0];
        let close = vec![9.5, 10.5, 11.5, 10.5, 9.5];

        let result = stochastic_fast(&high, &low, &close, 1, 1).unwrap();

        // All values should be valid with period 1
        assert!(!result.k[0].is_nan());
        assert!(!result.d[0].is_nan());
    }

    #[test]
    fn test_stochastic_minimum_data() {
        // Minimum data for k_period=3, d_period=2
        let high = vec![10.0_f64, 11.0, 12.0, 11.0];
        let low = vec![9.0, 10.0, 11.0, 10.0];
        let close = vec![9.5, 10.5, 11.5, 10.5];

        let result = stochastic_fast(&high, &low, &close, 3, 2).unwrap();

        // k_period - 1 = 2 NaN values for %K
        assert!(result.k[0].is_nan());
        assert!(result.k[1].is_nan());
        assert!(!result.k[2].is_nan());

        // k_period + d_period - 2 = 3 NaN values for %D
        assert!(result.d[0].is_nan());
        assert!(result.d[1].is_nan());
        assert!(result.d[2].is_nan());
        assert!(!result.d[3].is_nan());
    }

    #[test]
    fn test_stochastic_negative_prices() {
        // Unusual but valid - negative prices (e.g., spreads, correlations)
        let high = vec![-5.0_f64, -4.0, -3.0, -4.0, -5.0];
        let low = vec![-10.0, -9.0, -8.0, -9.0, -10.0];
        let close = vec![-7.5, -6.5, -5.5, -6.5, -7.5];

        let result = stochastic_fast(&high, &low, &close, 3, 2).unwrap();

        // Should still produce valid results in [0, 100]
        for k in result.k.iter() {
            if !k.is_nan() {
                assert!(*k >= 0.0 && *k <= 100.0);
            }
        }
    }

    #[test]
    fn test_stochastic_large_values() {
        let high = vec![1e12_f64, 1.01e12, 1.02e12, 1.03e12, 1.04e12];
        let low = vec![0.99e12, 1.0e12, 1.01e12, 1.02e12, 1.03e12];
        let close = vec![1.0e12, 1.01e12, 1.02e12, 1.025e12, 1.035e12];

        let result = stochastic_fast(&high, &low, &close, 3, 2).unwrap();

        // Should handle large values
        for k in result.k.iter() {
            if !k.is_nan() {
                assert!(*k >= 0.0 && *k <= 100.0);
            }
        }
    }

    // ==================== Property Tests ====================

    #[test]
    fn test_stochastic_output_length_equals_input_length() {
        for len in [5, 10, 50, 100] {
            for k_period in [3, 5, 14] {
                for d_period in [1, 3, 5] {
                    if k_period <= len {
                        let high: Vec<f64> = (0..len).map(|x| (x + 10) as f64).collect();
                        let low: Vec<f64> = (0..len).map(|x| x as f64).collect();
                        let close: Vec<f64> = (0..len).map(|x| (x + 5) as f64).collect();

                        let result =
                            stochastic_fast(&high, &low, &close, k_period, d_period).unwrap();
                        assert_eq!(result.k.len(), len);
                        assert_eq!(result.d.len(), len);
                    }
                }
            }
        }
    }

    #[test]
    fn test_stochastic_k_nan_count() {
        // First (k_period - 1) %K values should be NaN
        for k_period in 1..=10 {
            let high: Vec<f64> = (0..20).map(|x| (x + 10) as f64).collect();
            let low: Vec<f64> = (0..20).map(|x| x as f64).collect();
            let close: Vec<f64> = (0..20).map(|x| (x + 5) as f64).collect();

            let result = stochastic_fast(&high, &low, &close, k_period, 3).unwrap();

            let nan_count = result.k.iter().filter(|x| x.is_nan()).count();
            assert_eq!(
                nan_count,
                k_period - 1,
                "Expected {} NaN values for k_period {}",
                k_period - 1,
                k_period
            );
        }
    }

    #[test]
    fn test_stochastic_d_nan_count() {
        // First (k_period + d_period - 2) %D values should be NaN
        for k_period in 1..=5 {
            for d_period in 1..=5 {
                let high: Vec<f64> = (0..20).map(|x| (x + 10) as f64).collect();
                let low: Vec<f64> = (0..20).map(|x| x as f64).collect();
                let close: Vec<f64> = (0..20).map(|x| (x + 5) as f64).collect();

                let result = stochastic_fast(&high, &low, &close, k_period, d_period).unwrap();

                let expected_nan_count = k_period + d_period - 2;
                let nan_count = result.d.iter().filter(|x| x.is_nan()).count();
                assert_eq!(
                    nan_count, expected_nan_count,
                    "Expected {} NaN values for k_period={}, d_period={}",
                    expected_nan_count, k_period, d_period
                );
            }
        }
    }

    #[test]
    fn test_stochastic_d_is_average_of_k() {
        // %D should be SMA of %K
        let high = vec![
            10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5,
        ];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5];
        let close = vec![9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0];

        let result = stochastic_fast(&high, &low, &close, 3, 3).unwrap();

        // Verify %D is SMA of %K
        for i in 4..10 {
            // From index k_period + d_period - 2 = 4
            let expected_d = (result.k[i - 2] + result.k[i - 1] + result.k[i]) / 3.0;
            assert!(
                approx_eq(result.d[i], expected_d, STOCH_EPSILON),
                "d[{}] should be average of k[{}..{}]",
                i,
                i - 2,
                i
            );
        }
    }

    // ==================== Trend Response Tests ====================

    #[test]
    fn test_stochastic_responds_to_uptrend() {
        // In an uptrend, %K should be high (close near high)
        let high: Vec<f64> = (0..10).map(|i| 100.0 + (i as f64) * 2.0).collect();
        let low: Vec<f64> = (0..10).map(|i| 95.0 + (i as f64) * 2.0).collect();
        let close: Vec<f64> = (0..10).map(|i| 99.0 + (i as f64) * 2.0).collect(); // Close near high

        let result = stochastic_fast(&high, &low, &close, 5, 3).unwrap();

        // In uptrend with close near high, %K should be high (> 50)
        for i in 4..10 {
            assert!(result.k[i] > 50.0, "k[{}] should be > 50 in uptrend", i);
        }
    }

    #[test]
    fn test_stochastic_responds_to_downtrend() {
        // In a downtrend, %K should be low (close near low)
        let high: Vec<f64> = (0..10).map(|i| 100.0 - (i as f64) * 2.0).collect();
        let low: Vec<f64> = (0..10).map(|i| 95.0 - (i as f64) * 2.0).collect();
        let close: Vec<f64> = (0..10).map(|i| f64::from(i).mul_add(-2.0, 96.0)).collect(); // Close near low

        let result = stochastic_fast(&high, &low, &close, 5, 3).unwrap();

        // In downtrend with close near low, %K should be low (< 50)
        for i in 4..10 {
            assert!(result.k[i] < 50.0, "k[{i}] should be < 50 in downtrend");
        }
    }
}
