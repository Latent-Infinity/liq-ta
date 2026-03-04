# NaN Handling Performance Thresholds

These thresholds define acceptable regressions after NaN-handling changes. Defaults apply unless explicitly updated with human sign-off.

## Default Thresholds

- Rolling-window indicators: **≤5% median regression** vs baseline
- Cumulative indicators: **≤5% median regression** vs baseline
- Worst-case regression: **≤10%** (investigate and approve if exceeded)

## Benchmark Scope

- Use existing indicator benchmarks or add targeted micro-benchmarks for affected kernels.
- Include representative periods (short, medium, long) and input lengths that trigger SIMD paths.

## Sign-off

- Deviations from these thresholds require explicit human approval and a note in the changelog or PR description.
