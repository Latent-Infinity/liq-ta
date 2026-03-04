use liq_ta::indicators::candlestick as cdl;
use liq_ta::indicators::statistics::{
    beta, beta_into, correl, correl_into, cov, cov_into, kurt, kurt_into, linearreg,
    linearreg_angle, linearreg_angle_into, linearreg_intercept, linearreg_intercept_into,
    linearreg_into, linearreg_slope, linearreg_slope_into, mad, mad_into, sem, sem_into, skew,
    skew_into, stddev, stddev_into, tsf, tsf_into, var, var_into, zscore, zscore_into,
};
use liq_ta::indicators::stochastic::{
    Stochastic, StochasticOutput, stochastic, stochastic_fast, stochastic_fast_into,
    stochastic_full, stochastic_full_into, stochastic_into, stochastic_k_lookback,
    stochastic_min_len, stochastic_slow, stochastic_slow_into,
};
use liq_ta::indicators::trima::{trima, trima_into};
use liq_ta::indicators::wma::{wma, wma_into};
use liq_ta::kernels::rolling_extrema::{
    MonotonicDeque, compute_stochastic_fast_vhgw_f32, compute_stochastic_fast_vhgw_f64,
    compute_stochastic_full_vhgw_f32, compute_stochastic_full_vhgw_f64, rolling_extrema,
    rolling_extrema_fused_vhgw, rolling_extrema_into, rolling_max, rolling_max_into,
    rolling_max_nan_propagating, rolling_midpoint_vhgw_f64, rolling_min, rolling_min_into,
    rolling_min_nan_propagating,
};

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

#[test]
fn coverage_statistics_scalar_family_smoke() {
    let data: Vec<f64> = (1..=32).map(f64::from).collect();
    let period = 8;
    let lookback = period - 1;

    let out_var = var(&data, period).expect("var should succeed");
    let out_stddev = stddev(&data, period).expect("stddev should succeed");
    let out_skew = skew(&data, period).expect("skew should succeed");
    let out_kurt = kurt(&data, period).expect("kurt should succeed");
    let out_zscore = zscore(&data, period).expect("zscore should succeed");
    let out_mad = mad(&data, period).expect("mad should succeed");
    let out_sem = sem(&data, period).expect("sem should succeed");
    let out_linearreg = linearreg(&data, period).expect("linearreg should succeed");
    let out_slope = linearreg_slope(&data, period).expect("linearreg_slope should succeed");
    let out_intercept =
        linearreg_intercept(&data, period).expect("linearreg_intercept should succeed");
    let out_angle = linearreg_angle(&data, period).expect("linearreg_angle should succeed");
    let out_tsf = tsf(&data, period).expect("tsf should succeed");

    for out in [
        &out_var,
        &out_stddev,
        &out_skew,
        &out_kurt,
        &out_zscore,
        &out_mad,
        &out_sem,
        &out_linearreg,
        &out_slope,
        &out_intercept,
        &out_angle,
        &out_tsf,
    ] {
        assert_eq!(out.len(), data.len());
        assert!(out[..lookback].iter().all(|v| v.is_nan()));
        assert!(out[lookback..].iter().all(|v| v.is_finite()));
    }
}

#[test]
fn coverage_statistics_pairwise_relationships() {
    let x: Vec<f64> = (1..=40).map(f64::from).collect();
    let y: Vec<f64> = x.iter().map(|v| 2.0 * v + 3.0).collect();
    let period = 10;
    let lookback = period - 1;

    let cov_out = cov(&x, &y, period).expect("cov should succeed");
    let correl_out = correl(&x, &y, period).expect("correl should succeed");
    let beta_out = beta(&x, &y, period).expect("beta should succeed");

    assert_eq!(cov_out.len(), x.len());
    assert_eq!(correl_out.len(), x.len());
    assert_eq!(beta_out.len(), x.len());
    assert!(cov_out[lookback..].iter().all(|v| *v > 0.0));
    assert!(
        correl_out[lookback..]
            .iter()
            .all(|v| approx_eq(*v, 1.0, 1e-10))
    );
    assert!(
        beta_out[lookback..]
            .iter()
            .all(|v| approx_eq(*v, 0.5, 1e-10))
    );
}

#[test]
fn coverage_statistics_zero_variance_denominator_branches() {
    let x = vec![7.0_f64; 20];
    let y = vec![3.0_f64; 20];
    let period = 5;
    let lookback = period - 1;

    let correl_out = correl(&x, &y, period).expect("correl should succeed");
    let beta_out = beta(&x, &y, period).expect("beta should succeed");
    let sem_out = sem(&x, period).expect("sem should succeed");
    let zscore_out = zscore(&x, period).expect("zscore should succeed");
    let skew_out = skew(&x, period).expect("skew should succeed");
    let kurt_out = kurt(&x, period).expect("kurt should succeed");

    assert!(
        correl_out[lookback..]
            .iter()
            .all(|v| approx_eq(*v, 0.0, 1e-12))
    );
    assert!(
        beta_out[lookback..]
            .iter()
            .all(|v| approx_eq(*v, 0.0, 1e-12))
    );
    assert!(
        sem_out[lookback..]
            .iter()
            .all(|v| approx_eq(*v, 0.0, 1e-12))
    );
    assert_eq!(zscore_out.len(), x.len());
    assert_eq!(skew_out.len(), x.len());
    assert_eq!(kurt_out.len(), x.len());
}

#[test]
fn coverage_statistics_into_error_paths() {
    let data: Vec<f64> = (1..=10).map(f64::from).collect();
    let mut output = vec![0.0_f64; data.len()];
    let mut too_small = vec![0.0_f64; data.len() - 1];
    let shorter = &data[..data.len() - 1];

    assert!(var_into(&data, 3, &mut too_small).is_err());
    assert!(cov_into(&data, shorter, 3, &mut output).is_err());
    assert!(correl_into(&data, shorter, 3, &mut output).is_err());
}

#[test]
fn coverage_stochastic_flat_market_smoke() {
    let high = vec![10.0_f64; 32];
    let low = vec![10.0_f64; 32];
    let close = vec![10.0_f64; 32];

    assert!(stochastic(&high, &low, &close, 5, 3, 3).is_ok());
    assert!(stochastic_fast(&high, &low, &close, 5, 3).is_ok());
    assert!(stochastic_slow(&high, &low, &close, 5, 3).is_ok());
}

