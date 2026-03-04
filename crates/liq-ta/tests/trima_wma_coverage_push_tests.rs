use liq_ta::indicators::trima::{trima, trima_into, trima_lookback, trima_min_len};
use liq_ta::indicators::wma::{wma, wma_into, wma_lookback, wma_min_len};

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

fn sample_data(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| 50.0 + (i as f64) * 0.45 + ((i % 5) as f64 - 2.0) * 0.3)
        .collect()
}

#[test]
fn coverage_trima_surface_even_odd_and_into_parity() {
    let data = sample_data(64);
    assert_eq!(trima_lookback(1), 0);
    assert_eq!(trima_lookback(7), 6);
    assert_eq!(trima_min_len(1), 1);
    assert_eq!(trima_min_len(7), 7);

    for &period in &[5_usize, 6_usize] {
        let out = trima(&data, period).expect("trima should succeed");
        assert_eq!(out.len(), data.len());
        let lb = trima_lookback(period);
        assert!(out[..lb].iter().all(|v| v.is_nan()));
        assert!(out[lb..].iter().all(|v| v.is_finite()));

        let mut out_buf = vec![0.0_f64; data.len()];
        trima_into(&data, period, &mut out_buf).expect("trima_into should succeed");
        for i in 0..data.len() {
            if out[i].is_nan() {
                assert!(out_buf[i].is_nan());
            } else {
                assert!(approx_eq(out[i], out_buf[i], 1e-12));
            }
        }
    }

    let data32: Vec<f32> = data.iter().map(|&v| v as f32).collect();
    assert!(trima(&data32, 5).is_ok());
    assert!(trima(&data32, 6).is_ok());
}

#[test]
fn coverage_trima_period_one_and_error_matrix() {
    let data = sample_data(20);

    let out = trima(&data, 1).expect("period 1 trima should succeed");
    assert_eq!(out, data);

    let mut out_buf = vec![0.0_f64; data.len()];
    trima_into(&data, 1, &mut out_buf).expect("period 1 trima_into should succeed");
    assert_eq!(out_buf, data);

    assert!(trima(&[] as &[f64], 5).is_err());
    assert!(trima(&data, 0).is_err());
    assert!(trima(&data[..4], 5).is_err());

    let mut short = vec![0.0_f64; data.len() - 1];
    assert!(trima_into(&data, 5, &mut short).is_err());
    assert!(trima_into(&[] as &[f64], 5, &mut out_buf).is_err());
    assert!(trima_into(&data, 0, &mut out_buf).is_err());
    assert!(trima_into(&data[..4], 5, &mut out_buf).is_err());
}

#[test]
fn coverage_trima_non_finite_paths() {
    let mut data = sample_data(40);
    data[10] = f64::NAN;
    data[11] = f64::INFINITY;
    data[12] = f64::NEG_INFINITY;

    let out = trima(&data, 7).expect("trima should succeed with non-finite input");
    assert!(out.iter().skip(6).any(|v| v.is_nan()));

    let mut out_buf = vec![0.0_f64; data.len()];
    trima_into(&data, 8, &mut out_buf).expect("trima_into should succeed with non-finite input");
    assert!(out_buf.iter().skip(7).any(|v| v.is_nan()));
}

#[test]
fn coverage_wma_surface_even_odd_and_into_parity() {
    let data = sample_data(80);
    assert_eq!(wma_lookback(1), 0);
    assert_eq!(wma_lookback(7), 6);
    assert_eq!(wma_min_len(1), 1);
    assert_eq!(wma_min_len(7), 7);

    for &period in &[5_usize, 6_usize] {
        let out = wma(&data, period).expect("wma should succeed");
        assert_eq!(out.len(), data.len());
        let lb = wma_lookback(period);
        assert!(out[..lb].iter().all(|v| v.is_nan()));
        assert!(out[lb..].iter().all(|v| v.is_finite()));

        let mut out_buf = vec![0.0_f64; data.len()];
        let valid = wma_into(&data, period, &mut out_buf).expect("wma_into should succeed");
        assert_eq!(valid, data.len() - lb);
        for i in 0..data.len() {
            if out[i].is_nan() {
                assert!(out_buf[i].is_nan());
            } else {
                assert!(approx_eq(out[i], out_buf[i], 1e-12));
            }
        }
    }

    let data32: Vec<f32> = data.iter().map(|&v| v as f32).collect();
    assert!(wma(&data32, 5).is_ok());
    assert!(wma(&data32, 6).is_ok());
}

#[test]
fn coverage_wma_period_one_non_finite_and_errors() {
    let data = sample_data(24);

    let out = wma(&data, 1).expect("period 1 wma should succeed");
    assert_eq!(out, data);

    let mut out_buf = vec![0.0_f64; data.len()];
    let valid = wma_into(&data, 1, &mut out_buf).expect("period 1 wma_into should succeed");
    assert_eq!(valid, data.len());
    assert_eq!(out_buf, data);

    let mut bad = data.clone();
    bad[7] = f64::NAN;
    bad[8] = f64::INFINITY;
    let non_finite_out = wma(&bad, 5).expect("wma should succeed with non-finite input");
    assert!(non_finite_out.iter().skip(4).any(|v| v.is_nan()));

    let mut non_finite_buf = vec![0.0_f64; bad.len()];
    wma_into(&bad, 5, &mut non_finite_buf).expect("wma_into with non-finite should succeed");
    assert!(non_finite_buf.iter().skip(4).any(|v| v.is_nan()));

    assert!(wma(&[] as &[f64], 5).is_err());
    assert!(wma(&data, 0).is_err());
    assert!(wma(&data[..4], 5).is_err());

    let mut short = vec![0.0_f64; data.len() - 1];
    assert!(wma_into(&data, 5, &mut short).is_err());
    assert!(wma_into(&[] as &[f64], 5, &mut out_buf).is_err());
    assert!(wma_into(&data, 0, &mut out_buf).is_err());
    assert!(wma_into(&data[..4], 5, &mut out_buf).is_err());
}
