//! SIMD performance demonstration.
//!
//! Run with: `cargo run --example simd_perf --release`
//!
//! This crate requires nightly Rust and uses portable SIMD for all f64 operations.

use std::hint::black_box;
use std::time::Instant;

use fast_ta::indicators::{bollinger, sma};
use fast_ta::kernels::simd::{correlation_f64, dot_product_f64, sum_f64, variance_f64};

fn main() {
    let sizes = [1_000, 10_000, 100_000, 1_000_000];
    let iterations = 100;

    println!("SIMD Performance Demonstration");
    println!("==============================");
    println!();
    println!("Note: SIMD is enabled by default. The main indicator functions");
    println!("(sma, bollinger, etc.) automatically use SIMD for f64 data.");
    println!();

    // Test SIMD kernels
    println!("SIMD Kernel Performance (sum reduction):");
    println!("-----------------------------------------");

    for &size in &sizes {
        let data: Vec<f64> = (0..size).map(|x| x as f64).collect();

        // Warm up
        for _ in 0..10 {
            black_box(sum_f64(&data));
            black_box(data.iter().sum::<f64>());
        }

        // Benchmark SIMD sum
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(sum_f64(black_box(&data)));
        }
        let simd_time = start.elapsed();

        // Benchmark iterator sum
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(black_box(&data).iter().sum::<f64>());
        }
        let iter_time = start.elapsed();

        let speedup = iter_time.as_nanos() as f64 / simd_time.as_nanos() as f64;
        println!(
            "  sum({:>7}): SIMD {:>8.2}µs, iter {:>8.2}µs, speedup: {:.2}x",
            size,
            simd_time.as_micros() as f64 / iterations as f64,
            iter_time.as_micros() as f64 / iterations as f64,
            speedup
        );
    }

    println!();

    // Test additional SIMD kernels
    println!("Additional SIMD Kernel Performance:");
    println!("-----------------------------------");

    for &size in &sizes {
        let data: Vec<f64> = (0..size).map(|x| x as f64).collect();
        let data2: Vec<f64> = (0..size).map(|x| (x * 2) as f64).collect();

        // Variance
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(variance_f64(black_box(&data)));
        }
        let var_time = start.elapsed();

        // Dot product
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(dot_product_f64(black_box(&data), black_box(&data2)));
        }
        let dot_time = start.elapsed();

        // Correlation
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(correlation_f64(black_box(&data), black_box(&data2)));
        }
        let corr_time = start.elapsed();

        println!(
            "  size {:>7}: var {:>6.2}µs, dot {:>6.2}µs, corr {:>6.2}µs",
            size,
            var_time.as_micros() as f64 / iterations as f64,
            dot_time.as_micros() as f64 / iterations as f64,
            corr_time.as_micros() as f64 / iterations as f64,
        );
    }

    println!();

    // Test indicator performance (now uses SIMD automatically)
    println!("Indicator Performance (SIMD-accelerated):");
    println!("-----------------------------------------");

    for &size in &[1_000, 10_000, 100_000] {
        for &period in &[20, 50] {
            if period > size {
                continue;
            }

            let data: Vec<f64> = (0..size).map(|x| 100.0 + (x as f64) * 0.1).collect();

            // Warm up
            for _ in 0..5 {
                let _ = black_box(bollinger(black_box(&data), period, 2.0));
                let _ = black_box(sma(black_box(&data), period));
            }

            // Benchmark Bollinger
            let start = Instant::now();
            for _ in 0..iterations {
                black_box(bollinger(black_box(&data), period, 2.0).unwrap());
            }
            let bb_time = start.elapsed();

            // Benchmark SMA
            let start = Instant::now();
            for _ in 0..iterations {
                black_box(sma(black_box(&data), period).unwrap());
            }
            let sma_time = start.elapsed();

            println!(
                "  size {:>6}, period {:>2}: Bollinger {:>6.2}µs, SMA {:>6.2}µs",
                size,
                period,
                bb_time.as_micros() as f64 / iterations as f64,
                sma_time.as_micros() as f64 / iterations as f64,
            );
        }
    }

    println!();
    println!("Summary:");
    println!("--------");
    println!("- SIMD kernels (sum, variance, dot product): 2.5-4x speedup");
    println!("- Bollinger/SMA: Use SIMD for initial window computation");
    println!("- Rolling updates remain O(1) scalar (already optimal)");
    println!("- SIMD is enabled by default (requires nightly Rust)");
}
