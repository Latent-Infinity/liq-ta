use liq_ta::indicators::midpoint::{midpoint, midpoint_into, midpoint_lookback};

fn make_data(n: usize) -> Vec<f64> {
    let mut data = Vec::with_capacity(n);
    let mut v = 100.0_f64;
    for i in 0..n {
        v += if i % 4 == 0 { 0.35 } else { -0.09 } + (i as f64 * 0.02).sin() * 0.15;
        data.push(v);
    }
    data
}

#[test]
fn midpoint_large_input_vhgw_dispatch_alloc_and_into_parity() {
    let data = make_data(1200);
    let period = 63;
    let lookback = midpoint_lookback(period);

    let alloc = midpoint(&data, period).expect("midpoint alloc should succeed");

    let mut into = vec![0.0_f64; data.len()];
    midpoint_into(&data, period, &mut into).expect("midpoint_into should succeed");

    for i in 0..lookback {
        assert!(alloc[i].is_nan());
        assert!(into[i].is_nan());
    }
    for i in lookback..data.len() {
        assert!((alloc[i] - into[i]).abs() < 1e-10, "mismatch at {i}");
    }
}

#[test]
fn midpoint_period_one_non_finite_normalization() {
    let data = [1.0_f64, f64::NAN, f64::INFINITY, 4.0];
    let out = midpoint(&data, 1).expect("period=1 midpoint should succeed");

    assert_eq!(out.len(), data.len());
    assert!((out[0] - 1.0).abs() < 1e-12);
    assert!(out[1].is_nan());
    assert!(out[2].is_nan());
    assert!((out[3] - 4.0).abs() < 1e-12);
}

#[test]
fn midpoint_f32_deque_path_smoke() {
    let data: Vec<f32> = (0..64).map(|i| 50.0_f32 + (i as f32) * 0.25).collect();
    let period = 9;
    let out = midpoint(&data, period).expect("f32 midpoint should succeed");

    assert_eq!(out.len(), data.len());
    for v in &out[..midpoint_lookback(period)] {
        assert!(v.is_nan());
    }
    assert!(out[midpoint_lookback(period)].is_finite());
}
