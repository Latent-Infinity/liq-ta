//! Chaikin Accumulation/Distribution Line (AD)
//!
//! The Accumulation/Distribution Line is a cumulative volume-based indicator designed
//! to measure the cumulative flow of money into and out of a security.
//!
//! # Formula
//!
//! ```text
//! CLV (Close Location Value) = ((close - low) - (high - close)) / (high - low)
//!                            = (2 * close - high - low) / (high - low)
//! Money Flow Volume = CLV × volume
//! AD = cumulative sum of Money Flow Volume
//! ```
//!
//! # CLV Range
//!
//! CLV ranges from -1 to +1:
//! - +1: Close at high (maximum buying pressure)
//! - -1: Close at low (maximum selling pressure)
//! - 0: Close at midpoint of range
//!
//! # Edge Cases
//!
//! - When `high == low`, CLV is 0 (no range to compute)
//! - NaN/Inf in any input propagates NaN to all subsequent outputs (cumulative)
//!
//! # Lookback
//!
//! No lookback period (calculated per bar, cumulative sum).
//!
//! # Precision Behavior
//!
//! When `PrecisionMode::High` is active and input type is `f32`:
//! - Cumulative AD sum maintained in `f64`
//! - Money Flow calculations performed in `f64`
//! - Prevents precision loss with large volume accumulations
//!
//! **Tolerance**: hybrid(rel=1e-4, abs=1.0) when comparing f32 High mode to f64 reference.
//! AD is cumulative and can grow very large.
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::ad;
//!
//! let high = [25.0_f64, 26.0, 25.5, 26.5, 27.0];
//! let low = [24.0_f64, 24.5, 24.0, 25.0, 25.5];
//! let close = [24.5_f64, 25.5, 24.5, 26.0, 26.5];
//! let volume = [1000.0_f64, 1500.0, 1200.0, 1800.0, 2000.0];
//!
//! let result = ad(&high, &low, &close, &volume).unwrap();
//! assert_eq!(result.len(), 5);
//! ```

use crate::error::{Error, Result};
use crate::precision::{current_precision_mode, PrecisionMode};
use crate::traits::SeriesElement;

/// Returns true if we should use f64 precision for the given type.
#[inline]
fn use_f64_precision<T: 'static>() -> bool {
    use std::any::TypeId;
    TypeId::of::<T>() == TypeId::of::<f32>() && current_precision_mode() == PrecisionMode::High
}

/// Returns the lookback period for AD.
///
/// AD has no lookback - the first output is valid.
#[inline]
#[must_use]
pub const fn ad_lookback() -> usize {
    0
}

/// Returns the minimum data length required for AD.
#[inline]
#[must_use]
pub const fn ad_min_len() -> usize {
    1
}

/// Computes AD (Chaikin A/D Line) into a pre-allocated output buffer.
///
/// AD = cumulative sum of (CLV × volume)
///
/// Uses IEEE 754 NaN propagation: instead of checking each input for validity,
/// we compute the result and check once. NaN arithmetic naturally propagates.
/// Since AD is cumulative, once a NaN enters the sum, all subsequent values are NaN.
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `output` - Pre-allocated output buffer
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn ad_into<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    // Validate all arrays have same length
    if low.len() != n || close.len() != n || volume.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "HLCV arrays must have same length: high={}, low={}, close={}, volume={}",
                n,
                low.len(),
                close.len(),
                volume.len()
            ),
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "ad",
            required: n,
            actual: output.len(),
        });
    }

    if use_f64_precision::<T>() {
        ad_core_f64(high, low, close, volume, output)
    } else {
        ad_core_native(high, low, close, volume, output)
    }
}

