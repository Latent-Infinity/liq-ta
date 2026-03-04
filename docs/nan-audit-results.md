# NaN Audit Results

This document tracks the audit of NaN/Infinity handling across all liq-ta indicators.
The goal is to identify opportunities to use IEEE 754 NaN propagation for improved performance.

## Audit Status

| Status | Description |
|--------|-------------|
| OPTIMAL | Already uses IEEE 754 propagation, no changes needed |
| CANDIDATE | Can be optimized to use IEEE 754 propagation |
| VALIDATED | Complex indicator, current pattern validated as appropriate |
| PENDING | Not yet audited |

## Stage 1: Simple Pointwise Indicators

### price_transform.rs

**Status: OPTIMAL**

**Functions audited:**
- `avgprice` - (Open + High + Low + Close) / 4
- `medprice` - (High + Low) / 2
- `typprice` - (High + Low + Close) / 3
- `wclprice` - (High + Low + 2*Close) / 4

**Current NaN handling:**
- Output initialized with `T::nan()` for safety
- Computation uses simple arithmetic (addition, multiplication, division by constants)
- No explicit `is_invalid()` or `is_nan()` checks in computation loops
- All 4 `is_nan()` occurrences are in test validation code only

**IEEE 754 Analysis:**
These indicators use only addition and division by non-zero constants. IEEE 754 guarantees:
- `NaN + x = NaN`
- `NaN / x = NaN` (where x is finite non-zero)

**Conclusion:** Already optimally using IEEE 754 propagation. No changes needed.

**Division-by-zero risk:** None. Divisors are constant values (2, 3, 4).

---

### midpoint.rs

**Status: OPTIMAL**

**Function audited:**
- `midpoint` - (Highest(data, period) + Lowest(data, period)) / 2

**Current NaN handling:**
- Lookback period filled with `T::nan()` explicitly
- Rolling min/max computed via comparison operators (`<`, `>`)
- No explicit `is_invalid()` checks in computation loops
- 13 `is_nan()`/`is_finite()` occurrences are in test validation only

**IEEE 754 Analysis:**
Comparison operators with NaN have well-defined behavior:
- `NaN < x = false`
- `NaN > x = false`
- `NaN + x = NaN`

If input window contains NaN:
- NaN will NOT become highest/lowest (comparison returns false)
- But if ALL values in window are NaN, result will be NaN
- Arithmetic on non-NaN highest/lowest propagates correctly

**Edge case consideration:** If window contains mix of NaN and valid values, the min/max will use valid values only. This is CORRECT behavior for rolling window indicators - only propagate NaN when ALL values are NaN.

**Conclusion:** Already uses IEEE 754 semantics correctly. The comparison-based min/max naturally handles mixed NaN data appropriately.

**Division-by-zero risk:** None. Divisor is constant 2.

---

### midprice.rs

**Status: OPTIMAL**

**Function audited:**
- `midprice` - (Highest(high, period) + Lowest(low, period)) / 2

**Current NaN handling:**
- Lookback period filled with `T::nan()` explicitly
- Rolling highest-high and lowest-low computed via comparison operators
- No explicit `is_invalid()` checks in computation loops
- 13 `is_nan()`/`is_finite()` occurrences are in test validation only

**IEEE 754 Analysis:**
Same as midpoint - uses comparison operators for min/max finding.

**Conclusion:** Already uses IEEE 754 semantics correctly.

**Division-by-zero risk:** None. Divisor is constant 2.

---

## Stage 1.2: Division-Based Indicators

### roc.rs

**Status: VALIDATED**

**Functions audited:**
- `roc` - Rate of Change: ((price - price[n]) / price[n]) * 100
- `rocp` - Rate of Change Percentage: (price - price[n]) / price[n]
- `rocr` - Rate of Change Ratio: price / price[n]
- `rocr100` - Rate of Change Ratio * 100: (price / price[n]) * 100

**Division-by-zero check locations:**
- Line 102: `roc_into` - `if prev == T::zero()`
- Line 215: `rocp_into` - `if prev == T::zero()`
- Line 328: `rocr_into` - `if prev == T::zero()`
- Line 442: `rocr100_into` - `if prev == T::zero()`

