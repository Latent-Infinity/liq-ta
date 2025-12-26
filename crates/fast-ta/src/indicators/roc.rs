//! Rate of Change (ROC) family of indicators.
//!
//! This module provides several rate of change calculations:
//! - ROC: Rate of Change (percentage scaled by 100)
//! - ROCP: Rate of Change Percentage (decimal form)
//! - ROCR: Rate of Change Ratio
//! - ROCR100: Rate of Change Ratio * 100
//!
//! # Formulas
//!
//! ```text
//! ROC = ((price - price\[n\]) / price\[n\]) * 100
//! ROCP = (price - price\[n\]) / price\[n\]
//! ROCR = price / price\[n\]
//! ROCR100 = (price / price\[n\]) * 100
//! ```
//!
//! # Lookback
//!
//! All functions have a lookback period equal to the period parameter.

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

// =============================================================================
// ROC - Rate of Change (percentage * 100)
// =============================================================================

/// Computes the lookback period for ROC.
#[inline]
#[must_use]
pub const fn roc_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for ROC calculation.
#[inline]
#[must_use]
pub const fn roc_min_len(period: usize) -> usize {
    period + 1
}

/// Computes ROC (Rate of Change) and stores results in output.
///
/// ROC = ((price - price\[n\]) / price\[n\]) * 100
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - Lookback period
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn roc_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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
    let min_len = roc_min_len(period);

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "roc",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "roc",
            required: n,
            actual: output.len(),
        });
    }

    let hundred = T::from_f64(100.0)?;
    let lookback = roc_lookback(period);

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate ROC: ((price - price\[n\]) / price\[n\]) * 100
    for i in lookback..n {
        let prev = data[i - period];
        let cur = data[i];
        if is_invalid(prev) || is_invalid(cur) || prev == T::zero() {
            output[i] = T::nan();
        } else {
            output[i] = ((cur - prev) / prev) * hundred;
        }
    }

    Ok(())
}

/// Computes ROC (Rate of Change).
///
/// ROC = ((price - price\[n\]) / price\[n\]) * 100
///
/// # Example
///
/// ```
/// use fast_ta::indicators::roc;
///
/// let prices = vec![100.0_f64, 102.0, 104.0, 103.0, 105.0];
/// let result = roc(&prices, 2).unwrap();
/// // ROC[2] = ((104 - 100) / 100) * 100 = 4.0
/// assert!((result[2] - 4.0).abs() < 1e-10);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn roc<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    roc_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// ROCP - Rate of Change Percentage (decimal form)
// =============================================================================

/// Computes the lookback period for ROCP.
#[inline]
#[must_use]
pub const fn rocp_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for ROCP calculation.
#[inline]
#[must_use]
pub const fn rocp_min_len(period: usize) -> usize {
    period + 1
}

/// Computes ROCP (Rate of Change Percentage) and stores results in output.
///
/// ROCP = (price - price\[n\]) / price\[n\]
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - Lookback period
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn rocp_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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
    let min_len = rocp_min_len(period);

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "rocp",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "rocp",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = rocp_lookback(period);

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate ROCP: (price - price\[n\]) / price\[n\]
    for i in lookback..n {
        let prev = data[i - period];
        let cur = data[i];
        if is_invalid(prev) || is_invalid(cur) || prev == T::zero() {
            output[i] = T::nan();
        } else {
            output[i] = (cur - prev) / prev;
        }
    }

    Ok(())
}

/// Computes ROCP (Rate of Change Percentage).
///
/// ROCP = (price - price\[n\]) / price\[n\]
///
/// # Example
///
/// ```
/// use fast_ta::indicators::rocp;
///
/// let prices = vec![100.0_f64, 102.0, 104.0, 103.0, 105.0];
/// let result = rocp(&prices, 2).unwrap();
/// // ROCP[2] = (104 - 100) / 100 = 0.04
/// assert!((result[2] - 0.04).abs() < 1e-10);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn rocp<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    rocp_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// ROCR - Rate of Change Ratio
// =============================================================================

/// Computes the lookback period for ROCR.
#[inline]
#[must_use]
pub const fn rocr_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for ROCR calculation.
#[inline]
#[must_use]
pub const fn rocr_min_len(period: usize) -> usize {
    period + 1
}

