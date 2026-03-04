//! Awesome Oscillator (AO).
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::sma::{sma, sma_into, sma_lookback, sma_min_len};
use crate::traits::SeriesElement;

pub const AO_FAST_PERIOD: usize = 5;
pub const AO_SLOW_PERIOD: usize = 34;

#[inline]
#[must_use]
pub const fn ao_lookback() -> usize {
    let fast = sma_lookback(AO_FAST_PERIOD);
    let slow = sma_lookback(AO_SLOW_PERIOD);
    if fast > slow { fast } else { slow }
}

#[inline]
#[must_use]
pub const fn ao_min_len() -> usize {
    let fast = sma_min_len(AO_FAST_PERIOD);
    let slow = sma_min_len(AO_SLOW_PERIOD);
    if fast > slow { fast } else { slow }
}

#[must_use = "this returns a Result with AO values, which should be used"]
pub fn ao<T: SeriesElement>(high: &[T], low: &[T]) -> Result<Vec<T>> {
    if high.is_empty() {
        return Err(Error::EmptyInput);
    }
    if low.len() != high.len() {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", high.len(), low.len()),
        });
    }
    if high.len() < ao_min_len() {
        return Err(Error::InsufficientData {
            required: ao_min_len(),
            actual: high.len(),
            indicator: "ao",
        });
    }
    let n = high.len();
    let two = T::two();
    let mut median = vec![T::nan(); n];
    for i in 0..n {
        if high[i].is_finite() && low[i].is_finite() {
            median[i] = (high[i] + low[i]) / two;
        }
    }
    let fast = sma(&median, AO_FAST_PERIOD)?;
    let slow = sma(&median, AO_SLOW_PERIOD)?;
    let mut out = vec![T::nan(); n];
    for i in 0..n {
        if fast[i].is_finite() && slow[i].is_finite() {
            out[i] = fast[i] - slow[i];
        }
    }
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid AO values"]
pub fn ao_into<T: SeriesElement>(high: &[T], low: &[T], output: &mut [T]) -> Result<usize> {
    if high.is_empty() {
        return Err(Error::EmptyInput);
    }
    if low.len() != high.len() {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", high.len(), low.len()),
        });
    }
    let n = high.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "ao",
        });
    }
    if n < ao_min_len() {
        return Err(Error::InsufficientData {
            required: ao_min_len(),
            actual: n,
            indicator: "ao",
        });
    }
    let two = T::two();
    let mut median = vec![T::nan(); n];
    let mut fast = vec![T::nan(); n];
    let mut slow = vec![T::nan(); n];
    for i in 0..n {
        if high[i].is_finite() && low[i].is_finite() {
            median[i] = (high[i] + low[i]) / two;
        }
    }
    sma_into(&median, AO_FAST_PERIOD, &mut fast)?;
    sma_into(&median, AO_SLOW_PERIOD, &mut slow)?;
    output[..n].fill(T::nan());
    for i in 0..n {
        if fast[i].is_finite() && slow[i].is_finite() {
            output[i] = fast[i] - slow[i];
        }
    }
    Ok(n.saturating_sub(ao_lookback()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ao_shape() {
        let high: Vec<f64> = (0..80).map(|i| 100.0 + i as f64).collect();
        let low: Vec<f64> = high.iter().map(|x| x - 2.0).collect();
        let out = ao(&high, &low).unwrap();
        assert_eq!(out.len(), high.len());
        assert!(out[..ao_lookback()].iter().all(|v| v.is_nan()));
    }
}
