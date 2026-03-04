//! Connors RSI (CRSI).
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::rsi::{rsi, rsi_lookback, rsi_min_len};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub fn connors_rsi_lookback(rsi_period: usize, streak_period: usize, rank_period: usize) -> usize {
    let a = rsi_lookback(rsi_period);
    let b = rsi_lookback(streak_period);
    a.max(b).max(rank_period)
}

#[inline]
#[must_use]
pub fn connors_rsi_min_len(rsi_period: usize, streak_period: usize, rank_period: usize) -> usize {
    connors_rsi_lookback(rsi_period, streak_period, rank_period) + 1
}

fn validate<T: SeriesElement>(
    data: &[T],
    rsi_period: usize,
    streak_period: usize,
    rank_period: usize,
) -> Result<()> {
    if rsi_period == 0 {
        return Err(Error::InvalidPeriod {
            period: rsi_period,
            reason: "rsi_period must be at least 1",
        });
    }
    if streak_period == 0 {
        return Err(Error::InvalidPeriod {
            period: streak_period,
            reason: "streak_period must be at least 1",
        });
    }
    if rank_period == 0 {
        return Err(Error::InvalidPeriod {
            period: rank_period,
            reason: "rank_period must be at least 1",
        });
    }
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    let required = connors_rsi_min_len(rsi_period, streak_period, rank_period);
    if data.len() < required {
        return Err(Error::InsufficientData {
            required,
            actual: data.len(),
            indicator: "connors_rsi",
        });
    }
    let _ = rsi_min_len(rsi_period);
    Ok(())
}

#[must_use = "this returns a Result with Connors RSI values, which should be used"]
pub fn connors_rsi<T: SeriesElement>(
    data: &[T],
    rsi_period: usize,
    streak_period: usize,
    rank_period: usize,
) -> Result<Vec<T>> {
    validate(data, rsi_period, streak_period, rank_period)?;
    let mut out = vec![T::nan(); data.len()];
    connors_rsi_into(data, rsi_period, streak_period, rank_period, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid Connors RSI values"]
pub fn connors_rsi_into<T: SeriesElement>(
    data: &[T],
    rsi_period: usize,
    streak_period: usize,
    rank_period: usize,
    output: &mut [T],
) -> Result<usize> {
    validate(data, rsi_period, streak_period, rank_period)?;
    let n = data.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "connors_rsi",
        });
    }
    output[..n].fill(T::nan());

    let rsi_price = rsi(data, rsi_period)?;

    let mut streak = vec![T::zero(); n];
    for i in 1..n {
        if data[i] > data[i - 1] {
            streak[i] = if streak[i - 1] > T::zero() {
                streak[i - 1] + T::one()
            } else {
                T::one()
            };
        } else if data[i] < data[i - 1] {
            streak[i] = if streak[i - 1] < T::zero() {
                streak[i - 1] - T::one()
            } else {
                T::zero() - T::one()
            };
        } else {
            streak[i] = T::zero();
        }
    }
    let rsi_streak = rsi(&streak, streak_period)?;

    let hundred = T::hundred();
    let three = T::from_i32(3)?;
    let rank_period_t = T::from_usize(rank_period)?;
    let mut rank = vec![T::nan(); n];
    for i in rank_period..n {
        let mut count = 0usize;
        let current = data[i] - data[i - 1];
        if !current.is_finite() {
            continue;
        }
        for j in (i + 1 - rank_period)..=i {
            let roc = data[j] - data[j - 1];
            if roc.is_finite() && roc <= current {
                count += 1;
            }
        }
        rank[i] = hundred * T::from_usize(count)? / rank_period_t;
    }

    for i in 0..n {
        if rsi_price[i].is_finite() && rsi_streak[i].is_finite() && rank[i].is_finite() {
            output[i] = (rsi_price[i] + rsi_streak[i] + rank[i]) / three;
        }
    }

    Ok(n.saturating_sub(connors_rsi_lookback(rsi_period, streak_period, rank_period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connors_rsi_shape() {
        let data: Vec<f64> = (0..200).map(|i| 100.0 + i as f64 * 0.1).collect();
        let out = connors_rsi(&data, 3, 2, 50).unwrap();
        assert_eq!(out.len(), data.len());
    }
}
