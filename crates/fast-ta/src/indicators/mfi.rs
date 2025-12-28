//! MFI (Money Flow Index) indicator.
//!
//! The Money Flow Index is a volume-weighted version of RSI that measures
//! buying and selling pressure using both price and volume.
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
//! # Precision Behavior
//!
//! When `PrecisionMode::High` is active and input type is `f32`:
//! - Typical price and raw money flow calculated in `f64`
//! - Rolling positive/negative money flow sums use `f64`
//! - Final MFI computation performed in `f64`
//!
//! **Tolerance**: abs(0.01) when comparing f32 High mode to f64 reference.
//! MFI is bounded 0-100, so absolute tolerance is appropriate.

use crate::error::{Error, Result};
use crate::precision::{current_precision_mode, PrecisionMode};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns true if we should use f64 precision for the given type.
#[inline]
fn use_f64_precision<T: 'static>() -> bool {
    use std::any::TypeId;
    TypeId::of::<T>() == TypeId::of::<f32>() && current_precision_mode() == PrecisionMode::High
}

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

/// Computes MFI and stores results in output slice.
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
pub fn mfi_into<T: SeriesElement + 'static>(
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

    if use_f64_precision::<T>() {
        mfi_core_f64(high, low, close, volume, period, output)
    } else {
        mfi_core_native(high, low, close, volume, period, output)
    }
}

/// Core MFI computation using native precision with O(period) ring buffer.
///
/// This implementation uses ring buffers of size `period` instead of full-length
/// intermediate arrays, reducing memory usage from O(n) to O(period).
fn mfi_core_native<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    // Pre-scan for NaN - much faster than checking per-element in hot loop
    let has_nan = high.iter().take(n).any(|&v| is_invalid(v))
        || low.iter().take(n).any(|&v| is_invalid(v))
        || close.iter().take(n).any(|&v| is_invalid(v))
        || volume.iter().take(n).any(|&v| is_invalid(v));

    if has_nan {
        mfi_core_native_slow(high, low, close, volume, period, output)
    } else {
        mfi_core_native_fast(high, low, close, volume, period, output)
    }
}

/// Fast path: no NaN checks per element - tight loop.
#[inline]
fn mfi_core_native_fast<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();
    let lookback = mfi_lookback(period);
    let inv_three = T::from_f64(1.0 / 3.0)?;
    let hundred = T::from_f64(100.0)?;
    let zero = T::zero();

    // Fill lookback period with NaN
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Ring buffers of size `period` - O(period) space instead of O(n)
    let mut positive_ring = vec![zero; period];
    let mut negative_ring = vec![zero; period];

    // Rolling sums - f64 accumulators for precision
    let mut sum_positive: f64 = 0.0;
    let mut sum_negative: f64 = 0.0;

    // Compute first typical price
    let mut prev_tp = (high[0] + low[0] + close[0]) * inv_three;

    // Process indices 1..=period to fill the initial window
    for i in 1..=period {
        let ring_idx = (i - 1) % period;
        let tp = (high[i] + low[i] + close[i]) * inv_three;
        let raw_mf = tp * volume[i];

        if tp > prev_tp {
            positive_ring[ring_idx] = raw_mf;
            sum_positive += raw_mf.to_f64().unwrap_or(0.0);
        } else if tp < prev_tp {
            negative_ring[ring_idx] = raw_mf;
            sum_negative += raw_mf.to_f64().unwrap_or(0.0);
        }
        prev_tp = tp;
    }

    // Calculate first MFI at index = period
    if sum_negative == 0.0 {
        output[lookback] = hundred;
    } else if sum_positive == 0.0 {
        output[lookback] = zero;
    } else {
        let mfr = sum_positive / sum_negative;
        let mfi_val = 100.0 - (100.0 / (1.0 + mfr));
        output[lookback] = T::from_f64(mfi_val)?;
    }

    // Rolling calculation for remaining values - tight loop, no NaN checks
    for i in (lookback + 1)..n {
        let ring_idx = (i - 1) % period;

        // Remove old value from sums
        sum_positive -= positive_ring[ring_idx].to_f64().unwrap_or(0.0);
        sum_negative -= negative_ring[ring_idx].to_f64().unwrap_or(0.0);

        // Reset ring slot
        positive_ring[ring_idx] = zero;
        negative_ring[ring_idx] = zero;

        // Compute new value
        let tp = (high[i] + low[i] + close[i]) * inv_three;
        let raw_mf = tp * volume[i];

        if tp > prev_tp {
            positive_ring[ring_idx] = raw_mf;
            sum_positive += raw_mf.to_f64().unwrap_or(0.0);
        } else if tp < prev_tp {
            negative_ring[ring_idx] = raw_mf;
            sum_negative += raw_mf.to_f64().unwrap_or(0.0);
        }
        prev_tp = tp;

        // Calculate MFI
        if sum_negative == 0.0 {
            output[i] = hundred;
        } else if sum_positive == 0.0 {
            output[i] = zero;
        } else {
            let mfr = sum_positive / sum_negative;
            let mfi_val = 100.0 - (100.0 / (1.0 + mfr));
            output[i] = T::from_f64(mfi_val)?;
        }
    }

    Ok(())
}

