//! SIMD-accelerated kernels for high-performance computations.
//!
//! This module provides SIMD-optimized implementations of common operations
//! used in technical analysis indicators. These kernels use Rust's portable
//! SIMD feature (`std::simd`) available on nightly.
//!
//! # Available Kernels
//!
//! - [`sum_f64`]: SIMD-accelerated sum reduction for f64 slices
//! - [`sum_f32`]: SIMD-accelerated sum reduction for f32 slices
//! - [`min_f64`]: SIMD-accelerated minimum reduction for f64 slices
//! - [`max_f64`]: SIMD-accelerated maximum reduction for f64 slices
//!
//! # Performance
//!
//! These kernels provide 2-8x speedups over scalar implementations depending
//! on data size and CPU architecture. They work best with large arrays
//! (>64 elements) where SIMD overhead is amortized.
//!
//! # Example
//!
//! ```ignore
//! use liq_ta::kernels::simd;
//!
//! let data: Vec<f64> = (0..1000).map(|x| x as f64).collect();
//! let sum = simd::sum_f64(&data);
//! ```

use std::simd::{Select, f32x8, f64x4, num::SimdFloat};

/// The number of f64 elements processed per SIMD lane.
pub const F64_LANES: usize = 4;

/// The number of f32 elements processed per SIMD lane.
pub const F32_LANES: usize = 8;

/// Computes the sum of a slice of f64 values using SIMD.
///
/// This function processes 4 elements at a time using 256-bit SIMD vectors,
/// then handles any remaining elements with scalar operations.
///
/// # Arguments
///
/// * `data` - Slice of f64 values to sum
///
/// # Returns
///
/// The sum of all elements in the slice. Returns 0.0 for empty slices.
///
/// # Performance
///
/// - Uses 4-wide SIMD (f64x4) for the main loop
/// - Handles tail elements with scalar fallback
/// - Expected 2-4x speedup over scalar for large arrays
///
/// # Example
///
/// ```ignore
/// let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
/// let sum = simd::sum_f64(&data);
/// assert!((sum - 36.0).abs() < 1e-10);
/// ```
#[inline]
pub fn sum_f64(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    // SIMD accumulator
    let mut acc = f64x4::splat(0.0);

    // Process chunks of 4 elements
    for i in 0..chunks {
        let offset = i * F64_LANES;
        // Load 4 elements into SIMD vector
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);
        acc += chunk;
    }

    // Reduce SIMD accumulator to scalar
    let mut sum = acc.reduce_sum();

    // Handle remaining elements
    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        sum += value;
    }

    sum
}

/// Computes the sum of a slice of f32 values using SIMD.
///
/// This function processes 8 elements at a time using 256-bit SIMD vectors,
/// then handles any remaining elements with scalar operations.
///
/// # Arguments
///
/// * `data` - Slice of f32 values to sum
///
/// # Returns
///
/// The sum of all elements in the slice. Returns 0.0 for empty slices.
///
/// # Performance
///
/// - Uses 8-wide SIMD (f32x8) for the main loop
/// - Handles tail elements with scalar fallback
/// - Expected 4-8x speedup over scalar for large arrays
#[inline]
pub fn sum_f32(data: &[f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }

    let chunks = data.len() / F32_LANES;
    let remainder = data.len() % F32_LANES;

    // SIMD accumulator
    let mut acc = f32x8::splat(0.0);

    // Process chunks of 8 elements
    for i in 0..chunks {
        let offset = i * F32_LANES;
        let chunk = f32x8::from_slice(&data[offset..offset + F32_LANES]);
        acc += chunk;
    }

    // Reduce SIMD accumulator to scalar
    let mut sum = acc.reduce_sum();

    // Handle remaining elements
    let tail_start = chunks * F32_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        sum += value;
    }

    sum
}

/// Computes the minimum of a slice of f64 values using SIMD.
///
/// # Arguments
///
/// * `data` - Slice of f64 values
///
/// # Returns
///
/// The minimum value, or `f64::INFINITY` for empty slices.
///
/// # NaN Handling
///
/// NaN values are propagated - if any element is NaN, the result is NaN.
#[inline]
pub fn min_f64(data: &[f64]) -> f64 {
    if data.is_empty() {
        return f64::INFINITY;
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;
    let mut invalid = false;

    // SIMD accumulator initialized to infinity
    let mut acc = f64x4::splat(f64::INFINITY);
    let mut invalid_mask = f64x4::splat(0.0).is_nan();

    // Process chunks of 4 elements
    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);
        let mask = chunk.is_finite();
        invalid_mask |= !mask;
        acc = acc.simd_min(chunk);
    }

    // Reduce SIMD accumulator to scalar
    let mut min_val = acc.reduce_min();

    // Handle remaining elements
    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        if value.is_finite() {
            if value < min_val {
                min_val = value;
            }
        } else {
            invalid = true;
        }
    }

    if invalid || invalid_mask.any() {
        f64::NAN
    } else {
        min_val
    }
}

/// Computes the maximum of a slice of f64 values using SIMD.
///
/// # Arguments
///
/// * `data` - Slice of f64 values
///
/// # Returns
///
/// The maximum value, or `f64::NEG_INFINITY` for empty slices.
///
/// # NaN Handling
///
/// NaN values are propagated - if any element is NaN, the result is NaN.
#[inline]
pub fn max_f64(data: &[f64]) -> f64 {
    if data.is_empty() {
        return f64::NEG_INFINITY;
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;
    let mut invalid = false;

    // SIMD accumulator initialized to negative infinity
    let mut acc = f64x4::splat(f64::NEG_INFINITY);
    let mut invalid_mask = f64x4::splat(0.0).is_nan();

    // Process chunks of 4 elements
    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);
        let mask = chunk.is_finite();
        invalid_mask |= !mask;
        acc = acc.simd_max(chunk);
    }

    // Reduce SIMD accumulator to scalar
    let mut max_val = acc.reduce_max();

    // Handle remaining elements
    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        if value.is_finite() {
            if value > max_val {
                max_val = value;
            }
        } else {
            invalid = true;
        }
    }

    if invalid || invalid_mask.any() {
        f64::NAN
    } else {
        max_val
    }
}

