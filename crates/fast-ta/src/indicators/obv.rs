//! On-Balance Volume (OBV) indicator.
//!
//! OBV is a momentum indicator that uses volume flow to predict changes in stock price.
//! It was developed by Joseph Granville and introduced in his 1963 book "Granville's New
//! Key to Stock Market Profits."
//!
//! # Algorithm
//!
//! OBV is a cumulative indicator:
//!
//! ```text
//! OBV[0] = volume[0]
//!
//! If close[i] > close[i-1]: OBV[i] = OBV[i-1] + volume[i]
//! If close[i] < close[i-1]: OBV[i] = OBV[i-1] - volume[i]
//! If close[i] == close[i-1]: OBV[i] = OBV[i-1]
//! ```
//!
//! # Interpretation
//!
//! - Rising OBV indicates buying pressure (accumulation)
//! - Falling OBV indicates selling pressure (distribution)
//! - OBV diverging from price can signal potential reversals
//! - Volume precedes price according to OBV theory
//!
//! # NaN Handling
//!
//! - If any input for the current step is NaN, the output at that index is NaN.
//! - Because OBV is cumulative, once NaN appears, subsequent values remain NaN.
//! - The first value is `volume[0]` unless `close[0]` or `volume[0]` is NaN.
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::obv::obv;
//!
//! let close = vec![10.0_f64, 10.5, 10.2, 10.8, 10.5];
//! let volume = vec![1000.0, 1500.0, 1200.0, 1800.0, 1100.0];
//!
//! let result = obv(&close, &volume).unwrap();
//!
//! // OBV[0] = 1000 (first volume)
//! // OBV[1] = 1000 + 1500 = 2500 (close up)
//! // OBV[2] = 2500 - 1200 = 1300 (close down)
//! // OBV[3] = 1300 + 1800 = 3100 (close up)
//! // OBV[4] = 3100 - 1100 = 2000 (close down)
//! assert_eq!(result.len(), 5);
//! ```

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns the lookback period for OBV.
///
/// OBV has no lookback period since the first value is valid immediately.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::obv::obv_lookback;
///
/// assert_eq!(obv_lookback(), 0);
/// ```
#[inline]
#[must_use]
pub const fn obv_lookback() -> usize {
    0
}

/// Returns the minimum input length required for OBV.
///
/// At least 1 data point is required to compute OBV.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::obv::obv_min_len;
///
/// assert_eq!(obv_min_len(), 1);
/// ```
#[inline]
#[must_use]
pub const fn obv_min_len() -> usize {
    1
}

/// Computes On-Balance Volume for close and volume data.
///
/// # Arguments
///
/// * `close` - The closing prices series
/// * `volume` - The volume series
///
/// # Returns
///
/// A `Result` containing a vector of OBV values.
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
/// use fast_ta::indicators::obv::obv;
///
/// let close = vec![10.0_f64, 11.0, 10.5, 11.5, 11.0];
/// let volume = vec![100.0, 150.0, 120.0, 200.0, 130.0];
///
/// let result = obv(&close, &volume).unwrap();
///
/// // All values are valid (no lookback)
/// for val in &result {
///     assert!(!val.is_nan());
/// }
/// ```
#[must_use = "this returns a Result with OBV values, which should be used"]
pub fn obv<T: SeriesElement>(close: &[T], volume: &[T]) -> Result<Vec<T>> {
    validate_inputs(close, volume)?;

    let n = close.len();
    let mut output = Vec::with_capacity(n);

    // Compute OBV
    compute_obv_core(close, volume, &mut output);

    Ok(output)
}

