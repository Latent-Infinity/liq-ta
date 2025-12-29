//! Performance benchmarks for fast-ta indicators.
//!
//! Run with: `cargo bench -p fast-ta`
//!
//! These benchmarks measure throughput for each indicator across various
//! input sizes to validate O(n) complexity and establish performance baselines.
//!
//! ## Configuration
//!
//! Uses criterion.toml for default settings:
//! - 5s warmup, 10s measurement, 500 samples
//! - 2% noise threshold, 95% confidence level
//!
//! Slower benchmarks (stochastic) use extended 15s measurement time.

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::similar_names)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::type_complexity)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use fast_ta::indicators::{
    ad::ad, adx::adx, atr::atr, bollinger::bollinger, donchian::donchian, ema::ema, macd::macd, mfi::mfi,
    obv::obv, price_transform::{avgprice, medprice, typprice, wclprice},
    roc::roc, rsi::rsi, sma::sma, statistics::var, stochastic::stochastic, vwap::vwap, williams_r::williams_r,
};

/// Generate synthetic OHLCV data for benchmarks.
fn generate_ohlcv(size: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(size);
    let mut low = Vec::with_capacity(size);
    let mut close = Vec::with_capacity(size);
    let mut open = Vec::with_capacity(size);
    let mut volume = Vec::with_capacity(size);

    let mut price = 100.0;
    for i in 0..size {
        // Simple deterministic price movement for reproducibility
        let delta = ((i as f64 * 0.1).sin() * 2.0) + ((i as f64 * 0.03).cos() * 1.5);
        price += delta;
        price = price.max(10.0); // Keep price positive

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

/// Generate single series for simple indicators.
fn generate_series(size: usize) -> Vec<f64> {
    let mut data = Vec::with_capacity(size);
    let mut price = 100.0;
    for i in 0..size {
        let delta = ((i as f64 * 0.1).sin() * 2.0) + ((i as f64 * 0.03).cos() * 1.5);
        price += delta;
        price = price.max(10.0);
        data.push(price);
    }
    data
}

// Standard sizes for benchmarking
const SIZES: &[usize] = &[100, 1_000, 10_000, 100_000];

fn bench_sma(c: &mut Criterion) {
    let mut group = c.benchmark_group("sma");
    for &size in SIZES {
        let data = generate_series(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| sma(black_box(data), black_box(20)));
        });
    }
    group.finish();
}

fn bench_ema(c: &mut Criterion) {
    let mut group = c.benchmark_group("ema");
    for &size in SIZES {
        let data = generate_series(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| ema(black_box(data), black_box(20)));
        });
    }
    group.finish();
}

fn bench_rsi(c: &mut Criterion) {
    let mut group = c.benchmark_group("rsi");
    for &size in SIZES {
        let data = generate_series(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| rsi(black_box(data), black_box(14)));
        });
    }
    group.finish();
}

fn bench_macd(c: &mut Criterion) {
    let mut group = c.benchmark_group("macd");
    for &size in SIZES {
        let data = generate_series(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| macd(black_box(data), black_box(12), black_box(26), black_box(9)));
        });
    }
    group.finish();
}

fn bench_bollinger(c: &mut Criterion) {
    let mut group = c.benchmark_group("bollinger");
    for &size in SIZES {
        let data = generate_series(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| bollinger(black_box(data), black_box(20), black_box(2.0)));
        });
    }
    group.finish();
}

fn bench_atr(c: &mut Criterion) {
    let mut group = c.benchmark_group("atr");
    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low, close),
            |b, (h, l, c)| b.iter(|| atr(black_box(h), black_box(l), black_box(c), black_box(14))),
        );
    }
    group.finish();
}

fn bench_stochastic(c: &mut Criterion) {
    let mut group = c.benchmark_group("stochastic");
    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
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

fn bench_adx(c: &mut Criterion) {
    let mut group = c.benchmark_group("adx");
    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low, close),
            |b, (h, l, c)| b.iter(|| adx(black_box(h), black_box(l), black_box(c), black_box(14))),
        );
    }
    group.finish();
}

fn bench_williams_r(c: &mut Criterion) {
    let mut group = c.benchmark_group("williams_r");
    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low, close),
            |b, (h, l, c)| {
                b.iter(|| williams_r(black_box(h), black_box(l), black_box(c), black_box(14)));
            },
        );
    }
    group.finish();
}

