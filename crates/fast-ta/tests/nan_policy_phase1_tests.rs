//! Phase 1 policy enforcement tests for NaN/Infinity handling.

use fast_ta::indicators::{
    adx::adx,
    atr::atr,
    bollinger::bollinger,
    cci::cci,
    donchian::donchian,
    ema::ema,
    macd::macd,
    obv::obv,
    sma::sma,
    stochrsi::stochrsi,
    vwap::vwap,
};

const LEN: usize = 80;
const NAN_INDEX: usize = 25;

fn base_series() -> Vec<f64> {
    let mut data = Vec::with_capacity(LEN);
    for i in 0..LEN {
        data.push(100.0 + i as f64);
    }
    data
}

fn base_ohlc() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(LEN);
    let mut high = Vec::with_capacity(LEN);
    let mut low = Vec::with_capacity(LEN);
    let mut close = Vec::with_capacity(LEN);
    for i in 0..LEN {
        let value = 100.0 + i as f64;
        open.push(value + 0.2);
        high.push(value + 1.0);
        low.push(value - 1.0);
        close.push(value);
    }
    (open, high, low, close)
}

fn assert_nan_at(series: &[f64], name: &str) {
    assert!(
        series[NAN_INDEX].is_nan(),
        "{name} should be NaN at index {NAN_INDEX}"
    );
}

fn assert_nan_at_all(series: &[&[f64]], name: &str) {
    for s in series {
        assert!(
            s[NAN_INDEX].is_nan(),
            "{name} should be NaN at index {NAN_INDEX}"
        );
    }
}

#[test]
fn phase1_rolling_indicators_nan_and_infinity() {
    let mut data = base_series();
    data[NAN_INDEX] = f64::NAN;

    assert_nan_at(&sma(&data, 5).unwrap(), "sma");
    let boll = bollinger(&data, 5, 2.0).unwrap();
    assert_nan_at_all(&[&boll.upper, &boll.middle, &boll.lower], "bollinger");

    let mut data_inf = base_series();
    data_inf[NAN_INDEX] = f64::INFINITY;
    assert_nan_at(&sma(&data_inf, 5).unwrap(), "sma_inf");
    let boll_inf = bollinger(&data_inf, 5, 2.0).unwrap();
    assert_nan_at_all(
        &[&boll_inf.upper, &boll_inf.middle, &boll_inf.lower],
        "bollinger_inf",
    );

    let (_open, mut high, mut low, _close) = base_ohlc();
    high[NAN_INDEX] = f64::NAN;
    low[NAN_INDEX] = f64::NAN;
    let donch = donchian(&high, &low, 5).unwrap();
    assert_nan_at_all(&[&donch.upper, &donch.middle, &donch.lower], "donchian");

    let (_open, mut high, mut low, mut close) = base_ohlc();
    high[NAN_INDEX] = f64::INFINITY;
    low[NAN_INDEX] = f64::INFINITY;
    close[NAN_INDEX] = f64::INFINITY;
    assert_nan_at(&cci(&high, &low, &close, 5).unwrap(), "cci_inf");
}

#[test]
fn phase1_cumulative_indicators_nan_and_infinity() {
    let mut data = base_series();
    data[NAN_INDEX] = f64::NAN;

    let ema_out = ema(&data, 5).unwrap();
    assert_nan_at(&ema_out, "ema");
    assert!(ema_out[NAN_INDEX + 1].is_nan());

    let mut data_inf = base_series();
    data_inf[NAN_INDEX] = f64::INFINITY;
    let ema_inf = ema(&data_inf, 5).unwrap();
    assert_nan_at(&ema_inf, "ema_inf");
    assert!(ema_inf[NAN_INDEX + 1].is_nan());

    let close = vec![10.0_f64, 10.5, 10.2, 10.8, 10.5, 10.6, 10.7];
    let mut volume = vec![1000.0_f64; close.len()];
    volume[3] = f64::NAN;
    let obv_out = obv(&close, &volume).unwrap();
    assert!(obv_out[3].is_nan());
    assert!(obv_out[4].is_nan());

    let (_open, mut high, low, mut close) = base_ohlc();
    let mut volume = vec![1000.0_f64; LEN];
    high[NAN_INDEX] = f64::NAN;
    volume[NAN_INDEX] = f64::NAN;
    let vwap_out = vwap(&high, &low, &close, &volume).unwrap();
    assert!(vwap_out[NAN_INDEX].is_nan());
    assert!(vwap_out[NAN_INDEX + 1].is_nan());

    close[NAN_INDEX] = f64::INFINITY;
    let macd_out = macd(&close, 3, 5, 2).unwrap();
    assert_nan_at_all(
        &[&macd_out.macd_line, &macd_out.signal_line, &macd_out.histogram],
        "macd_inf",
    );
}

#[test]
fn phase1_mixed_behavior_indicators_nan_and_infinity() {
    let (_open, mut high, mut low, mut close) = base_ohlc();
    let _volume = vec![1000.0_f64; LEN];

    high[NAN_INDEX] = f64::NAN;
    low[NAN_INDEX] = f64::NAN;
    close[NAN_INDEX] = f64::NAN;
    assert_nan_at(&atr(&high, &low, &close, 5).unwrap(), "atr");

    let adx_out = adx(&high, &low, &close, 5).unwrap();
    assert_nan_at_all(
        &[&adx_out.adx, &adx_out.plus_di, &adx_out.minus_di],
        "adx",
    );

    let mut data = base_series();
    data[NAN_INDEX] = f64::NAN;
    let stochrsi_out = stochrsi(&data, 5, 5, 1, 3).unwrap();
    assert_nan_at_all(&[&stochrsi_out.fastk, &stochrsi_out.fastd], "stochrsi");

    let mut data_inf = base_series();
    data_inf[NAN_INDEX] = f64::INFINITY;
    let stochrsi_inf = stochrsi(&data_inf, 5, 5, 1, 3).unwrap();
    assert_nan_at_all(
        &[&stochrsi_inf.fastk, &stochrsi_inf.fastd],
        "stochrsi_inf",
    );
}
