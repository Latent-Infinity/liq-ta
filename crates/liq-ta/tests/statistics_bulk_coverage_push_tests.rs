use liq_ta::indicators::statistics::{
    beta, beta_into, correl, correl_into, cov, cov_into, kurt, kurt_into, linearreg,
    linearreg_angle, linearreg_angle_into, linearreg_intercept, linearreg_intercept_into,
    linearreg_into, linearreg_slope, linearreg_slope_into, mad, mad_into, sem, sem_into, skew,
    skew_into, stddev, stddev_into, tsf, tsf_into, var, var_into, zscore, zscore_into,
};

fn make_series(n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut x = Vec::with_capacity(n);
    let mut y = Vec::with_capacity(n);
    let mut a = 100.0_f64;
    let mut b = 80.0_f64;
    for i in 0..n {
        a += if i % 5 < 3 { 0.35 } else { -0.18 } + (i as f64 * 0.04).sin() * 0.2;
        b += if i % 7 < 4 { 0.28 } else { -0.14 } + (i as f64 * 0.03).cos() * 0.17;
        x.push(a);
        y.push(b + (a - 100.0) * 0.15);
    }
    (x, y)
}

fn assert_parity(a: &[f64], b: &[f64], eps: f64) {
    assert_eq!(a.len(), b.len());
    for i in 0..a.len() {
        if a[i].is_nan() || b[i].is_nan() {
            assert!(a[i].is_nan() && b[i].is_nan(), "NaN mismatch at {i}");
        } else {
            assert!(
                (a[i] - b[i]).abs() < eps,
                "value mismatch at {i}: {} vs {}",
                a[i],
                b[i]
            );
        }
    }
}

#[test]
fn statistics_bulk_alloc_into_parity_f64() {
    let (x, y) = make_series(220);
    let p = 20;

    let v_var = var(&x, p).expect("var");
    let v_std = stddev(&x, p).expect("stddev");
    let v_skew = skew(&x, p).expect("skew");
    let v_kurt = kurt(&x, p).expect("kurt");
    let v_z = zscore(&x, p).expect("zscore");
    let v_mad = mad(&x, p).expect("mad");
    let v_sem = sem(&x, p).expect("sem");
    let v_cov = cov(&x, &y, p).expect("cov");
    let v_cor = correl(&x, &y, p).expect("correl");
    let v_beta = beta(&x, &y, p).expect("beta");
    let v_lr = linearreg(&x, p).expect("linearreg");
    let v_slope = linearreg_slope(&x, p).expect("linearreg_slope");
    let v_intercept = linearreg_intercept(&x, p).expect("linearreg_intercept");
    let v_angle = linearreg_angle(&x, p).expect("linearreg_angle");
    let v_tsf = tsf(&x, p).expect("tsf");

    let mut o_var = vec![0.0_f64; x.len()];
    let mut o_std = vec![0.0_f64; x.len()];
    let mut o_skew = vec![0.0_f64; x.len()];
    let mut o_kurt = vec![0.0_f64; x.len()];
    let mut o_z = vec![0.0_f64; x.len()];
    let mut o_mad = vec![0.0_f64; x.len()];
    let mut o_sem = vec![0.0_f64; x.len()];
    let mut o_cov = vec![0.0_f64; x.len()];
    let mut o_cor = vec![0.0_f64; x.len()];
    let mut o_beta = vec![0.0_f64; x.len()];
    let mut o_lr = vec![0.0_f64; x.len()];
    let mut o_slope = vec![0.0_f64; x.len()];
    let mut o_intercept = vec![0.0_f64; x.len()];
    let mut o_angle = vec![0.0_f64; x.len()];
    let mut o_tsf = vec![0.0_f64; x.len()];

    var_into(&x, p, &mut o_var).expect("var_into");
    stddev_into(&x, p, &mut o_std).expect("stddev_into");
    skew_into(&x, p, &mut o_skew).expect("skew_into");
    kurt_into(&x, p, &mut o_kurt).expect("kurt_into");
    zscore_into(&x, p, &mut o_z).expect("zscore_into");
    mad_into(&x, p, &mut o_mad).expect("mad_into");
    sem_into(&x, p, &mut o_sem).expect("sem_into");
    cov_into(&x, &y, p, &mut o_cov).expect("cov_into");
    correl_into(&x, &y, p, &mut o_cor).expect("correl_into");
    beta_into(&x, &y, p, &mut o_beta).expect("beta_into");
    linearreg_into(&x, p, &mut o_lr).expect("linearreg_into");
    linearreg_slope_into(&x, p, &mut o_slope).expect("linearreg_slope_into");
    linearreg_intercept_into(&x, p, &mut o_intercept).expect("linearreg_intercept_into");
    linearreg_angle_into(&x, p, &mut o_angle).expect("linearreg_angle_into");
    tsf_into(&x, p, &mut o_tsf).expect("tsf_into");

    assert_parity(&v_var, &o_var, 1e-10);
    assert_parity(&v_std, &o_std, 1e-10);
    assert_parity(&v_skew, &o_skew, 1e-10);
    assert_parity(&v_kurt, &o_kurt, 1e-10);
    assert_parity(&v_z, &o_z, 1e-10);
    assert_parity(&v_mad, &o_mad, 1e-10);
    assert_parity(&v_sem, &o_sem, 1e-10);
    assert_parity(&v_cov, &o_cov, 1e-10);
    assert_parity(&v_cor, &o_cor, 1e-10);
    assert_parity(&v_beta, &o_beta, 1e-10);
    assert_parity(&v_lr, &o_lr, 1e-10);
    assert_parity(&v_slope, &o_slope, 1e-10);
    assert_parity(&v_intercept, &o_intercept, 1e-10);
    assert_parity(&v_angle, &o_angle, 1e-10);
    assert_parity(&v_tsf, &o_tsf, 1e-10);
}

#[test]
fn statistics_bulk_nan_and_f32_smoke() {
    let (x64, y64) = make_series(128);
    let mut x: Vec<f32> = x64.iter().map(|&v| v as f32).collect();
    let mut y: Vec<f32> = y64.iter().map(|&v| v as f32).collect();
    let p = 14;

    x[21] = f32::NAN;
    y[45] = f32::INFINITY;

    assert!(var(&x, p).is_ok());
    assert!(stddev(&x, p).is_ok());
    assert!(skew(&x, p).is_ok());
    assert!(kurt(&x, p).is_ok());
    assert!(zscore(&x, p).is_ok());
    assert!(mad(&x, p).is_ok());
    assert!(sem(&x, p).is_ok());
    assert!(cov(&x, &y, p).is_ok());
    assert!(correl(&x, &y, p).is_ok());
    assert!(beta(&x, &y, p).is_ok());
    assert!(linearreg(&x, p).is_ok());
    assert!(linearreg_slope(&x, p).is_ok());
    assert!(linearreg_intercept(&x, p).is_ok());
    assert!(linearreg_angle(&x, p).is_ok());
    assert!(tsf(&x, p).is_ok());
}
