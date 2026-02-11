# liq-ta Product Requirements Document

**Version**: 3.0
**Last Updated**: December 2025
**Status**: Production Architecture Defined

---

## 1. Executive Summary

### 1.1 Problem Statement

Existing technical analysis libraries have fundamental issues that affect quantitative trading workflows:

**Numerical Stability Gaps**:
- Undocumented initialization behavior leads to divergent results across libraries
- Edge cases (all gains, flat prices, NaN in data) produce inconsistent or undefined outputs
- No specification ownership—implementations inherit upstream quirks without guaranteeing behavior

**Performance Limitations**:
- O(n×k) algorithms for rolling operations that should be O(n)
- Fusion approaches that sound faster but prevent SIMD vectorization
- Allocation patterns that dominate runtime in hot loops

**Specification Opacity**:
- "TA-Lib compatible" means inheriting undocumented behavior, not correctness
- No authoritative source for what an indicator *should* compute
- Users cannot validate outputs because expected behavior isn't defined

**Target Users**: Quantitative developers who need:
- Reproducible indicator outputs across runs and platforms
- Documented edge-case behavior for backtesting validation
- Performance suitable for large-scale historical analysis
- A specification they can audit and trust

liq-ta addresses these gaps by owning its specification, documenting all edge cases, and validating performance through experiments.

### 1.2 Product Vision

liq-ta is a high-performance technical analysis library for financial markets, implemented in Rust. The library provides a comprehensive set of technical indicators optimized for speed, numerical stability, and memory efficiency.

### 1.3 Goals

1. **Performance**: Achieve O(n) time complexity for all indicators through optimized algorithms
2. **Accuracy**: Numerically correct, stable outputs with well-defined initialization rules; TA-Lib used as a *reference check* for validation, not a compatibility contract
3. **Efficiency**: Apply only proven optimizations (deque rolling extrema, interleaved writes, pre-allocation)
4. **Usability**: Simple, ergonomic API with sensible defaults; configuration types for complex indicators

> **Architecture Decision (v2.0)**: Plan mode infrastructure has been **archived** based on E07 results showing 1.4-2.2× slower performance than direct mode. The mainline library uses direct indicator calls only.

> **Archive Policy**: The `archive/experiments-v1` branch preserves plan mode and fusion kernel code for historical reference only. This code is not planned for production—experiments showed these approaches are slower under current conditions. Reconsidering would require new evidence (compiler improvements, different workloads, etc.) and fresh experiments. The branch exists for audit purposes and may be deleted after v1.0 stable release.

> **Specification Stance (v2.2)**: liq-ta defines its own indicator semantics. TA-Lib is used as a reference for validation but is not a compatibility requirement. Where our behavior differs from TA-Lib, liq-ta's specification is authoritative.

### 1.4 Performance Hypotheses

The following performance hypotheses were tested through 7 micro-experiments (E01-E07):

| ID | Hypothesis | Target | Experiment | Status |
|----|------------|--------|------------|--------|
| H1 | All baseline indicators achieve O(n) complexity | Linear scaling 10K→100K | E01 | **VALIDATED ✓** |
| H2 | Welford's algorithm provides faster fused mean/variance/stddev | ≥20% speedup vs separate passes | E02 | **INVALIDATED ✗** |
| H3 | Multi-EMA fusion reduces memory bandwidth | ≥15% speedup for ≥10 EMAs | E03 | **INVALIDATED ✗** |
| H4 | Monotonic deque provides O(n) rolling extrema | ≥5× speedup vs O(n×k) naive at k≥50 | E04 | **VALIDATED ✓** |
| H5 | Plan compilation overhead is acceptable | Break-even in <100 executions | E05 | **VALIDATED ✓** |
| H6 | Write patterns affect cache performance | ≥10% improvement possible | E06 | **VALIDATED ✓** |
| H7 | Plan mode outperforms direct mode for multi-indicator workloads | ≥1.5× speedup for ≥20 indicators | E07 | **INVALIDATED ✗** |

**Validation Summary** (based on experiments completed December 2025):

- **H1 (VALIDATED)**: All 7 indicators show near-linear scaling (8.9×-10.6×) from 10K→100K data points. See [E01 Report](../benches/experiments/E01_baseline/REPORT.md).
- **H2 (INVALIDATED)**: Welford-based fusion is 2.8× SLOWER than separate SMA + StdDev passes due to expensive division operations. See [E02 Report](../benches/experiments/E02_running_stat/REPORT.md).
- **H3 (INVALIDATED)**: Multi-EMA fusion is 30% SLOWER at 10 EMAs due to SIMD vectorization prevention and register pressure. See [E03 Report](../benches/experiments/E03_ema_fusion/REPORT.md).
- **H4 (VALIDATED)**: Monotonic deque achieves 4.3× speedup at k=50 and 24.4× at k=200. Crossover point is ~25; use naive for period<25, deque for period≥25. See [E04 Report](../benches/experiments/E04_rolling_extrema/REPORT.md).
- **H5 (VALIDATED)**: Plan compilation takes only 2.2 μs (500× better than 1ms target). Break-even is immediate with any fusion benefit. See [E05 Report](../benches/experiments/E05_plan_overhead/REPORT.md).
- **H6 (VALIDATED)**: Pre-allocation provides 17-28% speedup; interleaved multi-output writes are 2.53× faster than sequential. Buffered writes are 19-27% SLOWER than direct writes. See [E06 Report](../benches/experiments/E06_memory_writes/REPORT.md).
- **H7 (INVALIDATED)**: Plan mode is 1.4-2.2× SLOWER than direct mode across all configurations due to fusion kernel overhead (E02, E03 findings). Direct mode is recommended. See [E07 Report](../benches/experiments/E07_end_to_end/REPORT.md).

### 1.5 Replacement Strategy

liq-ta aims to be a credible **TA-Lib alternative** for Rust-native projects and, via FFI, for polyglot trading stacks. Replacement viability is defined by coverage, interop, and stability—not just performance.

**Coverage Tiers**:

| Tier | Scope | Target | Status |
|------|-------|--------|--------|
| **Tier 0 (Core)** | Most-used indicators for common strategies | v1.0 | **Complete** |
| **Tier 1 (Extended)** | Additional 20-30 common indicators | v1.x | Planned |
| **Tier 2 (Long Tail)** | Remaining TA-Lib coverage | v2.x+ | Future |

**Tier 0 Indicators** (minimum viable replacement set):

| Category | Indicators | Status |
|----------|------------|--------|
| Trend | SMA, EMA, ADX | ✓ Complete |
| Momentum | RSI, MACD, Stochastic, Williams %R | ✓ Complete |
| Volatility | ATR, Bollinger Bands, Donchian Channels | ✓ Complete |
| Volume | OBV, VWAP | ✓ Complete |

**Success Metric**: Tier 0 covers ~80% of indicator usage in common quantitative trading stacks (backtesting, signal generation, risk management).

**Completion Gate**: v1.0 ships when:
1. All Tier 0 indicators pass spec fixtures, reference checks, and real-world regression suite (§7.7)
2. Python bindings available via `pip install liq-ta` (§12.5)
3. Beta validation with ≥3 external users confirms no blocking issues

**Interop Strategy**: See §12.5 for bindings roadmap. Python bindings (PyO3 + NumPy) are the primary adoption vector.

---

## 2. Target Users

### 2.1 Primary Users

1. **Quantitative Traders**: Building algorithmic trading systems requiring fast indicator computation
2. **Financial Analysts**: Analyzing historical market data for patterns and trends
3. **Trading Platform Developers**: Integrating technical analysis into trading applications