#[test]
fn coverage_stochastic_validation_paths() {
    let high = vec![11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    let low = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let close = vec![6.0, 7.0, 8.0, 9.0, 10.0, 11.0];

    assert!(stochastic(&high, &low, &close, 0, 3, 3).is_err());
    assert!(stochastic_fast(&high, &low, &close, 3, 0).is_err());
    assert!(stochastic_slow(&high[..5], &low, &close, 3, 2).is_err());
}

#[test]
fn coverage_rolling_extrema_core_and_into_paths() {
    let data = vec![3.0_f64, 1.0, 5.0, 2.0, 6.0, 4.0, 7.0];
    let period = 3;
    let lookback = period - 1;

    let max_out = rolling_max(&data, period).expect("rolling_max should succeed");
    let min_out = rolling_min(&data, period).expect("rolling_min should succeed");

    let mut max_into = vec![0.0_f64; data.len()];
    let mut min_into = vec![0.0_f64; data.len()];
    let valid = rolling_extrema_into(&data, period, &mut max_into, &mut min_into)
        .expect("rolling_extrema_into should succeed");

    assert_eq!(valid, data.len() - lookback);
    let valid_max = rolling_max_into(&data, period, &mut max_into).expect("rolling_max_into ok");
    let valid_min = rolling_min_into(&data, period, &mut min_into).expect("rolling_min_into ok");
    assert_eq!(valid_max, valid);
    assert_eq!(valid_min, valid);

    for i in 0..data.len() {
        if i < lookback {
            assert!(max_out[i].is_nan());
            assert!(min_out[i].is_nan());
            assert!(max_into[i].is_nan());
            assert!(min_into[i].is_nan());
        } else {
            assert!(approx_eq(max_out[i], max_into[i], 1e-12));
            assert!(approx_eq(min_out[i], min_into[i], 1e-12));
        }
    }
}

#[test]
fn coverage_rolling_extrema_nan_propagating_paths() {
    let data = vec![1.0_f64, 3.0, f64::NAN, 2.0, 5.0, 4.0, 6.0];
    let period = 3;

    let max_out =
        rolling_max_nan_propagating(&data, period).expect("rolling_max_nan_propagating ok");
    let min_out =
        rolling_min_nan_propagating(&data, period).expect("rolling_min_nan_propagating ok");

    assert!(max_out.iter().skip(period - 1).any(|v| v.is_nan()));
    assert!(min_out.iter().skip(period - 1).any(|v| v.is_nan()));
}

#[test]
fn coverage_rolling_extrema_vhgw_paths() {
    let high = vec![9.0_f64, 8.0, 10.0, 7.0, 11.0, 12.0];
    let low = vec![1.0_f64, 2.0, 1.5, 3.0, 2.5, 4.0];

    assert!(rolling_extrema_fused_vhgw(&high, &low, 3).is_ok());
    assert!(rolling_extrema_fused_vhgw(&high[..5], &low, 3).is_err());
    assert!(rolling_extrema_fused_vhgw(&[] as &[f64], &[] as &[f64], 3).is_err());
    assert!(rolling_extrema_fused_vhgw(&high, &low, 0).is_err());
    assert!(rolling_extrema_fused_vhgw(&high[..2], &low[..2], 3).is_err());

    let data: Vec<f64> = (1..=10).map(f64::from).collect();
    let mut output = vec![0.0_f64; data.len()];
    let written =
        rolling_midpoint_vhgw_f64(&data, 4, &mut output).expect("rolling_midpoint_vhgw_f64 ok");

    assert_eq!(written, data.len() - 3);
    assert!(output[..3].iter().all(|v| v.is_nan()));
    assert!(approx_eq(output[3], 2.5, 1e-12));

    let mut too_small = vec![0.0_f64; data.len() - 1];
    assert!(rolling_midpoint_vhgw_f64(&data, 4, &mut too_small).is_err());
    assert!(rolling_midpoint_vhgw_f64(&[] as &[f64], 4, &mut output).is_err());
    assert!(rolling_midpoint_vhgw_f64(&data, 0, &mut output).is_err());
    assert!(rolling_midpoint_vhgw_f64(&data[..2], 4, &mut output).is_err());

    let period_one_data = vec![1.0_f64, f64::NAN, 3.0];
    let mut period_one_out = vec![0.0_f64; period_one_data.len()];
    let c = rolling_midpoint_vhgw_f64(&period_one_data, 1, &mut period_one_out)
        .expect("period 1 should succeed");
    assert_eq!(c, period_one_data.len());
    assert!(period_one_out[1].is_nan());
}

#[test]
fn coverage_rolling_extrema_deque_and_validation_surface() {
    let data = vec![3.0_f64, 1.0, 4.0, 1.5, 5.0, 9.0];
    let mut deque = MonotonicDeque::<f64>::new(3);
    assert_eq!(deque.period(), 3);
    assert!(deque.is_empty());
    assert_eq!(deque.len(), 0);
    assert!(deque.front_index().is_none());

    deque.push_max(0, &data);
    assert!(!deque.is_empty());
    assert_eq!(deque.len(), 1);
    assert!(deque.front_index().is_some());
    let _ = deque.get_extremum(&data);
    deque.clear();
    assert!(deque.is_empty());

    let mut out = vec![0.0_f64; data.len()];
    let mut out2 = vec![0.0_f64; data.len()];
    assert!(rolling_max(&data, 0).is_err());
    assert!(rolling_max(&[] as &[f64], 3).is_err());
    assert!(rolling_max(&data[..2], 3).is_err());
    assert!(rolling_min(&data, 0).is_err());
    assert!(rolling_min(&[] as &[f64], 3).is_err());
    assert!(rolling_min(&data[..2], 3).is_err());
    assert!(rolling_max_into(&data, 0, &mut out).is_err());
    assert!(rolling_max_into(&[] as &[f64], 3, &mut out).is_err());
    assert!(rolling_max_into(&data[..2], 3, &mut out).is_err());
    let mut short_out = vec![0.0_f64; data.len() - 1];
    assert!(rolling_max_into(&data, 3, &mut short_out).is_err());
    assert!(rolling_min_into(&data, 0, &mut out).is_err());
    assert!(rolling_min_into(&[] as &[f64], 3, &mut out).is_err());
    assert!(rolling_min_into(&data[..2], 3, &mut out).is_err());
    assert!(rolling_min_into(&data, 3, &mut short_out).is_err());
    assert!(rolling_extrema(&data, 0).is_err());
    assert!(rolling_extrema(&[] as &[f64], 3).is_err());
    assert!(rolling_extrema(&data[..2], 3).is_err());
    assert!(rolling_extrema(&data, 3).is_ok());
    assert!(rolling_extrema_into(&data, 0, &mut out, &mut out2).is_err());
    assert!(rolling_extrema_into(&[], 3, &mut out, &mut out2).is_err());
    assert!(rolling_extrema_into(&data[..2], 3, &mut out, &mut out2).is_err());

    let mut short_max = vec![0.0_f64; data.len() - 1];
    assert!(rolling_extrema_into(&data, 3, &mut short_max, &mut out2).is_err());
    let mut short_min = vec![0.0_f64; data.len() - 1];
    assert!(rolling_extrema_into(&data, 3, &mut out, &mut short_min).is_err());

    assert!(rolling_max_nan_propagating(&data, 0).is_err());
    assert!(rolling_max_nan_propagating(&[] as &[f64], 3).is_err());
    assert!(rolling_max_nan_propagating(&data[..2], 3).is_err());
    assert!(rolling_min_nan_propagating(&data, 0).is_err());
    assert!(rolling_min_nan_propagating(&[] as &[f64], 3).is_err());
    assert!(rolling_min_nan_propagating(&data[..2], 3).is_err());
}

#[test]
fn coverage_rolling_extrema_vhgw_stochastic_fast_full_paths() {
    let high = vec![11.0_f64, 12.0, 13.0, 12.5, 14.0, 15.0, 16.0, 15.5];
    let low = vec![1.0_f64, 2.0, 3.0, 2.5, 4.0, 5.0, 6.0, 5.5];
    let close = vec![6.0_f64, 7.0, 8.0, 7.5, 9.0, 10.0, 11.0, 10.5];
    let mut k = vec![0.0_f64; close.len()];
    let mut d = vec![0.0_f64; close.len()];

    assert!(compute_stochastic_fast_vhgw_f64(&high, &low, &close, 3, 2, &mut k, &mut d).is_ok());
    assert!(k[0].is_nan());
    assert!(d[0].is_nan());

    let high32: Vec<f32> = high.iter().map(|v| *v as f32).collect();
    let low32: Vec<f32> = low.iter().map(|v| *v as f32).collect();
    let close32: Vec<f32> = close.iter().map(|v| *v as f32).collect();
    let mut k32 = vec![0.0_f32; close32.len()];
    let mut d32 = vec![0.0_f32; close32.len()];
    assert!(
        compute_stochastic_fast_vhgw_f32(&high32, &low32, &close32, 3, 2, &mut k32, &mut d32)
            .is_ok()
    );

    assert!(compute_stochastic_full_vhgw_f64(&high, &low, &close, 3, 2, 2, &mut k, &mut d).is_ok());
    assert!(
        compute_stochastic_full_vhgw_f32(&high32, &low32, &close32, 3, 2, 2, &mut k32, &mut d32)
            .is_ok()
    );

    let short = &close[..2];
    let mut small_k = vec![0.0_f64; 2];
    let mut small_d = vec![0.0_f64; 2];
    assert!(
        compute_stochastic_fast_vhgw_f64(short, short, short, 3, 2, &mut small_k, &mut small_d)
            .is_err()
    );
    let short32 = vec![1.0_f32, 2.0_f32];
    let mut small_k32 = vec![0.0_f32; 2];
    let mut small_d32 = vec![0.0_f32; 2];
    assert!(
        compute_stochastic_fast_vhgw_f32(
            &short32,
            &short32,
            &short32,
            3,
            2,
            &mut small_k32,
            &mut small_d32
        )
        .is_err()
    );
    assert!(
        compute_stochastic_full_vhgw_f64(short, short, short, 3, 2, 2, &mut small_k, &mut small_d)
            .is_err()
    );
    assert!(
        compute_stochastic_full_vhgw_f32(
            &short32,
            &short32,
            &short32,
            3,
            2,
            2,
            &mut small_k32,
            &mut small_d32
        )
        .is_err()
    );
}

#[test]
fn coverage_rolling_extrema_vhgw_flat_range_branches() {
    let high = vec![10.0_f64; 8];
    let low = vec![10.0_f64; 8];
    let close = vec![10.0_f64; 8];
    let mut k = vec![0.0_f64; close.len()];
    let mut d = vec![0.0_f64; close.len()];
    assert!(compute_stochastic_fast_vhgw_f64(&high, &low, &close, 3, 2, &mut k, &mut d).is_ok());
    assert!(k.iter().skip(2).all(|v| approx_eq(*v, 50.0, 1e-12)));

    let high32 = vec![10.0_f32; 8];
    let low32 = vec![10.0_f32; 8];
    let close32 = vec![10.0_f32; 8];
    let mut k32 = vec![0.0_f32; close32.len()];
    let mut d32 = vec![0.0_f32; close32.len()];
    assert!(
        compute_stochastic_fast_vhgw_f32(&high32, &low32, &close32, 3, 2, &mut k32, &mut d32)
            .is_ok()
    );
    assert!(k32.iter().skip(2).all(|v| approx_eq(*v as f64, 50.0, 1e-6)));

    let mut kf = vec![0.0_f64; close.len()];
    let mut df = vec![0.0_f64; close.len()];
    assert!(
        compute_stochastic_full_vhgw_f64(&high, &low, &close, 3, 2, 2, &mut kf, &mut df).is_ok()
    );
    assert!(kf.iter().skip(3).all(|v| approx_eq(*v, 50.0, 1e-12)));

    let mut kf32 = vec![0.0_f32; close32.len()];
    let mut df32 = vec![0.0_f32; close32.len()];
    assert!(
        compute_stochastic_full_vhgw_f32(&high32, &low32, &close32, 3, 2, 2, &mut kf32, &mut df32)
            .is_ok()
    );
    assert!(
        kf32.iter()
            .skip(3)
            .all(|v| approx_eq(*v as f64, 50.0, 1e-6))
    );
}

#[test]
fn coverage_statistics_into_matrix_f32() {
    let data: Vec<f32> = (1..=30).map(|v| v as f32).collect();
    let x: Vec<f32> = data.clone();
    let y: Vec<f32> = x.iter().map(|v| 1.5 * *v + 2.0).collect();
    let period = 7;

    let mut out = vec![0.0_f32; data.len()];

    assert!(var_into(&data, period, &mut out).is_ok());
    assert!(stddev_into(&data, period, &mut out).is_ok());
    assert!(skew_into(&data, period, &mut out).is_ok());
    assert!(kurt_into(&data, period, &mut out).is_ok());
    assert!(zscore_into(&data, period, &mut out).is_ok());
    assert!(mad_into(&data, period, &mut out).is_ok());
    assert!(sem_into(&data, period, &mut out).is_ok());

    assert!(cov_into(&x, &y, period, &mut out).is_ok());
    assert!(correl_into(&x, &y, period, &mut out).is_ok());
    assert!(beta_into(&x, &y, period, &mut out).is_ok());

    assert!(linearreg_into(&data, period, &mut out).is_ok());
    assert!(linearreg_slope_into(&data, period, &mut out).is_ok());
    assert!(linearreg_intercept_into(&data, period, &mut out).is_ok());
    assert!(linearreg_angle_into(&data, period, &mut out).is_ok());
    assert!(tsf_into(&data, period, &mut out).is_ok());
}

#[test]
fn coverage_statistics_period_one_linearreg_branches() {
    let data = vec![2.0_f64, 4.0, 6.0, 8.0];
    let mut out = vec![0.0_f64; data.len()];

    assert!(linearreg_into(&data, 1, &mut out).is_ok());

    assert!(linearreg_slope_into(&data, 1, &mut out).is_ok());

    assert!(linearreg_intercept_into(&data, 1, &mut out).is_ok());

    assert!(linearreg_angle_into(&data, 1, &mut out).is_ok());

    assert!(tsf_into(&data, 1, &mut out).is_ok());
}

#[test]
fn coverage_statistics_error_surface_expansion() {
    let data = vec![1.0_f64, 2.0, 3.0];
    let pair = vec![2.0_f64, 3.0, 4.0];
    let mut out = vec![0.0_f64; data.len()];

    assert!(var(&data, 0).is_err());
    assert!(stddev(&data, 0).is_err());
    assert!(skew(&data, 0).is_err());
    assert!(kurt(&data, 0).is_err());
    assert!(zscore(&data, 0).is_err());
    assert!(mad(&data, 0).is_err());
    assert!(sem(&data, 0).is_err());
    assert!(linearreg(&data, 0).is_err());
    assert!(linearreg_slope(&data, 0).is_err());
    assert!(linearreg_intercept(&data, 0).is_err());
    assert!(linearreg_angle(&data, 0).is_err());
    assert!(tsf(&data, 0).is_err());

    assert!(cov(&data, &pair, 0).is_err());
    assert!(correl(&data, &pair, 0).is_err());
    assert!(beta(&data, &pair, 0).is_err());

    assert!(var_into(&data, 4, &mut out).is_err());
    assert!(cov_into(&data, &pair, 4, &mut out).is_err());
    assert!(correl_into(&data, &pair, 4, &mut out).is_err());
    assert!(beta_into(&data, &pair, 4, &mut out).is_err());
}

#[test]
fn coverage_stochastic_dispatch_and_f32_paths() {
    let high = vec![11.0_f64, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
    let low = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let close = vec![6.0_f64, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0];

    assert!(stochastic(&high, &low, &close, 5, 3, 1).is_ok());
    assert!(stochastic(&high, &low, &close, 5, 3, 2).is_ok());
    assert!(stochastic_full(&high, &low, &close, 5, 2, 3).is_ok());
    assert!(stochastic_fast(&high, &low, &close, 5, 3).is_ok());
    assert!(stochastic_slow(&high, &low, &close, 5, 3).is_ok());

    let high32: Vec<f32> = high.iter().map(|v| *v as f32).collect();
    let low32: Vec<f32> = low.iter().map(|v| *v as f32).collect();
    let close32: Vec<f32> = close.iter().map(|v| *v as f32).collect();

    assert!(stochastic(&high32, &low32, &close32, 5, 3, 1).is_ok());
    assert!(stochastic(&high32, &low32, &close32, 5, 3, 2).is_ok());
    assert!(stochastic_full(&high32, &low32, &close32, 5, 2, 3).is_ok());
    assert!(stochastic_fast(&high32, &low32, &close32, 5, 3).is_ok());
    assert!(stochastic_slow(&high32, &low32, &close32, 5, 3).is_ok());
}

#[test]
fn coverage_stochastic_nan_and_invalid_surfaces() {
    let high = vec![10.0_f64, 11.0, f64::NAN, 13.0, 14.0, 15.0, 16.0];
    let low = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let close = vec![5.0_f64, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0];

    assert!(stochastic(&high, &low, &close, 3, 2, 1).is_ok());
    assert!(stochastic_fast(&high, &low, &close, 3, 2).is_ok());
    assert!(stochastic_full(&high, &low, &close, 3, 2, 2).is_ok());

    assert!(stochastic(&high, &low, &close, 3, 0, 1).is_err());
    assert!(stochastic_full(&high, &low, &close, 3, 0, 2).is_err());
}

#[test]
fn coverage_trima_core_and_error_paths() {
    let data: Vec<f64> = (1..=32).map(f64::from).collect();
    let period = 7;
    let lookback = period - 1;

    let out = trima(&data, period).expect("trima should succeed");
    assert_eq!(out.len(), data.len());
    assert!(out[..lookback].iter().all(|v| v.is_nan()));
    assert!(out[lookback..].iter().all(|v| v.is_finite()));

    let mut into_out = vec![0.0_f64; data.len()];
    assert!(trima_into(&data, period, &mut into_out).is_ok());
    for i in lookback..data.len() {
        assert!(approx_eq(out[i], into_out[i], 1e-12));
    }

    let mut too_small = vec![0.0_f64; data.len() - 1];
    assert!(trima_into(&data, period, &mut too_small).is_err());
    assert!(trima(&data, 0).is_err());
    assert!(trima(&data[..3], 7).is_err());
}

#[test]
fn coverage_trima_period_one_f32_and_non_finite_paths() {
    let data = vec![1.0_f32, 2.0, 3.0, f32::NAN, 5.0, 6.0];
    let out = trima(&data, 1).expect("trima period 1 should succeed");
    assert_eq!(out.len(), data.len());

    let mut into_out = vec![0.0_f32; data.len()];
    assert!(trima_into(&data, 1, &mut into_out).is_ok());
    assert_eq!(out.len(), into_out.len());
}

#[test]
fn coverage_wma_core_and_error_paths() {
    let data: Vec<f64> = (1..=40).map(f64::from).collect();
    let period = 9;
    let lookback = period - 1;

    let out = wma(&data, period).expect("wma should succeed");
    assert_eq!(out.len(), data.len());
    assert!(out[..lookback].iter().all(|v| v.is_nan()));
    assert!(out[lookback..].iter().all(|v| v.is_finite()));

    let mut into_out = vec![0.0_f64; data.len()];
    assert!(wma_into(&data, period, &mut into_out).is_ok());
    for i in lookback..data.len() {
        assert!(approx_eq(out[i], into_out[i], 1e-12));
    }

    let mut too_small = vec![0.0_f64; data.len() - 1];
    assert!(wma_into(&data, period, &mut too_small).is_err());
    assert!(wma(&data, 0).is_err());
    assert!(wma(&data[..5], 9).is_err());
}

#[test]
fn coverage_wma_period_one_f32_and_non_finite_paths() {
    let data = vec![2.0_f32, 4.0, f32::INFINITY, 8.0, 10.0];
    let out = wma(&data, 1).expect("wma period 1 should succeed");
    assert_eq!(out.len(), data.len());

    let mut into_out = vec![0.0_f32; data.len()];
    assert!(wma_into(&data, 1, &mut into_out).is_ok());
    assert_eq!(out.len(), into_out.len());
}

fn make_candlestick_ohlc(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);

    for i in 0..len {
        let base = 100.0 + (i as f64 * 0.35);
        let drift = if i % 2 == 0 { 0.8 } else { -0.6 };
        let o = base + drift;
        let c = base - drift * 0.7;
        let h = o.max(c) + 1.2 + (i % 3) as f64 * 0.1;
        let l = o.min(c) - 1.1 - (i % 4) as f64 * 0.1;
        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);
    }

    (open, high, low, close)
}

