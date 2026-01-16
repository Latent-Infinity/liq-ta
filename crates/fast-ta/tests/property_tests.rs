//! Property-based tests for all indicators using proptest.
//!
//! These tests verify invariant properties that must hold for all valid inputs,
//! using randomly generated test data to find edge cases.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]

use proptest::prelude::*;

use fast_ta::indicators::{
    ad::ad,
    adx::adx,
    atr::{atr, true_range},
    bollinger::bollinger,
    donchian::donchian,
    ema::ema,
    macd::macd,
    mfi::mfi,
    obv::obv,
    price_transform::{avgprice, medprice, typprice, wclprice},
    roc::roc,
    rsi::rsi,
    sma::sma,
    statistics::var,
    stochastic::stochastic_fast,
    vwap::vwap,
    williams_r::williams_r,
};
use fast_ta::kernels::rolling_extrema::{rolling_max, rolling_min};

// ==================== Test Data Generators ====================

/// Generate a random price series (all positive values)
fn arb_price_series(min_len: usize, max_len: usize) -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(1.0..1000.0_f64, min_len..=max_len)
}

/// Generate a random OHLC series with valid constraints (high >= open, close; low <= open, close)
fn arb_ohlc_series(
    min_len: usize,
    max_len: usize,
) -> impl Strategy<Value = (Vec<f64>, Vec<f64>, Vec<f64>)> {
    prop::collection::vec(
        (1.0..1000.0_f64, 0.0..0.1_f64, 0.0..0.1_f64),
        min_len..=max_len,
    )
    .prop_map(|data| {
        let mut high = Vec::with_capacity(data.len());
        let mut low = Vec::with_capacity(data.len());
        let mut close = Vec::with_capacity(data.len());

        for (base, high_pct, low_pct) in data {
            let h = base * (1.0 + high_pct);
            let l = base * (1.0 - low_pct);
            let c = base; // close at base price
            high.push(h);
            low.push(l);
            close.push(c);
        }

        (high, low, close)
    })
}

/// Generate a random OHLC series with Open prices for avgprice
fn arb_ohlc_with_open_series(
    min_len: usize,
    max_len: usize,
) -> impl Strategy<Value = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
    prop::collection::vec(
        (1.0..1000.0_f64, 0.0..0.1_f64, 0.0..0.1_f64, 0.0..0.1_f64),
        min_len..=max_len,
    )
    .prop_map(|data| {
        let mut open = Vec::with_capacity(data.len());
        let mut high = Vec::with_capacity(data.len());
        let mut low = Vec::with_capacity(data.len());
        let mut close = Vec::with_capacity(data.len());

        for (base, high_pct, low_pct, open_offset) in data {
            let h = base * (1.0 + high_pct);
            let l = base * (1.0 - low_pct);
            let o = l + (h - l) * open_offset;
            let c = base;
            open.push(o);
            high.push(h);
            low.push(l);
            close.push(c);
        }

        (open, high, low, close)
    })
}

/// Generate a random OHLCV series with volume data
fn arb_ohlcv_series(
    min_len: usize,
    max_len: usize,
) -> impl Strategy<Value = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
    prop::collection::vec(
        (1.0..1000.0_f64, 0.0..0.1_f64, 0.0..0.1_f64, 100.0..10000.0_f64),
        min_len..=max_len,
    )
    .prop_map(|data| {
        let mut high = Vec::with_capacity(data.len());
        let mut low = Vec::with_capacity(data.len());
        let mut close = Vec::with_capacity(data.len());
        let mut volume = Vec::with_capacity(data.len());

        for (base, high_pct, low_pct, vol) in data {
            let h = base * (1.0 + high_pct);
            let l = base * (1.0 - low_pct);
            let c = base;
            high.push(h);
            low.push(l);
            close.push(c);
            volume.push(vol);
        }

        (high, low, close, volume)
    })
}

/// Generate close and volume series for OBV
fn arb_close_volume_series(
    min_len: usize,
    max_len: usize,
) -> impl Strategy<Value = (Vec<f64>, Vec<f64>)> {
    prop::collection::vec(
        (1.0..1000.0_f64, 100.0..10000.0_f64),
        min_len..=max_len,
    )
    .prop_map(|data| {
        let mut close = Vec::with_capacity(data.len());
        let mut volume = Vec::with_capacity(data.len());

        for (c, v) in data {
            close.push(c);
            volume.push(v);
        }

        (close, volume)
    })
}

