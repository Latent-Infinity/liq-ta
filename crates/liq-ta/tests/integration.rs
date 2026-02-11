//! Integration tests for the public API.
//!
//! These tests validate the ergonomics and usability of the liq-ta public API,
//! ensuring that typical usage patterns work correctly and feel natural.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]

use liq_ta::Error;
use liq_ta::prelude::*;

// Sample price data for testing
fn sample_prices() -> Vec<f64> {
    vec![
        44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 44.5, 43.5, 44.0, 45.0,
        46.0, 46.5, 45.5, 44.5, 45.0,
    ]
}

fn sample_ohlc() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let high = vec![
        45.0, 45.5, 44.5, 45.5, 45.0, 44.0, 43.5, 44.5, 45.5, 46.0, 46.5, 45.5, 44.5, 45.0, 46.0,
        47.0, 47.5, 46.5, 45.5, 46.0,
    ];
    let low = vec![
        43.0, 43.5, 42.5, 43.5, 43.0, 42.0, 41.5, 42.5, 43.5, 44.0, 44.5, 43.5, 42.5, 43.0, 44.0,
        45.0, 45.5, 44.5, 43.5, 44.0,
    ];
    let close = sample_prices();
    (high, low, close)
}

// ==================== Basic Usage Tests ====================

#[test]
fn test_prelude_import_basic() {
    // Verify that `use liq_ta::prelude::*` provides all needed types
    let prices = sample_prices();

    // All indicator functions should be available
    let _sma = sma(&prices, 5).unwrap();
    let _ema = ema(&prices, 5).unwrap();
    let _rsi = rsi(&prices, 5).unwrap();
}

#[test]
fn test_compute_single_indicator() {
    let prices = sample_prices();

    // SMA
    let sma_result = sma(&prices, 5).unwrap();
    assert_eq!(sma_result.len(), prices.len());
    assert_eq!(
        sma_result.iter().take_while(|x| x.is_nan()).count(),
        sma_lookback(5)
    );

    // EMA
    let ema_result = ema(&prices, 5).unwrap();
    assert_eq!(ema_result.len(), prices.len());
    assert_eq!(
        ema_result.iter().take_while(|x| x.is_nan()).count(),
        ema_lookback(5)
    );

    // RSI
    let rsi_result = rsi(&prices, 5).unwrap();
    assert_eq!(rsi_result.len(), prices.len());
    assert_eq!(
        rsi_result.iter().take_while(|x| x.is_nan()).count(),
        rsi_lookback(5)
    );
}

#[test]
fn test_compute_multiple_indicators_on_same_data() {
    let prices = sample_prices();

    // Compute multiple indicators on the same data
    let sma_result = sma(&prices, 5).unwrap();
    let ema_result = ema(&prices, 5).unwrap();
    let rsi_result = rsi(&prices, 5).unwrap();
    let macd_result = macd(&prices, 5, 10, 3).unwrap();

    // All outputs should have same length as input
    assert_eq!(sma_result.len(), prices.len());
    assert_eq!(ema_result.len(), prices.len());
    assert_eq!(rsi_result.len(), prices.len());
    assert_eq!(macd_result.macd_line.len(), prices.len());
}

// ==================== Error Handling Tests ====================

#[test]
fn test_handle_errors_gracefully() {
    let prices = sample_prices();

    // Period too long
    let result = sma(&prices, 100);
    assert!(result.is_err());
    match result {
        Err(Error::InsufficientData { .. }) => {}
        _ => panic!("Expected InsufficientData error"),
    }

    // Empty input
    let empty: Vec<f64> = vec![];
    let result = sma(&empty, 5);
    assert!(result.is_err());
    match result {
        Err(Error::EmptyInput) => {}
        _ => panic!("Expected EmptyInput error"),
    }

    // Invalid period
    let result = sma(&prices, 0);
    assert!(result.is_err());
    match result {
        Err(Error::InvalidPeriod { .. }) => {}
        _ => panic!("Expected InvalidPeriod error"),
    }
}

#[test]
fn test_error_messages_are_actionable() {
    let prices = vec![1.0, 2.0, 3.0];

    let result = sma(&prices, 10);
    let err = result.unwrap_err();
    let msg = format!("{err}");

    // Error message should explain what went wrong
    assert!(
        msg.contains("Insufficient") || msg.contains("insufficient") || msg.contains("data"),
        "Error message should explain the issue: {msg}"
    );
}

