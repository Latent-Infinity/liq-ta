//! liq-ta CLI library
//!
//! This module exposes the CLI components for testing and reuse.

// Pedantic lint suppressions for CLI code
#![allow(clippy::match_same_arms)] // Match arms are intentionally separate for clarity and future extensibility
#![allow(clippy::must_use_candidate)] // CLI functions are called for side effects
#![allow(clippy::doc_markdown)] // Parameter documentation doesn't need backticks
#![allow(clippy::unreadable_literal)] // Test values are clearer without separators
#![allow(clippy::similar_names)] // CLI argument names may be similar
#![allow(clippy::cast_possible_truncation)] // CLI runs on 64-bit systems
#![allow(clippy::missing_errors_doc)] // CLI error handling is straightforward
#![allow(clippy::unnecessary_wraps)] // Result wrappers needed for consistent API

pub mod args;
pub mod csv_parser;
pub mod csv_writer;
pub mod error;

pub use error::{CliError, Result};
