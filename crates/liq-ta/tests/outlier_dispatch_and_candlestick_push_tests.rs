use half::f16;
use liq_ta::indicators::{
    adx, bollinger, candlestick as cdl, dx, kama, midpoint, statistics as stats, stochastic,
    stochrsi,
};

fn make_close(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| 100.0 + (i as f64) * 0.15 + (((i * 7) % 11) as f64 - 5.0) * 0.07)
        .collect()
}

fn make_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let close = make_close(n);
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    for (i, &c) in close.iter().enumerate() {
        let o = c + if i % 2 == 0 { -0.22 } else { 0.18 };
        let h = o.max(c) + 1.0 + ((i % 3) as f64) * 0.04;
        let l = o.min(c) - 1.0 - ((i % 4) as f64) * 0.03;
        open.push(o);
        high.push(h);
        low.push(l);
    }
    (open, high, low, close)
}

fn to_f32(v: &[f64]) -> Vec<f32> {
    v.iter().map(|&x| x as f32).collect()
}

fn to_f16(v: &[f64]) -> Vec<f16> {
    v.iter().map(|&x| f16::from_f32(x as f32)).collect()
}

#[test]
fn outlier_dispatch_matrix_f32_and_f16() {
    let n = 128;
    let period = 14;
    let (_open, high, low, close) = make_ohlc(n);
    let high32 = to_f32(&high);
    let low32 = to_f32(&low);
    let close32 = to_f32(&close);
    let high16 = to_f16(&high);
    let low16 = to_f16(&low);
    let close16 = to_f16(&close);

    assert!(adx::adx(&high, &low, &close, period).is_ok());
    assert!(adx::adx(&high32, &low32, &close32, period).is_ok());
    assert!(adx::adx(&high16, &low16, &close16, period).is_ok());

    assert!(dx::dx(&high, &low, &close, period).is_ok());
    assert!(dx::dx(&high32, &low32, &close32, period).is_ok());
    assert!(dx::dx(&high16, &low16, &close16, period).is_ok());
    let mut dx_out16 = vec![f16::from_f32(f32::NAN); n];
    assert!(dx::dx_into(&high16, &low16, &close16, period, &mut dx_out16).is_ok());

    assert!(bollinger::bollinger(&close32, 20, 2.0_f32).is_ok());
    assert!(bollinger::rolling_stddev(&close, n + 1).is_err());

    assert!(midpoint::midpoint(&close32, 10).is_ok());
    assert!(midpoint::midpoint(&close16, 10).is_ok());
    let mut midpoint_out16 = vec![f16::from_f32(f32::NAN); n];
    assert!(midpoint::midpoint_into(&close16, 10, &mut midpoint_out16).is_ok());

    assert!(kama::kama(&close32, 10).is_ok());
    assert!(kama::kama_full(&close32, 10, 2, 30).is_ok());
    assert!(kama::kama(&close16, 10).is_ok());
    assert!(kama::kama_full(&close16, 10, 2, 30).is_ok());

    assert!(stochastic::stochastic_fast(&high32, &low32, &close32, 14, 3).is_ok());
    assert!(stochastic::stochastic_full(&high32, &low32, &close32, 14, 3, 3).is_ok());
    assert!(stochastic::stochastic_fast(&high16, &low16, &close16, 14, 3).is_ok());
    assert!(stochastic::stochastic_full(&high16, &low16, &close16, 14, 3, 3).is_ok());
    let mut st_fast_out16 = stochastic::StochasticOutput {
        k: vec![f16::from_f32(f32::NAN); n],
        d: vec![f16::from_f32(f32::NAN); n],
    };
    assert!(
        stochastic::stochastic_fast_into(&high16, &low16, &close16, 14, 3, &mut st_fast_out16)
            .is_ok()
    );
    let mut st_full_out16 = stochastic::StochasticOutput {
        k: vec![f16::from_f32(f32::NAN); n],
        d: vec![f16::from_f32(f32::NAN); n],
    };
    assert!(
        stochastic::stochastic_full_into(&high16, &low16, &close16, 14, 3, 3, &mut st_full_out16,)
            .is_ok()
    );

    assert!(stochrsi::stochrsi(&close32, 14, 14, 3, 3).is_ok());
    assert!(stochrsi::stochrsi(&close16, 14, 14, 3, 3).is_ok());
    let mut fastk16 = vec![f16::from_f32(f32::NAN); n];
    let mut fastd16 = vec![f16::from_f32(f32::NAN); n];
    assert!(stochrsi::stochrsi_into(&close16, 14, 14, 3, 3, &mut fastk16, &mut fastd16).is_ok());
}

