//! liq-ta command-line interface
//!
//! This binary provides a command-line interface for computing technical
//! analysis indicators on CSV data files.
//!
//! # Usage
//!
//! ```bash
//! liq-ta <indicator> <input.csv> [params] [-o output.csv] [-c column]
//! ```
//!
//! # Examples
//!
//! ```bash
//! # SMA with default period (20)
//! liq-ta sma prices.csv
//!
//! # SMA with custom period
//! liq-ta sma prices.csv 14
//!
//! # MACD with output to file
//! liq-ta macd prices.csv 12,26,9 -o macd_output.csv
//!
//! # Stochastic oscillator
//! liq-ta stochastic ohlc.csv 14,3,3
//! ```
//!
//! # Exit Codes (per PRD §5.4)
//!
//! - 0: Success
//! - 1: Argument error (invalid parameters)
//! - 2: Data error (file not found, parse error)
//! - 3: Computation error (indicator failed)

use liq_ta_cli::args::{
    Args, Command, parse_bollinger_params, parse_gaussian_channel_params, parse_ichimoku_params,
    parse_keltner_params, parse_macd_params, parse_qqe_params, parse_stochastic_params,
    parse_supertrend_params,
};
use liq_ta_cli::csv_parser::{self, parse_csv, parse_ohlc, parse_ohlcv};
use liq_ta_cli::csv_writer::{OutputDest, write_multi_output, write_single_output};
use liq_ta_cli::{CliError, Result};
use tracing::error;

use std::error::Error as _;

use liq_ta::indicators::{
    adx, ao, atr, bollinger, chop, donchian, ema, gaussian_channel, hma, hurst, ichimoku,
    keltner_channel, macd, obv, osma, qqe, rsi, sma, stochastic, supertrend, vwap, williams_r,
};

/// Exit codes per PRD §5.4
mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const ARGUMENT_ERROR: i32 = 1;
    pub const DATA_ERROR: i32 = 2;
    pub const COMPUTATION_ERROR: i32 = 3;
}

fn exit_code_for_error(error: &CliError) -> i32 {
    match error {
        CliError::InvalidArgument { .. } => exit_codes::ARGUMENT_ERROR,
        CliError::IoError { .. } | CliError::CsvParseError { .. } => exit_codes::DATA_ERROR,
        CliError::IndicatorError { .. } => exit_codes::COMPUTATION_ERROR,
    }
}

fn main() {
    init_tracing();
    let args = Args::parse_args();
    let debug_errors = args.debug_errors_enabled() || debug_errors_from_env();
    let result = run(args);

    match result {
        Ok(()) => std::process::exit(exit_codes::SUCCESS),
        Err(e) => {
            error!(
                error_class = e.class_name(),
                debug_errors,
                error = %e,
                "liq-ta CLI execution failed"
            );
            for line in format_error_lines(&e, debug_errors) {
                eprintln!("{line}");
            }
            std::process::exit(exit_code_for_error(&e));
        }
    }
}

fn init_tracing() {
    let init_result = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .without_time()
        .try_init();
    if init_result.is_err() {
        // A global subscriber may already be initialized by tests or embedding hosts.
    }
}

/// Main entry point for CLI logic.
fn run(args: Args) -> Result<()> {
    run_with_args(args)
}

fn debug_errors_from_env() -> bool {
    match std::env::var("LIQ_TA_DEBUG_ERRORS") {
        Ok(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "" | "0" | "false" | "off" | "no"
        ),
        Err(_) => false,
    }
}

fn format_error_lines(error: &CliError, debug: bool) -> Vec<String> {
    let mut lines = vec![format!("error: [{}] {}", error.class_name(), error)];
    if debug {
        let mut idx = 1usize;
        let mut source = error.source();
        while let Some(cause) = source {
            lines.push(format!("  caused_by[{idx}]: {cause}"));
            source = cause.source();
            idx += 1;
        }
        lines.push(format!("  debug_repr: {error:?}"));
    }
    lines
}

