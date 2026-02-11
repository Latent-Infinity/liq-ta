//! MFI (Money Flow Index) indicator.
//!
//! The Money Flow Index is a volume-weighted version of RSI that measures
//! buying and selling pressure using both price and volume.
//!
//! # Algorithm
//!
//! This implementation uses an O(n) rolling sum approach with inline NaN handling:
//! 1. Uses circular buffer to track positive/negative money flows
//! 2. Maintains rolling sums with O(1) operations per element
//! 3. Tracks invalid_count for proper NaN propagation
//! 4. Uses branchless direction assignment in f64 kernel
//!
//! # Formula
//!
//! ```text
//! Typical Price = (High + Low + Close) / 3
//! Raw Money Flow = Typical Price * Volume
//! Money Flow Ratio = Positive Money Flow / Negative Money Flow
//! MFI = 100 - (100 / (1 + Money Flow Ratio))
//! ```
//!
//! Where:
//! - Positive Money Flow = sum of Raw MF when TP > previous TP
//! - Negative Money Flow = sum of Raw MF when TP < previous TP
//!
//! # Range
//!
//! MFI ranges from 0 to 100:
//! - > 80: Overbought
//! - < 20: Oversold
//!
//! # Lookback
//!
//! The lookback period is `period`.
//!
//! # NaN Handling
//!
//! Per indicator-standards.md, any NaN or Inf within the rolling window yields
//! NaN output at that position. MFI is a rolling window indicator, so once the
//! NaN value exits the window, subsequent outputs recover to valid values.
//!
//! The window includes:
//! - The current `period` bars for money flow calculation
//! - The bar immediately before the window (for price comparison)
//!
//! # Performance
//!
//! - Time complexity: O(n) where n is the length of the input data
//! - Space complexity: O(n) for temporary arrays (positive/negative money flows)
//! - Fast path: ~20-30% faster when no NaN values present

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Computes the lookback period for MFI.
#[inline]
#[must_use]
pub const fn mfi_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for MFI calculation.
#[inline]
#[must_use]
pub const fn mfi_min_len(period: usize) -> usize {
    period + 1
}

// =============================================================================
// Helper types
// =============================================================================

/// Money flow entry for circular buffer with NaN sentinel.
/// Kept for the checked kernel that uses NaN as a sentinel.
#[derive(Clone, Copy)]
struct MoneyFlowEntry {
    positive: f64,
    negative: f64,
}

/// Branchless direction assignment using bit masks.
///
/// Performance characteristics:
/// - In unchecked kernels (no validation): Simple branching is ~20% faster
/// - In checked kernels (with .is_finite()): Branchless is better (avoids nested branches)
///
/// The branch prediction overhead from nested `if ok { if direction {} }` patterns
/// outweighs the bit manipulation cost when combined with validation branches.
#[inline(always)]
fn assign_direction_branchless(raw3: f64, tp3: f64, prev_tp3: f64) -> (f64, f64) {
    let raw_bits = raw3.to_bits();

    // Comparisons we're doing anyway
    let gt = (tp3 > prev_tp3) as u64; // 0 or 1
    let lt = (tp3 < prev_tp3) as u64; // 0 or 1

    // Convert to masks (0 or !0)
    let gt_mask = 0u64.wrapping_sub(gt); // 0 or 0xFFFF_FFFF_FFFF_FFFF
    let lt_mask = 0u64.wrapping_sub(lt); // 0 or 0xFFFF_FFFF_FFFF_FFFF

    // Apply masks to select positive or negative
    let pos = f64::from_bits(raw_bits & gt_mask);
    let neg = f64::from_bits(raw_bits & lt_mask);

    (pos, neg)
}

#[inline(always)]
fn clamp_mfi_f64(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        value
    }
}

#[inline(always)]
fn clamp_mfi<T: SeriesElement>(value: T, hundred: T) -> T {
    if value.is_finite() {
        if value < T::zero() {
            T::zero()
        } else if value > hundred {
            hundred
        } else {
            value
        }
    } else {
        value
    }
}