/// Computes ROCR (Rate of Change Ratio) and stores results in output.
///
/// ROCR = price / price\[n\]
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - Lookback period
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn rocr_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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
    let min_len = rocr_min_len(period);

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "rocr",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "rocr",
            required: n,
            actual: output.len(),
        });
    }

    let lookback = rocr_lookback(period);

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate ROCR: price / price\[n\]
    for i in lookback..n {
        let prev = data[i - period];
        let cur = data[i];
        if is_invalid(prev) || is_invalid(cur) || prev == T::zero() {
            output[i] = T::nan();
        } else {
            output[i] = cur / prev;
        }
    }

    Ok(())
}

/// Computes ROCR (Rate of Change Ratio).
///
/// ROCR = price / price\[n\]
///
/// # Example
///
/// ```
/// use fast_ta::indicators::rocr;
///
/// let prices = vec![100.0_f64, 102.0, 104.0, 103.0, 105.0];
/// let result = rocr(&prices, 2).unwrap();
/// // ROCR[2] = 104 / 100 = 1.04
/// assert!((result[2] - 1.04).abs() < 1e-10);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn rocr<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    rocr_into(data, period, &mut output)?;
    Ok(output)
}

// =============================================================================
// ROCR100 - Rate of Change Ratio * 100
// =============================================================================

/// Computes the lookback period for ROCR100.
#[inline]
#[must_use]
pub const fn rocr100_lookback(period: usize) -> usize {
    period
}

/// Returns the minimum input length required for ROCR100 calculation.
#[inline]
#[must_use]
pub const fn rocr100_min_len(period: usize) -> usize {
    period + 1
}

/// Computes ROCR100 (Rate of Change Ratio * 100) and stores results in output.
///
/// ROCR100 = (price / price\[n\]) * 100
///
/// # Arguments
///
/// * `data` - Input price data
/// * `period` - Lookback period
/// * `output` - Pre-allocated output slice
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn rocr100_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<()> {
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
    let min_len = rocr100_min_len(period);

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "rocr100",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "rocr100",
            required: n,
            actual: output.len(),
        });
    }

    let hundred = T::from_f64(100.0)?;
    let lookback = rocr100_lookback(period);

    // Fill lookback period with NaN
    for i in 0..lookback {
        output[i] = T::nan();
    }

    // Calculate ROCR100: (price / price\[n\]) * 100
    for i in lookback..n {
        let prev = data[i - period];
        let cur = data[i];
        if is_invalid(prev) || is_invalid(cur) || prev == T::zero() {
            output[i] = T::nan();
        } else {
            output[i] = (cur / prev) * hundred;
        }
    }

    Ok(())
}

