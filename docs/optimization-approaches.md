# Optimization Approaches (fast-ta)

This document describes optimization strategies for technical analysis indicators in fast-ta, with a focus on rolling window extrema algorithms.

## Overview

fast-ta employs multiple algorithmic approaches for computing rolling min/max operations, selecting the optimal strategy based on dataset size and access patterns. The primary approaches are:

| Approach | Time Complexity | Space Complexity | Best For |
|----------|-----------------|------------------|----------|
| **MonotonicDeque** | O(n) | O(k) | Small datasets, streaming |
| **VHGW** | O(3n) | O(4n) | Large datasets, batch SIMD |
| **Naive inline** | O(n×k) | O(1) | Very small periods |

## Van Herk/Gil-Werman (VHGW) Algorithm

### Algorithm Description

The Van Herk/Gil-Werman algorithm computes rolling min/max in O(3n) time with O(n) space using prefix-suffix decomposition. This produces SIMD-friendly memory access patterns ideal for batch processing workloads.

**Algorithm Steps:**

1. **Divide**: Split data into blocks of size K (the window period)
2. **Forward pass**: Compute prefix extrema within each block
3. **Backward pass**: Compute suffix extrema within each block
4. **Combine**: For window [i-K+1, i], result = extrema(suffix[i-K+1], prefix[i])

### Implementation Pattern

```rust
/// Van Herk/Gil-Werman SIMD algorithm for rolling extrema.
///
/// Uses prefix-suffix blocks for sliding max/min, which vectorizes extremely well.
/// This is a three-pass algorithm but each pass is SIMD-friendly.
fn rolling_max_vhgw<T: SeriesElement>(
    data: &[T],
    period: usize,
) -> Result<Vec<T>> {
    let n = data.len();

    // Allocate working buffers for prefix/suffix extrema
    let mut prefix = vec![T::nan(); n];
    let mut suffix = vec![T::nan(); n];

    // Pass 1: Forward scan (prefix extrema within blocks)
    for block_start in (0..n).step_by(period) {
        let block_end = (block_start + period).min(n);
        prefix[block_start] = data[block_start];
        for i in (block_start + 1)..block_end {
            prefix[i] = prefix[i - 1].max(data[i]);
        }
    }

    // Pass 2: Backward scan (suffix extrema within blocks)
    for block_start in (0..n).step_by(period) {
        let block_end = (block_start + period).min(n);
        suffix[block_end - 1] = data[block_end - 1];
        for i in (block_start..block_end - 1).rev() {
            suffix[i] = suffix[i + 1].max(data[i]);
        }
    }

    // Pass 3: Combine prefix and suffix for rolling max
    let mut output = vec![T::nan(); n];
    for i in (period - 1)..n {
        let left = i - period + 1;
        output[i] = suffix[left].max(prefix[i]);
    }

    Ok(output)
}
```

### Trade-offs vs MonotonicDeque

| Aspect | MonotonicDeque | VHGW |
|--------|---------------|------|
| Time Complexity | O(n) | O(3n) |
| Space | O(K) | O(4n) |
| Memory Access | Random (deque ops) | Sequential (SIMD) |
| Small Dataset | Better | Worse (overhead) |
| Large Dataset | Good | Better (SIMD) |
| Streaming | Excellent | Poor |
| Batch Processing | Good | Excellent |

### Threshold Dispatch

VHGW optimization is applied when `n >= 1000` elements based on empirical benchmarking:

```rust
pub const VHGW_DISPATCH_THRESHOLD: usize = 1000;

pub fn rolling_max<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    if data.len() >= VHGW_DISPATCH_THRESHOLD {
        rolling_max_vhgw(data, period)  // SIMD-friendly path
    } else {
        rolling_max_deque(data, period)  // MonotonicDeque path
    }
}
```

| Dataset Size | Winner | Reason |
|--------------|--------|--------|
| n < 100 | MonotonicDeque | VHGW setup cost dominates |
| 100 <= n < 1000 | MonotonicDeque | Comparable, simpler algorithm |
| n >= 1000 | VHGW | SIMD vectorization benefits |
| n >= 10000 | VHGW (strong) | Clear SIMD advantage |
| n >= 100000 | VHGW + parallel | Consider rayon for blocks |

## VHGW Applicable Indicators

### Investigation Results

Audit conducted December 2025 identified VHGW optimization candidates:

| Indicator | File | Current Impl | VHGW Applicable | Expected Improvement |
|-----------|------|--------------|-----------------|---------------------|
| **Stochastic** | `stochastic.rs` | O(n×k) naive | Yes | 50-60% |
| **MIDPOINT** | `midpoint.rs` | O(n×k) naive | Yes | ~50% |
| **MIDPRICE** | `midprice.rs` | O(n×k) naive | Yes | ~50% |
| **StochRSI** | `stochrsi.rs` | O(n×k) naive | Yes | ~40% |
| **Williams %R** | `williams_r.rs` | MonotonicDeque | Yes | 30-40% |
| **Donchian** | `donchian.rs` | MonotonicDeque | Yes | 30-40% |
| **Aroon** | `aroon.rs` | Index tracking | **No** | N/A |

### Pattern Matching

Indicators matching the VHGW pattern `y[i] = g(x[i], max(window), min(window))`:

- **Stochastic**: `%K = 100 × (Close - LL) / (HH - LL)`
- **Williams %R**: `%R = -100 × (HH - Close) / (HH - LL)`
- **Donchian**: `Upper = HH, Lower = LL, Middle = (HH + LL) / 2`
- **MIDPOINT**: `(MAX + MIN) / 2`
- **MIDPRICE**: `(HH + LL) / 2`
- **StochRSI**: `(RSI - min_RSI) / (max_RSI - min_RSI)`

### Excluded Indicators

| Indicator | Reason |
|-----------|--------|
| Aroon | Needs argmax/argmin (index tracking), not just values |
| RSI | EMA-based, no rolling extrema |
| MACD | EMA-based, no rolling extrema |
| ATR | EMA-based, no rolling extrema |
| Bollinger | Variance-based, uses Welford accumulators |
| SMA/EMA/WMA | Moving average, ring buffer pattern |

## Performance Expectations

### Current vs Expected Performance (100K Elements)

| Indicator | Current | Expected | Improvement | Notes |
|-----------|---------|----------|-------------|-------|
| Stochastic | 1.50 ms | ~0.6-0.75 ms | **50-60%** | Currently 2.3x slower than TA-Lib |
| MIDPOINT | 435 µs | ~220 µs | ~50% | Already 1.4x faster than TA-Lib |
| MIDPRICE | 446 µs | ~225 µs | ~50% | Near parity with TA-Lib |
| StochRSI | ~1.5 ms (est) | ~0.9 ms | ~40% | No current benchmark |
| Williams %R | 681 µs | ~450 µs | ~35% | MonotonicDeque baseline |
| Donchian | 664 µs | ~430 µs | ~35% | MonotonicDeque baseline |

### Proven Speedups

Williams %R VHGW optimization demonstrated **35-52% speedup** at 100K elements compared to naive implementation. Stochastic family indicators, currently using O(n×k) naive inline loops, are expected to see the largest gains (50-60%) due to both algorithmic improvement and SIMD benefits.

## NaN Handling

VHGW implementation supports two NaN handling modes:

### Skip Mode (Default)

- NaN values are skipped in extrema calculation
- Equivalent to MonotonicDeque behavior
- Used by: Williams %R, Donchian

### Propagate Mode

- Any NaN in window propagates to output
- Uses left_valid/right_valid tracking
- Used by: Stochastic, StochRSI

```rust
// Skip mode (default)
pub fn rolling_max_vhgw<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>>;

// Propagate mode
pub fn rolling_max_vhgw_nan_propagating<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>>;
```

## Implementation Roadmap

### Priority Order

| Priority | Task | Impact | Effort |
|----------|------|--------|--------|
| **P0** | VHGW Kernel in rolling_extrema.rs | All indicators | 2-3 days |
| **P1** | Stochastic Kernel Migration | Highest (50%+) | 1 day |
| **P2** | MIDPOINT Kernel Migration | High (50%+) | 0.5 day |
| **P3** | MIDPRICE Kernel Migration | High (50%+) | 0.5 day |
| **P4** | StochRSI Kernel Migration | High (40%+) | 1 day |
| **P5** | f64 Type Specialization | +10-15% | 1 day |

**Total Estimated Effort**: 7-9 days

### Dependencies

1. **P0 (Kernel)** must complete first - enables automatic optimization for kernel consumers
2. **P1-P4 (Migrations)** can run in parallel after P0
3. **P5 (Specialization)** builds on P0 foundation
4. Williams %R and Donchian auto-benefit from P0 (no code changes needed)

## API Design

### Core Functions

```rust
// Primary VHGW functions
pub fn rolling_max_vhgw<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>>;
pub fn rolling_min_vhgw<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>>;
pub fn rolling_extrema_fused_vhgw<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize
) -> Result<RollingExtremaOutput<T>>;

// Workspace reuse for repeated calculations
pub struct VhgwWorkspace<T> { /* ... */ }
pub fn rolling_extrema_with_workspace<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
    workspace: &mut VhgwWorkspace<T>
) -> Result<RollingExtremaOutput<T>>;
```

### Memory Management

| API Variant | Allocations | Use Case |
|-------------|-------------|----------|
| Standard | Output + 2n internal | Single computation |
| `*_into` | 2n internal | Pre-allocated output buffer |
| `*_with_workspace` | Output only | Repeated calculations |
| `*_with_workspace_into` | None | Maximum performance |

### Backward Compatibility

| Aspect | Guarantee |
|--------|-----------|
| Function Signatures | Unchanged |
| Return Types | Unchanged |
| Error Types | Unchanged |
| NaN Handling (default) | Unchanged - skip semantics match MonotonicDeque |
| Output Values | Bit-for-bit identical results |

## References

- Van Herk, M. (1992). "A fast algorithm for local minimum and maximum filters on rectangular and octagonal kernels"
- Gil, J., & Werman, M. (1993). "Computing 2-D min, median, and max filters"
- Investigation Report: `.auto-claude/specs/009-optimize-rolling-extrema-indicators-with-vhgw-algo/INVESTIGATION_REPORT.md`

## Document History

| Date | Version | Changes |
|------|---------|---------|
| 2025-12-30 | 1.0 | Initial creation from VHGW investigation findings |
