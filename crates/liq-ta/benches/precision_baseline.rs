//! Precision baseline benchmarks for liq-ta indicators.
//!
//! Run with: `cargo bench -p liq-ta --bench precision_baseline`
//!
//! These benchmarks establish the performance baseline before precision policy
//! changes are implemented. Results are saved for comparison after Stage 2.
//!
//! ## Key Indicators
//!
//! Benchmarks cover the indicators targeted for precision improvements:
//! - SMA: Rolling sum (simple case)
//! - Bollinger: Variance-based (sum-of-squares)
//! - Stochastic: Sensitive division
//! - RSI: Wilder smoothing
//! - VWAP: Cumulative sums
//! - OBV: Cumulative volume
//! - EMA: Reference (not targeted for precision change)
//!
//! ## Input Types
//!
//! Each indicator is benchmarked with both f32 and f64 input types to
//! establish type-specific baselines.

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::similar_names)]
#![allow(clippy::type_complexity)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use liq_ta::indicators::{
    bollinger::bollinger, ema::ema, obv::obv, rsi::rsi, sma::sma, stochastic::stochastic,
    vwap::vwap,
};

/// Generate single price series for simple indicators (f64).
fn generate_series_f64(size: usize) -> Vec<f64> {
    let mut data = Vec::with_capacity(size);
    let mut price = 100.0_f64;
    for i in 0..size {
        let delta = ((i as f64 * 0.1).sin() * 2.0) + ((i as f64 * 0.03).cos() * 1.5);
        price += delta;
        price = price.max(10.0);
        data.push(price);
    }
    data
}

/// Generate single price series for simple indicators (f32).
fn generate_series_f32(size: usize) -> Vec<f32> {
    let mut data = Vec::with_capacity(size);
    let mut price = 100.0_f32;
    for i in 0..size {
        let delta = ((i as f32 * 0.1).sin() * 2.0) + ((i as f32 * 0.03).cos() * 1.5);
        price += delta;
        price = price.max(10.0);
        data.push(price);
    }
    data
}

/// Generate synthetic OHLCV data for benchmarks (f64).
fn generate_ohlcv_f64(size: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(size);
    let mut low = Vec::with_capacity(size);
    let mut close = Vec::with_capacity(size);
    let mut open = Vec::with_capacity(size);
    let mut volume = Vec::with_capacity(size);

    let mut price = 100.0_f64;
    for i in 0..size {
        let delta = ((i as f64 * 0.1).sin() * 2.0) + ((i as f64 * 0.03).cos() * 1.5);
        price += delta;
        price = price.max(10.0);

        let h = price + 1.0 + (i as f64 * 0.07).sin().abs();
        let l = price - 1.0 - (i as f64 * 0.05).cos().abs();
        let c = price + ((i as f64 * 0.02).tan() * 0.5).clamp(-0.8, 0.8);
        let o = price + ((i as f64 * 0.04).sin() * 0.3);
        let v = 1_000_000.0 + (i as f64 * 1000.0).sin() * 500_000.0;

        high.push(h);
        low.push(l);
        close.push(c);
        open.push(o);
        volume.push(v.abs());
    }

    (open, high, low, close, volume)
}

/// Generate synthetic OHLCV data for benchmarks (f32).
fn generate_ohlcv_f32(size: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut high = Vec::with_capacity(size);
    let mut low = Vec::with_capacity(size);
    let mut close = Vec::with_capacity(size);
    let mut open = Vec::with_capacity(size);
    let mut volume = Vec::with_capacity(size);

    let mut price = 100.0_f32;
    for i in 0..size {
        let delta = ((i as f32 * 0.1).sin() * 2.0) + ((i as f32 * 0.03).cos() * 1.5);
        price += delta;
        price = price.max(10.0);

        let h = price + 1.0 + (i as f32 * 0.07).sin().abs();
        let l = price - 1.0 - (i as f32 * 0.05).cos().abs();
        let c = price + ((i as f32 * 0.02).tan() * 0.5).clamp(-0.8, 0.8);
        let o = price + ((i as f32 * 0.04).sin() * 0.3);
        let v = 1_000_000.0 + (i as f32 * 1000.0).sin() * 500_000.0;

        high.push(h);
        low.push(l);
        close.push(c);
        open.push(o);
        volume.push(v.abs());
    }

    (open, high, low, close, volume)
}

// Sizes for precision baseline benchmarks (includes 1M as per plan)
const SIZES: &[usize] = &[1_000, 10_000, 100_000, 1_000_000];

// =============================================================================
// SMA Benchmarks
// =============================================================================

fn bench_sma_f64(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/sma_f64");
    for &size in SIZES {
        let data = generate_series_f64(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| sma(black_box(data), black_box(20)));
        });
    }
    group.finish();
}

