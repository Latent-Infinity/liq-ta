//! Volume Weighted Average Price (VWAP) indicator.
//!
//! VWAP is the ratio of the value traded to total volume traded over a particular
//! time horizon. It measures the average price a security has traded at throughout
//! the trading session, based on both volume and price.
//!
//! # Algorithm
//!
//! VWAP is calculated using cumulative values:
//!
//! ```text
//! Typical Price = (High + Low + Close) / 3
//!
//! VWAP[i] = cumsum(Typical_Price × Volume)[i] / cumsum(Volume)[i]
//! ```
//!
//! # Interpretation
//!
//! - VWAP acts as a benchmark for trade execution quality
//! - Price above VWAP suggests bullish sentiment
//! - Price below VWAP suggests bearish sentiment
//! - Institutional traders often use VWAP to minimize market impact
//!
//! # NaN Handling
//!
//! - If any input at an index is NaN, the output at that index is NaN.
//! - Because VWAP is cumulative, once NaN appears, subsequent values remain NaN.
//!
//! # Note
//!
//! This implementation provides continuous (anchored) VWAP without session resets.
//! For intraday VWAP with daily resets, users should segment their data by session.
//!
//! # Precision Behavior
//!
//! When `PrecisionMode::High` is active and input type is `f32`:
//! - Cumulative TP×Volume and Volume sums use `f64` accumulators
//! - Prevents overflow and precision loss over long sessions
//! - Division performed in `f64`, then converted to `f32`
//!
//! **Tolerance**: hybrid(rel=1e-5, abs=1e-7) when comparing f32 High mode to f64 reference.
//!
//! For maximum precision over full trading days (>5000 bars), use `f64` input directly.
//!
//! # Example
//!
//! ```
//! use liq_ta::indicators::vwap::vwap;
//!
//! let high = vec![10.5_f64, 11.0, 10.8, 11.2, 11.0];
//! let low = vec![10.0_f64, 10.3, 10.2, 10.5, 10.3];
//! let close = vec![10.2_f64, 10.8, 10.5, 11.0, 10.7];
//! let volume = vec![1000.0, 1500.0, 1200.0, 1800.0, 1100.0];
//!
//! let result = vwap(&high, &low, &close, &volume).unwrap();
//!
//! assert_eq!(result.len(), 5);
//! // All values are valid (no lookback)
//! for val in &result {
//!     assert!(!val.is_nan());
//! }
//! ```

use crate::error::{Error, Result};
use crate::precision::{PrecisionMode, current_precision_mode};
use crate::traits::SeriesElement;

/// Returns true if we should use f64 precision for the given type.
#[inline]
fn use_f64_precision<T: 'static>() -> bool {
    use std::any::TypeId;
    TypeId::of::<T>() == TypeId::of::<f32>() && current_precision_mode() == PrecisionMode::High
}

/// Returns the lookback period for VWAP.
///
/// VWAP has no lookback period since the first value is valid immediately.
///
/// # Example
///
/// ```
/// use liq_ta::indicators::vwap::vwap_lookback;
///
/// assert_eq!(vwap_lookback(), 0);
/// ```
#[inline]
#[must_use]
pub const fn vwap_lookback() -> usize {
    0
}

/// Returns the minimum input length required for VWAP.
///
/// At least 1 data point is required to compute VWAP.
///
/// # Example
///
/// ```
/// use liq_ta::indicators::vwap::vwap_min_len;
///
/// assert_eq!(vwap_min_len(), 1);
/// ```
#[inline]
#[must_use]
pub const fn vwap_min_len() -> usize {
    1
}

/// Computes Volume Weighted Average Price for OHLCV data.
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `close` - The closing prices series
/// * `volume` - The volume series
///
/// # Returns
///
/// A `Result` containing a vector of VWAP values.
/// All values are valid (no NaN lookback period).
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The series have different lengths (`Error::LengthMismatch`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for the output vector
///
/// # Example
///
/// ```
/// use liq_ta::indicators::vwap::vwap;
///
/// let high = vec![10.5_f64, 11.0, 10.8];
/// let low = vec![10.0, 10.3, 10.2];
/// let close = vec![10.2, 10.8, 10.5];
/// let volume = vec![1000.0, 1500.0, 1200.0];
///
/// let result = vwap(&high, &low, &close, &volume).unwrap();
///
/// // VWAP is influenced by volume-weighted typical price
/// assert!(!result[0].is_nan());
/// ```
#[must_use = "this returns a Result with VWAP values, which should be used"]
pub fn vwap<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
) -> Result<Vec<T>> {
    validate_inputs(high, low, close, volume)?;

    let n = high.len();
    let mut output = Vec::with_capacity(n);

    if use_f64_precision::<T>() {
        compute_vwap_core_f64(high, low, close, volume, &mut output)?;
    } else {
        compute_vwap_core(high, low, close, volume, &mut output);
    }

    Ok(output)
}