// ==================== SMA Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// SMA output length equals input length
    #[test]
    fn prop_sma_output_length(data in arb_price_series(5, 100), period in 1usize..=10) {
        if data.len() >= period {
            let result = sma(&data, period).unwrap();
            prop_assert_eq!(result.len(), data.len());
        }
    }

    /// SMA has exactly period-1 NaN values at the start
    #[test]
    fn prop_sma_nan_count(data in arb_price_series(5, 100), period in 1usize..=10) {
        if data.len() >= period {
            let result = sma(&data, period).unwrap();
            let nan_count = result.iter().filter(|x| x.is_nan()).count();
            prop_assert_eq!(nan_count, period - 1);
        }
    }

    /// SMA of constant values equals that constant
    #[test]
    fn prop_sma_constant_input(constant in 1.0..1000.0_f64, len in 5usize..50, period in 1usize..=10) {
        if len >= period {
            let data = vec![constant; len];
            let result = sma(&data, period).unwrap();

            for i in (period - 1)..len {
                prop_assert!(
                    (result[i] - constant).abs() < 1e-10,
                    "SMA of constant {} at index {} is {}", constant, i, result[i]
                );
            }
        }
    }
}

// ==================== EMA Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// EMA output length equals input length
    #[test]
    fn prop_ema_output_length(data in arb_price_series(5, 100), period in 1usize..=10) {
        if data.len() >= period {
            let result = ema(&data, period).unwrap();
            prop_assert_eq!(result.len(), data.len());
        }
    }

    /// EMA has exactly period-1 NaN values at the start
    #[test]
    fn prop_ema_nan_count(data in arb_price_series(5, 100), period in 1usize..=10) {
        if data.len() >= period {
            let result = ema(&data, period).unwrap();
            let nan_count = result.iter().filter(|x| x.is_nan()).count();
            prop_assert_eq!(nan_count, period - 1);
        }
    }

    /// EMA of constant values equals that constant
    #[test]
    fn prop_ema_constant_input(constant in 1.0..1000.0_f64, len in 5usize..50, period in 1usize..=10) {
        if len >= period {
            let data = vec![constant; len];
            let result = ema(&data, period).unwrap();

            for i in (period - 1)..len {
                prop_assert!(
                    (result[i] - constant).abs() < 1e-10,
                    "EMA of constant {} at index {} is {}", constant, i, result[i]
                );
            }
        }
    }
}

// ==================== RSI Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// RSI output length equals input length
    #[test]
    fn prop_rsi_output_length(data in arb_price_series(5, 100), period in 1usize..=10) {
        if data.len() > period {
            let result = rsi(&data, period).unwrap();
            prop_assert_eq!(result.len(), data.len());
        }
    }

    /// RSI values are in range [0, 100]
    #[test]
    fn prop_rsi_bounded(data in arb_price_series(5, 100), period in 1usize..=10) {
        if data.len() > period {
            let result = rsi(&data, period).unwrap();

            for (i, &val) in result.iter().enumerate() {
                if !val.is_nan() {
                    prop_assert!(
                        (0.0..=100.0).contains(&val),
                        "RSI at index {} is out of bounds: {}", i, val
                    );
                }
            }
        }
    }

    /// RSI of strictly increasing prices equals 100
    #[test]
    fn prop_rsi_all_gains(start in 1.0..100.0_f64, step in 0.1..5.0_f64, len in 5usize..20, period in 1usize..=5) {
        if len > period {
            let data: Vec<f64> = (0..len).map(|i| start + step * (i as f64)).collect();
            let result = rsi(&data, period).unwrap();

            for i in period..len {
                prop_assert!(
                    (result[i] - 100.0).abs() < 1e-6,
                    "RSI of increasing prices at {} should be 100, got {}", i, result[i]
                );
            }
        }
    }

    /// RSI of strictly decreasing prices equals 0
    #[test]
    fn prop_rsi_all_losses(start in 100.0..200.0_f64, step in 0.1..5.0_f64, len in 5usize..20, period in 1usize..=5) {
        if len > period {
            let data: Vec<f64> = (0..len).map(|i| start - step * (i as f64)).filter(|&x| x > 0.0).collect();
            if data.len() > period {
                let result = rsi(&data, period).unwrap();

                for i in period..data.len() {
                    prop_assert!(
                        result[i].abs() < 1e-6,
                        "RSI of decreasing prices at {} should be 0, got {}", i, result[i]
                    );
                }
            }
        }
    }
}