fn run_with_args(args: Args) -> Result<()> {
    let output_dest = match args.output_path() {
        Some(path) => OutputDest::File(path.to_string()),
        None => OutputDest::Stdout,
    };

    match &args.command {
        Command::Sma {
            period,
            input,
            column,
            ..
        } => run_sma(input, *period, column.as_deref(), &output_dest),
        Command::Ema {
            period,
            input,
            column,
            ..
        } => run_ema(input, *period, column.as_deref(), &output_dest),
        Command::Rsi {
            period,
            input,
            column,
            ..
        } => run_rsi(input, *period, column.as_deref(), &output_dest),
        Command::Macd {
            params,
            input,
            column,
            ..
        } => run_macd(input, params, column.as_deref(), &output_dest),
        Command::Bollinger {
            params,
            input,
            column,
            ..
        } => run_bollinger(input, params, column.as_deref(), &output_dest),
        Command::Atr { period, input, .. } => run_atr(input, *period, &output_dest),
        Command::Stochastic { params, input, .. } => run_stochastic(input, params, &output_dest),
        Command::Adx { period, input, .. } => run_adx(input, *period, &output_dest),
        Command::WilliamsR { period, input, .. } => run_williams_r(input, *period, &output_dest),
        Command::Donchian { period, input, .. } => run_donchian(input, *period, &output_dest),
        Command::Keltner { params, input, .. } => run_keltner(input, params, &output_dest),
        Command::Ichimoku { params, input, .. } => run_ichimoku(input, params, &output_dest),
        Command::Qqe {
            params,
            input,
            column,
            ..
        } => run_qqe(input, params, column.as_deref(), &output_dest),
        Command::Hma {
            period,
            input,
            column,
            ..
        } => run_hma(input, *period, column.as_deref(), &output_dest),
        Command::Ao { input, .. } => run_ao(input, &output_dest),
        Command::Osma {
            params,
            input,
            column,
            ..
        } => run_osma(input, params, column.as_deref(), &output_dest),
        Command::Supertrend { params, input, .. } => run_supertrend(input, params, &output_dest),
        Command::Chop { period, input, .. } => run_chop(input, *period, &output_dest),
        Command::Hurst {
            period,
            input,
            column,
            ..
        } => run_hurst(input, *period, column.as_deref(), &output_dest),
        Command::GaussianChannel {
            params,
            input,
            column,
            ..
        } => run_gaussian_channel(input, params, column.as_deref(), &output_dest),
        Command::Obv { input, .. } => run_obv(input, &output_dest),
        Command::Vwap { input, .. } => run_vwap(input, &output_dest),
    }
}

/// Get close prices from parsed CSV, optionally using a specific column.
fn close_prices(parsed: &csv_parser::ParsedCsv, column: Option<&str>) -> Result<Vec<f64>> {
    if let Some(col_name) = column {
        let normalized = col_name.trim().to_lowercase();
        parsed
            .column(&normalized)
            .cloned()
            .ok_or_else(|| CliError::CsvParseError {
                message: format!("column '{col_name}' not found"),
                line: None,
            })
    } else {
        parsed
            .close()
            .cloned()
            .ok_or_else(|| CliError::CsvParseError {
                message: "no close price column found (expected 'close', 'price', or 'adj close')"
                    .to_string(),
                line: None,
            })
    }
}

/// Run SMA indicator.
fn run_sma(input: &str, period: usize, column: Option<&str>, dest: &OutputDest) -> Result<()> {
    let parsed = parse_csv(input)?;
    let close = close_prices(&parsed, column)?;

    let output = sma(&close, period)?;
    let lookback = period.saturating_sub(1);
    let header = format!("sma_{period}");

    write_single_output(&output, &header, parsed.dates.as_deref(), lookback, dest)
}

/// Run EMA indicator.
fn run_ema(input: &str, period: usize, column: Option<&str>, dest: &OutputDest) -> Result<()> {
    let parsed = parse_csv(input)?;
    let close = close_prices(&parsed, column)?;

    let output = ema(&close, period)?;
    let lookback = period.saturating_sub(1);
    let header = format!("ema_{period}");

    write_single_output(&output, &header, parsed.dates.as_deref(), lookback, dest)
}

/// Run RSI indicator.
fn run_rsi(input: &str, period: usize, column: Option<&str>, dest: &OutputDest) -> Result<()> {
    let parsed = parse_csv(input)?;
    let close = close_prices(&parsed, column)?;

    let output = rsi(&close, period)?;
    let lookback = period; // RSI has lookback equal to period
    let header = format!("rsi_{period}");

    write_single_output(&output, &header, parsed.dates.as_deref(), lookback, dest)
}

