//! liq-ta: High-performance technical analysis library
//!
//! This crate provides fast and accurate implementations of common technical
//! analysis indicators used in financial trading and analysis.
//!
//! # Features
//!
//! - **Performance**: Efficient numeric implementations with optimized memory access patterns
//! - **Stable Rust**: Safe chunked numeric kernels with portable scalar lanes
//! - **Accuracy**: Validated against spec fixtures with documented edge cases
//! - **Generics**: Works with both `f32` and `f64` data types
//! - **Safety**: Comprehensive error handling for edge cases
//! - **Ergonomic**: Simple API with `_into` variants for reusing output buffers
//!
//! # Numeric Kernels
//!
//! Chunked kernels use safe scalar lanes and may be vectorized by the compiler.
//! The `kernels::simd` module name remains available for existing callers.
//!
//! # Quick Start
//!
//! ```
//! use liq_ta::prelude::*;
//!
//! let prices = vec![44.0_f64, 44.5, 43.5, 44.0, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0];
//! let sma_result = sma(&prices, 5).unwrap();
//!
//! // First 4 values are NaN (lookback period)
//! assert!(sma_result[3].is_nan());
//! // SMA starts at index 4
//! assert!((sma_result[4] - 44.1).abs() < 0.01);
//! ```
//!
//! # Available Indicators
//!
//! ## Moving Averages
//! - [`indicators::sma()`]: Simple Moving Average
//! - [`indicators::ema()`]: Exponential Moving Average
//! - [`indicators::ema_wilder()`]: Wilder's Smoothing (alpha = 1/n)
//!
//! ## Momentum
//! - [`indicators::rsi()`]: Relative Strength Index
//! - [`indicators::macd()`]: Moving Average Convergence Divergence
//! - [`indicators::stochastic_fast()`]: Fast Stochastic Oscillator
//! - [`indicators::stochastic_slow()`]: Slow Stochastic Oscillator
//! - [`indicators::williams_r()`]: Williams %R
//! - [`indicators::adx()`]: Average Directional Index
//!
//! ## Volatility
//! - [`indicators::atr()`]: Average True Range
//! - [`indicators::bollinger()`]: Bollinger Bands
//! - [`indicators::donchian()`]: Donchian Channels
//!
//! ## Volume
//! - [`indicators::obv()`]: On-Balance Volume
//! - [`indicators::vwap()`]: Volume Weighted Average Price
//!
//! # API Layers
//!
//! ## Simple API
//!
//! The simplest way to compute indicators:
//!
//! ```
//! use liq_ta::prelude::*;
//!
//! let prices = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
//! let result = sma(&prices, 3).unwrap();
//! ```
//!
//! ## Buffer API (`_into` variants)
//!
//! Reuse pre-allocated output buffers. Some indicators also allocate internal scratch space:
//!
//! ```
//! use liq_ta::prelude::*;
//!
//! let prices = vec![1.0; 1000];
//! let mut output = vec![0.0; 1000];
//! sma_into(&prices, 20, &mut output).unwrap();
//! ```
//!
//! ## Configuration Types
//!
//! For indicators with many parameters:
//!
//! ```
//! use liq_ta::prelude::*;
//!
//! let prices = vec![1.0; 100];
//! let result = Macd::default().compute(&prices).unwrap();
//! ```
//!
//! # Error Handling
//!
//! All indicator functions return [`Result<T, Error>`] to handle edge cases:
//!
//! ```
//! use liq_ta::prelude::*;
//!
//! // Period too long for data
//! let short_data = vec![1.0_f64, 2.0];
//! let result = sma(&short_data, 10);
//! assert!(result.is_err());
//!
//! // Empty data
//! let empty: Vec<f64> = vec![];
//! let result = sma(&empty, 5);
//! assert!(result.is_err());
//! ```
//!
//! # Numeric Behavior
//!
//! - **NaN propagation**: NaN in window produces NaN output
//! - **Full-length output**: Output length equals input length (NaN prefix for lookback)
//! - **Deterministic**: Same inputs always produce identical outputs

#![deny(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::needless_collect)]
#![warn(clippy::or_fun_call)]
#![warn(clippy::inefficient_to_string)]
#![warn(clippy::useless_conversion)]
#![expect(
    clippy::style,
    reason = "numeric kernels favor explicit low-level style over idiomatic style lints"
)]
#![expect(
    clippy::if_same_then_else,
    reason = "branch shape is kept explicit for readability across type paths"
)]
#![expect(
    clippy::too_many_arguments,
    reason = "OHLCV indicators require multiple array parameters"
)]
#![expect(
    clippy::needless_range_loop,
    reason = "index loops often clearer for array algorithms"
)]
#![expect(
    clippy::int_plus_one,
    reason = "explicit +1/-1 comparisons clearer in index math"
)]
#![expect(
    clippy::manual_memcpy,
    reason = "explicit loops for clarity in algorithm code"
)]
#![expect(
    clippy::manual_clamp,
    reason = "explicit clamping keeps NaN handling visible"
)]

pub mod batch;
mod cast;
pub mod error;
pub mod indicators;
pub mod kernels;
pub mod precision;
pub mod prelude;
pub mod traits;
pub mod utils;

// Re-export commonly used types at crate root
pub use error::{Error, Result};
pub use indicators::sma;
pub use precision::{
    PrecisionMode, current_precision_mode, set_precision_mode, with_precision_mode,
};
pub use traits::{SeriesElement, ValidatedInput};
pub use utils::{
    EPSILON, LOOSE_EPSILON, approx_eq, approx_eq_relative, count_nan_prefix, count_nans,
};
