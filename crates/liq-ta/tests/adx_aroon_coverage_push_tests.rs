use liq_ta::indicators::adx::{adx, adx_into, adx_lookback, adx_min_len, di_lookback, di_min_len};
use liq_ta::indicators::aroon::{
    aroon, aroon_into, aroon_lookback, aroon_min_len, aroonosc, aroonosc_into, aroonosc_lookback,
    aroonosc_min_len,
};

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

fn sample_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);

    for i in 0..n {
        let base = 100.0 + (i as f64) * 0.35 + ((i % 7) as f64 - 3.0) * 0.2;
        let h = base + 1.2 + ((i % 3) as f64) * 0.1;
        let l = base - 1.1 - ((i % 4) as f64) * 0.1;
        let c = (h + l) * 0.5 + ((i % 2) as f64 - 0.5) * 0.2;
        high.push(h);
        low.push(l);
        close.push(c);
    }

    (high, low, close)
}

fn sample_high_low(n: usize) -> (Vec<f64>, Vec<f64>) {
    let (high, low, _) = sample_ohlc(n);
    (high, low)
}

#[test]
fn coverage_adx_surface_and_into_equivalence() {
    let period = 5;
    assert_eq!(adx_lookback(period), 2 * period - 1);
    assert_eq!(adx_min_len(period), 2 * period);
    assert_eq!(di_lookback(period), period);
    assert_eq!(di_min_len(period), period + 1);

    let (high, low, close) = sample_ohlc(64);
    let out = adx(&high, &low, &close, period).expect("adx should succeed");
    assert_eq!(out.adx.len(), high.len());
    assert_eq!(out.plus_di.len(), high.len());
    assert_eq!(out.minus_di.len(), high.len());

    let lookback = adx_lookback(period);
    assert!(out.adx[..lookback].iter().all(|v| v.is_nan()));
    assert!(out.adx[lookback..].iter().all(|v| v.is_finite()));

    let mut adx_buf = vec![0.0_f64; high.len()];
    let mut plus_buf = vec![0.0_f64; high.len()];
    let mut minus_buf = vec![0.0_f64; high.len()];
    adx_into(
        &high,
        &low,
        &close,
        period,
        &mut adx_buf,
        &mut plus_buf,
        &mut minus_buf,
    )
    .expect("adx_into should succeed");

    for i in 0..high.len() {
        if out.adx[i].is_nan() {
            assert!(adx_buf[i].is_nan());
        } else {
            assert!(approx_eq(out.adx[i], adx_buf[i], 1e-12));
        }
        if out.plus_di[i].is_nan() {
            assert!(plus_buf[i].is_nan());
        } else {
            assert!(approx_eq(out.plus_di[i], plus_buf[i], 1e-12));
        }
        if out.minus_di[i].is_nan() {
            assert!(minus_buf[i].is_nan());
        } else {
            assert!(approx_eq(out.minus_di[i], minus_buf[i], 1e-12));
        }
    }

    let high32: Vec<f32> = high.iter().map(|&v| v as f32).collect();
    let low32: Vec<f32> = low.iter().map(|&v| v as f32).collect();
    let close32: Vec<f32> = close.iter().map(|&v| v as f32).collect();
    assert!(adx(&high32, &low32, &close32, period).is_ok());
}

