//! Hull Moving Average (HMA).
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::wma::{wma, wma_into, wma_lookback};
use crate::traits::SeriesElement;

#[inline]
fn half_period(period: usize) -> usize {
    (period / 2).max(1)
}

#[inline]
fn sqrt_period(period: usize) -> usize {
    (period as f64).sqrt().round().max(1.0) as usize
}

#[inline]
#[must_use]
pub fn hma_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        wma_lookback(period) + wma_lookback(sqrt_period(period))
    }
}

#[inline]
#[must_use]
pub fn hma_min_len(period: usize) -> usize {
    hma_lookback(period) + 1
}

#[must_use = "this returns a Result with HMA values, which should be used"]
pub fn hma<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    let required = hma_min_len(period);
    if data.len() < required {
        return Err(Error::InsufficientData {
            required,
            actual: data.len(),
            indicator: "hma",
        });
    }

    let n = data.len();
    let half = half_period(period);
    let sqrt_p = sqrt_period(period);
    let two = T::two();

    let wma_half = wma(data, half)?;
    let wma_full = wma(data, period)?;
    let mut diff = vec![T::nan(); n];
    for i in 0..n {
        if wma_half[i].is_finite() && wma_full[i].is_finite() {
            diff[i] = two * wma_half[i] - wma_full[i];
        }
    }
    wma(&diff, sqrt_p)
}

#[must_use = "this returns a Result with the count of valid HMA values"]
pub fn hma_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    let n = data.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "hma",
        });
    }
    let required = hma_min_len(period);
    if n < required {
        return Err(Error::InsufficientData {
            required,
            actual: n,
            indicator: "hma",
        });
    }

    let half = half_period(period);
    let sqrt_p = sqrt_period(period);
    let two = T::two();
    let mut wma_half = vec![T::nan(); n];
    let mut wma_full = vec![T::nan(); n];
    let mut diff = vec![T::nan(); n];
    wma_into(data, half, &mut wma_half)?;
    wma_into(data, period, &mut wma_full)?;

    for i in 0..n {
        if wma_half[i].is_finite() && wma_full[i].is_finite() {
            diff[i] = two * wma_half[i] - wma_full[i];
        } else {
            diff[i] = T::nan();
        }
    }
    wma_into(&diff, sqrt_p, output)?;
    Ok(n.saturating_sub(hma_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hma_shape() {
        let data: Vec<f64> = (0..100).map(|i| 100.0 + i as f64).collect();
        let out = hma(&data, 20).unwrap();
        assert_eq!(out.len(), data.len());
        assert!(out[..hma_lookback(20)].iter().all(|v| v.is_nan()));
        assert!(out[hma_lookback(20)..].iter().all(|v| v.is_finite()));
    }

    #[test]
    fn test_hma_into_matches() {
        let data: Vec<f64> = (0..120).map(|i| 80.0 + (i as f64 * 0.2)).collect();
        let mut out = vec![0.0; data.len()];
        let valid = hma_into(&data, 16, &mut out).unwrap();
        let direct = hma(&data, 16).unwrap();
        assert_eq!(valid, data.len() - hma_lookback(16));
        for i in 0..out.len() {
            if out[i].is_nan() || direct[i].is_nan() {
                assert!(out[i].is_nan() && direct[i].is_nan());
            } else {
                assert!((out[i] - direct[i]).abs() < 1e-12);
            }
        }
    }
}