/// Run MACD indicator.
fn run_macd(input: &str, params: &str, column: Option<&str>, dest: &OutputDest) -> Result<()> {
    let (fast, slow, signal) = parse_macd_params(params)?;

    let parsed = parse_csv(input)?;
    let close = close_prices(&parsed, column)?;

    let result = macd(&close, fast, slow, signal)?;

    // MACD lookback is slow period + signal period - 2
    let lookback = slow + signal - 2;

    let columns: Vec<(&str, &[f64])> = vec![
        ("macd", &result.macd_line),
        ("signal", &result.signal_line),
        ("histogram", &result.histogram),
    ];

    write_multi_output(&columns, parsed.dates.as_deref(), lookback, dest)
}

/// Run Bollinger Bands indicator.
fn run_bollinger(input: &str, params: &str, column: Option<&str>, dest: &OutputDest) -> Result<()> {
    let (period, std_dev) = parse_bollinger_params(params)?;

    let parsed = parse_csv(input)?;
    let close = close_prices(&parsed, column)?;

    let result = bollinger(&close, period, std_dev)?;
    let lookback = period.saturating_sub(1);

    let columns: Vec<(&str, &[f64])> = vec![
        ("upper", &result.upper),
        ("middle", &result.middle),
        ("lower", &result.lower),
    ];

    write_multi_output(&columns, parsed.dates.as_deref(), lookback, dest)
}

/// Run ATR indicator.
fn run_atr(input: &str, period: usize, dest: &OutputDest) -> Result<()> {
    let ohlc = parse_ohlc(input)?;

    let output = atr(&ohlc.high, &ohlc.low, &ohlc.close, period)?;
    let lookback = period; // ATR has lookback equal to period
    let header = format!("atr_{period}");

    write_single_output(&output, &header, ohlc.dates.as_deref(), lookback, dest)
}

/// Run Stochastic indicator.
fn run_stochastic(input: &str, params: &str, dest: &OutputDest) -> Result<()> {
    let (k_period, d_period, k_slowing) = parse_stochastic_params(params)?;

    let ohlc = parse_ohlc(input)?;

    let result = stochastic(
        &ohlc.high,
        &ohlc.low,
        &ohlc.close,
        k_period,
        d_period,
        k_slowing,
    )?;

    // Stochastic lookback: k_period + k_slowing - 2 for %K, plus d_period - 1 for %D
    let lookback = k_period + k_slowing + d_period - 3;

    let columns: Vec<(&str, &[f64])> = vec![("percent_k", &result.k), ("percent_d", &result.d)];

    write_multi_output(&columns, ohlc.dates.as_deref(), lookback, dest)
}

/// Run ADX indicator.
fn run_adx(input: &str, period: usize, dest: &OutputDest) -> Result<()> {
    let ohlc = parse_ohlc(input)?;

    let result = adx(&ohlc.high, &ohlc.low, &ohlc.close, period)?;

    // ADX lookback = 2 * period - 1
    let lookback = 2 * period - 1;

    let columns: Vec<(&str, &[f64])> = vec![
        ("adx", &result.adx),
        ("plus_di", &result.plus_di),
        ("minus_di", &result.minus_di),
    ];

    write_multi_output(&columns, ohlc.dates.as_deref(), lookback, dest)
}

/// Run Williams %R indicator.
fn run_williams_r(input: &str, period: usize, dest: &OutputDest) -> Result<()> {
    let ohlc = parse_ohlc(input)?;

    let output = williams_r(&ohlc.high, &ohlc.low, &ohlc.close, period)?;
    let lookback = period.saturating_sub(1);
    let header = format!("williams_r_{period}");

    write_single_output(&output, &header, ohlc.dates.as_deref(), lookback, dest)
}

/// Run Donchian Channels indicator.
fn run_donchian(input: &str, period: usize, dest: &OutputDest) -> Result<()> {
    let ohlc = parse_ohlc(input)?;

    let result = donchian(&ohlc.high, &ohlc.low, period)?;
    let lookback = period.saturating_sub(1);

    let columns: Vec<(&str, &[f64])> = vec![
        ("donchian_upper", &result.upper),
        ("donchian_middle", &result.middle),
        ("donchian_lower", &result.lower),
    ];

    write_multi_output(&columns, ohlc.dates.as_deref(), lookback, dest)
}

