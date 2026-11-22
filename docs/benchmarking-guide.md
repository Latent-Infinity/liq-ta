# Benchmarking Guide (liq-ta)

Run these commands from the workspace root using the pinned toolchain in
`rust-toolchain.toml` (the workspace declares Rust 1.95 as its minimum).

## Indicator benchmarks

```bash
# Run the registered indicator suite sequentially
./scripts/run_benchmarks.sh

# Equivalent direct invocation
cargo bench --locked -p liq-ta --bench indicators

# Select a group, a size, or the large-period CHOP case
cargo bench --locked -p liq-ta --bench indicators -- sma
cargo bench --locked -p liq-ta --bench indicators -- 'sma/100000'
cargo bench --locked -p liq-ta --bench indicators -- 'chop/period_10000/100000'

# The wrapper forwards Criterion options and filters
./scripts/run_benchmarks.sh sma --save-baseline before
```

The indicator suite uses synthetic data at sizes 100, 1,000, 10,000 and
100,000; periods vary by indicator. CHOP also has a period-10,000 case at
100,000 elements. Standard groups use Criterion defaults; stochastic groups
configure a 5-second warmup, 15-second measurement and 500 samples in code.

Other registered suites are `workload`, `simd_comparison`,
`precision_baseline`, and `precision_comparison`; select one with
`--bench <name>`. The [historical baseline](benchmark-baseline.md) is not a
current performance claim.

## Comparing changes

```bash
# On the original revision
cargo bench --locked -p liq-ta --bench indicators -- sma --save-baseline before
# On the candidate revision, on the same machine and with the same settings
cargo bench --locked -p liq-ta --bench indicators -- sma --baseline before
```

Criterion stores results under `target/criterion/` and reports estimates and
confidence intervals. Compare like workloads and inspect variance rather than
assuming a fixed regression threshold. Record the source revision and local
changes, lockfile, exact Rust version, target triple, CPU, features, profile,
compiler flags, command, input and relevant environment settings with results.

Run benchmarks without concurrent tests or other intensive work. Separate
Criterion invocations running in parallel compete for CPU, cache and memory;
more samples cannot remove their systematic contention. Use a controlled machine,
manage thermal and frequency effects, and repeat runs when assessing changes.
Results on a shared machine should not be treated as a reliable performance gate.

On hybrid CPUs, which mix performance and efficiency cores, pin both revisions to
the same performance core, for example `taskset -c <cpu> cargo bench ...`. An
unpinned run can migrate between core types and swing by tens of percent. Running
the original revision again after the candidate (A/B/A) shows the run-to-run noise
to compare against.
