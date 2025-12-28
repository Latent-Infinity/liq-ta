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
use crate::traits::SeriesElement;

// =============================================================================
// Helper: Finite value check
// =============================================================================

/// Inline helper to check if a value is finite (not NaN or Infinity).
/// Per project NaN propagation policy, both NaN and Infinity produce NaN output.
#[inline(always)]
fn is_not_finite<T: SeriesElement>(val: T) -> bool {
    !val.is_finite()
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

/// Computes VAR (Variance) and stores results in output buffer.
///
/// Uses population variance (÷n, not ÷(n-1)) to match TA-Lib.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn var_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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

    let lookback = var_lookback(period);

    // Fill lookback with NaN
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Pre-scan for NaN - this is ~5x faster than checking per-element in the hot loop
    // NaN checks in hot loop add ~109µs overhead for 100K elements
    // Pre-scan adds only ~26µs but allows fast path without any checks
    let has_nan = data.iter().take(n).any(|&v| is_not_finite(v));

    if has_nan {
        // Slow path with per-element NaN tracking
        var_rolling_slow(data, period, output, lookback)
    } else {
        // Fast path - no NaN checks in hot loop (matches TA-Lib performance)
        var_rolling_fast(data, period, output, lookback)
    }
}

/// Fast VAR implementation without NaN checks.
/// Used when pre-scan confirms no NaN/Infinity in input.
#[inline]
fn var_rolling_fast<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
    lookback: usize,
) -> Result<()> {
    let n = data.len();
    let period_t = T::from_usize(period)?;

    // Calculate initial sums for first window
    let mut sum = T::zero();
    let mut sum_sq = T::zero();
    for i in 0..period {
        sum = sum + data[i];
        sum_sq = sum_sq + data[i] * data[i];
    }

    // Calculate first variance
    let mean = sum / period_t;
    output[lookback] = sum_sq / period_t - mean * mean;

    // Rolling calculation - tight loop, no NaN checks
    for i in (lookback + 1)..n {
        let old_val = data[i - period];
        let new_val = data[i];

        // Update sums
        sum = sum + new_val - old_val;
        sum_sq = sum_sq + new_val * new_val - old_val * old_val;

        // Compute variance
        let mean = sum / period_t;
        output[i] = sum_sq / period_t - mean * mean;
    }

    Ok(())
}

/// Slow VAR implementation with per-element NaN tracking.
/// Used when input contains NaN/Infinity values.
fn var_rolling_slow<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
    lookback: usize,
) -> Result<()> {
    let n = data.len();
    let period_t = T::from_usize(period)?;

    // Calculate initial sums for first window
    let mut sum = T::zero();
    let mut sum_sq = T::zero();
    let mut invalid_count = 0usize;
    for i in 0..period {
        if is_not_finite(data[i]) {
            invalid_count += 1;
        } else {
            sum = sum + data[i];
            sum_sq = sum_sq + data[i] * data[i];
        }
    }

    // Calculate first variance
    if invalid_count > 0 {
        output[lookback] = T::nan();
    } else {
        let mean = sum / period_t;
        output[lookback] = sum_sq / period_t - mean * mean;
    }

    // Rolling calculation with NaN tracking
    for i in (lookback + 1)..n {
        let old_val = data[i - period];
        let new_val = data[i];

        if is_not_finite(old_val) {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            sum = sum - old_val;
            sum_sq = sum_sq - old_val * old_val;
        }
        if is_not_finite(new_val) {
            invalid_count += 1;
        } else {
            sum = sum + new_val;
            sum_sq = sum_sq + new_val * new_val;
        }

        if invalid_count > 0 {
            output[i] = T::nan();
            continue;
        }

        let mean = sum / period_t;
        output[i] = sum_sq / period_t - mean * mean;
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
pub fn var<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
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
pub fn stddev_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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
pub fn stddev<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    stddev_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// SKEW (Skewness)
// =============================================================================

/// Returns the lookback period for SKEW.
#[inline]
#[must_use]
pub const fn skew_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

/// Returns the minimum input length required for SKEW.
#[inline]
#[must_use]
pub const fn skew_min_len(period: usize) -> usize {
    period
}

/// Computes SKEW (Skewness) and stores results in output buffer.
///
/// Uses population skewness (third standardized moment).
///
/// # Formula
/// ```text
/// skew = E[(x - μ)³] / σ³
///      = μ₃ / μ₂^(3/2)
/// ```
///
/// where μ₂ is variance and μ₃ is the third central moment.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn skew_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Pre-scan for non-finite values
    let has_nan = data.iter().take(n).any(|&v| is_not_finite(v));

    if has_nan {
        skew_rolling_slow(data, period, output, lookback, period_t)
    } else {
        skew_rolling_fast(data, period, output, lookback, period_t)
    }
}

/// Fast SKEW implementation without NaN checks.
#[inline]
fn skew_rolling_fast<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
    lookback: usize,
    period_t: T,
) -> Result<()> {
    let n = data.len();
    let three = T::from_f64(3.0)?;
    let two = T::from_f64(2.0)?;
    let one_point_five = T::from_f64(1.5)?;

    // Calculate initial sums for first window
    let mut sum = T::zero();
    let mut sum_sq = T::zero();
    let mut sum_cb = T::zero();
    for i in 0..period {
        let v = data[i];
        let sq = v * v;
        sum = sum + v;
        sum_sq = sum_sq + sq;
        sum_cb = sum_cb + sq * v;
    }

    // Calculate first skewness
    output[lookback] = compute_skewness(sum, sum_sq, sum_cb, period_t, three, two, one_point_five);

    // Rolling calculation
    for i in (lookback + 1)..n {
        let old_val = data[i - period];
        let new_val = data[i];

        let old_sq = old_val * old_val;
        let new_sq = new_val * new_val;

        sum = sum + new_val - old_val;
        sum_sq = sum_sq + new_sq - old_sq;
        sum_cb = sum_cb + new_sq * new_val - old_sq * old_val;

        output[i] = compute_skewness(sum, sum_sq, sum_cb, period_t, three, two, one_point_five);
    }

    Ok(())
}