### 2.2 User Requirements

| Requirement | Priority | Implementation |
|-------------|----------|----------------|
| Fast indicator computation | High | O(n) algorithms; <10ms per indicator at 100K samples |
| Numerical accuracy | High | Spec fixture tests (authoritative); TA-Lib reference checks |
| Memory efficiency | Medium | Pre-allocation, `_into` variants, interleaved writes |
| Easy-to-use API | High | Simple functions with configuration types for complex indicators |
| CLI for testing | Medium | CSV input/output for indicator validation |

---

## 3. Functional Requirements

### 3.1 Core Indicators (Implemented)

| Indicator | Type | Algorithm | Complexity |
|-----------|------|-----------|------------|
| **SMA** | Trend | Rolling sum | O(n) |
| **EMA** | Trend | Recursive formula | O(n) |
| **RSI** | Momentum | Wilder smoothing | O(n) |
| **MACD** | Trend/Momentum | Triple EMA | O(n) |
| **ATR** | Volatility | True Range + Wilder | O(n) |
| **Bollinger Bands** | Volatility | SMA + rolling stddev | O(n) |
| **Stochastic Oscillator** | Momentum | Rolling extrema + SMA | O(n) with deque |

**Stochastic Variants**: The API uses `stochastic()` as the canonical function name, configurable via `k_slowing`:
- **Fast Stochastic**: `k_slowing = 1` (default). %K = raw stochastic, %D = SMA(%K, d_period). No smoothing applied to %K.
- **Slow Stochastic**: `k_slowing > 1`. %K is smoothed with SMA(k_slowing) before computing %D. Setting `k_slowing = 3` produces traditional Slow Stochastic.
- **Full Stochastic**: Available via `stochastic_full()` for advanced use cases requiring explicit control over `slow_k_period` parameter ordering.

The `k_slowing` parameter applies SMA smoothing (window = `k_slowing`) to %K before computing %D. Smoothing is applied once, not repeatedly.

**API Preference**: Use `stochastic()` for typical use cases. The convenience functions (`stochastic_fast`, `stochastic_slow`, `stochastic_full`) are retained for explicit variant selection and backward compatibility.

**Stochastic Formula**:

```
raw_%K = (close - lowest_low) / (highest_high - lowest_low) * 100
```

Where `lowest_low` and `highest_high` are computed over the `k_period` window **including** the current bar.

**Edge Case Precedence** (evaluated in order):

1. If any of `high`, `low`, or `close` in the required window contains NaN → output NaN
2. If `highest_high == lowest_low` (zero range, all inputs finite) → %K = 50
3. Otherwise → apply formula

This means NaN propagation takes priority over the flat-price fallback.

### 3.2 Optimized Kernels (Production)

Based on E01-E07 experiment findings, only one optimization kernel is retained in production:

| Kernel | Purpose | Status | Notes |
|--------|---------|--------|-------|
| **Rolling Extrema** | Monotonic deque max/min | ✅ **Production** | Hybrid: naive for period < 25, deque for period ≥ 25 (E04: 4.3-24.4× faster). Threshold validated in E04; crossover measured at k≈25. |

### 3.3 Archived Kernels (Not in Production)

The following were tested and found to be slower than direct implementations:

| Kernel | Purpose | Status | Findings |
|--------|---------|--------|----------|
| **RunningStat** | Fused mean/variance/stddev | ❌ **Archived** | E02: 2.8× slower due to division overhead |
| **EMA Fusion** | Multi-EMA in single pass | ❌ **Archived** | E03: 30% slower due to SIMD prevention |
| **MACD Fusion** | Fused fast/slow/signal EMAs | ❌ **Archived** | E03: 5% slower at scale |
| **Plan Mode** | DAG-driven execution | ❌ **Archived** | E07: 1.4-2.2× slower than direct mode |

Archived code is preserved in the `archive/experiments-v1` branch.

---

## 4. Non-Functional Requirements

### 4.1 Performance Requirements

| Metric | Target | Validation |
|--------|--------|------------|
| Single indicator (100K data) | <10ms | Benchmark suite |
| 7 indicators (100K data) | <50ms | Benchmark suite |
| O(n) scaling | ≤11× from 10K→100K | Scaling regression test |
| Memory usage | O(n) per indicator | Design constraint |
| Performance regression | ≤15% vs baseline | CI benchmark gate |

### 4.2 Accuracy Requirements

| Metric | Target | Validation |
|--------|--------|------------|
| Spec correctness | Matches liq-ta specification | Spec fixture tests (authoritative) |
| Numerical stability | Handle extreme values | Rolling sum + sum-of-squares (Bollinger), stable EMA recursion |
| NaN handling | Propagation policy | Unit tests + property tests |
| Reference comparison | Sanity check; divergences investigated | Reference checks (informational, non-blocking) |

> **Note**: Spec fixtures are the authoritative source of truth. TA-Lib reference comparisons are used to catch major regressions but do not define correctness.

### 4.3 Numeric Policy

Explicit behavior for special floating-point values:

| Input Condition | Behavior | Rationale |
|-----------------|----------|-----------|
| **NaN in window** | Output NaN for that position | Propagation prevents silent corruption |
| **Infinity in input** | Propagate to output | Mathematical correctness |
| **Empty input** | Return `Error::EmptyInput` | Fail-fast at boundary |
| **All-NaN window** | Output NaN | Consistent with window-contains-NaN rule |
| **Subnormal values** | Process normally | No special handling needed |

**Design Rationale**: NaN propagation is preferred over error returns because:
1. Allows partial results when only some windows contain NaN
2. Follows common industry practice for numeric libraries
3. Caller can filter NaN from output if needed

**Indeterminate Operations**: Some operations produce mathematically indeterminate results (e.g., 0/0, ∞−∞). Rather than propagating NaN in these cases, indicators define deterministic outputs:
- RSI: 0/0 when avg_loss = 0 → RSI = 100 (all gains)
- Stochastic: 0/0 when high == low → %K = 50 (midpoint)

These are not NaN scenarios being "overridden"—they are explicitly defined behaviors for indeterminate operations. See §4.5 for the complete list.

### 4.4 Output Shape Contract

All indicators return **full-length outputs with NaN padding**. The output has the same length as the input, with NaN values marking positions where insufficient lookback data exists to compute a valid value.

**Rationale for Full-Length Output**:
- **Index alignment**: `prices[i]` and `sma[i]` correspond to the same time point, enabling direct iteration and comparison
- **Consistent NaN handling**: NaN at the start (lookback period) is consistent with NaN propagation for internal gaps
- **Easier indicator combination**: No offset calculations needed when combining indicators with different lookbacks
- **Uniform behavior**: All indicators return same-length outputs regardless of lookback requirements

**Canonical Definition**: The `*_lookback()` functions are the sole authoritative source for lookback values. The table below is explanatory only; if it ever disagrees with `*_lookback()`, the function is correct.

| Indicator | Lookback (illustrative) | Output Length | NaN Positions | Rationale |
|-----------|-------------------------|---------------|---------------|-----------|
| SMA(n) | n - 1 | input.len() | [0..n-1] | Need n values for first average |
| EMA(n) | n - 1 | input.len() | [0..n-1] | Seeded with SMA(n) of first window |
| RSI(n) | n | input.len() | [0..n] | Need n+1 prices for n diffs, then smooth |
| MACD(f,s,g) | s - 1 + g - 1 | input.len() | [0..lookback] | Slow EMA warmup + signal EMA warmup |
| ATR(n) | n | input.len() | [0..n] | First TR needs prior close; SMA initialization |
| Bollinger(n) | n - 1 | input.len() | [0..n-1] | Same as underlying SMA |
| Stochastic(k,d) | k - 1 + d - 1 | input.len() | [0..lookback] | Rolling extrema + %D smoothing (assumes k_slowing=1) |

