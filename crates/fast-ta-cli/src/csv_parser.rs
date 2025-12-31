//! CSV parsing module for reading price data from CSV files.
//!
//! This module provides functionality to parse CSV files containing price data
//! for technical analysis. It supports various formats including single-column
//! close prices, OHLC data, and OHLCV data.
//!
//! # Column Detection
//!
//! The parser automatically detects columns based on header names (case-insensitive):
//! - `close`, `price`, `adj close`, `adjusted close` → close prices
//! - `open` → open prices
//! - `high` → high prices
//! - `low` → low prices
//! - `volume`, `vol` → volume
//!
//! Date columns (`date`, `time`, `datetime`, `timestamp`) are preserved for output
//! alignment but not parsed as numeric data.

use crate::error::{CliError, Result};
use csv::ReaderBuilder;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// OHLC price data structure.
#[derive(Debug, Clone)]
pub struct OhlcData {
    /// Date/time strings (if present in CSV).
    pub dates: Option<Vec<String>>,
    /// Open prices.
    pub open: Vec<f64>,
    /// High prices.
    pub high: Vec<f64>,
    /// Low prices.
    pub low: Vec<f64>,
    /// Close prices.
    pub close: Vec<f64>,
}

/// OHLCV price data structure (OHLC + Volume).
#[derive(Debug, Clone)]
pub struct OhlcvData {
    /// Date/time strings (if present in CSV).
    pub dates: Option<Vec<String>>,
    /// Open prices.
    pub open: Vec<f64>,
    /// High prices.
    pub high: Vec<f64>,
    /// Low prices.
    pub low: Vec<f64>,
    /// Close prices.
    pub close: Vec<f64>,
    /// Volume.
    pub volume: Vec<f64>,
}

/// Parsed CSV data with column mapping.
#[derive(Debug, Clone)]
pub struct ParsedCsv {
    /// Column headers from the CSV.
    pub headers: Vec<String>,
    /// Mapping of normalized column name to column index.
    pub column_map: HashMap<String, usize>,
    /// Date column values (if found).
    pub dates: Option<Vec<String>>,
    /// All numeric data columns by index.
    pub columns: HashMap<usize, Vec<f64>>,
    /// Number of rows parsed.
    pub row_count: usize,
}

impl ParsedCsv {
    /// Get a column by normalized name (e.g., "close", "high").
    #[must_use] 
    pub fn get_column(&self, name: &str) -> Option<&Vec<f64>> {
        self.column_map
            .get(name)
            .and_then(|idx| self.columns.get(idx))
    }

    /// Take ownership of a column by normalized name, removing it from storage.
    pub fn take_column(&mut self, name: &str) -> Option<Vec<f64>> {
        let idx = self.column_map.get(name).copied()?;
        self.columns.remove(&idx)
    }

    /// Get close prices, trying multiple common column names.
    #[must_use] 
    pub fn get_close(&self) -> Option<&Vec<f64>> {
        self.get_column("close")
            .or_else(|| self.get_column("price"))
            .or_else(|| self.get_column("adj close"))
            .or_else(|| self.get_column("adjusted close"))
    }

    /// Take ownership of close prices, trying multiple common column names.
    pub fn take_close(&mut self) -> Option<Vec<f64>> {
        self.take_column("close")
            .or_else(|| self.take_column("price"))
            .or_else(|| self.take_column("adj close"))
            .or_else(|| self.take_column("adjusted close"))
    }
}

/// Normalize a column header name for matching.
fn normalize_header(header: &str) -> String {
    header.trim().to_lowercase()
}

/// Check if a header represents a date column.
fn is_date_column(header: &str) -> bool {
    let normalized = normalize_header(header);
    matches!(
        normalized.as_str(),
        "date" | "time" | "datetime" | "timestamp" | "dt"
    )
}

/// Parse a string value to f64, treating empty as NaN.
fn parse_value(value: &str) -> Result<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(f64::NAN)
    } else {
        trimmed.parse::<f64>().map_err(|_| CliError::CsvParseError {
            message: format!("cannot parse '{trimmed}' as number"),
            line: None,
        })
    }
}

/// Parse a CSV file into a structured format.
///
/// # Arguments
///
/// * `path` - Path to the CSV file
///
/// # Returns
///
/// A `ParsedCsv` structure containing the parsed data and column mapping.
///
/// # Errors
///
/// Returns `CliError::IoError` if the file cannot be read, or
/// `CliError::CsvParseError` if the CSV is malformed.
pub fn parse_csv<P: AsRef<Path>>(path: P) -> Result<ParsedCsv> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|e| CliError::IoError {
        source: e,
        path: Some(path.display().to_string()),
    })?;
    let reader = BufReader::new(file);
    parse_csv_from_reader(reader)
}

