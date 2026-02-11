//! Momentum Indicator Examples
//!
//! This example demonstrates RSI and MACD indicators.
//!
//! Run with: `cargo run --example momentum_indicators`

use liq_ta::indicators::{macd, rsi};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample price data
    let prices: Vec<f64> = vec![
        44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89, 46.03, 45.61, 46.28,
        46.28, 46.00, 46.03, 46.41, 46.22, 45.64, 46.21, 46.25, 45.71, 46.45, 45.78, 45.35, 44.03,
        44.18, 44.22, 44.57, 43.42, 42.66, 43.13,
    ];

    println!("=== RSI (Relative Strength Index) ===");
    println!();

    // Calculate 14-period RSI (standard)
    let rsi_14 = rsi(&prices, 14)?;

    println!("14-period RSI:");
    println!("  Interpretation:");
    println!("    > 70: Overbought");
    println!("    < 30: Oversold");
    println!();

    for (i, value) in rsi_14.iter().enumerate() {
        if !value.is_nan() {
            let signal = if *value > 70.0 {
                "OVERBOUGHT"
            } else if *value < 30.0 {
                "OVERSOLD"
            } else {
                ""
            };
            println!("  [{i}] RSI: {value:>6.2} {signal}");
        }
    }

    println!();
    println!("=== MACD (Moving Average Convergence Divergence) ===");
    println!();

    // Calculate MACD with standard parameters (12, 26, 9)
    let macd_result = macd(&prices, 12, 26, 9)?;

    println!("MACD (12, 26, 9):");
    println!("  MACD Line = EMA(12) - EMA(26)");
    println!("  Signal Line = EMA(9) of MACD Line");
    println!("  Histogram = MACD Line - Signal Line");
    println!();

    println!(
        "  {:>5} {:>10} {:>10} {:>10}",
        "Index", "MACD", "Signal", "Histogram"
    );

    for i in 0..prices.len() {
        if !macd_result.macd_line[i].is_nan() && !macd_result.signal_line[i].is_nan() {
            let crossover = if i > 0
                && !macd_result.macd_line[i - 1].is_nan()
                && !macd_result.signal_line[i - 1].is_nan()
            {
                let prev_diff = macd_result.macd_line[i - 1] - macd_result.signal_line[i - 1];
                let curr_diff = macd_result.macd_line[i] - macd_result.signal_line[i];
                if prev_diff < 0.0 && curr_diff > 0.0 {
                    " <- BULLISH CROSSOVER"
                } else if prev_diff > 0.0 && curr_diff < 0.0 {
                    " <- BEARISH CROSSOVER"
                } else {
                    ""
                }
            } else {
                ""
            };

            println!(
                "  {:>5} {:>10.4} {:>10.4} {:>10.4}{}",
                i,
                macd_result.macd_line[i],
                macd_result.signal_line[i],
                macd_result.histogram[i],
                crossover
            );
        }
    }

    Ok(())
}
