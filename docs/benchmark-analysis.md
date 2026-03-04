# liq-ta Performance Optimization Analysis

This document details the performance optimization work performed on liq-ta indicators
to bring underperforming indicators up to or exceeding TA-Lib benchmark performance.

## Overview

Based on the baseline measurements in [benchmark-baseline.md](./benchmark-baseline.md),
several indicators were identified as significantly slower than their TA-Lib equivalents.
This optimization effort applied proven patterns from already-optimized indicators
(CMO at 2.2x, APO at 1.79x, AROON at 1.87x) to the underperformers.

## Optimization Tiers

### Tier 1 Indicators (>30% slower than TA-Lib)

These indicators were prioritized for optimization due to significant performance gaps:

| Indicator   | Baseline | Target   | Optimization Applied                      |
|-------------|----------|----------|-------------------------------------------|
| Williams %R | 0.52x    | >=1.0x   | O(n) MonotonicDeque ring buffer           |
| MFI         | 0.50x    | >=1.0x   | On-the-fly typical price, no allocation   |
| SMA         | 0.69x    | >=0.95x  | Pre-computed 1/period reciprocal          |
| MIDPRICE    | 0.63x    | >=1.0x   | O(n) MonotonicDeque rolling extrema       |
| LINEARREG   | 0.67x    | >=0.95x  | Incremental rolling sums O(n)             |
| TSF         | 0.67x    | >=0.95x  | Shares LINEARREG core optimization        |

### Tier 2 Indicators (10-30% slower than TA-Lib)

Secondary priority optimizations for moderate performance gaps:

| Indicator | Baseline | Target   | Optimization Applied                      |
|-----------|----------|----------|-------------------------------------------|
| T3        | 0.73x    | >=1.0x   | Fused 6 EMA passes into single loop       |
| TRIX      | 0.77x    | >=1.0x   | Fused triple EMA + ROC                    |
| KAMA      | 0.79x    | >=1.0x   | Pre-computed abs changes, rolling sum     |
| DX        | 0.81x    | >=1.0x   | Direct +DI/-DI computation                |
| TRIMA     | 0.89x    | >=1.0x   | Pre-computed reciprocals for double-SMA   |
| MIDPOINT  | 0.90x    | >=1.0x   | O(n) MonotonicDeque rolling extrema       |
| AD        | 0.77x    | >=1.0x   | Optimized CLV formula                     |

## Optimization Patterns Applied

### Pattern 1: MonotonicDeque Ring Buffer (O(n) Rolling Extrema)

**Applied to:** Williams %R, MIDPRICE, MIDPOINT

**Before:** O(n x period) nested loop scanning for min/max in each window
```rust
for i in lookback..n {
    let mut max_val = data[i - period + 1];
    for j in (i - period + 2)..=i {
        if data[j] > max_val { max_val = data[j]; }
    }
}
```

**After:** O(n) amortized using monotonic deque
```rust
let mut deque: MonotonicDeque<T> = MonotonicDeque::new(period);
for i in 0..n {
    deque.push_max(i, data);
    if i >= lookback {
        let max_val = deque.get_extremum(data);
    }
}
```

**Benefit:** Each element is pushed/popped at most once, giving O(1) amortized per element.

---

### Pattern 2: Incremental Rolling Sums

**Applied to:** LINEARREG, TSF, KAMA

**Before:** O(n x period) recalculating sums for each window
```rust
for i in lookback..n {
    let mut sum_y = T::zero();
    for j in (i - period + 1)..=i {
        sum_y = sum_y + data[j];
    }
}
```

**After:** O(n) incremental update
```rust
let mut sum_y = initial_sum;
for i in (lookback + 1)..n {
    sum_y = sum_y - data[i - period] + data[i];
}
```

**Benefit:** Single-pass with O(1) update per element instead of O(period).

---

### Pattern 3: EMA Loop Fusion

**Applied to:** T3, TRIX