#[test]
fn statistics_f32_and_generic_f16_surface_matrix() {
    let n = 96;
    let p = 12;
    let x = make_close(n);
    let y: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, &v)| v * 0.97 + (i as f64) * 0.02)
        .collect();
    let x32 = to_f32(&x);
    let y32 = to_f32(&y);
    let x16 = to_f16(&x);
    let y16 = to_f16(&y);

    macro_rules! unary_stat {
        ($alloc:ident, $into:ident) => {{
            assert!(stats::$alloc(&x32, p).is_ok());
            let mut out32 = vec![f32::NAN; n];
            assert!(stats::$into(&x32, p, &mut out32).is_ok());

            assert!(stats::$alloc(&x16, p).is_ok());
            let mut out16 = vec![f16::from_f32(f32::NAN); n];
            assert!(stats::$into(&x16, p, &mut out16).is_ok());
        }};
    }

    macro_rules! binary_stat {
        ($alloc:ident, $into:ident) => {{
            assert!(stats::$alloc(&x32, &y32, p).is_ok());
            let mut out32 = vec![f32::NAN; n];
            assert!(stats::$into(&x32, &y32, p, &mut out32).is_ok());

            assert!(stats::$alloc(&x16, &y16, p).is_ok());
            let mut out16 = vec![f16::from_f32(f32::NAN); n];
            assert!(stats::$into(&x16, &y16, p, &mut out16).is_ok());
        }};
    }

    unary_stat!(var, var_into);
    unary_stat!(stddev, stddev_into);
    unary_stat!(skew, skew_into);
    unary_stat!(kurt, kurt_into);
    unary_stat!(zscore, zscore_into);
    unary_stat!(mad, mad_into);
    unary_stat!(sem, sem_into);
    unary_stat!(linearreg, linearreg_into);
    unary_stat!(linearreg_slope, linearreg_slope_into);
    unary_stat!(linearreg_intercept, linearreg_intercept_into);
    unary_stat!(linearreg_angle, linearreg_angle_into);
    unary_stat!(tsf, tsf_into);

    binary_stat!(cov, cov_into);
    binary_stat!(correl, correl_into);
    binary_stat!(beta, beta_into);
}

#[test]
fn stochastic_generic_rescan_paths_f16() {
    let low = vec![
        0.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0, -0.1, -0.2, -0.3, -0.4, -0.5,
        -0.6,
    ];
    let high = vec![
        30.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0,
        24.0, 25.0, 26.0,
    ];
    let close: Vec<f64> = high
        .iter()
        .zip(low.iter())
        .map(|(h, l)| l + (h - l) * 0.6)
        .collect();

    let high16 = to_f16(&high);
    let low16 = to_f16(&low);
    let close16 = to_f16(&close);

    let fast = stochastic::stochastic_fast(&high16, &low16, &close16, 5, 3).expect("fast f16");
    let full = stochastic::stochastic_full(&high16, &low16, &close16, 5, 3, 3).expect("full f16");
    assert_eq!(fast.k.len(), high16.len());
    assert_eq!(full.k.len(), high16.len());
}

#[test]
fn candlestick_short_line_bull_and_bear_paths() {
    let lookback = cdl::single::cdl_short_line_lookback();
    let n = lookback + 2;
    let mut open = vec![10.0; n];
    let mut high = vec![14.2; n];
    let mut low = vec![9.8; n];
    let mut close = vec![14.0; n];

    let b = lookback;
    open[b] = 10.0;
    close[b] = 11.0;
    high[b] = 11.2;
    low[b] = 9.8;

    let s = lookback + 1;
    open[s] = 11.0;
    close[s] = 10.0;
    high[s] = 11.2;
    low[s] = 9.8;

    let out = cdl::cdl_short_line(&open, &high, &low, &close).expect("short line");
    assert!(out[b] > 0);
    assert!(out[s] < 0);
}

