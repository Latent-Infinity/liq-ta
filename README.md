# fast-ta

High-performance technical analysis library for Rust.

[![Crates.io](https://img.shields.io/crates/v/fast-ta.svg)](https://crates.io/crates/fast-ta)
[![Documentation](https://docs.rs/fast-ta/badge.svg)](https://docs.rs/fast-ta)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Features

- **Fast**: O(n) algorithms with optimized memory access patterns
- **Accurate**: Validated against spec fixtures with documented edge-case behavior
- **Ergonomic**: Simple API with sensible defaults, plus `_into` variants for zero-allocation use
- **Safe**: Comprehensive error handling with actionable messages
- **Generic**: Works with both `f32` and `f64` data types

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
fast-ta = "0.1"
```

Calculate a simple moving average:

```rust
use fast_ta::prelude::*;

let prices = vec![44.0, 44.5, 43.5, 44.0, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0];
let sma = sma(&prices, 5).unwrap();

// First 4 values are NaN (lookback period)
assert!(sma[3].is_nan());
// SMA starts at index 4
assert!((sma[4] - 44.1).abs() < 0.01);
```

## Supported Indicators

### Moving Averages
| Function | Description | Default Period |
|----------|-------------|----------------|
| `sma()` | Simple Moving Average | 20 |
| `ema()` | Exponential Moving Average | 20 |
| `ema_wilder()` | Wilder's Smoothing (alpha = 1/n) | 14 |

### Momentum
| Function | Description | Default Parameters |
|----------|-------------|-------------------|
| `rsi()` | Relative Strength Index | period=14 |
| `macd()` | Moving Average Convergence Divergence | fast=12, slow=26, signal=9 |
| `stochastic_fast()` | Fast Stochastic Oscillator | k=14, d=3 |
| `stochastic_slow()` | Slow Stochastic Oscillator | k=14, d=3, slowing=3 |
| `williams_r()` | Williams %R | period=14 |
| `adx()` | Average Directional Index | period=14 |

### Volatility
| Function | Description | Default Parameters |
|----------|-------------|-------------------|
| `atr()` | Average True Range | period=14 |
| `bollinger()` | Bollinger Bands | period=20, std_dev=2.0 |
| `donchian()` | Donchian Channels | period=20 |

### Volume
| Function | Description |
|----------|-------------|
| `obv()` | On-Balance Volume |
| `vwap()` | Volume Weighted Average Price |

## API Patterns

### Simple API

The simplest way to use indicators:

```rust
use fast_ta::prelude::*;

let prices = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

// Single output
let sma_result = sma(&prices, 3)?;
let ema_result = ema(&prices, 3)?;
let rsi_result = rsi(&prices, 5)?;

// Multi-output (returns struct)
let macd_output = macd(&prices, 12, 26, 9)?;
println!("MACD: {:?}", macd_output.macd);
println!("Signal: {:?}", macd_output.signal);
println!("Histogram: {:?}", macd_output.histogram);
```

### Buffer API (`_into` variants)

For high-performance scenarios with pre-allocated buffers:

```rust
use fast_ta::prelude::*;

let prices = vec![1.0; 10000];
let mut output = vec![0.0; 10000];

// Reuse the same buffer for multiple computations
sma_into(&prices, 20, &mut output)?;
// ... process output ...

ema_into(&prices, 20, &mut output)?;
// ... process output ...
```

### Configuration Types

For indicators with many parameters:

```rust
use fast_ta::prelude::*;

let prices = vec![1.0; 100];

// Using defaults
let result = Macd::default().compute(&prices)?;

// Custom parameters with fluent API
let result = Macd::new()
    .fast_period(10)
    .slow_period(21)
    .signal_period(7)
    .compute(&prices)?;

// Bollinger with custom std dev
let bands = Bollinger::new()
    .period(20)
    .std_dev(2.5)
    .compute(&prices)?;
```

## OHLC Indicators

Some indicators require OHLC (Open, High, Low, Close) data:

```rust
use fast_ta::prelude::*;

let high = vec![45.5, 46.0, 46.5, 47.0, 47.5];
let low = vec![44.0, 44.5, 45.0, 45.5, 46.0];
let close = vec![45.0, 45.5, 46.0, 46.5, 47.0];
let volume = vec![1000.0, 1100.0, 900.0, 1200.0, 1000.0];

// ATR (requires high, low, close)
let atr_result = atr(&high, &low, &close, 3)?;

// Stochastic (requires high, low, close)
let stoch = stochastic_slow(&high, &low, &close, 3, 2, 2)?;
println!("K%: {:?}", stoch.k);
println!("D%: {:?}", stoch.d);

// OBV (requires close, volume)
let obv_result = obv(&close, &volume)?;

// VWAP (requires high, low, close, volume)
let vwap_result = vwap(&high, &low, &close, &volume)?;
```

## Lookback Functions

Every indicator has corresponding `*_lookback()` and `*_min_len()` functions:

```rust
use fast_ta::prelude::*;

// How many NaN values at the start of output?
assert_eq!(sma_lookback(20), 19);
assert_eq!(ema_lookback(20), 19);
assert_eq!(rsi_lookback(14), 14);
assert_eq!(macd_signal_lookback(12, 26, 9), 33);

// Minimum input length to avoid InsufficientData error?
assert_eq!(sma_min_len(20), 20);
assert_eq!(rsi_min_len(14), 15);
assert_eq!(macd_min_len(12, 26, 9), 34);
```

## Error Handling

All functions return `Result<T, Error>` with actionable error messages:

```rust
use fast_ta::prelude::*;

// Empty input
let result = sma(&[], 20);
assert!(matches!(result, Err(Error::EmptyInput)));

// Period too large
let result = sma(&[1.0, 2.0, 3.0], 100);
assert!(matches!(result, Err(Error::InsufficientData { .. })));

// Invalid period
let result = sma(&[1.0, 2.0, 3.0], 0);
assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
```

## CLI Tool

A command-line interface is available for quick computations:

```bash
# Install
cargo install fast-ta-cli

# Calculate SMA
fast-ta sma prices.csv 20

# Calculate MACD with output file
fast-ta macd prices.csv 12,26,9 -o macd_output.csv

# Calculate Bollinger Bands
fast-ta bollinger prices.csv 20,2.0

# Get help
fast-ta --help
```

See [fast-ta-cli README](crates/fast-ta-cli/README.md) for full CLI documentation.

## Performance

All indicators have O(n) time complexity with high throughput:

| Indicator | Throughput (100K elements) |
|-----------|---------------------------|
| EMA | ~710 Melem/s |
| VWAP | ~562 Melem/s |
| SMA | ~420 Melem/s |
| OBV | ~386 Melem/s |
| Bollinger | ~386 Melem/s |
| ATR | ~269 Melem/s |
| RSI | ~265 Melem/s |
| MACD | ~256 Melem/s |
| ADX | ~240 Melem/s |
| Donchian | ~150 Melem/s |
| Williams %R | ~147 Melem/s |
| Stochastic | ~78 Melem/s |

### Running Benchmarks

For reliable, variance-controlled benchmarks:

```bash
# Hybrid approach with multi-round execution (recommended)
./scripts/run_benchmarks.sh

# Traditional single-round benchmarks
cargo bench --bench talib_comparison
```

See [docs/benchmarking-guide.md](docs/benchmarking-guide.md) for comprehensive benchmarking methodology and [docs/benchmark-baseline.md](docs/benchmark-baseline.md) for detailed baseline results.

## Numeric Behavior

fast-ta follows strict numeric policies for correctness and performance:

- **IEEE 754 NaN propagation**: Leverages automatic NaN propagation where possible for optimal performance
- **Infinity handling**: Both NaN and ±Infinity propagate through all indicators
- **Full-length output**: Output length equals input length (NaN prefix for lookback)
- **Deterministic**: Same inputs always produce identical outputs
- **Precision modes**: Optional f64 accumulators for f32 inputs (High precision mode)

Edge case handling:
- RSI: all gains → 100, all losses → 0, both zero → 50
- Stochastic: high == low → %K = 50
- Bollinger: constant series → bands collapse to mean
- Division by zero: Returns 0 or NaN depending on context

See [docs/indicator-standards.md](docs/indicator-standards.md) for complete numeric policy details.

## License

Licensed under the MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT).
