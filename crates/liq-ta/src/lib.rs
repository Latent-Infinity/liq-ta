//! liq-ta: High-performance technical analysis library
//!
//! This crate provides fast and accurate implementations of common technical
//! analysis indicators used in financial trading and analysis.
//!
//! # Features
//!
//! - **Performance**: O(n) algorithms with optimized memory access patterns
//! - **SIMD**: Portable SIMD acceleration enabled by default (requires nightly Rust)
//! - **Accuracy**: Validated against spec fixtures with documented edge cases
//! - **Generics**: Works with both `f32` and `f64` data types
//! - **Safety**: Comprehensive error handling for edge cases
//! - **Ergonomic**: Simple API with `_into` variants for zero-allocation use
//!
//! # SIMD Acceleration
//!
//! This crate uses portable SIMD for high-performance computations and requires
//! nightly Rust. SIMD provides 2.5-4x speedup for bulk reduction operations.
//!
//! SIMD acceleration is used automatically in:
//! - [`indicators::sma()`] - SIMD for f64 initial window computation
//! - [`indicators::bollinger()`] - SIMD for f64 initial sum and sum-of-squares
//! - [`indicators::rolling_stddev`] - SIMD for f64 initial sum and sum-of-squares
//! - [`kernels::simd`] module - Low-level SIMD kernels (sum, variance, correlation, etc.)
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
//! For high-performance use with pre-allocated buffers:
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

// Enable portable_simd feature (requires nightly Rust)
#![feature(portable_simd)]
#![deny(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::needless_collect)]
#![warn(clippy::or_fun_call)]
#![warn(clippy::inefficient_to_string)]
#![warn(clippy::useless_conversion)]
#![allow(clippy::style)] // Numeric kernels favor explicit low-level style over idiomatic style lints
#![allow(clippy::pedantic)] // Pedantic lints are too noisy for this codebase's performance patterns
#![allow(clippy::nursery)] // Nursery lints are unstable and produce high churn
#![allow(clippy::restriction)] // Restriction lints conflict with intentionally unsafe/optimized internals
#![allow(clippy::doc_markdown)] // Documentation uses finance/math terminology without backticks everywhere
#![allow(clippy::must_use_candidate)] // API surface is intentionally selective about #[must_use]
#![allow(clippy::missing_const_for_fn)] // Constness here is not a project requirement
#![allow(clippy::ptr_as_ptr)] // Raw pointer casts are used in controlled SIMD/generic specializations
#![allow(clippy::transmute_ptr_to_ptr)] // Transmute-based casting is used in audited monomorphization paths
#![allow(clippy::missing_transmute_annotations)] // Type context is explicit in surrounding code and tests
#![allow(clippy::items_after_statements)] // Local helper placement keeps kernels adjacent to call-sites
#![allow(clippy::inline_always)] // Hot paths use explicit inlining hints for benchmark stability
#![allow(clippy::if_same_then_else)] // Branch shape is kept explicit for readability across type paths
#![allow(clippy::uninit_vec)]
// Output buffers are intentionally preallocated then fully written before reads
// Allowed pedantic lints - intentional design choices for this codebase
#![allow(clippy::module_name_repetitions)] // Types like error::Error are idiomatic in Rust libraries
#![allow(clippy::similar_names)] // Financial indicator variables often have similar names (e.g., ema1, ema2)
#![allow(clippy::cast_precision_loss)] // Expected when casting usize to f64 for indicator calculations
#![allow(clippy::cast_possible_truncation)] // Controlled truncation in period calculations
#![allow(clippy::cast_sign_loss)] // Controlled in validated contexts
#![allow(clippy::too_many_lines)] // Complex indicators naturally have longer functions
#![allow(clippy::too_many_arguments)] // OHLCV indicators require multiple array parameters
#![allow(clippy::needless_range_loop)] // Index loops often clearer for array algorithms
#![allow(clippy::float_cmp)] // Intentional NaN checks and exact comparisons
#![allow(clippy::suboptimal_flops)] // mul_add can change results; we prioritize reproducibility
#![allow(clippy::many_single_char_names)] // Common in mathematical code (i, j, n, etc.)
#![allow(clippy::manual_range_contains)] // Explicit comparisons often clearer in bounds checking
#![allow(clippy::cast_lossless)] // Explicit i32 to f64 casts are intentional
#![allow(clippy::redundant_else)] // Style preference for explicit else blocks
#![allow(clippy::int_plus_one)] // Explicit +1/-1 comparisons clearer in index math
#![allow(clippy::manual_slice_size_calculation)] // Explicit slice calculations clearer
#![allow(clippy::struct_field_names)] // Naming convention for period fields is intentional
#![allow(clippy::manual_memcpy)] // Explicit loops for clarity in algorithm code
#![allow(clippy::if_not_else)] // Style preference
#![allow(clippy::manual_clamp)] // Explicit clamping clearer in some contexts
#![allow(clippy::missing_panics_doc)] // Panics are unreachable in validated paths
#![allow(clippy::approx_constant)] // Explicit constant values for reproducibility
#![allow(clippy::type_complexity)] // Complex types acceptable for generic indicator functions
#![allow(clippy::unnecessary_wraps)] // Some Results are for API consistency

pub mod batch;
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