// ==================== Pre-allocated Buffer Tests ====================

#[test]
fn test_into_variant_basic_usage() {
    let prices = sample_prices();
    let mut output = vec![0.0; prices.len()];

    // _into variant should work with pre-allocated buffer
    let valid_count = sma_into(&prices, 5, &mut output).unwrap();
    assert_eq!(valid_count, prices.len() - sma_lookback(5));
    assert_eq!(output.len(), prices.len());
}

#[test]
fn test_into_variant_ema() {
    let prices = sample_prices();
    let mut output = vec![0.0; prices.len()];

    let valid_count = ema_into(&prices, 5, &mut output).unwrap();
    assert_eq!(valid_count, prices.len() - ema_lookback(5));

    // First (period-1) values should be NaN
    for i in 0..ema_lookback(5) {
        assert!(output[i].is_nan(), "Expected NaN at index {i}");
    }
}

#[test]
fn test_into_variant_rsi() {
    let prices = sample_prices();
    let mut output = vec![0.0; prices.len()];

    let valid_count = rsi_into(&prices, 5, &mut output).unwrap();
    assert_eq!(valid_count, prices.len() - rsi_lookback(5));
}

#[test]
fn test_buffer_too_small_error() {
    let prices = sample_prices();
    let mut small_buffer = vec![0.0; 5]; // Too small

    let result = sma_into(&prices, 5, &mut small_buffer);
    assert!(result.is_err());
    match result {
        Err(Error::BufferTooSmall { .. }) => {}
        _ => panic!("Expected BufferTooSmall error"),
    }
}

// ==================== Multi-output Indicator Tests ====================

#[test]
fn test_macd_multi_output() {
    let prices = sample_prices();

    let result = macd(&prices, 5, 10, 3).unwrap();

    // All outputs should have same length
    assert_eq!(result.macd_line.len(), prices.len());
    assert_eq!(result.signal_line.len(), prices.len());
    assert_eq!(result.histogram.len(), prices.len());

    // Histogram should be macd_line - signal_line for valid values
    let lookback = macd_signal_lookback(10, 3);
    for i in lookback..prices.len() {
        let expected_hist = result.macd_line[i] - result.signal_line[i];
        assert!(
            (result.histogram[i] - expected_hist).abs() < 1e-10,
            "Histogram mismatch at index {i}"
        );
    }
}

#[test]
fn test_macd_into_separate_buffers() {
    let prices = sample_prices();
    let mut macd_line = vec![0.0; prices.len()];
    let mut signal_line = vec![0.0; prices.len()];
    let mut histogram = vec![0.0; prices.len()];

    let (macd_valid, signal_valid) = macd_into(
        &prices,
        5,
        10,
        3,
        &mut macd_line,
        &mut signal_line,
        &mut histogram,
    )
    .unwrap();

    assert!(macd_valid > 0);
    assert!(signal_valid > 0);
    assert_eq!(macd_line.len(), prices.len());
    assert_eq!(signal_line.len(), prices.len());
    assert_eq!(histogram.len(), prices.len());
}

#[test]
fn test_bollinger_multi_output() {
    let prices = sample_prices();

    let result = bollinger(&prices, 5, 2.0).unwrap();

    // All outputs should have same length
    assert_eq!(result.upper.len(), prices.len());
    assert_eq!(result.middle.len(), prices.len());
    assert_eq!(result.lower.len(), prices.len());

    // Middle should equal SMA
    let sma_result = sma(&prices, 5).unwrap();
    for i in sma_lookback(5)..prices.len() {
        assert!(
            (result.middle[i] - sma_result[i]).abs() < 1e-10,
            "Middle band should equal SMA at index {i}"
        );
    }
}

#[test]
fn test_bollinger_into_output_struct() {
    let prices = sample_prices();
    let mut output = BollingerOutput {
        upper: vec![0.0; prices.len()],
        middle: vec![0.0; prices.len()],
        lower: vec![0.0; prices.len()],
    };

    let valid_count = bollinger_into(&prices, 5, 2.0, &mut output).unwrap();

    assert!(valid_count > 0);
    assert_eq!(output.upper.len(), prices.len());
    assert_eq!(output.middle.len(), prices.len());
    assert_eq!(output.lower.len(), prices.len());
}

