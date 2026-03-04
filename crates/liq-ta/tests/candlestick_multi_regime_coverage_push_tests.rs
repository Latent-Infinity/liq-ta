use liq_ta::indicators::candlestick::*;

fn make_uptrend_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);

    let mut p = 100.0_f64;
    for i in 0..n {
        p += 0.8 + (i as f64 * 0.07).sin() * 0.15;
        let o = p - 0.55;
        let c = p + 0.55;
        let h = c + 0.65 + ((i % 7) as f64) * 0.03;
        let l = o - 0.60 - ((i % 5) as f64) * 0.02;
        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);
    }

    (open, high, low, close)
}

fn make_downtrend_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);

    let mut p = 200.0_f64;
    for i in 0..n {
        p -= 0.75 + (i as f64 * 0.09).cos() * 0.14;
        let o = p + 0.50;
        let c = p - 0.50;
        let h = o + 0.70 + ((i % 6) as f64) * 0.02;
        let l = c - 0.75 - ((i % 4) as f64) * 0.03;
        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);
    }

    (open, high, low, close)
}

fn make_gap_volatile_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);

    let mut prev_close = 120.0_f64;
    for i in 0..n {
        let gap = if i % 2 == 0 { 1.4 } else { -1.35 };
        let o = prev_close + gap;
        let c = o + if i % 3 == 0 { 0.95 } else { -0.85 };
        let h = o.max(c) + 1.20 + ((i % 5) as f64) * 0.08;
        let l = o.min(c) - 1.15 - ((i % 4) as f64) * 0.06;
        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);
        prev_close = c;
    }

    (open, high, low, close)
}

fn make_doji_heavy_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);

    let mut p = 150.0_f64;
    for i in 0..n {
        p += (i as f64 * 0.12).sin() * 0.3;
        let o = p + if i % 2 == 0 { 0.015 } else { -0.015 };
        let c = p + if i % 2 == 0 { -0.012 } else { 0.012 };
        let h = o.max(c) + 1.30 + ((i % 3) as f64) * 0.04;
        let l = o.min(c) - 1.25 - ((i % 5) as f64) * 0.05;
        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);
    }

    (open, high, low, close)
}

macro_rules! smoke_pattern {
    ($name:literal, $f:ident, $into_f:ident, $o:expr, $h:expr, $l:expr, $c:expr) => {{
        let alloc = $f($o, $h, $l, $c).expect($name);
        let mut into = vec![0_i32; $o.len()];
        $into_f($o, $h, $l, $c, &mut into).expect($name);
        assert_eq!(alloc.len(), $o.len(), "{} len mismatch", $name);
        assert_eq!(alloc, into, "{} alloc/into mismatch", $name);
    }};
}