// ==================== MACD Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// MACD output lengths equal input length
    #[test]
    fn prop_macd_output_length(data in arb_price_series(40, 100)) {
        let fast = 12;
        let slow = 26;
        let signal = 9;
        let min_required = slow + signal - 1;

        if data.len() >= min_required {
            let result = macd(&data, fast, slow, signal).unwrap();
            prop_assert_eq!(result.macd_line.len(), data.len());
            prop_assert_eq!(result.signal_line.len(), data.len());
            prop_assert_eq!(result.histogram.len(), data.len());
        }
    }

    /// MACD histogram = MACD line - signal line
    #[test]
    fn prop_macd_histogram_definition(data in arb_price_series(50, 100)) {
        let fast = 12;
        let slow = 26;
        let signal = 9;

        if data.len() >= slow + signal - 1 {
            let result = macd(&data, fast, slow, signal).unwrap();

            for i in 0..data.len() {
                if !result.histogram[i].is_nan() {
                    let expected = result.macd_line[i] - result.signal_line[i];
                    prop_assert!(
                        (result.histogram[i] - expected).abs() < 1e-10,
                        "Histogram[{}] = {} != MACD - Signal = {}",
                        i, result.histogram[i], expected
                    );
                }
            }
        }
    }
}

// ==================== Bollinger Bands Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Bollinger bands maintain upper >= middle >= lower
    #[test]
    fn prop_bollinger_band_order(data in arb_price_series(25, 100), period in 5usize..=20) {
        if data.len() >= period {
            let result = bollinger(&data, period, 2.0).unwrap();

            for i in (period - 1)..data.len() {
                if !result.middle[i].is_nan() {
                    prop_assert!(
                        result.upper[i] >= result.middle[i],
                        "Upper {} < Middle {} at index {}", result.upper[i], result.middle[i], i
                    );
                    prop_assert!(
                        result.middle[i] >= result.lower[i],
                        "Middle {} < Lower {} at index {}", result.middle[i], result.lower[i], i
                    );
                }
            }
        }
    }

    /// Bollinger bands are symmetric around middle
    #[test]
    fn prop_bollinger_symmetric(data in arb_price_series(25, 100), period in 5usize..=20) {
        if data.len() >= period {
            let result = bollinger(&data, period, 2.0).unwrap();

            for i in (period - 1)..data.len() {
                if !result.middle[i].is_nan() {
                    let upper_diff = result.upper[i] - result.middle[i];
                    let lower_diff = result.middle[i] - result.lower[i];
                    prop_assert!(
                        (upper_diff - lower_diff).abs() < 1e-10,
                        "Bands not symmetric at {}: upper_diff={}, lower_diff={}",
                        i, upper_diff, lower_diff
                    );
                }
            }
        }
    }
}

// ==================== ATR Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// ATR is always non-negative
    #[test]
    fn prop_atr_non_negative((high, low, close) in arb_ohlc_series(20, 100), period in 1usize..=14) {
        if high.len() > period {
            let result = atr(&high, &low, &close, period).unwrap();

            for (i, &val) in result.iter().enumerate() {
                if !val.is_nan() {
                    prop_assert!(
                        val >= 0.0,
                        "ATR at index {} is negative: {}", i, val
                    );
                }
            }
        }
    }

    /// True Range is always non-negative
    #[test]
    fn prop_true_range_non_negative((high, low, close) in arb_ohlc_series(5, 100)) {
        let result = true_range(&high, &low, &close).unwrap();

        for (i, &val) in result.iter().enumerate() {
            if !val.is_nan() {
                prop_assert!(
                    val >= 0.0,
                    "True Range at index {} is negative: {}", i, val
                );
            }
        }
    }
}

