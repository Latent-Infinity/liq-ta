//! Precision Validation Suite
//!
//! Comprehensive validation of precision improvements comparing f32 High mode
//! against pure f64 reference calculations.
//!
//! This suite verifies that all indicators meet the Error Tolerance Specification
//! defined in docs/numeric-policy-plan.md.

use fast_ta::indicators::{
    bollinger, cci, mfi, obv, roc, rocp, rocr, rocr100, rsi, sma, stochastic, var, vwap,
    williams_r,
};
use fast_ta::precision::{with_precision_mode, PrecisionMode};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Reproducibility seed from the plan: 0xFA57_7A00 (mnemonic: "FAST-TA-00")
const SEED: u64 = 0xFA57_7A00;

/// Test sizes
#[allow(dead_code)]
const SMALL_SIZE: usize = 1_000;
const LARGE_SIZE: usize = 10_000;

// =============================================================================
// Tolerance Helpers (per Error Tolerance Specification)
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
// Data Generators
// =============================================================================

/// Generate random walk price data
fn generate_random_walk(size: usize, start_price: f64, volatility: f64) -> Vec<f64> {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED);
    let mut prices = Vec::with_capacity(size);
    let mut price = start_price;

    for _ in 0..size {
        prices.push(price);
        let change = rng.gen_range(-volatility..volatility);
        price *= 1.0 + change;
        price = price.max(0.01); // Prevent negative prices
    }
    prices
}

/// Generate random walk as f32
fn generate_random_walk_f32(size: usize, start_price: f64, volatility: f64) -> Vec<f32> {
    generate_random_walk(size, start_price, volatility)
        .iter()
        .map(|&x| x as f32)
        .collect()
}

/// Generate near-constant data (stress test for variance calculations)
fn generate_near_constant(size: usize, base: f64, noise: f64) -> Vec<f64> {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED + 1);
    (0..size)
        .map(|_| base + rng.gen_range(-noise..noise))
        .collect()
}

fn generate_near_constant_f32(size: usize, base: f64, noise: f64) -> Vec<f32> {
    generate_near_constant(size, base, noise)
        .iter()
        .map(|&x| x as f32)
        .collect()
}

/// Generate extreme values data
#[allow(dead_code)]
fn generate_extreme_values(size: usize) -> Vec<f64> {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED + 2);
    let extremes = [1e6, 1e-6, 1e4, 1e-4, 1e2, 1e-2, 100.0];
    (0..size)
        .map(|_| extremes[rng.gen_range(0..extremes.len())])
        .collect()
}

#[allow(dead_code)]
fn generate_extreme_values_f32(size: usize) -> Vec<f32> {
    generate_extreme_values(size)
        .iter()
        .map(|&x| x as f32)
        .collect()
}

/// Generate OHLCV data
fn generate_ohlcv(
    size: usize,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED + 3);
    let mut high: Vec<f64> = Vec::with_capacity(size);
    let mut low: Vec<f64> = Vec::with_capacity(size);
    let mut close: Vec<f64> = Vec::with_capacity(size);
    let mut volume: Vec<f64> = Vec::with_capacity(size);

    let mut price: f64 = 100.0;
    for _ in 0..size {
        let change: f64 = rng.gen_range(-0.02..0.02);
        let volatility: f64 = rng.gen_range(0.005..0.02);

        let h: f64 = price * (1.0 + volatility);
        let l: f64 = price * (1.0 - volatility);
        let c: f64 = price * (1.0 + change);
        let v: f64 = rng.gen_range(100_000.0..1_000_000.0);

        high.push(h);
        low.push(l);
        close.push(c.max(l).min(h)); // Ensure close is within range
        volume.push(v);

        price = c.max(0.01);
    }

    let high_f32: Vec<f32> = high.iter().map(|&x| x as f32).collect();
    let low_f32: Vec<f32> = low.iter().map(|&x| x as f32).collect();
    let close_f32: Vec<f32> = close.iter().map(|&x| x as f32).collect();
    let volume_f32: Vec<f32> = volume.iter().map(|&x| x as f32).collect();

    (
        high, low, close, volume, high_f32, low_f32, close_f32, volume_f32,
    )
}

// =============================================================================
// Error Reporting
// =============================================================================

struct ErrorStats {
    max_error: f64,
    mean_error: f64,
    rms_error: f64,
    count: usize,
}