/// Run Keltner Channel indicator.
fn run_keltner(input: &str, params: &str, dest: &OutputDest) -> Result<()> {
    let (period, atr_multiplier) = parse_keltner_params(params)?;
    let ohlc = parse_ohlc(input)?;

    let result = keltner_channel(&ohlc.high, &ohlc.low, &ohlc.close, period, atr_multiplier)?;
    let lookback = liq_ta::indicators::keltner_channel_lookback(period);

    let columns: Vec<(&str, &[f64])> = vec![
        ("keltner_upper", &result.upper),
        ("keltner_middle", &result.middle),
        ("keltner_lower", &result.lower),
    ];

    write_multi_output(&columns, ohlc.dates.as_deref(), lookback, dest)
}

/// Run Ichimoku indicator.
fn run_ichimoku(input: &str, params: &str, dest: &OutputDest) -> Result<()> {
    let (tenkan_period, kijun_period, senkou_b_period, displacement) =
        parse_ichimoku_params(params)?;
    let ohlc = parse_ohlc(input)?;

    let result = ichimoku(
        &ohlc.high,
        &ohlc.low,
        &ohlc.close,
        tenkan_period,
        kijun_period,
        senkou_b_period,
        displacement,
    )?;
    let lookback =
        liq_ta::indicators::ichimoku_lookback(tenkan_period, kijun_period, senkou_b_period);

    let columns: Vec<(&str, &[f64])> = vec![
        ("tenkan", &result.tenkan),
        ("kijun", &result.kijun),
        ("senkou_a", &result.senkou_a),
        ("senkou_b", &result.senkou_b),
        ("chikou", &result.chikou),
    ];

    write_multi_output(&columns, ohlc.dates.as_deref(), lookback, dest)
}

/// Run QQE indicator.
fn run_qqe(input: &str, params: &str, column: Option<&str>, dest: &OutputDest) -> Result<()> {
    let (rsi_period, smoothing, wilders, factor) = parse_qqe_params(params)?;
    let parsed = parse_csv(input)?;
    let close = close_prices(&parsed, column)?;

    let result = qqe(&close, rsi_period, smoothing, wilders, factor)?;
    let lookback = liq_ta::indicators::qqe_lookback(rsi_period, smoothing, wilders);
    let columns: Vec<(&str, &[f64])> = vec![
        ("qqe", &result.qqe),
        ("qqe_upper", &result.upper_band),
        ("qqe_lower", &result.lower_band),
    ];

    write_multi_output(&columns, parsed.dates.as_deref(), lookback, dest)
}

/// Run HMA indicator.
fn run_hma(input: &str, period: usize, column: Option<&str>, dest: &OutputDest) -> Result<()> {
    let parsed = parse_csv(input)?;
    let close = close_prices(&parsed, column)?;

    let output = hma(&close, period)?;
    let lookback = liq_ta::indicators::hma_lookback(period);
    let header = format!("hma_{period}");

    write_single_output(&output, &header, parsed.dates.as_deref(), lookback, dest)
}

/// Run AO indicator.
fn run_ao(input: &str, dest: &OutputDest) -> Result<()> {
    let ohlc = parse_ohlc(input)?;

    let output = ao(&ohlc.high, &ohlc.low)?;
    let lookback = liq_ta::indicators::ao_lookback();

    write_single_output(&output, "ao", ohlc.dates.as_deref(), lookback, dest)
}

/// Run OSMA indicator.
fn run_osma(input: &str, params: &str, column: Option<&str>, dest: &OutputDest) -> Result<()> {
    let (fast, slow, signal) = parse_macd_params(params)?;
    let parsed = parse_csv(input)?;
    let close = close_prices(&parsed, column)?;

    let output = osma(&close, fast, slow, signal)?;
    let lookback = liq_ta::indicators::osma_lookback(fast, slow, signal);
    let header = format!("osma_{fast}_{slow}_{signal}");

    write_single_output(&output, &header, parsed.dates.as_deref(), lookback, dest)
}

/// Run SuperTrend indicator.
fn run_supertrend(input: &str, params: &str, dest: &OutputDest) -> Result<()> {
    let (period, multiplier) = parse_supertrend_params(params)?;
    let ohlc = parse_ohlc(input)?;

    let result = supertrend(&ohlc.high, &ohlc.low, &ohlc.close, period, multiplier)?;
    let lookback = liq_ta::indicators::supertrend_lookback(period);
    let columns: Vec<(&str, &[f64])> = vec![
        ("supertrend", &result.supertrend),
        ("supertrend_upper", &result.upper_band),
        ("supertrend_lower", &result.lower_band),
        ("supertrend_trend", &result.trend),
    ];

    write_multi_output(&columns, ohlc.dates.as_deref(), lookback, dest)
}

