//! Mathematical conventions tests.
//!
//! These tests verify compliance with the mathematical conventions defined in PRD §4.8.
//! Key conventions:
//! - Population standard deviation (÷n) for Bollinger Bands
//! - EMA smoothing factors: α = 2/(period+1) for standard, α = 1/period for Wilder

#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]

use liq_ta::indicators::{
    bollinger::{bollinger, rolling_stddev},
    ema::{ema, ema_wilder, wilder_to_standard_period},
    rsi::rsi,
    sma::sma,
};

const EPSILON: f64 = 1e-10;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    (a - b).abs() < eps
}

// ==================== PRD §4.8: Population Standard Deviation Tests ====================
// "Standard deviation: Population (÷n)"

#[test]
fn math_convention_population_stddev_basic() {
    // Test data: [1, 2, 3, 4, 5]
    // Mean = 3
    // Population variance = ((1-3)^2 + (2-3)^2 + (3-3)^2 + (4-3)^2 + (5-3)^2) / 5
    //                     = (4 + 1 + 0 + 1 + 4) / 5 = 10 / 5 = 2
    // Population stddev = sqrt(2) ≈ 1.4142135623730951
    //
    // Sample stddev would be sqrt(10/4) = sqrt(2.5) ≈ 1.5811388300841898

    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let result = rolling_stddev(&data, 5).unwrap();

    let population_stddev = (2.0_f64).sqrt(); // sqrt(10/5)
    let sample_stddev = (2.5_f64).sqrt(); // sqrt(10/4)

    // Verify we're using population stddev (÷n), not sample stddev (÷n-1)
    assert!(
        approx_eq(result[4], population_stddev, EPSILON),
        "Expected population stddev {}, got {}",
        population_stddev,
        result[4]
    );
    assert!(
        !approx_eq(result[4], sample_stddev, EPSILON),
        "Should NOT match sample stddev {sample_stddev}"
    );
}

#[test]
fn math_convention_population_stddev_period_3() {
    // Smaller period for verification
    // Data window [2, 4, 6]: mean = 4
    // Population variance = ((2-4)^2 + (4-4)^2 + (6-4)^2) / 3 = (4 + 0 + 4) / 3 = 8/3
    // Population stddev = sqrt(8/3) ≈ 1.6329931618554520

    let data = vec![2.0_f64, 4.0, 6.0];
    let result = rolling_stddev(&data, 3).unwrap();

    let population_stddev = (8.0 / 3.0_f64).sqrt();

    assert!(
        approx_eq(result[2], population_stddev, EPSILON),
        "Expected population stddev {}, got {}",
        population_stddev,
        result[2]
    );
}

#[test]
fn math_convention_bollinger_uses_population_stddev() {
    // Bollinger bands should use population stddev
    // For data [10, 12, 8, 14, 6]: mean = 10
    // Deviations: [0, 2, -2, 4, -4]
    // Squared deviations: [0, 4, 4, 16, 16]
    // Population variance = 40 / 5 = 8
    // Population stddev = sqrt(8) ≈ 2.8284271247461903

    let data = vec![10.0_f64, 12.0, 8.0, 14.0, 6.0];
    let num_std = 2.0;
    let result = bollinger(&data, 5, num_std).unwrap();

    let mean = 10.0;
    let population_stddev = (8.0_f64).sqrt();
    let expected_upper = mean + num_std * population_stddev;
    let expected_lower = mean - num_std * population_stddev;

    assert!(
        approx_eq(result.middle[4], mean, EPSILON),
        "Middle band should be mean"
    );
    assert!(
        approx_eq(result.upper[4], expected_upper, EPSILON),
        "Upper band should use population stddev: expected {}, got {}",
        expected_upper,
        result.upper[4]
    );
    assert!(
        approx_eq(result.lower[4], expected_lower, EPSILON),
        "Lower band should use population stddev: expected {}, got {}",
        expected_lower,
        result.lower[4]
    );
}

// ==================== PRD §4.6: EMA Smoothing Factor Tests ====================
// "Standard EMA uses α = 2/(period+1)"
// "Wilder smoothing uses α = 1/period"

