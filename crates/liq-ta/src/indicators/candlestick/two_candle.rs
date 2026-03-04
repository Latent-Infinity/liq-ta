//! Two-candle pattern recognition functions.
//!
//! These patterns are identified based on the relationship between
//! two consecutive candlesticks.

use super::core::{
    AVG_LOOKBACK, PATTERN_BEARISH, PATTERN_BEARISH_STRONG, PATTERN_BULLISH, PATTERN_BULLISH_STRONG,
    PATTERN_NONE, TREND_LOOKBACK, approx_equal, average_body, average_range, body_bottom,
    body_midpoint, body_top, candle_range, f64_to_t, gap_down, gap_up, is_bearish, is_bullish,
    is_doji, is_downtrend, is_uptrend, lower_shadow, real_body, real_body_gap_down,
    real_body_gap_up, upper_shadow,
};
use crate::error::{Error, Result};
use crate::traits::SeriesElement;

/// Lookback for `CDL_ENGULFING` pattern.
#[must_use]
pub const fn cdl_engulfing_lookback() -> usize {
    AVG_LOOKBACK + 1
}

/// Minimum length for `CDL_ENGULFING`.
#[must_use]
pub const fn cdl_engulfing_min_len() -> usize {
    cdl_engulfing_lookback() + 1
}

/// Engulfing pattern recognition.
///
/// A Bullish Engulfing occurs when a large bullish candle completely
/// engulfs the previous bearish candle. Bearish Engulfing is the opposite.
///
/// Returns:
/// - `100`: Bullish Engulfing
/// - `-100`: Bearish Engulfing
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_engulfing<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_engulfing_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Engulfing pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_engulfing_into<T: SeriesElement>(
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
    if len < cdl_engulfing_min_len() {
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

    let lookback = cdl_engulfing_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let prev = i - 1;

        // Bullish engulfing: prev bearish, curr bullish, curr body engulfs prev body
        if is_bearish(open[prev], close[prev]) && is_bullish(open[i], close[i]) {
            let prev_body_top = body_top(open[prev], close[prev]);
            let prev_body_bottom = body_bottom(open[prev], close[prev]);
            let curr_body_top = body_top(open[i], close[i]);
            let curr_body_bottom = body_bottom(open[i], close[i]);

            if curr_body_bottom < prev_body_bottom && curr_body_top > prev_body_top {
                out[i] = PATTERN_BULLISH;
                continue;
            }
        }

        // Bearish engulfing: prev bullish, curr bearish, curr body engulfs prev body
        if is_bullish(open[prev], close[prev]) && is_bearish(open[i], close[i]) {
            let prev_body_top = body_top(open[prev], close[prev]);
            let prev_body_bottom = body_bottom(open[prev], close[prev]);
            let curr_body_top = body_top(open[i], close[i]);
            let curr_body_bottom = body_bottom(open[i], close[i]);

            if curr_body_bottom < prev_body_bottom && curr_body_top > prev_body_top {
                out[i] = PATTERN_BEARISH;
                continue;
            }
        }

        out[i] = PATTERN_NONE;
    }

    Ok(())
}

/// Lookback for `CDL_HARAMI` pattern.
#[must_use]
pub const fn cdl_harami_lookback() -> usize {
    AVG_LOOKBACK + 1
}

/// Minimum length for `CDL_HARAMI`.
#[must_use]
pub const fn cdl_harami_min_len() -> usize {
    cdl_harami_lookback() + 1
}

/// Harami pattern recognition.
///
/// A Harami occurs when a small candle is completely contained within
/// the body of the previous larger candle (opposite of Engulfing).
///
/// Returns:
/// - `100`: Bullish Harami (bearish then small bullish)
/// - `-100`: Bearish Harami (bullish then small bearish)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_harami<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_harami_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Harami pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_harami_into<T: SeriesElement>(
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
    if len < cdl_harami_min_len() {
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

    let lookback = cdl_harami_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let prev = i - 1;
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);
        let prev_body = real_body(open[prev], close[prev]);
        let curr_body = real_body(open[i], close[i]);

        // Previous candle must have a long body
        if prev_body < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Current candle must have a small body
        if curr_body > avg_body * f64_to_t(0.5) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let prev_body_top = body_top(open[prev], close[prev]);
        let prev_body_bottom = body_bottom(open[prev], close[prev]);
        let curr_body_top = body_top(open[i], close[i]);
        let curr_body_bottom = body_bottom(open[i], close[i]);

        // Current body must be inside previous body
        if curr_body_bottom < prev_body_bottom || curr_body_top > prev_body_top {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Bullish harami: prev bearish
        if is_bearish(open[prev], close[prev]) {
            out[i] = PATTERN_BULLISH;
            continue;
        }

        // Bearish harami: prev bullish
        if is_bullish(open[prev], close[prev]) {
            out[i] = PATTERN_BEARISH;
            continue;
        }

        out[i] = PATTERN_NONE;
    }

    Ok(())
}

/// Lookback for `CDL_HARAMI_CROSS` pattern.
#[must_use]
pub const fn cdl_harami_cross_lookback() -> usize {
    AVG_LOOKBACK + 1
}

/// Minimum length for `CDL_HARAMI_CROSS`.
#[must_use]
pub const fn cdl_harami_cross_min_len() -> usize {
    cdl_harami_cross_lookback() + 1
}