macro_rules! call_candlestick_non_into {
    ($open:expr, $high:expr, $low:expr, $close:expr, $($func:ident),+ $(,)?) => {{
        $(
            let out = cdl::$func($open, $high, $low, $close).expect(stringify!($func));
            assert_eq!(out.len(), $open.len());
        )+
    }};
}

macro_rules! call_candlestick_into {
    ($open:expr, $high:expr, $low:expr, $close:expr, $out:expr, $($func:ident),+ $(,)?) => {{
        $(
            cdl::$func($open, $high, $low, $close, $out).expect(stringify!($func));
        )+
    }};
}

macro_rules! call_candlestick_mismatch_errors {
    ($open:expr, $high:expr, $low:expr, $short_close:expr, $($func:ident),+ $(,)?) => {{
        $(
            assert!(cdl::$func($open, $high, $low, $short_close).is_err());
        )+
    }};
}

macro_rules! call_candlestick_non_into_discard {
    ($open:expr, $high:expr, $low:expr, $close:expr, $($func:ident),+ $(,)?) => {{
        $(
            let _ = cdl::$func($open, $high, $low, $close);
        )+
    }};
}

macro_rules! call_candlestick_into_expect_err {
    ($open:expr, $high:expr, $low:expr, $close:expr, $out:expr, $($func:ident),+ $(,)?) => {{
        $(
            assert!(cdl::$func($open, $high, $low, $close, $out).is_err());
        )+
    }};
}

