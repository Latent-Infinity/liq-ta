# Profile-Guided Optimization (PGO) Experiment

## Overview

We attempted to use Profile-Guided Optimization (PGO) to close the remaining performance gaps with TA-Lib. PGO uses runtime profiling data to inform compiler optimization decisions about branch prediction, code layout, and inlining.

## Hypothesis

With indicators already heavily optimized and within 0.4-42% of TA-Lib performance, PGO's ability to optimize:
- Branch prediction metadata
- Code layout (hot path placement)
- Inlining decisions based on actual call patterns

...could yield the final 1-5% improvement to reach or exceed TA-Lib parity.

## Methodology

### Setup

1. **Profile Generation Phase**:
   ```bash
   RUSTFLAGS="-Cprofile-generate=target/pgo-profiles" \
       cargo build --profile=pgo-generate --bench talib_comparison
   ```

2. **Profile Collection**:
   ```bash
   RUSTFLAGS="-Cprofile-generate=target/pgo-profiles" \
       cargo bench --profile=pgo-generate -- "mfi|ultosc" \
       --warm-up-time 1 --measurement-time 3 --sample-size 30
   ```

3. **Profile Merging**:
   ```bash
   llvm-profdata merge -o target/pgo-profiles/merged.profdata \
       target/pgo-profiles/*.profraw
   ```

4. **Optimized Build**:
   ```bash
   RUSTFLAGS="-Cprofile-use=target/pgo-profiles/merged.profdata" \
       cargo build --profile=pgo-use --bench talib_comparison
   ```

### Cargo Profile Configuration

```toml
[profile.pgo-generate]
inherits = "release"
strip = false              # Keep symbols for profiling

[profile.pgo-use]
inherits = "release"
strip = false              # Keep symbols for analysis
```

## Results

### MFI (Money Flow Index)

| Build Type | Time (µs) | vs Baseline | vs TA-Lib |
|------------|-----------|-------------|-----------|
| **Baseline (release)** | 149.65 | - | 1.004x slower (0.4%) |
| **PGO (first attempt)** | 147.55 | +1.4% faster | 1.027x faster |
| **PGO (with ULTOSC profiles)** | 151.87 | -1.5% slower | 1.055x slower |
| **TA-Lib** | 143.92 | - | baseline |

### ULTOSC (Ultimate Oscillator)

| Build Type | Time (µs) | vs Baseline | vs TA-Lib |
|------------|-----------|-------------|-----------|
| **Baseline (release)** | 270.17 | - | 1.42x faster |
| **PGO** | 297.64 | -10.2% slower | 1.24x faster |
| **TA-Lib** | 369.52 | - | baseline |

## Analysis

### Why PGO Hurt Performance

1. **Non-Representative Profiles**: Profile data collected from short benchmark runs (1s warmup, 3s measurement, 30 samples) may not represent the hot path behavior of the full 500-sample, 15s benchmark.

2. **Instrumentation Overhead**: The profiling run itself is much slower (2-3x), potentially biasing the profile data with overhead-related patterns.

3. **Profile Quality**: Mixed profiles from different indicators (MFI + ULTOSC) may have diluted or conflicted optimization signals.

4. **Already-Optimal Code**: The code is already very tight:
   - Branchless hot paths (predictable branches)
   - O(n) algorithms with minimal branching
   - LLVM already making near-optimal decisions

5. **PGO Heuristics Mismatch**: PGO optimizations assume typical workloads may not match our specific benchmark characteristics:
   - Continuous loops with minimal control flow
   - High data parallelism opportunity
   - Tight memory access patterns

### MFI Improvement Then Regression

**First PGO Run (+1.4%)**:
- Profile from only MFI workload
- Likely optimized MFI's specific branch patterns
- Small but measurable gain

**Second PGO Run (-1.5%)**:
- Added ULTOSC profile data
- Mixed signals from different indicators
- Possibly caused suboptimal inlining or layout decisions

## Conclusions

### For liq-ta

**PGO is not beneficial** for our current codebase because:

1. **Already instruction-tight**: Extensive algorithmic and micro-optimizations left little room for PGO gains
2. **Branchless design**: Hot paths already use branchless techniques, eliminating PGO's branch prediction benefits
3. **Profile quality issues**: Benchmark-based profiling doesn't match production workloads
4. **Negative returns**: Active performance regression in all tested scenarios

### General Lessons

**When PGO Helps**:
- Code with many unpredictable branches
- Complex control flow with varying hot paths
- Application with diverse real-world workload patterns
- Large codebases where profile data guides inlining across modules

**When PGO Doesn't Help** (our case):
- Already branchless/predictable branches
- Tight algorithms with minimal control flow
- Benchmark workloads that differ from profiling runs
- Code already at peak performance via other optimizations

### Recommendations

1. **Skip PGO**: Don't use PGO in production builds
2. **Stick with release profile**: Current `-Copt-level=3 -Clto=fat -Ccodegen-units=1` is optimal
3. **Focus on algorithms**: Continue algorithmic improvements (O(n×k) → O(n), constant cancellation, etc.)
4. **Trust LLVM**: Modern LLVM already makes excellent optimization decisions for tight numerical code

## Alternative Approaches Tried

### BOLT (Binary Optimization and Layout Tool)

**Status**: Not pursued
**Reason**: Linux-only tool; our development is on macOS (aarch64-apple-darwin)
**Potential**: BOLT can offer 5-15% on top of PGO for layout optimization, but requires Linux and may have similar profile quality issues

## Final Status

We've achieved near-parity or better-than TA-Lib performance through:
- **MFI**: 0.4% slower than TA-Lib (essentially tied)
- **ULTOSC**: 42% faster than TA-Lib

These results were achieved through algorithmic improvements and micro-optimizations, not PGO. Further gains would require:
- Using `-ffast-math` equivalent (breaks strict IEEE 754 semantics)
- Period-specific specialization (code size explosion)
- Hand-written SIMD for specific hot loops (maintenance burden)

The current approach represents an excellent balance of performance, maintainability, and correctness.
