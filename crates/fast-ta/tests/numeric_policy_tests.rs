//! Numeric policy compliance tests.
//!
//! These tests verify compliance with the numeric policy defined in PRD §4.3 and §4.5.
//! The policy covers NaN propagation, infinity handling, and edge case behaviors.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]

use fast_ta::indicators::{
    atr::{atr, true_range},
    bollinger::bollinger,
    ema::ema,
    macd::macd,
    rsi::rsi,
    sma::sma,
    stochastic::stochastic_fast,
};
use fast_ta::kernels::rolling_extrema::{rolling_max, rolling_min};

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

// ==================== PRD §4.3: NaN Propagation Tests ====================
// "NaN in window: Output NaN for that position"

#[test]
fn numeric_policy_nan_propagation_sma() {
    let data = vec![1.0_f64, 2.0, f64::NAN, 4.0, 5.0, 6.0, 7.0];
    let result = sma(&data, 3).unwrap();

    // SMA with period 3 should produce NaN when any value in window contains NaN
    // Window [2.0, NaN, 4.0] at index 3 should produce NaN
    assert!(result[3].is_nan(), "SMA should propagate NaN at index 3");
    // Window [NaN, 4.0, 5.0] at index 4 should produce NaN
    assert!(result[4].is_nan(), "SMA should propagate NaN at index 4");
    // Window [4.0, 5.0, 6.0] at index 5 should be valid
    assert!(
        !result[5].is_nan(),
        "SMA should be valid once NaN leaves window"
    );
}

#[test]
fn numeric_policy_nan_propagation_ema() {
    let data = vec![1.0_f64, 2.0, 3.0, f64::NAN, 5.0, 6.0, 7.0, 8.0];
    let result = ema(&data, 3).unwrap();

    // Once NaN appears, EMA should propagate NaN forward (due to recursive nature)
    // EMA at index 2 is the SMA seed (valid)
    assert!(!result[2].is_nan(), "EMA seed should be valid before NaN");
    // EMA at index 3 encounters NaN in data
    assert!(result[3].is_nan(), "EMA should be NaN when input is NaN");
    // EMA at index 4 depends on prior NaN EMA
    assert!(
        result[4].is_nan(),
        "EMA should propagate NaN forward due to recursion"
    );
}

#[test]
fn numeric_policy_nan_propagation_rsi() {
    let data = vec![10.0_f64, 11.0, 12.0, f64::NAN, 14.0, 15.0, 16.0, 17.0, 18.0];
    let result = rsi(&data, 3).unwrap();

    // RSI should propagate NaN when NaN appears in the input
    assert!(result[3].is_nan(), "RSI should be NaN at position with NaN");
}

#[test]
fn numeric_policy_nan_propagation_macd() {
    let mut data: Vec<f64> = (0..40).map(f64::from).collect();
    data[20] = f64::NAN;
    let result = macd(&data, 12, 26, 9).unwrap();

    // MACD should propagate NaN after encountering NaN in slow EMA calculation
    // The NaN at index 20 will affect the slow EMA
    let nan_after = result
        .macd_line
        .iter()
        .skip(21)
        .all(|x| x.is_nan() || !x.is_finite());
    assert!(nan_after || result.macd_line[20..].iter().any(|x| x.is_nan()));
}

#[test]
fn numeric_policy_nan_propagation_bollinger() {
    let data = vec![20.0_f64, 21.0, f64::NAN, 22.0, 23.0, 24.0, 25.0];
    let result = bollinger(&data, 3, 2.0).unwrap();

    // Bollinger should produce NaN when window contains NaN
    assert!(
        result.middle[3].is_nan(),
        "Bollinger middle should be NaN at index 3"
    );
    assert!(
        result.upper[3].is_nan(),
        "Bollinger upper should be NaN at index 3"
    );
    assert!(
        result.lower[3].is_nan(),
        "Bollinger lower should be NaN at index 3"
    );
}

