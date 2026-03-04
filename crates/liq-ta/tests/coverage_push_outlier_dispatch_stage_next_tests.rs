use half::f16;
use liq_ta::indicators::candlestick as cdl;
use liq_ta::indicators::{
    adx, adx_into, adxr, adxr_into, bollinger, bollinger_into, dx, dx_into, kama, kama_full,
    kama_full_into, kama_into, midpoint, midpoint_into, minus_dm_into, plus_dm_into, statistics,
    stochastic, stochastic_fast, stochastic_fast_into, stochastic_full, stochastic_full_into,
    stochastic_slow, stochrsi, stochrsi_into,
};
use liq_ta::precision::{PrecisionMode, with_precision_mode};

fn close_wave_f64(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let t = i as f64;
            100.0 + 0.2 * t + (t * 0.37).sin() * 1.5 + ((i % 7) as f64 - 3.0) * 0.05
        })
        .collect()
}

fn to_f32(data: &[f64]) -> Vec<f32> {
    data.iter().map(|&x| x as f32).collect()
}

fn to_f16(data: &[f64]) -> Vec<f16> {
    data.iter().map(|&x| f16::from_f64(x)).collect()
}

fn ohlc_from_close(close: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(close.len());
    let mut high = Vec::with_capacity(close.len());
    let mut low = Vec::with_capacity(close.len());

    for i in 0..close.len() {
        let o = if i == 0 { close[0] - 0.3 } else { close[i - 1] };
        let c = close[i];
        open.push(o);
        high.push(o.max(c) + 1.1);
        low.push(o.min(c) - 1.1);
    }

    (open, high, low, close.to_vec())
}

fn build_three_candle_base(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);

    for i in 0..len {
        let o = 120.0 - i as f64 * 0.1;
        let c = o - 0.2;
        open.push(o);
        close.push(c);
        high.push(o + 10.0);
        low.push(c - 10.0);
    }

    (open, high, low, close)
}

macro_rules! assert_ok_len {
    ($expr:expr, $len:expr, $ctx:expr) => {{
        let out = $expr.expect($ctx);
        assert_eq!(out.len(), $len);
    }};
}

#[test]
fn outlier_dispatch_adx_kama_midpoint_dx_stochrsi_statistics_matrix() {
    let close = close_wave_f64(96);
    let (open, high, low, close_for_ohlc) = ohlc_from_close(&close);
    let period = 14;

    let adx_f64 = adx(&high, &low, &close_for_ohlc, period).expect("adx f64");
    assert_eq!(adx_f64.adx.len(), close.len());
    assert!(adx_f64.adx.iter().skip(period * 2).any(|v| v.is_finite()));

    let high_f32 = to_f32(&high);
    let low_f32 = to_f32(&low);
    let close_f32 = to_f32(&close_for_ohlc);
    let adx_f32 = adx(&high_f32, &low_f32, &close_f32, period).expect("adx f32");
    assert_eq!(adx_f32.adx.len(), close.len());

    let high_f16 = to_f16(&high);
    let low_f16 = to_f16(&low);
    let close_f16 = to_f16(&close_for_ohlc);
    let adx_f16 = adx(&high_f16, &low_f16, &close_f16, period).expect("adx f16");
    assert_eq!(adx_f16.adx.len(), close.len());

    let dx_f16 = dx(&high_f16, &low_f16, &close_f16, 7).expect("dx f16");
    let adxr_f16 = adxr(&high_f16, &low_f16, &close_f16, 7).expect("adxr f16");
    assert_eq!(dx_f16.len(), close.len());
    assert_eq!(adxr_f16.len(), close.len());

    let midpoint_f16 = midpoint(&close_f16, 9).expect("midpoint f16");
    assert_eq!(midpoint_f16.len(), close.len());

    let kama_f32 = kama(&close_f32, 10).expect("kama f32");
    let kama_full_f32 = kama_full(&close_f32, 10, 2, 30).expect("kama_full f32");
    assert_eq!(kama_f32.len(), close.len());
    assert_eq!(kama_full_f32.len(), close.len());

    let stochrsi_f16 = stochrsi(&close_f16, 14, 14, 1, 3).expect("stochrsi f16");
    assert_eq!(stochrsi_f16.fastk.len(), close.len());
    assert_eq!(stochrsi_f16.fastd.len(), close.len());

    let stats_a_f32 = to_f32(&open);
    let stats_b_f32 = to_f32(&close_for_ohlc);
    let p = 12;
    with_precision_mode(PrecisionMode::High, || {
        let _ = statistics::stddev(&stats_a_f32, p).expect("stddev high");
        let _ = statistics::zscore(&stats_a_f32, p).expect("zscore high");
        let _ = statistics::cov(&stats_a_f32, &stats_b_f32, p).expect("cov high");
        let _ = statistics::correl(&stats_a_f32, &stats_b_f32, p).expect("correl high");
        let _ = statistics::beta(&stats_a_f32, &stats_b_f32, p).expect("beta high");
    });
    with_precision_mode(PrecisionMode::Fast, || {
        let _ = statistics::stddev(&stats_a_f32, p).expect("stddev fast");
        let _ = statistics::zscore(&stats_a_f32, p).expect("zscore fast");
        let _ = statistics::cov(&stats_a_f32, &stats_b_f32, p).expect("cov fast");
        let _ = statistics::correl(&stats_a_f32, &stats_b_f32, p).expect("correl fast");
        let _ = statistics::beta(&stats_a_f32, &stats_b_f32, p).expect("beta fast");
    });
}

