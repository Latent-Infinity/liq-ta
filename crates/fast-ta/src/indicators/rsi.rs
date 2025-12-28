//! Relative Strength Index (RSI) indicator.
//!
//! The Relative Strength Index is a momentum oscillator that measures the speed
//! and magnitude of price movements. It oscillates between 0 and 100, where
//! traditionally readings above 70 indicate overbought conditions and readings
//! below 30 indicate oversold conditions.
//!
//! # Algorithm
//!
//! This implementation computes RSI with O(n) time complexity using Wilder's
//! smoothing method:
//!
//! 1. Calculate price changes (current - previous)
//! 2. Separate changes into gains (positive) and losses (negative, stored as positive)
//! 3. Apply Wilder's exponential moving average to both gains and losses
//! 4. Calculate RS = Average Gain / Average Loss
//! 5. RSI = 100 - (100 / (1 + RS))
//!
//! For the first `period` values, a simple average is used as the seed, then
//! Wilder's smoothing is applied for subsequent values.
//!
//! # Formula
//!
//! ```text
//! Change[i] = Price[i] - Price[i-1]
//! Gain[i] = max(Change[i], 0)
//! Loss[i] = abs(min(Change[i], 0))
//!
//! First Average Gain = SMA(Gain[1..period])
//! First Average Loss = SMA(Loss[1..period])
//!
//! Subsequent:
//! Avg Gain[i] = (Avg Gain[i-1] * (period-1) + Gain[i]) / period
//! Avg Loss[i] = (Avg Loss[i-1] * (period-1) + Loss[i]) / period
//!
//! RS = Avg Gain / Avg Loss
//! RSI = 100 - (100 / (1 + RS))
//! ```
//!
//! # Mathematical Conventions (PRD §4.5, §4.8)
//!
//! - **Wilder's Smoothing**: Uses α = 1/period, equivalent to standard EMA with
//!   period 2n-1. This produces a slower-responding average than standard EMA.
//! - **Initialization**: First average gain/loss is the SMA of the first `period`
//!   price changes, per Wilder's original method.
//!
//! # Boundary Conditions
//!
//! These are deterministic outputs for indeterminate operations, not NaN overrides:
//!
//! - **All gains (no losses)**: RSI = 100 (maximum bullish momentum)
//! - **All losses (no gains)**: RSI = 0 (maximum bearish momentum)
//! - **No movement (no gains or losses)**: RSI = 50 (neutral midpoint)
//!
//! # Precision Behavior
//!
//! When `PrecisionMode::High` is active and input type is `f32`:
//! - Wilder smoothing state (avg_gain, avg_loss) maintained in `f64`
//! - Prevents drift accumulation over long series
//! - RSI computation and boundary checks performed in `f64`
//!
//! **Tolerance**: abs(0.01) when comparing f32 High mode to f64 reference.
//! RSI is bounded 0-100, so absolute tolerance is appropriate.
//!
//! For maximum precision over very long series (>10,000 bars), use `f64` input directly.
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::rsi::rsi;
//!
//! let data = vec![44.0_f64, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 43.25, 43.0];
//! let result = rsi(&data, 5).unwrap();
//!
//! // First 5 values are NaN (period values needed for first calculation)
//! assert!(result[0].is_nan());
//! assert!(result[4].is_nan());
//!
//! // RSI values start from index 5
//! assert!(!result[5].is_nan());
//! ```

use crate::error::{Error, Result};
use crate::precision::{current_precision_mode, PrecisionMode};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns true if we should use f64 precision for the given type.
#[inline]
fn use_f64_precision<T: 'static>() -> bool {
    use std::any::TypeId;
    TypeId::of::<T>() == TypeId::of::<f32>() && current_precision_mode() == PrecisionMode::High
}

