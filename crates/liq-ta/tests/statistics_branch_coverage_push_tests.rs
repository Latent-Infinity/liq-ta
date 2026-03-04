use liq_ta::indicators::statistics::{
    beta_into, correl_into, cov_into, kurt, kurt_into, linearreg_angle, linearreg_angle_into,
    linearreg_intercept, linearreg_intercept_into, linearreg_slope, linearreg_slope_into, mad,
    mad_into, sem, sem_into, skew, skew_into, tsf, tsf_into, var, var_into, zscore_into,
};
use liq_ta::precision::{PrecisionMode, with_precision_mode};

fn make_pair_f32(n: usize) -> (Vec<f32>, Vec<f32>) {
    let mut a = Vec::with_capacity(n);
    let mut b = Vec::with_capacity(n);
    for i in 0..n {
        let x = i as f32;
        a.push(12.0 + x * 0.19 + (x * 0.07).sin() * 0.8);
        b.push(9.0 + x * 0.15 + (x * 0.05).cos() * 0.6);
    }
    (a, b)
}

#[test]
fn statistics_f32_alloc_dispatch_surface() {
    let (a, _) = make_pair_f32(64);
    let p = 10;

    assert!(skew(&a, p).is_ok());
    assert!(kurt(&a, p).is_ok());
    assert!(mad(&a, p).is_ok());
    assert!(sem(&a, p).is_ok());
    assert!(linearreg_slope(&a, p).is_ok());
    assert!(linearreg_intercept(&a, p).is_ok());
    assert!(linearreg_angle(&a, p).is_ok());
    assert!(tsf(&a, p).is_ok());
}

#[test]
fn statistics_into_empty_input_matrix() {
    let empty: Vec<f64> = vec![];
    let mut out: Vec<f64> = vec![];

    assert!(var_into(&empty, 5, &mut out).is_err());
    assert!(skew_into(&empty, 5, &mut out).is_err());
    assert!(kurt_into(&empty, 5, &mut out).is_err());
    assert!(zscore_into(&empty, 5, &mut out).is_err());
    assert!(mad_into(&empty, 5, &mut out).is_err());
    assert!(sem_into(&empty, 5, &mut out).is_err());
    assert!(linearreg_slope_into(&empty, 5, &mut out).is_err());
    assert!(linearreg_intercept_into(&empty, 5, &mut out).is_err());
    assert!(linearreg_angle_into(&empty, 5, &mut out).is_err());
    assert!(tsf_into(&empty, 5, &mut out).is_err());

    assert!(cov_into(&empty, &empty, 5, &mut out).is_err());
    assert!(correl_into(&empty, &empty, 5, &mut out).is_err());
    assert!(beta_into(&empty, &empty, 5, &mut out).is_err());
}

#[test]
fn statistics_var_period_one_dispatch_matrix() {
    let finite_f64 = vec![1.0_f64, 2.0, 3.0, 4.0];
    let mut finite_out_f64 = vec![f64::NAN; finite_f64.len()];
    let v_f64 = var(&finite_f64, 1).expect("var f64 p1 should succeed");
    var_into(&finite_f64, 1, &mut finite_out_f64).expect("var_into f64 p1 should succeed");
    assert!(v_f64.iter().all(|x| x.is_finite() && *x == 0.0));
    assert!(finite_out_f64.iter().all(|x| x.is_finite() && *x == 0.0));

    let nan_f64 = vec![1.0_f64, f64::NAN, 2.0, 3.0];
    let mut nan_out_f64 = vec![f64::NAN; nan_f64.len()];
    let v_nan_f64 = var(&nan_f64, 1).expect("var f64 p1 nan should succeed");
    var_into(&nan_f64, 1, &mut nan_out_f64).expect("var_into f64 p1 nan should succeed");
    assert_eq!(v_nan_f64.len(), nan_out_f64.len());
    for i in 0..v_nan_f64.len() {
        if v_nan_f64[i].is_nan() || nan_out_f64[i].is_nan() {
            assert!(v_nan_f64[i].is_nan() && nan_out_f64[i].is_nan());
        } else {
            assert!((v_nan_f64[i] - nan_out_f64[i]).abs() < 1e-12);
        }
    }

    let finite_f32 = vec![1.0_f32, 2.0, 3.0, 4.0];
    let mut finite_out_f32 = vec![f32::NAN; finite_f32.len()];
    let v_f32_fast = with_precision_mode(PrecisionMode::Fast, || {
        var(&finite_f32, 1).expect("var f32 fast p1 should succeed")
    });
    with_precision_mode(PrecisionMode::Fast, || {
        var_into(&finite_f32, 1, &mut finite_out_f32).expect("var_into f32 fast p1 should succeed")
    });
    assert!(v_f32_fast.iter().all(|x| x.is_finite() && *x == 0.0));
    assert!(finite_out_f32.iter().all(|x| x.is_finite() && *x == 0.0));

    let nan_f32 = vec![1.0_f32, f32::NAN, 2.0, 3.0];
    let mut nan_out_f32 = vec![f32::NAN; nan_f32.len()];
    let v_f32_high = with_precision_mode(PrecisionMode::High, || {
        var(&nan_f32, 1).expect("var f32 high p1 nan should succeed")
    });
    with_precision_mode(PrecisionMode::High, || {
        var_into(&nan_f32, 1, &mut nan_out_f32).expect("var_into f32 high p1 nan should succeed")
    });
    assert_eq!(v_f32_high.len(), nan_out_f32.len());
    for i in 0..v_f32_high.len() {
        if v_f32_high[i].is_nan() || nan_out_f32[i].is_nan() {
            assert!(v_f32_high[i].is_nan() && nan_out_f32[i].is_nan());
        } else {
            assert!((v_f32_high[i] - nan_out_f32[i]).abs() < 1e-5);
        }
    }
}