**Lookback Function (Canonical)**: Each indicator exposes `*_lookback(params) -> usize` as the authoritative definition. These functions are part of the stable public API and follow semver (breaking changes require major version bump).

```rust
use liq_ta::indicators::{sma_lookback, macd_lookback};

let sma_lb = sma_lookback(20);           // Returns 19
let macd_lb = macd_lookback(12, 26, 9);  // Returns 33 (25 + 8)
```

**API Contract**:
```rust
// Function API returns Vec same length as input, with NaN for lookback positions
let sma = sma(&prices, 20)?;  // sma.len() == prices.len()
assert!(sma[0].is_nan());     // First 19 values are NaN
assert!(!sma[19].is_nan());   // First valid value at index 19 (lookback)

// NaN count at start matches lookback
let nan_count = sma.iter().take_while(|x| x.is_nan()).count();
assert_eq!(nan_count, sma_lookback(20));

// _into API requires buffer of at least input length
let mut buffer = vec![0.0; prices.len()];
sma_into(&prices, 20, &mut buffer)?;

// Error if buffer too small
let mut small = vec![0.0; 10];
assert!(sma_into(&prices, 20, &mut small).is_err()); // BufferTooSmall
```

**Multi-Output Alignment**: All fields in multi-output indicators are aligned and have identical length:
- MACD: `macd`, `signal`, `histogram` all same length
- Bollinger: `upper`, `middle`, `lower` all same length
- Stochastic: `k`, `d` all same length

The indicator's lookback is the maximum of all subcomponent lookbacks. All multi-output fields begin at the same logical time index (input position `lookback`) and refer to the same input-aligned positions. There is no "early" output of one field before another becomes valid.

**Multi-Output `_into` Contract**: Pre-allocated buffer APIs for multi-output indicators use separate mutable slices. All buffers must match input length:
```rust
// MACD: three output buffers, each of length >= input.len()
let mut macd_line = vec![0.0; prices.len()];
let mut signal = vec![0.0; prices.len()];
let mut histogram = vec![0.0; prices.len()];
macd_into(&prices, 12, 26, 9, &mut macd_line, &mut signal, &mut histogram)?;
// NaN in first macd_lookback(12, 26, 9) positions of each buffer

// Bollinger: three output buffers
let mut upper = vec![0.0; prices.len()];
let mut middle = vec![0.0; prices.len()];
let mut lower = vec![0.0; prices.len()];
bollinger_into(&prices, 20, 2.0, &mut upper, &mut middle, &mut lower)?;

// Stochastic: two output buffers
let mut k = vec![0.0; high.len()];
let mut d = vec![0.0; high.len()];
stochastic_into(&high, &low, &close, 14, 3, &mut k, &mut d)?;
```

All buffer length requirements are enforced; `Error::BufferTooSmall` returned if any buffer is undersized.

**Chaining Implications**: When chaining indicators, NaN values propagate:
```rust
let rsi = rsi(&prices, 14)?;           // len = prices.len(), NaN in first rsi_lookback(14) positions
let smoothed = ema(&rsi, 5)?;          // len = rsi.len(), NaN propagates and adds ema_lookback(5) more
// Total NaN prefix = rsi_lookback(14) + ema_lookback(5)
```

**Minimum Input Length**: To avoid `InsufficientData` errors, input length must be at least `lookback + 1`. Each indicator exports a `*_min_len()` function:

```rust
use liq_ta::indicators::{sma_min_len, macd_min_len};

// SMA(20) needs at least 20 values (lookback=19, min=20)
assert!(prices.len() >= sma_min_len(20));

// MACD(12,26,9) needs at least 34 values (lookback=33, min=34)
assert!(prices.len() >= macd_min_len(12, 26, 9));
```

For multi-input indicators (e.g., Stochastic with high/low/close), all inputs must meet this minimum. The `*_min_len()` functions are semver-stable alongside `*_lookback()`.

### 4.5 Indicator-Specific Numeric Edge Cases

Beyond the general numeric policy (§4.3), these indicators have specific edge behaviors:

| Indicator | Edge Case | Behavior | Rationale |
|-----------|-----------|----------|-----------|
| **RSI** | All gains (avg_loss = 0) | RSI = 100 | Maximum bullish momentum |
| **RSI** | All losses (avg_gain = 0) | RSI = 0 | Maximum bearish momentum |
| **Stochastic** | Denominator = 0 (high == low) | %K = 50 | Stable midpoint when range undefined |
| **Bollinger** | StdDev = 0 (constant prices) | Upper = Lower = Middle | Bands collapse to mean |
| **ATR** | First value initialization | SMA of first n True Ranges | Wilder's original method |

**Bollinger Bands Standard Deviation**: Uses **population stddev** (divide by n, not n-1). Calculated via sum-of-squares method for numerical stability.

> **Note**: Some implementations (including Excel) use sample stddev (n-1). liq-ta uses population stddev, which matches TA-Lib and most financial charting platforms. Users migrating from sample-stddev implementations may see slightly narrower bands.

### 4.6 Indicator Initialization Rules

These rules define how each indicator seeds its initial state. This is the authoritative specification for liq-ta:

| Indicator | Initialization | Description |
|-----------|----------------|-------------|
| **EMA** | SMA seed | First EMA value = SMA of first `period` values; then apply α = 2/(period+1) |
| **RSI** | Wilder smoothing | First avg_gain/avg_loss = SMA of first `period` diffs; then Wilder smooth (α = 1/period) |
| **ATR** | SMA then Wilder | First ATR = SMA of first `period` True Ranges; subsequent values use Wilder smoothing |
| **MACD signal** | EMA of MACD line | Signal line initialized after slow EMA warmup using EMA rules above |
| **Stochastic %D** | SMA of %K | %D = SMA of `d_period` %K values |

**EMA Smoothing Factor**: Standard EMA uses α = 2/(period+1). Wilder smoothing (RSI, ATR) uses α = 1/period.

> **Design Note**: SMA seeding for EMA is chosen for stability and predictability. Some implementations use first-value seeding for streaming scenarios; liq-ta uses SMA seeding for batch computation.

### 4.7 Code Quality Requirements

| Metric | Target | Tool |
|--------|--------|------|
| Test coverage (indicators) | ≥95% | cargo-tarpaulin |
| Test coverage (rolling extrema) | ≥90% | cargo-tarpaulin |
| Linting | Zero warnings | cargo clippy |
| Formatting | Consistent | cargo fmt |
| Documentation | 100% public items | cargo doc |

### 4.8 Mathematical Conventions

These conventions apply consistently across all indicators:

| Convention | Value | Notes |
|------------|-------|-------|
| **Standard deviation** | Population (÷n) | Not sample (÷n-1); used in Bollinger |
| **Variance algorithm** | Sum-of-squares | Preferred over Welford for cache efficiency (E02) |

**Tolerance Policy**:

Floating-point comparisons use mixed tolerance: `|a - b| <= abs_tol + rel_tol * max(|a|, |b|)`.

| Context | Absolute Tolerance | Relative Tolerance | Notes |
|---------|-------------------|-------------------|-------|
| **Spec fixtures** | 1e-14 | 1e-12 | Hand-constructed; tighter tolerance or exact match for small vectors |
| **Reference comparisons** | 1e-10 | 1e-10 | TA-Lib sanity checks; looser due to potential algorithm differences |
| **Cross-platform** | 1e-10 | 1e-10 | Expected variance across compilers/targets |

