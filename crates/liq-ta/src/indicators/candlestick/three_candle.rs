//! Three-candle and complex multi-candle pattern recognition functions.
//!
//! These patterns involve three or more consecutive candlesticks.

use super::core::{
    AVG_LOOKBACK, PATTERN_BEARISH, PATTERN_BULLISH, PATTERN_NONE, TREND_LOOKBACK, approx_equal,
    average_body, average_range, body_bottom, body_midpoint, body_top, candle_range, f64_to_t,
    is_bearish, is_bullish, is_doji, is_downtrend, is_uptrend, lower_shadow, real_body,
    upper_shadow,
};
use crate::error::{Error, Result};
use crate::traits::SeriesElement;

// ============================================================================
// Morning Star / Evening Star Patterns
// ============================================================================

/// Lookback for `CDL_MORNING_STAR` pattern.
#[must_use]
pub const fn cdl_morning_star_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK + 2
}

/// Minimum length for `CDL_MORNING_STAR`.
#[must_use]
pub const fn cdl_morning_star_min_len() -> usize {
    cdl_morning_star_lookback() + 1
}

/// Morning Star pattern recognition.
///
/// A three-candle bullish reversal pattern:
/// 1. Long bearish candle
/// 2. Small body (star) that gaps down
/// 3. Long bullish candle that closes into the first candle's body
///
/// Returns:
/// - `100`: Morning Star detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_morning_star<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_morning_star_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Morning Star pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_morning_star_into<T: SeriesElement>(
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
    if len < cdl_morning_star_min_len() {
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

    let lookback = cdl_morning_star_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        // Must be in downtrend
        if !is_downtrend(close, first, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // First: long bearish
        if !is_bearish(open[first], close[first]) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if real_body(open[first], close[first]) < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Second: small body (star), gaps down
        let second_body = real_body(open[second], close[second]);
        if second_body > avg_body * f64_to_t(0.5) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if body_top(open[second], close[second]) >= close[first] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Third: long bullish
        if !is_bullish(open[third], close[third]) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if real_body(open[third], close[third]) < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Third closes into first candle's body (above midpoint)
        let first_mid = body_midpoint(open[first], close[first]);
        if close[third] < first_mid {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BULLISH;
    }

    Ok(())
}

/// Lookback for `CDL_EVENING_STAR` pattern.
#[must_use]
pub const fn cdl_evening_star_lookback() -> usize {
    cdl_morning_star_lookback()
}

/// Minimum length for `CDL_EVENING_STAR`.
#[must_use]
pub const fn cdl_evening_star_min_len() -> usize {
    cdl_morning_star_min_len()
}

/// Evening Star pattern recognition.
///
/// The bearish counterpart to Morning Star.
///
/// Returns:
/// - `-100`: Evening Star detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_evening_star<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_evening_star_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Evening Star pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_evening_star_into<T: SeriesElement>(
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
    if len < cdl_evening_star_min_len() {
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

    let lookback = cdl_evening_star_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        // Must be in uptrend
        if !is_uptrend(close, first, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // First: long bullish
        if !is_bullish(open[first], close[first]) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if real_body(open[first], close[first]) < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Second: small body (star), gaps up
        let second_body = real_body(open[second], close[second]);
        if second_body > avg_body * f64_to_t(0.5) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if body_bottom(open[second], close[second]) <= close[first] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Third: long bearish
        if !is_bearish(open[third], close[third]) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if real_body(open[third], close[third]) < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Third closes into first candle's body
        let first_mid = body_midpoint(open[first], close[first]);
        if close[third] > first_mid {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BEARISH;
    }

    Ok(())
}

/// Lookback for `CDL_MORNING_DOJI_STAR` pattern.
#[must_use]
pub const fn cdl_morning_doji_star_lookback() -> usize {
    cdl_morning_star_lookback()
}

/// Minimum length for `CDL_MORNING_DOJI_STAR`.
#[must_use]
pub const fn cdl_morning_doji_star_min_len() -> usize {
    cdl_morning_star_min_len()
}

/// Morning Doji Star pattern recognition.
///
/// Same as Morning Star but the middle candle is a Doji.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_morning_doji_star<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_morning_doji_star_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Morning Doji Star pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_morning_doji_star_into<T: SeriesElement>(
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
    if len < cdl_morning_doji_star_min_len() {
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

    let lookback = cdl_morning_doji_star_lookback();
    let doji_threshold = f64_to_t(0.1);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        if !is_downtrend(close, first, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // First: long bearish
        if !is_bearish(open[first], close[first]) || real_body(open[first], close[first]) < avg_body
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Second: doji, gaps down
        if !is_doji(
            open[second],
            high[second],
            low[second],
            close[second],
            doji_threshold,
        ) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if high[second] >= close[first] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Third: long bullish closes into first
        if !is_bullish(open[third], close[third]) || real_body(open[third], close[third]) < avg_body
        {
            out[i] = PATTERN_NONE;
            continue;
        }
        if close[third] < body_midpoint(open[first], close[first]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BULLISH;
    }

    Ok(())
}

/// Lookback for `CDL_EVENING_DOJI_STAR` pattern.
#[must_use]
pub const fn cdl_evening_doji_star_lookback() -> usize {
    cdl_morning_star_lookback()
}

/// Minimum length for `CDL_EVENING_DOJI_STAR`.
#[must_use]
pub const fn cdl_evening_doji_star_min_len() -> usize {
    cdl_morning_star_min_len()
}

/// Evening Doji Star pattern recognition.
///
/// Bearish version of Morning Doji Star.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_evening_doji_star<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_evening_doji_star_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Evening Doji Star pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_evening_doji_star_into<T: SeriesElement>(
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
    if len < cdl_evening_doji_star_min_len() {
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

    let lookback = cdl_evening_doji_star_lookback();
    let doji_threshold = f64_to_t(0.1);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        if !is_uptrend(close, first, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // First: long bullish
        if !is_bullish(open[first], close[first]) || real_body(open[first], close[first]) < avg_body
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Second: doji, gaps up
        if !is_doji(
            open[second],
            high[second],
            low[second],
            close[second],
            doji_threshold,
        ) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if low[second] <= close[first] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Third: long bearish closes into first
        if !is_bearish(open[third], close[third]) || real_body(open[third], close[third]) < avg_body
        {
            out[i] = PATTERN_NONE;
            continue;
        }
        if close[third] > body_midpoint(open[first], close[first]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BEARISH;
    }

    Ok(())
}

/// Lookback for `CDL_ABANDONED_BABY` pattern.
#[must_use]
pub const fn cdl_abandoned_baby_lookback() -> usize {
    cdl_morning_star_lookback()
}

/// Minimum length for `CDL_ABANDONED_BABY`.
#[must_use]
pub const fn cdl_abandoned_baby_min_len() -> usize {
    cdl_morning_star_min_len()
}

/// Abandoned Baby pattern recognition.
///
/// A rare reversal pattern: doji star with gaps on both sides.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_abandoned_baby<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_abandoned_baby_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Abandoned Baby pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_abandoned_baby_into<T: SeriesElement>(
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
    if len < cdl_abandoned_baby_min_len() {
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

    let lookback = cdl_abandoned_baby_lookback();
    let doji_threshold = f64_to_t(0.1);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // Second must be doji
        if !is_doji(
            open[second],
            high[second],
            low[second],
            close[second],
            doji_threshold,
        ) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Bullish abandoned baby: downtrend
        if is_downtrend(close, first, TREND_LOOKBACK) {
            // First: long bearish
            if !is_bearish(open[first], close[first])
                || real_body(open[first], close[first]) < avg_body
            {
                out[i] = PATTERN_NONE;
                continue;
            }

            // Gap down from first to second
            if high[second] >= low[first] {
                out[i] = PATTERN_NONE;
                continue;
            }

            // Third: long bullish
            if !is_bullish(open[third], close[third])
                || real_body(open[third], close[third]) < avg_body
            {
                out[i] = PATTERN_NONE;
                continue;
            }

            // Gap up from second to third
            if low[third] <= high[second] {
                out[i] = PATTERN_NONE;
                continue;
            }

            out[i] = PATTERN_BULLISH;
            continue;
        }

        // Bearish abandoned baby: uptrend
        if is_uptrend(close, first, TREND_LOOKBACK) {
            // First: long bullish
            if !is_bullish(open[first], close[first])
                || real_body(open[first], close[first]) < avg_body
            {
                out[i] = PATTERN_NONE;
                continue;
            }

            // Gap up from first to second
            if low[second] <= high[first] {
                out[i] = PATTERN_NONE;
                continue;
            }

            // Third: long bearish
            if !is_bearish(open[third], close[third])
                || real_body(open[third], close[third]) < avg_body
            {
                out[i] = PATTERN_NONE;
                continue;
            }

            // Gap down from second to third
            if high[third] >= low[second] {
                out[i] = PATTERN_NONE;
                continue;
            }

            out[i] = PATTERN_BEARISH;
            continue;
        }

        out[i] = PATTERN_NONE;
    }

    Ok(())
}

// ============================================================================
// Three White Soldiers / Three Black Crows
// ============================================================================

/// Lookback for `CDL_3WHITE_SOLDIERS` pattern.
#[must_use]
pub const fn cdl_3white_soldiers_lookback() -> usize {
    AVG_LOOKBACK + 2
}

/// Minimum length for `CDL_3WHITE_SOLDIERS`.
#[must_use]
pub const fn cdl_3white_soldiers_min_len() -> usize {
    cdl_3white_soldiers_lookback() + 1
}

/// Three White Soldiers pattern recognition.
///
/// Three long bullish candles, each opening within the previous body
/// and closing near its high.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_3white_soldiers<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_3white_soldiers_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Three White Soldiers pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_3white_soldiers_into<T: SeriesElement>(
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
    if len < cdl_3white_soldiers_min_len() {
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

    let lookback = cdl_3white_soldiers_lookback();
    let shadow_max = f64_to_t(0.2);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // All three must be bullish
        if !is_bullish(open[first], close[first])
            || !is_bullish(open[second], close[second])
            || !is_bullish(open[third], close[third])
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        // All three should have significant bodies
        if real_body(open[first], close[first]) < avg_body * f64_to_t(0.8)
            || real_body(open[second], close[second]) < avg_body * f64_to_t(0.8)
            || real_body(open[third], close[third]) < avg_body * f64_to_t(0.8)
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Each opens within previous body (between body_bottom and body_top)
        #[allow(clippy::suspicious_operation_groupings)]
        if open[second] < open[first] || open[second] > close[first] {
            out[i] = PATTERN_NONE;
            continue;
        }
        #[allow(clippy::suspicious_operation_groupings)]
        if open[third] < open[second] || open[third] > close[second] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Each closes higher than previous
        if close[second] <= close[first] || close[third] <= close[second] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Upper shadows should be short
        let range1 = candle_range(high[first], low[first]);
        let range2 = candle_range(high[second], low[second]);
        let range3 = candle_range(high[third], low[third]);

        if range1 > T::zero()
            && upper_shadow(open[first], high[first], close[first]) > range1 * shadow_max
        {
            out[i] = PATTERN_NONE;
            continue;
        }
        if range2 > T::zero()
            && upper_shadow(open[second], high[second], close[second]) > range2 * shadow_max
        {
            out[i] = PATTERN_NONE;
            continue;
        }
        if range3 > T::zero()
            && upper_shadow(open[third], high[third], close[third]) > range3 * shadow_max
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BULLISH;
    }

    Ok(())
}

/// Lookback for `CDL_3BLACK_CROWS` pattern.
#[must_use]
pub const fn cdl_3black_crows_lookback() -> usize {
    cdl_3white_soldiers_lookback()
}

/// Minimum length for `CDL_3BLACK_CROWS`.
#[must_use]
pub const fn cdl_3black_crows_min_len() -> usize {
    cdl_3white_soldiers_min_len()
}

/// Three Black Crows pattern recognition.
///
/// Bearish version of Three White Soldiers.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_3black_crows<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_3black_crows_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Three Black Crows pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_3black_crows_into<T: SeriesElement>(
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
    if len < cdl_3black_crows_min_len() {
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

    let lookback = cdl_3black_crows_lookback();
    let shadow_max = f64_to_t(0.2);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // All three must be bearish
        if !is_bearish(open[first], close[first])
            || !is_bearish(open[second], close[second])
            || !is_bearish(open[third], close[third])
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        // All should have significant bodies
        if real_body(open[first], close[first]) < avg_body * f64_to_t(0.8)
            || real_body(open[second], close[second]) < avg_body * f64_to_t(0.8)
            || real_body(open[third], close[third]) < avg_body * f64_to_t(0.8)
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Each opens within previous body (between body_bottom and body_top)
        #[allow(clippy::suspicious_operation_groupings)]
        if open[second] > open[first] || open[second] < close[first] {
            out[i] = PATTERN_NONE;
            continue;
        }
        #[allow(clippy::suspicious_operation_groupings)]
        if open[third] > open[second] || open[third] < close[second] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Each closes lower
        if close[second] >= close[first] || close[third] >= close[second] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Lower shadows should be short
        let range1 = candle_range(high[first], low[first]);
        let range2 = candle_range(high[second], low[second]);
        let range3 = candle_range(high[third], low[third]);

        if range1 > T::zero()
            && lower_shadow(open[first], low[first], close[first]) > range1 * shadow_max
        {
            out[i] = PATTERN_NONE;
            continue;
        }
        if range2 > T::zero()
            && lower_shadow(open[second], low[second], close[second]) > range2 * shadow_max
        {
            out[i] = PATTERN_NONE;
            continue;
        }
        if range3 > T::zero()
            && lower_shadow(open[third], low[third], close[third]) > range3 * shadow_max
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BEARISH;
    }

    Ok(())
}

// ============================================================================
// Three Inside / Three Outside
// ============================================================================

/// Lookback for `CDL_3INSIDE` pattern.
#[must_use]
pub const fn cdl_3inside_lookback() -> usize {
    AVG_LOOKBACK + 2
}

/// Minimum length for `CDL_3INSIDE`.
#[must_use]
pub const fn cdl_3inside_min_len() -> usize {
    cdl_3inside_lookback() + 1
}

/// Three Inside Up/Down pattern recognition.
///
/// A Harami followed by a confirmation candle.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_3inside<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_3inside_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Three Inside Up/Down pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_3inside_into<T: SeriesElement>(
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
    if len < cdl_3inside_min_len() {
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

    let lookback = cdl_3inside_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // Three Inside Up: first bearish, second bullish inside, third bullish closes higher
        if is_bearish(open[first], close[first]) && real_body(open[first], close[first]) >= avg_body
        {
            // Second is small and inside first's body
            let first_top = body_top(open[first], close[first]);
            let first_bottom = body_bottom(open[first], close[first]);
            let second_top = body_top(open[second], close[second]);
            let second_bottom = body_bottom(open[second], close[second]);

            if second_bottom >= first_bottom
                && second_top <= first_top
                && is_bullish(open[third], close[third])
                && close[third] > first_top
            {
                out[i] = PATTERN_BULLISH;
                continue;
            }
        }

        // Three Inside Down: first bullish, second bearish inside, third bearish closes lower
        if is_bullish(open[first], close[first]) && real_body(open[first], close[first]) >= avg_body
        {
            let first_top = body_top(open[first], close[first]);
            let first_bottom = body_bottom(open[first], close[first]);
            let second_top = body_top(open[second], close[second]);
            let second_bottom = body_bottom(open[second], close[second]);

            if second_bottom >= first_bottom
                && second_top <= first_top
                && is_bearish(open[third], close[third])
                && close[third] < first_bottom
            {
                out[i] = PATTERN_BEARISH;
                continue;
            }
        }

        out[i] = PATTERN_NONE;
    }

    Ok(())
}

