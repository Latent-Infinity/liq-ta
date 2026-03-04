//! Hurst Exponent (rolling R/S estimate).
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub const fn hurst_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

#[inline]
#[must_use]
pub const fn hurst_min_len(period: usize) -> usize {
    period
}

fn validate<T: SeriesElement>(data: &[T], period: usize) -> Result<()> {
    if period < 2 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 2",
        });
    }
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    if data.len() < hurst_min_len(period) {
        return Err(Error::InsufficientData {
            required: hurst_min_len(period),
            actual: data.len(),
            indicator: "hurst",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with Hurst Exponent values, which should be used"]
pub fn hurst<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    validate(data, period)?;
    let mut out = vec![T::nan(); data.len()];
    hurst_into(data, period, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid Hurst values"]
pub fn hurst_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize> {
    validate(data, period)?;
    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "hurst",
        });
    }
    output[..data.len()].fill(T::nan());
    let period_t = T::from_usize(period)?;
    let log_period = period_t.ln();
    for i in (period - 1)..data.len() {
        let start = i + 1 - period;
        let window = &data[start..=i];
        if window.iter().any(|v| !v.is_finite()) {
            continue;
        }
        let mean = window.iter().copied().fold(T::zero(), |acc, v| acc + v) / period_t;
        let mut cumulative = T::zero();
        let mut min_cum = T::zero();
        let mut max_cum = T::zero();
        let mut var_sum = T::zero();
        for &v in window {
            let dev = v - mean;
            cumulative = cumulative + dev;
            if cumulative < min_cum {
                min_cum = cumulative;
            }
            if cumulative > max_cum {
                max_cum = cumulative;
            }
            var_sum = var_sum + dev * dev;
        }
        let r = max_cum - min_cum;
        let s = (var_sum / period_t).sqrt();
        if r > T::zero() && s > T::zero() {
            output[i] = (r / s).ln() / log_period;
        }
    }
    Ok(data.len().saturating_sub(hurst_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hurst_shape() {
        let data: Vec<f64> = (0..220).map(|i| 100.0 + i as f64 * 0.07).collect();
        let out = hurst(&data, 64).unwrap();
        assert_eq!(out.len(), data.len());
    }
}
