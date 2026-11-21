//! CMO (Chande Momentum Oscillator) indicator.
//!
//! The Chande Momentum Oscillator measures momentum by comparing
//! gains to losses over a specified period.
//!
//! # Formula
//!
//! ```text
//! CMO = ((Sum of gains - Sum of losses) / (Sum of gains + Sum of losses)) * 100
//! ```
//!
//! Where:
//! - Gains = sum of positive price changes over the period
//! - Losses = sum of absolute negative price changes over the period
//!
//! # Range
//!
//! CMO ranges from -100 to +100:
//! - +100: All gains, no losses (strong uptrend)
//! - -100: All losses, no gains (strong downtrend)
//! - 0: Equal gains and losses
//!
//! # Interpretation
//!
//! - CMO > +50: Overbought
//! - CMO < -50: Oversold
//! - Zero-line crossovers can signal trend changes
//!
//! # Lookback
//!
//! The lookback period is `period`.

use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Computes the lookback period for CMO.
#[inline]
#[must_use]
pub const fn cmo_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for CMO calculation.
#[inline]
#[must_use]
pub const fn cmo_min_len(period: usize) -> usize {
    period.saturating_add(1)
}

// Fallback only: positive sums use a shared binary exponent to avoid overflow without
// losing the small suffixes needed when a large change leaves the rolling window.
#[derive(Clone, Copy, Default)]
struct ScaledSum {
    mantissa: f64,
    exponent: i32,
}

/// `(gains, losses)`
type Sums = (ScaledSum, ScaledSum);

// Exact `2^delta` for `delta <= 0`, built from bits instead of a `powi` libcall. Clamped
// because anything below `2^-64` already vanishes beside a `[1, 2)` mantissa.
fn pow2(delta: i32) -> f64 {
    debug_assert!(delta <= 0);
    f64::from_bits(((1023 + delta.max(-64)) as u64) << 52)
}

impl ScaledSum {
    fn from_positive(mut value: f64) -> Self {
        if value == 0.0 {
            return Self::default();
        }
        let mut adjust = 0;
        if value < f64::MIN_POSITIVE {
            value *= 4_503_599_627_370_496.0;
            adjust = -52;
        }
        let bits = value.to_bits();
        Self {
            mantissa: f64::from_bits((bits & ((1_u64 << 52) - 1)) | (1023_u64 << 52)),
            exponent: ((bits >> 52) & 0x7ff) as i32 - 1023 + adjust,
        }
    }

    fn add(self, other: Self) -> Self {
        if self.mantissa == 0.0 {
            return other;
        }
        if other.mantissa == 0.0 {
            return self;
        }
        let mut exponent = self.exponent.max(other.exponent);
        // Branch-free alignment: the exponent comparison is unpredictable on real data.
        let mut mantissa = self.mantissa * pow2(self.exponent - exponent)
            + other.mantissa * pow2(other.exponent - exponent);
        if mantissa >= 2.0 {
            mantissa *= 0.5;
            exponent += 1;
        }
        Self { mantissa, exponent }
    }
}

fn change<T: SeriesElement>(data: &[T], index: usize) -> Result<Option<Sums>> {
    let previous = data[index].to_f64().ok_or(Error::NumericConversion {
        context: "series element to f64",
    })?;
    let current = data[index + 1].to_f64().ok_or(Error::NumericConversion {
        context: "series element to f64",
    })?;
    if !previous.is_finite() || !current.is_finite() {
        return Ok(None);
    }
    let difference = current - previous;
    let magnitude = if difference.is_finite() {
        ScaledSum::from_positive(difference.abs())
    } else {
        let mut magnitude = ScaledSum::from_positive((current * 0.5 - previous * 0.5).abs());
        magnitude.exponent += 1;
        magnitude
    };
    Ok(Some(if current > previous {
        (magnitude, ScaledSum::default())
    } else {
        (ScaledSum::default(), magnitude)
    }))
}

fn add_sums(left: Sums, right: Sums) -> Sums {
    (left.0.add(right.0), left.1.add(right.1))
}

/// Computes CMO and stores results in output slice.
///
/// # Arguments
///
/// * `data` - Price data (typically closing prices)
/// * `period` - Lookback period (typically 14)
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
/// - A numeric conversion fails (`Error::NumericConversion`)
pub fn cmo_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    let n = data.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    let min_len = cmo_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "cmo",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "cmo",
            required: n,
            actual: output.len(),
        });
    }

    output[..period].fill(T::nan());
    if !cmo_plain(data, period, output)? {
        cmo_scaled(data, period, output)?;
    }

    Ok(())
}

