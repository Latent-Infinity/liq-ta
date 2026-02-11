//! Donchian Channels indicator.
//!
//! Donchian Channels are a price channel indicator that shows the highest high
//! and lowest low over a specified period. They were developed by Richard Donchian
//! and are commonly used for trend following and breakout trading strategies.
//!
//! # Algorithm
//!
//! Donchian Channels consist of three bands:
//! - **Upper Band**: The highest high over the lookback period
//! - **Lower Band**: The lowest low over the lookback period
//! - **Middle Band**: The average of the upper and lower bands
//!
//! ```text
//! Upper = max(High[i-period+1..=i])
//! Lower = min(Low[i-period+1..=i])
//! Middle = (Upper + Lower) / 2
//! ```
//!
//! # Interpretation
//!
//! - Price touching the upper band indicates bullish momentum
//! - Price touching the lower band indicates bearish momentum
//! - Breakouts above the upper band may signal the start of an uptrend
//! - Breakouts below the lower band may signal the start of a downtrend
//! - Channel width indicates volatility (wider = more volatile)
//!
//! # NaN Handling
//!
//! The first `period - 1` values are NaN (insufficient lookback data).
//!
//! # Example
//!
//! ```
//! use liq_ta::indicators::donchian::donchian;
//!
//! let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35];
//! let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86];
//!
//! let result = donchian(&high, &low, 5).unwrap();
//!
//! // First 4 values are NaN
//! assert!(result.upper[3].is_nan());
//!
//! // Values start from index 4
//! assert!(!result.upper[4].is_nan());
//! assert!(!result.lower[4].is_nan());
//! assert!(!result.middle[4].is_nan());
//! ```

use crate::error::{Error, Result};
use crate::kernels::rolling_extrema::{rolling_max, rolling_min};
use crate::traits::SeriesElement;

/// Output structure for Donchian Channels containing upper, middle, and lower bands.
#[derive(Debug, Clone)]
pub struct DonchianOutput<T> {
    /// Upper band: highest high over the period.
    pub upper: Vec<T>,
    /// Middle band: average of upper and lower.
    pub middle: Vec<T>,
    /// Lower band: lowest low over the period.
    pub lower: Vec<T>,
}

/// Returns the lookback period for Donchian Channels.
///
/// The lookback is the number of NaN values at the start of the output.
/// For Donchian Channels, this is `period - 1`.
///
/// # Example
///
/// ```
/// use liq_ta::indicators::donchian::donchian_lookback;
///
/// assert_eq!(donchian_lookback(20), 19);
/// assert_eq!(donchian_lookback(5), 4);
/// ```
#[inline]
#[must_use]
pub const fn donchian_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

/// Returns the minimum input length required for Donchian Channels.
///
/// This is the smallest input size that will produce at least one valid output.
/// For Donchian Channels, this equals the period.
///
/// # Example
///
/// ```
/// use liq_ta::indicators::donchian::donchian_min_len;
///
/// assert_eq!(donchian_min_len(20), 20);
/// assert_eq!(donchian_min_len(5), 5);
/// ```
#[inline]
#[must_use]
pub const fn donchian_min_len(period: usize) -> usize {
    period
}

/// Computes Donchian Channels for high/low price data.
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `period` - The lookback period (commonly 20)
///
/// # Returns
///
/// A `Result` containing `DonchianOutput` with upper, middle, and lower bands.
/// The first `period - 1` values are NaN.
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The series have different lengths (`Error::LengthMismatch`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for the three output vectors
///
/// # Example
///
/// ```
/// use liq_ta::indicators::donchian::donchian;
///
/// let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35];
/// let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86];
///
/// let result = donchian(&high, &low, 5).unwrap();
///
/// // Upper band is highest high over period
/// // Lower band is lowest low over period
/// // Middle band is average of upper and lower
/// for i in 4..result.upper.len() {
///     assert!(result.upper[i] >= result.middle[i]);
///     assert!(result.middle[i] >= result.lower[i]);
/// }
/// ```
#[must_use = "this returns a Result with Donchian output, which should be used"]
pub fn donchian<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
) -> Result<DonchianOutput<T>> {
    validate_inputs(high, low, period)?;

    let n = high.len();
    let mut upper = vec![T::nan(); n];
    let mut middle = vec![T::nan(); n];
    let mut lower = vec![T::nan(); n];

    compute_donchian_core(high, low, period, &mut upper, &mut middle, &mut lower)?;

    Ok(DonchianOutput {
        upper,
        middle,
        lower,
    })
}

