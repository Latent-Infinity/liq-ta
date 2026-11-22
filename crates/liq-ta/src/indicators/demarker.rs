//! DeMarker indicator.
#![expect(
    missing_docs,
    reason = "public helpers and output structs are not yet documented"
)]

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

#[derive(Clone, Copy, Default)]
struct Magnitude {
    value: f64,
    exponent: i32,
}

impl Magnitude {
    fn difference(a: f64, b: f64) -> Self {
        let diff = a - b;
        if diff.is_finite() {
            Self {
                value: diff.max(0.0),
                exponent: 0,
            }
        } else if diff.is_sign_negative() {
            Self::default()
        } else {
            Self {
                value: (a * 0.5 - b * 0.5).max(0.0),
                exponent: 1,
            }
        }
    }

    fn add(self, other: Self) -> Self {
        let exponent = self.exponent.max(other.exponent);
        let a = if self.exponent == exponent {
            self.value
        } else {
            self.value * 2.0_f64.powi(self.exponent - exponent)
        };
        let b = if other.exponent == exponent {
            other.value
        } else {
            other.value * 2.0_f64.powi(other.exponent - exponent)
        };
        let value = a + b;
        if value.is_infinite() {
            Self {
                value: a * 0.5 + b * 0.5,
                exponent: exponent + 1,
            }
        } else {
            Self { value, exponent }
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Totals {
    max: Magnitude,
    min: Magnitude,
    invalid: usize,
}

impl Totals {
    fn add(self, other: Self) -> Self {
        Self {
            max: self.max.add(other.max),
            min: self.min.add(other.min),
            invalid: self.invalid + other.invalid,
        }
    }
}

fn push_sum(stack: &mut Vec<(Totals, Totals)>, value: Totals) {
    let sum = stack.last().map_or(value, |(_, sum)| sum.add(value));
    stack.push((value, sum));
}

#[inline]
#[must_use]
pub const fn demarker_lookback(period: usize) -> usize {
    period
}

#[inline]
#[must_use]
pub const fn demarker_min_len(period: usize) -> usize {
    period.saturating_add(1)
}

fn validate<T: SeriesElement>(high: &[T], low: &[T], period: usize) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if high.is_empty() {
        return Err(Error::EmptyInput);
    }
    if low.len() != high.len() {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", high.len(), low.len()),
        });
    }
    if period == usize::MAX {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period is too large",
        });
    }
    let required = demarker_min_len(period);
    if high.len() < required {
        return Err(Error::InsufficientData {
            required,
            actual: high.len(),
            indicator: "demarker",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with DeMarker values, which should be used"]
pub fn demarker<T: SeriesElement>(high: &[T], low: &[T], period: usize) -> Result<Vec<T>> {
    validate(high, low, period)?;
    let mut out = vec![T::nan(); high.len()];
    demarker_into(high, low, period, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid DeMarker values"]
pub fn demarker_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    validate(high, low, period)?;
    let n = high.len();
    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "demarker",
        });
    }
    output[..n].fill(T::nan());

    // Window sum = previous-block suffix + current-block prefix, so expired values are never
    // subtracted. Blocks align with the robust path's stack transfers, keeping the same
    // addition order. `ring[period]` stays zero: the empty suffix of the block's last slot.
    let mut ring = vec![(0.0_f64, 0.0_f64); period + 1];
    let (mut prefix_max, mut prefix_min) = (0.0_f64, 0.0_f64);
    let mut slot = 0;
    let mut valid = 0_usize;
    let mut prev_h = to_f64(high[0])?;
    let mut prev_l = to_f64(low[0])?;
    for i in 1..n {
        if slot == period {
            for s in (0..period).rev() {
                ring[s].0 += ring[s + 1].0;
                ring[s].1 += ring[s + 1].1;
            }
            (prefix_max, prefix_min, slot) = (0.0, 0.0, 0);
        }
        let h = to_f64(high[i])?;
        let l = to_f64(low[i])?;
        let value = if h.is_finite() && prev_h.is_finite() && l.is_finite() && prev_l.is_finite() {
            valid += 1;
            ((h - prev_h).max(0.0), (prev_l - l).max(0.0))
        } else {
            valid = 0;
            (0.0, 0.0)
        };
        (prev_h, prev_l) = (h, l);
        prefix_max += value.0;
        prefix_min += value.1;
        let (suffix_max, suffix_min) = ring[slot + 1];
        ring[slot] = value;
        slot += 1;
        if valid >= period {
            let max = suffix_max + prefix_max;
            let min = suffix_min + prefix_min;
            // Sums are non-negative, so overflow (including an infinite difference) shows as +Inf.
            if max.max(min) == f64::INFINITY {
                demarker_robust(high, low, period, output)?;
                break;
            }
            output[i] = <T as SeriesElement>::from_f64(ratio(max, min))?;
        }
    }
    Ok(n.saturating_sub(demarker_lookback(period)))
}

fn to_f64<T: SeriesElement>(value: T) -> Result<f64> {
    value.to_f64().ok_or(Error::NumericConversion {
        context: "series element to f64",
    })
}

fn ratio(max: f64, min: f64) -> f64 {
    if max == 0.0 && min == 0.0 {
        0.5
    } else if max >= min {
        1.0 / (1.0 + min / max)
    } else {
        let ratio = max / min;
        ratio / (1.0 + ratio)
    }
}

/// Exponent-scaled recomputation of the whole series for windows whose plain sums overflow.
#[cold]
fn demarker_robust<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();
    output[..n].fill(T::nan());
    // Aggregate stacks discard expired values without subtractive cancellation.
    let mut incoming = Vec::with_capacity(period);
    let mut outgoing = Vec::new();
    for i in 1..n {
        if i > period {
            if outgoing.is_empty() {
                if outgoing.capacity() == 0 {
                    outgoing.reserve(period);
                }
                while let Some((value, _)) = incoming.pop() {
                    push_sum(&mut outgoing, value);
                }
            }
            outgoing.pop();
        }
        let h = to_f64(high[i])?;
        let prev_h = to_f64(high[i - 1])?;
        let l = to_f64(low[i])?;
        let prev_l = to_f64(low[i - 1])?;
        let value =
            if !h.is_finite() || !prev_h.is_finite() || !l.is_finite() || !prev_l.is_finite() {
                Totals {
                    invalid: 1,
                    ..Totals::default()
                }
            } else {
                Totals {
                    max: Magnitude::difference(h, prev_h),
                    min: Magnitude::difference(prev_l, l),
                    invalid: 0,
                }
            };
        push_sum(&mut incoming, value);
        if i >= period {
            let left = outgoing
                .last()
                .map_or_else(Totals::default, |(_, sum)| *sum);
            let right = incoming
                .last()
                .map_or_else(Totals::default, |(_, sum)| *sum);
            let sum = left.add(right);
            if sum.invalid == 0 {
                let exponent = sum.max.exponent.max(sum.min.exponent);
                let max = if sum.max.exponent == exponent {
                    sum.max.value
                } else {
                    sum.max.value * 2.0_f64.powi(sum.max.exponent - exponent)
                };
                let min = if sum.min.exponent == exponent {
                    sum.min.value
                } else {
                    sum.min.value * 2.0_f64.powi(sum.min.exponent - exponent)
                };
                output[i] = <T as SeriesElement>::from_f64(ratio(max, min))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demarker_bounds() {
        let high: Vec<f64> = (0..100).map(|i| 100.0 + i as f64 * 0.5).collect();
        let low: Vec<f64> = high.iter().map(|v| v - 2.0).collect();
        let out = demarker(&high, &low, 14).unwrap();
        let valid = out.iter().copied().filter(|v| v.is_finite());
        for v in valid {
            assert!((0.0..=1.0).contains(&v));
        }
    }

    #[test]
    fn reference_windows_and_recovery() {
        let high = [10.0_f64, 12.0, 11.0, 13.0, 14.0, 15.0, 16.0, 17.0];
        let low = [9.0, 10.0, 8.0, 7.0, 8.0, 9.0, 10.0, 11.0];
        let out = demarker(&high, &low, 2).unwrap();
        assert!(out[0].is_nan() && out[1].is_nan());
        assert!((out[2] - 0.5).abs() < 1e-12);
        assert!((out[3] - 0.4).abs() < 1e-12);
        assert_eq!(out[4], 0.75);
        assert_eq!(demarker(&high, &low, 1).unwrap()[1], 1.0);
        assert_eq!(demarker(&high, &low, 1).unwrap()[2], 0.0);
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut damaged = high;
            damaged[2] = bad;
            let result = demarker(&damaged, &low, 2).unwrap();
            assert!(result[2..=4].iter().all(|x| x.is_nan()));
            assert_eq!(result[5], out[5]);
            let result = demarker(&high, &damaged, 2).unwrap();
            assert!(result[2..=4].iter().all(|x| x.is_nan()));
            assert!(result[5].is_finite());
        }
    }

    #[test]
    fn invalid_bar_at_stack_transfer_recovers() {
        let low = [0.0_f64; 10];
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut high = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
            high[4] = bad;
            let out = demarker(&high, &low, 3).unwrap();
            assert_eq!(out[3], 1.0);
            assert!(out[4..=7].iter().all(|value| value.is_nan()));
            assert_eq!(&out[8..], &[1.0, 1.0]);
        }
    }

    #[test]
    fn large_moves_leave_without_poisoning_rolling_sums() {
        let high = [0.0_f64, 1e20, 0.0, 0.0, 0.0, 1.0];
        let low = [0.0_f64, 0.0, 0.0, -1.0, -1.0, -1.0];
        let out = demarker(&high, &low, 2).unwrap();
        assert_eq!(&out[3..], &[0.0, 0.0, 1.0]);
        let high32 = high.map(|x| x as f32);
        let low32 = low.map(|x| x as f32);
        let out32 = demarker(&high32, &low32, 2).unwrap();
        assert_eq!(&out32[3..], &[0.0, 0.0, 1.0]);

        let max = f64::MAX;
        let out = demarker(&[-max, max, max], &[max, -max, -max], 1).unwrap();
        assert_eq!(out[1], 0.5);
        assert_eq!(out[2], 0.5);
    }

    #[test]
    fn subnormal_movements_remain_visible() {
        let tiny = f64::from_bits(1);
        let out = demarker(&[0.0, tiny, tiny], &[0.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(out[2], 1.0);
        let out = demarker(&[0.0, tiny, tiny], &[tiny, 0.0, 0.0], 2).unwrap();
        assert_eq!(out[2], 0.5);
    }

    #[test]
    fn mixed_exponents_and_overflowing_move_expiry() {
        let m = f64::MAX;
        let high = [-m, m, m, 0.0, 1.0, 2.0];
        let low = [0.0, -m, -m, -m, -m, -m];
        let out = demarker(&high, &low, 2).unwrap();
        // In units of MAX, the first window has gains 2 and losses 1.
        assert!((out[2] - 2.0 / 3.0).abs() < 1e-15);
        assert_eq!(&out[3..], &[0.5, 1.0, 1.0]);

        let m = f32::MAX;
        let high = [-m, m, m, 0.0, 1.0, 2.0];
        let low = [0.0, -m, -m, -m, -m, -m];
        let out = demarker(&high, &low, 2).unwrap();
        assert!((out[2] - 2.0 / 3.0).abs() < f32::EPSILON);
        assert_eq!(&out[3..], &[0.5, 1.0, 1.0]);
    }

    #[test]
    fn long_period_matches_direct_window_sums() {
        let period = 10_000;
        let high: Vec<f64> = (0..20_123).map(|i| ((i * 7) % 101) as f64).collect();
        let low: Vec<f64> = (0..high.len()).map(|i| ((i * 13) % 97) as f64).collect();
        let out = demarker(&high, &low, period).unwrap();
        for i in [period, period + 1, 15_000, high.len() - 1] {
            let mut gains = 0.0;
            let mut losses = 0.0;
            for j in i + 1 - period..=i {
                gains += (high[j] - high[j - 1]).max(0.0);
                losses += (low[j - 1] - low[j]).max(0.0);
            }
            assert!((out[i] - gains / (gains + losses)).abs() < 1e-14);
        }
    }

    #[test]
    fn into_buffer_boundaries() {
        let high = [1.0_f64, 2.0, 3.0, 4.0];
        let low = [1.0_f64, 1.0, 1.0, 1.0];
        let mut short = [7.0; 3];
        assert!(matches!(
            demarker_into(&high, &low, 2, &mut short),
            Err(Error::BufferTooSmall {
                required: 4,
                actual: 3,
                ..
            })
        ));
        assert_eq!(short, [7.0; 3]);
        let mut output = [7.0; 6];
        assert_eq!(demarker_into(&high, &low, 2, &mut output).unwrap(), 2);
        assert!(output[..2].iter().all(|v| v.is_nan()));
        assert_eq!(&output[2..], &[1.0, 1.0, 7.0, 7.0]);
        let mut exact = [7.0; 4];
        assert_eq!(demarker_into(&high, &low, 3, &mut exact).unwrap(), 1);
        assert!(exact[..3].iter().all(|v| v.is_nan()));
        assert_eq!(exact[3], 1.0);
    }

    #[test]
    fn overflowing_window_sums_fall_back_to_scaled_sums() {
        let high = [0.0_f64, 7.0, 0.0, 7.0, 0.0, 7.0, 1.0, 2.0, 3.0];
        let low = [7.0_f64, 0.0, 6.0, 1.0, 5.0, 2.0, 4.0, 3.0, 3.0];
        // Inputs and differences stay finite, but window sums reach 14 * 2^1021 > f64::MAX.
        let scale = 2.0_f64.powi(1021);
        let expected = demarker(&high, &low, 3).unwrap();
        let out = demarker(&high.map(|x| x * scale), &low.map(|x| x * scale), 3).unwrap();
        assert!(out[..3].iter().all(|x| x.is_nan()));
        // Power-of-two scaling is exact, so the scaled ratios are bit-identical.
        assert_eq!(&out[3..], &expected[3..]);
    }

    #[test]
    fn window_after_extreme_move_matches_unaffected_series() {
        let period = 3;
        let base_high: Vec<f64> = (0..20).map(|i| ((i * 7) % 11) as f64 * 0.37).collect();
        let base_low: Vec<f64> = (0..20).map(|i| ((i * 5) % 13) as f64 * 0.29).collect();
        let (mut high, mut low) = (base_high.clone(), base_low.clone());
        high[1] = 1e300;
        low[1] = -1e300;
        let expected = demarker(&base_high, &base_low, period).unwrap();
        let out = demarker(&high, &low, period).unwrap();
        // Bars 1 and 2 carry the extreme moves; windows ending at 2 + period exclude both.
        assert_eq!(&out[2 + period..], &expected[2 + period..]);
    }

    #[test]
    fn oversized_period_is_rejected() {
        assert_eq!(demarker_min_len(usize::MAX), usize::MAX);
        assert!(matches!(
            demarker(&[1.0], &[1.0], usize::MAX),
            Err(Error::InvalidPeriod { .. })
        ));
    }
}