/// Harami Cross pattern recognition.
///
/// A Harami Cross is a Harami where the second candle is a Doji.
///
/// Returns:
/// - `100`: Bullish Harami Cross
/// - `-100`: Bearish Harami Cross
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_harami_cross<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_harami_cross_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Harami Cross pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_harami_cross_into<T: SeriesElement>(
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
    if len < cdl_harami_cross_min_len() {
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

    let lookback = cdl_harami_cross_lookback();
    let doji_threshold = f64_to_t(0.1);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let prev = i - 1;

        // Current must be a doji
        if !is_doji(open[i], high[i], low[i], close[i], doji_threshold) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);
        let prev_body = real_body(open[prev], close[prev]);

        // Previous candle must have a long body
        if prev_body < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        let prev_body_top = body_top(open[prev], close[prev]);
        let prev_body_bottom = body_bottom(open[prev], close[prev]);
        let curr_mid = body_midpoint(open[i], close[i]);

        // Doji must be inside previous body
        if curr_mid < prev_body_bottom || curr_mid > prev_body_top {
            out[i] = PATTERN_NONE;
            continue;
        }

        if is_bearish(open[prev], close[prev]) {
            out[i] = PATTERN_BULLISH;
        } else if is_bullish(open[prev], close[prev]) {
            out[i] = PATTERN_BEARISH;
        } else {
            out[i] = PATTERN_NONE;
        }
    }

    Ok(())
}

/// Lookback for `CDL_PIERCING` pattern.
#[must_use]
pub const fn cdl_piercing_lookback() -> usize {
    AVG_LOOKBACK + 1
}

/// Minimum length for `CDL_PIERCING`.
#[must_use]
pub const fn cdl_piercing_min_len() -> usize {
    cdl_piercing_lookback() + 1
}