/// Computes the minimum of a slice of f32 values using SIMD.
#[inline]
pub fn min_f32(data: &[f32]) -> f32 {
    if data.is_empty() {
        return f32::INFINITY;
    }

    let chunks = data.len() / F32_LANES;
    let remainder = data.len() % F32_LANES;
    let mut invalid = false;

    let mut acc = f32x8::splat(f32::INFINITY);
    let mut invalid_mask = f32x8::splat(0.0).is_nan();

    for i in 0..chunks {
        let offset = i * F32_LANES;
        let chunk = f32x8::from_slice(&data[offset..offset + F32_LANES]);
        let mask = chunk.is_finite();
        invalid_mask |= !mask;
        acc = acc.simd_min(chunk);
    }

    let mut min_val = acc.reduce_min();

    let tail_start = chunks * F32_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        if value.is_finite() {
            if value < min_val {
                min_val = value;
            }
        } else {
            invalid = true;
        }
    }

    if invalid || invalid_mask.any() {
        f32::NAN
    } else {
        min_val
    }
}

/// Computes the maximum of a slice of f32 values using SIMD.
#[inline]
pub fn max_f32(data: &[f32]) -> f32 {
    if data.is_empty() {
        return f32::NEG_INFINITY;
    }

    let chunks = data.len() / F32_LANES;
    let remainder = data.len() % F32_LANES;
    let mut invalid = false;

    let mut acc = f32x8::splat(f32::NEG_INFINITY);
    let mut invalid_mask = f32x8::splat(0.0).is_nan();

    for i in 0..chunks {
        let offset = i * F32_LANES;
        let chunk = f32x8::from_slice(&data[offset..offset + F32_LANES]);
        let mask = chunk.is_finite();
        invalid_mask |= !mask;
        acc = acc.simd_max(chunk);
    }

    let mut max_val = acc.reduce_max();

    let tail_start = chunks * F32_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        if value.is_finite() {
            if value > max_val {
                max_val = value;
            }
        } else {
            invalid = true;
        }
    }

    if invalid || invalid_mask.any() {
        f32::NAN
    } else {
        max_val
    }
}

/// Computes sum and count of non-NaN elements simultaneously using SIMD.
///
/// This is useful for SMA where we need to track NaN counts efficiently.
///
/// # Returns
///
/// A tuple of (sum, non_nan_count)
#[inline]
pub fn sum_and_count_f64(data: &[f64]) -> (f64, usize) {
    if data.is_empty() {
        return (0.0, 0);
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    let mut sum_acc = f64x4::splat(0.0);
    let mut count = 0usize;

    // Process chunks of 4 elements
    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);

        let mask = chunk.is_finite();
        let mask_arr = mask.to_array();
        for &lane_valid in &mask_arr {
            if lane_valid {
                count += 1;
            }
        }

        // Replace non-finite values with 0 for sum
        let zero = f64x4::splat(0.0);
        let clean_chunk = mask.select(chunk, zero);
        sum_acc += clean_chunk;
    }

    let mut sum = sum_acc.reduce_sum();

    // Handle remaining elements
    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        if value.is_finite() {
            sum += value;
            count += 1;
        }
    }

    (sum, count)
}

// =============================================================================
// Sum of Squares (for Variance/Standard Deviation)
// =============================================================================

/// Computes the sum of squares of a slice of f64 values using SIMD.
///
/// This is useful for variance and standard deviation calculations.
///
/// # Formula
/// ```text
/// sum_sq = Σ(x_i²)
/// ```
///
/// # Performance
/// - Uses 4-wide SIMD for parallel squaring and summing
/// - Expected 3-4x speedup over scalar for large arrays
#[inline]
pub fn sum_of_squares_f64(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    let mut acc = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);
        acc += chunk * chunk;
    }

    let mut sum_sq = acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        sum_sq += value * value;
    }

    sum_sq
}

/// Computes sum and sum of squares simultaneously using SIMD.
///
/// This is optimized for Bollinger Bands which needs both values.
///
/// # Returns
/// A tuple of (sum, sum_of_squares)
///
/// # Performance
/// - Single pass through the data
/// - Uses SIMD for both accumulations in parallel
#[inline]
pub fn sum_and_sum_sq_f64(data: &[f64]) -> (f64, f64) {
    if data.is_empty() {
        return (0.0, 0.0);
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    let mut sum_acc = f64x4::splat(0.0);
    let mut sum_sq_acc = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);
        sum_acc += chunk;
        sum_sq_acc += chunk * chunk;
    }

    let mut sum = sum_acc.reduce_sum();
    let mut sum_sq = sum_sq_acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        sum += value;
        sum_sq += value * value;
    }

    (sum, sum_sq)
}