/// Lookback for `CDL_3OUTSIDE` pattern.
#[must_use]
pub const fn cdl_3outside_lookback() -> usize {
    cdl_3inside_lookback()
}

/// Minimum length for `CDL_3OUTSIDE`.
#[must_use]
pub const fn cdl_3outside_min_len() -> usize {
    cdl_3inside_min_len()
}

/// Three Outside Up/Down pattern recognition.
///
/// An Engulfing followed by a confirmation candle.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_3outside<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_3outside_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Three Outside Up/Down pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_3outside_into<T: SeriesElement>(
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
    if len < cdl_3outside_min_len() {
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

    let lookback = cdl_3outside_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        // Three Outside Up: engulfing bullish + third closes higher
        if is_bearish(open[first], close[first]) && is_bullish(open[second], close[second]) {
            let first_top = body_top(open[first], close[first]);
            let first_bottom = body_bottom(open[first], close[first]);
            let second_top = body_top(open[second], close[second]);
            let second_bottom = body_bottom(open[second], close[second]);

            // Second engulfs first
            if second_bottom < first_bottom
                && second_top > first_top
                && close[third] > close[second]
            {
                out[i] = PATTERN_BULLISH;
                continue;
            }
        }

        // Three Outside Down: engulfing bearish + third closes lower
        if is_bullish(open[first], close[first]) && is_bearish(open[second], close[second]) {
            let first_top = body_top(open[first], close[first]);
            let first_bottom = body_bottom(open[first], close[first]);
            let second_top = body_top(open[second], close[second]);
            let second_bottom = body_bottom(open[second], close[second]);

            if second_bottom < first_bottom
                && second_top > first_top
                && close[third] < close[second]
            {
                out[i] = PATTERN_BEARISH;
                continue;
            }
        }

        out[i] = PATTERN_NONE;
    }

    Ok(())
}

