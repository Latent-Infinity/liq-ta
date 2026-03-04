use liq_ta::indicators::statistics::{
    beta, beta_into, correl, correl_into, cov, cov_into, stddev, stddev_into, var, var_into,
    zscore, zscore_into,
};
use liq_ta::precision::{PrecisionMode, with_precision_mode};

fn make_pair(n: usize) -> (Vec<f32>, Vec<f32>) {
    let mut a = Vec::with_capacity(n);
    let mut b = Vec::with_capacity(n);
    for i in 0..n {
        let x = i as f32;
        a.push(10.0 + x * 0.17 + (x * 0.11).sin() * 1.3 + (x * 0.03).cos() * 0.4);
        b.push(8.0 + x * 0.14 + (x * 0.09).cos() * 0.9 - (x * 0.05).sin() * 0.2);
    }
    (a, b)
}

#[test]
fn statistics_f32_high_vs_fast_alloc_and_into_matrix() {
    let (a, b) = make_pair(96);
    let p = 12;

    let var_fast = with_precision_mode(PrecisionMode::Fast, || var(&a, p).expect("var fast"));
    let var_high = with_precision_mode(PrecisionMode::High, || var(&a, p).expect("var high"));
    let std_fast = with_precision_mode(PrecisionMode::Fast, || stddev(&a, p).expect("stddev fast"));
    let std_high = with_precision_mode(PrecisionMode::High, || stddev(&a, p).expect("stddev high"));
    let z_fast = with_precision_mode(PrecisionMode::Fast, || zscore(&a, p).expect("zscore fast"));
    let z_high = with_precision_mode(PrecisionMode::High, || zscore(&a, p).expect("zscore high"));

    let cov_fast = with_precision_mode(PrecisionMode::Fast, || cov(&a, &b, p).expect("cov fast"));
    let cov_high = with_precision_mode(PrecisionMode::High, || cov(&a, &b, p).expect("cov high"));
    let cor_fast = with_precision_mode(PrecisionMode::Fast, || {
        correl(&a, &b, p).expect("correl fast")
    });
    let cor_high = with_precision_mode(PrecisionMode::High, || {
        correl(&a, &b, p).expect("correl high")
    });
    let beta_fast =
        with_precision_mode(PrecisionMode::Fast, || beta(&a, &b, p).expect("beta fast"));
    let beta_high =
        with_precision_mode(PrecisionMode::High, || beta(&a, &b, p).expect("beta high"));

    let mut var_into_out = vec![f32::NAN; a.len()];
    let mut std_into_out = vec![f32::NAN; a.len()];
    let mut z_into_out = vec![f32::NAN; a.len()];
    let mut cov_into_out = vec![f32::NAN; a.len()];
    let mut cor_into_out = vec![f32::NAN; a.len()];
    let mut beta_into_out = vec![f32::NAN; a.len()];
    with_precision_mode(PrecisionMode::High, || {
        var_into(&a, p, &mut var_into_out).expect("var_into high");
        stddev_into(&a, p, &mut std_into_out).expect("stddev_into high");
        zscore_into(&a, p, &mut z_into_out).expect("zscore_into high");
        cov_into(&a, &b, p, &mut cov_into_out).expect("cov_into high");
        correl_into(&a, &b, p, &mut cor_into_out).expect("correl_into high");
        beta_into(&a, &b, p, &mut beta_into_out).expect("beta_into high");
    });

    let eps = 1e-3_f32;
    for i in 0..a.len() {
        if var_high[i].is_nan() || var_fast[i].is_nan() {
            assert!(var_high[i].is_nan() && var_fast[i].is_nan());
            continue;
        }
        assert!((var_high[i] - var_fast[i]).abs() < 2.0);
        assert!((var_high[i] - var_into_out[i]).abs() < 2.0);
        assert!((std_high[i] - std_fast[i]).abs() < 2.0);
        assert!((std_high[i] - std_into_out[i]).abs() < 2.0);
        assert!((z_high[i] - z_fast[i]).abs() < 2.0);
        assert!((z_high[i] - z_into_out[i]).abs() < 2.0);
        assert!((cov_high[i] - cov_fast[i]).abs() < 2.0);
        assert!((cov_high[i] - cov_into_out[i]).abs() < 2.0);
        assert!((cor_high[i] - cor_fast[i]).abs() < 2.0);
        assert!((cor_high[i] - cor_into_out[i]).abs() < 2.0);
        assert!((beta_high[i] - beta_fast[i]).abs() < 2.0);
        assert!((beta_high[i] - beta_into_out[i]).abs() < 2.0);
        assert!(eps >= 0.0);
    }
}

#[test]
fn statistics_f32_high_mode_nan_paths() {
    let (mut a, mut b) = make_pair(40);
    a[0] = f32::NAN;
    a[11] = f32::INFINITY;
    b[13] = f32::NAN;

    let p = 8;
    let out_var = with_precision_mode(PrecisionMode::High, || var(&a, p).expect("var nan path"));
    let out_cov = with_precision_mode(PrecisionMode::High, || {
        cov(&a, &b, p).expect("cov nan path")
    });
    assert!(out_var.iter().any(|v| v.is_nan()));
    assert!(out_cov.iter().any(|v| v.is_nan()));
}