fn run_all_patterns(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) {
    smoke_pattern!(
        "single/doji",
        cdl_doji,
        cdl_doji_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/dragonfly_doji",
        cdl_dragonfly_doji,
        cdl_dragonfly_doji_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/gravestone_doji",
        cdl_gravestone_doji,
        cdl_gravestone_doji_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/longleg_doji",
        cdl_longleg_doji,
        cdl_longleg_doji_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/rickshaw",
        cdl_rickshaw_man,
        cdl_rickshaw_man_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/marubozu",
        cdl_marubozu,
        cdl_marubozu_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/closing_marubozu",
        cdl_closing_marubozu,
        cdl_closing_marubozu_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/spinning_top",
        cdl_spinning_top,
        cdl_spinning_top_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/high_wave",
        cdl_high_wave,
        cdl_high_wave_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/long_line",
        cdl_long_line,
        cdl_long_line_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/short_line",
        cdl_short_line,
        cdl_short_line_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/hammer",
        cdl_hammer,
        cdl_hammer_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/hanging_man",
        cdl_hanging_man,
        cdl_hanging_man_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/inverted_hammer",
        cdl_inverted_hammer,
        cdl_inverted_hammer_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/shooting_star",
        cdl_shooting_star,
        cdl_shooting_star_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/belt_hold",
        cdl_belt_hold,
        cdl_belt_hold_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "single/takuri",
        cdl_takuri,
        cdl_takuri_into,
        open,
        high,
        low,
        close
    );

    smoke_pattern!(
        "two/engulfing",
        cdl_engulfing,
        cdl_engulfing_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/harami",
        cdl_harami,
        cdl_harami_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/harami_cross",
        cdl_harami_cross,
        cdl_harami_cross_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/piercing",
        cdl_piercing,
        cdl_piercing_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/dark_cloud_cover",
        cdl_dark_cloud_cover,
        cdl_dark_cloud_cover_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/doji_star",
        cdl_doji_star,
        cdl_doji_star_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/kicking",
        cdl_kicking,
        cdl_kicking_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/kicking_by_length",
        cdl_kicking_by_length,
        cdl_kicking_by_length_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/matching_low",
        cdl_matching_low,
        cdl_matching_low_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/homing_pigeon",
        cdl_homing_pigeon,
        cdl_homing_pigeon_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/in_neck",
        cdl_in_neck,
        cdl_in_neck_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/on_neck",
        cdl_on_neck,
        cdl_on_neck_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/thrusting",
        cdl_thrusting,
        cdl_thrusting_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/separating_lines",
        cdl_separating_lines,
        cdl_separating_lines_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/counter_attack",
        cdl_counter_attack,
        cdl_counter_attack_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/2crows",
        cdl_2crows,
        cdl_2crows_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/hikkake",
        cdl_hikkake,
        cdl_hikkake_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "two/hikkake_mod",
        cdl_hikkake_mod,
        cdl_hikkake_mod_into,
        open,
        high,
        low,
        close
    );

    smoke_pattern!(
        "three/morning_star",
        cdl_morning_star,
        cdl_morning_star_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/evening_star",
        cdl_evening_star,
        cdl_evening_star_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/morning_doji_star",
        cdl_morning_doji_star,
        cdl_morning_doji_star_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/evening_doji_star",
        cdl_evening_doji_star,
        cdl_evening_doji_star_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/abandoned_baby",
        cdl_abandoned_baby,
        cdl_abandoned_baby_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/3white_soldiers",
        cdl_3white_soldiers,
        cdl_3white_soldiers_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/3black_crows",
        cdl_3black_crows,
        cdl_3black_crows_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/3inside",
        cdl_3inside,
        cdl_3inside_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/3outside",
        cdl_3outside,
        cdl_3outside_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/3line_strike",
        cdl_3line_strike,
        cdl_3line_strike_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/3stars_in_south",
        cdl_3stars_in_south,
        cdl_3stars_in_south_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/tristar",
        cdl_tristar,
        cdl_tristar_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/identical_3crows",
        cdl_identical_3crows,
        cdl_identical_3crows_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/advance_block",
        cdl_advance_block,
        cdl_advance_block_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/breakaway",
        cdl_breakaway,
        cdl_breakaway_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/concealing_baby_swallow",
        cdl_concealing_baby_swallow,
        cdl_concealing_baby_swallow_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/gap_side_side_white",
        cdl_gap_side_side_white,
        cdl_gap_side_side_white_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/ladder_bottom",
        cdl_ladder_bottom,
        cdl_ladder_bottom_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/mat_hold",
        cdl_mat_hold,
        cdl_mat_hold_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/rise_fall_3methods",
        cdl_rise_fall_3methods,
        cdl_rise_fall_3methods_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/stalled_pattern",
        cdl_stalled_pattern,
        cdl_stalled_pattern_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/stick_sandwich",
        cdl_stick_sandwich,
        cdl_stick_sandwich_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/tasuki_gap",
        cdl_tasuki_gap,
        cdl_tasuki_gap_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/unique_3river",
        cdl_unique_3river,
        cdl_unique_3river_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/upside_gap_2crows",
        cdl_upside_gap_2crows,
        cdl_upside_gap_2crows_into,
        open,
        high,
        low,
        close
    );
    smoke_pattern!(
        "three/xside_gap_3methods",
        cdl_xside_gap_3methods,
        cdl_xside_gap_3methods_into,
        open,
        high,
        low,
        close
    );
}

#[test]
fn candlestick_multi_regime_alloc_into_surface() {
    let regimes = vec![
        make_uptrend_ohlc(96),
        make_downtrend_ohlc(96),
        make_gap_volatile_ohlc(96),
        make_doji_heavy_ohlc(96),
    ];

    for (open, high, low, close) in regimes {
        run_all_patterns(&open, &high, &low, &close);
    }
}
