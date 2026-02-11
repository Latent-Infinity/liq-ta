# Final Benchmark Results - Rust Standards Compliance Audit

This document captures the final benchmark comparison after all audit changes have been applied.
These results validate that no performance regressions were introduced during the audit.

## Benchmark Execution

### Prerequisites

```bash
# Ensure nightly toolchain is installed (required for portable_simd)
rustup install nightly

# Verify baseline was saved (from Phase 1)
ls target/criterion/*/audit-baseline/
```

### Run Final Comparison

Execute this command to compare current performance against the audit baseline:

```bash
cargo +nightly bench -p liq-ta -- --baseline audit-baseline
```

This will run all benchmark suites and compare against the baseline saved at audit start.

## Benchmark Suites

### 1. Core Indicators (`crates/liq-ta/benches/indicators.rs`)

**Benchmarks**: SMA, EMA, RSI, MACD, Bollinger, ATR, Stochastic, ADX, Williams%R, Donchian, OBV, VWAP

**Input Sizes**: 100, 1,000, 10,000, 100,000 elements

| Indicator   | Baseline Time | Post-Audit Time | Change | Status |
|-------------|---------------|-----------------|--------|--------|
| SMA (100K)  | - | - | - | - |
| EMA (100K)  | - | - | - | - |
| RSI (100K)  | - | - | - | - |
| MACD (100K) | - | - | - | - |
| Bollinger (100K) | - | - | - | - |
| ATR (100K)  | - | - | - | - |
| Stochastic (100K) | - | - | - | - |
| ADX (100K)  | - | - | - | - |
| Williams%R (100K) | - | - | - | - |
| Donchian (100K) | - | - | - | - |
| OBV (100K)  | - | - | - | - |
| VWAP (100K) | - | - | - | - |

### 2. TA-Lib Comparison (`crates/liq-ta/benches/talib_comparison.rs`)

**Categories**: Moving Averages, Momentum, Trend, Volatility, Stochastic, Volume, Statistics

**Input Size**: 100,000 elements

| Category | Indicator | liq-ta Time | TA-Lib Time | Speedup | Change vs Baseline |
|----------|-----------|--------------|-------------|---------|-------------------|
| Moving Averages | SMA | - | - | - | - |
| Moving Averages | EMA | - | - | - | - |
| Moving Averages | WMA | - | - | - | - |
| Moving Averages | DEMA | - | - | - | - |
| Moving Averages | TEMA | - | - | - | - |
| Moving Averages | TRIMA | - | - | - | - |
| Moving Averages | KAMA | - | - | - | - |
| Moving Averages | T3 | - | - | - | - |
| Momentum | RSI | - | - | - | - |
| Momentum | MACD | - | - | - | - |
| Momentum | MOM | - | - | - | - |
| Momentum | ROC | - | - | - | - |
| Momentum | CMO | - | - | - | - |
| Momentum | APO | - | - | - | - |
| Momentum | TRIX | - | - | - | - |
| Trend | ADX | - | - | - | - |
| Trend | DX | - | - | - | - |
| Trend | AROON | - | - | - | - |
| Trend | CCI | - | - | - | - |
| Volatility | ATR | - | - | - | - |
| Volatility | TRANGE | - | - | - | - |
| Volatility | Bollinger | - | - | - | - |
| Stochastic | STOCH | - | - | - | - |
| Stochastic | STOCHF | - | - | - | - |
| Stochastic | Williams%R | - | - | - | - |
| Stochastic | ULTOSC | - | - | - | - |
| Volume | OBV | - | - | - | - |
| Volume | AD | - | - | - | - |
| Volume | MFI | - | - | - | - |
| Statistics | VAR | - | - | - | - |
| Statistics | LINEARREG | - | - | - | - |
| Statistics | TSF | - | - | - | - |
| Other | MIDPOINT | - | - | - | - |
| Other | MIDPRICE | - | - | - | - |
| Other | BOP | - | - | - | - |

### 3. Workload Simulation (`crates/liq-ta/benches/workload.rs`)

**Scenario**: Realistic backtesting workload with 12 indicators (SMA, EMA, RSI, MACD, ATR, Bollinger, Stochastic, ADX, OBV, VWAP)

**Input Size**: 100,000 elements

