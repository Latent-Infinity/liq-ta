//! CLI argument parsing module.
//!
//! This module defines the command-line interface for liq-ta using clap.
//! The CLI follows the pattern: `liq-ta <indicator> <input.csv> [params] [-o output.csv]`
//!
//! # Examples
//!
//! ```bash
//! # Simple Moving Average with default period (20)
//! liq-ta sma input.csv
//!
//! # SMA with custom period
//! liq-ta sma input.csv 20
//!
//! # EMA with file output
//! liq-ta ema input.csv 20 -o output.csv
//!
//! # RSI with default period (14)
//! liq-ta rsi input.csv
//!
//! # MACD with custom parameters
//! liq-ta macd input.csv 12,26,9
//!
//! # Bollinger Bands
//! liq-ta bollinger input.csv 20,2.0
//!
//! # Stochastic
//! liq-ta stochastic input.csv 14,3
//! ```

use clap::{Parser, Subcommand};

use crate::error::{CliError, Result};

/// liq-ta: High-performance technical analysis CLI
#[derive(Parser, Debug)]
#[command(name = "liq-ta")]
#[command(
    author,
    version,
    about = "High-performance technical analysis indicators"
)]
#[command(
    long_about = "liq-ta provides fast, accurate technical analysis indicator \
    computation for financial data. Input is read from CSV files and output can be \
    written to files or stdout."
)]
#[command(after_help = "\
EXIT CODES:
    0    Success - computation completed successfully
    1    Argument error - invalid parameters or unknown command
    2    Data error - file not found, permission denied, or CSV parse error
    3    Computation error - indicator calculation failed (e.g., insufficient data)

EXAMPLES:
    liq-ta sma prices.csv 20          # Simple Moving Average (period 20)
    liq-ta ema prices.csv 12 -o out.csv   # EMA with output file
    liq-ta rsi prices.csv             # RSI with default period (14)
    liq-ta macd prices.csv 12,26,9    # MACD with custom periods
    liq-ta bollinger prices.csv 20,2.0    # Bollinger Bands
    liq-ta stochastic ohlc.csv 14,3,3     # Slow Stochastic (k=14, d=3, slow=3)
")]
pub struct Args {
    /// The indicator to compute
    #[command(subcommand)]
    pub command: Command,
}

