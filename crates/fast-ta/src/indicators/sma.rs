//! Simple Moving Average (SMA) indicator.
//!
//! The Simple Moving Average is a trend-following indicator that smooths price data
//! by creating a constantly updated average price. The SMA calculates the arithmetic
//! mean of a given set of values over a specified period.
//!
//! # Algorithm
//!
//! This implementation uses an O(n) rolling sum approach where:
//! 1. Initial sum is computed for the first `period` elements
//! 2. For each subsequent element, we add the new value and subtract the oldest value
//! 3. This maintains the rolling sum with O(1) operations per element
//!
//! # Performance Optimizations
//!
//! - **SIMD fast path**: When all data is finite (no NaN/Inf), uses SIMD-accelerated
//!   rolling sum without per-element validity checks
//! - **SIMD initial window**: Uses vectorized sum for the first window computation
//! - **Pre-computed reciprocal**: Multiplies by 1/period instead of dividing
//!
//! # Formula
//!
//! ```text
//! SMA = (P1 + P2 + ... + Pn) / n
//! ```
//!
//! Where `P` is the price and `n` is the period.
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::sma::sma;
//!
//! let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
//! let result = sma(&data, 3).unwrap();
//!
//! // First 2 values are NaN (period-1 lookback)
//! assert!(result[0].is_nan());
//! assert!(result[1].is_nan());
//!
//! // SMA starts from index 2 (period - 1)
//! assert!((result[2] - 2.0).abs() < 1e-10); // (1+2+3)/3 = 2.0
//! assert!((result[3] - 3.0).abs() < 1e-10); // (2+3+4)/3 = 3.0
//! assert!((result[4] - 4.0).abs() < 1e-10); // (3+4+5)/3 = 4.0
//! ```

use crate::error::{Error, Result};
use crate::kernels::simd;
use crate::traits::SeriesElement;
use std::any::TypeId;
use std::simd::{f64x4, num::SimdFloat};

/// Number of f64 lanes for SIMD operations.
const LANES: usize = 4;

/// Prefix-sum based SIMD SMA for f64 when all data is finite.
///
/// This approach has two phases:
/// 1. Compute prefix sums (sequential, O(n))
/// 2. Compute differences with SIMD (parallel, O(n/LANES))
///
/// The difference phase is embarrassingly parallel - no loop-carried
/// dependencies - allowing full SIMD utilization.
#[inline]
fn sma_f64_prefix_simd(data: &[f64], period: usize, output: &mut [f64]) {
    let n = data.len();
    let inv_period = 1.0 / period as f64;

    // Fill lookback with NaN
    for item in output.iter_mut().take(period - 1) {
        *item = f64::NAN;
    }

    // Build prefix sums: p[i] = sum of data[0..i]
    // p[0] = 0, p[1] = data[0], p[2] = data[0] + data[1], ...
    let mut prefix = vec![0.0f64; n + 1];
    for i in 0..n {
        prefix[i + 1] = prefix[i] + data[i];
    }

    // SIMD vectorized difference computation
    // SMA[i] = (prefix[i+1] - prefix[i+1-period]) / period
    // Output index: period-1 + j maps to prefix indices (period + j) - (j) = period apart
    let m = n - period + 1; // number of valid outputs
    let inv_vec = f64x4::splat(inv_period);

    let mut j = 0;
    while j + LANES <= m {
        let out_idx = period - 1 + j;
        let end_idx = period + j; // prefix[end_idx] = sum of data[0..end_idx]
        let start_idx = j; // prefix[start_idx] = sum of data[0..start_idx]

        let end_sums = f64x4::from_slice(&prefix[end_idx..end_idx + LANES]);
        let start_sums = f64x4::from_slice(&prefix[start_idx..start_idx + LANES]);
        let sma_vals = (end_sums - start_sums) * inv_vec;

        sma_vals.copy_to_slice(&mut output[out_idx..out_idx + LANES]);
        j += LANES;
    }

    // Scalar tail
    while j < m {
        let out_idx = period - 1 + j;
        output[out_idx] = (prefix[period + j] - prefix[j]) * inv_period;
        j += 1;
    }
}

/// Optimized f64 SMA kernel with unchecked rolling sum (no validation).
/// Fast path for clean data - single pass, minimal memory traffic.
#[inline]
fn sma_f64_unchecked(data: &[f64], period: usize, output: &mut [f64]) {
    let n = data.len();
    let inv_period = 1.0 / period as f64;

    // Fill lookback with NaN
    output[..period - 1].fill(f64::NAN);

    // Compute initial sum
    let mut sum = 0.0;
    for i in 0..period {
        sum += unsafe { *data.get_unchecked(i) };
    }

    // First output
    unsafe { *output.get_unchecked_mut(period - 1) = sum * inv_period; }

    // Rolling sum: no checks, just subtract old + add new
    for i in period..n {
        sum += unsafe { *data.get_unchecked(i) - *data.get_unchecked(i - period) };
        unsafe { *output.get_unchecked_mut(i) = sum * inv_period; }
    }
}

