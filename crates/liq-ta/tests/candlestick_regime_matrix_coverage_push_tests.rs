use liq_ta::indicators::candlestick as cdl;

type OhlcSeriesF64 = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>);

fn make_regimes_f64(n: usize) -> Vec<OhlcSeriesF64> {
    let mut up_o = Vec::with_capacity(n);
    let mut up_h = Vec::with_capacity(n);
    let mut up_l = Vec::with_capacity(n);
    let mut up_c = Vec::with_capacity(n);

    let mut down_o = Vec::with_capacity(n);
    let mut down_h = Vec::with_capacity(n);
    let mut down_l = Vec::with_capacity(n);
    let mut down_c = Vec::with_capacity(n);

    let mut chop_o = Vec::with_capacity(n);
    let mut chop_h = Vec::with_capacity(n);
    let mut chop_l = Vec::with_capacity(n);
    let mut chop_c = Vec::with_capacity(n);

    let mut gap_o = Vec::with_capacity(n);
    let mut gap_h = Vec::with_capacity(n);
    let mut gap_l = Vec::with_capacity(n);
    let mut gap_c = Vec::with_capacity(n);

    let mut prev_gap_close = 100.0_f64;

    for i in 0..n {
        let i_f = i as f64;

        let up_open = 50.0 + i_f * 0.28 + if i % 2 == 0 { 0.03 } else { -0.02 };
        let up_close = up_open + if i % 3 == 0 { 0.22 } else { 0.11 };
        let up_high = up_open.max(up_close) + 0.19;
        let up_low = up_open.min(up_close) - 0.17;
        up_o.push(up_open);
        up_h.push(up_high);
        up_l.push(up_low);
        up_c.push(up_close);

        let down_open = 160.0 - i_f * 0.31 + if i % 2 == 0 { 0.04 } else { -0.03 };
        let down_close = down_open - if i % 3 == 1 { 0.24 } else { 0.12 };
        let down_high = down_open.max(down_close) + 0.21;
        let down_low = down_open.min(down_close) - 0.20;
        down_o.push(down_open);
        down_h.push(down_high);
        down_l.push(down_low);
        down_c.push(down_close);

        let base = 90.0 + (i_f * 0.17).sin() * 1.3;
        let chop_open = base + if i % 2 == 0 { 0.04 } else { -0.04 };
        let chop_close = base + if i % 4 == 0 { 0.01 } else { -0.01 };
        let chop_high = chop_open.max(chop_close) + 0.35 + ((i % 5) as f64) * 0.03;
        let chop_low = chop_open.min(chop_close) - 0.33 - ((i % 3) as f64) * 0.02;
        chop_o.push(chop_open);
        chop_h.push(chop_high);
        chop_l.push(chop_low);
        chop_c.push(chop_close);

        let gap_open = if i % 7 == 0 {
            prev_gap_close + 1.9
        } else if i % 11 == 0 {
            prev_gap_close - 2.1
        } else {
            prev_gap_close + if i % 2 == 0 { 0.23 } else { -0.19 }
        };
        let gap_close = gap_open + if i % 3 == 0 { 0.61 } else { -0.57 };
        let gap_high = gap_open.max(gap_close) + 0.41;
        let gap_low = gap_open.min(gap_close) - 0.43;
        gap_o.push(gap_open);
        gap_h.push(gap_high);
        gap_l.push(gap_low);
        gap_c.push(gap_close);
        prev_gap_close = gap_close;
    }

    vec![
        (up_o, up_h, up_l, up_c),
        (down_o, down_h, down_l, down_c),
        (chop_o, chop_h, chop_l, chop_c),
        (gap_o, gap_h, gap_l, gap_c),
    ]
}

macro_rules! assert_non_into_ok {
    ($o:expr, $h:expr, $l:expr, $c:expr, $n:expr, $($f:ident),+ $(,)?) => {{
        $(
            let out = cdl::$f($o, $h, $l, $c).expect(concat!(stringify!($f), " should succeed"));
            assert_eq!(out.len(), $n, concat!(stringify!($f), " output length"));
        )+
    }};
}

