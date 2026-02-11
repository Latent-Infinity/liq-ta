//! Precision mode comparison benchmarks for liq-ta indicators.
//!
//! Run with: `cargo bench -p liq-ta --bench precision_comparison`
//!
//! These benchmarks compare performance of High mode vs Fast mode to verify
//! that the precision policy overhead meets the Performance Acceptance Criteria:
//!
//! | Mode | Indicator Type | Max Overhead vs Baseline |
//! |------|----------------|--------------------------|
//! | High | Simple (SMA, Stochastic, ROC) | 15% |
//! | High | Variance-based (Bollinger, VAR) | 20% |
//! | High | Cumulative (VWAP, OBV, AD) | 15% |
//! | High | RSI/Wilder smoothing | 15% |
//! | Fast | All indicators | 2% |

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::similar_names)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use liq_ta::indicators::{
    bollinger::bollinger, cci::cci, mfi::mfi, obv::obv, roc::roc, rsi::rsi, sma::sma,
    statistics::var, stochastic::stochastic, vwap::vwap, williams_r::williams_r,
};
use liq_ta::precision::{PrecisionMode, set_precision_mode};

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

/// Generate synthetic OHLCV data for benchmarks (f32).
fn generate_ohlcv_f32(size: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut high = Vec::with_capacity(size);
    let mut low = Vec::with_capacity(size);
    let mut close = Vec::with_capacity(size);
    let mut volume = Vec::with_capacity(size);

    let mut price = 100.0_f32;
    for i in 0..size {
        let delta = ((i as f32 * 0.1).sin() * 2.0) + ((i as f32 * 0.03).cos() * 1.5);
        price += delta;
        price = price.max(10.0);

        let h = price + 1.0 + (i as f32 * 0.07).sin().abs();
        let l = price - 1.0 - (i as f32 * 0.05).cos().abs();
        let c = price + ((i as f32 * 0.02).tan() * 0.5).clamp(-0.8, 0.8);
        let v = 1_000_000.0 + (i as f32 * 1000.0).sin() * 500_000.0;

        high.push(h);
        low.push(l);
        close.push(c);
        volume.push(v.abs());
    }

    (high, low, close, volume)
}

// Primary benchmark size (matches precision validation)
const SIZE: usize = 10_000;

// =============================================================================
// SMA Benchmarks (Simple - 15% max overhead)
// =============================================================================

fn bench_sma_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_comparison/sma_f32");
    let data = generate_series_f32(SIZE);
    group.throughput(Throughput::Elements(SIZE as u64));

    group.bench_with_input(BenchmarkId::new("fast_mode", SIZE), &data, |b, data| {
        set_precision_mode(PrecisionMode::Fast);
        b.iter(|| sma(black_box(data), black_box(20)));
    });

    group.bench_with_input(BenchmarkId::new("high_mode", SIZE), &data, |b, data| {
        set_precision_mode(PrecisionMode::High);
        b.iter(|| sma(black_box(data), black_box(20)));
    });

    group.finish();
}

// =============================================================================
// RSI Benchmarks (Wilder smoothing - 15% max overhead)
// =============================================================================

fn bench_rsi_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_comparison/rsi_f32");
    let data = generate_series_f32(SIZE);
    group.throughput(Throughput::Elements(SIZE as u64));

    group.bench_with_input(BenchmarkId::new("fast_mode", SIZE), &data, |b, data| {
        set_precision_mode(PrecisionMode::Fast);
        b.iter(|| rsi(black_box(data), black_box(14)));
    });

    group.bench_with_input(BenchmarkId::new("high_mode", SIZE), &data, |b, data| {
        set_precision_mode(PrecisionMode::High);
        b.iter(|| rsi(black_box(data), black_box(14)));
    });

    group.finish();
}

// =============================================================================
// Bollinger Benchmarks (Variance-based - 20% max overhead)
// =============================================================================

fn bench_bollinger_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_comparison/bollinger_f32");
    let data = generate_series_f32(SIZE);
    group.throughput(Throughput::Elements(SIZE as u64));

    group.bench_with_input(BenchmarkId::new("fast_mode", SIZE), &data, |b, data| {
        set_precision_mode(PrecisionMode::Fast);
        b.iter(|| bollinger(black_box(data), black_box(20), black_box(2.0_f32)));
    });

    group.bench_with_input(BenchmarkId::new("high_mode", SIZE), &data, |b, data| {
        set_precision_mode(PrecisionMode::High);
        b.iter(|| bollinger(black_box(data), black_box(20), black_box(2.0_f32)));
    });

    group.finish();
}

