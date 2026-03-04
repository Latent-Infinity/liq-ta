//! Super Trend indicator.
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::atr::{atr, atr_lookback, atr_min_len};
use crate::traits::SeriesElement;

#[derive(Debug, Clone)]
pub struct SuperTrendOutput<T> {
    pub supertrend: Vec<T>,
    pub upper_band: Vec<T>,
    pub lower_band: Vec<T>,
    pub trend: Vec<T>,
}

#[inline]
#[must_use]
pub const fn supertrend_lookback(period: usize) -> usize {
    atr_lookback(period)
}

#[inline]
#[must_use]
pub const fn supertrend_min_len(period: usize) -> usize {
    atr_min_len(period)
}

fn validate<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    multiplier: f64,
) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return Err(Error::LengthMismatch {
            description: "multiplier must be a positive finite number".to_string(),
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
    let required = supertrend_min_len(period);
    if n < required {
        return Err(Error::InsufficientData {
            required,
            actual: n,
            indicator: "supertrend",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with SuperTrend output, which should be used"]
pub fn supertrend<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    multiplier: f64,
) -> Result<SuperTrendOutput<T>> {
    validate(high, low, close, period, multiplier)?;

    let n = close.len();
    let mut out = SuperTrendOutput {
        supertrend: vec![T::nan(); n],
        upper_band: vec![T::nan(); n],
        lower_band: vec![T::nan(); n],
        trend: vec![T::nan(); n],
    };
    supertrend_into(
        high,
        low,
        close,
        period,
        multiplier,
        &mut out.supertrend,
        &mut out.upper_band,
        &mut out.lower_band,
        &mut out.trend,
    )?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid SuperTrend values"]
#[allow(clippy::too_many_arguments)]
pub fn supertrend_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    multiplier: f64,
    supertrend_out: &mut [T],
    upper_out: &mut [T],
    lower_out: &mut [T],
    trend_out: &mut [T],
) -> Result<usize> {
    validate(high, low, close, period, multiplier)?;
    let n = close.len();
    if supertrend_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: supertrend_out.len(),
            indicator: "supertrend (line)",
        });
    }
    if upper_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: upper_out.len(),
            indicator: "supertrend (upper_band)",
        });
    }
    if lower_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: lower_out.len(),
            indicator: "supertrend (lower_band)",
        });
    }
    if trend_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: trend_out.len(),
            indicator: "supertrend (trend)",
        });
    }

    let atr_values = atr(high, low, close, period)?;
    let mult = T::from_f64(multiplier)?;
    let two = T::two();
    let lookback = supertrend_lookback(period);

    supertrend_out[..n].fill(T::nan());
    upper_out[..n].fill(T::nan());
    lower_out[..n].fill(T::nan());
    trend_out[..n].fill(T::nan());

    for i in lookback..n {
        if !atr_values[i].is_finite() || !high[i].is_finite() || !low[i].is_finite() {
            continue;
        }
        let hl2 = (high[i] + low[i]) / two;
        let basic_upper = hl2 + mult * atr_values[i];
        let basic_lower = hl2 - mult * atr_values[i];

        if i == lookback {
            upper_out[i] = basic_upper;
            lower_out[i] = basic_lower;
            supertrend_out[i] = basic_lower;
            trend_out[i] = T::one();
            continue;
        }

        let prev_upper = upper_out[i - 1];
        let prev_lower = lower_out[i - 1];
        let prev_st = supertrend_out[i - 1];
        if !prev_upper.is_finite() || !prev_lower.is_finite() || !prev_st.is_finite() {
            upper_out[i] = basic_upper;
            lower_out[i] = basic_lower;
            supertrend_out[i] = basic_lower;
            trend_out[i] = T::one();
            continue;
        }

        let final_upper = if basic_upper < prev_upper || close[i - 1] > prev_upper {
            basic_upper
        } else {
            prev_upper
        };
        let final_lower = if basic_lower > prev_lower || close[i - 1] < prev_lower {
            basic_lower
        } else {
            prev_lower
        };
        upper_out[i] = final_upper;
        lower_out[i] = final_lower;

        let current_st = if prev_st == prev_upper {
            if close[i] <= final_upper {
                final_upper
            } else {
                final_lower
            }
        } else if close[i] >= final_lower {
            final_lower
        } else {
            final_upper
        };
        supertrend_out[i] = current_st;
        trend_out[i] = if current_st == final_lower {
            T::one()
        } else {
            T::zero() - T::one()
        };
    }
    Ok(n.saturating_sub(lookback))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);
        for i in 0..n {
            let c = 100.0 + (i as f64 * 0.4);
            high.push(c + 1.0);
            low.push(c - 1.0);
            close.push(c);
        }
        (high, low, close)
    }

    #[test]
    fn test_supertrend_shape() {
        let (h, l, c) = sample_ohlc(120);
        let out = supertrend(&h, &l, &c, 10, 3.0).unwrap();
        assert_eq!(out.supertrend.len(), c.len());
        assert_eq!(out.upper_band.len(), c.len());
        assert_eq!(out.lower_band.len(), c.len());
        assert_eq!(out.trend.len(), c.len());
    }
}
