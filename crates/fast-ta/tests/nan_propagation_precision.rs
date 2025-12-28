//! NaN/Infinity propagation verification for precision modes.
//!
//! This test suite verifies that f64 upcasting in High precision mode
//! doesn't alter NaN/Infinity propagation behavior compared to Fast mode.
//!
//! Tests cover:
//! - NaN propagation unchanged between modes
//! - Infinity → NaN behavior unchanged
//! - Edge cases (signed zeros, subnormals)

#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]

use fast_ta::indicators::{
    bollinger::bollinger, cci::cci, mfi::mfi, obv::obv, roc::{roc, rocp, rocr, rocr100},
    rsi::rsi, sma::sma, statistics::var, stochastic::stochastic, vwap::vwap,
    williams_r::williams_r,
};
use fast_ta::precision::{with_precision_mode, PrecisionMode};

const LEN: usize = 100;
const NAN_INDEX: usize = 25;

// =============================================================================
// Helper Functions
// =============================================================================

fn base_series_f32() -> Vec<f32> {
    (0..LEN).map(|i| 100.0 + i as f32).collect()
}

fn base_ohlcv_f32() -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut high = Vec::with_capacity(LEN);
    let mut low = Vec::with_capacity(LEN);
    let mut close = Vec::with_capacity(LEN);
    let mut volume = Vec::with_capacity(LEN);

    for i in 0..LEN {
        let value = 100.0 + i as f32;
        high.push(value + 1.0);
        low.push(value - 1.0);
        close.push(value);
        volume.push(1000.0 + i as f32 * 10.0);
    }

    (high, low, close, volume)
}

/// Check that NaN appears at the expected index in both modes
fn assert_nan_both_modes<F>(name: &str, compute: F)
where
    F: Fn() -> Vec<f32>,
{
    let fast_result = with_precision_mode(PrecisionMode::Fast, || compute());
    let high_result = with_precision_mode(PrecisionMode::High, || compute());

    assert!(
        fast_result[NAN_INDEX].is_nan(),
        "{} Fast mode: expected NaN at index {}, got {}",
        name,
        NAN_INDEX,
        fast_result[NAN_INDEX]
    );
    assert!(
        high_result[NAN_INDEX].is_nan(),
        "{} High mode: expected NaN at index {}, got {}",
        name,
        NAN_INDEX,
        high_result[NAN_INDEX]
    );
}

/// Check that NaN propagates identically in both modes
fn assert_nan_pattern_identical<F>(name: &str, compute: F)
where
    F: Fn() -> Vec<f32>,
{
    let fast_result = with_precision_mode(PrecisionMode::Fast, || compute());
    let high_result = with_precision_mode(PrecisionMode::High, || compute());

    assert_eq!(
        fast_result.len(),
        high_result.len(),
        "{}: length mismatch between modes",
        name
    );

    for i in 0..fast_result.len() {
        let fast_is_nan = fast_result[i].is_nan();
        let high_is_nan = high_result[i].is_nan();
        assert_eq!(
            fast_is_nan, high_is_nan,
            "{}: NaN pattern mismatch at index {} (fast={}, high={})",
            name, i, fast_is_nan, high_is_nan
        );
    }
}

// =============================================================================
// NaN Propagation Tests
// =============================================================================

#[test]
fn nan_propagation_sma() {
    let mut data = base_series_f32();
    data[NAN_INDEX] = f32::NAN;

    assert_nan_both_modes("SMA", || sma(&data, 5).unwrap());
    assert_nan_pattern_identical("SMA", || sma(&data, 5).unwrap());
}

#[test]
fn nan_propagation_rsi() {
    let mut data = base_series_f32();
    data[NAN_INDEX] = f32::NAN;

    assert_nan_both_modes("RSI", || rsi(&data, 14).unwrap());
    assert_nan_pattern_identical("RSI", || rsi(&data, 14).unwrap());
}

#[test]
fn nan_propagation_bollinger() {
    let mut data = base_series_f32();
    data[NAN_INDEX] = f32::NAN;

    // Check all three bands
    let check_middle = || bollinger(&data, 20, 2.0_f32).unwrap().middle;
    let check_upper = || bollinger(&data, 20, 2.0_f32).unwrap().upper;
    let check_lower = || bollinger(&data, 20, 2.0_f32).unwrap().lower;

    assert_nan_pattern_identical("Bollinger/middle", check_middle);
    assert_nan_pattern_identical("Bollinger/upper", check_upper);
    assert_nan_pattern_identical("Bollinger/lower", check_lower);
}

