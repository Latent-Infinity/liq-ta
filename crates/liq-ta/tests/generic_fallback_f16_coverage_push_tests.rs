use half::f16;
use liq_ta::indicators::ema::{ema_with_alpha, ema_with_alpha_into};
use liq_ta::indicators::kama::{kama, kama_full, kama_full_into, kama_into};
use liq_ta::indicators::midpoint::{midpoint, midpoint_into};
use liq_ta::indicators::midprice::{midprice, midprice_into};
use liq_ta::indicators::roc::{
    roc, roc_into, rocp, rocp_into, rocr, rocr_into, rocr100, rocr100_into,
};
use liq_ta::indicators::sma::{sma, sma_into};
use liq_ta::indicators::statistics::{
    beta, correl, cov, kurt, linearreg, linearreg_angle, linearreg_intercept, linearreg_into,
    linearreg_slope, mad, sem, skew, stddev, tsf, var, zscore,
};
use liq_ta::indicators::stochastic::{
    StochasticOutput, stochastic_fast, stochastic_fast_into, stochastic_full, stochastic_full_into,
};
use liq_ta::indicators::stochrsi::{stochrsi, stochrsi_into};
use liq_ta::indicators::trix::{trix, trix_into};
use liq_ta::indicators::vwap::vwap_into;
use liq_ta::indicators::williams_r::williams_r;
use liq_ta::indicators::wma::{wma, wma_into};

#[inline]
fn h(v: f32) -> f16 {
    f16::from_f32(v)
}

fn seq(len: usize, start: f32, step: f32) -> Vec<f16> {
    (0..len).map(|i| h(start + step * i as f32)).collect()
}