/// Slow path: handles NaN values per element.
fn mfi_core_native_slow<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();
    let lookback = mfi_lookback(period);
    let inv_three = T::from_f64(1.0 / 3.0)?;
    let hundred = T::from_f64(100.0)?;
    let zero = T::zero();

    // Fill lookback period with NaN
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Ring buffers of size `period` - O(period) space instead of O(n)
    let mut positive_ring = vec![zero; period];
    let mut negative_ring = vec![zero; period];
    let mut nan_ring = vec![false; period];

    // Rolling sums - f64 accumulators for precision
    let mut sum_positive: f64 = 0.0;
    let mut sum_negative: f64 = 0.0;
    let mut nan_count = 0usize;

    // Compute first typical price
    let mut prev_tp = (high[0] + low[0] + close[0]) * inv_three;
    let first_invalid =
        is_invalid(high[0]) || is_invalid(low[0]) || is_invalid(close[0]) || is_invalid(volume[0]);

    // Process indices 1..=period to fill the initial window
    // These correspond to ring indices 0..(period-1) for values at indices 1..period
    for i in 1..=period {
        let ring_idx = (i - 1) % period;

        let is_nan = if is_invalid(high[i])
            || is_invalid(low[i])
            || is_invalid(close[i])
            || is_invalid(volume[i])
        {
            prev_tp = (high[i] + low[i] + close[i]) * inv_three;
            true
        } else {
            let tp = (high[i] + low[i] + close[i]) * inv_three;
            let prev_invalid = if i == 1 { first_invalid } else { nan_ring[(i - 2) % period] };

            if is_invalid(prev_tp) || prev_invalid {
                prev_tp = tp;
                true
            } else {
                let raw_mf = tp * volume[i];
                if tp > prev_tp {
                    positive_ring[ring_idx] = raw_mf;
                    sum_positive += raw_mf.to_f64().unwrap_or(0.0);
                } else if tp < prev_tp {
                    negative_ring[ring_idx] = raw_mf;
                    sum_negative += raw_mf.to_f64().unwrap_or(0.0);
                }
                prev_tp = tp;
                false
            }
        };

        nan_ring[ring_idx] = is_nan;
        if is_nan {
            nan_count += 1;
        }
    }

    // Calculate first MFI at index = period
    if nan_count > 0 {
        output[lookback] = T::nan();
    } else if sum_negative == 0.0 {
        output[lookback] = hundred;
    } else if sum_positive == 0.0 {
        output[lookback] = zero;
    } else {
        let mfr = sum_positive / sum_negative;
        let mfi_val = 100.0 - (100.0 / (1.0 + mfr));
        output[lookback] = T::from_f64(mfi_val)?;
    }

    // Rolling calculation for remaining values
    for i in (lookback + 1)..n {
        // Ring index for the new value (which overwrites the oldest)
        let ring_idx = (i - 1) % period;

        // Remove old value from sums
        let old_positive = positive_ring[ring_idx];
        let old_negative = negative_ring[ring_idx];
        sum_positive -= old_positive.to_f64().unwrap_or(0.0);
        sum_negative -= old_negative.to_f64().unwrap_or(0.0);
        if nan_ring[ring_idx] {
            nan_count -= 1;
        }

        // Reset ring slot
        positive_ring[ring_idx] = zero;
        negative_ring[ring_idx] = zero;

        // Compute new value
        let is_nan = if is_invalid(high[i])
            || is_invalid(low[i])
            || is_invalid(close[i])
            || is_invalid(volume[i])
        {
            prev_tp = (high[i] + low[i] + close[i]) * inv_three;
            true
        } else {
            let tp = (high[i] + low[i] + close[i]) * inv_three;
            if is_invalid(prev_tp) {
                prev_tp = tp;
                true
            } else {
                let raw_mf = tp * volume[i];
                if tp > prev_tp {
                    positive_ring[ring_idx] = raw_mf;
                    sum_positive += raw_mf.to_f64().unwrap_or(0.0);
                } else if tp < prev_tp {
                    negative_ring[ring_idx] = raw_mf;
                    sum_negative += raw_mf.to_f64().unwrap_or(0.0);
                }
                prev_tp = tp;
                false
            }
        };

        nan_ring[ring_idx] = is_nan;
        if is_nan {
            nan_count += 1;
        }

        // Calculate MFI
        if nan_count > 0 {
            output[i] = T::nan();
        } else if sum_negative == 0.0 {
            output[i] = hundred;
        } else if sum_positive == 0.0 {
            output[i] = zero;
        } else {
            let mfr = sum_positive / sum_negative;
            let mfi_val = 100.0 - (100.0 / (1.0 + mfr));
            output[i] = T::from_f64(mfi_val)?;
        }
    }

    Ok(())
}

