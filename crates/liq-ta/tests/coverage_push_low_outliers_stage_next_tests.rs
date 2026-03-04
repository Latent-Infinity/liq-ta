use half::f16;
use liq_ta::indicators::candlestick as cdl;
use liq_ta::indicators::{adx, bollinger, dx, kama, midpoint, statistics, stochastic, stochrsi};
use liq_ta::precision::{PrecisionMode, with_precision_mode};

macro_rules! call_low_cdl_non_into {
    ($o:expr, $h:expr, $l:expr, $c:expr, $assert_expr:expr) => {{
        $assert_expr(cdl::cdl_doji($o, $h, $l, $c));
        $assert_expr(cdl::cdl_dragonfly_doji($o, $h, $l, $c));
        $assert_expr(cdl::cdl_gravestone_doji($o, $h, $l, $c));
        $assert_expr(cdl::cdl_longleg_doji($o, $h, $l, $c));
        $assert_expr(cdl::cdl_rickshaw_man($o, $h, $l, $c));
        $assert_expr(cdl::cdl_marubozu($o, $h, $l, $c));
        $assert_expr(cdl::cdl_closing_marubozu($o, $h, $l, $c));
        $assert_expr(cdl::cdl_spinning_top($o, $h, $l, $c));
        $assert_expr(cdl::cdl_high_wave($o, $h, $l, $c));
        $assert_expr(cdl::cdl_long_line($o, $h, $l, $c));
        $assert_expr(cdl::cdl_short_line($o, $h, $l, $c));
        $assert_expr(cdl::cdl_hammer($o, $h, $l, $c));
        $assert_expr(cdl::cdl_hanging_man($o, $h, $l, $c));
        $assert_expr(cdl::cdl_inverted_hammer($o, $h, $l, $c));
        $assert_expr(cdl::cdl_shooting_star($o, $h, $l, $c));
        $assert_expr(cdl::cdl_takuri($o, $h, $l, $c));
        $assert_expr(cdl::cdl_belt_hold($o, $h, $l, $c));

        $assert_expr(cdl::cdl_engulfing($o, $h, $l, $c));
        $assert_expr(cdl::cdl_harami($o, $h, $l, $c));
        $assert_expr(cdl::cdl_harami_cross($o, $h, $l, $c));
        $assert_expr(cdl::cdl_piercing($o, $h, $l, $c));
        $assert_expr(cdl::cdl_dark_cloud_cover($o, $h, $l, $c));
        $assert_expr(cdl::cdl_doji_star($o, $h, $l, $c));
        $assert_expr(cdl::cdl_kicking($o, $h, $l, $c));
        $assert_expr(cdl::cdl_kicking_by_length($o, $h, $l, $c));
        $assert_expr(cdl::cdl_matching_low($o, $h, $l, $c));
        $assert_expr(cdl::cdl_homing_pigeon($o, $h, $l, $c));
        $assert_expr(cdl::cdl_in_neck($o, $h, $l, $c));
        $assert_expr(cdl::cdl_on_neck($o, $h, $l, $c));
        $assert_expr(cdl::cdl_thrusting($o, $h, $l, $c));
        $assert_expr(cdl::cdl_separating_lines($o, $h, $l, $c));
        $assert_expr(cdl::cdl_counter_attack($o, $h, $l, $c));
        $assert_expr(cdl::cdl_2crows($o, $h, $l, $c));
        $assert_expr(cdl::cdl_hikkake($o, $h, $l, $c));
        $assert_expr(cdl::cdl_hikkake_mod($o, $h, $l, $c));

        $assert_expr(cdl::cdl_morning_star($o, $h, $l, $c));
        $assert_expr(cdl::cdl_evening_star($o, $h, $l, $c));
        $assert_expr(cdl::cdl_morning_doji_star($o, $h, $l, $c));
        $assert_expr(cdl::cdl_evening_doji_star($o, $h, $l, $c));
        $assert_expr(cdl::cdl_abandoned_baby($o, $h, $l, $c));
        $assert_expr(cdl::cdl_3white_soldiers($o, $h, $l, $c));
        $assert_expr(cdl::cdl_3black_crows($o, $h, $l, $c));
        $assert_expr(cdl::cdl_3inside($o, $h, $l, $c));
        $assert_expr(cdl::cdl_3outside($o, $h, $l, $c));
        $assert_expr(cdl::cdl_3line_strike($o, $h, $l, $c));
        $assert_expr(cdl::cdl_3stars_in_south($o, $h, $l, $c));
        $assert_expr(cdl::cdl_tristar($o, $h, $l, $c));
        $assert_expr(cdl::cdl_identical_3crows($o, $h, $l, $c));
    }};
}

macro_rules! call_low_cdl_into {
    ($o:expr, $h:expr, $l:expr, $c:expr, $out:expr, $assert_expr:expr) => {{
        $assert_expr(cdl::cdl_doji_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_dragonfly_doji_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_gravestone_doji_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_longleg_doji_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_rickshaw_man_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_marubozu_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_closing_marubozu_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_spinning_top_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_high_wave_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_long_line_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_short_line_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_hammer_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_hanging_man_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_inverted_hammer_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_shooting_star_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_takuri_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_belt_hold_into($o, $h, $l, $c, $out));

        $assert_expr(cdl::cdl_engulfing_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_harami_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_harami_cross_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_piercing_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_dark_cloud_cover_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_doji_star_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_kicking_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_kicking_by_length_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_matching_low_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_homing_pigeon_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_in_neck_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_on_neck_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_thrusting_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_separating_lines_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_counter_attack_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_2crows_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_hikkake_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_hikkake_mod_into($o, $h, $l, $c, $out));

        $assert_expr(cdl::cdl_morning_star_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_evening_star_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_morning_doji_star_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_evening_doji_star_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_abandoned_baby_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_3white_soldiers_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_3black_crows_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_3inside_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_3outside_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_3line_strike_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_3stars_in_south_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_tristar_into($o, $h, $l, $c, $out));
        $assert_expr(cdl::cdl_identical_3crows_into($o, $h, $l, $c, $out));
    }};
}