/// Computes On-Balance Volume into a pre-allocated output buffer.
///
/// This variant allows reusing existing buffers to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `close` - The closing prices series
/// * `volume` - The volume series
/// * `output` - Pre-allocated buffer for OBV values
///
/// # Returns
///
/// A `Result` containing the number of valid values computed (always n for OBV),
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
/// use fast_ta::indicators::obv::obv_into;
///
/// let close = vec![10.0_f64, 11.0, 10.5, 11.5, 11.0];
/// let volume = vec![100.0, 150.0, 120.0, 200.0, 130.0];
/// let mut output = vec![0.0_f64; 5];
///
/// let valid_count = obv_into(&close, &volume, &mut output).unwrap();
/// assert_eq!(valid_count, 5); // All values are valid
/// ```
#[must_use = "this returns a Result with the count of valid OBV values"]
pub fn obv_into<T: SeriesElement>(close: &[T], volume: &[T], output: &mut [T]) -> Result<usize> {
    validate_inputs(close, volume)?;

    let n = close.len();

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.len(),
            indicator: "obv",
        });
    }

    // Compute OBV directly into buffer
    compute_obv_core_into(close, volume, output);

    Ok(n)
}

/// Validates input data.
fn validate_inputs<T: SeriesElement>(close: &[T], volume: &[T]) -> Result<()> {
    if close.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = close.len();

    if volume.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("close has {} elements, volume has {}", n, volume.len()),
        });
    }

    Ok(())
}

/// Core OBV computation that allocates a new vector.
fn compute_obv_core<T: SeriesElement>(close: &[T], volume: &[T], output: &mut Vec<T>) {
    let n = close.len();

    if n == 0 {
        return;
    }

    // First OBV value is the first volume
    let first_vol = volume[0];
    let first_close = close[0];
    if is_invalid(first_vol) || is_invalid(first_close) {
        output.push(T::nan());
    } else {
        output.push(first_vol);
    }

    // Compute subsequent values
    for i in 1..n {
        let prev_obv = output[i - 1];
        let curr_close = close[i];
        let prev_close = close[i - 1];
        let curr_vol = volume[i];

        let new_obv = compute_obv_step(prev_obv, prev_close, curr_close, curr_vol);
        output.push(new_obv);
    }
}

/// Core OBV computation into pre-allocated buffer.
fn compute_obv_core_into<T: SeriesElement>(close: &[T], volume: &[T], output: &mut [T]) {
    let n = close.len();

    if n == 0 {
        return;
    }

    // First OBV value is the first volume
    let first_vol = volume[0];
    let first_close = close[0];
    output[0] = if is_invalid(first_vol) || is_invalid(first_close) {
        T::nan()
    } else {
        first_vol
    };

    // Compute subsequent values
    for i in 1..n {
        let prev_obv = output[i - 1];
        let curr_close = close[i];
        let prev_close = close[i - 1];
        let curr_vol = volume[i];

        output[i] = compute_obv_step(prev_obv, prev_close, curr_close, curr_vol);
    }
}