/// Optimized f32 SMA kernel with unchecked rolling sum (no validation).
/// Uses f64 accumulator for better accuracy.
#[inline]
fn sma_f32_unchecked(data: &[f32], period: usize, output: &mut [f32]) {
    let n = data.len();
    let inv_period = 1.0 / period as f64;

    // Fill lookback with NaN
    output[..period - 1].fill(f32::NAN);

    // Compute initial sum (use f64 for accuracy)
    let mut sum = 0.0f64;
    for i in 0..period {
        sum += unsafe { *data.get_unchecked(i) as f64 };
    }

    // First output
    unsafe { *output.get_unchecked_mut(period - 1) = (sum * inv_period) as f32; }

    // Rolling sum
    for i in period..n {
        sum += unsafe { *data.get_unchecked(i) as f64 - *data.get_unchecked(i - period) as f64 };
        unsafe { *output.get_unchecked_mut(i) = (sum * inv_period) as f32; }
    }
}

/// f64 SMA with ring buffer for invalid tracking (1 finiteness check per step).
#[inline]
fn sma_f64_with_tracking(data: &[f64], period: usize, output: &mut [f64]) {
    let n = data.len();
    let inv_period = 1.0 / period as f64;

    // Fill lookback
    output[..period - 1].fill(f64::NAN);

    // Ring buffers for sanitized values and invalid flags
    let mut buf = vec![0.0f64; period];
    let mut inv = vec![0u8; period];

    // Initialize first window
    let mut sum = 0.0;
    let mut invalid_count = 0usize;

    for i in 0..period {
        let val = data[i];
        let is_invalid = !val.is_finite();

        buf[i] = if is_invalid { 0.0 } else { val };
        inv[i] = is_invalid as u8;

        sum += buf[i];
        invalid_count += inv[i] as usize;
    }

    // First output
    if invalid_count == 0 {
        output[period - 1] = sum * inv_period;
    } else {
        output[period - 1] = f64::NAN;
    }

    // Rolling window with ring buffer
    for i in period..n {
        let idx = i % period;

        // Evict old value
        sum -= buf[idx];
        invalid_count -= inv[idx] as usize;

        // Insert new value (single finiteness check)
        let val = data[i];
        let is_invalid = !val.is_finite();

        buf[idx] = if is_invalid { 0.0 } else { val };
        inv[idx] = is_invalid as u8;

        sum += buf[idx];
        invalid_count += inv[idx] as usize;

        // Output
        if invalid_count == 0 {
            output[i] = sum * inv_period;
        } else {
            output[i] = f64::NAN;
        }
    }
}

/// f32 SMA with ring buffer for invalid tracking.
#[inline]
fn sma_f32_with_tracking(data: &[f32], period: usize, output: &mut [f32]) {
    let n = data.len();
    let inv_period = 1.0 / period as f64;

    output[..period - 1].fill(f32::NAN);

    let mut buf = vec![0.0f64; period];
    let mut inv = vec![0u8; period];

    let mut sum = 0.0f64;
    let mut invalid_count = 0usize;

    for i in 0..period {
        let val = data[i];
        let is_invalid = !val.is_finite();

        buf[i] = if is_invalid { 0.0 } else { val as f64 };
        inv[i] = is_invalid as u8;

        sum += buf[i];
        invalid_count += inv[i] as usize;
    }

    if invalid_count == 0 {
        output[period - 1] = (sum * inv_period) as f32;
    } else {
        output[period - 1] = f32::NAN;
    }

    for i in period..n {
        let idx = i % period;

        sum -= buf[idx];
        invalid_count -= inv[idx] as usize;

        let val = data[i];
        let is_invalid = !val.is_finite();

        buf[idx] = if is_invalid { 0.0 } else { val as f64 };
        inv[idx] = is_invalid as u8;

        sum += buf[idx];
        invalid_count += inv[idx] as usize;

        if invalid_count == 0 {
            output[i] = (sum * inv_period) as f32;
        } else {
            output[i] = f32::NAN;
        }
    }
}