macro_rules! touch_low_cdl_consts {
    () => {{
        let _ = cdl::single::cdl_doji_lookback();
        let _ = cdl::single::cdl_doji_min_len();
        let _ = cdl::single::cdl_dragonfly_doji_lookback();
        let _ = cdl::single::cdl_dragonfly_doji_min_len();
        let _ = cdl::single::cdl_gravestone_doji_lookback();
        let _ = cdl::single::cdl_gravestone_doji_min_len();
        let _ = cdl::single::cdl_longleg_doji_lookback();
        let _ = cdl::single::cdl_longleg_doji_min_len();
        let _ = cdl::single::cdl_rickshaw_man_lookback();
        let _ = cdl::single::cdl_rickshaw_man_min_len();
        let _ = cdl::single::cdl_marubozu_lookback();
        let _ = cdl::single::cdl_marubozu_min_len();
        let _ = cdl::single::cdl_closing_marubozu_lookback();
        let _ = cdl::single::cdl_closing_marubozu_min_len();
        let _ = cdl::single::cdl_spinning_top_lookback();
        let _ = cdl::single::cdl_spinning_top_min_len();
        let _ = cdl::single::cdl_high_wave_lookback();
        let _ = cdl::single::cdl_high_wave_min_len();
        let _ = cdl::single::cdl_long_line_lookback();
        let _ = cdl::single::cdl_long_line_min_len();
        let _ = cdl::single::cdl_short_line_lookback();
        let _ = cdl::single::cdl_short_line_min_len();
        let _ = cdl::single::cdl_hammer_lookback();
        let _ = cdl::single::cdl_hammer_min_len();
        let _ = cdl::single::cdl_hanging_man_lookback();
        let _ = cdl::single::cdl_hanging_man_min_len();
        let _ = cdl::single::cdl_inverted_hammer_lookback();
        let _ = cdl::single::cdl_inverted_hammer_min_len();
        let _ = cdl::single::cdl_shooting_star_lookback();
        let _ = cdl::single::cdl_shooting_star_min_len();
        let _ = cdl::single::cdl_takuri_lookback();
        let _ = cdl::single::cdl_takuri_min_len();
        let _ = cdl::single::cdl_belt_hold_lookback();
        let _ = cdl::single::cdl_belt_hold_min_len();

        let _ = cdl::two_candle::cdl_engulfing_lookback();
        let _ = cdl::two_candle::cdl_engulfing_min_len();
        let _ = cdl::two_candle::cdl_harami_lookback();
        let _ = cdl::two_candle::cdl_harami_min_len();
        let _ = cdl::two_candle::cdl_harami_cross_lookback();
        let _ = cdl::two_candle::cdl_harami_cross_min_len();
        let _ = cdl::two_candle::cdl_piercing_lookback();
        let _ = cdl::two_candle::cdl_piercing_min_len();
        let _ = cdl::two_candle::cdl_dark_cloud_cover_lookback();
        let _ = cdl::two_candle::cdl_dark_cloud_cover_min_len();
        let _ = cdl::two_candle::cdl_doji_star_lookback();
        let _ = cdl::two_candle::cdl_doji_star_min_len();
        let _ = cdl::two_candle::cdl_kicking_lookback();
        let _ = cdl::two_candle::cdl_kicking_min_len();
        let _ = cdl::two_candle::cdl_kicking_by_length_lookback();
        let _ = cdl::two_candle::cdl_kicking_by_length_min_len();
        let _ = cdl::two_candle::cdl_matching_low_lookback();
        let _ = cdl::two_candle::cdl_matching_low_min_len();
        let _ = cdl::two_candle::cdl_homing_pigeon_lookback();
        let _ = cdl::two_candle::cdl_homing_pigeon_min_len();
        let _ = cdl::two_candle::cdl_in_neck_lookback();
        let _ = cdl::two_candle::cdl_in_neck_min_len();
        let _ = cdl::two_candle::cdl_on_neck_lookback();
        let _ = cdl::two_candle::cdl_on_neck_min_len();
        let _ = cdl::two_candle::cdl_thrusting_lookback();
        let _ = cdl::two_candle::cdl_thrusting_min_len();
        let _ = cdl::two_candle::cdl_separating_lines_lookback();
        let _ = cdl::two_candle::cdl_separating_lines_min_len();
        let _ = cdl::two_candle::cdl_counter_attack_lookback();
        let _ = cdl::two_candle::cdl_counter_attack_min_len();
        let _ = cdl::two_candle::cdl_2crows_lookback();
        let _ = cdl::two_candle::cdl_2crows_min_len();
        let _ = cdl::two_candle::cdl_hikkake_lookback();
        let _ = cdl::two_candle::cdl_hikkake_min_len();
        let _ = cdl::two_candle::cdl_hikkake_mod_lookback();
        let _ = cdl::two_candle::cdl_hikkake_mod_min_len();

        let _ = cdl::three_candle::cdl_morning_star_lookback();
        let _ = cdl::three_candle::cdl_morning_star_min_len();
        let _ = cdl::three_candle::cdl_evening_star_lookback();
        let _ = cdl::three_candle::cdl_evening_star_min_len();
        let _ = cdl::three_candle::cdl_morning_doji_star_lookback();
        let _ = cdl::three_candle::cdl_morning_doji_star_min_len();
        let _ = cdl::three_candle::cdl_evening_doji_star_lookback();
        let _ = cdl::three_candle::cdl_evening_doji_star_min_len();
        let _ = cdl::three_candle::cdl_abandoned_baby_lookback();
        let _ = cdl::three_candle::cdl_abandoned_baby_min_len();
        let _ = cdl::three_candle::cdl_3white_soldiers_lookback();
        let _ = cdl::three_candle::cdl_3white_soldiers_min_len();
        let _ = cdl::three_candle::cdl_3black_crows_lookback();
        let _ = cdl::three_candle::cdl_3black_crows_min_len();
        let _ = cdl::three_candle::cdl_3inside_lookback();
        let _ = cdl::three_candle::cdl_3inside_min_len();
        let _ = cdl::three_candle::cdl_3outside_lookback();
        let _ = cdl::three_candle::cdl_3outside_min_len();
        let _ = cdl::three_candle::cdl_3line_strike_lookback();
        let _ = cdl::three_candle::cdl_3line_strike_min_len();
        let _ = cdl::three_candle::cdl_3stars_in_south_lookback();
        let _ = cdl::three_candle::cdl_3stars_in_south_min_len();
        let _ = cdl::three_candle::cdl_tristar_lookback();
        let _ = cdl::three_candle::cdl_tristar_min_len();
        let _ = cdl::three_candle::cdl_identical_3crows_lookback();
        let _ = cdl::three_candle::cdl_identical_3crows_min_len();
    }};
}