fn bench_donchian(c: &mut Criterion) {
    let mut group = c.benchmark_group("donchian");
    for &size in SIZES {
        let (_, high, low, _, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low),
            |b, (h, l)| b.iter(|| donchian(black_box(h), black_box(l), black_box(20))),
        );
    }
    group.finish();
}

fn bench_obv(c: &mut Criterion) {
    let mut group = c.benchmark_group("obv");
    for &size in SIZES {
        let (_, _, _, close, volume) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(close, volume),
            |b, (c, v)| b.iter(|| obv(black_box(c), black_box(v))),
        );
    }
    group.finish();
}

fn bench_vwap(c: &mut Criterion) {
    let mut group = c.benchmark_group("vwap");
    for &size in SIZES {
        let (_, high, low, close, volume) = generate_ohlcv(size);
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

// Price transform indicators - simple IEEE 754 NaN propagation pattern
fn bench_avgprice(c: &mut Criterion) {
    let mut group = c.benchmark_group("avgprice");
    for &size in SIZES {
        let (open, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(open, high, low, close),
            |b, (o, h, l, c)| {
                b.iter(|| avgprice(black_box(o), black_box(h), black_box(l), black_box(c)));
            },
        );
    }
    group.finish();
}

fn bench_medprice(c: &mut Criterion) {
    let mut group = c.benchmark_group("medprice");
    for &size in SIZES {
        let (_, high, low, _, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low),
            |b, (h, l)| {
                b.iter(|| medprice(black_box(h), black_box(l)));
            },
        );
    }
    group.finish();
}

fn bench_typprice(c: &mut Criterion) {
    let mut group = c.benchmark_group("typprice");
    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low, close),
            |b, (h, l, c)| {
                b.iter(|| typprice(black_box(h), black_box(l), black_box(c)));
            },
        );
    }
    group.finish();
}

fn bench_wclprice(c: &mut Criterion) {
    let mut group = c.benchmark_group("wclprice");
    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low, close),
            |b, (h, l, c)| {
                b.iter(|| wclprice(black_box(h), black_box(l), black_box(c)));
            },
        );
    }
    group.finish();
}

// AD (Accumulation/Distribution) - Division-based with IEEE 754 NaN propagation
fn bench_ad(c: &mut Criterion) {
    let mut group = c.benchmark_group("ad");
    for &size in SIZES {
        let (_, high, low, close, volume) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low, close, volume),
            |b, (h, l, c, v)| {
                b.iter(|| ad(black_box(h), black_box(l), black_box(c), black_box(v)));
            },
        );
    }
    group.finish();
}

// ROC (Rate of Change) - Division-based with IEEE 754 NaN propagation
fn bench_roc(c: &mut Criterion) {
    let mut group = c.benchmark_group("roc");
    for &size in SIZES {
        let data = generate_series(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| roc(black_box(data), black_box(10)));
        });
    }
    group.finish();
}

fn bench_mfi(c: &mut Criterion) {
    let mut group = c.benchmark_group("mfi");
    for &size in SIZES {
        let (_, high, low, close, volume) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(high, low, close, volume),
            |b, (h, l, c, v)| {
                b.iter(|| mfi(black_box(h), black_box(l), black_box(c), black_box(v), black_box(14)));
            },
        );
    }
    group.finish();
}

fn bench_var(c: &mut Criterion) {
    let mut group = c.benchmark_group("var");
    for &size in SIZES {
        let data = generate_series(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| var(black_box(data), black_box(20)));
        });
    }
    group.finish();
}

// Standard benchmarks with default configuration
criterion_group!(
    benches,
    bench_sma,
    bench_ema,
    bench_rsi,
    bench_macd,
    bench_bollinger,
    bench_atr,
    bench_adx,
    bench_williams_r,
    bench_donchian,
    bench_obv,
    bench_vwap,
    bench_avgprice,
    bench_medprice,
    bench_typprice,
    bench_wclprice,
    bench_ad,
    bench_roc,
    bench_mfi,
    bench_var,
);

// Slower benchmarks need extended measurement time
criterion_group! {
    name = slow_benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(15))
        .sample_size(500);
    targets = bench_stochastic
}

criterion_main!(benches, slow_benches);