/// Compute single OBV step.
#[inline]
fn compute_obv_step<T: SeriesElement>(prev_obv: T, prev_close: T, curr_close: T, curr_vol: T) -> T {
    // If any value is NaN, output stays NaN for cumulative consistency.
    if is_invalid(prev_obv)
        || is_invalid(curr_close)
        || is_invalid(prev_close)
        || is_invalid(curr_vol)
    {
        return T::nan();
    }

    if curr_close > prev_close {
        prev_obv + curr_vol
    } else if curr_close < prev_close {
        prev_obv - curr_vol
    } else {
        prev_obv
    }
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
    fn test_obv_lookback() {
        assert_eq!(obv_lookback(), 0);
    }

    #[test]
    fn test_obv_min_len() {
        assert_eq!(obv_min_len(), 1);
    }

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_obv_basic() {
        let close = vec![10.0_f64, 10.5, 10.2, 10.8, 10.5];
        let volume = vec![1000.0, 1500.0, 1200.0, 1800.0, 1100.0];

        let result = obv(&close, &volume).unwrap();

        assert_eq!(result.len(), 5);

        // All values should be valid (no NaN lookback)
        for val in &result {
            assert!(!val.is_nan(), "OBV values should not be NaN");
        }
    }

    #[test]
    fn test_obv_f32() {
        let close = vec![10.0_f32, 11.0, 10.5];
        let volume = vec![100.0, 150.0, 120.0];

        let result = obv(&close, &volume).unwrap();

        assert_eq!(result.len(), 3);
        assert!(approx_eq(result[0], 100.0, 0.001));
    }

    #[test]
    fn test_obv_single_value() {
        let close = vec![10.0_f64];
        let volume = vec![1000.0];

        let result = obv(&close, &volume).unwrap();

        assert_eq!(result.len(), 1);
        assert!(approx_eq(result[0], 1000.0, EPSILON));
    }

    // ==================== Known Value Tests ====================

    #[test]
    fn test_obv_known_values() {
        let close = vec![10.0_f64, 10.5, 10.2, 10.8, 10.5];
        let volume = vec![1000.0, 1500.0, 1200.0, 1800.0, 1100.0];

        let result = obv(&close, &volume).unwrap();

        // OBV[0] = 1000 (first volume)
        assert!(approx_eq(result[0], 1000.0, EPSILON));

        // OBV[1] = 1000 + 1500 = 2500 (close up: 10.5 > 10.0)
        assert!(approx_eq(result[1], 2500.0, EPSILON));

        // OBV[2] = 2500 - 1200 = 1300 (close down: 10.2 < 10.5)
        assert!(approx_eq(result[2], 1300.0, EPSILON));

        // OBV[3] = 1300 + 1800 = 3100 (close up: 10.8 > 10.2)
        assert!(approx_eq(result[3], 3100.0, EPSILON));

        // OBV[4] = 3100 - 1100 = 2000 (close down: 10.5 < 10.8)
        assert!(approx_eq(result[4], 2000.0, EPSILON));
    }

    #[test]
    fn test_obv_close_up() {
        // When close goes up, OBV should increase by volume
        let close = vec![10.0_f64, 11.0, 12.0, 13.0];
        let volume = vec![100.0, 200.0, 300.0, 400.0];

        let result = obv(&close, &volume).unwrap();

        // All up days: OBV = cumsum(volume)
        assert!(approx_eq(result[0], 100.0, EPSILON));
        assert!(approx_eq(result[1], 300.0, EPSILON)); // 100 + 200
        assert!(approx_eq(result[2], 600.0, EPSILON)); // 300 + 300
        assert!(approx_eq(result[3], 1000.0, EPSILON)); // 600 + 400
    }

    #[test]
    fn test_obv_close_down() {
        // When close goes down, OBV should decrease by volume
        let close = vec![13.0_f64, 12.0, 11.0, 10.0];
        let volume = vec![100.0, 200.0, 300.0, 400.0];

        let result = obv(&close, &volume).unwrap();

        // All down days: OBV = volume[0] - cumsum(volume[1:])
        assert!(approx_eq(result[0], 100.0, EPSILON));
        assert!(approx_eq(result[1], -100.0, EPSILON)); // 100 - 200
        assert!(approx_eq(result[2], -400.0, EPSILON)); // -100 - 300
        assert!(approx_eq(result[3], -800.0, EPSILON)); // -400 - 400
    }

    #[test]
    fn test_obv_close_unchanged() {
        // When close is unchanged, OBV should not change
        let close = vec![10.0_f64, 10.0, 10.0, 10.0];
        let volume = vec![100.0, 200.0, 300.0, 400.0];

        let result = obv(&close, &volume).unwrap();

        // OBV stays at first volume value
        assert!(approx_eq(result[0], 100.0, EPSILON));
        assert!(approx_eq(result[1], 100.0, EPSILON));
        assert!(approx_eq(result[2], 100.0, EPSILON));
        assert!(approx_eq(result[3], 100.0, EPSILON));
    }

    #[test]
    fn test_obv_cumulative_behavior() {
        // Verify cumulative sum behavior
        let close = vec![10.0_f64, 11.0, 10.0, 11.0, 10.0];
        let volume = vec![100.0; 5];

        let result = obv(&close, &volume).unwrap();

        // Alternating: +100, -100, +100, -100
        assert!(approx_eq(result[0], 100.0, EPSILON));
        assert!(approx_eq(result[1], 200.0, EPSILON)); // 100 + 100
        assert!(approx_eq(result[2], 100.0, EPSILON)); // 200 - 100
        assert!(approx_eq(result[3], 200.0, EPSILON)); // 100 + 100
        assert!(approx_eq(result[4], 100.0, EPSILON)); // 200 - 100
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_obv_empty_input() {
        let close: Vec<f64> = vec![];
        let volume: Vec<f64> = vec![];

        let result = obv(&close, &volume);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_obv_mismatched_lengths() {
        let close = vec![10.0_f64, 11.0, 12.0];
        let volume = vec![100.0, 200.0]; // One less

        let result = obv(&close, &volume);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    // ==================== NaN Handling Tests ====================

    #[test]
    fn test_obv_nan_close_current() {
        // NaN in current close: OBV becomes NaN and stays NaN
        let close = vec![10.0_f64, f64::NAN, 12.0];
        let volume = vec![100.0, 200.0, 300.0];

        let result = obv(&close, &volume).unwrap();

        assert!(approx_eq(result[0], 100.0, EPSILON));
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }

    #[test]
    fn test_obv_nan_volume() {
        // NaN in volume: OBV becomes NaN and stays NaN
        let close = vec![10.0_f64, 11.0, 12.0];
        let volume = vec![100.0, f64::NAN, 300.0];

        let result = obv(&close, &volume).unwrap();

        assert!(approx_eq(result[0], 100.0, EPSILON));
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }

    #[test]
    fn test_obv_nan_first_volume() {
        // NaN in first volume: OBV starts as NaN and stays NaN
        let close = vec![10.0_f64, 11.0, 12.0];
        let volume = vec![f64::NAN, 200.0, 300.0];

        let result = obv(&close, &volume).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }

    // ==================== obv_into Tests ====================

    #[test]
    fn test_obv_into_basic() {
        let close = vec![10.0_f64, 10.5, 10.2, 10.8, 10.5];
        let volume = vec![1000.0, 1500.0, 1200.0, 1800.0, 1100.0];
        let mut output = vec![0.0_f64; 5];

        let valid_count = obv_into(&close, &volume, &mut output).unwrap();

        assert_eq!(valid_count, 5);
        assert!(approx_eq(output[0], 1000.0, EPSILON));
        assert!(approx_eq(output[1], 2500.0, EPSILON));
    }

    #[test]
    fn test_obv_into_buffer_too_small() {
        let close = vec![10.0_f64; 10];
        let volume = vec![100.0_f64; 10];
        let mut output = vec![0.0_f64; 5]; // Too short

        let result = obv_into(&close, &volume, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_obv_and_obv_into_produce_same_result() {
        let close = vec![10.0_f64, 10.5, 10.2, 10.8, 10.5, 11.0, 10.8, 11.2];
        let volume = vec![
            1000.0, 1500.0, 1200.0, 1800.0, 1100.0, 1600.0, 1300.0, 1700.0,
        ];

        let result1 = obv(&close, &volume).unwrap();

        let mut output = vec![0.0_f64; 8];
        obv_into(&close, &volume, &mut output).unwrap();

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
    fn test_obv_output_length_equals_input_length() {
        for len in [1, 10, 50, 100] {
            let close: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) * 0.1).collect();
            let volume: Vec<f64> = (0..len).map(|i| 1000.0 + (i as f64) * 10.0).collect();

            let result = obv(&close, &volume).unwrap();
            assert_eq!(result.len(), len);
        }
    }

    #[test]
    fn test_obv_no_nan_in_output() {
        // With valid input, OBV should have no NaN
        let close: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64) * 0.1).collect();
        let volume: Vec<f64> = (0..50).map(|i| 1000.0 + (i as f64) * 10.0).collect();

        let result = obv(&close, &volume).unwrap();

        for (i, val) in result.iter().enumerate() {
            assert!(!val.is_nan(), "OBV should not have NaN at index {}", i);
        }
    }

    #[test]
    fn test_obv_monotonic_up_market() {
        // In a consistently up market, OBV should always increase
        let close: Vec<f64> = (0..20).map(|i| 100.0 + (i as f64)).collect();
        let volume: Vec<f64> = vec![100.0; 20];

        let result = obv(&close, &volume).unwrap();

        for i in 1..result.len() {
            assert!(
                result[i] > result[i - 1],
                "OBV should increase in up market: {} > {}",
                result[i],
                result[i - 1]
            );
        }
    }

    #[test]
    fn test_obv_monotonic_down_market() {
        // In a consistently down market, OBV should always decrease
        let close: Vec<f64> = (0..20).map(|i| 100.0 - (i as f64)).collect();
        let volume: Vec<f64> = vec![100.0; 20];

        let result = obv(&close, &volume).unwrap();

        for i in 1..result.len() {
            assert!(
                result[i] < result[i - 1],
                "OBV should decrease in down market: {} < {}",
                result[i],
                result[i - 1]
            );
        }
    }

    // ==================== Real-World Scenario Tests ====================

    #[test]
    fn test_obv_accumulation() {
        // Rising OBV with rising volume indicates accumulation
        // Price may be flat but OBV rising signals buying pressure
        let close = vec![100.0_f64, 100.5, 100.3, 101.0, 100.8, 101.5, 101.2, 102.0];
        let volume = vec![1000.0, 2000.0, 500.0, 3000.0, 800.0, 3500.0, 600.0, 4000.0];

        let result = obv(&close, &volume).unwrap();

        // OBV trend should be positive for accumulation
        // Compare first third to last third
        let first_avg = (result[0] + result[1] + result[2]) / 3.0;
        let last_avg = (result[5] + result[6] + result[7]) / 3.0;

        assert!(
            last_avg > first_avg,
            "OBV should trend up during accumulation: {} > {}",
            last_avg,
            first_avg
        );
    }

    #[test]
    fn test_obv_distribution() {
        // Falling OBV with rising volume indicates distribution
        let close = vec![100.0_f64, 99.5, 99.8, 99.0, 99.3, 98.5, 98.8, 98.0];
        let volume = vec![1000.0, 2000.0, 500.0, 3000.0, 800.0, 3500.0, 600.0, 4000.0];

        let result = obv(&close, &volume).unwrap();

        // OBV trend should be negative for distribution
        let first_avg = (result[0] + result[1] + result[2]) / 3.0;
        let last_avg = (result[5] + result[6] + result[7]) / 3.0;

        assert!(
            last_avg < first_avg,
            "OBV should trend down during distribution: {} < {}",
            last_avg,
            first_avg
        );
    }

    #[test]
    fn test_obv_divergence() {
        // Price making new highs but OBV not confirming could signal weakness
        // First, price up with volume; then price up but declining OBV (bearish divergence)
        let close = vec![100.0_f64, 102.0, 104.0, 106.0, 107.0, 108.0];
        let volume = vec![1000.0, 1500.0, 2000.0, 500.0, 300.0, 200.0];

        let result = obv(&close, &volume).unwrap();

        // Price makes new high at each bar, but volume dries up
        // OBV should still increase (all up days) but with smaller increments
        let early_obv_increase = result[2] - result[0];
        let late_obv_increase = result[5] - result[3];

        assert!(
            early_obv_increase > late_obv_increase,
            "Early OBV gains should exceed late gains: {} > {}",
            early_obv_increase,
            late_obv_increase
        );
    }

    #[test]
    fn test_obv_zero_volume() {
        // Zero volume days shouldn't affect OBV (similar to unchanged close)
        let close = vec![100.0_f64, 101.0, 102.0, 101.0];
        let volume = vec![1000.0, 0.0, 500.0, 0.0];

        let result = obv(&close, &volume).unwrap();

        assert!(approx_eq(result[0], 1000.0, EPSILON));
        assert!(approx_eq(result[1], 1000.0, EPSILON)); // +0
        assert!(approx_eq(result[2], 1500.0, EPSILON)); // +500
        assert!(approx_eq(result[3], 1500.0, EPSILON)); // -0
    }

    #[test]
    fn test_obv_large_values() {
        // Test with large volume values (institutional trading)
        let close = vec![100.0_f64, 101.0, 100.5];
        let volume = vec![1_000_000_000.0, 2_000_000_000.0, 1_500_000_000.0];

        let result = obv(&close, &volume).unwrap();

        assert!(approx_eq(result[0], 1_000_000_000.0, EPSILON));
        assert!(approx_eq(result[1], 3_000_000_000.0, EPSILON));
        assert!(approx_eq(result[2], 1_500_000_000.0, EPSILON));
    }
}