fn assert_err_vec_i32(res: liq_ta::error::Result<Vec<i32>>) {
    assert!(res.is_err());
}

fn assert_ok_vec_i32(res: liq_ta::error::Result<Vec<i32>>) {
    assert!(res.is_ok());
}

fn assert_err_unit(res: liq_ta::error::Result<()>) {
    assert!(res.is_err());
}

fn assert_ok_unit(res: liq_ta::error::Result<()>) {
    assert!(res.is_ok());
}

fn ignore_vec_i32(_: liq_ta::error::Result<Vec<i32>>) {}

fn ignore_unit(_: liq_ta::error::Result<()>) {}

fn make_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    let mut prev = 100.0_f64;
    for i in 0..n {
        let drift = (i as f64) * 0.01;
        let wave = ((i % 13) as f64 - 6.0) * 0.15;
        let o = prev + wave;
        let c = o + if i % 2 == 0 { 0.55 } else { -0.45 } + drift.sin() * 0.1;
        let h = o.max(c) + 0.4 + ((i % 5) as f64) * 0.05;
        let l = o.min(c) - 0.4 - ((i % 7) as f64) * 0.04;
        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);
        prev = c;
    }
    (open, high, low, close)
}

#[test]
fn coverage_low_outlier_lookback_min_len_and_builder_surfaces() {
    touch_low_cdl_consts!();

    let p = 7usize;
    let _ = adx::adx_lookback(p);
    let _ = adx::adx_min_len(p);
    let _ = adx::di_lookback(p);
    let _ = adx::di_min_len(p);

    let _ = dx::adxr_lookback(p);
    let _ = dx::adxr_min_len(p);
    let _ = dx::dx_lookback(p);
    let _ = dx::dx_min_len(p);
    let _ = dx::dm_lookback(p);
    let _ = dx::dm_min_len(p);

    let _ = kama::kama_lookback(0);
    let _ = kama::kama_lookback(p);
    let _ = kama::kama_min_len(p);

    let _ = midpoint::midpoint_lookback(0);
    let _ = midpoint::midpoint_lookback(p);
    let _ = midpoint::midpoint_min_len(0);
    let _ = midpoint::midpoint_min_len(p);

    let _ = bollinger::bollinger_lookback(0);
    let _ = bollinger::bollinger_lookback(p);
    let _ = bollinger::bollinger_min_len(p);

    let _ = stochastic::stochastic_k_lookback(p);
    let _ = stochastic::stochastic_d_lookback(p, 3);
    let _ = stochastic::stochastic_min_len(p, 3);

    let _ = stochrsi::stochrsi_k_lookback(p, p);
    let _ = stochrsi::stochrsi_d_lookback(p, p, 3);
    let _ = stochrsi::stochrsi_min_len(p, p, 3);

    let _ = statistics::var_lookback(0);
    let _ = statistics::var_min_len(p);
    let _ = statistics::stddev_lookback(0);
    let _ = statistics::stddev_min_len(p);
    let _ = statistics::skew_lookback(0);
    let _ = statistics::skew_min_len(p);
    let _ = statistics::kurt_lookback(0);
    let _ = statistics::kurt_min_len(p);
    let _ = statistics::cov_lookback(0);
    let _ = statistics::cov_min_len(p);
    let _ = statistics::zscore_lookback(0);
    let _ = statistics::zscore_min_len(p);
    let _ = statistics::mad_lookback(0);
    let _ = statistics::mad_min_len(p);
    let _ = statistics::sem_lookback(0);
    let _ = statistics::sem_min_len(p);
    let _ = statistics::correl_lookback(0);
    let _ = statistics::correl_min_len(p);
    let _ = statistics::beta_lookback(0);
    let _ = statistics::beta_min_len(p);
    let _ = statistics::linearreg_lookback(0);
    let _ = statistics::linearreg_min_len(p);
    let _ = statistics::linearreg_slope_lookback(0);
    let _ = statistics::linearreg_slope_min_len(p);
    let _ = statistics::linearreg_intercept_lookback(0);
    let _ = statistics::linearreg_intercept_min_len(p);
    let _ = statistics::linearreg_angle_lookback(0);
    let _ = statistics::linearreg_angle_min_len(p);
    let _ = statistics::tsf_lookback(0);
    let _ = statistics::tsf_min_len(p);

    let bb = bollinger::Bollinger::new()
        .with_period(11)
        .with_std_dev(2.5);
    let _ = bb.period();
    let _ = bb.std_dev();
    let _ = bb.lookback();
    let _ = bb.min_len();

    let st = stochastic::Stochastic::new()
        .with_k_period(9)
        .with_d_period(4)
        .with_k_slowing(2);
    let _ = st.k_period();
    let _ = st.d_period();
    let _ = st.k_slowing();
    let _ = st.k_lookback();
    let _ = st.d_lookback();
    let _ = st.min_len();
}

