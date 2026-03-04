use half::f16;
use liq_ta::batch::process_ohlc_batch;
use liq_ta::error::Error;
use liq_ta::indicators::atr::atr;
use liq_ta::indicators::demarker::{demarker, demarker_into, demarker_lookback};
use liq_ta::indicators::dss_bressert::{
    dss_bressert, dss_bressert_into, dss_bressert_lookback, dss_bressert_min_len,
};
use liq_ta::indicators::ht_phasor::{ht_phasor_into, ht_phasor_lookback, ht_phasor_min_len};
use liq_ta::indicators::ht_sine::{ht_sine_into, ht_sine_lookback, ht_sine_min_len};
use liq_ta::indicators::midpoint::midpoint_into;
use liq_ta::indicators::power::{
    bears_power, bears_power_into, bulls_power, bulls_power_into, power_lookback,
};
use liq_ta::indicators::supertrend::{supertrend, supertrend_into, supertrend_lookback};

fn sample_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    for i in 0..n {
        let base = 100.0 + i as f64 * 0.35 + ((i % 7) as f64) * 0.1;
        high.push(base + 1.0 + ((i % 3) as f64) * 0.2);
        low.push(base - 1.1 - ((i % 2) as f64) * 0.15);
        close.push(base + if i % 2 == 0 { 0.3 } else { -0.2 });
    }
    (high, low, close)
}

#[test]
fn coverage_push_batch_process_ohlc_batch_paths() {
    let (h1, l1, c1) = sample_ohlc(16);
    let (h2, l2, c2) = sample_ohlc(20);
    let datasets = vec![(h1.clone(), l1.clone(), c1.clone()), (h2, l2, c2)];
    let out =
        process_ohlc_batch(&datasets, |h, l, c| atr(h, l, c, 3)).expect("batch atr should work");
    assert_eq!(out.len(), 2);

    let failing = vec![(h1[..2].to_vec(), l1[..2].to_vec(), c1[..2].to_vec())];
    let err = process_ohlc_batch(&failing, |h, l, c| atr(h, l, c, 3))
        .expect_err("short ohlc should fail");
    assert!(matches!(err, Error::InsufficientData { .. }));
}

#[test]
fn coverage_push_power_validation_and_into_paths() {
    let (mut high, mut low, close) = sample_ohlc(40);
    let period = 7usize;

    assert!(matches!(
        bulls_power(&high, &low, &close, 0),
        Err(Error::InvalidPeriod { .. })
    ));
    assert!(matches!(
        bulls_power::<f64>(&[], &[], &[], period),
        Err(Error::EmptyInput)
    ));
    assert!(matches!(
        bears_power(&high, &low[..39], &close, period),
        Err(Error::LengthMismatch { .. })
    ));
    assert!(matches!(
        bears_power(&high[..5], &low[..5], &close[..5], period),
        Err(Error::InsufficientData { .. })
    ));

    high[9] = f64::NAN;
    low[11] = f64::INFINITY;
    let bulls = bulls_power(&high, &low, &close, period).expect("bulls power should succeed");
    let bears = bears_power(&high, &low, &close, period).expect("bears power should succeed");
    assert!(bulls[9].is_nan());
    assert!(bears[11].is_nan());

    let mut short_out = vec![0.0_f64; close.len() - 1];
    assert!(matches!(
        bulls_power_into(&high, &low, &close, period, &mut short_out),
        Err(Error::BufferTooSmall { .. })
    ));
    assert!(matches!(
        bears_power_into(&high, &low, &close, period, &mut short_out),
        Err(Error::BufferTooSmall { .. })
    ));

    let mut bulls_out = vec![0.0_f64; close.len()];
    let mut bears_out = vec![0.0_f64; close.len()];
    let bulls_valid = bulls_power_into(&high, &low, &close, period, &mut bulls_out)
        .expect("bulls into should succeed");
    let bears_valid = bears_power_into(&high, &low, &close, period, &mut bears_out)
        .expect("bears into should succeed");
    assert_eq!(bulls_valid, close.len() - power_lookback(period));
    assert_eq!(bears_valid, close.len() - power_lookback(period));
}

