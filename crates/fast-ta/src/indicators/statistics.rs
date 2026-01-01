//! Statistical Functions
//!
//! This module provides rolling statistical functions commonly used in technical analysis.
//!
//! # Indicators
//!
//! - [`var`] - Variance (population)
//! - [`stddev`] - Standard Deviation (population)
//! - [`skew`] - Skewness (third standardized moment)
//! - [`kurt`] - Kurtosis (fourth standardized moment, excess kurtosis)
//! - [`cov`] - Covariance
//! - [`zscore`] - Rolling Z-Score
//! - [`mad`] - Mean Absolute Deviation
//! - [`sem`] - Standard Error of Mean
//! - [`correl`] - Pearson Correlation Coefficient
//! - [`beta`] - Beta coefficient
//! - [`linearreg`] - Linear Regression (predicted value at end of period)
//! - [`linearreg_slope`] - Linear Regression Slope
//! - [`linearreg_intercept`] - Linear Regression Intercept
//! - [`linearreg_angle`] - Linear Regression Angle (in degrees)
//! - [`tsf`] - Time Series Forecast (one period ahead prediction)
//!
//! # Mathematical Conventions
//!
//! - **Population formulas**: Uses ÷n, not ÷(n-1) to match TA-Lib
//! - **Linear regression**: Uses least-squares method over rolling windows

use crate::error::{Error, Result};
use crate::precision::{current_precision_mode, PrecisionMode};
use crate::traits::SeriesElement;

/// Returns true if we should use f64 precision for f32 inputs.
#[inline]
fn use_f64_precision<T: 'static>() -> bool {
    use std::any::TypeId;
    TypeId::of::<T>() == TypeId::of::<f32>() && current_precision_mode() == PrecisionMode::High
}

// =============================================================================
// VAR (Variance)
// =============================================================================

/// Returns the lookback period for VAR.
#[inline]
#[must_use]
pub const fn var_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for VAR.
#[inline]
#[must_use]
pub const fn var_min_len(period: usize) -> usize {
    period
}

/// Computes VAR using Welford's online algorithm (fast path - no NaN handling).
///
/// This function uses Welford's numerically stable algorithm for the initial window,
/// then maintains rolling sums with a shift constant for O(n) computation.
///
/// Uses population variance (÷n, not ÷(n-1)) to match TA-Lib.
///
/// # Algorithm
///
/// For the initial window, uses Welford's online algorithm:
/// - Maintains mean and M2 (sum of squared differences from mean)
/// - Incrementally updates these values as each element is added
///
/// For rolling updates, uses shifted sums for numerical stability:
/// - Subtracts a constant (first value) from all data to keep values small
/// - Maintains sum and sum_sq with O(1) updates per element
/// - Variance = E[X²] - E[X]² (computed on shifted values, same result)
///
/// # Errors
///
/// Returns an error if conversion from usize fails.
#[inline]
fn var_welford_fast<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    let n = data.len();
    let lookback = var_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Handle period=1 edge case: variance is always 0
    if period == 1 {
        for i in 0..n {
            output[i] = T::zero();
        }
        return Ok(());
    }

    // Use Welford's online algorithm for initial window
    // This establishes mean and M2 with optimal numerical stability
    let mut mean = T::zero();
    let mut m2 = T::zero();

    for i in 0..period {
        let count_t = T::from_usize(i + 1)?;
        let delta = data[i] - mean;
        mean = mean + delta / count_t;
        let delta2 = data[i] - mean;
        m2 = m2 + delta * delta2;
    }
    output[lookback] = m2 / period_t;

    // For rolling updates, use shifted sums approach
    // This maintains O(n) complexity with numerical stability
    // Shifting by a constant K: Var(X) = Var(X - K) since variance is shift-invariant
    let shift = data[0];

    // Initialize shifted sums from initial window
    let mut sum = T::zero(); // Σ(x - shift)
    let mut sum_sq = T::zero(); // Σ(x - shift)²

    for i in 0..period {
        let shifted = data[i] - shift;
        sum = sum + shifted;
        sum_sq = sum_sq + shifted * shifted;
    }

    // Rolling updates: O(1) per element
    for i in period..n {
        let old_shifted = data[i - period] - shift;
        let new_shifted = data[i] - shift;

        // Update sums: remove old, add new
        sum = sum - old_shifted + new_shifted;
        sum_sq = sum_sq - old_shifted * old_shifted + new_shifted * new_shifted;

        // Compute variance: Var = E[X²] - E[X]²
        let mean_shifted = sum / period_t;
        output[i] = sum_sq / period_t - mean_shifted * mean_shifted;
    }

    Ok(())
}

