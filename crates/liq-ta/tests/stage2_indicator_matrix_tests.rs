//! Stage 2 indicator matrix scaffold tests.

use liq_ta::indicators::{
    ao, ao_min_len, hma, hma_min_len, osma, osma_min_len, supertrend, supertrend_min_len,
};

fn sample_series(n: usize) -> Vec<f64> {
    (0..n).map(|i| 100.0 + i as f64 * 0.2).collect()
}

fn sample_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let close = sample_series(n);
    let high: Vec<f64> = close.iter().map(|v| v + 1.0).collect();
    let low: Vec<f64> = close.iter().map(|v| v - 1.0).collect();
    (high, low, close)
}

#[test]
fn stage2_scaffold_hma_param_validation() {
    let data = sample_series(10);
    assert!(hma(&data, 0).is_err());
    assert!(hma(&data, 100).is_err());
    assert!(hma_min_len(20) > 0);
}

#[test]
fn stage2_scaffold_supertrend_param_validation() {
    let (high, low, close) = sample_ohlc(20);
    assert!(supertrend(&high, &low, &close, 0, 3.0).is_err());
    assert!(supertrend(&high, &low, &close, 10, 0.0).is_err());
    assert!(supertrend_min_len(10) > 0);
}

#[test]
fn stage2_scaffold_ao_param_validation() {
    let (high, low, _) = sample_ohlc(40);
    assert!(ao(&high, &low).is_ok());
    assert!(ao_min_len() >= 34);
    assert!(ao(&high[..10], &low[..10]).is_err());
}

#[test]
fn stage2_scaffold_osma_param_validation() {
    let data = sample_series(80);
    assert!(osma(&data, 12, 26, 9).is_ok());
    assert!(osma(&data, 26, 12, 9).is_err());
    assert!(osma_min_len(12, 26, 9) > 0);
}
