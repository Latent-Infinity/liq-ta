//! liq-ta CLI library
//!
//! This module exposes the CLI components for testing and reuse.

// Pedantic lint suppressions for CLI code
#![expect(
    clippy::match_same_arms,
    reason = "Match arms are intentionally separate for clarity and future extensibility"
)]
#![expect(
    clippy::doc_markdown,
    reason = "Parameter documentation doesn't need backticks"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "CLI runs on 64-bit systems"
)]
#![expect(
    clippy::missing_errors_doc,
    reason = "CLI error handling is straightforward"
)]

pub mod args;
pub mod csv_parser;
pub mod csv_writer;
pub mod error;

pub use error::{CliError, Result};