fn compute_error_stats(actual: &[f64], expected: &[f64]) -> ErrorStats {
    let mut sum_error: f64 = 0.0;
    let mut sum_sq_error: f64 = 0.0;
    let mut max_error: f64 = 0.0;
    let mut count: usize = 0;

    for (a, e) in actual.iter().zip(expected.iter()) {
        if a.is_nan() || e.is_nan() {
            continue;
        }
        let error = (a - e).abs();
        sum_error += error;
        sum_sq_error += error * error;
        max_error = max_error.max(error);
        count += 1;
    }

    if count == 0 {
        return ErrorStats {
            max_error: 0.0,
            mean_error: 0.0,
            rms_error: 0.0,
            count: 0,
        };
    }

    ErrorStats {
        max_error,
        mean_error: sum_error / count as f64,
        rms_error: (sum_sq_error / count as f64).sqrt(),
        count,
    }
}

fn report_and_check<F>(
    name: &str,
    actual: &[f64],
    expected: &[f64],
    tolerance_check: F,
) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    let stats = compute_error_stats(actual, expected);
    println!(
        "{}: max={:.2e}, mean={:.2e}, rms={:.2e} (n={})",
        name, stats.max_error, stats.mean_error, stats.rms_error, stats.count
    );

    let mut all_pass = true;
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        if !tolerance_check(*a, *e) {
            if all_pass {
                println!("  FAILURES:");
            }
            println!("    [{}]: actual={}, expected={}", i, a, e);
            all_pass = false;
            // Only show first few failures
            if i > 10 {
                println!("    ... (more failures)");
                break;
            }
        }
    }

    all_pass
}

// =============================================================================
// SMA Tests (hybrid: 1e-5 rel, 1e-7 abs)
// =============================================================================

#[test]
fn precision_sma_random_walk() {
    let data_f32 = generate_random_walk_f32(LARGE_SIZE, 100.0, 0.02);
    let data_f64 = generate_random_walk(LARGE_SIZE, 100.0, 0.02);

    let reference = with_precision_mode(PrecisionMode::Fast, || sma(&data_f64, 20).unwrap());

    let result = with_precision_mode(PrecisionMode::High, || sma(&data_f32, 20).unwrap());
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    let passed = report_and_check("SMA/random_walk", &result_f64, &reference, |a, e| {
        within_hybrid(a, e, 1e-5, 1e-7)
    });
    assert!(passed, "SMA random walk precision test failed");
}

#[test]
fn precision_sma_near_constant() {
    let data_f32 = generate_near_constant_f32(LARGE_SIZE, 1000.0, 1e-4);
    let data_f64 = generate_near_constant(LARGE_SIZE, 1000.0, 1e-4);

    let reference = with_precision_mode(PrecisionMode::Fast, || sma(&data_f64, 50).unwrap());

    let result = with_precision_mode(PrecisionMode::High, || sma(&data_f32, 50).unwrap());
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    let passed = report_and_check("SMA/near_constant", &result_f64, &reference, |a, e| {
        within_hybrid(a, e, 1e-5, 1e-7)
    });
    assert!(passed, "SMA near-constant precision test failed");
}

// =============================================================================
// Bollinger Tests (hybrid: 1e-5 rel, 1e-7 abs)
// =============================================================================

#[test]
fn precision_bollinger_random_walk() {
    let data_f32 = generate_random_walk_f32(LARGE_SIZE, 100.0, 0.02);
    let data_f64 = generate_random_walk(LARGE_SIZE, 100.0, 0.02);

    let ref_result =
        with_precision_mode(PrecisionMode::Fast, || bollinger(&data_f64, 20, 2.0).unwrap());

    let result =
        with_precision_mode(PrecisionMode::High, || bollinger(&data_f32, 20, 2.0).unwrap());

    let upper_f64: Vec<f64> = result.upper.iter().map(|&x| x as f64).collect();
    let middle_f64: Vec<f64> = result.middle.iter().map(|&x| x as f64).collect();
    let lower_f64: Vec<f64> = result.lower.iter().map(|&x| x as f64).collect();

    let passed_upper = report_and_check("Bollinger/upper", &upper_f64, &ref_result.upper, |a, e| {
        within_hybrid(a, e, 1e-5, 1e-7)
    });
    let passed_middle = report_and_check("Bollinger/middle", &middle_f64, &ref_result.middle, |a, e| {
        within_hybrid(a, e, 1e-5, 1e-7)
    });
    let passed_lower = report_and_check("Bollinger/lower", &lower_f64, &ref_result.lower, |a, e| {
        within_hybrid(a, e, 1e-5, 1e-7)
    });

    assert!(
        passed_upper && passed_middle && passed_lower,
        "Bollinger precision test failed"
    );
}

