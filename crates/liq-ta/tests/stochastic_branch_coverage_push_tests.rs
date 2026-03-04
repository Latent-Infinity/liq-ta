use liq_ta::indicators::stochastic::{
    StochasticOutput, stochastic, stochastic_fast, stochastic_fast_into, stochastic_full,
    stochastic_full_into, stochastic_into,
};
use liq_ta::precision::{PrecisionMode, with_precision_mode};

fn make_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    let mut p = 100.0_f64;
    for i in 0..n {
        p += if i % 2 == 0 { 0.42 } else { -0.17 } + (i as f64 * 0.11).sin() * 0.2;
        let c = p;
        let h = c + 0.8 + ((i % 3) as f64) * 0.05;
        let l = c - 0.7 - ((i % 4) as f64) * 0.04;
        high.push(h);
        low.push(l);
        close.push(c);
    }
    (high, low, close)
}

#[test]
fn stochastic_fast_into_valid_d_zero_boundary() {
    let (high, low, close) = make_ohlc(5);
    let mut out = StochasticOutput {
        k: vec![f64::NAN; 5],
        d: vec![f64::NAN; 5],
    };

    let (valid_k, valid_d) =
        stochastic_fast_into(&high, &low, &close, 5, 3, &mut out).expect("fast_into should work");
    assert_eq!(valid_k, 1);
    assert_eq!(valid_d, 0);
    assert!(out.k[4].is_finite());
    assert!(out.d.iter().all(|v| v.is_nan()));
}

#[test]
fn stochastic_full_into_zero_valid_counts_boundary_and_dispatch() {
    let (high, low, close) = make_ohlc(5);

    let mut out_full = StochasticOutput {
        k: vec![f64::NAN; 5],
        d: vec![f64::NAN; 5],
    };
    let (valid_k, valid_d) = stochastic_full_into(&high, &low, &close, 5, 3, 2, &mut out_full)
        .expect("full_into should work");
    assert_eq!(valid_k, 0);
    assert_eq!(valid_d, 0);

    let mut out_dispatch = StochasticOutput {
        k: vec![f64::NAN; 5],
        d: vec![f64::NAN; 5],
    };
    let (vk2, vd2) = stochastic_into(&high, &low, &close, 5, 2, 3, &mut out_dispatch)
        .expect("dispatch stochastic_into should work");
    assert_eq!((vk2, vd2), (0, 0));
}

#[test]
fn stochastic_f32_high_precision_nan_path_alloc_and_into() {
    let (h64, l64, c64) = make_ohlc(32);
    let mut high: Vec<f32> = h64.iter().map(|&v| v as f32).collect();
    let mut low: Vec<f32> = l64.iter().map(|&v| v as f32).collect();
    let mut close: Vec<f32> = c64.iter().map(|&v| v as f32).collect();

    high[10] = f32::NAN;
    low[21] = f32::NAN;
    close[0] = f32::NAN;

    let fast = with_precision_mode(PrecisionMode::High, || {
        stochastic_fast(&high, &low, &close, 5, 3).expect("fast high mode should succeed")
    });
    let full = with_precision_mode(PrecisionMode::High, || {
        stochastic_full(&high, &low, &close, 5, 3, 3).expect("full high mode should succeed")
    });

    assert_eq!(fast.k.len(), high.len());
    assert_eq!(full.k.len(), high.len());
    assert!(fast.k.iter().any(|v| v.is_nan()));
    assert!(full.k.iter().any(|v| v.is_nan()));

    let mut out = StochasticOutput {
        k: vec![0.0_f32; high.len()],
        d: vec![0.0_f32; high.len()],
    };
    with_precision_mode(PrecisionMode::High, || {
        stochastic_full_into(&high, &low, &close, 5, 3, 3, &mut out)
            .expect("full_into high mode should succeed");
    });
    assert!(out.k.iter().any(|v| v.is_nan()));
    assert!(out.d.iter().any(|v| v.is_nan()));
}

#[test]
fn stochastic_empty_axis_validation_matrix() {
    let one = [1.0_f64];

    assert!(stochastic_fast(&[], &one, &one, 1, 1).is_err());
    assert!(stochastic_fast(&one, &[], &one, 1, 1).is_err());
    assert!(stochastic_fast(&one, &one, &[], 1, 1).is_err());

    assert!(stochastic(&[], &one, &one, 1, 1, 1).is_err());
    assert!(stochastic(&one, &[], &one, 1, 1, 2).is_err());
    assert!(stochastic(&one, &one, &[], 1, 1, 3).is_err());
}
