use liq_ta::indicators::trima::{trima, trima_into};
use liq_ta::indicators::wma::{wma, wma_into};

fn make_series(n: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let x = i as f64;
        out.push(200.0 + x * 0.23 + (x * 0.14).sin() * 1.4 + (x * 0.03).cos() * 0.5);
    }
    out
}

fn assert_parity(a: &[f64], b: &[f64], eps: f64) {
    assert_eq!(a.len(), b.len());
    for i in 0..a.len() {
        if a[i].is_nan() || b[i].is_nan() {
            assert!(a[i].is_nan() && b[i].is_nan());
        } else {
            assert!(
                (a[i] - b[i]).abs() < eps,
                "mismatch at {}: {} vs {}",
                i,
                a[i],
                b[i]
            );
        }
    }
}

#[test]
fn trima_wma_alloc_vs_into_period_matrix() {
    let data = make_series(97);
    let periods = [1_usize, 2, 3, 5, 10, 32, data.len()];

    for period in periods {
        let trima_v = trima(&data, period).expect("trima should succeed");
        let wma_v = wma(&data, period).expect("wma should succeed");

        let mut trima_o = vec![f64::NAN; data.len()];
        let mut wma_o = vec![f64::NAN; data.len()];
        trima_into(&data, period, &mut trima_o).expect("trima_into should succeed");
        wma_into(&data, period, &mut wma_o).expect("wma_into should succeed");

        assert_parity(&trima_v, &trima_o, 1e-10);
        assert_parity(&wma_v, &wma_o, 1e-10);
    }
}

#[test]
fn trima_wma_nan_and_infinity_paths() {
    let mut data = make_series(48);
    data[0] = f64::INFINITY;
    data[12] = f64::NAN;
    data[26] = f64::NEG_INFINITY;

    let t = trima(&data, 5).expect("trima nan path should succeed");
    let w = wma(&data, 5).expect("wma nan path should succeed");
    assert!(t.iter().any(|v| v.is_nan()));
    assert!(w.iter().any(|v| v.is_nan()));

    let mut to = vec![0.0_f64; data.len()];
    let mut wo = vec![0.0_f64; data.len()];
    trima_into(&data, 5, &mut to).expect("trima_into nan path should succeed");
    wma_into(&data, 5, &mut wo).expect("wma_into nan path should succeed");
    assert!(to.iter().any(|v| v.is_nan()));
    assert!(wo.iter().any(|v| v.is_nan()));
}

#[test]
fn trima_wma_validation_matrix() {
    let data = make_series(12);
    let mut out = vec![0.0_f64; data.len()];
    let mut short = vec![0.0_f64; data.len() - 1];
    let empty: [f64; 0] = [];

    assert!(trima(&empty, 5).is_err());
    assert!(wma(&empty, 5).is_err());
    assert!(trima(&data, 0).is_err());
    assert!(wma(&data, 0).is_err());
    assert!(trima(&data, data.len() + 1).is_err());
    assert!(wma(&data, data.len() + 1).is_err());

    assert!(trima_into(&data, 5, &mut short).is_err());
    assert!(wma_into(&data, 5, &mut short).is_err());
    assert!(trima_into(&data, 0, &mut out).is_err());
    assert!(wma_into(&data, 0, &mut out).is_err());
}