> **Reproducibility Note**: On a fixed toolchain, target, and build profile (no `fast-math`, consistent FMA settings), results are expected to be stable. However, users should not depend on bit-identical results across different environments. The tolerances above are test expectations, not hard guarantees.

**Precision Acknowledgment**: For extremely large magnitude data with small variance, sum-of-squares may lose precision due to catastrophic cancellation. This is accepted; users with such data should pre-scale inputs.

### 4.9 Non-Goals (v2.x)

The following are explicitly out of scope for the v2.x release series:

| Feature | Status | Notes |
|---------|--------|-------|
| **Streaming/incremental API** | Deferred | Batch computation only; streaming is future work |
| **Configurable EMA seeding** | Deferred | Only SMA-seeded EMA; first-value seeding is future work |
| **Alternative MA types** | Deferred | Stochastic %D uses SMA only; EMA/Wilder variants are future work |
| **GPU acceleration** | Deferred | Insufficient workload size for GPU overhead |
| **Plan mode / fusion** | Archived | Proven slower in E02-E07 |

These may be revisited in future major versions based on user demand.

**Streaming Workaround**: For live trading scenarios requiring incremental updates, users can:
1. Maintain a rolling window of recent prices externally
2. Recompute indicators on each new bar using only the relevant window
3. Use `lookback()` to determine minimum window size needed

Performance impact is typically sub-millisecond for small windows (100-1000 values). For most real-time applications, batch recomputation is acceptable until a streaming API is added.

### 4.10 Stability Contracts

The following are **semver-stable** commitments. Breaking changes require a major version bump:

| Contract | Scope | Notes |
|----------|-------|-------|
| **Lookback functions** | `*_lookback()` return values | Part of public API; changes affect output alignment |
| **Initialization rules** | §4.6 seeding behavior | EMA SMA-seed, Wilder smoothing, etc. |
| **Output alignment** | Multi-output field lengths equal | All fields start at same logical index |
| **Output type schemas** | §5.5 struct field names and types | `MacdOutput`, `BollingerOutput`, `StochasticOutput` |
| **Edge case behaviors** | §4.5 indeterminate operation results | RSI=100 for all gains, Stochastic %K=50 for flat, etc. |

**Not semver-stable** (may change in minor versions):

| Item | Rationale |
|------|-----------|
| Exact floating-point bits | Platform/compiler variance expected |
| Performance characteristics | Optimizations may change timing |
| Error message text | Improved diagnostics |
| Internal module structure | Only `prelude` and public items are stable |

---

## 5. API Design

### 5.1 Primary API (Direct Mode)

Simple, low-overhead API for indicator computation:

```rust
use liq_ta::prelude::*;

// Single-output indicators (returns trimmed Vec per §4.4)
let sma_values = sma(&prices, 20)?;  // len = prices.len() - 19
let ema_values = ema(&prices, 20)?;
let rsi_values = rsi(&prices, 14)?;  // len = prices.len() - 14

// Multi-output indicators
let macd_result = macd(&prices, 12, 26, 9)?;
let bb_result = bollinger(&prices, 20, 2.0)?;
let stoch_result = stochastic(&high, &low, &close, 14, 3)?;

// Pre-allocated buffers for repeated computation (E06 optimization)
// Buffer must be at least output_length (see §4.4 for lookback formulas)
let output_len = prices.len() - 19;  // SMA lookback = period - 1
let mut output = vec![0.0; output_len];
sma_into(&prices, 20, &mut output)?;
```

### 5.2 Configured Indicator Types

For indicators with multiple parameters, configuration types implement `Default` and provide fluent setter methods:

**Default Values**:

| Type | Parameter | Default | Description |
|------|-----------|---------|-------------|
| `Macd` | `fast_period` | 12 | Fast EMA period |
| `Macd` | `slow_period` | 26 | Slow EMA period |
| `Macd` | `signal_period` | 9 | Signal line EMA period |
| `Bollinger` | `period` | 20 | SMA period for middle band |
| `Bollinger` | `std_dev` | 2.0 | Standard deviation multiplier |
| `Stochastic` | `k_period` | 14 | Lookback period for %K |
| `Stochastic` | `k_slowing` | 1 | Smoothing for %K (1 = no smoothing) |
| `Stochastic` | `d_period` | 3 | SMA period for %D |

```rust
use liq_ta::prelude::*;

// MACD with defaults (fast=12, slow=26, signal=9)
let macd_result = Macd::default().compute(&prices)?;

// MACD with custom parameters
let macd_result = Macd::new()
    .with_fast_period(10)
    .with_slow_period(21)
    .with_signal_period(7)
    .compute(&prices)?;

// Bollinger with defaults (period=20, std_dev=2.0)
let bb_result = Bollinger::default().compute(&prices)?;

// Fast Stochastic (default: k_slowing=1)
let fast_stoch = Stochastic::default().compute(&high, &low, &close)?;

// Slow Stochastic (k_slowing=3 is traditional)
let slow_stoch = Stochastic::new()
    .with_k_slowing(3)
    .compute(&high, &low, &close)?;
```

**Pattern**: These are not separate "builder" types but rather the indicator configuration types themselves implementing `Default + compute()`. The same type is used for configuration and execution.

### 5.3 Indicator Chaining

For computing derived indicators (composition without plan mode):

```rust
use liq_ta::prelude::*;

// RSI of a midrange price
let mut midrange: Vec<f64> = Vec::with_capacity(high.len());
for (h, l) in high.iter().zip(low.iter()) {
    midrange.push((h + l) / 2.0);
}
let rsi_of_midrange = rsi(&midrange, 14)?;

// EMA of RSI (smoothed RSI)
let rsi_values = rsi(&prices, 14)?;
let smoothed_rsi = ema(&rsi_values, 5)?;
```

**Alignment Pattern**: When combining outputs from different indicators, lengths may differ due to different lookbacks. Use tail alignment to match series. This is a recommended pattern, not an exported function:

```rust
// Pattern: align two slices by their tails (most recent values)
let min_len = rsi_vals.len().min(macd_result.macd.len());
let rsi_aligned = &rsi_vals[rsi_vals.len() - min_len..];
let macd_aligned = &macd_result.macd[macd_result.macd.len() - min_len..];
// Both now have same length, aligned to most recent values
```

This stays within direct-mode philosophy while making indicator combinations safer. Users may implement their own `align_tail` helper if desired.

### 5.4 CLI Contract

The CLI provides a testing interface for indicator computation.

**Core Behavior**:
- Accepts CSV with OHLC(+V) columns (header required)
- Preserves date column in output (if present)
- Outputs trimmed results aligned with first valid output (no NaN prefix)
- Multi-output indicators produce multiple named columns
- Internal NaN values preserved; only lookback rows dropped

**Invocation**:
```bash
liq-ta <indicator> <input.csv> [params] [-o output.csv] [-c column]
```

**Examples**:
```bash
# SMA with default period (20)
liq-ta sma prices.csv

# SMA with custom period
liq-ta sma prices.csv 14

# MACD with custom params, output to file
liq-ta macd prices.csv 12,26,9 -o macd_output.csv

# Stochastic with slow stochastic (k_slowing=3)
liq-ta stochastic ohlc.csv 14,3,3
```

**Exit Codes**:
| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Argument error (invalid indicator, missing file) |
| 2 | Data error (parse failure, insufficient data) |
| 3 | Computation error (indicator failed) |