#[test]
fn nan_propagation_var() {
    let mut data = base_series_f32();
    data[NAN_INDEX] = f32::NAN;

    assert_nan_both_modes("VAR", || var(&data, 20).unwrap());
    assert_nan_pattern_identical("VAR", || var(&data, 20).unwrap());
}

#[test]
fn nan_propagation_stochastic() {
    let (mut high, mut low, mut close, _) = base_ohlcv_f32();
    high[NAN_INDEX] = f32::NAN;
    low[NAN_INDEX] = f32::NAN;
    close[NAN_INDEX] = f32::NAN;

    let check_k = || stochastic(&high, &low, &close, 14, 3, 1).unwrap().k;
    let check_d = || stochastic(&high, &low, &close, 14, 3, 1).unwrap().d;

    assert_nan_pattern_identical("Stochastic/%K", check_k);
    assert_nan_pattern_identical("Stochastic/%D", check_d);
}

#[test]
fn nan_propagation_williams_r() {
    let (mut high, mut low, mut close, _) = base_ohlcv_f32();
    high[NAN_INDEX] = f32::NAN;
    low[NAN_INDEX] = f32::NAN;
    close[NAN_INDEX] = f32::NAN;

    assert_nan_both_modes("Williams %R", || {
        williams_r(&high, &low, &close, 14).unwrap()
    });
    assert_nan_pattern_identical("Williams %R", || {
        williams_r(&high, &low, &close, 14).unwrap()
    });
}

#[test]
fn nan_propagation_roc_family() {
    let mut data = base_series_f32();
    data[NAN_INDEX] = f32::NAN;

    assert_nan_pattern_identical("ROC", || roc(&data, 10).unwrap());
    assert_nan_pattern_identical("ROCP", || rocp(&data, 10).unwrap());
    assert_nan_pattern_identical("ROCR", || rocr(&data, 10).unwrap());
    assert_nan_pattern_identical("ROCR100", || rocr100(&data, 10).unwrap());
}

#[test]
fn nan_propagation_vwap() {
    let (mut high, mut low, mut close, mut volume) = base_ohlcv_f32();
    high[NAN_INDEX] = f32::NAN;
    low[NAN_INDEX] = f32::NAN;
    close[NAN_INDEX] = f32::NAN;
    volume[NAN_INDEX] = f32::NAN;

    assert_nan_both_modes("VWAP", || vwap(&high, &low, &close, &volume).unwrap());
    assert_nan_pattern_identical("VWAP", || vwap(&high, &low, &close, &volume).unwrap());
}

#[test]
fn nan_propagation_obv() {
    let (_, _, mut close, mut volume) = base_ohlcv_f32();
    close[NAN_INDEX] = f32::NAN;
    volume[NAN_INDEX] = f32::NAN;

    assert_nan_both_modes("OBV", || obv(&close, &volume).unwrap());
    assert_nan_pattern_identical("OBV", || obv(&close, &volume).unwrap());
}

#[test]
fn nan_propagation_cci() {
    let (mut high, mut low, mut close, _) = base_ohlcv_f32();
    high[NAN_INDEX] = f32::NAN;
    low[NAN_INDEX] = f32::NAN;
    close[NAN_INDEX] = f32::NAN;

    assert_nan_both_modes("CCI", || cci(&high, &low, &close, 20).unwrap());
    assert_nan_pattern_identical("CCI", || cci(&high, &low, &close, 20).unwrap());
}

#[test]
fn nan_propagation_mfi() {
    let (mut high, mut low, mut close, mut volume) = base_ohlcv_f32();
    high[NAN_INDEX] = f32::NAN;
    low[NAN_INDEX] = f32::NAN;
    close[NAN_INDEX] = f32::NAN;
    volume[NAN_INDEX] = f32::NAN;

    assert_nan_both_modes("MFI", || {
        mfi(&high, &low, &close, &volume, 14).unwrap()
    });
    assert_nan_pattern_identical("MFI", || {
        mfi(&high, &low, &close, &volume, 14).unwrap()
    });
}

// =============================================================================
// Infinity Propagation Tests
// =============================================================================

