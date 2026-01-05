//! Bollinger Bands indicator.
//!
//! Bollinger Bands are a volatility indicator that consists of three bands:
//! - **Middle Band**: Simple Moving Average (SMA) of the price
//! - **Upper Band**: Middle Band + (k × standard deviation)
//! - **Lower Band**: Middle Band - (k × standard deviation)
//!
//! Where `k` is typically 2 (two standard deviations).
//!
//! # Algorithm
//!
//! This implementation uses an O(n) rolling calculation approach where:
//! 1. Rolling SMA is computed using the rolling sum method
//! 2. Rolling standard deviation is computed using Welford's algorithm (High mode)
//!    or sum-of-squares (Fast mode)
//! 3. Upper and lower bands are computed as middle ± k × stddev
//!
//! # Mathematical Conventions (PRD §4.8)
//!
//! - **Population Standard Deviation**: Uses ÷n, not ÷(n-1). This matches TA-Lib and most
//!   financial charting platforms. Users migrating from sample-stddev implementations
//!   (e.g., Excel) may see slightly narrower bands.
//! - **Variance Algorithm**: Welford's algorithm is used in High precision mode for
//!   improved numerical stability. Sum-of-squares is used in Fast mode for performance.
//! - **Precision Note**: Welford's algorithm handles large mean with small variance
//!   without catastrophic cancellation.
//!
//! # Precision Behavior
//!
//! When `PrecisionMode::High` is active and input type is `f32`:
//! - Rolling sum and sum-of-squares accumulators use `f64` internally
//! - Variance computation performed in `f64`, then converted to `f32`
//! - Significantly reduces catastrophic cancellation for near-constant data
//!
//! **Tolerance**: hybrid(rel=1e-5, abs=1e-7) when comparing f32 High mode to f64 reference.
//!
//! For maximum precision with variance-sensitive applications, use `f64` input directly.
//!
//! # Formula
//!
//! ```text
//! Middle Band = SMA(price, period)
//! Standard Deviation = sqrt(sum((price - SMA)^2) / period)  // population stddev (÷n)
//! Upper Band = Middle Band + (k × Standard Deviation)
//! Lower Band = Middle Band - (k × Standard Deviation)
//! ```
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::bollinger::{bollinger, BollingerOutput};
//!
//! let data = vec![20.0_f64, 21.0, 22.0, 21.5, 22.5, 23.0, 22.0, 21.0, 20.5, 21.5];
//! let result = bollinger(&data, 5, 2.0).unwrap();
//!
//! // First 4 values (period - 1) are NaN
//! assert!(result.middle[0].is_nan());
//! assert!(result.middle[3].is_nan());
//!
//! // From index 4 (period - 1), we have valid values
//! assert!(!result.middle[4].is_nan());
//! assert!(!result.upper[4].is_nan());
//! assert!(!result.lower[4].is_nan());
//!
//! // Upper > Middle > Lower for any non-zero volatility
//! assert!(result.upper[4] > result.middle[4]);
//! assert!(result.middle[4] > result.lower[4]);
//! ```

use num_traits::Float;

use crate::error::{Error, Result};
use crate::kernels::accumulators::WelfordVarianceF64;
use crate::kernels::simd;
use crate::precision::{current_precision_mode, PrecisionMode};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns true if we should use f64 accumulators for the given type.
///
/// Uses f64 accumulators when:
/// - Input type is f32 AND PrecisionMode is High
#[inline]
fn use_f64_accumulator<T: 'static>() -> bool {
    use std::any::TypeId;
    TypeId::of::<T>() == TypeId::of::<f32>() && current_precision_mode() == PrecisionMode::High
}

/// Helper to compute initial sum and sum of squares with NaN count using SIMD for f64.
///
/// Uses the NaN-aware SIMD kernel `sum_and_sum_sq_and_count_f64` which properly
/// excludes NaN values from sums while counting them. Provides 2-4x speedup
/// for both clean and NaN-containing data.
#[inline]
fn compute_initial_sums<T: SeriesElement + 'static>(
    data: &[T],
    period: usize,
) -> (T, T, usize) {
    use std::any::TypeId;
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        // SAFETY: We verified T is f64 via TypeId
        let data_f64: &[f64] = unsafe { &*(data as *const [T] as *const [f64]) };
        let window = &data_f64[..period];

        // Use NaN-aware SIMD kernel that excludes NaN from sums
        let (sum, sum_sq, valid_count) = simd::sum_and_sum_sq_and_count_f64(window);
        let nan_count = period - valid_count;

        // SAFETY: We verified T is f64 via TypeId
        let sum_t: T = unsafe { *(&sum as *const f64 as *const T) };
        let sum_sq_t: T = unsafe { *(&sum_sq as *const f64 as *const T) };
        return (sum_t, sum_sq_t, nan_count);
    }
    // Fallback to scalar for f32 (no SIMD kernel yet)
    let mut sum = T::zero();
    let mut sum_sq = T::zero();
    let mut nan_count = 0usize;
    for &value in data.iter().take(period) {
        if is_invalid(value) {
            nan_count += 1;
        } else {
            sum = sum + value;
            sum_sq = sum_sq + value * value;
        }
    }
    (sum, sum_sq, nan_count)
}