> **Note**: Detailed CSV format specification, column mapping rules, and usage examples are documented in `crates/liq-ta-cli/README.md`.

### 5.5 Output Types (Semver Stable)

Multi-output indicators return typed structs with named fields. These types are part of the stable public API:

```rust
/// MACD indicator output
pub struct MacdOutput {
    pub macd: Vec<f64>,      // MACD line (fast EMA - slow EMA)
    pub signal: Vec<f64>,    // Signal line (EMA of MACD)
    pub histogram: Vec<f64>, // MACD - signal
}

/// Bollinger Bands output
pub struct BollingerOutput {
    pub upper: Vec<f64>,     // Upper band (middle + std_dev * σ)
    pub middle: Vec<f64>,    // Middle band (SMA)
    pub lower: Vec<f64>,     // Lower band (middle - std_dev * σ)
}

/// Stochastic oscillator output
pub struct StochasticOutput {
    pub k: Vec<f64>,         // %K line
    pub d: Vec<f64>,         // %D line (SMA of %K)
}
```

Field names and types are guaranteed stable under semver. All fields within an output have identical length (see §4.4 Multi-Output Alignment).

**Trait Implementations**: All output types derive `Clone` and `Debug`. With `--features serde`, they additionally derive `Serialize` and `Deserialize`. Output types do not implement `Default` (empty vectors would be semantically incorrect) or `PartialEq` (float comparison is context-dependent; use tolerance-based comparison from §4.8).

### 5.6 API Layers

liq-ta provides two API styles for different use cases:

**Simple API** (v1.0, current):

- Functions return `Result<Vec<f64>, Error>` or `Result<*Output, Error>`
- Allocates output on each call
- Best for: scripting, one-off computations, prototyping

```rust
let sma = sma(&prices, 20)?;           // Returns Vec<f64>
let macd = macd(&prices, 12, 26, 9)?;  // Returns MacdOutput
```

**Buffer API** (v1.0, current):

- `_into` variants write to pre-allocated buffers
- Avoids allocation overhead for repeated calls
- Best for: hot loops, backtesting, real-time systems

```rust
let mut buffer = vec![0.0; prices.len() - sma_lookback(20)];
sma_into(&prices, 20, &mut buffer)?;  // Writes to buffer
```

**Advanced API** (v2.x, future consideration):

For high-performance reuse scenarios, future versions may introduce:
- Generic numeric types (`f32` support)
- View-based outputs for zero-copy chaining
- Buffer pool integration

These would be additive; Simple and Buffer APIs remain stable.

---

## 6. Architecture

### 6.1 Crate Structure

```
liq-ta/
├── Cargo.toml                     # Workspace configuration
├── crates/
│   ├── liq-ta/                   # Library crate (published)
│   │   ├── src/
│   │   │   ├── lib.rs             # Library entry point
│   │   │   ├── prelude.rs         # Common imports
│   │   │   ├── error.rs           # Error types
│   │   │   ├── traits.rs          # SeriesElement, ValidatedInput
│   │   │   ├── kernels/
│   │   │   │   ├── mod.rs
│   │   │   │   └── rolling_extrema.rs  # E04 validated optimization
│   │   │   └── indicators/        # 7 baseline indicators
│   │   │       ├── mod.rs
│   │   │       ├── sma.rs
│   │   │       ├── ema.rs
│   │   │       ├── rsi.rs
│   │   │       ├── macd.rs
│   │   │       ├── atr.rs
│   │   │       ├── bollinger.rs
│   │   │       └── stochastic.rs
│   │   ├── benches/               # Performance benchmarks
│   │   ├── tests/
│   │   │   ├── fixtures/          # Spec fixtures (authoritative)
│   │   │   ├── golden/            # TA-Lib reference files (informational)
│   │   │   ├── integration.rs     # Public API tests
│   │   │   └── property_tests.rs  # proptest
│   │   └── Cargo.toml
│   └── liq-ta-cli/               # CLI binary (published separately)
│       ├── src/
│       │   ├── main.rs
│       │   ├── args.rs            # CLI argument parsing
│       │   ├── csv_parser.rs      # CSV input handling
│       │   └── csv_writer.rs      # CSV output handling
│       ├── tests/
│       │   └── cli_integration.rs
│       └── Cargo.toml
└── tools/
    └── golden/
        └── generate.py            # Golden file generator (dev-only, excluded from package)
```

> **Golden file location**: `crates/liq-ta/tests/golden/` is the canonical location. The generator script in `tools/golden/` is excluded from crates.io packaging via `Cargo.toml` exclude patterns.

**Crate Responsibilities**:
- `liq-ta`: Library crate with all indicator implementations, published to crates.io (semver stable)
- `liq-ta-cli`: Binary for testing indicators against CSV data, published separately

> **Design Note**: A single library crate is the idiomatic Rust pattern. There is no separate "core" crate—all implementation lives in `liq-ta`. This simplifies dependency management and versioning.

### 6.2 Data Flow Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           User API                                   │
│                         (liq-ta crate)                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────┐     ┌─────────────────────────────────┐   │
│  │   Function API      │     │    Config Types API              │   │
│  │   sma(), ema(), ... │     │   Macd::new().compute()          │   │
│  └──────────┬──────────┘     └───────────────┬─────────────────┘   │
│             │                                │                      │
│             └────────────────┬───────────────┘                      │
│                              ▼                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    Indicators Module                          │   │
│  │   SMA | EMA | RSI | MACD | ATR | Bollinger | Stochastic      │   │
│  └──────────────────────────────┬───────────────────────────────┘   │
│                                 │                                   │
│                                 ▼                                   │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              Rolling Extrema (hybrid deque/naive)             │   │
│  │                     Used by: Stochastic                       │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 6.3 Validated Optimizations

Based on E01-E07 experiments, only these optimizations are applied in production:

| Optimization | Finding | Implementation |
|--------------|---------|----------------|
| **Pre-allocation** | E06: 17-28% speedup | All `_into` variants pre-allocate output |
| **Interleaved writes** | E06: 2.53× faster for multi-output | MACD/Bollinger/Stochastic write columns interleaved |
| **Rolling extrema deque** | E04: 4.3-24× faster for period≥25 | Hybrid: naive for k<25, deque for k≥25 |
| **Direct writes** | E06: Buffered writes 19-27% slower | No buffering layer; write directly to output |

**Key Findings from Experiments**:
- Fusion kernels prevent SIMD auto-vectorization and increase register pressure (E02, E03)
- Simple loops with separate passes optimize better than complex fused loops
- Memory bandwidth is rarely the bottleneck; CPU efficiency dominates
- Plan mode adds overhead without benefit (E07: 1.4-2.2× slower)

### 6.4 Error Handling

```rust
pub enum Error {
    /// Input array is too short for the requested period
    InsufficientData { required: usize, actual: usize },
    /// Input array is empty
    EmptyInput,
    /// Period value is invalid (0, negative, etc.)
    InvalidPeriod { period: usize, reason: String },
    /// Buffer provided to `_into` variant is too small
    BufferTooSmall { required: usize, actual: usize },
}
```

**Error Recovery Guidance**:

| Error | Cause | Recommended Handling |
|-------|-------|---------------------|
| `InsufficientData` | Input shorter than lookback | Collect more data, or use shorter period. Not recoverable by padding—indicators need real data. |
| `EmptyInput` | Zero-length input | Validate input before calling. Likely a programming error. |
| `InvalidPeriod` | Period ≤ 0 or nonsensical | Validate parameters before calling. Display validation error to user. |
| `BufferTooSmall` | `_into` buffer undersized | Use `*_lookback()` to compute required size. Reallocate buffer. |

