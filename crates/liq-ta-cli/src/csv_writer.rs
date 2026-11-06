//! CSV output module for writing indicator results.
//!
//! This module handles writing indicator outputs to CSV format, following the
//! PRD §5.4 NaN row semantics:
//!
//! - Initial lookback rows are dropped (not written to output)
//! - Internal NaN values are preserved as empty cells
//! - Date column is aligned correctly after lookback offset
//!
//! # Output Format
//!
//! Single-output indicators produce a single column. Multi-output indicators
//! (MACD, Bollinger, Stochastic) produce multiple named columns.

use crate::error::{CliError, Result};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// Output destination: either stdout or a file.
pub enum OutputDest {
    /// Write to stdout.
    Stdout,
    /// Write to a file at the given path.
    File(String),
}

impl OutputDest {
    /// Create a writer for this output destination.
    pub fn writer(&self) -> Result<Box<dyn Write>> {
        match self {
            OutputDest::Stdout => Ok(Box::new(BufWriter::new(io::stdout()))),
            OutputDest::File(path) => {
                let file = File::create(path).map_err(|e| CliError::IoError {
                    source: e,
                    path: Some(path.clone()),
                })?;
                Ok(Box::new(BufWriter::new(file)))
            }
        }
    }
}

fn write_csv_field(writer: &mut dyn Write, field: &str) -> io::Result<()> {
    if field.contains([',', '"', '\r', '\n']) {
        write!(writer, "\"{}\"", field.replace('"', "\"\""))
    } else {
        writer.write_all(field.as_bytes())
    }
}

/// Write single-column indicator output to CSV.
///
/// # Arguments
///
/// * `output` - The indicator output values
/// * `header` - Column header name (e.g., "`sma_20`")
/// * `dates` - Optional date column to include
/// * `lookback` - Number of initial NaN values to skip
/// * `dest` - Output destination (stdout or file)
///
/// # NaN Handling (PRD §5.4)
///
/// - First `lookback` rows are dropped (these contain initial NaN values)
/// - Any remaining NaN values in the output are written as empty cells
pub fn write_single_output(
    output: &[f64],
    header: &str,
    dates: Option<&[String]>,
    lookback: usize,
    dest: &OutputDest,
) -> Result<()> {
    let mut writer = dest.writer()?;

    // Write header
    if dates.is_some() {
        write!(writer, "date,")?;
    }
    write_csv_field(&mut writer, header)?;
    writeln!(writer)?;

    // Write data rows, skipping lookback period
    for i in lookback..output.len() {
        if let Some(dates) = dates {
            if i < dates.len() {
                write_csv_field(&mut writer, &dates[i])?;
                write!(writer, ",")?;
            } else {
                write!(writer, ",")?;
            }
        }

        if output[i].is_nan() {
            writeln!(writer)?; // Empty cell for NaN
        } else {
            writeln!(writer, "{}", output[i])?;
        }
    }

    writer.flush()?;
    Ok(())
}

/// Write multi-column indicator output to CSV.
///
/// # Arguments
///
/// * `columns` - Vector of (`header_name`, values) pairs
/// * `dates` - Optional date column to include
/// * `lookback` - Number of initial NaN values to skip (uses max lookback)
/// * `dest` - Output destination
///
/// # NaN Handling (PRD §5.4)
///
/// All columns share the same lookback offset. Any internal NaN values
/// are written as empty cells.
pub fn write_multi_output(
    columns: &[(&str, &[f64])],
    dates: Option<&[String]>,
    lookback: usize,
    dest: &OutputDest,
) -> Result<()> {
    if columns.is_empty() {
        return Ok(());
    }

    let mut writer = dest.writer()?;

    // Write header
    let has_dates = dates.is_some();
    if has_dates {
        write!(writer, "date")?;
    }
    for (idx, (name, _)) in columns.iter().enumerate() {
        if has_dates || idx > 0 {
            write!(writer, ",")?;
        }
        write_csv_field(&mut writer, name)?;
    }
    writeln!(writer)?;

    // Get length from first column
    let len = columns[0].1.len();

    // Write data rows, skipping lookback period
    for i in lookback..len {
        if let Some(dates) = dates
            && i < dates.len()
        {
            write_csv_field(&mut writer, &dates[i])?;
        }

        for (idx, (_, values)) in columns.iter().enumerate() {
            let prefix = if dates.is_some() || idx > 0 { "," } else { "" };
            if i < values.len() && !values[i].is_nan() {
                write!(writer, "{}{}", prefix, values[i])?;
            } else {
                write!(writer, "{prefix}")?; // Empty cell for NaN or out-of-bounds
            }
        }
        writeln!(writer)?;
    }

    writer.flush()?;
    Ok(())
}

