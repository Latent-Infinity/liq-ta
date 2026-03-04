//! Stage 2 targeted coverage tests for into/validation/error branches.

use liq_ta::indicators::{
    ao, ao_into, autocorr, autocorr_into, bears_power, bears_power_into, bulls_power,
    bulls_power_into, chop, chop_into, connors_rsi, connors_rsi_into, demarker, demarker_into, dpo,
    dpo_into, dss_bressert, dss_bressert_into, gaussian_channel, gaussian_channel_into,
    gaussian_filter, gaussian_filter_into, hma, hma_atr_bands, hma_atr_bands_into,
    hma_bollinger_bands, hma_bollinger_bands_into, hma_into, hurst, hurst_into, laguerre_rsi,
    laguerre_rsi_into, osma, osma_into, rvi, rvi_into, stc, stc_into, supertrend, supertrend_into,
    ulcer_index, ulcer_index_into, vortex, vortex_into, vwap_atr_bands, vwap_atr_bands_into,
    vwap_bollinger_bands, vwap_bollinger_bands_into,
};

type Ohlcv = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>);

fn sample_ohlcv(n: usize) -> Ohlcv {
    let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.25).collect();
    let open: Vec<f64> = close.iter().map(|v| v - 0.1).collect();
    let high: Vec<f64> = close.iter().map(|v| v + 0.8).collect();
    let low: Vec<f64> = close.iter().map(|v| v - 0.8).collect();
    let volume: Vec<f64> = (0..n).map(|i| 1_000.0 + (i % 15) as f64 * 17.0).collect();
    (open, high, low, close, volume)
}

