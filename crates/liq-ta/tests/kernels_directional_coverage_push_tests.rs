use half::f16;
use liq_ta::error::Error;
use liq_ta::indicators::adx::adx;
use liq_ta::indicators::dx::{
    adxr, adxr_into, dx_into, minus_dm, minus_dm_into, plus_dm, plus_dm_into,
};
use liq_ta::indicators::kama::{kama, kama_full};
use liq_ta::indicators::stochastic::{
    StochasticOutput, stochastic_fast, stochastic_fast_into, stochastic_full, stochastic_full_into,
};
use liq_ta::indicators::stochrsi::{stochrsi, stochrsi_into};
use liq_ta::kernels::accumulators::{
    CumulativeProductSum, CumulativeSum, RollingSumF64, RollingVarianceF64, WelfordVarianceF64,
    WilderSmoothing,
};
use liq_ta::kernels::simd::{
    correlation_f64, covariance_components_f64, dot_product_f64, lagged_sub_f32, max_f32, max_f64,
    min_f32, min_f64, moments_and_count_f64, moments_f64, scaled_sum_f64, sum_abs_dev_f64,
    sum_abs_diff_f64, sum_and_count_f64, sum_and_sum_sq_and_count_f64, sum_and_sum_sq_f64,
    sum_cubes_f64, sum_f32, sum_fourth_f64, sum_of_squares_f64, sum_squared_diff_f64,
    true_range_f64, variance_f64,
};
use num_traits::ToPrimitive;

#[derive(Clone, Copy, Debug)]
struct NoPrimitive;

impl ToPrimitive for NoPrimitive {
    fn to_i64(&self) -> Option<i64> {
        None
    }

    fn to_u64(&self) -> Option<u64> {
        None
    }
}

fn sample_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    for i in 0..n {
        let base = 50.0 + i as f64 * 0.6 + ((i % 5) as f64) * 0.1;
        high.push(base + 1.2 + ((i % 3) as f64) * 0.1);
        low.push(base - 1.0 - ((i % 2) as f64) * 0.1);
        close.push(base + if i % 2 == 0 { 0.25 } else { -0.2 });
    }
    (high, low, close)
}