/// Returns the lookback period for Bollinger Bands.
///
/// The lookback is the number of NaN values at the start of the output.
/// For Bollinger Bands, this is `period - 1`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::bollinger::bollinger_lookback;
///
/// assert_eq!(bollinger_lookback(20), 19);
/// assert_eq!(bollinger_lookback(5), 4);
/// ```
#[inline]
#[must_use]
pub const fn bollinger_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for Bollinger Bands.
///
/// This is the smallest input size that will produce at least one valid output.
/// For Bollinger Bands, this equals the period.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::bollinger::bollinger_min_len;
///
/// assert_eq!(bollinger_min_len(20), 20);
/// assert_eq!(bollinger_min_len(5), 5);
/// ```
#[inline]
#[must_use]
pub const fn bollinger_min_len(period: usize) -> usize {
    period
}

/// Output structure containing all three Bollinger Bands.
///
/// Each vector has the same length as the input data. The first `period - 1`
/// values are NaN due to insufficient lookback data.
#[derive(Debug, Clone)]
pub struct BollingerOutput<T> {
    /// The middle band (Simple Moving Average).
    pub middle: Vec<T>,
    /// The upper band (middle + k × stddev).
    pub upper: Vec<T>,
    /// The lower band (middle - k × stddev).
    pub lower: Vec<T>,
}

/// Computes Bollinger Bands for a data series.
///
/// Returns the middle band (SMA), upper band, and lower band using the specified
/// period and number of standard deviations.
///
/// # Arguments
///
/// * `data` - The input data series (typically closing prices)
/// * `period` - The number of periods for the SMA and standard deviation (commonly 20)
/// * `num_std_dev` - The number of standard deviations for the bands (commonly 2.0)
///
/// # Returns
///
/// A `Result` containing a [`BollingerOutput`] with middle, upper, and lower bands,
/// or an error if validation fails.
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
/// - Space complexity: O(n) for each of the three output vectors
///
/// # NaN Handling
///
/// - The first `period - 1` elements of all outputs are NaN
/// - If any input value in the current window contains NaN, it will propagate to the output
///
/// # Example
///
/// ```
/// use fast_ta::indicators::bollinger::bollinger;
///
/// // Standard Bollinger Bands with period 20 and 2 standard deviations
/// let mut data: Vec<f64> = Vec::with_capacity(50);
/// for x in 0..50 {
///     data.push(100.0 + (x as f64 * 0.1));
/// }
/// let result = bollinger(&data, 20, 2.0).unwrap();
///
/// // Check that we have valid values after warmup
/// assert!(!result.middle[19].is_nan());
/// assert!(!result.upper[19].is_nan());
/// assert!(!result.lower[19].is_nan());
/// ```
#[must_use = "this returns a Result with Bollinger Bands values, which should be used"]
pub fn bollinger<T: SeriesElement + 'static>(
    data: &[T],
    period: usize,
    num_std_dev: T,
) -> Result<BollingerOutput<T>> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "bollinger",
        });
    }

    // Initialize output vectors with NaN
    let mut middle = vec![T::nan(); data.len()];
    let mut upper = vec![T::nan(); data.len()];
    let mut lower = vec![T::nan(); data.len()];

    // Use f64 accumulator for f32 inputs in High precision mode
    if use_f64_accumulator::<T>() {
        bollinger_f64_accum(data, period, num_std_dev, &mut middle, &mut upper, &mut lower)?;
    } else {
        bollinger_native_accum(data, period, num_std_dev, &mut middle, &mut upper, &mut lower)?;
    }

    Ok(BollingerOutput {
        middle,
        upper,
        lower,
    })
}

/// Bollinger Bands using Welford f64 accumulator for improved precision.
///
/// This is used for f32 inputs in High precision mode.
/// Uses Welford's algorithm for numerically stable variance computation.
#[inline]
fn bollinger_f64_accum<T: SeriesElement>(
    data: &[T],
    period: usize,
    num_std_dev: T,
    middle: &mut [T],
    upper: &mut [T],
    lower: &mut [T],
) -> Result<()> {
    let num_std_dev_f64 = num_std_dev.to_f64().unwrap_or(2.0);

    // Initialize Welford accumulator by pushing first window values
    let mut accum = WelfordVarianceF64::new();
    let mut nan_count = 0usize;

    for &value in data.iter().take(period) {
        if is_invalid(value) {
            nan_count += 1;
        } else {
            accum.push(value.to_f64().unwrap_or(0.0));
        }
    }

    // Compute first valid values at index (period - 1)
    let first_idx = period - 1;
    if nan_count == 0 {
        let mean = accum.mean();
        let stddev = accum.population_stddev();

        middle[first_idx] = T::from_f64(mean)?;
        upper[first_idx] = T::from_f64(mean + num_std_dev_f64 * stddev)?;
        lower[first_idx] = T::from_f64(mean - num_std_dev_f64 * stddev)?;
    }

    // Rolling calculation for remaining elements
    for i in period..data.len() {
        let old_value = data[i - period];
        let new_value = data[i];

        // Update Welford accumulator and NaN count
        if is_invalid(new_value) {
            nan_count += 1;
        } else {
            accum.push(new_value.to_f64().unwrap_or(0.0));
        }

        if is_invalid(old_value) {
            nan_count -= 1;
        } else {
            accum.pop(old_value.to_f64().unwrap_or(0.0));
        }

        // Compute bands only when window has no NaNs
        if nan_count == 0 {
            let mean = accum.mean();
            let stddev = accum.population_stddev();

            middle[i] = T::from_f64(mean)?;
            upper[i] = T::from_f64(mean + num_std_dev_f64 * stddev)?;
            lower[i] = T::from_f64(mean - num_std_dev_f64 * stddev)?;
        } else {
            middle[i] = T::nan();
            upper[i] = T::nan();
            lower[i] = T::nan();
        }
    }

    Ok(())
}

