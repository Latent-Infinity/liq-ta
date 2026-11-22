//! Ulcer Index indicator.
//!
//! RMS of percent drawdowns from the rolling peak close over `period` bars.
//! Output starts at index `period - 1`. Until index `2 * period - 2`, early
//! drawdowns use the shorter peak history available; Martin's standard
//! definition (`StockCharts`, pandas-ta) returns NaN there.
#![expect(
    missing_docs,
    reason = "indicator output structs and helpers are documented at module level"
)]

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use std::collections::VecDeque;

fn push_norm(stack: &mut Vec<(f64, f64)>, value: f64) {
    let previous = stack.last().map_or(0.0, |entry| entry.1);
    let norm = if value.is_nan() {
        previous
    } else {
        previous.hypot(value)
    };
    stack.push((value, norm));
}

#[inline]
#[must_use]
pub const fn ulcer_index_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

#[inline]
#[must_use]
pub const fn ulcer_index_min_len(period: usize) -> usize {
    period
}

fn validate<T: SeriesElement>(data: &[T], period: usize) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }
    if data.len() < ulcer_index_min_len(period) {
        return Err(Error::InsufficientData {
            required: ulcer_index_min_len(period),
            actual: data.len(),
            indicator: "ulcer_index",
        });
    }
    Ok(())
}

#[must_use = "this returns a Result with Ulcer Index values, which should be used"]
pub fn ulcer_index<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    validate(data, period)?;
    let mut out = vec![T::nan(); data.len()];
    ulcer_index_into(data, period, &mut out)?;
    Ok(out)
}