> **Note**: `CyclicDependency` was removed in v2.0 as plan mode (which required DAG cycle detection) has been archived. Numeric errors (NaN/Inf) are handled via propagation, not error returns (see §4.3).

---

## 7. Testing Strategy

### 7.1 Unit Tests

| Category | Coverage Target | Focus |
|----------|-----------------|-------|
| Indicators | ≥95% | Correctness, edge cases, NaN/Inf handling |
| Rolling Extrema | ≥90% | Deque vs naive equivalence, threshold behavior |
| CLI | ≥90% | Argument parsing, CSV I/O, error handling |

### 7.2 Property-Based Tests (proptest)

| Property | Indicators | Description |
|----------|------------|-------------|
| Output length | All | `output.len() == input.len()` (per §4.4) |
| NaN prefix | All | First `*_lookback()` positions are NaN |
| NaN propagation | All | NaN in window → NaN in output at that position |
| Range bounds | RSI, Stochastic | Output values in [0, 100] (excluding NaN) |
| Deque equivalence | Rolling Extrema | Naive and deque produce identical results for all periods |
| Edge case: RSI extremes | RSI | All gains → 100, all losses → 0 |
| Edge case: Stochastic denominator | Stochastic | high == low → %K = 50 |

### 7.3 Spec Fixture Tests (Authoritative)

Spec fixtures are hand-constructed test cases where expected outputs are derived directly from the initialization rules in §4.6 and the output shape contract in §4.4. These are the authoritative source of truth for liq-ta.

| Test Case | Purpose | Location |
|-----------|---------|----------|
| `spec_ema_sma_seed.json` | Verify EMA seeds with SMA of first period values | `tests/fixtures/` |
| `spec_rsi_wilder.json` | Verify RSI uses Wilder smoothing (α = 1/period) | `tests/fixtures/` |
| `spec_rsi_extremes.json` | All gains → 100, all losses → 0 | `tests/fixtures/` |
| `spec_atr_initialization.json` | First ATR = SMA of first period TRs | `tests/fixtures/` |
| `spec_stochastic_midpoint.json` | high == low → %K = 50 | `tests/fixtures/` |
| `spec_bollinger_collapse.json` | Constant series → bands collapse to mean | `tests/fixtures/` |
| `spec_macd_alignment.json` | Verify all outputs have identical length | `tests/fixtures/` |
| `spec_lookback_*.json` | Verify NaN count at start matches `*_lookback()` | `tests/fixtures/` |

**Fixture format**: JSON with:
```json
{
  "spec_version": "3.0",
  "input": [...],
  "params": {...},
  "expected": [...],
  "rationale": "..."
}
```

The `spec_version` field tracks which PRD version the fixture was created against and must not exceed the current PRD version. If initialization rules change in future versions, old fixtures are updated or marked deprecated. The `rationale` field documents why this specific expected output is correct per our specification.

### 7.4 Reference Comparison Tests (Informational)

TA-Lib reference comparisons are used to catch major deviations but are **non-blocking** in CI. These tests validate that liq-ta produces reasonable outputs for common cases but do not define correctness.

Golden files are stored in `crates/liq-ta/tests/golden/`:

| Indicator | Golden File | Tolerance | Notes |
|-----------|-------------|-----------|-------|
| SMA | `golden/sma_*.json` | 1e-10 relative | Typically close; sanity check |
| EMA | `golden/ema_*.json` | 1e-10 relative | Typically close; sanity check |
| RSI | `golden/rsi_*.json` | 1e-10 relative | Typically close; sanity check |
| MACD | `golden/macd_*.json` | 1e-10 relative | Typically close; sanity check |
| ATR | `golden/atr_*.json` | 1e-10 relative | Typically close; sanity check |
| Bollinger | `golden/bollinger_*.json` | 1e-10 relative | Divergence investigated |
| Stochastic | `golden/stochastic_*.json` | 1e-10 relative | Divergence investigated |

**Golden file format**: JSON with `{"input": [...], "params": {...}, "expected": [...]}`. Tests load via `serde_json` (dev-dependency only; output types do not require `serde` derives for golden tests).

**CI Behavior**: Reference comparison failures log warnings but do not fail the build. Use `--features reference-checks-strict` during development to treat divergences as errors for debugging purposes (not a supported compatibility mode).

### 7.5 Performance Tests

| Test Type | Purpose | Gate Type | Threshold |
|-----------|---------|-----------|-----------|
| Scaling | Verify O(n) complexity | **CI gate** | ≤11× from 10K→100K |
| Regression | Catch performance regressions | **CI gate** | ≤15% vs baseline |
| Absolute | Verify targets met | **Release checklist** | <10ms per indicator at 100K |

> **Note**: Absolute performance targets are measured on a reference machine (documented in `docs/BENCHMARK_BASELINE.md`) using `rustc` stable with default settings. CI gates are regression-only due to hardware variance across runners.

**Variance-Controlled Benchmarking** (recommended for releases):

For reliable performance measurements with low variance (CV < 10%):

```bash
# Hybrid approach: sequential groups, parallel indicators, multi-round execution
./scripts/run_benchmarks.sh
```

This approach:
- Reduces CPU contention vs full parallel (3-5x speedup vs sequential)
- Uses multi-round execution with cooldown for thermal stability
- Aggregates results using median/MAD (robust statistics)
- Detects outliers and reports quality metrics (Coefficient of Variation)

See [docs/benchmarking-guide.md](./benchmarking-guide.md) for complete methodology and troubleshooting.

### 7.6 User Validation Guidance

For users validating liq-ta outputs against their existing pipelines:

**Recommended Comparison Protocol**:

1. Run both implementations on identical input data
2. Use mixed tolerance comparison: `|a - b| <= 1e-10 + 1e-10 * max(|a|, |b|)`
3. Report max absolute error, max relative error, and percentage of points within tolerance
4. Investigate any outputs differing by >1e-6 as potential algorithm differences

**Expected Differences**:

| Indicator | Potential Difference | Cause |
|-----------|---------------------|-------|
| EMA | First few values | SMA seeding vs first-value seeding |
| RSI | Edge values (0, 100) | Indeterminate operation handling |
| Stochastic | Flat periods | %K=50 vs NaN for zero-range |
| Bollinger | Band width | Population vs sample stddev |

### 7.7 Real-World Regression Suite

Synthetic fixtures test edge cases, but realistic data tests robustness. We use deterministic synthetic data to avoid external dependencies while maintaining realistic market characteristics.

**Requirements**:

| Component | Description |
|-----------|-------------|
| **Dataset** | Deterministic synthetic OHLCV dataset (~750 rows) generated via seeded RNG |
| **Coverage** | Run all indicators with typical parameters |
| **Assertions** | No panics, no unexpected NaN blowups, output lengths match input, NaN prefix matches lookback |
| **CI Integration** | Part of standard test suite, runs on every PR |

**Dataset Criteria**:

- Generated deterministically from fixed seed (reproducible across runs)
- Contains realistic market regimes (uptrend, downtrend, range-bound, high volatility)
- Includes realistic features (gaps, wicks, volatility clusters)
- Small enough for fast CI (~750 rows)
- No external data dependencies (self-contained in test code)

**Implementation**: Uses `rand_chacha::ChaCha8Rng` with fixed seed `0xFA57_0000_2025` to generate consistent synthetic OHLCV data.

---

## 8. Reference Strategy