#[test]
fn numeric_policy_nan_propagation_atr() {
    let high = vec![11.0_f64, 12.0, f64::NAN, 14.0, 15.0, 16.0, 17.0];
    let low = vec![9.0_f64, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let close = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];

    let tr = true_range(&high, &low, &close).unwrap();

    // True range should be NaN when high contains NaN
    assert!(tr[2].is_nan(), "TR should be NaN when high is NaN");

    let atr_result = atr(&high, &low, &close, 3).unwrap();
    // ATR calculation should propagate NaN
    assert!(atr_result[3].is_nan(), "ATR should propagate NaN");
}

#[test]
fn numeric_policy_nan_propagation_stochastic() {
    let high = vec![15.0_f64, 16.0, f64::NAN, 18.0, 19.0, 20.0, 21.0];
    let low = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    let close = vec![12.0_f64, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];

    let result = stochastic_fast(&high, &low, &close, 3, 3).unwrap();

    // %K should be NaN when window contains NaN in any of high/low/close
    assert!(result.k[3].is_nan(), "%K should propagate NaN at index 3");
    assert!(result.k[4].is_nan(), "%K should propagate NaN at index 4");
}

#[test]
fn numeric_policy_nan_handling_rolling_extrema() {
    // Note: Rolling extrema has a DIFFERENT NaN policy than other indicators.
    // It skips NaN values and returns the max/min of non-NaN values in the window.
    // This is intentional for use in Stochastic oscillator which needs to find
    // extrema even when some values are missing.

    let data = vec![1.0_f64, f64::NAN, 3.0, 4.0, 5.0];

    let max_result = rolling_max(&data, 3).unwrap();
    let min_result = rolling_min(&data, 3).unwrap();

    // Window [1, NaN, 3]: max should be 3, min should be 1 (NaN skipped)
    assert!(
        approx_eq(max_result[2], 3.0, EPSILON),
        "Rolling max should skip NaN and return 3.0"
    );
    assert!(
        approx_eq(min_result[2], 1.0, EPSILON),
        "Rolling min should skip NaN and return 1.0"
    );

    // Window [NaN, 3, 4]: max should be 4, min should be 3
    assert!(
        approx_eq(max_result[3], 4.0, EPSILON),
        "Rolling max should skip NaN and return 4.0"
    );
    assert!(
        approx_eq(min_result[3], 3.0, EPSILON),
        "Rolling min should skip NaN and return 3.0"
    );
}

#[test]
fn numeric_policy_nan_all_nan_window_rolling_extrema() {
    // When ALL values in window are NaN, result should be NaN
    let data = vec![f64::NAN, f64::NAN, f64::NAN, 4.0, 5.0];

    let max_result = rolling_max(&data, 3).unwrap();

    // All-NaN window should produce NaN
    assert!(
        max_result[2].is_nan(),
        "All-NaN window should produce NaN in rolling max"
    );

    // Once valid values enter window, should produce valid result
    assert!(
        approx_eq(max_result[4], 5.0, EPSILON),
        "Rolling max should be valid after NaN leaves window"
    );
}

// ==================== PRD §4.3: Infinity Handling Tests ====================
// Infinity is treated as an invalid value (like NaN) and produces NaN output
// when present in the rolling window. This is consistent with the is_invalid()
// utility and prevents arithmetic issues like infinity - infinity = NaN.

#[test]
fn numeric_policy_infinity_propagation_sma() {
    // Need enough data so infinity leaves the window
    let data = vec![1.0_f64, 2.0, f64::INFINITY, 4.0, 5.0, 6.0, 7.0];
    let result = sma(&data, 3).unwrap();

    // SMA with infinity should produce NaN (infinity treated as invalid)
    // Window [1, 2, INF] at index 2 → NaN
    assert!(
        result[2].is_nan(),
        "SMA should produce NaN when infinity is in window"
    );
    // Window [2, INF, 4] at index 3 → NaN
    assert!(
        result[3].is_nan(),
        "SMA should produce NaN when infinity is in window at index 3"
    );
    // Window [INF, 4, 5] at index 4 → NaN
    assert!(
        result[4].is_nan(),
        "SMA should produce NaN when infinity is in window at index 4"
    );
    // Window [4, 5, 6] at index 5 → Finite (infinity has left the window)
    assert!(
        result[5].is_finite(),
        "SMA should recover after infinity leaves window"
    );
}