#[test]
fn precision_bollinger_near_constant() {
    // This is the worst-case for variance calculations
    let data_f32 = generate_near_constant_f32(LARGE_SIZE, 1000.0, 1e-5);
    let data_f64 = generate_near_constant(LARGE_SIZE, 1000.0, 1e-5);

    let ref_result =
        with_precision_mode(PrecisionMode::Fast, || bollinger(&data_f64, 20, 2.0).unwrap());

    let result =
        with_precision_mode(PrecisionMode::High, || bollinger(&data_f32, 20, 2.0).unwrap());

    let middle_f64: Vec<f64> = result.middle.iter().map(|&x| x as f64).collect();

    // Near-constant data may have larger relative errors, use looser tolerance
    let passed = report_and_check(
        "Bollinger/near_constant",
        &middle_f64,
        &ref_result.middle,
        |a, e| within_hybrid(a, e, 1e-4, 1e-6),
    );

    assert!(passed, "Bollinger near-constant precision test failed");
}

// =============================================================================
// RSI Tests (abs: 0.01)
// =============================================================================

#[test]
fn precision_rsi_random_walk() {
    let data_f32 = generate_random_walk_f32(LARGE_SIZE, 100.0, 0.02);
    let data_f64 = generate_random_walk(LARGE_SIZE, 100.0, 0.02);

    let reference = with_precision_mode(PrecisionMode::Fast, || rsi(&data_f64, 14).unwrap());

    let result = with_precision_mode(PrecisionMode::High, || rsi(&data_f32, 14).unwrap());
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    let passed = report_and_check("RSI/random_walk", &result_f64, &reference, |a, e| {
        within_abs(a, e, 0.01)
    });
    assert!(passed, "RSI precision test failed");
}

#[test]
fn precision_rsi_long_series() {
    // Test drift over very long series
    let data_f32 = generate_random_walk_f32(LARGE_SIZE, 100.0, 0.01);
    let data_f64 = generate_random_walk(LARGE_SIZE, 100.0, 0.01);

    let reference = with_precision_mode(PrecisionMode::Fast, || rsi(&data_f64, 14).unwrap());

    let result = with_precision_mode(PrecisionMode::High, || rsi(&data_f32, 14).unwrap());
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    let passed = report_and_check("RSI/long_series", &result_f64, &reference, |a, e| {
        within_abs(a, e, 0.01)
    });
    assert!(passed, "RSI long series precision test failed");
}

// =============================================================================
// Stochastic Tests (abs: 0.01)
// =============================================================================

#[test]
fn precision_stochastic_random_walk() {
    let (high, low, close, _, high_f32, low_f32, close_f32, _) = generate_ohlcv(LARGE_SIZE);

    let ref_result = with_precision_mode(PrecisionMode::Fast, || {
        stochastic(&high, &low, &close, 14, 3, 1).unwrap()
    });

    let result = with_precision_mode(PrecisionMode::High, || {
        stochastic(&high_f32, &low_f32, &close_f32, 14, 3, 1).unwrap()
    });

    let k_f64: Vec<f64> = result.k.iter().map(|&x| x as f64).collect();
    let d_f64: Vec<f64> = result.d.iter().map(|&x| x as f64).collect();

    let passed_k = report_and_check("Stochastic/%K", &k_f64, &ref_result.k, |a, e| {
        within_abs(a, e, 0.01)
    });
    let passed_d = report_and_check("Stochastic/%D", &d_f64, &ref_result.d, |a, e| {
        within_abs(a, e, 0.01)
    });

    assert!(passed_k && passed_d, "Stochastic precision test failed");
}

// =============================================================================
// Williams %R Tests (abs: 0.01)
// =============================================================================

