//! Golden Reference Tests for Precision Validation
//!
//! This module provides regression testing against captured golden reference values
//! and precision validation for High mode against pure f64 reference.
//!
//! # Test Types
//!
//! | Test Type | Mode | Input Type | Baseline Source | Tolerance |
//! |-----------|------|------------|-----------------|-----------|
//! | Regression | Fast | f32 | golden/fast_f32/*.json | 1e-6 rel |
//! | Regression | Fast | f64 | golden/fast_f64/*.json | 1e-15 rel |
//! | Precision  | High | f32 | Pure f64 computation | Per-indicator |
//! | Precision  | High | f64 | Same as Fast/f64 | 1e-15 rel |
//!
//! # Golden File Regeneration
//!
//! To regenerate golden files (run only when algorithm intentionally changes):
//! ```sh
//! REGENERATE_GOLDEN=1 cargo test golden_reference
//! ```
//!
//! # Per-Indicator Tolerances (from numeric-policy-plan.md)
//!
//! | Indicator | Relative Tol | Absolute Tol | Rule |
//! |-----------|--------------|--------------|------|
//! | RSI, Stochastic, Williams %R, MFI | - | 0.01 | abs |
//! | SMA, Bollinger | 1e-5 | 1e-7 | hybrid |
//! | VWAP | 1e-5 | 1e-7 | hybrid |
//! | OBV, AD | 1e-4 | 1.0 | hybrid |

#![allow(clippy::float_cmp)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::type_complexity)]

use liq_ta::indicators::{
    bollinger::bollinger, obv::obv, rsi::rsi, sma::sma, stochastic::stochastic, vwap::vwap,
};
use liq_ta::precision::{PrecisionMode, with_precision_mode};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

// =============================================================================
// Tolerance Helpers (per numeric-policy-plan.md Error Tolerance Specification)
// =============================================================================

/// For bounded indicators (RSI, Stochastic, Williams %R, MFI) - abs rule
fn within_abs(actual: f64, expected: f64, abs_tol: f64) -> bool {
    if actual.is_nan() && expected.is_nan() {
        return true;
    }
    if actual.is_nan() || expected.is_nan() {
        return false;
    }
    (actual - expected).abs() <= abs_tol
}

/// For unbounded/price-scale indicators - hybrid rule
fn within_hybrid(actual: f64, expected: f64, rel_tol: f64, abs_tol: f64) -> bool {
    if actual.is_nan() && expected.is_nan() {
        return true;
    }
    if actual.is_nan() || expected.is_nan() {
        return false;
    }
    let diff = (actual - expected).abs();
    diff <= abs_tol || diff <= rel_tol * expected.abs()
}

// =============================================================================
// Golden File Infrastructure
// =============================================================================

#[derive(Serialize, Deserialize)]
struct GoldenData {
    indicator: String,
    input_type: String,
    /// Values stored as Option<f64> to handle NaN (stored as null in JSON)
    values: Vec<Option<f64>>,
}

fn golden_file_path(indicator: &str, input_type: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!(
        "{}/tests/golden/fast_{}/{}.json",
        manifest_dir, input_type, indicator
    )
}

fn should_regenerate() -> bool {
    std::env::var("REGENERATE_GOLDEN").is_ok_and(|v| v == "1" || v.to_lowercase() == "true")
}

fn save_golden(indicator: &str, input_type: &str, values: &[f64]) {
    let path = golden_file_path(indicator, input_type);
    // Convert f64 to Option<f64>, where NaN becomes None (serializes as null)
    let opt_values: Vec<Option<f64>> = values
        .iter()
        .map(|&v| if v.is_nan() { None } else { Some(v) })
        .collect();
    let data = GoldenData {
        indicator: indicator.to_string(),
        input_type: input_type.to_string(),
        values: opt_values,
    };
    let json = serde_json::to_string_pretty(&data).expect("Failed to serialize golden data");
    fs::write(&path, json).unwrap_or_else(|e| panic!("Failed to write {path}: {e}"));
    eprintln!("[GOLDEN] Regenerated {path}");
}