#[test]
fn infinity_propagation_sma() {
    let mut data = base_series_f32();
    data[NAN_INDEX] = f32::INFINITY;

    // Infinity should eventually produce NaN or propagate through
    assert_nan_pattern_identical("SMA/infinity", || sma(&data, 5).unwrap());
}

#[test]
fn infinity_propagation_rsi() {
    let mut data = base_series_f32();
    data[NAN_INDEX] = f32::INFINITY;

    assert_nan_pattern_identical("RSI/infinity", || rsi(&data, 14).unwrap());
}

#[test]
fn infinity_propagation_bollinger() {
    let mut data = base_series_f32();
    data[NAN_INDEX] = f32::INFINITY;

    let check_middle = || bollinger(&data, 20, 2.0_f32).unwrap().middle;
    assert_nan_pattern_identical("Bollinger/infinity", check_middle);
}

#[test]
fn infinity_propagation_var() {
    let mut data = base_series_f32();
    data[NAN_INDEX] = f32::INFINITY;

    assert_nan_pattern_identical("VAR/infinity", || var(&data, 20).unwrap());
}

#[test]
fn infinity_propagation_vwap() {
    let (mut high, mut low, mut close, mut volume) = base_ohlcv_f32();
    high[NAN_INDEX] = f32::INFINITY;
    low[NAN_INDEX] = f32::INFINITY;
    close[NAN_INDEX] = f32::INFINITY;
    volume[NAN_INDEX] = f32::INFINITY;

    assert_nan_pattern_identical("VWAP/infinity", || {
        vwap(&high, &low, &close, &volume).unwrap()
    });
}

// =============================================================================
// Edge Cases: Signed Zeros
// =============================================================================

#[test]
fn signed_zero_propagation() {
    // Test that -0.0 and +0.0 behave identically in both modes
    let mut data_pos = base_series_f32();
    let mut data_neg = base_series_f32();
    data_pos[NAN_INDEX] = 0.0_f32;
    data_neg[NAN_INDEX] = -0.0_f32;

    // SMA should produce identical results for +0.0 and -0.0
    let sma_pos_fast =
        with_precision_mode(PrecisionMode::Fast, || sma(&data_pos, 5).unwrap());
    let sma_neg_fast =
        with_precision_mode(PrecisionMode::Fast, || sma(&data_neg, 5).unwrap());
    let sma_pos_high =
        with_precision_mode(PrecisionMode::High, || sma(&data_pos, 5).unwrap());
    let sma_neg_high =
        with_precision_mode(PrecisionMode::High, || sma(&data_neg, 5).unwrap());

    for i in 0..sma_pos_fast.len() {
        // Results should be equal (ignoring sign of zero)
        let diff_fast = (sma_pos_fast[i] - sma_neg_fast[i]).abs();
        let diff_high = (sma_pos_high[i] - sma_neg_high[i]).abs();
        assert!(
            diff_fast < 1e-10 || sma_pos_fast[i].is_nan(),
            "SMA Fast: +0/-0 difference at {} (fast)",
            i
        );
        assert!(
            diff_high < 1e-10 || sma_pos_high[i].is_nan(),
            "SMA High: +0/-0 difference at {} (high)",
            i
        );
    }
}

// =============================================================================
// Edge Cases: Subnormal Numbers
// =============================================================================

#[test]
fn subnormal_handling() {
    // Test that subnormal (denormalized) f32 numbers work correctly
    let mut data = base_series_f32();
    data[NAN_INDEX] = f32::MIN_POSITIVE / 2.0; // Subnormal number

    // Should not produce NaN or Infinity
    let sma_fast = with_precision_mode(PrecisionMode::Fast, || sma(&data, 5).unwrap());
    let sma_high = with_precision_mode(PrecisionMode::High, || sma(&data, 5).unwrap());

    // The subnormal should not cause NaN at the position
    assert!(
        !sma_fast[NAN_INDEX].is_nan(),
        "SMA Fast: subnormal caused NaN"
    );
    assert!(
        !sma_high[NAN_INDEX].is_nan(),
        "SMA High: subnormal caused NaN"
    );
    assert!(
        !sma_fast[NAN_INDEX].is_infinite(),
        "SMA Fast: subnormal caused Infinity"
    );
    assert!(
        !sma_high[NAN_INDEX].is_infinite(),
        "SMA High: subnormal caused Infinity"
    );
}

// =============================================================================
// Edge Cases: Very Large Numbers
// =============================================================================