/// Run CHOP indicator.
fn run_chop(input: &str, period: usize, dest: &OutputDest) -> Result<()> {
    let ohlc = parse_ohlc(input)?;

    let output = chop(&ohlc.high, &ohlc.low, &ohlc.close, period)?;
    let lookback = liq_ta::indicators::chop_lookback(period);
    let header = format!("chop_{period}");

    write_single_output(&output, &header, ohlc.dates.as_deref(), lookback, dest)
}

/// Run Hurst indicator.
fn run_hurst(input: &str, period: usize, column: Option<&str>, dest: &OutputDest) -> Result<()> {
    let parsed = parse_csv(input)?;
    let close = close_prices(&parsed, column)?;

    let output = hurst(&close, period)?;
    let lookback = liq_ta::indicators::hurst_lookback(period);
    let header = format!("hurst_{period}");

    write_single_output(&output, &header, parsed.dates.as_deref(), lookback, dest)
}

/// Run Gaussian Channel indicator.
fn run_gaussian_channel(
    input: &str,
    params: &str,
    column: Option<&str>,
    dest: &OutputDest,
) -> Result<()> {
    let (period, sigma, multiplier) = parse_gaussian_channel_params(params)?;
    let parsed = parse_csv(input)?;
    let close = close_prices(&parsed, column)?;

    let result = gaussian_channel(&close, period, sigma, multiplier)?;
    let lookback = liq_ta::indicators::gaussian_channel_lookback(period);
    let columns: Vec<(&str, &[f64])> = vec![
        ("gaussian_center", &result.center),
        ("gaussian_upper", &result.upper),
        ("gaussian_lower", &result.lower),
        ("gaussian_trend", &result.trend),
    ];

    write_multi_output(&columns, parsed.dates.as_deref(), lookback, dest)
}

/// Run OBV indicator.
fn run_obv(input: &str, dest: &OutputDest) -> Result<()> {
    let ohlcv = parse_ohlcv(input)?;

    let output = obv(&ohlcv.close, &ohlcv.volume)?;
    let lookback = 0; // OBV has no lookback

    write_single_output(&output, "obv", ohlcv.dates.as_deref(), lookback, dest)
}

