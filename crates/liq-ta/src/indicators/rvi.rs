//! Relative Vigor Index (RVI).
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub const fn rvi_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

#[inline]
#[must_use]
pub const fn rvi_min_len(period: usize) -> usize {
    period
}

fn validate<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if open.is_empty() {
        return Err(Error::EmptyInput);
    }
    let n = open.len();
    if high.len() != n || low.len() != n || close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "open has {n} elements, high has {}, low has {}, close has {}",
                high.len(),
                low.len(),
                close.len()
            ),
        });
    }
    if n < rvi_min_len(period) {
        return Err(Error::InsufficientData {
            required: rvi_min_len(period),
            actual: n,
            indicator: "rvi",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with RVI values, which should be used"]
pub fn rvi<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Result<Vec<T>> {
    validate(open, high, low, close, period)?;
    let mut out = vec![T::nan(); close.len()];
    rvi_into(open, high, low, close, period, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid RVI values"]
pub fn rvi_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    validate(open, high, low, close, period)?;
    let n = close.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "rvi",
        });
    }
    output[..n].fill(T::nan());
    let period_t = T::from_usize(period)?;
    for i in (period - 1)..n {
        let start = i + 1 - period;
        let mut num = T::zero();
        let mut den = T::zero();
        let mut invalid = false;
        for j in start..=i {
            let co = close[j] - open[j];
            let hl = high[j] - low[j];
            if !co.is_finite() || !hl.is_finite() {
                invalid = true;
                break;
            }
            num = num + co;
            den = den + hl;
        }
        if !invalid && den != T::zero() {
            output[i] = (num / period_t) / (den / period_t);
        }
    }
    Ok(n.saturating_sub(rvi_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rvi_shape() {
        let open: Vec<f64> = (0..80).map(|i| 100.0 + i as f64 * 0.2).collect();
        let high: Vec<f64> = open.iter().map(|v| v + 0.8).collect();
        let low: Vec<f64> = open.iter().map(|v| v - 0.8).collect();
        let close: Vec<f64> = open.iter().map(|v| v + 0.2).collect();
        let out = rvi(&open, &high, &low, &close, 10).unwrap();
        assert_eq!(out.len(), open.len());
    }
}
