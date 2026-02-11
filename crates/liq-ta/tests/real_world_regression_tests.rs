//! Real-world regression suite.
//!
//! This test suite generates synthetic but realistic OHLCV market data using a
//! deterministic seeded RNG, then runs all indicators to verify:
//! - No panics on realistic data
//! - No unexpected NaN blowups (NaN only in lookback period)
//! - Output lengths match lookback functions
//! - Bounds are respected (RSI 0-100, Stochastic 0-100, etc.)
//!
//! The synthetic data includes realistic market characteristics:
//! - Trends (uptrend, downtrend)
//! - Range-bound periods
//! - Volatility clusters
//! - Overnight gaps
//! - High/low wicks
//!
//! Per PRD §7.7: "Synthetic fixtures test edge cases, but real-world data tests robustness."

#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
#![allow(clippy::manual_clamp)]

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use liq_ta::indicators::{
    atr::{atr, atr_lookback, true_range, true_range_lookback},
    bollinger::{bollinger, bollinger_lookback},
    ema::{ema, ema_lookback, ema_wilder},
    macd::{macd, macd_line_lookback, macd_signal_lookback},
    rsi::{rsi, rsi_lookback},
    sma::{sma, sma_lookback},
    stochastic::{stochastic_d_lookback, stochastic_fast, stochastic_k_lookback},
};
use liq_ta::kernels::rolling_extrema::{rolling_extrema_lookback, rolling_max, rolling_min};

/// Fixed seed for reproducible test data generation.
/// This ensures the same dataset is generated on every test run.
/// 0xFA57 resembles "FAST", 2025 is the year.
const SEED: u64 = 0xFA57_0000_2025;

/// Number of data points to generate (simulates ~2-3 years of daily data).
const DATA_POINTS: usize = 750;

/// Starting price for the synthetic asset.
const STARTING_PRICE: f64 = 100.0;

/// OHLCV data structure for synthetic market data.
#[derive(Debug, Clone)]
struct OhlcvData {
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
}

/// Market regime for generating realistic price patterns.
#[derive(Debug, Clone, Copy)]
enum MarketRegime {
    Uptrend,
    Downtrend,
    RangeBound,
    HighVolatility,
}

/// Generates synthetic OHLCV data with realistic market characteristics.
///
/// The generator creates data with:
/// - Multiple market regimes (trends, ranges, volatility clusters)
/// - Realistic OHLC relationships (high >= max(open,close), low <= min(open,close))
/// - Occasional gaps (simulating overnight/weekend moves)
/// - Volume patterns correlated with volatility
fn generate_synthetic_ohlcv(seed: u64, num_points: usize) -> OhlcvData {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let mut open = Vec::with_capacity(num_points);
    let mut high = Vec::with_capacity(num_points);
    let mut low = Vec::with_capacity(num_points);
    let mut close = Vec::with_capacity(num_points);
    let mut volume = Vec::with_capacity(num_points);

    let mut current_price = STARTING_PRICE;
    let mut current_regime = MarketRegime::RangeBound;
    let mut regime_duration = 0;

    for i in 0..num_points {
        // Change regime periodically
        if regime_duration <= 0 {
            current_regime = match rng.gen_range(0..4) {
                0 => MarketRegime::Uptrend,
                1 => MarketRegime::Downtrend,
                2 => MarketRegime::RangeBound,
                _ => MarketRegime::HighVolatility,
            };
            regime_duration = rng.gen_range(20..80);
        }
        regime_duration -= 1;

        // Calculate daily volatility based on regime
        let base_volatility = match current_regime {
            MarketRegime::Uptrend => 0.01,
            MarketRegime::Downtrend => 0.012,
            MarketRegime::RangeBound => 0.008,
            MarketRegime::HighVolatility => 0.025,
        };

        // Add trend drift based on regime
        let drift = match current_regime {
            MarketRegime::Uptrend => 0.001,
            MarketRegime::Downtrend => -0.001,
            MarketRegime::RangeBound => 0.0,
            MarketRegime::HighVolatility => rng.gen_range(-0.002..0.002),
        };

        // Generate open price (with occasional gap)
        let gap_probability = 0.05;
        let open_price = if i == 0 {
            current_price
        } else if rng.r#gen::<f64>() < gap_probability {
            // Gap open
            let gap_size = rng.gen_range(-0.03..0.03);
            current_price * (1.0 + gap_size)
        } else {
            // Normal open near previous close
            current_price * (1.0 + rng.gen_range(-0.002..0.002))
        };

        // Generate close price with drift and volatility
        let daily_return = drift + rng.gen_range(-base_volatility..base_volatility);
        let close_price = open_price * (1.0 + daily_return);

        // Generate high and low with wicks
        let wick_factor = rng.gen_range(0.0..base_volatility * 2.0);
        let high_price = open_price.max(close_price) * (1.0 + wick_factor);
        let low_price = open_price.min(close_price) * (1.0 - wick_factor);

        // Generate volume (higher in volatile regimes)
        let base_volume = 1_000_000.0;
        let volume_multiplier = match current_regime {
            MarketRegime::HighVolatility => rng.gen_range(1.5..3.0),
            MarketRegime::Uptrend | MarketRegime::Downtrend => rng.gen_range(0.8..1.5),
            MarketRegime::RangeBound => rng.gen_range(0.5..1.0),
        };
        let daily_volume = base_volume * volume_multiplier * rng.gen_range(0.7..1.3);

        // Ensure price doesn't go negative or too extreme
        let high_price = high_price.max(0.01).min(10000.0);
        let low_price = low_price.max(0.01).min(high_price);
        let open_price = open_price.clamp(low_price, high_price);
        let close_price = close_price.clamp(low_price, high_price);

        open.push(open_price);
        high.push(high_price);
        low.push(low_price);
        close.push(close_price);
        volume.push(daily_volume);

        current_price = close_price;
    }

    OhlcvData {
        open,
        high,
        low,
        close,
        volume,
    }
}