| Scenario | Baseline Time | Post-Audit Time | Change | Status |
|----------|---------------|-----------------|--------|--------|
| alloc_each_iter | - | - | - | - |
| reuse_buffers | - | - | - | - |
| Buffer reuse speedup | - | - | - | - |

## Files Modified During Audit

The following files were modified during the standards compliance audit. Performance-critical files are marked:

| File | Change Type | Performance Risk |
|------|-------------|------------------|
| `lib.rs` | Comments only | None |
| `kernels/rolling_extrema.rs` | Loop simplification | Low (semantic equivalent) |
| `indicators/dema.rs` | Validation refactor | None (cold path) |
| `indicators/tema.rs` | Validation refactor | None (cold path) |
| `indicators/t3.rs` | Validation refactor | None (cold path) |
| `indicators/mama.rs` | Dead code removal | Positive (less code) |
| `indicators/cmo.rs` | Bug fix (T::nan()) | None (initialization) |
| `indicators/trix.rs` | Bug fix (T::nan()) | None (initialization) |
| `indicators/stochrsi.rs` | Bug fix (T::nan()) | None (initialization) |
| `indicators/mfi.rs` | Bug fix (T::nan()) | None (initialization) |
| `indicators/ultosc.rs` | Constant hoisting | Positive (optimization) |
| `indicators/ht_core.rs` | Dead code removal | Positive (less code) |
| `indicators/ht_trendline.rs` | Code reuse refactor | Low (shared function) |
| `indicators/candlestick/*.rs` | DRY improvements | None (helper exports) |
| `csv_parser.rs` (CLI) | Capacity hint | Positive (optimization) |

## Acceptance Criteria

Per the audit specification (spec.md), performance changes are acceptable if:

1. **No regression > 5%** compared to baseline
2. Measurement noise is expected at ~2% (criterion default)
3. Regressions in the 2-5% range require documentation/justification
4. Regressions > 5% are **BLOCKING** and must be fixed before sign-off

## Interpreting Criterion Output

Criterion will report one of three outcomes for each benchmark:

- **"No change in performance detected"** - Within measurement noise (typically ±2%)
- **"Performance has improved by X%"** - Faster than baseline
- **"Performance has regressed by X%"** - Slower than baseline (INVESTIGATE if > 5%)

Example output:
```
sma/100000              time:   [123.45 µs 124.00 µs 124.55 µs]
                        thrpt:  [803.22 Melem/s 806.45 Melem/s 810.02 Melem/s]
                        change: [-1.2345% -0.5678% +0.1234%] (p = 0.12 > 0.05)
                        No change in performance detected.
```

## Manual Verification Steps

Since cargo commands are blocked in the automation environment, the user must:

1. **Navigate to project root**:
   ```bash
   cd /path/to/liq-ta
   ```

2. **Verify baseline exists**:
   ```bash
   ls target/criterion/*/audit-baseline/
   ```
   If missing, run: `cargo +nightly bench -p liq-ta -- --save-baseline audit-baseline`

3. **Run comparison benchmarks**:
   ```bash
   cargo +nightly bench -p liq-ta -- --baseline audit-baseline
   ```

4. **Check for regressions**:
   - Look for any "Performance has regressed" messages
   - Any regression > 5% is BLOCKING
   - Document findings in this file

5. **Fill in results tables above**:
   - Record actual times from benchmark output
   - Calculate change percentages
   - Mark status as PASS (< 5% regression) or FAIL (> 5% regression)

## Results Summary

**Date**: [TO BE FILLED WHEN BENCHMARKS RUN]
**Rust Version**: [TO BE FILLED]
**Nightly Version**: [TO BE FILLED]
**Hardware**: [TO BE FILLED]

### Overall Outcome

- [ ] All benchmarks completed successfully
- [ ] No regressions > 5% detected
- [ ] Hot-path indicators (SMA, EMA, RSI, MACD) unchanged
- [ ] Workload simulation shows no regression

### Findings

[TO BE FILLED AFTER RUNNING BENCHMARKS]

### Sign-off

- [ ] Benchmark verification complete
- [ ] All acceptance criteria met
- [ ] Ready for QA sign-off

---
*This document is part of the Rust Standards Compliance Audit (Issue #011)*
*Subtask: 9-3 - Run full benchmark suite and compare to baseline*
