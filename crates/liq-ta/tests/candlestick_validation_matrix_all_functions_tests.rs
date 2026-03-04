use liq_ta::indicators::candlestick as cdl;

macro_rules! call_all_non_into {
    ($o:expr, $h:expr, $l:expr, $c:expr, $assert_expr:expr, $($f:ident),+ $(,)?) => {{
        $(
            $assert_expr(cdl::$f($o, $h, $l, $c));
        )+
    }};
}

macro_rules! call_all_into {
    ($o:expr, $h:expr, $l:expr, $c:expr, $out:expr, $assert_expr:expr, $($f:ident),+ $(,)?) => {{
        $(
            $assert_expr(cdl::$f($o, $h, $l, $c, $out));
        )+
    }};
}

fn assert_err_vec(res: liq_ta::error::Result<Vec<i32>>) {
    assert!(res.is_err());
}

fn assert_err_unit(res: liq_ta::error::Result<()>) {
    assert!(res.is_err());
}

fn assert_ok_vec(res: liq_ta::error::Result<Vec<i32>>) {
    assert!(res.is_ok());
}

fn assert_ok_unit(res: liq_ta::error::Result<()>) {
    assert!(res.is_ok());
}

#[test]
fn candlestick_all_functions_empty_and_mismatch_surface() {
    let empty: [f64; 0] = [];
    let open = [10.0, 10.2, 10.1, 10.6, 10.3, 10.9, 10.4, 10.8];
    let high = [10.5, 10.6, 10.4, 10.9, 10.8, 11.1, 10.9, 11.0];
    let low = [9.8, 9.9, 9.7, 10.1, 10.0, 10.3, 10.1, 10.2];
    let close = [10.3, 10.1, 10.2, 10.7, 10.6, 10.4, 10.8, 10.5];
    let open_short = &open[..6];

    call_all_non_into!(
        &empty,
        &empty,
        &empty,
        &empty,
        assert_err_vec,
        cdl_2crows,
        cdl_3black_crows,
        cdl_3inside,
        cdl_3line_strike,
        cdl_3outside,
        cdl_3stars_in_south,
        cdl_3white_soldiers,
        cdl_abandoned_baby,
        cdl_advance_block,
        cdl_belt_hold,
        cdl_breakaway,
        cdl_closing_marubozu,
        cdl_concealing_baby_swallow,
        cdl_counter_attack,
        cdl_dark_cloud_cover,
        cdl_doji,
        cdl_doji_star,
        cdl_dragonfly_doji,
        cdl_engulfing,
        cdl_evening_doji_star,
        cdl_evening_star,
        cdl_gap_side_side_white,
        cdl_gravestone_doji,
        cdl_hammer,
        cdl_hanging_man,
        cdl_harami,
        cdl_harami_cross,
        cdl_high_wave,
        cdl_hikkake,
        cdl_hikkake_mod,
        cdl_homing_pigeon,
        cdl_identical_3crows,
        cdl_in_neck,
        cdl_inverted_hammer,
        cdl_kicking,
        cdl_kicking_by_length,
        cdl_ladder_bottom,
        cdl_long_line,
        cdl_longleg_doji,
        cdl_marubozu,
        cdl_mat_hold,
        cdl_matching_low,
        cdl_morning_doji_star,
        cdl_morning_star,
        cdl_on_neck,
        cdl_piercing,
        cdl_rickshaw_man,
        cdl_rise_fall_3methods,
        cdl_separating_lines,
        cdl_shooting_star,
        cdl_short_line,
        cdl_spinning_top,
        cdl_stalled_pattern,
        cdl_stick_sandwich,
        cdl_takuri,
        cdl_tasuki_gap,
        cdl_thrusting,
        cdl_tristar,
        cdl_unique_3river,
        cdl_upside_gap_2crows,
        cdl_xside_gap_3methods
    );

    call_all_non_into!(
        open_short,
        &high,
        &low,
        &close,
        assert_err_vec,
        cdl_2crows,
        cdl_3black_crows,
        cdl_3inside,
        cdl_3line_strike,
        cdl_3outside,
        cdl_3stars_in_south,
        cdl_3white_soldiers,
        cdl_abandoned_baby,
        cdl_advance_block,
        cdl_belt_hold,
        cdl_breakaway,
        cdl_closing_marubozu,
        cdl_concealing_baby_swallow,
        cdl_counter_attack,
        cdl_dark_cloud_cover,
        cdl_doji,
        cdl_doji_star,
        cdl_dragonfly_doji,
        cdl_engulfing,
        cdl_evening_doji_star,
        cdl_evening_star,
        cdl_gap_side_side_white,
        cdl_gravestone_doji,
        cdl_hammer,
        cdl_hanging_man,
        cdl_harami,
        cdl_harami_cross,
        cdl_high_wave,
        cdl_hikkake,
        cdl_hikkake_mod,
        cdl_homing_pigeon,
        cdl_identical_3crows,
        cdl_in_neck,
        cdl_inverted_hammer,
        cdl_kicking,
        cdl_kicking_by_length,
        cdl_ladder_bottom,
        cdl_long_line,
        cdl_longleg_doji,
        cdl_marubozu,
        cdl_mat_hold,
        cdl_matching_low,
        cdl_morning_doji_star,
        cdl_morning_star,
        cdl_on_neck,
        cdl_piercing,
        cdl_rickshaw_man,
        cdl_rise_fall_3methods,
        cdl_separating_lines,
        cdl_shooting_star,
        cdl_short_line,
        cdl_spinning_top,
        cdl_stalled_pattern,
        cdl_stick_sandwich,
        cdl_takuri,
        cdl_tasuki_gap,
        cdl_thrusting,
        cdl_tristar,
        cdl_unique_3river,
        cdl_upside_gap_2crows,
        cdl_xside_gap_3methods
    );
}

