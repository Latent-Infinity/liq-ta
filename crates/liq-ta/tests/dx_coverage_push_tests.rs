use liq_ta::indicators::dx::{
    adxr, adxr_into, dx, dx_into, minus_dm, minus_dm_into, plus_dm, plus_dm_into,
};

fn make_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);

    let mut price = 100.0_f64;
    for i in 0..n {
        let drift = if i % 7 < 4 { 0.42 } else { -0.31 };
        price += drift + (i as f64 * 0.03).sin() * 0.4;
        let c = price;
        let h = c + 0.7 + ((i % 5) as f64) * 0.08;
        let l = c - 0.65 - ((i % 3) as f64) * 0.07;
        high.push(h.max(l + 0.01));
        low.push(l);
        close.push(c);
    }

    (high, low, close)
}

fn assert_vec_parity(a: &[f64], b: &[f64], eps: f64) {
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
fn dx_group_alloc_vs_into_parity_f64() {
    let (high, low, close) = make_ohlc(96);
    let p = 14;

    let dx_v = dx(&high, &low, &close, p).expect("dx should succeed");
    let adxr_v = adxr(&high, &low, &close, p).expect("adxr should succeed");
    let plus_v = plus_dm(&high, &low, p).expect("plus_dm should succeed");
    let minus_v = minus_dm(&high, &low, p).expect("minus_dm should succeed");

    let mut dx_o = vec![f64::NAN; high.len()];
    let mut adxr_o = vec![f64::NAN; high.len()];
    let mut plus_o = vec![f64::NAN; high.len()];
    let mut minus_o = vec![f64::NAN; high.len()];

    dx_into(&high, &low, &close, p, &mut dx_o).expect("dx_into should succeed");
    adxr_into(&high, &low, &close, p, &mut adxr_o).expect("adxr_into should succeed");
    plus_dm_into(&high, &low, p, &mut plus_o).expect("plus_dm_into should succeed");
    minus_dm_into(&high, &low, p, &mut minus_o).expect("minus_dm_into should succeed");

    assert_vec_parity(&dx_v, &dx_o, 1e-10);
    assert_vec_parity(&adxr_v, &adxr_o, 1e-10);
    assert_vec_parity(&plus_v, &plus_o, 1e-10);
    assert_vec_parity(&minus_v, &minus_o, 1e-10);
}

#[test]
fn dx_group_validation_errors() {
    let (high, low, close) = make_ohlc(20);
    let mut out = vec![f64::NAN; high.len()];
    let mut short = vec![f64::NAN; high.len() - 1];

    assert!(dx(&high, &low, &close, 0).is_err());
    assert!(adxr(&high, &low, &close, 0).is_err());
    assert!(plus_dm(&high, &low, 0).is_err());
    assert!(minus_dm(&high, &low, 0).is_err());

    assert!(dx(&high[..10], &low, &close, 5).is_err());
    assert!(adxr(&high, &low[..10], &close, 5).is_err());
    assert!(dx(&high, &low, &close[..10], 5).is_err());
    assert!(plus_dm(&high[..10], &low, 5).is_err());
    assert!(minus_dm(&high, &low[..10], 5).is_err());

    assert!(dx_into(&high, &low, &close, 5, &mut short).is_err());
    assert!(adxr_into(&high, &low, &close, 5, &mut short).is_err());
    assert!(plus_dm_into(&high, &low, 5, &mut short).is_err());
    assert!(minus_dm_into(&high, &low, 5, &mut short).is_err());

    assert!(dx_into(&high[..10], &low, &close, 5, &mut out).is_err());
    assert!(adxr_into(&high, &low[..10], &close, 5, &mut out).is_err());
    assert!(plus_dm_into(&high[..10], &low, 5, &mut out).is_err());
    assert!(minus_dm_into(&high, &low[..10], 5, &mut out).is_err());
}

#[test]
fn dx_group_f32_smoke() {
    let (h64, l64, c64) = make_ohlc(48);
    let high: Vec<f32> = h64.iter().map(|&v| v as f32).collect();
    let low: Vec<f32> = l64.iter().map(|&v| v as f32).collect();
    let close: Vec<f32> = c64.iter().map(|&v| v as f32).collect();
    let p = 10;

    assert!(dx(&high, &low, &close, p).is_ok());
    assert!(adxr(&high, &low, &close, p).is_ok());
    assert!(plus_dm(&high, &low, p).is_ok());
    assert!(minus_dm(&high, &low, p).is_ok());
}
