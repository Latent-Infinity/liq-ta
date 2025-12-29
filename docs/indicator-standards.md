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

## Output Shape and NaN Policy
- **Full-length outputs**: output length equals input length.
- **NaN prefix**: first `indicator_lookback(...)` elements are NaN.
- **NaN propagation**: any NaN within a rolling window yields NaN output at that position.
- **Infinity propagation**: any `+/-inf` in the window propagates to the output.
- **Subnormal values**: processed normally; no special handling.
- **Indeterminate operations**: use explicitly defined outputs:
  - RSI: `avg_loss = 0` -> RSI = 100; `avg_gain = 0` -> RSI = 0; `both = 0` -> RSI = 50
  - Stochastic: `high == low` -> %K = 50
  - Bollinger: `stddev = 0` -> upper = middle = lower
  - ATR: first value uses SMA of initial TR window (Wilder seed)
  - ROC/ROCP/ROCR: `prev = 0` -> output = 0 (division by zero edge case)
  - AD: `range = 0` (high == low) -> CLV = 0 (neutral, no position)
  - MFI: `negative_mf = 0` -> MFI = 100; `positive_mf = 0` -> MFI = 0
- **Multi-output alignment**: all fields align to the same lookback and input index.
- **Lookback canonical**: `*_lookback()` defines the NaN prefix length; `*_min_len()` defines the minimum input length.

## IEEE 754 NaN Propagation Strategy

fast-ta leverages IEEE 754 floating-point semantics for NaN propagation wherever appropriate.
The IEEE 754 standard guarantees:
- **Arithmetic propagation**: `NaN + x = NaN`, `NaN - x = NaN`, `NaN * x = NaN`, `NaN / x = NaN`
- **Comparison behavior**: `NaN < x = false`, `NaN > x = false`, `NaN == x = false`
- **Special results**: `0.0 / 0.0 = NaN`, `Inf - Inf = NaN`, `x / 0.0 = ±Inf`

### Pattern Selection Decision Matrix

