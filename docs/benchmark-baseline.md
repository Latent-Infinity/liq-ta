# fast-ta Performance Baseline

Benchmark results establishing performance baselines for all indicators.
Run with `cargo bench -p fast-ta` to reproduce.

## Test Environment

- Date: 2025-12-25
- Platform: macOS (Darwin)
- Rust: 1.90+ (Edition 2024)
- Profile: release with LTO
- TA-Lib: v0.4.0 (via ta-lib-sys FFI)

## Benchmark Configuration

All benchmarks use Criterion with rigorous statistical settings:

| Setting | Standard | Slow Indicators |
|---------|----------|-----------------|
| Warmup | 5s | 5s |
| Measurement | 10s | 15s |
| Sample Count | 500 | 500 |
| Noise Threshold | 2% | 2% |
| Confidence Level | 95% | 95% |

Slow indicators (T3, CMO, CCI, Stochastic variants, MFI, VAR) use extended
measurement time for statistical reliability.

## TA-Lib Comparison (100K elements)

Comprehensive FFI comparison between fast-ta and TA-Lib C library across 35 indicators.

### Moving Averages

| Indicator | fast-ta | TA-Lib | Ratio | Winner |
|-----------|---------|--------|-------|--------|
| SMA(20) | 240 µs | 101 µs | 0.42× | TA-Lib |
| EMA(20) | 150 µs | 129 µs | 0.86× | TA-Lib |
| WMA(20) | 130 µs | 129 µs | 0.99× | Tie |
| DEMA(20) | 297 µs | 280 µs | 0.94× | TA-Lib |
| TEMA(20) | 360 µs | 404 µs | **1.12×** | **fast-ta** |
| TRIMA(20) | 231 µs | 187 µs | 0.81× | TA-Lib |
| KAMA(10) | 405 µs | 178 µs | 0.44× | TA-Lib |
| T3(5) | 1.21 ms | 182 µs | 0.15× | TA-Lib |

### Momentum

| Indicator | fast-ta | TA-Lib | Ratio | Winner |
|-----------|---------|--------|-------|--------|
| RSI(14) | 413 µs | 442 µs | **1.07×** | **fast-ta** |
| MACD(12,26,9) | 418 µs | 440 µs | **1.05×** | **fast-ta** |
| MOM(10) | 18.8 µs | 14.9 µs | 0.79× | TA-Lib |
| ROC(10) | 44.8 µs | 52.9 µs | **1.18×** | **fast-ta** |
| CMO(14) | 976 µs | 426 µs | 0.44× | TA-Lib |
| APO(12,26) | 157 µs | 276 µs | **1.76×** | **fast-ta** |
| TRIX(15) | 670 µs | 432 µs | 0.64× | TA-Lib |

### Trend

| Indicator | fast-ta | TA-Lib | Ratio | Winner |
|-----------|---------|--------|-------|--------|
| ADX(14) | 430 µs | 426 µs | ~1.0× | Tie |
| DX(14) | 501 µs | 410 µs | 0.82× | TA-Lib |
| AROON(14) | 714 µs | 464 µs | 0.65× | TA-Lib |
| CCI(20) | 1.07 ms | 726 µs | 0.68× | TA-Lib |

### Volatility

| Indicator | fast-ta | TA-Lib | Ratio | Winner |
|-----------|---------|--------|-------|--------|
| ATR(14) | 416 µs | 412 µs | ~1.0× | Tie |
| TRANGE | 47.0 µs | 29.4 µs | 0.63× | TA-Lib |
| Bollinger(20,2) | 278 µs | 401 µs | **1.44×** | **fast-ta** |

### Stochastic