TA-Lib is used as a **reference check**, not a compatibility requirement. liq-ta defines its own specification (§4.4, §4.5, §4.6) and uses TA-Lib to validate that our outputs are reasonable for typical cases.

### 8.1 Design Philosophy

- **liq-ta owns its specification**: Initialization rules, output shapes, and edge cases are defined in this PRD
- **TA-Lib is inspiration, not gospel**: Where behaviors differ, liq-ta's documented behavior is correct
- **Reference checks catch regressions**: Large deviations from TA-Lib may indicate bugs worth investigating

### 8.2 Golden File Workflow

1. **Generation**: Python script (`tools/golden/generate.py`) generates reference outputs from TA-Lib
2. **Storage**: JSON files in `crates/liq-ta/tests/golden/`
3. **Validation**: Tolerance-based comparison (1e-10 relative error)
4. **CI Integration**: Golden files checked into repo; generator excluded from package

### 8.3 Handling Divergences

When liq-ta differs from TA-Lib:

| Scenario | Action |
|----------|--------|
| Edge case (constant prices, extremes) | Document in §4.5, add spec fixture |
| Initialization difference | Document in §4.6, add spec fixture |
| Unexpected divergence on normal data | Investigate as potential bug |
| Intentional algorithm improvement | Document rationale, update golden expectation |

### 8.4 Benefits

- Zero runtime dependency on TA-Lib
- CI-friendly testing (no TA-Lib installation required)
- Reproducible comparisons
- Clean separation of spec tests (authoritative) and reference tests (informational)

---

## 9. Dependencies

### 9.1 Core Dependencies (liq-ta)

| Dependency | Version | Purpose |
|------------|---------|---------|
| num-traits | 0.2.x | Float trait for generic numerics |
| thiserror | 1.x | Error derive macros |

### 9.2 CLI Dependencies (liq-ta-cli)

| Dependency | Version | Purpose |
|------------|---------|---------|
| clap | 4.x | CLI argument parsing |
| csv | 1.x | CSV reading/writing |
| liq-ta | workspace | Core indicator library |

### 9.3 Development Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| criterion | 0.5.x | Benchmarking framework |
| proptest | 1.x | Property-based testing |
| rand | 0.8.x | Random data generation |
| rand_chacha | 0.3.x | Deterministic RNG |
| serde | 1.x | Golden file serialization |
| serde_json | 1.x | Golden file I/O |

---

## 10. Deployment

### 10.1 Minimum Supported Rust Version

- **MSRV**: 1.90
- **Edition**: 2024
- **Toolchain**: Nightly (required for portable SIMD)

### 10.2 Platform Support

- All platforms supported by Rust stable
- No platform-specific code or dependencies

### 10.3 Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `std` | enabled | Standard library support |
| `serde` | disabled | Serialization support for output types |

**Dev-only flags** (not part of public API):

| Flag | Default | Description |
|------|---------|-------------|
| `reference-checks-strict` | disabled | Treat TA-Lib reference divergences as errors (debug aid, not a compatibility mode) |

---

## 11. Security Considerations

### 11.1 Input Validation

- All indicators validate input length against period
- NaN handling prevents unexpected behavior
- No unsafe code in core library

### 11.2 Dependency Audit

- Regular `cargo audit` for vulnerability scanning
- Minimal dependency footprint
- No network or filesystem access in core library

---

## 12. Future Work

### 12.1 Phase 2 Indicators (Planned)

| Indicator | Category | Priority |
|-----------|----------|----------|
| ADX | Trend | High |
| OBV | Volume | Medium |
| VWAP | Volume | Medium |
| CCI | Momentum | Medium |
| Williams %R | Momentum | Medium |
| Donchian Channels | Volatility | Medium |
| Parabolic SAR | Trend | Low |
| Ichimoku Cloud | Multi | Low |

### 12.2 v1.0 Requirements

| Feature | Description | Status |
|---------|-------------|--------|
| Python bindings | PyO3 + NumPy integration | **v1.0 blocker** |
| Tier 0 indicators | See §1.5 coverage table | In progress |
| Beta validation | ≥3 external users | Required |

### 12.3 Post-v1.0 Features (Planned)

| Feature | Description | Priority |
|---------|-------------|----------|
| Streaming API | Incremental indicator updates | Medium |
| SIMD optimization | AVX2/AVX-512 vectorization | Medium (defer until users need it) |
| C FFI | C-compatible ABI | Low |

### 12.4 Research (Archived)

The following were explored in experiments but are not planned for production:

| Feature | Status | Notes |
|---------|--------|-------|
| Plan mode execution | Archived | E07 showed 1.4-2.2× slower; code in `archive/experiments-v1` |
| Fusion kernels | Archived | E02-E03 showed slower than separate passes |
| GPU acceleration | Deferred | Insufficient workload size for GPU overhead

### 12.5 Interop Roadmap

Python bindings are the primary adoption vector for the quant/trading ecosystem. **Python is a v1.0 blocker.**

**v1.0: Python Bindings** (required for release):

| Component | Mechanism | Notes |
|-----------|-----------|-------|
| Core bindings | PyO3 | Direct Rust→Python without C intermediate |
| NumPy integration | numpy crate | Zero-copy where possible; accept/return `ndarray` |
| Package | maturin | Build and publish to PyPI as `liq-ta` |
| Scope | Tier 0 indicators | Minimal viable Python package |

A Rust-only library will not achieve adoption. `pip install liq-ta` must work before v1.0 ships.

**v2.x: C FFI** (deferred):

C-compatible ABI would enable R, Java, and other language bindings. Requirements TBD based on demand.

**Phase 3: Other Languages** (future):

R, Julia, and other bindings will be considered based on community interest post-v2.0.

---

## 13. Timeline

### 13.1 Completed Phases

| Phase | Duration | Status | Deliverables |
|-------|----------|--------|--------------|
| Phase 0: Foundation | 1 day | **COMPLETE** | Workspace, traits, data generators |
| Phase 1: Baseline Indicators | 2 days | **COMPLETE** | 7 indicators with ≥95% coverage |
| Phase 2: Kernels (E01-E04) | 2 days | **COMPLETE** | 3 fusion kernels, 4 experiment benchmarks |
| Phase 3: Plan (E05-E06) | 1 day | **COMPLETE** | DAG infrastructure, 2 experiment benchmarks |
| Phase 4: End-to-End (E07) | 1 day | **COMPLETE** | Direct/Plan modes, final benchmark |
| Phase 5: Documentation | 1 day | **COMPLETE** | PRD v1.6, experiment summary |

**Total Implementation Time**: ~8 days

### 13.2 Actual Complexity Assessment

| Component | Estimated | Actual | Notes |
|-----------|-----------|--------|-------|
| Indicator implementation | 1 day each | 0.5 days each | Faster with established patterns |
| Kernel development | 1 day each | 0.5 days each | Clear algorithms |
| Plan infrastructure | 2 days | 1.5 days | petgraph simplified DAG work |
| Benchmarking | 3 days | 2 days | Criterion setup reusable |
| Documentation | 2 days | 1 day | Structured report templates |

### 13.3 Pending Work

| Item | Dependency | Estimated Effort |
|------|------------|------------------|
| Remove archived code from main branch | Archive branch created | 2 hours |
| Generate initial golden files from TA-Lib | Python + TA-Lib installed | 4 hours |
| SIMD exploration for inner loops | Experimental | 2-3 days |

---

## 14. Success Metrics

### 14.1 Performance Metrics