#[test]
fn candlestick_takuri_zero_range_branch() {
    let lookback = cdl::single::cdl_takuri_lookback();
    let n = lookback + 1;

    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);

    for i in 0..n {
        let c = 100.0 - (i as f64) * 0.5; // enforce downtrend
        open.push(c + 0.1);
        high.push(c + 0.2);
        low.push(c - 0.2);
        close.push(c);
    }

    let i = n - 1;
    open[i] = 90.0;
    high[i] = 90.0;
    low[i] = 90.0; // zero range
    close[i] = 90.0;

    let out = cdl::cdl_takuri(&open, &high, &low, &close).expect("takuri");
    assert_eq!(out[i], 0);
}

#[test]
fn candlestick_three_candle_targeted_branches() {
    let lookback = cdl::three_candle::cdl_3stars_in_south_lookback();
    let n = lookback + 1;
    let mut open = vec![100.0; n];
    let mut high = vec![102.5; n];
    let mut low = vec![97.5; n];
    let mut close = vec![99.95; n]; // tiny body/range baseline

    let i = n - 1;
    let first = i - 2;
    let second = i - 1;
    let third = i;

    // first: bearish, long body, long lower shadow
    open[first] = 15.0;
    high[first] = 15.2;
    low[first] = 10.0;
    close[first] = 13.0;

    // second: bearish, lower low, but closes higher than first close
    open[second] = 14.0;
    high[second] = 14.2;
    low[second] = 9.5;
    close[second] = 13.2;

    // third: bearish short marubozu inside second range
    open[third] = 12.54;
    high[third] = 12.6;
    low[third] = 12.0;
    close[third] = 12.06;

    let south = cdl::cdl_3stars_in_south(&open, &high, &low, &close).expect("3stars in south");
    assert_eq!(south.len(), n);

    // Reuse arrays and target identical_3crows "closes not lower" rejection branch.
    let lookback_crows = cdl::three_candle::cdl_identical_3crows_lookback();
    let n2 = lookback_crows + 1;
    let mut o2 = vec![10.0; n2];
    let mut h2 = vec![12.5; n2];
    let mut l2 = vec![7.5; n2];
    let mut c2 = vec![9.95; n2];

    let j = n2 - 1;
    let f = j - 2;
    let s = j - 1;
    let t = j;

    o2[f] = 11.0;
    h2[f] = 11.2;
    l2[f] = 9.8;
    c2[f] = 10.5;

    o2[s] = 10.5; // opens at prev close
    h2[s] = 10.7;
    l2[s] = 9.6;
    c2[s] = 10.0;

    o2[t] = 10.2; // approx-equal to prev close within tolerance
    h2[t] = 10.3;
    l2[t] = 10.0;
    c2[t] = 10.1; // bearish but NOT lower than second close

    let crows = cdl::cdl_identical_3crows(&o2, &h2, &l2, &c2).expect("identical 3 crows rejection");
    assert_eq!(crows[t], 0);
}

