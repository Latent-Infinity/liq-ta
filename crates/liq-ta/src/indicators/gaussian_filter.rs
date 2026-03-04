//! Gaussian smoothing filter.
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub const fn gaussian_filter_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

#[inline]
#[must_use]
pub const fn gaussian_filter_min_len(period: usize) -> usize {
    period
}

fn validate<T: SeriesElement>(data: &[T], period: usize, sigma: f64) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if !sigma.is_finite() || sigma <= 0.0 {
        return Err(Error::LengthMismatch {
            description: "sigma must be a positive finite number".to_string(),
        });
    }
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    if data.len() < gaussian_filter_min_len(period) {
        return Err(Error::InsufficientData {
            required: gaussian_filter_min_len(period),
            actual: data.len(),
            indicator: "gaussian_filter",
        });
    }
    Ok(())
}

fn gaussian_weights(period: usize, sigma: f64) -> Vec<f64> {
    let center = (period - 1) as f64;
    let std = (sigma * period as f64 / 3.0).max(1e-12);
    let denom = 2.0 * std * std;
    let mut w = Vec::with_capacity(period);
    for i in 0..period {
        let x = i as f64 - center;
        w.push((-x * x / denom).exp());
    }
    let sum: f64 = w.iter().sum();
    if sum > 0.0 {
        for v in &mut w {
            *v /= sum;
        }
    }
    w
}

#[must_use = "this returns a Result with Gaussian-filtered values, which should be used"]
pub fn gaussian_filter<T: SeriesElement>(data: &[T], period: usize, sigma: f64) -> Result<Vec<T>> {
    validate(data, period, sigma)?;
    let mut out = vec![T::nan(); data.len()];
    gaussian_filter_into(data, period, sigma, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid Gaussian-filtered values"]
pub fn gaussian_filter_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    sigma: f64,
    output: &mut [T],
) -> Result<usize> {
    validate(data, period, sigma)?;
    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "gaussian_filter",
        });
    }
    output[..data.len()].fill(T::nan());
    let weights = gaussian_weights(period, sigma);
    for i in (period - 1)..data.len() {
        let start = i + 1 - period;
        let mut sum = T::zero();
        let mut valid = true;
        for (j, w) in weights.iter().enumerate() {
            let value = data[start + j];
            if !value.is_finite() {
                valid = false;
                break;
            }
            sum = sum + value * T::from_f64(*w)?;
        }
        if valid {
            output[i] = sum;
        }
    }
    Ok(data.len().saturating_sub(gaussian_filter_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaussian_filter_shape() {
        let data: Vec<f64> = (0..160).map(|i| 100.0 + i as f64 * 0.15).collect();
        let out = gaussian_filter(&data, 20, 0.5).unwrap();
        assert_eq!(out.len(), data.len());
        assert!(out[..19].iter().all(|v| v.is_nan()));
    }
}