/// Returns the lookback period for RSI.
///
/// The lookback is the number of NaN values at the start of the output.
/// For RSI, this is `period` (one more than typical indicators because
/// RSI requires price changes, not raw prices).
///
/// # Example
///
/// ```
/// use fast_ta::indicators::rsi::rsi_lookback;
///
/// assert_eq!(rsi_lookback(14), 14);
/// assert_eq!(rsi_lookback(5), 5);
/// ```
#[inline]
#[must_use]
pub const fn rsi_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for RSI.
///
/// This is the smallest input size that will produce at least one valid output.
/// For RSI, this is `period + 1` because we need `period` price changes.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::rsi::rsi_min_len;
///
/// assert_eq!(rsi_min_len(14), 15);
/// assert_eq!(rsi_min_len(5), 6);
/// ```
#[inline]
#[must_use]
pub const fn rsi_min_len(period: usize) -> usize {
    period + 1
}

/// Computes the Relative Strength Index (RSI) using Wilder's smoothing.
///
/// The RSI is a momentum oscillator that measures the speed and magnitude of
/// price changes on a scale of 0 to 100.
///
/// # Arguments
///
/// * `data` - The input price data series
/// * `period` - The number of periods for the RSI calculation (commonly 14)
///
/// # Returns
///
/// A `Result` containing a `Vec<T>` with the RSI values, or an error if validation fails.
/// The first `period` values are NaN (insufficient lookback data for both the initial
/// price change and the smoothing period).
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than period + 1 (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for the output vector
///
/// # Boundary Conditions
///
/// - If all price changes are gains (no losses), RSI = 100
/// - If all price changes are losses (no gains), RSI = 0
/// - These boundary conditions are handled correctly to avoid division by zero
///
/// # Example
///
/// ```
/// use fast_ta::indicators::rsi::rsi;
///
/// let data = vec![44.0_f64, 44.5, 45.0, 44.5, 44.0, 44.5, 45.0];
/// let result = rsi(&data, 3).unwrap();
///
/// // First 3 values are NaN
/// assert!(result[0].is_nan());
/// assert!(result[1].is_nan());
/// assert!(result[2].is_nan());
///
/// // RSI values start from index 3
/// assert!(!result[3].is_nan());
/// assert!(result[3] >= 0.0 && result[3] <= 100.0);
/// ```
#[inline]
#[must_use = "this returns a Result with the RSI values, which should be used"]
pub fn rsi<T: SeriesElement + 'static>(data: &[T], period: usize) -> Result<Vec<T>> {
    // Validate inputs
    validate_rsi_inputs(data, period)?;

    // Initialize result vector with NaN
    let mut result = vec![T::nan(); data.len()];

    // Compute RSI values into the result vector
    if use_f64_precision::<T>() {
        compute_rsi_core_f64(data, period, &mut result)?;
    } else {
        compute_rsi_core(data, period, &mut result)?;
    }

    Ok(result)
}

/// Computes the Relative Strength Index into a pre-allocated output buffer.
///
/// This variant allows reusing an existing buffer to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `data` - The input price data series
/// * `period` - The number of periods for the RSI calculation
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid RSI values computed (`data.len()` - period),
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than period + 1 (`Error::InsufficientData`)
/// - The output buffer is shorter than the input data
///
/// # Example
///
/// ```
/// use fast_ta::indicators::rsi::rsi_into;
///
/// let data = vec![44.0_f64, 44.5, 45.0, 44.5, 44.0, 44.5, 45.0];
/// let mut output = vec![0.0_f64; 7];
/// let valid_count = rsi_into(&data, 3, &mut output).unwrap();
///
/// assert_eq!(valid_count, 4); // 7 - 3 = 4 valid values
/// assert!(output[0].is_nan());
/// assert!(!output[3].is_nan());
/// ```
#[inline]
#[must_use = "this returns a Result with the count of valid RSI values"]
pub fn rsi_into<T: SeriesElement + 'static>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    // Validate inputs
    validate_rsi_inputs(data, period)?;

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "rsi",
        });
    }

    // Initialize lookback period with NaN
    for item in output.iter_mut().take(period) {
        *item = T::nan();
    }

    // Compute RSI values
    if use_f64_precision::<T>() {
        compute_rsi_core_f64(data, period, output)?;
    } else {
        compute_rsi_core(data, period, output)?;
    }

    // Return count of valid (non-NaN) values
    Ok(data.len() - period)
}

