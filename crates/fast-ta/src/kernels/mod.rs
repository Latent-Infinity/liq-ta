//! Fusion kernels for efficient computation of multiple statistics.
//!
//! This module provides optimized kernel implementations for performance-critical
//! operations that benefit from algorithmic optimization.
//!
//! # Kernels
//!
//! - [`mod@rolling_extrema`]: Monotonic deque algorithm for O(n) rolling max/min
//! - [`mod@simd`]: SIMD-accelerated reductions using portable SIMD
//!
//! # Performance
//!
//! The rolling extrema kernel uses a monotonic deque data structure to compute
//! rolling maximum and minimum values in O(n) time, compared to the O(n*k) naive
//! approach. This provides 10-100x speedups for larger window sizes.
//!
//! The SIMD kernels use portable SIMD (requires nightly Rust) to accelerate
//! reduction operations like sum, variance, dot product, and correlation,
//! providing 2.5-4x speedups on typical data sizes.

pub mod rolling_extrema;

/// SIMD-accelerated kernels (requires nightly Rust)
pub mod simd;

// Re-export kernel types for convenient access.
//
// These re-exports allow users to import directly from `kernels` without
// needing to specify the submodule, e.g., `use fast_ta::kernels::rolling_max;`

// Rolling extrema kernel exports: O(n) rolling max/min using monotonic deque
pub use rolling_extrema::{
    rolling_extrema, rolling_extrema_into, rolling_extrema_lookback, rolling_extrema_min_len,
    rolling_max, rolling_max_into, rolling_max_naive, rolling_min, rolling_min_into,
    rolling_min_naive, MonotonicDeque, RollingExtremaOutput,
    // NaN-propagating variants (for indicators that require strict NaN propagation)
    rolling_max_nan_propagating, rolling_min_nan_propagating,
    rolling_extrema_fused_nan_propagating, rolling_extrema_fused_nan_propagating_into,
};

// SIMD kernel exports
pub use simd::{
    // Constants
    F32_LANES, F64_LANES,
    // Basic reductions
    max_f32, max_f64, min_f32, min_f64, sum_and_count_f64, sum_f32, sum_f64,
    // Variance/stddev (NaN-propagating versions)
    sum_and_sum_sq_f64, sum_of_squares_f64, sum_squared_diff_f64, stddev_f64, variance_f64,
    // Variance/stddev (NaN-aware version - skips NaN values)
    sum_and_sum_sq_and_count_f64,
    // Dot product/weighted sum
    dot_product_f64, weighted_sum_f64, scaled_sum_f64,
    // Correlation
    correlation_f64, covariance_components_f64,
    // Other
    sum_abs_diff_f64,
};