**Current pattern:**
```rust
for i in lookback..n {
    let prev = data[i - period];
    if prev == T::zero() {
        output[i] = T::nan();
    } else {
        output[i] = ((data[i] - prev) / prev) * hundred;
    }
}
```

**IEEE 754 Analysis:**
1. **Explicit zero check is REQUIRED** - Zero is a valid price value that can occur in real data. Division by zero produces Infinity (not NaN), which is semantically incorrect for rate-of-change calculations.

2. **NaN input propagation is automatic:**
   - If `data[i]` is NaN: arithmetic produces NaN via IEEE 754
   - If `prev` is NaN: `prev == T::zero()` returns false (NaN comparisons), then `data[i] / NaN = NaN`

3. **Test coverage:** Explicit test `test_roc_division_by_zero` (line 568-574) verifies zero handling

**Conclusion:** The explicit zero check is necessary and correctly placed. NaN propagation already works via IEEE 754 semantics - no changes needed.

**Division-by-zero risk:** HIGH - Previous price can be zero in valid data (e.g., newly listed securities, adjusted prices). Check is mandatory.

---

### ad.rs

**Status: VALIDATED**

**Function audited:**
- `ad` - Accumulation/Distribution Line (cumulative volume-based indicator)

**Formula:**
```
Money Flow Multiplier = ((close - low) - (high - close)) / (high - low)
                      = (2 * close - high - low) / (high - low)
Money Flow Volume = MFM × volume
AD = cumulative sum of Money Flow Volume
```

**Division-by-zero check location:**
- Line 112: `if range > T::zero()`

**Current pattern:**
```rust
let range = h - l;

let mfm = if range > T::zero() {
    let two = T::from_f64(2.0).unwrap_or_else(|_| T::one() + T::one());
    (two * c - h - l) / range
} else {
    // high == low, no range, MFM = 0
    T::zero()
};
```

**IEEE 754 Analysis:**
1. **Explicit check is REQUIRED** - When high == low (doji candle, no trading range), there's no meaningful "position within range" to compute. Returning MFM = 0 is the correct semantic choice.

2. **Pattern uses `> T::zero()` instead of `== T::zero()`:**
   - Catches `high == low` (range = 0)
   - Also defensive against invalid OHLC data where `high < low`
   - When range is NaN (h or l is NaN), comparison returns false → MFM = 0

3. **NaN propagation behavior:**
   - **NaN in close:** `two * c = NaN`, propagates through division → NaN
   - **NaN in volume:** `mfm * v = NaN`, propagates through cumulative sum → permanent NaN
   - **NaN in high or low:** range = NaN, `NaN > 0` is false → MFM = 0 (NaN NOT propagated)

4. **Design decision on high/low NaN:** The current behavior treats NaN in high/low as "no range" (MFM = 0) rather than propagating. This is a reasonable design choice because:
   - AD is cumulative - once NaN enters, it stays forever
   - High/low NaN likely indicates missing OHLC bar, treating as neutral preserves prior trend
   - NaN in close/volume will still propagate (more critical for AD semantics)

**Test coverage:** `test_ad_high_equals_low` (line 261-271) verifies zero range handling

**Conclusion:** The explicit range check is necessary. The NaN handling for high/low inputs is a deliberate design choice favoring continuity over strict propagation, which is appropriate for cumulative indicators.

**Division-by-zero risk:** HIGH - Doji candles (high == low) are common in trading data. Check is mandatory.

---

## Summary for Stage 1.2

| File | Zero-check pattern | Locations | Status | Action |
|------|-------------------|-----------|--------|--------|
| roc.rs | `prev == T::zero()` | 4 (lines 102, 215, 328, 442) | VALIDATED | None - required check |
| ad.rs | `range > T::zero()` | 1 (line 112) | VALIDATED | None - required check |

### Key Findings for Division-Based Indicators

1. **Zero checks are mandatory** - Both indicators divide by values that can legitimately be zero from valid input data. These checks cannot be removed.

2. **Pattern difference:**
   - ROC uses `== T::zero()` (exact equality) - appropriate for single value check
   - AD uses `> T::zero()` (greater than) - more defensive, also catches negative ranges