#[test]
fn coverage_push_supertrend_validation_and_state_paths() {
    let (mut high, low, close) = sample_ohlc(96);
    let period = 10usize;
    let n = close.len();

    assert!(matches!(
        supertrend(&high, &low, &close, period, 0.0),
        Err(Error::LengthMismatch { .. })
    ));
    assert!(matches!(
        supertrend(&high, &low, &close, period, f64::NAN),
        Err(Error::LengthMismatch { .. })
    ));
    assert!(matches!(
        supertrend(&high[..90], &low, &close, period, 2.5),
        Err(Error::LengthMismatch { .. })
    ));

    let mut supertrend_out = vec![0.0_f64; n - 1];
    let mut upper = vec![0.0_f64; n];
    let mut lower = vec![0.0_f64; n];
    let mut trend = vec![0.0_f64; n];
    assert!(matches!(
        supertrend_into(
            &high,
            &low,
            &close,
            period,
            2.5,
            &mut supertrend_out,
            &mut upper,
            &mut lower,
            &mut trend
        ),
        Err(Error::BufferTooSmall { .. })
    ));

    supertrend_out = vec![0.0_f64; n];
    upper = vec![0.0_f64; n - 1];
    assert!(matches!(
        supertrend_into(
            &high,
            &low,
            &close,
            period,
            2.5,
            &mut supertrend_out,
            &mut upper,
            &mut lower,
            &mut trend
        ),
        Err(Error::BufferTooSmall { .. })
    ));

    upper = vec![0.0_f64; n];
    lower = vec![0.0_f64; n - 1];
    assert!(matches!(
        supertrend_into(
            &high,
            &low,
            &close,
            period,
            2.5,
            &mut supertrend_out,
            &mut upper,
            &mut lower,
            &mut trend
        ),
        Err(Error::BufferTooSmall { .. })
    ));

    lower = vec![0.0_f64; n];
    trend = vec![0.0_f64; n - 1];
    assert!(matches!(
        supertrend_into(
            &high,
            &low,
            &close,
            period,
            2.5,
            &mut supertrend_out,
            &mut upper,
            &mut lower,
            &mut trend
        ),
        Err(Error::BufferTooSmall { .. })
    ));

    let lb = supertrend_lookback(period);
    high[lb] = f64::NAN;
    trend = vec![0.0_f64; n];
    supertrend_into(
        &high,
        &low,
        &close,
        period,
        2.5,
        &mut supertrend_out,
        &mut upper,
        &mut lower,
        &mut trend,
    )
    .expect("supertrend should complete with nan propagation");
    assert!(trend.iter().any(|v| v.is_nan()));

    let (high_clean, low_clean, close_clean) = sample_ohlc(96);
    supertrend_into(
        &high_clean,
        &low_clean,
        &close_clean,
        period,
        2.5,
        &mut supertrend_out,
        &mut upper,
        &mut lower,
        &mut trend,
    )
    .expect("supertrend finite path should succeed");
    assert!(trend.iter().any(|v| v.is_finite()));
}

#[test]
fn coverage_push_dss_bressert_validation_and_nonfinite_paths() {
    let (mut high, mut low, close) = sample_ohlc(64);
    let stoch_period = 8usize;
    let ema_period = 5usize;

    assert!(matches!(
        dss_bressert(&high, &low, &close, 0, ema_period),
        Err(Error::InvalidPeriod { .. })
    ));
    assert!(matches!(
        dss_bressert(&high, &low, &close, stoch_period, 0),
        Err(Error::InvalidPeriod { .. })
    ));
    assert!(matches!(
        dss_bressert::<f64>(&[], &[], &[], stoch_period, ema_period),
        Err(Error::EmptyInput)
    ));
    assert!(matches!(
        dss_bressert(&high, &low[..63], &close, stoch_period, ema_period),
        Err(Error::LengthMismatch { .. })
    ));
    assert!(matches!(
        dss_bressert(&high[..8], &low[..8], &close[..8], stoch_period, ema_period),
        Err(Error::InsufficientData { .. })
    ));

    high[7] = f64::NAN;
    low[7] = f64::NAN;
    let out = dss_bressert(&high, &low, &close, stoch_period, ema_period)
        .expect("dss should handle non-finite");
    assert_eq!(out.len(), close.len());

    let mut short_output = vec![0.0_f64; close.len() - 1];
    assert!(matches!(
        dss_bressert_into(
            &high,
            &low,
            &close,
            stoch_period,
            ema_period,
            &mut short_output
        ),
        Err(Error::BufferTooSmall { .. })
    ));

    let mut out_into = vec![0.0_f64; close.len()];
    let valid = dss_bressert_into(&high, &low, &close, stoch_period, ema_period, &mut out_into)
        .expect("dss into should succeed");
    assert_eq!(
        valid,
        close.len() - dss_bressert_lookback(stoch_period, ema_period)
    );
    assert_eq!(
        dss_bressert_min_len(stoch_period, ema_period),
        dss_bressert_lookback(stoch_period, ema_period) + 1
    );
}

#[test]
fn coverage_push_demarker_validation_and_flat_series_path() {
    let period = 6usize;
    let (high, low, _) = sample_ohlc(40);

    assert!(matches!(
        demarker(&high, &low, 0),
        Err(Error::InvalidPeriod { .. })
    ));
    assert!(matches!(
        demarker::<f64>(&[], &[], period),
        Err(Error::EmptyInput)
    ));
    assert!(matches!(
        demarker(&high, &low[..39], period),
        Err(Error::LengthMismatch { .. })
    ));
    assert!(matches!(
        demarker(&high[..5], &low[..5], period),
        Err(Error::InsufficientData { .. })
    ));

    let flat_high = vec![10.0_f64; 32];
    let flat_low = vec![10.0_f64; 32];
    let out = demarker(&flat_high, &flat_low, period).expect("demarker on flat should succeed");
    for value in &out[demarker_lookback(period)..] {
        assert!((*value - 0.5).abs() < 1e-12);
    }

    let mut small = vec![0.0_f64; 31];
    assert!(matches!(
        demarker_into(&flat_high, &flat_low, period, &mut small),
        Err(Error::BufferTooSmall { .. })
    ));
}