#[test]
fn stage2_into_success_paths() {
    let n = 300;
    let (open, high, low, close, volume) = sample_ohlcv(n);

    let hma_out = hma(&close, 21).unwrap();
    let mut hma_buf = vec![f64::NAN; n];
    assert!(hma_into(&close, 21, &mut hma_buf).unwrap() > 0);
    assert_eq!(hma_out.len(), n);

    let gf_out = gaussian_filter(&close, 20, 0.5).unwrap();
    let mut gf_buf = vec![f64::NAN; n];
    assert!(gaussian_filter_into(&close, 20, 0.5, &mut gf_buf).unwrap() > 0);
    assert_eq!(gf_out.len(), n);

    let ao_out = ao(&high, &low).unwrap();
    let mut ao_buf = vec![f64::NAN; n];
    assert!(ao_into(&high, &low, &mut ao_buf).unwrap() > 0);
    assert_eq!(ao_out.len(), n);

    let bulls_out = bulls_power(&high, &low, &close, 13).unwrap();
    let bears_out = bears_power(&high, &low, &close, 13).unwrap();
    let mut bulls_buf = vec![f64::NAN; n];
    let mut bears_buf = vec![f64::NAN; n];
    assert!(bulls_power_into(&high, &low, &close, 13, &mut bulls_buf).unwrap() > 0);
    assert!(bears_power_into(&high, &low, &close, 13, &mut bears_buf).unwrap() > 0);
    assert_eq!(bulls_out.len(), n);
    assert_eq!(bears_out.len(), n);

    let demarker_out = demarker(&high, &low, 14).unwrap();
    let mut demarker_buf = vec![f64::NAN; n];
    assert!(demarker_into(&high, &low, 14, &mut demarker_buf).unwrap() > 0);
    assert_eq!(demarker_out.len(), n);

    let osma_out = osma(&close, 12, 26, 9).unwrap();
    let mut osma_buf = vec![f64::NAN; n];
    assert!(osma_into(&close, 12, 26, 9, &mut osma_buf).unwrap() > 0);
    assert_eq!(osma_out.len(), n);

    let vortex_out = vortex(&high, &low, &close, 14).unwrap();
    let mut plus = vec![f64::NAN; n];
    let mut minus = vec![f64::NAN; n];
    assert!(vortex_into(&high, &low, &close, 14, &mut plus, &mut minus).unwrap() > 0);
    assert_eq!(vortex_out.plus_vi.len(), n);
    assert_eq!(vortex_out.minus_vi.len(), n);

    let rvi_out = rvi(&open, &high, &low, &close, 10).unwrap();
    let mut rvi_buf = vec![f64::NAN; n];
    assert!(rvi_into(&open, &high, &low, &close, 10, &mut rvi_buf).unwrap() > 0);
    assert_eq!(rvi_out.len(), n);

    let dpo_out = dpo(&close, 20).unwrap();
    let mut dpo_buf = vec![f64::NAN; n];
    assert!(dpo_into(&close, 20, &mut dpo_buf).unwrap() > 0);
    assert_eq!(dpo_out.len(), n);

    let connors_out = connors_rsi(&close, 3, 2, 100).unwrap();
    let mut connors_buf = vec![f64::NAN; n];
    assert!(connors_rsi_into(&close, 3, 2, 100, &mut connors_buf).unwrap() > 0);
    assert_eq!(connors_out.len(), n);

    let stc_out = stc(&close, 23, 50, 10, 3).unwrap();
    let mut stc_buf = vec![f64::NAN; n];
    assert!(stc_into(&close, 23, 50, 10, 3, &mut stc_buf).unwrap() > 0);
    assert_eq!(stc_out.len(), n);

    let laguerre_out = laguerre_rsi(&close, 0.5).unwrap();
    let mut laguerre_buf = vec![f64::NAN; n];
    assert!(laguerre_rsi_into(&close, 0.5, &mut laguerre_buf).unwrap() > 0);
    assert_eq!(laguerre_out.len(), n);

    let dss_out = dss_bressert(&high, &low, &close, 14, 5).unwrap();
    let mut dss_buf = vec![f64::NAN; n];
    assert!(dss_bressert_into(&high, &low, &close, 14, 5, &mut dss_buf).unwrap() > 0);
    assert_eq!(dss_out.len(), n);

    let chop_out = chop(&high, &low, &close, 14).unwrap();
    let mut chop_buf = vec![f64::NAN; n];
    assert!(chop_into(&high, &low, &close, 14, &mut chop_buf).unwrap() > 0);
    assert_eq!(chop_out.len(), n);

    let ulcer_out = ulcer_index(&close, 14).unwrap();
    let mut ulcer_buf = vec![f64::NAN; n];
    assert!(ulcer_index_into(&close, 14, &mut ulcer_buf).unwrap() > 0);
    assert_eq!(ulcer_out.len(), n);

    let hurst_out = hurst(&close, 64).unwrap();
    let mut hurst_buf = vec![f64::NAN; n];
    assert!(hurst_into(&close, 64, &mut hurst_buf).unwrap() > 0);
    assert_eq!(hurst_out.len(), n);

    let autocorr_out = autocorr(&close, 32, 1).unwrap();
    let mut autocorr_buf = vec![f64::NAN; n];
    assert!(autocorr_into(&close, 32, 1, &mut autocorr_buf).unwrap() > 0);
    assert_eq!(autocorr_out.len(), n);

    let supertrend_out = supertrend(&high, &low, &close, 10, 3.0).unwrap();
    let mut st_line = vec![f64::NAN; n];
    let mut st_upper = vec![f64::NAN; n];
    let mut st_lower = vec![f64::NAN; n];
    let mut st_trend = vec![f64::NAN; n];
    assert!(
        supertrend_into(
            &high,
            &low,
            &close,
            10,
            3.0,
            &mut st_line,
            &mut st_upper,
            &mut st_lower,
            &mut st_trend,
        )
        .unwrap()
            > 0
    );
    assert_eq!(supertrend_out.supertrend.len(), n);

    let gc_out = gaussian_channel(&close, 20, 0.5, 2.0).unwrap();
    let mut gc_center = vec![f64::NAN; n];
    let mut gc_upper = vec![f64::NAN; n];
    let mut gc_lower = vec![f64::NAN; n];
    let mut gc_trend = vec![f64::NAN; n];
    assert!(
        gaussian_channel_into(
            &close,
            20,
            0.5,
            2.0,
            &mut gc_center,
            &mut gc_upper,
            &mut gc_lower,
            &mut gc_trend,
        )
        .unwrap()
            > 0
    );
    assert_eq!(gc_out.center.len(), n);

    let hma_atr_out = hma_atr_bands(&high, &low, &close, 21, 14, 2.0).unwrap();
    let mut hma_atr_upper = vec![f64::NAN; n];
    let mut hma_atr_middle = vec![f64::NAN; n];
    let mut hma_atr_lower = vec![f64::NAN; n];
    assert!(
        hma_atr_bands_into(
            &high,
            &low,
            &close,
            21,
            14,
            2.0,
            &mut hma_atr_upper,
            &mut hma_atr_middle,
            &mut hma_atr_lower,
        )
        .unwrap()
            > 0
    );
    assert_eq!(hma_atr_out.middle.len(), n);

    let hma_bb_out = hma_bollinger_bands(&close, 21, 20, 2.0).unwrap();
    let mut hma_bb_upper = vec![f64::NAN; n];
    let mut hma_bb_middle = vec![f64::NAN; n];
    let mut hma_bb_lower = vec![f64::NAN; n];
    assert!(
        hma_bollinger_bands_into(
            &close,
            21,
            20,
            2.0,
            &mut hma_bb_upper,
            &mut hma_bb_middle,
            &mut hma_bb_lower,
        )
        .unwrap()
            > 0
    );
    assert_eq!(hma_bb_out.middle.len(), n);

    let vwap_atr_out = vwap_atr_bands(&high, &low, &close, &volume, 14, 2.0).unwrap();
    let mut vwap_atr_upper = vec![f64::NAN; n];
    let mut vwap_atr_middle = vec![f64::NAN; n];
    let mut vwap_atr_lower = vec![f64::NAN; n];
    assert!(
        vwap_atr_bands_into(
            &high,
            &low,
            &close,
            &volume,
            14,
            2.0,
            &mut vwap_atr_upper,
            &mut vwap_atr_middle,
            &mut vwap_atr_lower,
        )
        .unwrap()
            > 0
    );
    assert_eq!(vwap_atr_out.middle.len(), n);

    let vwap_bb_out = vwap_bollinger_bands(&high, &low, &close, &volume, 20, 2.0).unwrap();
    let mut vwap_bb_upper = vec![f64::NAN; n];
    let mut vwap_bb_middle = vec![f64::NAN; n];
    let mut vwap_bb_lower = vec![f64::NAN; n];
    assert!(
        vwap_bollinger_bands_into(
            &high,
            &low,
            &close,
            &volume,
            20,
            2.0,
            &mut vwap_bb_upper,
            &mut vwap_bb_middle,
            &mut vwap_bb_lower,
        )
        .unwrap()
            > 0
    );
    assert_eq!(vwap_bb_out.middle.len(), n);
}