#[test]
fn coverage_adx_error_and_flat_market_paths() {
    let (high, low, close) = sample_ohlc(20);

    assert!(adx(&[] as &[f64], &[] as &[f64], &[] as &[f64], 5).is_err());
    assert!(adx(&high, &low, &close, 0).is_err());
    assert!(adx(&high[..10], &low[..9], &close[..10], 5).is_err());
    assert!(adx(&high[..9], &low[..9], &close[..9], 5).is_err());

    let mut adx_buf = vec![0.0_f64; high.len()];
    let mut plus_buf = vec![0.0_f64; high.len()];
    let mut minus_buf = vec![0.0_f64; high.len()];
    let mut short = vec![0.0_f64; high.len() - 1];

    assert!(
        adx_into(
            &high,
            &low,
            &close,
            5,
            &mut short,
            &mut plus_buf,
            &mut minus_buf
        )
        .is_err()
    );
    assert!(
        adx_into(
            &high,
            &low,
            &close,
            5,
            &mut adx_buf,
            &mut short,
            &mut minus_buf
        )
        .is_err()
    );
    assert!(
        adx_into(
            &high,
            &low,
            &close,
            5,
            &mut adx_buf,
            &mut plus_buf,
            &mut short
        )
        .is_err()
    );

    let flat = vec![42.0_f64; 40];
    let out = adx(&flat, &flat, &flat, 5).expect("adx flat series should succeed");
    let lb = adx_lookback(5);
    assert!(out.adx[..lb].iter().all(|v| v.is_nan()));
    assert!(out.adx[lb..].iter().all(|v| v.is_finite()));
}

#[test]
fn coverage_aroon_dispatch_surface_and_into_equivalence() {
    assert_eq!(aroon_lookback(5), 5);
    assert_eq!(aroon_min_len(5), 6);
    assert_eq!(aroonosc_lookback(5), 5);
    assert_eq!(aroonosc_min_len(5), 6);

    let (high_small, low_small) = sample_high_low(120);
    assert!(aroon(&high_small, &low_small, 5).is_ok());
    assert!(aroon(&high_small, &low_small, 13).is_ok());
    assert!(aroon(&high_small, &low_small, 34).is_ok());

    let (high_large, low_large) = sample_high_low(1200);
    assert!(aroon(&high_large, &low_large, 13).is_ok());

    let period = 14;
    let out = aroon(&high_small, &low_small, period).expect("aroon should succeed");
    assert_eq!(out.aroon_up.len(), high_small.len());
    assert_eq!(out.aroon_down.len(), high_small.len());

    let lookback = aroon_lookback(period);
    assert!(out.aroon_up[..lookback].iter().all(|v| v.is_nan()));
    assert!(out.aroon_down[..lookback].iter().all(|v| v.is_nan()));
    assert!(
        out.aroon_up[lookback..]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0)
    );
    assert!(
        out.aroon_down[lookback..]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0)
    );

    let mut up_buf = vec![0.0_f64; high_small.len()];
    let mut down_buf = vec![0.0_f64; high_small.len()];
    aroon_into(&high_small, &low_small, period, &mut up_buf, &mut down_buf)
        .expect("aroon_into should succeed");

    for i in 0..high_small.len() {
        if out.aroon_up[i].is_nan() {
            assert!(up_buf[i].is_nan());
        } else {
            assert!(approx_eq(out.aroon_up[i], up_buf[i], 1e-12));
        }
        if out.aroon_down[i].is_nan() {
            assert!(down_buf[i].is_nan());
        } else {
            assert!(approx_eq(out.aroon_down[i], down_buf[i], 1e-12));
        }
    }

    let osc = aroonosc(&high_small, &low_small, period).expect("aroonosc should succeed");
    let mut osc_buf = vec![0.0_f64; high_small.len()];
    aroonosc_into(&high_small, &low_small, period, &mut osc_buf).expect("aroonosc_into ok");
    for i in 0..high_small.len() {
        if osc[i].is_nan() {
            assert!(osc_buf[i].is_nan());
        } else {
            assert!(approx_eq(osc[i], osc_buf[i], 1e-12));
        }
    }

    let high32: Vec<f32> = high_small.iter().map(|&v| v as f32).collect();
    let low32: Vec<f32> = low_small.iter().map(|&v| v as f32).collect();
    assert!(aroon(&high32, &low32, period).is_ok());
    assert!(aroonosc(&high32, &low32, period).is_ok());
}