#[test]
fn coverage_push_ht_phasor_into_error_and_success_paths() {
    let min_len = ht_phasor_min_len();
    assert!(matches!(
        ht_phasor_into::<f64>(&[], &mut [], &mut []),
        Err(Error::EmptyInput)
    ));

    let short = vec![1.0_f64; min_len - 1];
    let mut short_i = vec![0.0_f64; short.len()];
    let mut short_q = vec![0.0_f64; short.len()];
    assert!(matches!(
        ht_phasor_into(&short, &mut short_i, &mut short_q),
        Err(Error::InsufficientData { .. })
    ));

    let n = 160usize;
    let data: Vec<f64> = (0..n)
        .map(|i| 100.0 + (i as f64 * 0.17).sin() * 7.0)
        .collect();
    let mut inphase = vec![0.0_f64; n - 1];
    let mut quadrature = vec![0.0_f64; n];
    assert!(matches!(
        ht_phasor_into(&data, &mut inphase, &mut quadrature),
        Err(Error::BufferTooSmall { .. })
    ));

    inphase = vec![0.0_f64; n];
    ht_phasor_into(&data, &mut inphase, &mut quadrature).expect("ht phasor into should succeed");
    let lb = ht_phasor_lookback();
    assert!(inphase[..lb].iter().all(|v| v.is_nan()));
    assert!(quadrature[..lb].iter().all(|v| v.is_nan()));
    assert!(inphase[lb..].iter().any(|v| v.is_finite()));
    assert!(quadrature[lb..].iter().any(|v| v.is_finite()));
}

#[test]
fn coverage_push_ht_sine_into_error_and_success_paths() {
    let min_len = ht_sine_min_len();
    assert!(matches!(
        ht_sine_into::<f64>(&[], &mut [], &mut []),
        Err(Error::EmptyInput)
    ));

    let short = vec![1.0_f64; min_len - 1];
    let mut short_s = vec![0.0_f64; short.len()];
    let mut short_l = vec![0.0_f64; short.len()];
    assert!(matches!(
        ht_sine_into(&short, &mut short_s, &mut short_l),
        Err(Error::InsufficientData { .. })
    ));

    let n = 160usize;
    let data: Vec<f64> = (0..n)
        .map(|i| 90.0 + (i as f64 * 0.21).cos() * 8.0)
        .collect();
    let mut sine = vec![0.0_f64; n];
    let mut lead = vec![0.0_f64; n - 1];
    assert!(matches!(
        ht_sine_into(&data, &mut sine, &mut lead),
        Err(Error::BufferTooSmall { .. })
    ));

    lead = vec![0.0_f64; n];
    ht_sine_into(&data, &mut sine, &mut lead).expect("ht sine into should succeed");
    let lb = ht_sine_lookback();
    assert!(sine[..lb].iter().all(|v| v.is_nan()));
    assert!(lead[..lb].iter().all(|v| v.is_nan()));
    assert!(sine[lb..].iter().all(|v| v.is_finite()));
    assert!(lead[lb..].iter().all(|v| v.is_finite()));
}

#[test]
fn coverage_push_midpoint_dispatch_and_nonfinite_normalization_paths() {
    let data: Vec<f64> = (0..1200).map(|i| 20.0 + i as f64 * 0.1).collect();
    let mut out = vec![0.0_f64; data.len()];
    midpoint_into(&data, 5, &mut out).expect("midpoint vhgw/deque path should succeed");
    assert!(out[4].is_finite());

    let p1 = vec![1.0_f64, f64::NAN, f64::INFINITY, -3.0];
    let mut p1_out = vec![0.0_f64; p1.len()];
    midpoint_into(&p1, 1, &mut p1_out).expect("midpoint period=1 should succeed");
    assert!(p1_out[0].is_finite());
    assert!(p1_out[1].is_nan());
    assert!(p1_out[2].is_nan());
    assert!(p1_out[3].is_finite());

    let data_f16: Vec<f16> = (0..80)
        .map(|i| f16::from_f32(10.0 + i as f32 * 0.2 + ((i % 3) as f32) * 0.1))
        .collect();
    let mut out_f16 = vec![f16::from_f32(0.0); data_f16.len()];
    midpoint_into(&data_f16, 7, &mut out_f16).expect("midpoint generic f16 path should succeed");
}