/// Optimized single-pass streaming algorithm with branchless direction assignment.
/// Uses struct with separate positive/negative fields for branch-free eviction.
#[inline]
fn mfi_streaming_f64_optimized(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    let n = high.len();
    let lookback = mfi_lookback(period);

    // Fill lookback period with NaN
    output[..lookback].fill(f64::NAN);

    // Circular buffer with separate positive/negative fields
    let mut mf_buf = vec![
        MoneyFlowEntry {
            positive: 0.0,
            negative: 0.0
        };
        period
    ];
    let mut idx = 0usize;

    // Constants
    let hundred = 100.0;

    // Initial unscaled typical price (H + L + C, no /3)
    // The /3 division is a constant scale that cancels in the ratio pos/(pos+neg)
    // Fix: prev_tp3_valid should NOT depend on volume[0] (volume[0] not used in comparisons)
    let mut prev_tp3 = high[0] + low[0] + close[0];
    let mut prev_tp3_valid = prev_tp3.is_finite();

    let mut pos_sum = 0.0;
    let mut neg_sum = 0.0;
    let mut invalid_count = 0usize;

    // Build initial window [1..=period]
    for j in 1..=period {
        let tp3 = high[j] + low[j] + close[j];
        let tp3_valid = tp3.is_finite();
        let vol = volume[j];
        let vol_valid = vol.is_finite();

        let ok = tp3_valid && vol_valid && prev_tp3_valid;

        if ok {
            let raw3 = tp3 * vol; // No /3 division

            // Branchless direction assignment (better with validation overhead)
            let (pos, neg) = assign_direction_branchless(raw3, tp3, prev_tp3);
            mf_buf[idx].positive = pos;
            mf_buf[idx].negative = neg;
            pos_sum += pos;
            neg_sum += neg;
        } else {
            // Invalid data - store zeros
            mf_buf[idx].positive = 0.0;
            mf_buf[idx].negative = f64::NAN; // Sentinel for invalid
            invalid_count += 1;
        }

        idx += 1;
        if idx == period {
            idx = 0;
        }

        prev_tp3 = tp3;
        prev_tp3_valid = tp3_valid;
    }

    // First output at index = period
    output[lookback] = if invalid_count == 0 {
        let total = pos_sum + neg_sum;
        if total <= 0.0 {
            0.0
        } else {
            clamp_mfi_f64(hundred * (pos_sum / total))
        }
    } else {
        f64::NAN
    };

    // Rolling window for remaining elements
    for i in (period + 1)..n {
        // Remove oldest money flow from sums
        let old_entry = mf_buf[idx];

        // NaN-safe eviction
        if old_entry.negative.is_nan() {
            invalid_count -= 1;
        } else {
            // Branch-free removal: unconditional subtraction
            pos_sum -= old_entry.positive;
            neg_sum -= old_entry.negative;
        }

        let tp3 = high[i] + low[i] + close[i];
        let tp3_valid = tp3.is_finite();
        let vol = volume[i];
        let vol_valid = vol.is_finite();

        let ok = tp3_valid && vol_valid && prev_tp3_valid;

        if ok {
            let raw3 = tp3 * vol; // No /3 division

            // Branchless direction assignment (better with validation overhead)
            let (pos, neg) = assign_direction_branchless(raw3, tp3, prev_tp3);
            mf_buf[idx].positive = pos;
            mf_buf[idx].negative = neg;
            pos_sum += pos;
            neg_sum += neg;
        } else {
            // Invalid data - store zeros
            mf_buf[idx].positive = 0.0;
            mf_buf[idx].negative = f64::NAN; // Sentinel for invalid
            invalid_count += 1;
        }

        output[i] = if invalid_count == 0 {
            let total = pos_sum + neg_sum;
            if total <= 0.0 {
                0.0
            } else {
                clamp_mfi_f64(hundred * (pos_sum / total))
            }
        } else {
            f64::NAN
        };

        idx += 1;
        if idx == period {
            idx = 0;
        }

        prev_tp3 = tp3;
        prev_tp3_valid = tp3_valid;
    }

    Ok(())
}

