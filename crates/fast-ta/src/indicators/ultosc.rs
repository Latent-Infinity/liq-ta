//! ULTOSC (Ultimate Oscillator) indicator.
//!
//! The Ultimate Oscillator is a technical indicator that measures momentum
//! across three different timeframes.
//!
//! # Formula
//!
//! ```text
//! BP = Close - min(Low, Prior Close)
//! TR = max(High, Prior Close) - min(Low, Prior Close)
//! Avg = Sum(BP, period) / Sum(TR, period)
//! ULTOSC = 100 * ((4 * Avg1) + (2 * Avg2) + Avg3) / 7
//! ```
//!
//! Where:
//! - BP is Buying Pressure
//! - TR is True Range
//! - Avg1, Avg2, Avg3 are averages for short, medium, long periods
//!
//! # Range
//!
//! ULTOSC ranges from 0 to 100:
//! - > 70: Overbought
//! - < 30: Oversold
//!
//! # Lookback
//!
//! The lookback period is max(period1, period2, period3).

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Computes the lookback period for ULTOSC.
#[inline]
#[must_use]
pub const fn ultosc_lookback(period1: usize, period2: usize, period3: usize) -> usize {
    // The longest period determines the lookback
    let max12 = if period1 > period2 { period1 } else { period2 };
    if max12 > period3 {
        max12
    } else {
        period3
    }
}

/// Returns the minimum input length required for ULTOSC calculation.
#[inline]
#[must_use]
pub const fn ultosc_min_len(period1: usize, period2: usize, period3: usize) -> usize {
    ultosc_lookback(period1, period2, period3) + 1
}

