use liq_ta::Result;
use liq_ta::indicators::candlestick::*;
use liq_ta::indicators::candlestick::{single, three_candle, two_candle};

type PatternFn = fn(&[f64], &[f64], &[f64], &[f64]) -> Result<Vec<i32>>;
type PatternIntoFn = fn(&[f64], &[f64], &[f64], &[f64], &mut [i32]) -> Result<()>;

fn make_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);

    let mut p = 100.0_f64;
    for i in 0..n {
        let drift = if i % 2 == 0 { 0.8 } else { -0.4 };
        p += drift + (i as f64 * 0.13).sin() * 0.5;
        let o = p - 0.2;
        let c = p + 0.25;
        let h = o.max(c) + 0.9 + ((i % 7) as f64) * 0.04;
        let l = o.min(c) - 0.85 - ((i % 5) as f64) * 0.03;
        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);
    }

    (open, high, low, close)
}

fn candlestick_cases() -> Vec<(&'static str, PatternFn, PatternIntoFn)> {
    vec![
        ("cdl_doji", cdl_doji::<f64>, cdl_doji_into::<f64>),
        (
            "cdl_dragonfly_doji",
            cdl_dragonfly_doji::<f64>,
            cdl_dragonfly_doji_into::<f64>,
        ),
        (
            "cdl_gravestone_doji",
            cdl_gravestone_doji::<f64>,
            cdl_gravestone_doji_into::<f64>,
        ),
        (
            "cdl_longleg_doji",
            cdl_longleg_doji::<f64>,
            cdl_longleg_doji_into::<f64>,
        ),
        (
            "cdl_rickshaw_man",
            cdl_rickshaw_man::<f64>,
            cdl_rickshaw_man_into::<f64>,
        ),
        (
            "cdl_marubozu",
            cdl_marubozu::<f64>,
            cdl_marubozu_into::<f64>,
        ),
        (
            "cdl_closing_marubozu",
            cdl_closing_marubozu::<f64>,
            cdl_closing_marubozu_into::<f64>,
        ),
        (
            "cdl_spinning_top",
            cdl_spinning_top::<f64>,
            cdl_spinning_top_into::<f64>,
        ),
        (
            "cdl_high_wave",
            cdl_high_wave::<f64>,
            cdl_high_wave_into::<f64>,
        ),
        (
            "cdl_long_line",
            cdl_long_line::<f64>,
            cdl_long_line_into::<f64>,
        ),
        (
            "cdl_short_line",
            cdl_short_line::<f64>,
            cdl_short_line_into::<f64>,
        ),
        ("cdl_hammer", cdl_hammer::<f64>, cdl_hammer_into::<f64>),
        (
            "cdl_hanging_man",
            cdl_hanging_man::<f64>,
            cdl_hanging_man_into::<f64>,
        ),
        (
            "cdl_inverted_hammer",
            cdl_inverted_hammer::<f64>,
            cdl_inverted_hammer_into::<f64>,
        ),
        (
            "cdl_shooting_star",
            cdl_shooting_star::<f64>,
            cdl_shooting_star_into::<f64>,
        ),
        ("cdl_takuri", cdl_takuri::<f64>, cdl_takuri_into::<f64>),
        (
            "cdl_belt_hold",
            cdl_belt_hold::<f64>,
            cdl_belt_hold_into::<f64>,
        ),
        (
            "cdl_engulfing",
            cdl_engulfing::<f64>,
            cdl_engulfing_into::<f64>,
        ),
        ("cdl_harami", cdl_harami::<f64>, cdl_harami_into::<f64>),
        (
            "cdl_harami_cross",
            cdl_harami_cross::<f64>,
            cdl_harami_cross_into::<f64>,
        ),
        (
            "cdl_piercing",
            cdl_piercing::<f64>,
            cdl_piercing_into::<f64>,
        ),
        (
            "cdl_dark_cloud_cover",
            cdl_dark_cloud_cover::<f64>,
            cdl_dark_cloud_cover_into::<f64>,
        ),
        (
            "cdl_doji_star",
            cdl_doji_star::<f64>,
            cdl_doji_star_into::<f64>,
        ),
        ("cdl_kicking", cdl_kicking::<f64>, cdl_kicking_into::<f64>),
        (
            "cdl_kicking_by_length",
            cdl_kicking_by_length::<f64>,
            cdl_kicking_by_length_into::<f64>,
        ),
        (
            "cdl_matching_low",
            cdl_matching_low::<f64>,
            cdl_matching_low_into::<f64>,
        ),
        (
            "cdl_homing_pigeon",
            cdl_homing_pigeon::<f64>,
            cdl_homing_pigeon_into::<f64>,
        ),
        ("cdl_in_neck", cdl_in_neck::<f64>, cdl_in_neck_into::<f64>),
        ("cdl_on_neck", cdl_on_neck::<f64>, cdl_on_neck_into::<f64>),
        (
            "cdl_thrusting",
            cdl_thrusting::<f64>,
            cdl_thrusting_into::<f64>,
        ),
        (
            "cdl_separating_lines",
            cdl_separating_lines::<f64>,
            cdl_separating_lines_into::<f64>,
        ),
        (
            "cdl_counter_attack",
            cdl_counter_attack::<f64>,
            cdl_counter_attack_into::<f64>,
        ),
        ("cdl_2crows", cdl_2crows::<f64>, cdl_2crows_into::<f64>),
        ("cdl_hikkake", cdl_hikkake::<f64>, cdl_hikkake_into::<f64>),
        (
            "cdl_hikkake_mod",
            cdl_hikkake_mod::<f64>,
            cdl_hikkake_mod_into::<f64>,
        ),
        (
            "cdl_morning_star",
            cdl_morning_star::<f64>,
            cdl_morning_star_into::<f64>,
        ),
        (
            "cdl_evening_star",
            cdl_evening_star::<f64>,
            cdl_evening_star_into::<f64>,
        ),
        (
            "cdl_morning_doji_star",
            cdl_morning_doji_star::<f64>,
            cdl_morning_doji_star_into::<f64>,
        ),
        (
            "cdl_evening_doji_star",
            cdl_evening_doji_star::<f64>,
            cdl_evening_doji_star_into::<f64>,
        ),
        (
            "cdl_abandoned_baby",
            cdl_abandoned_baby::<f64>,
            cdl_abandoned_baby_into::<f64>,
        ),
        (
            "cdl_3white_soldiers",
            cdl_3white_soldiers::<f64>,
            cdl_3white_soldiers_into::<f64>,
        ),
        (
            "cdl_3black_crows",
            cdl_3black_crows::<f64>,
            cdl_3black_crows_into::<f64>,
        ),
        ("cdl_3inside", cdl_3inside::<f64>, cdl_3inside_into::<f64>),
        (
            "cdl_3outside",
            cdl_3outside::<f64>,
            cdl_3outside_into::<f64>,
        ),
        (
            "cdl_3line_strike",
            cdl_3line_strike::<f64>,
            cdl_3line_strike_into::<f64>,
        ),
        (
            "cdl_3stars_in_south",
            cdl_3stars_in_south::<f64>,
            cdl_3stars_in_south_into::<f64>,
        ),
        ("cdl_tristar", cdl_tristar::<f64>, cdl_tristar_into::<f64>),
        (
            "cdl_identical_3crows",
            cdl_identical_3crows::<f64>,
            cdl_identical_3crows_into::<f64>,
        ),
    ]
}