3. **IEEE 754 NaN propagation still works** - NaN inputs propagate naturally through arithmetic. The zero checks only guard against valid-but-problematic inputs.

4. **Cumulative indicator consideration** - AD's choice to treat NaN in high/low as MFM=0 is appropriate because cumulative indicators permanently propagate any NaN. Treating edge cases as "neutral" preserves usability.

---

## Summary for Stage 1

| File | is_invalid usage | is_finite/is_nan in code | Status | Action |
|------|-----------------|-------------------------|--------|--------|
| price_transform.rs | 0 | 0 (4 in tests) | OPTIMAL | None |
| midpoint.rs | 0 | 0 (13 in tests) | OPTIMAL | None |
| midprice.rs | 0 | 0 (13 in tests) | OPTIMAL | None |

### Key Findings

1. **No `is_invalid()` function exists in the codebase** - The spec referenced this function, but it does not exist. The codebase uses `.is_nan()` and `.is_finite()` methods directly from the `num_traits::Float` trait via `SeriesElement`.

2. **Simple pointwise indicators already use IEEE 754 propagation** - These indicators perform simple arithmetic that naturally propagates NaN values without explicit checks.

3. **Test code vs production code** - All `.is_nan()` and `.is_finite()` calls in these files are in test functions for validation, not in production computation loops.

4. **Comparison operator semantics** - The rolling min/max indicators (midpoint, midprice) correctly leverage IEEE 754 comparison semantics where `NaN < x` and `NaN > x` both return false.

## Pattern Reference

### IEEE 754 NaN Propagation Pattern (from bop.rs)

```rust
// Good pattern - single result check instead of multiple input checks
for i in 0..n {
    let result = (close[i] - open[i]) / (high[i] - low[i]);
    output[i] = if result.is_finite() { result } else { T::nan() };
}
```

### When to use explicit checks

1. **Division by zero** - Must check when divisor can be zero from valid inputs
   - Example: BOP checks `range == T::zero()` before division
   - Example: ROC must check if previous price is zero

2. **Rolling window NaN tracking** - Complex indicators may need `invalid_count` tracking
   - Example: SMA needs to track when NaN enters/exits the window

3. **Cumulative indicators** - EMA/RSI where NaN propagates permanently
   - May need `nan_active` flag for efficient short-circuit

---

## Stage 1.3: Complex Indicators (SMA, EMA, RSI, MFI)

### sma.rs

**Status: VALIDATED**

**Function audited:**
- `sma` - Simple Moving Average with rolling sum optimization
- `sma_into` - Pre-allocated buffer variant

**NaN check count:** 34 total `is_nan()` occurrences (18 in computation, 16 in tests)

**NaN tracking locations in computation:**
- Line 145: `if value.is_nan()` - initial window sum calculation
- Line 162: `if new_value.is_nan()` - new value entering window
- Line 168: `if old_value.is_nan()` - old value exiting window
- Line 247: `if value.is_nan()` - initial window sum in `_into` variant
- Line 266: `if new_value.is_nan()` - new value entering window in `_into`
- Line 272: `if old_value.is_nan()` - old value exiting window in `_into`

**Current pattern:**
```rust
// Track NaN count in rolling window
let mut nan_count = 0usize;
for &value in data.iter().take(period) {
    if value.is_nan() {
        nan_count += 1;
    } else {
        sum = sum + value;
    }
}

// Rolling update: add new, subtract old
for i in period..data.len() {
    let new_value = data[i];
    let old_value = data[i - period];

    if new_value.is_nan() {
        nan_count += 1;
    } else {
        sum = sum + new_value;
    }

    if old_value.is_nan() {
        nan_count -= 1;
    } else {
        sum = sum - old_value;
    }

    if nan_count == 0 {
        result[i] = sum / period_t;
    } else {
        result[i] = T::nan();
    }
}
```

**IEEE 754 Analysis:**

