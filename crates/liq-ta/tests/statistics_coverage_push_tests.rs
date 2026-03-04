use liq_ta::indicators::statistics as stats;
use liq_ta::indicators::statistics::{
    beta, beta_into, correl, correl_into, cov, cov_into, kurt, kurt_into, linearreg,
    linearreg_angle, linearreg_angle_into, linearreg_intercept, linearreg_intercept_into,
    linearreg_into, linearreg_slope, linearreg_slope_into, mad, mad_into, sem, sem_into, skew,
    skew_into, stddev, stddev_into, tsf, tsf_into, var, var_into, zscore, zscore_into,
};

fn make_series(n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut a = Vec::with_capacity(n);
    let mut b = Vec::with_capacity(n);
    for i in 0..n {
        let x = i as f64;
        let av = 75.0 + x * 0.21 + (x * 0.13).sin() * 1.7 + (x * 0.03).cos() * 0.4;
        let bv = 30.0 + x * 0.19 + (x * 0.17).cos() * 1.2 - (x * 0.05).sin() * 0.3;
        a.push(av);
        b.push(bv);
    }
    (a, b)
}

macro_rules! assert_unary_pair {
    ($data:expr, $period:expr, $alloc_fn:ident, $into_fn:ident) => {{
        let expected = $alloc_fn($data, $period).expect(concat!(stringify!($alloc_fn), " alloc"));
        let mut out = vec![f64::NAN; $data.len()];
        $into_fn($data, $period, &mut out).expect(concat!(stringify!($into_fn), " into"));
        assert_eq!(expected.len(), out.len());
        for i in 0..out.len() {
            if expected[i].is_nan() || out[i].is_nan() {
                assert!(expected[i].is_nan() && out[i].is_nan());
            } else {
                assert!(
                    (expected[i] - out[i]).abs() < 1e-9,
                    "{} mismatch at {}: {} vs {}",
                    stringify!($alloc_fn),
                    i,
                    expected[i],
                    out[i]
                );
            }
        }
    }};
}

macro_rules! assert_binary_pair {
    ($a:expr, $b:expr, $period:expr, $alloc_fn:ident, $into_fn:ident) => {{
        let expected = $alloc_fn($a, $b, $period).expect(concat!(stringify!($alloc_fn), " alloc"));
        let mut out = vec![f64::NAN; $a.len()];
        $into_fn($a, $b, $period, &mut out).expect(concat!(stringify!($into_fn), " into"));
        assert_eq!(expected.len(), out.len());
        for i in 0..out.len() {
            if expected[i].is_nan() || out[i].is_nan() {
                assert!(expected[i].is_nan() && out[i].is_nan());
            } else {
                assert!(
                    (expected[i] - out[i]).abs() < 1e-9,
                    "{} mismatch at {}: {} vs {}",
                    stringify!($alloc_fn),
                    i,
                    expected[i],
                    out[i]
                );
            }
        }
    }};
}

#[test]
fn statistics_unary_matrix_f64_alloc_vs_into() {
    let (a, _) = make_series(96);
    let p = 12;

    assert_unary_pair!(&a, p, var, var_into);
    assert_unary_pair!(&a, p, stddev, stddev_into);
    assert_unary_pair!(&a, p, skew, skew_into);
    assert_unary_pair!(&a, p, kurt, kurt_into);
    assert_unary_pair!(&a, p, zscore, zscore_into);
    assert_unary_pair!(&a, p, mad, mad_into);
    assert_unary_pair!(&a, p, sem, sem_into);
    assert_unary_pair!(&a, p, linearreg, linearreg_into);
    assert_unary_pair!(&a, p, linearreg_slope, linearreg_slope_into);
    assert_unary_pair!(&a, p, linearreg_intercept, linearreg_intercept_into);
    assert_unary_pair!(&a, p, linearreg_angle, linearreg_angle_into);
    assert_unary_pair!(&a, p, tsf, tsf_into);
}

#[test]
fn statistics_binary_matrix_f64_alloc_vs_into() {
    let (a, b) = make_series(96);
    let p = 14;

    assert_binary_pair!(&a, &b, p, cov, cov_into);
    assert_binary_pair!(&a, &b, p, correl, correl_into);
    assert_binary_pair!(&a, &b, p, beta, beta_into);
}