// ==================== Stochastic Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Stochastic %K is in range [0, 100]
    #[test]
    fn prop_stochastic_k_bounded((high, low, close) in arb_ohlc_series(20, 100)) {
        let k_period = 14;
        let d_period = 3;

        if high.len() >= k_period {
            let result = stochastic_fast(&high, &low, &close, k_period, d_period).unwrap();

            for (i, &val) in result.k.iter().enumerate() {
                if !val.is_nan() {
                    prop_assert!(
                        (0.0..=100.0).contains(&val),
                        "%K at index {} is out of bounds: {}", i, val
                    );
                }
            }
        }
    }

    /// Stochastic %D is in range [0, 100]
    #[test]
    fn prop_stochastic_d_bounded((high, low, close) in arb_ohlc_series(20, 100)) {
        let k_period = 14;
        let d_period = 3;

        if high.len() >= k_period {
            let result = stochastic_fast(&high, &low, &close, k_period, d_period).unwrap();

            for (i, &val) in result.d.iter().enumerate() {
                if !val.is_nan() {
                    prop_assert!(
                        (0.0..=100.0).contains(&val),
                        "%D at index {} is out of bounds: {}", i, val
                    );
                }
            }
        }
    }
}

// ==================== Rolling Extrema Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Rolling max >= rolling min
    #[test]
    fn prop_rolling_max_gte_min(data in arb_price_series(10, 100), period in 1usize..=10) {
        if data.len() >= period {
            let max_result = rolling_max(&data, period).unwrap();
            let min_result = rolling_min(&data, period).unwrap();

            for i in (period - 1)..data.len() {
                prop_assert!(
                    max_result[i] >= min_result[i],
                    "Max {} < Min {} at index {}", max_result[i], min_result[i], i
                );
            }
        }
    }

    /// Rolling max is at least as large as current value
    #[test]
    fn prop_rolling_max_gte_current(data in arb_price_series(10, 100), period in 1usize..=10) {
        if data.len() >= period {
            let result = rolling_max(&data, period).unwrap();

            for i in (period - 1)..data.len() {
                if !result[i].is_nan() {
                    prop_assert!(
                        result[i] >= data[i],
                        "Rolling max {} < current value {} at index {}",
                        result[i], data[i], i
                    );
                }
            }
        }
    }

    /// Rolling min is at most as large as current value
    #[test]
    fn prop_rolling_min_lte_current(data in arb_price_series(10, 100), period in 1usize..=10) {
        if data.len() >= period {
            let result = rolling_min(&data, period).unwrap();

            for i in (period - 1)..data.len() {
                if !result[i].is_nan() {
                    prop_assert!(
                        result[i] <= data[i],
                        "Rolling min {} > current value {} at index {}",
                        result[i], data[i], i
                    );
                }
            }
        }
    }
}

// ==================== ADX Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// ADX output length equals input length
    #[test]
    fn prop_adx_output_length((high, low, close) in arb_ohlc_series(30, 100), period in 2usize..=10) {
        let min_len = 2 * period;
        if high.len() >= min_len {
            let result = adx(&high, &low, &close, period).unwrap();
            prop_assert_eq!(result.adx.len(), high.len());
            prop_assert_eq!(result.plus_di.len(), high.len());
            prop_assert_eq!(result.minus_di.len(), high.len());
        }
    }

    /// ADX values are in range [0, 100]
    #[test]
    fn prop_adx_bounded((high, low, close) in arb_ohlc_series(30, 100), period in 2usize..=10) {
        let min_len = 2 * period;
        if high.len() >= min_len {
            let result = adx(&high, &low, &close, period).unwrap();

            for (i, &val) in result.adx.iter().enumerate() {
                if !val.is_nan() {
                    prop_assert!(
                        val >= 0.0 && val <= 100.0,
                        "ADX at index {} is out of bounds: {}", i, val
                    );
                }
            }
        }
    }

    /// +DI values are in range [0, 100]
    #[test]
    fn prop_adx_plus_di_bounded((high, low, close) in arb_ohlc_series(30, 100), period in 2usize..=10) {
        let min_len = 2 * period;
        if high.len() >= min_len {
            let result = adx(&high, &low, &close, period).unwrap();

            for (i, &val) in result.plus_di.iter().enumerate() {
                if !val.is_nan() {
                    prop_assert!(
                        val >= 0.0 && val <= 100.0,
                        "+DI at index {} is out of bounds: {}", i, val
                    );
                }
            }
        }
    }

    /// -DI values are in range [0, 100]
    #[test]
    fn prop_adx_minus_di_bounded((high, low, close) in arb_ohlc_series(30, 100), period in 2usize..=10) {
        let min_len = 2 * period;
        if high.len() >= min_len {
            let result = adx(&high, &low, &close, period).unwrap();

            for (i, &val) in result.minus_di.iter().enumerate() {
                if !val.is_nan() {
                    prop_assert!(
                        val >= 0.0 && val <= 100.0,
                        "-DI at index {} is out of bounds: {}", i, val
                    );
                }
            }
        }
    }
}

