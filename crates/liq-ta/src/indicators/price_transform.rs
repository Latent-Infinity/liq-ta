//! Price Transform Indicators
//!
//! This module provides various price transformation functions commonly used
//! in technical analysis to compute average or representative prices from
//! OHLC (Open, High, Low, Close) data.
//!
//! # Indicators
//!
//! - [`avgprice`] - Average Price: (O + H + L + C) / 4
//! - [`medprice`] - Median Price: (H + L) / 2
//! - [`typprice`] - Typical Price: (H + L + C) / 3
//! - [`wclprice`] - Weighted Close Price: (H + L + 2*C) / 4
//!
//! All indicators have O(n) time complexity and produce a single output array
//! of the same length as the input.

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

// =============================================================================
// AVGPRICE (Average Price)
// =============================================================================

/// Returns the lookback period for AVGPRICE.
/// AVGPRICE has no lookback - first valid output is at index 0.
#[inline]
#[must_use]
pub const fn avgprice_lookback() -> usize {
    0
}

/// Returns the minimum input length required for AVGPRICE.
#[inline]
#[must_use]
pub const fn avgprice_min_len() -> usize {
    1
}

/// Computes AVGPRICE and stores results in output buffer.
///
/// AVGPRICE = (Open + High + Low + Close) / 4
///
/// # Arguments
///
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `output` - Pre-allocated output buffer
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn avgprice_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    output: &mut [T],
) -> Result<()> {
    let n = open.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }
    // Validate all arrays have same length
    if high.len() != n || low.len() != n || close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "OHLC arrays must have same length: open={}, high={}, low={}, close={}",
                n,
                high.len(),
                low.len(),
                close.len()
            ),
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "avgprice",
            required: n,
            actual: output.len(),
        });
    }

    let four = T::from_i32(4)?;

    // Calculate AVGPRICE for each bar
    for i in 0..n {
        if is_invalid(open[i]) || is_invalid(high[i]) || is_invalid(low[i]) || is_invalid(close[i])
        {
            output[i] = T::nan();
        } else {
            output[i] = (open[i] + high[i] + low[i] + close[i]) / four;
        }
    }

    Ok(())
}

/// Computes AVGPRICE (Average Price).
///
/// AVGPRICE = (Open + High + Low + Close) / 4
///
/// # Arguments
///
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
///
/// # Example
///
/// ```
/// use liq_ta::indicators::avgprice;
///
/// let open = vec![10.0_f64, 11.0, 12.0];
/// let high = vec![12.0, 13.0, 14.0];
/// let low = vec![9.0, 10.0, 11.0];
/// let close = vec![11.0, 12.0, 13.0];
/// let result = avgprice(&open, &high, &low, &close).unwrap();
/// // result[0] = (10 + 12 + 9 + 11) / 4 = 10.5
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn avgprice<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<T>> {
    let n = open.len();
    let mut output = vec![T::nan(); n];
    avgprice_into(open, high, low, close, &mut output)?;
    Ok(output)
}

// =============================================================================
// MEDPRICE (Median Price)
// =============================================================================

/// Returns the lookback period for MEDPRICE.
#[inline]
#[must_use]
pub const fn medprice_lookback() -> usize {
    0
}

/// Returns the minimum input length required for MEDPRICE.
#[inline]
#[must_use]
pub const fn medprice_min_len() -> usize {
    1
}

/// Computes MEDPRICE and stores results in output buffer.
///
/// MEDPRICE = (High + Low) / 2
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn medprice_into<T: SeriesElement>(high: &[T], low: &[T], output: &mut [T]) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    // Validate all arrays have same length
    if low.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "HL arrays must have same length: high={}, low={}",
                n,
                low.len()
            ),
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "medprice",
            required: n,
            actual: output.len(),
        });
    }

    let two = T::from_i32(2)?;

    // Calculate MEDPRICE for each bar
    for i in 0..n {
        if is_invalid(high[i]) || is_invalid(low[i]) {
            output[i] = T::nan();
        } else {
            output[i] = (high[i] + low[i]) / two;
        }
    }

    Ok(())
}

