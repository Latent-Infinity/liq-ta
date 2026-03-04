use liq_ta::indicators::ad::{ad, ad_into};
use liq_ta::precision::{PrecisionMode, with_precision_mode};

fn make_ohlcv(n: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    let mut volume = Vec::with_capacity(n);

    let mut p = 100.0_f32;
    for i in 0..n {
        p += if i % 3 == 0 { 0.55 } else { -0.21 } + ((i as f32) * 0.08).sin() * 0.25;
        let c = p;
        let h = c + 0.9 + ((i % 5) as f32) * 0.03;
        let l = c - 0.8 - ((i % 4) as f32) * 0.02;
        high.push(h);
        low.push(l);
        close.push(c);
        volume.push(80_000.0 + (i as f32) * 171.0 + ((i as f32) * 0.09).cos() * 80.0);
    }

    (high, low, close, volume)
}

fn make_finite_ohlcv(n: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    let mut volume = Vec::with_capacity(n);

    for i in 0..n {
        let h = 100.0 + i as f32;
        let l = h - 1.0;
        let c = h - 0.25;
        high.push(h);
        low.push(l);
        close.push(c);
        volume.push(1_000.0 + i as f32);
    }

    (high, low, close, volume)
}

fn assert_non_finite_propagation(mode: PrecisionMode, trigger_index: usize) {
    let (high, low, close, mut volume) = make_finite_ohlcv(5);
    volume[trigger_index] = f32::INFINITY;

    let mut out = vec![0.0_f32; high.len()];
    with_precision_mode(mode, || {
        ad_into(&high, &low, &close, &volume, &mut out).expect("ad_into should succeed");
    });

    for value in &out[..trigger_index] {
        assert!(value.is_finite());
    }
    assert!(out[trigger_index].is_nan());
    for value in &out[trigger_index + 1..] {
        assert!(value.is_nan());
    }
}

#[test]
fn ad_f32_high_vs_fast_alloc_and_into_paths() {
    let (high, low, close, volume) = make_ohlcv(128);

    let fast = with_precision_mode(PrecisionMode::Fast, || {
        ad(&high, &low, &close, &volume).expect("ad fast should succeed")
    });
    let high_mode = with_precision_mode(PrecisionMode::High, || {
        ad(&high, &low, &close, &volume).expect("ad high should succeed")
    });

    assert_eq!(fast.len(), high_mode.len());

    let mut out_fast = vec![f32::NAN; high.len()];
    let mut out_high = vec![f32::NAN; high.len()];
    with_precision_mode(PrecisionMode::Fast, || {
        ad_into(&high, &low, &close, &volume, &mut out_fast).expect("ad_into fast");
    });
    with_precision_mode(PrecisionMode::High, || {
        ad_into(&high, &low, &close, &volume, &mut out_high).expect("ad_into high");
    });

    for i in 0..high.len() {
        if fast[i].is_nan() || high_mode[i].is_nan() {
            assert!(fast[i].is_nan() && high_mode[i].is_nan());
        } else {
            assert!((fast[i] - high_mode[i]).abs() < 5.0);
            assert!((fast[i] - out_fast[i]).abs() < 5.0);
            assert!((high_mode[i] - out_high[i]).abs() < 5.0);
        }
    }
}

#[test]
fn ad_nan_zero_range_and_infinity_paths() {
    let (mut high, mut low, mut close, mut volume) = make_ohlcv(20);

    high[5] = 123.0;
    low[5] = 123.0;
    close[5] = 123.0;

    close[8] = f32::NAN;
    volume[9] = f32::INFINITY;

    let out = ad(&high, &low, &close, &volume).expect("ad should succeed with NaN propagation");
    assert!(out[8].is_nan());
    assert!(out[9].is_nan());
    assert!(out[10].is_nan());
}

#[test]
fn ad_validation_matrix() {
    let (high, low, close, volume) = make_ohlcv(10);
    let mut out = vec![0.0_f32; 10];
    let mut short = vec![0.0_f32; 9];

    let empty: [f32; 0] = [];
    assert!(ad(&empty, &empty, &empty, &empty).is_err());
    assert!(ad(&high[..9], &low, &close, &volume).is_err());
    assert!(ad(&high, &low[..9], &close, &volume).is_err());
    assert!(ad(&high, &low, &close[..9], &volume).is_err());
    assert!(ad(&high, &low, &close, &volume[..9]).is_err());

    assert!(ad_into(&high, &low, &close, &volume, &mut short).is_err());
    assert!(ad_into(&high[..9], &low, &close, &volume, &mut out).is_err());
}

#[test]
fn ad_into_empty_input_explicit_error_path() {
    let empty: [f32; 0] = [];
    let mut out: [f32; 0] = [];
    assert!(ad_into(&empty, &empty, &empty, &empty, &mut out).is_err());
}

#[test]
fn ad_fast_non_finite_propagation_by_position() {
    for trigger_index in 0..5 {
        assert_non_finite_propagation(PrecisionMode::Fast, trigger_index);
    }
}

#[test]
fn ad_high_non_finite_propagation_by_position() {
    for trigger_index in 0..5 {
        assert_non_finite_propagation(PrecisionMode::High, trigger_index);
    }
}

#[test]
fn ad_high_zero_range_with_non_finite_numerator() {
    let high = [1.0_f32];
    let low = [1.0_f32];
    let close = [f32::INFINITY];
    let volume = [1.0_f32];

    let out = with_precision_mode(PrecisionMode::High, || {
        ad(&high, &low, &close, &volume).expect("ad should succeed with NaN propagation")
    });
    assert_eq!(out.len(), 1);
    assert!(out[0].is_nan());
}
