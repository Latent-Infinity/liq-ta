# Baseline Benchmarks for Rust Standards Audit

This document captures baseline performance metrics for the fast-ta library before the standards compliance audit.
These baselines are used to verify that audit refactoring does not introduce performance regressions.

## Benchmark Configuration

**Criterion Settings** (from `crates/fast-ta/criterion.toml`):
- Warmup: 5 seconds
- Measurement: 10 seconds (15 seconds for slow benchmarks)
- Samples: 500
- Noise threshold: 2%
- Confidence level: 95%

**Test Sizes**: 100, 1,000, 10,000, 100,000 elements

## Benchmark Suites

### 1. Core Indicators (`crates/fast-ta/benches/indicators.rs`)

Measures throughput for individual indicators across various input sizes.

**Indicators Tested**:
- Moving Averages: SMA, EMA
- Oscillators: RSI, MACD, Stochastic
- Volatility: ATR, Bollinger Bands
- Trend: ADX, Williams %R, Donchian
- Volume: OBV, VWAP

### 2. TA-Lib Comparison (`crates/fast-ta/benches/talib_comparison.rs`)

Compares fast-ta performance against TA-Lib reference implementation.

**Categories Tested**:
- Moving Averages: SMA, EMA, WMA, DEMA, TEMA, TRIMA, KAMA, T3
- Momentum: RSI, MACD, MOM, ROC, CMO, APO, TRIX
- Trend: ADX, DX, Aroon, CCI
- Volatility: ATR, TRANGE, Bollinger
- Stochastic: STOCH, STOCHF, Williams %R, ULTOSC
- Volume: OBV, AD, MFI
- Statistics: VAR, LINEARREG, TSF
- Other: MIDPOINT, MIDPRICE, BOP

### 3. Workload Simulation (`crates/fast-ta/benches/workload.rs`)

Simulates realistic backtesting workload with multiple indicators.

**Scenarios**:
- `alloc_each_iter`: Allocates new buffers each iteration
- `reuse_buffers`: Reuses pre-allocated buffers (Buffer API)

## Running Benchmarks

### Prerequisites

```bash
# Ensure nightly toolchain is installed (required for portable_simd)
rustup install nightly

# Verify nightly is available
rustup run nightly cargo --version
```

### Capture Baseline

Run this command to capture the audit baseline:

```bash
cargo +nightly bench -p fast-ta -- --save-baseline audit-baseline
```

This saves baseline data to `target/criterion/<benchmark>/audit-baseline/`.

### Compare Against Baseline (After Audit)

Run this command after making changes to compare performance:

```bash
cargo +nightly bench -p fast-ta -- --baseline audit-baseline
```

Criterion will report:
- `No change` - Performance within noise threshold
- `Performance has improved` - Faster than baseline
- `Performance has regressed` - Slower than baseline (REQUIRES INVESTIGATION)

## Baseline Results

**Date**: [TO BE FILLED WHEN BENCHMARKS RUN]
**Rust Version**: [TO BE FILLED]
**Nightly Version**: [TO BE FILLED]
**Hardware**: [TO BE FILLED]

### Core Indicators Benchmark Results

| Indicator | 100 | 1,000 | 10,000 | 100,000 | Throughput (100K) |
|-----------|-----|-------|--------|---------|-------------------|
| SMA | - | - | - | - | - elem/s |
| EMA | - | - | - | - | - elem/s |
| RSI | - | - | - | - | - elem/s |
| MACD | - | - | - | - | - elem/s |
| Bollinger | - | - | - | - | - elem/s |
| ATR | - | - | - | - | - elem/s |
| Stochastic | - | - | - | - | - elem/s |
| ADX | - | - | - | - | - elem/s |
| Williams %R | - | - | - | - | - elem/s |
| Donchian | - | - | - | - | - elem/s |
| OBV | - | - | - | - | - elem/s |
| VWAP | - | - | - | - | - elem/s |

### TA-Lib Comparison Results (100K elements)

| Indicator | fast-ta | TA-Lib | Speedup |
|-----------|---------|--------|---------|
| SMA | - | - | -x |
| EMA | - | - | -x |
| RSI | - | - | -x |
| MACD | - | - | -x |
| Bollinger | - | - | -x |
| ATR | - | - | -x |
| ADX | - | - | -x |
| Stochastic | - | - | -x |

### Workload Simulation Results (100K elements)

| Scenario | Time | Throughput |
|----------|------|------------|
| alloc_each_iter | - | - elem/s |
| reuse_buffers | - | - elem/s |
| Buffer reuse speedup | - | -x |

## Acceptance Criteria

Per the audit specification, performance changes are acceptable if:

1. **No regression > 5%** compared to baseline
2. Measurement noise is expected at ~2% (criterion default)
3. Regressions in the 2-5% range require documentation/justification
4. Regressions > 5% are **blocking** and must be fixed

## Notes

- Benchmarks require `cargo +nightly` due to `portable_simd` feature
- Run benchmarks on a quiet system (minimal background processes)
- Run multiple times if results seem noisy
- Hot path indicators (SMA, EMA, RSI, MACD) are most critical
- Stochastic indicators have extended measurement time (15s) for reliability

## Instructions for Manual Execution

Since `cargo` commands may be restricted in the automation environment, run these manually:

```bash
# 1. Navigate to project root
cd /path/to/fast-ta

# 2. Save baseline before making any changes
cargo +nightly bench -p fast-ta -- --save-baseline audit-baseline

# 3. Copy benchmark output to this file
# Look for output like:
#   sma/100000           time:   [X.XXX µs Y.YYY µs Z.ZZZ µs]
#                        thrpt:  [X.XXX Melem/s Y.YYY Melem/s Z.ZZZ Melem/s]

# 4. After audit changes, compare to baseline
cargo +nightly bench -p fast-ta -- --baseline audit-baseline

# 5. Document any regressions in AUDIT_REPORT.md
```

---
*This document is part of the Rust Standards Compliance Audit (Issue #011)*
