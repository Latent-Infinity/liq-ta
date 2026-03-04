use liq_ta::indicators::obv::{obv, obv_into};
use liq_ta::precision::{PrecisionMode, with_precision_mode};

fn make_close_volume_f32(n: usize) -> (Vec<f32>, Vec<f32>) {
    let mut close = Vec::with_capacity(n);
    let mut volume = Vec::with_capacity(n);
    let mut p = 100.0_f32;
    for i in 0..n {
        p += if i % 3 == 0 { 0.75 } else { -0.31 } + ((i as f32) * 0.12).sin() * 0.2;
        close.push(p);
        volume.push(50_000.0 + (i as f32) * 137.0 + ((i as f32) * 0.09).cos() * 55.0);
    }
    (close, volume)
}

#[test]
fn obv_f32_high_vs_fast_alloc_and_into_paths() {
    let (close, volume) = make_close_volume_f32(128);

    let fast_alloc = with_precision_mode(PrecisionMode::Fast, || {
        obv(&close, &volume).expect("obv fast should succeed")
    });
    let high_alloc = with_precision_mode(PrecisionMode::High, || {
        obv(&close, &volume).expect("obv high should succeed")
    });

    assert_eq!(fast_alloc.len(), high_alloc.len());

    let mut fast_into = vec![f32::NAN; close.len()];
    let mut high_into = vec![f32::NAN; close.len()];
    with_precision_mode(PrecisionMode::Fast, || {
        obv_into(&close, &volume, &mut fast_into).expect("obv_into fast should succeed");
    });
    with_precision_mode(PrecisionMode::High, || {
        obv_into(&close, &volume, &mut high_into).expect("obv_into high should succeed");
    });

    for i in 0..close.len() {
        if fast_alloc[i].is_nan() || high_alloc[i].is_nan() {
            assert!(fast_alloc[i].is_nan() && high_alloc[i].is_nan());
        } else {
            assert!((fast_alloc[i] - high_alloc[i]).abs() < 2.0);
            assert!((fast_alloc[i] - fast_into[i]).abs() < 2.0);
            assert!((high_alloc[i] - high_into[i]).abs() < 2.0);
        }
    }
}

#[test]
fn obv_f32_high_mode_nan_propagation_into() {
    let (mut close, mut volume) = make_close_volume_f32(32);
    close[7] = f32::NAN;
    volume[0] = 42_000.0;
    volume[8] = f32::NAN;

    let mut out = vec![0.0_f32; close.len()];
    with_precision_mode(PrecisionMode::High, || {
        obv_into(&close, &volume, &mut out).expect("obv_into high nan path should succeed");
    });

    assert!(out[7].is_nan());
    for v in &out[8..] {
        assert!(v.is_nan());
    }
}