#[test]
fn coverage_candlestick_all_patterns_smoke() {
    let (open, high, low, close) = make_candlestick_ohlc(96);
    call_candlestick_non_into!(
        &open,
        &high,
        &low,
        &close,
        cdl_2crows,
        cdl_3black_crows,
        cdl_3inside,
        cdl_3line_strike,
        cdl_3outside,
        cdl_3stars_in_south,
        cdl_3white_soldiers,
        cdl_abandoned_baby,
        cdl_advance_block,
        cdl_belt_hold,
        cdl_breakaway,
        cdl_closing_marubozu,
        cdl_concealing_baby_swallow,
        cdl_counter_attack,
        cdl_dark_cloud_cover,
        cdl_doji,
        cdl_doji_star,
        cdl_dragonfly_doji,
        cdl_engulfing,
        cdl_evening_doji_star,
        cdl_evening_star,
        cdl_gap_side_side_white,
        cdl_gravestone_doji,
        cdl_hammer,
        cdl_hanging_man,
        cdl_harami,
        cdl_harami_cross,
        cdl_high_wave,
        cdl_hikkake,
        cdl_hikkake_mod,
        cdl_homing_pigeon,
        cdl_identical_3crows,
        cdl_in_neck,
        cdl_inverted_hammer,
        cdl_kicking,
        cdl_kicking_by_length,
        cdl_ladder_bottom,
        cdl_long_line,
        cdl_longleg_doji,
        cdl_marubozu,
        cdl_mat_hold,
        cdl_matching_low,
        cdl_morning_doji_star,
        cdl_morning_star,
        cdl_on_neck,
        cdl_piercing,
        cdl_rickshaw_man,
        cdl_rise_fall_3methods,
        cdl_separating_lines,
        cdl_shooting_star,
        cdl_short_line,
        cdl_spinning_top,
        cdl_stalled_pattern,
        cdl_stick_sandwich,
        cdl_takuri,
        cdl_tasuki_gap,
        cdl_thrusting,
        cdl_tristar,
        cdl_unique_3river,
        cdl_upside_gap_2crows,
        cdl_xside_gap_3methods
    );
}

