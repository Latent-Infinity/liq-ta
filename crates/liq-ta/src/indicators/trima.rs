//! Triangular Moving Average (TRIMA) indicator.
//!
//! TRIMA is a double-smoothed moving average that applies heavier weighting to
//! the middle of the price series. It's computed as an SMA of an SMA.
//!
//! # Formula
//!
//! For odd period n:
//! - SMA1 period = (n+1)/2
//! - SMA2 period = (n+1)/2
//!
//! For even period n:
//! - SMA1 period = n/2 + 1
//! - SMA2 period = n/2
//!
//! TRIMA = SMA(SMA(data, `SMA1_period`), `SMA2_period`)
//!
//! # Lookback
//!
//! The lookback period is `period - 1`.

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Computes the lookback period for TRIMA.
///
/// The lookback is `period - 1`, representing the number of data points
/// needed before the first valid TRIMA value can be calculated.
///
/// # Arguments
///
/// * `period` - The TRIMA period
///
/// # Returns
///
/// The lookback period (period - 1)
#[inline]
#[must_use]
pub const fn trima_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

/// Returns the minimum input length required for TRIMA calculation.
///
/// This is the lookback period plus 1.
///
/// # Arguments
///
/// * `period` - The TRIMA period
#[inline]
#[must_use]
pub const fn trima_min_len(period: usize) -> usize {
    if period == 0 { 1 } else { period }
}