#[test]
fn coverage_low_outlier_candlestick_flat_and_validation_matrix() {
    let empty: [f64; 0] = [];
    let mut out_empty: [i32; 0] = [];

    call_low_cdl_non_into!(&empty, &empty, &empty, &empty, assert_err_vec_i32);
    call_low_cdl_into!(
        &empty,
        &empty,
        &empty,
        &empty,
        &mut out_empty,
        assert_err_unit
    );

    let one_open = [100.0_f64];
    let one_high = [100.5_f64];
    let one_low = [99.5_f64];
    let one_close = [100.1_f64];
    let mut one_out = [0_i32; 1];
    call_low_cdl_non_into!(&one_open, &one_high, &one_low, &one_close, ignore_vec_i32);
    call_low_cdl_into!(
        &one_open,
        &one_high,
        &one_low,
        &one_close,
        &mut one_out,
        ignore_unit
    );
    let one_open32 = [100.0_f32];
    let one_high32 = [100.5_f32];
    let one_low32 = [99.5_f32];
    let one_close32 = [100.1_f32];
    let mut one_out32 = [0_i32; 1];
    call_low_cdl_non_into!(
        &one_open32,
        &one_high32,
        &one_low32,
        &one_close32,
        ignore_vec_i32
    );
    call_low_cdl_into!(
        &one_open32,
        &one_high32,
        &one_low32,
        &one_close32,
        &mut one_out32,
        ignore_unit
    );
    let one_open16 = [f16::from_f32(100.0)];
    let one_high16 = [f16::from_f32(100.5)];
    let one_low16 = [f16::from_f32(99.5)];
    let one_close16 = [f16::from_f32(100.1)];
    let mut one_out16 = [0_i32; 1];
    call_low_cdl_non_into!(
        &one_open16,
        &one_high16,
        &one_low16,
        &one_close16,
        ignore_vec_i32
    );
    call_low_cdl_into!(
        &one_open16,
        &one_high16,
        &one_low16,
        &one_close16,
        &mut one_out16,
        ignore_unit
    );

    let n = 128usize;
    let open = vec![100.0_f64; n];
    let high = vec![100.0_f64; n];
    let low = vec![100.0_f64; n];
    let close = vec![100.0_f64; n];

    let mut out_small = vec![0_i32; 64];
    let mut out_ok = vec![0_i32; n];

    call_low_cdl_into!(&open, &high, &low, &close, &mut out_small, assert_err_unit);
    call_low_cdl_non_into!(&open, &high, &low, &close, assert_ok_vec_i32);
    call_low_cdl_into!(&open, &high, &low, &close, &mut out_ok, assert_ok_unit);

    let open32: Vec<f32> = open.iter().map(|&v| v as f32).collect();
    let high32: Vec<f32> = high.iter().map(|&v| v as f32).collect();
    let low32: Vec<f32> = low.iter().map(|&v| v as f32).collect();
    let close32: Vec<f32> = close.iter().map(|&v| v as f32).collect();
    let mut out_small32 = vec![0_i32; 64];
    let mut out_ok32 = vec![0_i32; n];
    call_low_cdl_into!(
        &open32,
        &high32,
        &low32,
        &close32,
        &mut out_small32,
        assert_err_unit
    );
    call_low_cdl_non_into!(&open32, &high32, &low32, &close32, assert_ok_vec_i32);
    call_low_cdl_into!(
        &open32,
        &high32,
        &low32,
        &close32,
        &mut out_ok32,
        assert_ok_unit
    );

    let open16: Vec<f16> = open.iter().map(|&v| f16::from_f32(v as f32)).collect();
    let high16: Vec<f16> = high.iter().map(|&v| f16::from_f32(v as f32)).collect();
    let low16: Vec<f16> = low.iter().map(|&v| f16::from_f32(v as f32)).collect();
    let close16: Vec<f16> = close.iter().map(|&v| f16::from_f32(v as f32)).collect();
    let mut out_small16 = vec![0_i32; 64];
    let mut out_ok16 = vec![0_i32; n];
    call_low_cdl_into!(
        &open16,
        &high16,
        &low16,
        &close16,
        &mut out_small16,
        assert_err_unit
    );
    call_low_cdl_non_into!(&open16, &high16, &low16, &close16, assert_ok_vec_i32);
    call_low_cdl_into!(
        &open16,
        &high16,
        &low16,
        &close16,
        &mut out_ok16,
        assert_ok_unit
    );

    // Target remaining single-candle branches that require trend context + zero-range edge candles.
    let n_ctx = 48usize;
    let mut up_open = Vec::with_capacity(n_ctx);
    let mut up_high = Vec::with_capacity(n_ctx);
    let mut up_low = Vec::with_capacity(n_ctx);
    let mut up_close = Vec::with_capacity(n_ctx);
    for i in 0..n_ctx {
        let v = 10.0 + i as f64 * 0.2;
        up_open.push(v);
        up_high.push(v);
        up_low.push(v);
        up_close.push(v);
    }
    let mut down_open = Vec::with_capacity(n_ctx);
    let mut down_high = Vec::with_capacity(n_ctx);
    let mut down_low = Vec::with_capacity(n_ctx);
    let mut down_close = Vec::with_capacity(n_ctx);
    for i in 0..n_ctx {
        let v = 100.0 - i as f64 * 0.2;
        down_open.push(v);
        down_high.push(v);
        down_low.push(v);
        down_close.push(v);
    }
    let mut branch_out = vec![0_i32; n_ctx];
    cdl::cdl_hammer_into(
        &down_open,
        &down_high,
        &down_low,
        &down_close,
        &mut branch_out,
    )
    .expect("hammer zero-range branch");
    cdl::cdl_inverted_hammer_into(
        &down_open,
        &down_high,
        &down_low,
        &down_close,
        &mut branch_out,
    )
    .expect("inverted hammer zero-range branch");
    cdl::cdl_hanging_man_into(&up_open, &up_high, &up_low, &up_close, &mut branch_out)
        .expect("hanging man zero-range branch");
    cdl::cdl_shooting_star_into(&up_open, &up_high, &up_low, &up_close, &mut branch_out)
        .expect("shooting star zero-range branch");

    // Belt hold bullish/bearish outputs.
    let belt_open = [10.0, 11.0];
    let belt_high = [11.0, 11.0];
    let belt_low = [10.0, 10.0];
    let belt_close = [11.0, 10.0];
    let mut belt_out = [0_i32; 2];
    let _ = cdl::cdl_belt_hold_into(
        &belt_open,
        &belt_high,
        &belt_low,
        &belt_close,
        &mut belt_out,
    );

    // Dragonfly: pass doji + upper-short checks but fail lower-shadow threshold.
    let d_open = [10.0_f64];
    let d_high = [10.0002_f64];
    let d_low = [9.99995_f64];
    let d_close = [10.0_f64];
    let mut d_out = [0_i32; 1];
    cdl::cdl_dragonfly_doji_into(&d_open, &d_high, &d_low, &d_close, &mut d_out)
        .expect("dragonfly lower-shadow threshold branch");
}