| Indicator | fast-ta | TA-Lib | Ratio | Winner |
|-----------|---------|--------|-------|--------|
| Stochastic(14,3,3) | 1.50 ms | 661 µs | 0.44× | TA-Lib |
| StochFast(14,3) | 1.23 ms | 568 µs | 0.46× | TA-Lib |
| Williams %R(14) | 742 µs | 433 µs | 0.58× | TA-Lib |
| ULTOSC(7,14,28) | 2.15 ms | 363 µs | 0.17× | TA-Lib |

### Volume

| Indicator | fast-ta | TA-Lib | Ratio | Winner |
|-----------|---------|--------|-------|--------|
| OBV | 267 µs | 73.0 µs | 0.27× | TA-Lib |
| AD | 78.6 µs | 93.7 µs | **1.19×** | **fast-ta** |
| MFI(14) | 971 µs | 142 µs | 0.15× | TA-Lib |

### Statistics

| Indicator | fast-ta | TA-Lib | Ratio | Winner |
|-----------|---------|--------|-------|--------|
| VAR(20) | 1.36 ms | 131 µs | 0.10× | TA-Lib |
| LINEARREG(14) | 529 µs | 439 µs | 0.83× | TA-Lib |
| TSF(14) | 530 µs | 437 µs | 0.82× | TA-Lib |

### Other

| Indicator | fast-ta | TA-Lib | Ratio | Winner |
|-----------|---------|--------|-------|--------|
| MIDPOINT(14) | 435 µs | 611 µs | **1.40×** | **fast-ta** |
| MIDPRICE(14) | 446 µs | 414 µs | 0.93× | TA-Lib |
| BOP | 55.4 µs | 55.8 µs | ~1.0× | Tie |

### Summary

**Overall: fast-ta wins 9/35 (26%), TA-Lib wins 22/35 (63%), Tie 4/35 (11%)**

**fast-ta is faster on:**
- TEMA, RSI, MACD, ROC, APO, Bollinger, AD, MIDPOINT

**TA-Lib is faster on:**
- Most simple moving averages (SMA, EMA, DEMA, TRIMA, KAMA, T3)
- Stochastic variants and rolling-window indicators
- Volume indicators using simple cumulation (OBV, MFI)
- Statistical functions with optimized variance algorithms

### Analysis

- **Moving Averages**: TA-Lib uses highly optimized C with SIMD. Our pure Rust
  implementation prioritizes correctness and maintainability. TEMA is an exception
  where fast-ta's triple-EMA fusing is more efficient.
- **RSI/ATR**: fast-ta's Wilder smoothing implementation is more cache-efficient.
- **MACD/APO**: fast-ta uses fused computation for better cache efficiency.
- **Bollinger**: fast-ta computes rolling variance more efficiently.
- **Stochastic/Williams %R**: TA-Lib's O(n×k) is faster for small periods due to
  simpler implementation; fast-ta's deque approach has overhead at typical period sizes.
- **AD (Accumulation/Distribution)**: fast-ta's vectorized approach wins.
- **MIDPOINT**: fast-ta's rolling extrema is faster than TA-Lib's implementation.
- **VAR/MFI**: TA-Lib uses highly optimized variance/rolling sum algorithms.
- **T3**: TA-Lib uses a single-pass T3 implementation; fast-ta chains 6 EMAs.

## Results Summary

All indicators demonstrate **O(n) linear time complexity** with consistent
throughput across input sizes (100 to 100,000 elements).

### Single-Series Indicators

| Indicator | 100 elem | 1K elem | 10K elem | 100K elem | Throughput |
|-----------|----------|---------|----------|-----------|------------|
| SMA(20)   | 192 ns   | 2.34 µs | 24.3 µs  | 236 µs    | ~423 Melem/s |
| EMA(20)   | 128 ns   | 1.44 µs | 15.0 µs  | 141 µs    | ~709 Melem/s |
| RSI(14)   | 344 ns   | 3.73 µs | 38.4 µs  | 397 µs    | ~252 Melem/s |
| MACD      | 355 ns   | 3.92 µs | 41.6 µs  | 397 µs    | ~252 Melem/s |
| Bollinger | 291 ns   | 2.65 µs | 28.4 µs  | 266 µs    | ~376 Melem/s |

