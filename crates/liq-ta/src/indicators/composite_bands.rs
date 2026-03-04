//! Composite band indicators built from base indicators.
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::atr::{atr_into, atr_lookback};
use crate::indicators::bollinger::{rolling_stddev, rolling_stddev_into};
use crate::indicators::hma::{hma_into, hma_lookback};
use crate::indicators::vwap::{vwap_into, vwap_lookback};
use crate::traits::SeriesElement;

#[derive(Debug, Clone)]
pub struct CompositeBandsOutput<T> {
    pub upper: Vec<T>,
    pub middle: Vec<T>,
    pub lower: Vec<T>,
}

fn validate_positive_finite(name: &str, value: f64) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(Error::LengthMismatch {
            description: format!("{name} must be a positive finite number"),
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with HMA ATR Bands, which should be used"]
pub fn hma_atr_bands<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    hma_period: usize,
    atr_period: usize,
    atr_multiplier: f64,
) -> Result<CompositeBandsOutput<T>> {
    validate_positive_finite("atr_multiplier", atr_multiplier)?;
    let n = close.len();
    let mut out = CompositeBandsOutput {
        upper: vec![T::nan(); n],
        middle: vec![T::nan(); n],
        lower: vec![T::nan(); n],
    };
    hma_atr_bands_into(
        high,
        low,
        close,
        hma_period,
        atr_period,
        atr_multiplier,
        &mut out.upper,
        &mut out.middle,
        &mut out.lower,
    )?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid HMA ATR Band values"]
#[allow(clippy::too_many_arguments)]
pub fn hma_atr_bands_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    hma_period: usize,
    atr_period: usize,
    atr_multiplier: f64,
    upper_out: &mut [T],
    middle_out: &mut [T],
    lower_out: &mut [T],
) -> Result<usize> {
    validate_positive_finite("atr_multiplier", atr_multiplier)?;
    let n = close.len();
    if upper_out.len() < n || middle_out.len() < n || lower_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: upper_out.len().min(middle_out.len()).min(lower_out.len()),
            indicator: "hma_atr_bands",
        });
    }
    let mut atr_vals = vec![T::nan(); n];
    atr_into(high, low, close, atr_period, &mut atr_vals)?;
    hma_into(close, hma_period, middle_out)?;
    let mult = T::from_f64(atr_multiplier)?;
    upper_out[..n].fill(T::nan());
    lower_out[..n].fill(T::nan());
    for i in 0..n {
        if middle_out[i].is_finite() && atr_vals[i].is_finite() {
            let width = atr_vals[i] * mult;
            upper_out[i] = middle_out[i] + width;
            lower_out[i] = middle_out[i] - width;
        } else {
            middle_out[i] = T::nan();
        }
    }
    let lookback = hma_lookback(hma_period).max(atr_lookback(atr_period));
    Ok(n.saturating_sub(lookback))
}

#[must_use = "this returns a Result with HMA Bollinger Bands, which should be used"]
pub fn hma_bollinger_bands<T: SeriesElement>(
    data: &[T],
    hma_period: usize,
    std_period: usize,
    std_multiplier: f64,
) -> Result<CompositeBandsOutput<T>> {
    validate_positive_finite("std_multiplier", std_multiplier)?;
    let n = data.len();
    let mut out = CompositeBandsOutput {
        upper: vec![T::nan(); n],
        middle: vec![T::nan(); n],
        lower: vec![T::nan(); n],
    };
    hma_bollinger_bands_into(
        data,
        hma_period,
        std_period,
        std_multiplier,
        &mut out.upper,
        &mut out.middle,
        &mut out.lower,
    )?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid HMA Bollinger Band values"]
#[allow(clippy::too_many_arguments)]
pub fn hma_bollinger_bands_into<T: SeriesElement>(
    data: &[T],
    hma_period: usize,
    std_period: usize,
    std_multiplier: f64,
    upper_out: &mut [T],
    middle_out: &mut [T],
    lower_out: &mut [T],
) -> Result<usize> {
    validate_positive_finite("std_multiplier", std_multiplier)?;
    let n = data.len();
    if upper_out.len() < n || middle_out.len() < n || lower_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: upper_out.len().min(middle_out.len()).min(lower_out.len()),
            indicator: "hma_bollinger_bands",
        });
    }
    hma_into(data, hma_period, middle_out)?;
    let mut std = vec![T::nan(); n];
    rolling_stddev_into(middle_out, std_period, &mut std)?;
    let mult = T::from_f64(std_multiplier)?;
    upper_out[..n].fill(T::nan());
    lower_out[..n].fill(T::nan());
    for i in 0..n {
        if middle_out[i].is_finite() && std[i].is_finite() {
            let width = std[i] * mult;
            upper_out[i] = middle_out[i] + width;
            lower_out[i] = middle_out[i] - width;
        } else {
            middle_out[i] = T::nan();
        }
    }
    let lookback = hma_lookback(hma_period) + std_period.saturating_sub(1);
    Ok(n.saturating_sub(lookback))
}