/// Computes MEDPRICE (Median Price).
///
/// MEDPRICE = (High + Low) / 2
///
/// # Example
///
/// ```
/// use liq_ta::indicators::medprice;
///
/// let high = vec![12.0_f64, 13.0, 14.0];
/// let low = vec![8.0, 9.0, 10.0];
/// let result = medprice(&high, &low).unwrap();
/// // result[0] = (12 + 8) / 2 = 10.0
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn medprice<T: SeriesElement>(high: &[T], low: &[T]) -> Result<Vec<T>> {
    let n = high.len();
    let mut output = vec![T::nan(); n];
    medprice_into(high, low, &mut output)?;
    Ok(output)
}

// =============================================================================
// TYPPRICE (Typical Price)
// =============================================================================

/// Returns the lookback period for TYPPRICE.
#[inline]
#[must_use]
pub const fn typprice_lookback() -> usize {
    0
}

/// Returns the minimum input length required for TYPPRICE.
#[inline]
#[must_use]
pub const fn typprice_min_len() -> usize {
    1
}

/// Computes TYPPRICE and stores results in output buffer.
///
/// TYPPRICE = (High + Low + Close) / 3
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn typprice_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    // Validate all arrays have same length
    if low.len() != n || close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "HLC arrays must have same length: high={}, low={}, close={}",
                n,
                low.len(),
                close.len()
            ),
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "typprice",
            required: n,
            actual: output.len(),
        });
    }

    let three = T::from_i32(3)?;

    // Calculate TYPPRICE for each bar
    for i in 0..n {
        if is_invalid(high[i]) || is_invalid(low[i]) || is_invalid(close[i]) {
            output[i] = T::nan();
        } else {
            output[i] = (high[i] + low[i] + close[i]) / three;
        }
    }

    Ok(())
}

/// Computes TYPPRICE (Typical Price).
///
/// TYPPRICE = (High + Low + Close) / 3
///
/// # Example
///
/// ```
/// use liq_ta::indicators::typprice;
///
/// let high = vec![12.0_f64, 13.0, 14.0];
/// let low = vec![8.0, 9.0, 10.0];
/// let close = vec![10.0, 11.0, 12.0];
/// let result = typprice(&high, &low, &close).unwrap();
/// // result[0] = (12 + 8 + 10) / 3 = 10.0
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn typprice<T: SeriesElement>(high: &[T], low: &[T], close: &[T]) -> Result<Vec<T>> {
    let n = high.len();
    let mut output = vec![T::nan(); n];
    typprice_into(high, low, close, &mut output)?;
    Ok(output)
}

// =============================================================================
// WCLPRICE (Weighted Close Price)
// =============================================================================

/// Returns the lookback period for WCLPRICE.
#[inline]
#[must_use]
pub const fn wclprice_lookback() -> usize {
    0
}

/// Returns the minimum input length required for WCLPRICE.
#[inline]
#[must_use]
pub const fn wclprice_min_len() -> usize {
    1
}

/// Computes WCLPRICE and stores results in output buffer.
///
/// WCLPRICE = (High + Low + 2*Close) / 4
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn wclprice_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    close: &[T],
    output: &mut [T],
) -> Result<()> {
    let n = high.len();

    if n == 0 {
        return Err(Error::EmptyInput);
    }

    // Validate all arrays have same length
    if low.len() != n || close.len() != n {
        return Err(Error::LengthMismatch {
            description: format!(
                "HLC arrays must have same length: high={}, low={}, close={}",
                n,
                low.len(),
                close.len()
            ),
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "wclprice",
            required: n,
            actual: output.len(),
        });
    }

    let two = T::from_i32(2)?;
    let four = T::from_i32(4)?;

    // Calculate WCLPRICE for each bar
    for i in 0..n {
        if is_invalid(high[i]) || is_invalid(low[i]) || is_invalid(close[i]) {
            output[i] = T::nan();
        } else {
            output[i] = (high[i] + low[i] + close[i] * two) / four;
        }
    }

    Ok(())
}

