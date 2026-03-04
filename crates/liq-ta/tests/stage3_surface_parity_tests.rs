//! Stage 3 surface parity integration tests.

use liq_ta::indicators::{
    ao, ao_lookback, chop, chop_lookback, gaussian_channel, gaussian_channel_lookback, hma,
    hma_lookback, osma, osma_lookback, supertrend, supertrend_lookback,
};

type Ohlcv = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>);

fn sample_ohlcv(n: usize) -> Ohlcv {
    let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.2).collect();
    let open: Vec<f64> = close.iter().map(|v| v - 0.1).collect();
    let high: Vec<f64> = close.iter().map(|v| v + 0.8).collect();
    let low: Vec<f64> = close.iter().map(|v| v - 0.8).collect();
    let volume: Vec<f64> = (0..n).map(|i| 1_000.0 + (i % 12) as f64 * 15.0).collect();
    (open, high, low, close, volume)
}

#[test]
fn stage3_surface_parity_representative_categories() {
    let n = 260;
    let (_open, high, low, close, _volume) = sample_ohlcv(n);

    // MA category
    let hma_period = 21;
    let hma_out = hma(&close, hma_period).unwrap();
    let hma_lb = hma_lookback(hma_period);
    assert_eq!(hma_out.len(), n);
    assert!(hma_out[..hma_lb].iter().all(|v| v.is_nan()));
    assert!(hma_out[hma_lb..].iter().all(|v| v.is_finite()));

    // Trend category
    let st_period = 10;
    let st_out = supertrend(&high, &low, &close, st_period, 3.0).unwrap();
    let st_lb = supertrend_lookback(st_period);
    assert_eq!(st_out.supertrend.len(), n);
    assert_eq!(st_out.upper_band.len(), n);
    assert_eq!(st_out.lower_band.len(), n);
    assert_eq!(st_out.trend.len(), n);
    assert!(st_out.supertrend[..st_lb].iter().all(|v| v.is_nan()));
    assert!(st_out.trend[..st_lb].iter().all(|v| v.is_nan()));

    // Momentum category
    let osma_out = osma(&close, 12, 26, 9).unwrap();
    let osma_lb = osma_lookback(12, 26, 9);
    assert_eq!(osma_out.len(), n);
    assert!(osma_out[..osma_lb].iter().all(|v| v.is_nan()));

    let ao_out = ao(&high, &low).unwrap();
    let ao_lb = ao_lookback();
    assert_eq!(ao_out.len(), n);
    assert!(ao_out[..ao_lb].iter().all(|v| v.is_nan()));

    // Volatility category
    let chop_period = 14;
    let chop_out = chop(&high, &low, &close, chop_period).unwrap();
    let chop_lb = chop_lookback(chop_period);
    assert_eq!(chop_out.len(), n);
    assert!(chop_out[..chop_lb].iter().all(|v| v.is_nan()));

    // Regime category
    let gc_period = 20;
    let gc_out = gaussian_channel(&close, gc_period, 0.5, 2.0).unwrap();
    let gc_lb = gaussian_channel_lookback(gc_period);
    assert_eq!(gc_out.center.len(), n);
    assert_eq!(gc_out.upper.len(), n);
    assert_eq!(gc_out.lower.len(), n);
    assert_eq!(gc_out.trend.len(), n);
    assert!(gc_out.center[..gc_lb].iter().all(|v| v.is_nan()));
    assert!(gc_out.trend[..gc_lb].iter().all(|v| v.is_nan()));
}
