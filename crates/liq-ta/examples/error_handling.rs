//! Error Handling Examples
//!
//! This example demonstrates proper error handling patterns with liq-ta.
//!
//! Run with: `cargo run --example error_handling`

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::wildcard_enum_match_arm)]
#![allow(clippy::items_after_statements)]

use liq_ta::Error;
use liq_ta::indicators::sma;

fn main() {
    println!("=== Error Handling Examples ===");
    println!();

    // Example 1: Empty input
    println!("1. Empty Input:");
    let empty: Vec<f64> = vec![];
    match sma(&empty, 5) {
        Ok(_) => println!("   Unexpected success"),
        Err(Error::EmptyInput) => {
            println!("   Caught EmptyInput error (expected)");
            println!("   Fix: Provide at least one data point");
        }
        Err(e) => println!("   Unexpected error: {e}"),
    }
    println!();

    // Example 2: Insufficient data
    println!("2. Insufficient Data:");
    let short_data = vec![1.0, 2.0, 3.0];
    match sma(&short_data, 10) {
        Ok(_) => println!("   Unexpected success"),
        Err(Error::InsufficientData {
            required,
            actual,
            indicator,
        }) => {
            println!("   Caught InsufficientData error (expected)");
            println!("   Details: {indicator} requires {required} elements, got {actual}");
            println!("   Fix: Use {indicator}_min_len() to check minimum requirements");
        }
        Err(e) => println!("   Unexpected error: {e}"),
    }
    println!();

    // Example 3: Invalid period
    println!("3. Invalid Period (zero):");
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    match sma(&data, 0) {
        Ok(_) => println!("   Unexpected success"),
        Err(Error::InvalidPeriod { period, reason }) => {
            println!("   Caught InvalidPeriod error (expected)");
            println!("   Details: period={period}, reason: {reason}");
            println!("   Fix: Use a period of at least 1");
        }
        Err(e) => println!("   Unexpected error: {e}"),
    }
    println!();

    // Example 4: Using lookback functions to validate
    println!("4. Using Lookback Functions:");
    use liq_ta::indicators::sma::{sma_lookback, sma_min_len};

    let period = 20;
    let data_len = 50;

    println!("   For SMA with period {period}:");
    println!(
        "   - Lookback: {} (NaN values at start)",
        sma_lookback(period)
    );
    println!(
        "   - Min length: {} (minimum input size)",
        sma_min_len(period)
    );
    println!(
        "   - With {} data points: {} valid outputs",
        data_len,
        data_len - sma_lookback(period)
    );
    println!();

    // Example 5: Graceful handling with Result combinators
    println!("5. Using Result Combinators:");

    let prices: Vec<f64> = vec![44.0, 44.5, 43.5, 44.0, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0];

    // Chain operations, handle error at the end
    let result = sma(&prices, 5)
        .map(|sma_values| {
            // Get last valid value
            sma_values
                .iter()
                .rev()
                .find(|&&v| !v.is_nan())
                .copied()
                .unwrap_or(0.0)
        })
        .map(|last_sma| format!("Last SMA value: {last_sma:.4}"));

    match result {
        Ok(msg) => println!("   {msg}"),
        Err(e) => println!("   Error: {e}"),
    }
    println!();

    // Example 6: Converting to standard error types
    println!("6. Converting to Box<dyn Error>:");

    fn process_data(data: &[f64]) -> Result<f64, Box<dyn std::error::Error>> {
        let sma_result = sma(data, 5)?; // ? works with Box<dyn Error>
        Ok(sma_result.last().copied().unwrap_or(f64::NAN))
    }

    match process_data(&prices) {
        Ok(value) => println!("   Success: {value:.4}"),
        Err(e) => println!("   Error: {e}"),
    }

    println!();
    println!("=== All error examples completed ===");
}
