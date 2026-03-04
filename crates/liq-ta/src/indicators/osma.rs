//! OSMA (Moving Average of Oscillator).
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::macd::{macd, macd_min_len, macd_signal_lookback};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub const fn osma_lookback(fast_period: usize, slow_period: usize, signal_period: usize) -> usize {
    let _ = fast_period;
    macd_signal_lookback(slow_period, signal_period)
}

#[inline]
#[must_use]
pub const fn osma_min_len(fast_period: usize, slow_period: usize, signal_period: usize) -> usize {
    let _ = fast_period;
    macd_min_len(slow_period, signal_period)
}

#[must_use = "this returns a Result with OSMA values, which should be used"]
pub fn osma<T: SeriesElement>(
    data: &[T],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<Vec<T>> {
    let out = macd(data, fast_period, slow_period, signal_period)?;
    Ok(out.histogram)
}

#[must_use = "this returns a Result with the count of valid OSMA values"]
pub fn osma_into<T: SeriesElement>(
    data: &[T],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    output: &mut [T],
) -> Result<usize> {
    let hist = osma(data, fast_period, slow_period, signal_period)?;
    if output.len() < hist.len() {
        return Err(Error::BufferTooSmall {
            required: hist.len(),
            actual: output.len(),
            indicator: "osma",
        });
    }
    output[..hist.len()].copy_from_slice(&hist);
    Ok(hist
        .len()
        .saturating_sub(osma_lookback(fast_period, slow_period, signal_period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osma_shape() {
        let data: Vec<f64> = (0..120).map(|i| 100.0 + i as f64 * 0.25).collect();
        let out = osma(&data, 12, 26, 9).unwrap();
        assert_eq!(out.len(), data.len());
    }
}