#[test]
fn stage2_validation_error_paths() {
    let (open, high, low, close, volume) = sample_ohlcv(300);

    assert!(gaussian_filter(&close, 0, 0.5).is_err());
    assert!(gaussian_filter(&close, 20, 0.0).is_err());
    assert!(gaussian_channel(&close, 20, 0.0, 2.0).is_err());
    assert!(gaussian_channel(&close, 20, 0.5, 0.0).is_err());

    assert!(ao(&[] as &[f64], &[] as &[f64]).is_err());
    assert!(ao(&high, &low[..low.len() - 1]).is_err());
    assert!(ao(&high[..20], &low[..20]).is_err());

    assert!(hma(&close, 0).is_err());
    assert!(hma(&[] as &[f64], 21).is_err());
    assert!(hma(&close[..10], 21).is_err());
    assert!(supertrend(&high, &low, &close, 0, 3.0).is_err());
    assert!(supertrend(&high, &low, &close, 10, 0.0).is_err());
    assert!(supertrend(&high[..20], &low[..19], &close[..20], 10, 3.0).is_err());

    assert!(bulls_power(&high, &low, &close, 0).is_err());
    assert!(bears_power(&high, &low, &close, 0).is_err());
    assert!(demarker(&high, &low, 0).is_err());
    assert!(osma(&close, 26, 12, 9).is_err());
    assert!(vortex(&high, &low, &close, 0).is_err());
    assert!(rvi(&open, &high, &low, &close, 0).is_err());
    assert!(rvi(&open[..20], &high[..19], &low[..20], &close[..20], 10).is_err());
    assert!(rvi(&open[..5], &high[..5], &low[..5], &close[..5], 10).is_err());
    assert!(dpo(&close, 0).is_err());
    assert!(connors_rsi(&[] as &[f64], 3, 2, 20).is_err());
    assert!(connors_rsi(&close, 0, 2, 100).is_err());
    assert!(connors_rsi(&close, 3, 0, 100).is_err());
    assert!(connors_rsi(&close, 3, 2, 0).is_err());
    assert!(connors_rsi(&close[..15], 3, 2, 20).is_err());
    assert!(stc(&close, 50, 23, 10, 3).is_err());
    assert!(laguerre_rsi(&close, -0.1).is_err());
    assert!(dss_bressert(&high, &low, &close, 0, 5).is_err());
    assert!(dss_bressert(&high, &low, &close, 14, 0).is_err());
    assert!(dss_bressert(&high[..40], &low[..39], &close[..40], 14, 5).is_err());
    assert!(dss_bressert(&high[..10], &low[..10], &close[..10], 14, 5).is_err());
    assert!(chop(&high, &low, &close, 0).is_err());
    assert!(chop(&high[..20], &low[..19], &close[..20], 14).is_err());
    assert!(chop(&high[..10], &low[..10], &close[..10], 14).is_err());
    assert!(ulcer_index(&close, 0).is_err());
    assert!(hurst(&close, 0).is_err());
    assert!(autocorr(&close, 0, 1).is_err());
    assert!(autocorr(&close, 32, 0).is_err());
    assert!(autocorr(&close, 32, 32).is_err());
    assert!(autocorr(&[] as &[f64], 32, 1).is_err());

    assert!(hma_atr_bands(&high, &low, &close, 21, 14, 0.0).is_err());
    assert!(hma_bollinger_bands(&close, 21, 20, 0.0).is_err());
    assert!(vwap_atr_bands(&high, &low, &close, &volume, 14, 0.0).is_err());
    assert!(vwap_bollinger_bands(&high, &low, &close, &volume, 20, 0.0).is_err());
}

