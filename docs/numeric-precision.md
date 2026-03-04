# Numeric Precision Guide

This guide explains liq-ta's precision modes and helps you choose the right configuration for your use case.

## Overview

liq-ta supports two precision modes for f32 input data:

| Mode | Description | Use Case |
|------|-------------|----------|
| **High** (default) | Uses f64 accumulators for f32 inputs | Production, accuracy-critical |
| **Fast** | Uses native f32 accumulators | Benchmarking, maximum throughput |

When using f64 input, both modes produce identical results since f64 is already the accumulator type.

## Setting Precision Mode

### Runtime Configuration

```rust
use liq_ta::precision::{set_precision_mode, PrecisionMode};

// Set to High precision (default)
set_precision_mode(PrecisionMode::High);

// Set to Fast mode (native accumulators)
set_precision_mode(PrecisionMode::Fast);
```

### Environment Variable

```bash
# Set to Fast mode
export LIQ_TA_PRECISION=fast

# Set to High mode (default)
export LIQ_TA_PRECISION=high
```

### Compile-Time Feature

```toml
# Cargo.toml - use Fast mode by default
[dependencies]
liq-ta = { version = "0.1", features = ["precision-fast"] }
```

### Precedence

Configuration is resolved in this order (highest to lowest):
1. Thread-local override (`with_precision_mode()`)
2. Runtime `set_precision_mode()`
3. Environment variable `LIQ_TA_PRECISION`
4. Cargo feature `precision-fast`
5. Built-in default: `High`

## When to Use Each Mode

### Use High Mode (Default)

- Production applications requiring consistent results
- Long time series (>1000 bars)
- Near-constant data (small variance relative to magnitude)
- Cumulative indicators (VWAP, OBV, AD)
- When precision matters more than the last few percent of performance

### Use Fast Mode

- Performance benchmarking
- Latency-critical applications where ~10% overhead matters
- Short time series (<100 bars)
- When you're already using f64 input data
- Development/testing with quick iteration

## Precision Expectations by Indicator

When comparing f32 High mode against pure f64 reference:

### Bounded Indicators (Absolute Tolerance)

| Indicator | Range | Tolerance |
|-----------|-------|-----------|
| RSI | 0-100 | 0.01 |
| Stochastic %K/%D | 0-100 | 0.01 |
| Williams %R | -100-0 | 0.01 |
| MFI | 0-100 | 0.01 |

These use absolute tolerance because relative error is meaningless near zero.

### Price-Scale Indicators (Hybrid Tolerance)

| Indicator | Relative Tol | Absolute Tol |
|-----------|--------------|--------------|
| SMA | 1e-5 | 1e-7 |
| Bollinger Bands | 1e-5 | 1e-7 |
| VWAP | 1e-5 | 1e-7 |
| VAR/STDDEV | 1e-5 | 1e-10 |
| ROC/ROCR100 | 2e-4 | 2e-5 |
| ROCP/ROCR | 1e-4 | 1e-6 |
| CCI | 1e-4 | 0.1 |
| OBV/AD | 1e-4 | 1.0 |

Hybrid rule: `diff <= abs_tol || diff <= rel_tol * |expected|`

## Performance Impact

Typical overhead of High mode vs Fast mode for f32 input:

| Indicator Type | Overhead |
|----------------|----------|
| Simple (SMA, Stochastic, ROC) | ~8-15% |
| Variance-based (Bollinger, VAR) | ~3-20% |
| Cumulative (VWAP, OBV, AD) | ~10-15% |
| Wilder smoothing (RSI, MFI) | ~12-15% |

These overheads are typically acceptable for production use.

## Troubleshooting Precision Issues

### Symptom: Results differ between runs

**Cause**: Floating-point non-associativity with parallel processing.

**Solution**: Use sequential processing or accept small variations.

### Symptom: Large errors with near-constant data

**Cause**: Catastrophic cancellation in variance calculations.

**Solution**:
1. Ensure High precision mode is active
2. Consider pre-scaling data (subtract mean before processing)
3. Use f64 input for maximum precision

### Symptom: Cumulative indicator drift over long series

**Cause**: Accumulated rounding errors in f32 sums.

**Solution**:
1. Ensure High precision mode is active
2. Use f64 input for series >10,000 bars
3. For VWAP, consider session-based resets

### Symptom: RSI/MFI drift from expected values

**Cause**: Wilder smoothing accumulates small errors.

**Solution**:
1. Ensure High precision mode is active
2. Use f64 input for very long series
3. Accept small differences (within 0.01 tolerance)

## Best Practices

1. **Default to High mode** - The overhead is small and the precision improvement significant.

2. **Use f64 for maximum precision** - If precision is critical, use f64 input directly. Both modes produce identical results with f64.

3. **Test with your data** - Run the precision validation suite with your actual data characteristics.

4. **Monitor for edge cases** - Near-zero denominators, very small ranges, and near-constant data are common precision pitfalls.

5. **Document your choice** - If you use Fast mode, document why and the expected precision impact.

## Testing Precision

Run the precision validation suite:

```bash
cargo test --test precision_validation -- --nocapture
```

Run the NaN propagation tests:

```bash
cargo test --test nan_propagation_precision
```

Run precision comparison benchmarks:

```bash
cargo bench -p liq-ta --bench precision_comparison
```

## Technical Details

### How High Mode Works

When `PrecisionMode::High` is active and input type is `f32`:

1. **Accumulators**: Rolling sums, sum-of-squares, and cumulative totals use `f64`
2. **Divisions**: Sensitive divisions (small denominators) performed in `f64`
3. **State**: Wilder smoothing state maintained in `f64`
4. **Output**: Final result converted back to `f32`

This approach maintains f32 input/output for SIMD efficiency while using f64 internally for numerical stability.

### Why Not Always Use f64?

- **Memory**: f64 uses 2x the memory of f32
- **SIMD**: f32 processes 2x as many elements per vector operation
- **Cache**: More f32 values fit in cache
- **Compatibility**: Many data sources provide f32 data

High mode provides a middle ground: f32 arrays for efficiency, f64 accumulators for precision.
