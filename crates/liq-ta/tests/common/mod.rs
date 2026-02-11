//! Shared test utilities for liq-ta tests.
//!
//! This module provides common utilities used across multiple test files.

/// Approximate equality check for floating-point values.
///
/// Handles NaN values specially - two NaN values are considered equal for testing purposes.
///
/// # Arguments
///
/// * `a` - First value to compare
/// * `b` - Second value to compare
/// * `eps` - Epsilon tolerance for comparison
///
/// # Returns
///
/// `true` if the values are approximately equal (within `eps`), or if both are NaN.
#[allow(dead_code)]
pub fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    (a - b).abs() < eps
}

/// Approximate equality check for f32 floating-point values.
#[allow(dead_code)]
pub fn approx_eq_f32(a: f32, b: f32, eps: f32) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    (a - b).abs() < eps
}

/// Standard epsilon for high-precision comparisons.
#[allow(dead_code)]
pub const EPSILON: f64 = 1e-10;

/// Looser epsilon for comparisons involving accumulated floating-point operations.
#[allow(dead_code)]
pub const LOOSE_EPSILON: f64 = 1e-6;

/// Count the number of NaN values in a slice.
#[allow(dead_code)]
pub fn count_nans(data: &[f64]) -> usize {
    data.iter().filter(|x| x.is_nan()).count()
}

/// Verify that the first `n` values are NaN and the rest are not.
#[allow(dead_code)]
pub fn verify_nan_prefix(data: &[f64], expected_nan_count: usize) -> bool {
    let nan_count = count_nans(data);
    if nan_count != expected_nan_count {
        return false;
    }

    // Verify NaNs are at the beginning
    for (i, &val) in data.iter().enumerate() {
        if i < expected_nan_count {
            if !val.is_nan() {
                return false;
            }
        } else if val.is_nan() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approx_eq_basic() {
        assert!(approx_eq(1.0, 1.0, EPSILON));
        assert!(approx_eq(1.0, 1.0 + 1e-11, EPSILON));
        assert!(!approx_eq(1.0, 2.0, EPSILON));
    }

    #[test]
    fn test_approx_eq_nan() {
        assert!(approx_eq(f64::NAN, f64::NAN, EPSILON));
        assert!(!approx_eq(f64::NAN, 1.0, EPSILON));
        assert!(!approx_eq(1.0, f64::NAN, EPSILON));
    }

    #[test]
    fn test_count_nans() {
        let data = vec![f64::NAN, f64::NAN, 1.0, 2.0, f64::NAN];
        assert_eq!(count_nans(&data), 3);
    }

    #[test]
    fn test_verify_nan_prefix() {
        let data = vec![f64::NAN, f64::NAN, 1.0, 2.0, 3.0];
        assert!(verify_nan_prefix(&data, 2));
        assert!(!verify_nan_prefix(&data, 3));
        assert!(!verify_nan_prefix(&data, 1));
    }
}
