//! Performance comparison between fast-ta and TA-Lib (via FFI).
//!
//! Run with: `cargo bench -p fast-ta --bench talib_comparison`
//!
//! Requires TA-Lib to be installed on the system.
//!
//! ## Configuration
//!
//! Uses criterion.toml for default settings:
//! - 5s warmup, 10s measurement, 500 samples
//! - 2% noise threshold, 95% confidence level
//!
//! Slower benchmarks (t3, cmo, cci, stochastic, var) use extended 15s measurement time.

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::similar_names)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::many_single_char_names)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

/// OHLCV data tuple: (open, high, low, close, volume)
type OhlcvData = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>);

// TA-Lib FFI bindings
use ta_lib_sys::{
    MAType, AD, ADX, APO, AROON, ATR, BBANDS, BOP, CCI, CMO, DEMA, DX, EMA, KAMA, LINEARREG, MACD,
    MFI, MIDPOINT, MIDPRICE, MOM, OBV, ROC, RSI, SMA, STOCH, STOCHF, T3, TEMA, TRANGE, TRIMA, TRIX,
    TSF, ULTOSC, VAR, WILLR, WMA,
};

// fast-ta indicators
use fast_ta::indicators::{
    ad::ad,
    adx::adx,
    apo::apo,
    aroon::aroon,
    atr::{atr, true_range},
    bollinger::bollinger,
    bop::bop,
    cci::cci,
    cmo::cmo,
    dema::dema,
    dx::dx,
    ema::ema,
    kama::kama,
    macd::macd,
    mfi::mfi,
    midpoint::midpoint,
    midprice::midprice,
    mom::mom_into,
    obv::obv_into,
    roc::roc,
    rsi::rsi,
    sma::sma,
    stochastic::{stochastic, stochastic_fast},
    statistics::{linearreg, tsf, var},
    t3::t3,
    tema::tema,
    trima::trima,
    trix::trix,
    ultosc::ultosc,
    williams_r::williams_r,
    wma::wma,
};

/// Generate synthetic price data for benchmarks.
fn generate_close_prices(size: usize) -> Vec<f64> {
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

/// Generate OHLCV data for benchmarks.
fn generate_ohlcv(size: usize) -> OhlcvData {
    let mut high = Vec::with_capacity(size);
    let mut low = Vec::with_capacity(size);
    let mut close = Vec::with_capacity(size);
    let mut open = Vec::with_capacity(size);
    let mut volume = Vec::with_capacity(size);

    let mut price = 100.0;
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
        volume.push(v);
    }

    (open, high, low, close, volume)
}

// Use 100K for the primary comparison
const SIZES: &[usize] = &[100_000];

// Fibonacci-like periods to reveal O(n) vs O(n×period) complexity
// Short periods favor cache-friendly algorithms, long periods expose O(n×p) scaling
const PERIODS: &[usize] = &[5, 21, 55, 89, 233];

// =============================================================================
// Moving Averages
// =============================================================================