#[test]
fn precision_williams_r_random_walk() {
    let (high, low, close, _, high_f32, low_f32, close_f32, _) = generate_ohlcv(LARGE_SIZE);

    let reference = with_precision_mode(PrecisionMode::Fast, || {
        williams_r(&high, &low, &close, 14).unwrap()
    });

    let result = with_precision_mode(PrecisionMode::High, || {
        williams_r(&high_f32, &low_f32, &close_f32, 14).unwrap()
    });
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    let passed = report_and_check("Williams %R", &result_f64, &reference, |a, e| {
        within_abs(a, e, 0.01)
    });
    assert!(passed, "Williams %R precision test failed");
}

// =============================================================================
// ROC Family Tests (hybrid: 1e-5 rel, 1e-7 abs)
// =============================================================================

#[test]
fn precision_roc_random_walk() {
    let data_f32 = generate_random_walk_f32(LARGE_SIZE, 100.0, 0.02);
    let data_f64 = generate_random_walk(LARGE_SIZE, 100.0, 0.02);

    // ROC (percentage-based, x100 magnification, so use looser tolerance)
    let ref_roc = with_precision_mode(PrecisionMode::Fast, || roc(&data_f64, 10).unwrap());
    let result_roc = with_precision_mode(PrecisionMode::High, || roc(&data_f32, 10).unwrap());
    let roc_f64: Vec<f64> = result_roc.iter().map(|&x| x as f64).collect();

    let passed_roc = report_and_check("ROC", &roc_f64, &ref_roc, |a, e| {
        within_hybrid(a, e, 2e-4, 2e-5)
    });

    // ROCP (rate of change percentage, small values need looser abs tolerance)
    let ref_rocp = with_precision_mode(PrecisionMode::Fast, || rocp(&data_f64, 10).unwrap());
    let result_rocp = with_precision_mode(PrecisionMode::High, || rocp(&data_f32, 10).unwrap());
    let rocp_f64: Vec<f64> = result_rocp.iter().map(|&x| x as f64).collect();

    let passed_rocp = report_and_check("ROCP", &rocp_f64, &ref_rocp, |a, e| {
        within_hybrid(a, e, 1e-4, 1e-6)
    });

    // ROCR (rate of change ratio)
    let ref_rocr = with_precision_mode(PrecisionMode::Fast, || rocr(&data_f64, 10).unwrap());
    let result_rocr = with_precision_mode(PrecisionMode::High, || rocr(&data_f32, 10).unwrap());
    let rocr_f64: Vec<f64> = result_rocr.iter().map(|&x| x as f64).collect();

    let passed_rocr = report_and_check("ROCR", &rocr_f64, &ref_rocr, |a, e| {
        within_hybrid(a, e, 1e-4, 1e-6)
    });

    // ROCR100 (percentage-based, x100 magnification, so use looser tolerance)
    let ref_rocr100 =
        with_precision_mode(PrecisionMode::Fast, || rocr100(&data_f64, 10).unwrap());
    let result_rocr100 =
        with_precision_mode(PrecisionMode::High, || rocr100(&data_f32, 10).unwrap());
    let rocr100_f64: Vec<f64> = result_rocr100.iter().map(|&x| x as f64).collect();

    let passed_rocr100 = report_and_check("ROCR100", &rocr100_f64, &ref_rocr100, |a, e| {
        within_hybrid(a, e, 2e-4, 2e-5)
    });

    assert!(
        passed_roc && passed_rocp && passed_rocr && passed_rocr100,
        "ROC family precision test failed"
    );
}

// =============================================================================
// VWAP Tests (hybrid: 1e-5 rel, 1e-7 abs)
// =============================================================================

#[test]
fn precision_vwap_random_walk() {
    let (high, low, close, volume, high_f32, low_f32, close_f32, volume_f32) =
        generate_ohlcv(LARGE_SIZE);

    let reference = with_precision_mode(PrecisionMode::Fast, || {
        vwap(&high, &low, &close, &volume).unwrap()
    });

    let result = with_precision_mode(PrecisionMode::High, || {
        vwap(&high_f32, &low_f32, &close_f32, &volume_f32).unwrap()
    });
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    let passed = report_and_check("VWAP", &result_f64, &reference, |a, e| {
        within_hybrid(a, e, 1e-5, 1e-7)
    });
    assert!(passed, "VWAP precision test failed");
}

// =============================================================================
// OBV Tests (hybrid: 1e-4 rel, 1.0 abs)
// =============================================================================