#[test]
fn coverage_low_outlier_dispatch_and_precision_paths() {
    let n = 256usize;
    let (_open, high, low, close) = make_ohlc(n);
    let high32: Vec<f32> = high.iter().map(|&v| v as f32).collect();
    let low32: Vec<f32> = low.iter().map(|&v| v as f32).collect();
    let close32: Vec<f32> = close.iter().map(|&v| v as f32).collect();
    let high16: Vec<f16> = high.iter().map(|&v| f16::from_f32(v as f32)).collect();
    let low16: Vec<f16> = low.iter().map(|&v| f16::from_f32(v as f32)).collect();
    let close16: Vec<f16> = close.iter().map(|&v| f16::from_f32(v as f32)).collect();

    let _ = adx::adx(&high, &low, &close, 14).expect("adx f64");
    let _ = adx::adx::<f32>(&high32, &low32, &close32, 14).expect("adx f32");
    let _ = adx::adx::<f16>(&high16, &low16, &close16, 14).expect("adx f16");

    let _ = dx::dx(&high, &low, &close, 14).expect("dx f64");
    let _ = dx::dx(&high32, &low32, &close32, 14).expect("dx f32");
    let _ = dx::dx(&high16, &low16, &close16, 14).expect("dx f16");
    let _ = dx::adxr(&high, &low, &close, 14).expect("adxr f64");
    let _ = dx::adxr(&high32, &low32, &close32, 14).expect("adxr f32");
    let _ = dx::adxr(&high16, &low16, &close16, 14).expect("adxr f16");
    let _ = dx::plus_dm(&high, &low, 14).expect("plus_dm f64");
    let _ = dx::plus_dm(&high32, &low32, 14).expect("plus_dm f32");
    let _ = dx::minus_dm(&high, &low, 14).expect("minus_dm f64");
    let _ = dx::minus_dm(&high16, &low16, 14).expect("minus_dm f16");

    let _ = kama::kama::<f64>(&close, 10).expect("kama f64");
    let _ = kama::kama::<f32>(&close32, 10).expect("kama f32");
    let _ = kama::kama::<f16>(&close16, 10).expect("kama f16");
    let _ = kama::kama_full(&close, 10, 2, 30).expect("kama_full f64");
    let _ = kama::kama_full::<f32>(&close32, 10, 2, 30).expect("kama_full f32");
    let _ = kama::kama_full::<f16>(&close16, 10, 2, 30).expect("kama_full f16");

    let short16 = vec![f16::from_f32(1.0); 10];
    let mut short16_out = vec![f16::NAN; short16.len()];
    kama::kama_full_into(&short16, 10, 2, 30, &mut short16_out).expect("kama_full_into early exit");
    let short32 = vec![1.0_f32; 10];
    let mut short32_out = vec![f32::NAN; short32.len()];
    kama::kama_full_into(&short32, 10, 2, 30, &mut short32_out)
        .expect("kama_full_into f32 early exit");
    let flat16 = vec![f16::from_f32(42.0); 64];
    let mut flat16_out = vec![f16::NAN; flat16.len()];
    kama::kama_full_into(&flat16, 10, 2, 30, &mut flat16_out)
        .expect("kama_full_into zero volatility");

    let bb64 = bollinger::bollinger(&close, 20, 2.0_f64).expect("bollinger f64");
    assert_eq!(bb64.middle.len(), close.len());
    with_precision_mode(PrecisionMode::High, || {
        let bb32 = bollinger::bollinger(&close32, 20, 2.0_f32).expect("bollinger f32 high");
        assert_eq!(bb32.upper.len(), close32.len());
    });
    with_precision_mode(PrecisionMode::Fast, || {
        let bb32 = bollinger::bollinger(&close32, 20, 2.0_f32).expect("bollinger f32 fast");
        assert_eq!(bb32.lower.len(), close32.len());
    });
    let _ = bollinger::bollinger(&close16, 20, f16::from_f32(2.0)).expect("bollinger f16");

    let mut bb_out = bollinger::BollingerOutput {
        middle: vec![f64::NAN; close.len()],
        upper: vec![f64::NAN; close.len()],
        lower: vec![f64::NAN; close.len()],
    };
    let _ = bollinger::bollinger_into(&close, 20, 2.0_f64, &mut bb_out).expect("bollinger_into ok");
    let bad_period = bollinger::bollinger_into(&close, 0, 2.0_f64, &mut bb_out);
    assert!(bad_period.is_err());
    let empty: [f64; 0] = [];
    let empty_res = bollinger::bollinger_into(&empty, 5, 2.0_f64, &mut bb_out);
    assert!(empty_res.is_err());
    let short_res = bollinger::bollinger_into(&close[..5], 20, 2.0_f64, &mut bb_out);
    assert!(short_res.is_err());
    let mut bb_out_bad_upper = bollinger::BollingerOutput {
        middle: vec![f64::NAN; close.len()],
        upper: vec![f64::NAN; close.len() - 1],
        lower: vec![f64::NAN; close.len()],
    };
    assert!(bollinger::bollinger_into(&close, 20, 2.0_f64, &mut bb_out_bad_upper).is_err());
    let mut bb_out_bad_lower = bollinger::BollingerOutput {
        middle: vec![f64::NAN; close.len()],
        upper: vec![f64::NAN; close.len()],
        lower: vec![f64::NAN; close.len() - 1],
    };
    assert!(bollinger::bollinger_into(&close, 20, 2.0_f64, &mut bb_out_bad_lower).is_err());

    let _ = bollinger::rolling_stddev(&close, 20).expect("rolling_stddev f64");
    let _ = bollinger::rolling_stddev(&close32, 20).expect("rolling_stddev f32");
    let _ = bollinger::rolling_stddev(&close16, 20).expect("rolling_stddev f16");
    let _ = bollinger::rolling_stddev(&close, 1).expect("rolling_stddev period1");
    let mut std_out = vec![f64::NAN; close.len()];
    let _ =
        bollinger::rolling_stddev_into(&close, 20, &mut std_out).expect("rolling_stddev_into ok");
    assert!(bollinger::rolling_stddev_into(&close, 0, &mut std_out).is_err());
    assert!(bollinger::rolling_stddev_into(&empty, 5, &mut []).is_err());
    assert!(bollinger::rolling_stddev_into(&close[..5], 20, &mut std_out[..5]).is_err());
    assert!(bollinger::rolling_stddev_into(&close, 20, &mut std_out[..(close.len() - 1)]).is_err());

    let bb_cfg = bollinger::Bollinger::new()
        .with_period(20)
        .with_std_dev(2.1);
    let _ = bb_cfg.compute(&close).expect("bollinger cfg compute");
    let _ = bb_cfg
        .compute_into(&close, &mut bb_out)
        .expect("bollinger cfg compute_into");

    let long_data: Vec<f64> = (0..1200).map(|i| 10.0 + (i as f64) * 0.01).collect();
    let _ = midpoint::midpoint(&long_data, 25).expect("midpoint f64 vhgw");
    let _ = midpoint::midpoint(&close32, 20).expect("midpoint f32");
    let _ = midpoint::midpoint(&close16, 20).expect("midpoint f16");
    let non_finite_f16 = vec![f16::from_f32(1.0), f16::NAN, f16::INFINITY];
    let mut non_finite_out = vec![f16::from_f32(0.0); non_finite_f16.len()];
    midpoint::midpoint_into(&non_finite_f16, 1, &mut non_finite_out).expect("midpoint period1 f16");
    assert!(non_finite_out[1].is_nan());
    assert!(non_finite_out[2].is_nan());

    let sto_fast64 =
        stochastic::stochastic_fast(&high, &low, &close, 14, 3).expect("stochastic_fast f64");
    assert_eq!(sto_fast64.k.len(), close.len());
    let sto_fast32 =
        stochastic::stochastic_fast(&high32, &low32, &close32, 14, 3).expect("stochastic_fast f32");
    assert_eq!(sto_fast32.d.len(), close32.len());
    let sto_fast16 =
        stochastic::stochastic_fast(&high16, &low16, &close16, 14, 3).expect("stochastic_fast f16");
    assert_eq!(sto_fast16.k.len(), close16.len());

    let _ =
        stochastic::stochastic_full(&high, &low, &close, 14, 3, 3).expect("stochastic_full f64");
    let _ = stochastic::stochastic_full(&high32, &low32, &close32, 14, 3, 3)
        .expect("stochastic_full f32");
    let _ = stochastic::stochastic_full(&high16, &low16, &close16, 14, 3, 3)
        .expect("stochastic_full f16");
    let _ = stochastic::stochastic(&high, &low, &close, 14, 3, 1)
        .expect("stochastic fast-dispatch f64");
    let _ = stochastic::stochastic(&high32, &low32, &close32, 14, 3, 1)
        .expect("stochastic fast-dispatch f32");
    let _ = stochastic::stochastic(&high16, &low16, &close16, 14, 3, 1)
        .expect("stochastic fast-dispatch f16");
    let _ = stochastic::stochastic(&high, &low, &close, 14, 3, 3)
        .expect("stochastic full-dispatch f64");
    let _ = stochastic::stochastic(&high32, &low32, &close32, 14, 3, 3)
        .expect("stochastic full-dispatch f32");
    let _ = stochastic::stochastic(&high16, &low16, &close16, 14, 3, 3)
        .expect("stochastic full-dispatch f16");

    let mut st_out = stochastic::StochasticOutput {
        k: vec![f64::NAN; close.len()],
        d: vec![f64::NAN; close.len()],
    };
    let _ = stochastic::stochastic_fast_into(&high, &low, &close, 14, 3, &mut st_out)
        .expect("stochastic_fast_into f64");
    let _ = stochastic::stochastic_full_into(&high, &low, &close, 14, 3, 3, &mut st_out)
        .expect("stochastic_full_into f64");
    let mut st_out32 = stochastic::StochasticOutput {
        k: vec![f32::NAN; close32.len()],
        d: vec![f32::NAN; close32.len()],
    };
    let _ = stochastic::stochastic_fast_into(&high32, &low32, &close32, 14, 3, &mut st_out32)
        .expect("stochastic_fast_into f32");
    let _ = stochastic::stochastic_full_into(&high32, &low32, &close32, 14, 3, 3, &mut st_out32)
        .expect("stochastic_full_into f32");
    let mut st_out16 = stochastic::StochasticOutput {
        k: vec![f16::NAN; close16.len()],
        d: vec![f16::NAN; close16.len()],
    };
    let _ = stochastic::stochastic_fast_into(&high16, &low16, &close16, 14, 3, &mut st_out16)
        .expect("stochastic_fast_into f16");
    let _ = stochastic::stochastic_full_into(&high16, &low16, &close16, 14, 3, 3, &mut st_out16)
        .expect("stochastic_full_into f16");
    let mut st_bad_out = stochastic::StochasticOutput {
        k: vec![f64::NAN; close.len() - 1],
        d: vec![f64::NAN; close.len()],
    };
    assert!(stochastic::stochastic_fast_into(&high, &low, &close, 14, 3, &mut st_bad_out).is_err());
    assert!(
        stochastic::stochastic_full_into(&high, &low, &close, 14, 3, 3, &mut st_bad_out).is_err()
    );
    let st_cfg = stochastic::Stochastic::new()
        .with_k_period(14)
        .with_d_period(3)
        .with_k_slowing(3);
    let _ = st_cfg
        .compute(&high, &low, &close)
        .expect("stochastic cfg compute");
    let _ = st_cfg
        .compute_into(&high, &low, &close, &mut st_out)
        .expect("stochastic cfg compute_into");

    let _ = stochrsi::stochrsi(&close, 14, 14, 1, 3).expect("stochrsi f64");
    let _ = stochrsi::stochrsi::<f32>(&close32, 14, 14, 1, 3).expect("stochrsi f32");
    let _ = stochrsi::stochrsi::<f16>(&close16, 14, 14, 1, 3).expect("stochrsi f16");
    let _ = stochrsi::stochrsi_default(&close).expect("stochrsi_default");
    let mut fastk = vec![f64::NAN; close.len()];
    let mut fastd = vec![f64::NAN; close.len()];
    stochrsi::stochrsi_into(&close, 14, 14, 1, 3, &mut fastk, &mut fastd)
        .expect("stochrsi_into f64");
    let mut fastk32 = vec![f32::NAN; close32.len()];
    let mut fastd32 = vec![f32::NAN; close32.len()];
    stochrsi::stochrsi_into(&close32, 14, 14, 1, 3, &mut fastk32, &mut fastd32)
        .expect("stochrsi_into f32");
    let mut fastk16 = vec![f16::NAN; close16.len()];
    let mut fastd16 = vec![f16::NAN; close16.len()];
    stochrsi::stochrsi_into(&close16, 14, 14, 1, 3, &mut fastk16, &mut fastd16)
        .expect("stochrsi_into f16");
}