/// Computes VAR using Welford's online algorithm (slow path - with NaN handling).
///
/// This function tracks NaN values in the rolling window and outputs NaN when
/// any value in the current window is NaN or invalid.
///
/// Uses population variance (÷n, not ÷(n-1)) to match TA-Lib.
///
/// # Algorithm
///
/// For the initial window, uses Welford's online algorithm with NaN tracking:
/// - Maintains mean and M2 (sum of squared differences from mean)
/// - Tracks nan_count for NaN handling
/// - Outputs NaN if any value in window is NaN
///
/// For rolling updates, uses shifted sums for numerical stability:
/// - Maintains sum and sum_sq with O(1) updates per element
/// - Updates nan_count as values enter/exit the window
///
/// # Errors
///
/// Returns an error if conversion from usize fails.
#[inline]
fn var_welford_slow<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    let n = data.len();
    let lookback = var_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Handle period=1 edge case: variance is always 0 for valid values
    if period == 1 {
        for i in 0..n {
            if data[i].is_nan() {
                output[i] = T::nan();
            } else {
                output[i] = T::zero();
            }
        }
        return Ok(());
    }

    // Use Welford's online algorithm for initial window with NaN tracking
    let mut mean = T::zero();
    let mut m2 = T::zero();
    let mut nan_count = 0usize;
    let mut valid_count = 0usize;

    for i in 0..period {
        if data[i].is_nan() {
            nan_count += 1;
        } else {
            valid_count += 1;
            let count_t = T::from_usize(valid_count)?;
            let delta = data[i] - mean;
            mean = mean + delta / count_t;
            let delta2 = data[i] - mean;
            m2 = m2 + delta * delta2;
        }
    }

    // Output first variance value (or NaN if window contains NaN)
    if nan_count == 0 {
        output[lookback] = m2 / period_t;
    } else {
        output[lookback] = T::nan();
    }

    // For rolling updates, use shifted sums approach with NaN tracking
    // Find first valid value to use as shift constant (use 0 if all NaN)
    let shift = data
        .iter()
        .find(|&&x| !x.is_nan())
        .copied()
        .unwrap_or(T::zero());

    // Initialize shifted sums from initial window (excluding NaN values)
    let mut sum = T::zero(); // Σ(x - shift) for valid values
    let mut sum_sq = T::zero(); // Σ(x - shift)² for valid values

    for i in 0..period {
        if !data[i].is_nan() {
            let shifted = data[i] - shift;
            sum = sum + shifted;
            sum_sq = sum_sq + shifted * shifted;
        }
    }

    // Rolling updates: O(1) per element
    for i in period..n {
        let old_value = data[i - period];
        let new_value = data[i];

        // Update NaN count and sums for new value
        if new_value.is_nan() {
            nan_count += 1;
        } else {
            let new_shifted = new_value - shift;
            sum = sum + new_shifted;
            sum_sq = sum_sq + new_shifted * new_shifted;
        }

        // Update NaN count and sums for old value
        if old_value.is_nan() {
            nan_count = nan_count.saturating_sub(1);
        } else {
            let old_shifted = old_value - shift;
            sum = sum - old_shifted;
            sum_sq = sum_sq - old_shifted * old_shifted;
        }

        // Compute variance if no NaN values in window
        if nan_count == 0 {
            let mean_shifted = sum / period_t;
            output[i] = sum_sq / period_t - mean_shifted * mean_shifted;
        } else {
            output[i] = T::nan();
        }
    }

    Ok(())
}

/// Pre-scans input array to check for any NaN values.
/// Returns true if any NaN is found (requires slow path).
#[inline]
fn var_has_nan<T: SeriesElement>(data: &[T]) -> bool {
    for &value in data {
        if value.is_nan() {
            return true;
        }
    }
    false
}

/// Computes VAR (Variance) and stores results in output buffer.
///
/// This function uses pre-scan optimization to detect NaN values in the input:
/// - If no NaN is found, uses fast path with Welford's algorithm (no validity tracking)
/// - If NaN is found, uses slow path with Welford's algorithm and NaN tracking
///
/// Uses population variance (÷n, not ÷(n-1)) to match TA-Lib.
///
/// # Algorithm
///
/// Both paths use Welford's online algorithm for the initial window, then
/// shifted rolling sums for O(n) updates. This provides:
/// - Numerical stability (especially for near-constant data)
/// - O(n) time complexity
/// - Proper NaN propagation through rolling windows
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn var_into<T: SeriesElement + 'static>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    let n = data.len();
    if n < period {
        return Err(Error::InsufficientData {
            indicator: "var",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "var",
            required: n,
            actual: output.len(),
        });
    }

    // Use f64 precision path for f32 inputs in High precision mode
    if use_f64_precision::<T>() {
        return var_f64_precision(data, period, output);
    }

    // Pre-scan optimization: check for NaN in input data
    // Route to fast path (no NaN tracking) or slow path (with NaN tracking)
    if var_has_nan(data) {
        // Slow path: handles NaN values with nan_count tracking
        var_welford_slow(data, period, output)
    } else {
        // Fast path: no NaN tracking overhead
        var_welford_fast(data, period, output)
    }
}

/// Computes VAR using f64 accumulators for f32 inputs in High precision mode.
///
/// Converts f32 input to f64, performs all calculations in f64, then converts
/// output back to f32. This provides maximum numerical stability for f32 data.
#[inline]
fn var_f64_precision<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    let n = data.len();
    let lookback = var_lookback(period);
    let period_f64 = period as f64;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Handle period=1 edge case: variance is always 0
    if period == 1 {
        for i in 0..n {
            let val = data[i].to_f64().unwrap_or(f64::NAN);
            output[i] = if val.is_nan() {
                T::nan()
            } else {
                T::zero()
            };
        }
        return Ok(());
    }

    // Check for NaN to determine path (only NaN, not infinity - to match native behavior)
    let has_nan = data.iter().any(|x| x.is_nan());

    if has_nan {
        // Slow path with NaN tracking using f64 accumulators
        var_f64_precision_slow(data, period, period_f64, lookback, output)
    } else {
        // Fast path using f64 accumulators
        var_f64_precision_fast(data, period, period_f64, lookback, output)
    }
}