/// f64 SMA with optimistic fast path that bails to tracking on first invalid.
#[inline]
fn sma_f64_optimistic(data: &[f64], period: usize, output: &mut [f64]) {
    let n = data.len();
    let inv_period = 1.0 / period as f64;

    output[..period - 1].fill(f64::NAN);

    // Compute initial sum - check for invalids
    let mut sum = 0.0;
    let mut has_invalid = false;

    for i in 0..period {
        let val = unsafe { *data.get_unchecked(i) };
        if !val.is_finite() {
            has_invalid = true;
            break;
        }
        sum += val;
    }

    if has_invalid {
        // Start with tracking from beginning
        sma_f64_with_tracking(data, period, output);
        return;
    }

    // First output
    unsafe { *output.get_unchecked_mut(period - 1) = sum * inv_period; }

    // Optimistic fast path - no old value check needed
    // (old values guaranteed finite until we hit invalid new value)
    for i in period..n {
        let new_val = unsafe { *data.get_unchecked(i) };

        if new_val.is_finite() {
            sum += new_val - unsafe { *data.get_unchecked(i - period) };
            unsafe { *output.get_unchecked_mut(i) = sum * inv_period; }
        } else {
            // Hit invalid - switch to tracking for remainder
            sma_f64_with_tracking_from(data, period, output, i, sum);
            return;
        }
    }
}

/// Continue SMA with tracking from a specific index.
#[inline]
fn sma_f64_with_tracking_from(
    data: &[f64],
    period: usize,
    output: &mut [f64],
    start_i: usize,
    mut sum: f64,
) {
    let n = data.len();
    let inv_period = 1.0 / period as f64;

    // Allocate ring buffers
    let mut buf = vec![0.0f64; period];
    let mut inv = vec![0u8; period];

    // Populate ring with the valid window before start_i
    let mut invalid_count = 0usize;
    for j in 0..period {
        let idx_in_data = start_i - period + j;
        let val = data[idx_in_data];
        let is_invalid = !val.is_finite();

        buf[j] = if is_invalid { 0.0 } else { val };
        inv[j] = is_invalid as u8;
        invalid_count += inv[j] as usize;
    }

    // Recompute sum from sanitized buffer
    sum = buf.iter().sum();

    // Process from start_i onward with tracking
    for i in start_i..n {
        let idx = i % period;

        sum -= buf[idx];
        invalid_count -= inv[idx] as usize;

        let val = data[i];
        let is_invalid = !val.is_finite();

        buf[idx] = if is_invalid { 0.0 } else { val };
        inv[idx] = is_invalid as u8;

        sum += buf[idx];
        invalid_count += inv[idx] as usize;

        if invalid_count == 0 {
            output[i] = sum * inv_period;
        } else {
            output[i] = f64::NAN;
        }
    }
}

/// f32 SMA with optimistic fast path.
#[inline]
fn sma_f32_optimistic(data: &[f32], period: usize, output: &mut [f32]) {
    let n = data.len();
    let inv_period = 1.0 / period as f64;

    output[..period - 1].fill(f32::NAN);

    let mut sum = 0.0f64;
    let mut has_invalid = false;

    for i in 0..period {
        let val = unsafe { *data.get_unchecked(i) };
        if !val.is_finite() {
            has_invalid = true;
            break;
        }
        sum += val as f64;
    }

    if has_invalid {
        sma_f32_with_tracking(data, period, output);
        return;
    }

    unsafe { *output.get_unchecked_mut(period - 1) = (sum * inv_period) as f32; }

    for i in period..n {
        let new_val = unsafe { *data.get_unchecked(i) };

        if new_val.is_finite() {
            sum += new_val as f64 - unsafe { *data.get_unchecked(i - period) as f64 };
            unsafe { *output.get_unchecked_mut(i) = (sum * inv_period) as f32; }
        } else {
            sma_f32_with_tracking(data, period, output);
            return;
        }
    }
}

/// SIMD-optimized SMA for f64 - single pass with adaptive fast path.
///
/// Uses SIMD for initial window sum. If no invalid values are found during
/// initial window computation, switches to branchless rolling sum.
/// Otherwise uses nan_count tracking.
#[inline]
fn sma_f64_optimized(data: &[f64], period: usize, output: &mut [f64]) {
    let n = data.len();
    let inv_period = 1.0 / period as f64;

    // Fill lookback with NaN
    for item in output.iter_mut().take(period - 1) {
        *item = f64::NAN;
    }

    // Compute initial sum and count using SIMD
    let (mut sum, valid_count) = simd::sum_and_count_f64(&data[..period]);
    let invalid_count = period - valid_count;

    // First valid output
    if invalid_count == 0 {
        output[period - 1] = sum * inv_period;
    } else {
        output[period - 1] = f64::NAN;
    }

    // If initial window has no invalids, check if remaining data is also clean
    // by doing fast path with lazy validity detection
    if invalid_count == 0 {
        // Optimistic fast path - assume rest is clean, bail if we find invalid
        for i in period..n {
            let new_value = data[i];
            let old_value = data[i - period];

            // Branchless update when both values are finite
            // Most common case: both values are valid
            if new_value.is_finite() && old_value.is_finite() {
                sum = sum + new_value - old_value;
                output[i] = sum * inv_period;
            } else {
                // Hit an invalid value - fall back to tracking mode for rest
                sma_f64_tracking_tail(data, period, output, i, sum, new_value, old_value, inv_period);
                return;
            }
        }
    } else {
        // Slow path from start - use full tracking
        sma_f64_tracking_loop(data, period, output, period, sum, invalid_count, inv_period);
    }
}