/// Computes VWAP into a pre-allocated output buffer.
///
/// This variant allows reusing existing buffers to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `close` - The closing prices series
/// * `volume` - The volume series
/// * `output` - Pre-allocated buffer for VWAP values
///
/// # Returns
///
/// A `Result` containing the number of valid values computed (always n for VWAP),
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The series have different lengths (`Error::LengthMismatch`)
/// - The output buffer is shorter than the input data
///
/// # Example
///
/// ```
/// use liq_ta::indicators::vwap::vwap_into;
///
/// let high = vec![10.5_f64, 11.0, 10.8];
/// let low = vec![10.0, 10.3, 10.2];
/// let close = vec![10.2, 10.8, 10.5];
/// let volume = vec![1000.0, 1500.0, 1200.0];
/// let mut output = vec![0.0_f64; 3];
///
/// let valid_count = vwap_into(&high, &low, &close, &volume, &mut output).unwrap();
/// assert_eq!(valid_count, 3);
/// ```
#[must_use = "this returns a Result with the count of valid VWAP values"]
pub fn vwap_into<T: SeriesElement + 'static>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    output: &mut [T],
) -> Result<usize> {
    validate_inputs(high, low, close, volume)?;

    let n = high.len();

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "vwap",
        });
    }

    if use_f64_precision::<T>() {
        compute_vwap_core_into_f64(high, low, close, volume, output)?;
    } else {
        compute_vwap_core_into(high, low, close, volume, output);
    }

    Ok(n)
}

/// Validates input data.
fn validate_inputs<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
) -> Result<()> {
    if high.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = high.len();

    if low.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", n, low.len()),
        });
    }

    if close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, close has {}", n, close.len()),
        });
    }

    if volume.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, volume has {}", n, volume.len()),
        });
    }

    Ok(())
}

/// Core VWAP computation that allocates a new vector.
fn compute_vwap_core<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    output: &mut Vec<T>,
) {
    let n = high.len();
    let three = T::two() + T::one();

    let mut cumulative_tp_vol = T::zero();
    let mut cumulative_vol = T::zero();
    let mut last_valid_vwap = T::nan();
    let mut nan_active = false;

    for i in 0..n {
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let v = volume[i];

        if nan_active {
            output.push(T::nan());
            continue;
        }
        if !(h.is_finite() && l.is_finite() && c.is_finite() && v.is_finite()) {
            nan_active = true;
            output.push(T::nan());
            continue;
        }

        // Skip zero volume (VWAP unchanged)
        if v == T::zero() {
            output.push(last_valid_vwap);
            continue;
        }

        // Typical price
        let tp = (h + l + c) / three;

        // Update cumulative values
        cumulative_tp_vol = cumulative_tp_vol + (tp * v);
        cumulative_vol = cumulative_vol + v;

        // Calculate VWAP
        let vwap_val = if cumulative_vol > T::zero() {
            cumulative_tp_vol / cumulative_vol
        } else {
            tp
        };

        last_valid_vwap = vwap_val;
        output.push(vwap_val);
    }
}

/// Core VWAP computation into pre-allocated buffer.
fn compute_vwap_core_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    output: &mut [T],
) {
    let n = high.len();
    let three = T::two() + T::one();

    let mut cumulative_tp_vol = T::zero();
    let mut cumulative_vol = T::zero();
    let mut last_valid_vwap = T::nan();
    let mut nan_active = false;

    for i in 0..n {
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let v = volume[i];

        if nan_active {
            output[i] = T::nan();
            continue;
        }
        if !(h.is_finite() && l.is_finite() && c.is_finite() && v.is_finite()) {
            nan_active = true;
            output[i] = T::nan();
            continue;
        }

        // Skip zero volume (VWAP unchanged)
        if v == T::zero() {
            output[i] = last_valid_vwap;
            continue;
        }

        // Typical price
        let tp = (h + l + c) / three;

        // Update cumulative values
        cumulative_tp_vol = cumulative_tp_vol + (tp * v);
        cumulative_vol = cumulative_vol + v;

        // Calculate VWAP
        let vwap_val = if cumulative_vol > T::zero() {
            cumulative_tp_vol / cumulative_vol
        } else {
            tp
        };

        last_valid_vwap = vwap_val;
        output[i] = vwap_val;
    }
}