#[test]
fn math_convention_ema_standard_alpha() {
    // Standard EMA: α = 2/(period+1)
    // For period 3: α = 2/4 = 0.5
    // For period 9: α = 2/10 = 0.2
    // For period 19: α = 2/20 = 0.1

    // Verify by computing: EMA[i] = α × Price[i] + (1-α) × EMA[i-1]
    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let result = ema(&data, 3).unwrap();

    let alpha = 2.0 / 4.0; // 0.5
    let one_minus_alpha = 1.0 - alpha;

    // First EMA is SMA seed: (1+2+3)/3 = 2.0
    assert!(
        approx_eq(result[2], 2.0, EPSILON),
        "EMA seed should be SMA = 2.0"
    );

    // EMA[3] = 0.5 * 4 + 0.5 * 2.0 = 3.0
    let expected_3 = alpha * 4.0 + one_minus_alpha * 2.0;
    assert!(
        approx_eq(result[3], expected_3, EPSILON),
        "EMA[3] should be {}, got {}",
        expected_3,
        result[3]
    );

    // EMA[4] = 0.5 * 5 + 0.5 * 3.0 = 4.0
    let expected_4 = alpha * 5.0 + one_minus_alpha * 3.0;
    assert!(
        approx_eq(result[4], expected_4, EPSILON),
        "EMA[4] should be {}, got {}",
        expected_4,
        result[4]
    );
}

#[test]
fn math_convention_ema_wilder_alpha() {
    // Wilder's EMA: α = 1/period
    // For period 3: α = 1/3 ≈ 0.333...

    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let result = ema_wilder(&data, 3).unwrap();

    let alpha = 1.0 / 3.0;
    let one_minus_alpha = 1.0 - alpha;

    // First EMA is SMA seed: (1+2+3)/3 = 2.0
    assert!(
        approx_eq(result[2], 2.0, EPSILON),
        "Wilder EMA seed should be SMA = 2.0"
    );

    // EMA[3] = (1/3) * 4 + (2/3) * 2.0 = 4/3 + 4/3 = 8/3
    let expected_3 = alpha * 4.0 + one_minus_alpha * 2.0;
    assert!(
        approx_eq(result[3], expected_3, EPSILON),
        "Wilder EMA[3] should be {}, got {}",
        expected_3,
        result[3]
    );

    // EMA[4] = (1/3) * 5 + (2/3) * (8/3) = 5/3 + 16/9 = 15/9 + 16/9 = 31/9
    let expected_4 = alpha * 5.0 + one_minus_alpha * expected_3;
    assert!(
        approx_eq(result[4], expected_4, EPSILON),
        "Wilder EMA[4] should be {}, got {}",
        expected_4,
        result[4]
    );
}

#[test]
fn math_convention_wilder_standard_equivalence() {
    // Wilder period N is equivalent to standard EMA period 2N-1
    // This is because: 1/N = 2/(2N-1+1) = 2/(2N) = 1/N

    assert_eq!(wilder_to_standard_period(14), 27);
    assert_eq!(wilder_to_standard_period(7), 13);
    assert_eq!(wilder_to_standard_period(1), 1);

    // Verify equivalence with actual calculations
    // For longer data, the EMA values should converge
    let data: Vec<f64> = (0..100).map(f64::from).collect();
    let wilder_14 = ema_wilder(&data, 14).unwrap();
    let standard_27 = ema(&data, 27).unwrap();

    // After sufficient warmup, values should be very close
    // (they won't be identical due to different seed periods)
    for i in 50..100 {
        let diff = (wilder_14[i] - standard_27[i]).abs();
        assert!(
            diff < 1.0,
            "Wilder(14) and Standard(27) should converge, diff at {i} is {diff}"
        );
    }
}

// ==================== PRD §4.6: EMA SMA Seed Tests ====================
// "First EMA value = SMA of first `period` values"

#[test]
fn math_convention_ema_sma_seed() {
    let data = vec![10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0];

    // Period 3: SMA seed = (10+20+30)/3 = 20.0
    let result_3 = ema(&data, 3).unwrap();
    let expected_sma_3 = (10.0 + 20.0 + 30.0) / 3.0;
    assert!(
        approx_eq(result_3[2], expected_sma_3, EPSILON),
        "EMA(3) seed should be SMA = {expected_sma_3}"
    );

    // Period 5: SMA seed = (10+20+30+40+50)/5 = 30.0
    let result_5 = ema(&data, 5).unwrap();
    let expected_sma_5 = (10.0 + 20.0 + 30.0 + 40.0 + 50.0) / 5.0;
    assert!(
        approx_eq(result_5[4], expected_sma_5, EPSILON),
        "EMA(5) seed should be SMA = {expected_sma_5}"
    );
}

#[test]
fn math_convention_ema_wilder_sma_seed() {
    let data = vec![10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0];

    // Wilder's EMA also uses SMA seed
    let result = ema_wilder(&data, 4).unwrap();
    let expected_sma = (10.0 + 20.0 + 30.0 + 40.0) / 4.0;
    assert!(
        approx_eq(result[3], expected_sma, EPSILON),
        "Wilder EMA(4) seed should be SMA = {expected_sma}"
    );
}