#[test]
fn generic_f16_fallback_matrix_alloc_and_into() {
    let n = 40usize;
    let data = seq(n, 1.0, 0.25);
    let data_b = seq(n, 2.0, 0.5);

    // EMA/SMA/WMA generic fallbacks
    let ema_out = ema_with_alpha(&data, 5, h(0.5)).expect("ema_with_alpha f16");
    assert_eq!(ema_out.len(), n);
    let mut ema_into_out = vec![h(0.0); n];
    let ema_valid =
        ema_with_alpha_into(&data, 5, h(0.5), &mut ema_into_out).expect("ema_with_alpha_into f16");
    assert_eq!(ema_valid, n - 4);

    let sma_out = sma(&data, 5).expect("sma f16");
    assert_eq!(sma_out.len(), n);
    let mut sma_into_out = vec![h(0.0); n];
    let sma_valid = sma_into(&data, 5, &mut sma_into_out).expect("sma_into f16");
    assert_eq!(sma_valid, n - 4);

    let wma_out = wma(&data, 5).expect("wma f16");
    assert_eq!(wma_out.len(), n);
    let mut wma_into_out = vec![h(0.0); n];
    let wma_valid = wma_into(&data, 5, &mut wma_into_out).expect("wma_into f16");
    assert_eq!(wma_valid, n - 4);

    // Statistics generic fallback wrappers
    for out in [
        var(&data, 5).expect("var f16"),
        stddev(&data, 5).expect("stddev f16"),
        skew(&data, 5).expect("skew f16"),
        kurt(&data, 5).expect("kurt f16"),
        cov(&data, &data_b, 5).expect("cov f16"),
        sem(&data, 5).expect("sem f16"),
        zscore(&data, 5).expect("zscore f16"),
        mad(&data, 5).expect("mad f16"),
        correl(&data, &data_b, 5).expect("correl f16"),
        beta(&data, &data_b, 5).expect("beta f16"),
        linearreg(&data, 5).expect("linearreg f16"),
        linearreg_slope(&data, 5).expect("linearreg_slope f16"),
        linearreg_intercept(&data, 5).expect("linearreg_intercept f16"),
        linearreg_angle(&data, 5).expect("linearreg_angle f16"),
        tsf(&data, 5).expect("tsf f16"),
    ] {
        assert_eq!(out.len(), n);
    }

    // VWAP/Williams generic fallbacks
    let high = seq(n, 10.0, 0.2);
    let low = seq(n, 9.0, 0.2);
    let close = seq(n, 9.5, 0.2);
    let vol = seq(n, 100.0, 3.0);

    let mut vwap_out = vec![h(0.0); n];
    let vwap_valid = vwap_into(&high, &low, &close, &vol, &mut vwap_out).expect("vwap_into f16");
    assert_eq!(vwap_valid, n);

    let wr = williams_r(&high, &low, &close, 5).expect("williams_r f16");
    assert_eq!(wr.len(), n);

    // Stochastic generic fallback paths
    let st_fast = stochastic_fast(&high, &low, &close, 5, 3).expect("stochastic_fast f16");
    assert_eq!(st_fast.k.len(), n);
    assert_eq!(st_fast.d.len(), n);

    let mut st_fast_into = StochasticOutput {
        k: vec![h(0.0); n],
        d: vec![h(0.0); n],
    };
    let (valid_k_fast, valid_d_fast) =
        stochastic_fast_into(&high, &low, &close, 5, 3, &mut st_fast_into)
            .expect("stochastic_fast_into f16");
    assert_eq!(valid_k_fast, n - 4);
    assert_eq!(valid_d_fast, n - 6);

    let st_full = stochastic_full(&high, &low, &close, 5, 3, 3).expect("stochastic_full f16");
    assert_eq!(st_full.k.len(), n);
    assert_eq!(st_full.d.len(), n);

    let mut st_full_into = StochasticOutput {
        k: vec![h(0.0); n],
        d: vec![h(0.0); n],
    };
    let (valid_k_full, valid_d_full) =
        stochastic_full_into(&high, &low, &close, 5, 3, 3, &mut st_full_into)
            .expect("stochastic_full_into f16");
    assert_eq!(valid_k_full, n - 6);
    assert_eq!(valid_d_full, n - 8);

    // Stochastic generic fallback slow paths (has_nan=true)
    let mut high_nan = high.clone();
    let mut low_nan = low.clone();
    let mut close_nan = close.clone();
    high_nan[3] = h(f32::NAN);
    low_nan[3] = h(f32::NAN);
    close_nan[3] = h(f32::NAN);
    assert!(stochastic_fast(&high_nan, &low_nan, &close_nan, 5, 3).is_ok());
    assert!(stochastic_full(&high_nan, &low_nan, &close_nan, 5, 3, 3).is_ok());
    let mut st_fast_into_nan = StochasticOutput {
        k: vec![h(0.0); n],
        d: vec![h(0.0); n],
    };
    let mut st_full_into_nan = StochasticOutput {
        k: vec![h(0.0); n],
        d: vec![h(0.0); n],
    };
    assert!(
        stochastic_fast_into(&high_nan, &low_nan, &close_nan, 5, 3, &mut st_fast_into_nan).is_ok()
    );
    assert!(
        stochastic_full_into(
            &high_nan,
            &low_nan,
            &close_nan,
            5,
            3,
            3,
            &mut st_full_into_nan
        )
        .is_ok()
    );

    // KAMA generic fallback paths
    let kama_out = kama(&data, 10).expect("kama f16");
    assert_eq!(kama_out.len(), n);
    let mut kama_into_out = vec![h(0.0); n];
    kama_into(&data, 10, &mut kama_into_out).expect("kama_into f16");

    let kama_full_out = kama_full(&data, 10, 2, 30).expect("kama_full f16");
    assert_eq!(kama_full_out.len(), n);
    let mut kama_full_into_out = vec![h(0.0); n];
    kama_full_into(&data, 10, 2, 30, &mut kama_full_into_out).expect("kama_full_into f16");

    // Midpoint / Midprice generic fallbacks
    let midpoint_out = midpoint(&data, 5).expect("midpoint f16");
    assert_eq!(midpoint_out.len(), n);
    let mut midpoint_into_out = vec![h(0.0); n];
    midpoint_into(&data, 5, &mut midpoint_into_out).expect("midpoint_into f16");

    let midprice_out = midprice(&high, &low, 5).expect("midprice f16");
    assert_eq!(midprice_out.len(), n);
    let mut midprice_into_out = vec![h(0.0); n];
    midprice_into(&high, &low, 5, &mut midprice_into_out).expect("midprice_into f16");

    // ROC family generic fallbacks
    for out in [
        roc(&data, 5).expect("roc f16"),
        rocp(&data, 5).expect("rocp f16"),
        rocr(&data, 5).expect("rocr f16"),
        rocr100(&data, 5).expect("rocr100 f16"),
    ] {
        assert_eq!(out.len(), n);
    }
    let mut roc_into_out = vec![h(0.0); n];
    roc_into(&data, 5, &mut roc_into_out).expect("roc_into f16");
    rocp_into(&data, 5, &mut roc_into_out).expect("rocp_into f16");
    rocr_into(&data, 5, &mut roc_into_out).expect("rocr_into f16");
    rocr100_into(&data, 5, &mut roc_into_out).expect("rocr100_into f16");

    // StochRSI generic fallback paths
    let stochrsi_out = stochrsi(&data, 14, 14, 3, 3).expect("stochrsi f16");
    assert_eq!(stochrsi_out.fastk.len(), n);
    assert_eq!(stochrsi_out.fastd.len(), n);
    let mut stochrsi_fastk = vec![h(0.0); n];
    let mut stochrsi_fastd = vec![h(0.0); n];
    stochrsi_into(
        &data,
        14,
        14,
        3,
        3,
        &mut stochrsi_fastk,
        &mut stochrsi_fastd,
    )
    .expect("stochrsi_into f16");

    // TRIX generic fallback paths
    let trix_out = trix(&data, 5).expect("trix f16");
    assert_eq!(trix_out.len(), n);
    let mut trix_into_out = vec![h(0.0); n];
    trix_into(&data, 5, &mut trix_into_out).expect("trix_into f16");

    // WMA generic fallback NaN/recovery branches
    let mut wma_data_nan = data.clone();
    wma_data_nan[1] = h(f32::NAN);
    wma_data_nan[2] = h(f32::NAN);
    wma_data_nan[10] = h(11.0);
    let wma_nan_out = wma(&wma_data_nan, 5).expect("wma f16 nan branch");
    assert_eq!(wma_nan_out.len(), n);
    let mut wma_nan_into_out = vec![h(0.0); n];
    assert!(wma_into(&wma_data_nan, 5, &mut wma_nan_into_out).is_ok());

    // VWAP generic invalid/zero-volume branches
    let mut high_v = high.clone();
    let mut low_v = low.clone();
    let mut close_v = close.clone();
    let mut vol_v = vol.clone();
    vol_v[0] = h(0.0);
    high_v[2] = h(f32::INFINITY);
    low_v[2] = h(f32::INFINITY);
    close_v[2] = h(f32::INFINITY);
    let mut vwap_out2 = vec![h(0.0); n];
    assert!(vwap_into(&high_v, &low_v, &close_v, &vol_v, &mut vwap_out2).is_ok());

    // Williams %R generic fallback invalid/range-zero branches
    let flat_high = vec![h(10.0); n];
    let flat_low = vec![h(10.0); n];
    let flat_close = vec![h(10.0); n];
    let wr_flat = williams_r(&flat_high, &flat_low, &flat_close, 5).expect("williams_r flat");
    assert_eq!(wr_flat.len(), n);
    let mut high_bad = flat_high.clone();
    high_bad[6] = h(f32::NAN);
    assert!(williams_r(&high_bad, &flat_low, &flat_close, 5).is_ok());

    // Statistics specific branch: empty-input linearreg_into
    let mut lin_out = vec![h(0.0); 1];
    assert!(linearreg_into::<f16>(&[], 3, &mut lin_out).is_err());
}
