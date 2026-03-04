use liq_ta::indicators::stochastic::{
    StochasticOutput, stochastic_fast, stochastic_fast_into, stochastic_full, stochastic_full_into,
};
use liq_ta::indicators::williams_r::{williams_r, williams_r_into, williams_r_lookback};
use liq_ta::indicators::wma::{wma, wma_into, wma_lookback};

fn make_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    let mut p = 100.0_f64;
    for i in 0..n {
        p += if i % 6 < 3 { 0.31 } else { -0.19 } + (i as f64 * 0.015).sin() * 0.2;
        let c = p;
        let h = c + 0.85 + ((i % 5) as f64) * 0.02;
        let l = c - 0.80 - ((i % 7) as f64) * 0.015;
        high.push(h.max(l + 0.001));
        low.push(l);
        close.push(c);
    }
    (high, low, close)
}

fn make_series(n: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity(n);
    let mut v = 50.0_f64;
    for i in 0..n {
        v += if i % 4 == 0 { 0.44 } else { -0.11 } + (i as f64 * 0.03).cos() * 0.1;
        out.push(v);
    }
    out
}

#[test]
fn williams_r_large_input_parity_alloc_into() {
    let (high, low, close) = make_ohlc(1600);
    let period = 13;
    let lookback = williams_r_lookback(period);

    let alloc = williams_r(&high, &low, &close, period).expect("williams_r alloc should succeed");
    let mut into = vec![0.0_f64; high.len()];
    let valid = williams_r_into(&high, &low, &close, period, &mut into)
        .expect("williams_r_into should succeed");

    assert_eq!(valid, high.len() - lookback);
    for i in 0..lookback {
        assert!(alloc[i].is_nan());
        assert!(into[i].is_nan());
    }
    for i in lookback..high.len() {
        if alloc[i].is_nan() || into[i].is_nan() {
            assert!(alloc[i].is_nan() && into[i].is_nan());
        } else {
            assert!((alloc[i] - into[i]).abs() < 1e-10, "mismatch at {i}");
            assert!((-100.0..=0.0).contains(&alloc[i]));
        }
    }
}

#[test]
fn williams_r_large_input_non_finite_window_behavior_f32() {
    let (high64, low64, close64) = make_ohlc(1400);
    let mut high: Vec<f32> = high64.iter().map(|&v| v as f32).collect();
    let mut low: Vec<f32> = low64.iter().map(|&v| v as f32).collect();
    let close: Vec<f32> = close64.iter().map(|&v| v as f32).collect();

    high[700] = f32::NAN;
    low[703] = f32::INFINITY;

    let out = williams_r(&high, &low, &close, 13).expect("williams_r f32 should succeed");
    assert!(out[700].is_nan() || out[703].is_nan());
    assert!(out[720].is_finite() || out[721].is_finite());
}

#[test]
fn wma_optimistic_and_tracking_paths_parity() {
    let period = 11;
    let lookback = wma_lookback(period);
    let mut data = make_series(256);

    // Clean path (optimistic fast path)
    let clean_alloc = wma(&data, period).expect("wma clean alloc should succeed");
    let mut clean_into = vec![0.0_f64; data.len()];
    let clean_valid =
        wma_into(&data, period, &mut clean_into).expect("wma clean into should succeed");
    assert_eq!(clean_valid, data.len() - lookback);

    // Force fallback/tracking path: invalid in initial window and later entering window
    data[3] = f64::NAN;
    data[120] = f64::INFINITY;
    let dirty_alloc = wma(&data, period).expect("wma dirty alloc should succeed");
    let mut dirty_into = vec![0.0_f64; data.len()];
    wma_into(&data, period, &mut dirty_into).expect("wma dirty into should succeed");

    for i in 0..data.len() {
        if clean_alloc[i].is_nan() || clean_into[i].is_nan() {
            assert!(clean_alloc[i].is_nan() && clean_into[i].is_nan());
        } else {
            assert!(
                (clean_alloc[i] - clean_into[i]).abs() < 1e-10,
                "clean mismatch at {i}"
            );
        }

        if dirty_alloc[i].is_nan() || dirty_into[i].is_nan() {
            assert!(dirty_alloc[i].is_nan() && dirty_into[i].is_nan());
        } else {
            assert!(
                (dirty_alloc[i] - dirty_into[i]).abs() < 1e-10,
                "dirty mismatch at {i}"
            );
        }
    }
}