/// Piercing Line pattern recognition.
///
/// A bullish reversal pattern: bearish candle followed by a bullish candle
/// that opens below the previous low and closes above the midpoint of
/// the previous body.
///
/// Returns:
/// - `100`: Piercing Line detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_piercing<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_piercing_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Piercing Line pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_piercing_into<T: SeriesElement>(
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
    if len < cdl_piercing_min_len() {
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

    let lookback = cdl_piercing_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let prev = i - 1;
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // First candle must be bearish with significant body
        if !is_bearish(open[prev], close[prev]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let prev_body = real_body(open[prev], close[prev]);
        if prev_body < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Second candle must be bullish
        if !is_bullish(open[i], close[i]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Open below previous low
        if open[i] >= low[prev] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Close above midpoint of previous body
        let prev_mid = body_midpoint(open[prev], close[prev]);
        if close[i] <= prev_mid {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Close below previous open
        if close[i] >= open[prev] {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BULLISH;
    }

    Ok(())
}

/// Lookback for `CDL_DARK_CLOUD_COVER` pattern.
#[must_use]
pub const fn cdl_dark_cloud_cover_lookback() -> usize {
    AVG_LOOKBACK + 1
}

/// Minimum length for `CDL_DARK_CLOUD_COVER`.
#[must_use]
pub const fn cdl_dark_cloud_cover_min_len() -> usize {
    cdl_dark_cloud_cover_lookback() + 1
}

/// Dark Cloud Cover pattern recognition.
///
/// The bearish counterpart to Piercing Line: bullish candle followed
/// by a bearish candle that opens above the previous high and closes
/// below the midpoint of the previous body.
///
/// Returns:
/// - `-100`: Dark Cloud Cover detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_dark_cloud_cover<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_dark_cloud_cover_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Dark Cloud Cover pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_dark_cloud_cover_into<T: SeriesElement>(
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
    if len < cdl_dark_cloud_cover_min_len() {
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

    let lookback = cdl_dark_cloud_cover_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let prev = i - 1;
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // First candle must be bullish with significant body
        if !is_bullish(open[prev], close[prev]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let prev_body = real_body(open[prev], close[prev]);
        if prev_body < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Second candle must be bearish
        if !is_bearish(open[i], close[i]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Open above previous high
        if open[i] <= high[prev] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Close below midpoint of previous body
        let prev_mid = body_midpoint(open[prev], close[prev]);
        if close[i] >= prev_mid {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Close above previous open
        if close[i] <= open[prev] {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BEARISH;
    }

    Ok(())
}

/// Lookback for `CDL_DOJI_STAR` pattern.
#[must_use]
pub const fn cdl_doji_star_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK + 1
}

/// Minimum length for `CDL_DOJI_STAR`.
#[must_use]
pub const fn cdl_doji_star_min_len() -> usize {
    cdl_doji_star_lookback() + 1
}

/// Doji Star pattern recognition.
///
/// A long candle followed by a Doji that gaps away from it.
///
/// Returns:
/// - `100`: Bullish Doji Star (in downtrend)
/// - `-100`: Bearish Doji Star (in uptrend)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_doji_star<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_doji_star_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Doji Star pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_doji_star_into<T: SeriesElement>(
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
    if len < cdl_doji_star_min_len() {
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

    let lookback = cdl_doji_star_lookback();
    let doji_threshold = f64_to_t(0.1);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let prev = i - 1;

        // Current must be a doji
        if !is_doji(open[i], high[i], low[i], close[i], doji_threshold) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let avg_body = average_body(open, close, i, AVG_LOOKBACK);
        let prev_body = real_body(open[prev], close[prev]);

        // Previous must be a long body
        if prev_body < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Check for gap
        if is_bearish(open[prev], close[prev]) {
            // In downtrend, doji gaps down (bullish signal)
            if real_body_gap_down(open[prev], close[prev], open[i], close[i]) {
                out[i] = PATTERN_BULLISH;
                continue;
            }
        } else if is_bullish(open[prev], close[prev]) {
            // In uptrend, doji gaps up (bearish signal)
            if real_body_gap_up(open[prev], close[prev], open[i], close[i]) {
                out[i] = PATTERN_BEARISH;
                continue;
            }
        }

        out[i] = PATTERN_NONE;
    }

    Ok(())
}

/// Lookback for `CDL_KICKING` pattern.
#[must_use]
pub const fn cdl_kicking_lookback() -> usize {
    AVG_LOOKBACK + 1
}

/// Minimum length for `CDL_KICKING`.
#[must_use]
pub const fn cdl_kicking_min_len() -> usize {
    cdl_kicking_lookback() + 1
}

/// Kicking pattern recognition.
///
/// Two Marubozu candles of opposite colors with a gap between them.
///
/// Returns:
/// - `100`: Bullish Kicking (black marubozu, gap up, white marubozu)
/// - `-100`: Bearish Kicking (white marubozu, gap down, black marubozu)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_kicking<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_kicking_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Kicking pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_kicking_into<T: SeriesElement>(
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
    if len < cdl_kicking_min_len() {
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

    let lookback = cdl_kicking_lookback();
    let shadow_max = f64_to_t(0.05);

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let prev = i - 1;
        let prev_range = candle_range(high[prev], low[prev]);
        let curr_range = candle_range(high[i], low[i]);

        if prev_range <= T::zero() || curr_range <= T::zero() {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Check both candles are marubozu
        let prev_upper = upper_shadow(open[prev], high[prev], close[prev]);
        let prev_lower = lower_shadow(open[prev], low[prev], close[prev]);
        let curr_upper = upper_shadow(open[i], high[i], close[i]);
        let curr_lower = lower_shadow(open[i], low[i], close[i]);

        let is_prev_marubozu =
            prev_upper <= prev_range * shadow_max && prev_lower <= prev_range * shadow_max;
        let is_curr_marubozu =
            curr_upper <= curr_range * shadow_max && curr_lower <= curr_range * shadow_max;

        if !is_prev_marubozu || !is_curr_marubozu {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Bullish kicking: prev bearish, curr bullish, gap up
        if is_bearish(open[prev], close[prev])
            && is_bullish(open[i], close[i])
            && gap_up(high[prev], low[i])
        {
            out[i] = PATTERN_BULLISH;
            continue;
        }

        // Bearish kicking: prev bullish, curr bearish, gap down
        if is_bullish(open[prev], close[prev])
            && is_bearish(open[i], close[i])
            && gap_down(low[prev], high[i])
        {
            out[i] = PATTERN_BEARISH;
            continue;
        }

        out[i] = PATTERN_NONE;
    }

    Ok(())
}

/// Lookback for `CDL_KICKING_BY_LENGTH` pattern.
#[must_use]
pub const fn cdl_kicking_by_length_lookback() -> usize {
    cdl_kicking_lookback()
}

/// Minimum length for `CDL_KICKING_BY_LENGTH`.
#[must_use]
pub const fn cdl_kicking_by_length_min_len() -> usize {
    cdl_kicking_min_len()
}

/// Kicking By Length pattern recognition.
///
/// Same as Kicking but the signal strength depends on which marubozu is longer.
///
/// Returns:
/// - `100`/`200`: Bullish Kicking (stronger if bullish candle is longer)
/// - `-100`/`-200`: Bearish Kicking (stronger if bearish candle is longer)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_kicking_by_length<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_kicking_by_length_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Kicking By Length pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_kicking_by_length_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    // First, get the base kicking pattern
    cdl_kicking_into(open, high, low, close, out)?;

    // Adjust strength based on relative lengths
    let len = open.len();
    let lookback = cdl_kicking_by_length_lookback();

    for i in lookback..len {
        if out[i] == PATTERN_NONE {
            continue;
        }

        let prev = i - 1;
        let prev_body = real_body(open[prev], close[prev]);
        let curr_body = real_body(open[i], close[i]);

        if out[i] == PATTERN_BULLISH && curr_body > prev_body {
            out[i] = PATTERN_BULLISH_STRONG;
        } else if out[i] == PATTERN_BEARISH && curr_body > prev_body {
            out[i] = PATTERN_BEARISH_STRONG;
        }
    }

    Ok(())
}

/// Lookback for `CDL_MATCHING_LOW` pattern.
#[must_use]
pub const fn cdl_matching_low_lookback() -> usize {
    AVG_LOOKBACK + 1
}

/// Minimum length for `CDL_MATCHING_LOW`.
#[must_use]
pub const fn cdl_matching_low_min_len() -> usize {
    cdl_matching_low_lookback() + 1
}

/// Matching Low pattern recognition.
///
/// Two bearish candles with the same closing price, indicating support.
///
/// Returns:
/// - `100`: Matching Low detected (bullish reversal)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_matching_low<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_matching_low_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Matching Low pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_matching_low_into<T: SeriesElement>(
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
    if len < cdl_matching_low_min_len() {
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

    let lookback = cdl_matching_low_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let prev = i - 1;

        // Both must be bearish
        if !is_bearish(open[prev], close[prev]) || !is_bearish(open[i], close[i]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Closes must be approximately equal
        let avg_range = average_range(high, low, i, AVG_LOOKBACK);
        let tolerance = avg_range * f64_to_t(0.05);

        if approx_equal(close[prev], close[i], tolerance) {
            out[i] = PATTERN_BULLISH;
        } else {
            out[i] = PATTERN_NONE;
        }
    }

    Ok(())
}

/// Lookback for `CDL_HOMING_PIGEON` pattern.
#[must_use]
pub const fn cdl_homing_pigeon_lookback() -> usize {
    AVG_LOOKBACK + 1
}

/// Minimum length for `CDL_HOMING_PIGEON`.
#[must_use]
pub const fn cdl_homing_pigeon_min_len() -> usize {
    cdl_homing_pigeon_lookback() + 1
}

/// Homing Pigeon pattern recognition.
///
/// Two bearish candles where the second is completely inside the first.
/// Similar to Harami but both candles are bearish.
///
/// Returns:
/// - `100`: Homing Pigeon detected (bullish reversal)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_homing_pigeon<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_homing_pigeon_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Homing Pigeon pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_homing_pigeon_into<T: SeriesElement>(
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
    if len < cdl_homing_pigeon_min_len() {
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

    let lookback = cdl_homing_pigeon_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let prev = i - 1;

        // Both must be bearish
        if !is_bearish(open[prev], close[prev]) || !is_bearish(open[i], close[i]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Second candle must be inside first (body contained)
        if open[i] > open[prev] || close[i] < close[prev] {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BULLISH;
    }

    Ok(())
}

/// Lookback for `CDL_IN_NECK` pattern.
#[must_use]
pub const fn cdl_in_neck_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK + 1
}

/// Minimum length for `CDL_IN_NECK`.
#[must_use]
pub const fn cdl_in_neck_min_len() -> usize {
    cdl_in_neck_lookback() + 1
}

/// In-Neck pattern recognition.
///
/// Bearish continuation: long bearish candle followed by small bullish
/// candle that closes at or near the previous close.
///
/// Returns:
/// - `-100`: In-Neck detected (bearish continuation)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_in_neck<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_in_neck_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// In-Neck pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_in_neck_into<T: SeriesElement>(
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
    if len < cdl_in_neck_min_len() {
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

    let lookback = cdl_in_neck_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        // Must be in downtrend
        if !is_downtrend(close, i, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let prev = i - 1;
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);
        let avg_range = average_range(high, low, i, AVG_LOOKBACK);

        // First candle: long bearish
        if !is_bearish(open[prev], close[prev]) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if real_body(open[prev], close[prev]) < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Second candle: bullish
        if !is_bullish(open[i], close[i]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Close near previous close
        let tolerance = avg_range * f64_to_t(0.1);
        if approx_equal(close[i], close[prev], tolerance) {
            out[i] = PATTERN_BEARISH;
        } else {
            out[i] = PATTERN_NONE;
        }
    }

    Ok(())
}

/// Lookback for `CDL_ON_NECK` pattern.
#[must_use]
pub const fn cdl_on_neck_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK + 1
}

/// Minimum length for `CDL_ON_NECK`.
#[must_use]
pub const fn cdl_on_neck_min_len() -> usize {
    cdl_on_neck_lookback() + 1
}

/// On-Neck pattern recognition.
///
/// Similar to In-Neck but the bullish candle closes at the previous low.
///
/// Returns:
/// - `-100`: On-Neck detected (bearish continuation)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_on_neck<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_on_neck_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// On-Neck pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_on_neck_into<T: SeriesElement>(
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
    if len < cdl_on_neck_min_len() {
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

    let lookback = cdl_on_neck_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        // Must be in downtrend
        if !is_downtrend(close, i, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let prev = i - 1;
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);
        let avg_range = average_range(high, low, i, AVG_LOOKBACK);

        // First candle: long bearish
        if !is_bearish(open[prev], close[prev]) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if real_body(open[prev], close[prev]) < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Second candle: bullish
        if !is_bullish(open[i], close[i]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Close at previous low
        let tolerance = avg_range * f64_to_t(0.05);
        if approx_equal(close[i], low[prev], tolerance) {
            out[i] = PATTERN_BEARISH;
        } else {
            out[i] = PATTERN_NONE;
        }
    }

    Ok(())
}

/// Lookback for `CDL_THRUSTING` pattern.
#[must_use]
pub const fn cdl_thrusting_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK + 1
}

/// Minimum length for `CDL_THRUSTING`.
#[must_use]
pub const fn cdl_thrusting_min_len() -> usize {
    cdl_thrusting_lookback() + 1
}

/// Thrusting pattern recognition.
///
/// Similar to Piercing but weaker - the bullish candle closes below
/// the midpoint of the previous bearish candle.
///
/// Returns:
/// - `-100`: Thrusting detected (bearish continuation)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_thrusting<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_thrusting_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Thrusting pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_thrusting_into<T: SeriesElement>(
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
    if len < cdl_thrusting_min_len() {
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

    let lookback = cdl_thrusting_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        // Must be in downtrend
        if !is_downtrend(close, i, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let prev = i - 1;
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);

        // First candle: long bearish
        if !is_bearish(open[prev], close[prev]) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if real_body(open[prev], close[prev]) < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Second candle: bullish
        if !is_bullish(open[i], close[i]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Open below previous low
        if open[i] >= low[prev] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Close above previous close but below midpoint
        let prev_mid = body_midpoint(open[prev], close[prev]);
        if close[i] > close[prev] && close[i] < prev_mid {
            out[i] = PATTERN_BEARISH;
        } else {
            out[i] = PATTERN_NONE;
        }
    }

    Ok(())
}

/// Lookback for `CDL_SEPARATING_LINES` pattern.
#[must_use]
pub const fn cdl_separating_lines_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK + 1
}

/// Minimum length for `CDL_SEPARATING_LINES`.
#[must_use]
pub const fn cdl_separating_lines_min_len() -> usize {
    cdl_separating_lines_lookback() + 1
}

/// Separating Lines pattern recognition.
///
/// Two opposite-colored candles that open at the same price.
///
/// Returns:
/// - `100`: Bullish Separating Lines (in uptrend)
/// - `-100`: Bearish Separating Lines (in downtrend)
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_separating_lines<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_separating_lines_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Separating Lines pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_separating_lines_into<T: SeriesElement>(
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
    if len < cdl_separating_lines_min_len() {
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

    let lookback = cdl_separating_lines_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let prev = i - 1;
        let avg_range = average_range(high, low, i, AVG_LOOKBACK);
        let tolerance = avg_range * f64_to_t(0.05);

        // Opens must be approximately equal
        if !approx_equal(open[prev], open[i], tolerance) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Must be opposite colors
        if is_bullish(open[prev], close[prev]) && is_bearish(open[i], close[i]) {
            // In downtrend: bearish continuation
            if is_downtrend(close, i, TREND_LOOKBACK) {
                out[i] = PATTERN_BEARISH;
                continue;
            }
        }

        if is_bearish(open[prev], close[prev]) && is_bullish(open[i], close[i]) {
            // In uptrend: bullish continuation
            if is_uptrend(close, i, TREND_LOOKBACK) {
                out[i] = PATTERN_BULLISH;
                continue;
            }
        }

        out[i] = PATTERN_NONE;
    }

    Ok(())
}

/// Lookback for `CDL_COUNTER_ATTACK` pattern.
#[must_use]
pub const fn cdl_counter_attack_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK + 1
}