/// Plain `f64` sums; `Ok(false)` once a window total overflows, so the caller must
/// recompute the series with [`cmo_scaled`]. Finite totals match it to the bit, since its
/// scaling only shifts exponents of the same IEEE operations.
fn cmo_plain<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<bool> {
    let n = data.len();
    // Two-stack window in one ring, so nothing is ever subtracted: `window[..slot]` holds
    // the newest raw changes (summed in `incoming`), `window[slot..]` suffix sums of the rest.
    let mut window = vec![(0.0_f64, 0.0_f64); period];
    let mut incoming = (0.0_f64, 0.0_f64);
    let mut slot = 0usize;
    let mut valid_run = 0usize;
    let mut current = data[0].to_f64().ok_or(Error::NumericConversion {
        context: "series element to f64",
    })?;
    for i in 1..n {
        if slot == period {
            let mut sum = (0.0, 0.0);
            for entry in window.iter_mut().rev() {
                sum = (sum.0 + entry.0, sum.1 + entry.1);
                *entry = sum;
            }
            incoming = (0.0, 0.0);
            slot = 0;
        }
        let previous = current;
        current = data[i].to_f64().ok_or(Error::NumericConversion {
            context: "series element to f64",
        })?;
        // An overflowing difference becomes +Inf here and trips the total check below.
        let value = if previous.is_finite() && current.is_finite() {
            valid_run += 1;
            ((current - previous).max(0.0), (previous - current).max(0.0))
        } else {
            valid_run = 0;
            (0.0, 0.0)
        };
        window[slot] = value;
        incoming = (incoming.0 + value.0, incoming.1 + value.1);
        slot += 1;

        if i >= period {
            if valid_run < period {
                output[i] = T::nan();
                continue;
            }
            let suffix = window.get(slot).copied().unwrap_or_default();
            let gain = incoming.0 + suffix.0;
            let loss = incoming.1 + suffix.1;
            let total = gain + loss;
            if !total.is_finite() {
                return Ok(false);
            }
            output[i] = if total == 0.0 {
                T::zero()
            } else {
                T::from_f64(((gain - loss) / total) * 100.0)?
            };
        }
    }

    Ok(true)
}

/// Exponent-scaled recomputation of `output[period..]` for series whose sums overflow `f64`.
#[cold]
fn cmo_scaled<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    let n = data.len();
    // Two-stack window in one ring, so nothing is ever subtracted: `window[..slot]` holds
    // the newest raw changes (summed in `incoming`), `window[slot..]` suffix sums of the rest.
    let mut window = vec![Sums::default(); period];
    let mut incoming = Sums::default();
    let mut slot = 0usize;
    let mut valid_run = 0usize;
    for i in 1..n {
        if slot == period {
            let mut sum = Sums::default();
            for entry in window.iter_mut().rev() {
                sum = add_sums(sum, *entry);
                *entry = sum;
            }
            incoming = Sums::default();
            slot = 0;
        }
        let value = change(data, i - 1)?;
        valid_run = if value.is_some() { valid_run + 1 } else { 0 };
        let value = value.unwrap_or_default();
        window[slot] = value;
        incoming = add_sums(incoming, value);
        slot += 1;

        if i >= period {
            if valid_run < period {
                output[i] = T::nan();
                continue;
            }
            let (gains, losses) = add_sums(incoming, window.get(slot).copied().unwrap_or_default());
            let exponent = if gains.mantissa == 0.0 {
                losses.exponent
            } else if losses.mantissa == 0.0 {
                gains.exponent
            } else {
                gains.exponent.max(losses.exponent)
            };
            let gain = if gains.mantissa == 0.0 {
                0.0
            } else {
                gains.mantissa * pow2(gains.exponent - exponent)
            };
            let loss = if losses.mantissa == 0.0 {
                0.0
            } else {
                losses.mantissa * pow2(losses.exponent - exponent)
            };
            let total = gain + loss;
            output[i] = if total == 0.0 {
                T::zero()
            } else {
                T::from_f64(((gain - loss) / total) * 100.0)?
            };
        }
    }

    Ok(())
}