#[test]
fn large_number_handling() {
    // Test behavior with large (but not infinite) numbers
    let mut data = base_series_f32();
    data[NAN_INDEX] = f32::MAX / 2.0;

    let sma_fast = with_precision_mode(PrecisionMode::Fast, || sma(&data, 5).unwrap());
    let sma_high = with_precision_mode(PrecisionMode::High, || sma(&data, 5).unwrap());

    // High mode with f64 accumulators should handle large sums better
    // At minimum, patterns should be consistent
    for i in 0..sma_fast.len() {
        let fast_nan = sma_fast[i].is_nan();
        let high_nan = sma_high[i].is_nan();
        let fast_inf = sma_fast[i].is_infinite();
        let _high_inf = sma_high[i].is_infinite();

        // Note: High mode may actually prevent overflow that Fast mode has
        // So we just check that High mode is at least as good as Fast mode
        if !fast_nan && !fast_inf {
            assert!(
                !high_nan,
                "High mode produced NaN where Fast mode didn't at index {}",
                i
            );
        }
    }
}

// =============================================================================
// Multiple NaN Tests
// =============================================================================

#[test]
fn multiple_nan_propagation() {
    // Test with multiple NaN values in the data
    let mut data = base_series_f32();
    data[10] = f32::NAN;
    data[20] = f32::NAN;
    data[30] = f32::NAN;

    let sma_fast = with_precision_mode(PrecisionMode::Fast, || sma(&data, 5).unwrap());
    let sma_high = with_precision_mode(PrecisionMode::High, || sma(&data, 5).unwrap());

    // NaN pattern should be identical
    for i in 0..sma_fast.len() {
        assert_eq!(
            sma_fast[i].is_nan(),
            sma_high[i].is_nan(),
            "Multiple NaN: pattern mismatch at index {}",
            i
        );
    }
}

// =============================================================================
// Consecutive NaN Tests
// =============================================================================

#[test]
fn consecutive_nan_propagation() {
    // Test with consecutive NaN values
    let mut data = base_series_f32();
    for i in 20..25 {
        data[i] = f32::NAN;
    }

    let rsi_fast = with_precision_mode(PrecisionMode::Fast, || rsi(&data, 14).unwrap());
    let rsi_high = with_precision_mode(PrecisionMode::High, || rsi(&data, 14).unwrap());

    // NaN pattern should be identical
    for i in 0..rsi_fast.len() {
        assert_eq!(
            rsi_fast[i].is_nan(),
            rsi_high[i].is_nan(),
            "Consecutive NaN: RSI pattern mismatch at index {}",
            i
        );
    }
}

// =============================================================================
// NaN at Start Tests
// =============================================================================

#[test]
fn nan_at_start_propagation() {
    // Test NaN at the very beginning
    let mut data = base_series_f32();
    data[0] = f32::NAN;

    let sma_fast = with_precision_mode(PrecisionMode::Fast, || sma(&data, 5).unwrap());
    let sma_high = with_precision_mode(PrecisionMode::High, || sma(&data, 5).unwrap());

    // First several values should be NaN
    assert!(sma_fast[0].is_nan(), "SMA Fast: NaN at start should propagate");
    assert!(sma_high[0].is_nan(), "SMA High: NaN at start should propagate");

    // Pattern should match
    for i in 0..sma_fast.len() {
        assert_eq!(
            sma_fast[i].is_nan(),
            sma_high[i].is_nan(),
            "NaN at start: pattern mismatch at index {}",
            i
        );
    }
}

// =============================================================================
// NaN at End Tests
// =============================================================================

#[test]
fn nan_at_end_propagation() {
    // Test NaN at the very end
    let mut data = base_series_f32();
    data[LEN - 1] = f32::NAN;

    let sma_fast = with_precision_mode(PrecisionMode::Fast, || sma(&data, 5).unwrap());
    let sma_high = with_precision_mode(PrecisionMode::High, || sma(&data, 5).unwrap());

    // Last value should be NaN
    assert!(
        sma_fast[LEN - 1].is_nan(),
        "SMA Fast: NaN at end should propagate"
    );
    assert!(
        sma_high[LEN - 1].is_nan(),
        "SMA High: NaN at end should propagate"
    );

    // Pattern should match
    for i in 0..sma_fast.len() {
        assert_eq!(
            sma_fast[i].is_nan(),
            sma_high[i].is_nan(),
            "NaN at end: pattern mismatch at index {}",
            i
        );
    }
}
