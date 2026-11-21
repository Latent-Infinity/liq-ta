//! Integration tests for the liq-ta CLI.
//!
//! These tests verify end-to-end functionality from CSV input through
//! indicator computation to CSV output.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Get the path to the test fixtures directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Run the CLI with given arguments and return the output.
fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_liq-ta"))
        .args(args)
        .output()
        .expect("Failed to execute CLI")
}

/// Run the CLI and capture stdout as string.
fn run_cli_stdout(args: &[&str]) -> String {
    let output = run_cli(args);
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn chop_rejects_undefined_periods() {
    let input = fixtures_dir().join("ohlc.csv");
    for period in ["0", "1"] {
        let output = Command::new(env!("CARGO_BIN_EXE_liq-ta"))
            .args(["chop", input.to_str().unwrap(), period])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("period must be at least 2"));
    }
}

#[test]
fn test_close_aliases_obv_and_nonfinite_bollinger() {
    let input =
        std::env::temp_dir().join(format!("liq-ta-cli-regression-{}.csv", std::process::id()));
    let path = input.to_str().unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_liq-ta"))
            .args(args)
            .output()
            .unwrap()
    };

    for (alias, volume) in [
        ("close", "volume"),
        ("adj_close", "vol"),
        ("adjusted_close", "volume"),
    ] {
        fs::write(
            &input,
            format!(
                "date,{alias},{volume}\n2026-01-01,10,100\n2026-01-02,11,50\n2026-01-03,9,20\n"
            ),
        )
        .unwrap();
        let sma = run(&["sma", path, "2"]);
        assert!(
            sma.status.success(),
            "{}",
            String::from_utf8_lossy(&sma.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&sma.stdout),
            "date,sma_2\n2026-01-02,10.5\n2026-01-03,10\n"
        );

        let obv = run(&["obv", path]);
        assert!(
            obv.status.success(),
            "{}",
            String::from_utf8_lossy(&obv.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&obv.stdout),
            "date,obv\n2026-01-01,100\n2026-01-02,150\n2026-01-03,130\n"
        );
    }

    for deviation in ["NaN", "inf", "-inf"] {
        let output = run(&["bollinger", path, &format!("2,{deviation}")]);
        assert_eq!(output.status.code(), Some(1));
    }

    fs::write(&input, "date,close\n2026-01-01,10\n").unwrap();
    let missing_volume = run(&["obv", path]);
    assert_eq!(missing_volume.status.code(), Some(2));
    fs::write(&input, "date,close,volume\n2026-01-01,bad,10\n").unwrap();
    let malformed = run(&["obv", path]);
    assert_eq!(malformed.status.code(), Some(2));
    fs::write(&input, "date,volume\n2026-01-01,10\n").unwrap();
    assert_eq!(run(&["obv", path]).status.code(), Some(2));
    fs::write(
        &input,
        format!("close,volume\n{},10\n", "1".repeat(16 * 1024 + 1)),
    )
    .unwrap();
    assert_eq!(run(&["obv", path]).status.code(), Some(2));
    fs::remove_file(input).unwrap();
}

// =============================================================================
// Task 3.7 Test Cases
// =============================================================================

#[test]
fn test_end_to_end_csv_to_sma_to_csv_output() {
    let input = fixtures_dir().join("simple_close.csv");
    let temp_dir = std::env::temp_dir();
    let output = temp_dir.join(format!("liq-ta-sma-output-{}.csv", std::process::id()));

    // New argument order: input first, then period
    let output_result = run_cli(&[
        "sma",
        input.to_str().unwrap(),
        "3",
        "-o",
        output.to_str().unwrap(),
    ]);

    assert!(output_result.status.success(), "CLI should succeed");
    assert!(output.exists(), "Output file should be created");

    let content = fs::read_to_string(&output).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Header + data rows (minus lookback of 2)
    assert!(
        lines.len() >= 2,
        "Should have header and at least one data row"
    );
    assert!(
        lines[0].contains("sma"),
        "Header should contain indicator name"
    );

    fs::remove_file(&output).ok();
}

#[test]
fn test_end_to_end_csv_to_macd_to_csv_output() {
    let input = fixtures_dir().join("simple_close.csv");
    let temp_dir = std::env::temp_dir();
    let output = temp_dir.join(format!("liq-ta-macd-output-{}.csv", std::process::id()));

    // New argument order: input first, then params
    let output_result = run_cli(&[
        "macd",
        input.to_str().unwrap(),
        "2,3,2",
        "-o",
        output.to_str().unwrap(),
    ]);

    assert!(output_result.status.success());
    let content = fs::read_to_string(&output).unwrap();
    let header = content.lines().next().unwrap();
    assert!(header.contains("macd") && header.contains("signal") && header.contains("histogram"));

    fs::remove_file(&output).ok();
}

#[test]
fn test_exit_code_success() {
    let input = fixtures_dir().join("simple_close.csv");

    let output = run_cli(&["sma", input.to_str().unwrap(), "3"]);

    assert!(output.status.success(), "Exit code should be 0 for success");
}

