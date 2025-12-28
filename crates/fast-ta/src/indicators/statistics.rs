//! Statistical Functions
//!
//! This module provides rolling statistical functions commonly used in technical analysis.
//!
//! # Indicators
//!
//! - [`var`] - Variance (population)
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
use crate::utils::is_invalid;

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
    let has_nan = data.iter().take(n).any(|&v| is_invalid(v));

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
        if is_invalid(data[i]) {
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

        if is_invalid(old_val) {
            invalid_count = invalid_count.saturating_sub(1);
        } else {
            sum = sum - old_val;
            sum_sq = sum_sq - old_val * old_val;
        }
        if is_invalid(new_val) {
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
            if is_invalid(data0[j]) || is_invalid(data1[j]) {
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
            if is_invalid(data0[j]) || is_invalid(data1[j]) {
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
            if is_invalid(data[j]) {
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
}