#[test]
fn test_stochastic_multi_output() {
    let (high, low, close) = sample_ohlc();

    let result = stochastic_fast(&high, &low, &close, 5, 3).unwrap();

    // All outputs should have same length
    assert_eq!(result.k.len(), close.len());
    assert_eq!(result.d.len(), close.len());

    // %K and %D should be in range [0, 100] for valid values
    for i in stochastic_k_lookback(5)..close.len() {
        if !result.k[i].is_nan() {
            assert!(
                result.k[i] >= 0.0 && result.k[i] <= 100.0,
                "%K out of range at {i}"
            );
        }
    }
}

#[test]
fn test_stochastic_into_output_struct() {
    let (high, low, close) = sample_ohlc();
    let mut output = StochasticOutput {
        k: vec![0.0; close.len()],
        d: vec![0.0; close.len()],
    };

    let (k_valid, d_valid) = stochastic_fast_into(&high, &low, &close, 5, 3, &mut output).unwrap();

    assert!(k_valid > 0);
    assert!(d_valid > 0);
    assert_eq!(output.k.len(), close.len());
    assert_eq!(output.d.len(), close.len());
}

// ==================== OHLC Indicator Tests ====================

#[test]
fn test_atr_with_ohlc_data() {
    let (high, low, close) = sample_ohlc();

    let result = atr(&high, &low, &close, 5).unwrap();
    assert_eq!(result.len(), close.len());

    // ATR should be positive for valid values
    for i in atr_lookback(5)..close.len() {
        assert!(result[i] > 0.0, "ATR should be positive at index {i}");
    }
}

#[test]
fn test_true_range() {
    let (high, low, close) = sample_ohlc();

    let result = true_range(&high, &low, &close).unwrap();
    assert_eq!(result.len(), close.len());

    // First value is NaN (no previous close)
    assert!(result[0].is_nan());

    // TR should be positive for valid values
    for i in 1..close.len() {
        assert!(
            result[i] > 0.0,
            "True Range should be positive at index {i}"
        );
    }
}

// ==================== Lookback Function Tests ====================

#[test]
fn test_lookback_functions_exported() {
    // Verify all lookback functions are accessible and return expected values
    assert_eq!(sma_lookback(20), 19);
    assert_eq!(ema_lookback(20), 19);
    assert_eq!(rsi_lookback(14), 14);
    // MACD line lookback only depends on slow_period
    assert_eq!(macd_line_lookback(26), 25);
    // MACD signal lookback depends on slow_period and signal_period
    assert_eq!(macd_signal_lookback(26, 9), 33);
    assert_eq!(atr_lookback(14), 14);
    assert_eq!(bollinger_lookback(20), 19);
    assert_eq!(stochastic_k_lookback(14), 13);
    assert_eq!(stochastic_d_lookback(14, 3), 15);
}

#[test]
fn test_min_len_functions_exported() {
    // Verify all min_len functions are accessible and return expected values
    assert_eq!(sma_min_len(20), 20);
    assert_eq!(ema_min_len(20), 20);
    assert_eq!(rsi_min_len(14), 15);
    // MACD min_len depends on slow_period and signal_period
    assert_eq!(macd_min_len(26, 9), 34);
    assert_eq!(atr_min_len(14), 15);
    assert_eq!(bollinger_min_len(20), 20);
    assert_eq!(stochastic_min_len(14, 3), 16);
}

#[test]
fn test_lookback_matches_nan_prefix() {
    let prices = sample_prices();

    // For each indicator, verify NaN prefix count matches lookback
    let sma_result = sma(&prices, 5).unwrap();
    assert_eq!(
        sma_result.iter().take_while(|x| x.is_nan()).count(),
        sma_lookback(5)
    );

    let ema_result = ema(&prices, 5).unwrap();
    assert_eq!(
        ema_result.iter().take_while(|x| x.is_nan()).count(),
        ema_lookback(5)
    );

    let rsi_result = rsi(&prices, 5).unwrap();
    assert_eq!(
        rsi_result.iter().take_while(|x| x.is_nan()).count(),
        rsi_lookback(5)
    );
}

// ==================== Wilder EMA Tests ====================

#[test]
fn test_ema_wilder_available() {
    let prices = sample_prices();

    let result = ema_wilder(&prices, 5).unwrap();
    assert_eq!(result.len(), prices.len());

    // Wilder's EMA should differ from standard EMA
    let standard = ema(&prices, 5).unwrap();

    // They should be different (different alpha values)
    let mut different = false;
    for i in ema_lookback(5)..prices.len() {
        if (result[i] - standard[i]).abs() > 1e-10 {
            different = true;
            break;
        }
    }
    assert!(different, "Wilder EMA should differ from standard EMA");
}