/// Validates RSI inputs.
///
/// RSI requires `period + 1` data points (one extra for the initial price change).
#[inline]
fn validate_rsi_inputs<T: SeriesElement>(data: &[T], period: usize) -> Result<()> {
    crate::traits::validate_period(period)?;

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    // RSI needs at least period + 1 data points:
    // - period data points to calculate the first period changes
    // - but we need period changes, which requires period + 1 prices
    if data.len() < period + 1 {
        return Err(Error::InsufficientData {
            required: period + 1,
            actual: data.len(),
            indicator: "rsi",
        });
    }

    Ok(())
}

/// Core RSI computation algorithm using Wilder's smoothing.
///
/// This function assumes all validation has been done and output is properly sized.
/// It fills the output slice with RSI values starting at index `period`.
fn compute_rsi_core<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
    let period_t = T::from_usize(period)?;
    let period_minus_one_t = T::from_usize(period - 1)?;
    let one = T::one();
    let hundred = T::from_f64(100.0)?;
    let zero = T::zero();

    // Step 1: Calculate price changes and separate into gains and losses
    // We need to compute the first average gain/loss from the first `period` changes

    // Calculate initial sum of gains and losses for the first period
    let mut sum_gain = T::zero();
    let mut sum_loss = T::zero();
    let mut has_nan = false;

    for i in 1..=period {
        if is_invalid(data[i]) || is_invalid(data[i - 1]) {
            has_nan = true;
            break;
        }
        let change = data[i] - data[i - 1];
        if change > zero {
            sum_gain = sum_gain + change;
        } else if change < zero {
            sum_loss = sum_loss - change; // Make loss positive
        }
    }

    // Calculate initial average gain and loss (SMA seed)
    let mut avg_gain = if has_nan {
        T::nan()
    } else {
        sum_gain / period_t
    };
    let mut avg_loss = if has_nan {
        T::nan()
    } else {
        sum_loss / period_t
    };

    // Calculate first RSI value
    output[period] = if has_nan {
        T::nan()
    } else {
        compute_rsi_value(avg_gain, avg_loss, hundred, one)
    };

    // Step 2: Apply Wilder's smoothing for remaining values
    // Wilder's formula: Avg = (prev_avg * (period-1) + current) / period
    for i in (period + 1)..data.len() {
        if is_invalid(data[i])
            || is_invalid(data[i - 1])
            || is_invalid(avg_gain)
            || is_invalid(avg_loss)
        {
            avg_gain = T::nan();
            avg_loss = T::nan();
            output[i] = T::nan();
            continue;
        }

        let change = data[i] - data[i - 1];

        let (gain, loss) = if change > zero {
            (change, zero)
        } else if change < zero {
            (zero, -change)
        } else {
            (zero, zero)
        };

        // Wilder's smoothing: new_avg = (prev_avg * (period-1) + current_value) / period
        avg_gain = (avg_gain * period_minus_one_t + gain) / period_t;
        avg_loss = (avg_loss * period_minus_one_t + loss) / period_t;

        output[i] = compute_rsi_value(avg_gain, avg_loss, hundred, one);
    }

    Ok(())
}

