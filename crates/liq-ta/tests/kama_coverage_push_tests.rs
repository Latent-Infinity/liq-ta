use liq_ta::indicators::kama::{
    kama, kama_full, kama_full_into, kama_into, kama_lookback, kama_min_len,
};

fn make_wave(n: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let x = i as f64;
        out.push(100.0 + x * 0.35 + (x * 0.21).sin() * 2.3 + (x * 0.07).cos() * 0.9);
    }
    out
}

#[test]
fn kama_error_matrix_and_lookup_edges() {
    let data = make_wave(24);
    let mut out = vec![f64::NAN; data.len()];
    let mut short = vec![f64::NAN; data.len() - 1];

    assert_eq!(kama_lookback(0), 0);
    assert_eq!(kama_min_len(0), 1);

    assert!(kama_full_into::<f64>(&[], 10, 2, 30, &mut out).is_err());
    assert!(kama_full_into(&data, 0, 2, 30, &mut out).is_err());
    assert!(kama_full_into(&data, 10, 0, 30, &mut out).is_err());
    assert!(kama_full_into(&data, 10, 2, 0, &mut out).is_err());
    assert!(kama_full_into(&data[..8], 10, 2, 30, &mut out[..8]).is_err());
    assert!(kama_full_into(&data, 10, 2, 30, &mut short).is_err());
}

#[test]
fn kama_long_series_unrolled_and_tail_f64() {
    let data = make_wave(67);
    let mut out = vec![f64::NAN; data.len()];

    kama_full_into(&data, 10, 2, 30, &mut out).expect("kama_full_into f64 should succeed");
    let direct = kama_full(&data, 10, 2, 30).expect("kama_full f64 should succeed");

    assert_eq!(out.len(), data.len());
    assert_eq!(direct.len(), data.len());
    for i in 0..data.len() {
        if out[i].is_nan() || direct[i].is_nan() {
            assert!(out[i].is_nan() && direct[i].is_nan());
        } else {
            assert!((out[i] - direct[i]).abs() < 1e-10);
        }
    }
}

#[test]
fn kama_period_one_and_constant_series_stability() {
    let data = vec![42.0_f64; 32];
    let mut out = vec![f64::NAN; data.len()];

    kama_into(&data, 1, &mut out).expect("kama_into period=1 should succeed");
    for value in out {
        assert!(value.is_finite());
        assert!((value - 42.0).abs() < 1e-12);
    }
}

#[test]
fn kama_f32_paths_and_wrapper_consistency() {
    let data64 = make_wave(43);
    let data: Vec<f32> = data64.iter().map(|&v| v as f32).collect();
    let mut out = vec![f32::NAN; data.len()];

    kama_into(&data, 10, &mut out).expect("kama_into f32 should succeed");
    let wrap = kama(&data, 10).expect("kama wrapper f32 should succeed");
    let full = kama_full(&data, 10, 2, 30).expect("kama_full f32 should succeed");

    assert_eq!(out.len(), data.len());
    assert_eq!(wrap.len(), data.len());
    assert_eq!(full.len(), data.len());

    for i in 0..data.len() {
        if out[i].is_nan() || wrap[i].is_nan() || full[i].is_nan() {
            assert!(out[i].is_nan() && wrap[i].is_nan() && full[i].is_nan());
        } else {
            assert!((out[i] - wrap[i]).abs() < 1e-4);
            assert!((out[i] - full[i]).abs() < 1e-4);
        }
    }
}

#[test]
fn kama_full_zero_volatility_constant_series_branches() {
    let data64 = vec![123.45_f64; 48];
    let mut out64 = vec![f64::NAN; data64.len()];
    kama_full_into(&data64, 10, 2, 30, &mut out64)
        .expect("kama_full_into f64 constant should succeed");
    let wrap64 = kama_full(&data64, 10, 2, 30).expect("kama_full f64 constant should succeed");
    assert_eq!(out64.len(), wrap64.len());
    for i in 0..out64.len() {
        if out64[i].is_nan() || wrap64[i].is_nan() {
            assert!(out64[i].is_nan() && wrap64[i].is_nan());
        } else {
            assert!((out64[i] - 123.45).abs() < 1e-10);
            assert!((out64[i] - wrap64[i]).abs() < 1e-10);
        }
    }

    let data32: Vec<f32> = data64.iter().map(|&v| v as f32).collect();
    let mut out32 = vec![f32::NAN; data32.len()];
    kama_full_into(&data32, 10, 2, 30, &mut out32)
        .expect("kama_full_into f32 constant should succeed");
    let wrap32 = kama_full(&data32, 10, 2, 30).expect("kama_full f32 constant should succeed");
    assert_eq!(out32.len(), wrap32.len());
    for i in 0..out32.len() {
        if out32[i].is_nan() || wrap32[i].is_nan() {
            assert!(out32[i].is_nan() && wrap32[i].is_nan());
        } else {
            assert!((out32[i] - 123.45_f32).abs() < 1e-3);
            assert!((out32[i] - wrap32[i]).abs() < 1e-3);
        }
    }
}