#[test]
fn coverage_candlestick_all_into_and_buffer_paths() {
    let (open, high, low, close) = make_candlestick_ohlc(96);
    let mut out = vec![0_i32; open.len()];
    call_candlestick_into!(
        &open,
        &high,
        &low,
        &close,
        &mut out,
        cdl_2crows_into,
        cdl_3black_crows_into,
        cdl_3inside_into,
        cdl_3line_strike_into,
        cdl_3outside_into,
        cdl_3stars_in_south_into,
        cdl_3white_soldiers_into,
        cdl_abandoned_baby_into,
        cdl_advance_block_into,
        cdl_belt_hold_into,
        cdl_breakaway_into,
        cdl_closing_marubozu_into,
        cdl_concealing_baby_swallow_into,
        cdl_counter_attack_into,
        cdl_dark_cloud_cover_into,
        cdl_doji_into,
        cdl_doji_star_into,
        cdl_dragonfly_doji_into,
        cdl_engulfing_into,
        cdl_evening_doji_star_into,
        cdl_evening_star_into,
        cdl_gap_side_side_white_into,
        cdl_gravestone_doji_into,
        cdl_hammer_into,
        cdl_hanging_man_into,
        cdl_harami_cross_into,
        cdl_harami_into,
        cdl_high_wave_into,
        cdl_hikkake_into,
        cdl_hikkake_mod_into,
        cdl_homing_pigeon_into,
        cdl_identical_3crows_into,
        cdl_in_neck_into,
        cdl_inverted_hammer_into,
        cdl_kicking_by_length_into,
        cdl_kicking_into,
        cdl_ladder_bottom_into,
        cdl_long_line_into,
        cdl_longleg_doji_into,
        cdl_marubozu_into,
        cdl_mat_hold_into,
        cdl_matching_low_into,
        cdl_morning_doji_star_into,
        cdl_morning_star_into,
        cdl_on_neck_into,
        cdl_piercing_into,
        cdl_rickshaw_man_into,
        cdl_rise_fall_3methods_into,
        cdl_separating_lines_into,
        cdl_shooting_star_into,
        cdl_short_line_into,
        cdl_spinning_top_into,
        cdl_stalled_pattern_into,
        cdl_stick_sandwich_into,
        cdl_takuri_into,
        cdl_tasuki_gap_into,
        cdl_thrusting_into,
        cdl_tristar_into,
        cdl_unique_3river_into,
        cdl_upside_gap_2crows_into,
        cdl_xside_gap_3methods_into
    );

    let mut too_small = vec![0_i32; open.len() - 1];
    assert!(cdl::cdl_doji_into(&open, &high, &low, &close, &mut too_small).is_err());
    assert!(cdl::cdl_engulfing_into(&open, &high, &low, &close, &mut too_small).is_err());
    assert!(cdl::cdl_morning_star_into(&open, &high, &low, &close, &mut too_small).is_err());
}

#[test]
fn coverage_candlestick_length_mismatch_paths() {
    let (open, high, low, close) = make_candlestick_ohlc(32);
    let short_close = &close[..close.len() - 1];

    call_candlestick_mismatch_errors!(
        &open,
        &high,
        &low,
        short_close,
        cdl_2crows,
        cdl_3black_crows,
        cdl_3inside,
        cdl_3line_strike,
        cdl_3outside,
        cdl_3stars_in_south,
        cdl_3white_soldiers,
        cdl_abandoned_baby,
        cdl_advance_block,
        cdl_belt_hold,
        cdl_breakaway,
        cdl_closing_marubozu,
        cdl_concealing_baby_swallow,
        cdl_counter_attack,
        cdl_dark_cloud_cover,
        cdl_doji,
        cdl_doji_star,
        cdl_dragonfly_doji,
        cdl_engulfing,
        cdl_evening_doji_star,
        cdl_evening_star,
        cdl_gap_side_side_white,
        cdl_gravestone_doji,
        cdl_hammer,
        cdl_hanging_man,
        cdl_harami,
        cdl_harami_cross,
        cdl_high_wave,
        cdl_hikkake,
        cdl_hikkake_mod,
        cdl_homing_pigeon,
        cdl_identical_3crows,
        cdl_in_neck,
        cdl_inverted_hammer,
        cdl_kicking,
        cdl_kicking_by_length,
        cdl_ladder_bottom,
        cdl_long_line,
        cdl_longleg_doji,
        cdl_marubozu,
        cdl_mat_hold,
        cdl_matching_low,
        cdl_morning_doji_star,
        cdl_morning_star,
        cdl_on_neck,
        cdl_piercing,
        cdl_rickshaw_man,
        cdl_rise_fall_3methods,
        cdl_separating_lines,
        cdl_shooting_star,
        cdl_short_line,
        cdl_spinning_top,
        cdl_stalled_pattern,
        cdl_stick_sandwich,
        cdl_takuri,
        cdl_tasuki_gap,
        cdl_thrusting,
        cdl_tristar,
        cdl_unique_3river,
        cdl_upside_gap_2crows,
        cdl_xside_gap_3methods
    );
}