#[test]
fn candlestick_validation_error_matrix() {
    let (open, high, low, close) = make_ohlc(96);
    let empty: &[f64] = &[];

    for (name, alloc_fn, into_fn) in candlestick_cases() {
        assert!(
            alloc_fn(&open[..95], &high, &low, &close).is_err(),
            "{name}: alloc length mismatch should error"
        );
        assert!(
            alloc_fn(empty, empty, empty, empty).is_err(),
            "{name}: alloc empty input should error"
        );

        let mut short_out = vec![0_i32; 95];
        assert!(
            into_fn(&open, &high, &low, &close, &mut short_out).is_err(),
            "{name}: into short output should error"
        );
    }
}

#[test]
fn candlestick_min_len_surface() {
    assert!(single::cdl_doji_min_len() > 0);
    assert!(single::cdl_dragonfly_doji_min_len() > 0);
    assert!(single::cdl_gravestone_doji_min_len() > 0);
    assert!(single::cdl_longleg_doji_min_len() > 0);
    assert!(single::cdl_rickshaw_man_min_len() > 0);
    assert!(single::cdl_marubozu_min_len() > 0);
    assert!(single::cdl_closing_marubozu_min_len() > 0);
    assert!(single::cdl_spinning_top_min_len() > 0);
    assert!(single::cdl_high_wave_min_len() > 0);
    assert!(single::cdl_long_line_min_len() > 0);
    assert!(single::cdl_short_line_min_len() > 0);
    assert!(single::cdl_hammer_min_len() > 0);
    assert!(single::cdl_hanging_man_min_len() > 0);
    assert!(single::cdl_inverted_hammer_min_len() > 0);
    assert!(single::cdl_shooting_star_min_len() > 0);
    assert!(single::cdl_takuri_min_len() > 0);
    assert!(single::cdl_belt_hold_min_len() > 0);

    assert!(two_candle::cdl_engulfing_min_len() > 0);
    assert!(two_candle::cdl_harami_min_len() > 0);
    assert!(two_candle::cdl_harami_cross_min_len() > 0);
    assert!(two_candle::cdl_piercing_min_len() > 0);
    assert!(two_candle::cdl_dark_cloud_cover_min_len() > 0);
    assert!(two_candle::cdl_doji_star_min_len() > 0);
    assert!(two_candle::cdl_kicking_min_len() > 0);
    assert!(two_candle::cdl_kicking_by_length_min_len() > 0);
    assert!(two_candle::cdl_matching_low_min_len() > 0);
    assert!(two_candle::cdl_homing_pigeon_min_len() > 0);
    assert!(two_candle::cdl_in_neck_min_len() > 0);
    assert!(two_candle::cdl_on_neck_min_len() > 0);
    assert!(two_candle::cdl_thrusting_min_len() > 0);
    assert!(two_candle::cdl_separating_lines_min_len() > 0);
    assert!(two_candle::cdl_counter_attack_min_len() > 0);
    assert!(two_candle::cdl_2crows_min_len() > 0);
    assert!(two_candle::cdl_hikkake_min_len() > 0);
    assert!(two_candle::cdl_hikkake_mod_min_len() > 0);

    assert!(three_candle::cdl_morning_star_min_len() > 0);
    assert!(three_candle::cdl_evening_star_min_len() > 0);
    assert!(three_candle::cdl_morning_doji_star_min_len() > 0);
    assert!(three_candle::cdl_evening_doji_star_min_len() > 0);
    assert!(three_candle::cdl_abandoned_baby_min_len() > 0);
    assert!(three_candle::cdl_3white_soldiers_min_len() > 0);
    assert!(three_candle::cdl_3black_crows_min_len() > 0);
    assert!(three_candle::cdl_3inside_min_len() > 0);
    assert!(three_candle::cdl_3outside_min_len() > 0);
    assert!(three_candle::cdl_3line_strike_min_len() > 0);
    assert!(three_candle::cdl_3stars_in_south_min_len() > 0);
    assert!(three_candle::cdl_tristar_min_len() > 0);
    assert!(three_candle::cdl_identical_3crows_min_len() > 0);
}
