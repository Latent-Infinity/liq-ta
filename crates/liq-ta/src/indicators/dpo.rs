//! Detrended Price Oscillator (DPO).
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::sma::{sma_into, sma_lookback, sma_min_len};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub const fn dpo_shift(period: usize) -> usize {
    period / 2 + 1
}

#[inline]
#[must_use]
pub const fn dpo_lookback(period: usize) -> usize {
    sma_lookback(period) + dpo_shift(period)
}

#[inline]
#[must_use]
pub const fn dpo_min_len(period: usize) -> usize {
    sma_min_len(period) + dpo_shift(period)
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
    let required = dpo_min_len(period);
    if data.len() < required {
        return Err(Error::InsufficientData {
            required,
            actual: data.len(),
            indicator: "dpo",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with DPO values, which should be used"]
pub fn dpo<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    validate(data, period)?;
    let mut out = vec![T::nan(); data.len()];
    dpo_into(data, period, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid DPO values"]
pub fn dpo_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize> {
    validate(data, period)?;
    let n = data.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "dpo",
        });
    }
    let shift = dpo_shift(period);
    let mut sma_vals = vec![T::nan(); n];
    sma_into(data, period, &mut sma_vals)?;
    output[..n].fill(T::nan());

    for i in shift..n {
        let src = i - shift;
        if data[src].is_finite() && sma_vals[src].is_finite() {
            output[i] = data[src] - sma_vals[src];
        }
    }
    Ok(n.saturating_sub(dpo_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpo_shape() {
        let data: Vec<f64> = (0..120).map(|i| 100.0 + (i as f64 * 0.15)).collect();
        let out = dpo(&data, 20).unwrap();
        assert_eq!(out.len(), data.len());
    }
}