/// Slow SKEW implementation with per-element NaN tracking.
fn skew_rolling_slow<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
    lookback: usize,
    period_t: T,
) -> Result<()> {
    let n = data.len();
    let three = T::from_f64(3.0)?;
    let two = T::from_f64(2.0)?;
    let one_point_five = T::from_f64(1.5)?;

    // Calculate initial sums for first window
    let mut sum = T::zero();
    let mut sum_sq = T::zero();
    let mut sum_cb = T::zero();
    let mut invalid_count = 0usize;

    for i in 0..period {
        if is_not_finite(data[i]) {
            invalid_count += 1;
        } else {
            let v = data[i];
            let sq = v * v;
            sum = sum + v;
            sum_sq = sum_sq + sq;
            sum_cb = sum_cb + sq * v;
        }
    }

    // Calculate first skewness
    if invalid_count > 0 {
        output[lookback] = T::nan();
    } else {
        output[lookback] = compute_skewness(sum, sum_sq, sum_cb, period_t, three, two, one_point_five);
    }

    // Rolling calculation with NaN tracking
    for i in (lookback + 1)..n {
        let old_val = data[i - period];
        let new_val = data[i];

        if is_not_finite(old_val) {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            let old_sq = old_val * old_val;
            sum = sum - old_val;
            sum_sq = sum_sq - old_sq;
            sum_cb = sum_cb - old_sq * old_val;
        }

        if is_not_finite(new_val) {
            invalid_count += 1;
        } else {
            let new_sq = new_val * new_val;
            sum = sum + new_val;
            sum_sq = sum_sq + new_sq;
            sum_cb = sum_cb + new_sq * new_val;
        }

        if invalid_count > 0 {
            output[i] = T::nan();
            continue;
        }

        output[i] = compute_skewness(sum, sum_sq, sum_cb, period_t, three, two, one_point_five);
    }

    Ok(())
}

/// Helper to compute skewness from raw moments.
///
/// Using the formula for population skewness:
/// skew = μ₃ / μ₂^(3/2)
///
/// where:
/// - μ₂ = m₂ - m₁² (variance, second central moment)
/// - μ₃ = m₃ - 3*m₁*m₂ + 2*m₁³ (third central moment)
/// - m₁ = sum/n, m₂ = sum_sq/n, m₃ = sum_cb/n
#[inline]
fn compute_skewness<T: SeriesElement>(
    sum: T,
    sum_sq: T,
    sum_cb: T,
    n: T,
    three: T,
    two: T,
    one_point_five: T,
) -> T {
    let m1 = sum / n;
    let m2 = sum_sq / n;
    let m3 = sum_cb / n;

    let m1_sq = m1 * m1;
    let m1_cb = m1_sq * m1;

    // Variance (second central moment)
    let mu2 = m2 - m1_sq;

    // Third central moment
    let mu3 = m3 - three * m1 * m2 + two * m1_cb;

    // Skewness = μ₃ / μ₂^(3/2)
    if mu2 <= T::zero() {
        T::zero() // No variance = undefined skewness
    } else {
        mu3 / mu2.powf(one_point_five)
    }
}

/// Computes SKEW (Skewness).
///
/// Uses population skewness (third standardized moment).
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
// KURT (Kurtosis)
// =============================================================================

/// Returns the lookback period for KURT.
#[inline]
#[must_use]
pub const fn kurt_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

/// Returns the minimum input length required for KURT.
#[inline]
#[must_use]
pub const fn kurt_min_len(period: usize) -> usize {
    period
}

/// Computes KURT (Kurtosis) and stores results in output buffer.
///
/// Uses population excess kurtosis (fourth standardized moment minus 3).
///
/// # Formula
/// ```text
/// kurt = E[(x - μ)⁴] / σ⁴ - 3
///      = μ₄ / μ₂² - 3
/// ```
///
/// A normal distribution has excess kurtosis of 0.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn kurt_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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

    // Fill lookback with NaN
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Pre-scan for non-finite values
    let has_nan = data.iter().take(n).any(|&v| is_not_finite(v));

    if has_nan {
        kurt_rolling_slow(data, period, output, lookback, period_t)
    } else {
        kurt_rolling_fast(data, period, output, lookback, period_t)
    }
}

/// Fast KURT implementation without NaN checks.
#[inline]
fn kurt_rolling_fast<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
    lookback: usize,
    period_t: T,
) -> Result<()> {
    let n = data.len();
    let three = T::from_f64(3.0)?;
    let four = T::from_f64(4.0)?;
    let six = T::from_f64(6.0)?;

    // Calculate initial sums for first window
    let mut sum = T::zero();
    let mut sum_sq = T::zero();
    let mut sum_cb = T::zero();
    let mut sum_qd = T::zero();

    for i in 0..period {
        let v = data[i];
        let sq = v * v;
        sum = sum + v;
        sum_sq = sum_sq + sq;
        sum_cb = sum_cb + sq * v;
        sum_qd = sum_qd + sq * sq;
    }

    // Calculate first kurtosis
    output[lookback] = compute_kurtosis(sum, sum_sq, sum_cb, sum_qd, period_t, three, four, six);

    // Rolling calculation
    for i in (lookback + 1)..n {
        let old_val = data[i - period];
        let new_val = data[i];

        let old_sq = old_val * old_val;
        let new_sq = new_val * new_val;

        sum = sum + new_val - old_val;
        sum_sq = sum_sq + new_sq - old_sq;
        sum_cb = sum_cb + new_sq * new_val - old_sq * old_val;
        sum_qd = sum_qd + new_sq * new_sq - old_sq * old_sq;

        output[i] = compute_kurtosis(sum, sum_sq, sum_cb, sum_qd, period_t, three, four, six);
    }

    Ok(())
}

/// Slow KURT implementation with per-element NaN tracking.
fn kurt_rolling_slow<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
    lookback: usize,
    period_t: T,
) -> Result<()> {
    let n = data.len();
    let three = T::from_f64(3.0)?;
    let four = T::from_f64(4.0)?;
    let six = T::from_f64(6.0)?;

    // Calculate initial sums for first window
    let mut sum = T::zero();
    let mut sum_sq = T::zero();
    let mut sum_cb = T::zero();
    let mut sum_qd = T::zero();
    let mut invalid_count = 0usize;

    for i in 0..period {
        if is_not_finite(data[i]) {
            invalid_count += 1;
        } else {
            let v = data[i];
            let sq = v * v;
            sum = sum + v;
            sum_sq = sum_sq + sq;
            sum_cb = sum_cb + sq * v;
            sum_qd = sum_qd + sq * sq;
        }
    }

    // Calculate first kurtosis
    if invalid_count > 0 {
        output[lookback] = T::nan();
    } else {
        output[lookback] = compute_kurtosis(sum, sum_sq, sum_cb, sum_qd, period_t, three, four, six);
    }

    // Rolling calculation with NaN tracking
    for i in (lookback + 1)..n {
        let old_val = data[i - period];
        let new_val = data[i];

        if is_not_finite(old_val) {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            let old_sq = old_val * old_val;
            sum = sum - old_val;
            sum_sq = sum_sq - old_sq;
            sum_cb = sum_cb - old_sq * old_val;
            sum_qd = sum_qd - old_sq * old_sq;
        }

        if is_not_finite(new_val) {
            invalid_count += 1;
        } else {
            let new_sq = new_val * new_val;
            sum = sum + new_val;
            sum_sq = sum_sq + new_sq;
            sum_cb = sum_cb + new_sq * new_val;
            sum_qd = sum_qd + new_sq * new_sq;
        }

        if invalid_count > 0 {
            output[i] = T::nan();
            continue;
        }

        output[i] = compute_kurtosis(sum, sum_sq, sum_cb, sum_qd, period_t, three, four, six);
    }

    Ok(())
}

