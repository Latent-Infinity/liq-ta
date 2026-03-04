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

/// Computes the lookback period for ULTOSC.
#[inline]
#[must_use]
pub const fn ultosc_lookback(period1: usize, period2: usize, period3: usize) -> usize {
    // The longest period determines the lookback
    let max12 = if period1 > period2 { period1 } else { period2 };
    if max12 > period3 { max12 } else { period3 }
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

    // Precompute constants - hundred/seven eliminates one division per output
    let hundred_seventh = T::from_f64(100.0 / 7.0)?; // ~14.285714...
    let four = T::from_f64(4.0)?;
    let two = T::from_f64(2.0)?;

    // Calculate BP and TR for all bars using branchless min/max
    // Optimization: Use uninitialized allocation for f64/f32 since we write all elements
    use std::any::TypeId;
    let mut bp = if TypeId::of::<T>() == TypeId::of::<f64>() {
        let mut v: Vec<T> = Vec::with_capacity(n);
        unsafe {
            v.set_len(n);
        }
        v
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let mut v: Vec<T> = Vec::with_capacity(n);
        unsafe {
            v.set_len(n);
        }
        v
    } else {
        vec![T::zero(); n]
    };

    let mut tr = if TypeId::of::<T>() == TypeId::of::<f64>() {
        let mut v: Vec<T> = Vec::with_capacity(n);
        unsafe {
            v.set_len(n);
        }
        v
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let mut v: Vec<T> = Vec::with_capacity(n);
        unsafe {
            v.set_len(n);
        }
        v
    } else {
        vec![T::zero(); n]
    };

    // First bar has no prior close, so BP and TR are 0
    bp[0] = T::zero();
    tr[0] = T::zero();

    for i in 1..n {
        let prior_close = close[i - 1];

        // Branchless min/max using SeriesElement trait methods
        let true_low = low[i].min(prior_close);
        let true_high = high[i].max(prior_close);

        bp[i] = close[i] - true_low;
        tr[i] = true_high - true_low;
    }

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Use rolling sums instead of recalculating from scratch
    // This changes O(n×k) to O(n)

    // Initialize sums for the first window of each period
    let mut sum_bp1 = T::zero();
    let mut sum_tr1 = T::zero();
    let mut sum_bp2 = T::zero();
    let mut sum_tr2 = T::zero();
    let mut sum_bp3 = T::zero();
    let mut sum_tr3 = T::zero();

    // Build initial sums up to lookback
    for j in 1..=lookback {
        // Period 1 (shortest)
        if j > lookback - period1 {
            sum_bp1 = sum_bp1 + bp[j];
            sum_tr1 = sum_tr1 + tr[j];
        }
        // Period 2 (medium)
        if j > lookback - period2 {
            sum_bp2 = sum_bp2 + bp[j];
            sum_tr2 = sum_tr2 + tr[j];
        }
        // Period 3 (longest)
        if j > lookback - period3 {
            sum_bp3 = sum_bp3 + bp[j];
            sum_tr3 = sum_tr3 + tr[j];
        }
    }

    // Calculate ULTOSC for each bar after lookback using rolling sums
    for i in lookback..n {
        // Calculate averages
        let avg1 = if sum_tr1 == T::zero() {
            T::zero()
        } else {
            sum_bp1 / sum_tr1
        };
        let avg2 = if sum_tr2 == T::zero() {
            T::zero()
        } else {
            sum_bp2 / sum_tr2
        };
        let avg3 = if sum_tr3 == T::zero() {
            T::zero()
        } else {
            sum_bp3 / sum_tr3
        };

        // ULTOSC = 100 * ((4 * avg1) + (2 * avg2) + avg3) / 7
        // Optimized: precomputed hundred/seven = ~14.285714 to eliminate division
        output[i] = hundred_seventh * ((four * avg1) + (two * avg2) + avg3);

        // Update rolling sums for next iteration (if not last)
        if i + 1 < n {
            // Add new value, remove old value
            sum_bp1 = sum_bp1 + bp[i + 1] - bp[i + 1 - period1];
            sum_tr1 = sum_tr1 + tr[i + 1] - tr[i + 1 - period1];

            sum_bp2 = sum_bp2 + bp[i + 1] - bp[i + 1 - period2];
            sum_tr2 = sum_tr2 + tr[i + 1] - tr[i + 1 - period2];

            sum_bp3 = sum_bp3 + bp[i + 1] - bp[i + 1 - period3];
            sum_tr3 = sum_tr3 + tr[i + 1] - tr[i + 1 - period3];
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
/// use liq_ta::indicators::ultosc;
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
    use std::any::TypeId;
    let n = high.len();

    // Optimization: For f64/f32, allocate uninitialized memory since ultosc_into
    // fully overwrites every element (lookback + computed region).
    // This avoids the initialization penalty for large datasets.
    let mut output = if TypeId::of::<T>() == TypeId::of::<f64>() {
        let mut v: Vec<T> = Vec::with_capacity(n);
        unsafe {
            v.set_len(n);
        } // Safe: ultosc_into writes all n elements
        v
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let mut v: Vec<T> = Vec::with_capacity(n);
        unsafe {
            v.set_len(n);
        } // Safe: ultosc_into writes all n elements
        v
    } else {
        vec![T::nan(); n]
    };

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

    #[test]
    fn test_ultosc_validation_matrix_and_nonfinite_surface() {
        let high = vec![10.0_f64; 40];
        let low = vec![9.0_f64; 40];
        let close = vec![9.5_f64; 40];

        assert!(ultosc(&high, &low, &close, 7, 0, 28).is_err());
        assert!(ultosc(&high, &low, &close, 7, 14, 0).is_err());
        assert!(ultosc(&high, &low[..39], &close, 7, 14, 28).is_err());
        assert!(ultosc(&high[..20], &low[..20], &close[..20], 7, 14, 28).is_err());

        let mut out = vec![f64::NAN; 39];
        assert!(ultosc_into(&high, &low, &close, 7, 14, 28, &mut out).is_err());

        let mut high_nf = high.clone();
        let mut low_nf = low.clone();
        let mut close_nf = close.clone();
        high_nf[8] = f64::NAN;
        low_nf[13] = f64::INFINITY;
        close_nf[21] = f64::NEG_INFINITY;
        let mut out_nf = vec![f64::NAN; high_nf.len()];
        let _ = ultosc_into(&high_nf, &low_nf, &close_nf, 7, 14, 28, &mut out_nf);
    }
}
