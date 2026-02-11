//! Utility functions for liq-ta.
//!
//! This module provides shared utility functions used throughout the library
//! and exposed for user convenience.
//!
//! # Floating-Point Comparison (PRD §4.8)
//!
//! Due to the nature of floating-point arithmetic, exact equality comparison
//! is often inappropriate. This module provides tolerance-based comparison
//! functions for use in testing and validation.
//!
//! # Example
//!
//! ```
//! use liq_ta::utils::{approx_eq, EPSILON};
//!
//! let a = 1.0 / 3.0;
//! let b = 0.333333333333333;
//! assert!(approx_eq(a, b, EPSILON));
//! ```

use crate::traits::SeriesElement;

/// Standard epsilon for high-precision floating-point comparisons.
///
/// This tolerance (1e-10) is appropriate for most indicator calculations
/// where accumulated floating-point error is minimal.
pub const EPSILON: f64 = 1e-10;

/// Looser epsilon for comparisons involving accumulated floating-point operations.
///
/// Use this tolerance (1e-6) when comparing results that involve many
/// accumulated operations or when absolute precision is less critical.
pub const LOOSE_EPSILON: f64 = 1e-6;

#[inline]
#[must_use]
pub(crate) fn is_invalid<T: SeriesElement>(value: T) -> bool {
    value.is_nan() || value.is_infinite()
}

#[inline]
#[must_use]
#[allow(dead_code)]
pub(crate) fn build_invalid_mask<T: SeriesElement>(data: &[T]) -> Vec<u8> {
    data.iter()
        .map(|&value| if is_invalid(value) { 1 } else { 0 })
        .collect()
}

#[inline]
#[must_use]
#[allow(dead_code)]
pub(crate) fn init_invalid_count(mask: &[u8], start: usize, period: usize) -> usize {
    mask.iter()
        .skip(start)
        .take(period)
        .map(|&value| usize::from(value))
        .sum()
}

#[inline]
#[allow(dead_code)]
pub(crate) fn update_invalid_count(count: &mut usize, old_invalid: bool, new_invalid: bool) {
    if new_invalid {
        *count += 1;
    }
    if old_invalid {
        *count -= 1;
    }
}

/// Approximate equality check for floating-point values.
///
/// Returns `true` if `a` and `b` are within `tolerance` of each other,
/// or if both are NaN (for testing convenience).
///
/// # Arguments
///
/// * `a` - First value to compare
/// * `b` - Second value to compare
/// * `tolerance` - Maximum allowed absolute difference
///
/// # Returns
///
/// `true` if the values are approximately equal or both NaN.
///
/// # Example
///
/// ```
/// use liq_ta::utils::{approx_eq, EPSILON};
///
/// // Normal comparison
/// assert!(approx_eq(1.0, 1.0 + 1e-11, EPSILON));
/// assert!(!approx_eq(1.0, 2.0, EPSILON));
///
/// // NaN handling (both NaN considered equal for testing)
/// assert!(approx_eq(f64::NAN, f64::NAN, EPSILON));
/// assert!(!approx_eq(f64::NAN, 1.0, EPSILON));
/// ```
#[inline]
#[must_use]
pub fn approx_eq<T: SeriesElement>(a: T, b: T, tolerance: T) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    (a - b).abs() < tolerance
}