#[test]
fn stochastic_into_nan_slow_paths_fast_and_full() {
    let (mut high, mut low, mut close) = make_ohlc(128);
    high[0] = f64::NAN;
    low[33] = f64::INFINITY;
    close[65] = f64::NAN;

    let mut fast_out = StochasticOutput {
        k: vec![0.0_f64; high.len()],
        d: vec![0.0_f64; high.len()],
    };
    let mut full_out = StochasticOutput {
        k: vec![0.0_f64; high.len()],
        d: vec![0.0_f64; high.len()],
    };

    let fast_counts = stochastic_fast_into(&high, &low, &close, 14, 3, &mut fast_out)
        .expect("stochastic_fast_into should succeed");
    let full_counts = stochastic_full_into(&high, &low, &close, 14, 3, 3, &mut full_out)
        .expect("stochastic_full_into should succeed");

    assert_eq!(fast_counts.0, high.len() - 13);
    assert!(fast_counts.1 <= fast_counts.0);
    assert!(full_counts.0 <= fast_counts.0);
    assert!(full_counts.1 <= full_counts.0);
    assert!(fast_out.k.iter().any(|v| v.is_nan()));
    assert!(fast_out.d.iter().any(|v| v.is_nan()));
    assert!(full_out.k.iter().any(|v| v.is_nan()));
    assert!(full_out.d.iter().any(|v| v.is_nan()));
}

#[test]
fn stochastic_alloc_into_parity_finite_data() {
    let (high, low, close) = make_ohlc(192);
    let fast_alloc =
        stochastic_fast(&high, &low, &close, 14, 3).expect("stochastic_fast alloc should succeed");
    let full_alloc = stochastic_full(&high, &low, &close, 14, 3, 3)
        .expect("stochastic_full alloc should succeed");

    let mut fast_into = StochasticOutput {
        k: vec![0.0_f64; high.len()],
        d: vec![0.0_f64; high.len()],
    };
    let mut full_into = StochasticOutput {
        k: vec![0.0_f64; high.len()],
        d: vec![0.0_f64; high.len()],
    };
    stochastic_fast_into(&high, &low, &close, 14, 3, &mut fast_into)
        .expect("stochastic_fast_into should succeed");
    stochastic_full_into(&high, &low, &close, 14, 3, 3, &mut full_into)
        .expect("stochastic_full_into should succeed");

    for i in 0..high.len() {
        if fast_alloc.k[i].is_nan() || fast_into.k[i].is_nan() {
            assert!(fast_alloc.k[i].is_nan() && fast_into.k[i].is_nan());
        } else {
            assert!(
                (fast_alloc.k[i] - fast_into.k[i]).abs() < 1e-10,
                "fast k mismatch at {i}"
            );
        }
        if fast_alloc.d[i].is_nan() || fast_into.d[i].is_nan() {
            assert!(fast_alloc.d[i].is_nan() && fast_into.d[i].is_nan());
        } else {
            assert!(
                (fast_alloc.d[i] - fast_into.d[i]).abs() < 1e-10,
                "fast d mismatch at {i}"
            );
        }

        if full_alloc.k[i].is_nan() || full_into.k[i].is_nan() {
            assert!(full_alloc.k[i].is_nan() && full_into.k[i].is_nan());
        } else {
            assert!(
                (full_alloc.k[i] - full_into.k[i]).abs() < 1e-10,
                "full k mismatch at {i}"
            );
        }
        if full_alloc.d[i].is_nan() || full_into.d[i].is_nan() {
            assert!(full_alloc.d[i].is_nan() && full_into.d[i].is_nan());
        } else {
            assert!(
                (full_alloc.d[i] - full_into.d[i]).abs() < 1e-10,
                "full d mismatch at {i}"
            );
        }
    }
}
