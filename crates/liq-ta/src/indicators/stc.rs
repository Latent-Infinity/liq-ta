//! Schaff Trend Cycle (STC).
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::ema::{ema, ema_lookback};
use crate::indicators::macd::{macd, macd_line_lookback, macd_min_len};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub fn stc_lookback(
    fast_period: usize,
    slow_period: usize,
    cycle_period: usize,
    smooth_period: usize,
) -> usize {
    let _ = fast_period;
    macd_line_lookback(slow_period)
        + (cycle_period.saturating_sub(1))
        + ema_lookback(smooth_period)
        + (cycle_period.saturating_sub(1))
        + ema_lookback(smooth_period)
}

#[inline]
#[must_use]
pub fn stc_min_len(
    fast_period: usize,
    slow_period: usize,
    cycle_period: usize,
    smooth_period: usize,
) -> usize {
    stc_lookback(fast_period, slow_period, cycle_period, smooth_period) + 1
}

fn validate(
    len: usize,
    fast_period: usize,
    slow_period: usize,
    cycle_period: usize,
    smooth_period: usize,
) -> Result<()> {
    if fast_period == 0 || slow_period == 0 || cycle_period == 0 || smooth_period == 0 {
        return Err(Error::InvalidPeriod {
            period: 0,
            reason: "all periods must be at least 1",
        });
    }
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if fast_period >= slow_period {
        return Err(Error::InvalidPeriod {
            period: fast_period,
            reason: "fast_period must be less than slow_period",
        });
    }
    let required = stc_min_len(fast_period, slow_period, cycle_period, smooth_period);
    if len < required {
        return Err(Error::InsufficientData {
            required,
            actual: len,
            indicator: "stc",
        });
    }
    let _ = macd_min_len(slow_period, 9);
    Ok(())
}

fn stochastic_transform<T: SeriesElement>(data: &[T], period: usize) -> Vec<T> {
    let n = data.len();
    let mut out = vec![T::nan(); n];
    if period == 0 || n < period {
        return out;
    }
    let hundred = T::hundred();
    for i in (period - 1)..n {
        let start = i + 1 - period;
        let mut min_v = T::infinity();
        let mut max_v = T::neg_infinity();
        for value in data.iter().take(i + 1).skip(start).copied() {
            if !value.is_finite() {
                min_v = T::nan();
                break;
            }
            if value < min_v {
                min_v = value;
            }
            if value > max_v {
                max_v = value;
            }
        }
        if !min_v.is_finite() || !max_v.is_finite() {
            continue;
        }
        let range = max_v - min_v;
        if range > T::zero() {
            out[i] = hundred * (data[i] - min_v) / range;
        }
    }
    out
}

#[must_use = "this returns a Result with STC values, which should be used"]
pub fn stc<T: SeriesElement>(
    data: &[T],
    fast_period: usize,
    slow_period: usize,
    cycle_period: usize,
    smooth_period: usize,
) -> Result<Vec<T>> {
    validate(
        data.len(),
        fast_period,
        slow_period,
        cycle_period,
        smooth_period,
    )?;
    let macd_out = macd(data, fast_period, slow_period, 9)?;
    let k1 = stochastic_transform(&macd_out.macd_line, cycle_period);
    let d1 = ema(&k1, smooth_period)?;
    let k2 = stochastic_transform(&d1, cycle_period);
    ema(&k2, smooth_period)
}

#[must_use = "this returns a Result with the count of valid STC values"]
pub fn stc_into<T: SeriesElement>(
    data: &[T],
    fast_period: usize,
    slow_period: usize,
    cycle_period: usize,
    smooth_period: usize,
    output: &mut [T],
) -> Result<usize> {
    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "stc",
        });
    }
    let result = stc(data, fast_period, slow_period, cycle_period, smooth_period)?;
    output[..result.len()].copy_from_slice(&result);
    Ok(result.len().saturating_sub(stc_lookback(
        fast_period,
        slow_period,
        cycle_period,
        smooth_period,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stc_shape() {
        let data: Vec<f64> = (0..300).map(|i| 100.0 + i as f64 * 0.15).collect();
        let out = stc(&data, 23, 50, 10, 3).unwrap();
        assert_eq!(out.len(), data.len());
    }
}
