//! Awesome Oscillator (AO).
//!
//! For f32 inputs, High mode calculates midpoints and means in f64 and rounds
//! the final difference once; Fast mode uses native f32 arithmetic throughout.
#![expect(
    missing_docs,
    reason = "indicator output structs and helpers are documented at module level"
)]

use crate::error::{Error, Result};
use crate::indicators::sma::{sma_lookback, sma_min_len};
use crate::precision::{PrecisionMode, current_precision_mode};
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
    let mut out = vec![T::nan(); high.len()];
    ao_into(high, low, &mut out)?;
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
    if std::any::TypeId::of::<T>() == std::any::TypeId::of::<f32>()
        && current_precision_mode() == PrecisionMode::High
    {
        calculate(
            high,
            low,
            output,
            |v| v.to_f64().expect("f32 always widens to f64"),
            |v| {
                num_traits::NumCast::from(v)
                    .expect("f64 to f32 saturates to infinity instead of failing")
            },
        )?;
    } else {
        calculate(high, low, output, |v| v, |v| v)?;
    }
    Ok(n - ao_lookback())
}

// Sliding sum as the previous block's suffix plus the current block's prefix.
// Nothing is ever subtracted, so an outlier, NaN or Inf leaves no residue once
// it exits the window. `slots[k]` holds this block's sample for `k < pos` and
// the previous block's suffix sum from `k` otherwise; the last slot stays zero.
struct BlockSum<A: SeriesElement, const SLOTS: usize> {
    slots: [A; SLOTS],
    prefix: A,
    pos: usize,
}

impl<A: SeriesElement, const SLOTS: usize> BlockSum<A, SLOTS> {
    fn new() -> Self {
        let mut slots = [A::nan(); SLOTS];
        slots[SLOTS - 1] = A::zero();
        Self {
            slots,
            prefix: A::zero(),
            pos: 0,
        }
    }

    #[inline]
    fn push(&mut self, value: A) -> A {
        self.prefix = self.prefix + value;
        let sum = self.slots[self.pos + 1] + self.prefix;
        self.slots[self.pos] = value;
        self.pos += 1;
        if self.pos == SLOTS - 1 {
            let mut suffix = value;
            for slot in self.slots[1..SLOTS - 2].iter_mut().rev() {
                suffix = suffix + *slot;
                *slot = suffix;
            }
            self.prefix = A::zero();
            self.pos = 0;
        }
        sum
    }
}

// Offset of the newest invalid sample, scanned only when a window sum is non-finite.
#[cold]
#[inline(never)]
fn newest_invalid<T: SeriesElement>(high: &[T], low: &[T]) -> Option<usize> {
    high.iter()
        .zip(low)
        .rposition(|(h, l)| !(h.is_finite() && l.is_finite()))
}

