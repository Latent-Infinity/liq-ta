//! CLI error types for handling file I/O, parsing, and indicator errors.
//!
//! This module provides the [`CliError`] enum which wraps all possible errors
//! that can occur during CLI operations. Error messages are designed to be
//! actionable, providing both what went wrong and how to fix it.

use std::fmt;
use std::io;

/// CLI error type encompassing all possible error conditions.
///
/// Each variant provides context about what went wrong and, where applicable,
/// suggestions for how to fix the issue.
#[derive(Debug)]
pub enum CliError {
    /// An I/O error occurred while reading or writing files.
    IoError {
        /// The underlying I/O error.
        source: io::Error,
        /// Path that caused the error, if known.
        path: Option<String>,
    },
    /// An error occurred while parsing CSV data.
    CsvParseError {
        /// Description of the parse error.
        message: String,
        /// Line number where the error occurred, if known.
        line: Option<usize>,
    },
    /// An error occurred while computing an indicator.
    IndicatorError {
        /// The underlying liq-ta error.
        source: liq_ta::Error,
    },
    /// An invalid argument was provided.
    InvalidArgument {
        /// Name of the invalid argument.
        argument: String,
        /// Description of why it's invalid.
        reason: String,
        /// Suggestion for valid values.
        suggestion: Option<String>,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::IoError { source, path } => {
                if let Some(p) = path {
                    write!(f, "I/O error with file '{p}': {source}. ")?;
                    write!(
                        f,
                        "Check that the file exists and you have read permissions."
                    )
                } else {
                    write!(f, "I/O error: {source}")
                }
            }
            CliError::CsvParseError { message, line } => {
                if let Some(l) = line {
                    write!(f, "CSV parse error on line {l}: {message}. ")?;
                } else {
                    write!(f, "CSV parse error: {message}. ")?;
                }
                write!(
                    f,
                    "Ensure your CSV has valid format with numeric data columns."
                )
            }
            CliError::IndicatorError { source } => {
                write!(f, "Indicator computation error: {source}")
            }
            CliError::InvalidArgument {
                argument,
                reason,
                suggestion,
            } => {
                write!(f, "Invalid argument '{argument}': {reason}")?;
                if let Some(s) = suggestion {
                    write!(f, ". {s}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliError::IoError { source, .. } => Some(source),
            CliError::IndicatorError { source } => Some(source),
            CliError::CsvParseError { .. } | CliError::InvalidArgument { .. } => None,
        }
    }
}

impl From<io::Error> for CliError {
    fn from(err: io::Error) -> Self {
        CliError::IoError {
            source: err,
            path: None,
        }
    }
}

impl From<liq_ta::Error> for CliError {
    fn from(err: liq_ta::Error) -> Self {
        CliError::IndicatorError { source: err }
    }
}

impl From<csv::Error> for CliError {
    fn from(err: csv::Error) -> Self {
        let line = err.position().map(|p| p.line() as usize);
        CliError::CsvParseError {
            message: err.to_string(),
            line,
        }
    }
}

/// Result type alias for CLI operations.
pub type Result<T> = std::result::Result<T, CliError>;

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Error Construction Tests
    // ==========================================================================

    #[test]
    fn test_io_error_construction_with_path() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = CliError::IoError {
            source: io_err,
            path: Some("/path/to/file.csv".to_string()),
        };

        match err {
            CliError::IoError { path, .. } => {
                assert_eq!(path, Some("/path/to/file.csv".to_string()));
            }
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_io_error_construction_without_path() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let err = CliError::IoError {
            source: io_err,
            path: None,
        };

        match err {
            CliError::IoError { path, .. } => {
                assert!(path.is_none());
            }
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_csv_parse_error_construction_with_line() {
        let err = CliError::CsvParseError {
            message: "invalid number".to_string(),
            line: Some(42),
        };

        match err {
            CliError::CsvParseError { message, line } => {
                assert_eq!(message, "invalid number");
                assert_eq!(line, Some(42));
            }
            _ => panic!("Expected CsvParseError variant"),
        }
    }

    #[test]
    fn test_csv_parse_error_construction_without_line() {
        let err = CliError::CsvParseError {
            message: "unexpected EOF".to_string(),
            line: None,
        };

        match err {
            CliError::CsvParseError { message, line } => {
                assert_eq!(message, "unexpected EOF");
                assert!(line.is_none());
            }
            _ => panic!("Expected CsvParseError variant"),
        }
    }

    #[test]
    fn test_indicator_error_construction() {
        let ta_err = liq_ta::Error::EmptyInput;
        let err = CliError::IndicatorError { source: ta_err };

        match err {
            CliError::IndicatorError { source } => {
                assert!(matches!(source, liq_ta::Error::EmptyInput));
            }
            _ => panic!("Expected IndicatorError variant"),
        }
    }

    #[test]
    fn test_invalid_argument_construction_with_suggestion() {
        let err = CliError::InvalidArgument {
            argument: "period".to_string(),
            reason: "must be positive".to_string(),
            suggestion: Some("Try a value like 14 or 20".to_string()),
        };

        match err {
            CliError::InvalidArgument {
                argument,
                reason,
                suggestion,
            } => {
                assert_eq!(argument, "period");
                assert_eq!(reason, "must be positive");
                assert_eq!(suggestion, Some("Try a value like 14 or 20".to_string()));
            }
            _ => panic!("Expected InvalidArgument variant"),
        }
    }

    #[test]
    fn test_invalid_argument_construction_without_suggestion() {
        let err = CliError::InvalidArgument {
            argument: "indicator".to_string(),
            reason: "unknown indicator 'xyz'".to_string(),
            suggestion: None,
        };

        match err {
            CliError::InvalidArgument {
                argument,
                reason,
                suggestion,
            } => {
                assert_eq!(argument, "indicator");
                assert_eq!(reason, "unknown indicator 'xyz'");
                assert!(suggestion.is_none());
            }
            _ => panic!("Expected InvalidArgument variant"),
        }
    }

    // ==========================================================================
    // Display Implementation Tests
    // ==========================================================================

    #[test]
    fn test_display_io_error_with_path() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = CliError::IoError {
            source: io_err,
            path: Some("/path/to/file.csv".to_string()),
        };

        let display = format!("{err}");
        assert!(display.contains("/path/to/file.csv"));
        assert!(display.contains("file not found"));
        assert!(display.contains("Check that the file exists"));
    }

    #[test]
    fn test_display_io_error_without_path() {
        let io_err = io::Error::other("network error");
        let err = CliError::IoError {
            source: io_err,
            path: None,
        };

        let display = format!("{err}");
        assert!(display.contains("I/O error"));
        assert!(display.contains("network error"));
    }

    #[test]
    fn test_display_csv_parse_error_with_line() {
        let err = CliError::CsvParseError {
            message: "invalid float".to_string(),
            line: Some(10),
        };

        let display = format!("{err}");
        assert!(display.contains("line 10"));
        assert!(display.contains("invalid float"));
        assert!(display.contains("valid format"));
    }

    #[test]
    fn test_display_csv_parse_error_without_line() {
        let err = CliError::CsvParseError {
            message: "unexpected end of input".to_string(),
            line: None,
        };

        let display = format!("{err}");
        assert!(display.contains("unexpected end of input"));
        assert!(display.contains("valid format"));
    }

    #[test]
    fn test_display_indicator_error() {
        let ta_err = liq_ta::Error::EmptyInput;
        let err = CliError::IndicatorError { source: ta_err };

        let display = format!("{err}");
        assert!(display.contains("Indicator computation error"));
    }

    #[test]
    fn test_display_invalid_argument_with_suggestion() {
        let err = CliError::InvalidArgument {
            argument: "period".to_string(),
            reason: "cannot be zero".to_string(),
            suggestion: Some("Use a positive integer like 14".to_string()),
        };

        let display = format!("{err}");
        assert!(display.contains("'period'"));
        assert!(display.contains("cannot be zero"));
        assert!(display.contains("positive integer like 14"));
    }

    #[test]
    fn test_display_invalid_argument_without_suggestion() {
        let err = CliError::InvalidArgument {
            argument: "output".to_string(),
            reason: "invalid format".to_string(),
            suggestion: None,
        };

        let display = format!("{err}");
        assert!(display.contains("'output'"));
        assert!(display.contains("invalid format"));
    }

    // ==========================================================================
    // From Trait Tests
    // ==========================================================================

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "not found");
        let cli_err: CliError = io_err.into();

        match cli_err {
            CliError::IoError { path, .. } => {
                assert!(path.is_none());
            }
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_from_liq_ta_error() {
        let ta_err = liq_ta::Error::EmptyInput;
        let cli_err: CliError = ta_err.into();

        assert!(matches!(cli_err, CliError::IndicatorError { .. }));
    }

    #[test]
    fn test_from_csv_error() {
        // Create a CSV error by parsing invalid CSV
        let result: std::result::Result<csv::StringRecord, csv::Error> = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader("a,b\n1,\"unterminated".as_bytes())
            .records()
            .last()
            .unwrap();

        if let Err(csv_err) = result {
            let cli_err: CliError = csv_err.into();
            assert!(matches!(cli_err, CliError::CsvParseError { .. }));
        }
    }

    // ==========================================================================
    // Error Source Chain Tests
    // ==========================================================================

    #[test]
    fn test_error_source_io() {
        use std::error::Error;

        let io_err = io::Error::new(io::ErrorKind::NotFound, "not found");
        let err = CliError::IoError {
            source: io_err,
            path: None,
        };

        assert!(err.source().is_some());
    }

    #[test]
    fn test_error_source_indicator() {
        use std::error::Error;

        let ta_err = liq_ta::Error::EmptyInput;
        let err = CliError::IndicatorError { source: ta_err };

        assert!(err.source().is_some());
    }

    #[test]
    fn test_error_source_csv_parse() {
        use std::error::Error;

        let err = CliError::CsvParseError {
            message: "test".to_string(),
            line: None,
        };

        assert!(err.source().is_none());
    }

    #[test]
    fn test_error_source_invalid_argument() {
        use std::error::Error;

        let err = CliError::InvalidArgument {
            argument: "test".to_string(),
            reason: "test".to_string(),
            suggestion: None,
        };

        assert!(err.source().is_none());
    }

    // ==========================================================================
    // Result Type Alias Test
    // ==========================================================================

    #[test]
    fn test_result_type_alias() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(CliError::InvalidArgument {
                argument: "test".to_string(),
                reason: "test".to_string(),
                suggestion: None,
            })
        }

        assert!(returns_ok().is_ok());
        assert!(returns_err().is_err());
    }
}
