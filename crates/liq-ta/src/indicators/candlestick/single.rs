//! Single-candle pattern recognition functions.
//!
//! These patterns are identified based on a single candlestick's shape,
//! though some (like Hammer) require trend context for proper interpretation.

use super::core::{
    AVG_LOOKBACK, PATTERN_BEARISH, PATTERN_BULLISH, PATTERN_NONE, TREND_LOOKBACK, average_body,
    body_midpoint, candle_range, f64_to_t, is_bearish, is_bullish, is_doji, is_downtrend,
    is_uptrend, lower_shadow, real_body, upper_shadow,
};
use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Lookback for `CDL_DOJI` pattern (just current candle).
#[must_use]
pub const fn cdl_doji_lookback() -> usize {
    0
}

/// Minimum length for `CDL_DOJI`.
#[must_use]
pub const fn cdl_doji_min_len() -> usize {
    1
}

/// Doji pattern recognition.
///
/// A Doji is a candlestick with a very small body (open ≈ close),
/// indicating market indecision.
///
/// Returns:
/// - `100`: Doji detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_doji<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_doji_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Doji pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_doji_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let threshold = f64_to_t(0.1); // 10% of range

    for i in 0..len {
        if is_doji(open[i], high[i], low[i], close[i], threshold) {
            out[i] = PATTERN_BULLISH; // Doji is neutral, but we signal detection
        } else {
            out[i] = PATTERN_NONE;
        }
    }

    Ok(())
}

/// Lookback for `CDL_DRAGONFLY_DOJI` pattern.
#[must_use]
pub const fn cdl_dragonfly_doji_lookback() -> usize {
    0
}

/// Minimum length for `CDL_DRAGONFLY_DOJI`.
#[must_use]
pub const fn cdl_dragonfly_doji_min_len() -> usize {
    1
}

/// Dragonfly Doji pattern recognition.
///
/// A Dragonfly Doji has a long lower shadow and no upper shadow,
/// with open and close at or near the high. Bullish reversal signal.
///
/// Returns:
/// - `100`: Dragonfly Doji detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_dragonfly_doji<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_dragonfly_doji_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Dragonfly Doji pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_dragonfly_doji_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let body_threshold = f64_to_t(0.1);
    let shadow_threshold = f64_to_t(0.1);

    for i in 0..len {
        let range = candle_range(high[i], low[i]);
        if range <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Must be a doji
        if !is_doji(open[i], high[i], low[i], close[i], body_threshold) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Upper shadow must be very short
        let upper = upper_shadow(open[i], high[i], close[i]);
        if upper > range * shadow_threshold {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Lower shadow longness is implied by doji + short upper shadow constraints.
        let _lower = lower_shadow(open[i], low[i], close[i]);

        out[i] = PATTERN_BULLISH;
    }

    Ok(())
}

/// Lookback for `CDL_GRAVESTONE_DOJI` pattern.
#[must_use]
pub const fn cdl_gravestone_doji_lookback() -> usize {
    0
}

/// Minimum length for `CDL_GRAVESTONE_DOJI`.
#[must_use]
pub const fn cdl_gravestone_doji_min_len() -> usize {
    1
}

/// Gravestone Doji pattern recognition.
///
/// A Gravestone Doji has a long upper shadow and no lower shadow,
/// with open and close at or near the low. Bearish reversal signal.
///
/// Returns:
/// - `-100`: Gravestone Doji detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_gravestone_doji<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_gravestone_doji_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Gravestone Doji pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_gravestone_doji_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let body_threshold = f64_to_t(0.1);
    let shadow_threshold = f64_to_t(0.1);

    for i in 0..len {
        let range = candle_range(high[i], low[i]);
        if range <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Must be a doji
        if !is_doji(open[i], high[i], low[i], close[i], body_threshold) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Lower shadow must be very short
        let lower = lower_shadow(open[i], low[i], close[i]);
        if lower > range * shadow_threshold {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Upper shadow longness is implied by doji + short lower shadow constraints.
        let _upper = upper_shadow(open[i], high[i], close[i]);

        out[i] = PATTERN_BEARISH;
    }

    Ok(())
}

/// Lookback for `CDL_LONGLEG_DOJI` pattern.
#[must_use]
pub const fn cdl_longleg_doji_lookback() -> usize {
    0
}

/// Minimum length for `CDL_LONGLEG_DOJI`.
#[must_use]
pub const fn cdl_longleg_doji_min_len() -> usize {
    1
}

/// Long-Legged Doji pattern recognition.
///
/// A Long-Legged Doji has long shadows on both sides with a small body,
/// indicating significant indecision.
///
/// Returns:
/// - `100`: Long-Legged Doji detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_longleg_doji<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_longleg_doji_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Long-Legged Doji pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_longleg_doji_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let body_threshold = f64_to_t(0.1);
    let shadow_min = f64_to_t(0.3); // Each shadow at least 30% of range

    for i in 0..len {
        let range = candle_range(high[i], low[i]);
        if range <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Must be a doji
        if !is_doji(open[i], high[i], low[i], close[i], body_threshold) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Both shadows must be long
        let upper = upper_shadow(open[i], high[i], close[i]);
        let lower = lower_shadow(open[i], low[i], close[i]);

        if upper >= range * shadow_min && lower >= range * shadow_min {
            out[i] = PATTERN_BULLISH;
        } else {
            out[i] = PATTERN_NONE;
        }
    }

    Ok(())
}