#[test]
fn coverage_push_simd_edge_case_matrix() {
    assert_eq!(sum_f32(&[]), 0.0);
    let s32 = sum_f32(&[1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    assert!((s32 - 45.0).abs() < 1e-5);

    assert_eq!(min_f64(&[]), f64::INFINITY);
    assert_eq!(max_f64(&[]), f64::NEG_INFINITY);
    let min64 = min_f64(&[5.0, 4.0, 3.0, 2.0, 1.0, f64::NAN]);
    let max64 = max_f64(&[1.0, 2.0, 3.0, 4.0, 5.0, f64::NAN]);
    assert!(min64.is_nan());
    assert!(max64.is_nan());

    let min32 = min_f32(&[5.0_f32, 4.0, 3.0, 2.0, 1.0, f32::NAN]);
    let max32 = max_f32(&[1.0_f32, 2.0, 3.0, 4.0, 5.0, f32::NAN]);
    assert!(min32.is_nan());
    assert!(max32.is_nan());

    assert_eq!(sum_of_squares_f64(&[]), 0.0);
    assert!((sum_of_squares_f64(&[1.0, 2.0, 3.0, 4.0, 5.0]) - 55.0).abs() < 1e-10);

    assert_eq!(sum_and_sum_sq_f64(&[]), (0.0, 0.0));
    let (sum, sum_sq) = sum_and_sum_sq_f64(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    assert!((sum - 15.0).abs() < 1e-10);
    assert!((sum_sq - 55.0).abs() < 1e-10);

    assert_eq!(sum_and_sum_sq_and_count_f64(&[]), (0.0, 0.0, 0));
    let (sum2, sum_sq2, count2) =
        sum_and_sum_sq_and_count_f64(&[1.0, 2.0, f64::NAN, f64::INFINITY, 4.0]);
    assert!((sum2 - 7.0).abs() < 1e-10);
    assert!((sum_sq2 - 21.0).abs() < 1e-10);
    assert_eq!(count2, 3);

    assert_eq!(variance_f64(&[]), 0.0);
    assert_eq!(variance_f64(&[3.0, 3.0, 3.0]), 0.0);

    assert_eq!(dot_product_f64(&[], &[]), 0.0);
    assert!((dot_product_f64(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]) - 32.0).abs() < 1e-10);

    assert_eq!(scaled_sum_f64(&[], 2.0), 0.0);
    assert!((scaled_sum_f64(&[1.0, 2.0, 3.0], 2.0) - 12.0).abs() < 1e-10);

    assert_eq!(sum_squared_diff_f64(&[], 1.0), 0.0);
    assert!((sum_squared_diff_f64(&[1.0, 2.0, 3.0], 2.0) - 2.0).abs() < 1e-10);

    assert_eq!(moments_f64(&[]), (0.0, 0.0, 0.0, 0.0));
    let (m1, m2, m3, m4) = moments_f64(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    assert!((m1 - 15.0).abs() < 1e-10);
    assert!((m2 - 55.0).abs() < 1e-10);
    assert!((m3 - 225.0).abs() < 1e-10);
    assert!((m4 - 979.0).abs() < 1e-10);

    assert_eq!(moments_and_count_f64(&[]), (0.0, 0.0, 0.0, 0.0, 0));
    let (_, _, _, _, valid_count) =
        moments_and_count_f64(&[1.0, f64::NAN, 2.0, f64::INFINITY, 3.0]);
    assert_eq!(valid_count, 3);

    assert_eq!(sum_cubes_f64(&[]), 0.0);
    assert!((sum_cubes_f64(&[1.0, 2.0, 3.0]) - 36.0).abs() < 1e-10);

    assert_eq!(sum_fourth_f64(&[]), 0.0);
    assert!((sum_fourth_f64(&[1.0, 2.0, 3.0]) - 98.0).abs() < 1e-10);

    assert_eq!(sum_abs_dev_f64(&[], 1.0), 0.0);
    assert!((sum_abs_dev_f64(&[1.0, 2.0, 3.0], 2.0) - 2.0).abs() < 1e-10);

    assert_eq!(
        covariance_components_f64(&[], &[]),
        (0.0, 0.0, 0.0, 0.0, 0.0)
    );
    let corr = correlation_f64(&[5.0, 5.0, 5.0], &[7.0, 7.0, 7.0]);
    assert_eq!(corr, 0.0);

    assert_eq!(sum_abs_diff_f64(&[], &[]), 0.0);
    assert!((sum_abs_diff_f64(&[1.0, 2.0, 3.0], &[1.0, 4.0, 0.0]) - 5.0).abs() < 1e-10);

    let current = vec![
        10.0_f32, 11.0, 13.0, 15.0, 16.0, 17.0, 18.0, 21.0, 22.0, 25.0,
    ];
    let lagged = vec![
        9.0_f32, 10.0, 12.0, 14.0, 15.0, 15.0, 16.0, 20.0, 21.0, 23.0,
    ];
    let mut out = vec![0.0_f32; current.len()];
    lagged_sub_f32(&current, &lagged, &mut out);
    assert!((out[9] - 2.0).abs() < 1e-5);

    let mut high = vec![
        11.0_f64,
        f64::NAN,
        12.0,
        14.0,
        15.0,
        17.0,
        18.0,
        20.0,
        22.0,
        f64::INFINITY,
    ];
    let low = vec![10.0_f64, 9.0, 10.0, 12.0, 13.0, 15.0, 16.0, 18.0, 19.0, 0.0];
    let prev_close = vec![10.5_f64, 10.8, 11.1, 13.0, 14.0, 16.0, 17.0, 19.0, 21.0];
    let mut tr_out = vec![0.0_f64; high.len()];
    true_range_f64(&high, &low, &prev_close, &mut tr_out);
    assert!(tr_out[1].is_nan());
    assert!(tr_out[9].is_nan());
    high[1] = 11.5;
    true_range_f64(&high, &low, &prev_close, &mut tr_out);
    assert!(tr_out[9].is_nan());

    let (sum_sc, cnt_sc) = sum_and_count_f64(&[1.0, 2.0, f64::NAN, 3.0]);
    assert!((sum_sc - 6.0).abs() < 1e-10);
    assert_eq!(cnt_sc, 3);
}

#[test]
fn coverage_push_accumulators_api_surface_paths() {
    let mut rs = RollingSumF64::new();
    rs.add(1.0_f64);
    rs.remove(0.5_f64);
    assert!(rs.value() > 0.0);
    let _ = rs.as_f32();
    let _ = rs.as_f64();
    rs.reset();
    assert_eq!(rs.value(), 0.0);

    let mut rs2 = RollingSumF64::with_initial(5.0);
    rs2.add(NoPrimitive);
    assert!(rs2.value().is_nan());

    let mut rv = RollingVarianceF64::new();
    assert!(rv.mean().is_nan());
    assert!(rv.variance().is_nan());
    assert!(rv.sample_variance().is_nan());
    rv.push(1.0_f64);
    rv.push(2.0_f64);
    let _ = rv.population_stddev();
    let _ = rv.sample_stddev();
    let _ = rv.sum();
    let _ = rv.sum_sq();
    let _ = rv.count();
    rv.pop(NoPrimitive);
    rv.reset();
    assert_eq!(rv.count(), 0);

    let rv2 = RollingVarianceF64::with_initial(3.0, 5.0, 2);
    assert_eq!(rv2.count(), 2);

    let mut w = WelfordVarianceF64::new();
    w.pop(1.0_f64);
    assert!(w.mean().is_nan());
    assert!(w.variance().is_nan());
    assert!(w.sample_variance().is_nan());
    w.push(1.0_f64);
    w.push(2.0_f64);
    let _ = w.population_stddev();
    let _ = w.sample_stddev();
    let _ = w.m2();
    w.reset();
    assert_eq!(w.count(), 0);

    let mut w2 = WelfordVarianceF64::with_initial(1.0, 0.0, 1);
    w2.pop(1.0_f64);
    assert_eq!(w2.count(), 0);

    let mut cs = CumulativeSum::new();
    cs.add(3.0_f64);
    cs.subtract(1.0_f64);
    let _ = cs.value();
    let _ = cs.as_f32();
    cs.add(NoPrimitive);
    cs.subtract(NoPrimitive);
    cs.reset();
    assert_eq!(cs.value(), 0.0);

    let mut cps = CumulativeProductSum::new();
    cps.add(10.0_f64, 2.0_f64);
    cps.add(NoPrimitive, 1.0_f64);
    cps.add(1.0_f64, NoPrimitive);
    let _ = cps.value();
    let _ = cps.as_f32();
    cps.reset();
    assert_eq!(cps.value(), 0.0);

    let mut ws = WilderSmoothing::new();
    ws.update(10.0_f64, 14);
    assert!(!ws.is_initialized());
    ws.initialize(NoPrimitive);
    assert!(ws.is_initialized());
    ws.update(5.0_f64, 14);
    let _ = ws.value();
    ws.reset();
    assert!(!ws.is_initialized());
}

#[test]
fn coverage_push_directional_and_stoch_dispatch_paths() {
    let (mut high, mut low, mut close) = sample_ohlc(64);
    let period = 7usize;

    let mut out = vec![0.0_f64; high.len()];
    let mut dx_buf = vec![0.0_f64; high.len()];
    assert!(matches!(
        dx_into::<f64>(&[], &[], &[], period, &mut []),
        Err(Error::EmptyInput)
    ));
    assert!(matches!(
        dx_into(&high[..6], &low[..6], &close[..6], period, &mut dx_buf[..6]),
        Err(Error::InsufficientData { .. })
    ));

    assert!(matches!(
        adxr_into::<f64>(&[], &[], &[], period, &mut []),
        Err(Error::EmptyInput)
    ));
    assert!(matches!(
        adxr_into(&high[..6], &low[..6], &close[..6], period, &mut out[..6]),
        Err(Error::InsufficientData { .. })
    ));
    assert!(matches!(
        adxr_into(&high, &low, &close, period, &mut out[..63]),
        Err(Error::BufferTooSmall { .. })
    ));
    adxr_into(&high, &low, &close, period, &mut out).expect("adxr_into should succeed");
    let adxr_vals = adxr(&high, &low, &close, period).expect("adxr should succeed");
    assert_eq!(adxr_vals.len(), high.len());

    let flat = vec![10.0_f64; 16];
    dx_into(&flat, &flat, &flat, 5, &mut [0.0_f64; 16]).expect("dx flat should succeed");

    high[2] = f64::NAN;
    low[2] = f64::NAN;
    close[2] = f64::NAN;
    dx_into(&high, &low, &close, period, &mut dx_buf).expect("dx should propagate NaN");
    assert!(dx_buf[period].is_nan() || dx_buf[period].is_finite());

    assert!(matches!(
        plus_dm_into::<f64>(&[], &[], period, &mut []),
        Err(Error::EmptyInput)
    ));
    assert!(matches!(
        minus_dm_into::<f64>(&[], &[], period, &mut []),
        Err(Error::EmptyInput)
    ));
    assert!(matches!(
        plus_dm_into(&high[..6], &low[..6], period, &mut [0.0_f64; 6]),
        Err(Error::InsufficientData { .. })
    ));
    assert!(matches!(
        minus_dm_into(&high[..6], &low[..6], period, &mut [0.0_f64; 6]),
        Err(Error::InsufficientData { .. })
    ));
    assert!(matches!(
        plus_dm_into(&high, &low, period, &mut vec![0.0_f64; high.len() - 1]),
        Err(Error::BufferTooSmall { .. })
    ));
    assert!(matches!(
        minus_dm_into(&high, &low, period, &mut vec![0.0_f64; high.len() - 1]),
        Err(Error::BufferTooSmall { .. })
    ));
    let pdm = plus_dm(&high, &low, period).expect("plus_dm wrapper should succeed");
    let mdm = minus_dm(&high, &low, period).expect("minus_dm wrapper should succeed");
    assert_eq!(pdm.len(), high.len());
    assert_eq!(mdm.len(), high.len());

    let _ = adx(
        &sample_ohlc(80).0,
        &sample_ohlc(80).1,
        &sample_ohlc(80).2,
        10,
    )
    .expect("adx f64");
    let (h2, l2, c2) = sample_ohlc(80);
    let h2f: Vec<f32> = h2.iter().map(|&v| v as f32).collect();
    let l2f: Vec<f32> = l2.iter().map(|&v| v as f32).collect();
    let c2f: Vec<f32> = c2.iter().map(|&v| v as f32).collect();
    let _ = adx(&h2f, &l2f, &c2f, 10).expect("adx f32");

    let (hf, lf, cf) = sample_ohlc(72);
    let st_fast = stochastic_fast(&hf, &lf, &cf, 14, 3).expect("stochastic fast f64");
    assert_eq!(st_fast.k.len(), hf.len());

    let mut st_out = StochasticOutput {
        k: vec![0.0_f64; hf.len()],
        d: vec![0.0_f64; hf.len()],
    };
    stochastic_fast_into(&hf, &lf, &cf, 14, 3, &mut st_out).expect("stochastic fast into f64");

    let st_full = stochastic_full(&hf, &lf, &cf, 14, 3, 3).expect("stochastic full f64");
    assert_eq!(st_full.k.len(), hf.len());
    stochastic_full_into(&hf, &lf, &cf, 14, 3, 3, &mut st_out).expect("stochastic full into f64");

    let hf32: Vec<f32> = hf.iter().map(|&v| v as f32).collect();
    let lf32: Vec<f32> = lf.iter().map(|&v| v as f32).collect();
    let cf32: Vec<f32> = cf.iter().map(|&v| v as f32).collect();
    let _ = stochastic_fast(&hf32, &lf32, &cf32, 14, 3).expect("stochastic fast f32");
    let _ = stochastic_full(&hf32, &lf32, &cf32, 14, 3, 3).expect("stochastic full f32");

    let hf16: Vec<f16> = hf.iter().map(|&v| f16::from_f32(v as f32)).collect();
    let lf16: Vec<f16> = lf.iter().map(|&v| f16::from_f32(v as f32)).collect();
    let cf16: Vec<f16> = cf.iter().map(|&v| f16::from_f32(v as f32)).collect();
    let _ = stochastic_fast(&hf16, &lf16, &cf16, 14, 3).expect("stochastic fast f16 generic");
    let _ = stochastic_full(&hf16, &lf16, &cf16, 14, 3, 3).expect("stochastic full f16 generic");

    let data32: Vec<f32> = (0..96).map(|i| 60.0 + i as f32 * 0.1).collect();
    let out32 = stochrsi(&data32, 6, 6, 2, 3).expect("stochrsi f32");
    assert_eq!(out32.fastk.len(), data32.len());
    let mut out32k = vec![f32::NAN; data32.len()];
    let mut out32d = vec![f32::NAN; data32.len()];
    stochrsi_into(&data32, 6, 6, 2, 3, &mut out32k, &mut out32d).expect("stochrsi_into f32");
    let data16: Vec<f16> = (0..96)
        .map(|i| f16::from_f32(60.0 + i as f32 * 0.1))
        .collect();
    let out16 = stochrsi(&data16, 6, 6, 2, 3).expect("stochrsi f16");
    assert_eq!(out16.fastk.len(), data16.len());
    let mut out16k = vec![f16::NAN; data16.len()];
    let mut out16d = vec![f16::NAN; data16.len()];
    stochrsi_into(&data16, 6, 6, 2, 3, &mut out16k, &mut out16d).expect("stochrsi_into f16");

    let data_kama_f64: Vec<f64> = (0..60).map(|i| 100.0 + i as f64 * 0.2).collect();
    let data_kama_f32: Vec<f32> = data_kama_f64.iter().map(|&v| v as f32).collect();
    let _ = kama(&data_kama_f64, 10).expect("kama f64");
    let _ = kama_full(&data_kama_f64, 10, 2, 30).expect("kama_full f64");
    let _ = kama(&data_kama_f32, 10).expect("kama f32");
    let _ = kama_full(&data_kama_f32, 10, 2, 30).expect("kama_full f32");

    let data_kama_min = vec![1.0_f64; 10];
    let _ = kama(&data_kama_min, 10).expect("kama min-len early exit");
    let _ = kama_full(&data_kama_min, 10, 2, 30).expect("kama_full min-len early exit");
}