/// Core MFI computation using f64 precision with O(period) ring buffer.
///
/// This implementation uses ring buffers of size `period` instead of full-length
/// intermediate arrays, reducing memory usage from O(n) to O(period).
/// All computations are performed in f64 for improved precision with f32 inputs.
fn mfi_core_f64<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    // Pre-scan for NaN - much faster than checking per-element in hot loop
    let has_nan = high.iter().take(n).any(|&v| is_invalid(v))
        || low.iter().take(n).any(|&v| is_invalid(v))
        || close.iter().take(n).any(|&v| is_invalid(v))
        || volume.iter().take(n).any(|&v| is_invalid(v));

    if has_nan {
        mfi_core_f64_slow(high, low, close, volume, period, output)
    } else {
        mfi_core_f64_fast(high, low, close, volume, period, output)
    }
}

/// Fast path for f64 precision: no NaN checks per element.
#[inline]
fn mfi_core_f64_fast<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();
    let lookback = mfi_lookback(period);
    let inv_three = 1.0_f64 / 3.0;

    // Fill lookback period with NaN
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Ring buffers of size `period` - O(period) space instead of O(n)
    let mut positive_ring = vec![0.0_f64; period];
    let mut negative_ring = vec![0.0_f64; period];

    // Rolling sums - f64 accumulators
    let mut sum_positive: f64 = 0.0;
    let mut sum_negative: f64 = 0.0;

    // Compute first typical price in f64
    let h0 = high[0].to_f64().unwrap_or(0.0);
    let l0 = low[0].to_f64().unwrap_or(0.0);
    let c0 = close[0].to_f64().unwrap_or(0.0);
    let mut prev_tp = (h0 + l0 + c0) * inv_three;

    // Process indices 1..=period to fill the initial window
    for i in 1..=period {
        let ring_idx = (i - 1) % period;
        let h = high[i].to_f64().unwrap_or(0.0);
        let l = low[i].to_f64().unwrap_or(0.0);
        let c = close[i].to_f64().unwrap_or(0.0);
        let v = volume[i].to_f64().unwrap_or(0.0);
        let tp = (h + l + c) * inv_three;
        let raw_mf = tp * v;

        if tp > prev_tp {
            positive_ring[ring_idx] = raw_mf;
            sum_positive += raw_mf;
        } else if tp < prev_tp {
            negative_ring[ring_idx] = raw_mf;
            sum_negative += raw_mf;
        }
        prev_tp = tp;
    }

    // Calculate first MFI at index = period
    if sum_negative == 0.0 {
        output[lookback] = T::from_f64(100.0)?;
    } else if sum_positive == 0.0 {
        output[lookback] = T::from_f64(0.0)?;
    } else {
        let mfr = sum_positive / sum_negative;
        let mfi_val = 100.0 - (100.0 / (1.0 + mfr));
        output[lookback] = T::from_f64(mfi_val)?;
    }

    // Rolling calculation for remaining values - tight loop, no NaN checks
    for i in (lookback + 1)..n {
        let ring_idx = (i - 1) % period;

        // Remove old value from sums
        sum_positive -= positive_ring[ring_idx];
        sum_negative -= negative_ring[ring_idx];

        // Reset ring slot
        positive_ring[ring_idx] = 0.0;
        negative_ring[ring_idx] = 0.0;

        // Compute new value
        let h = high[i].to_f64().unwrap_or(0.0);
        let l = low[i].to_f64().unwrap_or(0.0);
        let c = close[i].to_f64().unwrap_or(0.0);
        let v = volume[i].to_f64().unwrap_or(0.0);
        let tp = (h + l + c) * inv_three;
        let raw_mf = tp * v;

        if tp > prev_tp {
            positive_ring[ring_idx] = raw_mf;
            sum_positive += raw_mf;
        } else if tp < prev_tp {
            negative_ring[ring_idx] = raw_mf;
            sum_negative += raw_mf;
        }
        prev_tp = tp;

        // Calculate MFI
        if sum_negative == 0.0 {
            output[i] = T::from_f64(100.0)?;
        } else if sum_positive == 0.0 {
            output[i] = T::from_f64(0.0)?;
        } else {
            let mfr = sum_positive / sum_negative;
            let mfi_val = 100.0 - (100.0 / (1.0 + mfr));
            output[i] = T::from_f64(mfi_val)?;
        }
    }

    Ok(())
}