/// Computes Donchian Channels into pre-allocated output buffers.
///
/// This variant allows reusing existing buffers to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `high` - The high prices series
/// * `low` - The low prices series
/// * `period` - The lookback period
/// * `upper_out` - Pre-allocated buffer for upper band
/// * `middle_out` - Pre-allocated buffer for middle band
/// * `lower_out` - Pre-allocated buffer for lower band
///
/// # Returns
///
/// A `Result` containing the number of valid values computed (n - period + 1),
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - Any input series is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The series have different lengths (`Error::LengthMismatch`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
/// - Any output buffer is shorter than the input data
///
/// # Example
///
/// ```
/// use liq_ta::indicators::donchian::donchian_into;
///
/// let high = vec![48.70_f64, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35];
/// let low = vec![47.79_f64, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86];
/// let mut upper = vec![0.0_f64; 8];
/// let mut middle = vec![0.0_f64; 8];
/// let mut lower = vec![0.0_f64; 8];
///
/// let valid_count = donchian_into(&high, &low, 5, &mut upper, &mut middle, &mut lower).unwrap();
/// assert_eq!(valid_count, 4); // 8 - 4 = 4 valid values
/// ```
#[must_use = "this returns a Result with the count of valid Donchian values"]
pub fn donchian_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
    upper_out: &mut [T],
    middle_out: &mut [T],
    lower_out: &mut [T],
) -> Result<usize> {
    validate_inputs(high, low, period)?;

    let n = high.len();

    if upper_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: upper_out.len(),
            indicator: "donchian (upper)",
        });
    }
    if middle_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: middle_out.len(),
            indicator: "donchian (middle)",
        });
    }
    if lower_out.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: lower_out.len(),
            indicator: "donchian (lower)",
        });
    }

    // Initialize lookback period with NaN
    let lookback = donchian_lookback(period);
    for i in 0..lookback.min(n) {
        upper_out[i] = T::nan();
        middle_out[i] = T::nan();
        lower_out[i] = T::nan();
    }

    compute_donchian_core(high, low, period, upper_out, middle_out, lower_out)?;

    Ok(n.saturating_sub(lookback))
}

/// Validates input data.
fn validate_inputs<T: SeriesElement>(high: &[T], low: &[T], period: usize) -> Result<()> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if high.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = high.len();

    if low.len() != n {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", n, low.len()),
        });
    }

    if n < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: n,
            indicator: "donchian",
        });
    }

    Ok(())
}