/// Computes sum, sum of squares, and count of non-NaN elements using SIMD.
///
/// This is optimized for Bollinger Bands when the data may contain NaN values.
/// NaN values are excluded from both sums and the count.
///
/// # Returns
/// A tuple of (sum, sum_of_squares, non_nan_count)
///
/// # Performance
/// - Single pass through the data
/// - Uses SIMD for both accumulations in parallel
/// - Masks out NaN values before accumulation
#[inline]
pub fn sum_and_sum_sq_and_count_f64(data: &[f64]) -> (f64, f64, usize) {
    if data.is_empty() {
        return (0.0, 0.0, 0);
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    let mut sum_acc = f64x4::splat(0.0);
    let mut sum_sq_acc = f64x4::splat(0.0);
    let mut count = 0usize;
    let zero = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);

        // Count finite values in this chunk
        let arr = chunk.to_array();
        for &val in &arr {
            if val.is_finite() {
                count += 1;
            }
        }

        // Replace non-finite values with 0 for sums
        let mask = chunk.is_finite();
        let clean_chunk = mask.select(chunk, zero);
        sum_acc += clean_chunk;
        sum_sq_acc += clean_chunk * clean_chunk;
    }

    let mut sum = sum_acc.reduce_sum();
    let mut sum_sq = sum_sq_acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        if value.is_finite() {
            sum += value;
            sum_sq += value * value;
            count += 1;
        }
    }

    (sum, sum_sq, count)
}

/// Computes variance of a slice using the computational formula.
///
/// Uses the formula: `Var(X) = E[X²] - E[X]²`
/// Which is: (sum_sq / n) - (sum / n)²
///
/// # Arguments
/// * `data` - Slice of f64 values
///
/// # Returns
/// The population variance, or 0.0 for empty slices.
#[inline]
pub fn variance_f64(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let n = data.len() as f64;
    let (sum, sum_sq) = sum_and_sum_sq_f64(data);
    let mean = sum / n;
    let mean_sq = sum_sq / n;

    // Var = E[X²] - E[X]²
    // Handle potential floating point precision issues
    let var = mean_sq - mean * mean;
    if var < 0.0 { 0.0 } else { var }
}

/// Computes standard deviation of a slice.
#[inline]
pub fn stddev_f64(data: &[f64]) -> f64 {
    variance_f64(data).sqrt()
}

// =============================================================================
// Dot Product (for WMA, Correlations)
// =============================================================================

/// Computes the dot product of two slices using SIMD.
///
/// # Formula
/// ```text
/// dot = Σ(a_i * b_i)
/// ```
///
/// # Arguments
/// * `a` - First slice
/// * `b` - Second slice (must have same length as `a`)
///
/// # Returns
/// The dot product, or 0.0 if slices are empty.
///
/// # Panics
/// Panics if slices have different lengths.
///
/// # Performance
/// - Uses 4-wide SIMD for parallel multiply-accumulate
/// - Expected 3-4x speedup over scalar for large arrays
#[inline]
pub fn dot_product_f64(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(
        a.len(),
        b.len(),
        "Slices must have same length for dot product"
    );

    if a.is_empty() {
        return 0.0;
    }

    let chunks = a.len() / F64_LANES;
    let remainder = a.len() % F64_LANES;

    let mut acc = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk_a = f64x4::from_slice(&a[offset..offset + F64_LANES]);
        let chunk_b = f64x4::from_slice(&b[offset..offset + F64_LANES]);
        acc += chunk_a * chunk_b;
    }

    let mut dot = acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for i in 0..remainder {
        dot += a[tail_start + i] * b[tail_start + i];
    }

    dot
}

/// Computes weighted sum using SIMD.
///
/// This is useful for WMA (Weighted Moving Average).
///
/// # Formula
/// ```text
/// weighted_sum = Σ(data_i * weights_i)
/// ```
///
/// This is equivalent to dot_product but with a clearer name for the use case.
#[inline]
pub fn weighted_sum_f64(data: &[f64], weights: &[f64]) -> f64 {
    dot_product_f64(data, weights)
}

// =============================================================================
// Element-wise Operations
// =============================================================================

/// Multiplies all elements by a scalar and sums the result using SIMD.
///
/// # Formula
/// ```text
/// result = Σ(data_i * scalar)
/// ```
///
/// This is useful for applying a smoothing factor to a sum.
#[inline]
pub fn scaled_sum_f64(data: &[f64], scalar: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    let scalar_vec = f64x4::splat(scalar);
    let mut acc = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);
        acc += chunk * scalar_vec;
    }

    let mut sum = acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        sum += value * scalar;
    }

    sum
}

/// Computes the difference from mean and sums squares (for variance with known mean).
///
/// # Formula
/// ```text
/// result = Σ((data_i - mean)²)
/// ```
///
/// This is useful when you already know the mean and want to compute variance.
#[inline]
pub fn sum_squared_diff_f64(data: &[f64], mean: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    let mean_vec = f64x4::splat(mean);
    let mut acc = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);
        let diff = chunk - mean_vec;
        acc += diff * diff;
    }

    let mut sum_sq = acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        let diff = value - mean;
        sum_sq += diff * diff;
    }

    sum_sq
}

// =============================================================================
// Higher Moments (for Skewness and Kurtosis)
// =============================================================================