/// Continue SMA computation with full NaN tracking from a given index.
#[inline]
fn sma_f64_tracking_tail(
    data: &[f64],
    period: usize,
    output: &mut [f64],
    start_i: usize,
    mut sum: f64,
    new_value: f64,
    old_value: f64,
    inv_period: f64,
) {
    // Process the current element that triggered the switch
    let mut invalid_count = 0usize;

    if !new_value.is_finite() {
        invalid_count += 1;
    } else {
        sum += new_value;
    }

    if !old_value.is_finite() {
        // old_value was invalid, so invalid_count was already 0, decrement not needed
    } else {
        sum -= old_value;
    }

    if invalid_count == 0 {
        output[start_i] = sum * inv_period;
    } else {
        output[start_i] = f64::NAN;
    }

    // Continue with tracking loop
    sma_f64_tracking_loop(data, period, output, start_i + 1, sum, invalid_count, inv_period);
}

/// Rolling sum loop with full invalid_count tracking.
#[inline]
fn sma_f64_tracking_loop(
    data: &[f64],
    period: usize,
    output: &mut [f64],
    start_i: usize,
    mut sum: f64,
    mut invalid_count: usize,
    inv_period: f64,
) {
    let n = data.len();

    for i in start_i..n {
        let new_value = data[i];
        let old_value = data[i - period];

        if !new_value.is_finite() {
            invalid_count += 1;
        } else {
            sum += new_value;
        }

        if !old_value.is_finite() {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            sum -= old_value;
        }

        if invalid_count == 0 {
            output[i] = sum * inv_period;
        } else {
            output[i] = f64::NAN;
        }
    }
}

/// Returns the lookback period for SMA.
///
/// The lookback is the number of NaN values at the start of the output.
/// For SMA, this is `period - 1`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::sma::sma_lookback;
///
/// assert_eq!(sma_lookback(5), 4);
/// assert_eq!(sma_lookback(14), 13);
/// ```
#[inline]
#[must_use]
pub const fn sma_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for SMA.
///
/// This is the smallest input size that will produce at least one valid output.
/// For SMA, this equals the period.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::sma::sma_min_len;
///
/// assert_eq!(sma_min_len(5), 5);
/// assert_eq!(sma_min_len(14), 14);
/// ```
#[inline]
#[must_use]
pub const fn sma_min_len(period: usize) -> usize {
    period
}

/// Computes the Simple Moving Average (SMA) of a data series.
///
/// Returns a vector of the same length as the input, where the first `period - 1`
/// values are NaN (insufficient lookback data) and subsequent values contain the
/// moving average.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods to average over
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` with the SMA values, or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for the output vector
/// - For f64 data: Uses SIMD-accelerated fast path when all values are finite
///
/// # NaN Handling
///
/// - The first `period - 1` elements of the output are NaN
/// - If any input value in the current window contains NaN, it will propagate to the output
///
/// # Example
///
/// ```
/// use fast_ta::indicators::sma::sma;
///
/// let data = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];
/// let result = sma(&data, 3).unwrap();
///
/// assert!(result[0].is_nan());
/// assert!(result[1].is_nan());
/// assert!((result[2] - 11.0).abs() < 1e-10);
/// ```
#[inline]
#[must_use = "this returns a Result with the SMA values, which should be used"]
pub fn sma<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    // Validate inputs
    crate::traits::validate_indicator_input(data, period, "sma")?;

    let n = data.len();

    // Optimized path for f64: uninitialized allocation + unchecked rolling sum
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        // SAFETY: We've verified T is f64
        let data_f64: &[f64] =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const f64, n) };

        // Allocate uninitialized (kernel writes all elements)
        let mut result_vec = Vec::<f64>::with_capacity(n);
        unsafe { result_vec.set_len(n); }
        let result_f64 = result_vec.as_mut_slice();

        // Use optimistic fast path (unchecked until first invalid)
        // Fastest for clean data, safe for data with NaN/Inf
        sma_f64_optimistic(data_f64, period, result_f64);

        // SAFETY: We've verified T is f64, transmute back
        return Ok(unsafe { std::mem::transmute(result_vec) });
    }

    // Optimized path for f32: uninitialized allocation + f64 accumulator
    if TypeId::of::<T>() == TypeId::of::<f32>() {
        let data_f32: &[f32] =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const f32, n) };

        let mut result_vec = Vec::<f32>::with_capacity(n);
        unsafe { result_vec.set_len(n); }
        let result_f32 = result_vec.as_mut_slice();

        sma_f32_optimistic(data_f32, period, result_f32);

        return Ok(unsafe { std::mem::transmute(result_vec) });
    }

    // Generic fallback for other types (still uses initialized vec)
    let mut result = vec![T::nan(); n];
    sma_generic(data, period, &mut result)?;
    Ok(result)
}

