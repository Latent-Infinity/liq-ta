//! DeMarker indicator.
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub const fn demarker_lookback(period: usize) -> usize {
    period
}

#[inline]
#[must_use]
pub const fn demarker_min_len(period: usize) -> usize {
    period + 1
}

fn validate<T: SeriesElement>(high: &[T], low: &[T], period: usize) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if high.is_empty() {
        return Err(Error::EmptyInput);
    }
    if low.len() != high.len() {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", high.len(), low.len()),
        });
    }
    let required = demarker_min_len(period);
    if high.len() < required {
        return Err(Error::InsufficientData {
            required,
            actual: high.len(),
            indicator: "demarker",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with DeMarker values, which should be used"]
pub fn demarker<T: SeriesElement>(high: &[T], low: &[T], period: usize) -> Result<Vec<T>> {
    validate(high, low, period)?;
    let mut out = vec![T::nan(); high.len()];
    demarker_into(high, low, period, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid DeMarker values"]
pub fn demarker_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    validate(high, low, period)?;
    let n = high.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "demarker",
        });
    }
    output[..n].fill(T::nan());

    let mut demax = vec![T::zero(); n];
    let mut demin = vec![T::zero(); n];
    for i in 1..n {
        if high[i].is_finite() && high[i - 1].is_finite() {
            let diff = high[i] - high[i - 1];
            if diff > T::zero() {
                demax[i] = diff;
            }
        }
        if low[i].is_finite() && low[i - 1].is_finite() {
            let diff = low[i - 1] - low[i];
            if diff > T::zero() {
                demin[i] = diff;
            }
        }
    }

    let period_t = T::from_usize(period)?;
    for i in period..n {
        let mut sum_max = T::zero();
        let mut sum_min = T::zero();
        for j in (i + 1 - period)..=i {
            sum_max = sum_max + demax[j];
            sum_min = sum_min + demin[j];
        }
        let avg_max = sum_max / period_t;
        let avg_min = sum_min / period_t;
        let denom = avg_max + avg_min;
        output[i] = if denom > T::zero() {
            avg_max / denom
        } else {
            T::fifty() / T::hundred()
        };
    }
    Ok(n.saturating_sub(demarker_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demarker_bounds() {
        let high: Vec<f64> = (0..100).map(|i| 100.0 + i as f64 * 0.5).collect();
        let low: Vec<f64> = high.iter().map(|v| v - 2.0).collect();
        let out = demarker(&high, &low, 14).unwrap();
        let valid = out.iter().copied().filter(|v| v.is_finite());
        for v in valid {
            assert!((0.0..=1.0).contains(&v));
        }
    }
}
