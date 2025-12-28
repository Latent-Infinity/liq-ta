//! TRIX indicator.
//!
//! TRIX is a momentum oscillator that displays the percent rate of change
//! of a triple exponentially smoothed moving average.
//!
//! # Formula
//!
//! ```text
//! EMA1 = EMA(price, period)
//! EMA2 = EMA(EMA1, period)
//! EMA3 = EMA(EMA2, period)
//! TRIX = 100 * (EMA3[i] - EMA3[i-1]) / EMA3[i-1]
//! ```
//!
//! # Interpretation
//!
//! - Positive TRIX: Upward momentum
//! - Negative TRIX: Downward momentum
//! - Zero-line crossovers signal trend changes
//! - Can be used with signal line for trade signals
//!
//! # Lookback
//!
//! The lookback period is `3 * (period - 1) + 1`.

use crate::error::{Error, Result};
use crate::indicators::ema::ema_lookback;
use crate::traits::SeriesElement;

/// Computes the lookback period for TRIX.
#[inline]
#[must_use]
pub const fn trix_lookback(period: usize) -> usize {
    // 3 EMAs + 1 for ROC
    3 * ema_lookback(period) + 1
}

/// Returns the minimum input length required for TRIX calculation.
#[inline]
#[must_use]
pub const fn trix_min_len(period: usize) -> usize {
    trix_lookback(period) + 1
}

/// Computes TRIX and stores results in output slice.
///
/// # Arguments
///
/// * `data` - Price data (typically closing prices)
/// * `period` - EMA period (typically 15)
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn trix_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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

    let min_len = trix_min_len(period);
    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "trix",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "trix",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = trix_lookback(period);
    let hundred = T::from_f64(100.0)?;
    let ema_lb = ema_lookback(period);
    let alpha = T::from_f64(2.0)? / (T::from_usize(period)? + T::from_f64(1.0)?);
    let one_minus_alpha = T::from_f64(1.0)? - alpha;
    let period_t = T::from_usize(period)?;

    // Calculate first EMA (of data)
    // Use IEEE 754 NaN propagation: sum + NaN = NaN, NaN * alpha = NaN
    let mut ema1 = vec![T::nan(); n];
    // First valid EMA1 is at index ema_lb, using SMA of first `period` values
    let mut sum = T::zero();
    for i in 0..period {
        sum = sum + data[i]; // NaN propagates naturally
    }
    ema1[ema_lb] = sum / period_t;
    // EMA propagation: if prev is NaN or data[i] is NaN, result is NaN
    for i in (ema_lb + 1)..n {
        ema1[i] = alpha * data[i] + one_minus_alpha * ema1[i - 1];
    }

    // Calculate second EMA (of EMA1)
    let ema2_start = 2 * ema_lb;
    let mut ema2 = vec![T::nan(); n];
    // First valid EMA2 at index 2*ema_lb
    let mut sum2 = T::zero();
    for i in 0..period {
        sum2 = sum2 + ema1[ema_lb + i]; // NaN propagates naturally
    }
    ema2[ema2_start] = sum2 / period_t;
    // EMA propagation
    for i in (ema2_start + 1)..n {
        ema2[i] = alpha * ema1[i] + one_minus_alpha * ema2[i - 1];
    }

    // Calculate third EMA (of EMA2)
    let ema3_start = 3 * ema_lb;
    let mut ema3 = vec![T::nan(); n];
    // First valid EMA3 at index 3*ema_lb
    let mut sum3 = T::zero();
    for i in 0..period {
        sum3 = sum3 + ema2[ema2_start + i]; // NaN propagates naturally
    }
    ema3[ema3_start] = sum3 / period_t;
    // EMA propagation
    for i in (ema3_start + 1)..n {
        ema3[i] = alpha * ema2[i] + one_minus_alpha * ema3[i - 1];
    }

    // Fill lookback period with NaN using efficient slice.fill()
    output[..lookback].fill(T::nan());

    // Calculate TRIX (ROC of triple EMA)
    // First valid TRIX at lookback = 3*ema_lb + 1
    // Use IEEE 754 propagation for NaN, handle zero-division for rate of change
    for i in lookback..n {
        let prev = ema3[i - 1];
        let curr = ema3[i];
        // For ROC: (curr - prev) / prev
        // If prev is NaN, curr is NaN, or prev is zero, handle appropriately
        if !prev.is_finite() || !curr.is_finite() {
            output[i] = T::nan();
        } else if prev != T::zero() {
            output[i] = hundred * (curr - prev) / prev;
        } else {
            output[i] = T::zero();
        }
    }

    Ok(())
}

