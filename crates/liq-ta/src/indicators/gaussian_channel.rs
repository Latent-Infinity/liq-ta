//! Gaussian Channel indicator.
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::bollinger::rolling_stddev;
use crate::indicators::gaussian_filter::{
    gaussian_filter, gaussian_filter_into, gaussian_filter_lookback, gaussian_filter_min_len,
};
use crate::traits::SeriesElement;

#[derive(Debug, Clone)]
pub struct GaussianChannelOutput<T> {
    pub center: Vec<T>,
    pub upper: Vec<T>,
    pub lower: Vec<T>,
    pub trend: Vec<T>,
}

#[inline]
#[must_use]
pub const fn gaussian_channel_lookback(period: usize) -> usize {
    gaussian_filter_lookback(period)
}

#[inline]
#[must_use]
pub const fn gaussian_channel_min_len(period: usize) -> usize {
    gaussian_filter_min_len(period)
}

fn validate<T: SeriesElement>(
    data: &[T],
    period: usize,
    sigma: f64,
    multiplier: f64,
) -> Result<()> {
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
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return Err(Error::LengthMismatch {
            description: "multiplier must be a positive finite number".to_string(),
        });
    }
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    if data.len() < gaussian_channel_min_len(period) {
        return Err(Error::InsufficientData {
            required: gaussian_channel_min_len(period),
            actual: data.len(),
            indicator: "gaussian_channel",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with Gaussian Channel output, which should be used"]
pub fn gaussian_channel<T: SeriesElement>(
    data: &[T],
    period: usize,
    sigma: f64,
    multiplier: f64,
) -> Result<GaussianChannelOutput<T>> {
    validate(data, period, sigma, multiplier)?;
    let center = gaussian_filter(data, period, sigma)?;
    let std = rolling_stddev(data, period)?;
    let mut upper = vec![T::nan(); data.len()];
    let mut lower = vec![T::nan(); data.len()];
    let mut trend = vec![T::nan(); data.len()];
    let mult = T::from_f64(multiplier)?;
    for i in 0..data.len() {
        if center[i].is_finite() && std[i].is_finite() {
            let width = std[i] * mult;
            upper[i] = center[i] + width;
            lower[i] = center[i] - width;
            trend[i] = if data[i] > center[i] {
                T::one()
            } else if data[i] < center[i] {
                T::zero() - T::one()
            } else if i > 0 && trend[i - 1].is_finite() {
                trend[i - 1]
            } else {
                T::zero()
            };
        }
    }
    Ok(GaussianChannelOutput {
        center,
        upper,
        lower,
        trend,
    })
}

#[must_use = "this returns a Result with the count of valid Gaussian Channel values"]
pub fn gaussian_channel_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    sigma: f64,
    multiplier: f64,
    center_out: &mut [T],
    upper_out: &mut [T],
    lower_out: &mut [T],
    trend_out: &mut [T],
) -> Result<usize> {
    validate(data, period, sigma, multiplier)?;
    let n = data.len();
    if center_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: center_out.len(),
            indicator: "gaussian_channel (center)",
        });
    }
    if upper_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: upper_out.len(),
            indicator: "gaussian_channel (upper)",
        });
    }
    if lower_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: lower_out.len(),
            indicator: "gaussian_channel (lower)",
        });
    }
    if trend_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: trend_out.len(),
            indicator: "gaussian_channel (trend)",
        });
    }

    gaussian_filter_into(data, period, sigma, center_out)?;
    let std = rolling_stddev(data, period)?;
    upper_out[..n].fill(T::nan());
    lower_out[..n].fill(T::nan());
    trend_out[..n].fill(T::nan());
    let mult = T::from_f64(multiplier)?;
    for i in 0..n {
        if center_out[i].is_finite() && std[i].is_finite() {
            let width = std[i] * mult;
            upper_out[i] = center_out[i] + width;
            lower_out[i] = center_out[i] - width;
            trend_out[i] = if data[i] > center_out[i] {
                T::one()
            } else if data[i] < center_out[i] {
                T::zero() - T::one()
            } else if i > 0 && trend_out[i - 1].is_finite() {
                trend_out[i - 1]
            } else {
                T::zero()
            };
        } else {
            center_out[i] = T::nan();
        }
    }
    Ok(n.saturating_sub(gaussian_channel_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaussian_channel_shape() {
        let data: Vec<f64> = (0..220).map(|i| 100.0 + i as f64 * 0.2).collect();
        let out = gaussian_channel(&data, 20, 0.5, 2.0).unwrap();
        assert_eq!(out.center.len(), data.len());
        assert_eq!(out.upper.len(), data.len());
        assert_eq!(out.lower.len(), data.len());
        assert_eq!(out.trend.len(), data.len());
    }
}
