//! Basic Moving Average Examples
//!
//! This example demonstrates how to use SMA and EMA indicators.
//!
//! Run with: `cargo run --example basic_moving_averages`

use liq_ta::indicators::{ema, sma};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample price data (e.g., daily closing prices)
    let prices: Vec<f64> = vec![
        44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89, 46.03, 45.61, 46.28,
        46.28, 46.00, 46.03, 46.41, 46.22, 45.64,
    ];

    println!("Price data: {} elements", prices.len());
    println!();

    // Calculate 5-period SMA
    let sma_5 = sma(&prices, 5)?;
    println!("5-period SMA:");
    println!("  Lookback: {} (first {} values are NaN)", 4, 4);
    for (i, &value) in sma_5.iter().enumerate() {
        if !value.is_nan() {
            println!("  [{i}] {value:.4}");
        }
    }
    println!();

    // Calculate 10-period EMA
    let ema_10 = ema(&prices, 10)?;
    println!("10-period EMA:");
    println!("  Lookback: {} (first {} values are NaN)", 9, 9);
    for (i, &value) in ema_10.iter().enumerate() {
        if !value.is_nan() {
            println!("  [{i}] {value:.4}");
        }
    }
    println!();

    // Compare SMA vs EMA responsiveness
    println!("SMA vs EMA comparison (period=5):");
    let sma_compare = sma(&prices, 5)?;
    let ema_compare = ema(&prices, 5)?;

    println!(
        "  {:>5} {:>10} {:>10} {:>10}",
        "Index", "Price", "SMA", "EMA"
    );
    for i in 4..prices.len() {
        println!(
            "  {:>5} {:>10.4} {:>10.4} {:>10.4}",
            i, prices[i], sma_compare[i], ema_compare[i]
        );
    }

    Ok(())
}