/// Bollinger Bands using native type accumulator.
///
/// This is used for f64 inputs or Fast precision mode.
#[inline]
fn bollinger_native_accum<T: SeriesElement + 'static>(
    data: &[T],
    period: usize,
    num_std_dev: T,
    middle: &mut [T],
    upper: &mut [T],
    lower: &mut [T],
) -> Result<()> {
    // Pre-compute reciprocal for efficient division
    let inv_period = T::one() / T::from_usize(period)?;

    // Compute initial sum and sum of squares using SIMD for f64
    let (mut sum, mut sum_sq, mut nan_count) = compute_initial_sums(data, period);

    // Compute first valid values at index (period - 1)
    let first_idx = period - 1;
    if nan_count == 0 {
        let mean = sum * inv_period;
        let variance = compute_variance(sum_sq, sum, inv_period);
        let stddev = variance.sqrt();

        middle[first_idx] = mean;
        upper[first_idx] = mean + num_std_dev * stddev;
        lower[first_idx] = mean - num_std_dev * stddev;
    }

    // Rolling calculation for remaining elements
    for i in period..data.len() {
        let old_value = data[i - period];
        let new_value = data[i];

        // Update rolling sums and NaN count
        if is_invalid(new_value) {
            nan_count += 1;
        } else {
            sum = sum + new_value;
            sum_sq = sum_sq + new_value * new_value;
        }

        if is_invalid(old_value) {
            nan_count -= 1;
        } else {
            sum = sum - old_value;
            sum_sq = sum_sq - old_value * old_value;
        }

        // Compute bands only when window has no NaNs
        if nan_count == 0 {
            let mean = sum * inv_period;
            let variance = compute_variance(sum_sq, sum, inv_period);
            let stddev = variance.sqrt();

            middle[i] = mean;
            upper[i] = mean + num_std_dev * stddev;
            lower[i] = mean - num_std_dev * stddev;
        } else {
            middle[i] = T::nan();
            upper[i] = T::nan();
            lower[i] = T::nan();
        }
    }

    Ok(())
}

/// Computes Bollinger Bands into pre-allocated output buffers.
///
/// This variant allows reusing existing buffers to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods for the SMA and standard deviation
/// * `num_std_dev` - The number of standard deviations for the bands
/// * `output` - Pre-allocated output structure (each vector must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid values computed (`data.len()` - period + 1),
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
/// - Any output buffer is shorter than the input data
///
/// # Example
///
/// ```
/// use fast_ta::indicators::bollinger::{bollinger_into, BollingerOutput};
///
/// let data = vec![20.0, 21.0, 22.0, 21.5, 22.5];
/// let mut output = BollingerOutput {
///     middle: vec![0.0_f64; 5],
///     upper: vec![0.0_f64; 5],
///     lower: vec![0.0_f64; 5],
/// };
/// let valid_count = bollinger_into(&data, 3, 2.0, &mut output).unwrap();
///
/// assert_eq!(valid_count, 3);
/// ```
#[must_use = "this returns a Result with the valid count, which should be used"]
pub fn bollinger_into<T: SeriesElement + 'static>(
    data: &[T],
    period: usize,
    num_std_dev: T,
    output: &mut BollingerOutput<T>,
) -> Result<usize> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "bollinger",
        });
    }

    // Validate output buffers
    if output.middle.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.middle.len(),
            indicator: "bollinger",
        });
    }
    if output.upper.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.upper.len(),
            indicator: "bollinger",
        });
    }
    if output.lower.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.lower.len(),
            indicator: "bollinger",
        });
    }

    // Pre-compute reciprocal for efficient division
    let inv_period = T::one() / T::from_usize(period)?;

    // Initialize lookback period with NaN
    for i in 0..(period - 1) {
        output.middle[i] = T::nan();
        output.upper[i] = T::nan();
        output.lower[i] = T::nan();
    }

    // Compute initial sum and sum of squares using SIMD for f64
    let (mut sum, mut sum_sq, mut nan_count) = compute_initial_sums(data, period);

    // Compute first valid values at index (period - 1)
    let first_idx = period - 1;
    if nan_count == 0 {
        let mean = sum * inv_period;
        let variance = compute_variance(sum_sq, sum, inv_period);
        let stddev = variance.sqrt();

        output.middle[first_idx] = mean;
        output.upper[first_idx] = mean + num_std_dev * stddev;
        output.lower[first_idx] = mean - num_std_dev * stddev;
    } else {
        output.middle[first_idx] = T::nan();
        output.upper[first_idx] = T::nan();
        output.lower[first_idx] = T::nan();
    }

    // Rolling calculation for remaining elements
    for i in period..data.len() {
        let old_value = data[i - period];
        let new_value = data[i];

        // Update rolling sums and NaN count
        if is_invalid(new_value) {
            nan_count += 1;
        } else {
            sum = sum + new_value;
            sum_sq = sum_sq + new_value * new_value;
        }

        if is_invalid(old_value) {
            nan_count -= 1;
        } else {
            sum = sum - old_value;
            sum_sq = sum_sq - old_value * old_value;
        }

        // Compute bands only when window has no NaNs
        if nan_count == 0 {
            let mean = sum * inv_period;
            let variance = compute_variance(sum_sq, sum, inv_period);
            let stddev = variance.sqrt();

            output.middle[i] = mean;
            output.upper[i] = mean + num_std_dev * stddev;
            output.lower[i] = mean - num_std_dev * stddev;
        } else {
            output.middle[i] = T::nan();
            output.upper[i] = T::nan();
            output.lower[i] = T::nan();
        }
    }

    // Return count of valid (non-NaN) values
    Ok(data.len() - period + 1)
}