/// Relative approximate equality check for floating-point values.
///
/// Returns `true` if the relative difference between `a` and `b` is less than
/// `rel_tolerance`, or if both are NaN.
///
/// This is more appropriate than absolute tolerance when comparing values
/// of varying magnitudes.
///
/// # Arguments
///
/// * `a` - First value to compare
/// * `b` - Second value to compare
/// * `rel_tolerance` - Maximum allowed relative difference (e.g., 1e-10 for 0.00000001%)
///
/// # Returns
///
/// `true` if the values are relatively equal or both NaN.
///
/// # Example
///
/// ```
/// use liq_ta::utils::approx_eq_relative;
///
/// // Large values with small relative difference
/// assert!(approx_eq_relative(1e10, 1e10 + 1.0, 1e-9));
///
/// // Small values with same relative difference
/// assert!(approx_eq_relative(1e-10, 1.000000001e-10, 1e-8));
/// ```
#[inline]
#[must_use]
pub fn approx_eq_relative<T: SeriesElement>(a: T, b: T, rel_tolerance: T) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }

    let diff = (a - b).abs();
    let max_abs = a.abs().max(b.abs());

    if max_abs == T::zero() {
        return diff == T::zero();
    }

    diff / max_abs < rel_tolerance
}

/// Count the number of NaN values in a slice.
///
/// # Example
///
/// ```
/// use liq_ta::utils::count_nans;
///
/// let data = vec![f64::NAN, 1.0, f64::NAN, 2.0];
/// assert_eq!(count_nans(&data), 2);
/// ```
#[inline]
#[must_use]
pub fn count_nans<T: SeriesElement>(data: &[T]) -> usize {
    data.iter().filter(|x| x.is_nan()).count()
}

/// Count the number of NaN values at the beginning of a slice.
///
/// This is useful for verifying the lookback period of indicator outputs.
///
/// # Example
///
/// ```
/// use liq_ta::utils::count_nan_prefix;
///
/// let data = vec![f64::NAN, f64::NAN, 1.0, 2.0, f64::NAN];
/// assert_eq!(count_nan_prefix(&data), 2);
/// ```
#[inline]
#[must_use]
pub fn count_nan_prefix<T: SeriesElement>(data: &[T]) -> usize {
    data.iter().take_while(|x| x.is_nan()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approx_eq_basic() {
        assert!(approx_eq(1.0_f64, 1.0, EPSILON));
        assert!(approx_eq(1.0_f64, 1.0 + 1e-11, EPSILON));
        assert!(!approx_eq(1.0_f64, 2.0, EPSILON));
    }

    #[test]
    fn test_approx_eq_nan() {
        assert!(approx_eq(f64::NAN, f64::NAN, EPSILON));
        assert!(!approx_eq(f64::NAN, 1.0, EPSILON));
        assert!(!approx_eq(1.0, f64::NAN, EPSILON));
    }

    #[test]
    fn test_approx_eq_f32() {
        assert!(approx_eq(1.0_f32, 1.0, 1e-5));
        assert!(!approx_eq(1.0_f32, 2.0, 1e-5));
    }

    #[test]
    fn test_approx_eq_relative_basic() {
        assert!(approx_eq_relative(1.0_f64, 1.0, 1e-10));
        assert!(approx_eq_relative(1e10_f64, 1e10 + 1.0, 1e-9));
        assert!(!approx_eq_relative(1.0_f64, 2.0, 1e-10));
    }

    #[test]
    fn test_approx_eq_relative_zero() {
        assert!(approx_eq_relative(0.0_f64, 0.0, 1e-10));
        assert!(!approx_eq_relative(0.0_f64, 1e-11, 1e-10));
    }

    #[test]
    fn test_count_nans() {
        let data = vec![f64::NAN, 1.0, f64::NAN, 2.0, f64::NAN];
        assert_eq!(count_nans(&data), 3);

        let no_nans = vec![1.0_f64, 2.0, 3.0];
        assert_eq!(count_nans(&no_nans), 0);
    }

    #[test]
    fn test_count_nan_prefix() {
        let data = vec![f64::NAN, f64::NAN, 1.0, 2.0, f64::NAN];
        assert_eq!(count_nan_prefix(&data), 2);

        let no_prefix = vec![1.0_f64, f64::NAN, 2.0];
        assert_eq!(count_nan_prefix(&no_prefix), 0);

        let all_nan = vec![f64::NAN, f64::NAN, f64::NAN];
        assert_eq!(count_nan_prefix(&all_nan), 3);
    }
}