1. **Why explicit NaN tracking is REQUIRED:**
   - SMA is a rolling window indicator where NaN can **exit** the window
   - Example: data = [1.0, NaN, 3.0, 4.0, 5.0], period = 2
     - At i=2: window is [NaN, 3.0] → output = NaN ✓
     - At i=3: window is [3.0, 4.0] → output = 3.5 (NaN has exited) ✓
   - IEEE 754 propagation cannot track when NaN leaves the window

2. **Alternative considered: IEEE propagation with rolling sum**
   - If we used `sum = sum + new_value - old_value` with NaN, the sum would become NaN permanently
   - Once `sum` is NaN, all future outputs would be NaN (incorrect)
   - The current pattern correctly "recovers" when NaN exits the window

3. **Performance consideration:**
   - Each iteration does 2 `is_nan()` checks (new value, old value)
   - Pre-scan approach would require O(n) scan + window tracking overhead
   - Current pattern is efficient: O(1) checks per element with O(1) counter update

**Conclusion:** The `nan_count` tracking pattern is necessary and correctly implemented for rolling window indicators. Cannot use pure IEEE 754 propagation because NaN must be allowed to exit the window.

**Division-by-zero risk:** None. Divisor is the period (always > 0 after validation).

---

### ema.rs

**Status: VALIDATED**

**Function audited:**
- `ema` - Exponential Moving Average with standard smoothing
- `ema_wilder` - EMA with Wilder's smoothing (α = 1/period)
- `ema_with_alpha` - EMA with custom smoothing factor
- All `_into` variants

**NaN check count:** 23 total `is_nan()` occurrences (4 in computation, 19 in tests)

**NaN tracking locations in computation:**
- Line 388: `if value.is_nan()` - checking initial SMA seed values
- Line 409: `if ema_prev.is_nan() || value.is_nan()` - main EMA loop

**Current pattern:**
```rust
// Initial SMA seed calculation
let mut nan_count = 0usize;
for &value in data.iter().take(period) {
    if value.is_nan() {
        nan_count += 1;
    } else {
        sum = sum + value;
    }
}

let mut ema_prev = if nan_count == 0 {
    let sma_seed = sum / period_t;
    output[period - 1] = sma_seed;
    sma_seed
} else {
    output[period - 1] = T::nan();
    T::nan()
};

// EMA computation with NaN propagation
for i in period..data.len() {
    let value = data[i];
    if ema_prev.is_nan() || value.is_nan() {
        output[i] = T::nan();
        ema_prev = T::nan();
    } else {
        let ema_current = alpha * value + one_minus_alpha * ema_prev;
        output[i] = ema_current;
        ema_prev = ema_current;
    }
}
```

**IEEE 754 Analysis:**

1. **Why explicit NaN checking is REQUIRED:**
   - EMA is a **cumulative** indicator with infinite memory
   - Once NaN enters the EMA state, it propagates permanently
   - The pattern correctly "short-circuits" when NaN is detected

2. **Alternative considered: Pure IEEE 754 propagation**
   - Could compute: `ema_current = alpha * value + one_minus_alpha * ema_prev`
   - If `value` is NaN: `alpha * NaN = NaN`, result is NaN ✓
   - If `ema_prev` is NaN: `one_minus_alpha * NaN = NaN`, result is NaN ✓
   - **IEEE 754 would work correctly** for EMA arithmetic!

3. **Why current pattern is still appropriate:**
   - The `is_nan()` check enables early short-circuit for performance
   - Once NaN is detected, skips all multiplications for remaining values
   - Explicit NaN tracking is clearer for maintenance

4. **Optimization potential:**
   - Could replace with pure IEEE 754 and single result check
   - Performance difference likely minimal (2 mul + 1 add vs 2 is_nan checks)
   - Current pattern is safe and validated

**Conclusion:** The current pattern is validated as correct. Pure IEEE 754 propagation would also work but the explicit check enables early short-circuit. No changes recommended.

**Division-by-zero risk:** None. Period validated > 0 before division.

---

### rsi.rs

**Status: VALIDATED**

**Function audited:**
- `rsi` - Relative Strength Index with Wilder's smoothing
- `rsi_into` - Pre-allocated buffer variant

**NaN check count:** 33 total `is_nan()` occurrences (8 in computation, 25 in tests)

