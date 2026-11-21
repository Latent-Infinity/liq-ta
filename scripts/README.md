# liq-ta Benchmark Scripts

From the workspace root, run the registered indicator suite sequentially:

```bash
./scripts/run_benchmarks.sh
./scripts/run_benchmarks.sh sma
./scripts/run_benchmarks.sh 'chop/period_10000/100000'
```

Arguments are passed to Criterion. See the [benchmarking guide](../docs/benchmarking-guide.md)
for baseline comparisons and measurement conditions. The older `benchmark.py`
TA-Lib comparison is retired because its benchmark target is unregistered; it
exits with a diagnostic instead of running a comparison.

`aggregate_benchmarks.py` and `compare_benchmarks.py` operate on historical
round/group baselines, which the current runner does not create.
