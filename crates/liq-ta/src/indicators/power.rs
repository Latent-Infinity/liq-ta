//! Bulls Power and Bears Power indicators.
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::ema::{ema, ema_into, ema_lookback, ema_min_len};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub const fn power_lookback(period: usize) -> usize {
    ema_lookback(period)
}

#[inline]
#[must_use]
pub const fn power_min_len(period: usize) -> usize {
    ema_min_len(period)
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
    let required = power_min_len(period);
    if n < required {
        return Err(Error::InsufficientData {
            required,
            actual: n,
            indicator: "power",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with Bulls Power values, which should be used"]
pub fn bulls_power<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Result<Vec<T>> {
    validate(high, low, close, period)?;
    let ema_close = ema(close, period)?;
    let mut out = vec![T::nan(); close.len()];
    for i in 0..close.len() {
        if high[i].is_finite() && ema_close[i].is_finite() {
            out[i] = high[i] - ema_close[i];
        }
    }
    Ok(out)
}

#[must_use = "this returns a Result with Bears Power values, which should be used"]
pub fn bears_power<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Result<Vec<T>> {
    validate(high, low, close, period)?;
    let ema_close = ema(close, period)?;
    let mut out = vec![T::nan(); close.len()];
    for i in 0..close.len() {
        if low[i].is_finite() && ema_close[i].is_finite() {
            out[i] = low[i] - ema_close[i];
        }
    }
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid Bulls Power values"]
pub fn bulls_power_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    validate(high, low, close, period)?;
    let n = close.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "bulls_power",
        });
    }
    let mut ema_close = vec![T::nan(); n];
    ema_into(close, period, &mut ema_close)?;
    output[..n].fill(T::nan());
    for i in 0..n {
        if high[i].is_finite() && ema_close[i].is_finite() {
            output[i] = high[i] - ema_close[i];
        }
    }
    Ok(n.saturating_sub(power_lookback(period)))
}

#[must_use = "this returns a Result with the count of valid Bears Power values"]
pub fn bears_power_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    validate(high, low, close, period)?;
    let n = close.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "bears_power",
        });
    }
    let mut ema_close = vec![T::nan(); n];
    ema_into(close, period, &mut ema_close)?;
    output[..n].fill(T::nan());
    for i in 0..n {
        if low[i].is_finite() && ema_close[i].is_finite() {
            output[i] = low[i] - ema_close[i];
        }
    }
    Ok(n.saturating_sub(power_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_shape() {
        let high: Vec<f64> = (0..80).map(|i| 100.0 + i as f64 * 0.4).collect();
        let low: Vec<f64> = high.iter().map(|v| v - 1.5).collect();
        let close: Vec<f64> = high.iter().map(|v| v - 0.7).collect();
        let bulls = bulls_power(&high, &low, &close, 13).unwrap();
        let bears = bears_power(&high, &low, &close, 13).unwrap();
        assert_eq!(bulls.len(), close.len());
        assert_eq!(bears.len(), close.len());
    }
}
