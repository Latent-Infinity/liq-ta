//! Double Smoothed Stochastic (DSS) Bressert.
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::ema::{ema, ema_lookback};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub fn dss_bressert_lookback(stochastic_period: usize, ema_period: usize) -> usize {
    (stochastic_period.saturating_sub(1))
        + ema_lookback(ema_period)
        + ema_lookback(ema_period)
        + (stochastic_period.saturating_sub(1))
        + ema_lookback(ema_period)
}

#[inline]
#[must_use]
pub fn dss_bressert_min_len(stochastic_period: usize, ema_period: usize) -> usize {
    dss_bressert_lookback(stochastic_period, ema_period) + 1
}

fn stochastic_from_hlc<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Vec<T> {
    let n = close.len();
    let mut out = vec![T::nan(); n];
    if period == 0 || n < period {
        return out;
    }
    for i in (period - 1)..n {
        let start = i + 1 - period;
        let mut hh = T::neg_infinity();
        let mut ll = T::infinity();
        for j in start..=i {
            if !high[j].is_finite() || !low[j].is_finite() {
                hh = T::nan();
                break;
            }
            if high[j] > hh {
                hh = high[j];
            }
            if low[j] < ll {
                ll = low[j];
            }
        }
        if !hh.is_finite() || !ll.is_finite() || !close[i].is_finite() {
            continue;
        }
        let range = hh - ll;
        if range > T::zero() {
            out[i] = T::hundred() * (close[i] - ll) / range;
        }
    }
    out
}

fn stochastic_from_series<T: SeriesElement>(data: &[T], period: usize) -> Vec<T> {
    let n = data.len();
    let mut out = vec![T::nan(); n];
    if period == 0 || n < period {
        return out;
    }
    for i in (period - 1)..n {
        let start = i + 1 - period;
        let mut hh = T::neg_infinity();
        let mut ll = T::infinity();
        for &value in data.iter().take(i + 1).skip(start) {
            if !value.is_finite() {
                hh = T::nan();
                break;
            }
            if value > hh {
                hh = value;
            }
            if value < ll {
                ll = value;
            }
        }
        if !hh.is_finite() || !ll.is_finite() || !data[i].is_finite() {
            continue;
        }
        let range = hh - ll;
        if range > T::zero() {
            out[i] = T::hundred() * (data[i] - ll) / range;
        }
    }
    out
}

fn validate<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    stochastic_period: usize,
    ema_period: usize,
) -> Result<()> {
    if stochastic_period == 0 {
        return Err(Error::InvalidPeriod {
            period: stochastic_period,
            reason: "stochastic_period must be at least 1",
        });
    }
    if ema_period == 0 {
        return Err(Error::InvalidPeriod {
            period: ema_period,
            reason: "ema_period must be at least 1",
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
    let required = dss_bressert_min_len(stochastic_period, ema_period);
    if n < required {
        return Err(Error::InsufficientData {
            required,
            actual: n,
            indicator: "dss_bressert",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with DSS Bressert values, which should be used"]
pub fn dss_bressert<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    stochastic_period: usize,
    ema_period: usize,
) -> Result<Vec<T>> {
    validate(high, low, close, stochastic_period, ema_period)?;
    let mut out = vec![T::nan(); close.len()];
    dss_bressert_into(high, low, close, stochastic_period, ema_period, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid DSS Bressert values"]
pub fn dss_bressert_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    stochastic_period: usize,
    ema_period: usize,
    output: &mut [T],
) -> Result<usize> {
    validate(high, low, close, stochastic_period, ema_period)?;
    if output.len() < close.len() {
        return Err(Error::BufferTooSmall {
            required: close.len(),
            actual: output.len(),
            indicator: "dss_bressert",
        });
    }
    let s1 = stochastic_from_hlc(high, low, close, stochastic_period);
    let e1 = ema(&s1, ema_period)?;
    let e2 = ema(&e1, ema_period)?;
    let s2 = stochastic_from_series(&e2, stochastic_period);
    let dss = ema(&s2, ema_period)?;
    output[..dss.len()].copy_from_slice(&dss);
    Ok(dss
        .len()
        .saturating_sub(dss_bressert_lookback(stochastic_period, ema_period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dss_bressert_shape() {
        let high: Vec<f64> = (0..260).map(|i| 100.0 + i as f64 * 0.3).collect();
        let low: Vec<f64> = high.iter().map(|v| v - 1.2).collect();
        let close: Vec<f64> = high.iter().map(|v| v - 0.4).collect();
        let out = dss_bressert(&high, &low, &close, 14, 5).unwrap();
        assert_eq!(out.len(), close.len());
    }
}
