use half::f16;
use liq_ta::indicators::{candlestick as cdl, statistics as stats};
use liq_ta::precision::{PrecisionMode, with_precision_mode};

fn make_ohlc(n: usize, base: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);

    for i in 0..n {
        let c = base + (i as f64) * 0.05;
        let o = c + 0.01;
        open.push(o);
        high.push(o + 0.05);
        low.push(c - 0.05);
        close.push(c);
    }

    (open, high, low, close)
}

#[test]
fn outlier_two_candle_kicking_by_length_bullish_strong_branch() {
    let n = cdl::two_candle::cdl_kicking_by_length_min_len().max(16);
    let (mut open, mut high, mut low, mut close) = make_ohlc(n, 100.0);

    let prev = n - 2;
    let curr = n - 1;

    // Previous: bearish marubozu
    open[prev] = 110.0;
    high[prev] = 110.0;
    low[prev] = 100.0;
    close[prev] = 100.0;

    // Current: bullish marubozu with larger body and gap up
    open[curr] = 112.0;
    high[curr] = 126.0;
    low[curr] = 112.0;
    close[curr] = 126.0;

    let base = cdl::cdl_kicking(&open, &high, &low, &close).expect("cdl_kicking");
    let by_len =
        cdl::cdl_kicking_by_length(&open, &high, &low, &close).expect("cdl_kicking_by_length");

    assert!(base[curr] > 0);
    assert!(by_len[curr] > base[curr]);
}

#[test]
fn outlier_three_candle_zero_range_and_non_decreasing_close_branches() {
    // Target missed line in cdl_3stars_in_south: third_range == 0 path
    let n = cdl::three_candle::cdl_3stars_in_south_min_len().max(32);
    let (mut open, mut high, mut low, mut close) = make_ohlc(n, 300.0);

    for i in 0..n {
        close[i] = 300.0 - (i as f64) * 0.7;
        open[i] = close[i] + 0.3;
        high[i] = open[i] + 0.2;
        low[i] = close[i] - 0.2;
    }

    let third = n - 1;
    let second = n - 2;
    let first = n - 3;

    // First: long bearish with long lower shadow
    open[first] = 30.0;
    close[first] = 20.0;
    high[first] = 30.0;
    low[first] = 5.0;

    // Second: bearish, lower low, higher close than first
    open[second] = 24.0;
    close[second] = 22.0;
    high[second] = 24.0;
    low[second] = 0.0;

    // Third: force range==0 while still bearish via non-validated OHLC relation
    open[third] = 1.0;
    close[third] = 0.9;
    high[third] = 0.5;
    low[third] = 0.5;

    let stars = cdl::cdl_3stars_in_south(&open, &high, &low, &close).expect("cdl_3stars_in_south");
    assert_eq!(stars.len(), n);

    // Target missed lines in cdl_identical_3crows: non-decreasing close rejection branch
    let m = cdl::three_candle::cdl_identical_3crows_min_len().max(32);
    let (mut open2, mut high2, mut low2, mut close2) = make_ohlc(m, 200.0);

    for i in 0..m {
        let c = 200.0 + (i as f64) * 0.2;
        close2[i] = c;
        open2[i] = c + 0.001;
        high2[i] = open2[i] + 0.02;
        low2[i] = c - 0.02;
    }

    let third2 = m - 1;
    let second2 = m - 2;
    let first2 = m - 3;

    open2[first2] = 120.006;
    close2[first2] = 120.002;
    high2[first2] = 120.020;
    low2[first2] = 119.980;

    open2[second2] = 120.002;
    close2[second2] = 119.998;
    high2[second2] = 120.016;
    low2[second2] = 119.976;

    // Keep bearish and near-equal opens, but do not close lower than the second candle.
    open2[third2] = 120.000;
    close2[third2] = 119.998;
    high2[third2] = 120.014;
    low2[third2] = 119.980;

    let crows =
        cdl::cdl_identical_3crows(&open2, &high2, &low2, &close2).expect("cdl_identical_3crows");
    assert_eq!(crows[third2], 0);
}

#[test]
fn outlier_statistics_precision_and_f16_dispatch_paths() {
    let n = 96;
    let p = 12;

    let data32: Vec<f32> = (0..n)
        .map(|i| 10.0 + (i as f32) * 0.31 + ((i % 7) as f32) * 0.03)
        .collect();

    with_precision_mode(PrecisionMode::High, || {
        let mut out = vec![f32::NAN; n];
        stats::var_into(&data32, p, &mut out).expect("var_into f32 high fast-path");

        let mut data32_nan = data32.clone();
        data32_nan[p / 2] = f32::NAN;
        stats::var_into(&data32_nan, p, &mut out).expect("var_into f32 high nan-path");

        let huge: Vec<f32> = (0..64)
            .map(|i| if i % 2 == 0 { 1.0e30 } else { -1.0e30 })
            .collect();
        let mut huge_out = vec![f32::NAN; huge.len()];
        let _ = stats::var_into(&huge, 4, &mut huge_out);
    });

    let data64: Vec<f64> = data32.iter().map(|&x| x as f64).collect();
    let mut out64 = vec![f64::NAN; n];
    stats::var_into(&data64, p, &mut out64).expect("var_into f64 specialization");

    let data16: Vec<f16> = data32.iter().map(|&x| f16::from_f32(x)).collect();
    with_precision_mode(PrecisionMode::Fast, || {
        let mut out16 = vec![f16::NAN; n];
        stats::var_into(&data16, p, &mut out16).expect("var_into f16 fast-path");

        let mut data16_nan = data16.clone();
        data16_nan[p / 2] = f16::NAN;
        stats::var_into(&data16_nan, p, &mut out16).expect("var_into f16 nan-path");
    });

    let mut angle_out16 = vec![f16::NAN; n];
    stats::linearreg_angle_into(&data16, p, &mut angle_out16).expect("linearreg_angle_into f16");
}