/// Computes ULTOSC and stores results in output slice.
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period1` - First (short) period (default 7)
/// * `period2` - Second (medium) period (default 14)
/// * `period3` - Third (long) period (default 28)
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn ultosc_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period1: usize,
    period2: usize,
    period3: usize,
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    if low.len() != n || close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "Arrays must have same length: high={}, low={}, close={}",
                n,
                low.len(),
                close.len()
            ),
        });
    }

    if period1 == 0 {
        return Err(Error::InvalidPeriod {
            period: period1,
            reason: "period1 must be at least 1",
        });
    }

    if period2 == 0 {
        return Err(Error::InvalidPeriod {
            period: period2,
            reason: "period2 must be at least 1",
        });
    }

    if period3 == 0 {
        return Err(Error::InvalidPeriod {
            period: period3,
            reason: "period3 must be at least 1",
        });
    }

    let min_len = ultosc_min_len(period1, period2, period3);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "ultosc",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "ultosc",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = ultosc_lookback(period1, period2, period3);
    let hundred = T::from_f64(100.0)?;
    let inv_seven = T::from_f64(1.0 / 7.0)?;
    let four = T::from_f64(4.0)?;
    let two = T::from_f64(2.0)?;
    let zero = T::zero();

    // Calculate BP and TR for all bars
    let mut bp = vec![zero; n];
    let mut tr = vec![zero; n];
    let mut invalid_flags = vec![false; n];

    // First bar has no prior close, so BP and TR are 0
    for i in 1..n {
        let prior_close = close[i - 1];
        if is_invalid(high[i])
            || is_invalid(low[i])
            || is_invalid(close[i])
            || is_invalid(prior_close)
        {
            invalid_flags[i] = true;
            continue;
        }
        let true_low = if low[i] < prior_close {
            low[i]
        } else {
            prior_close
        };
        let true_high = if high[i] > prior_close {
            high[i]
        } else {
            prior_close
        };

        bp[i] = close[i] - true_low;
        tr[i] = true_high - true_low;
    }

    // Fill lookback period with NaN
    for out in output.iter_mut().take(lookback) {
        *out = T::nan();
    }

    // Initialize rolling sums for all three periods
    // We calculate initial sums for the first valid output position (at index = lookback)
    let mut sum_bp1 = zero;
    let mut sum_tr1 = zero;
    let mut sum_bp2 = zero;
    let mut sum_tr2 = zero;
    let mut sum_bp3 = zero;
    let mut sum_tr3 = zero;
    let mut invalid_count1 = 0usize;
    let mut invalid_count2 = 0usize;
    let mut invalid_count3 = 0usize;

    // Period 1: window [lookback + 1 - period1 .. lookback]
    for j in (lookback + 1 - period1)..=lookback {
        if invalid_flags[j] {
            invalid_count1 += 1;
        } else {
            sum_bp1 = sum_bp1 + bp[j];
            sum_tr1 = sum_tr1 + tr[j];
        }
    }

    // Period 2: window [lookback + 1 - period2 .. lookback]
    for j in (lookback + 1 - period2)..=lookback {
        if invalid_flags[j] {
            invalid_count2 += 1;
        } else {
            sum_bp2 = sum_bp2 + bp[j];
            sum_tr2 = sum_tr2 + tr[j];
        }
    }

    // Period 3: window [lookback + 1 - period3 .. lookback]
    for j in (lookback + 1 - period3)..=lookback {
        if invalid_flags[j] {
            invalid_count3 += 1;
        } else {
            sum_bp3 = sum_bp3 + bp[j];
            sum_tr3 = sum_tr3 + tr[j];
        }
    }

    // Calculate first ULTOSC at index = lookback
    if invalid_count1 > 0 || invalid_count2 > 0 || invalid_count3 > 0 {
        output[lookback] = T::nan();
    } else {
        let avg1 = if sum_tr1 == zero { zero } else { sum_bp1 / sum_tr1 };
        let avg2 = if sum_tr2 == zero { zero } else { sum_bp2 / sum_tr2 };
        let avg3 = if sum_tr3 == zero { zero } else { sum_bp3 / sum_tr3 };
        output[lookback] = hundred * ((four * avg1) + (two * avg2) + avg3) * inv_seven;
    }

    // Rolling calculation for remaining values
    for i in (lookback + 1)..n {
        // Update period 1 rolling sum
        let old_idx1 = i - period1;
        if invalid_flags[old_idx1] {
            invalid_count1 = invalid_count1.saturating_sub(1);
        } else {
            sum_bp1 = sum_bp1 - bp[old_idx1];
            sum_tr1 = sum_tr1 - tr[old_idx1];
        }
        if invalid_flags[i] {
            invalid_count1 += 1;
        } else {
            sum_bp1 = sum_bp1 + bp[i];
            sum_tr1 = sum_tr1 + tr[i];
        }

        // Update period 2 rolling sum
        let old_idx2 = i - period2;
        if invalid_flags[old_idx2] {
            invalid_count2 = invalid_count2.saturating_sub(1);
        } else {
            sum_bp2 = sum_bp2 - bp[old_idx2];
            sum_tr2 = sum_tr2 - tr[old_idx2];
        }
        if invalid_flags[i] {
            invalid_count2 += 1;
        } else {
            sum_bp2 = sum_bp2 + bp[i];
            sum_tr2 = sum_tr2 + tr[i];
        }

        // Update period 3 rolling sum
        let old_idx3 = i - period3;
        if invalid_flags[old_idx3] {
            invalid_count3 = invalid_count3.saturating_sub(1);
        } else {
            sum_bp3 = sum_bp3 - bp[old_idx3];
            sum_tr3 = sum_tr3 - tr[old_idx3];
        }
        if invalid_flags[i] {
            invalid_count3 += 1;
        } else {
            sum_bp3 = sum_bp3 + bp[i];
            sum_tr3 = sum_tr3 + tr[i];
        }

        // Calculate averages and ULTOSC
        if invalid_count1 > 0 || invalid_count2 > 0 || invalid_count3 > 0 {
            output[i] = T::nan();
        } else {
            let avg1 = if sum_tr1 == zero { zero } else { sum_bp1 / sum_tr1 };
            let avg2 = if sum_tr2 == zero { zero } else { sum_bp2 / sum_tr2 };
            let avg3 = if sum_tr3 == zero { zero } else { sum_bp3 / sum_tr3 };
            output[i] = hundred * ((four * avg1) + (two * avg2) + avg3) * inv_seven;
        }
    }

    Ok(())
}

/// Computes ULTOSC (Ultimate Oscillator).
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period1` - First (short) period (default 7)
/// * `period2` - Second (medium) period (default 14)
/// * `period3` - Third (long) period (default 28)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - ULTOSC values (range 0 to 100)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::ultosc;
///
/// let high = vec![25.0, 26.0, 27.0, 28.0, 27.5, 27.0, 26.5, 27.0, 27.5, 28.0];
/// let low = vec![23.0, 24.0, 25.0, 26.0, 25.5, 25.0, 24.5, 25.0, 25.5, 26.0];
/// let close = vec![24.0, 25.0, 26.0, 27.0, 26.5, 26.0, 25.5, 26.5, 27.0, 27.5];
///
/// let result = ultosc(&high, &low, &close, 3, 5, 7).unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn ultosc<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    period1: usize,
    period2: usize,
    period3: usize,
) -> Result<Vec<T>> {
    let mut output = vec![T::zero(); high.len()];
    ultosc_into(high, low, close, period1, period2, period3, &mut output)?;
    Ok(output)
}

