# Benchmark Scripts

This directory contains scripts for running benchmarks with variance control.

## Prerequisites

The benchmark script requires a `criterion.toml` symlink in the workspace root:

```bash
ln -sf crates/fast-ta/criterion.toml criterion.toml
```

This is needed so Criterion can find its configuration when running the pre-built benchmark binary.

## Scripts

### `run_benchmarks.sh`

Hybrid benchmark execution with multi-round support and thermal management.

**Usage:**
```bash
# Default: 3 rounds, 10s cooldown
./scripts/run_benchmarks.sh

# Custom configuration (longer cooldown for thermal management)
ROUNDS=5 COOLDOWN=60 ./scripts/run_benchmarks.sh
```

**Environment Variables:**
- `ROUNDS` - Number of benchmark rounds (default: 3)
- `COOLDOWN` - Cooldown between groups in seconds (default: 10)

**Features:**
- Sequential benchmark groups (8 groups)
- Parallel execution within groups (2-6 cores)
- Multi-round execution with cooldown
- Automatic result aggregation
- Progress tracking with CPU temperature monitoring (macOS)

**Output:**
- Criterion results: `target/criterion/`
- Aggregated results: `target/criterion/aggregated/`
- Quality report printed to console

### `compare_benchmarks.py`

Compare benchmark results between two baseline runs (before/after changes).

**Usage:**
```bash
# Compare old baseline (round) with new baseline (after)
python3 scripts/compare_benchmarks.py \
    --baseline-old round \
    --baseline-new after \
    --rounds 3
```

**Options:**
- `--baseline-old` - Old baseline prefix (before changes)
- `--baseline-new` - New baseline prefix (after changes)
- `--rounds` - Number of rounds for each baseline (default: 3)
- `--results-dir` - Criterion results directory (default: `target/criterion`)

**Output:**
- Comparison table showing old/new times and % change
- Summary: improved/neutral/regressed counts
- Status indicators:
  - ✓ Improved: >2% faster
  - ≈ Neutral: ±2% change
  - ✗ Regressed: >2% slower

### `aggregate_benchmarks.py`

Statistical aggregation of multi-round benchmark results.

**Usage:**
```bash
# Default: 3 rounds with "round" prefix
python3 scripts/aggregate_benchmarks.py

# Custom configuration
python3 scripts/aggregate_benchmarks.py \
    --rounds 5 \
    --baseline-prefix "test" \
    --results-dir target/criterion
```

**Options:**
- `--results-dir` - Criterion results directory (default: `target/criterion`)
- `--rounds` - Number of rounds to aggregate (default: 3)
- `--baseline-prefix` - Baseline name prefix (default: `round`)

**Features:**
- Median aggregation (robust central tendency)
- MAD (Median Absolute Deviation) for variance
- Coefficient of Variation (CV) quality metric
- IQR outlier detection
- Quality assessment (Excellent/Good/Acceptable/Poor)

**Output:**
- Console: Formatted report with quality metrics
- Files: `target/criterion/aggregated/{benchmark}.json`

## Benchmark Groups

Indicators are organized into 3 groups for high-parallelism execution (up to 12 cores):

1. **fast_indicators** (12): SMA, EMA, WMA, DEMA, TEMA, TRIMA, RSI, MOM, ROC, CMO, APO, TRIX
2. **simple_volume** (12): ATR, TRANGE, BOLLINGER, BOP, AD, OBV, MIDPOINT, MIDPRICE, VAR, TSF, LINEARREG, T3
3. **complex_indicators** (11): ADX, DX, AROON, CCI, MFI, WILLIAMS_R, STOCHASTIC, STOCHASTIC_FAST, MACD, KAMA, ULTOSC

This configuration maximizes core utilization (12 of 16 cores) while maintaining thermal headroom.

## Quality Metrics

The aggregation script reports Coefficient of Variation (CV) for each benchmark:

| CV Range | Quality | Interpretation |
|----------|---------|----------------|
| < 5% | ✓ Excellent | Highly stable measurements |
| 5-10% | Good | Acceptable variance |
| 10-20% | Acceptable | Usable but not ideal |
| > 20% | ⚠ Poor | Investigate thermal/contention issues |

## Comparing Before/After Changes

### Complete Workflow

**1. Run baseline benchmarks (before changes):**
```bash
# Run with default prefix "round"
./scripts/run_benchmarks.sh

# This creates: round1_*, round2_*, round3_*
```

**2. Make your code changes:**
```bash
# Edit indicator logic, optimizations, etc.
git add -A
git commit -m "Update indicator logic"
```

**3. Run benchmarks with new baseline name:**
```bash
# Use a different prefix for after changes
BASELINE_PREFIX="after" ./scripts/run_benchmarks.sh

# This creates: after1_*, after2_*, after3_*
```

**4. Compare results:**
```bash
python3 scripts/compare_benchmarks.py \
    --baseline-old round \
    --baseline-new after \
    --rounds 3
```

**Example Output:**
```
Benchmark                      Old          New       Change   %Change       Status
-------------------------------------------------------------------------------------
sma                        115.34 µs    113.20 µs    -2.14 µs    -1.86%  ✓ Improved
ema                        140.30 µs    140.50 µs    +0.20 µs    +0.14%  ≈ Neutral
adx                        452.86 µs    460.12 µs    +7.26 µs    +1.60%  ≈ Neutral
mfi                          8.09 ms      7.95 ms   -140.00 µs    -1.73%  ≈ Neutral

SUMMARY
-------------------------------------------------------------------------------------
Total benchmarks: 35
  Improved (>2% faster):    5 (14.3%)
  Neutral (±2%):           28 (80.0%)
  Regressed (>2% slower):   2 (5.7%)
```

### Alternative: Individual Benchmark Comparison

For quick checks without re-running full suite:

```bash
# Compare single indicator
cargo bench --bench talib_comparison -- sma --baseline round1_moving_averages

# Compare multiple indicators
cargo bench --bench talib_comparison -- "^(sma|ema|wma)/" --baseline round1_moving_averages
```

Criterion shows inline comparison:
```
sma/fast-ta/100000  time:   [113.20 µs 113.34 µs 113.48 µs]
                    change: [-2.12% -1.89% -1.65%] (p = 0.00 < 0.05)
                    Performance has improved.
```

## Troubleshooting

### High Variance (CV > 20%)

**Causes:**
- Thermal throttling
- CPU contention
- Background processes
- Insufficient rounds

**Solutions:**
- Increase `COOLDOWN` (try 90s or 120s)
- Increase `ROUNDS` (try 5 or 7)
- Close heavy applications
- Ensure good laptop ventilation

### Missing Results

**Symptoms:**
```
Warning: No results found for benchmark: xxx
```

**Solutions:**
1. Check benchmark logs for errors
2. Verify benchmark names: `cargo bench --bench talib_comparison -- --list`
3. Check results directory: `ls -la target/criterion/`

## Documentation

See [../docs/benchmarking-guide.md](../docs/benchmarking-guide.md) for comprehensive benchmarking methodology, including:
- Variance control strategy
- Result interpretation
- Comparison with TA-Lib
- Advanced usage patterns
- Performance regression testing