/// Run VWAP indicator.
fn run_vwap(input: &str, dest: &OutputDest) -> Result<()> {
    let ohlcv = parse_ohlcv(input)?;

    let output = vwap(&ohlcv.high, &ohlcv.low, &ohlcv.close, &ohlcv.volume)?;
    let lookback = 0; // VWAP has no lookback

    write_single_output(&output, "vwap", ohlcv.dates.as_deref(), lookback, dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_path(prefix: &str, extension: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "{prefix}_{}_{}_{}.{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("t"),
            nanos,
            extension
        ))
    }

    fn write_ohlcv_fixture(rows: usize) -> String {
        let path = unique_path("liq_ta_cli_fixture", "csv");
        let mut csv = String::from("date,open,high,low,close,volume\n");

        for i in 0..rows {
            let base = 100.0 + (i as f64 * 0.5);
            let open = base;
            let high = base + 1.0;
            let low = base - 1.0;
            let close = base + 0.25;
            let volume = 1_000.0 + i as f64;
            csv.push_str(&format!(
                "t{i},{open:.6},{high:.6},{low:.6},{close:.6},{volume:.2}\n"
            ));
        }

        fs::write(&path, csv).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn write_no_close_fixture(rows: usize) -> String {
        let path = unique_path("liq_ta_cli_no_close", "csv");
        let mut csv = String::from("date,open,high,low,volume\n");

        for i in 0..rows {
            let base = 50.0 + i as f64;
            csv.push_str(&format!(
                "n{i},{:.6},{:.6},{:.6},{:.2}\n",
                base,
                base + 1.0,
                base - 1.0,
                500.0 + i as f64
            ));
        }

        fs::write(&path, csv).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn output_path(tag: &str) -> String {
        unique_path(&format!("liq_ta_cli_out_{tag}"), "csv")
            .to_string_lossy()
            .into_owned()
    }

    fn assert_output_header_contains(path: &str, expected_columns: &[&str]) {
        let content = fs::read_to_string(path).unwrap();
        let header = content.lines().next().unwrap_or_default();
        for col in expected_columns {
            assert!(header.contains(col), "missing column '{col}' in '{header}'");
        }
    }

    #[test]
    fn test_exit_codes_defined() {
        assert_eq!(exit_codes::SUCCESS, 0);
        assert_eq!(exit_codes::ARGUMENT_ERROR, 1);
        assert_eq!(exit_codes::DATA_ERROR, 2);
        assert_eq!(exit_codes::COMPUTATION_ERROR, 3);
    }

    #[test]
    fn test_exit_code_for_error_mapping() {
        let invalid = CliError::InvalidArgument {
            argument: "period".to_string(),
            reason: "must be positive".to_string(),
            suggestion: None,
        };
        assert_eq!(exit_code_for_error(&invalid), exit_codes::ARGUMENT_ERROR);

        let io_err = CliError::IoError {
            source: io::Error::new(io::ErrorKind::NotFound, "not found"),
            path: Some("missing.csv".to_string()),
        };
        assert_eq!(exit_code_for_error(&io_err), exit_codes::DATA_ERROR);

        let csv_err = CliError::CsvParseError {
            message: "bad row".to_string(),
            line: Some(3),
        };
        assert_eq!(exit_code_for_error(&csv_err), exit_codes::DATA_ERROR);

        let ind_err = CliError::IndicatorError {
            source: liq_ta::Error::EmptyInput,
        };
        assert_eq!(exit_code_for_error(&ind_err), exit_codes::COMPUTATION_ERROR);
    }

    #[test]
    fn test_close_prices_column_lookup_and_missing_close_error() {
        let with_close = write_ohlcv_fixture(10);
        let parsed = parse_csv(&with_close).unwrap();

        let close_named = close_prices(&parsed, Some(" close ")).unwrap();
        assert_eq!(close_named.len(), 10);

        let missing_col = close_prices(&parsed, Some("does_not_exist"));
        assert!(matches!(missing_col, Err(CliError::CsvParseError { .. })));

        let no_close = write_no_close_fixture(5);
        let parsed_no_close = parse_csv(&no_close).unwrap();
        let auto_close = close_prices(&parsed_no_close, None);
        assert!(matches!(auto_close, Err(CliError::CsvParseError { .. })));

        // Best-effort cleanup of temporary test files.
        let _ = fs::remove_file(with_close);
        // Best-effort cleanup of temporary test files.
        let _ = fs::remove_file(no_close);
    }

    #[test]
    fn test_format_error_lines_taxonomy_and_debug_chain() {
        let invalid = CliError::InvalidArgument {
            argument: "period".to_string(),
            reason: "must be positive".to_string(),
            suggestion: Some("Use a value like 14".to_string()),
        };
        let non_debug = format_error_lines(&invalid, false);
        assert_eq!(non_debug.len(), 1);
        assert!(non_debug[0].contains("[invalid_argument]"));

        let io_err = CliError::IoError {
            source: io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
            path: Some("prices.csv".to_string()),
        };
        let debug_lines = format_error_lines(&io_err, true);
        assert!(debug_lines[0].contains("[io_error]"));
        assert!(
            debug_lines
                .iter()
                .any(|line| line.contains("caused_by[1]: permission denied"))
        );
        assert!(
            debug_lines
                .last()
                .is_some_and(|line| line.starts_with("  debug_repr:"))
        );
    }

    #[test]
    fn test_run_with_args_dispatches_all_stage3_and_core_commands() {
        let input = write_ohlcv_fixture(160);

        let cases: Vec<(Vec<String>, Vec<&str>)> = vec![
            (
                vec![
                    "liq-ta".to_string(),
                    "sma".to_string(),
                    input.clone(),
                    "20".to_string(),
                    "-o".to_string(),
                    output_path("sma"),
                ],
                vec!["sma_20"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "ema".to_string(),
                    input.clone(),
                    "20".to_string(),
                    "-o".to_string(),
                    output_path("ema"),
                ],
                vec!["ema_20"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "rsi".to_string(),
                    input.clone(),
                    "14".to_string(),
                    "-o".to_string(),
                    output_path("rsi"),
                ],
                vec!["rsi_14"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "macd".to_string(),
                    input.clone(),
                    "12,26,9".to_string(),
                    "-o".to_string(),
                    output_path("macd"),
                ],
                vec!["macd", "signal", "histogram"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "bollinger".to_string(),
                    input.clone(),
                    "20,2.0".to_string(),
                    "-o".to_string(),
                    output_path("bollinger"),
                ],
                vec!["upper", "middle", "lower"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "atr".to_string(),
                    input.clone(),
                    "14".to_string(),
                    "-o".to_string(),
                    output_path("atr"),
                ],
                vec!["atr_14"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "stochastic".to_string(),
                    input.clone(),
                    "14,3,3".to_string(),
                    "-o".to_string(),
                    output_path("stochastic"),
                ],
                vec!["percent_k", "percent_d"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "adx".to_string(),
                    input.clone(),
                    "14".to_string(),
                    "-o".to_string(),
                    output_path("adx"),
                ],
                vec!["adx", "plus_di", "minus_di"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "williams-r".to_string(),
                    input.clone(),
                    "14".to_string(),
                    "-o".to_string(),
                    output_path("williams_r"),
                ],
                vec!["williams_r_14"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "donchian".to_string(),
                    input.clone(),
                    "20".to_string(),
                    "-o".to_string(),
                    output_path("donchian"),
                ],
                vec!["donchian_upper", "donchian_middle", "donchian_lower"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "keltner".to_string(),
                    input.clone(),
                    "20,2.0".to_string(),
                    "-o".to_string(),
                    output_path("keltner"),
                ],
                vec!["keltner_upper", "keltner_middle", "keltner_lower"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "ichimoku".to_string(),
                    input.clone(),
                    "9,26,52,26".to_string(),
                    "-o".to_string(),
                    output_path("ichimoku"),
                ],
                vec!["tenkan", "kijun", "senkou_a", "senkou_b", "chikou"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "qqe".to_string(),
                    input.clone(),
                    "14,5,14,4.236".to_string(),
                    "-o".to_string(),
                    output_path("qqe"),
                ],
                vec!["qqe", "qqe_upper", "qqe_lower"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "hma".to_string(),
                    input.clone(),
                    "20".to_string(),
                    "-o".to_string(),
                    output_path("hma"),
                ],
                vec!["hma_20"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "ao".to_string(),
                    input.clone(),
                    "-o".to_string(),
                    output_path("ao"),
                ],
                vec!["ao"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "osma".to_string(),
                    input.clone(),
                    "12,26,9".to_string(),
                    "-o".to_string(),
                    output_path("osma"),
                ],
                vec!["osma_12_26_9"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "supertrend".to_string(),
                    input.clone(),
                    "10,3.0".to_string(),
                    "-o".to_string(),
                    output_path("supertrend"),
                ],
                vec![
                    "supertrend",
                    "supertrend_upper",
                    "supertrend_lower",
                    "supertrend_trend",
                ],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "chop".to_string(),
                    input.clone(),
                    "14".to_string(),
                    "-o".to_string(),
                    output_path("chop"),
                ],
                vec!["chop_14"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "hurst".to_string(),
                    input.clone(),
                    "64".to_string(),
                    "-o".to_string(),
                    output_path("hurst"),
                ],
                vec!["hurst_64"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "gaussian-channel".to_string(),
                    input.clone(),
                    "20,0.5,2.0".to_string(),
                    "-o".to_string(),
                    output_path("gaussian_channel"),
                ],
                vec![
                    "gaussian_center",
                    "gaussian_upper",
                    "gaussian_lower",
                    "gaussian_trend",
                ],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "obv".to_string(),
                    input.clone(),
                    "-o".to_string(),
                    output_path("obv"),
                ],
                vec!["obv"],
            ),
            (
                vec![
                    "liq-ta".to_string(),
                    "vwap".to_string(),
                    input.clone(),
                    "-o".to_string(),
                    output_path("vwap"),
                ],
                vec!["vwap"],
            ),
        ];

        for (argv, expected_header_cols) in &cases {
            let parsed = Args::try_parse_from(argv.clone()).unwrap();
            run_with_args(parsed).unwrap();
            let out = argv.last().unwrap();
            assert_output_header_contains(out, expected_header_cols);
            // Best-effort cleanup of temporary test files.
            let _ = fs::remove_file(out);
        }

        // Best-effort cleanup of temporary test files.
        let _ = fs::remove_file(input);
    }
}
