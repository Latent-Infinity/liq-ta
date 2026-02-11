//! Core traits for liq-ta numeric operations.
//!
//! This module defines the traits used throughout the liq-ta library
//! for generic numeric operations on data series.
//!
//! # Overview
//!
//! The primary trait is [`SeriesElement`], which provides a common interface
//! for numeric operations on time series data, abstracting over `f32` and `f64`
//! types. The module also provides validation utilities through [`ValidatedInput`]
//! and standalone validation functions.
//!
//! # Example
//!
//! ```
//! use liq_ta::traits::{SeriesElement, ValidatedInput, validate_indicator_input};
//!
//! fn compute_weighted_sum<T: SeriesElement>(data: &[T], period: usize) -> liq_ta::error::Result<T> {
//!     validate_indicator_input(data, period, "weighted_sum")?;
//!
//!     let period_t = T::from_usize(period)?;
//!     let sum: T = data.iter().take(period).fold(T::zero(), |acc, &x| acc + x);
//!     Ok(sum / period_t)
//! }
//!
//! let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
//! let result = compute_weighted_sum(&data, 3).unwrap();
//! assert!((result - 2.0).abs() < 1e-10);
//! ```

use num_traits::{Float, NumCast};

use crate::error::{Error, Result};

/// A trait for types that can be used as elements in a data series.
///
/// This trait provides a common interface for numeric operations on series data,
/// abstracting over `f32` and `f64` types. It extends `num_traits::Float` with
/// additional methods specific to time series operations.
///
/// # Type Bounds
///
/// The trait requires:
/// - `Float`: Standard floating-point operations (NaN handling, infinity, arithmetic)
/// - `NumCast`: Safe conversion between numeric types
/// - `Copy`: Values can be copied (required for efficient iteration)
/// - `Default`: A default value exists (typically zero)
///
/// # Example
///
/// ```
/// use liq_ta::traits::SeriesElement;
/// use num_traits::Float;
///
/// fn compute_sum<T: SeriesElement>(data: &[T]) -> T {
///     data.iter().fold(T::zero(), |acc, &x| {
///         if x.is_nan() { acc } else { acc + x }
///     })
/// }
///
/// let data = vec![1.0_f64, 2.0, f64::NAN, 4.0];
/// let sum = compute_sum(&data);
/// assert!((sum - 7.0).abs() < 1e-10);
/// ```
pub trait SeriesElement: Float + NumCast + Copy + Default + Send + Sync + 'static {
    /// Creates a series element from a `usize` value.
    ///
    /// This is commonly used for converting period parameters to the series element type.
    ///
    /// # Errors
    ///
    /// Returns `Error::NumericConversion` if the value cannot be represented in this type.
    #[inline]
    fn from_usize(value: usize) -> Result<Self> {
        <Self as NumCast>::from(value).ok_or(Error::NumericConversion {
            context: "usize to series element",
        })
    }

    /// Creates a series element from an `i32` value.
    ///
    /// # Errors
    ///
    /// Returns `Error::NumericConversion` if the value cannot be represented in this type.
    #[inline]
    fn from_i32(value: i32) -> Result<Self> {
        <Self as NumCast>::from(value).ok_or(Error::NumericConversion {
            context: "i32 to series element",
        })
    }

    /// Creates a series element from an `f64` value.
    ///
    /// # Errors
    ///
    /// Returns `Error::NumericConversion` if the value cannot be represented in this type.
    #[inline]
    fn from_f64(value: f64) -> Result<Self> {
        <Self as NumCast>::from(value).ok_or(Error::NumericConversion {
            context: "f64 to series element",
        })
    }

    /// Creates a series element from an `f32` value.
    ///
    /// # Errors
    ///
    /// Returns `Error::NumericConversion` if the value cannot be represented in this type.
    #[inline]
    fn from_f32(value: f32) -> Result<Self> {
        <Self as NumCast>::from(value).ok_or(Error::NumericConversion {
            context: "f32 to series element",
        })
    }

    /// Returns the constant 2 as this type.
    ///
    /// This is commonly used in EMA calculations: `alpha = 2 / (period + 1)`.
    #[inline]
    #[must_use]
    fn two() -> Self {
        // Safe unwrap: 2 is always representable in Float types
        <Self as NumCast>::from(2).unwrap()
    }

    /// Returns the constant 100 as this type.
    ///
    /// This is commonly used for percentage calculations in indicators like RSI and Stochastic.
    #[inline]
    #[must_use]
    fn hundred() -> Self {
        // Safe unwrap: 100 is always representable in Float types
        <Self as NumCast>::from(100).unwrap()
    }

    /// Returns the constant 50 as this type.
    ///
    /// This is commonly used as a neutral/midpoint value (e.g., RSI=50 is neutral).
    #[inline]
    #[must_use]
    fn fifty() -> Self {
        // Safe unwrap: 50 is always representable in Float types
        <Self as NumCast>::from(50).unwrap()
    }
}