#[test]
fn numeric_policy_infinity_propagation_ema() {
    let data = vec![1.0_f64, 2.0, 3.0, f64::INFINITY, 5.0, 6.0];
    let result = ema(&data, 3).unwrap();

    // EMA with infinity should produce infinity or NaN (depending on computation)
    assert!(
        result[3].is_infinite() || result[3].is_nan(),
        "EMA should handle infinity"
    );
}

#[test]
fn numeric_policy_negative_infinity() {
    // Need enough data so infinity leaves the window
    let data = vec![1.0_f64, 2.0, f64::NEG_INFINITY, 4.0, 5.0, 6.0, 7.0];
    let result = sma(&data, 3).unwrap();

    // Negative infinity is also treated as invalid and produces NaN
    // Window [1, 2, -INF] at index 2 → NaN
    assert!(
        result[2].is_nan(),
        "SMA should produce NaN when negative infinity is in window"
    );
    // Window [2, -INF, 4] at index 3 → NaN
    assert!(
        result[3].is_nan(),
        "SMA should produce NaN when negative infinity is in window at index 3"
    );
    // Window [-INF, 4, 5] at index 4 → NaN
    assert!(
        result[4].is_nan(),
        "SMA should produce NaN when negative infinity is in window at index 4"
    );
    // Window [4, 5, 6] at index 5 → Finite (infinity has left the window)
    assert!(
        result[5].is_finite(),
        "SMA should recover after negative infinity leaves window"
    );
}

// ==================== PRD §4.5: RSI Edge Cases ====================
// "All gains (avg_loss = 0): RSI = 100"
// "All losses (avg_gain = 0): RSI = 0"

#[test]
fn numeric_policy_rsi_all_gains_equals_100() {
    let data = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
    let result = rsi(&data, 5).unwrap();

    // All gains: every change is positive, so RSI = 100
    for i in 5..result.len() {
        assert!(
            approx_eq(result[i], 100.0, LOOSE_EPSILON),
            "RSI should be 100 for all gains, got {} at index {}",
            result[i],
            i
        );
    }
}

#[test]
fn numeric_policy_rsi_all_losses_equals_0() {
    let data = vec![20.0_f64, 19.0, 18.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0];
    let result = rsi(&data, 5).unwrap();

    // All losses: every change is negative, so RSI = 0
    for i in 5..result.len() {
        assert!(
            approx_eq(result[i], 0.0, LOOSE_EPSILON),
            "RSI should be 0 for all losses, got {} at index {}",
            result[i],
            i
        );
    }
}

#[test]
fn numeric_policy_rsi_no_movement_equals_50() {
    let data = vec![50.0_f64; 10];
    let result = rsi(&data, 5).unwrap();

    // No movement: avg_gain = avg_loss = 0, RSI = 50 (neutral)
    for i in 5..result.len() {
        assert!(
            approx_eq(result[i], 50.0, LOOSE_EPSILON),
            "RSI should be 50 for no movement, got {} at index {}",
            result[i],
            i
        );
    }
}

// ==================== PRD §4.5: Stochastic Edge Cases ====================
// "Denominator = 0 (high == low): %K = 50"

#[test]
fn numeric_policy_stochastic_flat_price_k50() {
    let high = vec![50.0_f64; 10];
    let low = vec![50.0_f64; 10];
    let close = vec![50.0_f64; 10];

    let result = stochastic_fast(&high, &low, &close, 5, 3).unwrap();

    // When high == low == close (flat price), %K = 50
    for i in 4..result.k.len() {
        if !result.k[i].is_nan() {
            assert!(
                approx_eq(result.k[i], 50.0, LOOSE_EPSILON),
                "%K should be 50 for flat price, got {} at index {}",
                result.k[i],
                i
            );
        }
    }
}