/// Parse CSV data from a reader.
///
/// This is useful for testing or parsing from non-file sources.
pub fn parse_csv_from_reader<R: Read>(reader: R) -> Result<ParsedCsv> {
    let mut csv_reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(reader);

    // Get headers
    let headers: Vec<String> = csv_reader
        .headers()
        .map_err(|e| CliError::CsvParseError {
            message: e.to_string(),
            line: Some(1),
        })?
        .iter()
        .map(String::from)
        .collect();

    if headers.is_empty() {
        return Err(CliError::CsvParseError {
            message: "CSV file has no headers".to_string(),
            line: Some(1),
        });
    }

    // Build column map with pre-allocated capacity
    let mut column_map = HashMap::with_capacity(headers.len());
    let mut date_column_idx: Option<usize> = None;

    for (idx, header) in headers.iter().enumerate() {
        let normalized = normalize_header(header);
        if is_date_column(header) {
            date_column_idx = Some(idx);
        } else {
            column_map.insert(normalized, idx);
        }
    }

    // Initialize column storage with pre-allocated capacity
    let mut columns: HashMap<usize, Vec<f64>> = HashMap::with_capacity(column_map.len());
    for &idx in column_map.values() {
        columns.insert(idx, Vec::new());
    }
    let mut dates: Vec<String> = Vec::new();

    // Parse rows
    let mut row_count = 0;
    for (line_idx, result) in csv_reader.records().enumerate() {
        let record = result.map_err(|e| CliError::CsvParseError {
            message: e.to_string(),
            line: Some(line_idx + 2), // +2 for header and 0-indexing
        })?;

        // Extract date if present
        if let Some(date_idx) = date_column_idx {
            if let Some(date_value) = record.get(date_idx) {
                dates.push(date_value.to_string());
            } else {
                dates.push(String::new());
            }
        }

        // Extract numeric columns
        for (&col_idx, values) in &mut columns {
            let value = record.get(col_idx).unwrap_or("");
            let parsed = parse_value(value).map_err(|e| {
                if let CliError::CsvParseError { message, .. } = e {
                    CliError::CsvParseError {
                        message,
                        line: Some(line_idx + 2),
                    }
                } else {
                    e
                }
            })?;
            values.push(parsed);
        }

        row_count += 1;
    }

    Ok(ParsedCsv {
        headers,
        column_map,
        dates: if dates.is_empty() { None } else { Some(dates) },
        columns,
        row_count,
    })
}

/// Parse a CSV file into close prices only.
///
/// This is a convenience function for indicators that only need close prices.
pub fn parse_close_prices<P: AsRef<Path>>(path: P) -> Result<Vec<f64>> {
    let mut parsed = parse_csv(path)?;
    parsed
        .take_close()
        .ok_or_else(|| CliError::CsvParseError {
            message: "no close price column found (expected 'close', 'price', or 'adj close')"
                .to_string(),
            line: None,
        })
}

/// Parse a CSV file into OHLC data.
pub fn parse_ohlc<P: AsRef<Path>>(path: P) -> Result<OhlcData> {
    let mut parsed = parse_csv(path)?;

    let open = parsed
        .take_column("open")
        .ok_or_else(|| CliError::CsvParseError {
            message: "no 'open' column found".to_string(),
            line: None,
        })?;

    let high = parsed
        .take_column("high")
        .ok_or_else(|| CliError::CsvParseError {
            message: "no 'high' column found".to_string(),
            line: None,
        })?;

    let low = parsed
        .take_column("low")
        .ok_or_else(|| CliError::CsvParseError {
            message: "no 'low' column found".to_string(),
            line: None,
        })?;

    let close = parsed
        .take_close()
        .ok_or_else(|| CliError::CsvParseError {
            message: "no close price column found".to_string(),
            line: None,
        })?;

    Ok(OhlcData {
        dates: parsed.dates.take(),
        open,
        high,
        low,
        close,
    })
}