#[test]
fn outlier_dispatch_bollinger_precision_and_generic_paths() {
    let data = close_wave_f64(96);
    let p = 20;

    let bb_f64 = bollinger(&data, p, 2.0).expect("bollinger f64");
    assert_eq!(bb_f64.middle.len(), data.len());

    let data_f32 = to_f32(&data);
    with_precision_mode(PrecisionMode::High, || {
        let bb = bollinger(&data_f32, p, 2.0_f32).expect("bollinger f32 high");
        assert_eq!(bb.middle.len(), data_f32.len());
    });
    with_precision_mode(PrecisionMode::Fast, || {
        let bb = bollinger(&data_f32, p, 2.0_f32).expect("bollinger f32 fast");
        assert_eq!(bb.middle.len(), data_f32.len());
    });

    let data_f16 = to_f16(&data);
    let bb_f16 = bollinger(&data_f16, p, f16::from_f32(2.0)).expect("bollinger f16");
    assert_eq!(bb_f16.middle.len(), data_f16.len());
}

#[test]
fn outlier_three_candle_targeted_branches() {
    let len = 40;
    let (mut open, mut high, mut low, mut close) = build_three_candle_base(len);
    let i1 = len - 3;
    let i2 = len - 2;
    let i3 = len - 1;

    // Force a valid Three Stars in the South on the last candle.
    open[i1] = 120.0;
    close[i1] = 110.0;
    high[i1] = 121.0;
    low[i1] = 95.0;

    open[i2] = 116.0;
    close[i2] = 112.0;
    high[i2] = 117.3;
    low[i2] = 92.0;

    open[i3] = 111.0;
    close[i3] = 110.5;
    high[i3] = 111.0;
    low[i3] = 110.5;

    let stars = cdl::cdl_3stars_in_south(&open, &high, &low, &close).expect("3stars");
    assert_eq!(stars.len(), len);
    assert_eq!(stars[i3], cdl::PATTERN_BULLISH);

    // Mutate only the last candle to fail the "inside second range" check path.
    high[i3] = 117.4;
    low[i3] = 117.3;
    open[i3] = 117.4;
    close[i3] = 117.3;
    let stars_fail = cdl::cdl_3stars_in_south(&open, &high, &low, &close).expect("3stars fail");
    assert_eq!(stars_fail[i3], cdl::PATTERN_NONE);

    // Build a sequence that reaches the final close-order rejection in identical 3 crows.
    let (mut open2, mut high2, mut low2, mut close2) = build_three_candle_base(len);
    open2[i1] = 120.0;
    close2[i1] = 110.0;
    high2[i1] = 121.0;
    low2[i1] = 109.0;

    open2[i2] = 110.0; // equals previous close
    close2[i2] = 109.0;
    high2[i2] = 110.5;
    low2[i2] = 108.8;

    open2[i3] = 109.5; // approx previous close (tolerance based)
    close2[i3] = 109.2; // >= close2[i2], should trigger final rejection
    high2[i3] = 109.7;
    low2[i3] = 109.0;

    let crows = cdl::cdl_identical_3crows(&open2, &high2, &low2, &close2).expect("3crows");
    assert_eq!(crows.len(), len);
    assert_eq!(crows[i3], cdl::PATTERN_NONE);
}

