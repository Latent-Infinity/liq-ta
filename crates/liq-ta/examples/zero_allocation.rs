//! Zero-Allocation Example
//!
//! This example demonstrates how to use the `_into` variants for
//! high-performance scenarios where you want to avoid heap allocations.
//!
//! Run with: `cargo run --example zero_allocation`

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unreadable_literal)]

use liq_ta::indicators::{
    bollinger::{BollingerOutput, bollinger_into},
    ema::ema_into,
    rsi::rsi_into,
    sma::sma_into,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simulate streaming data processing
    let data_size = 1000;

    println!("=== Zero-Allocation Pattern ===");
    println!();
    println!("Processing {data_size} data points with pre-allocated buffers");
    println!();

    // Pre-allocate all buffers once
    let mut sma_output = vec![0.0_f64; data_size];
    let mut ema_output = vec![0.0_f64; data_size];
    let mut rsi_output = vec![0.0_f64; data_size];
    let mut bollinger_output = BollingerOutput {
        upper: vec![0.0_f64; data_size],
        middle: vec![0.0_f64; data_size],
        lower: vec![0.0_f64; data_size],
    };

    // Simulate multiple batches of data
    for batch in 0..3 {
        // Generate synthetic price data for this batch
        let prices: Vec<f64> = (0..data_size)
            .map(|i| 100.0 + (i as f64 * 0.1).sin() * 10.0 + f64::from(batch) * 5.0)
            .collect();

        println!("Batch {}: Processing...", batch + 1);

        // Reuse the same buffers for each computation - ZERO heap allocations!
        sma_into(&prices, 20, &mut sma_output)?;
        ema_into(&prices, 20, &mut ema_output)?;
        rsi_into(&prices, 14, &mut rsi_output)?;
        bollinger_into(&prices, 20, 2.0, &mut bollinger_output)?;

        // Show some results
        let last_idx = data_size - 1;
        println!("  SMA(20):      {:.4}", sma_output[last_idx]);
        println!("  EMA(20):      {:.4}", ema_output[last_idx]);
        println!("  RSI(14):      {:.2}", rsi_output[last_idx]);
        println!(
            "  Bollinger:    {:.4} / {:.4} / {:.4}",
            bollinger_output.lower[last_idx],
            bollinger_output.middle[last_idx],
            bollinger_output.upper[last_idx]
        );
        println!();
    }

    println!("=== Buffer Capacity Verification ===");
    println!();
    println!("Buffer capacities remain unchanged (no reallocation):");
    println!("  sma_output.capacity() = {}", sma_output.capacity());
    println!("  ema_output.capacity() = {}", ema_output.capacity());
    println!("  rsi_output.capacity() = {}", rsi_output.capacity());
    println!(
        "  bollinger upper/middle/lower = {} / {} / {}",
        bollinger_output.upper.capacity(),
        bollinger_output.middle.capacity(),
        bollinger_output.lower.capacity()
    );

    Ok(())
}
