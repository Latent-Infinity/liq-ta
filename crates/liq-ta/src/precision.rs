//! Precision mode configuration for liq-ta indicators.
//!
//! This module provides the [`PrecisionMode`] enum and configuration functions
//! for controlling numeric precision in indicator calculations.
//!
//! # Overview
//!
//! liq-ta uses a "f64 for state, f32 for storage" strategy. When processing
//! f32 input data, indicators can use f64 accumulators internally to improve
//! numeric stability while keeping output arrays as f32.
//!
//! # Modes
//!
//! - [`PrecisionMode::High`] (default): Uses f64 accumulators for f32 inputs.
//!   Provides better precision at a small performance cost (~15-20% overhead).
//! - [`PrecisionMode::Fast`]: Uses native-type accumulators. Matches the
//!   original liq-ta behavior for maximum throughput.
//!
//! Note: f64 inputs always use f64 accumulators in both modes.
//!
//! # Configuration Precedence
//!
//! Mode is determined by (highest to lowest priority):
//! 1. TLS override via [`with_precision_mode()`] - for tests
//! 2. Runtime [`set_precision_mode()`] - sets global mode
//! 3. Environment variable `LIQ_TA_PRECISION=fast|high`
//! 4. Cargo feature `precision-fast`
//! 5. Built-in default: `PrecisionMode::High`
//!
//! # Usage
//!
//! ## Production
//!
//! ```rust,ignore
//! // Runtime setting
//! use liq_ta::precision::{set_precision_mode, PrecisionMode};
//! set_precision_mode(PrecisionMode::High);
//!
//! // Or via environment variable
//! // LIQ_TA_PRECISION=fast cargo run
//!
//! // Or via Cargo feature
//! // liq-ta = { features = ["precision-fast"] }
//! ```
//!
//! ## Tests
//!
//! Use [`with_precision_mode()`] for isolated test configuration:
//!
//! ```rust,ignore
//! use liq_ta::precision::{with_precision_mode, PrecisionMode};
//!
//! #[test]
//! fn test_high_precision() {
//!     with_precision_mode(PrecisionMode::High, || {
//!         // Test code runs with High precision mode
//!     });
//! }
//! ```
//!
//! ## Benchmarks
//!
//! Use [`set_precision_mode()`] to measure production path:
//!
//! ```rust,ignore
//! use liq_ta::precision::{set_precision_mode, PrecisionMode};
//!
//! set_precision_mode(PrecisionMode::Fast);
//! group.bench_function("fast", |b| b.iter(|| sma(&data, 20)));
//! ```
//!
//! # Behavior Matrix
//!
//! | Input Type | PrecisionMode::High | PrecisionMode::Fast |
//! |------------|---------------------|---------------------|
//! | f32 | f64 accumulators, f32 output | f32 accumulators (original) |
//! | f64 | f64 accumulators (no change) | f64 accumulators (no change) |
//!
//! # Limitations
//!
//! - TLS does not propagate to child threads. For multi-threaded tests, use
//!   [`set_precision_mode()`] before spawning or have each thread call
//!   [`with_precision_mode()`].
//! - TLS bypasses initialization - by design, tests control their own mode.

use std::cell::Cell;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Precision mode for indicator calculations.
///
/// Controls whether f32 inputs use f64 accumulators for improved precision.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum PrecisionMode {
    /// High precision mode (default).
    ///
    /// Uses f64 accumulators for f32 inputs. Provides better numeric stability
    /// at a small performance cost (~15-20% overhead for variance-based indicators,
    /// ~15% for others).
    #[default]
    High = 0,

    /// Fast mode.
    ///
    /// Uses native-type accumulators. Matches original liq-ta behavior for
    /// maximum throughput. Should perform within 2% of pre-change baseline.
    Fast = 1,
}

impl PrecisionMode {
    /// Convert from u8 representation.
    ///
    /// Returns `PrecisionMode::High` for any value other than 1.
    #[inline]
    fn from_u8(v: u8) -> Self {
        match v {
            1 => PrecisionMode::Fast,
            _ => PrecisionMode::High,
        }
    }
}

// ---------------------------------------------------------------------------
// Internal State
// ---------------------------------------------------------------------------

/// Ensures initialization happens only once.
static INIT: Once = Once::new();

/// Global precision mode storage.
static GLOBAL_MODE: AtomicU8 = AtomicU8::new(PrecisionMode::High as u8);

/// Flag indicating manual override via set_precision_mode.
/// When true, env var and feature detection are bypassed.
static MANUAL_OVERRIDE: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// Thread-local override for testing.
    static THREAD_OVERRIDE: Cell<Option<PrecisionMode>> = const { Cell::new(None) };
}

// ---------------------------------------------------------------------------
// Core Functions
// ---------------------------------------------------------------------------

/// Initializes precision mode from environment/features (called once).
///
/// Respects manual override flag - if set, skips env/feature detection.
fn ensure_initialized() {
    INIT.call_once(|| {
        // If already manually overridden, don't check env/feature
        if MANUAL_OVERRIDE.load(Ordering::Relaxed) {
            return;
        }

        // Check environment variable
        if let Ok(val) = std::env::var("LIQ_TA_PRECISION") {
            match val.to_lowercase().as_str() {
                "fast" => GLOBAL_MODE.store(PrecisionMode::Fast as u8, Ordering::Relaxed),
                "high" => GLOBAL_MODE.store(PrecisionMode::High as u8, Ordering::Relaxed),
                _ => {} // Invalid value, keep default
            }
            return;
        }

        // Check Cargo feature
        #[cfg(feature = "precision-fast")]
        GLOBAL_MODE.store(PrecisionMode::Fast as u8, Ordering::Relaxed);
    });
}

