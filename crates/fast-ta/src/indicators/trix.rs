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
/// This implementation fuses the triple EMA and ROC calculations into a single
/// pass, eliminating the need for intermediate arrays.
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
    let ema_lb = ema_lookback(period);

    // Pre-compute constants
    let alpha = T::from_f64(2.0 / (period as f64 + 1.0))?;
    let one_minus_alpha = T::one() - alpha;
    let hundred = T::from_f64(100.0)?;
    let period_t = T::from_usize(period)?;

    // Fill lookback period with NaN
    output[..lookback].fill(T::nan());

    // Phase 1: Build initial SMA for EMA1 (indices 0..period)
    let mut ema1_sum = T::zero();
    for i in 0..period {
        ema1_sum = ema1_sum + data[i];
    }
    let mut ema1 = ema1_sum / period_t;

    // Phase 2: Continue EMA1 and build initial SMA for EMA2
    // EMA1 is now valid at index ema_lb (= period - 1)
    // We need `period` EMA1 values for EMA2's SMA: indices ema_lb through 2*ema_lb
    let mut ema2_sum = ema1;
    for i in (ema_lb + 1)..=(2 * ema_lb) {
        ema1 = alpha * data[i] + one_minus_alpha * ema1;
        ema2_sum = ema2_sum + ema1;
    }
    let mut ema2 = ema2_sum / period_t;

    // Phase 3: Continue EMA1, EMA2 and build initial SMA for EMA3
    // EMA2 is now valid at index 2*ema_lb
    // We need `period` EMA2 values for EMA3's SMA: indices 2*ema_lb through 3*ema_lb
    let mut ema3_sum = ema2;
    for i in (2 * ema_lb + 1)..=(3 * ema_lb) {
        ema1 = alpha * data[i] + one_minus_alpha * ema1;
        ema2 = alpha * ema1 + one_minus_alpha * ema2;
        ema3_sum = ema3_sum + ema2;
    }
    let mut ema3 = ema3_sum / period_t;

    // Phase 4: Calculate TRIX (ROC of triple EMA)
    // First valid TRIX is at index lookback = 3*ema_lb + 1
    // We need prev_ema3 (at index 3*ema_lb) for the ROC calculation
    let mut prev_ema3 = ema3;

    // Process first TRIX value at lookback
    ema1 = alpha * data[lookback] + one_minus_alpha * ema1;
    ema2 = alpha * ema1 + one_minus_alpha * ema2;
    ema3 = alpha * ema2 + one_minus_alpha * ema3;

    // For ROC: (curr - prev) / prev
    // Handle NaN propagation and zero-division appropriately
    if !prev_ema3.is_finite() || !ema3.is_finite() {
        output[lookback] = T::nan();
    } else if prev_ema3 != T::zero() {
        output[lookback] = hundred * (ema3 - prev_ema3) / prev_ema3;
    } else {
        output[lookback] = T::zero();
    }
    prev_ema3 = ema3;

    // Continue for rest of data
    for i in (lookback + 1)..n {
        ema1 = alpha * data[i] + one_minus_alpha * ema1;
        ema2 = alpha * ema1 + one_minus_alpha * ema2;
        ema3 = alpha * ema2 + one_minus_alpha * ema3;

        if !prev_ema3.is_finite() || !ema3.is_finite() {
            output[i] = T::nan();
        } else if prev_ema3 != T::zero() {
            output[i] = hundred * (ema3 - prev_ema3) / prev_ema3;
        } else {
            output[i] = T::zero();
        }
        prev_ema3 = ema3;
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