/// Verifies that a result vector has the expected NaN count based on lookback.
fn verify_nan_count(result: &[f64], expected_nan_count: usize, indicator_name: &str) {
    let actual_nan_count = result.iter().filter(|x| x.is_nan()).count();
    assert_eq!(
        actual_nan_count, expected_nan_count,
        "{indicator_name}: expected {expected_nan_count} NaN values, got {actual_nan_count}"
    );

    // Verify NaN values are at the beginning
    for (i, &val) in result.iter().enumerate() {
        if i < expected_nan_count {
            assert!(
                val.is_nan(),
                "{indicator_name}: expected NaN at index {i}, got {val}"
            );
        } else {
            assert!(
                !val.is_nan(),
                "{indicator_name}: unexpected NaN at index {i} (past lookback)"
            );
        }
    }
}

/// Verifies that all values are within expected bounds.
fn verify_bounds(result: &[f64], min: f64, max: f64, indicator_name: &str) {
    for (i, &val) in result.iter().enumerate() {
        if !val.is_nan() {
            assert!(
                val >= min && val <= max,
                "{indicator_name}: value {val} at index {i} is out of bounds [{min}, {max}]"
            );
        }
    }
}

// ==================== Data Generation Tests ====================

#[test]
fn regression_data_generation_deterministic() {
    // Verify that the data generation is deterministic
    let data1 = generate_synthetic_ohlcv(SEED, 100);
    let data2 = generate_synthetic_ohlcv(SEED, 100);

    assert_eq!(
        data1.close, data2.close,
        "Data generation should be deterministic"
    );
    assert_eq!(data1.high, data2.high);
    assert_eq!(data1.low, data2.low);
    assert_eq!(data1.open, data2.open);
}

#[test]
fn regression_data_ohlc_relationships() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    for i in 0..DATA_POINTS {
        // High should be >= max(open, close)
        assert!(
            data.high[i] >= data.open[i] && data.high[i] >= data.close[i],
            "High should be >= open and close at index {i}"
        );

        // Low should be <= min(open, close)
        assert!(
            data.low[i] <= data.open[i] && data.low[i] <= data.close[i],
            "Low should be <= open and close at index {i}"
        );

        // High >= Low
        assert!(
            data.high[i] >= data.low[i],
            "High should be >= Low at index {i}"
        );

        // All prices positive
        assert!(data.open[i] > 0.0, "Open should be positive at index {i}");
        assert!(data.high[i] > 0.0, "High should be positive at index {i}");
        assert!(data.low[i] > 0.0, "Low should be positive at index {i}");
        assert!(data.close[i] > 0.0, "Close should be positive at index {i}");
        assert!(
            data.volume[i] > 0.0,
            "Volume should be positive at index {i}"
        );
    }
}