// ==================== Williams %R Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Williams %R output length equals input length
    #[test]
    fn prop_williams_r_output_length((high, low, close) in arb_ohlc_series(20, 100), period in 1usize..=14) {
        if high.len() >= period {
            let result = williams_r(&high, &low, &close, period).unwrap();
            prop_assert_eq!(result.len(), high.len());
        }
    }

    /// Williams %R values are in range [-100, 0]
    #[test]
    fn prop_williams_r_bounded((high, low, close) in arb_ohlc_series(20, 100), period in 1usize..=14) {
        if high.len() >= period {
            let result = williams_r(&high, &low, &close, period).unwrap();

            for (i, &val) in result.iter().enumerate() {
                if !val.is_nan() {
                    prop_assert!(
                        val >= -100.0 && val <= 0.0,
                        "Williams %R at index {} is out of bounds: {}", i, val
                    );
                }
            }
        }
    }

    /// Williams %R has exactly period-1 NaN values at the start
    #[test]
    fn prop_williams_r_nan_count((high, low, close) in arb_ohlc_series(20, 100), period in 1usize..=14) {
        if high.len() >= period {
            let result = williams_r(&high, &low, &close, period).unwrap();
            let nan_count = result.iter().filter(|x| x.is_nan()).count();
            prop_assert_eq!(nan_count, period - 1);
        }
    }
}

// ==================== Donchian Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Donchian output length equals input length
    #[test]
    fn prop_donchian_output_length((high, low, _close) in arb_ohlc_series(25, 100), period in 1usize..=20) {
        if high.len() >= period {
            let result = donchian(&high, &low, period).unwrap();
            prop_assert_eq!(result.upper.len(), high.len());
            prop_assert_eq!(result.middle.len(), high.len());
            prop_assert_eq!(result.lower.len(), high.len());
        }
    }

    /// Donchian upper >= middle >= lower
    #[test]
    fn prop_donchian_band_order((high, low, _close) in arb_ohlc_series(25, 100), period in 1usize..=20) {
        if high.len() >= period {
            let result = donchian(&high, &low, period).unwrap();

            for i in (period - 1)..high.len() {
                if !result.middle[i].is_nan() {
                    prop_assert!(
                        result.upper[i] >= result.middle[i],
                        "Upper {} < Middle {} at index {}", result.upper[i], result.middle[i], i
                    );
                    prop_assert!(
                        result.middle[i] >= result.lower[i],
                        "Middle {} < Lower {} at index {}", result.middle[i], result.lower[i], i
                    );
                }
            }
        }
    }

    /// Donchian has exactly period-1 NaN values at the start
    #[test]
    fn prop_donchian_nan_count((high, low, _close) in arb_ohlc_series(25, 100), period in 1usize..=20) {
        if high.len() >= period {
            let result = donchian(&high, &low, period).unwrap();
            let nan_count = result.upper.iter().filter(|x| x.is_nan()).count();
            prop_assert_eq!(nan_count, period - 1);
        }
    }
}

// ==================== OBV Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// OBV output length equals input length
    #[test]
    fn prop_obv_output_length((close, volume) in arb_close_volume_series(5, 100)) {
        let result = obv(&close, &volume).unwrap();
        prop_assert_eq!(result.len(), close.len());
    }

    /// OBV has no NaN values with valid input (lookback is 0)
    #[test]
    fn prop_obv_no_nan_with_valid_input((close, volume) in arb_close_volume_series(5, 100)) {
        let result = obv(&close, &volume).unwrap();

        for (i, &val) in result.iter().enumerate() {
            prop_assert!(
                !val.is_nan(),
                "OBV should not have NaN at index {} with valid input", i
            );
        }
    }

    /// OBV first value equals first volume
    #[test]
    fn prop_obv_first_value((close, volume) in arb_close_volume_series(5, 100)) {
        let result = obv(&close, &volume).unwrap();

        prop_assert!(
            (result[0] - volume[0]).abs() < 1e-10,
            "OBV first value {} should equal first volume {}", result[0], volume[0]
        );
    }
}