#[test]
fn test_exit_code_missing_file() {
    let output = run_cli(&["sma", "/nonexistent/file.csv", "3"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("[io_error]"));
}

#[test]
fn invalid_macd_parameters_exit_with_argument_error() {
    let input = fixtures_dir().join("simple_close.csv");
    let output = run_cli(&["macd", input.to_str().unwrap(), "12,26"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("[invalid_argument]"));
}

#[test]
fn test_output_format_csv_valid() {
    let input = fixtures_dir().join("simple_close.csv");

    let stdout = run_cli_stdout(&["sma", input.to_str().unwrap(), "2"]);

    // Output should be valid CSV
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines.is_empty(), "Should produce output");

    // First line should be header
    let header = lines[0];
    assert!(!header.is_empty(), "Header should not be empty");
}

#[test]
fn test_lookback_rows_dropped_per_prd() {
    // PRD §5.4: Initial lookback rows should be dropped
    let input = fixtures_dir().join("simple_close.csv");

    let stdout = run_cli_stdout(&["sma", input.to_str().unwrap(), "3"]);

    let lines: Vec<&str> = stdout.lines().collect();

    // The fixture has 10 data points. With period 3, lookback is 2.
    // We expect header + 8 data rows = 9 lines total.
    // This verifies lookback rows are dropped (not 11 lines with all NaN rows).
    assert_eq!(
        lines.len(),
        9,
        "Expected 9 lines (header + 8 data rows after dropping 2 lookback)"
    );
    assert!(lines[0].contains("sma"), "First line should be header");
}

#[test]
fn test_date_column_preserved() {
    let input = fixtures_dir().join("ohlc.csv");
    let temp_dir = std::env::temp_dir();
    let output = temp_dir.join(format!("liq-ta-date-output-{}.csv", std::process::id()));

    let output_result = run_cli(&[
        "sma",
        input.to_str().unwrap(),
        "2",
        "-o",
        output.to_str().unwrap(),
    ]);

    assert!(output_result.status.success());
    let content = fs::read_to_string(&output).unwrap();
    assert!(content.lines().next().unwrap().contains("date"));

    fs::remove_file(&output).ok();
}

#[test]
fn test_help_flag() {
    let output = run_cli(&["--help"]);

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Help should mention available commands
    assert!(
        stdout.contains("sma") || stdout.contains("SMA") || stdout.contains("Usage"),
        "Help should mention available indicators"
    );
}

#[test]
fn test_version_flag() {
    let output = run_cli(&["--version"]);

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Version should contain version number
    assert!(
        stdout.contains("liq-ta") || stdout.contains('.'),
        "Version should be displayed"
    );
}

#[test]
fn test_ema_end_to_end() {
    let input = fixtures_dir().join("simple_close.csv");

    let stdout = run_cli_stdout(&["ema", input.to_str().unwrap(), "3"]);

    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines.is_empty(), "EMA should produce output");
    assert!(lines[0].contains("ema"), "Header should contain 'ema'");
}

#[test]
fn test_hma_end_to_end() {
    let input = fixtures_dir().join("simple_close.csv");
    let stdout = run_cli_stdout(&["hma", input.to_str().unwrap(), "3"]);
    let lines: Vec<&str> = stdout.lines().collect();

    // 10 input rows, hma_lookback(3)=3 => 7 data rows + header
    assert_eq!(lines.len(), 8);
    assert!(lines[0].contains("hma_3"));
}

#[test]
fn test_supertrend_end_to_end() {
    let input = fixtures_dir().join("ohlc.csv");
    let stdout = run_cli_stdout(&["supertrend", input.to_str().unwrap(), "3,2.0"]);
    let lines: Vec<&str> = stdout.lines().collect();

    // 10 input rows, supertrend_lookback(3)=3 => 7 data rows + header
    assert_eq!(lines.len(), 8);
    assert!(lines[0].contains("supertrend"));
    assert!(lines[0].contains("supertrend_upper"));
    assert!(lines[0].contains("supertrend_lower"));
    assert!(lines[0].contains("supertrend_trend"));
}

#[test]
fn test_gaussian_channel_end_to_end() {
    let input = fixtures_dir().join("simple_close.csv");
    let stdout = run_cli_stdout(&["gaussian-channel", input.to_str().unwrap(), "3,0.5,2.0"]);
    let lines: Vec<&str> = stdout.lines().collect();

    // 10 input rows, gaussian_lookback(3)=2 => 8 data rows + header
    assert_eq!(lines.len(), 9);
    assert!(lines[0].contains("gaussian_center"));
    assert!(lines[0].contains("gaussian_upper"));
    assert!(lines[0].contains("gaussian_lower"));
    assert!(lines[0].contains("gaussian_trend"));
}

#[test]
fn test_stage3_new_commands_smoke() {
    let simple = fixtures_dir().join("simple_close.csv");
    let ohlc = fixtures_dir().join("ohlc.csv");

    let hma = run_cli(&["hma", simple.to_str().unwrap(), "3"]);
    assert!(hma.status.success());

    let ao = run_cli(&["ao", ohlc.to_str().unwrap()]);
    assert!(!ao.status.success());

    let osma = run_cli(&["osma", simple.to_str().unwrap(), "2,3,2"]);
    assert!(osma.status.success());

    let chop = run_cli(&["chop", ohlc.to_str().unwrap(), "3"]);
    assert!(chop.status.success());

    let hurst = run_cli(&["hurst", simple.to_str().unwrap(), "5"]);
    assert!(hurst.status.success());
}

#[test]
fn test_supertrend_invalid_args() {
    let input = fixtures_dir().join("ohlc.csv");
    let output = run_cli(&["supertrend", input.to_str().unwrap(), "3,-1.0"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn test_gaussian_channel_invalid_args() {
    let input = fixtures_dir().join("simple_close.csv");
    let output = run_cli(&["gaussian-channel", input.to_str().unwrap(), "3,0.5"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}