| Metric | Target | Validation |
|--------|--------|------------|
| O(n) complexity | All indicators | E01 scaling analysis |
| Fusion benefit | N/A (invalidated) | E02, E03 benchmarks |
| Algorithm improvement | ≥5× for extrema | E04 benchmark |
| Plan mode benefit | N/A (invalidated) | E07 benchmark |

**Current Status**: E07 invalidated plan-mode speedup; direct mode is the performance baseline.

### 14.2 Quality Metrics

| Metric | Target | Tool |
|--------|--------|------|
| Test coverage (indicators) | ≥95% | cargo-tarpaulin |
| Test coverage (kernels) | ≥90% | cargo-tarpaulin |
| Clippy warnings | 0 | cargo clippy |
| Documentation coverage | 100% public items | cargo doc |

### 14.3 Outcome Metrics (v1.0 Success Criteria)

These measure whether liq-ta solves real user problems:

| Outcome | Target | Validation |
|---------|--------|------------|
| **Numerical trust** | Zero production discrepancies reported after 3 months | GitHub issues tracking |
| **Specification clarity** | Users can validate outputs against PRD without asking questions | Documentation feedback |
| **Performance satisfaction** | No user complaints about indicator computation speed | GitHub issues, beta feedback |
| **Adoption signal** | ≥1 credible open-source project using liq-ta | GitHub dependents |

**Beta Validation Requirements**:
- ≥3 external users run liq-ta against their backtesting pipelines
- Document and resolve all blocking issues before v1.0
- Collect qualitative feedback on specification clarity and API ergonomics

### 14.4 Adoption Metrics (Post-v1.0)

Adoption targets will be defined post-v1.0 release based on comparable libraries (e.g., `ta-rs`, `tulip`). Initial focus is on correctness and performance, not popularity metrics.

| Metric | Tracking |
|--------|----------|
| PyPI + Crates.io downloads | Tracked via package stats |
| GitHub stars | Tracked via GitHub |
| Issue response time | Target: <48h for critical bugs |
| Documentation completeness | All public items documented with examples |

---

## 15. Appendix

### 15.1 Glossary

| Term | Definition |
|------|------------|
| **EMA** | Exponential Moving Average - weighted moving average with exponential decay |
| **SMA** | Simple Moving Average - arithmetic mean over rolling window |
| **RSI** | Relative Strength Index - momentum oscillator (0-100 scale) |
| **MACD** | Moving Average Convergence Divergence - trend/momentum indicator |
| **ATR** | Average True Range - volatility indicator |
| **Bollinger Bands** | Volatility bands around moving average |
| **Stochastic** | Momentum oscillator based on price position within range |
| **Wilder Smoothing** | Exponential smoothing with α = 1/period |
| **Kernel Fusion** | Combining multiple computations into single data pass |
| **DAG** | Directed Acyclic Graph - dependency structure |

### 15.2 References

1. [TA-Lib Documentation](https://ta-lib.org/)
2. [Criterion.rs](https://bheisler.github.io/criterion.rs/book/)
3. [Welford's Algorithm](https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford%27s_online_algorithm)
4. [Monotonic Deque for Sliding Window](https://leetcode.com/problems/sliding-window-maximum/)
5. [proptest](https://docs.rs/proptest/) - Property-based testing

**Archived Research References** (no longer production dependencies):
- [petgraph](https://docs.rs/petgraph/) - Used in archived plan mode

### 15.3 Document History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | Dec 2025 | Initial PRD with performance hypotheses |
| 1.5 | Dec 2025 | Experiment implementation complete, benchmarks pending validation |
| 1.6 | Dec 2025 | Benchmarks finalized; recommendations updated based on results |
| 2.0 | Dec 2025 | **Architecture Decision**: Plan mode archived based on E07. Facade crate structure. Explicit numeric policy. Property-based testing. Golden file strategy retained. |
| 2.1 | Dec 2025 | **Specification Tightening**: Output shape contract (trimmed), indicator-specific edge cases, CLI contract, error enum cleanup, golden file location unified, CI vs release gates clarified. |
| 2.2 | Dec 2025 | **Specification Ownership**: Reframed TA-Lib as reference check, not compatibility requirement. Added §4.6 Indicator Initialization Rules. Split testing into spec fixtures (authoritative) vs reference comparisons (informational). Added lookback() functions. Added multi-output alignment rules. Added CLI NaN row semantics. |
| 2.3 | Dec 2025 | **Factoring Tightening**: Made lookback() canonical (table is documentation). Added multi-output `_into` buffer contracts. Added precedence rule for edge behaviors vs propagation. Moved CSV details to CLI README. Renamed feature flag to `reference-checks-strict`. Softened TA-Lib match expectations. |
| 2.4 | Dec 2025 | **Final Polish**: Added semver promise for lookback functions. Clarified multi-output logical time alignment. Added §4.8 Mathematical Conventions (determinism, tolerance, precision). Added §4.9 Non-Goals. (At the time: noted liq-ta-core was workspace-private; superseded by v2.5 merge.) Fixed performance claim to match benchmarks. Unified Config Types API terminology. |
| 2.5 | Dec 2025 | **Crate Simplification**: Merged liq-ta-core into liq-ta (single library crate). Updated file structure. Moved golden generator to tools/golden/generate.py. This is the idiomatic Rust pattern for library + CLI. |
| 2.6 | Dec 2025 | **Specification Clarity**: Added Stochastic variants documentation (Fast/Slow/Full). Documented all config type defaults. Rewrote NaN edge cases as "Indeterminate Operations". Added tolerance hierarchy explanation. Added fixture spec_version for versioning. Added Error Recovery Guidance table. Added Streaming Workaround section. Added population stddev note to Bollinger. Clarified archive branch deletion policy. Removed arbitrary adoption metrics. |
| 2.7 | Dec 2025 | **API Consistency & Tolerance Fix**: Unified Stochastic naming to `stochastic()` (fast-only in v2.x). Fixed tolerance policy with mixed abs+rel tolerances and split spec/reference thresholds. Made lookback table explicitly non-normative. Added `min_input_len` convention and `align_tail` helper. Added §5.5 Output Types with semver-stable struct schemas. Softened archive policy language. Removed environment-specific perf claims. Fixed v2.4 history to be explicitly historical. |
| 2.8 | Dec 2025 | **Consistency Cleanup**: Fixed k_slowing docs to acknowledge Slow Stochastic (k_slowing>1). Removed k=25 from pending work (already validated). Marked fusion benefit as invalidated in metrics. Clarified align_tail/min_input_len as patterns not API. Added trait implementations to output types. Added OBV/VWAP to Phase 2 indicators. Improved Stochastic examples. |
| 2.9 | Dec 2025 | **Product Positioning**: Added §1.4 Replacement Strategy with tiered coverage roadmap. Added §4.10 Stability Contracts. Added §5.6 API Layers (simple/buffer/advanced). Added §7.6 User Validation Guidance. Added §7.7 Real-World Regression Suite requirement. Added §12.4 Interop Roadmap (Python-focused). Promoted `*_min_len()` to stable API. Added explicit Stochastic formula with NaN precedence. Removed `PartialEq` from output types. Added benchmark conditions note. |
| 3.0 | Dec 2025 | **Product Strategy**: Added §1.1 Problem Statement (numerical stability, performance, specification opacity). Promoted Python bindings to v1.0 blocker. Added §14.3 Outcome Metrics with beta validation requirements. Restructured §12 with v1.0 Requirements vs Post-v1.0 Features. Updated completion gate to require Python + beta validation. |

---

*This document serves as the authoritative specification for the liq-ta project.*