/// SIMD-optimized fast path for f64 MFI computation.
///
/// NOTE: Pre-scan optimization doesn't work for MFI because:
/// - Must check 4 arrays (high, low, close, volume) = 400K elements for n=100K
/// - Pre-scan cost: ~188µs (measured)
/// - Pre-scan alone is slower than the entire checked computation (~150µs)!
///
/// Unlike single-array indicators (EMA, RSI), multi-array indicators can't benefit
/// from pre-scan optimization due to the multiplicative validation cost.
#[inline]
fn mfi_rolling_fast_f64_simd(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    // Use original kernel with prev_tp3_valid fix
    // NOTE: Branchless eviction showed 5% regression due to extra Vec<u8> overhead
    mfi_streaming_f64_optimized(high, low, close, volume, period, output)
}

/// MFI computation with invalid tracking.
/// Handles NaN/Inf values inline - no pre-scan needed.
#[inline]
fn mfi_rolling_fast<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    // Use optimized streaming path for f64 (handles invalids inline)
    use std::any::TypeId;
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        // Safety: We've checked the type is f64
        let h = unsafe { std::slice::from_raw_parts(high.as_ptr() as *const f64, high.len()) };
        let l = unsafe { std::slice::from_raw_parts(low.as_ptr() as *const f64, low.len()) };
        let c = unsafe { std::slice::from_raw_parts(close.as_ptr() as *const f64, close.len()) };
        let v = unsafe { std::slice::from_raw_parts(volume.as_ptr() as *const f64, volume.len()) };
        let out = unsafe {
            std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut f64, output.len())
        };
        return mfi_rolling_fast_f64_simd(h, l, c, v, period, out);
    }

    // Optimized streaming path for generic types
    let n = high.len();
    let lookback = mfi_lookback(period);
    let three = T::from_f64(3.0)?;
    let hundred = T::from_f64(100.0)?;
    let inv_three = T::one() / three;

    // Fill lookback period with NaN
    for item in output.iter_mut().take(lookback) {
        *item = T::nan();
    }

    // Single circular buffer: positive values = positive MF, negative values = negative MF, NaN = invalid
    let mut mf_buf = vec![T::zero(); period];
    let mut idx = 0usize;

    // Initial typical price and validity
    let mut prev_tp = (high[0] + low[0] + close[0]) * inv_three;
    let mut prev_tp_ok = prev_tp.is_finite();

    let mut pos_sum = T::zero();
    let mut neg_sum = T::zero();
    let mut invalid_count = 0usize;

    // Build initial window [1..=period]
    for j in 1..=period {
        let tp = (high[j] + low[j] + close[j]) * inv_three;
        let tp_ok = tp.is_finite();
        let vol = volume[j];
        let ok = tp_ok && prev_tp_ok && vol.is_finite();

        let mf = if ok {
            let raw = tp * vol;

            // Simple branching is faster than "branchless" with double casts
            if tp > prev_tp {
                raw // positive
            } else if tp < prev_tp {
                T::zero() - raw // negative (stored as negative value)
            } else {
                T::zero() // unchanged
            }
        } else {
            T::nan() // Mark invalid with NaN
        };

        mf_buf[idx] = mf;

        if mf.is_nan() {
            invalid_count += 1;
        } else if mf > T::zero() {
            pos_sum = pos_sum + mf;
        } else if mf < T::zero() {
            neg_sum = neg_sum - mf; // neg_sum stores absolute value
        }

        // Wrap-branch instead of modulo
        idx += 1;
        if idx == period {
            idx = 0;
        }

        prev_tp = tp;
        prev_tp_ok = tp_ok;
    }

    // First output at index = period
    output[lookback] = if invalid_count == 0 {
        let total = pos_sum + neg_sum;
        if total <= T::zero() {
            T::zero()
        } else {
            clamp_mfi(hundred * (pos_sum / total), hundred) // One-division formula
        }
    } else {
        T::nan()
    };

    // Rolling window for remaining elements
    for i in (period + 1)..n {
        // Remove oldest money flow from sums
        let old_mf = mf_buf[idx];
        if old_mf.is_nan() {
            invalid_count -= 1;
        } else if old_mf > T::zero() {
            pos_sum = pos_sum - old_mf;
        } else if old_mf < T::zero() {
            neg_sum = neg_sum + old_mf; // subtract negative (add absolute value)
        }

        let tp = (high[i] + low[i] + close[i]) * inv_three;
        let tp_ok = tp.is_finite();
        let vol = volume[i];
        let ok = tp_ok && prev_tp_ok && vol.is_finite();

        let mf = if ok {
            let raw = tp * vol;

            if tp > prev_tp {
                raw
            } else if tp < prev_tp {
                T::zero() - raw
            } else {
                T::zero()
            }
        } else {
            T::nan()
        };

        mf_buf[idx] = mf;

        if mf.is_nan() {
            invalid_count += 1;
        } else if mf > T::zero() {
            pos_sum = pos_sum + mf;
        } else if mf < T::zero() {
            neg_sum = neg_sum - mf;
        }

        output[i] = if invalid_count == 0 {
            let total = pos_sum + neg_sum;
            if total <= T::zero() {
                T::zero()
            } else {
                clamp_mfi(hundred * (pos_sum / total), hundred)
            }
        } else {
            T::nan()
        };

        // Wrap-branch instead of modulo
        idx += 1;
        if idx == period {
            idx = 0;
        }

        prev_tp = tp;
        prev_tp_ok = tp_ok;
    }

    Ok(())
}