/// Core VWAP computation with f64 accumulators that allocates a new vector.
fn compute_vwap_core_f64<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    output: &mut Vec<T>,
) -> Result<()> {
    let n = high.len();

    let mut cumulative_tp_vol: f64 = 0.0;
    let mut cumulative_vol: f64 = 0.0;
    let mut last_valid_vwap: f64 = f64::NAN;
    let mut nan_active = false;

    for i in 0..n {
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let v = volume[i];

        if nan_active {
            output.push(T::nan());
            continue;
        }
        if !(h.is_finite() && l.is_finite() && c.is_finite() && v.is_finite()) {
            nan_active = true;
            output.push(T::nan());
            continue;
        }

        // Convert to f64 for accumulation
        let h_f64 = h.to_f64().unwrap_or(0.0);
        let l_f64 = l.to_f64().unwrap_or(0.0);
        let c_f64 = c.to_f64().unwrap_or(0.0);
        let v_f64 = v.to_f64().unwrap_or(0.0);

        // Skip zero volume (VWAP unchanged)
        if v_f64 == 0.0 {
            output.push(T::from_f64(last_valid_vwap)?);
            continue;
        }

        // Typical price in f64
        let tp = (h_f64 + l_f64 + c_f64) / 3.0;

        // Update cumulative values in f64
        cumulative_tp_vol += tp * v_f64;
        cumulative_vol += v_f64;

        // Calculate VWAP in f64
        let vwap_val = if cumulative_vol > 0.0 {
            cumulative_tp_vol / cumulative_vol
        } else {
            tp
        };

        last_valid_vwap = vwap_val;
        output.push(T::from_f64(vwap_val)?);
    }

    Ok(())
}

