//! Laguerre RSI.
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

#[inline]
#[must_use]
pub const fn laguerre_rsi_lookback() -> usize {
    1
}

#[inline]
#[must_use]
pub const fn laguerre_rsi_min_len() -> usize {
    2
}

fn validate<T: SeriesElement>(data: &[T], gamma: f64) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    if data.len() < laguerre_rsi_min_len() {
        return Err(Error::InsufficientData {
            required: laguerre_rsi_min_len(),
            actual: data.len(),
            indicator: "laguerre_rsi",
        });
    }
    if !gamma.is_finite() || !(0.0..1.0).contains(&gamma) {
        return Err(Error::LengthMismatch {
            description: "gamma must be finite and in [0, 1)".to_string(),
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with Laguerre RSI values, which should be used"]
pub fn laguerre_rsi<T: SeriesElement>(data: &[T], gamma: f64) -> Result<Vec<T>> {
    validate(data, gamma)?;
    let mut out = vec![T::nan(); data.len()];
    laguerre_rsi_into(data, gamma, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid Laguerre RSI values"]
pub fn laguerre_rsi_into<T: SeriesElement>(
    data: &[T],
    gamma: f64,
    output: &mut [T],
) -> Result<usize> {
    validate(data, gamma)?;
    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "laguerre_rsi",
        });
    }
    output[..data.len()].fill(T::nan());
    let g = T::from_f64(gamma)?;
    let one = T::one();
    let mut l0 = data[0];
    let mut l1 = data[0];
    let mut l2 = data[0];
    let mut l3 = data[0];
    for i in 1..data.len() {
        let prev_l0 = l0;
        let prev_l1 = l1;
        let prev_l2 = l2;
        let prev_l3 = l3;

        l0 = (one - g) * data[i] + g * prev_l0;
        l1 = -g * l0 + prev_l0 + g * prev_l1;
        l2 = -g * l1 + prev_l1 + g * prev_l2;
        l3 = -g * l2 + prev_l2 + g * prev_l3;

        let cu = (l0 - l1).max(T::zero()) + (l1 - l2).max(T::zero()) + (l2 - l3).max(T::zero());
        let cd = (l1 - l0).max(T::zero()) + (l2 - l1).max(T::zero()) + (l3 - l2).max(T::zero());
        let denom = cu + cd;
        output[i] = if denom > T::zero() {
            T::hundred() * cu / denom
        } else {
            T::fifty()
        };
    }
    Ok(data.len().saturating_sub(laguerre_rsi_lookback()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_laguerre_rsi_bounds() {
        let data: Vec<f64> = (0..120).map(|i| 100.0 + i as f64 * 0.1).collect();
        let out = laguerre_rsi(&data, 0.5).unwrap();
        for value in out.iter().copied().filter(|v| v.is_finite()) {
            assert!((0.0..=100.0).contains(&value));
        }
    }
}