/// Generic SMA implementation for non-f64 types.
#[inline]
fn sma_generic<T: SeriesElement>(data: &[T], period: usize, result: &mut [T]) -> Result<()> {
    // Pre-compute reciprocal of period for faster multiply instead of divide
    let inv_period = T::one() / T::from_usize(period)?;

    // Compute initial sum for the first window, tracking non-finite values
    // Per project NaN propagation policy: both NaN and Infinity produce NaN output
    let mut sum = T::zero();
    let mut invalid_count = 0usize;
    for &value in data.iter().take(period) {
        if !value.is_finite() {
            invalid_count += 1;
        } else {
            sum = sum + value;
        }
    }

    // Set the first valid SMA value if no invalid values are present
    if invalid_count == 0 {
        result[period - 1] = sum * inv_period;
    }

    // Rolling sum for remaining elements: add new value, subtract oldest
    for i in period..data.len() {
        let new_value = data[i];
        let old_value = data[i - period];

        if !new_value.is_finite() {
            invalid_count += 1;
        } else {
            sum = sum + new_value;
        }

        if !old_value.is_finite() {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            sum = sum - old_value;
        }

        if invalid_count == 0 {
            result[i] = sum * inv_period;
        } else {
            result[i] = T::nan();
        }
    }

    Ok(())
}

/// Computes the Simple Moving Average into a pre-allocated output buffer.
///
/// This variant allows reusing an existing buffer to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods to average over
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid SMA values computed (`data.len()` - period + 1),
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
/// - The output buffer is shorter than the input data
///
/// # Example
///
/// ```
/// use fast_ta::indicators::sma::sma_into;
///
/// let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
/// let mut output = vec![0.0_f64; 5];
/// let valid_count = sma_into(&data, 3, &mut output).unwrap();
///
/// assert_eq!(valid_count, 3);
/// assert!(output[0].is_nan());
/// assert!((output[2] - 2.0).abs() < 1e-10);
/// ```
#[inline]
#[must_use = "this returns a Result with the count of valid SMA values"]
pub fn sma_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize> {
    // Validate inputs
    crate::traits::validate_indicator_input(data, period, "sma")?;

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "sma",
        });
    }

    // Optimized path for f64: unchecked rolling sum
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let data_f64: &[f64] =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const f64, data.len()) };
        let output_f64: &mut [f64] =
            unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut f64, data.len()) };

        sma_f64_optimistic(data_f64, period, output_f64);

        return Ok(data.len() - period + 1);
    }

    // Optimized path for f32
    if TypeId::of::<T>() == TypeId::of::<f32>() {
        let data_f32: &[f32] =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const f32, data.len()) };
        let output_f32: &mut [f32] =
            unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut f32, data.len()) };

        sma_f32_optimistic(data_f32, period, output_f32);

        return Ok(data.len() - period + 1);
    }

    // Generic fallback for other types
    sma_generic_into(data, period, output)
}

/// Generic SMA into buffer implementation for non-f64 types.
#[inline]
fn sma_generic_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize> {
    // Pre-compute reciprocal of period for faster multiply instead of divide
    let inv_period = T::one() / T::from_usize(period)?;

    // Initialize lookback period with NaN
    for item in output.iter_mut().take(period - 1) {
        *item = T::nan();
    }

    // Compute initial sum for the first window, tracking non-finite values
    // Per project NaN propagation policy: both NaN and Infinity produce NaN output
    let mut sum = T::zero();
    let mut invalid_count = 0usize;
    for &value in data.iter().take(period) {
        if !value.is_finite() {
            invalid_count += 1;
        } else {
            sum = sum + value;
        }
    }

    // Set the first valid SMA value if no invalid values are present
    if invalid_count == 0 {
        output[period - 1] = sum * inv_period;
    } else {
        output[period - 1] = T::nan();
    }

    // Rolling sum for remaining elements
    for i in period..data.len() {
        let new_value = data[i];
        let old_value = data[i - period];

        if !new_value.is_finite() {
            invalid_count += 1;
        } else {
            sum = sum + new_value;
        }

        if !old_value.is_finite() {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            sum = sum - old_value;
        }

        if invalid_count == 0 {
            output[i] = sum * inv_period;
        } else {
            output[i] = T::nan();
        }
    }

    // Return count of valid (non-NaN) values
    Ok(data.len() - period + 1)
}