#[must_use = "this returns a Result with VWAP ATR Bands, which should be used"]
pub fn vwap_atr_bands<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    atr_period: usize,
    atr_multiplier: f64,
) -> Result<CompositeBandsOutput<T>> {
    validate_positive_finite("atr_multiplier", atr_multiplier)?;
    let n = close.len();
    let mut out = CompositeBandsOutput {
        upper: vec![T::nan(); n],
        middle: vec![T::nan(); n],
        lower: vec![T::nan(); n],
    };
    vwap_atr_bands_into(
        high,
        low,
        close,
        volume,
        atr_period,
        atr_multiplier,
        &mut out.upper,
        &mut out.middle,
        &mut out.lower,
    )?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid VWAP ATR Band values"]
#[allow(clippy::too_many_arguments)]
pub fn vwap_atr_bands_into<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    atr_period: usize,
    atr_multiplier: f64,
    upper_out: &mut [T],
    middle_out: &mut [T],
    lower_out: &mut [T],
) -> Result<usize> {
    validate_positive_finite("atr_multiplier", atr_multiplier)?;
    let n = close.len();
    if upper_out.len() < n || middle_out.len() < n || lower_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: upper_out.len().min(middle_out.len()).min(lower_out.len()),
            indicator: "vwap_atr_bands",
        });
    }
    let mut atr_vals = vec![T::nan(); n];
    atr_into(high, low, close, atr_period, &mut atr_vals)?;
    vwap_into(high, low, close, volume, middle_out)?;
    let mult = T::from_f64(atr_multiplier)?;
    upper_out[..n].fill(T::nan());
    lower_out[..n].fill(T::nan());
    for i in 0..n {
        if middle_out[i].is_finite() && atr_vals[i].is_finite() {
            let width = atr_vals[i] * mult;
            upper_out[i] = middle_out[i] + width;
            lower_out[i] = middle_out[i] - width;
        }
    }
    let lookback = vwap_lookback().max(atr_lookback(atr_period));
    Ok(n.saturating_sub(lookback))
}

#[must_use = "this returns a Result with VWAP Bollinger Bands, which should be used"]
pub fn vwap_bollinger_bands<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    std_period: usize,
    std_multiplier: f64,
) -> Result<CompositeBandsOutput<T>> {
    validate_positive_finite("std_multiplier", std_multiplier)?;
    let n = close.len();
    let mut out = CompositeBandsOutput {
        upper: vec![T::nan(); n],
        middle: vec![T::nan(); n],
        lower: vec![T::nan(); n],
    };
    vwap_bollinger_bands_into(
        high,
        low,
        close,
        volume,
        std_period,
        std_multiplier,
        &mut out.upper,
        &mut out.middle,
        &mut out.lower,
    )?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid VWAP Bollinger Band values"]
#[allow(clippy::too_many_arguments)]
pub fn vwap_bollinger_bands_into<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    std_period: usize,
    std_multiplier: f64,
    upper_out: &mut [T],
    middle_out: &mut [T],
    lower_out: &mut [T],
) -> Result<usize> {
    validate_positive_finite("std_multiplier", std_multiplier)?;
    let n = close.len();
    if upper_out.len() < n || middle_out.len() < n || lower_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: upper_out.len().min(middle_out.len()).min(lower_out.len()),
            indicator: "vwap_bollinger_bands",
        });
    }
    vwap_into(high, low, close, volume, middle_out)?;
    let std = rolling_stddev(middle_out, std_period)?;
    let mult = T::from_f64(std_multiplier)?;
    upper_out[..n].fill(T::nan());
    lower_out[..n].fill(T::nan());
    for i in 0..n {
        if middle_out[i].is_finite() && std[i].is_finite() {
            let width = std[i] * mult;
            upper_out[i] = middle_out[i] + width;
            lower_out[i] = middle_out[i] - width;
        } else {
            middle_out[i] = T::nan();
        }
    }
    let lookback = std_period.saturating_sub(1);
    Ok(n.saturating_sub(lookback))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ohlcv(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.3).collect();
        let high: Vec<f64> = close.iter().map(|v| v + 0.8).collect();
        let low: Vec<f64> = close.iter().map(|v| v - 0.8).collect();
        let volume: Vec<f64> = (0..n).map(|i| 1000.0 + (i % 7) as f64 * 20.0).collect();
        (high, low, close, volume)
    }

    #[test]
    fn test_composite_shapes() {
        let (high, low, close, volume) = sample_ohlcv(180);
        let hma_atr = hma_atr_bands(&high, &low, &close, 21, 14, 2.0).unwrap();
        let hma_bb = hma_bollinger_bands(&close, 21, 20, 2.0).unwrap();
        let vwap_atr = vwap_atr_bands(&high, &low, &close, &volume, 14, 2.0).unwrap();
        let vwap_bb = vwap_bollinger_bands(&high, &low, &close, &volume, 20, 2.0).unwrap();
        assert_eq!(hma_atr.middle.len(), close.len());
        assert_eq!(hma_bb.middle.len(), close.len());
        assert_eq!(vwap_atr.middle.len(), close.len());
        assert_eq!(vwap_bb.middle.len(), close.len());
    }
}