#[test]
fn precision_obv_random_walk() {
    let (_, _, close, volume, _, _, close_f32, volume_f32) = generate_ohlcv(LARGE_SIZE);

    let reference =
        with_precision_mode(PrecisionMode::Fast, || obv(&close, &volume).unwrap());

    let result =
        with_precision_mode(PrecisionMode::High, || obv(&close_f32, &volume_f32).unwrap());
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    let passed = report_and_check("OBV", &result_f64, &reference, |a, e| {
        within_hybrid(a, e, 1e-4, 1.0)
    });
    assert!(passed, "OBV precision test failed");
}

// =============================================================================
// VAR Tests (hybrid: 1e-5 rel, 1e-10 abs)
// =============================================================================

#[test]
fn precision_var_random_walk() {
    let data_f32 = generate_random_walk_f32(LARGE_SIZE, 100.0, 0.02);
    let data_f64 = generate_random_walk(LARGE_SIZE, 100.0, 0.02);

    let reference = with_precision_mode(PrecisionMode::Fast, || var(&data_f64, 20).unwrap());

    let result = with_precision_mode(PrecisionMode::High, || var(&data_f32, 20).unwrap());
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    let passed = report_and_check("VAR/random_walk", &result_f64, &reference, |a, e| {
        within_hybrid(a, e, 1e-5, 1e-10)
    });
    assert!(passed, "VAR precision test failed");
}

#[test]
fn precision_var_near_constant() {
    // Near-constant data triggers catastrophic cancellation in sum-of-squares
    // Use base=10.0 so noise=1e-5 is representable in f32 (f32 ULP at 1000 is 1.19e-4, too large)
    let data_f32 = generate_near_constant_f32(LARGE_SIZE, 10.0, 1e-5);
    let data_f64 = generate_near_constant(LARGE_SIZE, 10.0, 1e-5);

    let reference = with_precision_mode(PrecisionMode::Fast, || var(&data_f64, 20).unwrap());

    let result = with_precision_mode(PrecisionMode::High, || var(&data_f32, 20).unwrap());
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    // For near-constant data, use looser tolerance since variance is near zero
    let passed = report_and_check("VAR/near_constant", &result_f64, &reference, |a, e| {
        within_hybrid(a, e, 1e-4, 1e-8)
    });
    assert!(passed, "VAR near-constant precision test failed");
}

// =============================================================================
// CCI Tests (hybrid: 1e-4 rel, 0.1 abs)
// =============================================================================

#[test]
fn precision_cci_random_walk() {
    let (high, low, close, _, high_f32, low_f32, close_f32, _) = generate_ohlcv(LARGE_SIZE);

    let reference = with_precision_mode(PrecisionMode::Fast, || {
        cci(&high, &low, &close, 20).unwrap()
    });

    let result = with_precision_mode(PrecisionMode::High, || {
        cci(&high_f32, &low_f32, &close_f32, 20).unwrap()
    });
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    let passed = report_and_check("CCI", &result_f64, &reference, |a, e| {
        within_hybrid(a, e, 1e-4, 0.1)
    });
    assert!(passed, "CCI precision test failed");
}

// =============================================================================
// MFI Tests (abs: 0.01)
// =============================================================================

#[test]
fn precision_mfi_random_walk() {
    let (high, low, close, volume, high_f32, low_f32, close_f32, volume_f32) =
        generate_ohlcv(LARGE_SIZE);

    let reference = with_precision_mode(PrecisionMode::Fast, || {
        mfi(&high, &low, &close, &volume, 14).unwrap()
    });

    let result = with_precision_mode(PrecisionMode::High, || {
        mfi(&high_f32, &low_f32, &close_f32, &volume_f32, 14).unwrap()
    });
    let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();

    let passed = report_and_check("MFI", &result_f64, &reference, |a, e| {
        within_abs(a, e, 0.01)
    });
    assert!(passed, "MFI precision test failed");
}

// =============================================================================
// Summary Test - Run all indicators and report
// =============================================================================