// ============================================================================
// Other Three-Candle Patterns
// ============================================================================

/// Lookback for `CDL_3LINE_STRIKE` pattern.
#[must_use]
pub const fn cdl_3line_strike_lookback() -> usize {
    AVG_LOOKBACK + 3
}

/// Minimum length for `CDL_3LINE_STRIKE`.
#[must_use]
pub const fn cdl_3line_strike_min_len() -> usize {
    cdl_3line_strike_lookback() + 1
}

/// Three Line Strike pattern recognition.
///
/// Three consecutive same-color candles followed by a large opposite candle
/// that engulfs all three.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_3line_strike<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_3line_strike_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Three Line Strike pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_3line_strike_into<T: SeriesElement>(
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
    if len < cdl_3line_strike_min_len() {
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

    let lookback = cdl_3line_strike_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let c1 = i - 3;
        let c2 = i - 2;
        let c3 = i - 1;
        let c4 = i;

        // Bullish: three bearish + one bullish that engulfs all
        if is_bearish(open[c1], close[c1])
            && is_bearish(open[c2], close[c2])
            && is_bearish(open[c3], close[c3])
            && is_bullish(open[c4], close[c4])
            && close[c2] < close[c1]
            && close[c3] < close[c2]
            && open[c4] <= close[c3]
            && close[c4] >= open[c1]
        {
            out[i] = PATTERN_BULLISH;
            continue;
        }

        // Bearish: three bullish + one bearish that engulfs all
        if is_bullish(open[c1], close[c1])
            && is_bullish(open[c2], close[c2])
            && is_bullish(open[c3], close[c3])
            && is_bearish(open[c4], close[c4])
            && close[c2] > close[c1]
            && close[c3] > close[c2]
            && open[c4] >= close[c3]
            && close[c4] <= open[c1]
        {
            out[i] = PATTERN_BEARISH;
            continue;
        }

        out[i] = PATTERN_NONE;
    }

    Ok(())
}