/// Lookback for `CDL_RICKSHAW_MAN` pattern.
#[must_use]
pub const fn cdl_rickshaw_man_lookback() -> usize {
    0
}

/// Minimum length for `CDL_RICKSHAW_MAN`.
#[must_use]
pub const fn cdl_rickshaw_man_min_len() -> usize {
    1
}

/// Rickshaw Man pattern recognition.
///
/// Similar to Long-Legged Doji but with the body in the middle of the range.
///
/// Returns:
/// - `100`: Rickshaw Man detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_rickshaw_man<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_rickshaw_man_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Rickshaw Man pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_rickshaw_man_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let body_threshold = f64_to_t(0.1);
    let shadow_min = f64_to_t(0.3);
    let center_tolerance = f64_to_t(0.1); // Body within 10% of center

    for i in 0..len {
        let range = candle_range(high[i], low[i]);
        if range <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Must be a doji
        if !is_doji(open[i], high[i], low[i], close[i], body_threshold) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Both shadows must be long
        let upper = upper_shadow(open[i], high[i], close[i]);
        let lower = lower_shadow(open[i], low[i], close[i]);

        if upper < range * shadow_min || lower < range * shadow_min {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Body should be near the center
        let body_mid = body_midpoint(open[i], close[i]);
        let range_mid = (high[i] + low[i]) / f64_to_t(2.0);
        if (body_mid - range_mid).abs() <= range * center_tolerance {
            out[i] = PATTERN_BULLISH;
        } else {
            out[i] = PATTERN_NONE;
        }
    }

    Ok(())
}

/// Lookback for `CDL_MARUBOZU` pattern.
#[must_use]
pub const fn cdl_marubozu_lookback() -> usize {
    AVG_LOOKBACK
}

/// Minimum length for `CDL_MARUBOZU`.
#[must_use]
pub const fn cdl_marubozu_min_len() -> usize {
    AVG_LOOKBACK + 1
}

/// Marubozu pattern recognition.
///
/// A Marubozu has no shadows (or very short shadows) - the open and close
/// are at the extremes of the range. Strong directional signal.
///
/// Returns:
/// - `100`: Bullish Marubozu (white/green - close > open at high)
/// - `-100`: Bearish Marubozu (black/red - close < open at low)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_marubozu<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_marubozu_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Marubozu pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_marubozu_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_marubozu_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let shadow_threshold = f64_to_t(0.05); // Max 5% shadows
    let body_multiplier = f64_to_t(1.0); // Body must be larger than average

    // Fill lookback period with zeros
    for i in 0..AVG_LOOKBACK {
        out[i] = PATTERN_NONE;
    }

    for i in AVG_LOOKBACK..len {
        let range = candle_range(high[i], low[i]);
        if range <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);
        let body = real_body(open[i], close[i]);

        // Body must be significant
        if body < avg_body * body_multiplier {
            out[i] = PATTERN_NONE;
            continue;
        }

        let upper = upper_shadow(open[i], high[i], close[i]);
        let lower = lower_shadow(open[i], low[i], close[i]);

        // Both shadows must be very short
        if upper > range * shadow_threshold || lower > range * shadow_threshold {
            out[i] = PATTERN_NONE;
            continue;
        }

        if is_bullish(open[i], close[i]) {
            out[i] = PATTERN_BULLISH;
        } else {
            out[i] = PATTERN_BEARISH;
        }
    }

    Ok(())
}

/// Lookback for `CDL_CLOSING_MARUBOZU` pattern.
#[must_use]
pub const fn cdl_closing_marubozu_lookback() -> usize {
    AVG_LOOKBACK
}

/// Minimum length for `CDL_CLOSING_MARUBOZU`.
#[must_use]
pub const fn cdl_closing_marubozu_min_len() -> usize {
    AVG_LOOKBACK + 1
}

/// Closing Marubozu pattern recognition.
///
/// A Closing Marubozu has no shadow on the closing end.
/// - Bullish: close at high, may have lower shadow
/// - Bearish: close at low, may have upper shadow
///
/// Returns:
/// - `100`: Bullish Closing Marubozu
/// - `-100`: Bearish Closing Marubozu
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_closing_marubozu<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_closing_marubozu_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Closing Marubozu pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_closing_marubozu_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_closing_marubozu_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let shadow_threshold = f64_to_t(0.05);
    let body_multiplier = f64_to_t(0.8);

    for i in 0..AVG_LOOKBACK {
        out[i] = PATTERN_NONE;
    }

    for i in AVG_LOOKBACK..len {
        let range = candle_range(high[i], low[i]);
        if range <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);
        let body = real_body(open[i], close[i]);

        // Body must be significant
        if body < avg_body * body_multiplier {
            out[i] = PATTERN_NONE;
            continue;
        }

        if is_bullish(open[i], close[i]) {
            // Bullish: close should be at or near high
            let upper = upper_shadow(open[i], high[i], close[i]);
            if upper <= range * shadow_threshold {
                out[i] = PATTERN_BULLISH;
            } else {
                out[i] = PATTERN_NONE;
            }
        } else {
            // Bearish: close should be at or near low
            let lower = lower_shadow(open[i], low[i], close[i]);
            if lower <= range * shadow_threshold {
                out[i] = PATTERN_BEARISH;
            } else {
                out[i] = PATTERN_NONE;
            }
        }
    }

    Ok(())
}