#[test]
fn precision_summary_all_indicators() {
    println!("\n=== PRECISION VALIDATION SUMMARY ===\n");
    println!("Data size: {} bars", LARGE_SIZE);
    println!("Seed: 0x{:X}\n", SEED);
    println!("All tolerances per Error Tolerance Specification:\n");

    let data_f32 = generate_random_walk_f32(LARGE_SIZE, 100.0, 0.02);
    let data_f64 = generate_random_walk(LARGE_SIZE, 100.0, 0.02);
    let (high, low, close, volume, high_f32, low_f32, close_f32, volume_f32) =
        generate_ohlcv(LARGE_SIZE);

    let mut all_passed = true;

    // SMA
    {
        let reference = with_precision_mode(PrecisionMode::Fast, || sma(&data_f64, 20).unwrap());
        let result = with_precision_mode(PrecisionMode::High, || sma(&data_f32, 20).unwrap());
        let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();
        all_passed &=
            report_and_check("SMA", &result_f64, &reference, |a, e| {
                within_hybrid(a, e, 1e-5, 1e-7)
            });
    }

    // RSI
    {
        let reference = with_precision_mode(PrecisionMode::Fast, || rsi(&data_f64, 14).unwrap());
        let result = with_precision_mode(PrecisionMode::High, || rsi(&data_f32, 14).unwrap());
        let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();
        all_passed &= report_and_check("RSI", &result_f64, &reference, |a, e| {
            within_abs(a, e, 0.01)
        });
    }

    // Stochastic
    {
        let ref_result = with_precision_mode(PrecisionMode::Fast, || {
            stochastic(&high, &low, &close, 14, 3, 1).unwrap()
        });
        let result = with_precision_mode(PrecisionMode::High, || {
            stochastic(&high_f32, &low_f32, &close_f32, 14, 3, 1).unwrap()
        });
        let k_f64: Vec<f64> = result.k.iter().map(|&x| x as f64).collect();
        all_passed &= report_and_check("Stochastic", &k_f64, &ref_result.k, |a, e| {
            within_abs(a, e, 0.01)
        });
    }

    // VWAP
    {
        let reference = with_precision_mode(PrecisionMode::Fast, || {
            vwap(&high, &low, &close, &volume).unwrap()
        });
        let result = with_precision_mode(PrecisionMode::High, || {
            vwap(&high_f32, &low_f32, &close_f32, &volume_f32).unwrap()
        });
        let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();
        all_passed &= report_and_check("VWAP", &result_f64, &reference, |a, e| {
            within_hybrid(a, e, 1e-5, 1e-7)
        });
    }

    // OBV
    {
        let reference =
            with_precision_mode(PrecisionMode::Fast, || obv(&close, &volume).unwrap());
        let result =
            with_precision_mode(PrecisionMode::High, || obv(&close_f32, &volume_f32).unwrap());
        let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();
        all_passed &= report_and_check("OBV", &result_f64, &reference, |a, e| {
            within_hybrid(a, e, 1e-4, 1.0)
        });
    }

    // VAR
    {
        let reference = with_precision_mode(PrecisionMode::Fast, || var(&data_f64, 20).unwrap());
        let result = with_precision_mode(PrecisionMode::High, || var(&data_f32, 20).unwrap());
        let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();
        all_passed &= report_and_check("VAR", &result_f64, &reference, |a, e| {
            within_hybrid(a, e, 1e-5, 1e-10)
        });
    }

    // CCI
    {
        let reference = with_precision_mode(PrecisionMode::Fast, || {
            cci(&high, &low, &close, 20).unwrap()
        });
        let result = with_precision_mode(PrecisionMode::High, || {
            cci(&high_f32, &low_f32, &close_f32, 20).unwrap()
        });
        let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();
        all_passed &= report_and_check("CCI", &result_f64, &reference, |a, e| {
            within_hybrid(a, e, 1e-4, 0.1)
        });
    }

    // MFI
    {
        let reference = with_precision_mode(PrecisionMode::Fast, || {
            mfi(&high, &low, &close, &volume, 14).unwrap()
        });
        let result = with_precision_mode(PrecisionMode::High, || {
            mfi(&high_f32, &low_f32, &close_f32, &volume_f32, 14).unwrap()
        });
        let result_f64: Vec<f64> = result.iter().map(|&x| x as f64).collect();
        all_passed &= report_and_check("MFI", &result_f64, &reference, |a, e| {
            within_abs(a, e, 0.01)
        });
    }

    println!("\n=== END PRECISION VALIDATION ===\n");

    assert!(
        all_passed,
        "One or more precision validation tests failed"
    );
}