#[test]
fn statistics_error_matrix_validation_paths() {
    let (a, b) = make_series(20);
    let mut out = vec![f64::NAN; a.len()];
    let mut short = vec![f64::NAN; a.len() - 1];

    assert!(var(&a, 0).is_err());
    assert!(stddev(&a, 0).is_err());
    assert!(skew(&a, 0).is_err());
    assert!(kurt(&a, 0).is_err());
    assert!(zscore(&a, 0).is_err());
    assert!(mad(&a, 0).is_err());
    assert!(sem(&a, 0).is_err());
    assert!(cov(&a, &b, 0).is_err());
    assert!(correl(&a, &b, 0).is_err());
    assert!(beta(&a, &b, 0).is_err());
    assert!(linearreg(&a, 0).is_err());
    assert!(linearreg_slope(&a, 0).is_err());
    assert!(linearreg_intercept(&a, 0).is_err());
    assert!(linearreg_angle(&a, 0).is_err());
    assert!(tsf(&a, 0).is_err());

    assert!(var_into(&a, 5, &mut short).is_err());
    assert!(stddev_into(&a, 5, &mut short).is_err());
    assert!(skew_into(&a, 5, &mut short).is_err());
    assert!(kurt_into(&a, 5, &mut short).is_err());
    assert!(zscore_into(&a, 5, &mut short).is_err());
    assert!(mad_into(&a, 5, &mut short).is_err());
    assert!(sem_into(&a, 5, &mut short).is_err());
    assert!(linearreg_into(&a, 5, &mut short).is_err());
    assert!(linearreg_slope_into(&a, 5, &mut short).is_err());
    assert!(linearreg_intercept_into(&a, 5, &mut short).is_err());
    assert!(linearreg_angle_into(&a, 5, &mut short).is_err());
    assert!(tsf_into(&a, 5, &mut short).is_err());

    assert!(cov(&a, &b[..b.len() - 1], 5).is_err());
    assert!(correl(&a, &b[..b.len() - 1], 5).is_err());
    assert!(beta(&a, &b[..b.len() - 1], 5).is_err());
    assert!(cov_into(&a, &b, 5, &mut short).is_err());
    assert!(correl_into(&a, &b, 5, &mut short).is_err());
    assert!(beta_into(&a, &b, 5, &mut short).is_err());
    assert!(cov_into(&a, &b[..b.len() - 1], 5, &mut out).is_err());
    assert!(correl_into(&a, &b[..b.len() - 1], 5, &mut out).is_err());
    assert!(beta_into(&a, &b[..b.len() - 1], 5, &mut out).is_err());

    assert!(var_into(&a, 0, &mut out).is_err());
    assert!(stddev_into(&a, 0, &mut out).is_err());
    assert!(skew_into(&a, 0, &mut out).is_err());
    assert!(kurt_into(&a, 0, &mut out).is_err());
    assert!(zscore_into(&a, 0, &mut out).is_err());
    assert!(mad_into(&a, 0, &mut out).is_err());
    assert!(sem_into(&a, 0, &mut out).is_err());
    assert!(linearreg_into(&a, 0, &mut out).is_err());
    assert!(linearreg_slope_into(&a, 0, &mut out).is_err());
    assert!(linearreg_intercept_into(&a, 0, &mut out).is_err());
    assert!(linearreg_angle_into(&a, 0, &mut out).is_err());
    assert!(tsf_into(&a, 0, &mut out).is_err());
    assert!(cov_into(&a, &b, 0, &mut out).is_err());
    assert!(correl_into(&a, &b, 0, &mut out).is_err());
    assert!(beta_into(&a, &b, 0, &mut out).is_err());

    let short_data = &a[..4];
    let short_data2 = &b[..4];
    assert!(var(short_data, 5).is_err());
    assert!(stddev(short_data, 5).is_err());
    assert!(skew(short_data, 5).is_err());
    assert!(kurt(short_data, 5).is_err());
    assert!(zscore(short_data, 5).is_err());
    assert!(mad(short_data, 5).is_err());
    assert!(sem(short_data, 5).is_err());
    assert!(linearreg(short_data, 5).is_err());
    assert!(linearreg_slope(short_data, 5).is_err());
    assert!(linearreg_intercept(short_data, 5).is_err());
    assert!(linearreg_angle(short_data, 5).is_err());
    assert!(tsf(short_data, 5).is_err());
    assert!(cov(short_data, short_data2, 5).is_err());
    assert!(correl(short_data, short_data2, 5).is_err());
    assert!(beta(short_data, short_data2, 5).is_err());
}