/// Simple ULTOSC with common defaults (7, 14, 28).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn ultosc_default<T: SeriesElement>(high: &[T], low: &[T], close: &[T]) -> Result<Vec<T>> {
    ultosc(high, low, close, 7, 14, 28)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_ultosc_lookback() {
        assert_eq!(ultosc_lookback(7, 14, 28), 28);
        assert_eq!(ultosc_lookback(3, 5, 7), 7);
    }

    #[test]
    fn test_ultosc_min_len() {
        assert_eq!(ultosc_min_len(7, 14, 28), 29);
        assert_eq!(ultosc_min_len(3, 5, 7), 8);
    }

    #[test]
    fn test_ultosc_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];
        let result = ultosc(&high, &low, &close, 3, 5, 7);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_ultosc_invalid_period() {
        let high: Vec<f64> = vec![10.0; 20];
        let low: Vec<f64> = vec![9.0; 20];
        let close: Vec<f64> = vec![9.5; 20];
        let result = ultosc(&high, &low, &close, 0, 5, 7);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_ultosc_length_mismatch() {
        let high: Vec<f64> = vec![10.0; 20];
        let low: Vec<f64> = vec![9.0; 15];
        let close: Vec<f64> = vec![9.5; 20];
        let result = ultosc(&high, &low, &close, 3, 5, 7);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_ultosc_insufficient_data() {
        let high: Vec<f64> = vec![10.0; 5];
        let low: Vec<f64> = vec![9.0; 5];
        let close: Vec<f64> = vec![9.5; 5];
        let result = ultosc(&high, &low, &close, 3, 5, 7);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_ultosc_output_length() {
        let high: Vec<f64> = vec![25.0, 26.0, 27.0, 28.0, 27.5, 27.0, 26.5, 27.0, 27.5, 28.0];
        let low: Vec<f64> = vec![23.0, 24.0, 25.0, 26.0, 25.5, 25.0, 24.5, 25.0, 25.5, 26.0];
        let close: Vec<f64> = vec![24.0, 25.0, 26.0, 27.0, 26.5, 26.0, 25.5, 26.5, 27.0, 27.5];
        let result = ultosc(&high, &low, &close, 3, 5, 7).unwrap();
        assert_eq!(result.len(), high.len());
    }

    #[test]
    fn test_ultosc_lookback_nan() {
        let high: Vec<f64> = vec![25.0, 26.0, 27.0, 28.0, 27.5, 27.0, 26.5, 27.0, 27.5, 28.0];
        let low: Vec<f64> = vec![23.0, 24.0, 25.0, 26.0, 25.5, 25.0, 24.5, 25.0, 25.5, 26.0];
        let close: Vec<f64> = vec![24.0, 25.0, 26.0, 27.0, 26.5, 26.0, 25.5, 26.5, 27.0, 27.5];
        let result = ultosc(&high, &low, &close, 3, 5, 7).unwrap();

        let lookback = ultosc_lookback(3, 5, 7);
        // Values up to lookback should be NaN
        for i in 0..lookback {
            assert!(result[i].is_nan(), "ultosc[{}] should be NaN", i);
        }

        // Values after lookback should be finite
        for i in lookback..result.len() {
            assert!(result[i].is_finite(), "ultosc[{}] should be finite", i);
        }
    }

    #[test]
    fn test_ultosc_range() {
        let high: Vec<f64> = vec![25.0, 26.0, 27.0, 28.0, 27.5, 27.0, 26.5, 27.0, 27.5, 28.0];
        let low: Vec<f64> = vec![23.0, 24.0, 25.0, 26.0, 25.5, 25.0, 24.5, 25.0, 25.5, 26.0];
        let close: Vec<f64> = vec![24.0, 25.0, 26.0, 27.0, 26.5, 26.0, 25.5, 26.5, 27.0, 27.5];
        let result = ultosc(&high, &low, &close, 3, 5, 7).unwrap();

        let lookback = ultosc_lookback(3, 5, 7);
        for i in lookback..result.len() {
            assert!(
                result[i] >= 0.0 && result[i] <= 100.0,
                "ultosc[{}] = {} should be in [0, 100]",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_ultosc_into() {
        let high: Vec<f64> = vec![25.0, 26.0, 27.0, 28.0, 27.5, 27.0, 26.5, 27.0, 27.5, 28.0];
        let low: Vec<f64> = vec![23.0, 24.0, 25.0, 26.0, 25.5, 25.0, 24.5, 25.0, 25.5, 26.0];
        let close: Vec<f64> = vec![24.0, 25.0, 26.0, 27.0, 26.5, 26.0, 25.5, 26.5, 27.0, 27.5];
        let mut output = vec![0.0_f64; 10];

        ultosc_into(&high, &low, &close, 3, 5, 7, &mut output).unwrap();

        let lookback = ultosc_lookback(3, 5, 7);
        assert!(output[lookback].is_finite());
    }

    #[test]
    fn test_ultosc_into_buffer_too_small() {
        let high: Vec<f64> = vec![25.0, 26.0, 27.0, 28.0, 27.5, 27.0, 26.5, 27.0, 27.5, 28.0];
        let low: Vec<f64> = vec![23.0, 24.0, 25.0, 26.0, 25.5, 25.0, 24.5, 25.0, 25.5, 26.0];
        let close: Vec<f64> = vec![24.0, 25.0, 26.0, 27.0, 26.5, 26.0, 25.5, 26.5, 27.0, 27.5];
        let mut output = vec![0.0_f64; 5]; // Too small

        let result = ultosc_into(&high, &low, &close, 3, 5, 7, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_ultosc_f32() {
        let high: Vec<f32> = vec![25.0, 26.0, 27.0, 28.0, 27.5, 27.0, 26.5, 27.0, 27.5, 28.0];
        let low: Vec<f32> = vec![23.0, 24.0, 25.0, 26.0, 25.5, 25.0, 24.5, 25.0, 25.5, 26.0];
        let close: Vec<f32> = vec![24.0, 25.0, 26.0, 27.0, 26.5, 26.0, 25.5, 26.5, 27.0, 27.5];
        let result = ultosc(&high, &low, &close, 3, 5, 7).unwrap();

        let lookback = ultosc_lookback(3, 5, 7);
        assert!(result[lookback].is_finite());
    }

    #[test]
    fn test_ultosc_default() {
        let n = 35;
        let high: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let low: Vec<f64> = (0..n).map(|i| 98.0 + (i as f64) * 0.5).collect();
        let close: Vec<f64> = (0..n).map(|i| 99.0 + (i as f64) * 0.5).collect();

        let result = ultosc_default(&high, &low, &close).unwrap();
        assert_eq!(result.len(), n);
    }
}
