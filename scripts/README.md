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

Indicators are organized into 8 groups for hybrid execution:

1. **moving_averages**: SMA, EMA, WMA, DEMA, TEMA, TRIMA
2. **momentum**: RSI, ROC, MOM, CMO, APO, TRIX
3. **volatility**: ATR, TRANGE, BOLLINGER
4. **trend**: ADX, DX, AROON
5. **oscillators**: CCI, MFI, WILLIAMS_R, BOP, ULTOSC
6. **stochastic**: STOCHASTIC, STOCHASTIC_FAST
7. **volume_price**: AD, OBV, MIDPOINT, MIDPRICE
8. **advanced**: VAR, TSF, LINEARREG, KAMA, T3, MACD

## Quality Metrics

The aggregation script reports Coefficient of Variation (CV) for each benchmark:

| CV Range | Quality | Interpretation |
|----------|---------|----------------|
| < 5% | ✓ Excellent | Highly stable measurements |
| 5-10% | Good | Acceptable variance |
| 10-20% | Acceptable | Usable but not ideal |
| > 20% | ⚠ Poor | Investigate thermal/contention issues |

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