**NaN tracking locations in computation:**
- Line 287: `if data[i].is_nan() || data[i - 1].is_nan()` - initial gain/loss calculation
- Line 321: `if data[i].is_nan() || data[i - 1].is_nan() || avg_gain.is_nan() || avg_loss.is_nan()` - main loop

**Current pattern:**
```rust
// Initial sum of gains and losses
for i in 1..=period {
    if data[i].is_nan() || data[i - 1].is_nan() {
        has_nan = true;
        break;
    }
    let change = data[i] - data[i - 1];
    if change > zero {
        sum_gain = sum_gain + change;
    } else if change < zero {
        sum_loss = sum_loss - change;
    }
}

// Wilder's smoothing with NaN propagation
for i in (period + 1)..data.len() {
    if data[i].is_nan() || data[i - 1].is_nan() || avg_gain.is_nan() || avg_loss.is_nan() {
        avg_gain = T::nan();
        avg_loss = T::nan();
        output[i] = T::nan();
        continue;
    }
    // ... Wilder's smoothing calculation
}
```

**IEEE 754 Analysis:**

1. **Why explicit NaN checking is REQUIRED:**
   - RSI requires computing price **changes** (data[i] - data[i-1])
   - Both current and previous values must be valid for a meaningful change
   - Cumulative avg_gain/avg_loss state must propagate NaN permanently

2. **Division-by-zero handling:**
   - Lines 358-375: `compute_rsi_value()` handles avg_loss = 0 and avg_gain = 0
   - Returns RSI = 100 (all gains), RSI = 0 (all losses), or RSI = 50 (no movement)
   - These are **deterministic boundary conditions**, not NaN

3. **Comparison operator semantics:**
   - `if change > zero` returns false when change is NaN
   - This naturally excludes NaN from gain/loss sums (correct behavior)

4. **Current pattern advantages:**
   - Clear early exit when NaN detected
   - Explicitly propagates NaN through cumulative state
   - Division-by-zero handled with meaningful values instead of NaN

**Conclusion:** The pattern is validated as correct and appropriate. The combination of NaN tracking for cumulative state and explicit division-by-zero handling is the right approach for RSI.

**Division-by-zero risk:** HANDLED - avg_loss = 0 returns RSI = 100 (or 50 if avg_gain also 0).

---

### mfi.rs

**Status: OPTIMAL**

**Function audited:**
- `mfi` - Money Flow Index (volume-weighted RSI variant)
- `mfi_into` - Pre-allocated buffer variant

**NaN check count:** 1 `is_nan()` in tests only, 2 `is_finite` in tests only

**Current pattern:**
```rust
// Calculate typical prices - IEEE 754 propagation
for i in 0..n {
    tp[i] = (high[i] + low[i] + close[i]) / three;
}

// Sum money flows using comparison operators
for j in start..=i {
    let raw_mf = tp[j] * volume[j];

    if tp[j] > tp[j - 1] {
        positive_mf = positive_mf + raw_mf;
    } else if tp[j] < tp[j - 1] {
        negative_mf = negative_mf + raw_mf;
    }
}

// Division-by-zero handling
if negative_mf == T::zero() {
    output[i] = hundred;
} else if positive_mf == T::zero() {
    output[i] = T::zero();
} else {
    let mfr = positive_mf / negative_mf;
    output[i] = hundred - (hundred / (one + mfr));
}
```

**IEEE 754 Analysis:**

1. **NaN propagation is implicit via IEEE 754:**
   - Typical price: `(h + l + c) / 3` → NaN if any input is NaN ✓
   - Raw money flow: `tp[j] * volume[j]` → NaN if either is NaN ✓
   - Comparison: `tp[j] > tp[j-1]` → false if either is NaN (NaN excluded from sums) ✓

2. **MFI does NOT use cumulative state:**
   - Each output is computed from a fresh window sum
   - No permanent state that would require NaN tracking
   - This is why pure IEEE 754 propagation works

3. **Division-by-zero handling:**
   - `negative_mf == T::zero()` → MFI = 100 (all positive flow)
   - `positive_mf == T::zero()` → MFI = 0 (all negative flow)
   - These are deterministic boundary conditions