**Before:** Multiple passes with intermediate array allocations
```rust
let mut ema1 = vec![T::nan(); n];  // O(n) allocation
let mut ema2 = vec![T::nan(); n];  // O(n) allocation
let mut ema3 = vec![T::nan(); n];  // O(n) allocation

// 3 separate passes
for i in ema_lb..n { ema1[i] = alpha * data[i] + one_minus_alpha * ema1[i-1]; }
for i in start2..n { ema2[i] = alpha * ema1[i] + one_minus_alpha * ema2[i-1]; }
for i in start3..n { ema3[i] = alpha * ema2[i] + one_minus_alpha * ema3[i-1]; }
```

**After:** Single pass with scalar state variables
```rust
let mut ema1 = initial_ema1;
let mut ema2 = initial_ema2;
let mut ema3 = initial_ema3;

// Single fused pass
for i in lookback..n {
    ema1 = alpha * data[i] + one_minus_alpha * ema1;
    ema2 = alpha * ema1 + one_minus_alpha * ema2;
    ema3 = alpha * ema2 + one_minus_alpha * ema3;
    output[i] = compute_result(ema1, ema2, ema3);
}
```

**Benefit:** Eliminates heap allocations, improves cache locality, single pass through data.

---

### Pattern 4: Pre-computed Reciprocals

**Applied to:** SMA, TRIMA

**Before:** Division in hot loop
```rust
for i in lookback..n {
    output[i] = sum / period;  // Division is slow
}
```

**After:** Multiply by pre-computed reciprocal
```rust
let inv_period = T::one() / T::from_usize(period)?;
for i in lookback..n {
    output[i] = sum * inv_period;  // Multiplication is ~10x faster
}
```

**Benefit:** Multiplication is significantly faster than division on modern CPUs.

---

### Pattern 5: Allocation Elimination

**Applied to:** MFI, T3, TRIX, DX

**Before:** Intermediate arrays for values computed once
```rust
let mut typical_prices = vec![T::zero(); n];  // O(n) heap allocation
for i in 0..n {
    typical_prices[i] = (high[i] + low[i] + close[i]) / three;
}
// Later: use typical_prices[j]
```

**After:** Compute on-the-fly
```rust
for j in window_start..=i {
    let tp = (high[j] + low[j] + close[j]) / three;  // Compute when needed
    // Use tp directly
}
```

**Benefit:** Reduces memory pressure and improves cache efficiency.

---

### Pattern 6: Shared Computation

**Applied to:** DX (sharing with ADX pattern)

**Before:** Calling full ADX computation for DX values
```rust
let adx_result = adx(high, low, close, period)?;  // Computes ADX we don't need
let plus_di = &adx_result.plus_di;
let minus_di = &adx_result.minus_di;
```

**After:** Direct +DI/-DI computation using shared helper functions
```rust
// Same computation pattern as ADX but without the ADX smoothing step
let tr = compute_true_range(high[i], low[i], close[i-1]);
let (plus_dm, minus_dm) = compute_directional_movement(...);
// Compute DX directly from +DI and -DI
```

**Benefit:** Eliminates unnecessary ADX computation overhead.

## Files Modified

### Tier 1 Optimizations

| File | Optimization |
|------|--------------|
| `crates/liq-ta/src/indicators/williams_r.rs` | MonotonicDeque ring buffer |
| `crates/liq-ta/src/indicators/mfi.rs` | On-the-fly typical price |
| `crates/liq-ta/src/indicators/sma.rs` | Pre-computed reciprocal |
| `crates/liq-ta/src/indicators/midprice.rs` | MonotonicDeque rolling extrema |
| `crates/liq-ta/src/indicators/statistics.rs` | Incremental rolling sums |

### Tier 2 Optimizations