/// Fast f64 precision path (no NaN in data)
#[inline]
fn var_f64_precision_fast<T: SeriesElement>(
    data: &[T],
    period: usize,
    period_f64: f64,
    lookback: usize,
    output: &mut [T],
) -> Result<()> {
    let n = data.len();

    // Use Welford's online algorithm for initial window with f64
    let mut mean: f64 = 0.0;
    let mut m2: f64 = 0.0;

    for i in 0..period {
        let val = data[i].to_f64().unwrap_or(0.0);
        let count = (i + 1) as f64;
        let delta = val - mean;
        mean += delta / count;
        let delta2 = val - mean;
        m2 += delta * delta2;
    }
    output[lookback] = T::from_f64(m2 / period_f64)?;

    // For rolling updates, use shifted sums approach with f64
    let shift = data[0].to_f64().unwrap_or(0.0);

    // Initialize shifted sums from initial window
    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;

    for i in 0..period {
        let shifted = data[i].to_f64().unwrap_or(0.0) - shift;
        sum += shifted;
        sum_sq += shifted * shifted;
    }

    // Rolling updates: O(1) per element
    for i in period..n {
        let old_shifted = data[i - period].to_f64().unwrap_or(0.0) - shift;
        let new_shifted = data[i].to_f64().unwrap_or(0.0) - shift;

        // Update sums: remove old, add new
        sum = sum - old_shifted + new_shifted;
        sum_sq = sum_sq - old_shifted * old_shifted + new_shifted * new_shifted;

        // Compute variance: Var = E[X²] - E[X]²
        let mean_shifted = sum / period_f64;
        let variance = sum_sq / period_f64 - mean_shifted * mean_shifted;
        output[i] = T::from_f64(variance)?;
    }

    Ok(())
}

/// Slow f64 precision path (with NaN tracking)
/// Uses is_nan() to match native f32 path behavior - infinity propagates through arithmetic
#[inline]
fn var_f64_precision_slow<T: SeriesElement>(
    data: &[T],
    period: usize,
    period_f64: f64,
    lookback: usize,
    output: &mut [T],
) -> Result<()> {
    let n = data.len();

    // Track NaN count in rolling window (only NaN, not infinity - to match native)
    let mut nan_count = 0usize;

    // Use Welford's online algorithm for initial window with f64
    let mut mean: f64 = 0.0;
    let mut m2: f64 = 0.0;
    let mut valid_count = 0usize;

    for i in 0..period {
        let val = data[i].to_f64().unwrap_or(f64::NAN);
        if val.is_nan() {
            nan_count += 1;
        } else {
            valid_count += 1;
            let count = valid_count as f64;
            let delta = val - mean;
            mean += delta / count;
            let delta2 = val - mean;
            m2 += delta * delta2;
        }
    }

    // First output value
    if nan_count > 0 {
        output[lookback] = T::nan();
    } else {
        output[lookback] = T::from_f64(m2 / period_f64)?;
    }

    // For rolling updates, use shifted sums approach with f64
    // Use first non-NaN value as shift, or 0 if none
    let shift = data.iter()
        .filter_map(|x| x.to_f64())
        .find(|x| !x.is_nan())
        .unwrap_or(0.0);

    // Initialize shifted sums from initial window (excluding NaN)
    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;

    for i in 0..period {
        let val = data[i].to_f64().unwrap_or(f64::NAN);
        if !val.is_nan() {
            let shifted = val - shift;
            sum += shifted;
            sum_sq += shifted * shifted;
        }
    }

    // Rolling updates with NaN tracking
    for i in period..n {
        let old_val = data[i - period].to_f64().unwrap_or(f64::NAN);
        let new_val = data[i].to_f64().unwrap_or(f64::NAN);

        // Update NaN count (only NaN, infinity is allowed in sums)
        if old_val.is_nan() {
            nan_count -= 1;
        }
        if new_val.is_nan() {
            nan_count += 1;
        }

        // Update sums (only for non-NaN values, infinity is included)
        if !old_val.is_nan() {
            let old_shifted = old_val - shift;
            sum -= old_shifted;
            sum_sq -= old_shifted * old_shifted;
        }
        if !new_val.is_nan() {
            let new_shifted = new_val - shift;
            sum += new_shifted;
            sum_sq += new_shifted * new_shifted;
        }

        // Output NaN if any NaN in window
        if nan_count > 0 {
            output[i] = T::nan();
        } else {
            let mean_shifted = sum / period_f64;
            let variance = sum_sq / period_f64 - mean_shifted * mean_shifted;
            output[i] = T::from_f64(variance)?;
        }
    }

    Ok(())
}

/// Computes VAR (Variance).
///
/// Uses population variance (÷n, not ÷(n-1)) to match TA-Lib.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn var<T: SeriesElement + 'static>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    var_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// STDDEV (Standard Deviation)
// =============================================================================