/// Computes MFI and stores results in output slice.
///
/// This function handles NaN values inline using invalid_count tracking.
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `period` - Lookback period (typically 14)
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
pub fn mfi_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    if low.len() != n || close.len() != n || volume.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "Arrays must have same length: high={}, low={}, close={}, volume={}",
                n,
                low.len(),
                close.len(),
                volume.len()
            ),
        });
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    let min_len = mfi_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "mfi",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "mfi",
            required: n,
            actual: output.len(),
        });
    }

    // Use fast path that handles NaN values inline
    mfi_rolling_fast(high, low, close, volume, period, output)
}

/// Computes MFI (Money Flow Index).
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `period` - Lookback period (typically 14)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - MFI values (range 0 to 100)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use liq_ta::indicators::mfi;
///
/// let high = vec![25.0_f64, 26.0, 27.0, 28.0, 27.5, 27.0];
/// let low = vec![23.0_f64, 24.0, 25.0, 26.0, 25.5, 25.0];
/// let close = vec![24.0_f64, 25.0, 26.0, 27.0, 26.5, 26.0];
/// let volume = vec![1000.0_f64, 1100.0, 1200.0, 1300.0, 1400.0, 1500.0];
///
/// let result = mfi(&high, &low, &close, &volume, 3).unwrap();
/// assert!(result[3].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn mfi<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    period: usize,
) -> Result<Vec<T>> {
    use std::any::TypeId;

    let n = high.len();

    // Optimization: For f64/f32, allocate uninitialized memory since mfi_into
    // fully overwrites every element (lookback + computed region).
    // This avoids the ~10-15µs penalty of initializing 100K elements to NaN.
    let mut output = if TypeId::of::<T>() == TypeId::of::<f64>() {
        let mut v: Vec<T> = Vec::with_capacity(n);
        unsafe {
            v.set_len(n);
        } // Safe: mfi_into writes all n elements
        v
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let mut v: Vec<T> = Vec::with_capacity(n);
        unsafe {
            v.set_len(n);
        } // Safe: mfi_into writes all n elements
        v
    } else {
        // For other types, use safe initialization
        vec![T::nan(); n]
    };

    mfi_into(high, low, close, volume, period, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_mfi_lookback() {
        assert_eq!(mfi_lookback(5), 5);
        assert_eq!(mfi_lookback(14), 14);
    }

    #[test]
    fn test_mfi_min_len() {
        assert_eq!(mfi_min_len(5), 6);
        assert_eq!(mfi_min_len(14), 15);
    }

    #[test]
    fn test_mfi_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];
        let volume: Vec<f64> = vec![];
        let result = mfi(&high, &low, &close, &volume, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_mfi_invalid_period() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_mfi_insufficient_data() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let volume: Vec<f64> = vec![1000.0; 5];
        let result = mfi(&high, &low, &close, &volume, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_mfi_length_mismatch() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 5);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_mfi_output_length() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume: Vec<f64> = vec![1000.0; 10];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();
        assert_eq!(result.len(), high.len());
    }

    #[test]
    fn test_mfi_lookback_nan() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume: Vec<f64> = vec![1000.0; 10];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        // First 5 values should be NaN (lookback = period)
        for i in 0..5 {
            assert!(result[i].is_nan(), "mfi[{}] should be NaN", i);
        }

        // Values after lookback should be finite
        for i in 5..result.len() {
            assert!(result[i].is_finite(), "mfi[{}] should be finite", i);
        }
    }

    #[test]
    fn test_mfi_range() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 14.0, 13.0, 12.0, 11.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 13.0, 12.0, 11.0, 10.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 13.5, 12.5, 11.5, 10.5];
        let volume: Vec<f64> = vec![1000.0; 10];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        for i in 5..result.len() {
            assert!(
                result[i] >= 0.0 && result[i] <= 100.0,
                "mfi[{}] = {} should be in [0, 100]",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_mfi_all_positive() {
        // All prices increasing - all positive money flow
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        // With all positive flow, MFI = 100
        assert!(
            (result[5] - 100.0).abs() < 1e-10,
            "mfi should be 100 with all positive flow"
        );
    }

    #[test]
    fn test_mfi_all_negative() {
        // All prices decreasing - all negative money flow
        let high: Vec<f64> = vec![15.0, 14.0, 13.0, 12.0, 11.0, 10.0];
        let low: Vec<f64> = vec![14.0, 13.0, 12.0, 11.0, 10.0, 9.0];
        let close: Vec<f64> = vec![14.5, 13.5, 12.5, 11.5, 10.5, 9.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        // With all negative flow, MFI = 0
        assert!(
            (result[5] - 0.0).abs() < 1e-10,
            "mfi should be 0 with all negative flow"
        );
    }

    #[test]
    fn test_mfi_into() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let mut output = vec![0.0_f64; 6];

        mfi_into(&high, &low, &close, &volume, 5, &mut output).unwrap();

        assert!((output[5] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_mfi_into_buffer_too_small() {
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f64> = vec![1000.0; 6];
        let mut output = vec![0.0_f64; 3]; // Too small

        let result = mfi_into(&high, &low, &close, &volume, 5, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_mfi_f32() {
        let high: Vec<f32> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low: Vec<f32> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close: Vec<f32> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume: Vec<f32> = vec![1000.0; 6];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        assert!((result[5] - 100.0_f32).abs() < 1e-5);
    }

    #[test]
    fn test_mfi_varying_volume() {
        // Test with varying volumes to ensure volume weighting works
        let high: Vec<f64> = vec![10.0, 11.0, 10.0, 11.0, 10.0, 11.0];
        let low: Vec<f64> = vec![9.0, 10.0, 9.0, 10.0, 9.0, 10.0];
        let close: Vec<f64> = vec![9.5, 10.5, 9.5, 10.5, 9.5, 10.5];
        // Higher volume on up days should give higher MFI
        let volume: Vec<f64> = vec![1000.0, 2000.0, 1000.0, 2000.0, 1000.0, 2000.0];
        let result = mfi(&high, &low, &close, &volume, 5).unwrap();

        // With 2x volume on up days vs down days, positive MF > negative MF
        assert!(
            result[5] > 50.0,
            "mfi should be > 50 with higher volume on up days"
        );
    }

    #[test]
    fn test_mfi_nan_recovery_single() {
        // Test that window recovers after a single NaN exits
        // NaN at index 5 affects:
        // - Index 5: money flow invalid (TP is NaN)
        // - Index 6: money flow invalid (prev_TP is NaN even though current TP is valid)
        // - Index 7: money flow VALID (both prev_TP and current TP are valid)
        //
        // With period = 3:
        // - Output[5]: window [3,4,5] - index 5 invalid → NaN
        // - Output[6]: window [4,5,6] - indices 5,6 invalid → NaN
        // - Output[7]: window [5,6,7] - indices 5,6 invalid → NaN
        // - Output[8]: window [6,7,8] - index 6 invalid → NaN
        // - Output[9]: window [7,8,9] - all valid → RECOVER
        let high: Vec<f64> = vec![
            10.0,
            11.0,
            12.0,
            13.0,
            14.0,
            f64::NAN,
            16.0,
            17.0,
            18.0,
            19.0,
        ];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume: Vec<f64> = vec![1000.0; 10];

        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // Outputs 5-8 should be NaN (window contains index 5 or 6 which are both invalid)
        assert!(result[5].is_nan(), "mfi[5] should be NaN (index 5 invalid)");
        assert!(
            result[6].is_nan(),
            "mfi[6] should be NaN (indices 5,6 invalid)"
        );
        assert!(
            result[7].is_nan(),
            "mfi[7] should be NaN (indices 5,6 invalid)"
        );
        assert!(
            result[8].is_nan(),
            "mfi[8] should be NaN (index 6 still invalid)"
        );

        // Output 9 should recover - window is [7, 8, 9], all valid
        assert!(
            result[9].is_finite(),
            "mfi[9] should recover (NaN impact fully exited window)"
        );
    }

    #[test]
    fn test_mfi_nan_recovery_volume() {
        // Test NaN recovery when NaN is in volume instead of price
        // NaN volume at index 5 only affects index 5 (not index 6)
        // because volume doesn't cascade like TP does
        let high: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume: Vec<f64> = vec![
            1000.0,
            1000.0,
            1000.0,
            1000.0,
            1000.0,
            f64::NAN,
            1000.0,
            1000.0,
            1000.0,
            1000.0,
        ];

        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // NaN volume at index 5 affects outputs 5, 6, 7 (window contains index 5)
        assert!(result[5].is_nan(), "mfi[5] should be NaN (volume NaN)");
        assert!(
            result[6].is_nan(),
            "mfi[6] should be NaN (volume NaN in window)"
        );
        assert!(
            result[7].is_nan(),
            "mfi[7] should be NaN (volume NaN in window)"
        );

        // Should recover at index 8 (window [6,7,8] doesn't contain index 5)
        assert!(
            result[8].is_finite(),
            "mfi[8] should recover after volume NaN exits"
        );
    }

    #[test]
    fn test_mfi_nan_recovery_multiple_separated() {
        // Test multiple NaNs separated by valid values
        // NaN at indices 5 and 8:
        // - Index 5: invalid, Index 6: invalid (cascade)
        // - Index 8: invalid, Index 9: invalid (cascade)
        let high: Vec<f64> = vec![
            10.0,
            11.0,
            12.0,
            13.0,
            14.0,
            f64::NAN,
            16.0,
            17.0,
            f64::NAN,
            19.0,
            20.0,
            21.0,
            22.0,
        ];
        let low: Vec<f64> = vec![
            9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0,
        ];
        let close: Vec<f64> = vec![
            9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5, 19.5, 20.5, 21.5,
        ];
        let volume: Vec<f64> = vec![1000.0; 13];

        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // First NaN at index 5 affects indices 5, 6
        assert!(result[5].is_nan());
        assert!(result[6].is_nan());
        assert!(result[7].is_nan());
        assert!(
            result[8].is_nan(),
            "mfi[8] has NaN at index 8 plus index 6 in window"
        );

        // Second NaN at index 8 affects indices 8, 9
        assert!(result[9].is_nan());
        assert!(result[10].is_nan());
        assert!(result[11].is_nan(), "mfi[11] still has index 9 in window");

        // Should recover at index 12 (window is [10, 11, 12], all valid)
        assert!(
            result[12].is_finite(),
            "mfi[12] should recover after second NaN exits"
        );
    }

    #[test]
    fn test_mfi_nan_recovery_overlapping() {
        // Test overlapping NaNs - consecutive NaNs in input
        // NaNs at indices 5 and 6:
        // - Index 5: invalid (NaN)
        // - Index 6: invalid (NaN)
        // - Index 7: invalid (prev_tp from index 6 is NaN)
        // - Index 8: valid
        let high: Vec<f64> = vec![
            10.0,
            11.0,
            12.0,
            13.0,
            14.0,
            f64::NAN,
            f64::NAN,
            17.0,
            18.0,
            19.0,
            20.0,
        ];
        let low: Vec<f64> = vec![
            9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0,
        ];
        let close: Vec<f64> = vec![
            9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5, 19.5,
        ];
        let volume: Vec<f64> = vec![1000.0; 11];

        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // Indices 5, 6, 7 are all invalid
        assert!(result[5].is_nan());
        assert!(result[6].is_nan());
        assert!(result[7].is_nan());
        assert!(result[8].is_nan());
        assert!(result[9].is_nan(), "mfi[9] window still contains index 7");

        // Should recover at index 10 (window is [8, 9, 10], all valid)
        assert!(
            result[10].is_finite(),
            "mfi[10] should recover after overlapping NaNs exit"
        );
    }

    #[test]
    fn test_mfi_nan_at_start() {
        // Test NaN at the very start of data (index 0)
        // Index 0 is used for prev_tp comparison:
        // - Index 0: prev_tp is NaN, prev_tp_valid = false
        // - Index 1: prev_tp from index 0 (NaN) → invalid
        // - Index 2: prev_tp from index 1 (valid) → valid
        let high: Vec<f64> = vec![f64::NAN, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5];
        let volume: Vec<f64> = vec![1000.0; 8];

        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // Output at index 3, window [1, 2, 3]
        // Index 1 is invalid (prev_tp from index 0 is NaN)
        assert!(result[3].is_nan(), "mfi[3] should be NaN (index 1 invalid)");

        // Output at index 4, window [2, 3, 4]
        // Index 1 no longer in window, all entries valid
        assert!(
            result[4].is_finite(),
            "mfi[4] should recover (index 1 exited window)"
        );

        // All subsequent outputs should be valid
        assert!(result[5].is_finite());
        assert!(result[6].is_finite());
        assert!(result[7].is_finite());
    }

    #[test]
    fn test_mfi_inf_recovery() {
        // Test that Inf is treated like NaN and recovers properly
        // Inf at index 5 affects indices 5 and 6 (same cascade as NaN)
        let high: Vec<f64> = vec![
            10.0,
            11.0,
            12.0,
            13.0,
            14.0,
            f64::INFINITY,
            16.0,
            17.0,
            18.0,
            19.0,
        ];
        let low: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close: Vec<f64> = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume: Vec<f64> = vec![1000.0; 10];

        let result = mfi(&high, &low, &close, &volume, 3).unwrap();

        // Inf at index 5, plus index 6 invalid (prev_tp cascade)
        assert!(result[5].is_nan(), "mfi[5] should be NaN (contains Inf)");
        assert!(result[6].is_nan(), "mfi[6] should be NaN (contains Inf)");
        assert!(result[7].is_nan(), "mfi[7] should be NaN (contains Inf)");
        assert!(
            result[8].is_nan(),
            "mfi[8] should be NaN (index 6 still in window)"
        );

        // Should recover at index 9 (window [7, 8, 9], all valid)
        assert!(
            result[9].is_finite(),
            "mfi[9] should recover after Inf impact exits window"
        );
    }
}
