//! Rolling autocorrelation.
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub const fn autocorr_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

#[inline]
#[must_use]
pub const fn autocorr_min_len(period: usize) -> usize {
    period
}

fn validate<T: SeriesElement>(data: &[T], period: usize, lag: usize) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if lag == 0 || lag >= period {
        return Err(Error::InvalidPeriod {
            period: lag,
            reason: "lag must be >= 1 and < period",
        });
    }
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    if data.len() < autocorr_min_len(period) {
        return Err(Error::InsufficientData {
            required: autocorr_min_len(period),
            actual: data.len(),
            indicator: "autocorr",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with autocorrelation values, which should be used"]
pub fn autocorr<T: SeriesElement>(data: &[T], period: usize, lag: usize) -> Result<Vec<T>> {
    validate(data, period, lag)?;
    let mut out = vec![T::nan(); data.len()];
    autocorr_into(data, period, lag, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid autocorrelation values"]
pub fn autocorr_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    lag: usize,
    output: &mut [T],
) -> Result<usize> {
    validate(data, period, lag)?;
    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "autocorr",
        });
    }
    output[..data.len()].fill(T::nan());

    let overlap = period - lag;
    let overlap_t = T::from_usize(overlap)?;
    for i in (period - 1)..data.len() {
        let start = i + 1 - period;
        let mut sum_x = T::zero();
        let mut sum_y = T::zero();
        let mut invalid = false;
        for k in 0..overlap {
            let x = data[start + lag + k];
            let y = data[start + k];
            if !x.is_finite() || !y.is_finite() {
                invalid = true;
                break;
            }
            sum_x = sum_x + x;
            sum_y = sum_y + y;
        }
        if invalid {
            continue;
        }
        let mean_x = sum_x / overlap_t;
        let mean_y = sum_y / overlap_t;

        let mut cov = T::zero();
        let mut var_x = T::zero();
        let mut var_y = T::zero();
        for k in 0..overlap {
            let dx = data[start + lag + k] - mean_x;
            let dy = data[start + k] - mean_y;
            cov = cov + dx * dy;
            var_x = var_x + dx * dx;
            var_y = var_y + dy * dy;
        }
        let denom = (var_x * var_y).sqrt();
        if denom > T::zero() {
            output[i] = cov / denom;
        }
    }
    Ok(data.len().saturating_sub(autocorr_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autocorr_shape() {
        let data: Vec<f64> = (0..200).map(|i| (i as f64).sin()).collect();
        let out = autocorr(&data, 32, 1).unwrap();
        assert_eq!(out.len(), data.len());
    }
}