#[test]
fn stage2_non_finite_and_flat_data_paths() {
    let n = 260;
    let (_, high, low, close, _) = sample_ohlcv(n);

    let mut close_with_nan = close.clone();
    close_with_nan[40] = f64::NAN;
    close_with_nan[41] = f64::NAN;
    assert_eq!(hma(&close_with_nan, 21).unwrap().len(), n);
    let mut hma_buf = vec![f64::NAN; n];
    assert!(hma_into(&close_with_nan, 21, &mut hma_buf).unwrap() > 0);

    let open_flat = vec![100.0; n];
    let high_flat = vec![100.0; n];
    let low_flat = vec![100.0; n];
    let close_flat = vec![100.0; n];

    let rvi_flat = rvi(&open_flat, &high_flat, &low_flat, &close_flat, 10).unwrap();
    assert_eq!(rvi_flat.len(), n);

    let chop_flat = chop(&high_flat, &low_flat, &close_flat, 14).unwrap();
    assert_eq!(chop_flat.len(), n);

    let dss_flat = dss_bressert(&high_flat, &low_flat, &close_flat, 14, 5).unwrap();
    assert_eq!(dss_flat.len(), n);

    let mut zigzag = Vec::with_capacity(n);
    for i in 0..n {
        let value = match i % 5 {
            0 => 100.0,
            1 => 101.0,
            2 => 100.0,
            3 => 100.0,
            _ => 102.0,
        };
        zigzag.push(value);
    }
    let connors = connors_rsi(&zigzag, 3, 2, 20).unwrap();
    assert_eq!(connors.len(), n);

    let mut high_with_nan = high.clone();
    let mut low_with_nan = low.clone();
    high_with_nan[30] = f64::NAN;
    low_with_nan[30] = f64::NAN;
    let ao_with_nan = ao(&high_with_nan, &low_with_nan).unwrap();
    assert_eq!(ao_with_nan.len(), n);

    let mut autocorr_nan = close.clone();
    autocorr_nan[25] = f64::NAN;
    let ac = autocorr(&autocorr_nan, 32, 1).unwrap();
    assert_eq!(ac.len(), n);
}

