//! Choppiness Index (CHOP).
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::atr::true_range;
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub const fn chop_lookback(period: usize) -> usize {
    period
}

#[inline]
#[must_use]
pub const fn chop_min_len(period: usize) -> usize {
    period + 1
}

fn validate<T: SeriesElement>(high: &[T], low: &[T], close: &[T], period: usize) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if high.is_empty() {
        return Err(Error::EmptyInput);
    }
    let n = high.len();
    if low.len() != n || close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "high has {n} elements, low has {}, close has {}",
                low.len(),
                close.len()
            ),
        });
    }
    let required = chop_min_len(period);
    if n < required {
        return Err(Error::InsufficientData {
            required,
            actual: n,
            indicator: "chop",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with CHOP values, which should be used"]
pub fn chop<T: SeriesElement>(high: &[T], low: &[T], close: &[T], period: usize) -> Result<Vec<T>> {
    validate(high, low, close, period)?;
    let mut out = vec![T::nan(); close.len()];
    chop_into(high, low, close, period, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid CHOP values"]
pub fn chop_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    validate(high, low, close, period)?;
    if output.len() < close.len() {
        return Err(Error::BufferTooSmall {
            required: close.len(),
            actual: output.len(),
            indicator: "chop",
        });
    }
    output[..close.len()].fill(T::nan());
    let tr = true_range(high, low, close)?;
    let hundred = T::hundred();
    let log_period = T::from_usize(period)?.ln();
    for i in period..close.len() {
        let start = i + 1 - period;
        let mut tr_sum = T::zero();
        let mut hh = T::neg_infinity();
        let mut ll = T::infinity();
        let mut invalid = false;
        for j in start..=i {
            if !tr[j].is_finite() || !high[j].is_finite() || !low[j].is_finite() {
                invalid = true;
                break;
            }
            tr_sum = tr_sum + tr[j];
            if high[j] > hh {
                hh = high[j];
            }
            if low[j] < ll {
                ll = low[j];
            }
        }
        let range = hh - ll;
        if !invalid && tr_sum > T::zero() && range > T::zero() {
            output[i] = hundred * ((tr_sum / range).ln() / log_period);
        }
    }
    Ok(close.len().saturating_sub(chop_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chop_shape() {
        let high: Vec<f64> = (0..200).map(|i| 120.0 + i as f64 * 0.2).collect();
        let low: Vec<f64> = high.iter().map(|v| v - 1.0).collect();
        let close: Vec<f64> = high.iter().map(|v| v - 0.4).collect();
        let out = chop(&high, &low, &close, 14).unwrap();
        assert_eq!(out.len(), close.len());
    }
}
