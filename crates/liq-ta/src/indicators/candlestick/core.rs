//! Core utilities for candlestick pattern recognition.
//!
//! This module provides helper functions and constants used across all
//! candlestick pattern implementations.

use crate::traits::SeriesElement;
use num_traits::NumCast;

/// Helper to convert f64 to T (infallible for valid float values).
/// This is used throughout candlestick patterns for numeric conversions.
#[inline]
pub fn f64_to_t<T: SeriesElement>(val: f64) -> T {
    <T as NumCast>::from(val).unwrap_or_else(T::nan)
}

/// Helper to convert usize to T (infallible for small values).
#[inline]
pub fn usize_to_t<T: SeriesElement>(val: usize) -> T {
    <T as NumCast>::from(val).unwrap_or_else(T::nan)
}

/// Default lookback period for trend detection and averaging.
pub const TREND_LOOKBACK: usize = 10;

/// Default lookback period for average calculations in pattern detection.
/// This is typically used for calculating average body size, shadow length, etc.
pub const AVG_LOOKBACK: usize = 10;

/// TA-Lib style setting ranges for pattern detection.
/// These control sensitivity thresholds for various pattern components.
#[derive(Debug, Clone, Copy)]
pub struct CandleSettings {
    /// Near threshold for body comparison (default: 5% of average range)
    pub body_near: f64,
    /// Long body threshold (default: 100% of average body)
    pub body_long: f64,
    /// Very long body threshold (default: 300% of average body)
    pub body_very_long: f64,
    /// Short body threshold (default: 100% of average body)
    pub body_short: f64,
    /// Doji body threshold (default: 10% of average range)
    pub body_doji: f64,
    /// Long shadow threshold (default: 100% of average body)
    pub shadow_long: f64,
    /// Very long shadow threshold (default: 200% of average body)
    pub shadow_very_long: f64,
    /// Short shadow threshold (default: 100% of average shadow)
    pub shadow_short: f64,
    /// Very short shadow threshold (default: 10% of range)
    pub shadow_very_short: f64,
    /// Near threshold for price comparison (default: 20% of average range)
    pub near: f64,
    /// Far threshold for gap detection (default: 60% of average range)
    pub far: f64,
    /// Equal threshold for exact comparison (default: 5% of average range)
    pub equal: f64,
}

impl Default for CandleSettings {
    fn default() -> Self {
        Self {
            body_near: 0.05,
            body_long: 1.0,
            body_very_long: 3.0,
            body_short: 1.0,
            body_doji: 0.1,
            shadow_long: 1.0,
            shadow_very_long: 2.0,
            shadow_short: 1.0,
            shadow_very_short: 0.1,
            near: 0.2,
            far: 0.6,
            equal: 0.05,
        }
    }
}

/// Calculate the real body of a candlestick (absolute difference between open and close).
#[inline]
pub fn real_body<T: SeriesElement>(open: T, close: T) -> T {
    (close - open).abs()
}

/// Calculate the upper shadow (wick above the body).
#[inline]
pub fn upper_shadow<T: SeriesElement>(open: T, high: T, close: T) -> T {
    high - open.max(close)
}

/// Calculate the lower shadow (wick below the body).
#[inline]
pub fn lower_shadow<T: SeriesElement>(open: T, low: T, close: T) -> T {
    open.min(close) - low
}

/// Calculate the high-low range of a candlestick.
#[inline]
pub fn candle_range<T: SeriesElement>(high: T, low: T) -> T {
    high - low
}

/// Check if a candle is bullish (close > open).
#[inline]
pub fn is_bullish<T: SeriesElement>(open: T, close: T) -> bool {
    close > open
}

/// Check if a candle is bearish (close < open).
#[inline]
pub fn is_bearish<T: SeriesElement>(open: T, close: T) -> bool {
    close < open
}

/// Calculate the body midpoint.
#[inline]
pub fn body_midpoint<T: SeriesElement>(open: T, close: T) -> T {
    (open + close) / f64_to_t(2.0)
}

/// Calculate the body top (higher of open/close).
#[inline]
pub fn body_top<T: SeriesElement>(open: T, close: T) -> T {
    open.max(close)
}

/// Calculate the body bottom (lower of open/close).
#[inline]
pub fn body_bottom<T: SeriesElement>(open: T, close: T) -> T {
    open.min(close)
}

/// Check if there's an upward gap between two candles.
#[inline]
pub fn gap_up<T: SeriesElement>(prev_high: T, curr_low: T) -> bool {
    curr_low > prev_high
}

/// Check if there's a downward gap between two candles.
#[inline]
pub fn gap_down<T: SeriesElement>(prev_low: T, curr_high: T) -> bool {
    curr_high < prev_low
}

/// Check if there's a real body gap up (gap between bodies, not wicks).
#[inline]
pub fn real_body_gap_up<T: SeriesElement>(
    prev_open: T,
    prev_close: T,
    curr_open: T,
    curr_close: T,
) -> bool {
    body_bottom(curr_open, curr_close) > body_top(prev_open, prev_close)
}