// =============================================================================
// VAR Benchmarks (Variance-based - 20% max overhead)
// =============================================================================

fn bench_var_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_comparison/var_f32");
    let data = generate_series_f32(SIZE);
    group.throughput(Throughput::Elements(SIZE as u64));

    group.bench_with_input(BenchmarkId::new("fast_mode", SIZE), &data, |b, data| {
        set_precision_mode(PrecisionMode::Fast);
        b.iter(|| var(black_box(data), black_box(20)));
    });

    group.bench_with_input(BenchmarkId::new("high_mode", SIZE), &data, |b, data| {
        set_precision_mode(PrecisionMode::High);
        b.iter(|| var(black_box(data), black_box(20)));
    });

    group.finish();
}

// =============================================================================
// Stochastic Benchmarks (Simple - 15% max overhead)
// =============================================================================

fn bench_stochastic_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_comparison/stochastic_f32");
    let (high, low, close, _) = generate_ohlcv_f32(SIZE);
    group.throughput(Throughput::Elements(SIZE as u64));

    group.bench_with_input(
        BenchmarkId::new("fast_mode", SIZE),
        &(high.clone(), low.clone(), close.clone()),
        |b, (h, l, c)| {
            set_precision_mode(PrecisionMode::Fast);
            b.iter(|| {
                stochastic(
                    black_box(h),
                    black_box(l),
                    black_box(c),
                    black_box(14),
                    black_box(3),
                    black_box(1),
                )
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("high_mode", SIZE),
        &(high, low, close),
        |b, (h, l, c)| {
            set_precision_mode(PrecisionMode::High);
            b.iter(|| {
                stochastic(
                    black_box(h),
                    black_box(l),
                    black_box(c),
                    black_box(14),
                    black_box(3),
                    black_box(1),
                )
            });
        },
    );

    group.finish();
}

// =============================================================================
// Williams %R Benchmarks (Simple - 15% max overhead)
// =============================================================================

fn bench_williams_r_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_comparison/williams_r_f32");
    let (high, low, close, _) = generate_ohlcv_f32(SIZE);
    group.throughput(Throughput::Elements(SIZE as u64));

    group.bench_with_input(
        BenchmarkId::new("fast_mode", SIZE),
        &(high.clone(), low.clone(), close.clone()),
        |b, (h, l, c)| {
            set_precision_mode(PrecisionMode::Fast);
            b.iter(|| williams_r(black_box(h), black_box(l), black_box(c), black_box(14)));
        },
    );

    group.bench_with_input(
        BenchmarkId::new("high_mode", SIZE),
        &(high, low, close),
        |b, (h, l, c)| {
            set_precision_mode(PrecisionMode::High);
            b.iter(|| williams_r(black_box(h), black_box(l), black_box(c), black_box(14)));
        },
    );

    group.finish();
}

// =============================================================================
// VWAP Benchmarks (Cumulative - 15% max overhead)
// =============================================================================

fn bench_vwap_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_comparison/vwap_f32");
    let (high, low, close, volume) = generate_ohlcv_f32(SIZE);
    group.throughput(Throughput::Elements(SIZE as u64));

    group.bench_with_input(
        BenchmarkId::new("fast_mode", SIZE),
        &(high.clone(), low.clone(), close.clone(), volume.clone()),
        |b, (h, l, c, v)| {
            set_precision_mode(PrecisionMode::Fast);
            b.iter(|| vwap(black_box(h), black_box(l), black_box(c), black_box(v)));
        },
    );

    group.bench_with_input(
        BenchmarkId::new("high_mode", SIZE),
        &(high, low, close, volume),
        |b, (h, l, c, v)| {
            set_precision_mode(PrecisionMode::High);
            b.iter(|| vwap(black_box(h), black_box(l), black_box(c), black_box(v)));
        },
    );

    group.finish();
}

// =============================================================================
// OBV Benchmarks (Cumulative - 15% max overhead)
// =============================================================================

