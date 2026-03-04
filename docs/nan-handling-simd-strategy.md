# NaN Handling SIMD Strategy

This document defines how NaN/Infinity detection and propagation works in SIMD paths for indicator kernels. It complements the NaN policy in `docs/indicator-standards.md`.

## Scope

- SIMD paths are always active unless a per-period scalar path is demonstrably faster.
- The goal is identical semantics between SIMD and scalar implementations.

## Detection Rules

- A lane is invalid if it is `NaN` or `+/-inf`.
- A window is invalid if **any** lane in the window is invalid.
- Invalid windows yield NaN outputs (rolling-window rule).
- For cumulative indicators, encountering an invalid input activates `nan_active` and forces all subsequent outputs to NaN.

## SIMD Detection Approach

- Use lane-wise `is_nan` and `is_infinite` checks to build an invalid-lane mask.
- Reduce masks with an any-lane check to decide whether the window is invalid.
- For rolling windows, maintain a scalar rolling count derived from SIMD masks to avoid rescans.

## Integration Points

- Rolling-window helpers should accept a precomputed invalid-mask buffer (`u8` per element) and use it to update rolling counts.
- SIMD kernels should generate invalid masks when loading data blocks; masks should be merged into the rolling count path.
- Scalar fallbacks must share the same helper API and policy rules.

## Test Strategy

- Policy enforcement tests must explicitly exercise SIMD paths (e.g., input lengths that trigger SIMD kernels).
- SIMD vs scalar output parity is required for NaN/Infinity behavior.