/// Check if there's a real body gap down (gap between bodies, not wicks).
#[inline]
pub fn real_body_gap_down<T: SeriesElement>(
    prev_open: T,
    prev_close: T,
    curr_open: T,
    curr_close: T,
) -> bool {
    body_top(curr_open, curr_close) < body_bottom(prev_open, prev_close)
}

/// Calculate the average body size over a lookback period.
pub fn average_body<T: SeriesElement>(
    open: &[T],
    close: &[T],
    end_idx: usize,
    lookback: usize,
) -> T {
    if lookback == 0 || end_idx < lookback {
        return T::nan();
    }

    let start = end_idx - lookback;
    let mut sum = T::zero();
    for i in start..end_idx {
        sum = sum + real_body(open[i], close[i]);
    }
    sum / usize_to_t(lookback)
}

/// Calculate the average high-low range over a lookback period.
pub fn average_range<T: SeriesElement>(
    high: &[T],
    low: &[T],
    end_idx: usize,
    lookback: usize,
) -> T {
    if lookback == 0 || end_idx < lookback {
        return T::nan();
    }

    let start = end_idx - lookback;
    let mut sum = T::zero();
    for i in start..end_idx {
        sum = sum + candle_range(high[i], low[i]);
    }
    sum / usize_to_t(lookback)
}

/// Calculate the average upper shadow over a lookback period.
pub fn average_upper_shadow<T: SeriesElement>(
    open: &[T],
    high: &[T],
    close: &[T],
    end_idx: usize,
    lookback: usize,
) -> T {
    if lookback == 0 || end_idx < lookback {
        return T::nan();
    }

    let start = end_idx - lookback;
    let mut sum = T::zero();
    for i in start..end_idx {
        sum = sum + upper_shadow(open[i], high[i], close[i]);
    }
    sum / usize_to_t(lookback)
}

/// Calculate the average lower shadow over a lookback period.
pub fn average_lower_shadow<T: SeriesElement>(
    open: &[T],
    low: &[T],
    close: &[T],
    end_idx: usize,
    lookback: usize,
) -> T {
    if lookback == 0 || end_idx < lookback {
        return T::nan();
    }

    let start = end_idx - lookback;
    let mut sum = T::zero();
    for i in start..end_idx {
        sum = sum + lower_shadow(open[i], low[i], close[i]);
    }
    sum / usize_to_t(lookback)
}

/// Check if body is a doji (very small body relative to range).
#[inline]
pub fn is_doji<T: SeriesElement>(open: T, high: T, low: T, close: T, threshold: T) -> bool {
    let body = real_body(open, close);
    let range = candle_range(high, low);
    if range <= T::zero() {
        return body <= T::zero();
    }
    body <= range * threshold
}

/// Check if candle has a long body.
#[inline]
pub fn is_long_body<T: SeriesElement>(open: T, close: T, avg_body: T, multiplier: T) -> bool {
    real_body(open, close) > avg_body * multiplier
}

/// Check if candle has a short body.
#[inline]
pub fn is_short_body<T: SeriesElement>(open: T, close: T, avg_body: T, multiplier: T) -> bool {
    real_body(open, close) < avg_body * multiplier
}

/// Check if candle has a long upper shadow.
#[inline]
pub fn is_long_upper_shadow<T: SeriesElement>(
    open: T,
    high: T,
    close: T,
    avg_body: T,
    multiplier: T,
) -> bool {
    upper_shadow(open, high, close) > avg_body * multiplier
}

/// Check if candle has a long lower shadow.
#[inline]
pub fn is_long_lower_shadow<T: SeriesElement>(
    open: T,
    low: T,
    close: T,
    avg_body: T,
    multiplier: T,
) -> bool {
    lower_shadow(open, low, close) > avg_body * multiplier
}

/// Check if upper shadow is very short (near zero).
#[inline]
pub fn is_very_short_upper_shadow<T: SeriesElement>(
    open: T,
    high: T,
    close: T,
    range: T,
    threshold: T,
) -> bool {
    upper_shadow(open, high, close) < range * threshold
}

/// Check if lower shadow is very short (near zero).
#[inline]
pub fn is_very_short_lower_shadow<T: SeriesElement>(
    open: T,
    low: T,
    close: T,
    range: T,
    threshold: T,
) -> bool {
    lower_shadow(open, low, close) < range * threshold
}

/// Detect if we're in an uptrend (simple SMA-based detection).
/// Returns true if the close prices are trending upward.
pub fn is_uptrend<T: SeriesElement>(close: &[T], idx: usize, lookback: usize) -> bool {
    if idx < lookback {
        return false;
    }

    // Simple trend: compare current price to SMA
    let start = idx - lookback;
    let mut sum = T::zero();
    for i in start..idx {
        sum = sum + close[i];
    }
    let sma = sum / usize_to_t(lookback);
    close[idx] > sma
}

