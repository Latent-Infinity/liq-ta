# Benchmarking Guide (fast-ta)

This guide explains how to run benchmarks effectively with variance control and proper statistical analysis.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Benchmark Architecture](#benchmark-architecture)
3. [Variance Control Strategy](#variance-control-strategy)
4. [Running Benchmarks](#running-benchmarks)
5. [Interpreting Results](#interpreting-results)
6. [Troubleshooting](#troubleshooting)
7. [Advanced Usage](#advanced-usage)

---

## Quick Start

### Run hybrid benchmarks with variance control (recommended)

```bash
# Default: 3 rounds, 60s cooldown between groups
./scripts/run_benchmarks.sh

# Custom configuration
ROUNDS=5 COOLDOWN=90 ./scripts/run_benchmarks.sh
```

### Run traditional single-round benchmarks

```bash
# All benchmarks (sequential)
cargo bench --bench talib_comparison

# Specific indicator
cargo bench --bench talib_comparison -- sma

# Specific size
cargo bench --bench talib_comparison -- "sma/100000"
```

---

## Benchmark Architecture

### Benchmark Suites

**1. `talib_comparison.rs`** (35 indicators)
- Compares fast-ta vs TA-Lib performance
- Default: 100,000 elements
- Purpose: Production performance validation

**2. `indicators.rs`** (18 indicators)
- Tests across multiple sizes: 100, 1k, 10k, 100k
- Default: period = 20
- Purpose: Complexity analysis (O(n) vs O(n×k))

### Configuration (criterion.toml)

```toml
warm_up_time = 5000        # 5s warmup
measurement_time = 10000   # 10s measurement (15s for slow)
sample_size = 500          # 500 iterations
noise_threshold = 0.02     # 2% noise threshold
confidence_level = 0.95    # 95% confidence interval
```

**Why sample_size = 500 is optimal:**
- Reduces random noise to ~0.09ns standard error
- Increasing to 5000 only improves to ~0.03ns (3× better precision, 10× slower)
- Systematic bias from parallel execution **cannot** be reduced by more samples
- Multi-round execution is more effective (see [Variance Control](#variance-control-strategy))

---

## Variance Control Strategy

### The Problem: Parallel Execution Introduces Systematic Bias

When running benchmarks in parallel on all 16 cores:

**❌ High CPU Contention**
- All cores compete for L3 cache
- Memory bandwidth saturation
- Thermal throttling (CPU frequency reduction)
- Power limit throttling (TDP constraints)

**Result**: Systematic bias (measurements are slower than actual performance)

### Why More Samples Don't Help

```
Sequential (true performance):
  Mean: 100ns ± 1ns  ✓ Correct

Parallel with 500 samples:
  Mean: 150ns ± 1ns  ✗ Wrong (50% slower due to contention)

Parallel with 5000 samples:
  Mean: 150ns ± 0.3ns  ✗ Still wrong (more precise, but still biased)
```

**Key insight**: Increasing sample size reduces **random noise**, not **systematic bias**.

### The Solution: Hybrid Approach with Multi-Round Execution

Our `run_benchmarks.sh` script implements:

**1. Hybrid Parallelism**
- Sequential groups (8 groups)
- Parallel indicators within each group (2-6 cores)
- Reduces contention while maintaining speedup

**2. Multi-Round Execution**
- 3 rounds by default (configurable)
- 60s cooldown between groups
- 120s cooldown between rounds
- Prevents thermal accumulation

**3. Robust Statistical Aggregation**
- Median (not mean) for central tendency
- MAD (Median Absolute Deviation) for variance
- Coefficient of Variation (CV) for quality assessment
- IQR outlier detection

### Expected Performance

| Approach | Speedup | Variance | Time | Quality |
|----------|---------|----------|------|---------|
| Sequential | 1x | Very Low | ~20 min | Excellent |
| Full Parallel (no variance control) | 5-10x | High (CV > 20%) | 2-3 min | Poor |
| **Hybrid (recommended)** | **3-5x** | **Low (CV < 10%)** | **5-7 min** | **Good** |

---

## Running Benchmarks

### Using the Hybrid Script (Recommended)

```bash
# Default configuration
./scripts/run_benchmarks.sh

# Custom rounds and cooldown
ROUNDS=5 COOLDOWN=90 ./scripts/run_benchmarks.sh

# Environment variables:
#   ROUNDS        - Number of rounds (default: 3)
#   COOLDOWN      - Seconds between groups (default: 60)
```

**What it does:**

1. Builds benchmarks
2. For each round:
   - Run group 1 (moving_averages): sma, ema, wma, dema, tema, trima
   - Cooldown 60s
   - Run group 2 (momentum): rsi, roc, mom, cmo, apo, trix
   - Cooldown 60s
   - ... (8 groups total)
   - Cooldown 120s before next round
3. Aggregate results with robust statistics
4. Generate quality report

**Output:**
- Criterion results: `target/criterion/`
- Aggregated results: `target/criterion/aggregated/`
- Quality report: printed to console

### Manual Aggregation

```bash
# If you ran benchmarks manually or want to re-aggregate
python3 scripts/aggregate_benchmarks.py \
    --results-dir target/criterion \
    --rounds 3 \
    --baseline-prefix round
```

---

## Interpreting Results

### Understanding the Aggregation Report

```
Benchmark                      Median          MAD       CV  Rounds  Outliers    Quality
------------------------------------------------------------------------------------------------
sma                          245.32 µs    12.15 µs    4.95%       3         0  ✓ Excellent
ema                          189.76 µs    18.23 µs    9.61%       3         0  Good
adx                          567.89 µs   125.34 µs   22.07%       3         1  ⚠ Poor
```

**Columns:**
- **Median**: Aggregated median time across rounds (robust central tendency)
- **MAD**: Median Absolute Deviation (robust variance measure)
- **CV**: Coefficient of Variation = (MAD / Median) × 100%
- **Rounds**: Number of rounds collected
- **Outliers**: Number of outlier rounds detected (IQR method)
- **Quality**: Assessment based on CV

### Quality Thresholds

| CV Range | Quality | Action |
|----------|---------|--------|
| **< 5%** | ✓ Excellent | Results are highly reliable |
| **5-10%** | Good | Results are acceptable |
| **10-20%** | Acceptable | Results usable but not ideal |
| **> 20%** | ⚠ Poor | **Investigate** (see [Troubleshooting](#troubleshooting)) |

### Comparing Against TA-Lib

Criterion automatically compares baselines:

```bash
# Run with new implementation
cargo bench --bench talib_comparison -- sma --save-baseline new

# Compare against previous
cargo bench --bench talib_comparison -- sma --baseline new

# Output shows:
#   change: -45.2% ✓ (faster)
#   change: +12.3% ✗ (slower)
```

---

## Troubleshooting

### High Variance (CV > 20%)

**Symptoms:**
- Aggregation report shows many "⚠ Poor" benchmarks
- Large MAD relative to median
- Many outliers detected

**Possible causes and solutions:**

#### 1. Thermal Throttling

**Check:**
```bash
# macOS
osx-cpu-temp

# Linux
sensors | grep temp
```

**Solutions:**
- Increase `COOLDOWN` (try 90s or 120s)
- Increase rounds and use median (filters thermal spikes)
- Ensure good laptop ventilation
- Run benchmarks in cooler environment

#### 2. CPU Contention (if running in parallel)

**Solutions:**
- Use the hybrid script (already reduces contention)
- Reduce parallel jobs in script (edit `run_benchmarks.sh`)
- Run purely sequential (slower but most stable):
  ```bash
  cargo bench --bench talib_comparison
  ```

#### 3. Background Processes

**Check:**
```bash
# macOS
top -o cpu

# Linux
htop
```

**Solutions:**
- Close browsers, IDEs, and other heavy applications
- Disable automatic updates
- Disable antivirus scans during benchmarking

#### 4. Insufficient Rounds

**Solution:**
```bash
# Increase rounds to 5 or 7
ROUNDS=5 ./scripts/run_benchmarks.sh
```

More rounds improve median robustness to outliers.

### Missing Results for Some Benchmarks

**Symptoms:**
- Aggregation script reports: "Warning: No results found for benchmark: xxx"

**Cause:**
- Benchmark failed during execution
- Wrong baseline prefix

**Solutions:**
1. Check benchmark logs for errors
2. Verify benchmark names:
   ```bash
   cargo bench --bench talib_comparison -- --list
   ```
3. Manually verify results directory:
   ```bash
   ls -la target/criterion/
   ```

### Python Script Errors

**Symptoms:**
```
ModuleNotFoundError: No module named 'statistics'
```

**Solution:**
Python 3.4+ required (statistics module is built-in). Update Python:
```bash
python3 --version  # Should be >= 3.4
```

---

## Advanced Usage

### Custom Benchmark Groups

Edit `scripts/run_benchmarks.sh` to reorganize groups:

```bash
GROUPS=(
    "fast_indicators:sma,ema,rsi"
    "slow_indicators:adx,stochastic,macd"
)
```

**Guidelines for grouping:**
- Group indicators with similar computation time
- Limit group size to 4-6 indicators (reduces contention)
- Separate CPU-intensive indicators across groups

### Running Single Group

```bash
# Manually run a specific group in parallel
echo "sma,ema,wma" | tr ',' '\n' | xargs -n1 -P3 -I{} \
    cargo bench --bench talib_comparison -- --exact "^{}\$"
```

### Comparing Different Optimizations

```bash
# Baseline (before optimization)
cargo bench --bench talib_comparison -- sma --save-baseline before

# Apply optimization...

# New version
cargo bench --bench talib_comparison -- sma --save-baseline after

# Compare
cargo bench --bench talib_comparison -- sma --baseline after
```

### Profiling Hot Paths

```bash
# Build release binary with debug symbols
cargo build --release --bench talib_comparison

# Profile with samply (macOS/Linux)
samply record ./target/release/deps/talib_comparison-* --bench

# Look for:
# - Cache misses (L1/L2/L3)
# - Branch mispredictions
# - Division operations
# - Memory allocations
```

### Criterion Command-Line Options

```bash
# Warm-up only (no measurement)
cargo bench -- --warm-up-time 10 --measurement-time 0

# Quick run (reduced samples)
cargo bench -- --sample-size 100

# Save HTML reports
cargo bench -- --plotting-backend plotters

# List all benchmarks
cargo bench -- --list

# Filter by pattern
cargo bench -- "sma.*100000"
```

---

## Benchmark Development Guidelines

### When Adding New Benchmarks

**1. Add to `talib_comparison.rs`** (if comparing with TA-Lib)

```rust
fn bench_my_indicator(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_indicator");

    for &size in SIZES.iter() {
        let data = generate_test_data(size);

        group.bench_with_input(
            BenchmarkId::new("fast_ta", size),
            &size,
            |b, _| {
                b.iter(|| {
                    my_indicator(&data, 20)
                });
            },
        );

        // TA-Lib comparison if available
        group.bench_with_input(
            BenchmarkId::new("ta_lib", size),
            &size,
            |b, _| {
                b.iter(|| {
                    talib::my_indicator(&data, 20)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_my_indicator);
```

**2. Add to `run_benchmarks.sh` groups**

Place in appropriate group based on indicator type:
- `moving_averages`: SMA-style indicators
- `momentum`: RSI, ROC-style indicators
- `volatility`: ATR, Bollinger-style indicators
- etc.

**3. Verify variance**

```bash
# Run hybrid benchmarks
./scripts/run_benchmarks.sh

# Check CV for your new benchmark
# Target: CV < 10%
```

### Benchmark Sizing

Use Fibonacci-like sequence for period testing:
```rust
const PERIODS: &[usize] = &[5, 8, 13, 21, 34, 55, 89];
```

**Why?**
- Reveals O(n) vs O(n×k) complexity differences
- Identifies algorithm crossover points
- Tests cache behavior at different window sizes

### Performance Regression Testing

Add to CI/CD:
```bash
# In .github/workflows/benchmark.yml
- name: Run benchmarks
  run: ./scripts/run_benchmarks.sh

- name: Check for regressions
  run: |
    python3 scripts/aggregate_benchmarks.py
    # Fail if any benchmark regressed > 10%
```

---

## Summary

### Key Recommendations

✅ **DO:**
- Use `./scripts/run_benchmarks.sh` for reliable results
- Keep `sample_size = 500` (optimal)
- Use multi-round execution for variance control
- Trust median/MAD over mean/stddev
- Aim for CV < 10%

❌ **DON'T:**
- Increase sample size to reduce parallel variance (doesn't work)
- Run full parallel without variance control (biased results)
- Use mean for aggregation (sensitive to outliers)
- Benchmark with heavy background processes
- Ignore thermal throttling

### Quick Reference

```bash
# Recommended workflow
./scripts/run_benchmarks.sh                    # Run benchmarks
python3 scripts/aggregate_benchmarks.py        # Aggregate (auto-runs)

# Check quality: target CV < 10%
# If CV > 20%, increase COOLDOWN or ROUNDS

# For development (fast iteration)
cargo bench --bench talib_comparison -- <indicator>

# For production validation
ROUNDS=5 COOLDOWN=90 ./scripts/run_benchmarks.sh
```

---

## References

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Sample Size Analysis](/tmp/sample_size_analysis.md) - Why more samples don't help
- [Optimization Approaches](./optimization-approaches.md) - Performance patterns
- [Product Requirements](./product-requirements.md) - Performance targets