// ==================== VWAP Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// VWAP output length equals input length
    #[test]
    fn prop_vwap_output_length((high, low, close, volume) in arb_ohlcv_series(5, 100)) {
        let result = vwap(&high, &low, &close, &volume).unwrap();
        prop_assert_eq!(result.len(), high.len());
    }

    /// VWAP has no NaN values with valid input (lookback is 0)
    #[test]
    fn prop_vwap_no_nan_with_valid_input((high, low, close, volume) in arb_ohlcv_series(5, 100)) {
        let result = vwap(&high, &low, &close, &volume).unwrap();

        for (i, &val) in result.iter().enumerate() {
            prop_assert!(
                !val.is_nan(),
                "VWAP should not have NaN at index {} with valid input", i
            );
        }
    }

    /// VWAP is always positive with positive inputs
    #[test]
    fn prop_vwap_always_positive((high, low, close, volume) in arb_ohlcv_series(5, 100)) {
        let result = vwap(&high, &low, &close, &volume).unwrap();

        for (i, &val) in result.iter().enumerate() {
            if !val.is_nan() {
                prop_assert!(
                    val > 0.0,
                    "VWAP at index {} should be positive: {}", i, val
                );
            }
        }
    }
}

// ==================== Price Transform Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// AVGPRICE output length equals input length
    #[test]
    fn prop_avgprice_output_length((open, high, low, close) in arb_ohlc_with_open_series(5, 100)) {
        let result = avgprice(&open, &high, &low, &close).unwrap();
        prop_assert_eq!(result.len(), high.len());
    }

    /// AVGPRICE has no NaN values (lookback is 0)
    #[test]
    fn prop_avgprice_no_nan((open, high, low, close) in arb_ohlc_with_open_series(5, 100)) {
        let result = avgprice(&open, &high, &low, &close).unwrap();

        for (i, &val) in result.iter().enumerate() {
            prop_assert!(
                !val.is_nan(),
                "AVGPRICE should not have NaN at index {}", i
            );
        }
    }

    /// MEDPRICE output length equals input length
    #[test]
    fn prop_medprice_output_length((high, low, _close) in arb_ohlc_series(5, 100)) {
        let result = medprice(&high, &low).unwrap();
        prop_assert_eq!(result.len(), high.len());
    }

    /// MEDPRICE has no NaN values (lookback is 0)
    #[test]
    fn prop_medprice_no_nan((high, low, _close) in arb_ohlc_series(5, 100)) {
        let result = medprice(&high, &low).unwrap();

        for (i, &val) in result.iter().enumerate() {
            prop_assert!(
                !val.is_nan(),
                "MEDPRICE should not have NaN at index {}", i
            );
        }
    }

    /// TYPPRICE output length equals input length
    #[test]
    fn prop_typprice_output_length((high, low, close) in arb_ohlc_series(5, 100)) {
        let result = typprice(&high, &low, &close).unwrap();
        prop_assert_eq!(result.len(), high.len());
    }

    /// TYPPRICE has no NaN values (lookback is 0)
    #[test]
    fn prop_typprice_no_nan((high, low, close) in arb_ohlc_series(5, 100)) {
        let result = typprice(&high, &low, &close).unwrap();

        for (i, &val) in result.iter().enumerate() {
            prop_assert!(
                !val.is_nan(),
                "TYPPRICE should not have NaN at index {}", i
            );
        }
    }

    /// WCLPRICE output length equals input length
    #[test]
    fn prop_wclprice_output_length((high, low, close) in arb_ohlc_series(5, 100)) {
        let result = wclprice(&high, &low, &close).unwrap();
        prop_assert_eq!(result.len(), high.len());
    }

    /// WCLPRICE has no NaN values (lookback is 0)
    #[test]
    fn prop_wclprice_no_nan((high, low, close) in arb_ohlc_series(5, 100)) {
        let result = wclprice(&high, &low, &close).unwrap();

        for (i, &val) in result.iter().enumerate() {
            prop_assert!(
                !val.is_nan(),
                "WCLPRICE should not have NaN at index {}", i
            );
        }
    }
}

