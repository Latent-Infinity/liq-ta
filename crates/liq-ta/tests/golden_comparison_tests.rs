//! Golden comparison tests - TA-Lib reference comparisons (informational, non-blocking).
//!
//! These tests compare liq-ta outputs to TA-Lib reference values. Per PRD §7.4,
//! these are informational only and do not block releases. Discrepancies are logged
//! as warnings rather than failures.
//!
//! Note: Differences may occur due to:
//! - Different initialization methods (e.g., SMA seed vs first value)
//! - Floating-point precision variations
//! - Implementation-specific choices within specification boundaries

#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]

use liq_ta::indicators::{ema::ema, rsi::rsi, sma::sma};

const EPSILON: f64 = 1e-10;
const LOOSE_EPSILON: f64 = 1e-6;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    (a - b).abs() < eps
}

/// Compare results and log discrepancies (non-blocking)
fn compare_results(
    indicator: &str,
    test_name: &str,
    actual: &[f64],
    expected: &[Option<f64>],
    tolerance: f64,
) -> usize {
    let mut discrepancies = 0;

    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        match e {
            None => {
                if !a.is_nan() {
                    eprintln!("[WARN] {indicator}/{test_name}: index {i} expected NaN, got {a}");
                    discrepancies += 1;
                }
            }
            Some(exp) => {
                if !approx_eq(*a, *exp, tolerance) {
                    eprintln!(
                        "[WARN] {}/{}: index {} expected {}, got {} (diff: {})",
                        indicator,
                        test_name,
                        i,
                        exp,
                        a,
                        (a - exp).abs()
                    );
                    discrepancies += 1;
                }
            }
        }
    }

    if discrepancies == 0 {
        eprintln!("[OK] {indicator}/{test_name}: all values match within tolerance");
    }

    discrepancies
}

// ==================== SMA Golden Tests ====================

#[test]
fn golden_sma_simple_ascending() {
    let input: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let expected: Vec<Option<f64>> = vec![
        None,
        None,
        Some(2.0),
        Some(3.0),
        Some(4.0),
        Some(5.0),
        Some(6.0),
        Some(7.0),
        Some(8.0),
        Some(9.0),
    ];

    let result = sma(&input, 3).unwrap();
    let discrepancies = compare_results("SMA", "simple_ascending", &result, &expected, EPSILON);

    // Non-blocking: log but don't fail
    if discrepancies > 0 {
        eprintln!("[INFO] SMA/simple_ascending: {discrepancies} discrepancies (informational)");
    }
}

#[test]
fn golden_sma_constant_values() {
    let input: Vec<f64> = vec![50.0, 50.0, 50.0, 50.0, 50.0];
    let expected: Vec<Option<f64>> = vec![None, None, Some(50.0), Some(50.0), Some(50.0)];

    let result = sma(&input, 3).unwrap();
    let discrepancies = compare_results("SMA", "constant_values", &result, &expected, EPSILON);

    if discrepancies > 0 {
        eprintln!("[INFO] SMA/constant_values: {discrepancies} discrepancies (informational)");
    }
}

#[test]
fn golden_sma_period_5() {
    let input: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0];
    let expected: Vec<Option<f64>> =
        vec![None, None, None, None, Some(30.0), Some(40.0), Some(50.0)];

    let result = sma(&input, 5).unwrap();
    let discrepancies = compare_results("SMA", "period_5", &result, &expected, EPSILON);

    if discrepancies > 0 {
        eprintln!("[INFO] SMA/period_5: {discrepancies} discrepancies (informational)");
    }
}

// ==================== EMA Golden Tests ====================

#[test]
fn golden_ema_constant_values() {
    let input: Vec<f64> = vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0];
    let expected: Vec<Option<f64>> = vec![
        None,
        None,
        Some(100.0),
        Some(100.0),
        Some(100.0),
        Some(100.0),
    ];

    let result = ema(&input, 3).unwrap();
    let discrepancies = compare_results("EMA", "constant_values", &result, &expected, EPSILON);

    if discrepancies > 0 {
        eprintln!("[INFO] EMA/constant_values: {discrepancies} discrepancies (informational)");
    }
}

#[test]
fn golden_ema_ascending_period_3() {
    // SMA seed = (1+2+3)/3 = 2.0, alpha = 2/(3+1) = 0.5
    // EMA[2] = 2.0 (SMA seed)
    // EMA[3] = 0.5 * 4.0 + 0.5 * 2.0 = 3.0
    // EMA[4] = 0.5 * 5.0 + 0.5 * 3.0 = 4.0
    let input: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let expected: Vec<Option<f64>> = vec![None, None, Some(2.0), Some(3.0), Some(4.0)];

    let result = ema(&input, 3).unwrap();
    let discrepancies = compare_results("EMA", "ascending_period_3", &result, &expected, EPSILON);

    if discrepancies > 0 {
        eprintln!("[INFO] EMA/ascending_period_3: {discrepancies} discrepancies (informational)");
    }
}

// ==================== RSI Golden Tests ====================

#[test]
fn golden_rsi_all_gains() {
    let input: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    let expected: Vec<Option<f64>> = vec![
        None,
        None,
        None,
        Some(100.0),
        Some(100.0),
        Some(100.0),
        Some(100.0),
    ];

    let result = rsi(&input, 3).unwrap();
    let discrepancies = compare_results("RSI", "all_gains", &result, &expected, LOOSE_EPSILON);

    if discrepancies > 0 {
        eprintln!("[INFO] RSI/all_gains: {discrepancies} discrepancies (informational)");
    }
}

#[test]
fn golden_rsi_all_losses() {
    let input: Vec<f64> = vec![20.0, 19.0, 18.0, 17.0, 16.0, 15.0, 14.0];
    let expected: Vec<Option<f64>> =
        vec![None, None, None, Some(0.0), Some(0.0), Some(0.0), Some(0.0)];

    let result = rsi(&input, 3).unwrap();
    let discrepancies = compare_results("RSI", "all_losses", &result, &expected, LOOSE_EPSILON);

    if discrepancies > 0 {
        eprintln!("[INFO] RSI/all_losses: {discrepancies} discrepancies (informational)");
    }
}

#[test]
fn golden_rsi_mixed_movement() {
    // For mixed movement, just verify RSI is in valid range
    let input: Vec<f64> = vec![50.0, 51.0, 50.0, 51.0, 50.0, 51.0, 50.0];

    let result = rsi(&input, 3).unwrap();

    // Check all valid values are in [0, 100]
    for (i, &val) in result.iter().enumerate() {
        if !val.is_nan() {
            assert!(
                (0.0..=100.0).contains(&val),
                "RSI at index {i} out of range: {val}"
            );
            // For alternating, expect roughly around 50
            if !(30.0..=70.0).contains(&val) {
                eprintln!(
                    "[WARN] RSI/mixed_movement: index {i} value {val} outside expected range [30, 70]"
                );
            }
        }
    }

    eprintln!("[OK] RSI/mixed_movement: all values in valid range");
}

// ==================== Summary Test ====================

#[test]
fn golden_summary() {
    // This test just prints a summary message
    eprintln!("\n=== Golden Comparison Test Summary ===");
    eprintln!("These tests compare liq-ta to TA-Lib reference values.");
    eprintln!("Discrepancies are INFORMATIONAL ONLY and do not block releases.");
    eprintln!("See PRD §7.4 for details on reference comparison policy.");
    eprintln!("==========================================\n");
}