macro_rules! assert_into_ok {
    ($o:expr, $h:expr, $l:expr, $c:expr, $n:expr, $($f:ident),+ $(,)?) => {{
        $(
            let mut out = vec![0_i32; $n];
            cdl::$f($o, $h, $l, $c, &mut out).expect(concat!(stringify!($f), " into should succeed"));
            assert_eq!(out.len(), $n, concat!(stringify!($f), " into output length"));
        )+
    }};
}

#[test]
fn candlestick_regime_matrix_non_into_and_into_f64() {
    for (open, high, low, close) in make_regimes_f64(128) {
        let n = open.len();
        assert_non_into_ok!(
            &open,
            &high,
            &low,
            &close,
            n,
            cdl_doji,
            cdl_dragonfly_doji,
            cdl_gravestone_doji,
            cdl_longleg_doji,
            cdl_rickshaw_man,
            cdl_marubozu,
            cdl_closing_marubozu,
            cdl_spinning_top,
            cdl_high_wave,
            cdl_long_line,
            cdl_short_line,
            cdl_hammer,
            cdl_hanging_man,
            cdl_inverted_hammer,
            cdl_shooting_star,
            cdl_takuri,
            cdl_belt_hold,
            cdl_engulfing,
            cdl_harami,
            cdl_harami_cross,
            cdl_piercing,
            cdl_dark_cloud_cover,
            cdl_doji_star,
            cdl_kicking,
            cdl_kicking_by_length,
            cdl_matching_low,
            cdl_homing_pigeon,
            cdl_in_neck,
            cdl_on_neck,
            cdl_thrusting,
            cdl_separating_lines,
            cdl_counter_attack,
            cdl_2crows,
            cdl_hikkake,
            cdl_hikkake_mod,
            cdl_morning_star,
            cdl_evening_star,
            cdl_morning_doji_star,
            cdl_evening_doji_star,
            cdl_abandoned_baby,
            cdl_3white_soldiers,
            cdl_3black_crows,
            cdl_3inside,
            cdl_3outside,
            cdl_3line_strike,
            cdl_3stars_in_south,
            cdl_tristar,
            cdl_identical_3crows
        );

        assert_into_ok!(
            &open,
            &high,
            &low,
            &close,
            n,
            cdl_doji_into,
            cdl_dragonfly_doji_into,
            cdl_gravestone_doji_into,
            cdl_longleg_doji_into,
            cdl_rickshaw_man_into,
            cdl_marubozu_into,
            cdl_closing_marubozu_into,
            cdl_spinning_top_into,
            cdl_high_wave_into,
            cdl_long_line_into,
            cdl_short_line_into,
            cdl_hammer_into,
            cdl_hanging_man_into,
            cdl_inverted_hammer_into,
            cdl_shooting_star_into,
            cdl_takuri_into,
            cdl_belt_hold_into,
            cdl_engulfing_into,
            cdl_harami_into,
            cdl_harami_cross_into,
            cdl_piercing_into,
            cdl_dark_cloud_cover_into,
            cdl_doji_star_into,
            cdl_kicking_into,
            cdl_kicking_by_length_into,
            cdl_matching_low_into,
            cdl_homing_pigeon_into,
            cdl_in_neck_into,
            cdl_on_neck_into,
            cdl_thrusting_into,
            cdl_separating_lines_into,
            cdl_counter_attack_into,
            cdl_2crows_into,
            cdl_hikkake_into,
            cdl_hikkake_mod_into,
            cdl_morning_star_into,
            cdl_evening_star_into,
            cdl_morning_doji_star_into,
            cdl_evening_doji_star_into,
            cdl_abandoned_baby_into,
            cdl_3white_soldiers_into,
            cdl_3black_crows_into,
            cdl_3inside_into,
            cdl_3outside_into,
            cdl_3line_strike_into,
            cdl_3stars_in_south_into,
            cdl_tristar_into,
            cdl_identical_3crows_into
        );
    }
}