#[test]
fn outlier_candlestick_wrapper_matrix_all_types() {
    let len = 512;
    let (open, high, low, close) = build_three_candle_base(len);
    let open_f32 = to_f32(&open);
    let high_f32 = to_f32(&high);
    let low_f32 = to_f32(&low);
    let close_f32 = to_f32(&close);
    let open_f16 = to_f16(&open);
    let high_f16 = to_f16(&high);
    let low_f16 = to_f16(&low);
    let close_f16 = to_f16(&close);

    macro_rules! call_all_cdl_non_into {
        ($o:expr, $h:expr, $l:expr, $c:expr, $n:expr) => {{
            assert_ok_len!(cdl::cdl_doji($o, $h, $l, $c), $n, "cdl_doji");
            assert_ok_len!(
                cdl::cdl_dragonfly_doji($o, $h, $l, $c),
                $n,
                "cdl_dragonfly_doji"
            );
            assert_ok_len!(
                cdl::cdl_gravestone_doji($o, $h, $l, $c),
                $n,
                "cdl_gravestone_doji"
            );
            assert_ok_len!(
                cdl::cdl_longleg_doji($o, $h, $l, $c),
                $n,
                "cdl_longleg_doji"
            );
            assert_ok_len!(
                cdl::cdl_rickshaw_man($o, $h, $l, $c),
                $n,
                "cdl_rickshaw_man"
            );
            assert_ok_len!(cdl::cdl_marubozu($o, $h, $l, $c), $n, "cdl_marubozu");
            assert_ok_len!(
                cdl::cdl_closing_marubozu($o, $h, $l, $c),
                $n,
                "cdl_closing_marubozu"
            );
            assert_ok_len!(
                cdl::cdl_spinning_top($o, $h, $l, $c),
                $n,
                "cdl_spinning_top"
            );
            assert_ok_len!(cdl::cdl_high_wave($o, $h, $l, $c), $n, "cdl_high_wave");
            assert_ok_len!(cdl::cdl_long_line($o, $h, $l, $c), $n, "cdl_long_line");
            assert_ok_len!(cdl::cdl_short_line($o, $h, $l, $c), $n, "cdl_short_line");
            assert_ok_len!(cdl::cdl_hammer($o, $h, $l, $c), $n, "cdl_hammer");
            assert_ok_len!(cdl::cdl_hanging_man($o, $h, $l, $c), $n, "cdl_hanging_man");
            assert_ok_len!(
                cdl::cdl_inverted_hammer($o, $h, $l, $c),
                $n,
                "cdl_inverted_hammer"
            );
            assert_ok_len!(
                cdl::cdl_shooting_star($o, $h, $l, $c),
                $n,
                "cdl_shooting_star"
            );
            assert_ok_len!(cdl::cdl_takuri($o, $h, $l, $c), $n, "cdl_takuri");
            assert_ok_len!(cdl::cdl_belt_hold($o, $h, $l, $c), $n, "cdl_belt_hold");

            assert_ok_len!(cdl::cdl_engulfing($o, $h, $l, $c), $n, "cdl_engulfing");
            assert_ok_len!(cdl::cdl_harami($o, $h, $l, $c), $n, "cdl_harami");
            assert_ok_len!(
                cdl::cdl_harami_cross($o, $h, $l, $c),
                $n,
                "cdl_harami_cross"
            );
            assert_ok_len!(cdl::cdl_piercing($o, $h, $l, $c), $n, "cdl_piercing");
            assert_ok_len!(
                cdl::cdl_dark_cloud_cover($o, $h, $l, $c),
                $n,
                "cdl_dark_cloud_cover"
            );
            assert_ok_len!(cdl::cdl_doji_star($o, $h, $l, $c), $n, "cdl_doji_star");
            assert_ok_len!(cdl::cdl_kicking($o, $h, $l, $c), $n, "cdl_kicking");
            assert_ok_len!(
                cdl::cdl_kicking_by_length($o, $h, $l, $c),
                $n,
                "cdl_kicking_by_length"
            );
            assert_ok_len!(
                cdl::cdl_matching_low($o, $h, $l, $c),
                $n,
                "cdl_matching_low"
            );
            assert_ok_len!(
                cdl::cdl_homing_pigeon($o, $h, $l, $c),
                $n,
                "cdl_homing_pigeon"
            );
            assert_ok_len!(cdl::cdl_in_neck($o, $h, $l, $c), $n, "cdl_in_neck");
            assert_ok_len!(cdl::cdl_on_neck($o, $h, $l, $c), $n, "cdl_on_neck");
            assert_ok_len!(cdl::cdl_thrusting($o, $h, $l, $c), $n, "cdl_thrusting");
            assert_ok_len!(
                cdl::cdl_separating_lines($o, $h, $l, $c),
                $n,
                "cdl_separating_lines"
            );
            assert_ok_len!(
                cdl::cdl_counter_attack($o, $h, $l, $c),
                $n,
                "cdl_counter_attack"
            );
            assert_ok_len!(cdl::cdl_2crows($o, $h, $l, $c), $n, "cdl_2crows");
            assert_ok_len!(cdl::cdl_hikkake($o, $h, $l, $c), $n, "cdl_hikkake");
            assert_ok_len!(cdl::cdl_hikkake_mod($o, $h, $l, $c), $n, "cdl_hikkake_mod");

            assert_ok_len!(
                cdl::cdl_morning_star($o, $h, $l, $c),
                $n,
                "cdl_morning_star"
            );
            assert_ok_len!(
                cdl::cdl_evening_star($o, $h, $l, $c),
                $n,
                "cdl_evening_star"
            );
            assert_ok_len!(
                cdl::cdl_morning_doji_star($o, $h, $l, $c),
                $n,
                "cdl_morning_doji_star"
            );
            assert_ok_len!(
                cdl::cdl_evening_doji_star($o, $h, $l, $c),
                $n,
                "cdl_evening_doji_star"
            );
            assert_ok_len!(
                cdl::cdl_abandoned_baby($o, $h, $l, $c),
                $n,
                "cdl_abandoned_baby"
            );
            assert_ok_len!(
                cdl::cdl_3white_soldiers($o, $h, $l, $c),
                $n,
                "cdl_3white_soldiers"
            );
            assert_ok_len!(
                cdl::cdl_3black_crows($o, $h, $l, $c),
                $n,
                "cdl_3black_crows"
            );
            assert_ok_len!(cdl::cdl_3inside($o, $h, $l, $c), $n, "cdl_3inside");
            assert_ok_len!(cdl::cdl_3outside($o, $h, $l, $c), $n, "cdl_3outside");
            assert_ok_len!(
                cdl::cdl_3line_strike($o, $h, $l, $c),
                $n,
                "cdl_3line_strike"
            );
            assert_ok_len!(
                cdl::cdl_3stars_in_south($o, $h, $l, $c),
                $n,
                "cdl_3stars_in_south"
            );
            assert_ok_len!(cdl::cdl_tristar($o, $h, $l, $c), $n, "cdl_tristar");
            assert_ok_len!(
                cdl::cdl_identical_3crows($o, $h, $l, $c),
                $n,
                "cdl_identical_3crows"
            );
        }};
    }

    call_all_cdl_non_into!(&open, &high, &low, &close, len);
    call_all_cdl_non_into!(&open_f32, &high_f32, &low_f32, &close_f32, len);
    call_all_cdl_non_into!(&open_f16, &high_f16, &low_f16, &close_f16, len);
}