#[test]
fn candlestick_wrapper_success_and_validation_matrix() {
    let n = 64;
    let (open, high, low, close) = make_ohlc(n);
    let low_short = &low[..n - 1];

    macro_rules! cdl_surface {
        ($f:path) => {{
            assert!($f(&open, &high, &low, &close).is_ok());
            assert!($f(&open, &high, low_short, &close).is_err());
        }};
    }

    cdl_surface!(cdl::cdl_doji);
    cdl_surface!(cdl::cdl_dragonfly_doji);
    cdl_surface!(cdl::cdl_gravestone_doji);
    cdl_surface!(cdl::cdl_longleg_doji);
    cdl_surface!(cdl::cdl_rickshaw_man);
    cdl_surface!(cdl::cdl_marubozu);
    cdl_surface!(cdl::cdl_closing_marubozu);
    cdl_surface!(cdl::cdl_spinning_top);
    cdl_surface!(cdl::cdl_high_wave);
    cdl_surface!(cdl::cdl_long_line);
    cdl_surface!(cdl::cdl_short_line);
    cdl_surface!(cdl::cdl_hammer);
    cdl_surface!(cdl::cdl_hanging_man);
    cdl_surface!(cdl::cdl_inverted_hammer);
    cdl_surface!(cdl::cdl_shooting_star);
    cdl_surface!(cdl::cdl_takuri);
    cdl_surface!(cdl::cdl_belt_hold);

    cdl_surface!(cdl::cdl_engulfing);
    cdl_surface!(cdl::cdl_harami);
    cdl_surface!(cdl::cdl_harami_cross);
    cdl_surface!(cdl::cdl_piercing);
    cdl_surface!(cdl::cdl_dark_cloud_cover);
    cdl_surface!(cdl::cdl_doji_star);
    cdl_surface!(cdl::cdl_kicking);
    cdl_surface!(cdl::cdl_kicking_by_length);
    cdl_surface!(cdl::cdl_matching_low);
    cdl_surface!(cdl::cdl_homing_pigeon);
    cdl_surface!(cdl::cdl_in_neck);
    cdl_surface!(cdl::cdl_on_neck);
    cdl_surface!(cdl::cdl_thrusting);
    cdl_surface!(cdl::cdl_separating_lines);
    cdl_surface!(cdl::cdl_counter_attack);
    cdl_surface!(cdl::cdl_2crows);
    cdl_surface!(cdl::cdl_hikkake);
    cdl_surface!(cdl::cdl_hikkake_mod);

    cdl_surface!(cdl::cdl_morning_star);
    cdl_surface!(cdl::cdl_evening_star);
    cdl_surface!(cdl::cdl_morning_doji_star);
    cdl_surface!(cdl::cdl_evening_doji_star);
    cdl_surface!(cdl::cdl_abandoned_baby);
    cdl_surface!(cdl::cdl_3white_soldiers);
    cdl_surface!(cdl::cdl_3black_crows);
    cdl_surface!(cdl::cdl_3inside);
    cdl_surface!(cdl::cdl_3outside);
    cdl_surface!(cdl::cdl_3line_strike);
    cdl_surface!(cdl::cdl_3stars_in_south);
    cdl_surface!(cdl::cdl_tristar);
    cdl_surface!(cdl::cdl_identical_3crows);

    cdl_surface!(cdl::cdl_stick_sandwich);
    cdl_surface!(cdl::cdl_unique_3river);
    cdl_surface!(cdl::cdl_advance_block);
    cdl_surface!(cdl::cdl_stalled_pattern);
    cdl_surface!(cdl::cdl_tasuki_gap);
    cdl_surface!(cdl::cdl_upside_gap_2crows);
    cdl_surface!(cdl::cdl_gap_side_side_white);
    cdl_surface!(cdl::cdl_breakaway);
    cdl_surface!(cdl::cdl_ladder_bottom);
    cdl_surface!(cdl::cdl_mat_hold);
    cdl_surface!(cdl::cdl_rise_fall_3methods);
    cdl_surface!(cdl::cdl_concealing_baby_swallow);
    cdl_surface!(cdl::cdl_xside_gap_3methods);
}