fn bench_sma_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/sma_f32");
    for &size in SIZES {
        let data = generate_series_f32(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| sma(black_box(data), black_box(20)));
        });
    }
    group.finish();
}

// =============================================================================
// EMA Benchmarks (reference - not targeted for precision change)
// =============================================================================

fn bench_ema_f64(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/ema_f64");
    for &size in SIZES {
        let data = generate_series_f64(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| ema(black_box(data), black_box(20)));
        });
    }
    group.finish();
}

fn bench_ema_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/ema_f32");
    for &size in SIZES {
        let data = generate_series_f32(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| ema(black_box(data), black_box(20)));
        });
    }
    group.finish();
}

// =============================================================================
// RSI Benchmarks (Wilder smoothing)
// =============================================================================

fn bench_rsi_f64(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/rsi_f64");
    for &size in SIZES {
        let data = generate_series_f64(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| rsi(black_box(data), black_box(14)));
        });
    }
    group.finish();
}

fn bench_rsi_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/rsi_f32");
    for &size in SIZES {
        let data = generate_series_f32(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| rsi(black_box(data), black_box(14)));
        });
    }
    group.finish();
}

// =============================================================================
// Bollinger Benchmarks (variance-based)
// =============================================================================

fn bench_bollinger_f64(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/bollinger_f64");
    for &size in SIZES {
        let data = generate_series_f64(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| bollinger(black_box(data), black_box(20), black_box(2.0)));
        });
    }
    group.finish();
}

fn bench_bollinger_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/bollinger_f32");
    for &size in SIZES {
        let data = generate_series_f32(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| bollinger(black_box(data), black_box(20), black_box(2.0_f32)));
        });
    }
    group.finish();
}

// =============================================================================
// Stochastic Benchmarks (sensitive division)
// =============================================================================

fn bench_stochastic_f64(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/stochastic_f64");
    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv_f64(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low, close),
            |b, (h, l, c)| {
                b.iter(|| {
                    stochastic(
                        black_box(h),
                        black_box(l),
                        black_box(c),
                        black_box(14),
                        black_box(3),
                        black_box(3),
                    )
                });
            },
        );
    }
    group.finish();
}

fn bench_stochastic_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/stochastic_f32");
    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv_f32(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low, close),
            |b, (h, l, c)| {
                b.iter(|| {
                    stochastic(
                        black_box(h),
                        black_box(l),
                        black_box(c),
                        black_box(14),
                        black_box(3),
                        black_box(3),
                    )
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// VWAP Benchmarks (cumulative sums)
// =============================================================================

fn bench_vwap_f64(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/vwap_f64");
    for &size in SIZES {
        let (_, high, low, close, volume) = generate_ohlcv_f64(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low, close, volume),
            |b, (h, l, c, v)| {
                b.iter(|| vwap(black_box(h), black_box(l), black_box(c), black_box(v)));
            },
        );
    }
    group.finish();
}

fn bench_vwap_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/vwap_f32");
    for &size in SIZES {
        let (_, high, low, close, volume) = generate_ohlcv_f32(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low, close, volume),
            |b, (h, l, c, v)| {
                b.iter(|| vwap(black_box(h), black_box(l), black_box(c), black_box(v)));
            },
        );
    }
    group.finish();
}

// =============================================================================
// OBV Benchmarks (cumulative volume)
// =============================================================================

fn bench_obv_f64(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/obv_f64");
    for &size in SIZES {
        let (_, _, _, close, volume) = generate_ohlcv_f64(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(close, volume),
            |b, (c, v)| b.iter(|| obv(black_box(c), black_box(v))),
        );
    }
    group.finish();
}

fn bench_obv_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_baseline/obv_f32");
    for &size in SIZES {
        let (_, _, _, close, volume) = generate_ohlcv_f32(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(close, volume),
            |b, (c, v)| b.iter(|| obv(black_box(c), black_box(v))),
        );
    }
    group.finish();
}

// =============================================================================
// Benchmark Groups
// =============================================================================

criterion_group!(sma_benches, bench_sma_f64, bench_sma_f32,);

criterion_group!(ema_benches, bench_ema_f64, bench_ema_f32,);

criterion_group!(rsi_benches, bench_rsi_f64, bench_rsi_f32,);

criterion_group!(bollinger_benches, bench_bollinger_f64, bench_bollinger_f32,);

criterion_group!(
    stochastic_benches,
    bench_stochastic_f64,
    bench_stochastic_f32,
);

criterion_group!(vwap_benches, bench_vwap_f64, bench_vwap_f32,);

criterion_group!(obv_benches, bench_obv_f64, bench_obv_f32,);

criterion_main!(
    sma_benches,
    ema_benches,
    rsi_benches,
    bollinger_benches,
    stochastic_benches,
    vwap_benches,
    obv_benches,
);