/// Parse a CSV file into OHLCV data.
pub fn parse_ohlcv<P: AsRef<Path>>(path: P) -> Result<OhlcvData> {
    let mut parsed = parse_csv(path)?;

    let open = parsed
        .take_column("open")
        .ok_or_else(|| CliError::CsvParseError {
            message: "no 'open' column found".to_string(),
            line: None,
        })?;

    let high = parsed
        .take_column("high")
        .ok_or_else(|| CliError::CsvParseError {
            message: "no 'high' column found".to_string(),
            line: None,
        })?;

    let low = parsed
        .take_column("low")
        .ok_or_else(|| CliError::CsvParseError {
            message: "no 'low' column found".to_string(),
            line: None,
        })?;

    let close = parsed
        .take_close()
        .ok_or_else(|| CliError::CsvParseError {
            message: "no close price column found".to_string(),
            line: None,
        })?;

    let volume = parsed
        .take_column("volume")
        .or_else(|| parsed.take_column("vol"))
        .ok_or_else(|| CliError::CsvParseError {
            message: "no 'volume' column found".to_string(),
            line: None,
        })?;

    Ok(OhlcvData {
        dates: parsed.dates.take(),
        open,
        high,
        low,
        close,
        volume,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // ==========================================================================
    // Task 3.1 Test Cases
    // ==========================================================================

    #[test]
    fn test_parse_simple_csv_with_close_prices() {
        let csv_data = "close\n44.0\n44.5\n43.5\n44.5\n44.0\n";
        let cursor = Cursor::new(csv_data);
        let parsed = parse_csv_from_reader(cursor).unwrap();

        let close = parsed.get_close().unwrap();
        assert_eq!(close.len(), 5);
        assert!((close[0] - 44.0).abs() < 1e-10);
        assert!((close[1] - 44.5).abs() < 1e-10);
        assert!((close[2] - 43.5).abs() < 1e-10);
    }

    #[test]
    fn test_parse_ohlc_csv() {
        let csv_data = "date,open,high,low,close\n\
                        2024-01-01,44.0,45.0,43.5,44.5\n\
                        2024-01-02,44.5,45.5,44.0,45.0\n";
        let cursor = Cursor::new(csv_data);
        let parsed = parse_csv_from_reader(cursor).unwrap();

        assert!(parsed.get_column("open").is_some());
        assert!(parsed.get_column("high").is_some());
        assert!(parsed.get_column("low").is_some());
        assert!(parsed.get_close().is_some());
        assert!(parsed.dates.is_some());

        let dates = parsed.dates.as_ref().unwrap();
        assert_eq!(dates[0], "2024-01-01");
        assert_eq!(dates[1], "2024-01-02");

        let close = parsed.get_close().unwrap();
        assert!((close[0] - 44.5).abs() < 1e-10);
        assert!((close[1] - 45.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_ohlcv_csv() {
        let csv_data = "date,open,high,low,close,volume\n\
                        2024-01-01,44.0,45.0,43.5,44.5,1000000\n\
                        2024-01-02,44.5,45.5,44.0,45.0,1100000\n";
        let cursor = Cursor::new(csv_data);
        let parsed = parse_csv_from_reader(cursor).unwrap();

        let volume = parsed.get_column("volume").unwrap();
        assert_eq!(volume.len(), 2);
        assert!((volume[0] - 1000000.0).abs() < 1e-10);
        assert!((volume[1] - 1100000.0).abs() < 1e-10);
    }

    #[test]
    fn test_handle_header_row() {
        let csv_data = "Close,HIGH,low,OPEN\n44.0,45.0,43.0,44.5\n";
        let cursor = Cursor::new(csv_data);
        let parsed = parse_csv_from_reader(cursor).unwrap();

        // Headers should be case-insensitive
        assert!(parsed.get_close().is_some());
        assert!(parsed.get_column("high").is_some());
        assert!(parsed.get_column("low").is_some());
        assert!(parsed.get_column("open").is_some());
    }

    #[test]
    fn test_handle_missing_values_as_nan() {
        let csv_data = "date,close\n2024-01-01,44.0\n2024-01-02,\n2024-01-03,45.0\n";
        let cursor = Cursor::new(csv_data);
        let parsed = parse_csv_from_reader(cursor).unwrap();

        let close = parsed.get_close().unwrap();
        assert_eq!(close.len(), 3);
        assert!((close[0] - 44.0).abs() < 1e-10);
        assert!(close[1].is_nan()); // Empty cell becomes NaN
        assert!((close[2] - 45.0).abs() < 1e-10);
    }

    #[test]
    fn test_handle_malformed_rows_error() {
        let csv_data = "date,close\n2024-01-01,44.0\n2024-01-02,not_a_number\n";
        let cursor = Cursor::new(csv_data);
        let result = parse_csv_from_reader(cursor);

        assert!(result.is_err());
        if let Err(CliError::CsvParseError { message, line }) = result {
            assert!(message.contains("not_a_number"));
            assert_eq!(line, Some(3)); // Line 3 (header=1, data row 1=2, data row 2=3)
        } else {
            panic!("Expected CsvParseError");
        }
    }

    #[test]
    fn test_error_file_not_found() {
        let result = parse_csv("/nonexistent/path/to/file.csv");

        assert!(result.is_err());
        if let Err(CliError::IoError { path, .. }) = result {
            assert!(path.is_some());
            assert!(path.unwrap().contains("nonexistent"));
        } else {
            panic!("Expected IoError");
        }
    }

    #[test]
    fn test_error_invalid_numeric_values() {
        let csv_data = "close\n44.0\nabc\n45.0\n";
        let cursor = Cursor::new(csv_data);
        let result = parse_csv_from_reader(cursor);

        assert!(result.is_err());
        if let Err(CliError::CsvParseError { message, .. }) = result {
            assert!(message.contains("abc"));
        } else {
            panic!("Expected CsvParseError");
        }
    }

    #[test]
    fn test_parse_close_convenience_function() {
        let csv_data = "close\n44.0\n44.5\n43.5\n";

        // Use a temporary file for this test
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_close_prices.csv");
        std::fs::write(&temp_path, csv_data).unwrap();

        let close = parse_close_prices(&temp_path).unwrap();
        assert_eq!(close.len(), 3);
        assert!((close[0] - 44.0).abs() < 1e-10);

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_alternative_close_column_names() {
        // Test 'price' column
        let csv_data = "price\n44.0\n44.5\n";
        let cursor = Cursor::new(csv_data);
        let parsed = parse_csv_from_reader(cursor).unwrap();
        assert!(parsed.get_close().is_some());

        // Test 'adj close' column
        let csv_data = "adj close\n44.0\n44.5\n";
        let cursor = Cursor::new(csv_data);
        let parsed = parse_csv_from_reader(cursor).unwrap();
        assert!(parsed.get_close().is_some());
    }

    #[test]
    fn test_parse_ohlc_struct() {
        let csv_data = "date,open,high,low,close\n\
                        2024-01-01,44.0,45.0,43.5,44.5\n\
                        2024-01-02,44.5,45.5,44.0,45.0\n";

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_ohlc.csv");
        std::fs::write(&temp_path, csv_data).unwrap();

        let ohlc = parse_ohlc(&temp_path).unwrap();
        assert_eq!(ohlc.open.len(), 2);
        assert_eq!(ohlc.high.len(), 2);
        assert_eq!(ohlc.low.len(), 2);
        assert_eq!(ohlc.close.len(), 2);
        assert!(ohlc.dates.is_some());

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_parse_ohlcv_struct() {
        let csv_data = "date,open,high,low,close,volume\n\
                        2024-01-01,44.0,45.0,43.5,44.5,1000000\n";

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_ohlcv.csv");
        std::fs::write(&temp_path, csv_data).unwrap();

        let ohlcv = parse_ohlcv(&temp_path).unwrap();
        assert_eq!(ohlcv.volume.len(), 1);
        assert!((ohlcv.volume[0] - 1000000.0).abs() < 1e-10);

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_error_missing_close_column() {
        let csv_data = "open,high,low\n44.0,45.0,43.5\n";

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_no_close.csv");
        std::fs::write(&temp_path, csv_data).unwrap();

        let result = parse_close_prices(&temp_path);
        assert!(result.is_err());

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_row_count() {
        let csv_data = "close\n44.0\n44.5\n43.5\n44.5\n44.0\n";
        let cursor = Cursor::new(csv_data);
        let parsed = parse_csv_from_reader(cursor).unwrap();
        assert_eq!(parsed.row_count, 5);
    }

    #[test]
    fn test_empty_csv_no_data_rows() {
        let csv_data = "close\n";
        let cursor = Cursor::new(csv_data);
        let parsed = parse_csv_from_reader(cursor).unwrap();
        assert_eq!(parsed.row_count, 0);
        let close = parsed.get_close().unwrap();
        assert!(close.is_empty());
    }

    #[test]
    fn test_whitespace_in_values() {
        let csv_data = "close\n  44.0  \n 44.5\n43.5 \n";
        let cursor = Cursor::new(csv_data);
        let parsed = parse_csv_from_reader(cursor).unwrap();
        let close = parsed.get_close().unwrap();
        assert!((close[0] - 44.0).abs() < 1e-10);
        assert!((close[1] - 44.5).abs() < 1e-10);
        assert!((close[2] - 43.5).abs() < 1e-10);
    }

    #[test]
    fn test_various_date_column_names() {
        for date_name in &["date", "Date", "DATE", "time", "datetime", "timestamp"] {
            let csv_data = format!("{date_name},close\n2024-01-01,44.0\n");
            let cursor = Cursor::new(csv_data);
            let parsed = parse_csv_from_reader(cursor).unwrap();
            assert!(
                parsed.dates.is_some(),
                "Failed to detect date column: {date_name}"
            );
        }
    }
}
