//! NaN propagation checks for all numeric indicators.
//!
//! Candlestick pattern indicators return integer codes; NaN propagation does not apply.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]

use fast_ta::indicators::{
    ad::ad,
    adosc::adosc,
    adx::adx,
    apo::{apo, ppo},
    aroon::{aroon, aroonosc},
    atr::{atr, true_range},
    bollinger::{bollinger, rolling_stddev},
    bop::bop,
    cci::cci,
    cmo::cmo,
    dema::dema,
    donchian::donchian,
    dx::{adxr, dx, minus_dm, plus_dm},
    ema::{ema, ema_wilder, ema_with_alpha},
    ht_dcperiod::ht_dcperiod,
    ht_dcphase::ht_dcphase,
    ht_phasor::ht_phasor,
    ht_sine::ht_sine,
    ht_trendline::ht_trendline,
    ht_trendmode::ht_trendmode,
    kama::kama,
    macd::macd,
    mama::mama,
    mavp::mavp,
    mfi::mfi,
    midpoint::midpoint,
    midprice::midprice,
    mom::mom,
    obv::obv,
    price_transform::{avgprice, medprice, typprice, wclprice},
    roc::{roc, rocp, rocr, rocr100},
    rsi::rsi,
    sar::sar,
    sarext::sarext,
    sma::sma,
    statistics::{
        beta, correl, linearreg, linearreg_angle, linearreg_intercept, linearreg_slope, tsf, var,
    },
    stochastic::{stochastic, stochastic_fast, stochastic_slow},
    stochrsi::stochrsi,
    t3::t3,
    tema::tema,
    trima::trima,
    trix::trix,
    ultosc::ultosc,
    vwap::vwap,
    williams_r::williams_r,
    wma::wma,
};

const LEN: usize = 120;
const NAN_INDEX: usize = 25;

fn base_series() -> Vec<f64> {
    let mut data = Vec::with_capacity(LEN);
    for i in 0..LEN {
        data.push(100.0 + i as f64);
    }
    data
}