/// Computes all four raw moments (Σx, Σx², Σx³, Σx⁴) in a single SIMD pass.
///
/// This is optimized for computing skewness and kurtosis together.
///
/// # Returns
/// A tuple of (sum, sum_sq, sum_cb, sum_qd) where:
/// - sum: Σx
/// - sum_sq: Σx²
/// - sum_cb: Σx³
/// - sum_qd: Σx⁴
///
/// # Performance
/// - Single pass through data
/// - Uses 4-wide SIMD for all four accumulations in parallel
#[inline]
pub fn moments_f64(data: &[f64]) -> (f64, f64, f64, f64) {
    if data.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    let mut sum_acc = f64x4::splat(0.0);
    let mut sum_sq_acc = f64x4::splat(0.0);
    let mut sum_cb_acc = f64x4::splat(0.0);
    let mut sum_qd_acc = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);
        let sq = chunk * chunk;
        sum_acc += chunk;
        sum_sq_acc += sq;
        sum_cb_acc += sq * chunk;
        sum_qd_acc += sq * sq;
    }

    let mut sum = sum_acc.reduce_sum();
    let mut sum_sq = sum_sq_acc.reduce_sum();
    let mut sum_cb = sum_cb_acc.reduce_sum();
    let mut sum_qd = sum_qd_acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        let sq = value * value;
        sum += value;
        sum_sq += sq;
        sum_cb += sq * value;
        sum_qd += sq * sq;
    }

    (sum, sum_sq, sum_cb, sum_qd)
}

/// Computes all four raw moments with NaN tracking in a single SIMD pass.
///
/// # Returns
/// A tuple of (sum, sum_sq, sum_cb, sum_qd, valid_count)
#[inline]
pub fn moments_and_count_f64(data: &[f64]) -> (f64, f64, f64, f64, usize) {
    if data.is_empty() {
        return (0.0, 0.0, 0.0, 0.0, 0);
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    let mut sum_acc = f64x4::splat(0.0);
    let mut sum_sq_acc = f64x4::splat(0.0);
    let mut sum_cb_acc = f64x4::splat(0.0);
    let mut sum_qd_acc = f64x4::splat(0.0);
    let mut count = 0usize;
    let zero = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);

        // Count finite values
        let arr = chunk.to_array();
        for &val in &arr {
            if val.is_finite() {
                count += 1;
            }
        }

        // Replace non-finite with 0
        let mask = chunk.is_finite();
        let clean = mask.select(chunk, zero);
        let sq = clean * clean;
        sum_acc += clean;
        sum_sq_acc += sq;
        sum_cb_acc += sq * clean;
        sum_qd_acc += sq * sq;
    }

    let mut sum = sum_acc.reduce_sum();
    let mut sum_sq = sum_sq_acc.reduce_sum();
    let mut sum_cb = sum_cb_acc.reduce_sum();
    let mut sum_qd = sum_qd_acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        if value.is_finite() {
            let sq = value * value;
            sum += value;
            sum_sq += sq;
            sum_cb += sq * value;
            sum_qd += sq * sq;
            count += 1;
        }
    }

    (sum, sum_sq, sum_cb, sum_qd, count)
}

/// Computes sum of cubes using SIMD.
///
/// # Formula
/// ```text
/// sum_cb = Σ(x_i³)
/// ```
#[inline]
pub fn sum_cubes_f64(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    let mut acc = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);
        acc += chunk * chunk * chunk;
    }

    let mut sum_cb = acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        sum_cb += value * value * value;
    }

    sum_cb
}

/// Computes sum of fourth powers using SIMD.
///
/// # Formula
/// ```text
/// sum_qd = Σ(x_i⁴)
/// ```
#[inline]
pub fn sum_fourth_f64(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    let mut acc = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);
        let sq = chunk * chunk;
        acc += sq * sq;
    }

    let mut sum_qd = acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        let sq = value * value;
        sum_qd += sq * sq;
    }

    sum_qd
}

/// Computes sum of absolute differences from mean using SIMD.
///
/// This is used for Mean Absolute Deviation (MAD).
///
/// # Formula
/// ```text
/// result = Σ|x_i - mean|
/// ```
#[inline]
pub fn sum_abs_dev_f64(data: &[f64], mean: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let chunks = data.len() / F64_LANES;
    let remainder = data.len() % F64_LANES;

    let mean_vec = f64x4::splat(mean);
    let mut acc = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk = f64x4::from_slice(&data[offset..offset + F64_LANES]);
        let diff = chunk - mean_vec;
        acc += diff.abs();
    }

    let mut sum_abs = acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for &value in &data[tail_start..tail_start + remainder] {
        sum_abs += (value - mean).abs();
    }

    sum_abs
}

// =============================================================================
// Covariance and Correlation helpers
// =============================================================================

/// Computes covariance components using SIMD.
///
/// Returns (sum_xy, sum_x, sum_y, sum_x2, sum_y2) for correlation calculation.
///
/// # Arguments
/// * `x` - First data series
/// * `y` - Second data series (must have same length as `x`)
///
/// # Returns
/// Tuple of (sum_xy, sum_x, sum_y, sum_xx, sum_yy)
#[inline]
pub fn covariance_components_f64(x: &[f64], y: &[f64]) -> (f64, f64, f64, f64, f64) {
    assert_eq!(x.len(), y.len(), "Slices must have same length");

    if x.is_empty() {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }

    let chunks = x.len() / F64_LANES;
    let remainder = x.len() % F64_LANES;

    let mut sum_xy_acc = f64x4::splat(0.0);
    let mut sum_x_acc = f64x4::splat(0.0);
    let mut sum_y_acc = f64x4::splat(0.0);
    let mut sum_xx_acc = f64x4::splat(0.0);
    let mut sum_yy_acc = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk_x = f64x4::from_slice(&x[offset..offset + F64_LANES]);
        let chunk_y = f64x4::from_slice(&y[offset..offset + F64_LANES]);

        sum_xy_acc += chunk_x * chunk_y;
        sum_x_acc += chunk_x;
        sum_y_acc += chunk_y;
        sum_xx_acc += chunk_x * chunk_x;
        sum_yy_acc += chunk_y * chunk_y;
    }

    let mut sum_xy = sum_xy_acc.reduce_sum();
    let mut sum_x = sum_x_acc.reduce_sum();
    let mut sum_y = sum_y_acc.reduce_sum();
    let mut sum_xx = sum_xx_acc.reduce_sum();
    let mut sum_yy = sum_yy_acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for i in 0..remainder {
        let xi = x[tail_start + i];
        let yi = y[tail_start + i];
        sum_xy += xi * yi;
        sum_x += xi;
        sum_y += yi;
        sum_xx += xi * xi;
        sum_yy += yi * yi;
    }

    (sum_xy, sum_x, sum_y, sum_xx, sum_yy)
}