fn load_golden(indicator: &str, input_type: &str) -> Option<Vec<f64>> {
    let path = golden_file_path(indicator, input_type);
    if !Path::new(&path).exists() {
        return None;
    }
    let json = fs::read_to_string(&path).ok()?;
    let data: GoldenData = serde_json::from_str(&json).ok()?;
    // Convert Option<f64> back to f64, where None becomes NaN
    let values: Vec<f64> = data.values.iter().map(|&v| v.unwrap_or(f64::NAN)).collect();
    Some(values)
}

// =============================================================================
// Test Data Generation (deterministic)
// =============================================================================

/// Generate deterministic price series for testing.
fn generate_test_prices_f64(size: usize) -> Vec<f64> {
    let mut data = Vec::with_capacity(size);
    let mut price = 100.0_f64;
    for i in 0..size {
        // Deterministic price movement
        let delta = ((i as f64 * 0.1).sin() * 2.0) + ((i as f64 * 0.03).cos() * 1.5);
        price += delta;
        price = price.max(10.0);
        data.push(price);
    }
    data
}

fn generate_test_prices_f32(size: usize) -> Vec<f32> {
    generate_test_prices_f64(size)
        .iter()
        .map(|&x| x as f32)
        .collect()
}

/// Generate deterministic OHLCV data for testing.
fn generate_test_ohlcv_f64(size: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(size);
    let mut low = Vec::with_capacity(size);
    let mut close = Vec::with_capacity(size);
    let mut open = Vec::with_capacity(size);
    let mut volume = Vec::with_capacity(size);

    let mut price = 100.0_f64;
    for i in 0..size {
        let delta = ((i as f64 * 0.1).sin() * 2.0) + ((i as f64 * 0.03).cos() * 1.5);
        price += delta;
        price = price.max(10.0);

        let h = price + 1.0 + (i as f64 * 0.07).sin().abs();
        let l = price - 1.0 - (i as f64 * 0.05).cos().abs();
        let c = price + ((i as f64 * 0.02).tan() * 0.5).clamp(-0.8, 0.8);
        let o = price + ((i as f64 * 0.04).sin() * 0.3);
        let v = 1_000_000.0 + (i as f64 * 1000.0).sin() * 500_000.0;

        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);
        volume.push(v.abs());
    }

    (open, high, low, close, volume)
}

fn generate_test_ohlcv_f32(size: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let (o, h, l, c, v) = generate_test_ohlcv_f64(size);
    (
        o.iter().map(|&x| x as f32).collect(),
        h.iter().map(|&x| x as f32).collect(),
        l.iter().map(|&x| x as f32).collect(),
        c.iter().map(|&x| x as f32).collect(),
        v.iter().map(|&x| x as f32).collect(),
    )
}

// =============================================================================
// Comparison Helpers
// =============================================================================

/// Compare arrays and report results
fn compare_arrays(
    indicator: &str,
    actual: &[f64],
    expected: &[f64],
    tolerance_fn: impl Fn(f64, f64) -> bool,
) -> bool {
    if actual.len() != expected.len() {
        eprintln!(
            "[FAIL] {}: length mismatch (actual={}, expected={})",
            indicator,
            actual.len(),
            expected.len()
        );
        return false;
    }

    let mut max_diff = 0.0_f64;
    let mut max_diff_idx = 0;
    let mut mismatches = 0;

    for (i, (&a, &e)) in actual.iter().zip(expected.iter()).enumerate() {
        if !tolerance_fn(a, e) {
            mismatches += 1;
            let diff = if a.is_nan() || e.is_nan() {
                f64::NAN
            } else {
                (a - e).abs()
            };
            if diff.is_nan() || diff > max_diff {
                max_diff = diff;
                max_diff_idx = i;
            }
            if mismatches <= 3 {
                eprintln!(
                    "[FAIL] {}: index {} - actual={}, expected={}, diff={}",
                    indicator, i, a, e, diff
                );
            }
        }
    }

    if mismatches > 0 {
        eprintln!(
            "[FAIL] {}: {} mismatches, max diff={} at index {}",
            indicator, mismatches, max_diff, max_diff_idx
        );
        false
    } else {
        eprintln!("[OK] {}: all values within tolerance", indicator);
        true
    }
}