/// Computes the Simple Moving Average starting from a given index into a pre-allocated buffer.
///
/// This variant is useful when you have data that is valid only from a certain index onwards
/// (e.g., when the first part of the data contains NaN values from a previous calculation).
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods to average over
/// * `start_idx` - The index from which to start computing SMA (all indices before this will be NaN)
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid SMA values computed, or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The output buffer is shorter than the input data
/// - `start_idx` is out of bounds
///
/// # Example
///
/// ```
/// use fast_ta::indicators::sma::sma_from_idx_into;
///
/// let data = vec![f64::NAN, f64::NAN, 1.0, 2.0, 3.0, 4.0, 5.0];
/// let mut output = vec![0.0_f64; 7];
/// let valid_count = sma_from_idx_into(&data, 3, 2, &mut output).unwrap();
///
/// // First 4 values are NaN (2 from start_idx + 2 from lookback)
/// assert!(output[0].is_nan());
/// assert!(output[3].is_nan());
/// assert!((output[4] - 2.0).abs() < 1e-10); // (1+2+3)/3
/// ```
#[inline]
#[must_use = "this returns a Result with the count of valid SMA values"]
pub fn sma_from_idx_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    start_idx: usize,
    output: &mut [T],
) -> Result<usize> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "sma_from_idx",
        });
    }

    // Handle edge case: start_idx beyond data
    if start_idx >= data.len() {
        // Fill all output with NaN
        for item in output.iter_mut().take(data.len()) {
            *item = T::nan();
        }
        return Ok(0);
    }

    // Fill all indices before the first valid SMA position with NaN
    // First valid SMA can be at start_idx + period - 1
    let first_valid_idx = start_idx + period - 1;

    // If there's not enough data from start_idx for even one SMA window
    if first_valid_idx >= data.len() {
        for item in output.iter_mut().take(data.len()) {
            *item = T::nan();
        }
        return Ok(0);
    }

    // Fill all positions before first valid output with NaN
    for item in output.iter_mut().take(first_valid_idx) {
        *item = T::nan();
    }

    // Pre-compute reciprocal of period for faster multiply instead of divide
    let inv_period = T::one() / T::from_usize(period)?;

    // Compute initial sum for the first window starting at start_idx
    // Per project NaN propagation policy: both NaN and Infinity produce NaN output
    let mut sum = T::zero();
    let mut invalid_count = 0usize;
    for i in start_idx..(start_idx + period) {
        let value = data[i];
        if !value.is_finite() {
            invalid_count += 1;
        } else {
            sum = sum + value;
        }
    }

    // Set the first valid SMA value if no invalid values are present
    if invalid_count == 0 {
        output[first_valid_idx] = sum * inv_period;
    } else {
        output[first_valid_idx] = T::nan();
    }

    // Rolling sum for remaining elements
    let mut valid_count = if invalid_count == 0 { 1 } else { 0 };
    for i in (first_valid_idx + 1)..data.len() {
        let new_value = data[i];
        let old_value = data[i - period];

        if !new_value.is_finite() {
            invalid_count += 1;
        } else {
            sum = sum + new_value;
        }

        if !old_value.is_finite() {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            sum = sum - old_value;
        }

        if invalid_count == 0 {
            output[i] = sum * inv_period;
            valid_count += 1;
        } else {
            output[i] = T::nan();
        }
    }

    Ok(valid_count)
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
        (a - b).abs() < epsilon
    }

    const EPSILON: f64 = 1e-10;
    const EPSILON_F32: f32 = 1e-5;

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_sma_basic() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&data, 3).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(approx_eq(result[2], 2.0, EPSILON)); // (1+2+3)/3
        assert!(approx_eq(result[3], 3.0, EPSILON)); // (2+3+4)/3
        assert!(approx_eq(result[4], 4.0, EPSILON)); // (3+4+5)/3
    }

    #[test]
    fn test_sma_f32() {
        let data = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&data, 3).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(approx_eq(result[2], 2.0_f32, EPSILON_F32));
        assert!(approx_eq(result[3], 3.0_f32, EPSILON_F32));
        assert!(approx_eq(result[4], 4.0_f32, EPSILON_F32));
    }

    #[test]
    fn test_sma_period_one() {
        // SMA(1) should equal the input values
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&data, 1).unwrap();

        assert_eq!(result.len(), 5);
        assert!(approx_eq(result[0], 1.0, EPSILON));
        assert!(approx_eq(result[1], 2.0, EPSILON));
        assert!(approx_eq(result[2], 3.0, EPSILON));
        assert!(approx_eq(result[3], 4.0, EPSILON));
        assert!(approx_eq(result[4], 5.0, EPSILON));
    }

    #[test]
    fn test_sma_period_equals_length() {
        // Period equals data length - only one valid output
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&data, 5).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
        assert!(approx_eq(result[4], 3.0, EPSILON)); // (1+2+3+4+5)/5 = 15/5 = 3
    }

    #[test]
    fn test_sma_single_element_period_one() {
        let data = vec![42.0_f64];
        let result = sma(&data, 1).unwrap();

        assert_eq!(result.len(), 1);
        assert!(approx_eq(result[0], 42.0, EPSILON));
    }

    // ==================== Reference Value Tests ====================

    #[test]
    fn test_sma_known_values() {
        // Test against known/expected SMA values
        let data = vec![
            22.27_f64, 22.19, 22.08, 22.17, 22.18, 22.13, 22.23, 22.43, 22.24, 22.29,
        ];
        let result = sma(&data, 5).unwrap();

        // Expected values calculated manually:
        // SMA[4] = (22.27 + 22.19 + 22.08 + 22.17 + 22.18) / 5 = 22.178
        // SMA[5] = (22.19 + 22.08 + 22.17 + 22.18 + 22.13) / 5 = 22.15
        // etc.

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
        assert!(approx_eq(result[4], 22.178, 1e-6));
        assert!(approx_eq(result[5], 22.15, 1e-6));
    }

    #[test]
    fn test_sma_constant_values() {
        // SMA of constant values should equal the constant
        let data = vec![5.0_f64; 10];
        let result = sma(&data, 3).unwrap();

        for i in 2..result.len() {
            assert!(approx_eq(result[i], 5.0, EPSILON));
        }
    }

    #[test]
    fn test_sma_linear_sequence() {
        // For a linear sequence 1,2,3,4,5,6,7,8,9,10 with period 3
        // SMA should be at the center of each window
        let data: Vec<f64> = (1..=10).map(|x| x as f64).collect();
        let result = sma(&data, 3).unwrap();

        // For odd-period SMA of a linear sequence, result equals middle value
        assert!(approx_eq(result[2], 2.0, EPSILON)); // Center of 1,2,3
        assert!(approx_eq(result[3], 3.0, EPSILON)); // Center of 2,3,4
        assert!(approx_eq(result[9], 9.0, EPSILON)); // Center of 8,9,10
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_sma_with_nan_in_data() {
        // NaN in the middle of the data should propagate
        let data = vec![1.0_f64, 2.0, f64::NAN, 4.0, 5.0, 6.0];
        let result = sma(&data, 3).unwrap();

        // Windows containing NaN should produce NaN output
        assert!(result[0].is_nan()); // lookback
        assert!(result[1].is_nan()); // lookback
        assert!(result[2].is_nan()); // window contains NaN
        assert!(result[3].is_nan()); // window contains NaN
        assert!(result[4].is_nan()); // window contains NaN
        assert!(approx_eq(result[5], 5.0, EPSILON)); // (4+5+6)/3 - NaN rolled out
    }

    #[test]
    fn test_sma_negative_values() {
        let data = vec![-5.0_f64, -3.0, -1.0, 1.0, 3.0, 5.0];
        let result = sma(&data, 3).unwrap();

        assert!(approx_eq(result[2], -3.0, EPSILON)); // (-5-3-1)/3
        assert!(approx_eq(result[3], -1.0, EPSILON)); // (-3-1+1)/3
        assert!(approx_eq(result[4], 1.0, EPSILON)); // (-1+1+3)/3
        assert!(approx_eq(result[5], 3.0, EPSILON)); // (1+3+5)/3
    }

    #[test]
    fn test_sma_large_values() {
        // Test with very large values to check for overflow issues
        let data = vec![1e15_f64, 2e15, 3e15, 4e15, 5e15];
        let result = sma(&data, 3).unwrap();

        assert!(approx_eq(result[2], 2e15, 1e5)); // Larger epsilon for large values
        assert!(approx_eq(result[3], 3e15, 1e5));
        assert!(approx_eq(result[4], 4e15, 1e5));
    }

    #[test]
    fn test_sma_small_values() {
        // Test with very small values
        let data = vec![1e-15_f64, 2e-15, 3e-15, 4e-15, 5e-15];
        let result = sma(&data, 3).unwrap();

        assert!(approx_eq(result[2], 2e-15, 1e-25));
        assert!(approx_eq(result[3], 3e-15, 1e-25));
        assert!(approx_eq(result[4], 4e-15, 1e-25));
    }

    #[test]
    fn test_sma_alternating_values() {
        // Test with alternating values
        let data = vec![1.0_f64, -1.0, 1.0, -1.0, 1.0, -1.0];
        let result = sma(&data, 2).unwrap();

        // (1 + -1) / 2 = 0 for all pairs
        assert!(result[0].is_nan());
        assert!(approx_eq(result[1], 0.0, EPSILON));
        assert!(approx_eq(result[2], 0.0, EPSILON));
        assert!(approx_eq(result[3], 0.0, EPSILON));
    }

    #[test]
    fn test_sma_infinity_handling() {
        // Test with infinity values
        // Per project NaN propagation policy: INFINITY produces NaN output
        let data = vec![1.0_f64, f64::INFINITY, 3.0, 4.0, 5.0];
        let result = sma(&data, 3).unwrap();

        // Window containing infinity produces NaN
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
        // After infinity exits the window, normal values resume
        assert!(!result[4].is_nan());
    }

    // ==================== Buffer Tests ====================

    #[test]
    fn test_sma_into_basic() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f64; 5];
        let valid_count = sma_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!(approx_eq(output[2], 2.0, EPSILON));
        assert!(approx_eq(output[3], 3.0, EPSILON));
        assert!(approx_eq(output[4], 4.0, EPSILON));
    }

    #[test]
    fn test_sma_into_larger_buffer() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f64; 10];
        let valid_count = sma_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!(approx_eq(output[2], 2.0, EPSILON));
        assert!(approx_eq(output[3], 3.0, EPSILON));
        assert!(approx_eq(output[4], 4.0, EPSILON));
    }

    #[test]
    fn test_sma_into_buffer_too_small() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f64; 3];
        let result = sma_into(&data, 3, &mut output);

        assert!(result.is_err());
    }

    #[test]
    fn test_sma_into_with_nan() {
        let data = vec![1.0_f64, 2.0, f64::NAN, 4.0, 5.0, 6.0];
        let mut output = vec![0.0_f64; 6];
        let valid_count = sma_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 4);
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!(output[2].is_nan()); // Contains NaN
        assert!(output[3].is_nan()); // Contains NaN
        assert!(output[4].is_nan()); // Contains NaN
        assert!(approx_eq(output[5], 5.0, EPSILON));
    }

    // ==================== Error Cases ====================

    #[test]
    fn test_sma_empty_input() {
        let data: Vec<f64> = vec![];
        let result = sma(&data, 3);

        assert!(result.is_err());
    }

    #[test]
    fn test_sma_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = sma(&data, 0);

        assert!(result.is_err());
    }

    #[test]
    fn test_sma_insufficient_data() {
        let data = vec![1.0_f64, 2.0];
        let result = sma(&data, 5);

        assert!(result.is_err());
    }

    #[test]
    fn test_sma_into_empty_input() {
        let data: Vec<f64> = vec![];
        let mut output = vec![0.0_f64; 5];
        let result = sma_into(&data, 3, &mut output);

        assert!(result.is_err());
    }

    #[test]
    fn test_sma_into_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let mut output = vec![0.0_f64; 3];
        let result = sma_into(&data, 0, &mut output);

        assert!(result.is_err());
    }

    #[test]
    fn test_sma_into_insufficient_data() {
        let data = vec![1.0_f64, 2.0];
        let mut output = vec![0.0_f64; 5];
        let result = sma_into(&data, 5, &mut output);

        assert!(result.is_err());
    }

    // ==================== Output Consistency Tests ====================

    #[test]
    fn test_sma_and_sma_into_consistency() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = sma(&data, 5).unwrap();

        let mut output = vec![0.0_f64; 10];
        sma_into(&data, 5, &mut output).unwrap();

        for i in 0..result.len() {
            if result[i].is_nan() {
                assert!(output[i].is_nan());
            } else {
                assert!(approx_eq(result[i], output[i], EPSILON));
            }
        }
    }

    #[test]
    fn test_sma_lookback_function() {
        assert_eq!(sma_lookback(1), 0);
        assert_eq!(sma_lookback(5), 4);
        assert_eq!(sma_lookback(14), 13);
        assert_eq!(sma_lookback(20), 19);
    }

    #[test]
    fn test_sma_min_len_function() {
        assert_eq!(sma_min_len(1), 1);
        assert_eq!(sma_min_len(5), 5);
        assert_eq!(sma_min_len(14), 14);
        assert_eq!(sma_min_len(20), 20);
    }

    #[test]
    fn test_sma_mathematical_properties() {
        // Test the mathematical property: SMA is the mean of the window
        let data = vec![10.0_f64, 20.0, 30.0, 40.0, 50.0];
        let result = sma(&data, 3).unwrap();

        // Manually verify some values
        let expected_2 = (10.0 + 20.0 + 30.0) / 3.0;
        let expected_3 = (20.0 + 30.0 + 40.0) / 3.0;
        let expected_4 = (30.0 + 40.0 + 50.0) / 3.0;

        assert!(approx_eq(result[2], expected_2, EPSILON));
        assert!(approx_eq(result[3], expected_3, EPSILON));
        assert!(approx_eq(result[4], expected_4, EPSILON));
    }
}