// ==================== Rolling Extrema Tests ====================

#[test]
fn test_rolling_extrema_available() {
    let prices = sample_prices();

    let max_result = rolling_max(&prices, 5).unwrap();
    let min_result = rolling_min(&prices, 5).unwrap();

    assert_eq!(max_result.len(), prices.len());
    assert_eq!(min_result.len(), prices.len());

    // Max should be >= corresponding value, min should be <= for valid positions
    for i in rolling_extrema_lookback(5)..prices.len() {
        assert!(
            max_result[i] >= prices[i - 4],
            "Max should be >= lookback window values"
        );
        assert!(min_result[i] <= prices[i], "Min should be <= current value");
    }
}

// ==================== API Discoverability Tests ====================

#[test]
fn test_output_types_accessible() {
    // Verify output types are directly usable
    let prices = sample_prices();
    let (high, low, close) = sample_ohlc();

    let macd_out: MacdOutput<f64> = macd(&prices, 5, 10, 3).unwrap();
    assert!(!macd_out.macd_line.is_empty());

    let boll_out: BollingerOutput<f64> = bollinger(&prices, 5, 2.0).unwrap();
    assert!(!boll_out.middle.is_empty());

    let stoch_out: StochasticOutput<f64> = stochastic_fast(&high, &low, &close, 5, 3).unwrap();
    assert!(!stoch_out.k.is_empty());
}

#[test]
fn test_result_type_alias_works() {
    use liq_ta::Result;

    fn compute_sma(prices: &[f64], period: usize) -> Result<Vec<f64>> {
        sma(prices, period)
    }

    let prices = sample_prices();
    let result = compute_sma(&prices, 5);
    assert!(result.is_ok());
}

// ==================== Stochastic Variants Tests ====================

#[test]
fn test_stochastic_variants() {
    let (high, low, close) = sample_ohlc();

    // Fast stochastic
    let fast = stochastic_fast(&high, &low, &close, 5, 3).unwrap();
    assert_eq!(fast.k.len(), close.len());

    // Slow stochastic
    let slow = stochastic_slow(&high, &low, &close, 5, 3).unwrap();
    assert_eq!(slow.k.len(), close.len());

    // Full stochastic
    let full = stochastic_full(&high, &low, &close, 5, 3, 3).unwrap();
    assert_eq!(full.k.len(), close.len());
}

// ==================== Configuration Type Tests ====================

use liq_ta::indicators::{Bollinger, Macd, Stochastic};

#[test]
fn test_macd_default_compute() {
    let prices: Vec<f64> = (0..50).map(|i| 100.0 + f64::from(i) * 0.5).collect();

    // Default (12, 26, 9)
    let result = Macd::default().compute(&prices).unwrap();
    assert_eq!(result.len(), 50);
    assert!(!result.macd_line[25].is_nan()); // slow_period - 1
}

#[test]
fn test_macd_fluent_api() {
    let prices: Vec<f64> = (0..50).map(|i| 100.0 + f64::from(i) * 0.5).collect();

    // Custom (10, 21, 7)
    let result = Macd::new()
        .with_fast_period(10)
        .with_slow_period(21)
        .with_signal_period(7)
        .compute(&prices)
        .unwrap();

    assert_eq!(result.len(), 50);
    // First valid MACD at slow_period - 1 = 20
    assert!(result.macd_line[19].is_nan());
    assert!(!result.macd_line[20].is_nan());
}

#[test]
fn test_macd_config_getters() {
    let config = Macd::new()
        .with_fast_period(10)
        .with_slow_period(21)
        .with_signal_period(7);

    assert_eq!(config.fast_period(), 10);
    assert_eq!(config.slow_period(), 21);
    assert_eq!(config.signal_period(), 7);
    assert_eq!(config.line_lookback(), 20); // 21 - 1
    assert_eq!(config.signal_lookback(), 26); // 21 + 7 - 2 = 26
    assert_eq!(config.min_len(), 27); // 21 + 7 - 1 = 27
}

#[test]
fn test_bollinger_default_compute() {
    let prices: Vec<f64> = (0..30).map(|i| 100.0 + f64::from(i) * 0.5).collect();

    // Default (20, 2.0)
    let result = Bollinger::default().compute(&prices).unwrap();
    assert_eq!(result.middle.len(), 30);
    assert!(!result.middle[19].is_nan()); // period - 1
}

