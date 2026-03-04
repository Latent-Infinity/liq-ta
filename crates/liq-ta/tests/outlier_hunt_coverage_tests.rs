use liq_ta::indicators::bollinger::{BollingerOutput, bollinger, bollinger_into};
use liq_ta::indicators::candlestick as cdl;
use liq_ta::indicators::gaussian_channel::{gaussian_channel, gaussian_channel_into};
use liq_ta::indicators::gaussian_filter::{gaussian_filter, gaussian_filter_into};
use liq_ta::indicators::kama::{kama, kama_full, kama_full_into, kama_into};
use liq_ta::indicators::stochrsi::{stochrsi, stochrsi_into};
use liq_ta::indicators::trix::{trix, trix_into};

fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

fn make_ohlc_seeded(seed: u64, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);

    let mut prev_close = 100.0_f64;
    for i in 0..n {
        let r1 = (lcg_next(&mut state) >> 11) as f64 / ((1_u64 << 53) as f64);
        let r2 = (lcg_next(&mut state) >> 11) as f64 / ((1_u64 << 53) as f64);
        let r3 = (lcg_next(&mut state) >> 11) as f64 / ((1_u64 << 53) as f64);

        let gap = if i % 31 == 0 {
            2.0
        } else if i % 29 == 0 {
            -2.0
        } else {
            (r1 - 0.5) * 0.9
        };
        let mut o = prev_close + gap;
        let mut c = o + (r2 - 0.5) * 2.8;
        let mut h = o.max(c) + 0.2 + r3 * 1.6;
        let mut l = o.min(c) - 0.2 - (1.0 - r3) * 1.5;

        if i % 23 == 0 {
            c = o + (r2 - 0.5) * 0.04;
            h = o.max(c) + 1.8;
            l = o.min(c) - 1.9;
        }
        if i % 37 == 0 {
            c = o + if i % 2 == 0 { 2.4 } else { -2.4 };
            h = o.max(c) + 0.05;
            l = o.min(c) - 0.05;
        }
        if i % 41 == 0 {
            o -= 1.7;
            c += 1.9;
            h = o.max(c) + 0.15;
            l = o.min(c) - 0.15;
        }

        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);
        prev_close = c;
    }

    (open, high, low, close)
}

fn mirror_ohlc(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut mo = Vec::with_capacity(open.len());
    let mut mh = Vec::with_capacity(high.len());
    let mut ml = Vec::with_capacity(low.len());
    let mut mc = Vec::with_capacity(close.len());

    for i in 0..open.len() {
        let o = -open[i];
        let c = -close[i];
        let h = -low[i];
        let l = -high[i];
        mo.push(o);
        mh.push(h.max(o.max(c)));
        ml.push(l.min(o.min(c)));
        mc.push(c);
    }

    (mo, mh, ml, mc)
}

fn reverse_ohlc(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut o = open.to_vec();
    let mut h = high.to_vec();
    let mut l = low.to_vec();
    let mut c = close.to_vec();
    o.reverse();
    h.reverse();
    l.reverse();
    c.reverse();
    (o, h, l, c)
}

macro_rules! call_non_into {
    ($o:expr, $h:expr, $l:expr, $c:expr, $sum:expr, $($f:ident),+ $(,)?) => {{
        $(
            let out = cdl::$f($o, $h, $l, $c).expect(stringify!($f));
            assert_eq!(out.len(), $o.len());
            $sum += out.iter().filter(|&&v| v != 0).count();
        )+
    }};
}

macro_rules! call_into {
    ($o:expr, $h:expr, $l:expr, $c:expr, $buf:expr, $sum:expr, $($f:ident),+ $(,)?) => {{
        $(
            cdl::$f($o, $h, $l, $c, $buf).expect(stringify!($f));
            $sum += $buf.iter().filter(|&&v| v != 0).count();
        )+
    }};
}