// ==================== PRD §4.5: Bollinger Edge Cases ====================
// "StdDev = 0 (constant prices): Upper = Lower = Middle"

#[test]
fn numeric_policy_bollinger_constant_price_bands_collapse() {
    let data = vec![100.0_f64; 10];
    let result = bollinger(&data, 5, 2.0).unwrap();

    // Constant price: stddev = 0, so all bands equal the mean
    for i in 4..result.middle.len() {
        if !result.middle[i].is_nan() {
            assert!(
                approx_eq(result.middle[i], 100.0, EPSILON),
                "Middle should be 100"
            );
            assert!(
                approx_eq(result.upper[i], 100.0, EPSILON),
                "Upper should equal middle when stddev = 0"
            );
            assert!(
                approx_eq(result.lower[i], 100.0, EPSILON),
                "Lower should equal middle when stddev = 0"
            );
        }
    }
}

#[test]
fn numeric_policy_bollinger_symmetric_bands() {
    let data = vec![20.0_f64, 22.0, 18.0, 24.0, 16.0, 26.0, 14.0];
    let result = bollinger(&data, 3, 2.0).unwrap();

    // Bands should always be symmetric around middle
    for i in 2..result.middle.len() {
        if !result.middle[i].is_nan() {
            let upper_diff = result.upper[i] - result.middle[i];
            let lower_diff = result.middle[i] - result.lower[i];
            assert!(
                approx_eq(upper_diff, lower_diff, EPSILON),
                "Bands should be symmetric at index {i}: upper_diff={upper_diff}, lower_diff={lower_diff}"
            );
        }
    }
}

// ==================== PRD §4.5: ATR Edge Cases ====================
// "First value initialization: SMA of first n True Ranges"