/// Computes the rolling standard deviation using pre-computed sums.
///
/// This is a separate function for computing just the standard deviation
/// component, which is useful for other indicators that need volatility
/// calculations.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods for the standard deviation
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` with the rolling standard deviation values,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
///
/// # Example
///
/// ```
/// use fast_ta::indicators::bollinger::rolling_stddev;
///
/// let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
/// let result = rolling_stddev(&data, 3).unwrap();
///
/// // First 2 values are NaN
/// assert!(result[0].is_nan());
/// assert!(result[1].is_nan());
/// // Standard deviation of [1,2,3] = sqrt(2/3) ≈ 0.8165
/// assert!((result[2] - 0.816496580927726).abs() < 1e-10);
/// ```
#[must_use = "this returns a Result with the rolling standard deviation values, which should be used"]
pub fn rolling_stddev<T: SeriesElement + 'static>(data: &[T], period: usize) -> Result<Vec<T>> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "rolling_stddev",
        });
    }

    // Pre-compute reciprocal for efficient division
    let inv_period = T::one() / T::from_usize(period)?;

    // Initialize output vector with NaN
    let mut result = vec![T::nan(); data.len()];

    // Compute initial sum and sum of squares using SIMD for f64
    let (mut sum, mut sum_sq, mut nan_count) = compute_initial_sums(data, period);

    // Compute first valid value at index (period - 1)
    let first_idx = period - 1;
    if nan_count == 0 {
        let variance = compute_variance(sum_sq, sum, inv_period);
        result[first_idx] = variance.sqrt();
    } else {
        result[first_idx] = T::nan();
    }

    // Rolling calculation for remaining elements
    for i in period..data.len() {
        let old_value = data[i - period];
        let new_value = data[i];

        // Update rolling sums and NaN count
        if is_invalid(new_value) {
            nan_count += 1;
        } else {
            sum = sum + new_value;
            sum_sq = sum_sq + new_value * new_value;
        }

        if is_invalid(old_value) {
            nan_count -= 1;
        } else {
            sum = sum - old_value;
            sum_sq = sum_sq - old_value * old_value;
        }

        // Compute standard deviation only when window has no NaNs
        if nan_count == 0 {
            let variance = compute_variance(sum_sq, sum, inv_period);
            result[i] = variance.sqrt();
        } else {
            result[i] = T::nan();
        }
    }

    Ok(result)
}

/// Computes the rolling standard deviation into a pre-allocated output buffer.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The number of periods for the standard deviation
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid values computed (`data.len()` - period + 1),
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
/// - The output buffer is shorter than the input data
#[must_use = "this returns a Result with the valid count, which should be used"]
pub fn rolling_stddev_into<T: SeriesElement + 'static>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "rolling_stddev",
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "rolling_stddev",
        });
    }

    // Pre-compute reciprocal for efficient division
    let inv_period = T::one() / T::from_usize(period)?;

    // Initialize lookback period with NaN
    for item in output.iter_mut().take(period - 1) {
        *item = T::nan();
    }

    // Compute initial sum and sum of squares using SIMD for f64
    let (mut sum, mut sum_sq, mut nan_count) = compute_initial_sums(data, period);

    // Compute first valid value at index (period - 1)
    let first_idx = period - 1;
    if nan_count == 0 {
        let variance = compute_variance(sum_sq, sum, inv_period);
        output[first_idx] = variance.sqrt();
    } else {
        output[first_idx] = T::nan();
    }

    // Rolling calculation for remaining elements
    for i in period..data.len() {
        let old_value = data[i - period];
        let new_value = data[i];

        // Update rolling sums and NaN count
        if is_invalid(new_value) {
            nan_count += 1;
        } else {
            sum = sum + new_value;
            sum_sq = sum_sq + new_value * new_value;
        }

        if is_invalid(old_value) {
            nan_count -= 1;
        } else {
            sum = sum - old_value;
            sum_sq = sum_sq - old_value * old_value;
        }

        // Compute standard deviation only when window has no NaNs
        if nan_count == 0 {
            let variance = compute_variance(sum_sq, sum, inv_period);
            output[i] = variance.sqrt();
        } else {
            output[i] = T::nan();
        }
    }

    // Return count of valid (non-NaN) values
    Ok(data.len() - period + 1)
}

/// Computes variance from sum of squares and sum using the population variance formula.
///
/// Formula: Var = (`sum_sq` / n) - (sum / n)^2 = (`sum_sq` - sum^2/n) / n
///
/// We use the population variance (divide by n, not n-1) since Bollinger Bands
/// typically use population standard deviation.
#[inline]
fn compute_variance<T: Float>(sum_sq: T, sum: T, inv_period: T) -> T {
    let mean = sum * inv_period;
    let mean_sq = mean * mean;
    let mean_of_squares = sum_sq * inv_period;
    // Ensure non-negative variance (floating-point rounding can cause tiny negatives)
    let variance = mean_of_squares - mean_sq;
    if variance < T::zero() {
        T::zero()
    } else {
        variance
    }
}