/// Core Donchian computation.
fn compute_donchian_core<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
    upper: &mut [T],
    middle: &mut [T],
    lower: &mut [T],
) -> Result<()> {
    let two = T::two();
    let n = high.len();

    // Calculate rolling highest high and lowest low
    let highest_high = rolling_max(high, period)?;
    let lowest_low = rolling_min(low, period)?;

    // Calculate middle band and copy to output
    let lookback = donchian_lookback(period);
    let mut nan_high = 0usize;
    let mut nan_low = 0usize;

    for i in 0..period {
        if !high[i].is_finite() {
            nan_high += 1;
        }
        if !low[i].is_finite() {
            nan_low += 1;
        }
    }

    for i in lookback..n {
        if nan_high + nan_low > 0 {
            upper[i] = T::nan();
            lower[i] = T::nan();
            middle[i] = T::nan();
        } else {
            let hh = highest_high[i];
            let ll = lowest_low[i];

            upper[i] = hh;
            lower[i] = ll;
            middle[i] = (hh + ll) / two;
        }

        let next = i + 1;
        if next < n {
            let out_idx = next - period;
            if !high[out_idx].is_finite() {
                nan_high = nan_high.saturating_sub(1);
            }
            if !low[out_idx].is_finite() {
                nan_low = nan_low.saturating_sub(1);
            }
            if !high[next].is_finite() {
                nan_high += 1;
            }
            if !low[next].is_finite() {
                nan_low += 1;
            }
        }
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
    fn test_donchian_lookback() {
        assert_eq!(donchian_lookback(20), 19);
        assert_eq!(donchian_lookback(5), 4);
        assert_eq!(donchian_lookback(1), 0);
        assert_eq!(donchian_lookback(0), 0);
    }

    #[test]
    fn test_donchian_min_len() {
        assert_eq!(donchian_min_len(20), 20);
        assert_eq!(donchian_min_len(5), 5);
        assert_eq!(donchian_min_len(1), 1);
    }

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_donchian_basic() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.5, 13.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.5, 12.5];

        let result = donchian(&high, &low, 3).unwrap();

        assert_eq!(result.upper.len(), 8);
        assert_eq!(result.middle.len(), 8);
        assert_eq!(result.lower.len(), 8);

        // First 2 values should be NaN (period - 1 = 2)
        assert!(result.upper[0].is_nan());
        assert!(result.upper[1].is_nan());
        assert!(result.middle[0].is_nan());
        assert!(result.lower[0].is_nan());

        // Values from index 2 onwards should be valid
        for i in 2..result.upper.len() {
            assert!(
                !result.upper[i].is_nan(),
                "Upper at {} should not be NaN",
                i
            );
            assert!(
                !result.middle[i].is_nan(),
                "Middle at {} should not be NaN",
                i
            );
            assert!(
                !result.lower[i].is_nan(),
                "Lower at {} should not be NaN",
                i
            );
        }
    }

    #[test]
    fn test_donchian_f32() {
        let high = vec![10.0_f32, 11.0, 12.0, 11.5, 12.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5];

        let result = donchian(&high, &low, 3).unwrap();

        assert_eq!(result.upper.len(), 5);
        assert!(!result.upper[2].is_nan());
    }

    #[test]
    fn test_donchian_period_1() {
        let high = vec![10.0_f64, 11.0, 10.5];
        let low = vec![9.0, 10.0, 9.5];

        let result = donchian(&high, &low, 1).unwrap();

        // With period 1, lookback = 0, all values valid
        // Upper = high, Lower = low
        for i in 0..result.upper.len() {
            assert!(approx_eq(result.upper[i], high[i], EPSILON));
            assert!(approx_eq(result.lower[i], low[i], EPSILON));
            assert!(approx_eq(
                result.middle[i],
                (high[i] + low[i]) / 2.0,
                EPSILON
            ));
        }
    }

    // ==================== Known Value Tests ====================

    #[test]
    fn test_donchian_known_values() {
        let high = vec![10.0_f64, 12.0, 11.0, 13.0, 12.0];
        let low = vec![8.0, 10.0, 9.0, 11.0, 10.0];

        let result = donchian(&high, &low, 3).unwrap();

        // At index 2: window [0,1,2]
        // Upper = max(10, 12, 11) = 12
        // Lower = min(8, 10, 9) = 8
        // Middle = (12 + 8) / 2 = 10
        assert!(approx_eq(result.upper[2], 12.0, EPSILON));
        assert!(approx_eq(result.lower[2], 8.0, EPSILON));
        assert!(approx_eq(result.middle[2], 10.0, EPSILON));

        // At index 3: window [1,2,3]
        // Upper = max(12, 11, 13) = 13
        // Lower = min(10, 9, 11) = 9
        // Middle = (13 + 9) / 2 = 11
        assert!(approx_eq(result.upper[3], 13.0, EPSILON));
        assert!(approx_eq(result.lower[3], 9.0, EPSILON));
        assert!(approx_eq(result.middle[3], 11.0, EPSILON));

        // At index 4: window [2,3,4]
        // Upper = max(11, 13, 12) = 13
        // Lower = min(9, 11, 10) = 9
        // Middle = (13 + 9) / 2 = 11
        assert!(approx_eq(result.upper[4], 13.0, EPSILON));
        assert!(approx_eq(result.lower[4], 9.0, EPSILON));
        assert!(approx_eq(result.middle[4], 11.0, EPSILON));
    }

    #[test]
    fn test_donchian_bands_collapse() {
        // Edge case: all same values - bands should collapse to that value
        let high = vec![100.0_f64; 10];
        let low = vec![100.0_f64; 10];

        let result = donchian(&high, &low, 3).unwrap();

        for i in 2..result.upper.len() {
            assert!(approx_eq(result.upper[i], 100.0, EPSILON));
            assert!(approx_eq(result.middle[i], 100.0, EPSILON));
            assert!(approx_eq(result.lower[i], 100.0, EPSILON));
        }
    }

    #[test]
    fn test_donchian_monotonically_increasing() {
        // Upper tracks high, lower lags behind
        let high: Vec<f64> = (0..10).map(|i| 100.0 + (i as f64) * 2.0 + 1.0).collect();
        let low: Vec<f64> = (0..10).map(|i| 100.0 + (i as f64) * 2.0 - 1.0).collect();

        let result = donchian(&high, &low, 3).unwrap();

        // In monotonically increasing data:
        // Upper = current high
        // Lower = low from (period-1) bars ago
        for i in 2..result.upper.len() {
            assert!(
                approx_eq(result.upper[i], high[i], EPSILON),
                "Upper should track high at {}",
                i
            );
            assert!(
                approx_eq(result.lower[i], low[i - 2], EPSILON),
                "Lower should be oldest low at {}",
                i
            );
        }
    }

    // ==================== Band Relationship Tests ====================

    #[test]
    fn test_donchian_upper_gte_middle_gte_lower() {
        let high: Vec<f64> = (0..30)
            .map(|i| 100.0 + 10.0 * ((i as f64) * 0.5).sin())
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();

        let result = donchian(&high, &low, 5).unwrap();

        for i in 4..result.upper.len() {
            assert!(
                result.upper[i] >= result.middle[i],
                "Upper should be >= middle at {}: {} >= {}",
                i,
                result.upper[i],
                result.middle[i]
            );
            assert!(
                result.middle[i] >= result.lower[i],
                "Middle should be >= lower at {}: {} >= {}",
                i,
                result.middle[i],
                result.lower[i]
            );
        }
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_donchian_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];

        let result = donchian(&high, &low, 5);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_donchian_zero_period() {
        let high = vec![10.0_f64; 10];
        let low = vec![9.0_f64; 10];

        let result = donchian(&high, &low, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_donchian_insufficient_data() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];

        let result = donchian(&high, &low, 5);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_donchian_mismatched_lengths() {
        let high = vec![10.0_f64, 11.0, 12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0, 12.0]; // One less

        let result = donchian(&high, &low, 3);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_donchian_minimum_data() {
        let high = vec![10.0_f64, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];

        let result = donchian(&high, &low, 3);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.upper[2].is_nan());
    }

    // ==================== donchian_into Tests ====================

    #[test]
    fn test_donchian_into_basic() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.5, 13.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.5, 12.5];
        let mut upper = vec![0.0_f64; 8];
        let mut middle = vec![0.0_f64; 8];
        let mut lower = vec![0.0_f64; 8];

        let valid_count =
            donchian_into(&high, &low, 3, &mut upper, &mut middle, &mut lower).unwrap();

        assert_eq!(valid_count, 6); // 8 - 2 = 6 valid values

        assert!(upper[0].is_nan());
        assert!(upper[1].is_nan());
        assert!(!upper[2].is_nan());
    }

    #[test]
    fn test_donchian_into_buffer_too_small() {
        let high = vec![10.0_f64; 10];
        let low = vec![9.0_f64; 10];
        let mut upper = vec![0.0_f64; 5]; // Too short
        let mut middle = vec![0.0_f64; 10];
        let mut lower = vec![0.0_f64; 10];

        let result = donchian_into(&high, &low, 3, &mut upper, &mut middle, &mut lower);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_donchian_and_donchian_into_produce_same_result() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0, 12.5, 13.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.5, 12.5];

        let result1 = donchian(&high, &low, 3).unwrap();

        let mut upper = vec![0.0_f64; 8];
        let mut middle = vec![0.0_f64; 8];
        let mut lower = vec![0.0_f64; 8];
        donchian_into(&high, &low, 3, &mut upper, &mut middle, &mut lower).unwrap();

        for i in 0..8 {
            assert!(
                approx_eq(result1.upper[i], upper[i], EPSILON),
                "Upper mismatch at {}: {} vs {}",
                i,
                result1.upper[i],
                upper[i]
            );
            assert!(
                approx_eq(result1.middle[i], middle[i], EPSILON),
                "Middle mismatch at {}: {} vs {}",
                i,
                result1.middle[i],
                middle[i]
            );
            assert!(
                approx_eq(result1.lower[i], lower[i], EPSILON),
                "Lower mismatch at {}: {} vs {}",
                i,
                result1.lower[i],
                lower[i]
            );
        }
    }

    // ==================== NaN Handling Tests ====================

    #[test]
    fn test_donchian_with_nan_in_data() {
        let high = vec![10.0_f64, f64::NAN, 12.0, 11.5, 12.5];
        let low = vec![9.0, 10.0, 11.0, 10.5, 11.5];

        let result = donchian(&high, &low, 3).unwrap();

        // NaN may propagate through rolling calculations
        assert_eq!(result.upper.len(), 5);
    }

    // ==================== Property-Based Tests ====================

    #[test]
    fn test_donchian_output_length_equals_input_length() {
        for len in [10, 20, 50, 100] {
            for period in [3, 5, 20] {
                if period <= len {
                    let high: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) + 2.0).collect();
                    let low: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) - 2.0).collect();

                    let result = donchian(&high, &low, period).unwrap();
                    assert_eq!(result.upper.len(), len);
                    assert_eq!(result.middle.len(), len);
                    assert_eq!(result.lower.len(), len);
                }
            }
        }
    }

    #[test]
    fn test_donchian_nan_count() {
        for period in [3, 5, 20] {
            let len = 30;
            let high: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) + 2.0).collect();
            let low: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64) - 2.0).collect();

            let result = donchian(&high, &low, period).unwrap();

            let upper_nan_count = result.upper.iter().filter(|x| x.is_nan()).count();
            let expected = period - 1;
            assert_eq!(
                upper_nan_count, expected,
                "Expected {} NaN values for period {}, got {}",
                expected, period, upper_nan_count
            );
        }
    }

    #[test]
    fn test_donchian_upper_always_gte_high_in_window() {
        let high: Vec<f64> = (0..50)
            .map(|i| 100.0 + 10.0 * ((i as f64) * 0.3).sin())
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();

        let period = 5;
        let result = donchian(&high, &low, period).unwrap();

        for i in (period - 1)..high.len() {
            // Upper should be >= any high in the window
            for j in (i + 1 - period)..=i {
                assert!(
                    result.upper[i] >= high[j],
                    "Upper at {} ({}) should be >= high at {} ({})",
                    i,
                    result.upper[i],
                    j,
                    high[j]
                );
            }
        }
    }

    #[test]
    fn test_donchian_lower_always_lte_low_in_window() {
        let high: Vec<f64> = (0..50)
            .map(|i| 100.0 + 10.0 * ((i as f64) * 0.3).sin())
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();

        let period = 5;
        let result = donchian(&high, &low, period).unwrap();

        for i in (period - 1)..low.len() {
            // Lower should be <= any low in the window
            for j in (i + 1 - period)..=i {
                assert!(
                    result.lower[i] <= low[j],
                    "Lower at {} ({}) should be <= low at {} ({})",
                    i,
                    result.lower[i],
                    j,
                    low[j]
                );
            }
        }
    }

    // ==================== Real-World Scenario Tests ====================

    #[test]
    fn test_donchian_breakout_detection() {
        // When current high exceeds previous upper band, it's a breakout
        let mut high = vec![100.0_f64; 10];
        let low = vec![98.0_f64; 10];

        // Create a breakout at index 8
        high[8] = 105.0;
        high[9] = 106.0;

        let result = donchian(&high, &low, 5).unwrap();

        // Upper band should expand with the breakout
        assert!(
            result.upper[8] > result.upper[7],
            "Upper should expand on breakout"
        );
        assert!(
            result.upper[9] > result.upper[7],
            "Upper should remain elevated"
        );
    }

    #[test]
    fn test_donchian_channel_width_measures_volatility() {
        // Wider channels indicate higher volatility
        let high_volatile: Vec<f64> = (0..20)
            .map(|i| 100.0 + 10.0 * ((i as f64) * 0.5).sin())
            .collect();
        let low_volatile: Vec<f64> = high_volatile.iter().map(|h| h - 5.0).collect();

        let high_calm: Vec<f64> = (0..20)
            .map(|i| 100.0 + 2.0 * ((i as f64) * 0.5).sin())
            .collect();
        let low_calm: Vec<f64> = high_calm.iter().map(|h| h - 1.0).collect();

        let result_volatile = donchian(&high_volatile, &low_volatile, 5).unwrap();
        let result_calm = donchian(&high_calm, &low_calm, 5).unwrap();

        // Average channel width should be larger for volatile data
        let avg_width_volatile: f64 = (10..20)
            .map(|i| result_volatile.upper[i] - result_volatile.lower[i])
            .sum::<f64>()
            / 10.0;
        let avg_width_calm: f64 = (10..20)
            .map(|i| result_calm.upper[i] - result_calm.lower[i])
            .sum::<f64>()
            / 10.0;

        assert!(
            avg_width_volatile > avg_width_calm,
            "Volatile market should have wider channels: {} vs {}",
            avg_width_volatile,
            avg_width_calm
        );
    }
}