#[test]
fn candlestick_kicking_bearish_and_placeholder_into_errors() {
    let lookback = cdl::two_candle::cdl_kicking_lookback();
    let n = lookback + 1;
    let mut open = vec![10.0; n];
    let mut high = vec![10.3; n];
    let mut low = vec![9.7; n];
    let mut close = vec![10.1; n];

    let i = n - 1;
    let prev = i - 1;
    open[prev] = 10.0;
    high[prev] = 11.0;
    low[prev] = 10.0;
    close[prev] = 11.0; // bullish marubozu

    open[i] = 8.0;
    high[i] = 8.0;
    low[i] = 6.0;
    close[i] = 6.0; // bearish marubozu with larger body, gapped down

    let kick = cdl::cdl_kicking(&open, &high, &low, &close).expect("kicking");
    assert!(kick[i] < 0);
    let kick_len = cdl::cdl_kicking_by_length(&open, &high, &low, &close).expect("kicking len");
    assert!(kick_len[i] < kick[i]);

    // Bullish strong variant for coverage of kicking-by-length bullish branch.
    open[prev] = 11.0;
    high[prev] = 11.0;
    low[prev] = 10.0;
    close[prev] = 10.0; // bearish marubozu
    open[i] = 12.0;
    high[i] = 14.5;
    low[i] = 12.0;
    close[i] = 14.5; // bullish marubozu with larger body and gap up
    let kick_bull = cdl::cdl_kicking(&open, &high, &low, &close).expect("kicking bull");
    assert!(kick_bull[i] > 0);
    let kick_len_bull =
        cdl::cdl_kicking_by_length(&open, &high, &low, &close).expect("kicking len bull");
    assert!(kick_len_bull[i] > kick_bull[i]);

    let mut out_empty = Vec::<i32>::new();
    assert!(
        cdl::cdl_stick_sandwich_into(
            &[] as &[f64],
            &[] as &[f64],
            &[] as &[f64],
            &[] as &[f64],
            &mut out_empty,
        )
        .is_err()
    );
    let mut out_mismatch = vec![0; n];
    assert!(
        cdl::cdl_stick_sandwich_into(&open, &high, &low[..n - 1], &close, &mut out_mismatch)
            .is_err()
    );
    let mut out_small = vec![0; n - 1];
    assert!(cdl::cdl_stick_sandwich_into(&open, &high, &low, &close, &mut out_small).is_err());
    let min_len = cdl::three_candle::cdl_stick_sandwich_min_len();
    if min_len > 1 {
        let (o, h, l, c) = make_ohlc(min_len - 1);
        let mut out = vec![0; min_len - 1];
        assert!(cdl::cdl_stick_sandwich_into(&o, &h, &l, &c, &mut out).is_err());
    }

    assert!(cdl::three_candle::cdl_stick_sandwich_lookback() > 0);
    assert!(
        cdl::three_candle::cdl_stick_sandwich_min_len()
            > cdl::three_candle::cdl_stick_sandwich_lookback()
    );
}

#[test]
fn f16_large_period_numeric_conversion_error_paths() {
    let p = 70_000usize;
    let n = p + 20;

    let close16: Vec<f16> = (0..n)
        .map(|i| f16::from_f32(100.0 + (i as f32) * 0.001))
        .collect();
    let mut open16 = close16.clone();
    let mut high16 = close16.clone();
    let mut low16 = close16.clone();
    for i in 0..n {
        open16[i] = f16::from_f32(close16[i].to_f32() - 0.05);
        high16[i] = f16::from_f32(close16[i].to_f32() + 0.20);
        low16[i] = f16::from_f32(close16[i].to_f32() - 0.20);
    }

    let _ = adx::adx(&high16, &low16, &close16, p);
    let _ = dx::dx(&high16, &low16, &close16, p);
    let _ = midpoint::midpoint(&close16, p);
    let _ = stochastic::stochastic_fast(&high16, &low16, &close16, p, 3);
    let _ = stochastic::stochastic_full(&high16, &low16, &close16, p, 3, 3);
    let _ = stochrsi::stochrsi(&close16, p, p, 3, 3);

    let _ = stats::var(&close16, p);
    let _ = stats::stddev(&close16, p);
    let _ = stats::skew(&close16, p);
    let _ = stats::kurt(&close16, p);
    let _ = stats::zscore(&close16, p);
    let _ = stats::mad(&close16, p);
    let _ = stats::sem(&close16, p);
    let _ = stats::linearreg(&close16, p);
    let _ = stats::linearreg_slope(&close16, p);
    let _ = stats::linearreg_intercept(&close16, p);
    let _ = stats::linearreg_angle(&close16, p);
    let _ = stats::tsf(&close16, p);
    let _ = stats::cov(&close16, &close16, p);
    let _ = stats::correl(&close16, &close16, p);
    let _ = stats::beta(&close16, &close16, p);
}
