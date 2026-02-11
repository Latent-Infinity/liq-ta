//! Candlestick pattern recognition functions.
//!
//! This module provides implementations of all 61 TA-Lib candlestick pattern
//! recognition functions. Patterns return integer values:
//! - `100` or `200`: Bullish pattern (stronger = higher value)
//! - `-100` or `-200`: Bearish pattern (stronger = more negative)
//! - `0`: No pattern detected
//!
//! # Pattern Categories
//!
//! ## Single-Candle Patterns
//! Patterns based on a single candlestick's shape:
//! - Doji variants: [`cdl_doji`], [`cdl_dragonfly_doji`], [`cdl_gravestone_doji`], [`cdl_longleg_doji`]
//! - Body types: [`cdl_marubozu`], [`cdl_spinning_top`], [`cdl_long_line`], [`cdl_short_line`]
//!
//! ## Two-Candle Patterns
//! Patterns involving two consecutive candlesticks:
//! - [`cdl_engulfing`], [`cdl_harami`], [`cdl_piercing`], [`cdl_dark_cloud_cover`]
//!
//! ## Three-Candle Patterns
//! Patterns involving three consecutive candlesticks:
//! - [`cdl_morning_star`], [`cdl_evening_star`]
//!
//! # Example
//!
//! ```
//! use liq_ta::indicators::candlestick::cdl_doji;
//!
//! let open = vec![100.0_f64, 100.0, 100.0, 100.0, 100.0];
//! let high = vec![105.0, 105.0, 105.0, 105.0, 105.0];
//! let low = vec![95.0, 95.0, 95.0, 95.0, 95.0];
//! let close = vec![100.0, 100.0, 100.0, 100.0, 100.0];  // Doji: open == close
//!
//! let result = cdl_doji(&open, &high, &low, &close).unwrap();
//! // result contains pattern signals (100 for doji, 0 for no pattern)
//! ```

pub mod core;
pub mod single;
pub mod three_candle;
pub mod two_candle;

// Re-export core utilities
pub use core::*;

// Re-export single-candle patterns
pub use single::{
    cdl_belt_hold, cdl_belt_hold_into, cdl_closing_marubozu, cdl_closing_marubozu_into, cdl_doji,
    cdl_doji_into, cdl_dragonfly_doji, cdl_dragonfly_doji_into, cdl_gravestone_doji,
    cdl_gravestone_doji_into, cdl_hammer, cdl_hammer_into, cdl_hanging_man, cdl_hanging_man_into,
    cdl_high_wave, cdl_high_wave_into, cdl_inverted_hammer, cdl_inverted_hammer_into,
    cdl_long_line, cdl_long_line_into, cdl_longleg_doji, cdl_longleg_doji_into, cdl_marubozu,
    cdl_marubozu_into, cdl_rickshaw_man, cdl_rickshaw_man_into, cdl_shooting_star,
    cdl_shooting_star_into, cdl_short_line, cdl_short_line_into, cdl_spinning_top,
    cdl_spinning_top_into, cdl_takuri, cdl_takuri_into,
};

// Re-export two-candle patterns
pub use two_candle::{
    cdl_2crows, cdl_2crows_into, cdl_counter_attack, cdl_counter_attack_into, cdl_dark_cloud_cover,
    cdl_dark_cloud_cover_into, cdl_doji_star, cdl_doji_star_into, cdl_engulfing,
    cdl_engulfing_into, cdl_harami, cdl_harami_cross, cdl_harami_cross_into, cdl_harami_into,
    cdl_hikkake, cdl_hikkake_into, cdl_hikkake_mod, cdl_hikkake_mod_into, cdl_homing_pigeon,
    cdl_homing_pigeon_into, cdl_in_neck, cdl_in_neck_into, cdl_kicking, cdl_kicking_by_length,
    cdl_kicking_by_length_into, cdl_kicking_into, cdl_matching_low, cdl_matching_low_into,
    cdl_on_neck, cdl_on_neck_into, cdl_piercing, cdl_piercing_into, cdl_separating_lines,
    cdl_separating_lines_into, cdl_thrusting, cdl_thrusting_into,
};

// Re-export three-candle patterns
pub use three_candle::{
    cdl_3black_crows, cdl_3black_crows_into, cdl_3inside, cdl_3inside_into, cdl_3line_strike,
    cdl_3line_strike_into, cdl_3outside, cdl_3outside_into, cdl_3stars_in_south,
    cdl_3stars_in_south_into, cdl_3white_soldiers, cdl_3white_soldiers_into, cdl_abandoned_baby,
    cdl_abandoned_baby_into, cdl_advance_block, cdl_advance_block_into, cdl_breakaway,
    cdl_breakaway_into, cdl_concealing_baby_swallow, cdl_concealing_baby_swallow_into,
    cdl_evening_doji_star, cdl_evening_doji_star_into, cdl_evening_star, cdl_evening_star_into,
    cdl_gap_side_side_white, cdl_gap_side_side_white_into, cdl_identical_3crows,
    cdl_identical_3crows_into, cdl_ladder_bottom, cdl_ladder_bottom_into, cdl_mat_hold,
    cdl_mat_hold_into, cdl_morning_doji_star, cdl_morning_doji_star_into, cdl_morning_star,
    cdl_morning_star_into, cdl_rise_fall_3methods, cdl_rise_fall_3methods_into,
    cdl_stalled_pattern, cdl_stalled_pattern_into, cdl_stick_sandwich, cdl_stick_sandwich_into,
    cdl_tasuki_gap, cdl_tasuki_gap_into, cdl_tristar, cdl_tristar_into, cdl_unique_3river,
    cdl_unique_3river_into, cdl_upside_gap_2crows, cdl_upside_gap_2crows_into,
    cdl_xside_gap_3methods, cdl_xside_gap_3methods_into,
};
