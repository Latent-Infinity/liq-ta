//! Test Data Generator for Precision Validation
//!
//! This module generates deterministic test datasets for precision validation tests.
//! All generated data is reproducible using the seed `0xFA57_7A00` (mnemonic: "FAST-TA-00").
//!
//! # Running the Generator
//!
//! To regenerate the test data:
//! ```sh
//! REGENERATE_TEST_DATA=1 cargo test -p fast-ta --test generate_test_data
//! ```
//!
//! # Generated Files
//!
//! - `random_walk.json`: 10,000 bars of random walk price data
//! - `extreme_values.json`: Data with extreme values for edge case testing
//! - `near_constant.json`: Nearly constant data for variance stress testing
//! - `typical_ohlcv.json`: Realistic OHLCV data with volume

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unreadable_literal)]

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::fs;

/// Reproducibility seed (mnemonic: "FAST-TA-00")
const SEED: u64 = 0xFA57_7A00;

/// Schema version for forward compatibility
const SCHEMA_VERSION: &str = "1.0";

// =============================================================================
// Data Structures
// =============================================================================

#[derive(Serialize, Deserialize)]
struct PriceDataset {
    metadata: Metadata,
    data: Vec<f64>,
}

#[derive(Serialize, Deserialize)]
struct OhlcvDataset {
    metadata: Metadata,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<u64>,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    seed: u64,
    schema_version: String,
    description: String,
    bars: usize,
}

// =============================================================================
// Generators
// =============================================================================

fn output_dir() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/test-data/precision", manifest_dir)
}

fn should_regenerate() -> bool {
    std::env::var("REGENERATE_TEST_DATA")
        .map_or(false, |v| v == "1" || v.to_lowercase() == "true")
}

/// Generate random walk price series.
///
/// Parameters:
/// - start: 100.0
/// - volatility: 2% daily
/// - bars: 10,000
fn generate_random_walk() -> PriceDataset {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED);
    let mut data = Vec::with_capacity(10_000);
    let mut price = 100.0_f64;
    let volatility = 0.02; // 2% daily

    for _ in 0..10_000 {
        data.push(price);
        // Log-normal random walk: price *= exp(volatility * N(0,1))
        let z: f64 = rng.sample(rand::distributions::Standard);
        price *= (volatility * z).exp();
        // Ensure price stays positive
        price = price.max(0.01);
    }

    PriceDataset {
        metadata: Metadata {
            seed: SEED,
            schema_version: SCHEMA_VERSION.to_string(),
            description: "Random walk price series: start=100, volatility=2%".to_string(),
            bars: 10_000,
        },
        data,
    }
}

/// Generate extreme values dataset for edge case testing.
///
/// Includes:
/// - Large values: ±1e6
/// - Small values: ±1e-6
/// - Alternating patterns
/// - Zeros and near-zeros
fn generate_extreme_values() -> PriceDataset {
    let mut data = Vec::with_capacity(1_000);

    // Section 1: Large positive values (200 bars)
    for i in 0..200 {
        data.push(1e6 + (i as f64) * 1000.0);
    }

    // Section 2: Large negative values (200 bars)
    for i in 0..200 {
        data.push(-1e6 + (i as f64) * 1000.0);
    }

    // Section 3: Small positive values near zero (200 bars)
    for i in 0..200 {
        data.push(1e-6 + (i as f64) * 1e-8);
    }

    // Section 4: Alternating large/small (200 bars)
    for i in 0..200 {
        if i % 2 == 0 {
            data.push(1e6);
        } else {
            data.push(1e-6);
        }
    }

    // Section 5: Mixed values with zeros (200 bars)
    let mut rng = ChaCha8Rng::seed_from_u64(SEED + 1);
    for _ in 0..200 {
        let r: f64 = rng.sample(rand::distributions::Standard);
        if r < 0.1 {
            data.push(0.0);
        } else if r < 0.3 {
            data.push(1e-6 * r);
        } else if r < 0.5 {
            data.push(-1e-6 * r);
        } else if r < 0.7 {
            data.push(1e6 * r);
        } else {
            data.push(-1e6 * r);
        }
    }

    PriceDataset {
        metadata: Metadata {
            seed: SEED,
            schema_version: SCHEMA_VERSION.to_string(),
            description: "Extreme values: +-1e6, +-1e-6, alternating, zeros".to_string(),
            bars: 1_000,
        },
        data,
    }
}