/// Computes Pearson correlation coefficient using SIMD.
///
/// # Formula
/// ```text
/// r = (n*Σxy - Σx*Σy) / sqrt((n*Σx² - (Σx)²) * (n*Σy² - (Σy)²))
/// ```
#[inline]
pub fn correlation_f64(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let (sum_xy, sum_x, sum_y, sum_xx, sum_yy) = covariance_components_f64(x, y);

    let numerator = n * sum_xy - sum_x * sum_y;
    let sum_x_sq = sum_x * sum_x;
    let sum_y_sq = sum_y * sum_y;
    let var_x = n * sum_xx - sum_x_sq;
    let var_y = n * sum_yy - sum_y_sq;
    let denominator = (var_x * var_y).sqrt();

    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

// =============================================================================
// True Range helpers (for ATR)
// =============================================================================

/// Computes the sum of absolute differences using SIMD.
///
/// # Formula
/// ```text
/// result = Σ|a_i - b_i|
/// ```
#[inline]
pub fn sum_abs_diff_f64(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len(), "Slices must have same length");

    if a.is_empty() {
        return 0.0;
    }

    let chunks = a.len() / F64_LANES;
    let remainder = a.len() % F64_LANES;

    let mut acc = f64x4::splat(0.0);

    for i in 0..chunks {
        let offset = i * F64_LANES;
        let chunk_a = f64x4::from_slice(&a[offset..offset + F64_LANES]);
        let chunk_b = f64x4::from_slice(&b[offset..offset + F64_LANES]);
        let diff = chunk_a - chunk_b;
        acc += diff.abs();
    }

    let mut sum = acc.reduce_sum();

    let tail_start = chunks * F64_LANES;
    for i in 0..remainder {
        sum += (a[tail_start + i] - b[tail_start + i]).abs();
    }

    sum
}

// =============================================================================
// Lagged Subtraction (for MOM, ROC numerator)
// =============================================================================

/// Computes lagged subtraction using SIMD: `output[i] = current[i] - lagged[i]`
///
/// This is optimized for MOM (Momentum) and similar indicators that compute
/// the difference between current and lagged values.
///
/// # Arguments
/// * `current` - Current values slice
/// * `lagged` - Lagged values slice (must have same length as current)
/// * `output` - Output slice (must have same length as current)
///
/// # Performance
/// - Uses 4-wide SIMD for parallel subtraction
/// - Expected 2-4x speedup over scalar for large arrays
#[inline]
pub fn lagged_sub_f64(current: &[f64], lagged: &[f64], output: &mut [f64]) {
    debug_assert_eq!(current.len(), lagged.len());
    debug_assert_eq!(current.len(), output.len());

    let n = current.len();
    let chunks = n / F64_LANES;
    let remainder = n % F64_LANES;

    // Process chunks of 4 elements
    for i in 0..chunks {
        let offset = i * F64_LANES;
        let cur_chunk = f64x4::from_slice(&current[offset..offset + F64_LANES]);
        let lag_chunk = f64x4::from_slice(&lagged[offset..offset + F64_LANES]);
        let result = cur_chunk - lag_chunk;
        result.copy_to_slice(&mut output[offset..offset + F64_LANES]);
    }

    // Handle remaining elements
    let tail_start = chunks * F64_LANES;
    for i in 0..remainder {
        output[tail_start + i] = current[tail_start + i] - lagged[tail_start + i];
    }
}

/// Computes lagged subtraction with infinity→NaN sanitization using SIMD.
///
/// This function performs `output[i] = current[i] - lagged[i]` and converts
/// any resulting infinity values to NaN per project numeric policy.
/// The conversion is fused into the main loop to avoid a second memory pass.
#[inline]
pub fn lagged_sub_sanitize_f64(current: &[f64], lagged: &[f64], output: &mut [f64]) {
    debug_assert_eq!(current.len(), lagged.len());
    debug_assert_eq!(current.len(), output.len());

    let n = current.len();
    let chunks = n / F64_LANES;
    let remainder = n % F64_LANES;

    let nan_vec = f64x4::splat(f64::NAN);

    // Process chunks of 4 elements
    for i in 0..chunks {
        let offset = i * F64_LANES;
        let cur_chunk = f64x4::from_slice(&current[offset..offset + F64_LANES]);
        let lag_chunk = f64x4::from_slice(&lagged[offset..offset + F64_LANES]);
        let result = cur_chunk - lag_chunk;

        // Convert infinity to NaN using SIMD select
        let is_inf = !result.is_finite() & !result.is_nan();
        let sanitized = is_inf.select(nan_vec, result);

        sanitized.copy_to_slice(&mut output[offset..offset + F64_LANES]);
    }

    // Handle remaining elements
    let tail_start = chunks * F64_LANES;
    for i in 0..remainder {
        let val = current[tail_start + i] - lagged[tail_start + i];
        output[tail_start + i] = if val.is_infinite() { f64::NAN } else { val };
    }
}