/// Core RSI computation using f64 precision for f32 inputs.
fn compute_rsi_core_f64<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<()> {
    let period_f64 = period as f64;
    let period_minus_one_f64 = (period - 1) as f64;

    // Step 1: Calculate initial sum of gains and losses for the first period
    let mut sum_gain: f64 = 0.0;
    let mut sum_loss: f64 = 0.0;
    let mut has_nan = false;

    for i in 1..=period {
        if is_invalid(data[i]) || is_invalid(data[i - 1]) {
            has_nan = true;
            break;
        }
        let cur = data[i].to_f64().unwrap_or(0.0);
        let prev = data[i - 1].to_f64().unwrap_or(0.0);
        let change = cur - prev;
        if change > 0.0 {
            sum_gain += change;
        } else if change < 0.0 {
            sum_loss -= change; // Make loss positive
        }
    }

    // Calculate initial average gain and loss (SMA seed)
    let mut avg_gain: f64 = if has_nan {
        f64::NAN
    } else {
        sum_gain / period_f64
    };
    let mut avg_loss: f64 = if has_nan {
        f64::NAN
    } else {
        sum_loss / period_f64
    };

    // Calculate first RSI value
    output[period] = if has_nan {
        T::nan()
    } else {
        T::from_f64(compute_rsi_value_f64(avg_gain, avg_loss))?
    };

    // Step 2: Apply Wilder's smoothing for remaining values
    for i in (period + 1)..data.len() {
        if is_invalid(data[i]) || is_invalid(data[i - 1]) || avg_gain.is_nan() || avg_loss.is_nan()
        {
            avg_gain = f64::NAN;
            avg_loss = f64::NAN;
            output[i] = T::nan();
            continue;
        }

        let cur = data[i].to_f64().unwrap_or(0.0);
        let prev = data[i - 1].to_f64().unwrap_or(0.0);
        let change = cur - prev;

        let (gain, loss) = if change > 0.0 {
            (change, 0.0)
        } else if change < 0.0 {
            (0.0, -change)
        } else {
            (0.0, 0.0)
        };

        // Wilder's smoothing in f64
        avg_gain = (avg_gain * period_minus_one_f64 + gain) / period_f64;
        avg_loss = (avg_loss * period_minus_one_f64 + loss) / period_f64;

        output[i] = T::from_f64(compute_rsi_value_f64(avg_gain, avg_loss))?;
    }

    Ok(())
}

/// Computes RSI value from average gain and loss in f64.
#[inline]
fn compute_rsi_value_f64(avg_gain: f64, avg_loss: f64) -> f64 {
    if avg_loss == 0.0 {
        if avg_gain == 0.0 {
            50.0 // No movement - neutral
        } else {
            100.0 // All gains
        }
    } else if avg_gain == 0.0 {
        0.0 // All losses
    } else {
        let rs = avg_gain / avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }
}