fn run_candlestick_matrix<T: liq_ta::traits::SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> usize {
    let mut signal_count = 0usize;
    call_non_into!(
        open,
        high,
        low,
        close,
        signal_count,
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

    let mut out = vec![0_i32; open.len()];
    call_into!(
        open,
        high,
        low,
        close,
        &mut out,
        signal_count,
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

    signal_count
}

#[test]
fn outlier_hunt_candlestick_seed_sweep_f64_and_f32() {
    let mut total_signals = 0usize;
    for seed in 0_u64..24 {
        let (open, high, low, close) = make_ohlc_seeded(seed * 7919 + 17, 320);
        total_signals += run_candlestick_matrix(&open, &high, &low, &close);

        let open32: Vec<f32> = open.iter().map(|&v| v as f32).collect();
        let high32: Vec<f32> = high.iter().map(|&v| v as f32).collect();
        let low32: Vec<f32> = low.iter().map(|&v| v as f32).collect();
        let close32: Vec<f32> = close.iter().map(|&v| v as f32).collect();
        total_signals += run_candlestick_matrix(&open32, &high32, &low32, &close32);
    }
    assert!(total_signals > 0);
}

#[test]
fn outlier_hunt_bollinger_kama_stochrsi_trix_gaussian_sweeps() {
    for seed in 0_u64..12 {
        let (open, high, low, close) = make_ohlc_seeded(seed * 104729 + 3, 260);
        let data = close;

        for period in [1usize, 2, 5, 14, 20, 34] {
            let bb = bollinger(&data, period, 2.0).expect("bollinger");
            assert_eq!(bb.upper.len(), data.len());
            let mut bb_into = BollingerOutput {
                upper: vec![f64::NAN; data.len()],
                middle: vec![f64::NAN; data.len()],
                lower: vec![f64::NAN; data.len()],
            };
            bollinger_into(&data, period, 2.0, &mut bb_into).expect("bollinger_into");

            let k = kama(&data, period.max(1)).expect("kama");
            assert_eq!(k.len(), data.len());
            let mut k_into = vec![f64::NAN; data.len()];
            kama_into(&data, period.max(1), &mut k_into).expect("kama_into");
            let mut k_full_into = vec![f64::NAN; data.len()];
            kama_full_into(&data, period.max(1), 2, 30, &mut k_full_into).expect("kama_full_into");
            let _ = kama_full(&data, period.max(1), 2, 30).expect("kama_full");
        }

        for (rp, sp, kp, dp) in [(5, 5, 1, 3), (7, 7, 3, 3), (14, 14, 1, 1), (10, 8, 2, 4)] {
            let sr = stochrsi(&data, rp, sp, kp, dp).expect("stochrsi");
            assert_eq!(sr.fastk.len(), data.len());
            let mut fk = vec![f64::NAN; data.len()];
            let mut fd = vec![f64::NAN; data.len()];
            stochrsi_into(&data, rp, sp, kp, dp, &mut fk, &mut fd).expect("stochrsi_into");
        }

        for period in [1usize, 2, 5, 15] {
            let tr = trix(&data, period).expect("trix");
            assert_eq!(tr.len(), data.len());
            let mut tr_into = vec![f64::NAN; data.len()];
            trix_into(&data, period, &mut tr_into).expect("trix_into");
        }

        for period in [1usize, 5, 20] {
            let gf = gaussian_filter(&data, period, 0.5).expect("gaussian_filter");
            assert_eq!(gf.len(), data.len());
            let mut gf_into = vec![f64::NAN; data.len()];
            gaussian_filter_into(&data, period, 0.5, &mut gf_into).expect("gaussian_filter_into");

            let gc = gaussian_channel(&data, period, 0.5, 2.0).expect("gaussian_channel");
            assert_eq!(gc.center.len(), data.len());
            let mut center = vec![f64::NAN; data.len()];
            let mut upper = vec![f64::NAN; data.len()];
            let mut lower = vec![f64::NAN; data.len()];
            let mut trend = vec![f64::NAN; data.len()];
            gaussian_channel_into(
                &data,
                period,
                0.5,
                2.0,
                &mut center,
                &mut upper,
                &mut lower,
                &mut trend,
            )
            .expect("gaussian_channel_into");
        }

        let _ = (open, high, low); // keep varied OHLC generation in the sweep
    }
}

#[test]
fn outlier_hunt_explicit_error_surfaces() {
    let data = vec![1.0_f64, 2.0, 3.0, 4.0];
    let mut out = vec![0.0_f64; data.len()];
    let mut bb_short = BollingerOutput {
        upper: vec![0.0_f64; 3],
        middle: vec![0.0_f64; 3],
        lower: vec![0.0_f64; 3],
    };

    assert!(bollinger(&data, 0, 2.0).is_err());
    assert!(bollinger_into(&data, 2, 2.0, &mut bb_short).is_err());

    assert!(kama(&data, 0).is_err());
    assert!(kama_full(&data, 2, 0, 30).is_err());
    assert!(kama_full(&data, 2, 2, 0).is_err());
    assert!(kama_full(&data, 2, 30, 2).is_ok());
    assert!(kama_into(&data, 2, &mut out[..3]).is_err());

    let mut fk = vec![0.0_f64; data.len()];
    let mut fd = vec![0.0_f64; data.len()];
    assert!(stochrsi(&data, 0, 2, 1, 3).is_err());
    assert!(stochrsi_into(&data, 2, 2, 1, 3, &mut fk[..3], &mut fd).is_err());

    assert!(trix(&data, 0).is_err());
    assert!(trix_into(&data, 2, &mut out[..3]).is_err());

    assert!(gaussian_filter(&data, 0, 0.5).is_err());
    assert!(gaussian_filter(&data, 2, f64::NAN).is_err());
    assert!(gaussian_filter_into(&data, 2, 0.5, &mut out[..3]).is_err());
    assert!(gaussian_channel(&data, 0, 0.5, 2.0).is_err());
    assert!(gaussian_channel(&data, 2, 0.0, 2.0).is_err());
    assert!(gaussian_channel(&data, 2, 0.5, 0.0).is_err());
}

#[test]
fn outlier_hunt_candlestick_regime_flood() {
    let mut total_signals = 0usize;

    for seed in 0_u64..160 {
        let (mut open, mut high, mut low, mut close) = make_ohlc_seeded(seed * 65537 + 97, 512);

        for i in (16..open.len()).step_by(17) {
            let base = 100.0 + (i as f64) * 0.03 + (seed as f64) * 0.07;
            let flip = if (i + seed as usize).is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            let o = base - 0.7 * flip;
            let c = base + 1.4 * flip;
            let h = o.max(c) + 0.15;
            let l = o.min(c) - 0.15;
            open[i] = o;
            close[i] = c;
            high[i] = h;
            low[i] = l;
        }
        for i in (24..open.len()).step_by(29) {
            let base = 120.0 + (i as f64) * 0.01;
            open[i] = base;
            close[i] = base + if i % 2 == 0 { 0.01 } else { -0.01 };
            high[i] = base + 2.4;
            low[i] = base - 2.6;
        }

        total_signals += run_candlestick_matrix(&open, &high, &low, &close);

        let open32: Vec<f32> = open.iter().map(|&v| v as f32).collect();
        let high32: Vec<f32> = high.iter().map(|&v| v as f32).collect();
        let low32: Vec<f32> = low.iter().map(|&v| v as f32).collect();
        let close32: Vec<f32> = close.iter().map(|&v| v as f32).collect();
        total_signals += run_candlestick_matrix(&open32, &high32, &low32, &close32);
    }

    assert!(total_signals > 5000);
}

#[test]
fn outlier_hunt_nonfinite_and_extreme_numeric_surfaces() {
    let n = 360usize;
    let mut data = (0..n)
        .map(|i| 100.0 + (i as f64) * 0.05 + (((i * 17) % 11) as f64 - 5.0) * 0.07)
        .collect::<Vec<_>>();
    for i in (5..n).step_by(41) {
        data[i] = f64::NAN;
    }
    for i in (11..n).step_by(53) {
        data[i] = f64::INFINITY;
    }
    for i in (19..n).step_by(67) {
        data[i] = f64::NEG_INFINITY;
    }

    for period in [1usize, 2, 5, 14, 34] {
        let _ = bollinger(&data, period, 2.0);
        let mut bb = BollingerOutput {
            upper: vec![f64::NAN; n],
            middle: vec![f64::NAN; n],
            lower: vec![f64::NAN; n],
        };
        let _ = bollinger_into(&data, period, 2.0, &mut bb);

        let _ = kama(&data, period.max(1));
        let mut k = vec![f64::NAN; n];
        let _ = kama_into(&data, period.max(1), &mut k);
        let mut kf = vec![f64::NAN; n];
        let _ = kama_full_into(&data, period.max(1), 2, 30, &mut kf);

        let _ = trix(&data, period.max(1));
        let mut tr = vec![f64::NAN; n];
        let _ = trix_into(&data, period.max(1), &mut tr);

        let _ = gaussian_filter(&data, period.max(1), 0.5);
        let mut gf = vec![f64::NAN; n];
        let _ = gaussian_filter_into(&data, period.max(1), 0.5, &mut gf);
    }

    for (rp, sp, kp, dp) in [(5, 5, 1, 3), (8, 8, 2, 4), (14, 14, 1, 1)] {
        let _ = stochrsi(&data, rp, sp, kp, dp);
        let mut fk = vec![f64::NAN; n];
        let mut fd = vec![f64::NAN; n];
        let _ = stochrsi_into(&data, rp, sp, kp, dp, &mut fk, &mut fd);
    }

    for period in [1usize, 5, 20] {
        let _ = gaussian_channel(&data, period, 0.5, 2.0);
        let mut center = vec![f64::NAN; n];
        let mut upper = vec![f64::NAN; n];
        let mut lower = vec![f64::NAN; n];
        let mut trend = vec![f64::NAN; n];
        let _ = gaussian_channel_into(
            &data,
            period,
            0.5,
            2.0,
            &mut center,
            &mut upper,
            &mut lower,
            &mut trend,
        );
    }

    let data32: Vec<f32> = data.iter().map(|&v| v as f32).collect();
    let _ = bollinger(&data32, 14, 2.0_f32);
    let mut bb32 = BollingerOutput {
        upper: vec![f32::NAN; n],
        middle: vec![f32::NAN; n],
        lower: vec![f32::NAN; n],
    };
    let _ = bollinger_into(&data32, 14, 2.0_f32, &mut bb32);
    let _ = stochrsi(&data32, 14, 14, 1, 3);
    let mut fk32 = vec![f32::NAN; n];
    let mut fd32 = vec![f32::NAN; n];
    let _ = stochrsi_into(&data32, 14, 14, 1, 3, &mut fk32, &mut fd32);
    let _ = trix(&data32, 14);
    let mut tr32 = vec![f32::NAN; n];
    let _ = trix_into(&data32, 14, &mut tr32);
}

#[test]
fn outlier_hunt_candlestick_deep_regime_flood() {
    let mut aggregate = 0usize;
    for seed in 0_u64..1400 {
        let (mut open, mut high, mut low, mut close) = make_ohlc_seeded(seed * 1_000_003 + 73, 220);

        for i in (12..open.len()).step_by(13) {
            let base = 80.0 + (i as f64) * 0.07 + (seed as f64) * 0.004;
            let dir = if (i + seed as usize).is_multiple_of(3) {
                1.0
            } else {
                -1.0
            };
            open[i] = base - 0.4 * dir;
            close[i] = base + 1.1 * dir;
            high[i] = open[i].max(close[i]) + 0.12;
            low[i] = open[i].min(close[i]) - 0.12;
        }
        for i in (21..open.len()).step_by(19) {
            let base = 92.0 + (i as f64) * 0.05;
            open[i] = base;
            close[i] = base + if i % 2 == 0 { 0.004 } else { -0.004 };
            high[i] = base + 1.9;
            low[i] = base - 2.1;
        }
        for i in (30..open.len()).step_by(23) {
            let base = 105.0 + (seed as f64) * 0.002;
            open[i] = base - 1.4;
            close[i] = base + 1.6;
            high[i] = close[i] + 0.03;
            low[i] = open[i] - 0.03;
        }

        aggregate += run_candlestick_matrix(&open, &high, &low, &close);
    }

    assert!(aggregate > 10_000);
}

#[test]
fn outlier_hunt_candlestick_polarity_mirror_and_reverse_flood() {
    let mut aggregate = 0usize;

    for seed in 0_u64..600 {
        let (open, high, low, close) = make_ohlc_seeded(seed * 2_147_483_647 + 101, 260);
        aggregate += run_candlestick_matrix(&open, &high, &low, &close);

        let (mo, mh, ml, mc) = mirror_ohlc(&open, &high, &low, &close);
        aggregate += run_candlestick_matrix(&mo, &mh, &ml, &mc);

        let (ro, rh, rl, rc) = reverse_ohlc(&open, &high, &low, &close);
        aggregate += run_candlestick_matrix(&ro, &rh, &rl, &rc);

        let (mro, mrh, mrl, mrc) = mirror_ohlc(&ro, &rh, &rl, &rc);
        aggregate += run_candlestick_matrix(&mro, &mrh, &mrl, &mrc);

        if seed % 5 == 0 {
            let mo32: Vec<f32> = mo.iter().map(|&v| v as f32).collect();
            let mh32: Vec<f32> = mh.iter().map(|&v| v as f32).collect();
            let ml32: Vec<f32> = ml.iter().map(|&v| v as f32).collect();
            let mc32: Vec<f32> = mc.iter().map(|&v| v as f32).collect();
            aggregate += run_candlestick_matrix(&mo32, &mh32, &ml32, &mc32);
        }
    }

    assert!(aggregate > 20_000);
}