// =============================================================================
// SMA Golden Tests
// =============================================================================

const TEST_SIZE: usize = 1000;

#[test]
fn golden_sma_f32_fast_mode() {
    let data = generate_test_prices_f32(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result = sma(&data, 20).expect("SMA computation failed");
        let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

        if should_regenerate() {
            save_golden("sma", "f32", &result_f64);
            return;
        }

        if let Some(golden) = load_golden("sma", "f32") {
            let passed = compare_arrays("SMA/f32/fast", &result_f64, &golden, |a, e| {
                within_hybrid(a, e, 1e-6, 1e-7)
            });
            assert!(passed, "SMA f32 golden test failed");
        } else {
            eprintln!("[SKIP] SMA/f32/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

#[test]
fn golden_sma_f64_fast_mode() {
    let data = generate_test_prices_f64(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result = sma(&data, 20).expect("SMA computation failed");

        if should_regenerate() {
            save_golden("sma", "f64", &result);
            return;
        }

        if let Some(golden) = load_golden("sma", "f64") {
            let passed = compare_arrays("SMA/f64/fast", &result, &golden, |a, e| {
                within_hybrid(a, e, 1e-15, 1e-17)
            });
            assert!(passed, "SMA f64 golden test failed");
        } else {
            eprintln!("[SKIP] SMA/f64/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

/// Precision test - validates High mode uses f64 accumulators.
/// Compares f32 High mode against f64 reference to verify numeric stability.
#[test]
fn precision_sma_f32_high_vs_f64_reference() {
    let data_f32 = generate_test_prices_f32(TEST_SIZE);
    let data_f64 = generate_test_prices_f64(TEST_SIZE);

    // Compute f64 reference (Fast mode since f64 uses f64 accumulators regardless)
    let reference = with_precision_mode(PrecisionMode::Fast, || {
        sma(&data_f64, 20).expect("SMA f64 computation failed")
    });

    // Compute f32 with High precision
    let result = with_precision_mode(PrecisionMode::High, || {
        sma(&data_f32, 20).expect("SMA f32 computation failed")
    });
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    // Per Error Tolerance Specification: SMA uses hybrid, 1e-5 rel, 1e-7 abs
    let passed = compare_arrays("SMA/f32/high_vs_f64", &result_f64, &reference, |a, e| {
        within_hybrid(a, e, 1e-5, 1e-7)
    });
    assert!(passed, "SMA f32 High mode precision test failed");
}

// =============================================================================
// RSI Golden Tests
// =============================================================================

#[test]
fn golden_rsi_f32_fast_mode() {
    let data = generate_test_prices_f32(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result = rsi(&data, 14).expect("RSI computation failed");
        let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

        if should_regenerate() {
            save_golden("rsi", "f32", &result_f64);
            return;
        }

        if let Some(golden) = load_golden("rsi", "f32") {
            let passed = compare_arrays("RSI/f32/fast", &result_f64, &golden, |a, e| {
                within_hybrid(a, e, 1e-6, 1e-7)
            });
            assert!(passed, "RSI f32 golden test failed");
        } else {
            eprintln!("[SKIP] RSI/f32/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

#[test]
fn golden_rsi_f64_fast_mode() {
    let data = generate_test_prices_f64(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result = rsi(&data, 14).expect("RSI computation failed");

        if should_regenerate() {
            save_golden("rsi", "f64", &result);
            return;
        }

        if let Some(golden) = load_golden("rsi", "f64") {
            let passed = compare_arrays("RSI/f64/fast", &result, &golden, |a, e| {
                within_hybrid(a, e, 1e-15, 1e-17)
            });
            assert!(passed, "RSI f64 golden test failed");
        } else {
            eprintln!("[SKIP] RSI/f64/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

/// Precision test - validates High mode uses f64 accumulators.
/// This test is ignored until Stage 2 implements f64 accumulators.
#[test]
fn precision_rsi_f32_high_vs_f64_reference() {
    let data_f32 = generate_test_prices_f32(TEST_SIZE);
    let data_f64 = generate_test_prices_f64(TEST_SIZE);

    let reference = with_precision_mode(PrecisionMode::Fast, || {
        rsi(&data_f64, 14).expect("RSI f64 computation failed")
    });

    let result = with_precision_mode(PrecisionMode::High, || {
        rsi(&data_f32, 14).expect("RSI f32 computation failed")
    });
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    // Per Error Tolerance Specification: RSI uses abs, 0.01
    let passed = compare_arrays("RSI/f32/high_vs_f64", &result_f64, &reference, |a, e| {
        within_abs(a, e, 0.01)
    });
    assert!(passed, "RSI f32 High mode precision test failed");
}

// =============================================================================
// Bollinger Golden Tests
// =============================================================================

#[test]
fn golden_bollinger_f32_fast_mode() {
    let data = generate_test_prices_f32(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result = bollinger(&data, 20, 2.0_f32).expect("Bollinger computation failed");
        // Store middle band for golden comparison
        let middle_f64: Vec<f64> = result.middle.iter().map(|&x| x as f64).collect();

        if should_regenerate() {
            save_golden("bollinger_middle", "f32", &middle_f64);
            return;
        }

        if let Some(golden) = load_golden("bollinger_middle", "f32") {
            let passed = compare_arrays("Bollinger/f32/fast", &middle_f64, &golden, |a, e| {
                within_hybrid(a, e, 1e-6, 1e-7)
            });
            assert!(passed, "Bollinger f32 golden test failed");
        } else {
            eprintln!("[SKIP] Bollinger/f32/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

#[test]
fn golden_bollinger_f64_fast_mode() {
    let data = generate_test_prices_f64(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result = bollinger(&data, 20, 2.0_f64).expect("Bollinger computation failed");
        let middle = result.middle;

        if should_regenerate() {
            save_golden("bollinger_middle", "f64", &middle);
            return;
        }

        if let Some(golden) = load_golden("bollinger_middle", "f64") {
            let passed = compare_arrays("Bollinger/f64/fast", &middle, &golden, |a, e| {
                within_hybrid(a, e, 1e-15, 1e-17)
            });
            assert!(passed, "Bollinger f64 golden test failed");
        } else {
            eprintln!("[SKIP] Bollinger/f64/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

/// Precision test - validates High mode uses f64 accumulators.
/// Compares f32 High mode against f64 reference to verify numeric stability.
#[test]
fn precision_bollinger_f32_high_vs_f64_reference() {
    let data_f32 = generate_test_prices_f32(TEST_SIZE);
    let data_f64 = generate_test_prices_f64(TEST_SIZE);

    let reference = with_precision_mode(PrecisionMode::Fast, || {
        bollinger(&data_f64, 20, 2.0_f64).expect("Bollinger f64 computation failed")
    });

    let result = with_precision_mode(PrecisionMode::High, || {
        bollinger(&data_f32, 20, 2.0_f32).expect("Bollinger f32 computation failed")
    });

    // Check middle band
    let result_middle: Vec<f64> = result.middle.iter().map(|&x| x as f64).collect();
    let passed = compare_arrays(
        "Bollinger/f32/high_vs_f64",
        &result_middle,
        &reference.middle,
        |a, e| within_hybrid(a, e, 1e-5, 1e-7),
    );
    assert!(passed, "Bollinger f32 High mode precision test failed");
}

// =============================================================================
// Stochastic Golden Tests
// =============================================================================

#[test]
fn golden_stochastic_f32_fast_mode() {
    let (_, high, low, close, _) = generate_test_ohlcv_f32(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result =
            stochastic(&high, &low, &close, 14, 3, 3).expect("Stochastic computation failed");
        let k_f64: Vec<f64> = result.k.iter().map(|&x| x as f64).collect();

        if should_regenerate() {
            save_golden("stochastic_k", "f32", &k_f64);
            return;
        }

        if let Some(golden) = load_golden("stochastic_k", "f32") {
            let passed = compare_arrays("Stochastic/f32/fast", &k_f64, &golden, |a, e| {
                within_hybrid(a, e, 1e-6, 1e-7)
            });
            assert!(passed, "Stochastic f32 golden test failed");
        } else {
            eprintln!("[SKIP] Stochastic/f32/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

#[test]
fn golden_stochastic_f64_fast_mode() {
    let (_, high, low, close, _) = generate_test_ohlcv_f64(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result =
            stochastic(&high, &low, &close, 14, 3, 3).expect("Stochastic computation failed");
        let k = result.k;

        if should_regenerate() {
            save_golden("stochastic_k", "f64", &k);
            return;
        }

        if let Some(golden) = load_golden("stochastic_k", "f64") {
            let passed = compare_arrays("Stochastic/f64/fast", &k, &golden, |a, e| {
                within_hybrid(a, e, 1e-15, 1e-17)
            });
            assert!(passed, "Stochastic f64 golden test failed");
        } else {
            eprintln!("[SKIP] Stochastic/f64/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

/// Precision test - validates High mode uses f64 accumulators.
/// Compares f32 High mode against f64 reference to verify numeric stability.
#[test]
fn precision_stochastic_f32_high_vs_f64_reference() {
    let (_, high_f32, low_f32, close_f32, _) = generate_test_ohlcv_f32(TEST_SIZE);
    let (_, high_f64, low_f64, close_f64, _) = generate_test_ohlcv_f64(TEST_SIZE);

    let reference = with_precision_mode(PrecisionMode::Fast, || {
        stochastic(&high_f64, &low_f64, &close_f64, 14, 3, 3)
            .expect("Stochastic f64 computation failed")
    });

    let result = with_precision_mode(PrecisionMode::High, || {
        stochastic(&high_f32, &low_f32, &close_f32, 14, 3, 3)
            .expect("Stochastic f32 computation failed")
    });

    let result_k: Vec<f64> = result.k.iter().map(|&x| x as f64).collect();
    // Per Error Tolerance Specification: Stochastic uses abs, 0.01
    let passed = compare_arrays(
        "Stochastic/f32/high_vs_f64",
        &result_k,
        &reference.k,
        |a, e| within_abs(a, e, 0.01),
    );
    assert!(passed, "Stochastic f32 High mode precision test failed");
}

// =============================================================================
// VWAP Golden Tests
// =============================================================================

#[test]
fn golden_vwap_f32_fast_mode() {
    let (_, high, low, close, volume) = generate_test_ohlcv_f32(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result = vwap(&high, &low, &close, &volume).expect("VWAP computation failed");
        let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

        if should_regenerate() {
            save_golden("vwap", "f32", &result_f64);
            return;
        }

        if let Some(golden) = load_golden("vwap", "f32") {
            let passed = compare_arrays("VWAP/f32/fast", &result_f64, &golden, |a, e| {
                within_hybrid(a, e, 1e-6, 1e-7)
            });
            assert!(passed, "VWAP f32 golden test failed");
        } else {
            eprintln!("[SKIP] VWAP/f32/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

#[test]
fn golden_vwap_f64_fast_mode() {
    let (_, high, low, close, volume) = generate_test_ohlcv_f64(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result = vwap(&high, &low, &close, &volume).expect("VWAP computation failed");

        if should_regenerate() {
            save_golden("vwap", "f64", &result);
            return;
        }

        if let Some(golden) = load_golden("vwap", "f64") {
            let passed = compare_arrays("VWAP/f64/fast", &result, &golden, |a, e| {
                within_hybrid(a, e, 1e-15, 1e-17)
            });
            assert!(passed, "VWAP f64 golden test failed");
        } else {
            eprintln!("[SKIP] VWAP/f64/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

/// Precision test - validates High mode uses f64 accumulators.
/// This test is ignored until Stage 2 implements f64 accumulators.
#[test]
fn precision_vwap_f32_high_vs_f64_reference() {
    let (_, high_f32, low_f32, close_f32, volume_f32) = generate_test_ohlcv_f32(TEST_SIZE);
    let (_, high_f64, low_f64, close_f64, volume_f64) = generate_test_ohlcv_f64(TEST_SIZE);

    let reference = with_precision_mode(PrecisionMode::Fast, || {
        vwap(&high_f64, &low_f64, &close_f64, &volume_f64).expect("VWAP f64 computation failed")
    });

    let result = with_precision_mode(PrecisionMode::High, || {
        vwap(&high_f32, &low_f32, &close_f32, &volume_f32).expect("VWAP f32 computation failed")
    });
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    // Per Error Tolerance Specification: VWAP uses hybrid, 1e-5 rel, 1e-7 abs
    let passed = compare_arrays("VWAP/f32/high_vs_f64", &result_f64, &reference, |a, e| {
        within_hybrid(a, e, 1e-5, 1e-7)
    });
    assert!(passed, "VWAP f32 High mode precision test failed");
}

// =============================================================================
// OBV Golden Tests
// =============================================================================

#[test]
fn golden_obv_f32_fast_mode() {
    let (_, _, _, close, volume) = generate_test_ohlcv_f32(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result = obv(&close, &volume).expect("OBV computation failed");
        let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

        if should_regenerate() {
            save_golden("obv", "f32", &result_f64);
            return;
        }

        if let Some(golden) = load_golden("obv", "f32") {
            let passed = compare_arrays("OBV/f32/fast", &result_f64, &golden, |a, e| {
                within_hybrid(a, e, 1e-6, 1e-7)
            });
            assert!(passed, "OBV f32 golden test failed");
        } else {
            eprintln!("[SKIP] OBV/f32/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

#[test]
fn golden_obv_f64_fast_mode() {
    let (_, _, _, close, volume) = generate_test_ohlcv_f64(TEST_SIZE);

    with_precision_mode(PrecisionMode::Fast, || {
        let result = obv(&close, &volume).expect("OBV computation failed");

        if should_regenerate() {
            save_golden("obv", "f64", &result);
            return;
        }

        if let Some(golden) = load_golden("obv", "f64") {
            let passed = compare_arrays("OBV/f64/fast", &result, &golden, |a, e| {
                within_hybrid(a, e, 1e-15, 1e-17)
            });
            assert!(passed, "OBV f64 golden test failed");
        } else {
            eprintln!("[SKIP] OBV/f64/fast: no golden file (run with REGENERATE_GOLDEN=1)");
        }
    });
}

/// Precision test - validates High mode uses f64 accumulators.
/// This test is ignored until Stage 2 implements f64 accumulators.
#[test]
fn precision_obv_f32_high_vs_f64_reference() {
    let (_, _, _, close_f32, volume_f32) = generate_test_ohlcv_f32(TEST_SIZE);
    let (_, _, _, close_f64, volume_f64) = generate_test_ohlcv_f64(TEST_SIZE);

    let reference = with_precision_mode(PrecisionMode::Fast, || {
        obv(&close_f64, &volume_f64).expect("OBV f64 computation failed")
    });

    let result = with_precision_mode(PrecisionMode::High, || {
        obv(&close_f32, &volume_f32).expect("OBV f32 computation failed")
    });
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    // Per Error Tolerance Specification: OBV uses hybrid, 1e-4 rel, 1.0 abs
    let passed = compare_arrays("OBV/f32/high_vs_f64", &result_f64, &reference, |a, e| {
        within_hybrid(a, e, 1e-4, 1.0)
    });
    assert!(passed, "OBV f32 High mode precision test failed");
}