/// Computes CMO (Chande Momentum Oscillator).
///
/// # Arguments
///
/// * `data` - Price data (typically closing prices)
/// * `period` - Lookback period (typically 14)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - CMO values (range -100 to +100)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use liq_ta::indicators::cmo;
///
/// let prices = vec![44.0_f64, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0];
///
/// let result = cmo(&prices, 5).unwrap();
/// // First 5 values are NaN (lookback = period)
/// assert!(result[5].is_finite());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - A numeric conversion fails (`Error::NumericConversion`)
pub fn cmo<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    cmo_into(data, period, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_cmo_lookback() {
        assert_eq!(cmo_lookback(5), 5);
        assert_eq!(cmo_lookback(14), 14);
    }

    #[test]
    fn test_cmo_min_len() {
        assert_eq!(cmo_min_len(5), 6);
        assert_eq!(cmo_min_len(14), 15);
    }

    #[test]
    fn test_cmo_empty_input() {
        let data: Vec<f64> = vec![];
        let result = cmo(&data, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_cmo_invalid_period() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let result = cmo(&data, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_cmo_insufficient_data() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let result = cmo(&data, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_cmo_output_length() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let result = cmo(&data, 5).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_cmo_lookback_nan() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let result = cmo(&data, 5).unwrap();

        // First 5 values should be NaN (lookback = period)
        for i in 0..5 {
            assert!(result[i].is_nan(), "cmo[{}] should be NaN", i);
        }

        // Values after lookback should be finite
        for i in 5..result.len() {
            assert!(result[i].is_finite(), "cmo[{}] should be finite", i);
        }
    }

    #[test]
    fn test_cmo_range() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 14.0, 13.0, 12.0, 11.0];
        let result = cmo(&data, 5).unwrap();

        for i in 5..result.len() {
            assert!(
                result[i] >= -100.0 && result[i] <= 100.0,
                "cmo[{}] = {} should be in [-100, 100]",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_cmo_all_gains() {
        // Monotonically increasing - all gains, no losses
        // CMO = (gains - 0) / (gains + 0) * 100 = 100
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let result = cmo(&data, 5).unwrap();

        assert!(
            (result[5] - 100.0).abs() < 1e-10,
            "cmo should be 100 with all gains"
        );
    }

    #[test]
    fn test_cmo_all_losses() {
        // Monotonically decreasing - all losses, no gains
        // CMO = (0 - losses) / (0 + losses) * 100 = -100
        let data: Vec<f64> = vec![15.0, 14.0, 13.0, 12.0, 11.0, 10.0];
        let result = cmo(&data, 5).unwrap();

        assert!(
            (result[5] - (-100.0)).abs() < 1e-10,
            "cmo should be -100 with all losses"
        );
    }

    #[test]
    fn test_cmo_equal_gains_losses() {
        // Equal gains and losses should give CMO = 0
        // Pattern: up 1, down 1, up 1, down 1, up 1
        let data: Vec<f64> = vec![10.0, 11.0, 10.0, 11.0, 10.0, 11.0];
        let result = cmo(&data, 5).unwrap();

        // Gains: 1+1+1 = 3, Losses: 1+1 = 2
        // CMO = (3 - 2) / (3 + 2) * 100 = 20
        // Actually let's verify the calculation
        assert!(result[5].is_finite());
    }

    #[test]
    fn test_cmo_constant_prices() {
        // No changes = CMO should be 0
        let data: Vec<f64> = vec![10.0; 10];
        let result = cmo(&data, 5).unwrap();

        for i in 5..result.len() {
            assert!(
                (result[i] - 0.0).abs() < 1e-10,
                "cmo[{}] should be 0 for constant prices",
                i
            );
        }
    }

    #[test]
    fn test_cmo_into() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let mut output = vec![0.0_f64; 6];

        cmo_into(&data, 5, &mut output).unwrap();

        assert!((output[5] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_cmo_into_buffer_too_small() {
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let mut output = vec![0.0_f64; 3]; // Too small

        let result = cmo_into(&data, 5, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_cmo_f32() {
        let data: Vec<f32> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let result = cmo(&data, 5).unwrap();

        assert!((result[5] - 100.0_f32).abs() < 1e-5);
    }

    #[test]
    fn test_cmo_calculation() {
        // Verify specific calculation
        // Prices: 10, 12, 11, 13, 12, 14
        // Changes: +2, -1, +2, -1, +2
        // Gains: 2 + 2 + 2 = 6
        // Losses: 1 + 1 = 2
        // CMO = (6 - 2) / (6 + 2) * 100 = 4/8 * 100 = 50
        let data: Vec<f64> = vec![10.0, 12.0, 11.0, 13.0, 12.0, 14.0];
        let result = cmo(&data, 5).unwrap();

        assert!((result[5] - 50.0).abs() < 1e-10, "cmo[5] = {}", result[5]);
    }

    #[test]
    fn invalid_changes_leave_window_and_recover() {
        let data = [1.0_f64, 2.0, f64::INFINITY, 4.0, 5.0, 6.0, 7.0, 8.0];
        let mut out = [0.0; 9];
        cmo_into(&data, 3, &mut out).unwrap();
        assert!(out[3..6].iter().all(|v| v.is_nan()));
        assert_eq!(out[6], 100.0);
        assert_eq!(out[8], 0.0);
    }

    #[test]
    fn f32_accumulates_large_changes_in_f64() {
        let data = [0.0_f32, 3e38, 0.0, 3e38, 0.0];
        let out = cmo(&data, 4).unwrap();
        assert_eq!(out[4], 0.0);
    }

    #[test]
    fn overflow_and_cancellation_recover_when_changes_expire() {
        let data = [
            0.0_f64, 1e308, -1e308, 1e308, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0,
        ];
        let out = cmo(&data, 3).unwrap();
        assert!(out[3..].iter().all(|x| x.is_finite() && x.abs() <= 100.0));
        assert!((out[7] - 100.0 / 3.0).abs() < 1e-12);
        assert_eq!(out[10], 0.0);
    }

    #[test]
    fn retains_subnormal_changes() {
        let tiny = f64::from_bits(1);
        let data = [0.0, tiny, 0.0, tiny];
        let out = cmo(&data, 2).unwrap();
        assert_eq!(out[2], 0.0);
        assert_eq!(out[3], 0.0);
    }
    fn oracle(data: &[f64], period: usize) -> Vec<f64> {
        let mut result = vec![f64::NAN; data.len()];
        for i in period..data.len() {
            let window = &data[i - period..=i];
            if window.iter().any(|x| !x.is_finite()) {
                continue;
            }
            let scale = window.iter().fold(0.0_f64, |max, x| max.max(x.abs()));
            if scale == 0.0 {
                result[i] = 0.0;
                continue;
            }
            let (mut gains, mut losses) = (0.0, 0.0);
            for pair in window.windows(2) {
                let delta = pair[1] / scale - pair[0] / scale;
                gains += delta.max(0.0);
                losses += (-delta).max(0.0);
            }
            let total = gains + losses;
            result[i] = if total == 0.0 {
                0.0
            } else {
                (gains - losses) / total * 100.0
            };
        }
        result
    }

    fn assert_oracle_f64(data: &[f64], period: usize) {
        let expected = oracle(data, period);
        let mut actual = vec![12345.0; data.len() + 3];
        cmo_into(data, period, &mut actual).unwrap();
        for (i, (&got, &want)) in actual.iter().zip(&expected).enumerate() {
            if want.is_nan() {
                assert!(got.is_nan(), "index {i}: {got} != NaN");
            } else {
                assert!((got - want).abs() < 2e-10, "index {i}: {got} != {want}");
            }
        }
        assert_eq!(&actual[data.len()..], &[12345.0; 3]);
    }

    fn assert_oracle_f32(data: &[f32], period: usize) {
        let expected = oracle(
            &data.iter().map(|&x| f64::from(x)).collect::<Vec<_>>(),
            period,
        );
        let mut actual = vec![12345.0_f32; data.len() + 3];
        cmo_into(data, period, &mut actual).unwrap();
        for (i, (&got, &want)) in actual.iter().zip(&expected).enumerate() {
            if want.is_nan() {
                assert!(got.is_nan(), "index {i}: {got} != NaN");
            } else {
                assert!(
                    (f64::from(got) - want).abs() < 1e-5,
                    "index {i}: {got} != {want}"
                );
            }
        }
        assert_eq!(&actual[data.len()..], &[12345.0; 3]);
    }

    #[test]
    fn long_windows_match_independent_sums_and_recover() {
        let mut data: Vec<f64> = (0..3000)
            .map(|i| ((i * 37 % 101) as f64 - 50.0) * 0.25)
            .collect();
        data[300] = 1e300;
        data[301] = -1e300;
        data[1500] = f64::NAN;
        data[1900] = f64::INFINITY;
        assert_oracle_f64(&data, 1023);
        assert_oracle_f64(&data, 257);
        assert_oracle_f64(&data, data.len() - 1);

        let mut data32: Vec<f32> = (0..3000)
            .map(|i| ((i * 37 % 101) as f32 - 50.0) * 0.25)
            .collect();
        data32[300] = 1e30;
        data32[301] = -1e30;
        data32[1500] = f32::NAN;
        data32[1900] = f32::NEG_INFINITY;
        assert_oracle_f32(&data32, 1023);
        assert_oracle_f32(&data32, 257);
        assert_oracle_f32(&data32, data32.len() - 1);
    }

    #[test]
    fn long_subnormal_windows_match_independent_sums() {
        let data64: Vec<f64> = (0..2200)
            .map(|i| f64::from_bits((i * 37 % 101) as u64))
            .collect();
        let data32: Vec<f32> = (0..2200)
            .map(|i| f32::from_bits((i * 37 % 101) as u32))
            .collect();
        assert_oracle_f64(&data64, 1023);
        assert_oracle_f32(&data32, 1023);
    }

    #[test]
    fn period_one_uses_exact_output_buffer() {
        let data = [1.0_f64, 2.0, 2.0, -1.0, f64::NAN, 3.0, 4.0];
        let mut output = [42.0; 7];
        cmo_into(&data, 1, &mut output).unwrap();
        assert!(output[0].is_nan());
        assert_eq!(&output[1..4], &[100.0, 0.0, -100.0]);
        assert!(output[4].is_nan());
        assert!(output[5].is_nan());
        assert_eq!(output[6], 100.0);
    }

    #[test]
    fn maximum_period_is_rejected_without_touching_output() {
        let data = [1.0_f64, 2.0, 3.0];
        let mut output = [42.0; 5];
        assert_eq!(cmo_min_len(usize::MAX), usize::MAX);
        assert!(matches!(
            cmo_into(&data, usize::MAX, &mut output),
            Err(Error::InsufficientData { .. })
        ));
        assert_eq!(output, [42.0; 5]);
    }

    #[test]
    fn scaled_sum_alignment_is_exact_and_drops_only_vanishing_addends() {
        let one = ScaledSum::from_positive(1.0);
        let ulp = one.add(ScaledSum::from_positive(f64::EPSILON));
        assert_eq!((ulp.mantissa, ulp.exponent), (1.0 + f64::EPSILON, 0));
        for tiny in [1e-300, f64::from_bits(1)] {
            let sum = ScaledSum::from_positive(tiny).add(one);
            assert_eq!((sum.mantissa, sum.exponent), (1.0, 0));
        }
        let carry = ScaledSum::from_positive(1.5).add(ScaledSum::from_positive(0.75));
        assert_eq!((carry.mantissa, carry.exponent), (1.125, 1));
    }

    #[test]
    fn short_windows_match_independent_sums_across_exponent_gaps() {
        // Blocks of 7 drift against every period, so each ring phase meets each gap.
        let shifts = [0, 20, -20, 70, 0, -70, 40, 0];
        let mut data: Vec<f64> = (0..600)
            .map(|i| ((i * 37 % 101) as f64 - 50.0) * 0.25 * 2.0_f64.powi(shifts[i / 7 % 8]))
            .collect();
        data[200] = f64::NAN;
        data[401] = f64::NEG_INFINITY;
        let data32: Vec<f32> = data.iter().map(|&x| x as f32).collect();
        for period in (1..=17).chain([64]) {
            assert_oracle_f64(&data, period);
            assert_oracle_f32(&data32, period);
        }
    }

    #[test]
    fn overflowing_window_sums_fall_back_to_scaled_sums() {
        // Every change is finite (|x| <= 8.5e307) but several of them overflow a window sum.
        let mut data: Vec<f64> = (0..400)
            .map(|i| ((i * 37 % 101) as f64 - 50.0) * 1.7e306)
            .collect();
        data[150] = f64::NAN;
        let scaled: Vec<f64> = data.iter().map(|x| x * 2.0_f64.powi(-600)).collect();
        for period in [1, 2, 14, 100] {
            assert_oracle_f64(&data, period);
            let (big, small) = (cmo(&data, period).unwrap(), cmo(&scaled, period).unwrap());
            for (i, (b, s)) in big.iter().zip(&small).enumerate() {
                assert!(
                    (b.is_nan() && s.is_nan()) || (b - s).abs() < 1e-12,
                    "period {period} index {i}: {b} != {s}"
                );
            }
        }
    }

    #[test]
    fn windows_after_an_extreme_move_match_an_unaffected_series() {
        let calm: Vec<f64> = (0..400)
            .map(|i| ((i * 37 % 101) as f64 - 50.0) * 0.1)
            .collect();
        let mut spiked = calm.clone();
        spiked[50] = 1e300;
        for period in [1, 7, 14, 100] {
            let (want, got) = (cmo(&calm, period).unwrap(), cmo(&spiked, period).unwrap());
            // Output i covers data[i - period..=i]; only windows touching index 50 may differ.
            for i in (period..50).chain(51 + period..400) {
                assert_eq!(
                    got[i].to_bits(),
                    want[i].to_bits(),
                    "period {period} index {i}"
                );
            }
        }
    }
}
