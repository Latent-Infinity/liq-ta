//! Realistic workload benchmarks focused on allocation overhead.
//!
//! Run with: `cargo bench -p liq-ta --bench workload`
//!
//! Compares Buffer API usage with per-iteration allocations vs buffer reuse.
//!
//! ## Configuration
//!
//! Uses extended measurement time (15s) with 500 samples for statistical reliability.

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::similar_names)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use liq_ta::indicators::{
    BollingerOutput, StochasticOutput, adx::adx_into, atr::atr_into, bollinger::bollinger_into,
    ema::ema_into, macd::macd_into, obv::obv_into, rsi::rsi_into, sma::sma_into,
    stochastic::stochastic_fast_into, vwap::vwap_into,
};
use std::time::Duration;

struct WorkloadData {
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
}

struct WorkloadBuffers {
    sma: Vec<f64>,
    ema: Vec<f64>,
    rsi: Vec<f64>,
    rsi_ema: Vec<f64>,
    macd: Vec<f64>,
    macd_signal: Vec<f64>,
    macd_hist: Vec<f64>,
    atr: Vec<f64>,
    bollinger: BollingerOutput<f64>,
    stochastic: StochasticOutput<f64>,
    adx: Vec<f64>,
    plus_di: Vec<f64>,
    minus_di: Vec<f64>,
    obv: Vec<f64>,
    vwap: Vec<f64>,
}

fn generate_ohlcv(size: usize) -> WorkloadData {
    let mut high = Vec::with_capacity(size);
    let mut low = Vec::with_capacity(size);
    let mut close = Vec::with_capacity(size);
    let mut volume = Vec::with_capacity(size);

    let mut price = 100.0;
    for i in 0..size {
        let delta = ((i as f64 * 0.1).sin() * 2.0) + ((i as f64 * 0.03).cos() * 1.5);
        price += delta;
        price = price.max(10.0);

        let h = price + 1.0 + (i as f64 * 0.07).sin().abs();
        let l = price - 1.0 - (i as f64 * 0.05).cos().abs();
        let c = price + ((i as f64 * 0.02).tan() * 0.5).clamp(-0.8, 0.8);
        let v = 1_000_000.0 + (i as f64 * 1000.0).sin() * 500_000.0;

        high.push(h);
        low.push(l);
        close.push(c);
        volume.push(v.abs());
    }

    WorkloadData {
        high,
        low,
        close,
        volume,
    }
}

fn allocate_buffers(len: usize) -> WorkloadBuffers {
    WorkloadBuffers {
        sma: vec![0.0; len],
        ema: vec![0.0; len],
        rsi: vec![0.0; len],
        rsi_ema: vec![0.0; len],
        macd: vec![0.0; len],
        macd_signal: vec![0.0; len],
        macd_hist: vec![0.0; len],
        atr: vec![0.0; len],
        bollinger: BollingerOutput {
            upper: vec![0.0; len],
            middle: vec![0.0; len],
            lower: vec![0.0; len],
        },
        stochastic: StochasticOutput {
            k: vec![0.0; len],
            d: vec![0.0; len],
        },
        adx: vec![0.0; len],
        plus_di: vec![0.0; len],
        minus_di: vec![0.0; len],
        obv: vec![0.0; len],
        vwap: vec![0.0; len],
    }
}

fn run_workload(data: &WorkloadData, buffers: &mut WorkloadBuffers) {
    let len = data.close.len();

    let sma_valid = sma_into(&data.close, 20, &mut buffers.sma).unwrap();
    let ema_valid = ema_into(&data.close, 20, &mut buffers.ema).unwrap();
    let rsi_valid = rsi_into(&data.close, 14, &mut buffers.rsi).unwrap();
    let rsi_ema_valid = ema_into(&buffers.rsi, 5, &mut buffers.rsi_ema).unwrap();
    let (macd_valid, macd_signal_valid) = macd_into(
        &data.close,
        12,
        26,
        9,
        &mut buffers.macd,
        &mut buffers.macd_signal,
        &mut buffers.macd_hist,
    )
    .unwrap();
    let atr_valid = atr_into(&data.high, &data.low, &data.close, 14, &mut buffers.atr).unwrap();
    let boll_valid = bollinger_into(&data.close, 20, 2.0, &mut buffers.bollinger).unwrap();
    let (stoch_k_valid, stoch_d_valid) = stochastic_fast_into(
        &data.high,
        &data.low,
        &data.close,
        14,
        3,
        &mut buffers.stochastic,
    )
    .unwrap();
    let adx_valid = adx_into(
        &data.high,
        &data.low,
        &data.close,
        14,
        &mut buffers.adx,
        &mut buffers.plus_di,
        &mut buffers.minus_di,
    )
    .unwrap();
    let obv_valid = obv_into(&data.close, &data.volume, &mut buffers.obv).unwrap();
    let vwap_valid = vwap_into(
        &data.high,
        &data.low,
        &data.close,
        &data.volume,
        &mut buffers.vwap,
    )
    .unwrap();

    black_box((
        len,
        sma_valid,
        ema_valid,
        rsi_valid,
        rsi_ema_valid,
        macd_valid,
        macd_signal_valid,
        atr_valid,
        boll_valid,
        stoch_k_valid,
        stoch_d_valid,
        adx_valid,
        obv_valid,
        vwap_valid,
    ));
}

fn bench_workload(c: &mut Criterion) {
    let size = 100_000;
    let data = generate_ohlcv(size);

    let mut group = c.benchmark_group("workload_backtest");
    group.throughput(Throughput::Elements(size as u64));

    group.bench_with_input(
        BenchmarkId::new("alloc_each_iter", size),
        &data,
        |b, data| {
            b.iter(|| {
                let mut buffers = allocate_buffers(data.close.len());
                run_workload(black_box(data), &mut buffers);
            });
        },
    );

    group.bench_with_input(BenchmarkId::new("reuse_buffers", size), &data, |b, data| {
        let mut buffers = allocate_buffers(data.close.len());
        b.iter(|| run_workload(black_box(data), &mut buffers));
    });

    group.finish();
}

// Workload benchmark with extended configuration for reliability
criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(15))
        .sample_size(500);
    targets = bench_workload
}

criterion_main!(benches);