/// Returns the lookback period for STDDEV.
#[inline]
#[must_use]
pub const fn stddev_lookback(period: usize) -> usize {
    var_lookback(period)
}

/// Returns the minimum input length required for STDDEV.
#[inline]
#[must_use]
pub const fn stddev_min_len(period: usize) -> usize {
    var_min_len(period)
}

/// Computes STDDEV (Standard Deviation) and stores results in output buffer.
///
/// Uses population standard deviation (sqrt of population variance) to match TA-Lib.
///
/// # Formula
/// ```text
/// STDDEV = sqrt(VAR)
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn stddev_into<T: SeriesElement + 'static>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    // Compute variance first
    var_into(data, period, output)?;

    // Take square root of each variance value
    let lookback = stddev_lookback(period);
    for i in lookback..data.len() {
        if output[i].is_finite() {
            output[i] = output[i].sqrt();
        }
    }

    Ok(())
}

/// Computes STDDEV (Standard Deviation).
///
/// Uses population standard deviation (sqrt of population variance) to match TA-Lib.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn stddev<T: SeriesElement + 'static>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    stddev_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// SKEW (Skewness - Third Standardized Moment)
// =============================================================================

/// Returns the lookback period for SKEW.
#[inline]
#[must_use]
pub const fn skew_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for SKEW.
#[inline]
#[must_use]
pub const fn skew_min_len(period: usize) -> usize {
    period
}