/// Minimum length for `CDL_COUNTER_ATTACK`.
#[must_use]
pub const fn cdl_counter_attack_min_len() -> usize {
    cdl_counter_attack_lookback() + 1
}

/// Counter Attack pattern recognition.
///
/// Two long opposite-colored candles with the same closing price.
///
/// Returns:
/// - `100`: Bullish Counter Attack
/// - `-100`: Bearish Counter Attack
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_counter_attack<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_counter_attack_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Counter Attack pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_counter_attack_into<T: SeriesElement>(
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
    if len < cdl_counter_attack_min_len() {
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

    let lookback = cdl_counter_attack_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        let prev = i - 1;
        let avg_body = average_body(open, close, i, AVG_LOOKBACK);
        let avg_range = average_range(high, low, i, AVG_LOOKBACK);

        // Both must have long bodies
        let prev_body = real_body(open[prev], close[prev]);
        let curr_body = real_body(open[i], close[i]);

        if prev_body < avg_body || curr_body < avg_body {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Closes must be approximately equal
        let tolerance = avg_range * f64_to_t(0.05);
        if !approx_equal(close[prev], close[i], tolerance) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Must be opposite colors
        if is_bearish(open[prev], close[prev]) && is_bullish(open[i], close[i]) {
            out[i] = PATTERN_BULLISH;
        } else if is_bullish(open[prev], close[prev]) && is_bearish(open[i], close[i]) {
            out[i] = PATTERN_BEARISH;
        } else {
            out[i] = PATTERN_NONE;
        }
    }

    Ok(())
}

/// Lookback for `CDL_2CROWS` pattern.
#[must_use]
pub const fn cdl_2crows_lookback() -> usize {
    AVG_LOOKBACK + TREND_LOOKBACK + 2
}

/// Minimum length for `CDL_2CROWS`.
#[must_use]
pub const fn cdl_2crows_min_len() -> usize {
    cdl_2crows_lookback() + 1
}

/// Two Crows pattern recognition.
///
/// Three-candle bearish reversal pattern (note: despite the name, it uses 3 candles):
/// 1. Long bullish candle in uptrend
/// 2. Small bearish candle that gaps up
/// 3. Bearish candle that opens within #2's body and closes within #1's body
///
/// Returns:
/// - `-100`: Two Crows detected
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_2crows<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_2crows_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Two Crows pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_2crows_into<T: SeriesElement>(
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
    if len < cdl_2crows_min_len() {
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

    let lookback = cdl_2crows_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    for i in lookback..len {
        // Must be in uptrend
        if !is_uptrend(close, i - 2, TREND_LOOKBACK) {
            out[i] = PATTERN_NONE;
            continue;
        }

        let first = i - 2;
        let second = i - 1;
        let third = i;

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

        // Second: bearish, gaps up from first
        if !is_bearish(open[second], close[second]) {
            out[i] = PATTERN_NONE;
            continue;
        }
        if !gap_up(close[first], open[second]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Third: bearish
        if !is_bearish(open[third], close[third]) {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Third opens within second's body
        if close[third] < close[second] || open[third] > open[second] {
            out[i] = PATTERN_NONE;
            continue;
        }

        // Third closes within first's body
        if open[third] < open[first] || close[third] > close[first] {
            out[i] = PATTERN_NONE;
            continue;
        }

        out[i] = PATTERN_BEARISH;
    }

    Ok(())
}

/// Lookback for `CDL_HIKKAKE` pattern.
#[must_use]
pub const fn cdl_hikkake_lookback() -> usize {
    5
}

/// Minimum length for `CDL_HIKKAKE`.
#[must_use]
pub const fn cdl_hikkake_min_len() -> usize {
    cdl_hikkake_lookback() + 1
}

/// Hikkake pattern recognition.
///
/// A complex pattern involving an inside bar followed by a false breakout.
///
/// Returns:
/// - `100`: Bullish Hikkake
/// - `-100`: Bearish Hikkake
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_hikkake<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_hikkake_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Hikkake pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_hikkake_into<T: SeriesElement>(
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
    if len < cdl_hikkake_min_len() {
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

    let lookback = cdl_hikkake_lookback();

    for i in 0..lookback {
        out[i] = PATTERN_NONE;
    }

    // Track inside bar positions for confirmation
    let mut inside_bar_high = vec![T::nan(); len];
    let mut inside_bar_low = vec![T::nan(); len];
    let mut pattern_direction = vec![0i32; len];

    for i in lookback..len {
        out[i] = PATTERN_NONE;

        // Check for inside bar (second candle inside first)
        let first = i - 2;
        let second = i - 1;

        // Is second an inside bar?
        if high[second] < high[first] && low[second] > low[first] {
            // Third candle (current) breaks out
            if close[i] > high[second] {
                // Bullish breakout - but this is the FALSE breakout
                // We need to track for reversal
                inside_bar_high[i] = high[first];
                inside_bar_low[i] = low[first];
                pattern_direction[i] = -1; // Expecting bearish reversal
            } else if close[i] < low[second] {
                // Bearish breakout - this is the FALSE breakout
                inside_bar_high[i] = high[first];
                inside_bar_low[i] = low[first];
                pattern_direction[i] = 1; // Expecting bullish reversal
            }
        }

        // Check for pattern confirmation (reversal after false breakout)
        for j in 1..=3 {
            if i >= lookback + j {
                let check_idx = i - j;
                if pattern_direction[check_idx] == 1 && close[i] > inside_bar_high[check_idx] {
                    out[i] = PATTERN_BULLISH;
                    break;
                } else if pattern_direction[check_idx] == -1 && close[i] < inside_bar_low[check_idx]
                {
                    out[i] = PATTERN_BEARISH;
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Lookback for `CDL_HIKKAKE_MOD` pattern.
#[must_use]
pub const fn cdl_hikkake_mod_lookback() -> usize {
    cdl_hikkake_lookback() + 1
}

/// Minimum length for `CDL_HIKKAKE_MOD`.
#[must_use]
pub const fn cdl_hikkake_mod_min_len() -> usize {
    cdl_hikkake_mod_lookback() + 1
}

/// Modified Hikkake pattern recognition.
///
/// A variation of Hikkake with additional confirmation requirements.
///
/// Returns:
/// - `100`: Bullish Modified Hikkake
/// - `-100`: Bearish Modified Hikkake
/// - `0`: No pattern
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
pub fn cdl_hikkake_mod<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
) -> Result<Vec<i32>> {
    let mut out = vec![0i32; open.len()];
    cdl_hikkake_mod_into(open, high, low, close, &mut out)?;
    Ok(out)
}

/// Modified Hikkake pattern recognition (zero-copy output).
///
/// # Errors
///
/// Returns an error if:
/// - The input arrays are empty (`Error::EmptyInput`)
/// - The OHLC arrays have different lengths (`Error::LengthMismatch`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn cdl_hikkake_mod_into<T: SeriesElement>(
    open: &[T],
    high: &[T],
    low: &[T],
    close: &[T],
    out: &mut [i32],
) -> Result<()> {
    // Modified hikkake has similar logic but with body requirements
    cdl_hikkake_into(open, high, low, close, out)?;

    let len = open.len();
    let lookback = cdl_hikkake_mod_lookback();

    // Apply additional filter: require significant body on confirmation candle
    for i in lookback..len {
        if out[i] != PATTERN_NONE {
            let body = real_body(open[i], close[i]);
            let range = candle_range(high[i], low[i]);
            if range > T::zero() && body < range * f64_to_t(0.3) {
                out[i] = PATTERN_NONE;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engulfing_bullish() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        // Need enough data for lookback + 1
        let mut open = vec![100.0; 12];
        let mut high = vec![105.0; 12];
        let mut low = vec![95.0; 12];
        let mut close = vec![103.0; 12];

        // Previous candle: bearish (small)
        open[10] = 102.0;
        high[10] = 103.0;
        low[10] = 99.0;
        close[10] = 100.0;

        // Current candle: bullish engulfing (large, engulfs previous)
        open[11] = 99.0; // Opens below previous close
        high[11] = 106.0;
        low[11] = 98.0;
        close[11] = 105.0; // Closes above previous open

        (open, high, low, close)
    }

    #[test]
    fn test_cdl_engulfing_bullish() {
        let (open, high, low, close) = make_engulfing_bullish();
        let result = cdl_engulfing(&open, &high, &low, &close).unwrap();
        assert_eq!(result[11], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_harami() {
        let mut open = vec![100.0; 12];
        let mut high = vec![110.0; 12];
        let mut low = vec![90.0; 12];
        let mut close = vec![105.0; 12];

        // Previous: large bearish
        open[10] = 108.0;
        high[10] = 110.0;
        low[10] = 90.0;
        close[10] = 92.0;

        // Current: small bullish inside previous body
        open[11] = 95.0;
        high[11] = 100.0;
        low[11] = 94.0;
        close[11] = 98.0;

        let result = cdl_harami(&open, &high, &low, &close).unwrap();
        assert_eq!(result[11], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_piercing() {
        let mut open = vec![100.0; 12];
        let mut high = vec![105.0; 12];
        let mut low = vec![95.0; 12];
        let mut close = vec![103.0; 12];

        // Previous: bearish
        open[10] = 105.0;
        high[10] = 106.0;
        low[10] = 95.0;
        close[10] = 96.0;

        // Current: bullish, opens below prev low, closes above midpoint
        open[11] = 94.0;
        high[11] = 103.0;
        low[11] = 93.0;
        close[11] = 102.0; // Above midpoint (100.5) but below prev open (105)

        let result = cdl_piercing(&open, &high, &low, &close).unwrap();
        assert_eq!(result[11], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_dark_cloud_cover() {
        let mut open = vec![100.0; 12];
        let mut high = vec![105.0; 12];
        let mut low = vec![95.0; 12];
        let mut close = vec![103.0; 12];

        // Previous: bullish
        open[10] = 95.0;
        high[10] = 106.0;
        low[10] = 94.0;
        close[10] = 105.0;

        // Current: bearish, opens above prev high, closes below midpoint
        open[11] = 107.0;
        high[11] = 108.0;
        low[11] = 98.0;
        close[11] = 99.0; // Below midpoint (100) but above prev open (95)

        let result = cdl_dark_cloud_cover(&open, &high, &low, &close).unwrap();
        assert_eq!(result[11], PATTERN_BEARISH);
    }

    #[test]
    fn test_cdl_kicking_bullish() {
        let mut open = vec![100.0; 12];
        let mut high = vec![105.0; 12];
        let mut low = vec![95.0; 12];
        let mut close = vec![103.0; 12];

        // Previous: bearish marubozu
        open[10] = 105.0;
        high[10] = 105.0; // No upper shadow
        low[10] = 95.0;
        close[10] = 95.0; // No lower shadow

        // Current: bullish marubozu with gap up
        open[11] = 106.0;
        high[11] = 116.0;
        low[11] = 106.0; // No lower shadow
        close[11] = 116.0; // No upper shadow

        let result = cdl_kicking(&open, &high, &low, &close).unwrap();
        assert_eq!(result[11], PATTERN_BULLISH);
    }

    #[test]
    fn test_cdl_kicking_by_length_bearish_strong_branch() {
        let mut open = vec![100.0; 12];
        let mut high = vec![105.0; 12];
        let mut low = vec![95.0; 12];
        let mut close = vec![103.0; 12];

        // Previous: bullish marubozu
        open[10] = 100.0;
        high[10] = 110.0;
        low[10] = 100.0;
        close[10] = 110.0;

        // Current: bearish marubozu with gap down and larger body than previous
        open[11] = 98.0;
        high[11] = 98.0;
        low[11] = 82.0;
        close[11] = 82.0;

        let result = cdl_kicking_by_length(&open, &high, &low, &close).unwrap();
        assert_eq!(result[11], PATTERN_BEARISH_STRONG);
    }

    #[test]
    fn test_cdl_matching_low() {
        let mut open = vec![100.0; 12];
        let mut high = vec![105.0; 12];
        let mut low = vec![95.0; 12];
        let mut close = vec![98.0; 12];

        // Two bearish candles with same close
        open[10] = 102.0;
        high[10] = 103.0;
        low[10] = 94.0;
        close[10] = 95.0;

        open[11] = 100.0;
        high[11] = 101.0;
        low[11] = 94.0;
        close[11] = 95.0; // Same close as previous

        let result = cdl_matching_low(&open, &high, &low, &close).unwrap();
        assert_eq!(result[11], PATTERN_BULLISH);
    }

    #[test]
    fn test_empty_input() {
        let empty: Vec<f64> = vec![];
        assert!(cdl_engulfing(&empty, &empty, &empty, &empty).is_err());
    }

    #[test]
    fn test_two_candle_wrappers_smoke_surface() {
        let n = 640;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        for i in 0..n {
            let base = 300.0 - (i as f64) * 0.15;
            open.push(base);
            close.push(base - 0.12);
            high.push(base + 6.0);
            low.push(base - 6.0);
        }

        macro_rules! assert_len_ok {
            ($expr:expr) => {{
                let out = $expr.unwrap();
                assert_eq!(out.len(), n);
            }};
        }

        assert_len_ok!(cdl_engulfing(&open, &high, &low, &close));
        assert_len_ok!(cdl_harami(&open, &high, &low, &close));
        assert_len_ok!(cdl_harami_cross(&open, &high, &low, &close));
        assert_len_ok!(cdl_piercing(&open, &high, &low, &close));
        assert_len_ok!(cdl_dark_cloud_cover(&open, &high, &low, &close));
        assert_len_ok!(cdl_doji_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_kicking(&open, &high, &low, &close));
        assert_len_ok!(cdl_kicking_by_length(&open, &high, &low, &close));
        assert_len_ok!(cdl_matching_low(&open, &high, &low, &close));
        assert_len_ok!(cdl_homing_pigeon(&open, &high, &low, &close));
        assert_len_ok!(cdl_in_neck(&open, &high, &low, &close));
        assert_len_ok!(cdl_on_neck(&open, &high, &low, &close));
        assert_len_ok!(cdl_thrusting(&open, &high, &low, &close));
        assert_len_ok!(cdl_separating_lines(&open, &high, &low, &close));
        assert_len_ok!(cdl_counter_attack(&open, &high, &low, &close));
        assert_len_ok!(cdl_2crows(&open, &high, &low, &close));
        assert_len_ok!(cdl_hikkake(&open, &high, &low, &close));
        assert_len_ok!(cdl_hikkake_mod(&open, &high, &low, &close));
    }

    #[test]
    fn test_two_candle_error_surface_all_patterns() {
        type TwoFn = fn(&[f64], &[f64], &[f64], &[f64]) -> crate::error::Result<Vec<i32>>;
        type TwoIntoFn = fn(&[f64], &[f64], &[f64], &[f64], &mut [i32]) -> crate::error::Result<()>;

        let n = 64;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        for i in 0..n {
            let base = 150.0 - (i as f64) * 0.2;
            open.push(base);
            close.push(base - 0.1);
            high.push(base + 4.0);
            low.push(base - 4.0);
        }

        let high_short = high[..(n - 1)].to_vec();
        let low_short = low[..(n - 1)].to_vec();
        let close_short = close[..(n - 1)].to_vec();
        let empty: Vec<f64> = vec![];

        let wrappers: [(&str, TwoFn); 18] = [
            ("cdl_engulfing", cdl_engulfing),
            ("cdl_harami", cdl_harami),
            ("cdl_harami_cross", cdl_harami_cross),
            ("cdl_piercing", cdl_piercing),
            ("cdl_dark_cloud_cover", cdl_dark_cloud_cover),
            ("cdl_doji_star", cdl_doji_star),
            ("cdl_kicking", cdl_kicking),
            ("cdl_kicking_by_length", cdl_kicking_by_length),
            ("cdl_matching_low", cdl_matching_low),
            ("cdl_homing_pigeon", cdl_homing_pigeon),
            ("cdl_in_neck", cdl_in_neck),
            ("cdl_on_neck", cdl_on_neck),
            ("cdl_thrusting", cdl_thrusting),
            ("cdl_separating_lines", cdl_separating_lines),
            ("cdl_counter_attack", cdl_counter_attack),
            ("cdl_2crows", cdl_2crows),
            ("cdl_hikkake", cdl_hikkake),
            ("cdl_hikkake_mod", cdl_hikkake_mod),
        ];

        for (name, f) in wrappers {
            assert!(f(&empty, &empty, &empty, &empty).is_err(), "{name}");
            assert!(f(&open, &high_short, &low, &close).is_err(), "{name}");
            assert!(f(&open, &high, &low_short, &close).is_err(), "{name}");
            assert!(f(&open, &high, &low, &close_short).is_err(), "{name}");
        }

        let into_fns: [(&str, TwoIntoFn); 18] = [
            ("cdl_engulfing_into", cdl_engulfing_into),
            ("cdl_harami_into", cdl_harami_into),
            ("cdl_harami_cross_into", cdl_harami_cross_into),
            ("cdl_piercing_into", cdl_piercing_into),
            ("cdl_dark_cloud_cover_into", cdl_dark_cloud_cover_into),
            ("cdl_doji_star_into", cdl_doji_star_into),
            ("cdl_kicking_into", cdl_kicking_into),
            ("cdl_kicking_by_length_into", cdl_kicking_by_length_into),
            ("cdl_matching_low_into", cdl_matching_low_into),
            ("cdl_homing_pigeon_into", cdl_homing_pigeon_into),
            ("cdl_in_neck_into", cdl_in_neck_into),
            ("cdl_on_neck_into", cdl_on_neck_into),
            ("cdl_thrusting_into", cdl_thrusting_into),
            ("cdl_separating_lines_into", cdl_separating_lines_into),
            ("cdl_counter_attack_into", cdl_counter_attack_into),
            ("cdl_2crows_into", cdl_2crows_into),
            ("cdl_hikkake_into", cdl_hikkake_into),
            ("cdl_hikkake_mod_into", cdl_hikkake_mod_into),
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
    fn test_two_candle_f32_surface_matrix() {
        let n = 320;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        for i in 0..n {
            let base = 170.0_f32 - (i as f32) * 0.13;
            open.push(base);
            close.push(base - 0.08);
            high.push(base + 4.2);
            low.push(base - 4.2);
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

        assert_len_ok!(cdl_engulfing(&open, &high, &low, &close));
        assert_len_ok!(cdl_harami(&open, &high, &low, &close));
        assert_len_ok!(cdl_harami_cross(&open, &high, &low, &close));
        assert_len_ok!(cdl_piercing(&open, &high, &low, &close));
        assert_len_ok!(cdl_dark_cloud_cover(&open, &high, &low, &close));
        assert_len_ok!(cdl_doji_star(&open, &high, &low, &close));
        assert_len_ok!(cdl_kicking(&open, &high, &low, &close));
        assert_len_ok!(cdl_kicking_by_length(&open, &high, &low, &close));
        assert_len_ok!(cdl_matching_low(&open, &high, &low, &close));
        assert_len_ok!(cdl_homing_pigeon(&open, &high, &low, &close));
        assert_len_ok!(cdl_in_neck(&open, &high, &low, &close));
        assert_len_ok!(cdl_on_neck(&open, &high, &low, &close));
        assert_len_ok!(cdl_thrusting(&open, &high, &low, &close));
        assert_len_ok!(cdl_separating_lines(&open, &high, &low, &close));
        assert_len_ok!(cdl_counter_attack(&open, &high, &low, &close));
        assert_len_ok!(cdl_2crows(&open, &high, &low, &close));
        assert_len_ok!(cdl_hikkake(&open, &high, &low, &close));
        assert_len_ok!(cdl_hikkake_mod(&open, &high, &low, &close));

        assert_into_ok!(cdl_engulfing_into);
        assert_into_ok!(cdl_harami_into);
        assert_into_ok!(cdl_harami_cross_into);
        assert_into_ok!(cdl_piercing_into);
        assert_into_ok!(cdl_dark_cloud_cover_into);
        assert_into_ok!(cdl_doji_star_into);
        assert_into_ok!(cdl_kicking_into);
        assert_into_ok!(cdl_kicking_by_length_into);
        assert_into_ok!(cdl_matching_low_into);
        assert_into_ok!(cdl_homing_pigeon_into);
        assert_into_ok!(cdl_in_neck_into);
        assert_into_ok!(cdl_on_neck_into);
        assert_into_ok!(cdl_thrusting_into);
        assert_into_ok!(cdl_separating_lines_into);
        assert_into_ok!(cdl_counter_attack_into);
        assert_into_ok!(cdl_2crows_into);
        assert_into_ok!(cdl_hikkake_into);
        assert_into_ok!(cdl_hikkake_mod_into);
    }

    #[test]
    fn test_two_candle_f32_error_surface_all_patterns() {
        type TwoFn = fn(&[f32], &[f32], &[f32], &[f32]) -> crate::error::Result<Vec<i32>>;
        type TwoIntoFn = fn(&[f32], &[f32], &[f32], &[f32], &mut [i32]) -> crate::error::Result<()>;

        let n = 64;
        let mut open = Vec::with_capacity(n);
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);

        for i in 0..n {
            let base = 155.0_f32 - (i as f32) * 0.2;
            open.push(base);
            close.push(base - 0.1);
            high.push(base + 4.0);
            low.push(base - 4.0);
        }

        let high_short = high[..(n - 1)].to_vec();
        let low_short = low[..(n - 1)].to_vec();
        let close_short = close[..(n - 1)].to_vec();
        let empty: Vec<f32> = vec![];

        let wrappers: [TwoFn; 18] = [
            cdl_engulfing,
            cdl_harami,
            cdl_harami_cross,
            cdl_piercing,
            cdl_dark_cloud_cover,
            cdl_doji_star,
            cdl_kicking,
            cdl_kicking_by_length,
            cdl_matching_low,
            cdl_homing_pigeon,
            cdl_in_neck,
            cdl_on_neck,
            cdl_thrusting,
            cdl_separating_lines,
            cdl_counter_attack,
            cdl_2crows,
            cdl_hikkake,
            cdl_hikkake_mod,
        ];
        for f in wrappers {
            assert!(f(&empty, &empty, &empty, &empty).is_err());
            assert!(f(&open, &high_short, &low, &close).is_err());
            assert!(f(&open, &high, &low_short, &close).is_err());
            assert!(f(&open, &high, &low, &close_short).is_err());
        }

        let into_fns: [TwoIntoFn; 18] = [
            cdl_engulfing_into,
            cdl_harami_into,
            cdl_harami_cross_into,
            cdl_piercing_into,
            cdl_dark_cloud_cover_into,
            cdl_doji_star_into,
            cdl_kicking_into,
            cdl_kicking_by_length_into,
            cdl_matching_low_into,
            cdl_homing_pigeon_into,
            cdl_in_neck_into,
            cdl_on_neck_into,
            cdl_thrusting_into,
            cdl_separating_lines_into,
            cdl_counter_attack_into,
            cdl_2crows_into,
            cdl_hikkake_into,
            cdl_hikkake_mod_into,
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
}