#[must_use = "this returns a Result with the count of valid Ulcer Index values"]
pub fn ulcer_index_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    validate(data, period)?;
    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "ulcer_index",
        });
    }
    output[..data.len()].fill(T::nan());
    // Report input conversion errors before converting any output, as before.
    for &value in data {
        value.to_f64().ok_or(Error::NumericConversion {
            context: "series element to f64",
        })?;
    }
    let mut peaks: VecDeque<(usize, f64)> = VecDeque::with_capacity(period);
    let mut last_invalid = None;
    let mut incoming = Vec::with_capacity(period);
    let mut outgoing = Vec::new();
    let mut invalid_drawdowns = 0_usize;
    let root_period = (period as f64).sqrt();
    for (i, &element) in data.iter().enumerate() {
        let value = element.to_f64().ok_or(Error::NumericConversion {
            context: "series element to f64",
        })?;
        let start = (i + 1).saturating_sub(period);
        while peaks.front().is_some_and(|&(index, _)| index < start) {
            peaks.pop_front();
        }
        if value.is_finite() {
            while peaks.back().is_some_and(|&(_, peak)| peak <= value) {
                peaks.pop_back();
            }
            peaks.push_back((i, value));
        } else {
            last_invalid = Some(i);
        }

        // Each drawdown uses its own partial or full peak history.
        let peak = peaks.front().map_or(0.0, |&(_, peak)| peak);
        let normalized = if last_invalid.is_some_and(|index| index >= start) || peak == 0.0 {
            f64::NAN
        } else {
            let ratio = value / peak;
            if ratio.is_finite() {
                let delta = ratio - 1.0;
                let pct = 100.0 * delta;
                if pct.is_finite() {
                    pct / root_period
                } else {
                    (delta / root_period) * 100.0
                }
            } else {
                ((value / root_period) / peak - 1.0 / root_period) * 100.0
            }
        };

        if i >= period {
            if outgoing.is_empty() {
                outgoing.reserve_exact(incoming.len());
                while let Some((drawdown, _)) = incoming.pop() {
                    push_norm(&mut outgoing, drawdown);
                }
            }
            if outgoing.pop().expect("full drawdown window").0.is_nan() {
                invalid_drawdowns -= 1;
            }
        }
        invalid_drawdowns += usize::from(normalized.is_nan());
        push_norm(&mut incoming, normalized);
        if i >= period - 1 && invalid_drawdowns == 0 {
            let left = outgoing.last().map_or(0.0, |entry| entry.1);
            let right = incoming.last().map_or(0.0, |entry| entry.1);
            output[i] = <T as SeriesElement>::from_f64(left.hypot(right))?;
        }
    }
    Ok(data.len().saturating_sub(ulcer_index_lookback(period)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(data: &[f64], period: usize) -> Vec<f64> {
        let drawdowns: Vec<_> = (0..data.len())
            .map(|i| {
                let window = &data[(i + 1).saturating_sub(period)..=i];
                if window.iter().any(|value| !value.is_finite()) {
                    return f64::NAN;
                }
                let peak = window.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                if peak == 0.0 {
                    f64::NAN
                } else {
                    (data[i] / peak - 1.0) * 100.0 / (period as f64).sqrt()
                }
            })
            .collect();
        (0..data.len())
            .map(|i| {
                if i + 1 < period {
                    return f64::NAN;
                }
                let window = &drawdowns[i + 1 - period..=i];
                if window.iter().any(|value| value.is_nan()) {
                    f64::NAN
                } else {
                    window
                        .iter()
                        .fold(0.0_f64, |norm, value| norm.hypot(*value))
                }
            })
            .collect()
    }

    #[test]
    fn rolling_results_match_direct_windows() {
        let mut data: Vec<f64> = (0..600)
            .map(|i| 20.0 + (i % 31) as f64 - (i % 7) as f64 * 3.0)
            .collect();
        data[14] = f64::NAN;
        data[39] = f64::INFINITY;
        data[83] = f64::NEG_INFINITY;
        data[115..118].fill(0.0);
        data[321] = -1e155;
        data[450] = -1e306;
        for period in [1, 2, 3, 9, 64, 257] {
            let expected = reference(&data, period);
            let actual = ulcer_index(&data, period).unwrap();
            for (index, (&actual, &expected)) in actual.iter().zip(&expected).enumerate() {
                if expected.is_nan() {
                    assert!(actual.is_nan(), "period {period}, index {index}");
                } else if expected == 0.0 {
                    assert_eq!(actual, 0.0, "period {period}, index {index}");
                } else {
                    assert!(
                        (actual / expected - 1.0).abs() < 1e-12,
                        "period {period}, index {index}"
                    );
                }
            }
        }

        let mut data32: Vec<f32> = data.iter().map(|&value| value as f32).collect();
        data32[321] = -1e30;
        data32[450] = -1e35;
        let as_f64: Vec<f64> = data32.iter().map(|&value| f64::from(value)).collect();
        for period in [1, 2, 9, 64, 257] {
            let expected = reference(&as_f64, period);
            let actual = ulcer_index(&data32, period).unwrap();
            for (index, (&actual, &expected)) in actual.iter().zip(&expected).enumerate() {
                let expected = expected as f32;
                if expected.is_nan() {
                    assert!(actual.is_nan(), "period {period}, index {index}");
                } else if expected.is_infinite() {
                    assert_eq!(actual, expected, "period {period}, index {index}");
                } else if expected == 0.0 {
                    assert_eq!(actual, 0.0, "period {period}, index {index}");
                } else {
                    assert!(
                        (actual / expected - 1.0).abs() < 1e-5,
                        "period {period}, index {index}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_ulcer_index_shape() {
        let data: Vec<f64> = (0..160)
            .map(|i| 100.0 + (i as f64 * 0.2) - ((i % 10) as f64 * 0.3))
            .collect();
        let out = ulcer_index(&data, 14).unwrap();
        assert_eq!(out.len(), data.len());
    }

    #[test]
    fn uses_each_observations_own_peak() {
        let rising = ulcer_index(&[100.0_f64, 110.0, 120.0, 130.0], 3).unwrap();
        assert!(rising[0].is_nan() && rising[1].is_nan());
        assert_eq!(&rising[2..], &[0.0, 0.0]);

        let out = ulcer_index(&[100.0_f64, 90.0, 80.0, 100.0, 90.0], 3).unwrap();
        assert!((out[2] - (500.0_f64 / 3.0).sqrt()).abs() < 1e-12);
        assert!((out[3] - (500.0_f64 / 3.0).sqrt()).abs() < 1e-12);
        assert!((out[4] - (500.0_f64 / 3.0).sqrt()).abs() < 1e-12);
        assert_eq!(ulcer_index(&[5.0_f64, 4.0, 6.0], 1).unwrap(), vec![0.0; 3]);
        assert!(ulcer_index(&[1.0_f64], 0).is_err());
        assert!(ulcer_index(&[1.0_f64], usize::MAX).is_err());
    }

    #[test]
    fn large_drawdowns_preserve_representable_rms() {
        let out = ulcer_index(&[1.0_f64, -1e155, -1e155, 1.0, 1.0], 2).unwrap();
        let expected = 1e157_f64;
        assert!(out[0].is_nan());
        assert!((out[1] / (expected / 2.0_f64.sqrt()) - 1.0).abs() < 1e-14);
        assert!((out[2] / (expected / 2.0_f64.sqrt()) - 1.0).abs() < 1e-14);
        assert_eq!(out[4], 0.0);

        // Both the squared terms and their unnormalized sum would overflow.
        let out = ulcer_index(&[1.0_f64, -1e306, 1.0, -1e306], 3).unwrap();
        assert!((out[3] / (1e308 * (2.0_f64 / 3.0).sqrt()) - 1.0).abs() < 1e-14);
        // The percentage itself overflows before division by sqrt(period).
        let out = ulcer_index(&[1.0_f64, -2e306], 2).unwrap();
        assert!((out[1] / (2e306 / 2.0_f64.sqrt() * 100.0) - 1.0).abs() < 1e-14);

        // The ratio itself overflows, but the normalized drawdown does not.
        let mut data = vec![1e-308_f64; 40_000];
        data[39_999] = -2.0;
        let out = ulcer_index(&data, data.len()).unwrap();
        assert!((out[39_999] / 1e308 - 1.0).abs() < 1e-14);

        let out = ulcer_index(&[1.0_f32, -1e30, -1e30, 1.0, 1.0], 2).unwrap();
        let expected = 1e30_f32 * 100.0;
        assert!(out[0].is_nan());
        assert!((out[1] / (expected / 2.0_f32.sqrt()) - 1.0).abs() < 1e-6);
        assert!((out[2] / (expected / 2.0_f32.sqrt()) - 1.0).abs() < 1e-6);
        assert_eq!(out[4], 0.0);
    }

    #[test]
    fn overflow_and_invalid_windows_recover() {
        let out = ulcer_index(&[1.0_f64, -1e308, 1.0, 1.0], 2).unwrap();
        assert!(out[1].is_infinite() && out[2].is_infinite());
        assert_eq!(out[3], 0.0);
        let out = ulcer_index(&[1.0_f32, -1e38, 1.0, 1.0], 2).unwrap();
        assert!(out[1].is_infinite() && out[2].is_infinite());
        assert_eq!(out[3], 0.0);

        let out = ulcer_index(&[10.0_f32, f32::NAN, 9.0, 10.0, 11.0], 2).unwrap();
        assert!(out[0..=3].iter().all(|value| value.is_nan()));
        assert_eq!(out[4], 0.0);

        let mut output = [42.0_f32; 4];
        assert_eq!(
            ulcer_index_into(&[5.0_f32, 4.0, 6.0], 1, &mut output).unwrap(),
            3
        );
        assert_eq!(output, [0.0, 0.0, 0.0, 42.0]);
        assert!(ulcer_index_into(&[1.0_f64, 2.0], 2, &mut [0.0]).is_err());
        assert!(ulcer_index::<f32>(&[], 1).is_err());
    }

    #[test]
    fn invalid_values_and_zero_peaks_recover() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let out = ulcer_index(&[10.0, 9.0, bad, 8.0, 9.0, 10.0], 2).unwrap();
            assert!(out[2..=4].iter().all(|value| value.is_nan()));
            assert!(out[5].is_finite());
        }
        let out = ulcer_index(&[0.0_f64, 1.0, 2.0], 2).unwrap();
        assert!(out[1].is_nan());
        assert_eq!(out[2], 0.0);
    }
}