/// Slow path for f64 precision: handles NaN values per element.
fn mfi_core_f64_slow<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();
    let lookback = mfi_lookback(period);
    let inv_three = 1.0_f64 / 3.0;

    // Fill lookback period with NaN
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Ring buffers of size `period` - O(period) space instead of O(n)
    let mut positive_ring = vec![0.0_f64; period];
    let mut negative_ring = vec![0.0_f64; period];
    let mut nan_ring = vec![false; period];

    // Rolling sums - f64 accumulators
    let mut sum_positive: f64 = 0.0;
    let mut sum_negative: f64 = 0.0;
    let mut nan_count = 0usize;

    // Compute first typical price
    let h0 = high[0].to_f64().unwrap_or(0.0);
    let l0 = low[0].to_f64().unwrap_or(0.0);
    let c0 = close[0].to_f64().unwrap_or(0.0);
    let mut prev_tp = (h0 + l0 + c0) * inv_three;
    let first_invalid =
        is_invalid(high[0]) || is_invalid(low[0]) || is_invalid(close[0]) || is_invalid(volume[0]);

    // Process indices 1..=period to fill the initial window
    for i in 1..=period {
        let ring_idx = (i - 1) % period;

        let is_nan = if is_invalid(high[i])
            || is_invalid(low[i])
            || is_invalid(close[i])
            || is_invalid(volume[i])
        {
            let h = high[i].to_f64().unwrap_or(0.0);
            let l = low[i].to_f64().unwrap_or(0.0);
            let c = close[i].to_f64().unwrap_or(0.0);
            prev_tp = (h + l + c) * inv_three;
            true
        } else {
            let h = high[i].to_f64().unwrap_or(0.0);
            let l = low[i].to_f64().unwrap_or(0.0);
            let c = close[i].to_f64().unwrap_or(0.0);
            let v = volume[i].to_f64().unwrap_or(0.0);
            let tp = (h + l + c) * inv_three;
            let prev_invalid = if i == 1 { first_invalid } else { nan_ring[(i - 2) % period] };

            if is_invalid(prev_tp) || prev_invalid {
                prev_tp = tp;
                true
            } else {
                let raw_mf = tp * v;
                if tp > prev_tp {
                    positive_ring[ring_idx] = raw_mf;
                    sum_positive += raw_mf;
                } else if tp < prev_tp {
                    negative_ring[ring_idx] = raw_mf;
                    sum_negative += raw_mf;
                }
                prev_tp = tp;
                false
            }
        };

        nan_ring[ring_idx] = is_nan;
        if is_nan {
            nan_count += 1;
        }
    }

    // Calculate first MFI at index = period
    if nan_count > 0 {
        output[lookback] = T::nan();
    } else if sum_negative == 0.0 {
        output[lookback] = T::from_f64(100.0)?;
    } else if sum_positive == 0.0 {
        output[lookback] = T::from_f64(0.0)?;
    } else {
        let mfr = sum_positive / sum_negative;
        let mfi_val = 100.0 - (100.0 / (1.0 + mfr));
        output[lookback] = T::from_f64(mfi_val)?;
    }

    // Rolling calculation for remaining values
    for i in (lookback + 1)..n {
        // Ring index for the new value (which overwrites the oldest)
        let ring_idx = (i - 1) % period;

        // Remove old value from sums
        let old_positive = positive_ring[ring_idx];
        let old_negative = negative_ring[ring_idx];
        sum_positive -= old_positive;
        sum_negative -= old_negative;
        if nan_ring[ring_idx] {
            nan_count -= 1;
        }

        // Reset ring slot
        positive_ring[ring_idx] = 0.0;
        negative_ring[ring_idx] = 0.0;

        // Compute new value
        let is_nan = if is_invalid(high[i])
            || is_invalid(low[i])
            || is_invalid(close[i])
            || is_invalid(volume[i])
        {
            let h = high[i].to_f64().unwrap_or(0.0);
            let l = low[i].to_f64().unwrap_or(0.0);
            let c = close[i].to_f64().unwrap_or(0.0);
            prev_tp = (h + l + c) * inv_three;
            true
        } else {
            let h = high[i].to_f64().unwrap_or(0.0);
            let l = low[i].to_f64().unwrap_or(0.0);
            let c = close[i].to_f64().unwrap_or(0.0);
            let v = volume[i].to_f64().unwrap_or(0.0);
            let tp = (h + l + c) * inv_three;
            if is_invalid(prev_tp) {
                prev_tp = tp;
                true
            } else {
                let raw_mf = tp * v;
                if tp > prev_tp {
                    positive_ring[ring_idx] = raw_mf;
                    sum_positive += raw_mf;
                } else if tp < prev_tp {
                    negative_ring[ring_idx] = raw_mf;
                    sum_negative += raw_mf;
                }
                prev_tp = tp;
                false
            }
        };

        nan_ring[ring_idx] = is_nan;
        if is_nan {
            nan_count += 1;
        }

        // Calculate MFI
        if nan_count > 0 {
            output[i] = T::nan();
        } else if sum_negative == 0.0 {
            output[i] = T::from_f64(100.0)?;
        } else if sum_positive == 0.0 {
            output[i] = T::from_f64(0.0)?;
        } else {
            let mfr = sum_positive / sum_negative;
            let mfi_val = 100.0 - (100.0 / (1.0 + mfr));
            output[i] = T::from_f64(mfi_val)?;
        }
    }

    Ok(())
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
/// use fast_ta::indicators::mfi;
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
    let mut output = vec![T::zero(); high.len()];
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
}