#[test]
fn statistics_f32_smoke() {
    let (a64, b64) = make_series(48);
    let a: Vec<f32> = a64.iter().map(|&v| v as f32).collect();
    let b: Vec<f32> = b64.iter().map(|&v| v as f32).collect();
    let p = 8;

    assert!(var(&a, p).is_ok());
    assert!(stddev(&a, p).is_ok());
    assert!(zscore(&a, p).is_ok());
    assert!(linearreg(&a, p).is_ok());
    assert!(cov(&a, &b, p).is_ok());
    assert!(correl(&a, &b, p).is_ok());
    assert!(beta(&a, &b, p).is_ok());
}

#[test]
fn statistics_lookback_and_min_len_surface() {
    assert_eq!(stats::var_lookback(0), 0);
    assert_eq!(stats::var_min_len(0), 0);
    assert_eq!(stats::stddev_lookback(0), 0);
    assert_eq!(stats::stddev_min_len(0), 0);
    assert_eq!(stats::skew_lookback(0), 0);
    assert_eq!(stats::skew_min_len(0), 0);
    assert_eq!(stats::kurt_lookback(0), 0);
    assert_eq!(stats::kurt_min_len(0), 0);
    assert_eq!(stats::cov_lookback(0), 0);
    assert_eq!(stats::cov_min_len(0), 0);
    assert_eq!(stats::zscore_lookback(0), 0);
    assert_eq!(stats::zscore_min_len(0), 0);
    assert_eq!(stats::mad_lookback(0), 0);
    assert_eq!(stats::mad_min_len(0), 0);
    assert_eq!(stats::sem_lookback(0), 0);
    assert_eq!(stats::sem_min_len(0), 0);
    assert_eq!(stats::correl_lookback(0), 0);
    assert_eq!(stats::correl_min_len(0), 0);
    assert_eq!(stats::beta_lookback(0), 0);
    assert_eq!(stats::beta_min_len(0), 0);
    assert_eq!(stats::linearreg_lookback(0), 0);
    assert_eq!(stats::linearreg_min_len(0), 0);
    assert_eq!(stats::linearreg_slope_lookback(0), 0);
    assert_eq!(stats::linearreg_slope_min_len(0), 0);
    assert_eq!(stats::linearreg_intercept_lookback(0), 0);
    assert_eq!(stats::linearreg_intercept_min_len(0), 0);
    assert_eq!(stats::linearreg_angle_lookback(0), 0);
    assert_eq!(stats::linearreg_angle_min_len(0), 0);
    assert_eq!(stats::tsf_lookback(0), 0);
    assert_eq!(stats::tsf_min_len(0), 0);

    assert_eq!(stats::var_lookback(7), 6);
    assert_eq!(stats::var_min_len(7), 7);
    assert_eq!(stats::stddev_lookback(7), 6);
    assert_eq!(stats::stddev_min_len(7), 7);
    assert_eq!(stats::skew_lookback(7), 6);
    assert_eq!(stats::skew_min_len(7), 7);
    assert_eq!(stats::kurt_lookback(7), 6);
    assert_eq!(stats::kurt_min_len(7), 7);
    assert_eq!(stats::cov_lookback(7), 6);
    assert_eq!(stats::cov_min_len(7), 7);
    assert_eq!(stats::zscore_lookback(7), 6);
    assert_eq!(stats::zscore_min_len(7), 7);
    assert_eq!(stats::mad_lookback(7), 6);
    assert_eq!(stats::mad_min_len(7), 7);
    assert_eq!(stats::sem_lookback(7), 6);
    assert_eq!(stats::sem_min_len(7), 7);
    assert_eq!(stats::correl_lookback(7), 6);
    assert_eq!(stats::correl_min_len(7), 7);
    assert_eq!(stats::beta_lookback(7), 6);
    assert_eq!(stats::beta_min_len(7), 7);
    assert_eq!(stats::linearreg_lookback(7), 6);
    assert_eq!(stats::linearreg_min_len(7), 7);
    assert_eq!(stats::linearreg_slope_lookback(7), 6);
    assert_eq!(stats::linearreg_slope_min_len(7), 7);
    assert_eq!(stats::linearreg_intercept_lookback(7), 6);
    assert_eq!(stats::linearreg_intercept_min_len(7), 7);
    assert_eq!(stats::linearreg_angle_lookback(7), 6);
    assert_eq!(stats::linearreg_angle_min_len(7), 7);
    assert_eq!(stats::tsf_lookback(7), 6);
    assert_eq!(stats::tsf_min_len(7), 7);
}