// Blanket implementation for all types that satisfy the bounds
impl<T: Float + NumCast + Copy + Default + Send + Sync + 'static> SeriesElement for T {}

/// Trait for validating input data before indicator computation.
///
/// This trait provides validation methods to check that input data meets
/// the requirements of an indicator before computation begins.
pub trait ValidatedInput {
    /// The element type of the series.
    type Element: SeriesElement;

    /// Returns the length of the series.
    fn len(&self) -> usize;

    /// Returns true if the series is empty.
    #[inline]
    #[must_use]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Validates that the series has at least `min_length` elements.
    ///
    /// # Errors
    ///
    /// Returns `Error::InsufficientData` if the series is shorter than `min_length`.
    #[inline]
    fn validate_min_length(&self, min_length: usize, indicator: &'static str) -> Result<()> {
        if self.len() < min_length {
            Err(Error::InsufficientData {
                required: min_length,
                actual: self.len(),
                indicator,
            })
        } else {
            Ok(())
        }
    }

    /// Validates that the series is not empty.
    ///
    /// # Errors
    ///
    /// Returns `Error::EmptyInput` if the series is empty.
    #[inline]
    fn validate_not_empty(&self) -> Result<()> {
        if self.is_empty() {
            Err(Error::EmptyInput)
        } else {
            Ok(())
        }
    }
}

// Implementation for slices
impl<T: SeriesElement> ValidatedInput for [T] {
    type Element = T;

    #[inline]
    fn len(&self) -> usize {
        self.len()
    }
}

// Implementation for Vec
impl<T: SeriesElement> ValidatedInput for Vec<T> {
    type Element = T;

    #[inline]
    fn len(&self) -> usize {
        self.len()
    }
}

/// Validates that a period is valid for indicator computation.
///
/// # Errors
///
/// Returns `Error::InvalidPeriod` if the period is zero.
#[inline]
pub const fn validate_period(period: usize) -> Result<()> {
    if period == 0 {
        Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        })
    } else {
        Ok(())
    }
}