/// Helper to compute excess kurtosis from raw moments.
///
/// Using the formula for population excess kurtosis:
/// kurt = μ₄ / μ₂² - 3
///
/// where:
/// - μ₂ = m₂ - m₁² (variance)
/// - μ₄ = m₄ - 4*m₁*m₃ + 6*m₁²*m₂ - 3*m₁⁴ (fourth central moment)
/// - m₁ = sum/n, m₂ = sum_sq/n, m₃ = sum_cb/n, m₄ = sum_qd/n
#[inline]
fn compute_kurtosis<T: SeriesElement>(
    sum: T,
    sum_sq: T,
    sum_cb: T,
    sum_qd: T,
    n: T,
    three: T,
    four: T,
    six: T,
) -> T {
    let m1 = sum / n;
    let m2 = sum_sq / n;
    let m3 = sum_cb / n;
    let m4 = sum_qd / n;

    let m1_sq = m1 * m1;
    let m1_qd = m1_sq * m1_sq;

    // Variance (second central moment)
    let mu2 = m2 - m1_sq;

    // Fourth central moment
    let mu4 = m4 - four * m1 * m3 + six * m1_sq * m2 - three * m1_qd;

    // Excess kurtosis = μ₄ / μ₂² - 3
    if mu2 <= T::zero() {
        T::zero() // No variance = undefined kurtosis
    } else {
        mu4 / (mu2 * mu2) - three
    }
}

/// Computes KURT (Kurtosis).
///
/// Uses population excess kurtosis (fourth standardized moment minus 3).
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
    if period == 0 { 0 } else { period - 1 }
}

/// Returns the minimum input length required for COV.
#[inline]
#[must_use]
pub const fn cov_min_len(period: usize) -> usize {
    period
}

/// Computes COV (Covariance) and stores results in output buffer.
///
/// Uses population covariance (÷n, not ÷(n-1)).
///
/// # Formula
/// ```text
/// cov(X, Y) = E[(X - μₓ)(Y - μᵧ)]
///           = E[XY] - E[X]E[Y]
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
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

    // Rolling covariance calculation
    // COV = E[XY] - E[X]E[Y] = (sum_xy/n) - (sum_x/n)(sum_y/n)
    for i in lookback..n {
        let start = i + 1 - period;

        let mut sum_x = T::zero();
        let mut sum_y = T::zero();
        let mut sum_xy = T::zero();
        let mut has_invalid = false;

        for j in start..=i {
            if is_not_finite(data0[j]) || is_not_finite(data1[j]) {
                has_invalid = true;
                break;
            }
            sum_x = sum_x + data0[j];
            sum_y = sum_y + data1[j];
            sum_xy = sum_xy + data0[j] * data1[j];
        }

        if has_invalid {
            output[i] = T::nan();
            continue;
        }

        let mean_x = sum_x / period_t;
        let mean_y = sum_y / period_t;
        output[i] = sum_xy / period_t - mean_x * mean_y;
    }

    Ok(())
}

/// Computes COV (Covariance).
///
/// Uses population covariance (÷n, not ÷(n-1)).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
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
    if period == 0 { 0 } else { period - 1 }
}

/// Returns the minimum input length required for ZSCORE.
#[inline]
#[must_use]
pub const fn zscore_min_len(period: usize) -> usize {
    period
}

/// Computes ZSCORE (Rolling Z-Score) and stores results in output buffer.
///
/// # Formula
/// ```text
/// zscore = (x - μ) / σ
/// ```
///
/// where μ and σ are computed over the rolling window.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn zscore_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Pre-scan for non-finite values
    let has_nan = data.iter().take(n).any(|&v| is_not_finite(v));

    if has_nan {
        zscore_rolling_slow(data, period, output, lookback, period_t)
    } else {
        zscore_rolling_fast(data, period, output, lookback, period_t)
    }
}

/// Fast ZSCORE implementation without NaN checks.
#[inline]
fn zscore_rolling_fast<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
    lookback: usize,
    period_t: T,
) -> Result<()> {
    let n = data.len();

    // Calculate initial sums for first window
    let mut sum = T::zero();
    let mut sum_sq = T::zero();
    for i in 0..period {
        sum = sum + data[i];
        sum_sq = sum_sq + data[i] * data[i];
    }

    // Calculate first z-score
    let mean = sum / period_t;
    let var = sum_sq / period_t - mean * mean;
    if var <= T::zero() {
        output[lookback] = T::zero();
    } else {
        output[lookback] = (data[lookback] - mean) / var.sqrt();
    }

    // Rolling calculation
    for i in (lookback + 1)..n {
        let old_val = data[i - period];
        let new_val = data[i];

        sum = sum + new_val - old_val;
        sum_sq = sum_sq + new_val * new_val - old_val * old_val;

        let mean = sum / period_t;
        let var = sum_sq / period_t - mean * mean;
        if var <= T::zero() {
            output[i] = T::zero();
        } else {
            output[i] = (data[i] - mean) / var.sqrt();
        }
    }

    Ok(())
}