/// Computes TRIX indicator.
///
/// # Arguments
///
/// * `data` - Price data (typically closing prices)
/// * `period` - EMA period (typically 15)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - TRIX values (percentage)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::trix;
///
/// let prices = vec![44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0,
///                   45.5, 46.0, 46.5, 47.0, 46.5, 46.0, 45.5, 45.0, 44.5, 45.0];
///
/// let result = trix(&prices, 5).unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn trix<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::zero(); data.len()];
    trix_into(data, period, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_trix_lookback() {
        // 3 * (period - 1) + 1
        // period = 5: 3 * 4 + 1 = 13
        assert_eq!(trix_lookback(5), 13);
        // period = 15: 3 * 14 + 1 = 43
        assert_eq!(trix_lookback(15), 43);
    }

    #[test]
    fn test_trix_min_len() {
        assert_eq!(trix_min_len(5), 14);
        assert_eq!(trix_min_len(15), 44);
    }

    #[test]
    fn test_trix_empty_input() {
        let data: Vec<f64> = vec![];
        let result = trix(&data, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_trix_invalid_period() {
        let data: Vec<f64> = vec![10.0; 20];
        let result = trix(&data, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_trix_insufficient_data() {
        let data: Vec<f64> = vec![10.0; 10];
        let result = trix(&data, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_trix_output_length() {
        let data: Vec<f64> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];
        let result = trix(&data, 5).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_trix_lookback_nan() {
        let data: Vec<f64> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];
        let result = trix(&data, 5).unwrap();

        let lookback = trix_lookback(5);
        // Values up to lookback should be NaN
        for i in 0..lookback {
            assert!(result[i].is_nan(), "trix[{}] should be NaN", i);
        }

        // Values after lookback should be finite
        for i in lookback..result.len() {
            assert!(result[i].is_finite(), "trix[{}] should be finite", i);
        }
    }

    #[test]
    fn test_trix_into() {
        let data: Vec<f64> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];
        let mut output = vec![0.0_f64; 20];

        trix_into(&data, 5, &mut output).unwrap();

        let lookback = trix_lookback(5);
        assert!(output[lookback].is_finite());
    }

    #[test]
    fn test_trix_into_buffer_too_small() {
        let data: Vec<f64> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];
        let mut output = vec![0.0_f64; 10]; // Too small

        let result = trix_into(&data, 5, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_trix_f32() {
        let data: Vec<f32> = vec![
            44.0, 44.5, 43.5, 44.5, 44.0, 43.0, 42.5, 43.5, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0,
            46.5, 46.0, 45.5, 45.0, 44.5, 45.0,
        ];
        let result = trix(&data, 5).unwrap();

        let lookback = trix_lookback(5);
        assert!(result[lookback].is_finite());
    }

    #[test]
    fn test_trix_increasing_prices() {
        // Monotonically increasing prices should give positive TRIX
        let data: Vec<f64> = (0..25).map(|i| 100.0 + i as f64).collect();
        let result = trix(&data, 5).unwrap();

        let lookback = trix_lookback(5);
        for i in lookback..result.len() {
            assert!(
                result[i] > 0.0,
                "trix[{}] = {} should be positive for increasing prices",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_trix_decreasing_prices() {
        // Monotonically decreasing prices should give negative TRIX
        let data: Vec<f64> = (0..25).map(|i| 100.0 - i as f64 * 0.5).collect();
        let result = trix(&data, 5).unwrap();

        let lookback = trix_lookback(5);
        for i in lookback..result.len() {
            assert!(
                result[i] < 0.0,
                "trix[{}] = {} should be negative for decreasing prices",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_trix_constant_prices() {
        // Constant prices should give TRIX = 0
        let data: Vec<f64> = vec![10.0; 25];
        let result = trix(&data, 5).unwrap();

        let lookback = trix_lookback(5);
        for i in lookback..result.len() {
            assert!(
                (result[i] - 0.0).abs() < 1e-10,
                "trix[{}] should be 0 for constant prices",
                i
            );
        }
    }
}