#[test]
fn coverage_candlestick_small_input_sweep() {
    let (open, high, low, close) = make_candlestick_ohlc(2);
    call_candlestick_non_into_discard!(
        &open,
        &high,
        &low,
        &close,
        cdl_2crows,
        cdl_3black_crows,
        cdl_3inside,
        cdl_3line_strike,
        cdl_3outside,
        cdl_3stars_in_south,
        cdl_3white_soldiers,
        cdl_abandoned_baby,
        cdl_advance_block,
        cdl_belt_hold,
        cdl_breakaway,
        cdl_closing_marubozu,
        cdl_concealing_baby_swallow,
        cdl_counter_attack,
        cdl_dark_cloud_cover,
        cdl_doji,
        cdl_doji_star,
        cdl_dragonfly_doji,
        cdl_engulfing,
        cdl_evening_doji_star,
        cdl_evening_star,
        cdl_gap_side_side_white,
        cdl_gravestone_doji,
        cdl_hammer,
        cdl_hanging_man,
        cdl_harami,
        cdl_harami_cross,
        cdl_high_wave,
        cdl_hikkake,
        cdl_hikkake_mod,
        cdl_homing_pigeon,
        cdl_identical_3crows,
        cdl_in_neck,
        cdl_inverted_hammer,
        cdl_kicking,
        cdl_kicking_by_length,
        cdl_ladder_bottom,
        cdl_long_line,
        cdl_longleg_doji,
        cdl_marubozu,
        cdl_mat_hold,
        cdl_matching_low,
        cdl_morning_doji_star,
        cdl_morning_star,
        cdl_on_neck,
        cdl_piercing,
        cdl_rickshaw_man,
        cdl_rise_fall_3methods,
        cdl_separating_lines,
        cdl_shooting_star,
        cdl_short_line,
        cdl_spinning_top,
        cdl_stalled_pattern,
        cdl_stick_sandwich,
        cdl_takuri,
        cdl_tasuki_gap,
        cdl_thrusting,
        cdl_tristar,
        cdl_unique_3river,
        cdl_upside_gap_2crows,
        cdl_xside_gap_3methods
    );
}

#[test]
fn coverage_candlestick_small_buffer_sweep() {
    let (open, high, low, close) = make_candlestick_ohlc(96);
    let mut too_small = vec![0_i32; open.len() - 1];
    call_candlestick_into_expect_err!(
        &open,
        &high,
        &low,
        &close,
        &mut too_small,
        cdl_2crows_into,
        cdl_3black_crows_into,
        cdl_3inside_into,
        cdl_3line_strike_into,
        cdl_3outside_into,
        cdl_3stars_in_south_into,
        cdl_3white_soldiers_into,
        cdl_abandoned_baby_into,
        cdl_advance_block_into,
        cdl_belt_hold_into,
        cdl_breakaway_into,
        cdl_closing_marubozu_into,
        cdl_concealing_baby_swallow_into,
        cdl_counter_attack_into,
        cdl_dark_cloud_cover_into,
        cdl_doji_into,
        cdl_doji_star_into,
        cdl_dragonfly_doji_into,
        cdl_engulfing_into,
        cdl_evening_doji_star_into,
        cdl_evening_star_into,
        cdl_gap_side_side_white_into,
        cdl_gravestone_doji_into,
        cdl_hammer_into,
        cdl_hanging_man_into,
        cdl_harami_cross_into,
        cdl_harami_into,
        cdl_high_wave_into,
        cdl_hikkake_into,
        cdl_hikkake_mod_into,
        cdl_homing_pigeon_into,
        cdl_identical_3crows_into,
        cdl_in_neck_into,
        cdl_inverted_hammer_into,
        cdl_kicking_by_length_into,
        cdl_kicking_into,
        cdl_ladder_bottom_into,
        cdl_long_line_into,
        cdl_longleg_doji_into,
        cdl_marubozu_into,
        cdl_mat_hold_into,
        cdl_matching_low_into,
        cdl_morning_doji_star_into,
        cdl_morning_star_into,
        cdl_on_neck_into,
        cdl_piercing_into,
        cdl_rickshaw_man_into,
        cdl_rise_fall_3methods_into,
        cdl_separating_lines_into,
        cdl_shooting_star_into,
        cdl_short_line_into,
        cdl_spinning_top_into,
        cdl_stalled_pattern_into,
        cdl_stick_sandwich_into,
        cdl_takuri_into,
        cdl_tasuki_gap_into,
        cdl_thrusting_into,
        cdl_tristar_into,
        cdl_unique_3river_into,
        cdl_upside_gap_2crows_into,
        cdl_xside_gap_3methods_into
    );
}

#[test]
fn coverage_candlestick_core_settings_and_constants() {
    let settings = cdl::core::CandleSettings::default();
    assert!(approx_eq(settings.body_near, 0.05, 1e-12));
    assert!(approx_eq(settings.body_long, 1.0, 1e-12));
    assert!(approx_eq(settings.body_very_long, 3.0, 1e-12));
    assert!(approx_eq(settings.body_doji, 0.1, 1e-12));
    assert!(approx_eq(settings.shadow_very_short, 0.1, 1e-12));
    assert!(approx_eq(settings.equal, 0.05, 1e-12));

    assert_eq!(cdl::core::TREND_LOOKBACK, 10);
    assert_eq!(cdl::core::AVG_LOOKBACK, 10);
    assert_eq!(cdl::core::PATTERN_BULLISH, 100);
    assert_eq!(cdl::core::PATTERN_BEARISH, -100);
    assert_eq!(cdl::core::PATTERN_BULLISH_STRONG, 200);
    assert_eq!(cdl::core::PATTERN_BEARISH_STRONG, -200);
    assert_eq!(cdl::core::PATTERN_NONE, 0);

    let f = cdl::core::f64_to_t::<f64>(1.25);
    assert!(approx_eq(f, 1.25, 1e-12));
    let u = cdl::core::usize_to_t::<f64>(7);
    assert!(approx_eq(u, 7.0, 1e-12));
}

#[test]
fn coverage_candlestick_core_average_guard_paths() {
    let open = [10.0_f64, 11.0, 12.0, 13.0];
    let high = [11.5_f64, 12.5, 13.5, 14.5];
    let low = [9.5_f64, 10.5, 11.5, 12.5];
    let close = [10.8_f64, 10.9, 12.8, 12.9];

    assert!(cdl::core::average_body(&open, &close, 2, 0).is_nan());
    assert!(cdl::core::average_body(&open, &close, 2, 3).is_nan());
    assert!(cdl::core::average_range(&high, &low, 2, 0).is_nan());
    assert!(cdl::core::average_range(&high, &low, 2, 3).is_nan());
    assert!(cdl::core::average_upper_shadow(&open, &high, &close, 2, 0).is_nan());
    assert!(cdl::core::average_upper_shadow(&open, &high, &close, 2, 3).is_nan());
    assert!(cdl::core::average_lower_shadow(&open, &low, &close, 2, 0).is_nan());
    assert!(cdl::core::average_lower_shadow(&open, &low, &close, 2, 3).is_nan());

    assert!(cdl::core::average_body(&open, &close, 4, 2).is_finite());
    assert!(cdl::core::average_range(&high, &low, 4, 2).is_finite());
    assert!(cdl::core::average_upper_shadow(&open, &high, &close, 4, 2).is_finite());
    assert!(cdl::core::average_lower_shadow(&open, &low, &close, 4, 2).is_finite());
}

