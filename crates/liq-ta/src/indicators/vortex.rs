//! Vortex Indicator.
#![allow(missing_docs)]

use crate::error::{Error, Result};
use crate::indicators::atr::true_range;
use crate::traits::SeriesElement;

#[derive(Debug, Clone)]
pub struct VortexOutput<T> {
    pub plus_vi: Vec<T>,
    pub minus_vi: Vec<T>,
}

#[inline]
#[must_use]
pub const fn vortex_lookback(period: usize) -> usize {
    period
}

#[inline]
#[must_use]
pub const fn vortex_min_len(period: usize) -> usize {
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
    let required = vortex_min_len(period);
    if n < required {
        return Err(Error::InsufficientData {
            required,
            actual: n,
            indicator: "vortex",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with Vortex output, which should be used"]
pub fn vortex<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
) -> Result<VortexOutput<T>> {
    validate(high, low, close, period)?;
    let n = close.len();
    let mut plus_vi = vec![T::nan(); n];
    let mut minus_vi = vec![T::nan(); n];
    vortex_into(high, low, close, period, &mut plus_vi, &mut minus_vi)?;
    Ok(VortexOutput { plus_vi, minus_vi })
}

#[must_use = "this returns a Result with the count of valid Vortex values"]
pub fn vortex_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period: usize,
    plus_out: &mut [T],
    minus_out: &mut [T],
) -> Result<usize> {
    validate(high, low, close, period)?;
    let n = close.len();
    if plus_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: plus_out.len(),
            indicator: "vortex (plus_vi)",
        });
    }
    if minus_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: minus_out.len(),
            indicator: "vortex (minus_vi)",
        });
    }
    plus_out[..n].fill(T::nan());
    minus_out[..n].fill(T::nan());

    let tr = true_range(high, low, close)?;
    let mut vm_plus = vec![T::nan(); n];
    let mut vm_minus = vec![T::nan(); n];
    for i in 1..n {
        if high[i].is_finite() && low[i - 1].is_finite() {
            vm_plus[i] = (high[i] - low[i - 1]).abs();
        }
        if low[i].is_finite() && high[i - 1].is_finite() {
            vm_minus[i] = (low[i] - high[i - 1]).abs();
        }
    }

    for i in period..n {
        let start = i + 1 - period;
        let mut sum_tr = T::zero();
        let mut sum_plus = T::zero();
        let mut sum_minus = T::zero();
        let mut invalid = false;
        for j in start..=i {
            if !tr[j].is_finite() || !vm_plus[j].is_finite() || !vm_minus[j].is_finite() {
                invalid = true;
                break;
            }
            sum_tr = sum_tr + tr[j];
            sum_plus = sum_plus + vm_plus[j];
            sum_minus = sum_minus + vm_minus[j];
        }
        if !invalid && sum_tr > T::zero() {
            plus_out[i] = sum_plus / sum_tr;
            minus_out[i] = sum_minus / sum_tr;
        }
    }
    Ok(n.saturating_sub(vortex_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vortex_shape() {
        let high: Vec<f64> = (0..120).map(|i| 100.0 + i as f64 * 0.6).collect();
        let low: Vec<f64> = high.iter().map(|v| v - 1.2).collect();
        let close: Vec<f64> = high.iter().map(|v| v - 0.5).collect();
        let out = vortex(&high, &low, &close, 14).unwrap();
        assert_eq!(out.plus_vi.len(), close.len());
        assert_eq!(out.minus_vi.len(), close.len());
    }

    #[test]
    fn test_vortex_validation_error_surface() {
        let empty: Vec<f64> = vec![];
        assert!(vortex(&empty, &empty, &empty, 14).is_err());

        let high = vec![10.0_f64; 20];
        let low_short = vec![9.0_f64; 19];
        let close = vec![9.5_f64; 20];
        assert!(vortex(&high, &low_short, &close, 14).is_err());
        assert!(vortex(&high, &[9.0; 20], &close, 0).is_err());
        assert!(vortex(&high[..10], &[9.0; 10], &[9.5; 10], 14).is_err());
    }

    #[test]
    fn test_vortex_into_buffer_and_nonfinite_surface() {
        let n = 64usize;
        let high: Vec<f64> = (0..n).map(|i| 40.0 + i as f64 * 0.4).collect();
        let low: Vec<f64> = high.iter().map(|v| v - 1.3).collect();
        let mut close: Vec<f64> = high.iter().map(|v| v - 0.5).collect();
        close[10] = f64::NAN;

        let mut plus_small = vec![f64::NAN; n - 1];
        let mut minus = vec![f64::NAN; n];
        assert!(vortex_into(&high, &low, &close, 14, &mut plus_small, &mut minus).is_err());

        let mut plus = vec![f64::NAN; n];
        let mut minus_small = vec![f64::NAN; n - 1];
        assert!(vortex_into(&high, &low, &close, 14, &mut plus, &mut minus_small).is_err());

        let mut plus_ok = vec![f64::NAN; n];
        let mut minus_ok = vec![f64::NAN; n];
        let _ = vortex_into(&high, &low, &close, 14, &mut plus_ok, &mut minus_ok).unwrap();
    }
}
