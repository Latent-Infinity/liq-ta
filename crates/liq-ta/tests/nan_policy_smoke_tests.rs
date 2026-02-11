//! Policy enforcement smoke tests for NaN/Infinity handling.

use liq_ta::indicators::{
    bollinger::bollinger, macd::macd, obv::obv, sma::sma, stochrsi::stochrsi, vwap::vwap,
};

#[test]
fn smoke_rolling_nan_and_infinity() {
    let data = vec![1.0_f64, 2.0, f64::NAN, 4.0, 5.0, 6.0];
    let out = sma(&data, 3).unwrap();
    assert!(out[2].is_nan());

    let bands = bollinger(&data, 3, 2.0).unwrap();
    assert!(bands.middle[2].is_nan());
    assert!(bands.upper[2].is_nan());
    assert!(bands.lower[2].is_nan());

    let data_inf = vec![1.0_f64, 2.0, f64::INFINITY, 4.0, 5.0, 6.0];
    let out_inf = sma(&data_inf, 3).unwrap();
    assert!(out_inf[2].is_nan());
}

#[test]
fn smoke_cumulative_nan_and_infinity() {
    let close = vec![10.0_f64, 10.5, 10.2, 10.8, 10.5];
    let volume_nan = vec![1000.0_f64, 1500.0, f64::NAN, 1800.0, 1100.0];
    let obv_out = obv(&close, &volume_nan).unwrap();
    assert!(obv_out[2].is_nan());
    assert!(obv_out[3].is_nan());

    let high_inf = vec![10.0_f64, f64::INFINITY, 12.0, 11.5, 11.0];
    let low = vec![9.0_f64, 9.5, 11.0, 10.5, 10.0];
    let close = vec![9.5_f64, 10.5, 11.5, 11.0, 10.8];
    let volume = vec![1000.0_f64, 1200.0, 1100.0, 1300.0, 1250.0];
    let vwap_out = vwap(&high_inf, &low, &close, &volume).unwrap();
    assert!(vwap_out[1].is_nan());
    assert!(vwap_out[2].is_nan());
}

#[test]
fn smoke_multi_period_and_mixed_behavior() {
    let mut data = Vec::with_capacity(30);
    for i in 0..30 {
        data.push(100.0 + i as f64);
    }
    data[6] = f64::NAN;

    let macd_out = macd(&data, 3, 5, 2).unwrap();
    assert!(macd_out.macd_line[6].is_nan());
    assert!(macd_out.signal_line[6].is_nan());
    assert!(macd_out.histogram[6].is_nan());

    let stochrsi_out = stochrsi(&data, 3, 3, 1, 2).unwrap();
    assert!(stochrsi_out.fastk[6].is_nan());
    assert!(stochrsi_out.fastd[6].is_nan());
}