/// Lookback for `CDL_3STARS_IN_SOUTH` pattern.
#[must_use]
pub const fn cdl_3stars_in_south_lookback() -> usize {
    AVG_LOOKBACK + 2
}

/// Minimum length for `CDL_3STARS_IN_SOUTH`.
#[must_use]
pub const fn cdl_3stars_in_south_min_len() -> usize {
    cdl_3stars_in_south_lookback() + 1
}

/// Three Stars in the South pattern recognition.
///
/// Rare bullish reversal pattern with three bearish candles showing
/// diminishing selling pressure.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_3stars_in_south<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_3stars_in_south_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Three Stars in the South pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_3stars_in_south_into<T: SeriesElement>(
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
    if len < cdl_3stars_in_south_min_len() {
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

    let lookback = cdl_3stars_in_south_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        // All three must be bearish
        if !is_bearish(open[first], close[first])
            || !is_bearish(open[second], close[second])
            || !is_bearish(open[third], close[third])
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        // First: long body with long lower shadow
        let first_body = real_body(open[first], close[first]);
        let first_lower = lower_shadow(open[first], low[first], close[first]);
        if first_lower < first_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Second: body inside first, lower shadow makes new low but closes higher
        if low[second] >= low[first] || close[second] <= close[first] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Third: short marubozu inside second's range
        let third_range = candle_range(high[third], low[third]);
        if third_range > T::zero() {
            let third_upper = upper_shadow(open[third], high[third], close[third]);
            let third_lower = lower_shadow(open[third], low[third], close[third]);
            if third_upper > third_range * f64_to_t(0.1)
                || third_lower > third_range * f64_to_t(0.1)
            {
                out[i] = PATTERN_NONE;
                continue;
            }
        }
        if high[third] > high[second] || low[third] < low[second] {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BULLISH;
    }

    Ok(())
}

/// Lookback for `CDL_TRISTAR` pattern.
#[must_use]
pub const fn cdl_tristar_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK + 2
}

/// Minimum length for `CDL_TRISTAR`.
#[must_use]
pub const fn cdl_tristar_min_len() -> usize {
    cdl_tristar_lookback() + 1
}

/// Tristar pattern recognition.
///
/// Three consecutive dojis with the middle one gapping.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_tristar<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_tristar_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Tristar pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_tristar_into<T: SeriesElement>(
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
    if len < cdl_tristar_min_len() {
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

    let lookback = cdl_tristar_lookback();
    let doji_threshold = f64_to_t(0.1);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        // All three must be dojis
        if !is_doji(
            open[first],
            high[first],
            low[first],
            close[first],
            doji_threshold,
        ) || !is_doji(
            open[second],
            high[second],
            low[second],
            close[second],
            doji_threshold,
        ) || !is_doji(
            open[third],
            high[third],
            low[third],
            close[third],
            doji_threshold,
        ) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Middle doji should gap from first and third
        let first_mid = body_midpoint(open[first], close[first]);
        let second_mid = body_midpoint(open[second], close[second]);
        let third_mid = body_midpoint(open[third], close[third]);

        // Bullish tristar: in downtrend, middle gaps down
        if is_downtrend(close, first, TREND_LOOKBACK)
            && second_mid < first_mid
            && second_mid < third_mid
        {
            out[i] = PATTERN_BULLISH;
            continue;
        }

        // Bearish tristar: in uptrend, middle gaps up
        if is_uptrend(close, first, TREND_LOOKBACK)
            && second_mid > first_mid
            && second_mid > third_mid
        {
            out[i] = PATTERN_BEARISH;
            continue;
        }

        out[i] = PATTERN_NONE;
    }

    Ok(())
}

/// Lookback for `CDL_IDENTICAL_3CROWS` pattern.
#[must_use]
pub const fn cdl_identical_3crows_lookback() -> usize {
    AVG_LOOKBACK + 2
}

/// Minimum length for `CDL_IDENTICAL_3CROWS`.
#[must_use]
pub const fn cdl_identical_3crows_min_len() -> usize {
    cdl_identical_3crows_lookback() + 1
}