| File | Optimization |
|------|--------------|
| `crates/liq-ta/src/indicators/t3.rs` | Fused 6 EMA passes |
| `crates/liq-ta/src/indicators/trix.rs` | Fused triple EMA + ROC |
| `crates/liq-ta/src/indicators/kama.rs` | Pre-computed abs changes |
| `crates/liq-ta/src/indicators/dx.rs` | Direct DI computation |
| `crates/liq-ta/src/indicators/trima.rs` | Pre-computed reciprocals |
| `crates/liq-ta/src/indicators/midpoint.rs` | MonotonicDeque rolling extrema |
| `crates/liq-ta/src/indicators/ad.rs` | Optimized CLV formula |

## Complexity Improvements

| Indicator   | Before           | After  | Improvement Factor |
|-------------|------------------|--------|-------------------|
| Williams %R | O(n x period)    | O(n)   | period x          |
| MIDPRICE    | O(n x period)    | O(n)   | period x          |
| MIDPOINT    | O(n x period)    | O(n)   | period x          |
| LINEARREG   | O(n x period)    | O(n)   | period x          |
| TSF         | O(n x period)    | O(n)   | period x          |
| KAMA        | O(n x period)    | O(n)   | period x          |
| T3          | O(n) 7 passes    | O(n) 1 pass | ~7x cache efficiency |
| TRIX        | O(n) 4 passes    | O(n) 1 pass | ~4x cache efficiency |
| DX          | O(n) + overhead  | O(n)   | Reduced constant factor |
| SMA         | O(n) division    | O(n) multiply | ~10x per operation |
| TRIMA       | O(n) division    | O(n) multiply | ~10x per operation |
| AD          | O(n)             | O(n)   | Reduced operations |
| MFI         | O(n) + O(n) alloc| O(n)   | Reduced allocation |

## Reference Implementations

These well-optimized indicators served as patterns for the optimization work:

| Indicator | Performance | Key Technique |
|-----------|-------------|---------------|
| CMO       | 2.2x faster | Rolling window with pre-computed gains/losses |
| APO       | 1.79x faster | Efficient dual EMA loop fusion |
| AROON     | 1.87x faster | Hybrid dispatch (naive for small periods, deque for large) |
| Bollinger | 1.44x faster | SIMD-optimized variance calculation |
| MACD      | 1.42x faster | Fused triple-EMA computation |

## Verification Commands

```bash
# Run full test suite to verify correctness
cargo test --package liq-ta --release

# Run all benchmarks against TA-Lib
cargo bench --bench talib_comparison

# Run Tier 1 indicator benchmarks
cargo bench --bench talib_comparison -- 'williams_r|mfi|sma|midprice|linearreg|tsf'

# Run Tier 2 indicator benchmarks
cargo bench --bench talib_comparison -- 't3|trix|kama|dx|trima|midpoint|ad'

# Verify no regression in already-optimized indicators
cargo bench --bench talib_comparison -- 'cmo|apo|macd|bollinger'
```

## Regression Baselines

These indicators should not regress from their current performance:

| Indicator  | Expected Ratio | Minimum Acceptable |
|------------|----------------|-------------------|
| CMO        | ~2.2x          | >=2.0x            |
| APO        | ~1.79x         | >=1.6x            |
| MACD       | ~1.42x         | >=1.3x            |
| Bollinger  | ~1.44x         | >=1.3x            |
| AROON      | ~1.87x         | >=1.7x            |

## Notes

1. **Benchmarks are environment-dependent**: Results may vary based on CPU, system load,
   and compiler optimizations. Run benchmarks in a controlled environment.

2. **Period sensitivity**: Some optimizations (MonotonicDeque, incremental sums) show
   greater improvement for larger periods due to O(period) -> O(1) reduction.

3. **Memory vs. computation tradeoff**: Pre-computed arrays (like abs_changes in KAMA)
   trade O(n) memory for O(n x period) computation reduction.

4. **Cache efficiency**: Loop fusion (T3, TRIX) improves performance through better
   cache locality, even when algorithmic complexity remains the same.

## Latest Benchmark Results (500 samples, 100k elements)

*Updated: December 2024*

### liq-ta WINS (faster than TA-Lib)