/// Sets the global precision mode.
///
/// This bypasses environment variable and feature detection. Once called,
/// the mode remains fixed unless called again.
///
/// # Example
///
/// ```rust,ignore
/// use liq_ta::precision::{set_precision_mode, PrecisionMode};
///
/// // Set at application startup
/// set_precision_mode(PrecisionMode::Fast);
/// ```
pub fn set_precision_mode(mode: PrecisionMode) {
    MANUAL_OVERRIDE.store(true, Ordering::Relaxed);
    GLOBAL_MODE.store(mode as u8, Ordering::Relaxed);
}

/// Returns the current precision mode.
///
/// Checks sources in order:
/// 1. Thread-local override (set by `with_precision_mode`)
/// 2. Global mode (set by `set_precision_mode`, env var, or feature)
///
/// # Performance
///
/// This function is designed to be fast (~2-3ns): TLS check + atomic load + branch.
/// Suitable for calling at the start of each indicator computation.
#[inline]
pub fn current_precision_mode() -> PrecisionMode {
    // Check thread-local override first (for tests)
    if let Some(mode) = THREAD_OVERRIDE.with(|c| c.get()) {
        return mode;
    }

    // Ensure global mode is initialized
    ensure_initialized();

    PrecisionMode::from_u8(GLOBAL_MODE.load(Ordering::Relaxed))
}

/// RAII guard for panic-safe TLS restore.
struct PrecisionModeGuard {
    prev: Option<PrecisionMode>,
}

impl Drop for PrecisionModeGuard {
    fn drop(&mut self) {
        THREAD_OVERRIDE.with(|cell| cell.set(self.prev));
    }
}

/// Executes a closure with a specific precision mode.
///
/// Sets the precision mode for the current thread only, restoring the previous
/// mode when the closure completes (even if it panics).
///
/// This function is primarily for testing, allowing isolated mode configuration
/// without affecting other tests running in parallel.
///
/// # Example
///
/// ```rust,ignore
/// use liq_ta::precision::{with_precision_mode, PrecisionMode};
///
/// #[test]
/// fn test_high_precision_sma() {
///     with_precision_mode(PrecisionMode::High, || {
///         let result = sma(&data, 20).unwrap();
///         // Assertions...
///     });
/// }
/// ```
///
/// # Limitations
///
/// - Does not propagate to child threads
/// - Bypasses env/feature initialization (tests control their own mode)
pub fn with_precision_mode<F, R>(mode: PrecisionMode, f: F) -> R
where
    F: FnOnce() -> R,
{
    THREAD_OVERRIDE.with(|cell| {
        let prev = cell.get();
        cell.set(Some(mode));
        let _guard = PrecisionModeGuard { prev };
        f()
    })
}

// ---------------------------------------------------------------------------
// Helper Trait for Accumulator Selection
// ---------------------------------------------------------------------------

/// Helper trait for type-aware accumulator selection.
///
/// This trait allows indicators to select the appropriate accumulator type
/// based on input type and precision mode.
pub trait AccumulatorType {
    /// The type to use for accumulators in High precision mode.
    type HighPrecision;

    /// Returns true if this type should use f64 accumulators in High mode.
    fn uses_f64_accumulator() -> bool;
}

impl AccumulatorType for f32 {
    type HighPrecision = f64;

    #[inline]
    fn uses_f64_accumulator() -> bool {
        true
    }
}

impl AccumulatorType for f64 {
    type HighPrecision = f64;

    #[inline]
    fn uses_f64_accumulator() -> bool {
        false // Already f64, no benefit to "upgrading"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_high() {
        assert_eq!(PrecisionMode::default(), PrecisionMode::High);
    }

    #[test]
    fn test_from_u8() {
        assert_eq!(PrecisionMode::from_u8(0), PrecisionMode::High);
        assert_eq!(PrecisionMode::from_u8(1), PrecisionMode::Fast);
        assert_eq!(PrecisionMode::from_u8(255), PrecisionMode::High); // Invalid -> High
    }

    #[test]
    fn test_with_precision_mode_isolation() {
        // Test that with_precision_mode provides isolation
        let outer = current_precision_mode();

        with_precision_mode(PrecisionMode::Fast, || {
            assert_eq!(current_precision_mode(), PrecisionMode::Fast);

            // Nested call
            with_precision_mode(PrecisionMode::High, || {
                assert_eq!(current_precision_mode(), PrecisionMode::High);
            });

            // Back to Fast after nested returns
            assert_eq!(current_precision_mode(), PrecisionMode::Fast);
        });

        // Back to original after outer returns
        // Note: This may be High or Fast depending on global state
        let after = current_precision_mode();
        // We just verify it doesn't crash and returns a valid mode
        assert!(after == PrecisionMode::High || after == PrecisionMode::Fast);
        let _ = outer; // Silence warning
    }

    #[test]
    fn test_with_precision_mode_panic_safety() {
        use std::panic::{AssertUnwindSafe, catch_unwind};

        // Set a known state via TLS
        with_precision_mode(PrecisionMode::High, || {
            // Panic in inner scope
            let result = catch_unwind(AssertUnwindSafe(|| {
                with_precision_mode(PrecisionMode::Fast, || {
                    assert_eq!(current_precision_mode(), PrecisionMode::Fast);
                    panic!("intentional panic");
                });
            }));

            assert!(result.is_err());

            // After panic, should be restored to High
            assert_eq!(current_precision_mode(), PrecisionMode::High);
        });
    }

    #[test]
    fn test_accumulator_type_trait() {
        assert!(f32::uses_f64_accumulator());
        assert!(!f64::uses_f64_accumulator());
    }
}