#[test]
fn test_bollinger_fluent_api() {
    let prices: Vec<f64> = (0..20).map(|i| 100.0 + f64::from(i) * 0.5).collect();

    // Custom (10, 2.5)
    let result = Bollinger::new()
        .with_period(10)
        .with_std_dev(2.5)
        .compute(&prices)
        .unwrap();

    assert_eq!(result.middle.len(), 20);
    // First valid value at period - 1 = 9
    assert!(result.middle[8].is_nan());
    assert!(!result.middle[9].is_nan());
}

#[test]
fn test_bollinger_config_getters() {
    let config = Bollinger::new().with_period(10).with_std_dev(2.5);

    assert_eq!(config.period(), 10);
    assert!((config.std_dev() - 2.5).abs() < 1e-10);
    assert_eq!(config.lookback(), 9); // 10 - 1
    assert_eq!(config.min_len(), 10);
}

#[test]
fn test_stochastic_default_compute() {
    let (high, low, close) = sample_ohlc();

    // Default is now fast stochastic (k_slowing=1), test with slow (k_slowing=3)
    let result = Stochastic::new()
        .with_k_period(5)
        .with_d_period(3)
        .with_k_slowing(3)
        .compute(&high, &low, &close)
        .unwrap();

    assert_eq!(result.k.len(), close.len());
    assert_eq!(result.d.len(), close.len());
}

#[test]
fn test_stochastic_fast_constructor() {
    let (high, low, close) = sample_ohlc();

    // Fast stochastic (k_smooth = 1)
    let result = Stochastic::fast(5, 3).compute(&high, &low, &close).unwrap();

    // Should produce same result as stochastic_fast function
    let direct = stochastic_fast(&high, &low, &close, 5, 3).unwrap();

    for i in 0..close.len() {
        if result.k[i].is_nan() && direct.k[i].is_nan() {
            continue;
        }
        assert!(
            (result.k[i] - direct.k[i]).abs() < 1e-10,
            "Mismatch at index {}: {} vs {}",
            i,
            result.k[i],
            direct.k[i]
        );
    }
}

#[test]
fn test_stochastic_slow_constructor() {
    let (high, low, close) = sample_ohlc();

    // Slow stochastic (k_smooth = 3)
    let result = Stochastic::slow(5, 3).compute(&high, &low, &close).unwrap();

    assert_eq!(result.k.len(), close.len());
    assert_eq!(result.d.len(), close.len());
}

#[test]
fn test_stochastic_config_getters() {
    let config = Stochastic::new()
        .with_k_period(14)
        .with_d_period(5)
        .with_k_slowing(3);

    assert_eq!(config.k_period(), 14);
    assert_eq!(config.d_period(), 5);
    assert_eq!(config.k_slowing(), 3);
    assert_eq!(config.k_lookback(), 13); // 14 - 1
    assert_eq!(config.d_lookback(), 17); // 14 + 5 - 2
    assert_eq!(config.min_len(), 18); // 14 + 5 - 1
}

#[test]
fn test_config_types_reusable() {
    let prices1: Vec<f64> = (0..50).map(|i| 100.0 + f64::from(i)).collect();
    let prices2: Vec<f64> = (0..50).map(|i| 200.0 - f64::from(i)).collect();

    // Same config can be reused for multiple computations
    let macd_config = Macd::new()
        .with_fast_period(5)
        .with_slow_period(10)
        .with_signal_period(3);

    let result1 = macd_config.compute(&prices1).unwrap();
    let result2 = macd_config.compute(&prices2).unwrap();

    // Results should be different (different input data)
    assert!(result1.macd_line[15] != result2.macd_line[15]);
}

#[test]
fn test_config_types_invalid_params_fail_fast() {
    let prices: Vec<f64> = (0..50).map(|i| 100.0 + f64::from(i)).collect();

    // MACD with fast >= slow should error
    let result = Macd::new()
        .with_fast_period(26)
        .with_slow_period(12)
        .with_signal_period(9)
        .compute(&prices);
    assert!(result.is_err());

    // MACD with zero period should error
    let result = Macd::new()
        .with_fast_period(0)
        .with_slow_period(26)
        .with_signal_period(9)
        .compute(&prices);
    assert!(result.is_err());
}