#[test]
fn coverage_candlestick_core_classifiers_and_gap_paths() {
    assert!(cdl::core::is_doji(10.0_f64, 12.0, 8.0, 10.1, 0.1));
    assert!(!cdl::core::is_doji(10.0_f64, 12.0, 8.0, 11.5, 0.1));

    assert!(cdl::core::is_doji(10.0_f64, 10.0, 10.0, 10.0, 0.1));
    assert!(!cdl::core::is_doji(10.0_f64, 10.0, 10.0, 10.5, 0.1));

    assert!(cdl::core::is_long_body(10.0_f64, 12.0, 1.0, 1.0));
    assert!(!cdl::core::is_long_body(10.0_f64, 10.5, 1.0, 1.0));
    assert!(cdl::core::is_short_body(10.0_f64, 10.2, 1.0, 1.0));
    assert!(!cdl::core::is_short_body(10.0_f64, 12.0, 1.0, 1.0));

    assert!(cdl::core::is_long_upper_shadow(
        10.0_f64, 14.0, 11.0, 1.0, 1.0
    ));
    assert!(!cdl::core::is_long_upper_shadow(
        10.0_f64, 11.2, 11.0, 1.0, 1.0
    ));
    assert!(cdl::core::is_long_lower_shadow(
        10.0_f64, 6.0, 11.0, 1.0, 1.0
    ));
    assert!(!cdl::core::is_long_lower_shadow(
        10.0_f64, 9.8, 11.0, 1.0, 1.0
    ));

    assert!(cdl::core::is_very_short_upper_shadow(
        10.0_f64, 10.2, 10.1, 2.0, 0.2
    ));
    assert!(!cdl::core::is_very_short_upper_shadow(
        10.0_f64, 11.5, 10.1, 2.0, 0.2
    ));
    assert!(cdl::core::is_very_short_lower_shadow(
        10.1_f64, 10.0, 10.2, 2.0, 0.2
    ));
    assert!(!cdl::core::is_very_short_lower_shadow(
        10.1_f64, 8.0, 10.2, 2.0, 0.2
    ));

    assert!(cdl::core::gap_up(10.0_f64, 10.1));
    assert!(!cdl::core::gap_up(10.0_f64, 10.0));
    assert!(cdl::core::gap_down(10.0_f64, 9.9));
    assert!(!cdl::core::gap_down(10.0_f64, 10.0));

    assert!(cdl::core::real_body_gap_up(10.0_f64, 11.0, 12.0, 13.0));
    assert!(!cdl::core::real_body_gap_up(10.0_f64, 11.0, 10.5, 12.0));
    assert!(cdl::core::real_body_gap_down(12.0_f64, 11.0, 9.5, 10.0));
    assert!(!cdl::core::real_body_gap_down(12.0_f64, 11.0, 11.0, 10.2));
}

#[test]
fn coverage_candlestick_core_trend_and_misc_paths() {
    let close = [10.0_f64, 11.0, 12.0, 13.0, 14.0, 16.0];
    assert!(!cdl::core::is_uptrend(&close, 2, 3));
    assert!(!cdl::core::is_downtrend(&close, 2, 3));
    assert!(cdl::core::is_uptrend(&close, 5, 3));
    assert!(!cdl::core::is_downtrend(&close, 5, 3));

    let close_down = [16.0_f64, 15.0, 14.0, 13.0, 12.0, 10.0];
    assert!(cdl::core::is_downtrend(&close_down, 5, 3));
    assert!(!cdl::core::is_uptrend(&close_down, 5, 3));

    assert!(cdl::core::approx_equal(10.0_f64, 10.04, 0.05));
    assert!(!cdl::core::approx_equal(10.0_f64, 10.2, 0.05));

    assert!(approx_eq(
        cdl::core::body_midpoint(10.0_f64, 12.0),
        11.0,
        1e-12
    ));
    assert!(approx_eq(
        cdl::core::candle_range(12.5_f64, 9.5),
        3.0,
        1e-12
    ));
}

