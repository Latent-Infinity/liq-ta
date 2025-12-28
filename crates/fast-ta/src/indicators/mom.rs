//! MOM (Momentum) indicator.
//!
//! Momentum measures the rate of change in price by comparing the current price
//! to the price N periods ago.
//!
//! # Formula
//!
//! ```text
//! MOM = price - price[n periods ago]
//! ```
//!
//! # Lookback
//!
//! The lookback period is equal to the period parameter (e.g., period 10 means
//! first 10 values are NaN).
//!
//! # Performance
//!
//! This implementation uses SIMD vectorization for f64 data, achieving
//! performance competitive with TA-Lib. Uses IEEE 754 natural NaN propagation
//! for clean, zero-overhead NaN handling in the subtraction operation.

use std::any::TypeId;

use crate::error::{Error, Result};
use crate::kernels::simd::lagged_sub_sanitize_f64;
use crate::traits::SeriesElement;

/// Computes the lookback period for MOM.
///
/// Lookback = period
#[inline]
#[must_use]
pub const fn mom_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for MOM calculation.
#[inline]
#[must_use]
pub const fn mom_min_len(period: usize) -> usize {
    period + 1
}

/// Computes Momentum and stores results in output.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - Lookback period (number of bars to look back)
/// * `output` - Pre-allocated output slice
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(Error)` if inputs are invalid
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn mom_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    let n = data.len();
    let min_len = mom_min_len(period);

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "mom",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "mom",
            required: n,
            actual: output.len(),
        });
    }

    // Fill lookback period with NaN
    let lookback = mom_lookback(period);
    output[..lookback].fill(T::nan());

    // Calculate MOM: current price - price N periods ago
    //
    // PERFORMANCE CRITICAL: We use SIMD without pre-scan for maximum performance.
    // IEEE 754 naturally propagates NaN through subtraction (NaN - x = NaN).
    // Infinity → NaN conversion is handled inside mom_compute_fast for f64.
    mom_compute_fast(data, period, lookback, output);

    Ok(())
}

/// SIMD-optimized computation with type-specialized fast paths.
/// Uses IEEE 754 natural NaN propagation - no pre-scan or per-element checks needed.
#[inline]
fn mom_compute_fast<T: SeriesElement + 'static>(data: &[T], period: usize, lookback: usize, output: &mut [T]) {
    let n = data.len();

    // Use SIMD for f64
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        // SAFETY: We've verified T is f64, and the slices have compatible layouts
        let data_f64: &[f64] = unsafe {
            std::slice::from_raw_parts(data.as_ptr() as *const f64, data.len())
        };
        let output_slice = &mut output[lookback..n];
        let output_f64: &mut [f64] = unsafe {
            std::slice::from_raw_parts_mut(output_slice.as_mut_ptr() as *mut f64, output_slice.len())
        };

        let current = &data_f64[period..];
        let lagged = &data_f64[..n - period];

        // Use sanitized variant that converts infinity→NaN inline
        lagged_sub_sanitize_f64(current, lagged, output_f64);
    } else {
        // Scalar path for other types (f32, etc.)
        let current = &data[period..];
        let lagged = &data[..n - period];
        let out = &mut output[lookback..n];

        for i in 0..out.len() {
            out[i] = current[i] - lagged[i];
        }
    }
}

/// Computes Momentum indicator.
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - Lookback period (number of bars to look back)
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of momentum values
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::mom;
///
/// let prices = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
/// let result = mom(&prices, 3).unwrap();
/// // MOM[3] = 13.0 - 10.0 = 3.0
/// // MOM[4] = 14.0 - 11.0 = 3.0
/// // MOM[5] = 15.0 - 12.0 = 3.0
/// assert!(result[3] == 3.0);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn mom<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    mom_into(data, period, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    #[test]
    fn test_mom_lookback() {
        assert_eq!(mom_lookback(1), 1);
        assert_eq!(mom_lookback(10), 10);
        assert_eq!(mom_lookback(14), 14);
    }

    #[test]
    fn test_mom_min_len() {
        assert_eq!(mom_min_len(1), 2);
        assert_eq!(mom_min_len(10), 11);
        assert_eq!(mom_min_len(14), 15);
    }

    #[test]
    fn test_mom_empty_input() {
        let data: Vec<f64> = vec![];
        let result = mom(&data, 10);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_mom_invalid_period() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = mom(&data, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_mom_insufficient_data() {
        let data = vec![1.0, 2.0, 3.0];
        let result = mom(&data, 10);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_mom_output_length() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let result = mom(&data, 10).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_mom_nan_count() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let period = 10;
        let result = mom(&data, period).unwrap();

        let lookback = mom_lookback(period);
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, lookback);
    }

    #[test]
    fn test_mom_basic_calculation() {
        let prices: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let result = mom(&prices, 3).unwrap();

        // First 3 values should be NaN
        for i in 0..3 {
            assert!(result[i].is_nan(), "result[{}] should be NaN", i);
        }

        // MOM[3] = 13.0 - 10.0 = 3.0
        assert!((result[3] - 3.0).abs() < 1e-10);
        // MOM[4] = 14.0 - 11.0 = 3.0
        assert!((result[4] - 3.0).abs() < 1e-10);
        // MOM[5] = 15.0 - 12.0 = 3.0
        assert!((result[5] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_mom_constant_price() {
        let prices: Vec<f64> = vec![50.0; 10];
        let result = mom(&prices, 3).unwrap();

        // Momentum should be 0 for constant price
        for i in 3..result.len() {
            assert!((result[i] - 0.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_mom_decreasing_prices() {
        let prices: Vec<f64> = vec![20.0, 19.0, 18.0, 17.0, 16.0, 15.0];
        let result = mom(&prices, 3).unwrap();

        // MOM[3] = 17.0 - 20.0 = -3.0
        assert!((result[3] - (-3.0)).abs() < 1e-10);
        // MOM[4] = 16.0 - 19.0 = -3.0
        assert!((result[4] - (-3.0)).abs() < 1e-10);
    }

    #[test]
    fn test_mom_period_1() {
        let prices: Vec<f64> = vec![10.0, 12.0, 11.0, 14.0, 13.0];
        let result = mom(&prices, 1).unwrap();

        // MOM with period 1 is just price difference
        assert!(result[0].is_nan());
        assert!((result[1] - 2.0).abs() < 1e-10); // 12 - 10
        assert!((result[2] - (-1.0)).abs() < 1e-10); // 11 - 12
        assert!((result[3] - 3.0).abs() < 1e-10); // 14 - 11
        assert!((result[4] - (-1.0)).abs() < 1e-10); // 13 - 14
    }

    #[test]
    fn test_mom_into() {
        let data: Vec<f64> = (1..=10).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; data.len()];
        mom_into(&data, 5, &mut output).unwrap();

        let lookback = mom_lookback(5);
        for i in 0..lookback {
            assert!(output[i].is_nan());
        }
        for i in lookback..output.len() {
            assert!(output[i].is_finite());
            // MOM should be 5 for linear sequence with period 5
            assert!((output[i] - 5.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_mom_into_buffer_too_small() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 10]; // Too small
        let result = mom_into(&data, 5, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_mom_f32() {
        let prices: Vec<f32> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let result = mom(&prices, 3).unwrap();

        assert_eq!(result.len(), prices.len());
        assert!((result[3] - 3.0_f32).abs() < 1e-6);
    }

    #[test]
    fn test_mom_minimum_length() {
        let data: Vec<f64> = vec![10.0, 15.0];
        let result = mom(&data, 1).unwrap();

        assert_eq!(result.len(), 2);
        assert!(result[0].is_nan());
        assert!((result[1] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_mom_large_period() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = mom(&data, 50).unwrap();

        // First 50 values should be NaN
        for i in 0..50 {
            assert!(result[i].is_nan());
        }
        // After that, MOM should be 50 (since data is linear)
        for i in 50..result.len() {
            assert!((result[i] - 50.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_mom_nan_in_data() {
        let prices = vec![10.0, f64::NAN, 12.0, 13.0, 14.0, 15.0];
        let result = mom(&prices, 3).unwrap();

        // MOM[3] = 13.0 - 10.0 = 3.0 (no NaN involved)
        assert!((result[3] - 3.0).abs() < 1e-10);
        // MOM[4] = 14.0 - NaN = NaN
        assert!(result[4].is_nan());
        // MOM[5] = 15.0 - 12.0 = 3.0
        assert!((result[5] - 3.0).abs() < 1e-10);
    }
}