#[test]
fn candlestick_all_functions_into_buffer_and_valid_surface() {
    let n = 120usize;
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    let mut prev = 100.0_f64;
    for i in 0..n {
        let drift = (i as f64) * 0.02;
        let wave = ((i % 9) as f64 - 4.0) * 0.11;
        let o = prev + wave;
        let c = o + if i % 2 == 0 { 0.6 } else { -0.5 } + drift.sin() * 0.1;
        let h = o.max(c) + 0.3 + ((i % 5) as f64) * 0.04;
        let l = o.min(c) - 0.3 - ((i % 7) as f64) * 0.03;
        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);
        prev = c;
    }

    let mut out_small = vec![0_i32; 64];
    let mut out_ok = vec![0_i32; n];

    call_all_into!(
        &open,
        &high,
        &low,
        &close,
        &mut out_small,
        assert_err_unit,
        cdl_2crows_into,
        cdl_3black_crows_into,
        cdl_3inside_into,
        cdl_3line_strike_into,
        cdl_3outside_into,
        cdl_3stars_in_south_into,
        cdl_3white_soldiers_into,
        cdl_abandoned_baby_into,
        cdl_advance_block_into,
        cdl_belt_hold_into,
        cdl_breakaway_into,
        cdl_closing_marubozu_into,
        cdl_concealing_baby_swallow_into,
        cdl_counter_attack_into,
        cdl_dark_cloud_cover_into,
        cdl_doji_into,
        cdl_doji_star_into,
        cdl_dragonfly_doji_into,
        cdl_engulfing_into,
        cdl_evening_doji_star_into,
        cdl_evening_star_into,
        cdl_gap_side_side_white_into,
        cdl_gravestone_doji_into,
        cdl_hammer_into,
        cdl_hanging_man_into,
        cdl_harami_into,
        cdl_harami_cross_into,
        cdl_high_wave_into,
        cdl_hikkake_into,
        cdl_hikkake_mod_into,
        cdl_homing_pigeon_into,
        cdl_identical_3crows_into,
        cdl_in_neck_into,
        cdl_inverted_hammer_into,
        cdl_kicking_into,
        cdl_kicking_by_length_into,
        cdl_ladder_bottom_into,
        cdl_long_line_into,
        cdl_longleg_doji_into,
        cdl_marubozu_into,
        cdl_mat_hold_into,
        cdl_matching_low_into,
        cdl_morning_doji_star_into,
        cdl_morning_star_into,
        cdl_on_neck_into,
        cdl_piercing_into,
        cdl_rickshaw_man_into,
        cdl_rise_fall_3methods_into,
        cdl_separating_lines_into,
        cdl_shooting_star_into,
        cdl_short_line_into,
        cdl_spinning_top_into,
        cdl_stalled_pattern_into,
        cdl_stick_sandwich_into,
        cdl_takuri_into,
        cdl_tasuki_gap_into,
        cdl_thrusting_into,
        cdl_tristar_into,
        cdl_unique_3river_into,
        cdl_upside_gap_2crows_into,
        cdl_xside_gap_3methods_into
    );

    call_all_non_into!(
        &open,
        &high,
        &low,
        &close,
        assert_ok_vec,
        cdl_2crows,
        cdl_3black_crows,
        cdl_3inside,
        cdl_3line_strike,
        cdl_3outside,
        cdl_3stars_in_south,
        cdl_3white_soldiers,
        cdl_abandoned_baby,
        cdl_advance_block,
        cdl_belt_hold,
        cdl_breakaway,
        cdl_closing_marubozu,
        cdl_concealing_baby_swallow,
        cdl_counter_attack,
        cdl_dark_cloud_cover,
        cdl_doji,
        cdl_doji_star,
        cdl_dragonfly_doji,
        cdl_engulfing,
        cdl_evening_doji_star,
        cdl_evening_star,
        cdl_gap_side_side_white,
        cdl_gravestone_doji,
        cdl_hammer,
        cdl_hanging_man,
        cdl_harami,
        cdl_harami_cross,
        cdl_high_wave,
        cdl_hikkake,
        cdl_hikkake_mod,
        cdl_homing_pigeon,
        cdl_identical_3crows,
        cdl_in_neck,
        cdl_inverted_hammer,
        cdl_kicking,
        cdl_kicking_by_length,
        cdl_ladder_bottom,
        cdl_long_line,
        cdl_longleg_doji,
        cdl_marubozu,
        cdl_mat_hold,
        cdl_matching_low,
        cdl_morning_doji_star,
        cdl_morning_star,
        cdl_on_neck,
        cdl_piercing,
        cdl_rickshaw_man,
        cdl_rise_fall_3methods,
        cdl_separating_lines,
        cdl_shooting_star,
        cdl_short_line,
        cdl_spinning_top,
        cdl_stalled_pattern,
        cdl_stick_sandwich,
        cdl_takuri,
        cdl_tasuki_gap,
        cdl_thrusting,
        cdl_tristar,
        cdl_unique_3river,
        cdl_upside_gap_2crows,
        cdl_xside_gap_3methods
    );

    call_all_into!(
        &open,
        &high,
        &low,
        &close,
        &mut out_ok,
        assert_ok_unit,
        cdl_2crows_into,
        cdl_3black_crows_into,
        cdl_3inside_into,
        cdl_3line_strike_into,
        cdl_3outside_into,
        cdl_3stars_in_south_into,
        cdl_3white_soldiers_into,
        cdl_abandoned_baby_into,
        cdl_advance_block_into,
        cdl_belt_hold_into,
        cdl_breakaway_into,
        cdl_closing_marubozu_into,
        cdl_concealing_baby_swallow_into,
        cdl_counter_attack_into,
        cdl_dark_cloud_cover_into,
        cdl_doji_into,
        cdl_doji_star_into,
        cdl_dragonfly_doji_into,
        cdl_engulfing_into,
        cdl_evening_doji_star_into,
        cdl_evening_star_into,
        cdl_gap_side_side_white_into,
        cdl_gravestone_doji_into,
        cdl_hammer_into,
        cdl_hanging_man_into,
        cdl_harami_into,
        cdl_harami_cross_into,
        cdl_high_wave_into,
        cdl_hikkake_into,
        cdl_hikkake_mod_into,
        cdl_homing_pigeon_into,
        cdl_identical_3crows_into,
        cdl_in_neck_into,
        cdl_inverted_hammer_into,
        cdl_kicking_into,
        cdl_kicking_by_length_into,
        cdl_ladder_bottom_into,
        cdl_long_line_into,
        cdl_longleg_doji_into,
        cdl_marubozu_into,
        cdl_mat_hold_into,
        cdl_matching_low_into,
        cdl_morning_doji_star_into,
        cdl_morning_star_into,
        cdl_on_neck_into,
        cdl_piercing_into,
        cdl_rickshaw_man_into,
        cdl_rise_fall_3methods_into,
        cdl_separating_lines_into,
        cdl_shooting_star_into,
        cdl_short_line_into,
        cdl_spinning_top_into,
        cdl_stalled_pattern_into,
        cdl_stick_sandwich_into,
        cdl_takuri_into,
        cdl_tasuki_gap_into,
        cdl_thrusting_into,
        cdl_tristar_into,
        cdl_unique_3river_into,
        cdl_upside_gap_2crows_into,
        cdl_xside_gap_3methods_into
    );
}