#[test]
fn coverage_stochastic_config_surface_and_compute_into_paths() {
    let high = vec![11.0_f64, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
    let low = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let close = vec![6.0_f64, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0];

    let cfg = Stochastic::new()
        .with_k_period(5)
        .with_d_period(3)
        .with_k_slowing(2);

    assert_eq!(cfg.k_period(), 5);
    assert_eq!(cfg.d_period(), 3);
    assert_eq!(cfg.k_slowing(), 2);
    assert_eq!(cfg.k_lookback(), stochastic_k_lookback(5));
    assert_eq!(cfg.min_len(), stochastic_min_len(5, 3));
    assert!(cfg.d_lookback() >= cfg.k_lookback());

    let out = cfg
        .compute(&high, &low, &close)
        .expect("compute should succeed");
    assert_eq!(out.k.len(), high.len());
    assert_eq!(out.d.len(), high.len());

    let mut out_buf = StochasticOutput {
        k: vec![0.0_f64; high.len()],
        d: vec![0.0_f64; high.len()],
    };
    let (k_valid, d_valid) = cfg
        .compute_into(&high, &low, &close, &mut out_buf)
        .expect("compute_into should succeed");
    assert!(k_valid > 0);
    assert!(d_valid > 0);

    let mut too_small = StochasticOutput {
        k: vec![0.0_f64; high.len() - 1],
        d: vec![0.0_f64; high.len() - 1],
    };
    assert!(
        cfg.compute_into(&high, &low, &close, &mut too_small)
            .is_err()
    );

    let fast_cfg = Stochastic::fast(5, 3);
    let slow_cfg = Stochastic::slow(5, 3);
    assert_eq!(fast_cfg.k_slowing(), 1);
    assert_eq!(slow_cfg.k_slowing(), 3);
    assert!(fast_cfg.compute(&high, &low, &close).is_ok());
    assert!(slow_cfg.compute(&high, &low, &close).is_ok());
}

#[test]
fn coverage_stochastic_wrapper_and_into_equivalence_matrix() {
    let high = vec![
        10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5,
    ];
    let low = vec![9.0_f64, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5];
    let close = vec![
        9.5_f64, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0,
    ];
    let n = close.len();

    let fast = stochastic_fast(&high, &low, &close, 5, 3).expect("stochastic_fast should succeed");
    let slow = stochastic_slow(&high, &low, &close, 5, 3).expect("stochastic_slow should succeed");
    let full =
        stochastic_full(&high, &low, &close, 5, 3, 3).expect("stochastic_full should succeed");
    let dispatch_fast =
        stochastic(&high, &low, &close, 5, 3, 1).expect("stochastic dispatch-fast should succeed");
    let dispatch_full =
        stochastic(&high, &low, &close, 5, 3, 3).expect("stochastic dispatch-full should succeed");

    for i in 0..n {
        if fast.k[i].is_nan() {
            assert!(dispatch_fast.k[i].is_nan());
        } else {
            assert!(approx_eq(fast.k[i], dispatch_fast.k[i], 1e-12));
        }
        if fast.d[i].is_nan() {
            assert!(dispatch_fast.d[i].is_nan());
        } else {
            assert!(approx_eq(fast.d[i], dispatch_fast.d[i], 1e-12));
        }

        if full.k[i].is_nan() {
            assert!(slow.k[i].is_nan());
            assert!(dispatch_full.k[i].is_nan());
        } else {
            assert!(approx_eq(full.k[i], slow.k[i], 1e-12));
            assert!(approx_eq(full.k[i], dispatch_full.k[i], 1e-12));
        }
        if full.d[i].is_nan() {
            assert!(slow.d[i].is_nan());
            assert!(dispatch_full.d[i].is_nan());
        } else {
            assert!(approx_eq(full.d[i], slow.d[i], 1e-12));
            assert!(approx_eq(full.d[i], dispatch_full.d[i], 1e-12));
        }
    }

    let mut output = StochasticOutput {
        k: vec![0.0_f64; n],
        d: vec![0.0_f64; n],
    };

    let (k_valid_fast, d_valid_fast) =
        stochastic_fast_into(&high, &low, &close, 5, 3, &mut output).expect("fast_into ok");
    assert!(k_valid_fast > 0);
    assert!(d_valid_fast > 0);
    for i in 0..n {
        if fast.k[i].is_nan() {
            assert!(output.k[i].is_nan());
        } else {
            assert!(approx_eq(fast.k[i], output.k[i], 1e-12));
        }
        if fast.d[i].is_nan() {
            assert!(output.d[i].is_nan());
        } else {
            assert!(approx_eq(fast.d[i], output.d[i], 1e-12));
        }
    }

    let (k_valid_slow, d_valid_slow) =
        stochastic_slow_into(&high, &low, &close, 5, 3, &mut output).expect("slow_into ok");
    assert!(k_valid_slow > 0);
    assert!(d_valid_slow > 0);
    for i in 0..n {
        if slow.k[i].is_nan() {
            assert!(output.k[i].is_nan());
        } else {
            assert!(approx_eq(slow.k[i], output.k[i], 1e-12));
        }
        if slow.d[i].is_nan() {
            assert!(output.d[i].is_nan());
        } else {
            assert!(approx_eq(slow.d[i], output.d[i], 1e-12));
        }
    }

    let (k_valid_full, d_valid_full) =
        stochastic_full_into(&high, &low, &close, 5, 3, 3, &mut output).expect("full_into ok");
    assert!(k_valid_full > 0);
    assert!(d_valid_full > 0);
    for i in 0..n {
        if full.k[i].is_nan() {
            assert!(output.k[i].is_nan());
        } else {
            assert!(approx_eq(full.k[i], output.k[i], 1e-12));
        }
        if full.d[i].is_nan() {
            assert!(output.d[i].is_nan());
        } else {
            assert!(approx_eq(full.d[i], output.d[i], 1e-12));
        }
    }

    let (k_valid_dispatch_fast, d_valid_dispatch_fast) =
        stochastic_into(&high, &low, &close, 5, 3, 1, &mut output).expect("dispatch fast_into ok");
    assert!(k_valid_dispatch_fast > 0);
    assert!(d_valid_dispatch_fast > 0);

    let (k_valid_dispatch_full, d_valid_dispatch_full) =
        stochastic_into(&high, &low, &close, 5, 3, 3, &mut output).expect("dispatch full_into ok");
    assert!(k_valid_dispatch_full > 0);
    assert!(d_valid_dispatch_full > 0);
}

#[test]
fn coverage_stochastic_into_f32_and_error_matrix_expansion() {
    let high = vec![
        10.0_f32, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5,
    ];
    let low = vec![9.0_f32, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5];
    let close = vec![
        9.5_f32, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0,
    ];
    let n = close.len();

    let mut out32 = StochasticOutput {
        k: vec![0.0_f32; n],
        d: vec![0.0_f32; n],
    };
    assert!(stochastic_fast_into(&high, &low, &close, 5, 3, &mut out32).is_ok());
    assert!(stochastic_slow_into(&high, &low, &close, 5, 3, &mut out32).is_ok());
    assert!(stochastic_full_into(&high, &low, &close, 5, 3, 3, &mut out32).is_ok());
    assert!(stochastic_into(&high, &low, &close, 5, 3, 1, &mut out32).is_ok());
    assert!(stochastic_into(&high, &low, &close, 5, 3, 3, &mut out32).is_ok());

    assert!(stochastic_fast(&high, &low, &close, 5, 3).is_ok());
    assert!(stochastic_slow(&high, &low, &close, 5, 3).is_ok());
    assert!(stochastic_full(&high, &low, &close, 5, 3, 3).is_ok());
    assert!(stochastic(&high, &low, &close, 5, 3, 1).is_ok());
    assert!(stochastic(&high, &low, &close, 5, 3, 3).is_ok());

    let mut short_k = StochasticOutput {
        k: vec![0.0_f32; n - 1],
        d: vec![0.0_f32; n],
    };
    assert!(stochastic_fast_into(&high, &low, &close, 5, 3, &mut short_k).is_err());
    let mut short_d = StochasticOutput {
        k: vec![0.0_f32; n],
        d: vec![0.0_f32; n - 1],
    };
    assert!(stochastic_fast_into(&high, &low, &close, 5, 3, &mut short_d).is_err());

    assert!(stochastic_fast_into(&high[..n - 1], &low, &close, 5, 3, &mut out32).is_err());
    assert!(stochastic_slow_into(&high, &low[..n - 1], &close, 5, 3, &mut out32).is_err());
    assert!(stochastic_full_into(&high, &low, &close[..n - 1], 5, 3, 3, &mut out32).is_err());

    assert!(stochastic_fast_into(&high, &low, &close, 0, 3, &mut out32).is_err());
    assert!(stochastic_fast_into(&high, &low, &close, 5, 0, &mut out32).is_err());
    assert!(stochastic_slow_into(&high, &low, &close, 0, 3, &mut out32).is_err());
    assert!(stochastic_slow_into(&high, &low, &close, 5, 0, &mut out32).is_err());
    assert!(stochastic_full_into(&high, &low, &close, 0, 3, 3, &mut out32).is_err());
    assert!(stochastic_full_into(&high, &low, &close, 5, 0, 3, &mut out32).is_err());
    assert!(stochastic_full_into(&high, &low, &close, 5, 3, 0, &mut out32).is_err());
    assert!(stochastic_into(&high, &low, &close, 5, 0, 1, &mut out32).is_err());
    assert!(stochastic_into(&high, &low, &close, 5, 3, 0, &mut out32).is_err());

    assert!(stochastic_fast(&high, &low, &close, 0, 3).is_err());
    assert!(stochastic_fast(&high, &low, &close, 5, 0).is_err());
    assert!(stochastic_slow(&high, &low, &close, 0, 3).is_err());
    assert!(stochastic_slow(&high, &low, &close, 5, 0).is_err());
    assert!(stochastic_full(&high, &low, &close, 0, 3, 3).is_err());
    assert!(stochastic_full(&high, &low, &close, 5, 0, 3).is_err());
    assert!(stochastic_full(&high, &low, &close, 5, 3, 0).is_err());
    assert!(stochastic(&high, &low, &close, 0, 3, 1).is_err());
    assert!(stochastic(&high, &low, &close, 5, 0, 1).is_err());
    assert!(stochastic(&high, &low, &close, 5, 3, 0).is_err());
}