/// Computes the RSI value from average gain and average loss.
///
/// Handles boundary conditions:
/// - All gains (`avg_loss` = 0): RSI = 100
/// - All losses (`avg_gain` = 0): RSI = 0
/// - Normal case: RSI = 100 - (100 / (1 + RS))
#[inline]
fn compute_rsi_value<T: SeriesElement>(avg_gain: T, avg_loss: T, hundred: T, one: T) -> T {
    let zero = T::zero();

    if avg_loss == zero {
        if avg_gain == zero {
            // No movement - return 50 (neutral)
            // Some implementations return NaN here, but 50 is more practical
            T::from_f64(50.0).unwrap_or_else(|_| hundred / T::two())
        } else {
            // All gains, no losses - RSI = 100
            hundred
        }
    } else if avg_gain == zero {
        // All losses, no gains - RSI = 0
        zero
    } else {
        // Normal case: RSI = 100 - (100 / (1 + RS))
        // where RS = avg_gain / avg_loss
        let rs = avg_gain / avg_loss;
        hundred - (hundred / (one + rs))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;
    use num_traits::Float;

    // Helper function to compare floating point values
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
    const EPSILON_F32: f32 = 1e-5;
    // Looser epsilon for RSI calculations involving multiple operations
    const RSI_EPSILON: f64 = 1e-6;

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_rsi_basic() {
        // Simple ascending prices - should show RSI > 50
        let data = vec![40.0_f64, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0];
        let result = rsi(&data, 3).unwrap();

        assert_eq!(result.len(), 7);
        // First 3 values are NaN
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        // RSI values start from index 3
        assert!(!result[3].is_nan());
        // All gains, no losses - RSI should be 100
        assert!(approx_eq(result[3], 100.0, RSI_EPSILON));
    }

    #[test]
    fn test_rsi_f32() {
        let data = vec![40.0_f32, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0];
        let result = rsi(&data, 3).unwrap();

        assert_eq!(result.len(), 7);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(!result[3].is_nan());
        assert!(approx_eq(result[3], 100.0_f32, EPSILON_F32));
    }

    #[test]
    fn test_rsi_period_one() {
        // RSI(1) should show extreme values
        let data = vec![40.0_f64, 41.0, 42.0, 41.0, 42.0];
        let result = rsi(&data, 1).unwrap();

        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        // Each value represents just that day's movement
        assert!(approx_eq(result[1], 100.0, RSI_EPSILON)); // Gain
        assert!(approx_eq(result[2], 100.0, RSI_EPSILON)); // Gain
        assert!(approx_eq(result[3], 0.0, RSI_EPSILON)); // Loss
        assert!(approx_eq(result[4], 100.0, RSI_EPSILON)); // Gain
    }

    #[test]
    fn test_rsi_descending_prices() {
        // Descending prices - should show RSI < 50
        let data = vec![50.0_f64, 49.0, 48.0, 47.0, 46.0, 45.0, 44.0];
        let result = rsi(&data, 3).unwrap();

        // All losses, no gains - RSI should be 0
        assert!(approx_eq(result[3], 0.0, RSI_EPSILON));
        assert!(approx_eq(result[4], 0.0, RSI_EPSILON));
    }

    // ==================== Boundary Condition Tests ====================

    #[test]
    fn test_rsi_all_gains() {
        // All price movements are gains - RSI should be 100
        let data = vec![
            10.0_f64, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
        ];
        let result = rsi(&data, 5).unwrap();

        // All values after lookback should be 100
        for i in 5..result.len() {
            assert!(
                approx_eq(result[i], 100.0, RSI_EPSILON),
                "Expected RSI=100 at index {}, got {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_rsi_all_losses() {
        // All price movements are losses - RSI should be 0
        let data = vec![
            20.0_f64, 19.0, 18.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 10.0,
        ];
        let result = rsi(&data, 5).unwrap();

        // All values after lookback should be 0
        for i in 5..result.len() {
            assert!(
                approx_eq(result[i], 0.0, RSI_EPSILON),
                "Expected RSI=0 at index {}, got {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_rsi_no_movement() {
        // No price changes - RSI should be 50 (neutral)
        let data = vec![50.0_f64; 10];
        let result = rsi(&data, 5).unwrap();

        for i in 5..result.len() {
            assert!(
                approx_eq(result[i], 50.0, RSI_EPSILON),
                "Expected RSI=50 at index {}, got {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_rsi_transition_from_losses_to_gains() {
        // Start with losses, then gains - RSI should increase
        let data = vec![50.0_f64, 49.0, 48.0, 47.0, 48.0, 49.0, 50.0, 51.0];
        let result = rsi(&data, 3).unwrap();

        // First valid RSI (at index 3) should be low (mostly losses)
        assert!(result[3] < 50.0);
        // Last RSI should be higher (gains are dominating)
        assert!(result[result.len() - 1] > result[3]);
    }

    // ==================== Reference Value Tests ====================

    #[test]
    fn test_rsi_known_values() {
        // Test against known RSI values
        // Using example data from standard RSI calculation references
        let data = vec![
            44.0_f64, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 43.25, 43.0,
        ];
        let result = rsi(&data, 5).unwrap();

        // First 5 values should be NaN
        for i in 0..5 {
            assert!(result[i].is_nan(), "Expected NaN at index {}", i);
        }

        // RSI values should be in valid range
        for i in 5..result.len() {
            assert!(
                result[i] >= 0.0 && result[i] <= 100.0,
                "RSI at index {} is out of range: {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_rsi_typical_14_period() {
        // Standard 14-period RSI
        let data: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let result = rsi(&data, 14).unwrap();

        // First 14 values should be NaN
        for i in 0..14 {
            assert!(result[i].is_nan());
        }

        // All gains - RSI should be 100
        for i in 14..result.len() {
            assert!(approx_eq(result[i], 100.0, RSI_EPSILON));
        }
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_rsi_with_nan_in_data() {
        // NaN in the data should propagate through RSI
        let data = vec![44.0_f64, 44.5, f64::NAN, 44.0, 44.5, 45.0, 45.5];
        let result = rsi(&data, 3).unwrap();

        // NaN in data will cause NaN in calculations involving that value
        assert!(result[3].is_nan());
    }

    #[test]
    fn test_rsi_negative_prices() {
        // RSI should work with negative values (unusual but valid)
        let data = vec![-10.0_f64, -9.0, -8.0, -7.0, -6.0, -5.0];
        let result = rsi(&data, 3).unwrap();

        // All gains - RSI should be 100
        assert!(approx_eq(result[3], 100.0, RSI_EPSILON));
    }

    #[test]
    fn test_rsi_large_values() {
        let data = vec![1e12_f64, 1.01e12, 1.02e12, 1.03e12, 1.04e12, 1.05e12];
        let result = rsi(&data, 3).unwrap();

        // All gains - RSI should be 100
        assert!(approx_eq(result[3], 100.0, RSI_EPSILON));
    }

    #[test]
    fn test_rsi_small_values() {
        let data = vec![1e-10_f64, 1.1e-10, 1.2e-10, 1.3e-10, 1.4e-10, 1.5e-10];
        let result = rsi(&data, 3).unwrap();

        // All gains - RSI should be 100
        assert!(approx_eq(result[3], 100.0, RSI_EPSILON));
    }

    #[test]
    fn test_rsi_alternating_prices() {
        // Alternating up/down movements
        let data = vec![50.0_f64, 51.0, 50.0, 51.0, 50.0, 51.0, 50.0, 51.0];
        let result = rsi(&data, 3).unwrap();

        // With alternating gains and losses of equal magnitude, RSI should be around 50
        for i in 3..result.len() {
            assert!(
                result[i] >= 30.0 && result[i] <= 70.0,
                "RSI at index {} should be around 50, got {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_rsi_infinity_handling() {
        // Infinity in data - result should handle gracefully
        let data = vec![1.0_f64, 2.0, f64::INFINITY, 4.0, 5.0, 6.0];
        let result = rsi(&data, 3).unwrap();

        // Results involving infinity should be NaN
        assert!(result[3].is_nan());
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_rsi_empty_input() {
        let data: Vec<f64> = vec![];
        let result = rsi(&data, 3);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_rsi_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = rsi(&data, 0);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_rsi_period_exceeds_length() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = rsi(&data, 5);

        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                required: 6,
                actual: 3,
                ..
            })
        ));
    }

    #[test]
    fn test_rsi_period_equals_length_minus_one() {
        // Need period + 1 data points
        let data = vec![1.0_f64, 2.0, 3.0, 4.0];
        let result = rsi(&data, 3);

        assert!(result.is_ok());
        let res = result.unwrap();
        // Should have exactly one valid RSI value
        assert!(res[3] > 0.0);
    }

    #[test]
    fn test_rsi_minimum_data() {
        // Minimum valid case: period + 1 data points
        let data = vec![1.0_f64, 2.0];
        let result = rsi(&data, 1);

        assert!(result.is_ok());
        let res = result.unwrap();
        assert!(res[0].is_nan());
        assert!(approx_eq(res[1], 100.0, RSI_EPSILON)); // All gains
    }

    // ==================== rsi_into Tests ====================

    #[test]
    fn test_rsi_into_basic() {
        let data = vec![40.0_f64, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0];
        let mut output = vec![0.0_f64; 7];
        let valid_count = rsi_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 4); // 7 - 3 = 4 valid values
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!(output[2].is_nan());
        assert!(approx_eq(output[3], 100.0, RSI_EPSILON));
    }

    #[test]
    fn test_rsi_into_buffer_reuse() {
        let data1 = vec![40.0_f64, 41.0, 42.0, 43.0, 44.0];
        let data2 = vec![50.0_f64, 49.0, 48.0, 47.0, 46.0];
        let mut output = vec![0.0_f64; 5];

        rsi_into(&data1, 3, &mut output).unwrap();
        assert!(approx_eq(output[3], 100.0, RSI_EPSILON)); // All gains

        rsi_into(&data2, 3, &mut output).unwrap();
        assert!(approx_eq(output[3], 0.0, RSI_EPSILON)); // All losses
    }

    #[test]
    fn test_rsi_into_insufficient_output() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f64; 3]; // Too short
        let result = rsi_into(&data, 3, &mut output);

        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_rsi_into_empty_input() {
        let data: Vec<f64> = vec![];
        let mut output = vec![0.0_f64; 5];
        let result = rsi_into(&data, 3, &mut output);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_rsi_into_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let mut output = vec![0.0_f64; 3];
        let result = rsi_into(&data, 0, &mut output);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_rsi_into_f32() {
        let data = vec![40.0_f32, 41.0, 42.0, 43.0, 44.0];
        let mut output = vec![0.0_f32; 5];
        let valid_count = rsi_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 2);
        assert!(approx_eq(output[3], 100.0_f32, EPSILON_F32));
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_rsi_and_rsi_into_produce_same_result() {
        let data = vec![50.0_f64, 51.0, 49.0, 52.0, 48.0, 53.0, 47.0, 54.0];
        let result1 = rsi(&data, 4).unwrap();

        let mut result2 = vec![0.0_f64; data.len()];
        rsi_into(&data, 4, &mut result2).unwrap();

        for i in 0..data.len() {
            assert!(
                approx_eq(result1[i], result2[i], EPSILON),
                "Mismatch at index {}: {} vs {}",
                i,
                result1[i],
                result2[i]
            );
        }
    }

    #[test]
    fn test_rsi_valid_count() {
        let data = vec![1.0_f64; 100];
        let mut output = vec![0.0_f64; 100];

        let valid_count = rsi_into(&data, 10, &mut output).unwrap();
        assert_eq!(valid_count, 90); // 100 - 10 = 90

        let valid_count = rsi_into(&data, 1, &mut output).unwrap();
        assert_eq!(valid_count, 99); // 100 - 1 = 99

        let valid_count = rsi_into(&data, 99, &mut output).unwrap();
        assert_eq!(valid_count, 1); // 100 - 99 = 1
    }

    // ==================== Property-Based-Like Tests ====================

    #[test]
    fn test_rsi_output_length_equals_input_length() {
        for len in [5, 10, 50, 100] {
            for period in [1, 2, 5] {
                if period < len {
                    let data: Vec<f64> = (0..len).map(|x| x as f64).collect();
                    let result = rsi(&data, period).unwrap();
                    assert_eq!(result.len(), len);
                }
            }
        }
    }

    #[test]
    fn test_rsi_nan_count() {
        // First `period` values should be NaN
        for period in 1..=10 {
            let data: Vec<f64> = (0..20).map(|x| x as f64).collect();
            let result = rsi(&data, period).unwrap();

            let nan_count = result.iter().filter(|x| x.is_nan()).count();
            assert_eq!(
                nan_count, period,
                "Expected {} NaN values for period {}",
                period, period
            );
        }
    }

    #[test]
    fn test_rsi_bounds() {
        // RSI should always be between 0 and 100
        let data: Vec<f64> = vec![
            50.0, 51.0, 49.0, 52.0, 48.0, 53.0, 47.0, 54.0, 46.0, 55.0, 45.0, 56.0, 44.0, 57.0,
            43.0, 58.0, 42.0, 59.0, 41.0, 60.0,
        ];
        let result = rsi(&data, 5).unwrap();

        for (i, &val) in result.iter().enumerate() {
            if !val.is_nan() {
                assert!(
                    val >= 0.0 && val <= 100.0,
                    "RSI at index {} is out of bounds: {}",
                    i,
                    val
                );
            }
        }
    }

    #[test]
    fn test_rsi_responds_to_trend_changes() {
        // RSI should respond to trend reversals
        let mut data: Vec<f64> = (0..10).map(|i| 50.0 + i as f64).collect(); // Uptrend
        data.extend((0..10).map(|i| 59.0 - i as f64)); // Downtrend

        let result = rsi(&data, 5).unwrap();

        // Find RSI values during uptrend and downtrend
        let uptrend_rsi = result[8]; // During uptrend
        let downtrend_rsi = result[18]; // During downtrend

        assert!(
            uptrend_rsi > downtrend_rsi,
            "RSI during uptrend ({}) should be > RSI during downtrend ({})",
            uptrend_rsi,
            downtrend_rsi
        );
    }

    // ==================== RSI Value Computation Tests ====================

    #[test]
    fn test_compute_rsi_value_all_gains() {
        let hundred = 100.0_f64;
        let one = 1.0_f64;
        let result = compute_rsi_value(1.0_f64, 0.0_f64, hundred, one);
        assert!(approx_eq(result, 100.0, EPSILON));
    }

    #[test]
    fn test_compute_rsi_value_all_losses() {
        let hundred = 100.0_f64;
        let one = 1.0_f64;
        let result = compute_rsi_value(0.0_f64, 1.0_f64, hundred, one);
        assert!(approx_eq(result, 0.0, EPSILON));
    }

    #[test]
    fn test_compute_rsi_value_no_movement() {
        let hundred = 100.0_f64;
        let one = 1.0_f64;
        let result = compute_rsi_value(0.0_f64, 0.0_f64, hundred, one);
        assert!(approx_eq(result, 50.0, EPSILON));
    }

    #[test]
    fn test_compute_rsi_value_equal_gains_and_losses() {
        let hundred = 100.0_f64;
        let one = 1.0_f64;
        // RS = 1, RSI = 100 - (100 / 2) = 50
        let result = compute_rsi_value(1.0_f64, 1.0_f64, hundred, one);
        assert!(approx_eq(result, 50.0, EPSILON));
    }

    #[test]
    fn test_compute_rsi_value_double_gains() {
        let hundred = 100.0_f64;
        let one = 1.0_f64;
        // RS = 2, RSI = 100 - (100 / 3) = 66.67
        let result = compute_rsi_value(2.0_f64, 1.0_f64, hundred, one);
        assert!(approx_eq(result, 100.0 - 100.0 / 3.0, EPSILON));
    }

    // ==================== Wilder Smoothing Verification ====================

    #[test]
    fn test_rsi_wilder_smoothing_behavior() {
        // Verify that RSI uses Wilder's smoothing correctly
        // After a sudden change, RSI should gradually revert, not jump immediately

        // Create data with a spike then flat
        let mut data = vec![50.0_f64; 10];
        data[5] = 60.0; // Spike up
                        // Then flat at 60
        for i in 6..data.len() {
            data[i] = 60.0;
        }

        let result = rsi(&data, 3).unwrap();

        // After the spike, RSI should gradually decrease toward neutral
        // as the smoothed average adjusts
        if !result[7].is_nan() && !result[8].is_nan() {
            // RSI should stabilize/decrease after initial spike impact
            // (Wilder smoothing has memory effect)
            assert!(result[7] > 50.0); // Still elevated from the spike
        }
    }
}