/// Lookback for `CDL_SPINNING_TOP` pattern.
#[must_use]
pub const fn cdl_spinning_top_lookback() -> usize {
    AVG_LOOKBACK
}

/// Minimum length for `CDL_SPINNING_TOP`.
#[must_use]
pub const fn cdl_spinning_top_min_len() -> usize {
    AVG_LOOKBACK + 1
}

/// Spinning Top pattern recognition.
///
/// A Spinning Top has a small body with upper and lower shadows that
/// exceed the body length. Indicates indecision.
///
/// Returns:
/// - `100`: Bullish Spinning Top
/// - `-100`: Bearish Spinning Top
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_spinning_top<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_spinning_top_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Spinning Top pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_spinning_top_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_spinning_top_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    for i in 0..AVG_LOOKBACK {
        out[i] = PATTERN_NONE;
    }

    for i in AVG_LOOKBACK..len {
        let body = real_body(open[i], close[i]);
        let upper = upper_shadow(open[i], high[i], close[i]);
        let lower = lower_shadow(open[i], low[i], close[i]);
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // Body must be small
        if body > avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Both shadows must be longer than the body
        if upper <= body || lower <= body {
            out[i] = PATTERN_NONE;
            continue;
        }

        if is_bullish(open[i], close[i]) {
            out[i] = PATTERN_BULLISH;
        } else if is_bearish(open[i], close[i]) {
            out[i] = PATTERN_BEARISH;
        } else {
            out[i] = PATTERN_BULLISH; // Neutral case
        }
    }

    Ok(())
}

/// Lookback for `CDL_HIGH_WAVE` pattern.
#[must_use]
pub const fn cdl_high_wave_lookback() -> usize {
    AVG_LOOKBACK
}

/// Minimum length for `CDL_HIGH_WAVE`.
#[must_use]
pub const fn cdl_high_wave_min_len() -> usize {
    AVG_LOOKBACK + 1
}

/// High Wave pattern recognition.
///
/// A High Wave candle has a small body with very long upper and lower
/// shadows, indicating extreme indecision.
///
/// Returns:
/// - `100`: High Wave detected (bullish)
/// - `-100`: High Wave detected (bearish)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_high_wave<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_high_wave_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// High Wave pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_high_wave_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_high_wave_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let shadow_multiplier = f64_to_t(3.0); // Shadows 3x body

    for i in 0..AVG_LOOKBACK {
        out[i] = PATTERN_NONE;
    }

    for i in AVG_LOOKBACK..len {
        let body = real_body(open[i], close[i]);
        let upper = upper_shadow(open[i], high[i], close[i]);
        let lower = lower_shadow(open[i], low[i], close[i]);
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // Body must be small
        if body > avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Both shadows must be very long (at least 3x the body)
        let body_ref = if body > T::zero() {
            body
        } else {
            avg_body * f64_to_t(0.1)
        };
        if upper < body_ref * shadow_multiplier || lower < body_ref * shadow_multiplier {
            out[i] = PATTERN_NONE;
            continue;
        }

        if is_bullish(open[i], close[i]) {
            out[i] = PATTERN_BULLISH;
        } else {
            out[i] = PATTERN_BEARISH;
        }
    }

    Ok(())
}

/// Lookback for `CDL_LONG_LINE` pattern.
#[must_use]
pub const fn cdl_long_line_lookback() -> usize {
    AVG_LOOKBACK
}

/// Minimum length for `CDL_LONG_LINE`.
#[must_use]
pub const fn cdl_long_line_min_len() -> usize {
    AVG_LOOKBACK + 1
}

/// Long Line pattern recognition.
///
/// A Long Line candle has a long body with short shadows.
///
/// Returns:
/// - `100`: Bullish Long Line
/// - `-100`: Bearish Long Line
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_long_line<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_long_line_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Long Line pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_long_line_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_long_line_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let body_multiplier = f64_to_t(3.0); // Body 3x average

    for i in 0..AVG_LOOKBACK {
        out[i] = PATTERN_NONE;
    }

    for i in AVG_LOOKBACK..len {
        let body = real_body(open[i], close[i]);
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // Body must be very long
        if body < avg_body * body_multiplier {
            out[i] = PATTERN_NONE;
            continue;
        }

        if is_bullish(open[i], close[i]) {
            out[i] = PATTERN_BULLISH;
        } else {
            out[i] = PATTERN_BEARISH;
        }
    }

    Ok(())
}

/// Lookback for `CDL_SHORT_LINE` pattern.
#[must_use]
pub const fn cdl_short_line_lookback() -> usize {
    AVG_LOOKBACK
}