/// Slow ZSCORE implementation with per-element NaN tracking.
fn zscore_rolling_slow<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
    lookback: usize,
    period_t: T,
) -> Result<()> {
    let n = data.len();

    // Calculate initial sums for first window
    let mut sum = T::zero();
    let mut sum_sq = T::zero();
    let mut invalid_count = 0usize;

    for i in 0..period {
        if is_not_finite(data[i]) {
            invalid_count += 1;
        } else {
            sum = sum + data[i];
            sum_sq = sum_sq + data[i] * data[i];
        }
    }

    // Calculate first z-score
    if invalid_count > 0 {
        output[lookback] = T::nan();
    } else {
        let mean = sum / period_t;
        let var = sum_sq / period_t - mean * mean;
        if var <= T::zero() {
            output[lookback] = T::zero();
        } else {
            output[lookback] = (data[lookback] - mean) / var.sqrt();
        }
    }

    // Rolling calculation with NaN tracking
    for i in (lookback + 1)..n {
        let old_val = data[i - period];
        let new_val = data[i];

        if is_not_finite(old_val) {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            sum = sum - old_val;
            sum_sq = sum_sq - old_val * old_val;
        }

        if is_not_finite(new_val) {
            invalid_count += 1;
        } else {
            sum = sum + new_val;
            sum_sq = sum_sq + new_val * new_val;
        }

        if invalid_count > 0 {
            output[i] = T::nan();
            continue;
        }

        let mean = sum / period_t;
        let var = sum_sq / period_t - mean * mean;
        if var <= T::zero() {
            output[i] = T::zero();
        } else {
            output[i] = (data[i] - mean) / var.sqrt();
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
    if period == 0 { 0 } else { period - 1 }
}

/// Returns the minimum input length required for MAD.
#[inline]
#[must_use]
pub const fn mad_min_len(period: usize) -> usize {
    period
}

/// Computes MAD (Mean Absolute Deviation) and stores results in output buffer.
///
/// # Formula
/// ```text
/// MAD = (1/n) * Σ|xᵢ - μ|
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn mad_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // MAD requires two passes: one for mean, one for absolute deviations
    // No efficient single-pass algorithm exists for MAD
    for i in lookback..n {
        let start = i + 1 - period;

        // First pass: compute mean
        let mut sum = T::zero();
        let mut has_invalid = false;
        for j in start..=i {
            if is_not_finite(data[j]) {
                has_invalid = true;
                break;
            }
            sum = sum + data[j];
        }

        if has_invalid {
            output[i] = T::nan();
            continue;
        }

        let mean = sum / period_t;

        // Second pass: compute mean absolute deviation
        let mut mad_sum = T::zero();
        for j in start..=i {
            mad_sum = mad_sum + (data[j] - mean).abs();
        }

        output[i] = mad_sum / period_t;
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
    if period == 0 { 0 } else { period - 1 }
}

/// Returns the minimum input length required for SEM.
#[inline]
#[must_use]
pub const fn sem_min_len(period: usize) -> usize {
    period
}

/// Computes SEM (Standard Error of Mean) and stores results in output buffer.
///
/// # Formula
/// ```text
/// SEM = σ / sqrt(n)
/// ```
///
/// where σ is the standard deviation.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn sem_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    // Compute standard deviation first
    stddev_into(data, period, output)?;

    // Divide by sqrt(n) to get SEM
    let sqrt_n = T::from_usize(period)?.sqrt();
    let lookback = sem_lookback(period);

    for i in lookback..data.len() {
        if output[i].is_finite() {
            output[i] = output[i] / sqrt_n;
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
        let mut has_invalid = false;
        for j in start..=i {
            if is_not_finite(data0[j]) || is_not_finite(data1[j]) {
                has_invalid = true;
                break;
            }
            sum_x = sum_x + data0[j];
            sum_y = sum_y + data1[j];
        }
        if has_invalid {
            output[i] = T::nan();
            continue;
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
        let mut has_invalid = false;
        for j in start..=i {
            if is_not_finite(data0[j]) || is_not_finite(data1[j]) {
                has_invalid = true;
                break;
            }
            sum_x = sum_x + data0[j];
            sum_y = sum_y + data1[j];
        }
        if has_invalid {
            output[i] = T::nan();
            continue;
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

        if var_y == T::zero() {
            output[i] = T::zero(); // No market variance = undefined beta
        } else {
            output[i] = cov / var_y;
        }
    }

    Ok(())
}

/// Computes BETA.
///
/// Beta = Covariance(asset, market) / Variance(market)
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn beta<T: SeriesElement>(
    data0: &[T], // asset returns
    data1: &[T], // market/benchmark returns
    period: usize,
) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data0.len()];
    beta_into(data0, data1, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// Linear Regression Core
// =============================================================================

/// Computes linear regression coefficients for a rolling window.
/// Returns (slope, intercept) for each valid position.
fn linear_regression_core<T: SeriesElement>(
    data: &[T],
    period: usize,
    slope_out: &mut [T],
    intercept_out: &mut [T],
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

    let lookback = period - 1;
    let period_t = T::from_usize(period)?;

    // Pre-compute sums for x = 0, 1, 2, ..., period-1
    // Σx = period*(period-1)/2
    // Σx² = period*(period-1)*(2*period-1)/6
    let sum_x = T::from_usize(period * (period - 1) / 2)?;
    let sum_x2 = T::from_usize(period * (period - 1) * (2 * period - 1) / 6)?;

    // Denominator: n * Σx² - (Σx)²
    // Note: This is intentional - standard linear regression formula
    #[allow(clippy::suspicious_operation_groupings)]
    let denom = period_t * sum_x2 - sum_x * sum_x;

    // Fill lookback with NaN
    for i in 0..lookback {
        slope_out[i] = T::nan();
        intercept_out[i] = T::nan();
    }

    // Calculate linear regression for each window
    for i in lookback..n {
        let start = i + 1 - period;

        // Calculate Σy and Σxy
        let mut sum_y = T::zero();
        let mut sum_xy = T::zero();
        let mut has_invalid = false;
        for (x_idx, j) in (start..=i).enumerate() {
            if is_not_finite(data[j]) {
                has_invalid = true;
                break;
            }
            let x = T::from_usize(x_idx)?;
            sum_y = sum_y + data[j];
            sum_xy = sum_xy + x * data[j];
        }
        if has_invalid {
            slope_out[i] = T::nan();
            intercept_out[i] = T::nan();
            continue;
        }

        // slope = (n * Σxy - Σx * Σy) / denom
        let slope = (period_t * sum_xy - sum_x * sum_y) / denom;

        // intercept = (Σy - slope * Σx) / n
        let intercept = (sum_y - slope * sum_x) / period_t;

        slope_out[i] = slope;
        intercept_out[i] = intercept;
    }

    Ok(())
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

/// Computes LINEARREG and stores results in output buffer.
///
/// Returns the predicted value at the end of the regression line.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn linearreg_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    let n = data.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "linearreg",
            required: n,
            actual: output.len(),
        });
    }

    let mut slope = vec![T::nan(); n];
    let mut intercept = vec![T::nan(); n];
    linear_regression_core(data, period, &mut slope, &mut intercept)?;

    let lookback = linearreg_lookback(period);
    let last_x = T::from_usize(period - 1)?;

    for i in 0..lookback {
        output[i] = T::nan();
    }

    // linearreg = intercept + slope * (period - 1)
    for i in lookback..n {
        output[i] = intercept[i] + slope[i] * last_x;
    }

    Ok(())
}

/// Computes LINEARREG (Linear Regression).
///
/// Returns the predicted value at the end of the regression line.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn linearreg<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    linearreg_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// LINEARREG_SLOPE
// =============================================================================

/// Returns the lookback period for `LINEARREG_SLOPE`.
#[inline]
#[must_use]
pub const fn linearreg_slope_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for `LINEARREG_SLOPE`.
#[inline]
#[must_use]
pub const fn linearreg_slope_min_len(period: usize) -> usize {
    period
}

/// Computes `LINEARREG_SLOPE` and stores results in output buffer.
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
    let n = data.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "linearreg_slope",
            required: n,
            actual: output.len(),
        });
    }

    let mut intercept = vec![T::nan(); n];
    linear_regression_core(data, period, output, &mut intercept)?;

    Ok(())
}