/// Computes WCLPRICE (Weighted Close Price).
///
/// WCLPRICE = (High + Low + 2*Close) / 4
///
/// # Example
///
/// ```
/// use liq_ta::indicators::wclprice;
///
/// let high = vec![12.0_f64, 13.0, 14.0];
/// let low = vec![8.0, 9.0, 10.0];
/// let close = vec![11.0, 12.0, 13.0];
/// let result = wclprice(&high, &low, &close).unwrap();
/// // result[0] = (12 + 8 + 2*11) / 4 = 10.5
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The input arrays have different lengths (`Error::LengthMismatch`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn wclprice<T: SeriesElement>(high: &[T], low: &[T], close: &[T]) -> Result<Vec<T>> {
    let n = high.len();
    let mut output = vec![T::nan(); n];
    wclprice_into(high, low, close, &mut output)?;
    Ok(output)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    // ==================== AVGPRICE Tests ====================

    #[test]
    fn test_avgprice_lookback() {
        assert_eq!(avgprice_lookback(), 0);
    }

    #[test]
    fn test_avgprice_min_len() {
        assert_eq!(avgprice_min_len(), 1);
    }

    #[test]
    fn test_avgprice_basic() {
        let open = vec![10.0_f64, 11.0, 12.0];
        let high = vec![12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![11.0, 12.0, 13.0];
        let result = avgprice(&open, &high, &low, &close).unwrap();

        // (10+12+9+11)/4 = 42/4 = 10.5
        assert!(approx_eq(result[0], 10.5));
        // (11+13+10+12)/4 = 46/4 = 11.5
        assert!(approx_eq(result[1], 11.5));
        // (12+14+11+13)/4 = 50/4 = 12.5
        assert!(approx_eq(result[2], 12.5));
    }

    #[test]
    fn test_avgprice_empty_input() {
        let open: Vec<f64> = vec![];
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];
        let result = avgprice(&open, &high, &low, &close);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_avgprice_length_mismatch() {
        let open = vec![10.0_f64, 11.0, 12.0];
        let high = vec![12.0, 13.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![11.0, 12.0, 13.0];
        let result = avgprice(&open, &high, &low, &close);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_avgprice_buffer_too_small() {
        let open = vec![10.0_f64, 11.0, 12.0];
        let high = vec![12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![11.0, 12.0, 13.0];
        let mut output = vec![0.0_f64; 2];
        let result = avgprice_into(&open, &high, &low, &close, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_avgprice_f32() {
        let open = vec![10.0_f32, 11.0, 12.0];
        let high = vec![12.0_f32, 13.0, 14.0];
        let low = vec![9.0_f32, 10.0, 11.0];
        let close = vec![11.0_f32, 12.0, 13.0];
        let result = avgprice(&open, &high, &low, &close).unwrap();
        assert!((result[0] - 10.5_f32).abs() < 1e-5);
    }

    // ==================== MEDPRICE Tests ====================

    #[test]
    fn test_medprice_lookback() {
        assert_eq!(medprice_lookback(), 0);
    }

    #[test]
    fn test_medprice_min_len() {
        assert_eq!(medprice_min_len(), 1);
    }

    #[test]
    fn test_medprice_basic() {
        let high = vec![12.0_f64, 13.0, 14.0];
        let low = vec![8.0, 9.0, 10.0];
        let result = medprice(&high, &low).unwrap();

        // (12+8)/2 = 10.0
        assert!(approx_eq(result[0], 10.0));
        // (13+9)/2 = 11.0
        assert!(approx_eq(result[1], 11.0));
        // (14+10)/2 = 12.0
        assert!(approx_eq(result[2], 12.0));
    }

    #[test]
    fn test_medprice_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let result = medprice(&high, &low);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_medprice_length_mismatch() {
        let high = vec![12.0_f64, 13.0, 14.0];
        let low = vec![8.0, 9.0];
        let result = medprice(&high, &low);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_medprice_buffer_too_small() {
        let high = vec![12.0_f64, 13.0, 14.0];
        let low = vec![8.0, 9.0, 10.0];
        let mut output = vec![0.0_f64; 2];
        let result = medprice_into(&high, &low, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_medprice_f32() {
        let high = vec![12.0_f32, 13.0, 14.0];
        let low = vec![8.0_f32, 9.0, 10.0];
        let result = medprice(&high, &low).unwrap();
        assert!((result[0] - 10.0_f32).abs() < 1e-5);
    }

    // ==================== TYPPRICE Tests ====================

    #[test]
    fn test_typprice_lookback() {
        assert_eq!(typprice_lookback(), 0);
    }

    #[test]
    fn test_typprice_min_len() {
        assert_eq!(typprice_min_len(), 1);
    }

    #[test]
    fn test_typprice_basic() {
        let high = vec![12.0_f64, 13.0, 14.0];
        let low = vec![8.0, 9.0, 10.0];
        let close = vec![10.0, 11.0, 12.0];
        let result = typprice(&high, &low, &close).unwrap();

        // (12+8+10)/3 = 30/3 = 10.0
        assert!(approx_eq(result[0], 10.0));
        // (13+9+11)/3 = 33/3 = 11.0
        assert!(approx_eq(result[1], 11.0));
        // (14+10+12)/3 = 36/3 = 12.0
        assert!(approx_eq(result[2], 12.0));
    }

    #[test]
    fn test_typprice_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];
        let result = typprice(&high, &low, &close);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_typprice_length_mismatch() {
        let high = vec![12.0_f64, 13.0, 14.0];
        let low = vec![8.0, 9.0];
        let close = vec![10.0, 11.0, 12.0];
        let result = typprice(&high, &low, &close);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_typprice_buffer_too_small() {
        let high = vec![12.0_f64, 13.0, 14.0];
        let low = vec![8.0, 9.0, 10.0];
        let close = vec![10.0, 11.0, 12.0];
        let mut output = vec![0.0_f64; 2];
        let result = typprice_into(&high, &low, &close, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_typprice_f32() {
        let high = vec![12.0_f32, 13.0, 14.0];
        let low = vec![8.0_f32, 9.0, 10.0];
        let close = vec![10.0_f32, 11.0, 12.0];
        let result = typprice(&high, &low, &close).unwrap();
        assert!((result[0] - 10.0_f32).abs() < 1e-5);
    }

    // ==================== WCLPRICE Tests ====================

    #[test]
    fn test_wclprice_lookback() {
        assert_eq!(wclprice_lookback(), 0);
    }

    #[test]
    fn test_wclprice_min_len() {
        assert_eq!(wclprice_min_len(), 1);
    }

    #[test]
    fn test_wclprice_basic() {
        let high = vec![12.0_f64, 13.0, 14.0];
        let low = vec![8.0, 9.0, 10.0];
        let close = vec![11.0, 12.0, 13.0];
        let result = wclprice(&high, &low, &close).unwrap();

        // (12+8+2*11)/4 = (12+8+22)/4 = 42/4 = 10.5
        assert!(approx_eq(result[0], 10.5));
        // (13+9+2*12)/4 = (13+9+24)/4 = 46/4 = 11.5
        assert!(approx_eq(result[1], 11.5));
        // (14+10+2*13)/4 = (14+10+26)/4 = 50/4 = 12.5
        assert!(approx_eq(result[2], 12.5));
    }

    #[test]
    fn test_wclprice_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];
        let result = wclprice(&high, &low, &close);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_wclprice_length_mismatch() {
        let high = vec![12.0_f64, 13.0, 14.0];
        let low = vec![8.0, 9.0];
        let close = vec![11.0, 12.0, 13.0];
        let result = wclprice(&high, &low, &close);
        assert!(matches!(result, Err(Error::LengthMismatch { .. })));
    }

    #[test]
    fn test_wclprice_buffer_too_small() {
        let high = vec![12.0_f64, 13.0, 14.0];
        let low = vec![8.0, 9.0, 10.0];
        let close = vec![11.0, 12.0, 13.0];
        let mut output = vec![0.0_f64; 2];
        let result = wclprice_into(&high, &low, &close, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_wclprice_f32() {
        let high = vec![12.0_f32, 13.0, 14.0];
        let low = vec![8.0_f32, 9.0, 10.0];
        let close = vec![11.0_f32, 12.0, 13.0];
        let result = wclprice(&high, &low, &close).unwrap();
        assert!((result[0] - 10.5_f32).abs() < 1e-5);
    }
}