// ==================== SMA Regression Tests ====================

#[test]
fn regression_sma_typical_periods() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    for period in [5, 10, 20, 50, 100, 200] {
        let result = sma(&data.close, period).unwrap();

        assert_eq!(
            result.len(),
            data.close.len(),
            "SMA({period}) output length"
        );
        verify_nan_count(&result, sma_lookback(period), &format!("SMA({period})"));

        // SMA should be within the price range
        let min_price = data.close.iter().copied().fold(f64::INFINITY, f64::min);
        let max_price = data.close.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        verify_bounds(
            &result,
            min_price * 0.5,
            max_price * 1.5,
            &format!("SMA({period})"),
        );
    }
}

// ==================== EMA Regression Tests ====================

#[test]
fn regression_ema_typical_periods() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    for period in [5, 12, 26, 50, 100, 200] {
        let result = ema(&data.close, period).unwrap();

        assert_eq!(
            result.len(),
            data.close.len(),
            "EMA({period}) output length"
        );
        verify_nan_count(&result, ema_lookback(period), &format!("EMA({period})"));
    }
}

#[test]
fn regression_ema_wilder_typical_periods() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    for period in [7, 14, 21] {
        let result = ema_wilder(&data.close, period).unwrap();

        assert_eq!(
            result.len(),
            data.close.len(),
            "EMA_Wilder({period}) output length"
        );
        verify_nan_count(
            &result,
            ema_lookback(period),
            &format!("EMA_Wilder({period})"),
        );
    }
}

// ==================== RSI Regression Tests ====================

#[test]
fn regression_rsi_typical_periods() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    for period in [7, 14, 21] {
        let result = rsi(&data.close, period).unwrap();

        assert_eq!(
            result.len(),
            data.close.len(),
            "RSI({period}) output length"
        );
        verify_nan_count(&result, rsi_lookback(period), &format!("RSI({period})"));

        // RSI must be in [0, 100]
        verify_bounds(&result, 0.0, 100.0, &format!("RSI({period})"));
    }
}

// ==================== MACD Regression Tests ====================

#[test]
fn regression_macd_standard() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    let fast = 12;
    let slow = 26;
    let signal = 9;

    let result = macd(&data.close, fast, slow, signal).unwrap();

    assert_eq!(result.macd_line.len(), data.close.len(), "MACD line length");
    assert_eq!(
        result.signal_line.len(),
        data.close.len(),
        "Signal line length"
    );
    assert_eq!(result.histogram.len(), data.close.len(), "Histogram length");

    // Verify NaN counts
    let macd_nan = result.macd_line.iter().filter(|x| x.is_nan()).count();
    let signal_nan = result.signal_line.iter().filter(|x| x.is_nan()).count();
    let hist_nan = result.histogram.iter().filter(|x| x.is_nan()).count();

    assert_eq!(macd_nan, macd_line_lookback(slow), "MACD line NaN count");
    assert_eq!(
        signal_nan,
        macd_signal_lookback(slow, signal),
        "Signal line NaN count"
    );
    assert_eq!(
        hist_nan,
        macd_signal_lookback(slow, signal),
        "Histogram NaN count"
    );

    // Verify histogram = MACD - Signal
    for i in macd_signal_lookback(slow, signal)..data.close.len() {
        let expected = result.macd_line[i] - result.signal_line[i];
        assert!(
            (result.histogram[i] - expected).abs() < 1e-10,
            "Histogram should equal MACD - Signal at index {i}"
        );
    }
}

// ==================== ATR Regression Tests ====================

#[test]
fn regression_atr_typical_periods() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    for period in [7, 14, 21] {
        let result = atr(&data.high, &data.low, &data.close, period).unwrap();

        assert_eq!(
            result.len(),
            data.close.len(),
            "ATR({period}) output length"
        );
        verify_nan_count(&result, atr_lookback(period), &format!("ATR({period})"));

        // ATR must be non-negative
        verify_bounds(&result, 0.0, f64::INFINITY, &format!("ATR({period})"));
    }
}

