//! Choppiness Index (CHOP).
//!
//! The period must be at least 2; period 1 makes the logarithmic
//! normalization undefined.
#![expect(
    missing_docs,
    reason = "indicator output structs and helpers are documented at module level"
)]

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

// Identity of the [true-range sum, highest high, lowest low] window aggregate.
const EMPTY: [f64; 3] = [0.0, f64::NEG_INFINITY, f64::INFINITY];

#[inline]
#[must_use]
pub const fn chop_lookback(period: usize) -> usize {
    period
}

#[inline]
#[must_use]
pub const fn chop_min_len(period: usize) -> usize {
    period.saturating_add(1)
}

fn validate<T: SeriesElement>(high: &[T], low: &[T], close: &[T], period: usize) -> Result<()> {
    if period < 2 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 2",
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

// Negative values encode the negated logarithm of a positive value above f64::MAX.
fn positive_log(value: f64) -> f64 {
    if value < 0.0 { -value } else { value.ln() }
}

fn positive_add(a: f64, b: f64) -> f64 {
    if a >= 0.0 && b >= 0.0 {
        let sum = a + b;
        if sum.is_finite() {
            return sum;
        }
    }
    let a = positive_log(a);
    let b = positive_log(b);
    let max = a.max(b);
    -(max + (a.min(b) - max).exp().ln_1p())
}

fn positive_difference(high: f64, low: f64) -> f64 {
    if high <= low {
        return 0.0;
    }
    let difference = high - low;
    if difference.is_finite() {
        difference
    } else {
        -((high * 0.5 - low * 0.5).ln() + std::f64::consts::LN_2)
    }
}

/// Computes the Choppiness Index.
///
/// # Errors
/// Returns an error for a period below 2, empty or mismatched inputs,
/// insufficient data, or a numeric conversion failure.
#[must_use = "this returns a Result with CHOP values, which should be used"]
pub fn chop<T: SeriesElement>(high: &[T], low: &[T], close: &[T], period: usize) -> Result<Vec<T>> {
    validate(high, low, close, period)?;
    let mut out = vec![T::nan(); close.len()];
    chop_into(high, low, close, period, &mut out)?;
    Ok(out)
}

/// Computes the Choppiness Index into a caller-provided buffer.
///
/// # Errors
/// Returns an error for a period below 2, empty or mismatched inputs,
/// insufficient data, a short output buffer, or a numeric conversion failure.
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
    // Each slot holds a bar's [true range, high, low] until its block of `period`
    // bars is full, then the block's suffix aggregate. `incoming` aggregates the
    // current block, so nothing is ever subtracted and recovery stays exact.
    let mut ring = vec![EMPTY; period];
    let mut slot = 0usize;
    let mut incoming = EMPTY;
    // Consecutive valid bars ending at the current one.
    let mut valid = 0usize;
    let log_period = (period as f64).ln();
    let to_f64 = |value: T| {
        value.to_f64().ok_or(Error::NumericConversion {
            context: "series element to f64",
        })
    };
    for i in 1..close.len() {
        let bar = if high[i].is_finite() && low[i].is_finite() && close[i - 1].is_finite() {
            let h = to_f64(high[i])?;
            let l = to_f64(low[i])?;
            let previous_close = to_f64(close[i - 1])?;
            if h.is_finite() && l.is_finite() && previous_close.is_finite() {
                let tr = (h - l)
                    .max((h - previous_close).abs())
                    .max((l - previous_close).abs());
                if tr.is_finite() {
                    Some([tr, h, l])
                } else {
                    let half = (h * 0.5 - l * 0.5)
                        .max((h * 0.5 - previous_close * 0.5).abs())
                        .max((l * 0.5 - previous_close * 0.5).abs());
                    Some([-(half.ln() + std::f64::consts::LN_2), h, l])
                }
            } else {
                None
            }
        } else {
            None
        };
        valid = if bar.is_some() { valid + 1 } else { 0 };
        let bar = bar.unwrap_or(EMPTY);
        if slot == period {
            // The block is full and starts to expire: fold it into suffix aggregates.
            let mut suffix = EMPTY;
            for entry in ring.iter_mut().rev() {
                suffix = [
                    positive_add(suffix[0], entry[0]),
                    suffix[1].max(entry[1]),
                    suffix[2].min(entry[2]),
                ];
                *entry = suffix;
            }
            incoming = EMPTY;
            slot = 0;
        }
        incoming = [
            positive_add(incoming[0], bar[0]),
            incoming[1].max(bar[1]),
            incoming[2].min(bar[2]),
        ];
        ring[slot] = bar;
        slot += 1;

        if valid >= period {
            // Bars of the previous block that are still inside the window.
            let outgoing = ring.get(slot).copied().unwrap_or(EMPTY);
            let hh = incoming[1].max(outgoing[1]);
            let ll = incoming[2].min(outgoing[2]);
            let range = positive_difference(hh, ll);
            let sum = positive_add(incoming[0], outgoing[0]);
            if sum != 0.0 && range != 0.0 {
                let ratio = if sum > 0.0 && range > 0.0 {
                    sum / range
                } else {
                    0.0
                };
                let log_ratio = if ratio >= f64::MIN_POSITIVE && ratio.is_finite() {
                    ratio.ln()
                } else {
                    positive_log(sum) - positive_log(range)
                };
                output[i] = T::from_f64(100.0 * (log_ratio / log_period))?;
            }
        }
    }
    Ok(close.len().saturating_sub(chop_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_one_is_invalid_for_both_apis() {
        let high = [2.0_f64; 2];
        let low = [1.0; 2];
        let close = [1.5; 2];
        let mut output = [7.0; 2];
        assert!(matches!(
            chop(&high, &low, &close, 1),
            Err(Error::InvalidPeriod { period: 1, .. })
        ));
        assert!(matches!(
            chop_into(&high, &low, &close, 1, &mut output),
            Err(Error::InvalidPeriod { period: 1, .. })
        ));
        assert_eq!(output, [7.0; 2]);
    }

    #[test]
    fn f32_extremes_and_subnormal_ranges() {
        let max = f32::MAX;
        let high = [max; 4];
        let low = [-max; 4];
        let close = [0.0_f32; 4];
        let out = chop(&high, &low, &close, 2).unwrap();
        assert!(out[..2].iter().all(|v| v.is_nan()));
        assert!(out[2..].iter().all(|v| (*v - 100.0).abs() < 1e-4));

        let tiny = f32::from_bits(1);
        let out = chop(&[tiny; 4], &[0.0; 4], &[0.0; 4], 2).unwrap();
        assert!(out[2..].iter().all(|v| (*v - 100.0).abs() < 1e-4));
    }

    #[test]
    fn f64_extreme_range_and_subnormal_range() {
        let high = [f64::MAX; 4];
        let low = [-f64::MAX; 4];
        let close = [0.0; 4];
        let out = chop(&high, &low, &close, 2).unwrap();
        assert!(out[2..].iter().all(|v| (*v - 100.0).abs() < 1e-10));

        let tiny = f64::from_bits(1);
        let out = chop(&[tiny; 4], &[0.0; 4], &[0.0; 4], 2).unwrap();
        assert!(out[2..].iter().all(|v| (*v - 100.0).abs() < 1e-10));
    }

    #[test]
    fn scale_invariance_with_overflow_and_invalid_window_transfers() {
        let mut high = [
            1.0_f64, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0,
        ];
        let low: Vec<_> = high.iter().map(|&value| -value).collect();
        high[5] = f64::NAN;
        let close = [0.0; 13];
        let reference = chop(&high, &low, &close, 3).unwrap();
        let scale = f64::MAX / 2.0;
        let scaled_high: Vec<_> = high.iter().map(|&value| value * scale).collect();
        let scaled_low: Vec<_> = low.iter().map(|&value| value * scale).collect();
        let scaled = chop(&scaled_high, &scaled_low, &close, 3).unwrap();
        for i in 3..high.len() {
            if (5..=7).contains(&i) {
                assert!(reference[i].is_nan() && scaled[i].is_nan());
            } else {
                assert!((reference[i] - scaled[i]).abs() < 1e-10, "index {i}");
            }
        }
    }

    #[test]
    fn subnormal_ratio_uses_unrounded_logarithms() {
        let tiny = f64::from_bits(3);
        let out = chop(&[0.0, 2.0, tiny], &[0.0, 2.0, 0.0], &[2.0, 0.0, 0.0], 2).unwrap();
        let expected = 100.0 * ((tiny.ln() - 2.0_f64.ln()) / 2.0_f64.ln());
        assert!((out[2] - expected).abs() < 1e-9);
    }

    #[test]
    fn nonfinite_inputs_propagate_then_recover() {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for series in 0..3 {
                let mut high = [2.0_f32; 8];
                let mut low = [0.0_f32; 8];
                let mut close = [1.0_f32; 8];
                match series {
                    0 => high[2] = invalid,
                    1 => low[2] = invalid,
                    _ => close[2] = invalid,
                }
                let out = chop(&high, &low, &close, 2).unwrap();
                let first = if series == 2 { 3 } else { 2 };
                assert!(out[first].is_nan());
                assert!(out[first + 1].is_nan());
                assert!((out[first + 2] - 100.0).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn test_chop_shape() {
        let high: Vec<f64> = (0..200).map(|i| 120.0 + i as f64 * 0.2).collect();
        let low: Vec<f64> = high.iter().map(|v| v - 1.0).collect();
        let close: Vec<f64> = high.iter().map(|v| v - 0.4).collect();
        let out = chop(&high, &low, &close, 14).unwrap();
        assert_eq!(out.len(), close.len());
    }

    #[test]
    fn rolling_matches_reference_for_large_period_and_recovers() {
        let n = 4096;
        let period = 1024;
        let high: Vec<f64> = (0..n).map(|i| 100.0 + (i % 37) as f64 * 0.1).collect();
        let low: Vec<f64> = high.iter().map(|v| v - 2.0).collect();
        let mut close: Vec<f64> = high.iter().map(|v| v - 1.0).collect();
        close[1100] = f64::NAN;
        let out = chop(&high, &low, &close, period).unwrap();
        for i in period..n {
            let start = i + 1 - period;
            if (start..=i).any(|j| !close[j - 1].is_finite()) {
                assert!(out[i].is_nan());
                continue;
            }
            let mut sum = 0.0;
            let mut hh = f64::NEG_INFINITY;
            let mut ll = f64::INFINITY;
            for j in start..=i {
                sum += (high[j] - low[j])
                    .max((high[j] - close[j - 1]).abs())
                    .max((low[j] - close[j - 1]).abs());
                hh = hh.max(high[j]);
                ll = ll.min(low[j]);
            }
            let expected = 100.0 * ((sum / (hh - ll)).ln() / (period as f64).ln());
            assert!((out[i] - expected).abs() < 1e-10);
        }
        assert!(out[2125].is_finite());
    }

    #[test]
    fn block_aggregates_match_naive_windows_around_invalid_bars() {
        fn check(high: &[f64], low: &[f64], close: &[f64]) {
            for period in [2, 3, 7, 16, 48, 96] {
                let out = chop(high, low, close, period).unwrap();
                for i in 0..high.len() {
                    let start = (i + 1).saturating_sub(period);
                    let finite = |j: usize| {
                        high[j].is_finite() && low[j].is_finite() && close[j - 1].is_finite()
                    };
                    if i < period || !(start..=i).all(finite) {
                        assert!(out[i].is_nan(), "period {period}, index {i}");
                        continue;
                    }
                    let mut sum = 0.0;
                    let mut hh = f64::NEG_INFINITY;
                    let mut ll = f64::INFINITY;
                    for j in start..=i {
                        sum += (high[j] - low[j])
                            .max((high[j] - close[j - 1]).abs())
                            .max((low[j] - close[j - 1]).abs());
                        hh = hh.max(high[j]);
                        ll = ll.min(low[j]);
                    }
                    let expected = 100.0 * ((sum / (hh - ll)).ln() / (period as f64).ln());
                    assert!(
                        (out[i] - expected).abs() < 1e-10,
                        "period {period}, index {i}: {} != {expected}",
                        out[i]
                    );
                }
            }
        }

        // Integer bars keep every sum exact while extrema fall on both sides of
        // each block boundary.
        let n = 97_usize;
        let mut high: Vec<f64> = (0..n).map(|i| 200.0 + (i * 37 % 101) as f64).collect();
        let mut low: Vec<f64> = (0..n).map(|i| (i * 53 % 97) as f64).collect();
        let mut close: Vec<f64> = (0..n).map(|i| 100.0 + (i * 29 % 89) as f64).collect();
        check(&high, &low, &close);
        high[20] = f64::NAN;
        low[41] = f64::INFINITY;
        close[62] = f64::NEG_INFINITY;
        check(&high, &low, &close);
    }

    #[test]
    fn rolling_sum_recovers_after_overflowing_window() {
        let high = [2.0_f64, 1e308, 1e308, 2.0, 2.0, 2.0, 2.0];
        let low = [0.0; 7];
        let close = [1.0; 7];
        let out = chop(&high, &low, &close, 3).unwrap();
        let expected = 100.0 * (2.0_f64.ln() / 3.0_f64.ln());
        assert!((out[3] - expected).abs() < 1e-10);
        assert!((out[5] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn f32_output_and_buffer_boundaries() {
        let high = [2.0_f32; 8];
        let low = [0.0_f32; 8];
        let close = [1.0_f32; 8];
        let mut short = [0.0; 7];
        assert!(matches!(
            chop_into(&high, &low, &close, 3, &mut short),
            Err(Error::BufferTooSmall { .. })
        ));
        let mut out = [42.0; 9];
        assert_eq!(chop_into(&high, &low, &close, 3, &mut out).unwrap(), 5);
        assert!(out[..3].iter().all(|v| v.is_nan()));
        assert!(out[3..8].iter().all(|v| (*v - 100.0).abs() < 1e-5));
        assert_eq!(out[8], 42.0);
        assert!(matches!(
            chop_into(&high, &low, &close, 0, &mut out),
            Err(Error::InvalidPeriod { .. })
        ));
        assert!(matches!(
            chop_into(&high, &low, &close, usize::MAX, &mut out),
            Err(Error::InsufficientData { .. })
        ));
    }
}