/// Validates that input data is suitable for indicator computation.
///
/// This function performs the following checks:
/// 1. The period is valid (non-zero)
/// 2. The data is not empty
/// 3. The data has at least `period` elements
///
/// # Errors
///
/// Returns an appropriate error if any validation fails:
/// - `Error::InvalidPeriod` if the period is zero
/// - `Error::EmptyInput` if the data is empty
/// - `Error::InsufficientData` if data length is less than the period
#[inline]
pub fn validate_indicator_input<T: SeriesElement>(
    data: &[T],
    period: usize,
    indicator: &'static str,
) -> Result<()> {
    validate_period(period)?;
    data.validate_not_empty()?;
    data.validate_min_length(period, indicator)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_series_element_from_usize() {
        let val: f64 = SeriesElement::from_usize(42).unwrap();
        assert!((val - 42.0).abs() < 1e-10);

        let val_f32: f32 = SeriesElement::from_usize(100).unwrap();
        assert!((val_f32 - 100.0).abs() < 1e-5);
    }

    #[test]
    fn test_series_element_from_i32() {
        let val: f64 = SeriesElement::from_i32(-5).unwrap();
        assert!((val - (-5.0)).abs() < 1e-10);
    }

    #[test]
    fn test_series_element_from_f64() {
        let val: f64 = SeriesElement::from_f64(std::f64::consts::PI).unwrap();
        assert!((val - std::f64::consts::PI).abs() < 1e-10);

        // Test conversion from f64 to f32 (may lose precision)
        let val_f32: f32 = SeriesElement::from_f64(std::f64::consts::PI).unwrap();
        assert!((val_f32 - std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn test_series_element_two() {
        let two_f64: f64 = SeriesElement::two();
        assert!((two_f64 - 2.0).abs() < 1e-10);

        let two_f32: f32 = SeriesElement::two();
        assert!((two_f32 - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_series_element_from_f32() {
        let val: f64 = SeriesElement::from_f32(3.14159_f32).unwrap();
        assert!((val - 3.14159).abs() < 1e-5);

        let val_f32: f32 = SeriesElement::from_f32(2.71828_f32).unwrap();
        assert!((val_f32 - 2.71828).abs() < 1e-5);
    }

    #[test]
    fn test_series_element_hundred() {
        let hundred_f64: f64 = SeriesElement::hundred();
        assert!((hundred_f64 - 100.0).abs() < 1e-10);

        let hundred_f32: f32 = SeriesElement::hundred();
        assert!((hundred_f32 - 100.0).abs() < 1e-5);
    }

    #[test]
    fn test_series_element_fifty() {
        let fifty_f64: f64 = SeriesElement::fifty();
        assert!((fifty_f64 - 50.0).abs() < 1e-10);

        let fifty_f32: f32 = SeriesElement::fifty();
        assert!((fifty_f32 - 50.0).abs() < 1e-5);
    }

    #[test]
    fn test_validated_input_len() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0];
        assert_eq!(ValidatedInput::len(&data), 3);

        let slice: &[f64] = &[1.0, 2.0, 3.0, 4.0];
        assert_eq!(ValidatedInput::len(slice), 4);
    }

    #[test]
    fn test_validated_input_is_empty() {
        let empty: Vec<f64> = vec![];
        assert!(ValidatedInput::is_empty(&empty));

        let non_empty: Vec<f64> = vec![1.0];
        assert!(!ValidatedInput::is_empty(&non_empty));
    }

    #[test]
    fn test_validate_min_length_success() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(data.validate_min_length(5, "test").is_ok());
        assert!(data.validate_min_length(3, "test").is_ok());
    }

    #[test]
    fn test_validate_min_length_failure() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0];
        let result = data.validate_min_length(5, "test");
        assert!(result.is_err());
        match result {
            Err(Error::InsufficientData {
                required, actual, ..
            }) => {
                assert_eq!(required, 5);
                assert_eq!(actual, 3);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_validate_not_empty_success() {
        let data: Vec<f64> = vec![1.0];
        assert!(data.validate_not_empty().is_ok());
    }

    #[test]
    fn test_validate_not_empty_failure() {
        let data: Vec<f64> = vec![];
        let result = data.validate_not_empty();
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_validate_period_success() {
        assert!(validate_period(1).is_ok());
        assert!(validate_period(100).is_ok());
    }

    #[test]
    fn test_validate_period_zero() {
        let result = validate_period(0);
        assert!(result.is_err());
        match result {
            Err(Error::InvalidPeriod { period, reason }) => {
                assert_eq!(period, 0);
                assert!(!reason.is_empty());
            }
            _ => panic!("Expected InvalidPeriod error"),
        }
    }

    #[test]
    fn test_validate_indicator_input_success() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(validate_indicator_input(&data, 3, "test").is_ok());
        assert!(validate_indicator_input(&data, 5, "test").is_ok());
    }

    #[test]
    fn test_validate_indicator_input_empty() {
        let data: Vec<f64> = vec![];
        let result = validate_indicator_input(&data, 3, "test");
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { .. } | Error::EmptyInput)
        ));
    }

    #[test]
    fn test_validate_indicator_input_insufficient() {
        let data: Vec<f64> = vec![1.0, 2.0];
        let result = validate_indicator_input(&data, 5, "test");
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_validate_indicator_input_zero_period() {
        let data: Vec<f64> = vec![1.0, 2.0, 3.0];
        let result = validate_indicator_input(&data, 0, "test");
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_series_element_nan_handling() {
        // Test that NaN values work correctly
        let nan: f64 = f64::NAN;
        assert!(nan.is_nan());

        let data: Vec<f64> = vec![1.0, f64::NAN, 3.0];
        assert_eq!(data.len(), 3);
    }

    #[test]
    fn test_series_element_infinity_handling() {
        // Test that infinity values are representable
        let inf: f64 = f64::INFINITY;
        let neg_inf: f64 = f64::NEG_INFINITY;

        assert!(inf.is_infinite());
        assert!(neg_inf.is_infinite());
        assert!(inf.is_sign_positive());
        assert!(neg_inf.is_sign_negative());
    }

    #[test]
    fn test_slice_validated_input() {
        let slice: &[f64] = &[1.0, 2.0, 3.0];
        assert!(slice.validate_min_length(2, "test").is_ok());
        assert!(slice.validate_not_empty().is_ok());
    }

    #[test]
    fn test_series_element_default() {
        // Test that Default is implemented (returns zero)
        let default: f64 = f64::default();
        assert!((default - 0.0).abs() < 1e-10);

        let default_f32: f32 = f32::default();
        assert!((default_f32 - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_series_element_negative_zero() {
        // Test that negative zero (-0.0) is handled correctly
        // IEEE 754: -0.0 == 0.0 but they have different bit representations
        let neg_zero_f64: f64 = -0.0;
        let pos_zero_f64: f64 = 0.0;

        // -0.0 and 0.0 compare equal
        assert!(neg_zero_f64 == pos_zero_f64);

        // But have different signs
        assert!(neg_zero_f64.is_sign_negative());
        assert!(pos_zero_f64.is_sign_positive());

        // Can be distinguished using to_bits
        assert_ne!(neg_zero_f64.to_bits(), pos_zero_f64.to_bits());

        // Same tests for f32
        let neg_zero_f32: f32 = -0.0;
        let pos_zero_f32: f32 = 0.0;
        assert!(neg_zero_f32 == pos_zero_f32);
        assert!(neg_zero_f32.is_sign_negative());
        assert!(pos_zero_f32.is_sign_positive());
    }

    #[test]
    fn test_series_element_negative_zero_in_data() {
        // Test that negative zero works in validated input
        let data: Vec<f64> = vec![-0.0, 0.0, 1.0];
        assert!(validate_indicator_input(&data, 2, "test").is_ok());
        assert_eq!(data.len(), 3);

        // Negative zero is not NaN
        assert!(!(-0.0_f64).is_nan());

        // Negative zero is finite
        assert!((-0.0_f64).is_finite());
    }

    #[test]
    fn test_series_element_negative_zero_arithmetic() {
        // Test that negative zero behaves correctly in arithmetic
        let neg_zero: f64 = -0.0;

        // Adding to negative zero
        let sum = neg_zero + 1.0;
        assert!((sum - 1.0).abs() < 1e-10);

        // Multiplying negative zero
        let product = neg_zero * 2.0;
        assert!(product == 0.0);
        // Note: -0.0 * positive = -0.0
        assert!(product.is_sign_negative());

        // Division involving negative zero
        // 1.0 / -0.0 = -inf
        let div_result = 1.0 / neg_zero;
        assert!(div_result.is_infinite());
        assert!(div_result.is_sign_negative());
    }

    #[test]
    fn test_series_element_send_sync() {
        // Compile-time test that SeriesElement types are Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<f64>();
        assert_send_sync::<f32>();
    }

    #[test]
    fn test_series_element_large_usize_f64() {
        // Large usize values should convert to f64 without issue
        let large: usize = 1_000_000_000;
        let val: f64 = SeriesElement::from_usize(large).unwrap();
        assert!((val - 1e9).abs() < 1.0);
    }

    #[test]
    fn test_series_element_large_usize_f32() {
        // Large usize may lose precision in f32, but should still succeed
        let large: usize = 16_777_216; // 2^24, max exact integer in f32
        let val: f32 = SeriesElement::from_usize(large).unwrap();
        assert!((val - 16_777_216.0).abs() < 1.0);
    }

    #[test]
    fn test_series_element_negative_i32() {
        let val: f64 = SeriesElement::from_i32(-100).unwrap();
        assert!((val - (-100.0)).abs() < 1e-10);

        let val_f32: f32 = SeriesElement::from_i32(-42).unwrap();
        assert!((val_f32 - (-42.0)).abs() < 1e-5);
    }

    #[test]
    fn test_constants_in_calculations() {
        // Verify constants work in typical indicator calculations
        // RSI percentage: gain / (gain + loss) * 100
        let gain: f64 = 0.25;
        let loss: f64 = 0.75;
        let hundred: f64 = SeriesElement::hundred();
        let rsi = (gain / (gain + loss)) * hundred;
        assert!((rsi - 25.0).abs() < 1e-10);

        // Neutral midpoint check
        let fifty: f64 = SeriesElement::fifty();
        assert!(rsi < fifty); // 25 < 50, bearish

        // EMA alpha: 2 / (period + 1)
        let two: f64 = SeriesElement::two();
        let period: f64 = 13.0;
        let alpha = two / (period + 1.0);
        assert!((alpha - (2.0 / 14.0)).abs() < 1e-10);
    }
}