fn base_series_pair() -> (Vec<f64>, Vec<f64>) {
    let mut data0 = Vec::with_capacity(LEN);
    let mut data1 = Vec::with_capacity(LEN);
    for i in 0..LEN {
        let value = 100.0 + i as f64;
        data0.push(value);
        data1.push(value * 1.01);
    }
    (data0, data1)
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

fn run_single_series_suite(data: &[f64]) {
    assert_nan_at(&sma(&data, 5).unwrap(), "sma");
    assert_nan_at(&ema(&data, 5).unwrap(), "ema");
    assert_nan_at(&ema_wilder(&data, 5).unwrap(), "ema_wilder");
    assert_nan_at(&ema_with_alpha(&data, 5, 0.3).unwrap(), "ema_with_alpha");
    assert_nan_at(&wma(&data, 5).unwrap(), "wma");
    assert_nan_at(&dema(&data, 5).unwrap(), "dema");
    assert_nan_at(&tema(&data, 5).unwrap(), "tema");
    assert_nan_at(&trima(&data, 5).unwrap(), "trima");
    assert_nan_at(&kama(&data, 5).unwrap(), "kama");
    assert_nan_at(&t3(&data, 5).unwrap(), "t3");
    assert_nan_at(&mom(&data, 5).unwrap(), "mom");
    assert_nan_at(&roc(&data, 5).unwrap(), "roc");
    assert_nan_at(&rocp(&data, 5).unwrap(), "rocp");
    assert_nan_at(&rocr(&data, 5).unwrap(), "rocr");
    assert_nan_at(&rocr100(&data, 5).unwrap(), "rocr100");
    assert_nan_at(&cmo(&data, 5).unwrap(), "cmo");
    assert_nan_at(&rsi(&data, 5).unwrap(), "rsi");
    assert_nan_at(&apo(&data, 5, 8).unwrap(), "apo");
    assert_nan_at(&ppo(&data, 5, 8).unwrap(), "ppo");
    assert_nan_at(&trix(&data, 5).unwrap(), "trix");
    assert_nan_at(&midpoint(&data, 5).unwrap(), "midpoint");
    assert_nan_at(&rolling_stddev(&data, 5).unwrap(), "rolling_stddev");

    let stochrsi_result = stochrsi(&data, 5, 5, 1, 3).unwrap();
    assert_nan_at_all(&[&stochrsi_result.fastk, &stochrsi_result.fastd], "stochrsi");

    let mama_out = mama(&data).unwrap();
    assert_nan_at_all(&[&mama_out.mama, &mama_out.fama], "mama");

    let periods = vec![5.0_f64; LEN];
    assert_nan_at(&mavp(&data, &periods, 2, 30).unwrap(), "mavp");

    let ht_period = ht_dcperiod(&data).unwrap();
    assert_nan_at(&ht_period, "ht_dcperiod");
    assert_nan_at(&ht_dcphase(&data).unwrap(), "ht_dcphase");
    assert_nan_at(&ht_trendline(&data).unwrap(), "ht_trendline");
    assert_nan_at(&ht_trendmode(&data).unwrap(), "ht_trendmode");

    let ht_phasor_out = ht_phasor(&data).unwrap();
    assert_nan_at_all(
        &[&ht_phasor_out.inphase, &ht_phasor_out.quadrature],
        "ht_phasor",
    );

    let ht_sine_out = ht_sine(&data).unwrap();
    assert_nan_at_all(&[&ht_sine_out.sine, &ht_sine_out.lead_sine], "ht_sine");
}

fn run_multi_series_suite(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    data0: &[f64],
    data1: &[f64],
) {
    let macd_out = macd(&close, 5, 8, 3).unwrap();
    assert_nan_at_all(
        &[&macd_out.macd_line, &macd_out.signal_line, &macd_out.histogram],
        "macd",
    );

    let boll_out = bollinger(&close, 5, 2.0).unwrap();
    assert_nan_at_all(&[&boll_out.upper, &boll_out.middle, &boll_out.lower], "bollinger");

    assert_nan_at(&atr(&high, &low, &close, 5).unwrap(), "atr");
    assert_nan_at(&true_range(&high, &low, &close).unwrap(), "true_range");
    let donchian_out = donchian(&high, &low, 5).unwrap();
    assert_nan_at_all(
        &[&donchian_out.upper, &donchian_out.middle, &donchian_out.lower],
        "donchian",
    );

    let stoch_out = stochastic(&high, &low, &close, 5, 3, 1).unwrap();
    assert_nan_at_all(&[&stoch_out.k, &stoch_out.d], "stochastic");
    let stoch_fast_out = stochastic_fast(&high, &low, &close, 5, 3).unwrap();
    assert_nan_at_all(&[&stoch_fast_out.k, &stoch_fast_out.d], "stochastic_fast");
    let stoch_slow_out = stochastic_slow(&high, &low, &close, 5, 3).unwrap();
    assert_nan_at_all(&[&stoch_slow_out.k, &stoch_slow_out.d], "stochastic_slow");

    assert_nan_at(&williams_r(&high, &low, &close, 5).unwrap(), "williams_r");
    let adx_out = adx(&high, &low, &close, 5).unwrap();
    assert_nan_at_all(
        &[&adx_out.adx, &adx_out.plus_di, &adx_out.minus_di],
        "adx",
    );
    assert_nan_at(&adx_out.plus_di, "adx_plus_di");
    assert_nan_at(&adx_out.minus_di, "adx_minus_di");
    assert_nan_at(&adxr(&high, &low, &close, 5).unwrap(), "adxr");
    assert_nan_at(&dx(&high, &low, &close, 5).unwrap(), "dx");
    assert_nan_at(&plus_dm(&high, &low, 5).unwrap(), "plus_dm");
    assert_nan_at(&minus_dm(&high, &low, 5).unwrap(), "minus_dm");

    let aroon_out = aroon(&high, &low, 5).unwrap();
    assert_nan_at_all(&[&aroon_out.aroon_up, &aroon_out.aroon_down], "aroon");
    assert_nan_at(&aroonosc(&high, &low, 5).unwrap(), "aroonosc");

    assert_nan_at(&bop(&open, &high, &low, &close).unwrap(), "bop");
    assert_nan_at(&cci(&high, &low, &close, 5).unwrap(), "cci");
    assert_nan_at(&mfi(&high, &low, &close, &volume, 5).unwrap(), "mfi");
    assert_nan_at(&ultosc(&high, &low, &close, 3, 5, 7).unwrap(), "ultosc");

    assert_nan_at(&obv(&close, &volume).unwrap(), "obv");
    assert_nan_at(&ad(&high, &low, &close, &volume).unwrap(), "ad");
    assert_nan_at(&adosc(&high, &low, &close, &volume, 3, 10).unwrap(), "adosc");
    assert_nan_at(&vwap(&high, &low, &close, &volume).unwrap(), "vwap");

    assert_nan_at(&sar(&high, &low).unwrap(), "sar");
    assert_nan_at(&sarext(&high, &low).unwrap(), "sarext");

    assert_nan_at(&avgprice(&open, &high, &low, &close).unwrap(), "avgprice");
    assert_nan_at(&medprice(&high, &low).unwrap(), "medprice");
    assert_nan_at(&typprice(&high, &low, &close).unwrap(), "typprice");
    assert_nan_at(&wclprice(&high, &low, &close).unwrap(), "wclprice");

    assert_nan_at(&midprice(&high, &low, 5).unwrap(), "midprice");

    assert_nan_at(&var(&data0, 5).unwrap(), "var");
    assert_nan_at(&correl(&data0, &data1, 5).unwrap(), "correl");
    assert_nan_at(&beta(&data0, &data1, 5).unwrap(), "beta");
    assert_nan_at(&linearreg(&data0, 5).unwrap(), "linearreg");
    assert_nan_at(&linearreg_slope(&data0, 5).unwrap(), "linearreg_slope");
    assert_nan_at(&linearreg_intercept(&data0, 5).unwrap(), "linearreg_intercept");
    assert_nan_at(&linearreg_angle(&data0, 5).unwrap(), "linearreg_angle");
    assert_nan_at(&tsf(&data0, 5).unwrap(), "tsf");
}

#[test]
fn nan_propagation_single_series_indicators() {
    let mut data = base_series();
    data[NAN_INDEX] = f64::NAN;
    run_single_series_suite(&data);
}

#[test]
fn nan_propagation_multi_series_indicators() {
    let (mut open, mut high, mut low, mut close) = base_ohlc();
    let mut volume = vec![0.0_f64; LEN];
    for i in 0..LEN {
        volume[i] = 1000.0 + i as f64 * 10.0;
    }

    open[NAN_INDEX] = f64::NAN;
    high[NAN_INDEX] = f64::NAN;
    low[NAN_INDEX] = f64::NAN;
    close[NAN_INDEX] = f64::NAN;
    volume[NAN_INDEX] = f64::NAN;

    let (data0, mut data1) = base_series_pair();
    let mut data0 = data0;
    data0[NAN_INDEX] = f64::NAN;
    data1[NAN_INDEX] = f64::NAN;

    run_multi_series_suite(&open, &high, &low, &close, &volume, &data0, &data1);
}

#[test]
fn infinity_propagation_single_series_indicators() {
    let mut data = base_series();
    data[NAN_INDEX] = f64::INFINITY;
    run_single_series_suite(&data);
}

#[test]
fn infinity_propagation_multi_series_indicators() {
    let (mut open, mut high, mut low, mut close) = base_ohlc();
    let mut volume = vec![0.0_f64; LEN];
    for i in 0..LEN {
        volume[i] = 1000.0 + i as f64 * 10.0;
    }

    open[NAN_INDEX] = f64::INFINITY;
    high[NAN_INDEX] = f64::INFINITY;
    low[NAN_INDEX] = f64::INFINITY;
    close[NAN_INDEX] = f64::INFINITY;
    volume[NAN_INDEX] = f64::INFINITY;

    let (data0, mut data1) = base_series_pair();
    let mut data0 = data0;
    data0[NAN_INDEX] = f64::INFINITY;
    data1[NAN_INDEX] = f64::INFINITY;

    run_multi_series_suite(&open, &high, &low, &close, &volume, &data0, &data1);
}