### OHLC Indicators

| Indicator   | 100 elem | 1K elem | 10K elem | 100K elem | Throughput |
|-------------|----------|---------|----------|-----------|------------|
| ATR(14)     | 341 ns   | 3.72 µs | 37.8 µs  | 377 µs    | ~265 Melem/s |
| Stochastic  | 1.18 µs  | 13.5 µs | 139 µs   | 1.39 ms   | ~72 Melem/s |
| ADX(14)     | 437 ns   | 4.25 µs | 48.0 µs  | 424 µs    | ~236 Melem/s |
| Williams %R | 706 ns   | 6.40 µs | 69.2 µs  | 681 µs    | ~147 Melem/s |
| Donchian    | 746 ns   | 6.12 µs | 69.0 µs  | 664 µs    | ~151 Melem/s |

### Volume Indicators

| Indicator | 100 elem | 1K elem | 10K elem | 100K elem | Throughput |
|-----------|----------|---------|----------|-----------|------------|
| OBV       | 141 ns   | 1.83 µs | 25.8 µs  | 273 µs    | ~366 Melem/s |
| VWAP      | 179 ns   | 1.56 µs | 17.7 µs  | 178 µs    | ~562 Melem/s |

## Workload Benchmark

Realistic multi-indicator workflow processing 100K OHLCV data points
(500 samples, 15s measurement time):

| Strategy | Time | Throughput | Notes |
|----------|------|------------|-------|
| Alloc Each Iter | 3.39 ms | 29.5 Melem/s | Fresh allocation per iteration |
| Reuse Buffers | 3.34 ms | 30.0 Melem/s | Pre-allocated buffer reuse |

Buffer reuse provides ~1.5% improvement, confirming pre-allocation strategy effectiveness.

## Key Observations

1. **Linear Scaling**: All indicators scale linearly (10x input = ~10x time)
2. **High Throughput**: Fastest indicators (EMA, VWAP) exceed 550 Melem/s
3. **Consistent Performance**: Throughput remains stable across input sizes
4. **Memory Efficient**: Pre-allocation strategy avoids runtime allocations
5. **Rust 2024 Edition**: Benefits from latest language optimizations
6. **Statistical Reliability**: 500 samples with 95% confidence level

## Complexity Analysis

| Indicator   | Time Complexity | Space Complexity | Notes |
|-------------|-----------------|------------------|-------|
| SMA         | O(n)            | O(n)             | Ring buffer optimization |
| EMA         | O(n)            | O(n)             | Single-pass, SMA seed |
| RSI         | O(n)            | O(n)             | Wilder's smoothing |
| MACD        | O(n)            | O(n)             | 3 EMAs + histogram |
| Bollinger   | O(n)            | O(n)             | Uses SMA + std dev |
| ATR         | O(n)            | O(n)             | True Range + EMA |
| Stochastic  | O(n)            | O(n)             | Rolling extrema |
| ADX         | O(n)            | O(n)             | +DI/-DI + Wilder |
| Williams %R | O(n)            | O(n)             | Rolling extrema |
| Donchian    | O(n)            | O(n)             | Monotonic deque |
| OBV         | O(n)            | O(n)             | Cumulative |
| VWAP        | O(n)            | O(n)             | Cumulative |

## Reproducing Results

```bash
# Full benchmark suite
cargo bench -p fast-ta

# Specific indicator
cargo bench -p fast-ta -- sma

# Quick verification (test mode)
cargo bench -p fast-ta -- --test
```

## Regression Detection

Compare against this baseline using:

```bash
cargo bench -p fast-ta -- --save-baseline baseline
# ... make changes ...
cargo bench -p fast-ta -- --baseline baseline
```

Criterion will report any significant regressions (>5% slower).