fn bench_obv_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_comparison/obv_f32");
    let (_, _, close, volume) = generate_ohlcv_f32(SIZE);
    group.throughput(Throughput::Elements(SIZE as u64));

    group.bench_with_input(
        BenchmarkId::new("fast_mode", SIZE),
        &(close.clone(), volume.clone()),
        |b, (c, v)| {
            set_precision_mode(PrecisionMode::Fast);
            b.iter(|| obv(black_box(c), black_box(v)));
        },
    );

    group.bench_with_input(
        BenchmarkId::new("high_mode", SIZE),
        &(close, volume),
        |b, (c, v)| {
            set_precision_mode(PrecisionMode::High);
            b.iter(|| obv(black_box(c), black_box(v)));
        },
    );

    group.finish();
}

// =============================================================================
// CCI Benchmarks (Variance-like - 20% max overhead)
// =============================================================================

fn bench_cci_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_comparison/cci_f32");
    let (high, low, close, _) = generate_ohlcv_f32(SIZE);
    group.throughput(Throughput::Elements(SIZE as u64));

    group.bench_with_input(
        BenchmarkId::new("fast_mode", SIZE),
        &(high.clone(), low.clone(), close.clone()),
        |b, (h, l, c)| {
            set_precision_mode(PrecisionMode::Fast);
            b.iter(|| cci(black_box(h), black_box(l), black_box(c), black_box(20)));
        },
    );

    group.bench_with_input(
        BenchmarkId::new("high_mode", SIZE),
        &(high, low, close),
        |b, (h, l, c)| {
            set_precision_mode(PrecisionMode::High);
            b.iter(|| cci(black_box(h), black_box(l), black_box(c), black_box(20)));
        },
    );

    group.finish();
}

// =============================================================================
// MFI Benchmarks (Wilder-like - 15% max overhead)
// =============================================================================

fn bench_mfi_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_comparison/mfi_f32");
    let (high, low, close, volume) = generate_ohlcv_f32(SIZE);
    group.throughput(Throughput::Elements(SIZE as u64));

    group.bench_with_input(
        BenchmarkId::new("fast_mode", SIZE),
        &(high.clone(), low.clone(), close.clone(), volume.clone()),
        |b, (h, l, c, v)| {
            set_precision_mode(PrecisionMode::Fast);
            b.iter(|| {
                mfi(
                    black_box(h),
                    black_box(l),
                    black_box(c),
                    black_box(v),
                    black_box(14),
                )
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("high_mode", SIZE),
        &(high, low, close, volume),
        |b, (h, l, c, v)| {
            set_precision_mode(PrecisionMode::High);
            b.iter(|| {
                mfi(
                    black_box(h),
                    black_box(l),
                    black_box(c),
                    black_box(v),
                    black_box(14),
                )
            });
        },
    );

    group.finish();
}

// =============================================================================
// ROC Benchmarks (Simple - 15% max overhead)
// =============================================================================

fn bench_roc_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("precision_comparison/roc_f32");
    let data = generate_series_f32(SIZE);
    group.throughput(Throughput::Elements(SIZE as u64));

    group.bench_with_input(BenchmarkId::new("fast_mode", SIZE), &data, |b, data| {
        set_precision_mode(PrecisionMode::Fast);
        b.iter(|| roc(black_box(data), black_box(10)));
    });

    group.bench_with_input(BenchmarkId::new("high_mode", SIZE), &data, |b, data| {
        set_precision_mode(PrecisionMode::High);
        b.iter(|| roc(black_box(data), black_box(10)));
    });

    group.finish();
}

// =============================================================================
// Benchmark Groups
// =============================================================================

criterion_group!(
    simple_benches,
    bench_sma_precision,
    bench_stochastic_precision,
    bench_williams_r_precision,
    bench_roc_precision,
);

criterion_group!(
    variance_benches,
    bench_bollinger_precision,
    bench_var_precision,
    bench_cci_precision,
);

criterion_group!(
    cumulative_benches,
    bench_vwap_precision,
    bench_obv_precision,
);

criterion_group!(wilder_benches, bench_rsi_precision, bench_mfi_precision,);

criterion_main!(
    simple_benches,
    variance_benches,
    cumulative_benches,
    wilder_benches
);