/// Write to a file path.
pub fn write_to_file<P: AsRef<Path>>(
    output: &[f64],
    header: &str,
    dates: Option<&[String]>,
    lookback: usize,
    path: P,
) -> Result<()> {
    let dest = OutputDest::File(path.as_ref().display().to_string());
    write_single_output(output, header, dates, lookback, &dest)
}

/// Write to stdout.
pub fn write_to_stdout(
    output: &[f64],
    header: &str,
    dates: Option<&[String]>,
    lookback: usize,
) -> Result<()> {
    write_single_output(output, header, dates, lookback, &OutputDest::Stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_output_preserves_date_with_csv_special_characters() {
        let path =
            std::env::temp_dir().join(format!("liq-ta-single-date-{}.csv", std::process::id()));
        let dates = vec!["quoted \"date\",\ncontinued".to_string()];

        write_to_file(&[1.0], "value,quoted", Some(&dates), 0, &path).unwrap();

        let mut reader = csv::Reader::from_path(&path).unwrap();
        assert_eq!(reader.headers().unwrap().get(1), Some("value,quoted"));
        let record = reader.records().next().unwrap().unwrap();
        assert_eq!(record.get(0), Some(dates[0].as_str()));
        assert_eq!(record.get(1), Some("1"));
        assert_eq!(record.len(), 2);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn multi_output_preserves_date_with_csv_special_characters() {
        let path =
            std::env::temp_dir().join(format!("liq-ta-multi-date-{}.csv", std::process::id()));
        let dates = vec!["quoted \"date\",\ncontinued".to_string()];
        let columns: &[(&str, &[f64])] = &[("first,quoted", &[1.0]), ("second", &[2.0])];

        write_multi_output(
            columns,
            Some(&dates),
            0,
            &OutputDest::File(path.display().to_string()),
        )
        .unwrap();

        let mut reader = csv::Reader::from_path(&path).unwrap();
        assert_eq!(reader.headers().unwrap().get(1), Some("first,quoted"));
        let record = reader.records().next().unwrap().unwrap();
        assert_eq!(record.get(0), Some(dates[0].as_str()));
        assert_eq!(record.get(1), Some("1"));
        assert_eq!(record.get(2), Some("2"));
        assert_eq!(record.len(), 3);
        std::fs::remove_file(path).unwrap();
    }

    // ==========================================================================
    // Task 3.3 Test Cases
    // ==========================================================================

    #[test]
    fn test_write_single_indicator_output() {
        let output = vec![f64::NAN, f64::NAN, 44.0, 44.5, 44.0];
        let lookback = 2;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_single_output.csv");

        write_to_file(&output, "sma_3", None, lookback, &temp_path).unwrap();

        let content = std::fs::read_to_string(&temp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines[0], "sma_3");
        assert_eq!(lines[1], "44");
        assert_eq!(lines[2], "44.5");
        assert_eq!(lines[3], "44");

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_write_multi_output_indicator() {
        let macd = vec![f64::NAN, f64::NAN, 1.0, 1.5, 2.0];
        let signal = vec![f64::NAN, f64::NAN, 0.5, 0.8, 1.0];
        let histogram = vec![f64::NAN, f64::NAN, 0.5, 0.7, 1.0];
        let lookback = 2;

        let columns: Vec<(&str, &[f64])> = vec![
            ("macd", &macd),
            ("signal", &signal),
            ("histogram", &histogram),
        ];

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_multi_output.csv");

        write_multi_output(
            &columns,
            None,
            lookback,
            &OutputDest::File(temp_path.display().to_string()),
        )
        .unwrap();

        let content = std::fs::read_to_string(&temp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines[0], "macd,signal,histogram");
        assert!(lines[1].contains('1')); // First data row after lookback
        assert_eq!(lines.len(), 4); // Header + 3 data rows

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_write_with_header_row() {
        let output = vec![44.0, 44.5, 44.0];

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_with_header.csv");

        write_to_file(&output, "close_sma", None, 0, &temp_path).unwrap();

        let content = std::fs::read_to_string(&temp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines[0], "close_sma");

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_write_to_file() {
        let output = vec![44.0, 44.5, 44.0];

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_file_output.csv");

        write_to_file(&output, "test", None, 0, &temp_path).unwrap();

        assert!(temp_path.exists());
        let content = std::fs::read_to_string(&temp_path).unwrap();
        assert!(!content.is_empty());

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_nan_row_semantics_lookback_dropped() {
        // PRD §5.4: Initial lookback rows dropped
        let output = vec![f64::NAN, f64::NAN, f64::NAN, 44.0, 44.5];
        let lookback = 3;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_lookback_dropped.csv");

        write_to_file(&output, "sma", None, lookback, &temp_path).unwrap();

        let content = std::fs::read_to_string(&temp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        // Should have header + 2 data rows (indices 3 and 4)
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "sma");

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_nan_row_semantics_internal_nan_preserved() {
        // PRD §5.4: Internal NaN values preserved as empty cells
        let output = vec![44.0, f64::NAN, 44.5]; // Internal NaN at index 1
        let lookback = 0;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_internal_nan.csv");

        write_to_file(&output, "test", None, lookback, &temp_path).unwrap();

        let content = std::fs::read_to_string(&temp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines[0], "test");
        assert_eq!(lines[1], "44");
        assert_eq!(lines[2], ""); // Internal NaN → empty cell
        assert_eq!(lines[3], "44.5");

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_nan_row_semantics_date_aligned() {
        // PRD §5.4: Date column aligned correctly after lookback offset
        let output = vec![f64::NAN, f64::NAN, 44.0, 44.5, 45.0];
        let dates = vec![
            "2024-01-01".to_string(),
            "2024-01-02".to_string(),
            "2024-01-03".to_string(),
            "2024-01-04".to_string(),
            "2024-01-05".to_string(),
        ];
        let lookback = 2;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_date_aligned.csv");

        write_to_file(&output, "sma", Some(&dates), lookback, &temp_path).unwrap();

        let content = std::fs::read_to_string(&temp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines[0], "date,sma");
        assert!(lines[1].starts_with("2024-01-03")); // First date after lookback
        assert!(lines[2].starts_with("2024-01-04"));
        assert!(lines[3].starts_with("2024-01-05"));

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_multi_output_with_dates() {
        let col1 = vec![f64::NAN, 1.0, 2.0];
        let col2 = vec![f64::NAN, 0.5, 1.0];
        let dates = vec![
            "2024-01-01".to_string(),
            "2024-01-02".to_string(),
            "2024-01-03".to_string(),
        ];
        let lookback = 1;

        let columns: Vec<(&str, &[f64])> = vec![("col1", &col1), ("col2", &col2)];

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_multi_with_dates.csv");

        write_multi_output(
            &columns,
            Some(&dates),
            lookback,
            &OutputDest::File(temp_path.display().to_string()),
        )
        .unwrap();

        let content = std::fs::read_to_string(&temp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines[0], "date,col1,col2");
        assert!(lines[1].starts_with("2024-01-02"));

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_empty_output() {
        let output: Vec<f64> = vec![];

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_empty.csv");

        write_to_file(&output, "empty", None, 0, &temp_path).unwrap();

        let content = std::fs::read_to_string(&temp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 1); // Just header
        assert_eq!(lines[0], "empty");

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_output_dest_file_creation() {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_dest_file.csv");

        let dest = OutputDest::File(temp_path.display().to_string());
        let writer = dest.writer();

        assert!(writer.is_ok());

        // Clean up
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_full_precision_output() {
        let output = vec![44.123456789012345];

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_precision.csv");

        write_to_file(&output, "precise", None, 0, &temp_path).unwrap();

        let content = std::fs::read_to_string(&temp_path).unwrap();

        // Should preserve reasonable precision
        assert!(content.contains("44.123456789012"));

        std::fs::remove_file(&temp_path).ok();
    }
}
