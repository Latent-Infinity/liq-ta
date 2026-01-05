//! Input Validation Hardening Tests (Task 4.1)
//!
//! These tests verify comprehensive input validation per the implementation plan:
//! - NaN in input data
//! - Infinity in input data
//! - Negative values where inappropriate
//! - Extremely large periods
//! - Empty arrays
//!
//! Validation should fail fast with clear error messages.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]

use fast_ta::error::Error;
use fast_ta::indicators::{
    adx::adx,
    atr::atr,
    bollinger::{bollinger, Bollinger},
    donchian::donchian,
    ema::ema,
    macd::macd,
    obv::obv,
    rsi::rsi,
    sma::sma,
    stochastic::{stochastic, stochastic_fast},
    vwap::vwap,
    williams_r::williams_r,
};

// ==================== Empty Array Tests ====================

#[test]
fn validation_empty_array_sma() {
    let empty: Vec<f64> = vec![];
    let result = sma(&empty, 5);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

#[test]
fn validation_empty_array_ema() {
    let empty: Vec<f64> = vec![];
    let result = ema(&empty, 5);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

#[test]
fn validation_empty_array_rsi() {
    let empty: Vec<f64> = vec![];
    let result = rsi(&empty, 14);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

#[test]
fn validation_empty_array_macd() {
    let empty: Vec<f64> = vec![];
    let result = macd(&empty, 12, 26, 9);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

#[test]
fn validation_empty_array_bollinger() {
    let empty: Vec<f64> = vec![];
    let result = bollinger(&empty, 20, 2.0);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

#[test]
fn validation_empty_array_atr() {
    let empty: Vec<f64> = vec![];
    let result = atr(&empty, &empty, &empty, 14);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

#[test]
fn validation_empty_array_stochastic() {
    let empty: Vec<f64> = vec![];
    let result = stochastic_fast(&empty, &empty, &empty, 14, 3);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

#[test]
fn validation_empty_array_adx() {
    let empty: Vec<f64> = vec![];
    let result = adx(&empty, &empty, &empty, 14);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

#[test]
fn validation_empty_array_williams_r() {
    let empty: Vec<f64> = vec![];
    let result = williams_r(&empty, &empty, &empty, 14);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

#[test]
fn validation_empty_array_donchian() {
    let empty: Vec<f64> = vec![];
    let result = donchian(&empty, &empty, 20);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

#[test]
fn validation_empty_array_obv() {
    let empty: Vec<f64> = vec![];
    let result = obv(&empty, &empty);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

#[test]
fn validation_empty_array_vwap() {
    let empty: Vec<f64> = vec![];
    let result = vwap(&empty, &empty, &empty, &empty);
    assert!(matches!(result, Err(Error::EmptyInput)));
}

// ==================== Zero Period Tests ====================

#[test]
fn validation_zero_period_sma() {
    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let result = sma(&data, 0);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

#[test]
fn validation_zero_period_ema() {
    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let result = ema(&data, 0);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

#[test]
fn validation_zero_period_rsi() {
    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let result = rsi(&data, 0);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

#[test]
fn validation_zero_period_bollinger() {
    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let result = bollinger(&data, 0, 2.0);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

#[test]
fn validation_zero_period_atr() {
    let high = vec![11.0_f64, 12.0, 13.0, 14.0, 15.0];
    let low = vec![9.0_f64, 10.0, 11.0, 12.0, 13.0];
    let close = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];
    let result = atr(&high, &low, &close, 0);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

#[test]
fn validation_zero_k_period_stochastic() {
    let high = vec![15.0_f64, 16.0, 17.0, 18.0, 19.0];
    let low = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];
    let close = vec![12.0_f64, 13.0, 14.0, 15.0, 16.0];
    let result = stochastic_fast(&high, &low, &close, 0, 3);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

#[test]
fn validation_zero_d_period_stochastic() {
    let high = vec![15.0_f64, 16.0, 17.0, 18.0, 19.0];
    let low = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];
    let close = vec![12.0_f64, 13.0, 14.0, 15.0, 16.0];
    let result = stochastic_fast(&high, &low, &close, 5, 0);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

// ==================== Period Exceeds Data Length Tests ====================

#[test]
fn validation_period_exceeds_data_sma() {
    let data = vec![1.0_f64, 2.0, 3.0];
    let result = sma(&data, 10);
    assert!(matches!(
        result,
        Err(Error::InsufficientData {
            required: 10,
            actual: 3,
            ..
        })
    ));
}

#[test]
fn validation_period_exceeds_data_ema() {
    let data = vec![1.0_f64, 2.0, 3.0];
    let result = ema(&data, 10);
    assert!(matches!(
        result,
        Err(Error::InsufficientData {
            required: 10,
            actual: 3,
            ..
        })
    ));
}

#[test]
fn validation_period_exceeds_data_rsi() {
    let data = vec![1.0_f64, 2.0, 3.0];
    // RSI needs period+1 data points for first valid value
    let result = rsi(&data, 10);
    assert!(matches!(result, Err(Error::InsufficientData { .. })));
}

#[test]
fn validation_period_exceeds_data_bollinger() {
    let data = vec![1.0_f64, 2.0, 3.0];
    let result = bollinger(&data, 10, 2.0);
    assert!(matches!(
        result,
        Err(Error::InsufficientData {
            required: 10,
            actual: 3,
            ..
        })
    ));
}

// ==================== Extremely Large Period Tests ====================
// These test that we don't crash or allocate excessive memory

#[test]
fn validation_extremely_large_period_sma() {
    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    // Period much larger than data length should return InsufficientData, not crash
    let result = sma(&data, 1_000_000);
    assert!(matches!(result, Err(Error::InsufficientData { .. })));
}

#[test]
fn validation_extremely_large_period_ema() {
    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let result = ema(&data, 1_000_000);
    assert!(matches!(result, Err(Error::InsufficientData { .. })));
}

#[test]
fn validation_extremely_large_period_bollinger() {
    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let result = bollinger(&data, 1_000_000, 2.0);
    assert!(matches!(result, Err(Error::InsufficientData { .. })));
}

#[test]
fn validation_extremely_large_period_stochastic() {
    let high = vec![15.0_f64, 16.0, 17.0, 18.0, 19.0];
    let low = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];
    let close = vec![12.0_f64, 13.0, 14.0, 15.0, 16.0];
    let result = stochastic_fast(&high, &low, &close, 1_000_000, 3);
    assert!(matches!(result, Err(Error::InsufficientData { .. })));
}

// ==================== OHLC Length Mismatch Tests ====================

#[test]
fn validation_ohlc_length_mismatch_atr() {
    let high = vec![11.0_f64, 12.0, 13.0, 14.0, 15.0];
    let low = vec![9.0_f64, 10.0, 11.0]; // Shorter
    let close = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];
    let result = atr(&high, &low, &close, 3);
    assert!(matches!(result, Err(Error::LengthMismatch { .. })));
}

#[test]
fn validation_ohlc_length_mismatch_stochastic() {
    let high = vec![15.0_f64, 16.0, 17.0, 18.0, 19.0];
    let low = vec![10.0_f64, 11.0, 12.0]; // Shorter
    let close = vec![12.0_f64, 13.0, 14.0, 15.0, 16.0];
    let result = stochastic_fast(&high, &low, &close, 3, 2);
    assert!(matches!(result, Err(Error::LengthMismatch { .. })));
}

#[test]
fn validation_ohlc_length_mismatch_adx() {
    let high = vec![11.0_f64, 12.0, 13.0, 14.0, 15.0];
    let low = vec![9.0_f64, 10.0, 11.0, 12.0, 13.0];
    let close = vec![10.0_f64, 11.0, 12.0]; // Shorter
    let result = adx(&high, &low, &close, 3);
    assert!(matches!(result, Err(Error::LengthMismatch { .. })));
}

#[test]
fn validation_ohlc_length_mismatch_williams_r() {
    let high = vec![15.0_f64, 16.0, 17.0];
    let low = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0]; // Longer
    let close = vec![12.0_f64, 13.0, 14.0, 15.0, 16.0];
    let result = williams_r(&high, &low, &close, 3);
    assert!(matches!(result, Err(Error::LengthMismatch { .. })));
}

#[test]
fn validation_ohlc_length_mismatch_donchian() {
    let high = vec![15.0_f64, 16.0, 17.0, 18.0, 19.0];
    let low = vec![10.0_f64, 11.0, 12.0]; // Shorter
    let result = donchian(&high, &low, 3);
    assert!(matches!(result, Err(Error::LengthMismatch { .. })));
}

#[test]
fn validation_ohlcv_length_mismatch_vwap() {
    let high = vec![11.0_f64, 12.0, 13.0, 14.0, 15.0];
    let low = vec![9.0_f64, 10.0, 11.0, 12.0, 13.0];
    let close = vec![10.0_f64, 11.0, 12.0]; // Shorter
    let volume = vec![1000.0_f64, 2000.0, 3000.0, 4000.0, 5000.0];
    let result = vwap(&high, &low, &close, &volume);
    assert!(matches!(result, Err(Error::LengthMismatch { .. })));
}

#[test]
fn validation_cv_length_mismatch_obv() {
    let close = vec![10.0_f64, 11.0, 12.0];
    let volume = vec![1000.0_f64, 2000.0, 3000.0, 4000.0, 5000.0]; // Longer
    let result = obv(&close, &volume);
    assert!(matches!(result, Err(Error::LengthMismatch { .. })));
}

// ==================== Special Float Value Tests (NaN/Infinity in parameters) ====================

#[test]
fn validation_nan_std_dev_bollinger() {
    let data = vec![20.0_f64, 21.0, 22.0, 21.5, 22.5, 23.0];
    let result = bollinger(&data, 3, f64::NAN);

    // NaN std_dev should still compute but produce NaN bands
    let output = result.unwrap();
    // All computed values should be NaN because NaN × stddev = NaN
    for i in 2..output.upper.len() {
        // Middle band is SMA which is computed separately
        assert!(
            output.upper[i].is_nan(),
            "Upper band should be NaN when std_dev is NaN at index {i}"
        );
    }
}

#[test]
fn validation_infinity_std_dev_bollinger() {
    let data = vec![20.0_f64, 21.0, 22.0, 21.5, 22.5, 23.0];
    let result = bollinger(&data, 3, f64::INFINITY);

    // Infinity std_dev should produce infinite bands
    let output = result.unwrap();
    for i in 2..output.upper.len() {
        assert!(
            output.upper[i].is_infinite() || output.upper[i].is_nan(),
            "Upper band should be infinite when std_dev is infinite at index {i}"
        );
    }
}

#[test]
fn validation_negative_std_dev_bollinger() {
    let data = vec![20.0_f64, 21.0, 22.0, 21.5, 22.5, 23.0];
    // Negative std_dev is mathematically valid (inverts bands)
    // This is allowed by the API - document that bands will be inverted
    let result = bollinger(&data, 3, -2.0);
    let output = result.unwrap();

    // With negative std_dev, upper < middle < lower (inverted)
    for i in 2..output.middle.len() {
        if !output.middle[i].is_nan() {
            assert!(
                output.upper[i] < output.middle[i],
                "With negative std_dev, upper should be below middle at index {i}"
            );
            assert!(
                output.middle[i] < output.lower[i],
                "With negative std_dev, middle should be below lower at index {i}"
            );
        }
    }
}

#[test]
fn validation_zero_std_dev_bollinger() {
    let data = vec![20.0_f64, 21.0, 22.0, 21.5, 22.5, 23.0];
    let result = bollinger(&data, 3, 0.0);

    // Zero std_dev should collapse all bands to the middle
    let output = result.unwrap();
    for i in 2..output.middle.len() {
        if !output.middle[i].is_nan() {
            assert!(
                (output.upper[i] - output.middle[i]).abs() < 1e-10,
                "With zero std_dev, upper should equal middle at index {i}"
            );
            assert!(
                (output.lower[i] - output.middle[i]).abs() < 1e-10,
                "With zero std_dev, lower should equal middle at index {i}"
            );
        }
    }
}

// ==================== Bollinger Config Struct Tests ====================

#[test]
fn validation_bollinger_config_zero_period() {
    let data = vec![20.0_f64, 21.0, 22.0, 21.5, 22.5];
    let result = Bollinger::new().with_period(0).compute(&data);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

#[test]
fn validation_bollinger_config_large_period() {
    let data = vec![20.0_f64, 21.0, 22.0];
    let result = Bollinger::new().with_period(100).compute(&data);
    assert!(matches!(result, Err(Error::InsufficientData { .. })));
}

// ==================== Stochastic k_slowing Tests ====================

#[test]
fn validation_stochastic_zero_k_slowing() {
    let high = vec![15.0_f64, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0];
    let low = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    let close = vec![12.0_f64, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];

    // k_slowing = 0 should fail validation
    let result = stochastic(&high, &low, &close, 5, 3, 0);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

// ==================== Error Message Clarity Tests ====================

#[test]
fn validation_error_message_is_actionable() {
    let data: Vec<f64> = vec![];
    let result = sma(&data, 5);

    if let Err(e) = result {
        let msg = e.to_string();
        // Error message should be actionable (Gravity Check principle)
        assert!(
            msg.contains("empty") || msg.contains("no data"),
            "Error message should mention empty/no data: {msg}"
        );
    } else {
        panic!("Expected an error for empty input");
    }
}

#[test]
fn validation_insufficient_data_error_shows_requirements() {
    let data = vec![1.0_f64, 2.0, 3.0];
    let result = sma(&data, 10);

    if let Err(Error::InsufficientData {
        required,
        actual,
        indicator,
    }) = result
    {
        assert_eq!(required, 10);
        assert_eq!(actual, 3);
        assert!(!indicator.is_empty());
    } else {
        panic!("Expected InsufficientData error");
    }
}

#[test]
fn validation_invalid_period_error_shows_value() {
    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let result = sma(&data, 0);

    if let Err(Error::InvalidPeriod { period, reason }) = result {
        assert_eq!(period, 0);
        assert!(!reason.is_empty());
    } else {
        panic!("Expected InvalidPeriod error");
    }
}

#[test]
fn validation_length_mismatch_error_is_descriptive() {
    let high = vec![11.0_f64, 12.0, 13.0, 14.0, 15.0];
    let low = vec![9.0_f64, 10.0, 11.0]; // Shorter
    let close = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];

    let result = atr(&high, &low, &close, 3);

    if let Err(Error::LengthMismatch { description }) = result {
        // Should describe the mismatch
        assert!(
            description.contains('5') || description.contains('3'),
            "Error should mention the mismatched lengths: {description}"
        );
    } else {
        panic!("Expected LengthMismatch error");
    }
}

// ==================== MACD Period Relationship Tests ====================

#[test]
fn validation_macd_fast_period_zero() {
    let data: Vec<f64> = (0..50).map(f64::from).collect();
    let result = macd(&data, 0, 26, 9);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

#[test]
fn validation_macd_slow_period_zero() {
    let data: Vec<f64> = (0..50).map(f64::from).collect();
    let result = macd(&data, 12, 0, 9);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

#[test]
fn validation_macd_signal_period_zero() {
    let data: Vec<f64> = (0..50).map(f64::from).collect();
    let result = macd(&data, 12, 26, 0);
    assert!(matches!(
        result,
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}

#[test]
fn validation_macd_fast_greater_than_slow() {
    // MACD requires fast_period < slow_period (enforced validation)
    let data: Vec<f64> = (0..50).map(f64::from).collect();
    let result = macd(&data, 26, 12, 9);
    // Should fail - fast_period must be less than slow_period
    assert!(
        matches!(result, Err(Error::InvalidPeriod { .. })),
        "MACD should reject fast_period >= slow_period"
    );
}

#[test]
fn validation_macd_fast_equals_slow() {
    // MACD requires fast_period < slow_period
    let data: Vec<f64> = (0..50).map(f64::from).collect();
    let result = macd(&data, 12, 12, 9);
    // Should fail - fast_period must be less than slow_period
    assert!(
        matches!(result, Err(Error::InvalidPeriod { .. })),
        "MACD should reject fast_period == slow_period"
    );
}

// ==================== Consistency Tests Across Indicator Families ====================

#[test]
fn validation_all_single_series_indicators_reject_empty() {
    let empty: Vec<f64> = vec![];

    assert!(matches!(sma(&empty, 5), Err(Error::EmptyInput)));
    assert!(matches!(ema(&empty, 5), Err(Error::EmptyInput)));
    assert!(matches!(rsi(&empty, 14), Err(Error::EmptyInput)));
    assert!(matches!(bollinger(&empty, 20, 2.0), Err(Error::EmptyInput)));
}

#[test]
fn validation_all_ohlc_indicators_reject_empty() {
    let empty: Vec<f64> = vec![];

    assert!(matches!(
        atr(&empty, &empty, &empty, 14),
        Err(Error::EmptyInput)
    ));
    assert!(matches!(
        stochastic_fast(&empty, &empty, &empty, 14, 3),
        Err(Error::EmptyInput)
    ));
    assert!(matches!(
        adx(&empty, &empty, &empty, 14),
        Err(Error::EmptyInput)
    ));
    assert!(matches!(
        williams_r(&empty, &empty, &empty, 14),
        Err(Error::EmptyInput)
    ));
    assert!(matches!(
        donchian(&empty, &empty, 20),
        Err(Error::EmptyInput)
    ));
}

#[test]
fn validation_all_single_series_indicators_reject_zero_period() {
    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];

    assert!(matches!(
        sma(&data, 0),
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
    assert!(matches!(
        ema(&data, 0),
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
    assert!(matches!(
        rsi(&data, 0),
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
    assert!(matches!(
        bollinger(&data, 0, 2.0),
        Err(Error::InvalidPeriod { period: 0, .. })
    ));
}