#[test]
fn regression_true_range() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    let result = true_range(&data.high, &data.low, &data.close).unwrap();

    assert_eq!(result.len(), data.close.len(), "TR output length");
    verify_nan_count(&result, true_range_lookback(), "True Range");

    // True Range must be non-negative
    verify_bounds(&result, 0.0, f64::INFINITY, "True Range");
}

// ==================== Bollinger Bands Regression Tests ====================

#[test]
fn regression_bollinger_typical() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    let period = 20;
    let num_std = 2.0;

    let result = bollinger(&data.close, period, num_std).unwrap();

    assert_eq!(
        result.middle.len(),
        data.close.len(),
        "Bollinger middle length"
    );
    assert_eq!(
        result.upper.len(),
        data.close.len(),
        "Bollinger upper length"
    );
    assert_eq!(
        result.lower.len(),
        data.close.len(),
        "Bollinger lower length"
    );

    let expected_nan = bollinger_lookback(period);
    verify_nan_count(&result.middle, expected_nan, "Bollinger middle");
    verify_nan_count(&result.upper, expected_nan, "Bollinger upper");
    verify_nan_count(&result.lower, expected_nan, "Bollinger lower");

    // Verify band ordering: upper >= middle >= lower
    for i in expected_nan..data.close.len() {
        assert!(
            result.upper[i] >= result.middle[i],
            "Upper >= Middle at index {i}"
        );
        assert!(
            result.middle[i] >= result.lower[i],
            "Middle >= Lower at index {i}"
        );
    }
}

// ==================== Stochastic Regression Tests ====================

#[test]
fn regression_stochastic_typical() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    let k_period = 14;
    let d_period = 3;

    let result = stochastic_fast(&data.high, &data.low, &data.close, k_period, d_period).unwrap();

    assert_eq!(result.k.len(), data.close.len(), "Stochastic %K length");
    assert_eq!(result.d.len(), data.close.len(), "Stochastic %D length");

    verify_nan_count(&result.k, stochastic_k_lookback(k_period), "Stochastic %K");
    verify_nan_count(
        &result.d,
        stochastic_d_lookback(k_period, d_period),
        "Stochastic %D",
    );

    // Stochastic must be in [0, 100]
    verify_bounds(&result.k, 0.0, 100.0, "Stochastic %K");
    verify_bounds(&result.d, 0.0, 100.0, "Stochastic %D");
}

// ==================== Rolling Extrema Regression Tests ====================

#[test]
fn regression_rolling_extrema_typical() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    for period in [5, 14, 50] {
        let max_result = rolling_max(&data.high, period).unwrap();
        let min_result = rolling_min(&data.low, period).unwrap();

        assert_eq!(
            max_result.len(),
            data.high.len(),
            "Rolling max({period}) output length"
        );
        assert_eq!(
            min_result.len(),
            data.low.len(),
            "Rolling min({period}) output length"
        );

        verify_nan_count(
            &max_result,
            rolling_extrema_lookback(period),
            &format!("Rolling max({period})"),
        );
        verify_nan_count(
            &min_result,
            rolling_extrema_lookback(period),
            &format!("Rolling min({period})"),
        );

        // Rolling max should be >= current value
        // Rolling min should be <= current value
        for i in rolling_extrema_lookback(period)..data.high.len() {
            assert!(
                max_result[i] >= data.high[i],
                "Rolling max >= current high at index {i}"
            );
            assert!(
                min_result[i] <= data.low[i],
                "Rolling min <= current low at index {i}"
            );
        }
    }
}

// ==================== Combined Workflow Tests ====================