fn bench_sma(c: &mut Criterion) {
    let mut group = c.benchmark_group("sma");

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        for &period in PERIODS {
            let period_i32 = period as i32;

            group.bench_with_input(
                BenchmarkId::new(format!("fast-ta/p{}", period), size),
                &data,
                |b, data| {
                    b.iter(|| sma(black_box(data), black_box(period)));
                },
            );

            group.bench_with_input(
                BenchmarkId::new(format!("ta-lib/p{}", period), size),
                &data,
                |b, data| {
                    b.iter(|| {
                        let mut out_begin: i32 = 0;
                        let mut out_nb_element: i32 = 0;
                        let mut output = vec![0.0f64; data.len()];
                        unsafe {
                            SMA(
                                0,
                                (data.len() - 1) as i32,
                                data.as_ptr(),
                                period_i32,
                                &mut out_begin,
                                &mut out_nb_element,
                                output.as_mut_ptr(),
                            );
                        }
                        black_box(output)
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_ema(c: &mut Criterion) {
    let mut group = c.benchmark_group("ema");
    let period: i32 = 20;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| ema(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    EMA(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

fn bench_wma(c: &mut Criterion) {
    let mut group = c.benchmark_group("wma");
    let period: i32 = 20;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| wma(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    WMA(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

fn bench_dema(c: &mut Criterion) {
    let mut group = c.benchmark_group("dema");
    let period: i32 = 20;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| dema(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    DEMA(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

fn bench_tema(c: &mut Criterion) {
    let mut group = c.benchmark_group("tema");
    let period: i32 = 20;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| tema(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    TEMA(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

fn bench_trima(c: &mut Criterion) {
    let mut group = c.benchmark_group("trima");
    let period: i32 = 20;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| trima(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    TRIMA(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

fn bench_kama(c: &mut Criterion) {
    let mut group = c.benchmark_group("kama");
    let period: i32 = 10;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| kama(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    KAMA(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

fn bench_t3(c: &mut Criterion) {
    let mut group = c.benchmark_group("t3");
    let period: i32 = 5;
    let v_factor: f64 = 0.7;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| t3(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    T3(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        v_factor,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

// =============================================================================
// Momentum Indicators
// =============================================================================

fn bench_rsi(c: &mut Criterion) {
    let mut group = c.benchmark_group("rsi");
    let period: i32 = 14;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| rsi(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    RSI(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

fn bench_macd(c: &mut Criterion) {
    let mut group = c.benchmark_group("macd");
    let fast_period: i32 = 12;
    let slow_period: i32 = 26;
    let signal_period: i32 = 9;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| {
                macd(
                    black_box(data),
                    black_box(fast_period as usize),
                    black_box(slow_period as usize),
                    black_box(signal_period as usize),
                )
            });
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut macd_out = vec![0.0f64; data.len()];
                let mut signal_out = vec![0.0f64; data.len()];
                let mut hist_out = vec![0.0f64; data.len()];
                unsafe {
                    MACD(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        fast_period,
                        slow_period,
                        signal_period,
                        &mut out_begin,
                        &mut out_nb_element,
                        macd_out.as_mut_ptr(),
                        signal_out.as_mut_ptr(),
                        hist_out.as_mut_ptr(),
                    );
                }
                black_box((macd_out, signal_out, hist_out))
            });
        });
    }
    group.finish();
}

fn bench_mom(c: &mut Criterion) {
    let mut group = c.benchmark_group("mom");
    let period: i32 = 10;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        // Pre-allocate output buffer outside the benchmark loop
        let mut output = vec![0.0f64; size];
        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| {
                mom_into(black_box(data), black_box(period as usize), black_box(&mut output)).unwrap()
            });
        });

        let mut output = vec![0.0f64; size];
        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                unsafe {
                    MOM(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    )
                }
            });
        });
    }
    group.finish();
}

fn bench_roc(c: &mut Criterion) {
    let mut group = c.benchmark_group("roc");
    let period: i32 = 10;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| roc(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    ROC(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

fn bench_cmo(c: &mut Criterion) {
    let mut group = c.benchmark_group("cmo");

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        for &period in PERIODS {
            let period_i32 = period as i32;

            group.bench_with_input(
                BenchmarkId::new(format!("fast-ta/p{}", period), size),
                &data,
                |b, data| {
                    b.iter(|| cmo(black_box(data), black_box(period)));
                },
            );

            group.bench_with_input(
                BenchmarkId::new(format!("ta-lib/p{}", period), size),
                &data,
                |b, data| {
                    b.iter(|| {
                        let mut out_begin: i32 = 0;
                        let mut out_nb_element: i32 = 0;
                        let mut output = vec![0.0f64; data.len()];
                        unsafe {
                            CMO(
                                0,
                                (data.len() - 1) as i32,
                                data.as_ptr(),
                                period_i32,
                                &mut out_begin,
                                &mut out_nb_element,
                                output.as_mut_ptr(),
                            );
                        }
                        black_box(output)
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_apo(c: &mut Criterion) {
    let mut group = c.benchmark_group("apo");
    let fast_period: i32 = 12;
    let slow_period: i32 = 26;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| {
                apo(
                    black_box(data),
                    black_box(fast_period as usize),
                    black_box(slow_period as usize),
                )
            });
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    APO(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        fast_period,
                        slow_period,
                        MAType::MAType_EMA,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

fn bench_trix(c: &mut Criterion) {
    let mut group = c.benchmark_group("trix");
    let period: i32 = 15;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| trix(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    TRIX(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

// =============================================================================
// Trend Indicators (OHLC)
// =============================================================================

fn bench_adx(c: &mut Criterion) {
    let mut group = c.benchmark_group("adx");
    let period: i32 = 14;

    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    adx(
                        black_box(*h),
                        black_box(*l),
                        black_box(*c),
                        black_box(period as usize),
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    let mut output = vec![0.0f64; h.len()];
                    unsafe {
                        ADX(
                            0,
                            (h.len() - 1) as i32,
                            h.as_ptr(),
                            l.as_ptr(),
                            c.as_ptr(),
                            period,
                            &mut out_begin,
                            &mut out_nb_element,
                            output.as_mut_ptr(),
                        );
                    }
                    black_box(output)
                });
            },
        );
    }
    group.finish();
}

fn bench_dx(c: &mut Criterion) {
    let mut group = c.benchmark_group("dx");
    let period: i32 = 14;

    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    dx(
                        black_box(*h),
                        black_box(*l),
                        black_box(*c),
                        black_box(period as usize),
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    let mut output = vec![0.0f64; h.len()];
                    unsafe {
                        DX(
                            0,
                            (h.len() - 1) as i32,
                            h.as_ptr(),
                            l.as_ptr(),
                            c.as_ptr(),
                            period,
                            &mut out_begin,
                            &mut out_nb_element,
                            output.as_mut_ptr(),
                        );
                    }
                    black_box(output)
                });
            },
        );
    }
    group.finish();
}

fn bench_aroon(c: &mut Criterion) {
    let mut group = c.benchmark_group("aroon");

    for &size in SIZES {
        let (_, high, low, _, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        for &period in PERIODS {
            let period_i32 = period as i32;

            group.bench_with_input(
                BenchmarkId::new(format!("fast-ta/p{}", period), size),
                &(&high, &low),
                |b, (h, l)| {
                    b.iter(|| aroon(black_box(*h), black_box(*l), black_box(period)));
                },
            );

            group.bench_with_input(
                BenchmarkId::new(format!("ta-lib/p{}", period), size),
                &(&high, &low),
                |b, (h, l)| {
                    b.iter(|| {
                        let mut out_begin: i32 = 0;
                        let mut out_nb_element: i32 = 0;
                        let mut aroon_down = vec![0.0f64; h.len()];
                        let mut aroon_up = vec![0.0f64; h.len()];
                        unsafe {
                            AROON(
                                0,
                                (h.len() - 1) as i32,
                                h.as_ptr(),
                                l.as_ptr(),
                                period_i32,
                                &mut out_begin,
                                &mut out_nb_element,
                                aroon_down.as_mut_ptr(),
                                aroon_up.as_mut_ptr(),
                            );
                        }
                        black_box((aroon_down, aroon_up))
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_cci(c: &mut Criterion) {
    let mut group = c.benchmark_group("cci");

    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        for &period in PERIODS {
            let period_i32 = period as i32;

            group.bench_with_input(
                BenchmarkId::new(format!("fast-ta/p{}", period), size),
                &(&high, &low, &close),
                |b, (h, l, c)| {
                    b.iter(|| cci(black_box(*h), black_box(*l), black_box(*c), black_box(period)));
                },
            );

            group.bench_with_input(
                BenchmarkId::new(format!("ta-lib/p{}", period), size),
                &(&high, &low, &close),
                |b, (h, l, c)| {
                    b.iter(|| {
                        let mut out_begin: i32 = 0;
                        let mut out_nb_element: i32 = 0;
                        let mut output = vec![0.0f64; h.len()];
                        unsafe {
                            CCI(
                                0,
                                (h.len() - 1) as i32,
                                h.as_ptr(),
                                l.as_ptr(),
                                c.as_ptr(),
                                period_i32,
                                &mut out_begin,
                                &mut out_nb_element,
                                output.as_mut_ptr(),
                            );
                        }
                        black_box(output)
                    });
                },
            );
        }
    }
    group.finish();
}

// =============================================================================
// Volatility Indicators
// =============================================================================

fn bench_atr(c: &mut Criterion) {
    let mut group = c.benchmark_group("atr");
    let period: i32 = 14;

    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    atr(
                        black_box(*h),
                        black_box(*l),
                        black_box(*c),
                        black_box(period as usize),
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    let mut output = vec![0.0f64; h.len()];
                    unsafe {
                        ATR(
                            0,
                            (h.len() - 1) as i32,
                            h.as_ptr(),
                            l.as_ptr(),
                            c.as_ptr(),
                            period,
                            &mut out_begin,
                            &mut out_nb_element,
                            output.as_mut_ptr(),
                        );
                    }
                    black_box(output)
                });
            },
        );
    }
    group.finish();
}

fn bench_trange(c: &mut Criterion) {
    let mut group = c.benchmark_group("trange");

    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| true_range(black_box(*h), black_box(*l), black_box(*c)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    let mut output = vec![0.0f64; h.len()];
                    unsafe {
                        TRANGE(
                            0,
                            (h.len() - 1) as i32,
                            h.as_ptr(),
                            l.as_ptr(),
                            c.as_ptr(),
                            &mut out_begin,
                            &mut out_nb_element,
                            output.as_mut_ptr(),
                        );
                    }
                    black_box(output)
                });
            },
        );
    }
    group.finish();
}

fn bench_bollinger(c: &mut Criterion) {
    let mut group = c.benchmark_group("bollinger");
    let period: i32 = 20;
    let num_std: f64 = 2.0;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| {
                bollinger(
                    black_box(data),
                    black_box(period as usize),
                    black_box(num_std),
                )
            });
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut upper = vec![0.0f64; data.len()];
                let mut middle = vec![0.0f64; data.len()];
                let mut lower = vec![0.0f64; data.len()];
                unsafe {
                    BBANDS(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        num_std,
                        num_std,
                        MAType::MAType_SMA,
                        &mut out_begin,
                        &mut out_nb_element,
                        upper.as_mut_ptr(),
                        middle.as_mut_ptr(),
                        lower.as_mut_ptr(),
                    );
                }
                black_box((upper, middle, lower))
            });
        });
    }
    group.finish();
}

// =============================================================================
// Stochastic Indicators
// =============================================================================

fn bench_stochastic(c: &mut Criterion) {
    let mut group = c.benchmark_group("stochastic");
    let k_period: i32 = 14;
    let d_period: i32 = 3;
    let k_slowing: i32 = 3;

    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    stochastic(
                        black_box(*h),
                        black_box(*l),
                        black_box(*c),
                        black_box(k_period as usize),
                        black_box(d_period as usize),
                        black_box(k_slowing as usize),
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    let mut k_out = vec![0.0f64; h.len()];
                    let mut d_out = vec![0.0f64; h.len()];
                    unsafe {
                        STOCH(
                            0,
                            (h.len() - 1) as i32,
                            h.as_ptr(),
                            l.as_ptr(),
                            c.as_ptr(),
                            k_period,
                            k_slowing,
                            MAType::MAType_SMA,
                            d_period,
                            MAType::MAType_SMA,
                            &mut out_begin,
                            &mut out_nb_element,
                            k_out.as_mut_ptr(),
                            d_out.as_mut_ptr(),
                        );
                    }
                    black_box((k_out, d_out))
                });
            },
        );
    }
    group.finish();
}

fn bench_stochastic_fast(c: &mut Criterion) {
    let mut group = c.benchmark_group("stochastic_fast");
    let k_period: i32 = 14;
    let d_period: i32 = 3;

    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    stochastic_fast(
                        black_box(*h),
                        black_box(*l),
                        black_box(*c),
                        black_box(k_period as usize),
                        black_box(d_period as usize),
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    let mut k_out = vec![0.0f64; h.len()];
                    let mut d_out = vec![0.0f64; h.len()];
                    unsafe {
                        STOCHF(
                            0,
                            (h.len() - 1) as i32,
                            h.as_ptr(),
                            l.as_ptr(),
                            c.as_ptr(),
                            k_period,
                            d_period,
                            MAType::MAType_SMA,
                            &mut out_begin,
                            &mut out_nb_element,
                            k_out.as_mut_ptr(),
                            d_out.as_mut_ptr(),
                        );
                    }
                    black_box((k_out, d_out))
                });
            },
        );
    }
    group.finish();
}

fn bench_williams_r(c: &mut Criterion) {
    let mut group = c.benchmark_group("williams_r");
    let period: i32 = 14;

    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    williams_r(
                        black_box(*h),
                        black_box(*l),
                        black_box(*c),
                        black_box(period as usize),
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    let mut output = vec![0.0f64; h.len()];
                    unsafe {
                        WILLR(
                            0,
                            (h.len() - 1) as i32,
                            h.as_ptr(),
                            l.as_ptr(),
                            c.as_ptr(),
                            period,
                            &mut out_begin,
                            &mut out_nb_element,
                            output.as_mut_ptr(),
                        );
                    }
                    black_box(output)
                });
            },
        );
    }
    group.finish();
}

fn bench_ultosc(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultosc");
    let period1: i32 = 7;
    let period2: i32 = 14;
    let period3: i32 = 28;

    for &size in SIZES {
        let (_, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    ultosc(
                        black_box(*h),
                        black_box(*l),
                        black_box(*c),
                        black_box(period1 as usize),
                        black_box(period2 as usize),
                        black_box(period3 as usize),
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&high, &low, &close),
            |b, (h, l, c)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    let mut output = vec![0.0f64; h.len()];
                    unsafe {
                        ULTOSC(
                            0,
                            (h.len() - 1) as i32,
                            h.as_ptr(),
                            l.as_ptr(),
                            c.as_ptr(),
                            period1,
                            period2,
                            period3,
                            &mut out_begin,
                            &mut out_nb_element,
                            output.as_mut_ptr(),
                        );
                    }
                    black_box(output)
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// Volume Indicators
// =============================================================================

fn bench_obv(c: &mut Criterion) {
    let mut group = c.benchmark_group("obv");

    for &size in SIZES {
        let (_, _, _, close, volume) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        // Pre-allocate output buffer outside the benchmark loop
        let mut output = vec![0.0f64; size];
        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&close, &volume),
            |b, (c, v)| {
                b.iter(|| {
                    obv_into(black_box(*c), black_box(*v), black_box(&mut output)).unwrap()
                });
            },
        );

        let mut output = vec![0.0f64; size];
        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&close, &volume),
            |b, (c, v)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    unsafe {
                        OBV(
                            0,
                            (c.len() - 1) as i32,
                            c.as_ptr(),
                            v.as_ptr(),
                            &mut out_begin,
                            &mut out_nb_element,
                            output.as_mut_ptr(),
                        )
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_ad(c: &mut Criterion) {
    let mut group = c.benchmark_group("ad");

    for &size in SIZES {
        let (_, high, low, close, volume) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&high, &low, &close, &volume),
            |b, (h, l, c, v)| {
                b.iter(|| ad(black_box(*h), black_box(*l), black_box(*c), black_box(*v)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&high, &low, &close, &volume),
            |b, (h, l, c, v)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    let mut output = vec![0.0f64; h.len()];
                    unsafe {
                        AD(
                            0,
                            (h.len() - 1) as i32,
                            h.as_ptr(),
                            l.as_ptr(),
                            c.as_ptr(),
                            v.as_ptr(),
                            &mut out_begin,
                            &mut out_nb_element,
                            output.as_mut_ptr(),
                        );
                    }
                    black_box(output)
                });
            },
        );
    }
    group.finish();
}

fn bench_mfi(c: &mut Criterion) {
    let mut group = c.benchmark_group("mfi");

    for &size in SIZES {
        let (_, high, low, close, volume) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        for &period in PERIODS {
            let period_i32 = period as i32;

            group.bench_with_input(
                BenchmarkId::new(format!("fast-ta/p{}", period), size),
                &(&high, &low, &close, &volume),
                |b, (h, l, c, v)| {
                    b.iter(|| {
                        mfi(
                            black_box(*h),
                            black_box(*l),
                            black_box(*c),
                            black_box(*v),
                            black_box(period),
                        )
                    });
                },
            );

            group.bench_with_input(
                BenchmarkId::new(format!("ta-lib/p{}", period), size),
                &(&high, &low, &close, &volume),
                |b, (h, l, c, v)| {
                    b.iter(|| {
                        let mut out_begin: i32 = 0;
                        let mut out_nb_element: i32 = 0;
                        let mut output = vec![0.0f64; h.len()];
                        unsafe {
                            MFI(
                                0,
                                (h.len() - 1) as i32,
                                h.as_ptr(),
                                l.as_ptr(),
                                c.as_ptr(),
                                v.as_ptr(),
                                period_i32,
                                &mut out_begin,
                                &mut out_nb_element,
                                output.as_mut_ptr(),
                            );
                        }
                        black_box(output)
                    });
                },
            );
        }
    }
    group.finish();
}

// =============================================================================
// Statistics
// =============================================================================

fn bench_var(c: &mut Criterion) {
    let mut group = c.benchmark_group("var");

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        for &period in PERIODS {
            let period_i32 = period as i32;

            group.bench_with_input(
                BenchmarkId::new(format!("fast-ta/p{}", period), size),
                &data,
                |b, data| {
                    b.iter(|| var(black_box(data), black_box(period)));
                },
            );

            group.bench_with_input(
                BenchmarkId::new(format!("ta-lib/p{}", period), size),
                &data,
                |b, data| {
                    b.iter(|| {
                        let mut out_begin: i32 = 0;
                        let mut out_nb_element: i32 = 0;
                        let mut output = vec![0.0f64; data.len()];
                        unsafe {
                            VAR(
                                0,
                                (data.len() - 1) as i32,
                                data.as_ptr(),
                                period_i32,
                                1.0,
                                &mut out_begin,
                                &mut out_nb_element,
                                output.as_mut_ptr(),
                            );
                        }
                        black_box(output)
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_linearreg(c: &mut Criterion) {
    let mut group = c.benchmark_group("linearreg");
    let period: i32 = 14;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| linearreg(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    LINEARREG(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

fn bench_tsf(c: &mut Criterion) {
    let mut group = c.benchmark_group("tsf");
    let period: i32 = 14;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| tsf(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    TSF(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

// =============================================================================
// Other Indicators
// =============================================================================

fn bench_midpoint(c: &mut Criterion) {
    let mut group = c.benchmark_group("midpoint");
    let period: i32 = 14;

    for &size in SIZES {
        let data = generate_close_prices(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("fast-ta", size), &data, |b, data| {
            b.iter(|| midpoint(black_box(data), black_box(period as usize)));
        });

        group.bench_with_input(BenchmarkId::new("ta-lib", size), &data, |b, data| {
            b.iter(|| {
                let mut out_begin: i32 = 0;
                let mut out_nb_element: i32 = 0;
                let mut output = vec![0.0f64; data.len()];
                unsafe {
                    MIDPOINT(
                        0,
                        (data.len() - 1) as i32,
                        data.as_ptr(),
                        period,
                        &mut out_begin,
                        &mut out_nb_element,
                        output.as_mut_ptr(),
                    );
                }
                black_box(output)
            });
        });
    }
    group.finish();
}

fn bench_midprice(c: &mut Criterion) {
    let mut group = c.benchmark_group("midprice");
    let period: i32 = 14;

    for &size in SIZES {
        let (_, high, low, _, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&high, &low),
            |b, (h, l)| {
                b.iter(|| midprice(black_box(*h), black_box(*l), black_box(period as usize)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&high, &low),
            |b, (h, l)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    let mut output = vec![0.0f64; h.len()];
                    unsafe {
                        MIDPRICE(
                            0,
                            (h.len() - 1) as i32,
                            h.as_ptr(),
                            l.as_ptr(),
                            period,
                            &mut out_begin,
                            &mut out_nb_element,
                            output.as_mut_ptr(),
                        );
                    }
                    black_box(output)
                });
            },
        );
    }
    group.finish();
}

fn bench_bop(c: &mut Criterion) {
    let mut group = c.benchmark_group("bop");

    for &size in SIZES {
        let (open, high, low, close, _) = generate_ohlcv(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast-ta", size),
            &(&open, &high, &low, &close),
            |b, (o, h, l, c)| {
                b.iter(|| bop(black_box(*o), black_box(*h), black_box(*l), black_box(*c)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ta-lib", size),
            &(&open, &high, &low, &close),
            |b, (o, h, l, c)| {
                b.iter(|| {
                    let mut out_begin: i32 = 0;
                    let mut out_nb_element: i32 = 0;
                    let mut output = vec![0.0f64; h.len()];
                    unsafe {
                        BOP(
                            0,
                            (h.len() - 1) as i32,
                            o.as_ptr(),
                            h.as_ptr(),
                            l.as_ptr(),
                            c.as_ptr(),
                            &mut out_begin,
                            &mut out_nb_element,
                            output.as_mut_ptr(),
                        );
                    }
                    black_box(output)
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// Benchmark Groups with Custom Configuration
// =============================================================================

// Standard moving averages (excluding slow T3)
criterion_group!(
    moving_averages,
    bench_sma,
    bench_ema,
    bench_wma,
    bench_dema,
    bench_tema,
    bench_trima,
    bench_kama,
);

// T3 is slow - needs extended measurement time
criterion_group! {
    name = moving_averages_slow;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(15))
        .sample_size(500);
    targets = bench_t3
}

// Standard momentum indicators (excluding slow CMO)
criterion_group!(
    momentum,
    bench_rsi,
    bench_macd,
    bench_mom,
    bench_roc,
    bench_apo,
    bench_trix,
);

// CMO is slow - needs extended measurement time
criterion_group! {
    name = momentum_slow;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(15))
        .sample_size(500);
    targets = bench_cmo
}

// Standard trend indicators (excluding slow CCI)
criterion_group!(trend, bench_adx, bench_dx, bench_aroon,);

// CCI is slow - needs extended measurement time
criterion_group! {
    name = trend_slow;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(15))
        .sample_size(500);
    targets = bench_cci
}

criterion_group!(
    volatility,
    bench_atr,
    bench_trange,
    bench_bollinger,
);

// Stochastic indicators are slow - need extended measurement time
criterion_group! {
    name = stochastic_group;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(15))
        .sample_size(500);
    targets = bench_stochastic, bench_stochastic_fast, bench_williams_r, bench_ultosc
}

// Standard volume indicators (excluding slow MFI)
criterion_group!(volume, bench_obv, bench_ad,);

// MFI is slow - needs extended measurement time
criterion_group! {
    name = volume_slow;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(15))
        .sample_size(500);
    targets = bench_mfi
}

// Standard statistics (excluding slow VAR)
criterion_group!(statistics, bench_linearreg, bench_tsf,);

// VAR is slow - needs extended measurement time
criterion_group! {
    name = statistics_slow;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(15))
        .sample_size(500);
    targets = bench_var
}

criterion_group!(other, bench_midpoint, bench_midprice, bench_bop,);

criterion_main!(
    moving_averages,
    moving_averages_slow,
    momentum,
    momentum_slow,
    trend,
    trend_slow,
    volatility,
    stochastic_group,
    volume,
    volume_slow,
    statistics,
    statistics_slow,
    other,
);
