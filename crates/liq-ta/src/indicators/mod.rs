//! Technical analysis indicators.
//!
//! This module provides implementations of common technical analysis indicators
//! used for analyzing price data and identifying trading signals.
//!
//! # Overview
//!
//! All indicators in this module share the following properties:
//!
//! - **Generic**: Work with both `f32` and `f64` types via the
//!   [`SeriesElement`](crate::traits::SeriesElement) trait
//! - **Efficient**: O(n) time complexity using optimized rolling algorithms
//! - **NaN-aware**: Handle missing data in lookback periods gracefully
//! - **Error-safe**: Return typed errors for edge cases (insufficient data, invalid periods)
//!
//! # Indicator Categories
//!
//! ## Trend Indicators
//!
//! - [`sma()`] - Simple Moving Average: arithmetic mean over a rolling window
//! - [`ema()`] - Exponential Moving Average: weighted average emphasizing recent data
//! - [`ema_wilder()`] - Wilder's EMA: uses smoothing factor α = 1/period
//! - [`wma()`] - Weighted Moving Average: linearly weighted average emphasizing recent data
//! - [`dema()`] - Double Exponential Moving Average: reduced lag via double smoothing
//! - [`tema()`] - Triple Exponential Moving Average: further reduced lag via triple smoothing
//! - [`trima()`] - Triangular Moving Average: double-smoothed SMA for extra smoothness
//! - [`macd()`] - MACD: trend-following momentum using EMA differences
//! - [`adx()`] - Average Directional Index: measures trend strength (not direction)
//! - [`donchian()`] - Donchian Channels: price channels for breakout identification
//!
//! ## Momentum Indicators
//!
//! - [`rsi()`] - Relative Strength Index: measures speed and magnitude of price changes
//! - [`stochastic()`] - Stochastic Oscillator: compares closing price to price range (canonical API)
//! - [`stochastic_fast()`], [`stochastic_slow()`], [`stochastic_full()`] - Convenience variants
//! - [`williams_r()`] - Williams %R: momentum oscillator comparing close to high-low range
//!
//! ## Volatility Indicators
//!
//! - [`atr()`] - Average True Range: measures market volatility using price ranges
//! - [`true_range()`] - True Range: single-period volatility component
//! - [`bollinger()`] - Bollinger Bands: price envelope based on standard deviation
//! - [`rolling_stddev()`] - Rolling Standard Deviation: statistical dispersion measure
//!
//! ## Volume Indicators
//!
//! - [`obv()`] - On-Balance Volume: cumulative volume flow to predict price changes
//! - [`vwap()`] - Volume Weighted Average Price: average price weighted by volume
//!
//! # Example
//!
//! ```
//! use liq_ta::indicators::{sma, ema, rsi};
//!
//! let prices = vec![44.0_f64, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0];
//!
//! // Calculate a 5-period Simple Moving Average
//! let sma_result = sma(&prices, 5).unwrap();
//!
//! // Calculate a 5-period Exponential Moving Average
//! let ema_result = ema(&prices, 5).unwrap();
//!
//! // Calculate the 5-period RSI
//! let rsi_result = rsi(&prices, 5).unwrap();
//! ```
//!
//! # NaN Handling
//!
//! All indicators return NaN values for the lookback period (typically `period - 1`
//! elements at the start). This design ensures output arrays have the same length
//! as input arrays, simplifying alignment with original data.
//!
//! # Error Handling
//!
//! Indicators return [`Result<T, Error>`](crate::error::Error) to handle:
//!
//! - Empty input data ([`EmptyInput`](crate::error::Error::EmptyInput))
//! - Invalid period values ([`InvalidPeriod`](crate::error::Error::InvalidPeriod))
//! - Insufficient data for the requested period
//!   ([`InsufficientData`](crate::error::Error::InsufficientData))