#[test]
fn regression_typical_trading_workflow() {
    // Simulate a typical trading system that uses multiple indicators
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    // Trend indicators
    let sma_20 = sma(&data.close, 20).unwrap();
    let sma_50 = sma(&data.close, 50).unwrap();
    let ema_12 = ema(&data.close, 12).unwrap();
    let ema_26 = ema(&data.close, 26).unwrap();

    // Momentum indicators
    let rsi_14 = rsi(&data.close, 14).unwrap();
    let macd_result = macd(&data.close, 12, 26, 9).unwrap();
    let stoch = stochastic_fast(&data.high, &data.low, &data.close, 14, 3).unwrap();

    // Volatility indicators
    let atr_14 = atr(&data.high, &data.low, &data.close, 14).unwrap();
    let bb = bollinger(&data.close, 20, 2.0).unwrap();

    // Verify all outputs have correct length
    assert_eq!(sma_20.len(), DATA_POINTS);
    assert_eq!(sma_50.len(), DATA_POINTS);
    assert_eq!(ema_12.len(), DATA_POINTS);
    assert_eq!(ema_26.len(), DATA_POINTS);
    assert_eq!(rsi_14.len(), DATA_POINTS);
    assert_eq!(macd_result.macd_line.len(), DATA_POINTS);
    assert_eq!(stoch.k.len(), DATA_POINTS);
    assert_eq!(atr_14.len(), DATA_POINTS);
    assert_eq!(bb.middle.len(), DATA_POINTS);

    // Count valid (non-NaN) values after longest lookback
    let valid_start = 50; // After SMA(50) lookback
    let mut valid_count = 0;
    for i in valid_start..DATA_POINTS {
        if !sma_20[i].is_nan()
            && !sma_50[i].is_nan()
            && !rsi_14[i].is_nan()
            && !macd_result.signal_line[i].is_nan()
        {
            valid_count += 1;
        }
    }

    assert!(
        valid_count > DATA_POINTS - 100,
        "Should have many valid combined signals: {valid_count}"
    );
}

#[test]
fn regression_chained_indicators() {
    // Test chaining indicators (e.g., RSI of RSI)
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    // First RSI
    let rsi_14 = rsi(&data.close, 14).unwrap();

    // Get only the valid (non-NaN) values for the second RSI
    let valid_rsi: Vec<f64> = rsi_14.iter().filter(|x| !x.is_nan()).copied().collect();

    if valid_rsi.len() >= 15 {
        // RSI of RSI (smoothed RSI)
        let rsi_of_rsi = rsi(&valid_rsi, 7).unwrap();

        // Verify it produces valid output
        let valid_count = rsi_of_rsi.iter().filter(|x| !x.is_nan()).count();
        assert!(valid_count > 0, "RSI of RSI should have valid values");

        // RSI of RSI should still be bounded [0, 100]
        verify_bounds(&rsi_of_rsi, 0.0, 100.0, "RSI of RSI");
    }
}

// ==================== Stress Tests ====================

#[test]
fn regression_stress_all_indicators_no_panic() {
    // Generate multiple datasets with different seeds and verify no panics
    for seed_offset in 0..5 {
        let data = generate_synthetic_ohlcv(SEED + seed_offset, DATA_POINTS);

        // Run all indicators - the test passes if no panic occurs
        let _ = sma(&data.close, 20).unwrap();
        let _ = ema(&data.close, 20).unwrap();
        let _ = rsi(&data.close, 14).unwrap();
        let _ = macd(&data.close, 12, 26, 9).unwrap();
        let _ = atr(&data.high, &data.low, &data.close, 14).unwrap();
        let _ = bollinger(&data.close, 20, 2.0).unwrap();
        let _ = stochastic_fast(&data.high, &data.low, &data.close, 14, 3).unwrap();
        let _ = rolling_max(&data.high, 14).unwrap();
        let _ = rolling_min(&data.low, 14).unwrap();
    }
}

#[test]
fn regression_edge_periods() {
    let data = generate_synthetic_ohlcv(SEED, DATA_POINTS);

    // Test with period = 1 (edge case)
    let sma_1 = sma(&data.close, 1).unwrap();
    assert_eq!(sma_1.len(), DATA_POINTS);
    // SMA(1) should equal the input
    for i in 0..DATA_POINTS {
        assert!(
            (sma_1[i] - data.close[i]).abs() < 1e-10,
            "SMA(1) should equal input"
        );
    }

    // Test with large periods
    let sma_200 = sma(&data.close, 200).unwrap();
    assert_eq!(sma_200.len(), DATA_POINTS);
    verify_nan_count(&sma_200, sma_lookback(200), "SMA(200)");
}