/// Detect if we're in a downtrend (simple SMA-based detection).
/// Returns true if the close prices are trending downward.
pub fn is_downtrend<T: SeriesElement>(close: &[T], idx: usize, lookback: usize) -> bool {
    if idx < lookback {
        return false;
    }

    // Simple trend: compare current price to SMA
    let start = idx - lookback;
    let mut sum = T::zero();
    for i in start..idx {
        sum = sum + close[i];
    }
    let sma = sum / usize_to_t(lookback);
    close[idx] < sma
}

/// Check if two values are approximately equal within a threshold.
#[inline]
pub fn approx_equal<T: SeriesElement>(a: T, b: T, threshold: T) -> bool {
    (a - b).abs() <= threshold
}

/// Bullish pattern signal (TA-Lib compatible).
pub const PATTERN_BULLISH: i32 = 100;
/// Bearish pattern signal (TA-Lib compatible).
pub const PATTERN_BEARISH: i32 = -100;
/// Strong bullish pattern signal (TA-Lib compatible).
pub const PATTERN_BULLISH_STRONG: i32 = 200;
/// Strong bearish pattern signal (TA-Lib compatible).
pub const PATTERN_BEARISH_STRONG: i32 = -200;
/// No pattern detected (TA-Lib compatible).
pub const PATTERN_NONE: i32 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_body() {
        assert!((real_body(100.0_f64, 105.0) - 5.0).abs() < 1e-10);
        assert!((real_body(105.0_f64, 100.0) - 5.0).abs() < 1e-10);
        assert!((real_body(100.0_f64, 100.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_upper_shadow() {
        // Bullish candle: open=100, high=110, close=105
        assert!((upper_shadow(100.0_f64, 110.0, 105.0) - 5.0).abs() < 1e-10);
        // Bearish candle: open=105, high=110, close=100
        assert!((upper_shadow(105.0_f64, 110.0, 100.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_lower_shadow() {
        // Bullish candle: open=100, low=95, close=105
        assert!((lower_shadow(100.0_f64, 95.0, 105.0) - 5.0).abs() < 1e-10);
        // Bearish candle: open=105, low=95, close=100
        assert!((lower_shadow(105.0_f64, 95.0, 100.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_is_bullish_bearish() {
        assert!(is_bullish(100.0_f64, 105.0));
        assert!(!is_bullish(105.0_f64, 100.0));
        assert!(is_bearish(105.0_f64, 100.0));
        assert!(!is_bearish(100.0_f64, 105.0));
    }

    #[test]
    fn test_is_doji() {
        // Perfect doji
        assert!(is_doji(100.0_f64, 110.0, 90.0, 100.0, 0.1));
        // Small body, should be doji at 10% threshold
        assert!(is_doji(100.0_f64, 110.0, 90.0, 101.0, 0.1));
        // Larger body, not a doji
        assert!(!is_doji(100.0_f64, 110.0, 90.0, 105.0, 0.1));
    }

    #[test]
    fn test_gap_detection() {
        assert!(gap_up(100.0_f64, 101.0));
        assert!(!gap_up(100.0_f64, 99.0));
        assert!(gap_down(100.0_f64, 99.0));
        assert!(!gap_down(100.0_f64, 101.0));
    }

    #[test]
    fn test_body_positions() {
        assert!((body_top(100.0_f64, 105.0) - 105.0).abs() < 1e-10);
        assert!((body_top(105.0_f64, 100.0) - 105.0).abs() < 1e-10);
        assert!((body_bottom(100.0_f64, 105.0) - 100.0).abs() < 1e-10);
        assert!((body_bottom(105.0_f64, 100.0) - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_average_body() {
        let open = [100.0_f64, 100.0, 100.0, 100.0, 100.0];
        let close = [105.0, 103.0, 107.0, 102.0, 108.0];
        // Bodies: 5, 3, 7, 2, 8 = 25/5 = 5.0
        let avg = average_body(&open, &close, 5, 5);
        assert!((avg - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_average_range() {
        let high = [110.0_f64, 108.0, 112.0, 106.0, 114.0];
        let low = [90.0, 92.0, 88.0, 94.0, 86.0];
        // Ranges: 20, 16, 24, 12, 28 = 100/5 = 20.0
        let avg = average_range(&high, &low, 5, 5);
        assert!((avg - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_trend_detection() {
        // Uptrend: prices increasing
        let close_up = [100.0_f64, 101.0, 102.0, 103.0, 104.0, 110.0];
        assert!(is_uptrend(&close_up, 5, 5));

        // Downtrend: prices decreasing
        let close_down = [110.0_f64, 109.0, 108.0, 107.0, 106.0, 100.0];
        assert!(is_downtrend(&close_down, 5, 5));
    }
}