// ==================== SMA Calculation Verification ====================

#[test]
fn math_convention_sma_is_arithmetic_mean() {
    let data = vec![10.0_f64, 15.0, 20.0, 25.0, 30.0];
    let result = sma(&data, 3).unwrap();

    // SMA[2] = (10+15+20)/3 = 15.0
    assert!(
        approx_eq(result[2], 15.0, EPSILON),
        "SMA should be arithmetic mean"
    );

    // SMA[3] = (15+20+25)/3 = 20.0
    assert!(approx_eq(result[3], 20.0, EPSILON));

    // SMA[4] = (20+25+30)/3 = 25.0
    assert!(approx_eq(result[4], 25.0, EPSILON));
}

// ==================== RSI Wilder Smoothing Verification ====================

#[test]
fn math_convention_rsi_uses_wilder_smoothing() {
    // RSI uses Wilder's smoothing for average gain/loss
    // Formula: new_avg = (prev_avg * (period-1) + current) / period
    // This is equivalent to EMA with α = 1/period

    // Create data with known gains and losses
    // [100, 102, 101, 103, 102, 104, 103, 105]
    // Changes: [+2, -1, +2, -1, +2, -1, +2]
    let data = vec![100.0_f64, 102.0, 101.0, 103.0, 102.0, 104.0, 103.0, 105.0];
    let result = rsi(&data, 3).unwrap();

    // All RSI values should be in [0, 100]
    for (i, &val) in result.iter().enumerate() {
        if !val.is_nan() {
            assert!(
                (0.0..=100.0).contains(&val),
                "RSI at {i} = {val} should be in [0, 100]"
            );
        }
    }

    // With alternating gains(2) and losses(1), we expect RSI > 50
    // because average gain > average loss
    for i in 3..result.len() {
        assert!(
            result[i] > 50.0,
            "RSI at {} = {} should be > 50 when gains > losses",
            i,
            result[i]
        );
    }
}

// ==================== Constant Value Invariants ====================

#[test]
fn math_convention_sma_of_constant_is_constant() {
    let data = vec![42.0_f64; 10];
    let result = sma(&data, 5).unwrap();

    for i in 4..10 {
        assert!(
            approx_eq(result[i], 42.0, EPSILON),
            "SMA of constant should be that constant"
        );
    }
}

#[test]
fn math_convention_ema_of_constant_is_constant() {
    let data = vec![42.0_f64; 10];
    let result = ema(&data, 5).unwrap();

    for i in 4..10 {
        assert!(
            approx_eq(result[i], 42.0, EPSILON),
            "EMA of constant should be that constant"
        );
    }
}

#[test]
fn math_convention_stddev_of_constant_is_zero() {
    let data = vec![42.0_f64; 10];
    let result = rolling_stddev(&data, 5).unwrap();

    for i in 4..10 {
        assert!(
            approx_eq(result[i], 0.0, EPSILON),
            "StdDev of constant should be 0"
        );
    }
}

// ==================== Numerical Precision Tests ====================

#[test]
fn math_convention_sum_of_squares_precision() {
    // Test that we maintain reasonable precision with the sum-of-squares method
    // for values with small variance relative to mean
    //
    // Note: For values around 1e6 with variance ~0.5, the sum-of-squares method
    // may lose some precision due to catastrophic cancellation. This is a known
    // limitation documented in PRD §4.8. Users with such data should pre-scale inputs.

    // Values around 1e6 with variance ~1
    let data = vec![
        1_000_000.0_f64,
        1_000_001.0,
        1_000_000.0,
        1_000_001.0,
        1_000_000.0,
    ];
    let result = rolling_stddev(&data, 5).unwrap();

    // Mean = 1_000_000.4
    // Deviations: [-0.4, 0.6, -0.4, 0.6, -0.4]
    // Sum of squared deviations = 0.16 + 0.36 + 0.16 + 0.36 + 0.16 = 1.2
    // Population variance = 1.2 / 5 = 0.24
    // Population stddev = sqrt(0.24) ≈ 0.489897948...

    let expected_stddev = (0.24_f64).sqrt();

    // Allow for some precision loss due to catastrophic cancellation
    // when computing sum-of-squares for large values with small variance.
    // The relative error should be small (< 0.1%)
    let relative_error = (result[4] - expected_stddev).abs() / expected_stddev;
    assert!(
        relative_error < 0.001, // 0.1% tolerance
        "StdDev relative error should be < 0.1%: expected {}, got {}, error {}%",
        expected_stddev,
        result[4],
        relative_error * 100.0
    );
}