// ==================== Configuration Type ====================

/// Bollinger Bands configuration with fluent builder API.
///
/// Provides sensible defaults (period=20, `std_dev=2.0`) and fluent setters
/// for customization. Implements `Default` for zero-config usage per
/// Gravity Check 1.1.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::bollinger::Bollinger;
///
/// let prices = vec![
///     44.0_f64, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0,
///     45.5, 44.5, 43.5, 44.0, 45.0, 46.0, 46.5, 45.5, 44.5, 45.0,
/// ];
///
/// // Use defaults (20, 2.0)
/// let result = Bollinger::default().compute(&prices).unwrap();
///
/// // Or customize with fluent API
/// let result = Bollinger::new()
///     .with_period(10)
///     .with_std_dev(2.5)
///     .compute(&prices)
///     .unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Bollinger {
    period: usize,
    std_dev: f64,
}

impl Default for Bollinger {
    /// Creates a Bollinger Bands configuration with standard parameters (20, 2.0).
    fn default() -> Self {
        Self {
            period: 20,
            std_dev: 2.0,
        }
    }
}

impl Bollinger {
    /// Creates a new Bollinger Bands configuration with standard parameters (20, 2.0).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the lookback period.
    ///
    /// Default: 20
    #[must_use]
    pub const fn with_period(mut self, period: usize) -> Self {
        self.period = period;
        self
    }

    /// Sets the standard deviation multiplier for the bands.
    ///
    /// Default: 2.0
    #[must_use]
    pub const fn with_std_dev(mut self, std_dev: f64) -> Self {
        self.std_dev = std_dev;
        self
    }

    /// Computes Bollinger Bands using the configured parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The input data is empty
    /// - Period is 0
    /// - Insufficient data for the configured period
    pub fn compute<T: SeriesElement>(&self, data: &[T]) -> Result<BollingerOutput<T>> {
        let std_dev = T::from_f64(self.std_dev)?;
        bollinger(data, self.period, std_dev)
    }

    /// Computes Bollinger Bands into a pre-allocated output struct.
    ///
    /// Returns the number of valid values.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Output buffers are smaller than input length
    /// - The input data is empty
    /// - Period is 0
    /// - Insufficient data for the configured period
    pub fn compute_into<T: SeriesElement>(
        &self,
        data: &[T],
        output: &mut BollingerOutput<T>,
    ) -> Result<usize> {
        let std_dev = T::from_f64(self.std_dev)?;
        bollinger_into(data, self.period, std_dev, output)
    }

    /// Returns the period.
    #[must_use]
    pub const fn period(&self) -> usize {
        self.period
    }

    /// Returns the standard deviation multiplier.
    #[must_use]
    pub const fn std_dev(&self) -> f64 {
        self.std_dev
    }

    /// Returns the lookback for this configuration.
    #[must_use]
    pub const fn lookback(&self) -> usize {
        bollinger_lookback(self.period)
    }

    /// Returns the minimum input length for this configuration.
    #[must_use]
    pub const fn min_len(&self) -> usize {
        bollinger_min_len(self.period)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

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
    fn test_bollinger_basic() {
        let data = vec![
            20.0_f64, 21.0, 22.0, 21.5, 22.5, 23.0, 22.0, 21.0, 20.5, 21.5,
        ];
        let result = bollinger(&data, 5, 2.0).unwrap();

        assert_eq!(result.middle.len(), 10);
        assert_eq!(result.upper.len(), 10);
        assert_eq!(result.lower.len(), 10);

        // First 4 values should be NaN
        for i in 0..4 {
            assert!(result.middle[i].is_nan());
            assert!(result.upper[i].is_nan());
            assert!(result.lower[i].is_nan());
        }

        // From index 4, values should be valid
        for i in 4..10 {
            assert!(!result.middle[i].is_nan());
            assert!(!result.upper[i].is_nan());
            assert!(!result.lower[i].is_nan());
        }

        // Upper > Middle > Lower
        for i in 4..10 {
            assert!(result.upper[i] > result.middle[i]);
            assert!(result.middle[i] > result.lower[i]);
        }
    }

    #[test]
    fn test_bollinger_f32() {
        let data = vec![20.0_f32, 21.0, 22.0, 21.5, 22.5];
        let result = bollinger(&data, 3, 2.0_f32).unwrap();

        assert_eq!(result.middle.len(), 5);
        assert!(result.middle[0].is_nan());
        assert!(result.middle[1].is_nan());
        assert!(!result.middle[2].is_nan());
    }

    #[test]
    fn test_bollinger_period_one() {
        // With period 1, stddev should be 0, so upper = middle = lower
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = bollinger(&data, 1, 2.0).unwrap();

        // All values are valid (no lookback needed)
        for i in 0..5 {
            assert!(approx_eq(result.middle[i], data[i], EPSILON));
            assert!(approx_eq(result.upper[i], data[i], EPSILON));
            assert!(approx_eq(result.lower[i], data[i], EPSILON));
        }
    }

    #[test]
    fn test_bollinger_period_equals_length() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = bollinger(&data, 5, 2.0).unwrap();

        // Only the last value is valid
        for i in 0..4 {
            assert!(result.middle[i].is_nan());
        }
        assert!(!result.middle[4].is_nan());

        // Middle should be the mean: (1+2+3+4+5)/5 = 3.0
        assert!(approx_eq(result.middle[4], 3.0, EPSILON));
    }

