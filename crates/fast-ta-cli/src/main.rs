//! fast-ta command-line interface
//!
//! This binary provides a command-line interface for computing technical
//! analysis indicators on CSV data files.
//!
//! # Usage
//!
//! ```bash
//! fast-ta <indicator> <input.csv> [params] [-o output.csv] [-c column]
//! ```
//!
//! # Examples
//!
//! ```bash
//! # SMA with default period (20)
//! fast-ta sma prices.csv
//!
//! # SMA with custom period
//! fast-ta sma prices.csv 14
//!
//! # MACD with output to file
//! fast-ta macd prices.csv 12,26,9 -o macd_output.csv
//!
//! # Stochastic oscillator
//! fast-ta stochastic ohlc.csv 14,3,3
//! ```
//!
//! # Exit Codes (per PRD §5.4)
//!
//! - 0: Success
//! - 1: Argument error (invalid parameters)
//! - 2: Data error (file not found, parse error)
//! - 3: Computation error (indicator failed)

use fast_ta_cli::args::{
    parse_bollinger_params, parse_macd_params, parse_stochastic_params, Args, Command,
};
use fast_ta_cli::csv_parser::{self, parse_csv, parse_ohlc, parse_ohlcv};
use fast_ta_cli::csv_writer::{write_multi_output, write_single_output, OutputDest};
use fast_ta_cli::{CliError, Result};

use fast_ta::indicators::{
    adx, atr, bollinger, donchian, ema, macd, obv, rsi, sma, stochastic, vwap, williams_r,
};

/// Exit codes per PRD §5.4
mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const ARGUMENT_ERROR: i32 = 1;
    pub const DATA_ERROR: i32 = 2;
    pub const COMPUTATION_ERROR: i32 = 3;
}

fn main() {
    let result = run();

    match result {
        Ok(()) => std::process::exit(exit_codes::SUCCESS),
        Err(e) => {
            eprintln!("error: {e}");
            let code = match &e {
                CliError::InvalidArgument { .. } => exit_codes::ARGUMENT_ERROR,
                CliError::IoError { .. } | CliError::CsvParseError { .. } => exit_codes::DATA_ERROR,
                CliError::IndicatorError { .. } => exit_codes::COMPUTATION_ERROR,
            };
            std::process::exit(code);
        }
    }
}

/// Main entry point for CLI logic.
fn run() -> Result<()> {
    let args = Args::parse_args();
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

    #[test]
    fn test_exit_codes_defined() {
        assert_eq!(exit_codes::SUCCESS, 0);
        assert_eq!(exit_codes::ARGUMENT_ERROR, 1);
        assert_eq!(exit_codes::DATA_ERROR, 2);
        assert_eq!(exit_codes::COMPUTATION_ERROR, 3);
    }
}