/// Minimum length for `CDL_SHORT_LINE`.
#[must_use]
pub const fn cdl_short_line_min_len() -> usize {
    AVG_LOOKBACK + 1
}

/// Short Line pattern recognition.
///
/// A Short Line candle has a short body with short shadows.
///
/// Returns:
/// - `100`: Bullish Short Line
/// - `-100`: Bearish Short Line
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_short_line<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_short_line_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Short Line pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_short_line_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_short_line_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let body_threshold = f64_to_t(0.5); // Body less than 50% of average
    let shadow_threshold = f64_to_t(0.3); // Shadows less than 30% of range

    for i in 0..AVG_LOOKBACK {
        out[i] = PATTERN_NONE;
    }

    for i in AVG_LOOKBACK..len {
        let body = real_body(open[i], close[i]);
        let range = candle_range(high[i], low[i]);
        let upper = upper_shadow(open[i], high[i], close[i]);
        let lower = lower_shadow(open[i], low[i], close[i]);
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // Body must be short
        if body > avg_body * body_threshold {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Shadows must be short
        if upper > range * shadow_threshold || lower > range * shadow_threshold {
            out[i] = PATTERN_NONE;
            continue;
        }

        if is_bullish(open[i], close[i]) {
            out[i] = PATTERN_BULLISH;
        } else if is_bearish(open[i], close[i]) {
            out[i] = PATTERN_BEARISH;
        } else {
            out[i] = PATTERN_BULLISH;
        }
    }

    Ok(())
}

/// Lookback for `CDL_HAMMER` pattern.
#[must_use]
pub const fn cdl_hammer_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK
}

/// Minimum length for `CDL_HAMMER`.
#[must_use]
pub const fn cdl_hammer_min_len() -> usize {
    cdl_hammer_lookback() + 1
}

/// Hammer pattern recognition.
///
/// A Hammer has a small body at the top of the range with a long lower
/// shadow (at least 2x the body) and little or no upper shadow.
/// Appears in a downtrend as a bullish reversal signal.
///
/// Returns:
/// - `100`: Hammer detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_hammer<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_hammer_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Hammer pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_hammer_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_hammer_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let lookback = cdl_hammer_lookback();
    let lower_shadow_mult = f64_to_t(2.0);
    let upper_shadow_max = f64_to_t(0.1);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        // Must be in a downtrend
        if !is_downtrend(close, i, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let body = real_body(open[i], close[i]);
        let range = candle_range(high[i], low[i]);
        let upper = upper_shadow(open[i], high[i], close[i]);
        let lower = lower_shadow(open[i], low[i], close[i]);
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        if range <= T::zero() || avg_body <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Small body
        if body > avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Long lower shadow (at least 2x body)
        let body_ref = if body > T::zero() {
            body
        } else {
            avg_body * f64_to_t(0.1)
        };
        if lower < body_ref * lower_shadow_mult {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Very short upper shadow
        if upper > range * upper_shadow_max {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BULLISH;
    }

    Ok(())
}

/// Lookback for `CDL_HANGING_MAN` pattern.
#[must_use]
pub const fn cdl_hanging_man_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK
}

/// Minimum length for `CDL_HANGING_MAN`.
#[must_use]
pub const fn cdl_hanging_man_min_len() -> usize {
    cdl_hanging_man_lookback() + 1
}

/// Hanging Man pattern recognition.
///
/// Same shape as Hammer but appears in an uptrend as a bearish reversal.
///
/// Returns:
/// - `-100`: Hanging Man detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_hanging_man<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_hanging_man_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Hanging Man pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_hanging_man_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_hanging_man_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let lookback = cdl_hanging_man_lookback();
    let lower_shadow_mult = f64_to_t(2.0);
    let upper_shadow_max = f64_to_t(0.1);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        // Must be in an uptrend
        if !is_uptrend(close, i, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let body = real_body(open[i], close[i]);
        let range = candle_range(high[i], low[i]);
        let upper = upper_shadow(open[i], high[i], close[i]);
        let lower = lower_shadow(open[i], low[i], close[i]);
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        if range <= T::zero() || avg_body <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Small body
        if body > avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Long lower shadow
        let body_ref = if body > T::zero() {
            body
        } else {
            avg_body * f64_to_t(0.1)
        };
        if lower < body_ref * lower_shadow_mult {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Very short upper shadow
        if upper > range * upper_shadow_max {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BEARISH;
    }

    Ok(())
}

/// Lookback for `CDL_INVERTED_HAMMER` pattern.
#[must_use]
pub const fn cdl_inverted_hammer_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK
}

/// Minimum length for `CDL_INVERTED_HAMMER`.
#[must_use]
pub const fn cdl_inverted_hammer_min_len() -> usize {
    cdl_inverted_hammer_lookback() + 1
}