#[test]
fn coverage_aroon_error_and_non_finite_paths() {
    let (high, low) = sample_high_low(32);

    assert!(aroon(&[] as &[f64], &[] as &[f64], 5).is_err());
    assert!(aroon(&high, &low, 0).is_err());
    assert!(aroon(&high, &low[..low.len() - 1], 5).is_err());
    assert!(aroon(&high[..5], &low[..5], 5).is_err());

    let mut up_buf = vec![0.0_f64; high.len()];
    let mut down_buf = vec![0.0_f64; high.len()];
    let mut short = vec![0.0_f64; high.len() - 1];
    assert!(aroon_into(&high, &low, 5, &mut short, &mut down_buf).is_err());
    assert!(aroon_into(&high, &low, 5, &mut up_buf, &mut short).is_err());

    let mut osc_short = vec![0.0_f64; high.len() - 1];
    assert!(aroonosc_into(&high, &low, 5, &mut osc_short).is_err());
    assert!(aroonosc(&[] as &[f64], &[] as &[f64], 5).is_err());
    assert!(aroonosc(&high, &low, 0).is_err());
    assert!(aroonosc(&high, &low[..low.len() - 1], 5).is_err());
    assert!(aroonosc(&high[..5], &low[..5], 5).is_err());

    let mut high_bad = high.clone();
    let mut low_bad = low.clone();
    high_bad[10] = f64::NAN;
    low_bad[11] = f64::INFINITY;
    let out = aroon(&high_bad, &low_bad, 5).expect("aroon should succeed with non-finite input");
    let osc = aroonosc(&high_bad, &low_bad, 5).expect("aroonosc should succeed with bad input");
    assert!(out.aroon_up.iter().skip(5).any(|v| v.is_nan()));
    assert!(out.aroon_down.iter().skip(5).any(|v| v.is_nan()));
    assert!(osc.iter().skip(5).any(|v| v.is_nan()));
}

#[test]
fn coverage_aroon_f32_dispatch_matrix_and_into() {
    let (high_small, low_small) = sample_high_low(256);
    let high_small32: Vec<f32> = high_small.iter().map(|&v| v as f32).collect();
    let low_small32: Vec<f32> = low_small.iter().map(|&v| v as f32).collect();

    for &period in &[5_usize, 13_usize, 34_usize] {
        let out = aroon(&high_small32, &low_small32, period).expect("aroon f32 should succeed");
        let osc =
            aroonosc(&high_small32, &low_small32, period).expect("aroonosc f32 should succeed");

        let mut up = vec![0.0_f32; high_small32.len()];
        let mut down = vec![0.0_f32; high_small32.len()];
        let mut osc_buf = vec![0.0_f32; high_small32.len()];
        aroon_into(&high_small32, &low_small32, period, &mut up, &mut down)
            .expect("aroon_into f32 should succeed");
        aroonosc_into(&high_small32, &low_small32, period, &mut osc_buf)
            .expect("aroonosc_into f32 should succeed");

        let lookback = aroon_lookback(period);
        assert!(out.aroon_up[..lookback].iter().all(|v| v.is_nan()));
        assert!(out.aroon_down[..lookback].iter().all(|v| v.is_nan()));
        assert!(osc[..lookback].iter().all(|v| v.is_nan()));
        assert!(
            out.aroon_up[lookback..]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0)
        );
        assert!(
            out.aroon_down[lookback..]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0)
        );
        assert!(osc[lookback..].iter().all(|v| v.is_finite()));
    }

    let (high_large, low_large) = sample_high_low(1500);
    let high_large32: Vec<f32> = high_large.iter().map(|&v| v as f32).collect();
    let low_large32: Vec<f32> = low_large.iter().map(|&v| v as f32).collect();
    assert!(aroon(&high_large32, &low_large32, 13).is_ok());
    assert!(aroonosc(&high_large32, &low_large32, 13).is_ok());

    let mut high_bad = high_small32.clone();
    let mut low_bad = low_small32.clone();
    high_bad[20] = f32::NAN;
    low_bad[21] = f32::INFINITY;
    let out_bad = aroon(&high_bad, &low_bad, 5).expect("aroon f32 non-finite should succeed");
    let osc_bad = aroonosc(&high_bad, &low_bad, 5).expect("aroonosc f32 non-finite should succeed");
    assert!(out_bad.aroon_up.iter().skip(5).any(|v| v.is_nan()));
    assert!(out_bad.aroon_down.iter().skip(5).any(|v| v.is_nan()));
    assert!(osc_bad.iter().skip(5).any(|v| v.is_nan()));
}