/// Core AD computation using native precision.
fn ad_core_native<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    output: &mut [T],
) -> Result<()> {
    let mut ad_value = T::zero();

    // Calculate AD using IEEE 754 NaN propagation
    for i in 0..high.len() {
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let v = volume[i];

        // CLV = ((close - low) - (high - close)) / (high - low)
        //     = (2 * close - high - low) / (high - low)
        //     = (close + close - high - low) / range
        let range = h - l;
        let numerator = c + c - h - l; // Avoids T::from_f64(2.0)

        let clv = if range == T::zero() {
            // When high == low (valid finite values), CLV = 0
            // If any input was NaN/Inf, numerator will be non-finite
            if numerator.is_finite() {
                T::zero()
            } else {
                T::nan()
            }
        } else {
            // Normal case: compute CLV
            let result = numerator / range;
            // Normalize non-finite (NaN or Inf) to NaN
            if result.is_finite() {
                result
            } else {
                T::nan()
            }
        };

        // Money Flow Volume = CLV × volume
        // IEEE propagates NaN from either clv or volume
        let mfv = clv * v;

        // AD is cumulative - IEEE propagates NaN through addition
        ad_value = ad_value + mfv;

        // Normalize any non-finite result to NaN (handles Inf edge cases)
        if !ad_value.is_finite() {
            ad_value = T::nan();
        }

        output[i] = ad_value;
    }

    Ok(())
}

/// Core AD computation using f64 precision for f32 inputs.
fn ad_core_f64<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    output: &mut [T],
) -> Result<()> {
    let mut ad_accum: f64 = 0.0;

    // Calculate AD using IEEE 754 NaN propagation with f64 accumulation
    for i in 0..high.len() {
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let v = volume[i];

        // Convert to f64 for precision
        let h_f64 = h.to_f64().unwrap_or(0.0);
        let l_f64 = l.to_f64().unwrap_or(0.0);
        let c_f64 = c.to_f64().unwrap_or(0.0);
        let v_f64 = v.to_f64().unwrap_or(0.0);

        // CLV = ((close - low) - (high - close)) / (high - low)
        let range = h_f64 - l_f64;
        let numerator = c_f64 + c_f64 - h_f64 - l_f64;

        let clv = if range == 0.0 {
            // When high == low (valid finite values), CLV = 0
            // If any input was NaN/Inf, numerator will be non-finite
            if numerator.is_finite() {
                0.0
            } else {
                f64::NAN
            }
        } else {
            // Normal case: compute CLV
            let result = numerator / range;
            // Normalize non-finite (NaN or Inf) to NaN
            if result.is_finite() {
                result
            } else {
                f64::NAN
            }
        };

        // Money Flow Volume = CLV × volume
        // IEEE propagates NaN from either clv or volume
        let mfv = clv * v_f64;

        // AD is cumulative - IEEE propagates NaN through addition
        ad_accum += mfv;

        // Normalize any non-finite result to NaN (handles Inf edge cases)
        if !ad_accum.is_finite() {
            ad_accum = f64::NAN;
        }

        output[i] = T::from_f64(ad_accum)?;
    }

    Ok(())
}