fn calculate<T: SeriesElement, A: SeriesElement>(
    high: &[T],
    low: &[T],
    output: &mut [T],
    to_acc: impl Fn(T) -> A,
    from_acc: impl Fn(A) -> T,
) -> Result<()> {
    let half = A::one() / A::two();
    let fast_weight = A::one() / <A as SeriesElement>::from_usize(AO_FAST_PERIOD)?;
    let slow_weight = A::one() / <A as SeriesElement>::from_usize(AO_SLOW_PERIOD)?;
    // Midpoint halving with 2^-7 headroom for overflowing windows; exact powers of two.
    let down = <A as SeriesElement>::from_f64(1.0 / 256.0)?;
    let up = <A as SeriesElement>::from_f64(128.0)?;
    let mut fast = BlockSum::<A, { AO_FAST_PERIOD + 1 }>::new();
    let mut slow = BlockSum::<A, { AO_SLOW_PERIOD + 1 }>::new();
    // Outputs before this index are NaN: warm-up, or a known invalid sample in the window.
    let mut nan_until = ao_lookback();
    let mut overflow = None;
    for (i, ((&h, &l), out)) in high.iter().zip(low).zip(output.iter_mut()).enumerate() {
        // Invalid samples and overflowed sums both stay non-finite in every block
        // sum covering them, so a finite difference needs no further checks.
        let value = (to_acc(h) + to_acc(l)) * half;
        let diff = fast.push(value) * fast_weight - slow.push(value) * slow_weight;
        *out = if diff.is_finite() {
            from_acc(diff)
        } else if i < nan_until {
            T::nan()
        } else {
            let start = i - ao_lookback();
            let invalid = newest_invalid(&high[start..=i], &low[start..=i]);
            let Some(offset) = invalid else {
                overflow = Some(i);
                break;
            };
            nan_until = start + offset + AO_SLOW_PERIOD;
            T::nan()
        };
    }
    let Some(first) = overflow else {
        return Ok(());
    };

    // A plain sum overflowed. From this window on, also keep sums of midpoints scaled
    // by 2^-7, of which 34 cannot overflow; they are non-finite only for invalid samples.
    // This doubles the remaining per-sample work.
    let mut scaled_fast = BlockSum::<A, { AO_FAST_PERIOD + 1 }>::new();
    let mut scaled_slow = BlockSum::<A, { AO_SLOW_PERIOD + 1 }>::new();
    let mut scaled = |h: T, l: T| {
        let value = to_acc(h) * down + to_acc(l) * down;
        scaled_fast.push(value) * fast_weight - scaled_slow.push(value) * slow_weight
    };
    let start = first - ao_lookback();
    let mut scaled_diff = A::nan();
    for (&h, &l) in high[start..=first].iter().zip(&low[start..=first]) {
        scaled_diff = scaled(h, l);
    }
    let samples = high.iter().zip(low).zip(output.iter_mut());
    for (i, ((&h, &l), out)) in samples.enumerate().skip(first) {
        if i > first {
            let value = (to_acc(h) + to_acc(l)) * half;
            let diff = fast.push(value) * fast_weight - slow.push(value) * slow_weight;
            scaled_diff = scaled(h, l);
            if diff.is_finite() {
                *out = from_acc(diff);
                continue;
            }
        }
        *out = if scaled_diff.is_finite() {
            from_acc(scaled_diff * up)
        } else {
            T::nan()
        };
    }
    Ok(())
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

    fn check_reference<T: SeriesElement + std::fmt::Debug>(high: &[T], low: &[T], tolerance: f64) {
        let median: Vec<f64> = high
            .iter()
            .zip(low)
            .map(|(&h, &l)| {
                if h.is_finite() && l.is_finite() {
                    (h.to_f64().unwrap() + l.to_f64().unwrap()) / 2.0
                } else {
                    f64::NAN
                }
            })
            .collect();
        let mut actual = vec![T::one(); high.len() + 3];
        assert_eq!(
            ao_into(high, low, &mut actual).unwrap(),
            high.len() - ao_lookback()
        );
        for i in 0..high.len() {
            let expected = if i >= ao_lookback() {
                let window = &median[i + 1 - AO_SLOW_PERIOD..=i];
                if window.iter().all(|v| v.is_finite()) {
                    let slow = window.iter().sum::<f64>() / AO_SLOW_PERIOD as f64;
                    let fast = window[AO_SLOW_PERIOD - AO_FAST_PERIOD..]
                        .iter()
                        .sum::<f64>()
                        / AO_FAST_PERIOD as f64;
                    fast - slow
                } else {
                    f64::NAN
                }
            } else {
                f64::NAN
            };
            let value = actual[i].to_f64().unwrap();
            assert!(
                (value.is_nan() && expected.is_nan()) || (value - expected).abs() <= tolerance,
                "index {i}: {value} != {expected}"
            );
        }
        assert!(actual[high.len()..].iter().all(|&v| v == T::one()));
        let mut short = vec![T::one(); high.len() - 1];
        assert!(matches!(
            ao_into(high, low, &mut short),
            Err(Error::BufferTooSmall { .. })
        ));
        assert!(short.iter().all(|&v| v == T::one()));
    }

    #[test]
    fn independent_reference_and_invalid_ring_offsets() {
        use crate::precision::{PrecisionMode, with_precision_mode};

        for mode in [PrecisionMode::High, PrecisionMode::Fast] {
            with_precision_mode(mode, || {
                let tolerance = if mode == PrecisionMode::High {
                    1e-6
                } else {
                    1e-4
                };
                let high: Vec<f32> = (0..70).map(|i| 50.0 + i as f32 / 7.0).collect();
                check_reference(&high, &high, tolerance);
                let mut initial = high.clone();
                initial[0] = f32::NAN;
                check_reference(&initial, &high, tolerance);
                for offset in 0..AO_SLOW_PERIOD {
                    let mut high: Vec<f32> = (0..150)
                        .map(|i| 100.0 + (i * 37 % 127) as f32 / 13.0)
                        .collect();
                    let low: Vec<f32> = high.iter().map(|v| v - 3.17).collect();
                    high[40 + offset] = if offset % 2 == 0 {
                        f32::NAN
                    } else {
                        f32::INFINITY
                    };
                    check_reference(&high, &low, tolerance);
                }
                let high: Vec<f32> = (0..150)
                    .map(|i| ((i * 71 % 211) as f32 - 105.0) / 7.3)
                    .collect();
                let mut low: Vec<f32> = high.iter().map(|&v| v - 0.37).collect();
                low[61] = f32::NEG_INFINITY;
                low[62] = f32::NAN;
                check_reference(&high, &low, tolerance);
            });
        }
        let high: Vec<f64> = (0..70).map(|i| 50.0 + i as f64 / 7.0).collect();
        check_reference(&high, &high, 1e-10);
        let mut initial = high.clone();
        initial[0] = f64::INFINITY;
        check_reference(&initial, &high, 1e-10);
        for offset in 0..AO_SLOW_PERIOD {
            let mut high: Vec<f64> = (0..150)
                .map(|i| 100.0 + (i * 37 % 127) as f64 / 13.0)
                .collect();
            let low: Vec<f64> = high.iter().map(|v| v - 3.17).collect();
            high[40 + offset] = if offset % 2 == 0 {
                f64::NAN
            } else {
                f64::NEG_INFINITY
            };
            check_reference(&high, &low, 1e-10);
        }
    }

    #[test]
    fn mixed_sign_reference_across_stack_transfers() {
        let high: Vec<f64> = (0..400)
            .map(|i| ((i * 71 % 211) as f64 - 105.0) / 7.3)
            .collect();
        let low: Vec<f64> = high
            .iter()
            .enumerate()
            .map(|(i, &v)| v - (i % 9) as f64 / 11.0)
            .collect();
        check_reference(&high, &low, 1e-10);
    }

    #[test]
    fn f32_precision_modes_handle_small_signals() {
        use crate::precision::{PrecisionMode, with_precision_mode};

        for mode in [PrecisionMode::High, PrecisionMode::Fast] {
            with_precision_mode(mode, || {
                let mut high = [100_000.0_f32; AO_SLOW_PERIOD];
                let low = high;
                high[AO_SLOW_PERIOD - 1] = f32::from_bits(100_000.0_f32.to_bits() + 1);
                let change = (f64::from(high[AO_SLOW_PERIOD - 1]) - 100_000.0) / 2.0;
                let expected = change * (1.0 / 5.0 - 1.0 / 34.0);
                let result = ao(&high, &low).unwrap();
                if mode == PrecisionMode::High {
                    assert!((f64::from(result[AO_SLOW_PERIOD - 1]) - expected).abs() < 1e-9);
                } else {
                    assert_eq!(result[AO_SLOW_PERIOD - 1], 0.0);
                }

                let tiny = f32::from_bits(1);
                let mut high = [-tiny; AO_SLOW_PERIOD];
                high[AO_SLOW_PERIOD - AO_FAST_PERIOD..].fill(tiny);
                let low = [0.0_f32; AO_SLOW_PERIOD];
                let result = ao(&high, &low).unwrap();
                if mode == PrecisionMode::High {
                    assert_eq!(result[AO_SLOW_PERIOD - 1].to_bits(), tiny.to_bits());
                } else {
                    assert_eq!(result[AO_SLOW_PERIOD - 1], 0.0);
                }
            });
        }
    }

    #[test]
    fn adjacent_and_trailing_invalids_recover_by_window() {
        let mut high = [1.0_f64; 100];
        let low = high;
        high[41] = f64::NAN;
        high[42] = f64::INFINITY;
        high[99] = f64::NEG_INFINITY;
        let result = ao(&high, &low).unwrap();
        assert_eq!(result[40], 0.0);
        assert!(result[41..76].iter().all(|v| v.is_nan()));
        assert!(result[76..99].iter().all(|&v| v == 0.0));
        assert!(result[99].is_nan());
    }

    #[test]
    fn extremes_leave_without_losing_the_remaining_window() {
        let max = [f64::MAX; 80];
        let result = ao(&max, &max).unwrap();
        // Flat windows can round to a few ulps of the scaled sums, but must stay finite.
        assert!(
            result[ao_lookback()..]
                .iter()
                .all(|&v| v.abs() <= f64::MAX * 1e-15)
        );
        for mode in [PrecisionMode::High, PrecisionMode::Fast] {
            crate::precision::with_precision_mode(mode, || {
                let max32 = [f32::MAX; 80];
                let result = ao(&max32, &max32).unwrap();
                assert!(
                    result[ao_lookback()..]
                        .iter()
                        .all(|&v| v.abs() <= f32::MAX * 1e-6)
                );
                let mut values = [1.0_f32; 100];
                values[40] = f32::MAX;
                let result = ao(&values, &values).unwrap();
                assert!(result[74..].iter().all(|&v| v == 0.0));
            });
        }
        let alternating: Vec<f64> = (0..80)
            .map(|i| if i % 2 == 0 { f64::MAX } else { -f64::MAX })
            .collect();
        let result = ao(&alternating, &alternating).unwrap();
        for (i, &value) in result.iter().enumerate().skip(ao_lookback()) {
            let expected = if i % 2 == 0 { 0.2 } else { -0.2 };
            assert!((value / f64::MAX - expected).abs() < 1e-14);
        }
        let mut values = [1.0_f64; 100];
        values[40] = 1e155;
        let result = ao(&values, &values).unwrap();
        assert!(result[74..].iter().all(|&v| v == 0.0));
        let subnormal = [f64::from_bits(1); 80];
        let result = ao(&subnormal, &subnormal).unwrap();
        assert!(result[ao_lookback()..].iter().all(|&v| v == 0.0));
    }

    #[test]
    fn short_input_does_not_modify_output() {
        let high = [1.0_f64; AO_SLOW_PERIOD - 1];
        let mut output = [42.0; AO_SLOW_PERIOD];
        assert!(matches!(
            ao_into(&high, &high, &mut output),
            Err(Error::InsufficientData { .. })
        ));
        assert_eq!(output, [42.0; AO_SLOW_PERIOD]);
    }

    #[test]
    fn outliers_and_invalids_leave_no_residue_at_any_block_phase() {
        let clean_high: Vec<f64> = (0..150)
            .map(|i| ((i * 71 % 211) as f64 - 105.0) / 7.3e3)
            .collect();
        let clean_low: Vec<f64> = clean_high
            .iter()
            .enumerate()
            .map(|(i, &v)| v - (i % 9) as f64 / 11.0e4)
            .collect();
        let clean = ao(&clean_high, &clean_low).unwrap();
        for offset in 0..AO_SLOW_PERIOD {
            let at = 40 + offset;
            for outlier in [1e300, -1e300, f64::MAX, -f64::MAX, f64::INFINITY, f64::NAN] {
                let (mut high, mut low) = (clean_high.clone(), clean_low.clone());
                high[at] = outlier;
                low[at] = outlier;
                let result = ao(&high, &low).unwrap();
                // Bit-identical outside the windows that contain the outlier.
                assert_eq!(result[ao_lookback()..at], clean[ao_lookback()..at]);
                assert_eq!(result[at + AO_SLOW_PERIOD..], clean[at + AO_SLOW_PERIOD..]);
                for &value in &result[at..at + AO_SLOW_PERIOD] {
                    if outlier.is_finite() {
                        assert!(value.is_finite(), "offset {offset}, outlier {outlier}");
                    } else {
                        assert!(value.is_nan(), "offset {offset}, outlier {outlier}");
                    }
                }
            }
        }
    }

    #[test]
    fn overflowing_block_sums_match_exactly_scaled_series() {
        // Sums of the large series overflow and switch to scaled sums; the series scaled
        // by an exact power of two never overflows. Invalid samples fall before the
        // first overflow, inside the scaled phase, and as a (+Inf, -Inf) pair.
        let large: Vec<f64> = (0..160)
            .map(|i| {
                let magnitude = f64::MAX * (0.5 + (i * 37 % 127) as f64 / 254.0);
                if i % 3 == 0 { -magnitude } else { magnitude }
            })
            .collect();
        let (mut high, mut low) = (large.clone(), large);
        high[20] = f64::NAN;
        high[80] = f64::INFINITY;
        low[80] = f64::NEG_INFINITY;
        low[120] = f64::NAN;
        let scale = |values: &[f64]| values.iter().map(|v| v / 64.0).collect::<Vec<_>>();
        let result = ao(&high, &low).unwrap();
        let expected = ao(&scale(&high), &scale(&low)).unwrap();
        for i in ao_lookback()..high.len() {
            if [20, 80, 120]
                .iter()
                .any(|&j| (j..j + AO_SLOW_PERIOD).contains(&i))
            {
                assert!(result[i].is_nan() && expected[i].is_nan(), "index {i}");
                continue;
            }
            assert!(result[i].is_finite(), "index {i}");
            let error = (result[i] / 64.0 - expected[i]).abs() / (f64::MAX / 64.0);
            assert!(error < 1e-14, "index {i}: relative error {error}");
        }
    }

    #[test]
    fn staggered_invalids_extend_the_invalid_region() {
        let mut high: Vec<f64> = (0..200)
            .map(|i| 100.0 + (i * 37 % 127) as f64 / 13.0)
            .collect();
        let mut low: Vec<f64> = high.iter().map(|v| v - 3.17).collect();
        high[40] = f64::NAN;
        low[57] = f64::INFINITY;
        // Last index still covered by 57, then the first index free of 90.
        high[90] = f64::NAN;
        low[124] = f64::NEG_INFINITY;
        check_reference(&high, &low, 1e-10);
    }
}
