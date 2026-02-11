//! Error types for liq-ta.
//!
//! This module defines the error types used throughout the liq-ta library
//! for handling various failure conditions. All errors follow the Gravity Check
//! principle of being actionable: they explain what failed, why, and how to fix it.
//!
//! # Supported Conversions
//!
//! The `Error` type implements key `From` conversions for ergonomic error handling:
//!
//! - `From<String>` - Creates a `LengthMismatch` error from a description string
//! - `From<&str>` - Creates a `LengthMismatch` error from a static description
//! - `Into<Box<dyn std::error::Error>>` - Automatic via `std::error::Error` impl
//! - `Into<Box<dyn std::error::Error + Send + Sync>>` - For use with `anyhow`, etc.
//!
//! # Example
//!
//! ```
//! use liq_ta::Error;
//!
//! // From String
//! let err: Error = "high and low arrays differ in length".to_string().into();
//!
//! // From &str
//! let err: Error = "mismatched input lengths".into();
//!
//! // Into Box<dyn Error>
//! let boxed: Box<dyn std::error::Error> = Error::EmptyInput.into();
//! ```

use thiserror::Error;

/// The main error type for liq-ta operations.
///
/// Each variant provides context about what went wrong and, where applicable,
/// guidance on how to fix the issue.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// The input data series is empty.
    ///
    /// This error is returned when no data is provided to an indicator function.
    ///
    /// # How to Fix
    /// Ensure your input data contains at least one element. For most indicators,
    /// you'll need at least `period` elements.
    #[error(
        "empty input: no data provided. Provide at least one data point, or check that your data source is returning results"
    )]
    EmptyInput,

    /// The input data series is too short for the requested operation.
    ///
    /// This error is returned when the input data has fewer elements than
    /// required by the indicator's period or lookback window.
    ///
    /// # How to Fix
    /// Either provide more data or use a smaller period. Use `*_min_len()` functions
    /// to determine the minimum required input length for a given indicator configuration.
    #[error(
        "insufficient data: need {required} elements but got {actual}. Provide more data or use a smaller period (use `{indicator}_min_len()` to check minimum requirements)"
    )]
    InsufficientData {
        /// The number of data points required.
        required: usize,
        /// The number of data points provided.
        actual: usize,
        /// Name of the indicator for the error message.
        indicator: &'static str,
    },

    /// The output buffer is too small to hold the result.
    ///
    /// This error is returned from `*_into()` functions when the provided
    /// output buffer doesn't have enough capacity.
    ///
    /// # How to Fix
    /// Allocate a buffer with at least `input.len() - lookback` elements,
    /// or use the non-`_into` variant which allocates automatically.
    #[error(
        "buffer too small: need capacity for {required} elements but buffer has {actual}. Use `{indicator}_lookback()` to calculate required output size: input_len - lookback"
    )]
    BufferTooSmall {
        /// The minimum buffer capacity required.
        required: usize,
        /// The actual buffer capacity provided.
        actual: usize,
        /// Name of the indicator for the error message.
        indicator: &'static str,
    },

    /// The period parameter is invalid.
    ///
    /// This error is returned when the period is zero or otherwise invalid
    /// for the requested operation.
    ///
    /// # How to Fix
    /// Use a period of at least 1. Common periods are 14 for RSI/ATR,
    /// 20 for Bollinger Bands, and 12/26/9 for MACD.
    #[error(
        "invalid period {period}: {reason}. Use a positive integer (common values: RSI=14, SMA/EMA=20, MACD=12/26/9)"
    )]
    InvalidPeriod {
        /// The invalid period value that was provided.
        period: usize,
        /// Description of why the period is invalid.
        reason: &'static str,
    },

    /// OHLC input series have mismatched lengths.
    ///
    /// This error is returned when high, low, close (and optionally open)
    /// series have different lengths.
    ///
    /// # How to Fix
    /// Ensure all OHLC series have the same length. Check your data source
    /// for missing or extra values.
    #[error(
        "length mismatch in OHLC data: {description}. Ensure high, low, and close arrays all have the same length"
    )]
    LengthMismatch {
        /// Description of the mismatch.
        description: String,
    },

    /// Failed to convert a numeric value to the target type.
    ///
    /// This error occurs when using `NumCast::from()` to convert values
    /// (e.g., converting a `usize` period to a generic `Float` type) and
    /// the conversion fails.
    ///
    /// # How to Fix
    /// This is typically an internal error. If you encounter it, please
    /// report it as a bug with the input values that caused it.
    #[error(
        "numeric conversion failed: {context}. This is likely a bug - please report with your input values"
    )]
    NumericConversion {
        /// Description of the conversion that failed.
        context: &'static str,
    },
}