#[test]
fn coverage_low_outlier_statistics_all_public_alloc_surfaces() {
    let data: Vec<f64> = (0..128)
        .map(|i| 10.0 + (i as f64) * 0.25 + ((i % 5) as f64) * 0.1)
        .collect();
    let data2: Vec<f64> = data
        .iter()
        .enumerate()
        .map(|(i, &v)| v * 1.1 + (i as f64) * 0.05)
        .collect();
    let data16: Vec<f16> = data.iter().map(|&v| f16::from_f32(v as f32)).collect();
    let data16_b: Vec<f16> = data2.iter().map(|&v| f16::from_f32(v as f32)).collect();
    let period = 10usize;

    let _ = statistics::var(&data, period).expect("var f64");
    let _ = statistics::stddev(&data, period).expect("stddev f64");
    let _ = statistics::skew(&data, period).expect("skew f64");
    let _ = statistics::kurt(&data, period).expect("kurt f64");
    let _ = statistics::cov(&data, &data2, period).expect("cov f64");
    let _ = statistics::zscore(&data, period).expect("zscore f64");
    let _ = statistics::mad(&data, period).expect("mad f64");
    let _ = statistics::sem(&data, period).expect("sem f64");
    let _ = statistics::correl(&data, &data2, period).expect("correl f64");
    let _ = statistics::beta(&data, &data2, period).expect("beta f64");
    let _ = statistics::linearreg(&data, period).expect("linearreg f64");
    let _ = statistics::linearreg_slope(&data, period).expect("linearreg_slope f64");
    let _ = statistics::linearreg_intercept(&data, period).expect("linearreg_intercept f64");
    let _ = statistics::linearreg_angle(&data, period).expect("linearreg_angle f64");
    let _ = statistics::tsf(&data, period).expect("tsf f64");

    let _ = statistics::var(&data16, period).expect("var f16");
    let _ = statistics::stddev(&data16, period).expect("stddev f16");
    let _ = statistics::skew(&data16, period).expect("skew f16");
    let _ = statistics::kurt(&data16, period).expect("kurt f16");
    let _ = statistics::cov(&data16, &data16_b, period).expect("cov f16");
    let _ = statistics::zscore(&data16, period).expect("zscore f16");
    let _ = statistics::mad(&data16, period).expect("mad f16");
    let _ = statistics::sem(&data16, period).expect("sem f16");
    let _ = statistics::correl(&data16, &data16_b, period).expect("correl f16");
    let _ = statistics::beta(&data16, &data16_b, period).expect("beta f16");
    let _ = statistics::linearreg(&data16, period).expect("linearreg f16");
    let _ = statistics::linearreg_slope(&data16, period).expect("linearreg_slope f16");
    let _ = statistics::linearreg_intercept(&data16, period).expect("linearreg_intercept f16");
    let _ = statistics::linearreg_angle(&data16, period).expect("linearreg_angle f16");
    let _ = statistics::tsf(&data16, period).expect("tsf f16");
}