/// Computes ROCR100 (Rate of Change Ratio * 100).
///
/// ROCR100 = (price / price\[n\]) * 100
///
/// # Example
///
/// ```
/// use fast_ta::indicators::rocr100;
///
/// let prices = vec![100.0_f64, 102.0, 104.0, 103.0, 105.0];
/// let result = rocr100(&prices, 2).unwrap();
/// // ROCR100[2] = (104 / 100) * 100 = 104.0
/// assert!((result[2] - 104.0).abs() < 1e-10);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn rocr100<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    rocr100_into(data, period, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    // =========================================================================
    // ROC Tests
    // =========================================================================

    #[test]
    fn test_roc_lookback() {
        assert_eq!(roc_lookback(1), 1);
        assert_eq!(roc_lookback(10), 10);
    }

    #[test]
    fn test_roc_min_len() {
        assert_eq!(roc_min_len(1), 2);
        assert_eq!(roc_min_len(10), 11);
    }

    #[test]
    fn test_roc_empty_input() {
        let data: Vec<f64> = vec![];
        let result = roc(&data, 10);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_roc_invalid_period() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = roc(&data, 0);
        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_roc_insufficient_data() {
        let data = vec![1.0, 2.0, 3.0];
        let result = roc(&data, 10);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_roc_basic_calculation() {
        let prices: Vec<f64> = vec![100.0, 102.0, 104.0, 103.0, 105.0];
        let result = roc(&prices, 2).unwrap();

        // First 2 values should be NaN
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());

        // ROC[2] = ((104 - 100) / 100) * 100 = 4.0
        assert!((result[2] - 4.0).abs() < 1e-10);
        // ROC[3] = ((103 - 102) / 102) * 100 ≈ 0.98039
        assert!((result[3] - (1.0 / 102.0 * 100.0)).abs() < 1e-10);
        // ROC[4] = ((105 - 104) / 104) * 100 ≈ 0.96154
        assert!((result[4] - (1.0 / 104.0 * 100.0)).abs() < 1e-10);
    }

    #[test]
    fn test_roc_constant_price() {
        let prices: Vec<f64> = vec![50.0; 10];
        let result = roc(&prices, 3).unwrap();

        // ROC should be 0 for constant price
        for i in 3..result.len() {
            assert!((result[i] - 0.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_roc_doubling() {
        let prices: Vec<f64> = vec![50.0, 60.0, 100.0, 80.0];
        let result = roc(&prices, 2).unwrap();

        // ROC[2] = ((100 - 50) / 50) * 100 = 100.0 (doubled)
        assert!((result[2] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_roc_halving() {
        let prices: Vec<f64> = vec![100.0, 80.0, 50.0, 60.0];
        let result = roc(&prices, 2).unwrap();

        // ROC[2] = ((50 - 100) / 100) * 100 = -50.0 (halved)
        assert!((result[2] - (-50.0)).abs() < 1e-10);
    }

    #[test]
    fn test_roc_division_by_zero() {
        let prices: Vec<f64> = vec![0.0, 10.0, 20.0];
        let result = roc(&prices, 2).unwrap();

        // ROC[2] = (20 - 0) / 0 = NaN
        assert!(result[2].is_nan());
    }

    // =========================================================================
    // ROCP Tests
    // =========================================================================

    #[test]
    fn test_rocp_lookback() {
        assert_eq!(rocp_lookback(1), 1);
        assert_eq!(rocp_lookback(10), 10);
    }

    #[test]
    fn test_rocp_basic_calculation() {
        let prices: Vec<f64> = vec![100.0, 102.0, 104.0, 103.0, 105.0];
        let result = rocp(&prices, 2).unwrap();

        // ROCP[2] = (104 - 100) / 100 = 0.04
        assert!((result[2] - 0.04).abs() < 1e-10);
    }

    #[test]
    fn test_rocp_constant_price() {
        let prices: Vec<f64> = vec![50.0; 10];
        let result = rocp(&prices, 3).unwrap();

        // ROCP should be 0 for constant price
        for i in 3..result.len() {
            assert!((result[i] - 0.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_rocp_doubling() {
        let prices: Vec<f64> = vec![50.0, 60.0, 100.0, 80.0];
        let result = rocp(&prices, 2).unwrap();

        // ROCP[2] = (100 - 50) / 50 = 1.0 (100% increase)
        assert!((result[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_rocp_halving() {
        let prices: Vec<f64> = vec![100.0, 80.0, 50.0, 60.0];
        let result = rocp(&prices, 2).unwrap();

        // ROCP[2] = (50 - 100) / 100 = -0.5 (50% decrease)
        assert!((result[2] - (-0.5)).abs() < 1e-10);
    }

    // =========================================================================
    // ROCR Tests
    // =========================================================================

    #[test]
    fn test_rocr_lookback() {
        assert_eq!(rocr_lookback(1), 1);
        assert_eq!(rocr_lookback(10), 10);
    }

    #[test]
    fn test_rocr_basic_calculation() {
        let prices: Vec<f64> = vec![100.0, 102.0, 104.0, 103.0, 105.0];
        let result = rocr(&prices, 2).unwrap();

        // ROCR[2] = 104 / 100 = 1.04
        assert!((result[2] - 1.04).abs() < 1e-10);
    }

    #[test]
    fn test_rocr_constant_price() {
        let prices: Vec<f64> = vec![50.0; 10];
        let result = rocr(&prices, 3).unwrap();

        // ROCR should be 1.0 for constant price
        for i in 3..result.len() {
            assert!((result[i] - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_rocr_doubling() {
        let prices: Vec<f64> = vec![50.0, 60.0, 100.0, 80.0];
        let result = rocr(&prices, 2).unwrap();

        // ROCR[2] = 100 / 50 = 2.0 (doubled)
        assert!((result[2] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_rocr_halving() {
        let prices: Vec<f64> = vec![100.0, 80.0, 50.0, 60.0];
        let result = rocr(&prices, 2).unwrap();

        // ROCR[2] = 50 / 100 = 0.5 (halved)
        assert!((result[2] - 0.5).abs() < 1e-10);
    }

    // =========================================================================
    // ROCR100 Tests
    // =========================================================================

    #[test]
    fn test_rocr100_lookback() {
        assert_eq!(rocr100_lookback(1), 1);
        assert_eq!(rocr100_lookback(10), 10);
    }

    #[test]
    fn test_rocr100_basic_calculation() {
        let prices: Vec<f64> = vec![100.0, 102.0, 104.0, 103.0, 105.0];
        let result = rocr100(&prices, 2).unwrap();

        // ROCR100[2] = (104 / 100) * 100 = 104.0
        assert!((result[2] - 104.0).abs() < 1e-10);
    }

    #[test]
    fn test_rocr100_constant_price() {
        let prices: Vec<f64> = vec![50.0; 10];
        let result = rocr100(&prices, 3).unwrap();

        // ROCR100 should be 100.0 for constant price
        for i in 3..result.len() {
            assert!((result[i] - 100.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_rocr100_doubling() {
        let prices: Vec<f64> = vec![50.0, 60.0, 100.0, 80.0];
        let result = rocr100(&prices, 2).unwrap();

        // ROCR100[2] = (100 / 50) * 100 = 200.0 (doubled)
        assert!((result[2] - 200.0).abs() < 1e-10);
    }

    #[test]
    fn test_rocr100_halving() {
        let prices: Vec<f64> = vec![100.0, 80.0, 50.0, 60.0];
        let result = rocr100(&prices, 2).unwrap();

        // ROCR100[2] = (50 / 100) * 100 = 50.0 (halved)
        assert!((result[2] - 50.0).abs() < 1e-10);
    }

    // =========================================================================
    // Relationship Tests
    // =========================================================================

    #[test]
    fn test_roc_rocp_relationship() {
        let prices: Vec<f64> = vec![100.0, 102.0, 104.0, 103.0, 105.0];
        let roc_result = roc(&prices, 2).unwrap();
        let rocp_result = rocp(&prices, 2).unwrap();

        // ROC = ROCP * 100
        for i in 2..prices.len() {
            assert!((roc_result[i] - rocp_result[i] * 100.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_rocr_rocr100_relationship() {
        let prices: Vec<f64> = vec![100.0, 102.0, 104.0, 103.0, 105.0];
        let rocr_result = rocr(&prices, 2).unwrap();
        let rocr100_result = rocr100(&prices, 2).unwrap();

        // ROCR100 = ROCR * 100
        for i in 2..prices.len() {
            assert!((rocr100_result[i] - rocr_result[i] * 100.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_rocp_rocr_relationship() {
        let prices: Vec<f64> = vec![100.0, 102.0, 104.0, 103.0, 105.0];
        let rocp_result = rocp(&prices, 2).unwrap();
        let rocr_result = rocr(&prices, 2).unwrap();

        // ROCR = ROCP + 1
        for i in 2..prices.len() {
            assert!((rocr_result[i] - (rocp_result[i] + 1.0)).abs() < 1e-10);
        }
    }

    // =========================================================================
    // Into variant tests
    // =========================================================================

    #[test]
    fn test_roc_into() {
        let data: Vec<f64> = vec![100.0, 110.0, 121.0, 133.1, 146.41];
        let mut output = vec![0.0_f64; data.len()];
        roc_into(&data, 2, &mut output).unwrap();

        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        // ROC[2] = ((121 - 100) / 100) * 100 = 21.0
        assert!((output[2] - 21.0).abs() < 1e-10);
    }

    #[test]
    fn test_roc_into_buffer_too_small() {
        let data: Vec<f64> = vec![100.0; 10];
        let mut output = vec![0.0_f64; 5]; // Too small
        let result = roc_into(&data, 2, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_roc_f32() {
        let prices: Vec<f32> = vec![100.0, 102.0, 104.0, 103.0, 105.0];
        let result = roc(&prices, 2).unwrap();
        assert!((result[2] - 4.0_f32).abs() < 1e-5);
    }
}