/// Inverted Hammer pattern recognition.
///
/// An Inverted Hammer has a small body at the bottom with a long upper
/// shadow. Appears in a downtrend as a bullish reversal signal.
///
/// Returns:
/// - `100`: Inverted Hammer detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_inverted_hammer<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_inverted_hammer_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Inverted Hammer pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_inverted_hammer_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_inverted_hammer_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let lookback = cdl_inverted_hammer_lookback();
    let upper_shadow_mult = f64_to_t(2.0);
    let lower_shadow_max = f64_to_t(0.1);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        // Must be in a downtrend
        if !is_downtrend(close, i, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let body = real_body(open[i], close[i]);
        let range = candle_range(high[i], low[i]);
        let upper = upper_shadow(open[i], high[i], close[i]);
        let lower = lower_shadow(open[i], low[i], close[i]);
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        if range <= T::zero() || avg_body <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Small body
        if body > avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Long upper shadow
        let body_ref = if body > T::zero() {
            body
        } else {
            avg_body * f64_to_t(0.1)
        };
        if upper < body_ref * upper_shadow_mult {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Very short lower shadow
        if lower > range * lower_shadow_max {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BULLISH;
    }

    Ok(())
}

/// Lookback for `CDL_SHOOTING_STAR` pattern.
#[must_use]
pub const fn cdl_shooting_star_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK
}

/// Minimum length for `CDL_SHOOTING_STAR`.
#[must_use]
pub const fn cdl_shooting_star_min_len() -> usize {
    cdl_shooting_star_lookback() + 1
}

/// Shooting Star pattern recognition.
///
/// Same shape as Inverted Hammer but appears in an uptrend as a bearish reversal.
///
/// Returns:
/// - `-100`: Shooting Star detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_shooting_star<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_shooting_star_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Shooting Star pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_shooting_star_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_shooting_star_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let lookback = cdl_shooting_star_lookback();
    let upper_shadow_mult = f64_to_t(2.0);
    let lower_shadow_max = f64_to_t(0.1);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        // Must be in an uptrend
        if !is_uptrend(close, i, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let body = real_body(open[i], close[i]);
        let range = candle_range(high[i], low[i]);
        let upper = upper_shadow(open[i], high[i], close[i]);
        let lower = lower_shadow(open[i], low[i], close[i]);
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        if range <= T::zero() || avg_body <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Small body
        if body > avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Long upper shadow
        let body_ref = if body > T::zero() {
            body
        } else {
            avg_body * f64_to_t(0.1)
        };
        if upper < body_ref * upper_shadow_mult {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Very short lower shadow
        if lower > range * lower_shadow_max {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BEARISH;
    }

    Ok(())
}

/// Lookback for `CDL_TAKURI` pattern.
#[must_use]
pub const fn cdl_takuri_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK
}

/// Minimum length for `CDL_TAKURI`.
#[must_use]
pub const fn cdl_takuri_min_len() -> usize {
    cdl_takuri_lookback() + 1
}

/// Takuri (Dragonfly Doji with trend) pattern recognition.
///
/// A Takuri is essentially a Dragonfly Doji that appears in a downtrend,
/// making it a strong bullish reversal signal.
///
/// Returns:
/// - `100`: Takuri detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_takuri<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_takuri_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Takuri pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_takuri_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_takuri_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let lookback = cdl_takuri_lookback();
    let body_threshold = f64_to_t(0.1);
    let upper_shadow_max = f64_to_t(0.1);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        // Must be in a downtrend
        if !is_downtrend(close, i, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let range = candle_range(high[i], low[i]);
        if range <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Must be a doji
        if !is_doji(open[i], high[i], low[i], close[i], body_threshold) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let upper = upper_shadow(open[i], high[i], close[i]);
        let lower = lower_shadow(open[i], low[i], close[i]);
        let body = real_body(open[i], close[i]);

        // Upper shadow must be very short
        if upper > range * upper_shadow_max {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Lower-shadow longness is implied by doji + short upper-shadow constraints.
        let _body = body;
        let _lower = lower;

        out[i] = PATTERN_BULLISH;
    }

    Ok(())
}

/// Lookback for `CDL_BELT_HOLD` pattern.
#[must_use]
pub const fn cdl_belt_hold_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK
}

/// Minimum length for `CDL_BELT_HOLD`.
#[must_use]
pub const fn cdl_belt_hold_min_len() -> usize {
    cdl_belt_hold_lookback() + 1
}