/// Core VWAP computation with f64 accumulators into pre-allocated buffer.
fn compute_vwap_core_into_f64<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    volume: &[T],
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    let mut cumulative_tp_vol: f64 = 0.0;
    let mut cumulative_vol: f64 = 0.0;
    let mut last_valid_vwap: f64 = f64::NAN;
    let mut nan_active = false;

    for i in 0..n {
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let v = volume[i];

        if nan_active {
            output[i] = T::nan();
            continue;
        }
        if !(h.is_finite() && l.is_finite() && c.is_finite() && v.is_finite()) {
            nan_active = true;
            output[i] = T::nan();
            continue;
        }

        // Convert to f64 for accumulation
        let h_f64 = h.to_f64().unwrap_or(0.0);
        let l_f64 = l.to_f64().unwrap_or(0.0);
        let c_f64 = c.to_f64().unwrap_or(0.0);
        let v_f64 = v.to_f64().unwrap_or(0.0);

        // Skip zero volume (VWAP unchanged)
        if v_f64 == 0.0 {
            output[i] = T::from_f64(last_valid_vwap)?;
            continue;
        }

        // Typical price in f64
        let tp = (h_f64 + l_f64 + c_f64) / 3.0;

        // Update cumulative values in f64
        cumulative_tp_vol += tp * v_f64;
        cumulative_vol += v_f64;

        // Calculate VWAP in f64
        let vwap_val = if cumulative_vol > 0.0 {
            cumulative_tp_vol / cumulative_vol
        } else {
            tp
        };

        last_valid_vwap = vwap_val;
        output[i] = T::from_f64(vwap_val)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;
    use num_traits::Float;

    fn approx_eq<T: Float>(a: T, b: T, epsilon: T) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        if a.is_nan() || b.is_nan() {
            return false;
        }
        (a - b).abs() < epsilon
    }

    const EPSILON: f64 = 1e-10;

    // ==================== Lookback and Min Length Tests ====================

    #[test]
    fn test_vwap_lookback() {
        assert_eq!(vwap_lookback(), 0);
    }

    #[test]
    fn test_vwap_min_len() {
        assert_eq!(vwap_min_len(), 1);
    }

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_vwap_basic() {
        let high = vec![10.5_f64, 11.0, 10.8, 11.2, 11.0];
        let low = vec![10.0, 10.3, 10.2, 10.5, 10.3];
        let close = vec![10.2, 10.8, 10.5, 11.0, 10.7];
        let volume = vec![1000.0, 1500.0, 1200.0, 1800.0, 1100.0];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        assert_eq!(result.len(), 5);

        // All values should be valid (no NaN lookback)
        for val in &result {
            assert!(!val.is_nan(), "VWAP values should not be NaN");
        }
    }

    #[test]
    fn test_vwap_f32() {
        let high = vec![10.5_f32, 11.0, 10.8];
        let low = vec![10.0, 10.3, 10.2];
        let close = vec![10.2, 10.8, 10.5];
        let volume = vec![1000.0, 1500.0, 1200.0];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result[0].is_nan());
    }

    #[test]
    fn test_vwap_single_value() {
        let high = vec![10.5_f64];
        let low = vec![10.0];
        let close = vec![10.2];
        let volume = vec![1000.0];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        assert_eq!(result.len(), 1);
        // First VWAP = typical price = (10.5 + 10.0 + 10.2) / 3
        let expected_tp = (10.5 + 10.0 + 10.2) / 3.0;
        assert!(approx_eq(result[0], expected_tp, EPSILON));
    }

    // ==================== Known Value Tests ====================

    #[test]
    fn test_vwap_known_values() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];
        let volume = vec![100.0, 200.0, 100.0];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        // TP[0] = (10 + 9 + 9.5) / 3 = 9.5
        // TP[1] = (11 + 10 + 10.5) / 3 = 10.5
        // TP[2] = (12 + 11 + 11.5) / 3 = 11.5

        // VWAP[0] = (9.5 * 100) / 100 = 9.5
        let tp0 = (10.0 + 9.0 + 9.5) / 3.0;
        assert!(approx_eq(result[0], tp0, EPSILON));

        // VWAP[1] = (9.5 * 100 + 10.5 * 200) / (100 + 200)
        let tp1 = (11.0 + 10.0 + 10.5) / 3.0;
        let expected_vwap1 = (tp0 * 100.0 + tp1 * 200.0) / 300.0;
        assert!(approx_eq(result[1], expected_vwap1, EPSILON));

        // VWAP[2] = (9.5 * 100 + 10.5 * 200 + 11.5 * 100) / (100 + 200 + 100)
        let tp2 = (12.0 + 11.0 + 11.5) / 3.0;
        let expected_vwap2 = (tp0 * 100.0 + tp1 * 200.0 + tp2 * 100.0) / 400.0;
        assert!(approx_eq(result[2], expected_vwap2, EPSILON));
    }

    #[test]
    fn test_vwap_typical_price_formula() {
        // Verify typical price = (H + L + C) / 3
        let high = vec![15.0_f64];
        let low = vec![12.0];
        let close = vec![14.0];
        let volume = vec![100.0];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        // TP = (15 + 12 + 14) / 3 = 41 / 3 ≈ 13.6667
        let expected = (15.0 + 12.0 + 14.0) / 3.0;
        assert!(approx_eq(result[0], expected, EPSILON));
    }

    #[test]
    fn test_vwap_cumulative_behavior() {
        // Verify cumulative sum behavior
        let high = vec![10.0_f64, 10.0, 10.0, 10.0];
        let low = vec![10.0, 10.0, 10.0, 10.0];
        let close = vec![10.0, 10.0, 10.0, 10.0];
        let volume = vec![100.0, 100.0, 100.0, 100.0];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        // All same typical price (10), all same volume
        // VWAP should be 10 throughout
        for val in &result {
            assert!(approx_eq(*val, 10.0, EPSILON));
        }
    }

    #[test]
    fn test_vwap_volume_weighted() {
        // Higher volume should pull VWAP toward that price
        let high = vec![10.0_f64, 20.0];
        let low = vec![10.0, 20.0];
        let close = vec![10.0, 20.0];
        let volume = vec![100.0, 900.0]; // 9x more volume at 20

        let result = vwap(&high, &low, &close, &volume).unwrap();

        // VWAP[1] = (10 * 100 + 20 * 900) / 1000 = 19000 / 1000 = 19
        assert!(approx_eq(result[0], 10.0, EPSILON));
        assert!(approx_eq(result[1], 19.0, EPSILON)); // Closer to 20 due to higher volume
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_vwap_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];
        let volume: Vec<f64> = vec![];

        let result = vwap(&high, &low, &close, &volume);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_vwap_mismatched_lengths() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0];
        let close = vec![9.5, 10.5, 11.5];
        let volume = vec![100.0, 200.0, 100.0];

        let result = vwap(&high, &low, &close, &volume);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    // ==================== NaN Handling Tests ====================

    #[test]
    fn test_vwap_nan_in_high() {
        let high = vec![10.0_f64, f64::NAN, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];
        let volume = vec![100.0, 200.0, 100.0];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        assert!(!result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }

    #[test]
    fn test_vwap_nan_in_volume() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];
        let volume = vec![100.0, f64::NAN, 100.0];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        assert!(!result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }

    #[test]
    fn test_vwap_nan_first_bar() {
        let high = vec![f64::NAN, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];
        let volume = vec![100.0, 200.0, 100.0];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        // First bar NaN → result is NaN
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
    }

    // ==================== Zero Volume Tests ====================

    #[test]
    fn test_vwap_zero_volume() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];
        let volume = vec![100.0, 0.0, 100.0]; // Zero volume in middle

        let result = vwap(&high, &low, &close, &volume).unwrap();

        // Zero volume → VWAP unchanged
        assert!(!result[0].is_nan());
        assert!(approx_eq(result[1], result[0], EPSILON));
        assert!(!result[2].is_nan());
    }

    #[test]
    fn test_vwap_first_bar_zero_volume() {
        let high = vec![10.0_f64, 11.0];
        let low = vec![9.0, 10.0];
        let close = vec![9.5, 10.5];
        let volume = vec![0.0, 100.0]; // Zero volume on first bar

        let result = vwap(&high, &low, &close, &volume).unwrap();

        // First bar zero volume → NaN (no previous VWAP)
        assert!(result[0].is_nan());
        // Second bar has volume
        assert!(!result[1].is_nan());
    }

    // ==================== vwap_into Tests ====================

    #[test]
    fn test_vwap_into_basic() {
        let high = vec![10.5_f64, 11.0, 10.8];
        let low = vec![10.0, 10.3, 10.2];
        let close = vec![10.2, 10.8, 10.5];
        let volume = vec![1000.0, 1500.0, 1200.0];
        let mut output = vec![0.0_f64; 3];

        let valid_count = vwap_into(&high, &low, &close, &volume, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(!output[0].is_nan());
    }

    #[test]
    fn test_vwap_into_buffer_too_small() {
        let high = vec![10.0_f64; 10];
        let low = vec![9.0_f64; 10];
        let close = vec![9.5_f64; 10];
        let volume = vec![100.0_f64; 10];
        let mut output = vec![0.0_f64; 5]; // Too short

        let result = vwap_into(&high, &low, &close, &volume, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_vwap_and_vwap_into_produce_same_result() {
        let high = vec![10.5_f64, 11.0, 10.8, 11.2, 11.0, 11.5, 11.2, 11.8];
        let low = vec![10.0, 10.3, 10.2, 10.5, 10.3, 10.8, 10.5, 11.0];
        let close = vec![10.2, 10.8, 10.5, 11.0, 10.7, 11.2, 10.9, 11.5];
        let volume = vec![
            1000.0, 1500.0, 1200.0, 1800.0, 1100.0, 1600.0, 1300.0, 1700.0,
        ];

        let result1 = vwap(&high, &low, &close, &volume).unwrap();

        let mut output = vec![0.0_f64; 8];
        vwap_into(&high, &low, &close, &volume, &mut output).unwrap();

        for i in 0..8 {
            assert!(
                approx_eq(result1[i], output[i], EPSILON),
                "Mismatch at {}: {} vs {}",
                i,
                result1[i],
                output[i]
            );
        }
    }

    // ==================== Property-Based Tests ====================

    #[test]
    fn test_vwap_output_length_equals_input_length() {
        for len in [1, 10, 50, 100] {
            let high: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) + 1.0).collect();
            let low: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) - 1.0).collect();
            let close: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64)).collect();
            let volume: Vec<f64> = (0..len).map(|i| 1000.0 + (i as f64) * 10.0).collect();

            let result = vwap(&high, &low, &close, &volume).unwrap();
            assert_eq!(result.len(), len);
        }
    }

    #[test]
    fn test_vwap_no_nan_with_valid_input() {
        // With all valid inputs, VWAP should have no NaN
        let high: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64) + 1.0).collect();
        let low: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64) - 1.0).collect();
        let close: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64)).collect();
        let volume: Vec<f64> = (0..50).map(|_| 1000.0).collect();

        let result = vwap(&high, &low, &close, &volume).unwrap();

        for (i, val) in result.iter().enumerate() {
            assert!(!val.is_nan(), "VWAP should not have NaN at index {}", i);
        }
    }

    #[test]
    fn test_vwap_between_price_extremes() {
        // VWAP should always be between the lowest low and highest high
        let high: Vec<f64> = (0..30)
            .map(|i| 110.0 + 5.0 * ((i as f64) * 0.3).sin())
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();
        let close: Vec<f64> = high.iter().map(|h| h - 1.5).collect();
        let volume: Vec<f64> = (0..30).map(|_| 1000.0).collect();

        let result = vwap(&high, &low, &close, &volume).unwrap();

        let min_low = low.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_high = high.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        for (i, val) in result.iter().enumerate() {
            assert!(
                *val >= min_low - EPSILON,
                "VWAP at {} should be >= min low: {} >= {}",
                i,
                val,
                min_low
            );
            assert!(
                *val <= max_high + EPSILON,
                "VWAP at {} should be <= max high: {} <= {}",
                i,
                val,
                max_high
            );
        }
    }

    // ==================== Real-World Scenario Tests ====================

    #[test]
    fn test_vwap_price_above_is_bullish() {
        // When current price is above VWAP, sentiment is bullish
        let high = vec![100.0_f64, 102.0, 104.0, 106.0, 108.0];
        let low = vec![99.0, 101.0, 103.0, 105.0, 107.0];
        let close = vec![99.5, 101.5, 103.5, 105.5, 107.5];
        let volume = vec![1000.0; 5];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        // In an uptrend with equal volume, price should be above VWAP
        let typical_last = (108.0 + 107.0 + 107.5) / 3.0;
        assert!(
            typical_last > result[4],
            "Current typical price {} should be above VWAP {} in uptrend",
            typical_last,
            result[4]
        );
    }

    #[test]
    fn test_vwap_institutional_benchmark() {
        // VWAP is used as an execution benchmark
        // Early high-volume trades should anchor VWAP
        let high = vec![100.0_f64, 100.5, 101.0, 101.5, 102.0];
        let low = vec![99.0, 99.5, 100.0, 100.5, 101.0];
        let close = vec![99.5, 100.0, 100.5, 101.0, 101.5];
        // High volume early, lower volume later (typical trading pattern)
        let volume = vec![10000.0, 5000.0, 3000.0, 2000.0, 1000.0];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        // VWAP should be closer to early prices due to higher volume
        let early_typical = (100.0 + 99.0 + 99.5) / 3.0;
        let late_typical = (102.0 + 101.0 + 101.5) / 3.0;

        // Final VWAP should be closer to early typical than late typical
        let dist_to_early = (result[4] - early_typical).abs();
        let dist_to_late = (result[4] - late_typical).abs();
        assert!(
            dist_to_early < dist_to_late,
            "VWAP {} should be closer to early typical {} than late typical {}",
            result[4],
            early_typical,
            late_typical
        );
    }

    #[test]
    fn test_vwap_stable_with_constant_inputs() {
        // With constant price and volume, VWAP should equal typical price
        let high = vec![100.0_f64; 10];
        let low = vec![98.0_f64; 10];
        let close = vec![99.0_f64; 10];
        let volume = vec![1000.0; 10];

        let result = vwap(&high, &low, &close, &volume).unwrap();

        let expected_tp = (100.0 + 98.0 + 99.0) / 3.0;

        for (i, val) in result.iter().enumerate() {
            assert!(
                approx_eq(*val, expected_tp, EPSILON),
                "VWAP at {} should be {}, got {}",
                i,
                expected_tp,
                val
            );
        }
    }
}