// ==================== AD Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// AD output length equals input length
    #[test]
    fn prop_ad_output_length((high, low, close, volume) in arb_ohlcv_series(5, 100)) {
        let result = ad(&high, &low, &close, &volume).unwrap();
        prop_assert_eq!(result.len(), high.len());
    }

    /// AD has no NaN values with valid input (lookback is 0)
    #[test]
    fn prop_ad_no_nan_with_valid_input((high, low, close, volume) in arb_ohlcv_series(5, 100)) {
        let result = ad(&high, &low, &close, &volume).unwrap();

        for (i, &val) in result.iter().enumerate() {
            prop_assert!(
                !val.is_nan(),
                "AD should not have NaN at index {} with valid input", i
            );
        }
    }
}

// ==================== ROC Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// ROC output length equals input length
    #[test]
    fn prop_roc_output_length(data in arb_price_series(15, 100), period in 1usize..=10) {
        if data.len() > period {
            let result = roc(&data, period).unwrap();
            prop_assert_eq!(result.len(), data.len());
        }
    }

    /// ROC has exactly period NaN values at the start
    #[test]
    fn prop_roc_nan_count(data in arb_price_series(15, 100), period in 1usize..=10) {
        if data.len() > period {
            let result = roc(&data, period).unwrap();
            let nan_count = result.iter().filter(|x| x.is_nan()).count();
            prop_assert_eq!(nan_count, period);
        }
    }

    /// ROC of constant values equals zero
    #[test]
    fn prop_roc_constant_input(constant in 1.0..1000.0_f64, len in 15usize..50, period in 1usize..=10) {
        if len > period {
            let data = vec![constant; len];
            let result = roc(&data, period).unwrap();

            for i in period..len {
                prop_assert!(
                    result[i].abs() < 1e-10,
                    "ROC of constant {} at index {} should be 0, got {}", constant, i, result[i]
                );
            }
        }
    }
}

// ==================== MFI Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// MFI output length equals input length
    #[test]
    fn prop_mfi_output_length((high, low, close, volume) in arb_ohlcv_series(20, 100), period in 2usize..=14) {
        if high.len() > period {
            let result = mfi(&high, &low, &close, &volume, period).unwrap();
            prop_assert_eq!(result.len(), high.len());
        }
    }

    /// MFI values are in range [0, 100]
    #[test]
    fn prop_mfi_bounded((high, low, close, volume) in arb_ohlcv_series(20, 100), period in 2usize..=14) {
        if high.len() > period {
            let result = mfi(&high, &low, &close, &volume, period).unwrap();

            for (i, &val) in result.iter().enumerate() {
                if !val.is_nan() {
                    prop_assert!(
                        val >= 0.0 && val <= 100.0,
                        "MFI at index {} is out of bounds: {}", i, val
                    );
                }
            }
        }
    }
}

// ==================== VAR Properties ====================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// VAR output length equals input length
    #[test]
    fn prop_var_output_length(data in arb_price_series(15, 100), period in 2usize..=10) {
        if data.len() >= period {
            let result = var(&data, period).unwrap();
            prop_assert_eq!(result.len(), data.len());
        }
    }

    /// VAR is always non-negative
    #[test]
    fn prop_var_non_negative(data in arb_price_series(15, 100), period in 2usize..=10) {
        if data.len() >= period {
            let result = var(&data, period).unwrap();

            for (i, &val) in result.iter().enumerate() {
                if !val.is_nan() {
                    prop_assert!(
                        val >= 0.0,
                        "VAR at index {} should be non-negative: {}", i, val
                    );
                }
            }
        }
    }

    /// VAR of constant values equals zero
    #[test]
    fn prop_var_constant_input(constant in 1.0..1000.0_f64, len in 15usize..50, period in 2usize..=10) {
        if len >= period {
            let data = vec![constant; len];
            let result = var(&data, period).unwrap();

            for i in (period - 1)..len {
                prop_assert!(
                    result[i].abs() < 1e-10,
                    "VAR of constant {} at index {} should be 0, got {}", constant, i, result[i]
                );
            }
        }
    }

    /// VAR has exactly period-1 NaN values at the start
    #[test]
    fn prop_var_nan_count(data in arb_price_series(15, 100), period in 2usize..=10) {
        if data.len() >= period {
            let result = var(&data, period).unwrap();
            let nan_count = result.iter().filter(|x| x.is_nan()).count();
            prop_assert_eq!(nan_count, period - 1);
        }
    }
}
