use liq_ta::indicators::stochastic::{
    Stochastic, StochasticOutput, stochastic, stochastic_d_lookback, stochastic_fast,
    stochastic_fast_into, stochastic_full, stochastic_full_into, stochastic_into,
    stochastic_k_lookback, stochastic_min_len, stochastic_slow, stochastic_slow_into,
};

fn sample_ohlc_f64(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    for i in 0..n {
        let base = 100.0 + (i as f64) * 0.1 + ((i % 11) as f64 - 5.0) * 0.07;
        let h = base + 1.2 + ((i % 3) as f64) * 0.05;
        let l = base - 1.0 - ((i % 4) as f64) * 0.05;
        let c = (h + l) * 0.5 + ((i % 2) as f64 - 0.5) * 0.1;
        high.push(h);
        low.push(l);
        close.push(c);
    }
    (high, low, close)
}

fn sample_ohlc_f32(n: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    for i in 0..n {
        let b = 100.0_f32 + (i % 17) as f32;
        high.push(b + 5.0);
        low.push(b - 5.0);
        close.push(b);
    }
    (high, low, close)
}

#[test]
fn coverage_stochastic_lookback_edge_surface() {
    assert_eq!(stochastic_k_lookback(5), 4);
    assert_eq!(stochastic_d_lookback(5, 3), 6);
    assert_eq!(stochastic_min_len(5, 3), 7);

    let _ = stochastic_k_lookback(0);
    let _ = stochastic_d_lookback(0, 0);
    let _ = stochastic_min_len(0, 0);
}

#[test]
fn coverage_stochastic_f32_dispatch_and_into_matrix() {
    let (high, low, close) = sample_ohlc_f32(96);

    assert!(stochastic_fast(&high, &low, &close, 5, 3).is_ok());
    assert!(stochastic_slow(&high, &low, &close, 5, 3).is_ok());
    assert!(stochastic_full(&high, &low, &close, 5, 3, 3).is_ok());
    assert!(stochastic(&high, &low, &close, 5, 3, 1).is_ok());
    assert!(stochastic(&high, &low, &close, 5, 3, 3).is_ok());

    let mut out = StochasticOutput {
        k: vec![0.0_f32; high.len()],
        d: vec![0.0_f32; high.len()],
    };
    assert!(stochastic_fast_into(&high, &low, &close, 5, 3, &mut out).is_ok());
    assert!(stochastic_slow_into(&high, &low, &close, 5, 3, &mut out).is_ok());
    assert!(stochastic_full_into(&high, &low, &close, 5, 3, 3, &mut out).is_ok());
    assert!(stochastic_into(&high, &low, &close, 5, 3, 1, &mut out).is_ok());
    assert!(stochastic_into(&high, &low, &close, 5, 3, 3, &mut out).is_ok());

    let mut short = StochasticOutput {
        k: vec![0.0_f32; high.len() - 1],
        d: vec![0.0_f32; high.len()],
    };
    assert!(stochastic_fast_into(&high, &low, &close, 5, 3, &mut short).is_err());
    let mut short2 = StochasticOutput {
        k: vec![0.0_f32; high.len()],
        d: vec![0.0_f32; high.len() - 1],
    };
    assert!(stochastic_slow_into(&high, &low, &close, 5, 3, &mut short2).is_err());
}

#[test]
fn coverage_stochastic_f32_builder_surface() {
    let (high, low, close) = sample_ohlc_f32(120);
    let cfg = Stochastic::new()
        .with_k_period(7)
        .with_d_period(4)
        .with_k_slowing(3);

    assert_eq!(cfg.k_period(), 7);
    assert_eq!(cfg.d_period(), 4);
    assert_eq!(cfg.k_slowing(), 3);
    assert!(cfg.k_lookback() <= cfg.d_lookback());
    assert!(cfg.min_len() > 0);

    assert!(cfg.compute(&high, &low, &close).is_ok());
    let mut out = StochasticOutput {
        k: vec![0.0_f32; high.len()],
        d: vec![0.0_f32; high.len()],
    };
    assert!(cfg.compute_into(&high, &low, &close, &mut out).is_ok());

    let fast_cfg = Stochastic::fast(7, 4);
    let slow_cfg = Stochastic::slow(7, 4);
    assert!(fast_cfg.compute(&high, &low, &close).is_ok());
    assert!(slow_cfg.compute(&high, &low, &close).is_ok());
}

#[test]
fn coverage_stochastic_large_period_length_matrix_f64() {
    let (high_small, low_small, close_small) = sample_ohlc_f64(320);
    let (high_large, low_large, close_large) = sample_ohlc_f64(4096);

    for &(k, d, s) in &[(9_usize, 3_usize, 1_usize), (21, 5, 3), (55, 9, 5)] {
        assert!(stochastic_fast(&high_small, &low_small, &close_small, k, d).is_ok());
        assert!(stochastic_slow(&high_small, &low_small, &close_small, k, d).is_ok());
        assert!(stochastic_full(&high_small, &low_small, &close_small, k, s, d).is_ok());
        assert!(stochastic(&high_small, &low_small, &close_small, k, d, s).is_ok());

        assert!(stochastic_fast(&high_large, &low_large, &close_large, k, d).is_ok());
        assert!(stochastic_full(&high_large, &low_large, &close_large, k, s, d).is_ok());
    }

    let mut out = StochasticOutput {
        k: vec![0.0_f64; high_large.len()],
        d: vec![0.0_f64; high_large.len()],
    };
    assert!(
        stochastic_full_into(&high_large, &low_large, &close_large, 55, 5, 9, &mut out).is_ok()
    );
    assert!(stochastic_into(&high_large, &low_large, &close_large, 55, 9, 5, &mut out).is_ok());
}
