//! Ulcer Index indicator.
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub const fn ulcer_index_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

#[inline]
#[must_use]
pub const fn ulcer_index_min_len(period: usize) -> usize {
    period
}

fn validate<T: SeriesElement>(data: &[T], period: usize) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    if data.len() < ulcer_index_min_len(period) {
        return Err(Error::InsufficientData {
            required: ulcer_index_min_len(period),
            actual: data.len(),
            indicator: "ulcer_index",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with Ulcer Index values, which should be used"]
pub fn ulcer_index<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    validate(data, period)?;
    let mut out = vec![T::nan(); data.len()];
    ulcer_index_into(data, period, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid Ulcer Index values"]
pub fn ulcer_index_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    validate(data, period)?;
    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "ulcer_index",
        });
    }
    output[..data.len()].fill(T::nan());
    let hundred = T::hundred();
    let period_t = T::from_usize(period)?;
    for i in (period - 1)..data.len() {
        let start = i + 1 - period;
        let mut peak = T::neg_infinity();
        for &value in data.iter().take(i + 1).skip(start) {
            if value > peak {
                peak = value;
            }
        }
        if !peak.is_finite() || peak == T::zero() {
            continue;
        }
        let mut sum_sq = T::zero();
        let mut invalid = false;
        for &value in data.iter().take(i + 1).skip(start) {
            if !value.is_finite() {
                invalid = true;
                break;
            }
            let drawdown_pct = hundred * ((value / peak) - T::one());
            sum_sq = sum_sq + drawdown_pct * drawdown_pct;
        }
        if !invalid {
            output[i] = (sum_sq / period_t).sqrt();
        }
    }
    Ok(data.len().saturating_sub(ulcer_index_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ulcer_index_shape() {
        let data: Vec<f64> = (0..160)
            .map(|i| 100.0 + (i as f64 * 0.2) - ((i % 10) as f64 * 0.3))
            .collect();
        let out = ulcer_index(&data, 14).unwrap();
        assert_eq!(out.len(), data.len());
    }
}