#[test]
fn outlier_statistics_and_stochastic_multitype_surface() {
    let close = close_wave_f64(1500);
    let (open, high, low, close_ohlc) = ohlc_from_close(&close);
    let high_f32 = to_f32(&high);
    let low_f32 = to_f32(&low);
    let close_f32 = to_f32(&close_ohlc);
    let high_f16 = to_f16(&high);
    let low_f16 = to_f16(&low);
    let close_f16 = to_f16(&close_ohlc);
    let n = close.len();

    let mut adx_out = vec![f64::NAN; n];
    let mut plus_di = vec![f64::NAN; n];
    let mut minus_di = vec![f64::NAN; n];
    adx_into(
        &high,
        &low,
        &close_ohlc,
        14,
        &mut adx_out,
        &mut plus_di,
        &mut minus_di,
    )
    .expect("adx_into f64");

    let mut adx_out_f32 = vec![f32::NAN; n];
    let mut plus_di_f32 = vec![f32::NAN; n];
    let mut minus_di_f32 = vec![f32::NAN; n];
    adx_into(
        &high_f32,
        &low_f32,
        &close_f32,
        14,
        &mut adx_out_f32,
        &mut plus_di_f32,
        &mut minus_di_f32,
    )
    .expect("adx_into f32");

    let mut adx_out_f16 = vec![f16::NAN; n];
    let mut plus_di_f16 = vec![f16::NAN; n];
    let mut minus_di_f16 = vec![f16::NAN; n];
    adx_into(
        &high_f16,
        &low_f16,
        &close_f16,
        14,
        &mut adx_out_f16,
        &mut plus_di_f16,
        &mut minus_di_f16,
    )
    .expect("adx_into f16");

    let mut dx_out = vec![f64::NAN; n];
    dx_into(&high, &low, &close_ohlc, 14, &mut dx_out).expect("dx_into f64");
    let mut adxr_out = vec![f64::NAN; n];
    adxr_into(&high, &low, &close_ohlc, 14, &mut adxr_out).expect("adxr_into f64");

    let mut plus_dm = vec![f64::NAN; n];
    let mut minus_dm = vec![f64::NAN; n];
    plus_dm_into(&high, &low, 14, &mut plus_dm).expect("plus_dm_into");
    minus_dm_into(&high, &low, 14, &mut minus_dm).expect("minus_dm_into");

    let mut midpoint_out = vec![f64::NAN; n];
    midpoint_into(&close_ohlc, 20, &mut midpoint_out).expect("midpoint_into f64");
    assert_eq!(midpoint_out.len(), n);

    let mut kama_out = vec![f64::NAN; n];
    kama_into(&close_ohlc, 10, &mut kama_out).expect("kama_into f64");
    let mut kama_full_out = vec![f64::NAN; n];
    kama_full_into(&close_ohlc, 10, 2, 30, &mut kama_full_out).expect("kama_full_into f64");

    let mut bb_out = liq_ta::indicators::BollingerOutput {
        middle: vec![f64::NAN; n],
        upper: vec![f64::NAN; n],
        lower: vec![f64::NAN; n],
    };
    bollinger_into(&close_ohlc, 20, 2.0, &mut bb_out).expect("bollinger_into f64");

    let mut k = vec![f64::NAN; n];
    let mut d = vec![f64::NAN; n];
    stochrsi_into(&close_ohlc, 14, 14, 3, 4, &mut k, &mut d).expect("stochrsi_into f64");

    let mut out_fast = liq_ta::indicators::StochasticOutput {
        k: vec![f64::NAN; n],
        d: vec![f64::NAN; n],
    };
    stochastic_fast_into(&high, &low, &close_ohlc, 14, 3, &mut out_fast)
        .expect("stochastic_fast_into f64");
    stochastic_full_into(&high, &low, &close_ohlc, 14, 3, 3, &mut out_fast)
        .expect("stochastic_full_into f64");
    let _ = stochastic_fast(&high, &low, &close_ohlc, 14, 3).expect("stochastic_fast");
    let _ = stochastic_full(&high, &low, &close_ohlc, 14, 3, 3).expect("stochastic_full");
    let _ = stochastic_slow(&high, &low, &close_ohlc, 14, 3).expect("stochastic_slow");
    let _ = stochastic(&high, &low, &close_ohlc, 14, 3, 1).expect("stochastic fast route");
    let _ = stochastic(&high, &low, &close_ohlc, 14, 3, 3).expect("stochastic full route");

    let mut high_nan = high.clone();
    high_nan[100] = f64::NAN;
    let _ = stochastic_fast(&high_nan, &low, &close_ohlc, 14, 3).expect("stochastic_fast nan path");
    let _ =
        stochastic_full(&high_nan, &low, &close_ohlc, 14, 3, 3).expect("stochastic_full nan path");

    let stats_a = close_ohlc.iter().copied().take(256).collect::<Vec<_>>();
    let stats_b = open.iter().copied().take(256).collect::<Vec<_>>();
    let stats_a_f32 = to_f32(&stats_a);
    let stats_b_f32 = to_f32(&stats_b);
    let stats_a_f16 = to_f16(&stats_a);
    let stats_b_f16 = to_f16(&stats_b);
    let p = 20;

    let _ = statistics::var(&stats_a, p).expect("var f64");
    let _ = statistics::stddev(&stats_a, p).expect("stddev f64");
    let _ = statistics::cov(&stats_a, &stats_b, p).expect("cov f64");
    let _ = statistics::zscore(&stats_a, p).expect("zscore f64");
    let _ = statistics::correl(&stats_a, &stats_b, p).expect("correl f64");
    let _ = statistics::beta(&stats_a, &stats_b, p).expect("beta f64");
    let _ = statistics::linearreg(&stats_a, p).expect("linearreg f64");
    let _ = statistics::linearreg_slope(&stats_a, p).expect("linearreg_slope f64");
    let _ = statistics::linearreg_intercept(&stats_a, p).expect("linearreg_intercept f64");
    let _ = statistics::linearreg_angle(&stats_a, p).expect("linearreg_angle f64");
    let _ = statistics::tsf(&stats_a, p).expect("tsf f64");

    let mut out_stats = vec![f64::NAN; stats_a.len()];
    statistics::skew_into(&stats_a, p, &mut out_stats).expect("skew_into f64");
    statistics::kurt_into(&stats_a, p, &mut out_stats).expect("kurt_into f64");
    statistics::mad_into(&stats_a, p, &mut out_stats).expect("mad_into f64");
    statistics::sem_into(&stats_a, p, &mut out_stats).expect("sem_into f64");

    with_precision_mode(PrecisionMode::High, || {
        let _ = statistics::var(&stats_a_f32, p).expect("var f32 high");
        let _ = statistics::stddev(&stats_a_f32, p).expect("stddev f32 high");
        let _ = statistics::cov(&stats_a_f32, &stats_b_f32, p).expect("cov f32 high");
        let _ = statistics::zscore(&stats_a_f32, p).expect("zscore f32 high");
        let _ = statistics::correl(&stats_a_f32, &stats_b_f32, p).expect("correl f32 high");
        let _ = statistics::beta(&stats_a_f32, &stats_b_f32, p).expect("beta f32 high");
    });
    with_precision_mode(PrecisionMode::Fast, || {
        let _ = statistics::var(&stats_a_f32, p).expect("var f32 fast");
        let _ = statistics::stddev(&stats_a_f32, p).expect("stddev f32 fast");
        let _ = statistics::cov(&stats_a_f32, &stats_b_f32, p).expect("cov f32 fast");
        let _ = statistics::zscore(&stats_a_f32, p).expect("zscore f32 fast");
        let _ = statistics::correl(&stats_a_f32, &stats_b_f32, p).expect("correl f32 fast");
        let _ = statistics::beta(&stats_a_f32, &stats_b_f32, p).expect("beta f32 fast");
    });

    let _ = statistics::var(&stats_a_f16, p).expect("var f16");
    let _ = statistics::stddev(&stats_a_f16, p).expect("stddev f16");
    let _ = statistics::cov(&stats_a_f16, &stats_b_f16, p).expect("cov f16");
    let _ = statistics::zscore(&stats_a_f16, p).expect("zscore f16");
    let _ = statistics::correl(&stats_a_f16, &stats_b_f16, p).expect("correl f16");
    let _ = statistics::beta(&stats_a_f16, &stats_b_f16, p).expect("beta f16");
    let _ = statistics::linearreg(&stats_a_f16, p).expect("linearreg f16");
    let _ = statistics::linearreg_slope(&stats_a_f16, p).expect("linearreg_slope f16");
    let _ = statistics::linearreg_intercept(&stats_a_f16, p).expect("linearreg_intercept f16");
    let _ = statistics::linearreg_angle(&stats_a_f16, p).expect("linearreg_angle f16");
    let _ = statistics::tsf(&stats_a_f16, p).expect("tsf f16");
}