    #[test]
    fn test_bollinger_single_element_period_one() {
        let data = vec![42.0_f64];
        let result = bollinger(&data, 1, 2.0).unwrap();

        assert_eq!(result.middle.len(), 1);
        assert!(approx_eq(result.middle[0], 42.0, EPSILON));
        assert!(approx_eq(result.upper[0], 42.0, EPSILON));
        assert!(approx_eq(result.lower[0], 42.0, EPSILON));
    }

    // ==================== Reference Value Tests ====================

    #[test]
    fn test_bollinger_known_values() {
        // Test with known calculated values
        // Data: [1, 2, 3, 4, 5], period 3, num_std_dev 2
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = bollinger(&data, 3, 2.0).unwrap();

        // For window [1,2,3]:
        // Mean = 2.0
        // Variance = ((1-2)^2 + (2-2)^2 + (3-2)^2) / 3 = (1+0+1)/3 = 2/3
        // StdDev = sqrt(2/3) ≈ 0.8165
        let expected_mean_0 = 2.0;
        let expected_stddev_0 = (2.0_f64 / 3.0).sqrt();

        assert!(approx_eq(result.middle[2], expected_mean_0, EPSILON));
        assert!(approx_eq(
            result.upper[2],
            expected_mean_0 + 2.0 * expected_stddev_0,
            EPSILON
        ));
        assert!(approx_eq(
            result.lower[2],
            expected_mean_0 - 2.0 * expected_stddev_0,
            EPSILON
        ));

        // For window [2,3,4]:
        // Mean = 3.0
        // Variance = ((2-3)^2 + (3-3)^2 + (4-3)^2) / 3 = 2/3
        // StdDev = sqrt(2/3)
        assert!(approx_eq(result.middle[3], 3.0, EPSILON));

        // For window [3,4,5]:
        // Mean = 4.0
        // Same stddev due to same spread
        assert!(approx_eq(result.middle[4], 4.0, EPSILON));
    }

    #[test]
    fn test_bollinger_constant_values() {
        // Bollinger Bands of constant values: stddev = 0, so all bands equal the constant
        let data = vec![5.0_f64; 10];
        let result = bollinger(&data, 3, 2.0).unwrap();

        for i in 2..result.middle.len() {
            assert!(approx_eq(result.middle[i], 5.0, EPSILON));
            assert!(approx_eq(result.upper[i], 5.0, EPSILON));
            assert!(approx_eq(result.lower[i], 5.0, EPSILON));
        }
    }

    #[test]
    fn test_bollinger_symmetric_data() {
        // Test with symmetric data around a mean
        // Values: [8, 10, 12, 10, 8, 10, 12, 10, 8, 10]
        // This should produce symmetric bands around 10
        let data = vec![8.0_f64, 10.0, 12.0, 10.0, 8.0, 10.0, 12.0, 10.0, 8.0, 10.0];
        let result = bollinger(&data, 5, 2.0).unwrap();

        // Check that bands are symmetric around middle
        for i in 4..10 {
            let upper_dist = result.upper[i] - result.middle[i];
            let lower_dist = result.middle[i] - result.lower[i];
            assert!(
                approx_eq(upper_dist, lower_dist, EPSILON),
                "Bands should be symmetric at index {}",
                i
            );
        }
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_bollinger_with_nan_in_data() {
        // NaN in the data should propagate
        let data = vec![1.0_f64, 2.0, f64::NAN, 4.0, 5.0, 6.0];
        let result = bollinger(&data, 3, 2.0).unwrap();

        // Windows containing NaN should produce NaN output
        assert!(result.middle[2].is_nan()); // window [1, 2, NaN]
        assert!(result.middle[3].is_nan()); // window [2, NaN, 4]
        assert!(result.middle[4].is_nan()); // window [NaN, 4, 5]
        assert!(!result.middle[5].is_nan()); // window [4, 5, 6] - NaN rolled out
    }

    #[test]
    fn test_bollinger_negative_values() {
        let data = vec![-5.0_f64, -3.0, -1.0, 1.0, 3.0, 5.0];
        let result = bollinger(&data, 3, 2.0).unwrap();

        // Mean of [-5,-3,-1] = -3.0
        assert!(approx_eq(result.middle[2], -3.0, EPSILON));

        // Mean of [1,3,5] = 3.0
        assert!(approx_eq(result.middle[5], 3.0, EPSILON));

        // Bands should still maintain upper > middle > lower
        for i in 2..6 {
            assert!(result.upper[i] > result.middle[i]);
            assert!(result.middle[i] > result.lower[i]);
        }
    }

    #[test]
    fn test_bollinger_large_values() {
        let data = vec![1e15_f64, 2e15, 3e15, 4e15, 5e15];
        let result = bollinger(&data, 3, 2.0).unwrap();

        // Mean of first window = 2e15
        assert!(approx_eq(result.middle[2], 2e15, 1e5));
    }

    #[test]
    fn test_bollinger_small_values() {
        let data = vec![1e-15_f64, 2e-15, 3e-15, 4e-15, 5e-15];
        let result = bollinger(&data, 3, 2.0).unwrap();

        // Mean of first window = 2e-15
        assert!(approx_eq(result.middle[2], 2e-15, 1e-25));
    }

    #[test]
    fn test_bollinger_different_num_std_dev() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result_2 = bollinger(&data, 3, 2.0).unwrap();
        let result_1 = bollinger(&data, 3, 1.0).unwrap();

        // With 2 std dev, bands should be twice as wide as with 1 std dev
        for i in 2..5 {
            let width_2 = result_2.upper[i] - result_2.lower[i];
            let width_1 = result_1.upper[i] - result_1.lower[i];
            assert!(approx_eq(width_2, 2.0 * width_1, EPSILON));
        }
    }