#[test]
fn candlestick_regime_matrix_f32_and_buffer_errors() {
    for (open64, high64, low64, close64) in make_regimes_f64(96) {
        let open: Vec<f32> = open64.iter().map(|&v| v as f32).collect();
        let high: Vec<f32> = high64.iter().map(|&v| v as f32).collect();
        let low: Vec<f32> = low64.iter().map(|&v| v as f32).collect();
        let close: Vec<f32> = close64.iter().map(|&v| v as f32).collect();

        let n = open.len();
        assert_non_into_ok!(
            &open,
            &high,
            &low,
            &close,
            n,
            cdl_doji,
            cdl_dragonfly_doji,
            cdl_gravestone_doji,
            cdl_longleg_doji,
            cdl_rickshaw_man,
            cdl_marubozu,
            cdl_closing_marubozu,
            cdl_spinning_top,
            cdl_high_wave,
            cdl_long_line,
            cdl_short_line,
            cdl_hammer,
            cdl_hanging_man,
            cdl_inverted_hammer,
            cdl_shooting_star,
            cdl_takuri,
            cdl_belt_hold,
            cdl_engulfing,
            cdl_harami,
            cdl_harami_cross,
            cdl_piercing,
            cdl_dark_cloud_cover,
            cdl_doji_star,
            cdl_kicking,
            cdl_kicking_by_length,
            cdl_matching_low,
            cdl_homing_pigeon,
            cdl_in_neck,
            cdl_on_neck,
            cdl_thrusting,
            cdl_separating_lines,
            cdl_counter_attack,
            cdl_2crows,
            cdl_hikkake,
            cdl_hikkake_mod,
            cdl_morning_star,
            cdl_evening_star,
            cdl_morning_doji_star,
            cdl_evening_doji_star,
            cdl_abandoned_baby,
            cdl_3white_soldiers,
            cdl_3black_crows,
            cdl_3inside,
            cdl_3outside,
            cdl_3line_strike,
            cdl_3stars_in_south,
            cdl_tristar,
            cdl_identical_3crows
        );

        assert_into_ok!(
            &open,
            &high,
            &low,
            &close,
            n,
            cdl_doji_into,
            cdl_dragonfly_doji_into,
            cdl_gravestone_doji_into,
            cdl_longleg_doji_into,
            cdl_rickshaw_man_into,
            cdl_marubozu_into,
            cdl_closing_marubozu_into,
            cdl_spinning_top_into,
            cdl_high_wave_into,
            cdl_long_line_into,
            cdl_short_line_into,
            cdl_hammer_into,
            cdl_hanging_man_into,
            cdl_inverted_hammer_into,
            cdl_shooting_star_into,
            cdl_takuri_into,
            cdl_belt_hold_into,
            cdl_engulfing_into,
            cdl_harami_into,
            cdl_harami_cross_into,
            cdl_piercing_into,
            cdl_dark_cloud_cover_into,
            cdl_doji_star_into,
            cdl_kicking_into,
            cdl_kicking_by_length_into,
            cdl_matching_low_into,
            cdl_homing_pigeon_into,
            cdl_in_neck_into,
            cdl_on_neck_into,
            cdl_thrusting_into,
            cdl_separating_lines_into,
            cdl_counter_attack_into,
            cdl_2crows_into,
            cdl_hikkake_into,
            cdl_hikkake_mod_into,
            cdl_morning_star_into,
            cdl_evening_star_into,
            cdl_morning_doji_star_into,
            cdl_evening_doji_star_into,
            cdl_abandoned_baby_into,
            cdl_3white_soldiers_into,
            cdl_3black_crows_into,
            cdl_3inside_into,
            cdl_3outside_into,
            cdl_3line_strike_into,
            cdl_3stars_in_south_into,
            cdl_tristar_into,
            cdl_identical_3crows_into
        );

        let mut short = vec![0_i32; n - 1];
        assert!(cdl::cdl_doji_into(&open, &high, &low, &close, &mut short).is_err());
        assert!(cdl::cdl_engulfing_into(&open, &high, &low, &close, &mut short).is_err());
        assert!(cdl::cdl_morning_star_into(&open, &high, &low, &close, &mut short).is_err());
    }
}
