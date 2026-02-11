//! Commonly used types and traits for convenient importing.
//!
//! This prelude provides the most frequently used types, traits, and functions
//! from `liq-ta` to simplify imports in typical usage scenarios.
//!
//! # Usage
//!
//! ```
//! use liq_ta::prelude::*;
//!
//! let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
//!
//! // All indicator functions are available
//! let sma_result = sma(&prices, 3).unwrap();
//! let ema_result = ema(&prices, 3).unwrap();
//! let rsi_result = rsi(&prices, 5).unwrap();
//! ```
//!
//! # Contents
//!
//! This prelude re-exports:
//!
//! ## Error Handling
//! - [`Error`]: The main error type for indicator computation failures
//! - [`Result`]: Type alias for `std::result::Result<T, Error>`
//!
//! ## Traits
//! - [`SeriesElement`]: Trait for numeric types usable in indicators
//! - [`ValidatedInput`]: Extension trait for input validation
//!
//! ## Indicator Functions
//! All indicator functions and their `_into` variants:
//! - `sma`, `ema`, `rsi`, `macd`, `atr`, `bollinger`, `stochastic_*`
//!
//! ## Output Types
//! - [`MacdOutput`]: MACD line, signal line, and histogram
//! - [`BollingerOutput`]: Upper, middle, and lower bands
//! - [`StochasticOutput`]: %K and %D lines
//!
//! ## Lookback Functions
//! - `*_lookback()`: Returns the number of NaN values at the start of output
//! - `*_min_len()`: Returns the minimum input length to avoid errors

// Error types
pub use crate::error::{Error, Result};

// Traits
pub use crate::traits::{SeriesElement, ValidatedInput};

// Indicator functions (simple API)
pub use crate::indicators::{
    atr, bollinger, ema, ema_wilder, ema_with_alpha, macd, rolling_stddev, rsi, sma, stochastic,
    stochastic_fast, stochastic_full, stochastic_slow, true_range,
};

// Indicator functions (_into API for pre-allocated buffers)
pub use crate::indicators::{
    atr_into, bollinger_into, ema_into, ema_wilder_into, ema_with_alpha_into, macd_into,
    rolling_stddev_into, rsi_into, sma_into, stochastic_fast_into, stochastic_full_into,
    stochastic_into, stochastic_slow_into, true_range_into,
};

// Multi-output types
pub use crate::indicators::{BollingerOutput, MacdOutput, StochasticOutput};

// Configuration types (Gravity Check 1.1 - Zero-Config)
pub use crate::indicators::{Bollinger, Macd, Stochastic};

// Lookback functions (PRD §4.4 - semver-stable)
pub use crate::indicators::{
    atr_lookback, atr_min_len, bollinger_lookback, bollinger_min_len, ema_lookback, ema_min_len,
    macd_line_lookback, macd_min_len, macd_signal_lookback, rsi_lookback, rsi_min_len,
    sma_lookback, sma_min_len, stochastic_d_lookback, stochastic_k_lookback, stochastic_min_len,
    true_range_lookback,
};

// Rolling extrema functions
pub use crate::kernels::rolling_extrema::{
    rolling_extrema_lookback, rolling_extrema_min_len, rolling_max, rolling_min,
};