#[test]
fn stage2_buffer_too_small_error_paths() {
    let n = 300;
    let (open, high, low, close, volume) = sample_ohlcv(n);

    let mut short = vec![f64::NAN; n - 1];
    assert!(hma_into(&close, 21, &mut short).is_err());
    assert!(gaussian_filter_into(&close, 20, 0.5, &mut short).is_err());
    assert!(ao_into(&high, &low, &mut short).is_err());
    assert!(bulls_power_into(&high, &low, &close, 13, &mut short).is_err());
    assert!(bears_power_into(&high, &low, &close, 13, &mut short).is_err());
    assert!(demarker_into(&high, &low, 14, &mut short).is_err());
    assert!(osma_into(&close, 12, 26, 9, &mut short).is_err());
    assert!(rvi_into(&open, &high, &low, &close, 10, &mut short).is_err());
    assert!(dpo_into(&close, 20, &mut short).is_err());
    assert!(connors_rsi_into(&close, 3, 2, 100, &mut short).is_err());
    assert!(stc_into(&close, 23, 50, 10, 3, &mut short).is_err());
    assert!(laguerre_rsi_into(&close, 0.5, &mut short).is_err());
    assert!(dss_bressert_into(&high, &low, &close, 14, 5, &mut short).is_err());
    assert!(chop_into(&high, &low, &close, 14, &mut short).is_err());
    assert!(ulcer_index_into(&close, 14, &mut short).is_err());
    assert!(hurst_into(&close, 64, &mut short).is_err());
    assert!(autocorr_into(&close, 32, 1, &mut short).is_err());

    let mut full = vec![f64::NAN; n];
    let mut short2 = vec![f64::NAN; n - 1];
    assert!(vortex_into(&high, &low, &close, 14, &mut short2, &mut full).is_err());
    assert!(vortex_into(&high, &low, &close, 14, &mut full, &mut short2).is_err());

    let mut a = vec![f64::NAN; n];
    let mut b = vec![f64::NAN; n];
    let mut c = vec![f64::NAN; n];
    let mut d = vec![f64::NAN; n];
    assert!(
        supertrend_into(
            &high,
            &low,
            &close,
            10,
            3.0,
            &mut short2,
            &mut b,
            &mut c,
            &mut d,
        )
        .is_err()
    );
    assert!(
        supertrend_into(
            &high,
            &low,
            &close,
            10,
            3.0,
            &mut a,
            &mut short2,
            &mut c,
            &mut d,
        )
        .is_err()
    );
    assert!(
        supertrend_into(
            &high,
            &low,
            &close,
            10,
            3.0,
            &mut a,
            &mut b,
            &mut short2,
            &mut d,
        )
        .is_err()
    );
    assert!(
        supertrend_into(
            &high,
            &low,
            &close,
            10,
            3.0,
            &mut a,
            &mut b,
            &mut c,
            &mut short2,
        )
        .is_err()
    );

    assert!(
        gaussian_channel_into(&close, 20, 0.5, 2.0, &mut short2, &mut b, &mut c, &mut d,).is_err()
    );
    assert!(
        gaussian_channel_into(&close, 20, 0.5, 2.0, &mut a, &mut short2, &mut c, &mut d,).is_err()
    );
    assert!(
        gaussian_channel_into(&close, 20, 0.5, 2.0, &mut a, &mut b, &mut short2, &mut d,).is_err()
    );
    assert!(
        gaussian_channel_into(&close, 20, 0.5, 2.0, &mut a, &mut b, &mut c, &mut short2,).is_err()
    );

    assert!(
        hma_atr_bands_into(
            &high,
            &low,
            &close,
            21,
            14,
            2.0,
            &mut short2,
            &mut b,
            &mut c,
        )
        .is_err()
    );
    assert!(hma_bollinger_bands_into(&close, 21, 20, 2.0, &mut short2, &mut b, &mut c).is_err());
    assert!(
        vwap_atr_bands_into(
            &high,
            &low,
            &close,
            &volume,
            14,
            2.0,
            &mut short2,
            &mut b,
            &mut c,
        )
        .is_err()
    );
    assert!(
        vwap_bollinger_bands_into(
            &high,
            &low,
            &close,
            &volume,
            20,
            2.0,
            &mut short2,
            &mut b,
            &mut c,
        )
        .is_err()
    );
}