    #[test]
    fn test_bollinger_zero_std_dev() {
        // With 0 standard deviations, all bands equal the middle
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = bollinger(&data, 3, 0.0).unwrap();

        for i in 2..5 {
            assert!(approx_eq(result.upper[i], result.middle[i], EPSILON));
            assert!(approx_eq(result.lower[i], result.middle[i], EPSILON));
        }
    }

    #[test]
    fn test_bollinger_infinity_handling() {
        let data = vec![1.0_f64, f64::INFINITY, 3.0, 4.0, 5.0];
        let result = bollinger(&data, 3, 2.0).unwrap();

        // Window containing infinity should produce NaN values
        assert!(result.middle[2].is_nan());
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_bollinger_empty_input() {
        let data: Vec<f64> = vec![];
        let result = bollinger(&data, 3, 2.0);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_bollinger_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = bollinger(&data, 0, 2.0);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_bollinger_period_exceeds_length() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = bollinger(&data, 5, 2.0);

        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                required: 5,
                actual: 3,
                ..
            })
        ));
    }

    // ==================== bollinger_into Tests ====================

    #[test]
    fn test_bollinger_into_basic() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = BollingerOutput {
            middle: vec![0.0_f64; 5],
            upper: vec![0.0_f64; 5],
            lower: vec![0.0_f64; 5],
        };
        let valid_count = bollinger_into(&data, 3, 2.0, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(output.middle[0].is_nan());
        assert!(output.middle[1].is_nan());
        assert!(approx_eq(output.middle[2], 2.0, EPSILON));
        assert!(approx_eq(output.middle[3], 3.0, EPSILON));
        assert!(approx_eq(output.middle[4], 4.0, EPSILON));
    }

    #[test]
    fn test_bollinger_into_buffer_reuse() {
        let data1 = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let data2 = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
        let mut output = BollingerOutput {
            middle: vec![0.0_f64; 5],
            upper: vec![0.0_f64; 5],
            lower: vec![0.0_f64; 5],
        };

        bollinger_into(&data1, 3, 2.0, &mut output).unwrap();
        assert!(approx_eq(output.middle[2], 2.0, EPSILON));

        bollinger_into(&data2, 3, 2.0, &mut output).unwrap();
        assert!(approx_eq(output.middle[2], 4.0, EPSILON)); // (5+4+3)/3
    }

    #[test]
    fn test_bollinger_into_insufficient_output() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = BollingerOutput {
            middle: vec![0.0_f64; 3], // Too short
            upper: vec![0.0_f64; 5],
            lower: vec![0.0_f64; 5],
        };
        let result = bollinger_into(&data, 3, 2.0, &mut output);

        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_bollinger_into_f32() {
        let data = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
        let mut output = BollingerOutput {
            middle: vec![0.0_f32; 5],
            upper: vec![0.0_f32; 5],
            lower: vec![0.0_f32; 5],
        };
        let valid_count = bollinger_into(&data, 3, 2.0_f32, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(approx_eq(output.middle[2], 2.0_f32, EPSILON_F32));
    }

    // ==================== Rolling StdDev Tests ====================

    #[test]
    fn test_rolling_stddev_basic() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = rolling_stddev(&data, 3).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());

        // StdDev of [1,2,3] = sqrt(2/3) ≈ 0.8165
        let expected_stddev = (2.0_f64 / 3.0).sqrt();
        assert!(approx_eq(result[2], expected_stddev, EPSILON));
    }

    #[test]
    fn test_rolling_stddev_constant_values() {
        let data = vec![5.0_f64; 10];
        let result = rolling_stddev(&data, 3).unwrap();

        // Standard deviation of constants is 0
        for i in 2..result.len() {
            assert!(approx_eq(result[i], 0.0, EPSILON));
        }
    }

    #[test]
    fn test_rolling_stddev_into_basic() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f64; 5];
        let valid_count = rolling_stddev_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(output[0].is_nan());
        let expected_stddev = (2.0_f64 / 3.0).sqrt();
        assert!(approx_eq(output[2], expected_stddev, EPSILON));
    }

    #[test]
    fn test_rolling_stddev_empty_input() {
        let data: Vec<f64> = vec![];
        let result = rolling_stddev(&data, 3);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_rolling_stddev_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = rolling_stddev(&data, 0);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_bollinger_and_bollinger_into_produce_same_result() {
        let data = vec![10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        let result1 = bollinger(&data, 4, 2.0).unwrap();

        let mut result2 = BollingerOutput {
            middle: vec![0.0_f64; data.len()],
            upper: vec![0.0_f64; data.len()],
            lower: vec![0.0_f64; data.len()],
        };
        bollinger_into(&data, 4, 2.0, &mut result2).unwrap();

        for i in 0..data.len() {
            assert!(approx_eq(result1.middle[i], result2.middle[i], EPSILON));
            assert!(approx_eq(result1.upper[i], result2.upper[i], EPSILON));
            assert!(approx_eq(result1.lower[i], result2.lower[i], EPSILON));
        }
    }

    #[test]
    fn test_bollinger_middle_equals_sma() {
        // The middle band should be exactly equal to the SMA
        use crate::indicators::sma::sma;

        let data = vec![
            20.0_f64, 21.0, 22.0, 21.5, 22.5, 23.0, 22.0, 21.0, 20.5, 21.5,
        ];
        let bb = bollinger(&data, 5, 2.0).unwrap();
        let sma_result = sma(&data, 5).unwrap();

        for i in 0..data.len() {
            assert!(
                approx_eq(bb.middle[i], sma_result[i], EPSILON),
                "Middle band should equal SMA at index {}",
                i
            );
        }
    }

    #[test]
    fn test_bollinger_valid_count() {
        let data = vec![1.0_f64; 100];
        let mut output = BollingerOutput {
            middle: vec![0.0_f64; 100],
            upper: vec![0.0_f64; 100],
            lower: vec![0.0_f64; 100],
        };

        let valid_count = bollinger_into(&data, 10, 2.0, &mut output).unwrap();
        assert_eq!(valid_count, 91); // 100 - 10 + 1

        let valid_count = bollinger_into(&data, 1, 2.0, &mut output).unwrap();
        assert_eq!(valid_count, 100); // All values valid

        let valid_count = bollinger_into(&data, 100, 2.0, &mut output).unwrap();
        assert_eq!(valid_count, 1); // Only last value valid
    }

    // ==================== Property-Based-Like Tests ====================

    #[test]
    fn test_bollinger_output_length_equals_input_length() {
        for len in [5, 10, 50, 100] {
            for period in [1, 2, 5] {
                if period <= len {
                    let data: Vec<f64> = (0..len).map(|x| x as f64).collect();
                    let result = bollinger(&data, period, 2.0).unwrap();
                    assert_eq!(result.middle.len(), len);
                    assert_eq!(result.upper.len(), len);
                    assert_eq!(result.lower.len(), len);
                }
            }
        }
    }

    #[test]
    fn test_bollinger_nan_count() {
        // First (period - 1) values should be NaN
        for period in 1..=10 {
            let data: Vec<f64> = (0..20).map(|x| x as f64).collect();
            let result = bollinger(&data, period, 2.0).unwrap();

            let nan_count = result.middle.iter().filter(|x| x.is_nan()).count();
            assert_eq!(nan_count, period - 1);
        }
    }

    #[test]
    fn test_bollinger_band_ordering() {
        // Upper > Middle > Lower for non-constant data
        let data: Vec<f64> = (0..100).map(|x| (x as f64) + (x % 5) as f64).collect();
        let result = bollinger(&data, 10, 2.0).unwrap();

        for i in 9..100 {
            if !result.middle[i].is_nan() {
                assert!(
                    result.upper[i] >= result.middle[i],
                    "Upper should be >= middle at index {}",
                    i
                );
                assert!(
                    result.middle[i] >= result.lower[i],
                    "Middle should be >= lower at index {}",
                    i
                );
            }
        }
    }

    #[test]
    fn test_bollinger_band_symmetry() {
        // Upper - Middle should equal Middle - Lower
        let data: Vec<f64> = (0..50).map(|x| (x as f64).sin() * 10.0 + 50.0).collect();
        let result = bollinger(&data, 5, 2.0).unwrap();

        for i in 4..50 {
            if !result.middle[i].is_nan() {
                let upper_dist = result.upper[i] - result.middle[i];
                let lower_dist = result.middle[i] - result.lower[i];
                assert!(
                    approx_eq(upper_dist, lower_dist, EPSILON),
                    "Bands should be symmetric at index {}",
                    i
                );
            }
        }
    }

    #[test]
    fn test_bollinger_middle_within_bands() {
        // Middle should always be between upper and lower
        let data: Vec<f64> = (0..100)
            .map(|x| (x as f64 * 0.1).cos() * 20.0 + 100.0)
            .collect();
        let result = bollinger(&data, 20, 2.0).unwrap();

        for i in 19..100 {
            assert!(result.upper[i] >= result.middle[i]);
            assert!(result.middle[i] >= result.lower[i]);
        }
    }

    #[test]
    fn test_bollinger_higher_volatility_wider_bands() {
        // More volatile data should have wider bands
        let low_vol: Vec<f64> = (0..50).map(|x| 100.0 + (x as f64 * 0.01).sin()).collect();
        let high_vol: Vec<f64> = (0..50)
            .map(|x| 100.0 + (x as f64 * 0.01).sin() * 10.0)
            .collect();

        let result_low = bollinger(&low_vol, 10, 2.0).unwrap();
        let result_high = bollinger(&high_vol, 10, 2.0).unwrap();

        // Compare band widths (at the end where both are stable)
        let width_low = result_low.upper[49] - result_low.lower[49];
        let width_high = result_high.upper[49] - result_high.lower[49];

        assert!(
            width_high > width_low,
            "Higher volatility data should have wider bands"
        );
    }
}