// ==========================================================================
// From Implementations
// ==========================================================================

impl From<String> for Error {
    /// Convert a String into a `LengthMismatch` error.
    ///
    /// This is useful when you need to construct an error from a dynamically
    /// generated error message, particularly for OHLC length validation.
    ///
    /// # Example
    ///
    /// ```
    /// use liq_ta::Error;
    ///
    /// let err: Error = format!("high has {} elements, low has {}", 100, 99).into();
    /// assert!(err.to_string().contains("100"));
    /// ```
    fn from(description: String) -> Self {
        Self::LengthMismatch { description }
    }
}

impl From<&str> for Error {
    /// Convert a &str into a `LengthMismatch` error.
    ///
    /// This is a convenience for creating errors from static strings.
    ///
    /// # Example
    ///
    /// ```
    /// use liq_ta::Error;
    ///
    /// let err: Error = "arrays have different lengths".into();
    /// assert!(err.to_string().contains("different lengths"));
    /// ```
    fn from(description: &str) -> Self {
        Self::LengthMismatch {
            description: description.to_string(),
        }
    }
}

/// Convenience type alias for Results using the liq-ta Error type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Construction Tests - verify each variant can be constructed
    // ==========================================================================

    #[test]
    fn test_empty_input_construction() {
        let err = Error::EmptyInput;
        assert!(matches!(err, Error::EmptyInput));
    }

    #[test]
    fn test_insufficient_data_construction() {
        let err = Error::InsufficientData {
            required: 20,
            actual: 10,
            indicator: "sma",
        };
        match err {
            Error::InsufficientData {
                required,
                actual,
                indicator,
            } => {
                assert_eq!(required, 20);
                assert_eq!(actual, 10);
                assert_eq!(indicator, "sma");
            }
            _ => panic!("Expected InsufficientData variant"),
        }
    }

    #[test]
    fn test_buffer_too_small_construction() {
        let err = Error::BufferTooSmall {
            required: 100,
            actual: 50,
            indicator: "ema",
        };
        match err {
            Error::BufferTooSmall {
                required,
                actual,
                indicator,
            } => {
                assert_eq!(required, 100);
                assert_eq!(actual, 50);
                assert_eq!(indicator, "ema");
            }
            _ => panic!("Expected BufferTooSmall variant"),
        }
    }

    #[test]
    fn test_invalid_period_construction() {
        let err = Error::InvalidPeriod {
            period: 0,
            reason: "period must be at least 1",
        };
        match err {
            Error::InvalidPeriod { period, reason } => {
                assert_eq!(period, 0);
                assert_eq!(reason, "period must be at least 1");
            }
            _ => panic!("Expected InvalidPeriod variant"),
        }
    }

    #[test]
    fn test_length_mismatch_construction() {
        let err = Error::LengthMismatch {
            description: "high has 100 elements, low has 99".to_string(),
        };
        match err {
            Error::LengthMismatch { description } => {
                assert_eq!(description, "high has 100 elements, low has 99");
            }
            _ => panic!("Expected LengthMismatch variant"),
        }
    }

    #[test]
    fn test_numeric_conversion_construction() {
        let err = Error::NumericConversion {
            context: "converting period to float",
        };
        match err {
            Error::NumericConversion { context } => {
                assert_eq!(context, "converting period to float");
            }
            _ => panic!("Expected NumericConversion variant"),
        }
    }

    // ==========================================================================
    // Display Tests - verify error messages are clear and actionable
    // ==========================================================================

    #[test]
    fn test_empty_input_display_is_actionable() {
        let err = Error::EmptyInput;
        let msg = err.to_string();
        // Message should explain what failed
        assert!(msg.contains("empty input"), "Should mention empty input");
        // Message should suggest how to fix
        assert!(
            msg.contains("Provide") || msg.contains("check"),
            "Should suggest a fix"
        );
    }

    #[test]
    fn test_insufficient_data_display_is_actionable() {
        let err = Error::InsufficientData {
            required: 20,
            actual: 10,
            indicator: "sma",
        };
        let msg = err.to_string();
        // Message should explain what's needed
        assert!(msg.contains("20"), "Should mention required count");
        assert!(msg.contains("10"), "Should mention actual count");
        // Message should suggest how to fix
        assert!(
            msg.contains("min_len") || msg.contains("smaller period"),
            "Should suggest using min_len or smaller period"
        );
    }

    #[test]
    fn test_buffer_too_small_display_is_actionable() {
        let err = Error::BufferTooSmall {
            required: 100,
            actual: 50,
            indicator: "ema",
        };
        let msg = err.to_string();
        // Message should explain what's needed
        assert!(msg.contains("100"), "Should mention required capacity");
        assert!(msg.contains("50"), "Should mention actual capacity");
        // Message should suggest how to fix
        assert!(
            msg.contains("lookback"),
            "Should mention using lookback to calculate size"
        );
    }

    #[test]
    fn test_invalid_period_display_is_actionable() {
        let err = Error::InvalidPeriod {
            period: 0,
            reason: "period must be at least 1",
        };
        let msg = err.to_string();
        // Message should explain what's invalid
        assert!(msg.contains('0'), "Should mention the invalid period");
        assert!(
            msg.contains("at least 1"),
            "Should mention the validation rule"
        );
        // Message should suggest common values
        assert!(
            msg.contains("14") || msg.contains("20"),
            "Should suggest common period values"
        );
    }

    #[test]
    fn test_length_mismatch_display_is_actionable() {
        let err = Error::LengthMismatch {
            description: "high has 100 elements, low has 99".to_string(),
        };
        let msg = err.to_string();
        // Message should explain the mismatch
        assert!(msg.contains("100"), "Should mention first length");
        assert!(msg.contains("99"), "Should mention second length");
        // Message should suggest how to fix
        assert!(
            msg.contains("same length"),
            "Should suggest ensuring same length"
        );
    }

    #[test]
    fn test_numeric_conversion_display_mentions_bug_report() {
        let err = Error::NumericConversion {
            context: "converting period to float",
        };
        let msg = err.to_string();
        // Message should mention it's likely a bug
        assert!(
            msg.contains("bug") || msg.contains("report"),
            "Should mention reporting as bug"
        );
    }

    // ==========================================================================
    // Equality and Clone Tests
    // ==========================================================================

    #[test]
    fn test_error_equality() {
        let err1 = Error::InsufficientData {
            required: 20,
            actual: 10,
            indicator: "sma",
        };
        let err2 = Error::InsufficientData {
            required: 20,
            actual: 10,
            indicator: "sma",
        };
        let err3 = Error::InsufficientData {
            required: 30,
            actual: 10,
            indicator: "sma",
        };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_error_clone() {
        let err = Error::InvalidPeriod {
            period: 5,
            reason: "test",
        };
        let err_clone = err.clone();
        assert_eq!(err, err_clone);
    }

    #[test]
    fn test_length_mismatch_clone() {
        let err = Error::LengthMismatch {
            description: "test mismatch".to_string(),
        };
        let err_clone = err.clone();
        assert_eq!(err, err_clone);
    }

    // ==========================================================================
    // Debug Tests
    // ==========================================================================

    #[test]
    fn test_error_debug_empty_input() {
        let err = Error::EmptyInput;
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("EmptyInput"));
    }

    #[test]
    fn test_error_debug_insufficient_data() {
        let err = Error::InsufficientData {
            required: 20,
            actual: 10,
            indicator: "rsi",
        };
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("InsufficientData"));
        assert!(debug_str.contains("20"));
        assert!(debug_str.contains("10"));
    }

    #[test]
    fn test_error_debug_buffer_too_small() {
        let err = Error::BufferTooSmall {
            required: 100,
            actual: 50,
            indicator: "macd",
        };
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("BufferTooSmall"));
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("50"));
    }

    // ==========================================================================
    // Result Type Alias Tests
    // ==========================================================================

    #[test]
    fn test_result_type_alias_ok() {
        fn test_fn() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(test_fn().unwrap(), 42);
    }

    #[test]
    fn test_result_type_alias_err() {
        fn test_fn() -> Result<i32> {
            Err(Error::EmptyInput)
        }
        assert!(test_fn().is_err());
    }

    // ==========================================================================
    // std::error::Error Trait Tests
    // ==========================================================================

    #[test]
    fn test_error_is_std_error() {
        fn accepts_std_error<E: std::error::Error>(_: E) {}
        let err = Error::EmptyInput;
        accepts_std_error(err);
    }

    #[test]
    fn test_error_source_is_none() {
        // Our errors don't wrap other errors, so source should be None
        use std::error::Error as StdError;
        let err = Error::EmptyInput;
        assert!(err.source().is_none());
    }

    // ==========================================================================
    // Send + Sync Tests (for thread safety)
    // ==========================================================================

    #[test]
    fn test_error_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Error>();
    }

    #[test]
    fn test_error_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Error>();
    }

    // ==========================================================================
    // From/Into Conversion Tests
    // ==========================================================================

    #[test]
    fn test_from_string() {
        // Test From<String> creates LengthMismatch
        let msg = "high has 100 elements, low has 99".to_string();
        let err: Error = msg.into();
        match err {
            Error::LengthMismatch { description } => {
                assert_eq!(description, "high has 100 elements, low has 99");
            }
            _ => panic!("Expected LengthMismatch variant"),
        }
    }

    #[test]
    fn test_from_str() {
        // Test From<&str> creates LengthMismatch
        let err: Error = "arrays differ in length".into();
        match err {
            Error::LengthMismatch { description } => {
                assert_eq!(description, "arrays differ in length");
            }
            _ => panic!("Expected LengthMismatch variant"),
        }
    }

    #[test]
    fn test_from_format_string() {
        // Test From with format! macro
        let high_len = 100;
        let low_len = 99;
        let err: Error = format!("high: {high_len}, low: {low_len}").into();
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("99"));
    }

    #[test]
    fn test_error_into_box_dyn_error() {
        // Verify Error can be converted to Box<dyn std::error::Error>
        // This is the primary conversion used when propagating errors across API boundaries
        let err = Error::EmptyInput;
        let boxed: Box<dyn std::error::Error> = Box::new(err);
        assert!(boxed.to_string().contains("empty input"));
    }

    #[test]
    fn test_error_into_box_dyn_error_via_into() {
        // Test the Into conversion directly
        let err = Error::EmptyInput;
        let boxed: Box<dyn std::error::Error> = err.into();
        assert!(boxed.to_string().contains("empty input"));
    }

    #[test]
    fn test_error_into_box_dyn_error_send_sync() {
        // Verify Error can be converted to Box<dyn Error + Send + Sync>
        // This is required for use with anyhow and other error handling crates
        let err = Error::InvalidPeriod {
            period: 0,
            reason: "test",
        };
        let boxed: Box<dyn std::error::Error + Send + Sync> = err.into();
        assert!(boxed.to_string().contains("invalid period"));
    }

    #[test]
    fn test_result_map_err_to_box() {
        // Verify Result<T, Error> can be mapped to Result<T, Box<dyn Error>>
        fn returns_result() -> Result<i32> {
            Err(Error::EmptyInput)
        }

        let boxed_result: std::result::Result<i32, Box<dyn std::error::Error>> =
            returns_result().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
        assert!(boxed_result.is_err());
    }

    #[test]
    fn test_from_conversions_in_result_context() {
        // Test using From in a Result context with ? operator pattern
        fn validate_lengths(high_len: usize, low_len: usize) -> Result<()> {
            if high_len != low_len {
                return Err(format!("high: {high_len}, low: {low_len}").into());
            }
            Ok(())
        }

        let result = validate_lengths(100, 99);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("100"));
    }
}