#[test]
fn coverage_aroon_large_lazy_rescan_f64_invalid_window_recovery() {
    let (mut high, mut low) = sample_high_low(1400);
    let period = 13; // lazy-rescan path when n >= 1000
    let lookback = aroon_lookback(period);

    high[700] = f64::NAN;
    low[702] = f64::INFINITY;

    let out = aroon(&high, &low, period).expect("aroon large f64 should succeed");
    assert!(out.aroon_up[lookback].is_finite());
    assert!(out.aroon_down[lookback].is_finite());
    assert!(out.aroon_up[700].is_nan() || out.aroon_down[700].is_nan());
    assert!(out.aroon_up[716].is_finite() || out.aroon_down[716].is_finite());
}

#[test]
fn coverage_aroon_large_lazy_rescan_f32_invalid_window_recovery() {
    let (high64, low64) = sample_high_low(1400);
    let mut high: Vec<f32> = high64.iter().map(|&v| v as f32).collect();
    let mut low: Vec<f32> = low64.iter().map(|&v| v as f32).collect();
    let period = 13; // lazy-rescan path when n >= 1000

    high[650] = f32::NAN;
    low[653] = f32::INFINITY;

    let out = aroon(&high, &low, period).expect("aroon large f32 should succeed");
    assert!(out.aroon_up[650].is_nan() || out.aroon_down[650].is_nan());
    assert!(out.aroon_up[670].is_finite() || out.aroon_down[670].is_finite());
}

#[test]
fn coverage_aroon_van_herk_invalid_window_path() {
    let (mut high, mut low) = sample_high_low(420);
    let period = 34; // van Herk path (period >= threshold)
    let lookback = aroon_lookback(period);

    high[200] = f64::NAN;
    low[201] = f64::INFINITY;

    let out = aroon(&high, &low, period).expect("aroon van Herk path should succeed");
    assert!(out.aroon_up[lookback].is_finite() || out.aroon_down[lookback].is_finite());
    assert!(out.aroon_up[201].is_nan() || out.aroon_down[201].is_nan());
    assert!(out.aroon_up[240].is_finite() || out.aroon_down[240].is_finite());
}

#[test]
fn coverage_adx_f32_non_finite_and_flat_paths() {
    let (high, low, close) = sample_ohlc(72);
    let high32: Vec<f32> = high.iter().map(|&v| v as f32).collect();
    let low32: Vec<f32> = low.iter().map(|&v| v as f32).collect();
    let close32: Vec<f32> = close.iter().map(|&v| v as f32).collect();

    let out = adx(&high32, &low32, &close32, 7).expect("adx f32 should succeed");
    let mut adx_buf = vec![0.0_f32; high32.len()];
    let mut plus_buf = vec![0.0_f32; high32.len()];
    let mut minus_buf = vec![0.0_f32; high32.len()];
    adx_into(
        &high32,
        &low32,
        &close32,
        7,
        &mut adx_buf,
        &mut plus_buf,
        &mut minus_buf,
    )
    .expect("adx_into f32 should succeed");
    assert_eq!(out.adx.len(), adx_buf.len());

    let flat = vec![33.0_f32; 48];
    assert!(adx(&flat, &flat, &flat, 7).is_ok());

    let mut high_bad = high32.clone();
    let mut low_bad = low32.clone();
    let mut close_bad = close32.clone();
    high_bad[18] = f32::NAN;
    low_bad[19] = f32::INFINITY;
    close_bad[20] = f32::NEG_INFINITY;
    let bad = adx(&high_bad, &low_bad, &close_bad, 7).expect("adx f32 non-finite should succeed");
    assert!(bad.adx.iter().skip(adx_lookback(7)).any(|v| v.is_nan()));
}