4. **Why MFI differs from RSI:**
   - RSI uses Wilder's smoothing (cumulative state)
   - MFI uses simple window sums (no state carry-over)
   - MFI can use pure IEEE 754; RSI cannot

**Conclusion:** Already optimal. No explicit NaN checks needed in computation. The indicator correctly uses IEEE 754 propagation with appropriate division-by-zero handling.

**Division-by-zero risk:** HANDLED - Returns MFI = 100 or MFI = 0 for edge cases.

---

## Summary for Stage 1.3

| File | Pattern | `is_nan()` in code | `is_nan()` in tests | Status | Reason |
|------|---------|-------------------|---------------------|--------|--------|
| sma.rs | `nan_count` tracking | 18 | 16 | VALIDATED | Rolling window - NaN can exit |
| ema.rs | Cumulative state check | 4 | 19 | VALIDATED | Cumulative - NaN permanent |
| rsi.rs | Cumulative state check + div-zero | 8 | 25 | VALIDATED | Cumulative + boundary conditions |
| mfi.rs | IEEE 754 propagation | 0 | 3 | OPTIMAL | Window sums - no state carry |

### Key Findings for Complex Indicators

1. **Rolling window indicators (SMA)** require explicit `nan_count` tracking because NaN can exit the window. IEEE 754 propagation would incorrectly make all subsequent outputs NaN.

2. **Cumulative indicators (EMA, RSI)** use explicit NaN checks for early short-circuit. The cumulative state (`ema_prev`, `avg_gain`, `avg_loss`) propagates NaN permanently once encountered.

3. **Window sum indicators (MFI)** can use pure IEEE 754 propagation because each output is computed from a fresh window with no carry-over state.

4. **Division-by-zero** is handled with deterministic boundary conditions:
   - RSI: avg_loss=0 → 100, avg_gain=0 → 0, both=0 → 50
   - MFI: negative_mf=0 → 100, positive_mf=0 → 0

5. **Comparison operator semantics** (NaN < x = false, NaN > x = false) are leveraged correctly in both RSI and MFI for excluding NaN from gain/loss categorization.

### Classification Matrix (Updated)

| Indicator Type | Example | NaN Pattern | Can Use IEEE 754? |
|---------------|---------|-------------|-------------------|
| Pointwise | price_transform | None needed | Yes ✓ |
| Rolling min/max | midpoint | Comparison semantics | Yes ✓ |
| Rolling sum | SMA | `nan_count` tracking | **No** - NaN must exit |
| Cumulative | EMA, RSI | State propagation | Partial - state check needed |
| Window sum | MFI | IEEE 754 propagation | Yes ✓ |
| Division-based | ROC, AD | Zero check required | Yes for NaN, No for zero |

---

---

## Stage 4: Medium Complexity Indicators - Evaluation

### wma.rs

**Status: VALIDATED**

**Function audited:**
- `wma` - Weighted Moving Average with rolling weighted sum optimization
- `wma_into` - Pre-allocated buffer variant

**NaN check count:** 8 `is_nan()` occurrences in computation code, 21 in tests

**NaN tracking locations in computation:**
- Line 148: `if value.is_nan()` - initial window check
- Line 169-170: `let nan_entering = new_value.is_nan(); let nan_exiting = old_value.is_nan()` - enter/exit detection
- Line 180: `.iter().any(|v| v.is_nan())` - window rescan when NaN exits
- Lines 277, 299-300, 308 - Same pattern in `wma_into` variant

**Current pattern:**
```rust
// Boolean flag + rescan approach
let mut has_nan = false;

// Initial window check
for (i, &value) in data.iter().take(period).enumerate() {
    if value.is_nan() {
        has_nan = true;
    }
    let weight = T::from_usize(i + 1)?;
    weighted_sum = weighted_sum + value * weight;
    simple_sum = simple_sum + value;
}

// Rolling update
for i in period..data.len() {
    let nan_entering = new_value.is_nan();
    let nan_exiting = old_value.is_nan();

    if nan_entering { has_nan = true; }

    if has_nan {
        if nan_exiting && !nan_entering {
            // O(period) rescan to check if window is clean
            has_nan = data[i - period + 1..=i].iter().any(|v| v.is_nan());

            if !has_nan {
                // O(period) recomputation of sums
                weighted_sum = T::zero();
                simple_sum = T::zero();
                for (j, &val) in data[i - period + 1..=i].iter().enumerate() {
                    let weight = T::from_usize(j + 1).unwrap();
                    weighted_sum = weighted_sum + val * weight;
                    simple_sum = simple_sum + val;
                }
            }
        }
    } else {
        // Normal O(1) rolling update
        weighted_sum = weighted_sum - simple_sum + new_value * period_t;
        simple_sum = simple_sum - old_value + new_value;
    }
}
```