pub mod ad;
pub mod adosc;
pub mod adx;
pub mod ao;
pub mod apo;
pub mod aroon;
pub mod atr;
pub mod autocorr;
pub mod bollinger;
pub mod bop;
pub mod candlestick;
pub mod cci;
pub mod chop;
pub mod cmo;
pub mod composite_bands;
pub mod connors_rsi;
pub mod dema;
pub mod demarker;
pub mod donchian;
pub mod dpo;
pub mod dss_bressert;
pub mod dx;
pub mod ema;
pub mod gaussian_channel;
pub mod gaussian_filter;
pub mod hma;
pub mod ht_core;
pub mod ht_dcperiod;
pub mod ht_dcphase;
pub mod ht_phasor;
pub mod ht_sine;
pub mod ht_trendline;
pub mod ht_trendmode;
pub mod hurst;
pub mod ichimoku;
pub mod kama;
pub mod keltner;
pub mod laguerre_rsi;
pub mod macd;
pub mod mama;
pub mod mavp;
pub mod mfi;
pub mod midpoint;
pub mod midprice;
pub mod mom;
pub mod obv;
pub mod osma;
pub mod power;
pub mod price_transform;
pub mod qqe;
pub mod roc;
pub mod rsi;
pub mod rvi;
pub mod sar;
pub mod sarext;
pub mod sma;
pub mod statistics;
pub mod stc;
pub mod stochastic;
pub mod stochrsi;
pub mod supertrend;
pub mod t3;
pub mod tema;
pub mod trima;
pub mod trix;
pub mod ulcer_index;
pub mod ultosc;
pub mod vortex;
pub mod vwap;
pub mod williams_r;
pub mod wma;

// Re-export indicator functions for convenient access.
//
// These re-exports allow users to import directly from `indicators` without
// needing to specify the submodule, e.g., `use liq_ta::indicators::sma;`

// ADX (Average Directional Index)
pub use adx::{AdxOutput, adx, adx_into, adx_lookback, adx_min_len, di_lookback};

// ATR and True Range
pub use atr::{
    atr, atr_into, atr_lookback, atr_min_len, true_range, true_range_into, true_range_lookback,
};

// Bollinger Bands (uses SIMD internally for f64 when the simd feature is enabled)
pub use bollinger::{
    Bollinger, BollingerOutput, bollinger, bollinger_into, bollinger_lookback, bollinger_min_len,
    rolling_stddev, rolling_stddev_into,
};

// Double Exponential Moving Average
pub use dema::{dema, dema_into, dema_lookback, dema_min_len};

// Triple Exponential Moving Average
pub use tema::{tema, tema_into, tema_lookback, tema_min_len};

// Triangular Moving Average
pub use trima::{trima, trima_into, trima_lookback, trima_min_len};

// Donchian Channels
pub use donchian::{DonchianOutput, donchian, donchian_into, donchian_lookback, donchian_min_len};

// Keltner Channels
pub use keltner::{
    KeltnerChannelOutput, keltner_channel, keltner_channel_into, keltner_channel_lookback,
    keltner_channel_min_len,
};

// Ichimoku Kinko Hyo
pub use ichimoku::{IchimokuOutput, ichimoku, ichimoku_into, ichimoku_lookback, ichimoku_min_len};

// Exponential Moving Average
pub use ema::{
    ema, ema_into, ema_lookback, ema_min_len, ema_wilder, ema_wilder_into, ema_with_alpha,
    ema_with_alpha_into,
};

// KAMA (Kaufman Adaptive Moving Average)
pub use kama::{kama, kama_full, kama_full_into, kama_into, kama_lookback, kama_min_len};

// MACD
pub use macd::{
    Macd, MacdOutput, macd, macd_into, macd_line_lookback, macd_min_len, macd_signal_lookback,
};

// MIDPOINT
pub use midpoint::{midpoint, midpoint_into, midpoint_lookback, midpoint_min_len};

// MIDPRICE
pub use midprice::{midprice, midprice_into, midprice_lookback, midprice_min_len};

// OBV (On-Balance Volume)
pub use obv::{obv, obv_into, obv_lookback, obv_min_len};

// RSI
pub use rsi::{rsi, rsi_into, rsi_lookback, rsi_min_len};

// QQE
pub use qqe::{QqeOutput, qqe, qqe_into, qqe_lookback, qqe_min_len};

// Simple Moving Average (uses SIMD internally for f64 when the simd feature is enabled)
pub use sma::{sma, sma_from_idx_into, sma_into, sma_lookback, sma_min_len};

// Stochastic Oscillator
pub use stochastic::{
    Stochastic, StochasticOutput, stochastic, stochastic_d_lookback, stochastic_fast,
    stochastic_fast_into, stochastic_full, stochastic_full_into, stochastic_into,
    stochastic_k_lookback, stochastic_min_len, stochastic_slow, stochastic_slow_into,
};

// VWAP (Volume Weighted Average Price)
pub use vwap::{vwap, vwap_into, vwap_lookback, vwap_min_len};