/// Belt Hold pattern recognition.
///
/// A Belt Hold is a long candlestick that opens at its extreme (high for bearish,
/// low for bullish) with no shadow on the opening side.
///
/// Returns:
/// - `100`: Bullish Belt Hold (opens at low in downtrend)
/// - `-100`: Bearish Belt Hold (opens at high in uptrend)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_belt_hold<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_belt_hold_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Belt Hold pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_belt_hold_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    let len = open.len();
    if len == 0 {
        return Err(Error::EmptyInput);
    }
    if high.len() != len || low.len() != len || close.len() != len {
        return Err(Error::LengthMismatch {
            description: "OHLC arrays have different lengths".to_string(),
        });
    }
    if len < cdl_belt_hold_min_len() {
        return Err(Error::InsufficientData {
            required: 0,
            actual: len,
            indicator: "candlestick",
        });
    }
    if out.len() < len {
        return Err(Error::BufferTooSmall {
            required: len,
            actual: out.len(),
            indicator: "candlestick",
        });
    }

    let lookback = cdl_belt_hold_lookback();
    let body_multiplier = f64_to_t(1.0);
    let shadow_max = f64_to_t(0.05);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let body = real_body(open[i], close[i]);
        let range = candle_range(high[i], low[i]);
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        if range <= T::zero() || avg_body <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Body must be long
        if body < avg_body * body_multiplier {
            out[i] = PATTERN_NONE;
            continue;
        }

        if is_bullish(open[i], close[i]) {
            // Bullish: must be in downtrend, open at low
            if !is_downtrend(close, i, TREND_LOOKBACK) {
                out[i] = PATTERN_NONE;
                continue;
            }
            let lower = lower_shadow(open[i], low[i], close[i]);
            if lower <= range * shadow_max {
                out[i] = PATTERN_BULLISH;
            } else {
                out[i] = PATTERN_NONE;
            }
        } else {
            // Bearish: must be in uptrend, open at high
            if !is_uptrend(close, i, TREND_LOOKBACK) {
                out[i] = PATTERN_NONE;
                continue;
            }
            let upper = upper_shadow(open[i], high[i], close[i]);
            if upper <= range * shadow_max {
                out[i] = PATTERN_BEARISH;
            } else {
                out[i] = PATTERN_NONE;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doji() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        // Perfect doji: open == close, with shadows
        (
            vec![100.0], // open
            vec![110.0], // high
            vec![90.0],  // low
            vec![100.0], // close
        )
    }

    fn make_dragonfly_doji() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        // Dragonfly: open/close at high, long lower shadow
        (
            vec![100.0], // open
            vec![100.5], // high (very small upper shadow)
            vec![80.0],  // low (long lower shadow)
            vec![100.0], // close
        )
    }

    fn make_gravestone_doji() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        // Gravestone: open/close at low, long upper shadow
        (
            vec![100.0], // open
            vec![120.0], // high (long upper shadow)
            vec![99.5],  // low (very small lower shadow)
            vec![100.0], // close
        )
    }

    #[test]
    fn test_cdl_doji_basic() {
        let (open, high, low, close) = make_doji();
        let result = cdl_doji(&open, &high, &low, &close).unwrap();
        assert_eq!(result[0], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_doji_not_doji() {
        // Large body - not a doji
        let open = vec![100.0];
        let high = vec![110.0];
        let low = vec![90.0];
        let close = vec![108.0]; // 8% body on 20% range = 40% ratio, not doji
        let result = cdl_doji(&open, &high, &low, &close).unwrap();
        assert_eq!(result[0], PATTERN_NONE);
    }

    #[test]
    fn test_cdl_dragonfly_doji() {
        let (open, high, low, close) = make_dragonfly_doji();
        let result = cdl_dragonfly_doji(&open, &high, &low, &close).unwrap();
        assert_eq!(result[0], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_gravestone_doji() {
        let (open, high, low, close) = make_gravestone_doji();
        let result = cdl_gravestone_doji(&open, &high, &low, &close).unwrap();
        assert_eq!(result[0], PATTERN_BEARISH);
    }

    #[test]
    fn test_cdl_longleg_doji() {
        // Long-legged doji: both shadows long, body small
        let open = vec![100.0];
        let high = vec![115.0]; // 15 point upper shadow
        let low = vec![85.0]; // 15 point lower shadow
        let close = vec![100.0]; // Doji
        let result = cdl_longleg_doji(&open, &high, &low, &close).unwrap();
        assert_eq!(result[0], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_marubozu() {
        // Need 11 data points for lookback
        let mut open = vec![100.0; 11];
        let mut high = vec![105.0; 11];
        let mut low = vec![95.0; 11];
        let mut close = vec![103.0; 11];

        // Last candle is a marubozu (no shadows, long body)
        open[10] = 100.0;
        high[10] = 115.0;
        low[10] = 100.0;
        close[10] = 115.0;

        let result = cdl_marubozu(&open, &high, &low, &close).unwrap();
        assert_eq!(result[10], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_spinning_top() {
        let mut open = vec![100.0; 11];
        let mut high = vec![110.0; 11];
        let mut low = vec![90.0; 11];
        let mut close = vec![105.0; 11];

        // Last candle: small body, long shadows
        open[10] = 100.0;
        high[10] = 115.0; // 14 point upper shadow
        low[10] = 85.0; // 14 point lower shadow
        close[10] = 101.0; // 1 point body

        let result = cdl_spinning_top(&open, &high, &low, &close).unwrap();
        assert_eq!(result[10], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_hammer_in_downtrend() {
        // Create a downtrend followed by hammer
        let n = 25;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        // Create downtrend
        for i in 0..n - 1 {
            let base = 120.0 - (i as f64) * 2.0;
            open.push(base);
            high.push(base + 2.0);
            low.push(base - 4.0);
            close.push(base - 2.0);
        }

        // Add hammer at the end
        let last_base = 120.0 - ((n - 1) as f64) * 2.0;
        open.push(last_base);
        high.push(last_base + 1.0);
        low.push(last_base - 15.0); // Long lower shadow
        close.push(last_base + 0.5); // Small bullish body

        let result = cdl_hammer(&open, &high, &low, &close).unwrap();
        assert_eq!(result[n - 1], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_shooting_star_in_uptrend() {
        // Create an uptrend followed by shooting star
        let n = 25;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        // Create uptrend
        for i in 0..n - 1 {
            let base = 80.0 + (i as f64) * 2.0;
            open.push(base);
            high.push(base + 4.0);
            low.push(base - 2.0);
            close.push(base + 2.0);
        }

        // Add shooting star at the end
        let last_base = 80.0 + ((n - 1) as f64) * 2.0;
        open.push(last_base);
        high.push(last_base + 15.0); // Long upper shadow
        low.push(last_base - 1.0);
        close.push(last_base - 0.5); // Small bearish body

        let result = cdl_shooting_star(&open, &high, &low, &close).unwrap();
        assert_eq!(result[n - 1], PATTERN_BEARISH);
    }

    #[test]
    fn test_empty_input() {
        let empty: Vec<f64> = vec![];
        assert!(cdl_doji(&empty, &empty, &empty, &empty).is_err());
    }

    #[test]
    fn test_length_mismatch() {
        let open = vec![100.0, 101.0];
        let high = vec![105.0];
        let low = vec![95.0, 96.0];
        let close = vec![102.0, 103.0];
        assert!(cdl_doji(&open, &high, &low, &close).is_err());
    }

    #[test]
    fn test_single_wrappers_smoke_surface() {
        let n = 640;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        for i in 0..n {
            let base = 200.0 - (i as f64) * 0.2;
            open.push(base);
            close.push(base - 0.1);
            high.push(base + 5.0);
            low.push(base - 5.0);
        }

        macro_rules! assert_len_ok {
            ($expr:expr) => {{
                let out = $expr.unwrap();
                assert_eq!(out.len(), n);
            }};
        }

        assert_len_ok!(cdl_doji(&open, &high, &low, &close));
        assert_len_ok!(cdl_dragonfly_doji(&open, &high, &low, &close));
        assert_len_ok!(cdl_gravestone_doji(&open, &high, &low, &close));
        assert_len_ok!(cdl_longleg_doji(&open, &high, &low, &close));
        assert_len_ok!(cdl_rickshaw_man(&open, &high, &low, &close));
        assert_len_ok!(cdl_marubozu(&open, &high, &low, &close));
        assert_len_ok!(cdl_closing_marubozu(&open, &high, &low, &close));
        assert_len_ok!(cdl_spinning_top(&open, &high, &low, &close));
        assert_len_ok!(cdl_high_wave(&open, &high, &low, &close));
        assert_len_ok!(cdl_long_line(&open, &high, &low, &close));
        assert_len_ok!(cdl_short_line(&open, &high, &low, &close));
        assert_len_ok!(cdl_hammer(&open, &high, &low, &close));
        assert_len_ok!(cdl_hanging_man(&open, &high, &low, &close));
        assert_len_ok!(cdl_inverted_hammer(&open, &high, &low, &close));
        assert_len_ok!(cdl_shooting_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_takuri(&open, &high, &low, &close));
        assert_len_ok!(cdl_belt_hold(&open, &high, &low, &close));
    }

    #[test]
    fn test_single_error_surface_all_patterns() {
        type SingleFn = fn(&[f64], &[f64], &[f64], &[f64]) -> crate::error::Result<Vec<i32>>;
        type SingleIntoFn =
            fn(&[f64], &[f64], &[f64], &[f64], &mut [i32]) -> crate::error::Result<()>;

        let n = 64;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        for i in 0..n {
            let base = 100.0 - (i as f64) * 0.1;
            open.push(base);
            close.push(base - 0.05);
            high.push(base + 3.0);
            low.push(base - 3.0);
        }

        let high_short = high[..(n - 1)].to_vec();
        let low_short = low[..(n - 1)].to_vec();
        let close_short = close[..(n - 1)].to_vec();
        let empty: Vec<f64> = vec![];

        let wrappers: [(&str, SingleFn); 17] = [
            ("cdl_doji", cdl_doji),
            ("cdl_dragonfly_doji", cdl_dragonfly_doji),
            ("cdl_gravestone_doji", cdl_gravestone_doji),
            ("cdl_longleg_doji", cdl_longleg_doji),
            ("cdl_rickshaw_man", cdl_rickshaw_man),
            ("cdl_marubozu", cdl_marubozu),
            ("cdl_closing_marubozu", cdl_closing_marubozu),
            ("cdl_spinning_top", cdl_spinning_top),
            ("cdl_high_wave", cdl_high_wave),
            ("cdl_long_line", cdl_long_line),
            ("cdl_short_line", cdl_short_line),
            ("cdl_hammer", cdl_hammer),
            ("cdl_hanging_man", cdl_hanging_man),
            ("cdl_inverted_hammer", cdl_inverted_hammer),
            ("cdl_shooting_star", cdl_shooting_star),
            ("cdl_takuri", cdl_takuri),
            ("cdl_belt_hold", cdl_belt_hold),
        ];

        for (name, f) in wrappers {
            assert!(f(&empty, &empty, &empty, &empty).is_err(), "{name}");
            assert!(f(&open, &high_short, &low, &close).is_err(), "{name}");
            assert!(f(&open, &high, &low_short, &close).is_err(), "{name}");
            assert!(f(&open, &high, &low, &close_short).is_err(), "{name}");
        }

        let into_fns: [(&str, SingleIntoFn); 17] = [
            ("cdl_doji_into", cdl_doji_into),
            ("cdl_dragonfly_doji_into", cdl_dragonfly_doji_into),
            ("cdl_gravestone_doji_into", cdl_gravestone_doji_into),
            ("cdl_longleg_doji_into", cdl_longleg_doji_into),
            ("cdl_rickshaw_man_into", cdl_rickshaw_man_into),
            ("cdl_marubozu_into", cdl_marubozu_into),
            ("cdl_closing_marubozu_into", cdl_closing_marubozu_into),
            ("cdl_spinning_top_into", cdl_spinning_top_into),
            ("cdl_high_wave_into", cdl_high_wave_into),
            ("cdl_long_line_into", cdl_long_line_into),
            ("cdl_short_line_into", cdl_short_line_into),
            ("cdl_hammer_into", cdl_hammer_into),
            ("cdl_hanging_man_into", cdl_hanging_man_into),
            ("cdl_inverted_hammer_into", cdl_inverted_hammer_into),
            ("cdl_shooting_star_into", cdl_shooting_star_into),
            ("cdl_takuri_into", cdl_takuri_into),
            ("cdl_belt_hold_into", cdl_belt_hold_into),
        ];

        let mut out_ok = vec![0_i32; n];
        let mut out_short = vec![0_i32; n - 1];

        for (name, f) in into_fns {
            f(&open, &high, &low, &close, &mut out_ok).unwrap();
            assert!(
                f(&open, &high, &low, &close, &mut out_short).is_err(),
                "{name}"
            );
            assert!(
                f(&open, &high_short, &low, &close, &mut out_ok).is_err(),
                "{name}"
            );
            assert!(
                f(&open, &high, &low_short, &close, &mut out_ok).is_err(),
                "{name}"
            );
            assert!(
                f(&open, &high, &low, &close_short, &mut out_ok).is_err(),
                "{name}"
            );
            assert!(
                f(&empty, &empty, &empty, &empty, &mut []).is_err(),
                "{name}"
            );
        }
    }

    #[test]
    fn test_single_f32_surface_matrix() {
        let n = 320;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        for i in 0..n {
            let base = 120.0_f32 - (i as f32) * 0.11;
            open.push(base);
            close.push(base - 0.07);
            high.push(base + 3.5);
            low.push(base - 3.5);
        }

        macro_rules! assert_len_ok {
            ($expr:expr) => {{
                let out = $expr.unwrap();
                assert_eq!(out.len(), n);
            }};
        }
        macro_rules! assert_into_ok {
            ($f:ident) => {{
                let mut out = vec![0_i32; n];
                $f(&open, &high, &low, &close, &mut out).unwrap();
                assert_eq!(out.len(), n);
            }};
        }

        assert_len_ok!(cdl_doji(&open, &high, &low, &close));
        assert_len_ok!(cdl_dragonfly_doji(&open, &high, &low, &close));
        assert_len_ok!(cdl_gravestone_doji(&open, &high, &low, &close));
        assert_len_ok!(cdl_longleg_doji(&open, &high, &low, &close));
        assert_len_ok!(cdl_rickshaw_man(&open, &high, &low, &close));
        assert_len_ok!(cdl_marubozu(&open, &high, &low, &close));
        assert_len_ok!(cdl_closing_marubozu(&open, &high, &low, &close));
        assert_len_ok!(cdl_spinning_top(&open, &high, &low, &close));
        assert_len_ok!(cdl_high_wave(&open, &high, &low, &close));
        assert_len_ok!(cdl_long_line(&open, &high, &low, &close));
        assert_len_ok!(cdl_short_line(&open, &high, &low, &close));
        assert_len_ok!(cdl_hammer(&open, &high, &low, &close));
        assert_len_ok!(cdl_hanging_man(&open, &high, &low, &close));
        assert_len_ok!(cdl_inverted_hammer(&open, &high, &low, &close));
        assert_len_ok!(cdl_shooting_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_takuri(&open, &high, &low, &close));
        assert_len_ok!(cdl_belt_hold(&open, &high, &low, &close));

        assert_into_ok!(cdl_doji_into);
        assert_into_ok!(cdl_dragonfly_doji_into);
        assert_into_ok!(cdl_gravestone_doji_into);
        assert_into_ok!(cdl_longleg_doji_into);
        assert_into_ok!(cdl_rickshaw_man_into);
        assert_into_ok!(cdl_marubozu_into);
        assert_into_ok!(cdl_closing_marubozu_into);
        assert_into_ok!(cdl_spinning_top_into);
        assert_into_ok!(cdl_high_wave_into);
        assert_into_ok!(cdl_long_line_into);
        assert_into_ok!(cdl_short_line_into);
        assert_into_ok!(cdl_hammer_into);
        assert_into_ok!(cdl_hanging_man_into);
        assert_into_ok!(cdl_inverted_hammer_into);
        assert_into_ok!(cdl_shooting_star_into);
        assert_into_ok!(cdl_takuri_into);
        assert_into_ok!(cdl_belt_hold_into);
    }
}