/// Computes SKEW (Skewness) and stores results in output buffer.
///
/// Skewness measures the asymmetry of the probability distribution of a real-valued random variable.
/// - Positive skew: tail on the right side
/// - Negative skew: tail on the left side
/// - Zero skew: symmetrical distribution
///
/// # Formula
/// ```text
/// SKEW = E[(X-μ)³] / σ³
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn skew_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    let n = data.len();
    if n < period {
        return Err(Error::InsufficientData {
            indicator: "skew",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "skew",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = skew_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate skewness for each window
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate mean
        let mut sum = T::zero();
        for j in start..=i {
            sum = sum + data[j];
        }
        let mean = sum / period_t;

        // Calculate variance and third moment
        let mut var_sum = T::zero();
        let mut m3 = T::zero();
        for j in start..=i {
            let diff = data[j] - mean;
            var_sum = var_sum + diff * diff;
            m3 = m3 + diff * diff * diff;
        }

        // Calculate skewness
        let variance = var_sum / period_t;
        if variance == T::zero() {
            output[i] = T::nan(); // Undefined for zero variance
        } else {
            let stddev = variance.sqrt();
            let m3_norm = m3 / period_t;
            output[i] = m3_norm / (stddev * stddev * stddev);
        }
    }

    Ok(())
}

/// Computes SKEW (Skewness).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn skew<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    skew_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// KURT (Kurtosis - Fourth Standardized Moment, Excess Kurtosis)
// =============================================================================

/// Returns the lookback period for KURT.
#[inline]
#[must_use]
pub const fn kurt_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for KURT.
#[inline]
#[must_use]
pub const fn kurt_min_len(period: usize) -> usize {
    period
}

/// Computes KURT (Excess Kurtosis) and stores results in output buffer.
///
/// Kurtosis measures the "tailedness" of the probability distribution.
/// This returns excess kurtosis (kurtosis - 3).
/// - Positive: heavier tails than normal distribution (leptokurtic)
/// - Negative: lighter tails than normal distribution (platykurtic)
/// - Zero: similar to normal distribution (mesokurtic)
///
/// # Formula
/// ```text
/// KURT = (E[(X-μ)⁴] / σ⁴) - 3
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn kurt_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    let n = data.len();
    if n < period {
        return Err(Error::InsufficientData {
            indicator: "kurt",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "kurt",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = kurt_lookback(period);
    let period_t = T::from_usize(period)?;
    let three = T::from_usize(3)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate kurtosis for each window
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate mean
        let mut sum = T::zero();
        for j in start..=i {
            sum = sum + data[j];
        }
        let mean = sum / period_t;

        // Calculate variance and fourth moment
        let mut var_sum = T::zero();
        let mut m4 = T::zero();
        for j in start..=i {
            let diff = data[j] - mean;
            var_sum = var_sum + diff * diff;
            m4 = m4 + diff * diff * diff * diff;
        }

        // Calculate kurtosis
        let variance = var_sum / period_t;
        if variance == T::zero() {
            output[i] = T::nan(); // Undefined for zero variance
        } else {
            let m4_norm = m4 / period_t;
            let var_sq = variance * variance;
            output[i] = m4_norm / var_sq - three;
        }
    }

    Ok(())
}

/// Computes KURT (Excess Kurtosis).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn kurt<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    kurt_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// COV (Covariance)
// =============================================================================

/// Returns the lookback period for COV.
#[inline]
#[must_use]
pub const fn cov_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for COV.
#[inline]
#[must_use]
pub const fn cov_min_len(period: usize) -> usize {
    period
}

/// Computes COV (Covariance) and stores results in output buffer.
///
/// Covariance measures how two variables move together.
/// - Positive: variables move in the same direction
/// - Negative: variables move in opposite directions
/// - Zero: variables are independent
///
/// # Formula
/// ```text
/// COV = E[(X-μx)(Y-μy)]
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - The arrays have different lengths (`Error::LengthMismatch`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cov_into<T: SeriesElement>(
    data0: &[T],
    data1: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data0.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = data0.len();
    if data1.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("data0 has {} elements, data1 has {}", n, data1.len()),
        });
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    if n < period {
        return Err(Error::InsufficientData {
            indicator: "cov",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "cov",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = cov_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate covariance for each window
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate means
        let mut sum_x = T::zero();
        let mut sum_y = T::zero();
        for j in start..=i {
            sum_x = sum_x + data0[j];
            sum_y = sum_y + data1[j];
        }
        let mean_x = sum_x / period_t;
        let mean_y = sum_y / period_t;

        // Calculate covariance
        let mut cov = T::zero();
        for j in start..=i {
            let dx = data0[j] - mean_x;
            let dy = data1[j] - mean_y;
            cov = cov + dx * dy;
        }

        output[i] = cov / period_t;
    }

    Ok(())
}

/// Computes COV (Covariance).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - The arrays have different lengths (`Error::LengthMismatch`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn cov<T: SeriesElement>(data0: &[T], data1: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data0.len()];
    cov_into(data0, data1, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// ZSCORE (Rolling Z-Score)
// =============================================================================

/// Returns the lookback period for ZSCORE.
#[inline]
#[must_use]
pub const fn zscore_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for ZSCORE.
#[inline]
#[must_use]
pub const fn zscore_min_len(period: usize) -> usize {
    period
}

/// Computes ZSCORE (Rolling Z-Score) and stores results in output buffer.
///
/// Z-Score measures how many standard deviations an element is from the mean.
/// - Positive: value is above the mean
/// - Negative: value is below the mean
/// - Zero: value is at the mean
///
/// # Formula
/// ```text
/// ZSCORE = (X - μ) / σ
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn zscore_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    let n = data.len();
    if n < period {
        return Err(Error::InsufficientData {
            indicator: "zscore",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "zscore",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = zscore_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate z-score for each window
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate mean
        let mut sum = T::zero();
        for j in start..=i {
            sum = sum + data[j];
        }
        let mean = sum / period_t;

        // Calculate variance
        let mut var_sum = T::zero();
        for j in start..=i {
            let diff = data[j] - mean;
            var_sum = var_sum + diff * diff;
        }
        let variance = var_sum / period_t;

        if variance == T::zero() {
            output[i] = T::nan(); // Undefined for zero variance
        } else {
            let stddev = variance.sqrt();
            output[i] = (data[i] - mean) / stddev;
        }
    }

    Ok(())
}

/// Computes ZSCORE (Rolling Z-Score).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn zscore<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    zscore_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// MAD (Mean Absolute Deviation)
// =============================================================================

/// Returns the lookback period for MAD.
#[inline]
#[must_use]
pub const fn mad_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for MAD.
#[inline]
#[must_use]
pub const fn mad_min_len(period: usize) -> usize {
    period
}

/// Computes MAD (Mean Absolute Deviation) and stores results in output buffer.
///
/// Mean Absolute Deviation measures the average distance of data points from the mean.
/// More robust to outliers than standard deviation.
///
/// # Formula
/// ```text
/// MAD = E[|X - μ|]
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn mad_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    let n = data.len();
    if n < period {
        return Err(Error::InsufficientData {
            indicator: "mad",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "mad",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = mad_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate MAD for each window
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate mean
        let mut sum = T::zero();
        for j in start..=i {
            sum = sum + data[j];
        }
        let mean = sum / period_t;

        // Calculate mean absolute deviation
        let mut mad = T::zero();
        for j in start..=i {
            let diff = (data[j] - mean).abs();
            mad = mad + diff;
        }

        output[i] = mad / period_t;
    }

    Ok(())
}

/// Computes MAD (Mean Absolute Deviation).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn mad<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    mad_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// SEM (Standard Error of Mean)
// =============================================================================

/// Returns the lookback period for SEM.
#[inline]
#[must_use]
pub const fn sem_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for SEM.
#[inline]
#[must_use]
pub const fn sem_min_len(period: usize) -> usize {
    period
}

/// Computes SEM (Standard Error of Mean) and stores results in output buffer.
///
/// Standard Error of Mean measures the standard deviation of the sample mean.
/// Used to estimate the precision of the mean estimate.
///
/// # Formula
/// ```text
/// SEM = σ / sqrt(n)
/// ```
/// where σ is the population standard deviation and n is the sample size.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn sem_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    let n = data.len();
    if n < period {
        return Err(Error::InsufficientData {
            indicator: "sem",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "sem",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = sem_lookback(period);
    let period_t = T::from_usize(period)?;
    let period_sqrt = period_t.sqrt();

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate SEM for each window
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate mean
        let mut sum = T::zero();
        for j in start..=i {
            sum = sum + data[j];
        }
        let mean = sum / period_t;

        // Calculate variance
        let mut var_sum = T::zero();
        for j in start..=i {
            let diff = data[j] - mean;
            var_sum = var_sum + diff * diff;
        }
        let variance = var_sum / period_t;

        if variance == T::zero() {
            output[i] = T::zero();
        } else {
            let stddev = variance.sqrt();
            output[i] = stddev / period_sqrt;
        }
    }

    Ok(())
}

/// Computes SEM (Standard Error of Mean).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn sem<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    sem_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// CORREL (Pearson Correlation Coefficient)
// =============================================================================

/// Returns the lookback period for CORREL.
#[inline]
#[must_use]
pub const fn correl_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for CORREL.
#[inline]
#[must_use]
pub const fn correl_min_len(period: usize) -> usize {
    period
}

/// Computes CORREL (Pearson Correlation Coefficient) and stores results in output buffer.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn correl_into<T: SeriesElement>(
    data0: &[T],
    data1: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data0.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = data0.len();
    if data1.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("data0 has {} elements, data1 has {}", n, data1.len()),
        });
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    if n < period {
        return Err(Error::InsufficientData {
            indicator: "correl",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "correl",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = correl_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Pearson correlation: r = Σ((x-μx)(y-μy)) / sqrt(Σ(x-μx)² * Σ(y-μy)²)
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate means
        let mut sum_x = T::zero();
        let mut sum_y = T::zero();
        for j in start..=i {
            sum_x = sum_x + data0[j];
            sum_y = sum_y + data1[j];
        }
        let mean_x = sum_x / period_t;
        let mean_y = sum_y / period_t;

        // Calculate covariance and variances
        let mut cov = T::zero();
        let mut var_x = T::zero();
        let mut var_y = T::zero();
        for j in start..=i {
            let dx = data0[j] - mean_x;
            let dy = data1[j] - mean_y;
            cov = cov + dx * dy;
            var_x = var_x + dx * dx;
            var_y = var_y + dy * dy;
        }

        let denom = (var_x * var_y).sqrt();
        if denom == T::zero() {
            output[i] = T::zero(); // No variance = undefined correlation, return 0
        } else {
            output[i] = cov / denom;
        }
    }

    Ok(())
}

/// Computes CORREL (Pearson Correlation Coefficient).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn correl<T: SeriesElement>(data0: &[T], data1: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data0.len()];
    correl_into(data0, data1, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// BETA
// =============================================================================

/// Returns the lookback period for BETA.
#[inline]
#[must_use]
pub const fn beta_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for BETA.
#[inline]
#[must_use]
pub const fn beta_min_len(period: usize) -> usize {
    period
}

/// Computes BETA and stores results in output buffer.
///
/// Beta = Covariance(asset, market) / Variance(market)
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn beta_into<T: SeriesElement>(
    data0: &[T], // asset returns
    data1: &[T], // market/benchmark returns
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data0.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = data0.len();
    if data1.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("data0 has {} elements, data1 has {}", n, data1.len()),
        });
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    if n < period {
        return Err(Error::InsufficientData {
            indicator: "beta",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "beta",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = beta_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Beta = Cov(asset, market) / Var(market)
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate means
        let mut sum_x = T::zero();
        let mut sum_y = T::zero();
        for j in start..=i {
            sum_x = sum_x + data0[j];
            sum_y = sum_y + data1[j];
        }
        let mean_x = sum_x / period_t;
        let mean_y = sum_y / period_t;

        // Calculate covariance and market variance
        let mut cov = T::zero();
        let mut var_y = T::zero();
        for j in start..=i {
            let dx = data0[j] - mean_x;
            let dy = data1[j] - mean_y;
            cov = cov + dx * dy;
            var_y = var_y + dy * dy;
        }

        let market_var = var_y / period_t;
        if market_var == T::zero() {
            output[i] = T::zero(); // No variance = undefined beta, return 0
        } else {
            output[i] = cov / market_var;
        }
    }

    Ok(())
}

/// Computes BETA.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn beta<T: SeriesElement>(
    data0: &[T],
    data1: &[T],
    period: usize,
) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data0.len()];
    beta_into(data0, data1, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// LINEARREG (Linear Regression)
// =============================================================================

/// Returns the lookback period for LINEARREG.
#[inline]
#[must_use]
pub const fn linearreg_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for LINEARREG.
#[inline]
#[must_use]
pub const fn linearreg_min_len(period: usize) -> usize {
    period
}

/// Computes LINEARREG (predicted value at the end of period) and stores results in output buffer.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn linearreg_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    let n = data.len();
    if n < period {
        return Err(Error::InsufficientData {
            indicator: "linearreg",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "linearreg",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = linearreg_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Pre-compute sum of squares and other constants (x values are always 0..period-1)
    let mut x_sum = T::zero();
    let mut x_sq_sum = T::zero();
    for i in 0..period {
        let x = T::from_usize(i)?;
        x_sum = x_sum + x;
        x_sq_sum = x_sq_sum + x * x;
    }

    // Precompute constants
    let x_end = T::from_usize(period - 1)?;
    let denominator_const = period_t * x_sq_sum - x_sum * x_sum;

    // Check for degenerate case (all x values are same, should not happen for period > 1)
    if denominator_const == T::zero() {
        for i in lookback..n {
            output[i] = T::nan();
        }
        return Ok(());
    }

    // Initialize sums for first window using O(k) loop
    let mut y_sum = T::zero();
    let mut xy_sum = T::zero();
    for j in 0..period {
        let x = T::from_usize(j)?;
        y_sum = y_sum + data[j];
        xy_sum = xy_sum + x * data[j];
    }

    // Compute first output
    let numerator = period_t * xy_sum - x_sum * y_sum;
    let b = numerator / denominator_const;
    let a = (y_sum - b * x_sum) / period_t;
    output[lookback] = a + b * x_end;

    // Rolling updates: O(1) per element
    for i in (lookback + 1)..n {
        let old_val = data[i - period];
        let new_val = data[i];

        // Update y_sum: remove old, add new
        y_sum = y_sum - old_val + new_val;

        // Update xy_sum with rolling formula:
        // When window slides right, all x coordinates shift down by 1
        // xy_sum_new = xy_sum_old - y_sum_old + old_val + (period-1) * new_val
        // Simplified after y_sum update: xy_sum - y_sum + new_val * period
        xy_sum = xy_sum - y_sum + new_val * period_t;

        // Compute regression parameters and output
        let numerator = period_t * xy_sum - x_sum * y_sum;
        let b = numerator / denominator_const;
        let a = (y_sum - b * x_sum) / period_t;
        output[i] = a + b * x_end;
    }

    Ok(())
}

/// Computes LINEARREG (predicted value at the end of period).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn linearreg<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    use std::any::TypeId;

    // Optimized allocation for f64/f32: avoid double-initialization
    // linearreg_into() writes all elements, so this is pure double-write tax
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let data_f64: &[f64] = unsafe { std::mem::transmute(data) };
        let mut output: Vec<f64> = Vec::with_capacity(data.len());
        unsafe { output.set_len(data.len()); }
        linearreg_into(data_f64, period, &mut output)?;
        Ok(unsafe { std::mem::transmute(output) })
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let data_f32: &[f32] = unsafe { std::mem::transmute(data) };
        let mut output: Vec<f32> = Vec::with_capacity(data.len());
        unsafe { output.set_len(data.len()); }
        linearreg_into(data_f32, period, &mut output)?;
        Ok(unsafe { std::mem::transmute(output) })
    } else {
        // Generic fallback: initialize to NaN
        let mut output = vec![T::nan(); data.len()];
        linearreg_into(data, period, &mut output)?;
        Ok(output)
    }
}

// =============================================================================
// LINEARREG_SLOPE
// =============================================================================

/// Returns the lookback period for LINEARREG_SLOPE.
#[inline]
#[must_use]
pub const fn linearreg_slope_lookback(period: usize) -> usize {
    linearreg_lookback(period)
}

/// Returns the minimum input length required for LINEARREG_SLOPE.
#[inline]
#[must_use]
pub const fn linearreg_slope_min_len(period: usize) -> usize {
    linearreg_min_len(period)
}

/// Computes LINEARREG_SLOPE (linear regression slope) and stores results in output buffer.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn linearreg_slope_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    let n = data.len();
    if n < period {
        return Err(Error::InsufficientData {
            indicator: "linearreg_slope",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "linearreg_slope",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = linearreg_slope_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Pre-compute sum of squares and other constants
    let mut x_sum = T::zero();
    let mut x_sq_sum = T::zero();
    for i in 0..period {
        let x = T::from_usize(i)?;
        x_sum = x_sum + x;
        x_sq_sum = x_sq_sum + x * x;
    }

    // Calculate linear regression slope for each window
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate y sum and xy sum
        let mut y_sum = T::zero();
        let mut xy_sum = T::zero();

        for j in 0..period {
            let x = T::from_usize(j)?;
            y_sum = y_sum + data[start + j];
            xy_sum = xy_sum + x * data[start + j];
        }

        // Linear regression: y = a + bx
        // b = (n*Σxy - Σx*Σy) / (n*Σx² - (Σx)²)
        let numerator = period_t * xy_sum - x_sum * y_sum;
        let denominator = period_t * x_sq_sum - x_sum * x_sum;

        if denominator == T::zero() {
            output[i] = T::nan();
        } else {
            output[i] = numerator / denominator;
        }
    }

    Ok(())
}

/// Computes LINEARREG_SLOPE (linear regression slope).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn linearreg_slope<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    linearreg_slope_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// LINEARREG_INTERCEPT
// =============================================================================

/// Returns the lookback period for LINEARREG_INTERCEPT.
#[inline]
#[must_use]
pub const fn linearreg_intercept_lookback(period: usize) -> usize {
    linearreg_lookback(period)
}

/// Returns the minimum input length required for LINEARREG_INTERCEPT.
#[inline]
#[must_use]
pub const fn linearreg_intercept_min_len(period: usize) -> usize {
    linearreg_min_len(period)
}

/// Computes LINEARREG_INTERCEPT (linear regression intercept) and stores results in output buffer.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn linearreg_intercept_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    let n = data.len();
    if n < period {
        return Err(Error::InsufficientData {
            indicator: "linearreg_intercept",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "linearreg_intercept",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = linearreg_intercept_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Pre-compute sum of squares and other constants
    let mut x_sum = T::zero();
    let mut x_sq_sum = T::zero();
    for i in 0..period {
        let x = T::from_usize(i)?;
        x_sum = x_sum + x;
        x_sq_sum = x_sq_sum + x * x;
    }

    // Calculate linear regression intercept for each window
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate y sum and xy sum
        let mut y_sum = T::zero();
        let mut xy_sum = T::zero();

        for j in 0..period {
            let x = T::from_usize(j)?;
            y_sum = y_sum + data[start + j];
            xy_sum = xy_sum + x * data[start + j];
        }

        // Linear regression: y = a + bx
        // b = (n*Σxy - Σx*Σy) / (n*Σx² - (Σx)²)
        // a = (Σy - b*Σx) / n
        let numerator = period_t * xy_sum - x_sum * y_sum;
        let denominator = period_t * x_sq_sum - x_sum * x_sum;

        if denominator == T::zero() {
            output[i] = T::nan();
        } else {
            let b = numerator / denominator;
            output[i] = (y_sum - b * x_sum) / period_t;
        }
    }

    Ok(())
}

/// Computes LINEARREG_INTERCEPT (linear regression intercept).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn linearreg_intercept<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    linearreg_intercept_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// LINEARREG_ANGLE
// =============================================================================

/// Returns the lookback period for LINEARREG_ANGLE.
#[inline]
#[must_use]
pub const fn linearreg_angle_lookback(period: usize) -> usize {
    linearreg_lookback(period)
}

/// Returns the minimum input length required for LINEARREG_ANGLE.
#[inline]
#[must_use]
pub const fn linearreg_angle_min_len(period: usize) -> usize {
    linearreg_min_len(period)
}

/// Computes LINEARREG_ANGLE (linear regression angle in degrees) and stores results in output buffer.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn linearreg_angle_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    let n = data.len();
    if n < period {
        return Err(Error::InsufficientData {
            indicator: "linearreg_angle",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "linearreg_angle",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = linearreg_angle_lookback(period);
    let period_t = T::from_usize(period)?;
    let degrees_per_radian = T::from_f64(180.0 / std::f64::consts::PI)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Pre-compute sum of squares and other constants
    let mut x_sum = T::zero();
    let mut x_sq_sum = T::zero();
    for i in 0..period {
        let x = T::from_usize(i)?;
        x_sum = x_sum + x;
        x_sq_sum = x_sq_sum + x * x;
    }

    // Calculate linear regression angle for each window
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate y sum and xy sum
        let mut y_sum = T::zero();
        let mut xy_sum = T::zero();

        for j in 0..period {
            let x = T::from_usize(j)?;
            y_sum = y_sum + data[start + j];
            xy_sum = xy_sum + x * data[start + j];
        }

        // Linear regression: y = a + bx
        // b = (n*Σxy - Σx*Σy) / (n*Σx² - (Σx)²)
        let numerator = period_t * xy_sum - x_sum * y_sum;
        let denominator = period_t * x_sq_sum - x_sum * x_sum;

        if denominator == T::zero() {
            output[i] = T::nan();
        } else {
            let b = numerator / denominator;
            output[i] = b.atan() * degrees_per_radian;
        }
    }

    Ok(())
}

/// Computes LINEARREG_ANGLE (linear regression angle in degrees).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn linearreg_angle<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    linearreg_angle_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// TSF (Time Series Forecast)
// =============================================================================

/// Returns the lookback period for TSF.
#[inline]
#[must_use]
pub const fn tsf_lookback(period: usize) -> usize {
    linearreg_lookback(period)
}

/// Returns the minimum input length required for TSF.
#[inline]
#[must_use]
pub const fn tsf_min_len(period: usize) -> usize {
    linearreg_min_len(period)
}

/// Computes TSF (Time Series Forecast - one period ahead prediction) and stores results in output buffer.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn tsf_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "period must be >= 1",
        });
    }

    let n = data.len();
    if n < period {
        return Err(Error::InsufficientData {
            indicator: "tsf",
            required: period,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "tsf",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = tsf_lookback(period);
    let period_t = T::from_usize(period)?;

    // Fill lookback with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Pre-compute sum of squares and other constants
    let mut x_sum = T::zero();
    let mut x_sq_sum = T::zero();
    for i in 0..period {
        let x = T::from_usize(i)?;
        x_sum = x_sum + x;
        x_sq_sum = x_sq_sum + x * x;
    }

    // Calculate TSF for each window
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate y sum and xy sum
        let mut y_sum = T::zero();
        let mut xy_sum = T::zero();

        for j in 0..period {
            let x = T::from_usize(j)?;
            y_sum = y_sum + data[start + j];
            xy_sum = xy_sum + x * data[start + j];
        }

        // Linear regression: y = a + bx
        // b = (n*Σxy - Σx*Σy) / (n*Σx² - (Σx)²)
        let numerator = period_t * xy_sum - x_sum * y_sum;
        let denominator = period_t * x_sq_sum - x_sum * x_sum;

        if denominator == T::zero() {
            output[i] = T::nan();
        } else {
            let b = numerator / denominator;
            let a = (y_sum - b * x_sum) / period_t;

            // Predicted value one period ahead (x = period)
            let x_ahead = T::from_usize(period)?;
            output[i] = a + b * x_ahead;
        }
    }

    Ok(())
}

/// Computes TSF (Time Series Forecast - one period ahead prediction).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn tsf<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    tsf_into(data, period, &mut output)?;
    Ok(output)
}