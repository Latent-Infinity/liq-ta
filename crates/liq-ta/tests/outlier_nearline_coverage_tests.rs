use half::f16;
use liq_ta::indicators::adx::adx;
use liq_ta::indicators::hurst::{hurst, hurst_into};
use liq_ta::indicators::kama::{kama, kama_full};
use liq_ta::indicators::midpoint::midpoint;
use liq_ta::indicators::trix::{trix, trix_into};
use liq_ta::indicators::ulcer_index::{ulcer_index, ulcer_index_into};
use liq_ta::indicators::ultosc::{ultosc, ultosc_default, ultosc_into};
use liq_ta::indicators::vortex::{vortex, vortex_into};

#[test]
fn adx_alloc_surface_f64_and_f32() {
    let n = 90usize;
    let high: Vec<f64> = (0..n)
        .map(|i| 100.0 + i as f64 * 0.7 + ((i % 3) as f64) * 0.1)
        .collect();
    let low: Vec<f64> = high.iter().map(|v| v - 1.8).collect();
    let close: Vec<f64> = high.iter().map(|v| v - 0.6).collect();
    let out = adx(&high, &low, &close, 14).expect("adx f64");
    assert_eq!(out.adx.len(), n);
    assert_eq!(out.plus_di.len(), n);
    assert_eq!(out.minus_di.len(), n);

    let high32: Vec<f32> = high.iter().map(|&v| v as f32).collect();
    let low32: Vec<f32> = low.iter().map(|&v| v as f32).collect();
    let close32: Vec<f32> = close.iter().map(|&v| v as f32).collect();
    let out32 = adx(&high32, &low32, &close32, 14).expect("adx f32");
    assert_eq!(out32.adx.len(), n);
}

#[test]
fn generic_fallback_f16_midpoint_kama_trix() {
    let data: Vec<f16> = (0..96)
        .map(|i| f16::from_f32(20.0 + (i as f32) * 0.3 + (((i * 7) % 5) as f32) * 0.1))
        .collect();

    let mid = midpoint(&data, 10).expect("midpoint f16");
    assert_eq!(mid.len(), data.len());

    let k = kama(&data, 10).expect("kama f16");
    assert_eq!(k.len(), data.len());
    let kf = kama_full(&data, 10, 2, 30).expect("kama_full f16");
    assert_eq!(kf.len(), data.len());

    let tr = trix(&data, 10).expect("trix f16");
    assert_eq!(tr.len(), data.len());
}

#[test]
fn trix_period_one_zero_nan_surface() {
    let data = vec![0.0_f64, 1.0, 2.0, f64::NAN, 4.0, 0.0, 3.0, 6.0];
    let mut out = vec![f64::NAN; data.len()];
    trix_into(&data, 1, &mut out).expect("trix_into period=1");
    assert!(out[0].is_nan());
    assert_eq!(out[1], 0.0);
    assert!(out[3].is_nan());
}

#[test]
fn hurst_and_ulcer_error_and_buffer_surfaces() {
    let short = vec![1.0_f64, 2.0, 3.0];
    assert!(hurst(&short, 5).is_err());
    assert!(ulcer_index(&short, 5).is_err());
    assert!(hurst(&short, 1).is_err());
    assert!(ulcer_index(&short, 0).is_err());

    let data: Vec<f64> = (0..80)
        .map(|i| 100.0 + (i as f64) * 0.2 - ((i % 7) as f64) * 0.4)
        .collect();
    let mut h_small = vec![f64::NAN; data.len() - 1];
    let mut ui_small = vec![f64::NAN; data.len() - 1];
    assert!(hurst_into(&data, 16, &mut h_small).is_err());
    assert!(ulcer_index_into(&data, 14, &mut ui_small).is_err());
}

#[test]
fn ultosc_and_vortex_error_and_alloc_surfaces() {
    let n = 96usize;
    let high: Vec<f64> = (0..n)
        .map(|i| 80.0 + i as f64 * 0.5 + ((i % 4) as f64) * 0.2)
        .collect();
    let low: Vec<f64> = high.iter().map(|v| v - 2.0).collect();
    let close: Vec<f64> = high.iter().map(|v| v - 0.8).collect();

    assert!(ultosc(&high, &low, &close, 0, 14, 28).is_err());
    assert!(vortex(&high, &low, &close, 0).is_err());

    let out = ultosc(&high, &low, &close, 7, 14, 28).expect("ultosc");
    assert_eq!(out.len(), n);
    let out_default = ultosc_default(&high, &low, &close).expect("ultosc_default");
    assert_eq!(out_default.len(), n);

    let vor = vortex(&high, &low, &close, 14).expect("vortex");
    assert_eq!(vor.plus_vi.len(), n);
    assert_eq!(vor.minus_vi.len(), n);

    let mut u_small = vec![f64::NAN; n - 1];
    assert!(ultosc_into(&high, &low, &close, 7, 14, 28, &mut u_small).is_err());
    let mut vp = vec![f64::NAN; n];
    let mut vm = vec![f64::NAN; n - 1];
    assert!(vortex_into(&high, &low, &close, 14, &mut vp, &mut vm).is_err());
}