/// Generate near-constant data for variance stress testing.
///
/// This triggers catastrophic cancellation in naive variance algorithms.
/// - base: 1000.0
/// - noise: < 1e-4
fn generate_near_constant() -> PriceDataset {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED + 2);
    let mut data = Vec::with_capacity(10_000);
    let base = 1000.0_f64;

    for _ in 0..10_000 {
        // Very small noise relative to base value
        let noise: f64 = rng.sample(rand::distributions::Standard);
        let value = base + noise * 1e-4;
        data.push(value);
    }

    PriceDataset {
        metadata: Metadata {
            seed: SEED,
            schema_version: SCHEMA_VERSION.to_string(),
            description: "Near-constant data: base=1000, noise<1e-4 (variance stress test)"
                .to_string(),
            bars: 10_000,
        },
        data,
    }
}

/// Generate realistic OHLCV data.
///
/// - Bars: 10,000
/// - Starting price: 100
/// - Daily volatility: 2%
/// - Volume: realistic distribution with u64
fn generate_typical_ohlcv() -> OhlcvDataset {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED + 3);
    let bars = 10_000;

    let mut open = Vec::with_capacity(bars);
    let mut high = Vec::with_capacity(bars);
    let mut low = Vec::with_capacity(bars);
    let mut close = Vec::with_capacity(bars);
    let mut volume = Vec::with_capacity(bars);

    let mut price = 100.0_f64;
    let volatility = 0.02;

    for _ in 0..bars {
        let o = price;

        // Generate intrabar range
        let high_pct: f64 = rng.gen_range(0.0..0.03); // 0-3% above open
        let low_pct: f64 = rng.gen_range(0.0..0.03); // 0-3% below open
        let h = o * (1.0 + high_pct);
        let l = o * (1.0 - low_pct);

        // Close somewhere in range
        let c = rng.gen_range(l..=h);

        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);

        // Volume: base 1M with variance
        let vol_mult: f64 = rng.gen_range(0.5..2.0);
        let vol = (1_000_000.0 * vol_mult) as u64;
        volume.push(vol);

        // Next bar opens near this close
        let z: f64 = rng.sample(rand::distributions::Standard);
        price = c * (volatility * 0.5 * z).exp();
        price = price.max(0.01);
    }

    OhlcvDataset {
        metadata: Metadata {
            seed: SEED,
            schema_version: SCHEMA_VERSION.to_string(),
            description: "Typical OHLCV: start=100, volatility=2%, volume as u64".to_string(),
            bars,
        },
        open,
        high,
        low,
        close,
        volume,
    }
}

// =============================================================================
// Tests (run with REGENERATE_TEST_DATA=1 to regenerate)
// =============================================================================

#[test]
fn generate_random_walk_dataset() {
    if !should_regenerate() {
        eprintln!("[SKIP] Set REGENERATE_TEST_DATA=1 to regenerate");
        return;
    }

    let data = generate_random_walk();
    let path = format!("{}/random_walk.json", output_dir());
    let json = serde_json::to_string_pretty(&data).expect("Failed to serialize");
    fs::write(&path, json).expect("Failed to write file");
    eprintln!("[GENERATED] {}", path);
}

#[test]
fn generate_extreme_values_dataset() {
    if !should_regenerate() {
        eprintln!("[SKIP] Set REGENERATE_TEST_DATA=1 to regenerate");
        return;
    }

    let data = generate_extreme_values();
    let path = format!("{}/extreme_values.json", output_dir());
    let json = serde_json::to_string_pretty(&data).expect("Failed to serialize");
    fs::write(&path, json).expect("Failed to write file");
    eprintln!("[GENERATED] {}", path);
}

#[test]
fn generate_near_constant_dataset() {
    if !should_regenerate() {
        eprintln!("[SKIP] Set REGENERATE_TEST_DATA=1 to regenerate");
        return;
    }

    let data = generate_near_constant();
    let path = format!("{}/near_constant.json", output_dir());
    let json = serde_json::to_string_pretty(&data).expect("Failed to serialize");
    fs::write(&path, json).expect("Failed to write file");
    eprintln!("[GENERATED] {}", path);
}

#[test]
fn generate_typical_ohlcv_dataset() {
    if !should_regenerate() {
        eprintln!("[SKIP] Set REGENERATE_TEST_DATA=1 to regenerate");
        return;
    }

    let data = generate_typical_ohlcv();
    let path = format!("{}/typical_ohlcv.json", output_dir());
    let json = serde_json::to_string_pretty(&data).expect("Failed to serialize");
    fs::write(&path, json).expect("Failed to write file");
    eprintln!("[GENERATED] {}", path);
}

#[test]
fn verify_datasets_exist() {
    let dir = output_dir();
    let files = ["random_walk.json", "extreme_values.json", "near_constant.json", "typical_ohlcv.json"];

    for file in &files {
        let path = format!("{}/{}", dir, file);
        if std::path::Path::new(&path).exists() {
            eprintln!("[OK] {} exists", file);
        } else {
            eprintln!(
                "[MISSING] {} - run with REGENERATE_TEST_DATA=1 to create",
                file
            );
        }
    }
}