/// Computes Triangular Moving Average (TRIMA) and stores results in the provided output slice.
///
/// TRIMA is a double-smoothed moving average that gives more weight to the middle
/// of the data range, resulting in a smoother line than SMA.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The TRIMA period (must be >= 1)
/// * `output` - Pre-allocated output slice (must have length >= `data.len()`)
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(Error)` if period is invalid or data insufficient
///
/// # NaN Handling
///
/// The first `period - 1` elements of the output will be NaN.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn trima_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    // Validate inputs
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            indicator: "trima",
            required: period,
            actual: data.len(),
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            indicator: "trima",
            required: data.len(),
            actual: output.len(),
        });
    }

    // For period 1, TRIMA equals the input
    if period == 1 {
        output[..data.len()].copy_from_slice(data);
        return Ok(());
    }

    let lookback = trima_lookback(period);

    // Fill lookback period with NaN
    output[..lookback].fill(T::nan());

    // No pre-scan: dispatch directly to specialized paths
    use std::any::TypeId;
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let data_f64: &[f64] = unsafe { std::mem::transmute(data) };
        let output_f64: &mut [f64] = unsafe { std::mem::transmute(output) };
        return trima_into_fast_f64_inline_nan(data_f64, period, output_f64, lookback);
    }

    if TypeId::of::<T>() == TypeId::of::<f32>() {
        let data_f32: &[f32] = unsafe { std::mem::transmute(data) };
        let output_f32: &mut [f32] = unsafe { std::mem::transmute(output) };
        return trima_into_fast_f32_inline_nan(data_f32, period, output_f32, lookback);
    }

    // Generic fallback: use inline NaN handling (no pre-scan needed)
    trima_into_generic_inline_nan(data, period, output, lookback)
}

/// f64-specialized fast path with inline NaN handling - no pre-scan needed.
/// Tracks invalid_count in rolling window and sanitizes inputs to prevent NaN propagation.
#[inline]
fn trima_into_fast_f64_inline_nan(
    data: &[f64],
    period: usize,
    output: &mut [f64],
    lookback: usize,
) -> Result<()> {
    let n = data.len();

    if period % 2 == 1 {
        // Odd period logic
        let i = period >> 1;
        let factor = 1.0 / ((i + 1) * (i + 1)) as f64;

        let mut trailing_idx = 0usize;
        let mut middle_idx = trailing_idx + i;
        let mut today_idx = middle_idx + i;

        let mut numerator = 0.0_f64;
        let mut numerator_sub = 0.0_f64;
        let mut invalid_count = 0usize;

        // Initialize - sanitize invalids to 0.0
        for j in (trailing_idx..=middle_idx).rev() {
            let val = data[j];
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_sub += sane_val;
            numerator += numerator_sub;
        }

        let mut numerator_add = 0.0_f64;
        for j in (middle_idx + 1)..=today_idx {
            let val = data[j];
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_add += sane_val;
            numerator += numerator_add;
        }

        // Track trailing value for window updates
        let trailing_val = data[trailing_idx];
        let trailing_invalid = !trailing_val.is_finite();
        let temp_real_sane = if trailing_invalid { 0.0 } else { trailing_val };

        // First output
        output[lookback] = if invalid_count > 0 {
            f64::NAN
        } else {
            numerator * factor
        };

        trailing_idx += 1;
        middle_idx += 1;
        today_idx += 1;

        // Main loop with inline invalid tracking
        let mut temp_sane = temp_real_sane;
        let mut temp_was_invalid = trailing_invalid;

        while today_idx < n {
            // Update invalid count for exiting element
            if temp_was_invalid {
                invalid_count -= 1;
            }

            // Step 1: Update numeratorSub
            numerator -= numerator_sub;
            numerator_sub -= temp_sane;

            let middle_val = data[middle_idx];
            let middle_invalid = !middle_val.is_finite();
            let middle_sane = if middle_invalid { 0.0 } else { middle_val };
            numerator_sub += middle_sane;

            // Step 2: Update numeratorAdd
            numerator += numerator_add;
            numerator_add -= middle_sane;

            let today_val = data[today_idx];
            let today_invalid = !today_val.is_finite();
            let today_sane = if today_invalid { 0.0 } else { today_val };
            if today_invalid {
                invalid_count += 1;
            }
            numerator_add += today_sane;
            numerator += today_sane;

            // Prepare next iteration's trailing value
            let next_trailing_val = data[trailing_idx];
            temp_was_invalid = !next_trailing_val.is_finite();
            temp_sane = if temp_was_invalid {
                0.0
            } else {
                next_trailing_val
            };

            // Output
            output[today_idx] = if invalid_count > 0 {
                f64::NAN
            } else {
                numerator * factor
            };

            trailing_idx += 1;
            middle_idx += 1;
            today_idx += 1;
        }
    } else {
        // Even period logic
        let i = period >> 1;
        let factor = 1.0 / (i * (i + 1)) as f64;

        let mut trailing_idx = 0usize;
        let mut middle_idx = trailing_idx + i - 1;
        let mut today_idx = middle_idx + i;

        let mut numerator = 0.0_f64;
        let mut numerator_sub = 0.0_f64;
        let mut invalid_count = 0usize;

        // Initialize
        for j in (trailing_idx..=middle_idx).rev() {
            let val = data[j];
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_sub += sane_val;
            numerator += numerator_sub;
        }

        let mut numerator_add = 0.0_f64;
        for j in (middle_idx + 1)..=today_idx {
            let val = data[j];
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_add += sane_val;
            numerator += numerator_add;
        }

        // Track trailing value
        let trailing_val = data[trailing_idx];
        let trailing_invalid = !trailing_val.is_finite();
        let temp_real_sane = if trailing_invalid { 0.0 } else { trailing_val };

        // First output
        output[lookback] = if invalid_count > 0 {
            f64::NAN
        } else {
            numerator * factor
        };

        trailing_idx += 1;
        middle_idx += 1;
        today_idx += 1;

        // Main loop
        let mut temp_sane = temp_real_sane;
        let mut temp_was_invalid = trailing_invalid;

        while today_idx < n {
            // Update invalid count for exiting element
            if temp_was_invalid {
                invalid_count -= 1;
            }

            // Step 1: Update numeratorSub
            numerator -= numerator_sub;
            numerator_sub -= temp_sane;

            let middle_val = data[middle_idx];
            let middle_invalid = !middle_val.is_finite();
            let middle_sane = if middle_invalid { 0.0 } else { middle_val };
            numerator_sub += middle_sane;

            // Step 2: Update numeratorAdd (even period differs)
            numerator_add -= middle_sane;
            numerator += numerator_add;

            let today_val = data[today_idx];
            let today_invalid = !today_val.is_finite();
            let today_sane = if today_invalid { 0.0 } else { today_val };
            if today_invalid {
                invalid_count += 1;
            }
            numerator_add += today_sane;
            numerator += today_sane;

            // Prepare next iteration's trailing value
            let next_trailing_val = data[trailing_idx];
            temp_was_invalid = !next_trailing_val.is_finite();
            temp_sane = if temp_was_invalid {
                0.0
            } else {
                next_trailing_val
            };

            // Output
            output[today_idx] = if invalid_count > 0 {
                f64::NAN
            } else {
                numerator * factor
            };

            trailing_idx += 1;
            middle_idx += 1;
            today_idx += 1;
        }
    }

    Ok(())
}

/// f32-specialized fast path with inline NaN handling - uses f64 accumulators, no trait overhead.
#[inline]
fn trima_into_fast_f32_inline_nan(
    data: &[f32],
    period: usize,
    output: &mut [f32],
    lookback: usize,
) -> Result<()> {
    let n = data.len();

    if period % 2 == 1 {
        // Odd period logic with f64 accumulators
        let i = period >> 1;
        let factor = 1.0 / ((i + 1) * (i + 1)) as f64;

        let mut trailing_idx = 0usize;
        let mut middle_idx = trailing_idx + i;
        let mut today_idx = middle_idx + i;

        let mut numerator = 0.0_f64;
        let mut numerator_sub = 0.0_f64;
        let mut invalid_count = 0usize;

        // Initialize
        for j in (trailing_idx..=middle_idx).rev() {
            let val = data[j] as f64;
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_sub += sane_val;
            numerator += numerator_sub;
        }

        let mut numerator_add = 0.0_f64;
        for j in (middle_idx + 1)..=today_idx {
            let val = data[j] as f64;
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_add += sane_val;
            numerator += numerator_add;
        }

        let trailing_val = data[trailing_idx] as f64;
        let trailing_invalid = !trailing_val.is_finite();
        let temp_real_sane = if trailing_invalid { 0.0 } else { trailing_val };

        output[lookback] = if invalid_count > 0 {
            f32::NAN
        } else {
            (numerator * factor) as f32
        };

        trailing_idx += 1;
        middle_idx += 1;
        today_idx += 1;

        let mut temp_sane = temp_real_sane;
        let mut temp_was_invalid = trailing_invalid;

        while today_idx < n {
            if temp_was_invalid {
                invalid_count -= 1;
            }

            numerator -= numerator_sub;
            numerator_sub -= temp_sane;

            let middle_val = data[middle_idx] as f64;
            let middle_invalid = !middle_val.is_finite();
            let middle_sane = if middle_invalid { 0.0 } else { middle_val };
            numerator_sub += middle_sane;

            numerator += numerator_add;
            numerator_add -= middle_sane;

            let today_val = data[today_idx] as f64;
            let today_invalid = !today_val.is_finite();
            let today_sane = if today_invalid { 0.0 } else { today_val };
            if today_invalid {
                invalid_count += 1;
            }
            numerator_add += today_sane;
            numerator += today_sane;

            let next_trailing_val = data[trailing_idx] as f64;
            temp_was_invalid = !next_trailing_val.is_finite();
            temp_sane = if temp_was_invalid {
                0.0
            } else {
                next_trailing_val
            };

            output[today_idx] = if invalid_count > 0 {
                f32::NAN
            } else {
                (numerator * factor) as f32
            };

            trailing_idx += 1;
            middle_idx += 1;
            today_idx += 1;
        }
    } else {
        // Even period logic with f64 accumulators
        let i = period >> 1;
        let factor = 1.0 / (i * (i + 1)) as f64;

        let mut trailing_idx = 0usize;
        let mut middle_idx = trailing_idx + i - 1;
        let mut today_idx = middle_idx + i;

        let mut numerator = 0.0_f64;
        let mut numerator_sub = 0.0_f64;
        let mut invalid_count = 0usize;

        for j in (trailing_idx..=middle_idx).rev() {
            let val = data[j] as f64;
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_sub += sane_val;
            numerator += numerator_sub;
        }

        let mut numerator_add = 0.0_f64;
        for j in (middle_idx + 1)..=today_idx {
            let val = data[j] as f64;
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_add += sane_val;
            numerator += numerator_add;
        }

        let trailing_val = data[trailing_idx] as f64;
        let trailing_invalid = !trailing_val.is_finite();
        let temp_real_sane = if trailing_invalid { 0.0 } else { trailing_val };

        output[lookback] = if invalid_count > 0 {
            f32::NAN
        } else {
            (numerator * factor) as f32
        };

        trailing_idx += 1;
        middle_idx += 1;
        today_idx += 1;

        let mut temp_sane = temp_real_sane;
        let mut temp_was_invalid = trailing_invalid;

        while today_idx < n {
            if temp_was_invalid {
                invalid_count -= 1;
            }

            numerator -= numerator_sub;
            numerator_sub -= temp_sane;

            let middle_val = data[middle_idx] as f64;
            let middle_invalid = !middle_val.is_finite();
            let middle_sane = if middle_invalid { 0.0 } else { middle_val };
            numerator_sub += middle_sane;

            numerator_add -= middle_sane;
            numerator += numerator_add;

            let today_val = data[today_idx] as f64;
            let today_invalid = !today_val.is_finite();
            let today_sane = if today_invalid { 0.0 } else { today_val };
            if today_invalid {
                invalid_count += 1;
            }
            numerator_add += today_sane;
            numerator += today_sane;

            let next_trailing_val = data[trailing_idx] as f64;
            temp_was_invalid = !next_trailing_val.is_finite();
            temp_sane = if temp_was_invalid {
                0.0
            } else {
                next_trailing_val
            };

            output[today_idx] = if invalid_count > 0 {
                f32::NAN
            } else {
                (numerator * factor) as f32
            };

            trailing_idx += 1;
            middle_idx += 1;
            today_idx += 1;
        }
    }

    Ok(())
}

/// Generic inline NaN handling - uses f64 accumulators for precision, no pre-scan needed.
/// Replaces the slow SMA-of-SMA path with the same inline invalid tracking as f64/f32.
#[inline]
fn trima_into_generic_inline_nan<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
    lookback: usize,
) -> Result<()> {
    let n = data.len();

    if period % 2 == 1 {
        // Odd period logic
        let i = period >> 1;
        let factor = 1.0 / ((i + 1) * (i + 1)) as f64;

        let trailing_idx = 0usize;
        let mut middle_idx = trailing_idx + i;
        let mut today_idx = middle_idx + i;

        let mut numerator = 0.0_f64;
        let mut numerator_sub = 0.0_f64;
        let mut invalid_count = 0usize;

        // Initialize - sanitize invalids to 0.0
        for j in (trailing_idx..=middle_idx).rev() {
            let val = data[j].to_f64().unwrap_or(0.0);
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_sub += sane_val;
            numerator += numerator_sub;
        }

        let mut numerator_add = 0.0_f64;
        for j in (middle_idx + 1)..=today_idx {
            let val = data[j].to_f64().unwrap_or(0.0);
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_add += sane_val;
            numerator += numerator_add;
        }

        // Track trailing value for window updates
        let trailing_val = data[trailing_idx].to_f64().unwrap_or(0.0);
        let trailing_invalid = !trailing_val.is_finite();
        let temp_real_sane = if trailing_invalid { 0.0 } else { trailing_val };

        // First output
        output[lookback] = if invalid_count > 0 {
            T::nan()
        } else {
            T::from_f64(numerator * factor)?
        };

        middle_idx += 1;
        today_idx += 1;

        // Main loop with inline invalid tracking
        let mut temp_sane = temp_real_sane;
        let mut temp_was_invalid = trailing_invalid;

        while today_idx < n {
            // Update invalid count for exiting element
            if temp_was_invalid {
                invalid_count -= 1;
            }

            // Step 1: Update numeratorSub
            let middle_val = data[middle_idx].to_f64().unwrap_or(0.0);
            let middle_invalid = !middle_val.is_finite();
            let middle_sane = if middle_invalid { 0.0 } else { middle_val };

            numerator_sub = numerator_sub - temp_sane + middle_sane;

            // Step 2: Update numeratorAdd
            let today_val = data[today_idx].to_f64().unwrap_or(0.0);
            let today_invalid = !today_val.is_finite();
            let today_sane = if today_invalid { 0.0 } else { today_val };

            if today_invalid {
                invalid_count += 1;
            }

            numerator_add = numerator_add + today_sane;

            // Step 3: Update numerator and output
            numerator = numerator - temp_sane + numerator_add;

            output[today_idx] = if invalid_count > 0 {
                T::nan()
            } else {
                T::from_f64(numerator * factor)?
            };

            // Prepare for next iteration
            temp_sane = middle_sane;
            temp_was_invalid = middle_invalid;

            middle_idx += 1;
            today_idx += 1;
        }
    } else {
        // Even period logic
        let i = period >> 1;
        let factor = 1.0 / (i * (i + 1)) as f64;

        let trailing_idx = 1usize;
        let mut middle_idx = trailing_idx + i - 1;
        let mut today_idx = middle_idx + i;

        let mut numerator = 0.0_f64;
        let mut numerator_sub = 0.0_f64;
        let mut invalid_count = 0usize;

        // Initialize lower half
        for j in (trailing_idx..=middle_idx).rev() {
            let val = data[j].to_f64().unwrap_or(0.0);
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_sub += sane_val;
            numerator += numerator_sub;
        }

        // Initialize upper half
        let mut numerator_add = 0.0_f64;
        for j in (middle_idx + 1)..=today_idx {
            let val = data[j].to_f64().unwrap_or(0.0);
            let is_bad = !val.is_finite();
            let sane_val = if is_bad { 0.0 } else { val };
            if is_bad {
                invalid_count += 1;
            }
            numerator_add += sane_val;
            numerator += numerator_add;
        }

        // Track trailing value
        let trailing_val = data[trailing_idx].to_f64().unwrap_or(0.0);
        let trailing_invalid = !trailing_val.is_finite();
        let temp_real_sane = if trailing_invalid { 0.0 } else { trailing_val };

        // First output
        output[lookback] = if invalid_count > 0 {
            T::nan()
        } else {
            T::from_f64(numerator * factor)?
        };

        middle_idx += 1;
        today_idx += 1;

        // Main loop
        let mut temp_sane = temp_real_sane;
        let mut temp_was_invalid = trailing_invalid;

        while today_idx < n {
            if temp_was_invalid {
                invalid_count -= 1;
            }

            let middle_val = data[middle_idx].to_f64().unwrap_or(0.0);
            let middle_invalid = !middle_val.is_finite();
            let middle_sane = if middle_invalid { 0.0 } else { middle_val };

            numerator_sub = numerator_sub - temp_sane + middle_sane;

            let today_val = data[today_idx].to_f64().unwrap_or(0.0);
            let today_invalid = !today_val.is_finite();
            let today_sane = if today_invalid { 0.0 } else { today_val };

            if today_invalid {
                invalid_count += 1;
            }

            numerator_add = numerator_add + today_sane;
            numerator = numerator - temp_sane + numerator_add;

            output[today_idx] = if invalid_count > 0 {
                T::nan()
            } else {
                T::from_f64(numerator * factor)?
            };

            temp_sane = middle_sane;
            temp_was_invalid = middle_invalid;

            middle_idx += 1;
            today_idx += 1;
        }
    }

    Ok(())
}

/// Computes Triangular Moving Average (TRIMA).
///
/// TRIMA is a double-smoothed moving average that gives more weight to the middle
/// of the data range, resulting in a smoother line than SMA.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - The TRIMA period (must be >= 1)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of TRIMA values with same length as input
/// * `Err(Error)` if period is invalid or data insufficient
///
/// # NaN Handling
///
/// The first `period - 1` elements will be NaN.
///
/// # Example
///
/// ```
/// use liq_ta::indicators::trima;
///
/// let prices = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
/// let result = trima(&prices, 5).unwrap();
/// // First 4 values are NaN, then TRIMA values
/// assert!(result[4].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn trima<T: SeriesElement + 'static>(data: &[T], period: usize) -> Result<Vec<T>> {
    use std::any::TypeId;

    // Wrapper optimization: uninitialized allocation for f64/f32
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let data_f64: &[f64] = unsafe { std::mem::transmute(data) };
        let mut output: Vec<f64> = Vec::with_capacity(data.len());
        unsafe {
            output.set_len(data.len());
        }
        trima_into(data_f64, period, &mut output)?;
        Ok(unsafe { std::mem::transmute(output) })
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let data_f32: &[f32] = unsafe { std::mem::transmute(data) };
        let mut output: Vec<f32> = Vec::with_capacity(data.len());
        unsafe {
            output.set_len(data.len());
        }
        trima_into(data_f32, period, &mut output)?;
        Ok(unsafe { std::mem::transmute(output) })
    } else {
        // Generic fallback with safe initialization
        let mut output = vec![T::nan(); data.len()];
        trima_into(data, period, &mut output)?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;
    use num_traits::Float;

    fn approx_eq<T: Float>(a: T, b: T, epsilon: T) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        (a - b).abs() < epsilon
    }

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_trima_lookback() {
        assert_eq!(trima_lookback(1), 0);
        assert_eq!(trima_lookback(2), 1);
        assert_eq!(trima_lookback(5), 4);
        assert_eq!(trima_lookback(10), 9);
        assert_eq!(trima_lookback(0), 0);
    }

    #[test]
    fn test_trima_min_len() {
        assert_eq!(trima_min_len(1), 1);
        assert_eq!(trima_min_len(2), 2);
        assert_eq!(trima_min_len(5), 5);
        assert_eq!(trima_min_len(10), 10);
    }

    #[test]
    fn test_trima_empty_input() {
        let data: Vec<f64> = vec![];
        let result = trima(&data, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_trima_zero_period() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = trima(&data, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_trima_insufficient_data() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0];
        let result = trima(&data, 5);
        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                indicator: "trima",
                required: 5,
                actual: 3,
            })
        ));
    }

    #[test]
    fn test_trima_period_one() {
        let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = trima(&data, 1).unwrap();
        // TRIMA with period 1 equals input
        assert_eq!(result.len(), data.len());
        for i in 0..data.len() {
            assert!(approx_eq(result[i], data[i], EPSILON));
        }
    }

    #[test]
    fn test_trima_output_length_equals_input_length() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = trima(&data, 5).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_trima_nan_count() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;
        let result = trima(&data, period).unwrap();

        // Count NaN values - should be period - 1 = 4
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, period - 1);
    }

    #[test]
    fn test_trima_valid_count() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;
        let result = trima(&data, period).unwrap();

        // Valid values start at index period - 1
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        assert_eq!(valid_count, data.len() - (period - 1));
    }

    #[test]
    fn test_trima_basic_odd_period() {
        // Period 5 (odd): SMA1_period = 3, SMA2_period = 3
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let result = trima(&data, 5).unwrap();

        // First 4 values should be NaN
        for i in 0..4 {
            assert!(result[i].is_nan());
        }

        // For odd period 5:
        // SMA1 of data with period 3: [2, 3, 4, 5, 6]
        // SMA2 of SMA1 with period 3: [3, 4, 5]
        // These go at indices 4, 5, 6
        assert!(result[4].is_finite());
        assert!(result[5].is_finite());
        assert!(result[6].is_finite());

        // Expected: SMA of [2,3,4] = 3, SMA of [3,4,5] = 4, SMA of [4,5,6] = 5
        assert!(approx_eq(result[4], 3.0, EPSILON));
        assert!(approx_eq(result[5], 4.0, EPSILON));
        assert!(approx_eq(result[6], 5.0, EPSILON));
    }

    #[test]
    fn test_trima_basic_even_period() {
        // Period 4 (even): SMA1_period = 3, SMA2_period = 2
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let result = trima(&data, 4).unwrap();

        // First 3 values should be NaN
        for i in 0..3 {
            assert!(result[i].is_nan());
        }

        // For even period 4:
        // SMA1 of data with period 3: [2, 3, 4, 5, 6]
        // SMA2 of SMA1 with period 2: [2.5, 3.5, 4.5, 5.5]
        // These go at indices 3, 4, 5, 6
        assert!(result[3].is_finite());
        assert!(result[4].is_finite());

        // Expected: SMA of [2,3] = 2.5, SMA of [3,4] = 3.5, etc.
        assert!(approx_eq(result[3], 2.5, EPSILON));
        assert!(approx_eq(result[4], 3.5, EPSILON));
    }

    #[test]
    fn test_trima_smoother_than_sma() {
        // TRIMA should be smoother than SMA due to double smoothing
        use crate::indicators::sma;

        let data: Vec<f64> = vec![10.0, 12.0, 11.0, 13.0, 12.0, 14.0, 13.0, 15.0, 14.0, 16.0];
        let period = 5;

        let trima_result = trima(&data, period).unwrap();
        let sma_result = sma(&data, period).unwrap();

        // Both should have same number of valid values
        let trima_valid: Vec<f64> = trima_result
            .iter()
            .filter(|x| !x.is_nan())
            .cloned()
            .collect();
        let sma_valid: Vec<f64> = sma_result.iter().filter(|x| !x.is_nan()).cloned().collect();

        // Calculate variance of changes
        let trima_changes: Vec<f64> = trima_valid
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .collect();
        let sma_changes: Vec<f64> = sma_valid.windows(2).map(|w| (w[1] - w[0]).abs()).collect();

        let trima_avg_change: f64 = trima_changes.iter().sum::<f64>() / trima_changes.len() as f64;
        let sma_avg_change: f64 = sma_changes.iter().sum::<f64>() / sma_changes.len() as f64;

        // TRIMA should have smaller average changes (smoother)
        assert!(
            trima_avg_change <= sma_avg_change,
            "TRIMA avg change {} should be <= SMA avg change {}",
            trima_avg_change,
            sma_avg_change
        );
    }

    #[test]
    fn test_trima_period_two() {
        // Period 2 (even): SMA1_period = 2, SMA2_period = 1
        let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = trima(&data, 2).unwrap();

        // First 1 value should be NaN
        assert!(result[0].is_nan());

        // For period 2: SMA1 = [15, 25, 35, 45], SMA2 with period 1 = same
        assert!(approx_eq(result[1], 15.0, EPSILON));
        assert!(approx_eq(result[2], 25.0, EPSILON));
        assert!(approx_eq(result[3], 35.0, EPSILON));
        assert!(approx_eq(result[4], 45.0, EPSILON));
    }

    #[test]
    fn test_trima_period_three() {
        // Period 3 (odd): SMA1_period = 2, SMA2_period = 2
        let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = trima(&data, 3).unwrap();

        // First 2 values should be NaN
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());

        // For period 3: SMA1 of period 2 = [15, 25, 35, 45]
        // SMA2 of period 2 = [20, 30, 40]
        assert!(approx_eq(result[2], 20.0, EPSILON));
        assert!(approx_eq(result[3], 30.0, EPSILON));
        assert!(approx_eq(result[4], 40.0, EPSILON));
    }

    #[test]
    fn test_trima_f32() {
        let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = trima(&data, 5).unwrap();

        assert_eq!(result.len(), data.len());

        // First 4 should be NaN
        for i in 0..4 {
            assert!(result[i].is_nan());
        }

        // Rest should be valid
        for i in 4..10 {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_trima_into_f32() {
        let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut output = vec![0.0_f32; data.len()];
        trima_into(&data, 5, &mut output).unwrap();

        // First 4 should be NaN
        for i in 0..4 {
            assert!(output[i].is_nan());
        }

        // Rest should be valid
        for i in 4..10 {
            assert!(output[i].is_finite());
        }
    }

    #[test]
    fn test_trima_into_insufficient_output() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mut output: Vec<f64> = vec![0.0; 3]; // Too small
        let result = trima_into(&data, 3, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_trima_minimum_length() {
        // Test with exactly the minimum required data
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = trima(&data, 5);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 5);
    }

    #[test]
    fn test_trima_min_len_zero_period_surface() {
        assert_eq!(trima_min_len(0), 1);
    }

    #[test]
    fn test_trima_internal_generic_inline_nan_surface_f64() {
        let data: Vec<f64> = (1..=24).map(|x| x as f64).collect();

        let p_odd = 5;
        let lb_odd = trima_lookback(p_odd);
        let mut gen_odd = vec![f64::NAN; data.len()];
        let mut fast_odd = vec![f64::NAN; data.len()];
        trima_into_generic_inline_nan(&data, p_odd, &mut gen_odd, lb_odd).unwrap();
        trima_into_fast_f64_inline_nan(&data, p_odd, &mut fast_odd, lb_odd).unwrap();
        assert!(gen_odd.iter().take(lb_odd).all(|v| v.is_nan()));
        assert!(fast_odd.iter().take(lb_odd).all(|v| v.is_nan()));
        assert!(gen_odd.iter().skip(lb_odd).all(|v| v.is_finite()));
        assert!(fast_odd.iter().skip(lb_odd).all(|v| v.is_finite()));

        let p_even = 6;
        let lb_even = trima_lookback(p_even);
        let mut gen_even = vec![f64::NAN; data.len()];
        let mut fast_even = vec![f64::NAN; data.len()];
        trima_into_generic_inline_nan(&data, p_even, &mut gen_even, lb_even).unwrap();
        trima_into_fast_f64_inline_nan(&data, p_even, &mut fast_even, lb_even).unwrap();
        assert!(gen_even.iter().take(lb_even).all(|v| v.is_nan()));
        assert!(fast_even.iter().take(lb_even).all(|v| v.is_nan()));
        assert!(gen_even.iter().skip(lb_even).any(|v| v.is_finite()));
        assert!(fast_even.iter().skip(lb_even).any(|v| v.is_finite()));
    }

    #[test]
    fn test_trima_internal_inline_nan_non_finite_windows_f64_f32() {
        let data_f64 = vec![
            1.0_f64,
            2.0,
            3.0,
            f64::NAN,
            5.0,
            f64::INFINITY,
            7.0,
            8.0,
            9.0,
            10.0,
            11.0,
            12.0,
            13.0,
            14.0,
        ];
        let period_f64 = 5;
        let lb_f64 = trima_lookback(period_f64);

        let mut out_generic = vec![f64::NAN; data_f64.len()];
        let mut out_fast = vec![f64::NAN; data_f64.len()];
        trima_into_generic_inline_nan(&data_f64, period_f64, &mut out_generic, lb_f64).unwrap();
        trima_into_fast_f64_inline_nan(&data_f64, period_f64, &mut out_fast, lb_f64).unwrap();
        assert!(out_generic[lb_f64].is_nan());
        assert!(out_fast[lb_f64].is_nan());
        assert!(out_generic.iter().skip(lb_f64 + 6).any(|v| v.is_finite()));

        let data_f32 = vec![
            1.0_f32,
            2.0,
            3.0,
            f32::NAN,
            5.0,
            6.0,
            7.0,
            f32::INFINITY,
            9.0,
            10.0,
            11.0,
            12.0,
            13.0,
            14.0,
        ];
        let period_f32 = 4;
        let lb_f32 = trima_lookback(period_f32);
        let mut out_f32 = vec![f32::NAN; data_f32.len()];
        trima_into_fast_f32_inline_nan(&data_f32, period_f32, &mut out_f32, lb_f32).unwrap();
        assert!(out_f32[lb_f32].is_nan());
        assert!(out_f32.iter().skip(lb_f32 + 6).any(|v| v.is_finite()));
    }
}