#[test]
fn coverage_low_outlier_into_and_error_matrix() {
    let n = 96usize;
    let data: Vec<f64> = (0..n)
        .map(|i| 100.0 + (i as f64) * 0.3 + ((i % 7) as f64) * 0.2)
        .collect();
    let data2: Vec<f64> = data
        .iter()
        .enumerate()
        .map(|(i, &v)| v * 0.8 + (i as f64) * 0.11)
        .collect();
    let period = 14usize;

    let mut out = vec![f64::NAN; n];
    let mut out_small = vec![f64::NAN; n - 1];

    statistics::var_into(&data, period, &mut out).expect("var_into");
    statistics::stddev_into(&data, period, &mut out).expect("stddev_into");
    statistics::skew_into(&data, period, &mut out).expect("skew_into");
    statistics::kurt_into(&data, period, &mut out).expect("kurt_into");
    statistics::zscore_into(&data, period, &mut out).expect("zscore_into");
    statistics::mad_into(&data, period, &mut out).expect("mad_into");
    statistics::sem_into(&data, period, &mut out).expect("sem_into");
    statistics::linearreg_into(&data, period, &mut out).expect("linearreg_into");
    statistics::linearreg_slope_into(&data, period, &mut out).expect("linearreg_slope_into");
    statistics::linearreg_intercept_into(&data, period, &mut out)
        .expect("linearreg_intercept_into");
    statistics::linearreg_angle_into(&data, period, &mut out).expect("linearreg_angle_into");
    statistics::tsf_into(&data, period, &mut out).expect("tsf_into");
    statistics::cov_into(&data, &data2, period, &mut out).expect("cov_into");
    statistics::correl_into(&data, &data2, period, &mut out).expect("correl_into");
    statistics::beta_into(&data, &data2, period, &mut out).expect("beta_into");

    assert!(statistics::var_into(&data, 0, &mut out).is_err());
    assert!(statistics::stddev_into(&data, 0, &mut out).is_err());
    assert!(statistics::skew_into(&data, 0, &mut out).is_err());
    assert!(statistics::kurt_into(&data, 0, &mut out).is_err());
    assert!(statistics::zscore_into(&data, 0, &mut out).is_err());
    assert!(statistics::mad_into(&data, 0, &mut out).is_err());
    assert!(statistics::sem_into(&data, 0, &mut out).is_err());
    assert!(statistics::linearreg_into(&data, 0, &mut out).is_err());
    assert!(statistics::linearreg_slope_into(&data, 0, &mut out).is_err());
    assert!(statistics::linearreg_intercept_into(&data, 0, &mut out).is_err());
    assert!(statistics::linearreg_angle_into(&data, 0, &mut out).is_err());
    assert!(statistics::tsf_into(&data, 0, &mut out).is_err());
    assert!(statistics::cov_into(&data, &data2, 0, &mut out).is_err());
    assert!(statistics::correl_into(&data, &data2, 0, &mut out).is_err());
    assert!(statistics::beta_into(&data, &data2, 0, &mut out).is_err());

    assert!(statistics::var_into(&data, period, &mut out_small).is_err());
    assert!(statistics::cov_into(&data, &data2, period, &mut out_small).is_err());

    let data_short = vec![1.0_f64, 2.0, 3.0];
    let mut out_short = vec![f64::NAN; data_short.len()];
    assert!(statistics::var_into(&data_short, period, &mut out_short).is_err());
    assert!(statistics::cov_into(&data_short, &data_short, period, &mut out_short).is_err());

    let (_, high, low, close) = make_ohlc(n);
    let mut adx_out = vec![f64::NAN; n];
    let mut plus_out = vec![f64::NAN; n];
    let mut minus_out = vec![f64::NAN; n];
    let _ = adx::adx_into(
        &high,
        &low,
        &close,
        period,
        &mut adx_out,
        &mut plus_out,
        &mut minus_out,
    )
    .expect("adx_into");
    assert!(
        adx::adx_into(
            &high,
            &low,
            &close,
            0,
            &mut adx_out,
            &mut plus_out,
            &mut minus_out
        )
        .is_err()
    );
    assert!(
        adx::adx_into(
            &high,
            &low,
            &close,
            period,
            &mut adx_out[..(n - 1)],
            &mut plus_out,
            &mut minus_out
        )
        .is_err()
    );

    let mut dx_out = vec![f64::NAN; n];
    dx::dx_into(&high, &low, &close, period, &mut dx_out).expect("dx_into");
    dx::adxr_into(&high, &low, &close, period, &mut dx_out).expect("adxr_into");
    dx::plus_dm_into(&high, &low, period, &mut dx_out).expect("plus_dm_into");
    dx::minus_dm_into(&high, &low, period, &mut dx_out).expect("minus_dm_into");
    assert!(dx::dx_into(&high, &low, &close, 0, &mut dx_out).is_err());
    assert!(dx::adxr_into(&high, &low, &close, 0, &mut dx_out).is_err());
    assert!(dx::plus_dm_into(&high, &low, 0, &mut dx_out).is_err());
    assert!(dx::minus_dm_into(&high, &low, 0, &mut dx_out).is_err());

    midpoint::midpoint_into(&close, period, &mut dx_out).expect("midpoint_into");
    assert!(midpoint::midpoint_into(&close, 0, &mut dx_out).is_err());
    assert!(midpoint::midpoint_into(&close, period, &mut dx_out[..(n - 1)]).is_err());
}