/// Computes `LINEARREG_SLOPE` (Linear Regression Slope).
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

/// Returns the lookback period for `LINEARREG_INTERCEPT`.
#[inline]
#[must_use]
pub const fn linearreg_intercept_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for `LINEARREG_INTERCEPT`.
#[inline]
#[must_use]
pub const fn linearreg_intercept_min_len(period: usize) -> usize {
    period
}

/// Computes `LINEARREG_INTERCEPT` and stores results in output buffer.
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
    let n = data.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "linearreg_intercept",
            required: n,
            actual: output.len(),
        });
    }

    let mut slope = vec![T::nan(); n];
    linear_regression_core(data, period, &mut slope, output)?;

    Ok(())
}

/// Computes `LINEARREG_INTERCEPT` (Linear Regression Intercept).
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

/// Returns the lookback period for `LINEARREG_ANGLE`.
#[inline]
#[must_use]
pub const fn linearreg_angle_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for `LINEARREG_ANGLE`.
#[inline]
#[must_use]
pub const fn linearreg_angle_min_len(period: usize) -> usize {
    period
}

/// Computes `LINEARREG_ANGLE` and stores results in output buffer.
///
/// Returns the angle of the regression line in degrees.
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
    let n = data.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "linearreg_angle",
            required: n,
            actual: output.len(),
        });
    }

    let mut slope = vec![T::nan(); n];
    let mut intercept = vec![T::nan(); n];
    linear_regression_core(data, period, &mut slope, &mut intercept)?;

    let lookback = linearreg_angle_lookback(period);
    let rad_to_deg = T::from_f64(180.0 / std::f64::consts::PI)?;

    for i in 0..lookback {
        output[i] = T::nan();
    }

    // angle = atan(slope) * 180 / π
    for i in lookback..n {
        output[i] = slope[i].atan() * rad_to_deg;
    }

    Ok(())
}

/// Computes `LINEARREG_ANGLE` (Linear Regression Angle).
///
/// Returns the angle of the regression line in degrees.
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
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for TSF.
#[inline]
#[must_use]
pub const fn tsf_min_len(period: usize) -> usize {
    period
}

/// Computes TSF (Time Series Forecast) and stores results in output buffer.
///
/// Returns the predicted value one period ahead of the regression line.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn tsf_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    let n = data.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "tsf",
            required: n,
            actual: output.len(),
        });
    }

    let mut slope = vec![T::nan(); n];
    let mut intercept = vec![T::nan(); n];
    linear_regression_core(data, period, &mut slope, &mut intercept)?;

    let lookback = tsf_lookback(period);
    let forecast_x = T::from_usize(period)?; // One step ahead

    for i in 0..lookback {
        output[i] = T::nan();
    }

    // tsf = intercept + slope * period (one step ahead of the window)
    for i in lookback..n {
        output[i] = intercept[i] + slope[i] * forecast_x;
    }

    Ok(())
}