// Williams %R
pub use williams_r::{williams_r, williams_r_into, williams_r_lookback, williams_r_min_len};

// Weighted Moving Average
pub use wma::{wma, wma_into, wma_lookback, wma_min_len};

// T3 (Tillson T3 Moving Average)
pub use t3::{t3, t3_full, t3_full_into, t3_into, t3_lookback, t3_min_len};

// SAR (Parabolic Stop and Reverse)
pub use sar::{sar, sar_full, sar_full_into, sar_into, sar_lookback, sar_min_len};

// SAREXT (Extended Parabolic SAR)
pub use sarext::{
    SarExtParams, sarext, sarext_full, sarext_full_into, sarext_into, sarext_lookback,
    sarext_min_len,
};

// HT_TRENDLINE (Hilbert Transform - Instantaneous Trendline)
pub use ht_trendline::{
    ht_trendline, ht_trendline_into, ht_trendline_lookback, ht_trendline_min_len,
};

// MAMA (MESA Adaptive Moving Average)
pub use mama::{
    MamaOutput, mama, mama_full, mama_full_into, mama_into, mama_lookback, mama_min_len,
};

// MOM (Momentum)
pub use mom::{mom, mom_into, mom_lookback, mom_min_len};

// ROC family (Rate of Change)
pub use roc::{
    roc, roc_into, roc_lookback, roc_min_len, rocp, rocp_into, rocp_lookback, rocp_min_len, rocr,
    rocr_into, rocr_lookback, rocr_min_len, rocr100, rocr100_into, rocr100_lookback,
    rocr100_min_len,
};

// APO (Absolute Price Oscillator) and PPO (Percentage Price Oscillator)
pub use apo::{apo, apo_into, apo_lookback, apo_min_len, ppo, ppo_into, ppo_lookback, ppo_min_len};

// AO (Awesome Oscillator)
pub use ao::{ao, ao_into, ao_lookback, ao_min_len};

// HMA (Hull Moving Average)
pub use hma::{hma, hma_into, hma_lookback, hma_min_len};

// Gaussian filter and channel
pub use gaussian_channel::{
    GaussianChannelOutput, gaussian_channel, gaussian_channel_into, gaussian_channel_lookback,
    gaussian_channel_min_len,
};
pub use gaussian_filter::{
    gaussian_filter, gaussian_filter_into, gaussian_filter_lookback, gaussian_filter_min_len,
};

// SuperTrend
pub use supertrend::{
    SuperTrendOutput, supertrend, supertrend_into, supertrend_lookback, supertrend_min_len,
};

// Bulls/Bears Power
pub use power::{
    bears_power, bears_power_into, bulls_power, bulls_power_into, power_lookback, power_min_len,
};

// DeMarker
pub use demarker::{demarker, demarker_into, demarker_lookback, demarker_min_len};

// OSMA
pub use osma::{osma, osma_into, osma_lookback, osma_min_len};

// Vortex
pub use vortex::{VortexOutput, vortex, vortex_into, vortex_lookback, vortex_min_len};

// RVI
pub use rvi::{rvi, rvi_into, rvi_lookback, rvi_min_len};

// DPO
pub use dpo::{dpo, dpo_into, dpo_lookback, dpo_min_len, dpo_shift};

// Connors RSI
pub use connors_rsi::{connors_rsi, connors_rsi_into, connors_rsi_lookback, connors_rsi_min_len};

// STC
pub use stc::{stc, stc_into, stc_lookback, stc_min_len};

// Laguerre RSI
pub use laguerre_rsi::{
    laguerre_rsi, laguerre_rsi_into, laguerre_rsi_lookback, laguerre_rsi_min_len,
};

// DSS Bressert
pub use dss_bressert::{
    dss_bressert, dss_bressert_into, dss_bressert_lookback, dss_bressert_min_len,
};

// CHOP
pub use chop::{chop, chop_into, chop_lookback, chop_min_len};

// Ulcer Index
pub use ulcer_index::{ulcer_index, ulcer_index_into, ulcer_index_lookback, ulcer_index_min_len};

// Hurst
pub use hurst::{hurst, hurst_into, hurst_lookback, hurst_min_len};

// Autocorrelation
pub use autocorr::{autocorr, autocorr_into, autocorr_lookback, autocorr_min_len};

// Composite bands
pub use composite_bands::{
    CompositeBandsOutput, hma_atr_bands, hma_atr_bands_into, hma_bollinger_bands,
    hma_bollinger_bands_into, vwap_atr_bands, vwap_atr_bands_into, vwap_bollinger_bands,
    vwap_bollinger_bands_into,
};