#[test]
fn numeric_policy_atr_first_value_is_sma() {
    let high = vec![11.0_f64, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
    let low = vec![9.0_f64, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let close = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];

    let tr = true_range(&high, &low, &close).unwrap();
    let atr_result = atr(&high, &low, &close, 3).unwrap();

    // First ATR value should be SMA of first 3 true ranges
    // TR[0] is NaN, TR[1] and TR[2] are valid
    // ATR[3] should be average of TR[1], TR[2], TR[3]
    if !atr_result[3].is_nan() {
        let expected_sma = (tr[1] + tr[2] + tr[3]) / 3.0;
        assert!(
            approx_eq(atr_result[3], expected_sma, EPSILON),
            "First ATR value should be SMA of first 3 TR values"
        );
    }
}

// ==================== Additional Numeric Edge Cases ====================

#[test]
fn numeric_policy_very_large_values() {
    let data = vec![1e300_f64, 2e300, 3e300, 4e300, 5e300];
    let result = sma(&data, 3).unwrap();

    // Should handle very large values without overflow
    assert!(
        !result[2].is_nan() && !result[2].is_infinite(),
        "SMA should handle large values"
    );
    assert!(
        approx_eq(result[2], 2e300, 1e290),
        "SMA of large values should be correct"
    );
}

#[test]
fn numeric_policy_very_small_values() {
    let data = vec![1e-300_f64, 2e-300, 3e-300, 4e-300, 5e-300];
    let result = sma(&data, 3).unwrap();

    // Should handle very small values without underflow
    assert!(
        !result[2].is_nan(),
        "SMA should handle small values without underflow"
    );
    assert!(
        approx_eq(result[2], 2e-300, 1e-310),
        "SMA of small values should be correct"
    );
}

#[test]
fn numeric_policy_mixed_signs() {
    let data = vec![-5.0_f64, -3.0, 0.0, 3.0, 5.0];
    let result = sma(&data, 3).unwrap();

    // Should handle mixed positive and negative values
    assert!(
        approx_eq(result[2], (-5.0 - 3.0 + 0.0) / 3.0, EPSILON),
        "SMA should handle mixed signs"
    );
    assert!(
        approx_eq(result[3], (-3.0 + 0.0 + 3.0) / 3.0, EPSILON),
        "SMA should handle crossing zero"
    );
}

#[test]
fn numeric_policy_all_nan_window() {
    let data = vec![f64::NAN, f64::NAN, f64::NAN, 4.0, 5.0, 6.0];
    let result = sma(&data, 3).unwrap();

    // All-NaN window should produce NaN
    assert!(result[2].is_nan(), "All-NaN window should produce NaN");
    // But once NaN leaves window, output should be valid
    assert!(
        approx_eq(result[5], 5.0, EPSILON),
        "SMA should be valid after NaN leaves window"
    );
}

#[test]
fn numeric_policy_rsi_bounds() {
    // Generate random-ish price movements
    let data: Vec<f64> = vec![
        50.0, 51.0, 49.0, 52.0, 48.0, 53.0, 47.0, 54.0, 46.0, 55.0, 45.0, 56.0, 44.0, 57.0, 43.0,
    ];
    let result = rsi(&data, 5).unwrap();

    // RSI should always be in [0, 100]
    for (i, &val) in result.iter().enumerate() {
        if !val.is_nan() {
            assert!(
                (0.0..=100.0).contains(&val),
                "RSI at index {i} = {val} is out of bounds"
            );
        }
    }
}

#[test]
fn numeric_policy_stochastic_bounds() {
    let high = vec![
        55.0_f64, 56.0, 54.0, 57.0, 53.0, 58.0, 52.0, 59.0, 51.0, 60.0,
    ];
    let low = vec![
        45.0_f64, 46.0, 44.0, 47.0, 43.0, 48.0, 42.0, 49.0, 41.0, 50.0,
    ];
    let close = vec![
        50.0_f64, 51.0, 49.0, 52.0, 48.0, 53.0, 47.0, 54.0, 46.0, 55.0,
    ];

    let result = stochastic_fast(&high, &low, &close, 5, 3).unwrap();

    // %K and %D should always be in [0, 100]
    for (i, &val) in result.k.iter().enumerate() {
        if !val.is_nan() {
            assert!(
                (0.0..=100.0).contains(&val),
                "%K at index {i} = {val} is out of bounds"
            );
        }
    }
    for (i, &val) in result.d.iter().enumerate() {
        if !val.is_nan() {
            assert!(
                (0.0..=100.0).contains(&val),
                "%D at index {i} = {val} is out of bounds"
            );
        }
    }
}

// ==================== Task 4.2: Numeric Stability Edge Cases ====================
// Tests for extreme values, denormalized numbers, and potential overflow/underflow

#[test]
fn numeric_stability_values_near_f64_max() {
    // Values near f64::MAX (1.7976931348623157e308)
    let data = vec![1e307_f64, 2e307, 3e307, 4e307, 5e307];
    let result = sma(&data, 3).unwrap();

    // Should not overflow
    assert!(
        !result[2].is_nan() && !result[2].is_infinite(),
        "SMA should handle values near f64::MAX without overflow"
    );
    // Mean of [1e307, 2e307, 3e307] = 2e307
    assert!(
        (result[2] - 2e307).abs() / 2e307 < 1e-10,
        "SMA should be accurate for large values"
    );
}

#[test]
fn numeric_stability_values_near_f64_min_positive() {
    // Values near f64::MIN_POSITIVE (2.2250738585072014e-308)
    let data = vec![1e-307_f64, 2e-307, 3e-307, 4e-307, 5e-307];
    let result = sma(&data, 3).unwrap();

    // Should not underflow to zero
    assert!(
        result[2] > 0.0,
        "SMA should not underflow for small positive values"
    );
    assert!(
        !result[2].is_nan(),
        "SMA should not produce NaN for small values"
    );
}

#[test]
fn numeric_stability_denormalized_numbers() {
    // Denormalized (subnormal) numbers: smaller than MIN_POSITIVE but > 0
    let tiny = f64::MIN_POSITIVE / 2.0; // Subnormal number
    let data = vec![tiny, tiny * 2.0, tiny * 3.0, tiny * 4.0, tiny * 5.0];

    let result = sma(&data, 3).unwrap();

    // Should handle subnormal numbers
    assert!(
        result[2] > 0.0,
        "SMA should produce positive result for subnormal inputs"
    );
    assert!(
        !result[2].is_nan(),
        "SMA should not produce NaN for subnormal inputs"
    );
}

#[test]
fn numeric_stability_mixed_large_and_small() {
    // Mix of large and small values in the same calculation
    // This can cause precision loss
    let data = vec![1e15_f64, 1.0, 1e15, 1.0, 1e15];
    let result = sma(&data, 3).unwrap();

    // The small values should not be "lost" entirely
    // Mean of [1e15, 1.0, 1e15] ≈ 6.67e14
    let expected = (1e15 + 1.0 + 1e15) / 3.0;
    assert!(
        !result[2].is_nan() && !result[2].is_infinite(),
        "SMA should handle mixed scales"
    );
    // Allow some precision loss due to floating point
    assert!(
        (result[2] - expected).abs() / expected < 1e-10,
        "SMA should be reasonably accurate for mixed scales"
    );
}

#[test]
fn numeric_stability_ema_large_values() {
    // EMA with large values - test that alpha calculation doesn't overflow
    let data = vec![1e200_f64, 2e200, 3e200, 4e200, 5e200, 6e200, 7e200];
    let result = ema(&data, 3).unwrap();

    // Should produce finite values
    for (i, &val) in result.iter().enumerate().skip(2) {
        assert!(
            !val.is_nan() && !val.is_infinite(),
            "EMA[{i}] should be finite for large inputs, got {val}"
        );
    }
}

#[test]
fn numeric_stability_ema_small_values() {
    // EMA with small values - test that alpha calculation doesn't underflow
    let data = vec![1e-200_f64, 2e-200, 3e-200, 4e-200, 5e-200, 6e-200, 7e-200];
    let result = ema(&data, 3).unwrap();

    // Should produce positive values (not underflow to zero)
    for (i, &val) in result.iter().enumerate().skip(2) {
        assert!(
            val > 0.0 && !val.is_nan(),
            "EMA[{i}] should be positive for small inputs, got {val}"
        );
    }
}

#[test]
fn numeric_stability_bollinger_large_values() {
    // Bollinger Bands with large values - variance calculation could overflow
    let data = vec![1e150_f64, 1.01e150, 1.02e150, 1.03e150, 1.04e150];
    let result = bollinger(&data, 3, 2.0).unwrap();

    // Should produce finite bands
    for i in 2..result.middle.len() {
        assert!(
            !result.middle[i].is_nan() && !result.middle[i].is_infinite(),
            "Bollinger middle[{i}] should be finite for large values"
        );
        assert!(
            !result.upper[i].is_nan() && !result.upper[i].is_infinite(),
            "Bollinger upper[{i}] should be finite for large values"
        );
        assert!(
            !result.lower[i].is_nan() && !result.lower[i].is_infinite(),
            "Bollinger lower[{i}] should be finite for large values"
        );
    }
}

#[test]
fn numeric_stability_bollinger_small_variance() {
    // Bollinger with very small variance - stddev calculation shouldn't underflow
    let data = vec![1.0_f64, 1.0 + 1e-15, 1.0 + 2e-15, 1.0 + 3e-15, 1.0 + 4e-15];
    let result = bollinger(&data, 3, 2.0).unwrap();

    // Should produce valid bands (even if very narrow)
    for i in 2..result.middle.len() {
        assert!(
            !result.middle[i].is_nan(),
            "Bollinger middle should be valid for small variance"
        );
        // Upper should be >= middle (or equal if variance is 0)
        assert!(
            result.upper[i] >= result.middle[i] - 1e-10,
            "Bollinger upper should be >= middle"
        );
    }
}

#[test]
fn numeric_stability_rsi_large_moves() {
    // RSI with very large price moves
    let data = vec![
        1e100_f64, 2e100, 1e100, 2e100, 1e100, 2e100, 1e100, 2e100, 1e100, 2e100,
    ];
    let result = rsi(&data, 5).unwrap();

    // RSI should still be bounded [0, 100] even with large values
    for (i, &val) in result.iter().enumerate() {
        if !val.is_nan() {
            assert!(
                (0.0..=100.0).contains(&val),
                "RSI[{i}] = {val} should be in [0, 100] for large moves"
            );
        }
    }
}

#[test]
fn numeric_stability_rsi_tiny_moves() {
    // RSI with very tiny price moves (potential precision issues)
    let data = vec![
        1.0_f64,
        1.0 + 1e-14,
        1.0,
        1.0 + 1e-14,
        1.0,
        1.0 + 1e-14,
        1.0,
        1.0 + 1e-14,
        1.0,
        1.0 + 1e-14,
    ];
    let result = rsi(&data, 5).unwrap();

    // RSI should still be bounded [0, 100]
    for (i, &val) in result.iter().enumerate() {
        if !val.is_nan() {
            assert!(
                (0.0..=100.0).contains(&val),
                "RSI[{i}] = {val} should be in [0, 100] for tiny moves"
            );
        }
    }
}

#[test]
fn numeric_stability_atr_large_ranges() {
    // ATR with very large price ranges
    let high = vec![2e100_f64, 2.1e100, 2.2e100, 2.3e100, 2.4e100, 2.5e100];
    let low = vec![1e100_f64, 1.1e100, 1.2e100, 1.3e100, 1.4e100, 1.5e100];
    let close = vec![1.5e100_f64, 1.6e100, 1.7e100, 1.8e100, 1.9e100, 2.0e100];

    let result = atr(&high, &low, &close, 3).unwrap();

    // Should produce finite results
    for (i, &val) in result.iter().enumerate() {
        if !val.is_nan() {
            assert!(
                val.is_finite(),
                "ATR[{i}] should be finite for large ranges, got {val}"
            );
        }
    }
}

#[test]
fn numeric_stability_macd_large_values() {
    // MACD with large values
    let data: Vec<f64> = (0..40).map(|x| 1e100 + f64::from(x) * 1e98).collect();
    let result = macd(&data, 12, 26, 9).unwrap();

    // MACD line should be finite
    for (i, &val) in result.macd_line.iter().enumerate() {
        if !val.is_nan() {
            assert!(
                val.is_finite(),
                "MACD line[{i}] should be finite for large values, got {val}"
            );
        }
    }
}

#[test]
fn numeric_stability_stochastic_extreme_range() {
    // Stochastic with extreme high-low range
    let high = vec![1e100_f64, 1e100, 1e100, 1e100, 1e100, 1e100];
    let low = vec![1e50_f64, 1e50, 1e50, 1e50, 1e50, 1e50];
    let close = vec![5e99_f64, 5e99, 5e99, 5e99, 5e99, 5e99]; // Close at ~50% of range

    let result = stochastic_fast(&high, &low, &close, 3, 2).unwrap();

    // %K should still be in [0, 100]
    for (i, &val) in result.k.iter().enumerate() {
        if !val.is_nan() {
            assert!(
                (0.0..=100.0).contains(&val),
                "%K[{i}] = {val} should be in [0, 100] for extreme range"
            );
        }
    }
}

#[test]
fn numeric_stability_no_panics_for_extreme_inputs() {
    // Verify no panics occur for extreme but valid inputs
    let large = vec![f64::MAX / 2.0; 10];
    let small = vec![f64::MIN_POSITIVE; 10];
    let mixed = vec![
        1e200, -1e200, 1e-200, -1e-200, 0.0, 1e200, -1e200, 1e-200, -1e-200, 0.0,
    ];

    // SMA should not panic
    let _ = sma(&large, 3);
    let _ = sma(&small, 3);
    let _ = sma(&mixed, 3);

    // EMA should not panic
    let _ = ema(&large, 3);
    let _ = ema(&small, 3);
    let _ = ema(&mixed, 3);

    // Bollinger should not panic (may produce infinity/NaN but shouldn't panic)
    let _ = bollinger(&large, 3, 2.0);
    let _ = bollinger(&small, 3, 2.0);
    let _ = bollinger(&mixed, 3, 2.0);
}

#[test]
fn numeric_stability_rolling_sum_precision() {
    // Test that rolling SMA maintains precision over many iterations
    // This tests for error accumulation in the rolling sum
    let n = 10000;
    let data: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.01).collect();
    let result = sma(&data, 100).unwrap();

    // Check a few values for correctness
    // At index 99: SMA of data[0..100] = mean of 100.00 to 100.99
    // Expected: 100.495
    assert!(
        (result[99] - 100.495).abs() < 1e-10,
        "First valid SMA should be accurate: got {}",
        result[99]
    );

    // At index n-1: SMA of data[n-100..n]
    // Expected: mean of (100 + (n-100)*0.01) to (100 + (n-1)*0.01)
    let last_idx = n - 1;
    let expected_last = (0..100)
        .map(|i| 100.0 + ((n - 100 + i) as f64) * 0.01)
        .sum::<f64>()
        / 100.0;
    assert!(
        (result[last_idx] - expected_last).abs() < 1e-8,
        "Last SMA should be accurate after many iterations: expected {}, got {}",
        expected_last,
        result[last_idx]
    );
}