/// Computes lagged subtraction using SIMD for f32.
#[inline]
pub fn lagged_sub_f32(current: &[f32], lagged: &[f32], output: &mut [f32]) {
    debug_assert_eq!(current.len(), lagged.len());
    debug_assert_eq!(current.len(), output.len());

    let n = current.len();
    let chunks = n / F32_LANES;
    let remainder = n % F32_LANES;

    // Process chunks of 8 elements
    for i in 0..chunks {
        let offset = i * F32_LANES;
        let cur_chunk = f32x8::from_slice(&current[offset..offset + F32_LANES]);
        let lag_chunk = f32x8::from_slice(&lagged[offset..offset + F32_LANES]);
        let result = cur_chunk - lag_chunk;
        result.copy_to_slice(&mut output[offset..offset + F32_LANES]);
    }

    // Handle remaining elements
    let tail_start = chunks * F32_LANES;
    for i in 0..remainder {
        output[tail_start + i] = current[tail_start + i] - lagged[tail_start + i];
    }
}

// =============================================================================
// True Range (for ATR, TRANGE indicators)
// =============================================================================

/// Computes True Range using SIMD: TR = max(hl, |hc|, |lc|)
///
/// True Range = max(high - low, |high - prev_close|, |low - prev_close|)
///
/// # Arguments
/// * `high` - High prices (indices 1..n used)
/// * `low` - Low prices (indices 1..n used)
/// * `prev_close` - Previous close prices (indices 0..n-1 used)
/// * `output` - Output slice (indices 1..n filled, index 0 left unchanged)
///
/// # Performance
/// - Uses 4-wide SIMD for parallel computation
/// - Processes max(hl, hc, lc) in a single pass
#[inline]
pub fn true_range_f64(high: &[f64], low: &[f64], prev_close: &[f64], output: &mut [f64]) {
    let n = high.len();
    debug_assert!(n >= 2);
    debug_assert_eq!(low.len(), n);
    debug_assert!(prev_close.len() >= n - 1);
    debug_assert!(output.len() >= n);

    let compute_len = n - 1;
    let chunks = compute_len / F64_LANES;
    let remainder = compute_len % F64_LANES;

    // SIMD hot loop with efficient validity checking
    // NaN propagates through subtraction/abs, so check intermediate results instead of inputs
    let nan_vec = f64x4::splat(f64::NAN);

    for c in 0..chunks {
        let offset = c * F64_LANES;
        let i = offset + 1;

        let h = f64x4::from_slice(&high[i..i + F64_LANES]);
        let l = f64x4::from_slice(&low[i..i + F64_LANES]);
        let pc = f64x4::from_slice(&prev_close[offset..offset + F64_LANES]);

        // NaN propagates naturally through these operations
        let hl = h - l;
        let hc = (h - pc).abs();
        let lc = (l - pc).abs();

        // simd_max(NaN, x) = x, so we need to detect if any component was NaN
        // If hl, hc, or lc is NaN, output should be NaN
        let any_nan = hl.is_nan() | hc.is_nan() | lc.is_nan();

        let result = hl.simd_max(hc.simd_max(lc));

        // Also convert infinity to NaN per project policy
        let is_inf = result.is_infinite();
        let needs_nan = any_nan | is_inf;
        let sanitized = needs_nan.select(nan_vec, result);

        sanitized.copy_to_slice(&mut output[i..i + F64_LANES]);
    }

    // Scalar tail with matching validity logic
    let tail_start = chunks * F64_LANES;
    for j in 0..remainder {
        let i = tail_start + 1 + j;
        let pc_idx = tail_start + j;

        let hl = high[i] - low[i];
        let hc = (high[i] - prev_close[pc_idx]).abs();
        let lc = (low[i] - prev_close[pc_idx]).abs();

        // Check if any intermediate result is NaN (input had NaN/infinity)
        if hl.is_nan() || hc.is_nan() || lc.is_nan() {
            output[i] = f64::NAN;
            continue;
        }

        let result = hl.max(hc).max(lc);
        // Also convert infinity to NaN per project policy
        output[i] = if result.is_infinite() {
            f64::NAN
        } else {
            result
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;
    const EPSILON_F32: f32 = 1e-5;

    #[test]
    fn test_sum_f64_empty() {
        assert_eq!(sum_f64(&[]), 0.0);
    }

    #[test]
    fn test_sum_f64_single() {
        assert!((sum_f64(&[42.0]) - 42.0).abs() < EPSILON);
    }

    #[test]
    fn test_sum_f64_small() {
        let data = vec![1.0, 2.0, 3.0];
        assert!((sum_f64(&data) - 6.0).abs() < EPSILON);
    }

    #[test]
    fn test_sum_f64_exact_lanes() {
        // Exactly 4 elements (one SIMD chunk)
        let data = vec![1.0, 2.0, 3.0, 4.0];
        assert!((sum_f64(&data) - 10.0).abs() < EPSILON);
    }

    #[test]
    fn test_sum_f64_multiple_lanes() {
        // 8 elements (two SIMD chunks)
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        assert!((sum_f64(&data) - 36.0).abs() < EPSILON);
    }

    #[test]
    fn test_sum_f64_with_remainder() {
        // 10 elements (2 full chunks + 2 remainder)
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert!((sum_f64(&data) - 55.0).abs() < EPSILON);
    }

    #[test]
    fn test_sum_f64_large() {
        let data: Vec<f64> = (1..=1000).map(|x| x as f64).collect();
        let expected = 1000.0 * 1001.0 / 2.0; // Sum formula: n(n+1)/2
        assert!((sum_f64(&data) - expected).abs() < 1e-6);
    }

    #[test]
    fn test_sum_f32_basic() {
        let data = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        assert!((sum_f32(&data) - 36.0).abs() < EPSILON_F32);
    }

    #[test]
    fn test_min_f64_basic() {
        let data = vec![5.0, 2.0, 8.0, 1.0, 9.0, 3.0];
        assert!((min_f64(&data) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_max_f64_basic() {
        let data = vec![5.0, 2.0, 8.0, 1.0, 9.0, 3.0];
        assert!((max_f64(&data) - 9.0).abs() < EPSILON);
    }

    #[test]
    fn test_min_f64_empty() {
        assert_eq!(min_f64(&[]), f64::INFINITY);
    }

    #[test]
    fn test_max_f64_empty() {
        assert_eq!(max_f64(&[]), f64::NEG_INFINITY);
    }

    #[test]
    fn test_sum_and_count_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (sum, count) = sum_and_count_f64(&data);
        assert!((sum - 15.0).abs() < EPSILON);
        assert_eq!(count, 5);
    }

    #[test]
    fn test_sum_and_count_with_nan() {
        let data = vec![1.0, f64::NAN, 3.0, f64::NAN, 5.0];
        let (sum, count) = sum_and_count_f64(&data);
        assert!((sum - 9.0).abs() < EPSILON);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_sum_and_count_empty() {
        let (sum, count) = sum_and_count_f64(&[]);
        assert_eq!(sum, 0.0);
        assert_eq!(count, 0);
    }

    // ==================== Sum of Squares Tests ====================

    #[test]
    fn test_sum_of_squares_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        // 1 + 4 + 9 + 16 = 30
        assert!((sum_of_squares_f64(&data) - 30.0).abs() < EPSILON);
    }

    #[test]
    fn test_sum_of_squares_large() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        // Sum of squares formula: n(n+1)(2n+1)/6
        let expected = 100.0 * 101.0 * 201.0 / 6.0;
        assert!((sum_of_squares_f64(&data) - expected).abs() < 1e-6);
    }

    #[test]
    fn test_sum_and_sum_sq_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (sum, sum_sq) = sum_and_sum_sq_f64(&data);
        assert!((sum - 15.0).abs() < EPSILON);
        assert!((sum_sq - 55.0).abs() < EPSILON); // 1+4+9+16+25
    }

    #[test]
    fn test_variance_constant() {
        let data = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        assert!((variance_f64(&data)).abs() < EPSILON);
    }

    #[test]
    fn test_variance_known() {
        // Data: [1, 2, 3, 4, 5], mean = 3
        // Variance = ((1-3)² + (2-3)² + (3-3)² + (4-3)² + (5-3)²) / 5
        //          = (4 + 1 + 0 + 1 + 4) / 5 = 10/5 = 2
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((variance_f64(&data) - 2.0).abs() < EPSILON);
    }

    #[test]
    fn test_stddev_known() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((stddev_f64(&data) - 2.0_f64.sqrt()).abs() < EPSILON);
    }

    // ==================== Dot Product Tests ====================

    #[test]
    fn test_dot_product_basic() {
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![1.0, 1.0, 1.0, 1.0];
        // 1*1 + 2*1 + 3*1 + 4*1 = 10
        assert!((dot_product_f64(&a, &b) - 10.0).abs() < EPSILON);
    }

    #[test]
    fn test_dot_product_squares() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        // dot(x, x) = sum of squares
        assert!((dot_product_f64(&data, &data) - 30.0).abs() < EPSILON);
    }

    #[test]
    fn test_dot_product_large() {
        let a: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let b: Vec<f64> = vec![1.0; 100];
        // dot with all ones = sum
        let expected = 100.0 * 101.0 / 2.0;
        assert!((dot_product_f64(&a, &b) - expected).abs() < 1e-6);
    }

    #[test]
    fn test_weighted_sum() {
        let data = vec![10.0, 20.0, 30.0];
        let weights = vec![0.5, 0.3, 0.2];
        // 10*0.5 + 20*0.3 + 30*0.2 = 5 + 6 + 6 = 17
        assert!((weighted_sum_f64(&data, &weights) - 17.0).abs() < EPSILON);
    }

    // ==================== Element-wise Tests ====================

    #[test]
    fn test_scaled_sum() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        // sum = 10, scaled by 0.5 = 5
        assert!((scaled_sum_f64(&data, 0.5) - 5.0).abs() < EPSILON);
    }

    #[test]
    fn test_sum_squared_diff() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mean = 3.0;
        // (1-3)² + (2-3)² + (3-3)² + (4-3)² + (5-3)² = 4+1+0+1+4 = 10
        assert!((sum_squared_diff_f64(&data, mean) - 10.0).abs() < EPSILON);
    }

    // ==================== Covariance/Correlation Tests ====================

    #[test]
    fn test_covariance_components() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![2.0, 4.0, 6.0, 8.0];
        let (sum_xy, sum_x, sum_y, sum_xx, sum_yy) = covariance_components_f64(&x, &y);

        assert!((sum_x - 10.0).abs() < EPSILON); // 1+2+3+4
        assert!((sum_y - 20.0).abs() < EPSILON); // 2+4+6+8
        assert!((sum_xy - 60.0).abs() < EPSILON); // 2+8+18+32
        assert!((sum_xx - 30.0).abs() < EPSILON); // 1+4+9+16
        assert!((sum_yy - 120.0).abs() < EPSILON); // 4+16+36+64
    }

    #[test]
    fn test_correlation_perfect() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        // Perfect positive correlation
        assert!((correlation_f64(&x, &y) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_correlation_negative() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 8.0, 6.0, 4.0, 2.0];
        // Perfect negative correlation
        assert!((correlation_f64(&x, &y) - (-1.0)).abs() < EPSILON);
    }

    #[test]
    fn test_correlation_zero() {
        // Constant data has zero correlation
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        assert!((correlation_f64(&x, &y)).abs() < EPSILON);
    }

    // ==================== Sum Abs Diff Tests ====================

    #[test]
    fn test_sum_abs_diff_basic() {
        let a = vec![1.0, 5.0, 3.0, 7.0];
        let b = vec![2.0, 3.0, 5.0, 4.0];
        // |1-2| + |5-3| + |3-5| + |7-4| = 1 + 2 + 2 + 3 = 8
        assert!((sum_abs_diff_f64(&a, &b) - 8.0).abs() < EPSILON);
    }

    #[test]
    fn test_sum_abs_diff_same() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        assert!((sum_abs_diff_f64(&data, &data)).abs() < EPSILON);
    }

    // ==================== Lagged Subtraction Tests ====================

    #[test]
    fn test_lagged_sub_f64_basic() {
        let current = vec![5.0, 6.0, 7.0, 8.0];
        let lagged = vec![1.0, 2.0, 3.0, 4.0];
        let mut output = vec![0.0; 4];
        lagged_sub_f64(&current, &lagged, &mut output);
        assert!((output[0] - 4.0).abs() < EPSILON);
        assert!((output[1] - 4.0).abs() < EPSILON);
        assert!((output[2] - 4.0).abs() < EPSILON);
        assert!((output[3] - 4.0).abs() < EPSILON);
    }

    #[test]
    fn test_lagged_sub_f64_with_remainder() {
        let current = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
        let lagged = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mut output = vec![0.0; 6];
        lagged_sub_f64(&current, &lagged, &mut output);
        for i in 0..6 {
            let expected = (i + 1) as f64 * 9.0;
            assert!((output[i] - expected).abs() < EPSILON);
        }
    }

    #[test]
    fn test_lagged_sub_f64_large() {
        let n = 1000;
        let current: Vec<f64> = (0..n).map(|x| (x + 100) as f64).collect();
        let lagged: Vec<f64> = (0..n).map(|x| x as f64).collect();
        let mut output = vec![0.0; n];
        lagged_sub_f64(&current, &lagged, &mut output);
        for &val in &output {
            assert!((val - 100.0).abs() < EPSILON);
        }
    }

    #[test]
    fn test_lagged_sub_f32_basic() {
        let current = vec![5.0_f32, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
        let lagged = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let mut output = vec![0.0_f32; 8];
        lagged_sub_f32(&current, &lagged, &mut output);
        for &val in &output {
            assert!((val - 4.0).abs() < EPSILON_F32);
        }
    }

    // ==================== Higher Moments Tests ====================

    #[test]
    fn test_moments_f64_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let (sum, sum_sq, sum_cb, sum_qd) = moments_f64(&data);
        assert!((sum - 10.0).abs() < EPSILON); // 1+2+3+4
        assert!((sum_sq - 30.0).abs() < EPSILON); // 1+4+9+16
        assert!((sum_cb - 100.0).abs() < EPSILON); // 1+8+27+64
        assert!((sum_qd - 354.0).abs() < EPSILON); // 1+16+81+256
    }

    #[test]
    fn test_moments_f64_large() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let (sum, sum_sq, sum_cb, _sum_qd) = moments_f64(&data);
        // Sum = n(n+1)/2 = 5050
        assert!((sum - 5050.0).abs() < 1e-6);
        // Sum of squares = n(n+1)(2n+1)/6 = 338350
        assert!((sum_sq - 338350.0).abs() < 1e-6);
        // Sum of cubes = [n(n+1)/2]² = 25502500
        assert!((sum_cb - 25502500.0).abs() < 1e-3);
    }

    #[test]
    fn test_moments_and_count_f64_with_nan() {
        let data = vec![1.0, f64::NAN, 3.0, f64::INFINITY, 5.0];
        let (sum, sum_sq, sum_cb, sum_qd, count) = moments_and_count_f64(&data);
        assert!((sum - 9.0).abs() < EPSILON); // 1+3+5
        assert!((sum_sq - 35.0).abs() < EPSILON); // 1+9+25
        assert!((sum_cb - 153.0).abs() < EPSILON); // 1+27+125
        assert!((sum_qd - 707.0).abs() < EPSILON); // 1+81+625
        assert_eq!(count, 3);
    }

    #[test]
    fn test_sum_cubes_f64() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        // 1 + 8 + 27 + 64 = 100
        assert!((sum_cubes_f64(&data) - 100.0).abs() < EPSILON);
    }

    #[test]
    fn test_sum_fourth_f64() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        // 1 + 16 + 81 + 256 = 354
        assert!((sum_fourth_f64(&data) - 354.0).abs() < EPSILON);
    }

    #[test]
    fn test_sum_abs_dev_f64() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mean = 3.0;
        // |1-3| + |2-3| + |3-3| + |4-3| + |5-3| = 2+1+0+1+2 = 6
        assert!((sum_abs_dev_f64(&data, mean) - 6.0).abs() < EPSILON);
    }

    #[test]
    fn test_sum_abs_dev_f64_large() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let mean = 50.5; // (1+100)/2
        let result = sum_abs_dev_f64(&data, mean);
        // MAD for uniform 1..100 = 2500
        assert!((result - 2500.0).abs() < 1e-6);
    }
}