/// Computes AD (Chaikin A/D Line) and returns a newly allocated vector.
///
/// AD = cumulative sum of (CLV × volume)
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
///
/// # Returns
///
/// * `Ok(Vec<T>)` - Vector of AD values (cumulative money flow)
/// * `Err(Error)` if inputs are invalid
///
/// # Example
///
/// ```
/// use fast_ta::indicators::ad;
///
/// let high = [25.0_f64, 26.0, 25.5, 26.5, 27.0];
/// let low = [24.0_f64, 24.5, 24.0, 25.0, 25.5];
/// let close = [24.5_f64, 25.5, 24.5, 26.0, 26.5];
/// let volume = [1000.0_f64, 1500.0, 1200.0, 1800.0, 2000.0];
///
/// let result = ad(&high, &low, &close, &volume).unwrap();
/// assert_eq!(result.len(), 5);
/// // First bar with close at midpoint: CLV = 0, AD = 0
/// assert!((result[0] - 0.0).abs() < 1e-10);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
pub fn ad<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
) -> Result<Vec<T>> {
    let len = high.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }

    let mut output = vec![T::zero(); len];
    ad_into(high, low, close, volume, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        (a - b).abs() < tol
    }

    #[test]
    fn test_ad_lookback() {
        assert_eq!(ad_lookback(), 0);
    }

    #[test]
    fn test_ad_min_len() {
        assert_eq!(ad_min_len(), 1);
    }

    #[test]
    fn test_ad_empty_input() {
        let high: [f64; 0] = [];
        let low: [f64; 0] = [];
        let close: [f64; 0] = [];
        let volume: [f64; 0] = [];
        let result = ad(&high, &low, &close, &volume);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_ad_length_mismatch() {
        let high = [25.0_f64, 26.0];
        let low = [24.0_f64];
        let close = [24.5_f64, 25.5];
        let volume = [1000.0_f64, 1500.0];
        let result = ad(&high, &low, &close, &volume);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_ad_basic() {
        // Test case where close is at high (bullish)
        // MFM = (2*25 - 25 - 24) / (25 - 24) = (50 - 49) / 1 = 1
        // MFV = 1 * 1000 = 1000
        // AD = 1000
        let high = [25.0_f64];
        let low = [24.0_f64];
        let close = [25.0_f64]; // close at high
        let volume = [1000.0_f64];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert!(approx_eq(result[0], 1000.0, 1e-10));
    }

    #[test]
    fn test_ad_close_at_low() {
        // Test case where close is at low (bearish)
        // MFM = (2*24 - 25 - 24) / (25 - 24) = (48 - 49) / 1 = -1
        // MFV = -1 * 1000 = -1000
        // AD = -1000
        let high = [25.0_f64];
        let low = [24.0_f64];
        let close = [24.0_f64]; // close at low
        let volume = [1000.0_f64];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert!(approx_eq(result[0], -1000.0, 1e-10));
    }

    #[test]
    fn test_ad_close_at_midpoint() {
        // Test case where close is at midpoint
        // MFM = (2*24.5 - 25 - 24) / (25 - 24) = (49 - 49) / 1 = 0
        // MFV = 0 * 1000 = 0
        // AD = 0
        let high = [25.0_f64];
        let low = [24.0_f64];
        let close = [24.5_f64]; // close at midpoint
        let volume = [1000.0_f64];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert!(approx_eq(result[0], 0.0, 1e-10));
    }

    #[test]
    fn test_ad_high_equals_low() {
        // When high == low, MFM = 0, so AD doesn't change
        let high = [25.0_f64, 25.0];
        let low = [25.0_f64, 25.0];
        let close = [25.0_f64, 25.0];
        let volume = [1000.0_f64, 2000.0];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert!(approx_eq(result[0], 0.0, 1e-10));
        assert!(approx_eq(result[1], 0.0, 1e-10));
    }

    #[test]
    fn test_ad_cumulative() {
        // Test cumulative behavior
        // Bar 1: close at high -> MFM = 1, MFV = 1000, AD = 1000
        // Bar 2: close at low -> MFM = -1, MFV = -1500, AD = 1000 - 1500 = -500
        // Bar 3: close at high -> MFM = 1, MFV = 1200, AD = -500 + 1200 = 700
        let high = [25.0_f64, 26.0, 25.5];
        let low = [24.0_f64, 25.0, 24.5];
        let close = [25.0_f64, 25.0, 25.5]; // high, low, high
        let volume = [1000.0_f64, 1500.0, 1200.0];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert!(approx_eq(result[0], 1000.0, 1e-10));
        assert!(approx_eq(result[1], 1000.0 - 1500.0, 1e-10));
        assert!(approx_eq(result[2], -500.0 + 1200.0, 1e-10));
    }

    #[test]
    fn test_ad_into_buffer_too_small() {
        let high = [25.0_f64, 26.0];
        let low = [24.0_f64, 25.0];
        let close = [24.5_f64, 25.5];
        let volume = [1000.0_f64, 1500.0];
        let mut output = [0.0_f64; 1];

        let result = ad_into(&high, &low, &close, &volume, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_ad_into_success() {
        let high = [25.0_f64, 26.0];
        let low = [24.0_f64, 25.0];
        let close = [25.0_f64, 25.0]; // high, low
        let volume = [1000.0_f64, 1500.0];
        let mut output = [0.0_f64; 2];

        ad_into(&high, &low, &close, &volume, &mut output).unwrap();
        assert!(approx_eq(output[0], 1000.0, 1e-10));
        assert!(approx_eq(output[1], 1000.0 - 1500.0, 1e-10));
    }

    #[test]
    fn test_ad_f32() {
        let high = [25.0_f32];
        let low = [24.0_f32];
        let close = [25.0_f32];
        let volume = [1000.0_f32];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert!((result[0] - 1000.0).abs() < 1e-5);
    }

    #[test]
    fn test_ad_realistic_data() {
        // Realistic OHLCV data
        let high = [45.0_f64, 46.0, 46.5, 45.5, 46.0];
        let low = [44.0_f64, 44.5, 45.0, 44.0, 44.5];
        let close = [44.5_f64, 45.5, 46.0, 44.5, 45.5];
        let volume = [1000.0_f64, 1200.0, 800.0, 1500.0, 1100.0];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert_eq!(result.len(), 5);

        // First bar: MFM = (2*44.5 - 45 - 44) / (45 - 44) = (89 - 89) / 1 = 0
        assert!(approx_eq(result[0], 0.0, 1e-10));
    }

    #[test]
    fn test_ad_output_length() {
        let high = [25.0_f64; 100];
        let low = [24.0_f64; 100];
        let close = [24.5_f64; 100];
        let volume = [1000.0_f64; 100];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_ad_nan_in_high_propagates() {
        // NaN in high should propagate to output and all subsequent values
        let high = [25.0_f64, f64::NAN, 25.5];
        let low = [24.0_f64, 24.5, 24.5];
        let close = [25.0_f64, 25.0, 25.0];
        let volume = [1000.0_f64, 1500.0, 1200.0];

        let result = ad(&high, &low, &close, &volume).unwrap();
        // First value is valid
        assert!(result[0].is_finite());
        // NaN at index 1 propagates to all subsequent values
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }

    #[test]
    fn test_ad_nan_in_low_propagates() {
        // NaN in low should propagate to output and all subsequent values
        let high = [25.0_f64, 26.0, 25.5];
        let low = [24.0_f64, f64::NAN, 24.5];
        let close = [25.0_f64, 25.5, 25.0];
        let volume = [1000.0_f64, 1500.0, 1200.0];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert!(result[0].is_finite());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }

    #[test]
    fn test_ad_nan_in_close_propagates() {
        // NaN in close should propagate to output and all subsequent values
        let high = [25.0_f64, 26.0, 25.5];
        let low = [24.0_f64, 25.0, 24.5];
        let close = [25.0_f64, f64::NAN, 25.0];
        let volume = [1000.0_f64, 1500.0, 1200.0];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert!(result[0].is_finite());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }

    #[test]
    fn test_ad_nan_in_volume_propagates() {
        // NaN in volume should propagate to output and all subsequent values
        let high = [25.0_f64, 26.0, 25.5];
        let low = [24.0_f64, 25.0, 24.5];
        let close = [25.0_f64, 25.5, 25.0];
        let volume = [1000.0_f64, f64::NAN, 1200.0];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert!(result[0].is_finite());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }

    #[test]
    fn test_ad_nan_at_first_bar_propagates() {
        // NaN at first bar should propagate to all values
        let high = [f64::NAN, 26.0, 25.5];
        let low = [24.0_f64, 25.0, 24.5];
        let close = [25.0_f64, 25.5, 25.0];
        let volume = [1000.0_f64, 1500.0, 1200.0];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }

    #[test]
    fn test_ad_nan_with_zero_range() {
        // When high == low and an input is NaN, should propagate NaN
        let high = [25.0_f64, 26.0, 26.0];
        let low = [24.0_f64, 26.0, 26.0]; // zero range at index 1 and 2
        let close = [25.0_f64, f64::NAN, 26.0];
        let volume = [1000.0_f64, 1500.0, 1200.0];

        let result = ad(&high, &low, &close, &volume).unwrap();
        assert!(result[0].is_finite());
        assert!(result[1].is_nan()); // NaN in close with zero range
        assert!(result[2].is_nan()); // Propagated from previous NaN
    }

    #[test]
    fn test_ad_no_nan_for_valid_input() {
        // AD with valid input should never produce NaN
        let high = [25.0_f64, 26.0, 25.5];
        let low = [24.0_f64, 25.0, 24.5];
        let close = [25.0_f64, 25.5, 25.0];
        let volume = [1000.0_f64, 1500.0, 1200.0];

        let result = ad(&high, &low, &close, &volume).unwrap();
        for &val in &result {
            assert!(val.is_finite(), "Valid input should not produce NaN");
        }
    }
}