#[test]
fn numeric_stability_catastrophic_cancellation_realistic() {
    // Test variance calculation with realistic financial data magnitudes
    // Real stock prices are typically 1-10000, with small daily variance
    let base = 1000.0_f64; // Realistic stock price
    let data = vec![base, base + 0.5, base + 1.0, base + 1.5, base + 2.0];
    let result = bollinger(&data, 3, 2.0).unwrap();

    // Variance of [1000, 1000.5, 1001] window:
    // Mean = 1000.5, Variance = (0.25 + 0 + 0.25) / 3 = 1/6
    // StdDev = sqrt(1/6) ≈ 0.408
    let expected_stddev = (1.0_f64 / 6.0).sqrt();
    let actual_width = (result.upper[2] - result.middle[2]) / 2.0;

    assert!(
        (actual_width - expected_stddev).abs() < 1e-10,
        "Bollinger should be accurate for realistic financial data: expected stddev {expected_stddev}, got {actual_width}"
    );
}

#[test]
fn numeric_stability_catastrophic_cancellation_documented_limitation() {
    // KNOWN LIMITATION: For extremely large magnitude data (1e10+) with tiny variance,
    // the sum-of-squares variance method loses precision due to catastrophic cancellation.
    // This is documented in bollinger.rs: "Users with such data should pre-scale inputs."
    //
    // This test documents the limitation - it's not a bug but a design tradeoff for O(n) speed.

    let base = 1e10_f64; // Extreme magnitude - outside typical financial data range
    let data = vec![base, base + 1.0, base + 2.0, base + 3.0, base + 4.0];
    let result = bollinger(&data, 3, 2.0).unwrap();

    // The algorithm produces finite output (no panics/NaN)
    assert!(
        result.upper[2].is_finite(),
        "Bollinger should produce finite output even for extreme magnitudes"
    );

    // NOTE: Precision IS lost here. The expected stddev is ~0.816 but actual may differ.
    // This is the documented tradeoff. Users with 1e10+ magnitude data should pre-scale.
    let expected_stddev = (2.0_f64 / 3.0).sqrt();
    let actual_width = (result.upper[2] - result.middle[2]) / 2.0;

    // Just verify it's not wildly wrong in a way that would cause problems
    // (bands should still be ordered correctly and finite)
    assert!(
        result.upper[2] >= result.middle[2],
        "Upper band should still be >= middle even with precision loss"
    );
    assert!(
        result.middle[2] >= result.lower[2],
        "Middle band should still be >= lower even with precision loss"
    );

    // Document the precision loss (this will show during test runs)
    if (actual_width - expected_stddev).abs() > 1.0 {
        // This is expected for extreme magnitudes - documented limitation
        eprintln!(
            "NOTE: Precision loss detected for extreme magnitude (1e10) data. \
             Expected stddev: {expected_stddev:.6}, Actual: {actual_width:.6}. \
             This is documented behavior - pre-scale inputs for such data."
        );
    }
}
