//! Volatility Indicator Examples
//!
//! This example demonstrates ATR and Bollinger Bands using OHLC data.
//!
//! Run with: `cargo run --example volatility_indicators`

#![allow(clippy::unreadable_literal)]
#![allow(clippy::needless_range_loop)]

use liq_ta::indicators::{atr, bollinger, donchian};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample OHLC data (Open, High, Low, Close)
    let high: Vec<f64> = vec![
        48.70, 48.72, 48.90, 48.87, 48.82, 49.05, 49.20, 49.35, 49.92, 50.19, 50.12, 49.66, 49.88,
        50.19, 50.36, 50.57, 50.65, 50.43, 49.63, 50.33,
    ];
    let low: Vec<f64> = vec![
        47.79, 48.14, 48.39, 48.37, 48.24, 48.64, 48.94, 48.86, 49.50, 49.87, 49.20, 48.90, 49.43,
        49.73, 49.26, 50.09, 50.30, 49.21, 48.98, 49.61,
    ];
    let close: Vec<f64> = vec![
        48.16, 48.61, 48.75, 48.63, 48.74, 49.03, 49.07, 49.32, 49.91, 50.13, 49.53, 49.50, 49.75,
        50.03, 50.31, 50.52, 50.41, 49.34, 49.37, 50.23,
    ];

    println!("=== ATR (Average True Range) ===");
    println!();

    // Calculate 14-period ATR
    let atr_14 = atr(&high, &low, &close, 14)?;

    println!("14-period ATR (measures volatility):");
    println!("  Higher ATR = Higher volatility");
    println!("  Lower ATR = Lower volatility");
    println!();

    for (i, &value) in atr_14.iter().enumerate() {
        if !value.is_nan() {
            println!("  [{i}] ATR: {value:.4}");
        }
    }

    println!();
    println!("=== Bollinger Bands ===");
    println!();

    // Calculate Bollinger Bands (20-period, 2 std dev)
    let bb = bollinger(&close, 20, 2.0)?;

    println!("Bollinger Bands (20, 2.0):");
    println!("  Upper Band = SMA(20) + 2 * StdDev");
    println!("  Middle Band = SMA(20)");
    println!("  Lower Band = SMA(20) - 2 * StdDev");
    println!();

    println!(
        "  {:>5} {:>8} {:>10} {:>10} {:>10}",
        "Index", "Close", "Upper", "Middle", "Lower"
    );

    for i in 0..close.len() {
        if !bb.middle[i].is_nan() {
            let position = if close[i] > bb.upper[i] {
                " ABOVE"
            } else if close[i] < bb.lower[i] {
                " BELOW"
            } else {
                ""
            };
            println!(
                "  {:>5} {:>8.2} {:>10.4} {:>10.4} {:>10.4}{}",
                i, close[i], bb.upper[i], bb.middle[i], bb.lower[i], position
            );
        }
    }

    println!();
    println!("=== Donchian Channels ===");
    println!();

    // Calculate 10-period Donchian Channels
    let dc = donchian(&high, &low, 10)?;

    println!("10-period Donchian Channels:");
    println!("  Upper = Highest High over period");
    println!("  Lower = Lowest Low over period");
    println!("  Middle = (Upper + Lower) / 2");
    println!();

    println!(
        "  {:>5} {:>10} {:>10} {:>10}",
        "Index", "Upper", "Middle", "Lower"
    );

    for i in 0..high.len() {
        if !dc.upper[i].is_nan() {
            println!(
                "  {:>5} {:>10.2} {:>10.2} {:>10.2}",
                i, dc.upper[i], dc.middle[i], dc.lower[i]
            );
        }
    }

    Ok(())
}
