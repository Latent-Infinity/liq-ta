//! Benchmarks for SIMD-accelerated implementations.
//!
//! Run with: `cargo bench --bench simd_comparison`
//!
//! Note: SIMD is now enabled by default. The main indicator functions (sma, bollinger, etc.)
//! automatically use SIMD for f64 data.

use criterion::{
    black_box, criterion_group, criterion_main, measurement::Measurement, BenchmarkGroup,
    BenchmarkId, Criterion,
};
use fast_ta::indicators::{bollinger, sma, sma_into};
use std::time::Duration;

/// Generate test data of specified size
fn generate_data(size: usize) -> Vec<f64> {
    (0..size).map(|x| (x as f64) * 0.5 + 100.0).collect()
}

/// Benchmark SMA (uses SIMD internally for f64)
fn bench_sma<M: Measurement>(group: &mut BenchmarkGroup<M>, size: usize, period: usize) {
    let data = generate_data(size);

    group.bench_with_input(
        BenchmarkId::new("sma", format!("{size}x{period}")),
        &(&data, period),
        |b, (data, period)| {
            b.iter(|| sma(black_box(*data), black_box(*period)).unwrap());
        },
    );
}

/// Benchmark SMA with pre-allocated buffer
fn bench_sma_into<M: Measurement>(group: &mut BenchmarkGroup<M>, size: usize, period: usize) {
    let data = generate_data(size);
    let mut output = vec![0.0_f64; size];

    group.bench_with_input(
        BenchmarkId::new("sma_into", format!("{size}x{period}")),
        &(&data, period),
        |b, (data, period)| {
            b.iter(|| sma_into(black_box(*data), black_box(*period), &mut output).unwrap());
        },
    );
}

/// Benchmark Bollinger Bands (uses SIMD internally for f64)
fn bench_bollinger<M: Measurement>(group: &mut BenchmarkGroup<M>, size: usize, period: usize) {
    let data = generate_data(size);

    group.bench_with_input(
        BenchmarkId::new("bollinger", format!("{size}x{period}")),
        &(&data, period),
        |b, (data, period)| {
            b.iter(|| bollinger(black_box(*data), black_box(*period), 2.0).unwrap());
        },
    );
}

fn bench_indicators(c: &mut Criterion) {
    let mut group = c.benchmark_group("indicators_simd");
    group
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(500);

    // Test various sizes and periods
    let configs = [
        // (size, period)
        (1_000, 20),
        (10_000, 20),
        (100_000, 20),
        (100_000, 50),
        (100_000, 100),
        (100_000, 200),
    ];

    for (size, period) in configs {
        bench_sma(&mut group, size, period);
        bench_sma_into(&mut group, size, period);
        bench_bollinger(&mut group, size, period);
    }

    group.finish();
}

/// Benchmark raw SIMD kernels
fn bench_simd_kernels(c: &mut Criterion) {
    use fast_ta::kernels::simd::{
        correlation_f64, dot_product_f64, max_f64, min_f64, sum_and_count_f64, sum_and_sum_sq_f64,
        sum_f64, variance_f64,
    };

    let mut group = c.benchmark_group("simd_kernels");
    group
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5))
        .sample_size(500);

    let sizes = [100, 1_000, 10_000, 100_000];

    for size in sizes {
        let data: Vec<f64> = (0..size).map(|x| x as f64).collect();
        let data2: Vec<f64> = (0..size).map(|x| (x * 2) as f64).collect();

        group.bench_with_input(BenchmarkId::new("sum_f64", size), &data, |b, data| {
            b.iter(|| sum_f64(black_box(data)));
        });

        group.bench_with_input(BenchmarkId::new("min_f64", size), &data, |b, data| {
            b.iter(|| min_f64(black_box(data)));
        });

        group.bench_with_input(BenchmarkId::new("max_f64", size), &data, |b, data| {
            b.iter(|| max_f64(black_box(data)));
        });

        group.bench_with_input(
            BenchmarkId::new("sum_and_count_f64", size),
            &data,
            |b, data| {
                b.iter(|| sum_and_count_f64(black_box(data)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sum_and_sum_sq_f64", size),
            &data,
            |b, data| {
                b.iter(|| sum_and_sum_sq_f64(black_box(data)));
            },
        );

        group.bench_with_input(BenchmarkId::new("variance_f64", size), &data, |b, data| {
            b.iter(|| variance_f64(black_box(data)));
        });

        group.bench_with_input(
            BenchmarkId::new("dot_product_f64", size),
            &(&data, &data2),
            |b, (data, data2)| {
                b.iter(|| dot_product_f64(black_box(*data), black_box(*data2)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("correlation_f64", size),
            &(&data, &data2),
            |b, (data, data2)| {
                b.iter(|| correlation_f64(black_box(*data), black_box(*data2)));
            },
        );

        // Compare with iterator sum for baseline
        group.bench_with_input(BenchmarkId::new("iter_sum", size), &data, |b, data| {
            b.iter(|| black_box(data).iter().sum::<f64>());
        });
    }

    group.finish();
}

criterion_group!(benches, bench_indicators, bench_simd_kernels);

criterion_main!(benches);