| Indicator | liq-ta | TA-Lib | Speedup |
|-----------|---------|--------|---------|
| CMO (all periods) | 211-336µs | 475-621µs | **2.0-2.3x** |
| AROON p89 | 729µs | 1.87ms | **2.56x** |
| APO | 189µs | 324µs | **1.72x** |
| Bollinger | 411µs | 596µs | **1.45x** |
| CCI p233 | 12.8ms | 17.8ms | **1.39x** |
| MACD | 485µs | 671µs | **1.38x** |
| AROON p21 | 744µs | 964µs | **1.30x** |
| BOP | 61µs | 74µs | **1.21x** |
| RSI | 539µs | 641µs | **1.19x** |
| ADX | 607µs | 709µs | **1.17x** |
| AROON p55 | 709µs | 798µs | **1.13x** |
| ATR | 419µs | 464µs | **1.11x** |
| CCI p89 | 4.91ms | 5.39ms | **1.10x** |
| WMA | 191µs | 209µs | **1.09x** |
| Stochastic | 639µs | 686µs | **1.07x** |
| TEMA | 513µs | 549µs | **1.07x** |
| EMA | 177µs | 188µs | **1.06x** |
| CCI p55 | 2.56ms | 2.70ms | **1.05x** |

### TA-Lib WINS (needs optimization)

| Indicator | liq-ta | TA-Lib | Ratio | Priority |
|-----------|---------|--------|-------|----------|
| MOM | 14.1µs | 10.3µs | 0.73x | LOW |
| TRANGE | 52.5µs | 31.3µs | 0.59x | MEDIUM |
| Williams %R | 704µs | 434µs | 0.62x | HIGH |
| MFI | 273µs | 145µs | 0.53x | HIGH |
| MIDPRICE | 1.28ms | 705µs | 0.55x | HIGH |
| T3 | 327µs | 200µs | 0.61x | MEDIUM |
| SMA | 161-252µs | 115-157µs | 0.65x | MEDIUM |
| TSF | 1.11ms | 746µs | 0.67x | MEDIUM |
| AD | 199µs | 138µs | 0.69x | MEDIUM |
| VAR | 183-233µs | 137-145µs | 0.74x | MEDIUM |
| KAMA | 382µs | 281µs | 0.74x | MEDIUM |
| TRIX | 758µs | 608µs | 0.80x | LOW |
| DX | 789µs | 641µs | 0.81x | LOW |

### Summary

**Current standing: 18 wins, 13 losses**

**Recent Optimizations:**
- ⚠️ **MOM**: 0.21x → 0.73x (3.5x speedup) - SIMD `lagged_sub_sanitize_f64` kernel with fused infinity→NaN conversion
  - Note: IEEE 754 SIMD achieved 1.02x, but project policy requires INFINITY→NaN conversion which adds overhead
- ❌ **TRANGE**: Remains at 0.59x - SIMD attempted but reverted due to NaN/infinity handling overhead exceeding gains
  - IEEE 754 simd_max uses maxNum semantics (returns non-NaN), requiring explicit NaN propagation checks

**Key Learnings:**
- SIMD + NaN/infinity handling overhead can exceed SIMD gains for complex operations
- Simple operations (subtraction) benefit from fused SIMD+sanitization kernels
- Complex operations (max of 3 abs values) may not benefit from SIMD when NaN handling is required

Top remaining optimization targets:
1. **MFI** (0.53x) - Type conversion overhead in tight loop
2. **Williams %R** (0.62x) - Uses MonotonicDeque, needs deeper work
3. **MIDPRICE** (0.55x) - SIMD rolling max/min
4. **TRANGE** (0.59x) - Needs alternative approach (not SIMD)
5. **T3** (0.61x) - EMA loop fusion

## Related Documentation

- [benchmark-baseline.md](./benchmark-baseline.md) - Original performance baselines
- [rust-code-standards.md](./rust-code-standards.md) - Performance coding standards
- [indicator-standards.md](./indicator-standards.md) - Indicator implementation guidelines