// BOP (Balance of Power)
pub use bop::{bop, bop_into, bop_lookback, bop_min_len};

// AROON
pub use aroon::{AroonOutput, aroon, aroon_into, aroon_lookback, aroon_min_len};

// AROONOSC (Aroon Oscillator)
pub use aroon::{aroonosc, aroonosc_into, aroonosc_lookback, aroonosc_min_len};

// CCI (Commodity Channel Index)
pub use cci::{cci, cci_into, cci_lookback, cci_min_len};

// CMO (Chande Momentum Oscillator)
pub use cmo::{cmo, cmo_into, cmo_lookback, cmo_min_len};

// MFI (Money Flow Index)
pub use mfi::{mfi, mfi_into, mfi_lookback, mfi_min_len};

// STOCHRSI (Stochastic RSI)
pub use stochrsi::{
    StochRsiOutput, stochrsi, stochrsi_d_lookback, stochrsi_default, stochrsi_into,
    stochrsi_k_lookback, stochrsi_min_len,
};

// TRIX
pub use trix::{trix, trix_into, trix_lookback, trix_min_len};

// ULTOSC (Ultimate Oscillator)
pub use ultosc::{ultosc, ultosc_default, ultosc_into, ultosc_lookback, ultosc_min_len};

// DX family (Directional Movement indicators)
pub use dx::{
    adxr, adxr_into, adxr_lookback, adxr_min_len, dm_lookback, dm_min_len, dx, dx_into,
    dx_lookback, dx_min_len, minus_dm, minus_dm_into, plus_dm, plus_dm_into,
};

// AD (Chaikin Accumulation/Distribution Line)
pub use ad::{ad, ad_into, ad_lookback, ad_min_len};

// ADOSC (Chaikin A/D Oscillator)
pub use adosc::{adosc, adosc_default, adosc_into, adosc_lookback, adosc_min_len};

// Hilbert Transform Core
pub use ht_core::{HilbertState, hilbert_transform, ht_lookback, ht_min_len};

// HT_DCPERIOD (Hilbert Transform - Dominant Cycle Period)
pub use ht_dcperiod::{ht_dcperiod, ht_dcperiod_into, ht_dcperiod_lookback, ht_dcperiod_min_len};

// HT_DCPHASE (Hilbert Transform - Dominant Cycle Phase)
pub use ht_dcphase::{ht_dcphase, ht_dcphase_into, ht_dcphase_lookback, ht_dcphase_min_len};

// HT_PHASOR (Hilbert Transform - Phasor Components)
pub use ht_phasor::{
    HtPhasorOutput, ht_phasor, ht_phasor_into, ht_phasor_lookback, ht_phasor_min_len,
};

// HT_SINE (Hilbert Transform - SineWave)
pub use ht_sine::{HtSineOutput, ht_sine, ht_sine_into, ht_sine_lookback, ht_sine_min_len};

// HT_TRENDMODE (Hilbert Transform - Trend vs Cycle Mode)
pub use ht_trendmode::{
    ht_trendmode, ht_trendmode_into, ht_trendmode_lookback, ht_trendmode_min_len,
};

// MAVP (Moving Average Variable Period)
pub use mavp::{mavp, mavp_default, mavp_into, mavp_lookback, mavp_min_len};

// Price Transform Indicators
pub use price_transform::{
    avgprice, avgprice_into, avgprice_lookback, avgprice_min_len, medprice, medprice_into,
    medprice_lookback, medprice_min_len, typprice, typprice_into, typprice_lookback,
    typprice_min_len, wclprice, wclprice_into, wclprice_lookback, wclprice_min_len,
};

// Statistical Functions
pub use statistics::{
    beta, beta_into, beta_lookback, beta_min_len, correl, correl_into, correl_lookback,
    correl_min_len, linearreg, linearreg_angle, linearreg_angle_into, linearreg_angle_lookback,
    linearreg_angle_min_len, linearreg_intercept, linearreg_intercept_into,
    linearreg_intercept_lookback, linearreg_intercept_min_len, linearreg_into, linearreg_lookback,
    linearreg_min_len, linearreg_slope, linearreg_slope_into, linearreg_slope_lookback,
    linearreg_slope_min_len, tsf, tsf_into, tsf_lookback, tsf_min_len, var, var_into, var_lookback,
    var_min_len,
};
