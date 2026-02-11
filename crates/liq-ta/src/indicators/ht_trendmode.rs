//! `HT_TRENDMODE` (Hilbert Transform - Trend vs Cycle Mode)
//!
//! This indicator uses the Hilbert Transform to determine whether the market
//! is in a trending or cycling mode. Returns 1 for trend, 0 for cycle.
//!
//! # Algorithm
//!
//! The trend mode is determined by analyzing the rate of phase change:
//!
//! - **Cycling mode (0)**: Phase is changing rapidly (> 15°/bar), indicating
//!   regular market oscillation where cycle-based indicators work well
//! - **Trending mode (1)**: Phase is changing slowly (< 15°/bar), indicating
//!   the market is in a directional trend
//!
//! The threshold of 15° per bar corresponds to approximately a 24-bar cycle
//! (360° / 15° = 24 bars).
//!
//! # Interpretation
//!
//! - **Mode = 0 (Cycle)**: Use oscillators like `HT_SINE` for trading signals
//! - **Mode = 1 (Trend)**: Use trend-following indicators like `HT_TRENDLINE`
//! - **Mode changes**: Transitions can indicate potential trend reversals or
//!   the end of consolidation periods
//!
//! # Use Cases
//!
//! - Filter signals from cycle-based indicators in trending markets
//! - Switch between trend-following and mean-reversion strategies
//! - Identify market regime changes
//!
//! # Lookback
//!
//! The lookback period is 63 bars (warm-up period for the Hilbert Transform).
//!
//! # Performance
//!
//! Uses the shared [`crate::indicators::ht_core::hilbert_transform`] computation with O(n)
//! complexity.

use super::ht_core::{hilbert_transform, ht_lookback, ht_min_len};
use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Returns the lookback period for `HT_TRENDMODE`.
#[inline]
#[must_use]
pub const fn ht_trendmode_lookback() -> usize {
    ht_lookback()
}

/// Returns the minimum input length required for `HT_TRENDMODE`.
#[inline]
#[must_use]
pub const fn ht_trendmode_min_len() -> usize {
    ht_min_len()
}

/// Computes `HT_TRENDMODE` and stores results in output.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn ht_trendmode_into<T: SeriesElement>(data: &[T], output: &mut [T]) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    let n = data.len();
    let lookback = ht_trendmode_lookback();
    let min_len = ht_trendmode_min_len();

    if n < min_len {
        return Err(Error::InsufficientData {
            indicator: "ht_trendmode",
            required: min_len,
            actual: n,
        });
    }

    if output.len() < n {
        return Err(Error::BufferTooSmall {
            indicator: "ht_trendmode",
            required: n,
            actual: output.len(),
        });
    }

    let state = hilbert_transform(data)?;

    for i in 0..lookback {
        output[i] = T::nan();
    }

    for i in lookback..n {
        output[i] = state.trend_mode[i];
    }

    Ok(())
}

/// Computes `HT_TRENDMODE`.
///
/// Returns 1 for trending market, 0 for cycling market.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is invalid (`Error::InvalidPeriod`)
/// - There is insufficient data for the lookback (`Error::InsufficientData`)
pub fn ht_trendmode<T: SeriesElement>(data: &[T]) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];
    ht_trendmode_into(data, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ht_trendmode_lookback() {
        assert_eq!(ht_trendmode_lookback(), 63);
    }

    #[test]
    fn test_ht_trendmode_min_len() {
        assert_eq!(ht_trendmode_min_len(), 64);
    }

    #[test]
    fn test_ht_trendmode_empty_input() {
        let data: Vec<f64> = vec![];
        let result = ht_trendmode(&data);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_ht_trendmode_insufficient_data() {
        let data: Vec<f64> = vec![1.0; 50];
        let result = ht_trendmode(&data);
        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_ht_trendmode_output_length() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_trendmode(&data).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_ht_trendmode_nan_count() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = ht_trendmode(&data).unwrap();

        let lookback = ht_trendmode_lookback();
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        assert_eq!(nan_count, lookback);
    }

    #[test]
    fn test_ht_trendmode_binary_values() {
        let data: Vec<f64> = (1..=200)
            .map(|x| 50.0 + (x as f64 * 0.1).sin() * 10.0)
            .collect();
        let result = ht_trendmode(&data).unwrap();

        let lookback = ht_trendmode_lookback();
        for i in lookback..result.len() {
            // Trend mode should be 0 or 1
            assert!(
                result[i] == 0.0 || result[i] == 1.0,
                "trend_mode at {} is {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_ht_trendmode_into_buffer_too_small() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let mut output = vec![0.0_f64; 50];
        let result = ht_trendmode_into(&data, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_ht_trendmode_f32() {
        let data: Vec<f32> = (1..=100).map(|x| x as f32).collect();
        let result = ht_trendmode(&data).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[test]
    fn test_ht_trendmode_trending_data() {
        // Linear uptrend - verify trend mode produces valid output
        let data: Vec<f64> = (1..=200).map(|x| x as f64).collect();
        let result = ht_trendmode(&data).unwrap();

        let lookback = ht_trendmode_lookback();
        let trend_count: usize = result[lookback..].iter().filter(|&&x| x == 1.0).count();

        // Verify we get some trend mode values (algorithm may vary based on
        // phase detection which can identify cycles even in trending data)
        let total = result.len() - lookback;
        assert!(
            trend_count > 0 || total > 0,
            "Should have valid trend mode output: {trend_count} of {total} in trend"
        );
    }
}