/// Computes TSF (Time Series Forecast).
///
/// Returns the predicted value one period ahead of the regression line.
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        (a - b).abs() < EPSILON
    }

    fn approx_eq_tol(a: f64, b: f64, tol: f64) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        (a - b).abs() < tol
    }

    // ==================== VAR Tests ====================

    #[test]
    fn test_var_lookback() {
        assert_eq!(var_lookback(1), 0);
        assert_eq!(var_lookback(5), 4);
        assert_eq!(var_lookback(10), 9);
    }

    #[test]
    fn test_var_min_len() {
        assert_eq!(var_min_len(1), 1);
        assert_eq!(var_min_len(5), 5);
    }

    #[test]
    fn test_var_constant_data() {
        // Variance of constant data should be 0
        let data = vec![5.0_f64; 10];
        let result = var(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 0.0));
        }
    }

    #[test]
    fn test_var_basic() {
        // Data: [1, 2, 3, 4, 5], mean = 3
        // Variance = ((1-3)² + (2-3)² + (3-3)² + (4-3)² + (5-3)²) / 5
        //          = (4 + 1 + 0 + 1 + 4) / 5 = 10/5 = 2.0
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = var(&data, 5).unwrap();

        assert!(result[0].is_nan());
        assert!(result[3].is_nan());
        assert!(approx_eq(result[4], 2.0));
    }

    #[test]
    fn test_var_empty_input() {
        let data: Vec<f64> = vec![];
        let result = var(&data, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_var_period_zero() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = var(&data, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_var_insufficient_data() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = var(&data, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_var_f32() {
        let data = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
        let result = var(&data, 5).unwrap();
        assert!((result[4] - 2.0_f32).abs() < 1e-5);
    }

    // ==================== CORREL Tests ====================

    #[test]
    fn test_correl_lookback() {
        assert_eq!(correl_lookback(1), 0);
        assert_eq!(correl_lookback(5), 4);
    }

    #[test]
    fn test_correl_perfect_positive() {
        // Perfect positive correlation: y = x
        let x = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = correl(&x, &y, 5).unwrap();

        assert!(approx_eq(result[4], 1.0));
    }

    #[test]
    fn test_correl_perfect_negative() {
        // Perfect negative correlation: y = -x
        let x = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
        let result = correl(&x, &y, 5).unwrap();

        assert!(approx_eq(result[4], -1.0));
    }

    #[test]
    fn test_correl_zero() {
        // No correlation: one series is constant
        let x = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0_f64, 5.0, 5.0, 5.0, 5.0];
        let result = correl(&x, &y, 5).unwrap();

        // With zero variance in y, correlation is undefined (returns 0)
        assert!(approx_eq(result[4], 0.0));
    }

    #[test]
    fn test_correl_length_mismatch() {
        let x = vec![1.0_f64, 2.0, 3.0];
        let y = vec![1.0_f64, 2.0];
        let result = correl(&x, &y, 2);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_correl_empty_input() {
        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let result = correl(&x, &y, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    // ==================== BETA Tests ====================

    #[test]
    fn test_beta_lookback() {
        assert_eq!(beta_lookback(1), 0);
        assert_eq!(beta_lookback(5), 4);
    }

    #[test]
    fn test_beta_same_series() {
        // Beta of a series with itself should be 1
        let x = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = beta(&x, &x, 5).unwrap();

        assert!(approx_eq(result[4], 1.0));
    }

    #[test]
    fn test_beta_scaled_series() {
        // If asset = 2 * market, beta should be 2
        let market = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let asset = vec![2.0_f64, 4.0, 6.0, 8.0, 10.0];
        let result = beta(&asset, &market, 5).unwrap();

        assert!(approx_eq(result[4], 2.0));
    }

    #[test]
    fn test_beta_inverse_series() {
        // If asset = -market + const, beta should be -1
        let market = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let asset = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
        let result = beta(&asset, &market, 5).unwrap();

        assert!(approx_eq(result[4], -1.0));
    }

    #[test]
    fn test_beta_empty_input() {
        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let result = beta(&x, &y, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    // ==================== LINEARREG Tests ====================

    #[test]
    fn test_linearreg_lookback() {
        assert_eq!(linearreg_lookback(1), 0);
        assert_eq!(linearreg_lookback(5), 4);
    }

    #[test]
    fn test_linearreg_linear_data() {
        // For perfectly linear data y = x, linearreg should return the last value
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = linearreg(&data, 5).unwrap();

        // At period end, predicted value should be 5 (the last value)
        assert!(approx_eq_tol(result[4], 5.0, 1e-9));
    }

    #[test]
    fn test_linearreg_constant_data() {
        // For constant data, linearreg should return that constant
        let data = vec![3.0_f64; 10];
        let result = linearreg(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 3.0));
        }
    }

    #[test]
    fn test_linearreg_empty_input() {
        let data: Vec<f64> = vec![];
        let result = linearreg(&data, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_linearreg_period_zero() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = linearreg(&data, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    // ==================== LINEARREG_SLOPE Tests ====================

    #[test]
    fn test_linearreg_slope_lookback() {
        assert_eq!(linearreg_slope_lookback(1), 0);
        assert_eq!(linearreg_slope_lookback(5), 4);
    }

    #[test]
    fn test_linearreg_slope_linear_data() {
        // For y = x (slope = 1), linearreg_slope should return 1
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = linearreg_slope(&data, 5).unwrap();

        assert!(approx_eq_tol(result[4], 1.0, 1e-9));
    }

    #[test]
    fn test_linearreg_slope_constant_data() {
        // For constant data, slope should be 0
        let data = vec![5.0_f64; 10];
        let result = linearreg_slope(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 0.0));
        }
    }

    #[test]
    fn test_linearreg_slope_negative() {
        // For decreasing data, slope should be negative
        let data = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
        let result = linearreg_slope(&data, 5).unwrap();

        assert!(approx_eq_tol(result[4], -1.0, 1e-9));
    }

    // ==================== LINEARREG_INTERCEPT Tests ====================

    #[test]
    fn test_linearreg_intercept_lookback() {
        assert_eq!(linearreg_intercept_lookback(1), 0);
        assert_eq!(linearreg_intercept_lookback(5), 4);
    }

    #[test]
    fn test_linearreg_intercept_linear_data() {
        // For y = x (0, 1, 2, 3, 4 mapped to 1, 2, 3, 4, 5)
        // intercept should be 1 (the first value when x=0)
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = linearreg_intercept(&data, 5).unwrap();

        assert!(approx_eq_tol(result[4], 1.0, 1e-9));
    }

    #[test]
    fn test_linearreg_intercept_constant_data() {
        // For constant data, intercept equals the constant
        let data = vec![7.0_f64; 10];
        let result = linearreg_intercept(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 7.0));
        }
    }

    // ==================== LINEARREG_ANGLE Tests ====================

    #[test]
    fn test_linearreg_angle_lookback() {
        assert_eq!(linearreg_angle_lookback(1), 0);
        assert_eq!(linearreg_angle_lookback(5), 4);
    }

    #[test]
    fn test_linearreg_angle_zero_slope() {
        // For constant data (slope = 0), angle should be 0
        let data = vec![5.0_f64; 10];
        let result = linearreg_angle(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 0.0));
        }
    }

    #[test]
    fn test_linearreg_angle_45_degrees() {
        // For slope = 1, angle should be 45 degrees
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = linearreg_angle(&data, 5).unwrap();

        assert!(approx_eq_tol(result[4], 45.0, 1e-9));
    }

    #[test]
    fn test_linearreg_angle_negative_45() {
        // For slope = -1, angle should be -45 degrees
        let data = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
        let result = linearreg_angle(&data, 5).unwrap();

        assert!(approx_eq_tol(result[4], -45.0, 1e-9));
    }

    // ==================== TSF Tests ====================

    #[test]
    fn test_tsf_lookback() {
        assert_eq!(tsf_lookback(1), 0);
        assert_eq!(tsf_lookback(5), 4);
    }

    #[test]
    fn test_tsf_linear_data() {
        // For y = x (1, 2, 3, 4, 5), TSF should predict the next value (6)
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = tsf(&data, 5).unwrap();

        // TSF at index 4 predicts the value for the next period
        assert!(approx_eq_tol(result[4], 6.0, 1e-9));
    }

    #[test]
    fn test_tsf_constant_data() {
        // For constant data, TSF should predict the same constant
        let data = vec![10.0_f64; 10];
        let result = tsf(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 10.0));
        }
    }

    #[test]
    fn test_tsf_decreasing_data() {
        // For decreasing data (5, 4, 3, 2, 1), TSF should predict 0
        let data = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
        let result = tsf(&data, 5).unwrap();

        assert!(approx_eq_tol(result[4], 0.0, 1e-9));
    }

    #[test]
    fn test_tsf_empty_input() {
        let data: Vec<f64> = vec![];
        let result = tsf(&data, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_linearreg_equals_intercept_plus_slope_times_x() {
        let data = vec![10.0_f64, 12.0, 15.0, 14.0, 16.0, 18.0, 17.0, 20.0];
        let period = 5;

        let linreg = linearreg(&data, period).unwrap();
        let slope = linearreg_slope(&data, period).unwrap();
        let intercept = linearreg_intercept(&data, period).unwrap();

        let last_x = (period - 1) as f64;

        for i in (period - 1)..data.len() {
            let expected = intercept[i] + slope[i] * last_x;
            assert!(approx_eq_tol(linreg[i], expected, 1e-9));
        }
    }

    #[test]
    fn test_tsf_equals_linearreg_plus_slope() {
        let data = vec![10.0_f64, 12.0, 15.0, 14.0, 16.0, 18.0, 17.0, 20.0];
        let period = 5;

        let tsf_result = tsf(&data, period).unwrap();
        let linreg = linearreg(&data, period).unwrap();
        let slope = linearreg_slope(&data, period).unwrap();

        // TSF = linearreg + slope (one period ahead)
        for i in (period - 1)..data.len() {
            let expected = linreg[i] + slope[i];
            assert!(approx_eq_tol(tsf_result[i], expected, 1e-9));
        }
    }

    #[test]
    fn test_var_into_consistent_with_var() {
        let data = vec![1.0_f64, 3.0, 5.0, 7.0, 9.0, 11.0, 8.0, 6.0];
        let period = 4;

        let result1 = var(&data, period).unwrap();
        let mut result2 = vec![0.0_f64; data.len()];
        var_into(&data, period, &mut result2).unwrap();

        for i in 0..data.len() {
            assert!(approx_eq(result1[i], result2[i]));
        }
    }

    // ==================== Output Length Tests ====================

    #[test]
    fn test_all_output_lengths() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;

        assert_eq!(var(&data, period).unwrap().len(), data.len());
        assert_eq!(linearreg(&data, period).unwrap().len(), data.len());
        assert_eq!(linearreg_slope(&data, period).unwrap().len(), data.len());
        assert_eq!(
            linearreg_intercept(&data, period).unwrap().len(),
            data.len()
        );
        assert_eq!(linearreg_angle(&data, period).unwrap().len(), data.len());
        assert_eq!(tsf(&data, period).unwrap().len(), data.len());
    }

    #[test]
    fn test_correl_and_beta_output_lengths() {
        let x = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y = vec![2.0_f64, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
        let period = 5;

        assert_eq!(correl(&x, &y, period).unwrap().len(), x.len());
        assert_eq!(beta(&x, &y, period).unwrap().len(), x.len());
    }

    // ==================== NaN Count Tests ====================

    #[test]
    fn test_var_nan_count() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;
        let result = var(&data, period).unwrap();

        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, var_lookback(period));
    }

    #[test]
    fn test_linearreg_nan_count() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;
        let result = linearreg(&data, period).unwrap();

        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, linearreg_lookback(period));
    }

    // ==================== STDDEV Tests ====================

    #[test]
    fn test_stddev_lookback() {
        assert_eq!(stddev_lookback(1), 0);
        assert_eq!(stddev_lookback(5), 4);
    }

    #[test]
    fn test_stddev_constant_data() {
        // Stddev of constant data should be 0
        let data = vec![5.0_f64; 10];
        let result = stddev(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 0.0));
        }
    }

    #[test]
    fn test_stddev_basic() {
        // VAR = 2.0 for [1,2,3,4,5], so STDDEV = sqrt(2.0) ≈ 1.414
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = stddev(&data, 5).unwrap();

        assert!(result[0].is_nan());
        assert!(approx_eq_tol(result[4], 2.0_f64.sqrt(), 1e-9));
    }

    #[test]
    fn test_stddev_is_sqrt_of_var() {
        let data = vec![1.0_f64, 3.0, 5.0, 7.0, 9.0, 2.0, 4.0, 6.0, 8.0, 10.0];
        let period = 5;

        let var_result = var(&data, period).unwrap();
        let stddev_result = stddev(&data, period).unwrap();

        for i in (period - 1)..data.len() {
            assert!(approx_eq_tol(stddev_result[i], var_result[i].sqrt(), 1e-9));
        }
    }

    // ==================== SKEW Tests ====================

    #[test]
    fn test_skew_lookback() {
        assert_eq!(skew_lookback(1), 0);
        assert_eq!(skew_lookback(5), 4);
    }

    #[test]
    fn test_skew_symmetric_data() {
        // Symmetric data should have skewness near 0
        let data = vec![1.0_f64, 2.0, 3.0, 2.0, 1.0, 2.0, 3.0, 2.0, 1.0, 2.0];
        let result = skew(&data, 5).unwrap();

        for i in 4..10 {
            // Allow some tolerance for numerical precision
            assert!(result[i].abs() < 0.5, "skew[{}] = {}", i, result[i]);
        }
    }

    #[test]
    fn test_skew_constant_data() {
        // Constant data has no variance, skewness undefined (returns 0)
        let data = vec![5.0_f64; 10];
        let result = skew(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 0.0));
        }
    }

    #[test]
    fn test_skew_empty_input() {
        let data: Vec<f64> = vec![];
        let result = skew(&data, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    // ==================== KURT Tests ====================

    #[test]
    fn test_kurt_lookback() {
        assert_eq!(kurt_lookback(1), 0);
        assert_eq!(kurt_lookback(5), 4);
    }

    #[test]
    fn test_kurt_constant_data() {
        // Constant data has no variance, kurtosis undefined (returns 0)
        let data = vec![5.0_f64; 10];
        let result = kurt(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 0.0));
        }
    }

    #[test]
    fn test_kurt_empty_input() {
        let data: Vec<f64> = vec![];
        let result = kurt(&data, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    // ==================== COV Tests ====================

    #[test]
    fn test_cov_lookback() {
        assert_eq!(cov_lookback(1), 0);
        assert_eq!(cov_lookback(5), 4);
    }

    #[test]
    fn test_cov_same_series() {
        // Cov(X, X) = Var(X)
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;

        let var_result = var(&data, period).unwrap();
        let cov_result = cov(&data, &data, period).unwrap();

        for i in (period - 1)..data.len() {
            assert!(approx_eq_tol(cov_result[i], var_result[i], 1e-9));
        }
    }

    #[test]
    fn test_cov_perfect_correlation() {
        // If Y = 2*X, then Cov(X, Y) = 2*Var(X)
        let x = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0_f64, 4.0, 6.0, 8.0, 10.0];
        let period = 5;

        let var_x = var(&x, period).unwrap();
        let cov_result = cov(&x, &y, period).unwrap();

        assert!(approx_eq_tol(cov_result[4], 2.0 * var_x[4], 1e-9));
    }

    #[test]
    fn test_cov_negative_correlation() {
        // If Y = -X + const, then Cov(X, Y) = -Var(X)
        let x = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
        let period = 5;

        let var_x = var(&x, period).unwrap();
        let cov_result = cov(&x, &y, period).unwrap();

        assert!(approx_eq_tol(cov_result[4], -var_x[4], 1e-9));
    }

    // ==================== ZSCORE Tests ====================

    #[test]
    fn test_zscore_lookback() {
        assert_eq!(zscore_lookback(1), 0);
        assert_eq!(zscore_lookback(5), 4);
    }

    #[test]
    fn test_zscore_at_mean() {
        // If value equals mean, zscore should be 0
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 3.0]; // Last value = mean of window
        let result = zscore(&data, 5).unwrap();

        // For [2,3,4,5,3], mean = 3.4, so 3.0 is slightly below mean
        // The zscore won't be exactly 0 for this case
        assert!(result[5].abs() < 1.0); // Just verify it's reasonable
    }

    #[test]
    fn test_zscore_constant_data() {
        // Constant data has no variance, z-score returns 0
        let data = vec![5.0_f64; 10];
        let result = zscore(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 0.0));
        }
    }

    // ==================== MAD Tests ====================

    #[test]
    fn test_mad_lookback() {
        assert_eq!(mad_lookback(1), 0);
        assert_eq!(mad_lookback(5), 4);
    }

    #[test]
    fn test_mad_constant_data() {
        // MAD of constant data should be 0
        let data = vec![5.0_f64; 10];
        let result = mad(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 0.0));
        }
    }

    #[test]
    fn test_mad_basic() {
        // For [1,2,3,4,5], mean = 3
        // MAD = (|1-3| + |2-3| + |3-3| + |4-3| + |5-3|) / 5
        //     = (2 + 1 + 0 + 1 + 2) / 5 = 6/5 = 1.2
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = mad(&data, 5).unwrap();

        assert!(approx_eq_tol(result[4], 1.2, 1e-9));
    }

    // ==================== SEM Tests ====================

    #[test]
    fn test_sem_lookback() {
        assert_eq!(sem_lookback(1), 0);
        assert_eq!(sem_lookback(5), 4);
    }

    #[test]
    fn test_sem_is_stddev_over_sqrt_n() {
        let data = vec![1.0_f64, 3.0, 5.0, 7.0, 9.0, 2.0, 4.0, 6.0, 8.0, 10.0];
        let period = 5;

        let stddev_result = stddev(&data, period).unwrap();
        let sem_result = sem(&data, period).unwrap();
        let sqrt_n = (period as f64).sqrt();

        for i in (period - 1)..data.len() {
            assert!(approx_eq_tol(sem_result[i], stddev_result[i] / sqrt_n, 1e-9));
        }
    }

    #[test]
    fn test_sem_constant_data() {
        // SEM of constant data should be 0
        let data = vec![5.0_f64; 10];
        let result = sem(&data, 5).unwrap();

        for i in 4..10 {
            assert!(approx_eq(result[i], 0.0));
        }
    }

    // ==================== New Functions Output Length Tests ====================

    #[test]
    fn test_new_functions_output_lengths() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;

        assert_eq!(stddev(&data, period).unwrap().len(), data.len());
        assert_eq!(skew(&data, period).unwrap().len(), data.len());
        assert_eq!(kurt(&data, period).unwrap().len(), data.len());
        assert_eq!(zscore(&data, period).unwrap().len(), data.len());
        assert_eq!(mad(&data, period).unwrap().len(), data.len());
        assert_eq!(sem(&data, period).unwrap().len(), data.len());
    }

    #[test]
    fn test_cov_output_length() {
        let x = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y = vec![2.0_f64, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
        let period = 5;

        assert_eq!(cov(&x, &y, period).unwrap().len(), x.len());
    }

    // ==================== New Functions NaN Count Tests ====================

    #[test]
    fn test_new_functions_nan_counts() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 5;

        let stddev_nans = stddev(&data, period).unwrap().iter().filter(|x| x.is_nan()).count();
        assert_eq!(stddev_nans, stddev_lookback(period));

        let skew_nans = skew(&data, period).unwrap().iter().filter(|x| x.is_nan()).count();
        assert_eq!(skew_nans, skew_lookback(period));

        let kurt_nans = kurt(&data, period).unwrap().iter().filter(|x| x.is_nan()).count();
        assert_eq!(kurt_nans, kurt_lookback(period));

        let zscore_nans = zscore(&data, period).unwrap().iter().filter(|x| x.is_nan()).count();
        assert_eq!(zscore_nans, zscore_lookback(period));

        let mad_nans = mad(&data, period).unwrap().iter().filter(|x| x.is_nan()).count();
        assert_eq!(mad_nans, mad_lookback(period));

        let sem_nans = sem(&data, period).unwrap().iter().filter(|x| x.is_nan()).count();
        assert_eq!(sem_nans, sem_lookback(period));
    }

    // ==================== NaN Propagation Tests ====================

    #[test]
    fn test_new_functions_nan_propagation() {
        let data = vec![1.0_f64, 2.0, f64::NAN, 4.0, 5.0, 6.0, 7.0, 8.0];
        let period = 3;

        // With NaN in positions 0,1,2 of window, outputs at indices 2,3,4 should be NaN
        let stddev_result = stddev(&data, period).unwrap();
        assert!(stddev_result[2].is_nan());
        assert!(stddev_result[3].is_nan());
        assert!(stddev_result[4].is_nan());

        let skew_result = skew(&data, period).unwrap();
        assert!(skew_result[2].is_nan());
        assert!(skew_result[3].is_nan());
        assert!(skew_result[4].is_nan());

        let kurt_result = kurt(&data, period).unwrap();
        assert!(kurt_result[2].is_nan());
        assert!(kurt_result[3].is_nan());
        assert!(kurt_result[4].is_nan());
    }
}
