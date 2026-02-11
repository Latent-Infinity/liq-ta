# ADR-001: TA-Lib Comparison Strategy

**Status:** Accepted
**Date:** 2024-12-20
**Decision Makers:** liq-ta Development Team

## Context

The liq-ta library aims to provide high-performance technical analysis indicators in Rust. To validate our implementation correctness and benchmark performance claims, we need a reliable comparison with TA-Lib, the de-facto standard library for technical analysis.

We need to determine the best strategy for:
1. **Correctness Validation**: Ensuring our indicator outputs match TA-Lib's reference implementation
2. **Performance Benchmarking**: Comparing execution speed between liq-ta and TA-Lib

This decision impacts Experiment E07 (End-to-End Comparison) and overall project validation.

## Options Considered

### Option 1: Python Subprocess

**Description:** Use Python's `ta-lib` wrapper via subprocess calls during benchmark runs.

**Pros:**
- Easy to implement using standard library subprocess
- TA-Lib Python wrapper is well-maintained and widely used
- No need for C/FFI complexity
- Works in CI environments with TA-Lib installed

**Cons:**
- Subprocess overhead adds latency to benchmark measurements
- Python interpreter startup time skews results
- Data serialization overhead (Rust → JSON/CSV → Python → TA-Lib)
- Difficult to measure pure TA-Lib computation time accurately
- Requires Python and ta-lib package in build environment

### Option 2: Golden Files (Pre-computed Reference Data)

**Description:** Generate reference outputs from TA-Lib offline and store them as golden files. Compare liq-ta outputs against these stored reference values.

**Pros:**
- Zero runtime dependency on TA-Lib or Python
- Fastest test execution - pure file reads
- Deterministic comparisons
- Works in any environment without additional setup
- Clean separation between correctness validation and performance benchmarking
- Simpler CI configuration

**Cons:**
- Requires regenerating golden files if test data changes
- Storage overhead for pre-computed results
- Cannot dynamically test with random data (limited to pre-defined datasets)
- Separate tooling needed to generate golden files initially

### Option 3: FFI (Foreign Function Interface)

**Description:** Directly call TA-Lib C library functions via Rust FFI bindings.

**Pros:**
- Most accurate performance comparison (no interpretation overhead)
- Can use identical data buffers for both implementations
- True apples-to-apples benchmark comparison
- Full access to all TA-Lib functions

**Cons:**
- Requires TA-Lib C library installation (platform-specific)
- Complex FFI setup and unsafe code blocks
- Build system complexity (linking, platform differences)
- CI/CD requires TA-Lib native library on all platforms
- Maintenance burden for FFI bindings
- Memory safety concerns at the FFI boundary

## Decision

**Chosen Approach: Golden Files (Option 2)**

We will use the **Golden Files** approach for TA-Lib comparison with the following implementation:

### Implementation Details

1. **Golden File Generation (One-time Setup)**
   - Create a Python script that generates reference outputs using `ta-lib`
   - Run against standardized test datasets (1K, 10K, 100K points)
   - Store results in `benches/golden/` directory as JSON files
   - Include metadata: TA-Lib version, generation date, parameters

2. **Correctness Validation**
   - Unit tests compare liq-ta outputs against golden files
   - Tolerance-based comparison for floating-point values (1e-10 relative error)
   - Full coverage of all 7 baseline indicators

3. **Performance Benchmarking Strategy**
   - Benchmark liq-ta in isolation using Criterion (accurate Rust measurements)
   - Document TA-Lib reference timings from published benchmarks
   - For E07, focus on measuring our implementation's absolute performance
   - Report comparative speedups based on documented TA-Lib baselines

### Directory Structure

```
benches/
  golden/
    sma.json          # Golden outputs for SMA
    ema.json          # Golden outputs for EMA
    rsi.json          # Golden outputs for RSI
    macd.json         # Golden outputs for MACD
    atr.json          # Golden outputs for ATR
    bollinger.json    # Golden outputs for Bollinger Bands
    stochastic.json   # Golden outputs for Stochastic
    metadata.json     # TA-Lib version, generation info
tools/
  generate_golden.py  # Script to regenerate golden files
```

### Golden File Format

```json
{
  "indicator": "SMA",
  "parameters": { "period": 14 },
  "talib_version": "0.4.32",
  "generated_at": "2024-12-20T00:00:00Z",
  "test_cases": [
    {
      "name": "random_walk_1k",
      "input_file": "test_data/random_walk_1k.json",
      "seed": 42,
      "output": [null, null, ..., 100.234, 100.456, ...]
    }
  ]
}
```

## Rationale

1. **Simplicity over Complexity**: FFI introduces significant complexity with minimal benefit for our validation goals. We don't need real-time TA-Lib calls - pre-computed reference values serve validation equally well.

2. **CI/CD Friendliness**: Golden files work everywhere without external dependencies. No need to install TA-Lib, Python, or configure platform-specific linking.

3. **Separation of Concerns**: Correctness validation (comparing outputs) is orthogonal to performance benchmarking (measuring our implementation). Golden files handle correctness perfectly.

4. **Performance Benchmark Strategy**: For E07, we measure liq-ta performance accurately with Criterion. Comparative claims against TA-Lib can reference:
   - Our own isolated TA-Lib measurements (run separately)
   - Published TA-Lib benchmarks from the community
   - User-provided comparison data

5. **Reproducibility**: Golden files provide deterministic, version-controlled test expectations. Any failures indicate implementation bugs, not environment differences.

## Consequences

### Positive
- Zero external runtime dependencies for testing
- Fast CI builds (no TA-Lib installation step)
- Deterministic test results across all platforms
- Clear separation between correctness and performance testing
- Simple maintenance - regenerate golden files only when test data changes

### Negative
- Initial effort to create golden file generation tooling
- Cannot test with dynamically generated random data
- Must regenerate golden files if reference TA-Lib version changes
- Performance comparisons are indirect (reference timings, not live measurements)

### Mitigations
- Provide clear documentation for regenerating golden files
- Include golden file generation in development setup instructions
- Store generation script and metadata with golden files for reproducibility
- For users needing live comparisons, document how to run TA-Lib benchmarks separately

## Implementation Checklist

- [ ] Create `tools/generate_golden.py` script
- [ ] Generate golden files for all 7 indicators with standardized test data
- [ ] Create `benches/golden/metadata.json` with TA-Lib version info
- [ ] Implement golden file comparison utilities in liq-ta-experiments
- [ ] Document golden file regeneration process in developer docs
- [ ] Add golden file validation to E07 experiment

## Related Decisions

- This decision informs the implementation of subtask-4-1 (TA-Lib comparison baseline)
- Affects E07 experiment design and reporting methodology

## References

- [TA-Lib Official Documentation](https://ta-lib.org/)
- [TA-Lib Python Wrapper](https://github.com/ta-lib/ta-lib-python)
- [Criterion.rs Benchmarking](https://bheisler.github.io/criterion.rs/book/)
- [ADR Format](https://adr.github.io/)