/// Identical Three Crows pattern recognition.
///
/// Three Black Crows where each opens at or near the previous close.
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_identical_3crows<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_identical_3crows_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Identical Three Crows pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_identical_3crows_into<T: SeriesElement>(
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
    if len < cdl_identical_3crows_min_len() {
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

    let lookback = cdl_identical_3crows_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let first = i - 2;
        let second = i - 1;
        let third = i;

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);
        let avg_range = average_range(high, low, i, AVG_LOOKBACK);
        let tolerance = avg_range * f64_to_t(0.05);

        // All must be bearish with long bodies
        if !is_bearish(open[first], close[first])
            || !is_bearish(open[second], close[second])
            || !is_bearish(open[third], close[third])
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        if real_body(open[first], close[first]) < avg_body
            || real_body(open[second], close[second]) < avg_body
            || real_body(open[third], close[third]) < avg_body
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Each opens at previous close
        if !approx_equal(open[second], close[first], tolerance)
            || !approx_equal(open[third], close[second], tolerance)
        {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Each closes lower
        if close[second] >= close[first] || close[third] >= close[second] {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BEARISH;
    }

    Ok(())
}

// ============================================================================
// Remaining patterns (simplified implementations)
// ============================================================================

macro_rules! define_simple_pattern {
    ($name:ident, $name_into:ident, $lookback_fn:ident, $min_len_fn:ident, $lookback:expr, $doc:expr) => {
        #[doc = concat!("Lookback for ", $doc, " pattern.")]
        pub const fn $lookback_fn() -> usize {
            $lookback
        }

        #[doc = concat!("Minimum length for ", $doc, " pattern.")]
        pub const fn $min_len_fn() -> usize {
            $lookback + 1
        }

        #[doc = concat!($doc, " pattern recognition (placeholder).")]
        ///
        /// # Errors
        ///
        /// Returns an error if:
        /// - The input arrays are empty (`Error::EmptyInput`)
        /// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
        /// - There is insufficient data for the lookback (`Error::InsufficientData`)
        pub fn $name<T: SeriesElement>(
            open: &[T],
            high: &[T],
            low: &[T],
            close: &[T],
        ) -> Result<Vec<i32>> {
            let len = open.len();
            if len == 0 {
                return Err(Error::EmptyInput);
            }
            if high.len() != len || low.len() != len || close.len() != len {
                return Err(Error::LengthMismatch {
                    description: "OHLC arrays have different lengths".to_string(),
                });
            }
            if len < $min_len_fn() {
                return Err(Error::InsufficientData {
                    required: 0,
                    actual: len,
                    indicator: "candlestick",
                });
            }

            let mut out = vec![0i32; len];
            $name_into(open, high, low, close, &mut out)?;
            Ok(out)
        }

        #[doc = concat!($doc, " pattern recognition (zero-copy output).")]
        ///
        /// # Errors
        ///
        /// Returns an error if:
        /// - The input arrays are empty (`Error::EmptyInput`)
        /// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
        /// - There is insufficient data for the lookback (`Error::InsufficientData`)
        /// - The output buffer is too small (`Error::BufferTooSmall`)
        pub fn $name_into<T: SeriesElement>(
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
            if len < $min_len_fn() {
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

            // Placeholder: returns no pattern
            // These would need full implementation
            for i in 0..len {
                out[i] = PATTERN_NONE;
            }

            Ok(())
        }
    };
}

// Remaining patterns with placeholder implementations
define_simple_pattern!(
    cdl_stick_sandwich,
    cdl_stick_sandwich_into,
    cdl_stick_sandwich_lookback,
    cdl_stick_sandwich_min_len,
    AVG_LOOKBACK + 2,
    "Stick Sandwich"
);
define_simple_pattern!(
    cdl_unique_3river,
    cdl_unique_3river_into,
    cdl_unique_3river_lookback,
    cdl_unique_3river_min_len,
    AVG_LOOKBACK + 2,
    "Unique 3 River"
);
define_simple_pattern!(
    cdl_advance_block,
    cdl_advance_block_into,
    cdl_advance_block_lookback,
    cdl_advance_block_min_len,
    AVG_LOOKBACK + 2,
    "Advance Block"
);
define_simple_pattern!(
    cdl_stalled_pattern,
    cdl_stalled_pattern_into,
    cdl_stalled_pattern_lookback,
    cdl_stalled_pattern_min_len,
    AVG_LOOKBACK + 2,
    "Stalled Pattern"
);
define_simple_pattern!(
    cdl_tasuki_gap,
    cdl_tasuki_gap_into,
    cdl_tasuki_gap_lookback,
    cdl_tasuki_gap_min_len,
    AVG_LOOKBACK + 2,
    "Tasuki Gap"
);
define_simple_pattern!(
    cdl_upside_gap_2crows,
    cdl_upside_gap_2crows_into,
    cdl_upside_gap_2crows_lookback,
    cdl_upside_gap_2crows_min_len,
    AVG_LOOKBACK + 2,
    "Upside Gap Two Crows"
);
define_simple_pattern!(
    cdl_gap_side_side_white,
    cdl_gap_side_side_white_into,
    cdl_gap_side_side_white_lookback,
    cdl_gap_side_side_white_min_len,
    AVG_LOOKBACK + 1,
    "Gap Side Side White"
);
define_simple_pattern!(
    cdl_breakaway,
    cdl_breakaway_into,
    cdl_breakaway_lookback,
    cdl_breakaway_min_len,
    AVG_LOOKBACK + 4,
    "Breakaway"
);
define_simple_pattern!(
    cdl_ladder_bottom,
    cdl_ladder_bottom_into,
    cdl_ladder_bottom_lookback,
    cdl_ladder_bottom_min_len,
    AVG_LOOKBACK + 4,
    "Ladder Bottom"
);
define_simple_pattern!(
    cdl_mat_hold,
    cdl_mat_hold_into,
    cdl_mat_hold_lookback,
    cdl_mat_hold_min_len,
    AVG_LOOKBACK + 4,
    "Mat Hold"
);
define_simple_pattern!(
    cdl_rise_fall_3methods,
    cdl_rise_fall_3methods_into,
    cdl_rise_fall_3methods_lookback,
    cdl_rise_fall_3methods_min_len,
    AVG_LOOKBACK + 4,
    "Rising/Falling Three Methods"
);
define_simple_pattern!(
    cdl_concealing_baby_swallow,
    cdl_concealing_baby_swallow_into,
    cdl_concealing_baby_swallow_lookback,
    cdl_concealing_baby_swallow_min_len,
    AVG_LOOKBACK + 3,
    "Concealing Baby Swallow"
);
define_simple_pattern!(
    cdl_xside_gap_3methods,
    cdl_xside_gap_3methods_into,
    cdl_xside_gap_3methods_lookback,
    cdl_xside_gap_3methods_min_len,
    AVG_LOOKBACK + 2,
    "Up/Down-Gap Side Side Three Methods"
);

#[cfg(test)]
mod tests {
    use super::*;

    fn make_morning_star() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        // Create downtrend + morning star
        let n = 25;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        // Downtrend
        for i in 0..n - 3 {
            let base = 120.0 - (i as f64) * 2.0;
            open.push(base);
            high.push(base + 2.0);
            low.push(base - 4.0);
            close.push(base - 2.0);
        }

        // First: long bearish
        let base = 120.0 - ((n - 3) as f64) * 2.0;
        open.push(base);
        high.push(base + 1.0);
        low.push(base - 12.0);
        close.push(base - 10.0);

        // Second: small star (gaps down)
        open.push(base - 12.0);
        high.push(base - 11.0);
        low.push(base - 14.0);
        close.push(base - 12.5);

        // Third: long bullish
        open.push(base - 11.0);
        high.push(base - 2.0);
        low.push(base - 12.0);
        close.push(base - 3.0);

        (open, high, low, close)
    }

    #[test]
    fn test_cdl_morning_star() {
        let (open, high, low, close) = make_morning_star();
        let result = cdl_morning_star(&open, &high, &low, &close).unwrap();
        assert_eq!(result[result.len() - 1], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_3white_soldiers() {
        let mut open = vec![100.0; 13];
        let mut high = vec![105.0; 13];
        let mut low = vec![95.0; 13];
        let mut close = vec![103.0; 13];

        // Three white soldiers at the end
        open[10] = 100.0;
        high[10] = 110.0;
        low[10] = 99.0;
        close[10] = 109.0;

        open[11] = 105.0;
        high[11] = 118.0;
        low[11] = 104.0;
        close[11] = 117.0;

        open[12] = 112.0;
        high[12] = 126.0;
        low[12] = 111.0;
        close[12] = 125.0;

        let result = cdl_3white_soldiers(&open, &high, &low, &close).unwrap();
        assert_eq!(result[12], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_3black_crows() {
        let mut open = vec![100.0; 13];
        let mut high = vec![105.0; 13];
        let mut low = vec![95.0; 13];
        let mut close = vec![103.0; 13];

        // Three black crows at the end
        open[10] = 110.0;
        high[10] = 111.0;
        low[10] = 100.0;
        close[10] = 101.0;

        open[11] = 105.0;
        high[11] = 106.0;
        low[11] = 92.0;
        close[11] = 93.0;

        open[12] = 97.0;
        high[12] = 98.0;
        low[12] = 84.0;
        close[12] = 85.0;

        let result = cdl_3black_crows(&open, &high, &low, &close).unwrap();
        assert_eq!(result[12], PATTERN_BEARISH);
    }

    #[test]
    fn test_empty_input() {
        let empty: Vec<f64> = vec![];
        assert!(cdl_morning_star(&empty, &empty, &empty, &empty).is_err());
    }

    #[test]
    fn test_three_candle_wrappers_smoke_surface() {
        let n = 700;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        for i in 0..n {
            let base = 400.0 - (i as f64) * 0.1;
            open.push(base);
            close.push(base - 0.08);
            high.push(base + 7.0);
            low.push(base - 7.0);
        }

        macro_rules! assert_len_ok {
            ($expr:expr) => {{
                let out = $expr.unwrap();
                assert_eq!(out.len(), n);
            }};
        }

        assert_len_ok!(cdl_morning_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_evening_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_morning_doji_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_evening_doji_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_abandoned_baby(&open, &high, &low, &close));
        assert_len_ok!(cdl_3white_soldiers(&open, &high, &low, &close));
        assert_len_ok!(cdl_3black_crows(&open, &high, &low, &close));
        assert_len_ok!(cdl_3inside(&open, &high, &low, &close));
        assert_len_ok!(cdl_3outside(&open, &high, &low, &close));
        assert_len_ok!(cdl_3line_strike(&open, &high, &low, &close));
        assert_len_ok!(cdl_3stars_in_south(&open, &high, &low, &close));
        assert_len_ok!(cdl_tristar(&open, &high, &low, &close));
        assert_len_ok!(cdl_identical_3crows(&open, &high, &low, &close));
        assert_len_ok!(cdl_advance_block(&open, &high, &low, &close));
        assert_len_ok!(cdl_stalled_pattern(&open, &high, &low, &close));
        assert_len_ok!(cdl_unique_3river(&open, &high, &low, &close));
        assert_len_ok!(cdl_stick_sandwich(&open, &high, &low, &close));
        assert_len_ok!(cdl_tasuki_gap(&open, &high, &low, &close));
        assert_len_ok!(cdl_upside_gap_2crows(&open, &high, &low, &close));
        assert_len_ok!(cdl_gap_side_side_white(&open, &high, &low, &close));
        assert_len_ok!(cdl_breakaway(&open, &high, &low, &close));
        assert_len_ok!(cdl_ladder_bottom(&open, &high, &low, &close));
        assert_len_ok!(cdl_mat_hold(&open, &high, &low, &close));
        assert_len_ok!(cdl_rise_fall_3methods(&open, &high, &low, &close));
        assert_len_ok!(cdl_concealing_baby_swallow(&open, &high, &low, &close));
        assert_len_ok!(cdl_xside_gap_3methods(&open, &high, &low, &close));
    }

    #[test]
    fn test_three_candle_error_surface_all_patterns() {
        type ThreeFn = fn(&[f64], &[f64], &[f64], &[f64]) -> crate::error::Result<Vec<i32>>;
        type ThreeIntoFn =
            fn(&[f64], &[f64], &[f64], &[f64], &mut [i32]) -> crate::error::Result<()>;

        let n = 96;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        for i in 0..n {
            let base = 220.0 - (i as f64) * 0.1;
            open.push(base);
            close.push(base - 0.06);
            high.push(base + 5.0);
            low.push(base - 5.0);
        }

        let high_short = high[..(n - 1)].to_vec();
        let low_short = low[..(n - 1)].to_vec();
        let close_short = close[..(n - 1)].to_vec();
        let empty: Vec<f64> = vec![];

        let wrappers: [(&str, ThreeFn); 26] = [
            ("cdl_morning_star", cdl_morning_star),
            ("cdl_evening_star", cdl_evening_star),
            ("cdl_morning_doji_star", cdl_morning_doji_star),
            ("cdl_evening_doji_star", cdl_evening_doji_star),
            ("cdl_abandoned_baby", cdl_abandoned_baby),
            ("cdl_3white_soldiers", cdl_3white_soldiers),
            ("cdl_3black_crows", cdl_3black_crows),
            ("cdl_3inside", cdl_3inside),
            ("cdl_3outside", cdl_3outside),
            ("cdl_3line_strike", cdl_3line_strike),
            ("cdl_3stars_in_south", cdl_3stars_in_south),
            ("cdl_tristar", cdl_tristar),
            ("cdl_identical_3crows", cdl_identical_3crows),
            ("cdl_advance_block", cdl_advance_block),
            ("cdl_stalled_pattern", cdl_stalled_pattern),
            ("cdl_unique_3river", cdl_unique_3river),
            ("cdl_stick_sandwich", cdl_stick_sandwich),
            ("cdl_tasuki_gap", cdl_tasuki_gap),
            ("cdl_upside_gap_2crows", cdl_upside_gap_2crows),
            ("cdl_gap_side_side_white", cdl_gap_side_side_white),
            ("cdl_breakaway", cdl_breakaway),
            ("cdl_ladder_bottom", cdl_ladder_bottom),
            ("cdl_mat_hold", cdl_mat_hold),
            ("cdl_rise_fall_3methods", cdl_rise_fall_3methods),
            ("cdl_concealing_baby_swallow", cdl_concealing_baby_swallow),
            ("cdl_xside_gap_3methods", cdl_xside_gap_3methods),
        ];

        for (name, f) in wrappers {
            assert!(f(&empty, &empty, &empty, &empty).is_err(), "{name}");
            assert!(f(&open, &high_short, &low, &close).is_err(), "{name}");
            assert!(f(&open, &high, &low_short, &close).is_err(), "{name}");
            assert!(f(&open, &high, &low, &close_short).is_err(), "{name}");
        }

        let into_fns: [(&str, ThreeIntoFn); 26] = [
            ("cdl_morning_star_into", cdl_morning_star_into),
            ("cdl_evening_star_into", cdl_evening_star_into),
            ("cdl_morning_doji_star_into", cdl_morning_doji_star_into),
            ("cdl_evening_doji_star_into", cdl_evening_doji_star_into),
            ("cdl_abandoned_baby_into", cdl_abandoned_baby_into),
            ("cdl_3white_soldiers_into", cdl_3white_soldiers_into),
            ("cdl_3black_crows_into", cdl_3black_crows_into),
            ("cdl_3inside_into", cdl_3inside_into),
            ("cdl_3outside_into", cdl_3outside_into),
            ("cdl_3line_strike_into", cdl_3line_strike_into),
            ("cdl_3stars_in_south_into", cdl_3stars_in_south_into),
            ("cdl_tristar_into", cdl_tristar_into),
            ("cdl_identical_3crows_into", cdl_identical_3crows_into),
            ("cdl_advance_block_into", cdl_advance_block_into),
            ("cdl_stalled_pattern_into", cdl_stalled_pattern_into),
            ("cdl_unique_3river_into", cdl_unique_3river_into),
            ("cdl_stick_sandwich_into", cdl_stick_sandwich_into),
            ("cdl_tasuki_gap_into", cdl_tasuki_gap_into),
            ("cdl_upside_gap_2crows_into", cdl_upside_gap_2crows_into),
            ("cdl_gap_side_side_white_into", cdl_gap_side_side_white_into),
            ("cdl_breakaway_into", cdl_breakaway_into),
            ("cdl_ladder_bottom_into", cdl_ladder_bottom_into),
            ("cdl_mat_hold_into", cdl_mat_hold_into),
            ("cdl_rise_fall_3methods_into", cdl_rise_fall_3methods_into),
            (
                "cdl_concealing_baby_swallow_into",
                cdl_concealing_baby_swallow_into,
            ),
            ("cdl_xside_gap_3methods_into", cdl_xside_gap_3methods_into),
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
    fn test_three_candle_f32_surface_matrix() {
        let n = 360;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        for i in 0..n {
            let base = 260.0_f32 - (i as f32) * 0.09;
            open.push(base);
            close.push(base - 0.07);
            high.push(base + 4.8);
            low.push(base - 4.8);
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

        assert_len_ok!(cdl_morning_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_evening_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_morning_doji_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_evening_doji_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_abandoned_baby(&open, &high, &low, &close));
        assert_len_ok!(cdl_3white_soldiers(&open, &high, &low, &close));
        assert_len_ok!(cdl_3black_crows(&open, &high, &low, &close));
        assert_len_ok!(cdl_3inside(&open, &high, &low, &close));
        assert_len_ok!(cdl_3outside(&open, &high, &low, &close));
        assert_len_ok!(cdl_3line_strike(&open, &high, &low, &close));
        assert_len_ok!(cdl_3stars_in_south(&open, &high, &low, &close));
        assert_len_ok!(cdl_tristar(&open, &high, &low, &close));
        assert_len_ok!(cdl_identical_3crows(&open, &high, &low, &close));
        assert_len_ok!(cdl_advance_block(&open, &high, &low, &close));
        assert_len_ok!(cdl_stalled_pattern(&open, &high, &low, &close));
        assert_len_ok!(cdl_unique_3river(&open, &high, &low, &close));
        assert_len_ok!(cdl_stick_sandwich(&open, &high, &low, &close));
        assert_len_ok!(cdl_tasuki_gap(&open, &high, &low, &close));
        assert_len_ok!(cdl_upside_gap_2crows(&open, &high, &low, &close));
        assert_len_ok!(cdl_gap_side_side_white(&open, &high, &low, &close));
        assert_len_ok!(cdl_breakaway(&open, &high, &low, &close));
        assert_len_ok!(cdl_ladder_bottom(&open, &high, &low, &close));
        assert_len_ok!(cdl_mat_hold(&open, &high, &low, &close));
        assert_len_ok!(cdl_rise_fall_3methods(&open, &high, &low, &close));
        assert_len_ok!(cdl_concealing_baby_swallow(&open, &high, &low, &close));
        assert_len_ok!(cdl_xside_gap_3methods(&open, &high, &low, &close));

        assert_into_ok!(cdl_morning_star_into);
        assert_into_ok!(cdl_evening_star_into);
        assert_into_ok!(cdl_morning_doji_star_into);
        assert_into_ok!(cdl_evening_doji_star_into);
        assert_into_ok!(cdl_abandoned_baby_into);
        assert_into_ok!(cdl_3white_soldiers_into);
        assert_into_ok!(cdl_3black_crows_into);
        assert_into_ok!(cdl_3inside_into);
        assert_into_ok!(cdl_3outside_into);
        assert_into_ok!(cdl_3line_strike_into);
        assert_into_ok!(cdl_3stars_in_south_into);
        assert_into_ok!(cdl_tristar_into);
        assert_into_ok!(cdl_identical_3crows_into);
        assert_into_ok!(cdl_advance_block_into);
        assert_into_ok!(cdl_stalled_pattern_into);
        assert_into_ok!(cdl_unique_3river_into);
        assert_into_ok!(cdl_stick_sandwich_into);
        assert_into_ok!(cdl_tasuki_gap_into);
        assert_into_ok!(cdl_upside_gap_2crows_into);
        assert_into_ok!(cdl_gap_side_side_white_into);
        assert_into_ok!(cdl_breakaway_into);
        assert_into_ok!(cdl_ladder_bottom_into);
        assert_into_ok!(cdl_mat_hold_into);
        assert_into_ok!(cdl_rise_fall_3methods_into);
        assert_into_ok!(cdl_concealing_baby_swallow_into);
        assert_into_ok!(cdl_xside_gap_3methods_into);
    }

    #[test]
    fn test_three_candle_f32_error_surface_all_patterns() {
        type ThreeFn = fn(&[f32], &[f32], &[f32], &[f32]) -> crate::error::Result<Vec<i32>>;
        type ThreeIntoFn =
            fn(&[f32], &[f32], &[f32], &[f32], &mut [i32]) -> crate::error::Result<()>;

        let n = 96;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        for i in 0..n {
            let base = 230.0_f32 - (i as f32) * 0.1;
            open.push(base);
            close.push(base - 0.06);
            high.push(base + 5.0);
            low.push(base - 5.0);
        }

        let high_short = high[..(n - 1)].to_vec();
        let low_short = low[..(n - 1)].to_vec();
        let close_short = close[..(n - 1)].to_vec();
        let empty: Vec<f32> = vec![];

        let wrappers: [ThreeFn; 26] = [
            cdl_morning_star,
            cdl_evening_star,
            cdl_morning_doji_star,
            cdl_evening_doji_star,
            cdl_abandoned_baby,
            cdl_3white_soldiers,
            cdl_3black_crows,
            cdl_3inside,
            cdl_3outside,
            cdl_3line_strike,
            cdl_3stars_in_south,
            cdl_tristar,
            cdl_identical_3crows,
            cdl_advance_block,
            cdl_stalled_pattern,
            cdl_unique_3river,
            cdl_stick_sandwich,
            cdl_tasuki_gap,
            cdl_upside_gap_2crows,
            cdl_gap_side_side_white,
            cdl_breakaway,
            cdl_ladder_bottom,
            cdl_mat_hold,
            cdl_rise_fall_3methods,
            cdl_concealing_baby_swallow,
            cdl_xside_gap_3methods,
        ];
        for f in wrappers {
            assert!(f(&empty, &empty, &empty, &empty).is_err());
            assert!(f(&open, &high_short, &low, &close).is_err());
            assert!(f(&open, &high, &low_short, &close).is_err());
            assert!(f(&open, &high, &low, &close_short).is_err());
        }

        let into_fns: [ThreeIntoFn; 26] = [
            cdl_morning_star_into,
            cdl_evening_star_into,
            cdl_morning_doji_star_into,
            cdl_evening_doji_star_into,
            cdl_abandoned_baby_into,
            cdl_3white_soldiers_into,
            cdl_3black_crows_into,
            cdl_3inside_into,
            cdl_3outside_into,
            cdl_3line_strike_into,
            cdl_3stars_in_south_into,
            cdl_tristar_into,
            cdl_identical_3crows_into,
            cdl_advance_block_into,
            cdl_stalled_pattern_into,
            cdl_unique_3river_into,
            cdl_stick_sandwich_into,
            cdl_tasuki_gap_into,
            cdl_upside_gap_2crows_into,
            cdl_gap_side_side_white_into,
            cdl_breakaway_into,
            cdl_ladder_bottom_into,
            cdl_mat_hold_into,
            cdl_rise_fall_3methods_into,
            cdl_concealing_baby_swallow_into,
            cdl_xside_gap_3methods_into,
        ];
        let mut out_ok = vec![0_i32; n];
        let mut out_short = vec![0_i32; n - 1];
        for f in into_fns {
            assert!(f(&open, &high, &low, &close, &mut out_short).is_err());
            assert!(f(&open, &high_short, &low, &close, &mut out_ok).is_err());
            assert!(f(&open, &high, &low_short, &close, &mut out_ok).is_err());
            assert!(f(&open, &high, &low, &close_short, &mut out_ok).is_err());
            assert!(f(&empty, &empty, &empty, &empty, &mut []).is_err());
        }
    }

    #[test]
    fn test_three_candle_simple_pattern_lookback_min_len_surface() {
        assert_eq!(
            cdl_stick_sandwich_min_len(),
            cdl_stick_sandwich_lookback() + 1
        );
        assert_eq!(
            cdl_unique_3river_min_len(),
            cdl_unique_3river_lookback() + 1
        );
        assert_eq!(
            cdl_advance_block_min_len(),
            cdl_advance_block_lookback() + 1
        );
        assert_eq!(
            cdl_stalled_pattern_min_len(),
            cdl_stalled_pattern_lookback() + 1
        );
        assert_eq!(cdl_tasuki_gap_min_len(), cdl_tasuki_gap_lookback() + 1);
        assert_eq!(
            cdl_upside_gap_2crows_min_len(),
            cdl_upside_gap_2crows_lookback() + 1
        );
        assert_eq!(
            cdl_gap_side_side_white_min_len(),
            cdl_gap_side_side_white_lookback() + 1
        );
        assert_eq!(cdl_breakaway_min_len(), cdl_breakaway_lookback() + 1);
        assert_eq!(
            cdl_ladder_bottom_min_len(),
            cdl_ladder_bottom_lookback() + 1
        );
        assert_eq!(cdl_mat_hold_min_len(), cdl_mat_hold_lookback() + 1);
        assert_eq!(
            cdl_rise_fall_3methods_min_len(),
            cdl_rise_fall_3methods_lookback() + 1
        );
        assert_eq!(
            cdl_concealing_baby_swallow_min_len(),
            cdl_concealing_baby_swallow_lookback() + 1
        );
        assert_eq!(
            cdl_xside_gap_3methods_min_len(),
            cdl_xside_gap_3methods_lookback() + 1
        );
    }
}
