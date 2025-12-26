# Indicator Standards (fast-ta)

## Purpose
Define the standards for indicator behavior, API shape, testing, and documentation so new indicators are easy to add and consistent with existing ones. This document is self-contained; it includes the required contracts and references to authoritative sources without assuming prior context.

## Scope
Applies to all indicators in `crates/fast-ta/src/indicators`, including single- and multi-output indicators, and any future indicators added to the public API.

## Definitions
- **Lookback**: The number of initial positions in the output that must be NaN due to insufficient prior data.
- **Min length**: The minimum input length required to compute at least one valid output (`lookback + 1` for most indicators).
- **Full-length output**: Output length equals input length with NaN prefix.

## Indicator API Contract
- **Module location**: `crates/fast-ta/src/indicators/<indicator>.rs`
- **Primary function**: `indicator(data, params...) -> Result<Vec<T>>`
- **Pre-allocated variant**: `indicator_into(data, params..., output: &mut [T]) -> Result<usize>`
  - Output buffer length must be `>= data.len()` for full-length output with NaN prefix.
  - Return the count of valid (non-NaN prefix) values where applicable.
- **Lookback/min length**:
  - `indicator_lookback(params...) -> usize`
  - `indicator_min_len(params...) -> usize` (typically `lookback + 1`)
- **Multi-output indicators**:
  - Provide `IndicatorOutput<T>` struct with named fields.
  - Provide `_into` with separate buffers per field.
  - All output fields must have identical length and aligned NaN prefix.
- **Config types (when parameter-heavy)**:
  - Add a `struct Indicator` with `Default` for standard params.
  - Provide fluent setters and `compute()` / `compute_into()` methods.
  - Use the existing pattern in `macd`, `bollinger`, `stochastic`, `adx` as examples.

## Output Shape and NaN/Infinity Policy
- **Full-length outputs**: output length equals input length.
- **NaN prefix**: first `indicator_lookback(...)` elements are NaN.
- **Rolling-window rule**: any NaN in the current window yields NaN output at that position.
- **Cumulative rule**: if a NaN is encountered in the input/state, outputs are NaN at that index and all subsequent indices (`nan_active`).
  - Examples: OBV, VWAP, running sums/ratios.
- **Infinity handling**: treat `+/-inf` as invalid (NaN-like). Any window containing `+/-inf` yields NaN output.
- **Warmup/lookback**: outputs are NaN until the full window required by `*_lookback(...)` is available.
- **Multi-period indicators**: if any required internal window contains NaN (e.g., MACD fast/slow/signal windows), all output fields at that index are NaN.
- **Mixed behavior**: if an indicator combines rolling and cumulative state, a NaN in the rolling window yields NaN output and activates cumulative NaN propagation.
- **Subnormal values**: processed normally; no special handling.
- **Indeterminate operations**: use explicitly defined outputs:
  - RSI: `avg_loss = 0` -> RSI = 100; `avg_gain = 0` -> RSI = 0
  - Stochastic: `high == low` -> %K = 50
  - Bollinger: `stddev = 0` -> upper = middle = lower
  - ATR: first value uses SMA of initial TR window (Wilder seed)
- **Multi-output alignment**: all fields align to the same lookback and input index.
- **Lookback canonical**: `*_lookback()` defines the NaN prefix length; `*_min_len()` defines the minimum input length.

### Internal Helper API Shape (NaN Handling)
Use a shared internal helper for rolling NaN tracking and cumulative propagation to keep behavior consistent and SIMD-friendly.

**Rolling-window helpers (proposed shape):**
- `fn build_nan_mask<T: SeriesElement>(data: &[T]) -> Vec<u8>` where `1` indicates NaN or +/-inf.
- `fn init_nan_count(mask: &[u8], start: usize, period: usize) -> usize`
- `fn update_nan_count(count: &mut usize, old_is_nan: bool, new_is_nan: bool)`

**Cumulative helpers (proposed shape):**
- `struct NanActive { active: bool }` with `fn step(&mut self, input_has_nan: bool) -> bool` returning whether output should be NaN.