**IEEE 754 Analysis:**

1. **Why explicit NaN tracking is REQUIRED:**
   - WMA is a rolling window indicator where NaN can **exit** the window
   - Must detect when window becomes clean to resume rolling updates
   - IEEE 754 propagation would make sums permanently NaN with no recovery

2. **Why WMA cannot use SMA's `nan_count` pattern:**
   - SMA can exclude NaN from sums because all values are equally weighted
   - WMA's rolling formula `weighted_sum = weighted_sum - simple_sum + new * period` implicitly adjusts weights for ALL positions
   - Subtracting `simple_sum` reduces weight of every value by 1 - this requires all positions to have values
   - Can't maintain "partial" weighted sums with NaN holes

3. **Current pattern trade-offs:**
   - O(1) checks per element in happy path (no NaN in window)
   - O(period) rescan when NaN exits window (only when `nan_exiting && !nan_entering`)
   - O(period) recomputation when window becomes clean
   - This is optimal given the constraint that weighted sums require full windows

4. **Comparison with SMA:**
   | Aspect | SMA | WMA |
   |--------|-----|-----|
   | NaN tracking | `nan_count` counter | `has_nan` boolean |
   | Can exclude NaN from sums | Yes | No |
   | Recovery from NaN | O(1) (decrement counter) | O(period) (rescan + recompute) |
   | Reason | Equal weights | Position-dependent weights |

5. **Potential optimization considered:**
   - Could track `nan_count` instead of boolean to avoid rescan
   - But recomputation is still O(period), so benefit is marginal
   - Would add complexity for minimal gain

**Conclusion:** The current pattern is validated as appropriate for WMA. The weighted sum formula requires all window positions to have values, so NaN cannot be "excluded" like in SMA. The `has_nan` + rescan + recompute approach is correct and efficient for this constraint.

**Division-by-zero risk:** None. Weight sum divisor is `period * (period + 1) / 2` which is always > 0 for valid period.

---

## Summary for Stage 4.1

| File | Pattern | `is_nan()` in code | Status | Reason |
|------|---------|-------------------|--------|--------|
| wma.rs | `has_nan` + rescan | 8 | VALIDATED | Rolling weighted sum - requires full window |

### Key Findings for WMA

1. **WMA differs from SMA fundamentally:**
   - SMA: uniform weights → can exclude NaN from rolling sum
   - WMA: position weights → all positions needed for weight adjustment

2. **Rolling update formula constraint:**
   - `weighted_sum = weighted_sum - simple_sum + new * period`
   - Subtracting `simple_sum` adjusts ALL weights simultaneously
   - Cannot work with partial windows

3. **Pattern is optimal for this constraint:**
   - Happy path: O(1) per element
   - NaN recovery: O(period) only when NaN exits (unavoidable given weight requirements)

4. **IEEE 754 propagation not applicable:**
   - Would permanently corrupt sums
   - No automatic recovery when NaN exits window

---

## Next Steps

- [x] Audit division-based indicators (roc.rs, ad.rs) - Stage 1.2 ✓
- [x] Audit complex indicators (sma, ema, rsi, mfi) - Stage 1.3 ✓
- [x] Apply IEEE pattern where beneficial - Stage 2-3 ✓
- [x] Evaluate wma.rs for IEEE optimization - Stage 4.1 ✓
- [ ] Evaluate ema.rs NaN handling strategy - Stage 4.2
- [ ] Evaluate rsi.rs NaN handling strategy - Stage 4.3
- [ ] Validate complex indicators use appropriate patterns - Stage 5