| Indicator Type | Example Indicators | Recommended Pattern | IEEE 754 Applicable? |
|---------------|-------------------|---------------------|---------------------|
| **Pointwise arithmetic** | avgprice, medprice, typprice | IEEE 754 auto-propagation | Yes |
| **Rolling min/max** | midpoint, midprice | IEEE 754 + sum accumulator | Partial (comparisons don't propagate) |
| **Rolling sum** | SMA | `nan_count` tracking | No (NaN must exit window) |
| **Weighted rolling** | WMA | `has_nan` + rescan | No (weighted sums need all positions) |
| **Cumulative/recursive** | EMA, RSI, ADX | Explicit NaN checks | Partial (IEEE 754 would work but explicit is clearer) |
| **Window sum** | MFI | IEEE 754 + `has_nan` window check | Yes (fresh sums per output) |
| **Division-based** | ROC, AD, BOP | IEEE 754 + explicit zero check | Yes for NaN, No for zero |

### Implementation Patterns

#### Pattern 1: IEEE 754 Auto-Propagation (Simple Arithmetic)

Use for indicators with simple pointwise operations (2-4 ops per element):

```rust
// Example: Typical Price = (H + L + C) / 3
// No explicit NaN checks needed - IEEE 754 propagates automatically
for i in 0..n {
    output[i] = (high[i] + low[i] + close[i]) / three;
}
```

**Applicable to:** avgprice, medprice, typprice, wclprice

#### Pattern 2: IEEE 754 with Division Edge Cases

Use for division-based indicators where divisor can legitimately be zero:

```rust
// Example: BOP = (Close - Open) / (High - Low)
for i in 0..n {
    let range = high[i] - low[i];
    if range == T::zero() {
        // Doji candle (high == low): return neutral value
        output[i] = if (close[i] - open[i]).is_finite() { T::zero() } else { T::nan() };
    } else {
        let result = (close[i] - open[i]) / range;
        // IEEE 754: if any input was NaN, result is NaN
        output[i] = if result.is_finite() { result } else { T::nan() };
    }
}
```

**Applicable to:** BOP, ROC, ROCP, ROCR, ROCR100, AD

#### Pattern 3: Sum Accumulator for Min/Max Detection

Use for rolling min/max indicators where IEEE 754 comparisons don't propagate NaN:

```rust
// Example: MIDPOINT = (Highest + Lowest) / 2
// Problem: NaN comparisons return false, so NaN won't be selected as min/max
// Solution: Track window sum which WILL become NaN if any element is NaN
let mut window_sum = T::zero();
for &val in &data[0..period] {
    window_sum = window_sum + val;  // IEEE 754: NaN + x = NaN
    // ... track min/max ...
}

if window_sum.is_finite() {
    output[i] = (highest + lowest) / two;
} else {
    output[i] = T::nan();
}
```

**Applicable to:** midpoint, midprice

#### Pattern 4: nan_count Tracking (Rolling Windows)

Use for rolling window indicators where NaN can exit the window:

```rust
// Example: SMA with NaN recovery
let mut nan_count = 0usize;
let mut sum = T::zero();

// Initial window
for &value in data.iter().take(period) {
    if value.is_nan() {
        nan_count += 1;
    } else {
        sum = sum + value;
    }
}

// Rolling updates - O(1) per element
for i in period..n {
    let new_val = data[i];
    let old_val = data[i - period];

    if new_val.is_nan() { nan_count += 1; } else { sum = sum + new_val; }
    if old_val.is_nan() { nan_count -= 1; } else { sum = sum - old_val; }

    output[i] = if nan_count == 0 { sum / period_t } else { T::nan() };
}
```

**Why IEEE 754 doesn't work here:** If we used `sum = sum + new_val - old_val` directly,
a single NaN would permanently corrupt the sum with no recovery path. The `nan_count`
pattern allows recovery when NaN exits the window.

**Applicable to:** SMA, Bollinger, Stochastic (for SMA smoothing)

#### Pattern 5: Explicit NaN Checks (Cumulative/Recursive)

Use for recursive indicators where NaN permanently corrupts state:

```rust
// Example: EMA with permanent NaN propagation
for i in period..n {
    let value = data[i];
    if ema_prev.is_nan() || value.is_nan() {
        output[i] = T::nan();
        ema_prev = T::nan();  // State is permanently corrupted
    } else {
        let ema_current = alpha * value + one_minus_alpha * ema_prev;
        output[i] = ema_current;
        ema_prev = ema_current;
    }
}
```

**Note:** IEEE 754 would also work here (`alpha * NaN = NaN`), but explicit checks
provide clearer intent and enable early short-circuit for remaining elements.

**Applicable to:** EMA, RSI, MACD, ADX, ATR

### NaN Propagation Audit Results (2024-12)

The comprehensive NaN propagation audit examined all indicator files and classified each by
the optimal pattern for their use case.

#### Audit Summary by Category

| Category | Files Audited | Status | Notes |
|----------|---------------|--------|-------|
| Pointwise | price_transform.rs | OPTIMAL | Already uses IEEE 754 auto-propagation |
| Rolling min/max | midpoint.rs, midprice.rs | OPTIMIZED | Added sum accumulator for NaN detection |
| Division-based | roc.rs, ad.rs, bop.rs | VALIDATED | Zero checks required and correctly placed |
| Rolling sum | sma.rs | OPTIMAL | nan_count pattern is correct and efficient |
| Weighted rolling | wma.rs | VALIDATED | has_nan + rescan is required for weighted sums |
| Cumulative | ema.rs, rsi.rs | VALIDATED | Explicit checks appropriate for recursive state |
| Composite | macd.rs, adx.rs | FIXED (ADX) | ADX had NaN propagation bug in comparisons |
| Volume-based | mfi.rs, ad.rs | OPTIMIZED | Added proper window NaN checking |

#### Key Audit Findings

1. **No `is_invalid()` function exists** - The codebase uses `.is_nan()` and `.is_finite()`
   from `num_traits::Float` via the `SeriesElement` trait.

2. **Simple pointwise indicators already optimal** - price_transform.rs indicators
   (avgprice, medprice, typprice, wclprice) naturally use IEEE 754 propagation through
   simple arithmetic with no explicit checks needed.

3. **Rolling window indicators correctly use `nan_count`** - SMA, Bollinger, and Stochastic
   properly track NaN count to allow recovery when NaN exits the window.

4. **Recursive indicators correctly propagate NaN permanently** - EMA, RSI, MACD use
   explicit checks and permanently corrupt state when NaN enters (intentional design
   for indicators with infinite memory).

5. **ADX comparison pattern bug fixed** - The original code used `smoothed_tr > T::zero()`
   which returns false when `smoothed_tr` is NaN (IEEE 754 comparison semantics), causing
   zero output instead of NaN. Fixed by adding explicit `is_nan()` checks before comparisons.

6. **WMA requires different pattern than SMA** - WMA cannot use `nan_count` because weighted
   sums require all window positions. Uses `has_nan` boolean with O(period) rescan when
   NaN exits window.

7. **Min/max comparisons don't propagate NaN** - midpoint and midprice use a sum accumulator
   to detect NaN in window because IEEE 754 comparisons (`NaN > x`) always return false.

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