**SIMD note:**
- SIMD paths are always active unless a per-period scalar fast path is demonstrably faster.
- SIMD NaN detection uses lane masks; any-lane NaN (or +/-inf) marks the window as invalid.

## Input Validation and Errors
- Use `validate_indicator_input` for single-series indicators.
- For OHLC inputs, enforce equal lengths and return `Error::LengthMismatch`.
- Validate periods (non-zero, ordered constraints like fast < slow).
- Return `Error::InsufficientData` if `data.len() < indicator_min_len(...)`.
- `_into` variants must return `Error::BufferTooSmall` when buffers are undersized.

## Performance Standards
- Target **O(n)** time complexity and **O(n)** memory.
- Avoid per-element allocations; pre-allocate outputs once.
- Prefer rolling sums/rolling windows and reuse computed state.
- Keep hot loops simple and branch-light.

## Quality Gates (Must-Haves)
Add or update coverage across these areas:
- **Spec fixtures**: `crates/fast-ta/tests/fixtures/` with rationale and expected values.
- **JSON fixture tests**: ensure new indicator is wired into `crates/fast-ta/tests/json_fixture_tests.rs`.
- **Numeric policy**: extend `crates/fast-ta/tests/numeric_policy_tests.rs` when applicable.
- **Property tests**: update `crates/fast-ta/tests/property_tests.rs` for shape/NaN guarantees.
- **Integration**: update `crates/fast-ta/tests/integration.rs` for API presence and basic usage.
- **Golden/reference checks** (if applicable): `crates/fast-ta/tests/golden/*` and `crates/fast-ta/tests/reference_tests.rs`.
- **Benchmarks**: add to `crates/fast-ta/benches/indicators.rs` for performance tracking.

## CLI Contract (fast-ta-cli)
If the indicator is CLI-exposed:
- Add a subcommand in `crates/fast-ta-cli/src/args.rs`.
- Wire compute logic in `crates/fast-ta-cli/src/main.rs`.
- Add CSV handling for input columns if needed in `crates/fast-ta-cli/src/csv_parser.rs`.
- Add CLI integration tests in `crates/fast-ta-cli/tests/cli_integration.rs`.

## Documentation Checklist for New Indicators
- Add module docs and examples in `crates/fast-ta/src/indicators/<indicator>.rs`.
- Export from `crates/fast-ta/src/indicators/mod.rs`.
- Update `crates/fast-ta/src/prelude.rs` if the indicator is part of the prelude surface.
- Note any TA-Lib or reference differences in fixtures or docs (if applicable).

## Minimal Add-Indicator Checklist
1. Implement indicator logic + `_into`.
2. Add `*_lookback` and `*_min_len`.
3. Add tests (fixtures, JSON wiring, property/integration).
4. Export in `indicators/mod.rs` (and prelude if needed).
5. Add CLI wiring/tests if exposed.
6. Add benchmarks if it is core or performance-sensitive.

## Examples

### Single-output (SMA)
```rust
use fast_ta::indicators::sma::{sma, sma_into, sma_lookback, sma_min_len};

let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
let out = sma(&prices, 3).unwrap();
assert_eq!(out.len(), prices.len());
assert!(out[0].is_nan());
assert_eq!(sma_lookback(3), 2);
assert_eq!(sma_min_len(3), 3);

let mut buffer = vec![0.0_f64; prices.len()];
sma_into(&prices, 3, &mut buffer).unwrap();
```

### Multi-output (Bollinger)
```rust
use fast_ta::indicators::bollinger::{bollinger_into, bollinger_lookback, BollingerOutput};

let prices = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
let mut out = BollingerOutput {
    upper: vec![0.0; prices.len()],
    middle: vec![0.0; prices.len()],
    lower: vec![0.0; prices.len()],
};

bollinger_into(&prices, 3, 2.0, &mut out).unwrap();
assert_eq!(bollinger_lookback(3), 2);
assert!(out.upper[0].is_nan());
```