/// Available indicator commands.
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Simple Moving Average
    #[command(about = "Simple Moving Average (SMA)")]
    Sma {
        /// Input CSV file
        input: String,

        /// Period for the moving average
        #[arg(default_value = "20")]
        period: usize,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Column to use for close prices (auto-detected if not specified)
        #[arg(short, long)]
        column: Option<String>,
    },

    /// Exponential Moving Average
    #[command(about = "Exponential Moving Average (EMA)")]
    Ema {
        /// Input CSV file
        input: String,

        /// Period for the moving average
        #[arg(default_value = "20")]
        period: usize,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Column to use for close prices
        #[arg(short, long)]
        column: Option<String>,
    },

    /// Relative Strength Index
    #[command(about = "Relative Strength Index (RSI)")]
    Rsi {
        /// Input CSV file
        input: String,

        /// Period for RSI calculation
        #[arg(default_value = "14")]
        period: usize,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Column to use for close prices
        #[arg(short, long)]
        column: Option<String>,
    },

    /// Moving Average Convergence Divergence
    #[command(about = "MACD (Moving Average Convergence Divergence)")]
    Macd {
        /// Input CSV file
        input: String,

        /// Parameters: `fast_period,slow_period,signal_period` (e.g., 12,26,9)
        #[arg(default_value = "12,26,9")]
        params: String,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Column to use for close prices
        #[arg(short, long)]
        column: Option<String>,
    },

    /// Bollinger Bands
    #[command(about = "Bollinger Bands")]
    Bollinger {
        /// Input CSV file
        input: String,

        /// Parameters: `period,std_dev` (e.g., 20,2.0)
        #[arg(default_value = "20,2.0")]
        params: String,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Column to use for close prices
        #[arg(short, long)]
        column: Option<String>,
    },

    /// Average True Range
    #[command(about = "Average True Range (ATR)")]
    Atr {
        /// Input CSV file
        input: String,

        /// Period for ATR calculation
        #[arg(default_value = "14")]
        period: usize,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Stochastic Oscillator
    #[command(about = "Stochastic Oscillator")]
    Stochastic {
        /// Input CSV file
        input: String,

        /// Parameters: `k_period,d_period`\[,`k_slowing`\] (e.g., 14,3 or 14,3,3)
        #[arg(default_value = "14,3")]
        params: String,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Average Directional Index
    #[command(about = "Average Directional Index (ADX)")]
    Adx {
        /// Input CSV file
        input: String,

        /// Period for ADX calculation
        #[arg(default_value = "14")]
        period: usize,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Williams %R
    #[command(about = "Williams %R oscillator")]
    WilliamsR {
        /// Input CSV file
        input: String,

        /// Period for Williams %R calculation
        #[arg(default_value = "14")]
        period: usize,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Donchian Channels
    #[command(about = "Donchian Channels (price channel)")]
    Donchian {
        /// Input CSV file
        input: String,

        /// Period for Donchian Channels
        #[arg(default_value = "20")]
        period: usize,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// On-Balance Volume
    #[command(about = "On-Balance Volume (OBV)")]
    Obv {
        /// Input CSV file
        input: String,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Volume Weighted Average Price
    #[command(about = "Volume Weighted Average Price (VWAP)")]
    Vwap {
        /// Input CSV file
        input: String,

        /// Output CSV file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
}

impl Args {
    /// Parse command-line arguments.
    #[must_use]
    pub fn parse_args() -> Self {
        Args::parse()
    }

    /// Get the input file path from the command.
    #[must_use]
    pub fn input_path(&self) -> &str {
        match &self.command {
            Command::Sma { input, .. } => input,
            Command::Ema { input, .. } => input,
            Command::Rsi { input, .. } => input,
            Command::Macd { input, .. } => input,
            Command::Bollinger { input, .. } => input,
            Command::Atr { input, .. } => input,
            Command::Stochastic { input, .. } => input,
            Command::Adx { input, .. } => input,
            Command::WilliamsR { input, .. } => input,
            Command::Donchian { input, .. } => input,
            Command::Obv { input, .. } => input,
            Command::Vwap { input, .. } => input,
        }
    }

    /// Get the output file path from the command, if specified.
    #[must_use]
    pub fn output_path(&self) -> Option<&str> {
        match &self.command {
            Command::Sma { output, .. } => output.as_deref(),
            Command::Ema { output, .. } => output.as_deref(),
            Command::Rsi { output, .. } => output.as_deref(),
            Command::Macd { output, .. } => output.as_deref(),
            Command::Bollinger { output, .. } => output.as_deref(),
            Command::Atr { output, .. } => output.as_deref(),
            Command::Stochastic { output, .. } => output.as_deref(),
            Command::Adx { output, .. } => output.as_deref(),
            Command::WilliamsR { output, .. } => output.as_deref(),
            Command::Donchian { output, .. } => output.as_deref(),
            Command::Obv { output, .. } => output.as_deref(),
            Command::Vwap { output, .. } => output.as_deref(),
        }
    }
}

/// Parse MACD parameters from string "fast,slow,signal".
pub fn parse_macd_params(params: &str) -> Result<(usize, usize, usize)> {
    let parts: Vec<&str> = params.split(',').collect();
    if parts.len() != 3 {
        return Err(CliError::InvalidArgument {
            argument: "params".to_string(),
            reason: format!("MACD requires 3 parameters, got {}", parts.len()),
            suggestion: Some("Use format: fast,slow,signal (e.g., 12,26,9)".to_string()),
        });
    }

    let fast = parts[0]
        .trim()
        .parse::<usize>()
        .map_err(|_| CliError::InvalidArgument {
            argument: "fast_period".to_string(),
            reason: format!("cannot parse '{}' as integer", parts[0]),
            suggestion: Some("Use a positive integer like 12".to_string()),
        })?;

    let slow = parts[1]
        .trim()
        .parse::<usize>()
        .map_err(|_| CliError::InvalidArgument {
            argument: "slow_period".to_string(),
            reason: format!("cannot parse '{}' as integer", parts[1]),
            suggestion: Some("Use a positive integer like 26".to_string()),
        })?;

    let signal = parts[2]
        .trim()
        .parse::<usize>()
        .map_err(|_| CliError::InvalidArgument {
            argument: "signal_period".to_string(),
            reason: format!("cannot parse '{}' as integer", parts[2]),
            suggestion: Some("Use a positive integer like 9".to_string()),
        })?;

    if fast == 0 || slow == 0 || signal == 0 {
        return Err(CliError::InvalidArgument {
            argument: "params".to_string(),
            reason: "all periods must be positive".to_string(),
            suggestion: Some("Use positive integers like 12,26,9".to_string()),
        });
    }

    if fast >= slow {
        return Err(CliError::InvalidArgument {
            argument: "params".to_string(),
            reason: "fast period must be less than slow period".to_string(),
            suggestion: Some("Use fast < slow (e.g., 12,26,9)".to_string()),
        });
    }

    Ok((fast, slow, signal))
}

/// Parse Bollinger parameters from string "`period,std_dev`".
pub fn parse_bollinger_params(params: &str) -> Result<(usize, f64)> {
    let parts: Vec<&str> = params.split(',').collect();
    if parts.len() != 2 {
        return Err(CliError::InvalidArgument {
            argument: "params".to_string(),
            reason: format!("Bollinger requires 2 parameters, got {}", parts.len()),
            suggestion: Some("Use format: period,std_dev (e.g., 20,2.0)".to_string()),
        });
    }

    let period = parts[0]
        .trim()
        .parse::<usize>()
        .map_err(|_| CliError::InvalidArgument {
            argument: "period".to_string(),
            reason: format!("cannot parse '{}' as integer", parts[0]),
            suggestion: Some("Use a positive integer like 20".to_string()),
        })?;

    let std_dev = parts[1]
        .trim()
        .parse::<f64>()
        .map_err(|_| CliError::InvalidArgument {
            argument: "std_dev".to_string(),
            reason: format!("cannot parse '{}' as number", parts[1]),
            suggestion: Some("Use a positive number like 2.0".to_string()),
        })?;

    if period == 0 {
        return Err(CliError::InvalidArgument {
            argument: "period".to_string(),
            reason: "period must be positive".to_string(),
            suggestion: Some("Use a positive integer like 20".to_string()),
        });
    }

    if std_dev <= 0.0 {
        return Err(CliError::InvalidArgument {
            argument: "std_dev".to_string(),
            reason: "std_dev must be positive".to_string(),
            suggestion: Some("Use a positive number like 2.0".to_string()),
        });
    }

    Ok((period, std_dev))
}

/// Parse Stochastic parameters from string "`k_period,d_period`\[,`k_slowing`\]".
pub fn parse_stochastic_params(params: &str) -> Result<(usize, usize, usize)> {
    let parts: Vec<&str> = params.split(',').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Err(CliError::InvalidArgument {
            argument: "params".to_string(),
            reason: format!("Stochastic requires 2-3 parameters, got {}", parts.len()),
            suggestion: Some(
                "Use format: k_period,d_period[,k_slowing] (e.g., 14,3 or 14,3,3)".to_string(),
            ),
        });
    }

    let k_period = parts[0]
        .trim()
        .parse::<usize>()
        .map_err(|_| CliError::InvalidArgument {
            argument: "k_period".to_string(),
            reason: format!("cannot parse '{}' as integer", parts[0]),
            suggestion: Some("Use a positive integer like 14".to_string()),
        })?;

    let d_period = parts[1]
        .trim()
        .parse::<usize>()
        .map_err(|_| CliError::InvalidArgument {
            argument: "d_period".to_string(),
            reason: format!("cannot parse '{}' as integer", parts[1]),
            suggestion: Some("Use a positive integer like 3".to_string()),
        })?;

    let k_slowing = if parts.len() == 3 {
        parts[2]
            .trim()
            .parse::<usize>()
            .map_err(|_| CliError::InvalidArgument {
                argument: "k_slowing".to_string(),
                reason: format!("cannot parse '{}' as integer", parts[2]),
                suggestion: Some("Use a positive integer like 3".to_string()),
            })?
    } else {
        1 // Default to fast stochastic
    };

    if k_period == 0 || d_period == 0 || k_slowing == 0 {
        return Err(CliError::InvalidArgument {
            argument: "params".to_string(),
            reason: "all periods must be positive".to_string(),
            suggestion: Some("Use positive integers like 14,3".to_string()),
        });
    }

    Ok((k_period, d_period, k_slowing))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Task 3.5 Test Cases
    // ==========================================================================

    #[test]
    fn test_parse_sma_basic() {
        // New order: input first, then period
        let args = Args::try_parse_from(["liq-ta", "sma", "input.csv", "20"]).unwrap();
        match args.command {
            Command::Sma {
                period,
                input,
                output,
                ..
            } => {
                assert_eq!(period, 20);
                assert_eq!(input, "input.csv");
                assert!(output.is_none());
            }
            _ => panic!("Expected Sma command"),
        }
    }

    #[test]
    fn test_parse_ema_with_output() {
        let args =
            Args::try_parse_from(["liq-ta", "ema", "input.csv", "20", "-o", "output.csv"]).unwrap();
        match args.command {
            Command::Ema {
                period,
                input,
                output,
                ..
            } => {
                assert_eq!(period, 20);
                assert_eq!(input, "input.csv");
                assert_eq!(output, Some("output.csv".to_string()));
            }
            _ => panic!("Expected Ema command"),
        }
    }

    #[test]
    fn test_parse_rsi_with_period() {
        let args = Args::try_parse_from(["liq-ta", "rsi", "input.csv", "14"]).unwrap();
        match args.command {
            Command::Rsi { period, input, .. } => {
                assert_eq!(period, 14);
                assert_eq!(input, "input.csv");
            }
            _ => panic!("Expected Rsi command"),
        }
    }

    #[test]
    fn test_parse_macd_multi_param() {
        let args = Args::try_parse_from(["liq-ta", "macd", "input.csv", "12,26,9"]).unwrap();
        match args.command {
            Command::Macd { params, input, .. } => {
                assert_eq!(params, "12,26,9");
                assert_eq!(input, "input.csv");
                let (fast, slow, signal) = parse_macd_params(&params).unwrap();
                assert_eq!(fast, 12);
                assert_eq!(slow, 26);
                assert_eq!(signal, 9);
            }
            _ => panic!("Expected Macd command"),
        }
    }

    #[test]
    fn test_parse_bollinger_with_float() {
        let args = Args::try_parse_from(["liq-ta", "bollinger", "input.csv", "20,2.0"]).unwrap();
        match args.command {
            Command::Bollinger { params, input, .. } => {
                assert_eq!(params, "20,2.0");
                assert_eq!(input, "input.csv");
                let (period, std_dev) = parse_bollinger_params(&params).unwrap();
                assert_eq!(period, 20);
                assert!((std_dev - 2.0).abs() < 1e-10);
            }
            _ => panic!("Expected Bollinger command"),
        }
    }

    #[test]
    fn test_parse_stochastic() {
        let args = Args::try_parse_from(["liq-ta", "stochastic", "input.csv", "14,3"]).unwrap();
        match args.command {
            Command::Stochastic { params, input, .. } => {
                assert_eq!(params, "14,3");
                assert_eq!(input, "input.csv");
                let (k, d, slowing) = parse_stochastic_params(&params).unwrap();
                assert_eq!(k, 14);
                assert_eq!(d, 3);
                assert_eq!(slowing, 1); // Default
            }
            _ => panic!("Expected Stochastic command"),
        }
    }

    #[test]
    fn test_parse_stochastic_with_slowing() {
        let (k, d, slowing) = parse_stochastic_params("14,3,3").unwrap();
        assert_eq!(k, 14);
        assert_eq!(d, 3);
        assert_eq!(slowing, 3);
    }

    #[test]
    fn test_parse_help() {
        let result = Args::try_parse_from(["liq-ta", "--help"]);
        assert!(result.is_err()); // --help causes parse to "fail" with help display
    }

    #[test]
    fn test_parse_version() {
        let result = Args::try_parse_from(["liq-ta", "--version"]);
        assert!(result.is_err()); // --version causes parse to "fail" with version display
    }

    #[test]
    fn test_error_missing_indicator() {
        let result = Args::try_parse_from(["liq-ta"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_missing_input_file() {
        let result = Args::try_parse_from(["liq-ta", "sma"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_invalid_macd_params() {
        let result = parse_macd_params("12,26");
        assert!(result.is_err());
        if let Err(CliError::InvalidArgument { reason, .. }) = result {
            assert!(reason.contains("3 parameters"));
        }
    }

    #[test]
    fn test_error_invalid_bollinger_params() {
        let result = parse_bollinger_params("20");
        assert!(result.is_err());
        if let Err(CliError::InvalidArgument { reason, .. }) = result {
            assert!(reason.contains("2 parameters"));
        }
    }

    #[test]
    fn test_error_non_numeric_param() {
        let result = parse_macd_params("12,abc,9");
        assert!(result.is_err());
        if let Err(CliError::InvalidArgument { argument, .. }) = result {
            assert_eq!(argument, "slow_period");
        }
    }

    #[test]
    fn test_error_zero_period() {
        let result = parse_macd_params("0,26,9");
        assert!(result.is_err());
        if let Err(CliError::InvalidArgument { reason, .. }) = result {
            assert!(reason.contains("positive"));
        }
    }

    #[test]
    fn test_error_fast_ge_slow() {
        let result = parse_macd_params("26,12,9");
        assert!(result.is_err());
        if let Err(CliError::InvalidArgument { reason, .. }) = result {
            assert!(reason.contains("fast period must be less"));
        }
    }

    #[test]
    fn test_input_path_accessor() {
        let args = Args::try_parse_from(["liq-ta", "sma", "test.csv", "20"]).unwrap();
        assert_eq!(args.input_path(), "test.csv");
    }

    #[test]
    fn test_output_path_accessor() {
        let args =
            Args::try_parse_from(["liq-ta", "sma", "test.csv", "20", "-o", "out.csv"]).unwrap();
        assert_eq!(args.output_path(), Some("out.csv"));

        let args2 = Args::try_parse_from(["liq-ta", "sma", "test.csv"]).unwrap();
        assert_eq!(args2.output_path(), None);
    }

    #[test]
    fn test_default_period_values() {
        // SMA default = 20
        let args = Args::try_parse_from(["liq-ta", "sma", "input.csv"]).unwrap();
        match args.command {
            Command::Sma { period, .. } => assert_eq!(period, 20),
            _ => panic!("Expected Sma"),
        }

        // RSI default = 14
        let args = Args::try_parse_from(["liq-ta", "rsi", "input.csv"]).unwrap();
        match args.command {
            Command::Rsi { period, .. } => assert_eq!(period, 14),
            _ => panic!("Expected Rsi"),
        }
    }

    #[test]
    fn test_atr_command() {
        let args = Args::try_parse_from(["liq-ta", "atr", "input.csv", "14"]).unwrap();
        match args.command {
            Command::Atr { period, input, .. } => {
                assert_eq!(period, 14);
                assert_eq!(input, "input.csv");
            }
            _ => panic!("Expected Atr command"),
        }
    }

    #[test]
    fn test_negative_std_dev_error() {
        let result = parse_bollinger_params("20,-2.0");
        assert!(result.is_err());
        if let Err(CliError::InvalidArgument { reason, .. }) = result {
            assert!(reason.contains("positive"));
        }
    }
}
