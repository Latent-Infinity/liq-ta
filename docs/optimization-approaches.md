# fast-ta Optimization Guide

This document is the **optimization playbook** for fast-ta indicators. It defines:
- the **performance contract** (what “faster” means in this repo),
- the **correctness contract** (NaN/Inf behavior),
- a **decision framework** for algorithm selection and micro-optimizations,
- a **benchmark-gated workflow** (changes land only if they are measurably faster).

Optimization is the primary concern, but the code must remain maintainable:
- **SOLID**: separate dispatch / allocation / core math.
- **DRY**: shared kernels live in shared modules.
- **KISS**: simplest change that wins benchmarks.

---

## 0) Policy: Benchmark-Gated Changes Only

### 0.1 Source of truth
**Benchmarks decide.** No optimization is accepted based on intuition.

### 0.2 Acceptance rule (high level)
A change is accepted only if:
1. It preserves the indicator’s documented semantics (including NaN/Inf policy), and
2. It improves the **overall performance score** across the standard period sweep (see §2), and
3. It does not introduce unacceptable regressions (guardrails in §2.4).

This explicitly supports the real-world case:
- an O(n) algorithm may be slightly slower at `period=5` due to overhead,
- but significantly faster at `period=55/89`,
- and is still a win for the library if the **suite** improves.

---

## 1) Standard Benchmark Protocol

### 1.1 Benchmark matrix (minimum)
Run at least:

**Sizes**
- `n ∈ {100, 1_000, 10_000, 100_000}`

**Periods (single-period indicators)**
- `period ∈ {5, 13, 21, 55, 89}`

**Rationale (why these periods)**
- This sweep detects crossovers where algorithmic changes (e.g., O(n·k) → O(n)) may:
  - match/lose at small k (overhead dominates),
  - win at medium/long k (asymptotics dominate).
- The goal is to prevent “optimized for one period” outcomes.

**Correctness edge periods**
- `period ∈ {1, 2}` must be covered by unit tests where applicable.
- They are not part of the default performance sweep unless an indicator is known to have performance cliffs there.

### 1.2 Data distributions (minimum)
Benchmarks must include:
- **Random-ish** (worst-case branch prediction)
- **Trending** (best-case branch prediction)
- **Flat/repeated** (exercises equality and zero-range edge logic)

### 1.3 NaN/Inf scenarios (minimum)
Include at least:
- clean finite inputs
- a single NaN injected
- an Inf injected

(Use indicator semantics: windowed indicators must recover; cumulative indicators must poison forward.)

### 1.4 Parity rules (avoid apples-to-oranges)
When comparing to TA-Lib or other baselines, state clearly:
- whether the benchmark includes allocation (wrapper) or uses `*_into`
- output length conventions (leading NaNs written vs omitted)
- compilation settings (fast-math vs strict IEEE)

### 1.5 Reporting format (copy/paste)
Include a table like:

| Impl | n | period | time | throughput | delta |
|------|---|--------|------|------------|-------|
| before | 100K | 21 | 151.13µs | 661.7 Melem/s | baseline |
| after  | 100K | 21 | 149.65µs | 668.2 Melem/s | +1.0% |

Also include:
- CPU model
- build profile/flags (`lto`, `codegen-units`, `target-cpu`)
- median-of-N runs (and N)

---

## 2) Performance Score and Acceptance Criteria

We optimize for the library, not a single period. A change can be accepted even if it is slightly worse at `period=5`, as long as it is faster **overall** across the canonical sweep.

### 2.1 Primary metric: weighted geometric mean speedup
For each benchmark case `c` (a particular `(n, period, distribution)`), compute:

- `speedup_c = time_before_c / time_after_c`  
  (greater than 1.0 is good)

Aggregate with a **weighted geometric mean**:

- `score = exp( Σ (w_c * ln(speedup_c)) )`

This is standard practice for suites because it:
- rewards broad improvements,
- prevents one outlier from dominating,
- correctly composes multiplicative speedups.

### 2.2 Default weighting (KISS, but period-aware)
We care more about realistic workloads than micro-n. Default weights:
- Sizes:
  - `n=100`: 0.05
  - `n=1_000`: 0.10
  - `n=10_000`: 0.35
  - `n=100_000`: 0.50
- Periods (within each size):
  - equal weight across `{5, 13, 21, 55, 89}`

If your production usage is known (e.g., mostly `period=21`), define an override, but keep the canonical sweep as the baseline.

### 2.3 Acceptance threshold
A change is accepted if:
- `score >= 1.01` (≥ 1.0% improvement overall), **OR**
- `score >= 1.005` (≥ 0.5%) **and** it repeats across at least 3 independent benchmark runs with stable conditions.

This prevents churn from noise.

### 2.4 Regression guardrails (don’t trade one cliff for a suite win)
Even if the overall score improves, block changes that introduce sharp regressions:

**Guardrail A (primary workloads)**
- For `n >= 10_000`, no single `(period ∈ {13,21,55,89})` case may regress by more than **2%** unless the overall score is **≥ 1.03**.

**Guardrail B (small-period tolerance)**
- For `period=5`, a regression up to **3%** may be acceptable if the overall score meets the acceptance threshold, because this is a known crossover point for O(n) vs O(n·k) changes.

**Guardrail C (small n)**
- For `n ∈ {100, 1_000}`, regressions are tolerated up to **5%** if the overall score improves and `n >= 10_000` improves, because small n is dominated by overhead and is not the primary performance target.

These guardrails exist to support algorithmic upgrades without letting obvious cliffs slip in.

### 2.5 Practical implication: use dispatch to avoid crossovers when possible
If an O(n) method is slower for small k but faster for large k:
- implement both,
- add dispatch based on `n` and/or `period`,
- choose thresholds using benchmarks.

Do **not** guess thresholds. Bench them and document the crossover.

---

## 3) Correctness Contract (NaN/Inf)

### 3.1 Default policy
fast-ta defaults to **strict IEEE 754 propagation** unless documented otherwise.

### 3.2 Indicator classes: NaN behavior
- **Windowed indicators**: any NaN/Inf inside the active window yields NaN output at that position; outputs **recover** once NaN exits the window.
- **Cumulative indicators**: once NaN/Inf enters the accumulator, all subsequent outputs are NaN (**no recovery**).

### 3.3 Rolling extrema NaN modes
Rolling min/max kernels must support:
- **Skip mode**: NaNs ignored in extrema calculation
- **Propagate mode**: any NaN in window produces NaN output

Each indicator must explicitly choose and test its mode.

---

## 4) Step 1 Always: Classify the Indicator (Performance Shape)

Most failed “optimizations” happen because the wrong technique is applied to the wrong shape.

| Archetype | Examples | Primary limiter | Primary lever |
|---|---|---|---|
| Rolling extrema (min/max) | Donchian, Williams %R, Stochastic, Midpoint/Midprice | O(n·k) or cache patterns | VHGW (batch) or Deque (stream/small n) |
| Rolling sums/ratios | MFI, ULTOSC | redundant overlap work | rolling update (add new - old), reduce divisions |
| Cumulative | AD, OBV | DIV latency + dependency | early exit + bulk fill; unroll to pipeline DIV |
| EMA/IIR filters | T3, DEMA, TEMA, RSI(EMA), MACD, ATR | dependency chain latency | difference-form EMA; critical path reduction |
| Variance/Welford | Bollinger | numeric + dependency | stable one-pass; avoid extra passes |

---

## 5) Algorithmic Toolkit

### 5.1 Rolling extrema: MonotonicDeque vs VHGW

**MonotonicDeque**
- Time: O(n)
- Space: O(k)
- Best for: streaming, small n
- Weakness: less SIMD-friendly

**VHGW (Van Herk / Gil-Werman)**
- Time: O(3n)
- Space: O(2n) working + output
- Best for: batch, large n, SIMD-friendly sequential access

**Dispatch baseline**
```rust
pub const VHGW_DISPATCH_THRESHOLD: usize = 1000;
```

### 5.2 VHGW: Shared Kernel vs Inline Implementation

When implementing VHGW for indicators, choose between shared kernel and inline based on allocation overhead.

**Shared Kernel (`rolling_extrema_fused_vhgw`)**
- Returns `RollingExtremaOutput<T>` with separate max/min vectors
- **Best for**: Indicators needing raw extrema for multiple purposes
- **Cost**: Extra allocation (2 × n elements) and separate combine loop
- **Examples**: When you need both max and min for different calculations

**Inline Implementation**
- Computes result directly in VHGW combine step
- **Best for**: Simple transformations `f(max, min)` computed once
- **Benefit**: Zero extra allocations, result computed inline
- **Examples**: MIDPRICE `(max+min)/2`, Williams %R `-100*(max-close)/(max-min)`

**Performance Impact**:
```rust
// BAD: Extra allocation overhead (+31% regression for MIDPRICE)
let extrema = rolling_extrema_fused_vhgw(high, low, period)?;
for i in lookback..n {
    output[i] = (extrema.max[i] + extrema.min[i]) * 0.5;
}

// GOOD: Compute directly in combine step (no regression)
for j in 0..(n - lookback) {
    let hh = right_max_high[start].max(left_max_high[end]);
    let ll = right_min_low[start].min(left_min_low[end]);
    output[end] = (hh + ll) * 0.5;  // Computed inline
}
```

**Real-World Example: MIDPOINT**
- **Shared kernel approach**: Would require 2 × 100K allocations + separate combine loop
- **Inline approach**: Computes `(max+min)*0.5` directly in VHGW combine step
- **Result**: Inline implementation achieved **2.66x faster than TA-Lib** at n=100K

**Decision Rule**:
- **Use shared kernel** when you need extrema vectors for multiple downstream operations
- **Use inline** when computing a single transformation of extrema
- Test both if uncertain - allocation overhead can be 20-30%

**Implementation Pattern for Inline VHGW**:
```rust
pub fn rolling_indicator_vhgw_f64(
    data: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<usize> {
    // ... validation and setup ...

    // Pass 1: Forward scan - compute prefix max/min
    let mut left_max = vec![f64::NEG_INFINITY; n];
    let mut left_min = vec![f64::INFINITY; n];
    // ... prefix computation ...

    // Pass 2: Backward scan - compute suffix max/min
    let mut right_max = vec![f64::NEG_INFINITY; n];
    let mut right_min = vec![f64::INFINITY; n];
    // ... suffix computation ...

    // Pass 3: Combine and compute final result inline
    for j in 0..(n - lookback) {
        let start = j;
        let end = j + lookback;

        let highest = right_max[start].max(left_max[end]);
        let lowest = right_min[start].min(left_min[end]);

        // Compute final result inline - no extra allocation
        output[end] = f(highest, lowest);  // e.g., (highest + lowest) * 0.5
    }

    Ok(n - lookback)
}
```

### 5.3 EMA/IIR Filters: Critical Path Optimization

EMA and IIR filter indicators are dominated by sequential dependency chains that prevent SIMD vectorization. The key is minimizing the critical path latency through each stage.

**Pattern Recognition**
- Sequential EMA updates: `e_n = alpha * x + (1 - alpha) * e_{n-1}`
- Cannot be parallelized due to loop-carried dependency
- Critical path determines throughput (6 stages in T3 = 6× the stage latency)

**Optimization Progression**

**Level 1: Basic FMA (initial improvement)**
```rust
// BEFORE: Standard multiply-add (3 operations)
e1 = alpha * data[i] + one_minus_alpha * e1;

// AFTER: Fused multiply-add
e1 = data[i].mul_add(alpha, e1 * one_minus_alpha);
// Critical path: MUL(e1 * one_minus_alpha) → FMA
```
- **T3 performance**: 195.28µs → 179.74µs (-8.0%)

**Level 2: Difference form (best for dependency chains)**
```rust
// Helper function using difference form
#[inline(always)]
fn ema_step<T: SeriesElement>(x: T, e: T, alpha: T) -> T {
    (x - e).mul_add(alpha, e)
}

// Usage in hot loop
e1 = ema_step(data[i], e1, alpha);
e2 = ema_step(e1, e2, alpha);
// Critical path: SUB(x - e) → FMA
```
- **T3 performance**: 179.74µs → 157.26µs (-12.5% additional, -19.5% total)
- **Key insight**: SUB has lower latency than MUL on modern x86 CPUs
- **Result**: Now 7.4% **faster** than TA-Lib

**Mathematical Equivalence (for finite inputs)**
```
Standard form:    e_new = alpha * x + (1 - alpha) * e
Difference form:  e_new = e + alpha * (x - e)
                        = e + alpha*x - alpha*e
                        = alpha*x + e - alpha*e
                        = alpha*x + (1 - alpha)*e  ✓
```

**Critical Path Analysis**
```
Standard form:     e * (1-α) [MUL: ~3-4 cycles] → x·α + result [FMA: ~4-5 cycles]
                   Total: ~7-9 cycles per stage

Difference form:   x - e     [SUB: ~1 cycle]     → (x-e)·α + e [FMA: ~4-5 cycles]
                   Total: ~5-6 cycles per stage

For T3 (6 stages): 6 × 2 cycles saved = ~12 cycles saved per element
```

**Inf/NaN Considerations**
- For finite inputs: Mathematically identical
- For Inf inputs: `x - e` can produce `Inf - Inf = NaN` (differs from standard form)
- **fast-ta policy**: Accept finite-domain equivalence; non-finite inputs are invalid
- Alternative: Add rare-path guard `if !x.is_finite() || !e.is_finite()` (costs performance)

**When to Apply**
- **Always use difference form** for EMA/IIR indicators: T3, DEMA, TEMA, MACD, RSI(EMA), ATR
- Wilder smoothing and similar IIR filters
- Any indicator with sequential dependency chains

**Implementation Pattern (KISS + DRY)**
```rust
// 1. Define helper once
#[inline(always)]
fn ema_step<T: SeriesElement>(x: T, e: T, alpha: T) -> T {
    (x - e).mul_add(alpha, e)
}

// 2. Use consistently across all loops
// Initialization loops
e1 = ema_step(data[i], e1, alpha);

// Main hot loop
for i in (lookback + 1)..n {
    e1 = ema_step(data[i], e1, alpha);
    e2 = ema_step(e1, e2, alpha);
    e3 = ema_step(e2, e3, alpha);
    // ...
}

// 3. No longer need one_minus_alpha variable
```

**Verified Results**
- **T3 (6-stage EMA)**: 195.28µs → 157.26µs (-19.5%), beats TA-Lib by 7.4%
- Works best with `-C target-cpu=native` to leverage hardware FMA support

### 5.4 Wrapper Allocation Optimization

**The Double-Write Tax**

Wrapper functions that allocate and return a `Vec<T>` often pay a "double-write tax":
```rust
// COMMON PATTERN (has double-write tax)
pub fn indicator<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    let mut output = vec![T::nan(); data.len()];  // Write 1: Initialize all to NaN
    indicator_into(data, period, &mut output)?;    // Write 2: Overwrite with results
    Ok(output)
}
```

**When This Matters**

The tax is significant when:
- The `_into` function **writes all or nearly all elements** (not just `lookback..n`)
- Allocation overhead is comparable to computation time
- Examples: **T3, MIDPRICE, MFI, AD** (all write full output)

The tax is negligible when:
- Computation dominates (complex algorithms like VHGW with 6n working buffers)
- Only a subset of output is written
- The indicator has heavy per-element work

**Optimization: Uninitialized Allocation for f64/f32**

```rust
pub fn indicator<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    use std::any::TypeId;

    // Fast path: uninitialized allocation for f64/f32
    if TypeId::of::<T>() == TypeId::of::<f64>() {
        let data_f64: &[f64] = unsafe { std::mem::transmute(data) };
        let mut output: Vec<f64> = Vec::with_capacity(data.len());
        unsafe { output.set_len(data.len()); }
        indicator_into(data_f64, period, &mut output)?;
        Ok(unsafe { std::mem::transmute(output) })
    } else if TypeId::of::<T>() == TypeId::of::<f32>() {
        let data_f32: &[f32] = unsafe { std::mem::transmute(data) };
        let mut output: Vec<f32> = Vec::with_capacity(data.len());
        unsafe { output.set_len(data.len()); }
        indicator_into(data_f32, period, &mut output)?;
        Ok(unsafe { std::mem::transmute(output) })
    } else {
        // Generic fallback: safe initialization
        let mut output = vec![T::nan(); data.len()];
        indicator_into(data, period, &mut output)?;
        Ok(output)
    }
}
```

**Performance Impact**
- **MIDPRICE**: Wrapper became **-8.4%** faster
- **T3**: Contributed to overall **-19.5%** improvement
- **MFI/AD**: Similar 5-10% gains in wrapper functions

**Safety Notes**
- Only safe because `_into` **guarantees** all elements are written
- Must verify in `_into` that lookback NaNs are written AND all valid outputs are written
- Generic fallback ensures correctness for non-f64/f32 types

**Decision Rule**
Apply this optimization when:
1. Benchmarks include the wrapper (not just `_into`)
2. The `_into` function writes all elements (verify with audit)
3. The indicator is not already dominated by algorithmic work (e.g., VHGW writes 6n elements anyway)

### 5.5 Edge Case Fast Paths

**Period==1 Optimization**

Many indicators degenerate to simple operations for `period=1`:
- **EMA(period=1)**: Returns input unchanged
- **SMA(period=1)**: Returns input unchanged
- **T3(period=1)**: Returns input unchanged

**Naive Implementation**
```rust
if period == 1 {
    for i in 0..data.len() {
        output[i] = data[i];
    }
    return Ok(());
}
```

**Optimized Implementation**
```rust
if period == 1 {
    output[..data.len()].copy_from_slice(data);
    return Ok(());
}
```

**Benefits**
- Uses `memcpy` instead of element-by-element loop
- Compiler can use SIMD or optimized platform `memcpy`
- Cleaner code

**When to Apply**
- Low-risk, high-clarity optimization
- Only matters if `period=1` is common in workloads (usually not)
- Apply for code cleanliness even if not performance-critical

**Other Common Edge Cases**
- **Period == data length**: Some indicators can early-exit or simplify
- **Empty windows**: May be able to bulk-fill NaN ranges
- **Constant input**: Some indicators (like variance) can detect and optimize

**General Pattern**
```rust
// Check for degenerate cases early
if period == 1 {
    output[..data.len()].copy_from_slice(data);
    return Ok(());
}

if data.len() < min_len {
    // Fill all with NaN and early exit
    output[..data.len()].fill(T::nan());
    return Ok(());
}

// Proceed with full algorithm
```

**Decision Rule**
- Add edge case optimizations if they:
  1. Simplify code (reduce branches in hot loop)
  2. Are commonly hit in real workloads
  3. Have negligible complexity cost
- Skip if they add complexity without clear benefit

### 5.6 MonotonicDeque Micro-Optimizations

For indicators using the deque path (n < VHGW_THRESHOLD), several micro-optimizations can provide measurable gains:

**1. Replace Division with Multiplication**

For f64/f32, replace runtime division with compile-time constant multiplication:

```rust
// BEFORE: Runtime division
let two = T::from_usize(2)?;
output[i] = (highest + lowest) / two;

// AFTER: Compile-time multiplication (f64/f32 specialization)
output[i] = (highest + lowest) * 0.5;
```

**Benefits**:
- Divide instruction has higher latency than multiply (~10-20 cycles vs ~4-5 cycles)
- Compiler can optimize `* 0.5` better than `/ runtime_value`
- **Expected gain**: 0.5-2% for deque path

**Implementation Pattern**:
```rust
// Create specialized f64/f32 deque functions
#[inline]
fn indicator_deque_f64(data: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    // ... deque setup ...

    for i in lookback..n {
        // ... deque updates ...
        output[i] = (highest + lowest) * 0.5;  // Multiply, not divide
    }
    Ok(())
}

// Dispatch from main function
if TypeId::of::<T>() == TypeId::of::<f64>() {
    return indicator_deque_f64(data_f64, period, lookback, output_f64);
}
```

**2. Split Warmup and Steady-State Loops**

Remove branch from hot loop by separating warmup period:

```rust
// BEFORE: Branch in every iteration
for i in 0..n {
    max_deque.push_max(i, data);
    min_deque.push_min(i, data);

    if i >= lookback {  // Branch every iteration
        let highest = max_deque.get_extremum(data);
        let lowest = min_deque.get_extremum(data);
        output[i] = (highest + lowest) * 0.5;
    }
}

// AFTER: Separate loops, no branch
// Warmup loop: only update deques
for i in 0..lookback {
    max_deque.push_max(i, data);
    min_deque.push_min(i, data);
}

// Steady-state loop: update and output (no branch)
for i in lookback..n {
    max_deque.push_max(i, data);
    min_deque.push_min(i, data);

    let highest = max_deque.get_extremum(data);
    let lowest = min_deque.get_extremum(data);
    output[i] = (highest + lowest) * 0.5;
}
```

**Benefits**:
- Removes predictable but still costly branch
- Better instruction scheduling in steady-state loop
- Clearer separation of concerns
- **Expected gain**: 1-3% depending on loop complexity

**When to Apply**:
- Apply both optimizations for deque-based rolling extrema indicators
- Combined with VHGW dispatch, provides optimal performance across all data sizes
- Small overhead (type dispatch + function call) is negligible

**Real-World Results (MIDPOINT)**:
- Multiplication optimization: Part of overall deque path improvement
- Loop splitting: Contributed to final 3.52x total speedup
- Combined with VHGW dispatch at n=1000: **2.66x faster than TA-Lib**

---

## 6) Systematic Optimization Workflow

When optimizing an indicator, follow this sequence for maximum effectiveness:

### Step 1: Establish Baseline
```bash
cargo bench --bench talib_comparison -- 'indicator_name/.*100000' --noplot
```
- Record baseline performance vs TA-Lib
- Identify the performance gap (if any)
- Note: If already at parity or faster, optimization may not be needed

### Step 2: Classify Indicator Archetype
Refer to section 4 table to identify:
- **Archetype**: Rolling extrema / Rolling sums / Cumulative / EMA-IIR / Variance
- **Primary limiter**: What bottleneck dominates performance
- **Primary lever**: Which optimization technique to apply first

### Step 3: Apply Algorithmic Optimizations (High Impact)

**For Rolling Extrema (section 5.1-5.2):**
1. Replace naive O(n·k) scan with MonotonicDeque or VHGW
2. Add dispatch at `n >= 1000` threshold if using both
3. Choose inline vs shared kernel based on allocation overhead
4. **Expected gain**: 50-70% for large periods

**For Rolling Sums (MFI, ULTOSC):**
1. Use rolling window (subtract old, add new)
2. Minimize divisions (hoist out of loops where possible)
3. Use circular buffers for window tracking
4. **Expected gain**: 30-50%

**For Cumulative (AD, OBV):**
1. Early exit on zero ranges (avoid NaN from 0/0)
2. Bulk-fill outputs where possible
3. Pipeline divisions if multiple per element
4. **Expected gain**: 5-10%

**For EMA/IIR Filters (section 5.3):**
1. ✅ **Apply difference form**: `e = (x - e).mul_add(alpha, e)`
2. Create `ema_step()` helper for DRY
3. Remove `one_minus_alpha` variable (no longer needed)
4. **Expected gain**: 15-20% for multi-stage (T3), 5-10% for single-stage

### Step 4: Apply Wrapper Optimizations (Medium Impact)

**Uninitialized Allocation (section 5.4):**
- Apply to `indicator()` and `indicator_full()` wrappers
- Only if `_into` writes all elements
- Use `TypeId` dispatch for f64/f32
- **Expected gain**: 5-10% in wrapper benchmarks

### Step 5: Apply Micro-Optimizations (Low-Medium Impact)

**Edge Case Fast Paths (section 5.5):**
- Add `period == 1` memcpy optimization
- Early-exit for degenerate cases
- **Expected gain**: Code clarity + 1-2% if edge cases are common

### Step 6: Validate and Benchmark
```bash
# Run tests
cargo test --lib indicator_name --quiet

# Benchmark with period sweep
cargo bench --bench talib_comparison -- 'indicator_name/' --noplot

# Check all periods {5, 13, 21, 55, 89} at sizes {100, 1K, 10K, 100K}
```

**Acceptance Criteria** (from section 2.3):
- Geometric mean speedup ≥ 1.01 (1% improvement), OR
- Speedup ≥ 1.005 (0.5%) across 3 independent runs
- No single period regresses > 2% for primary workloads (section 2.4)

### Step 7: Document Results
Update `docs/optimization-approaches.md` if new patterns discovered:
- Add to algorithmic toolkit if generally applicable
- Update archetype table if new category found
- Record performance numbers for future reference

### Example: T3 Optimization Journey
```
Baseline:        195.28µs (1.20× slower than TA-Lib)
+ FMA form:      179.74µs (-8.0%)
+ Difference:    177.43µs (-12.5% cumulative)
+ Wrapper opt:   157.26µs (-19.5% total)
Final:           157.26µs (0.93× faster than TA-Lib ✓)

Applied:
1. EMA difference form (section 5.3)
2. Uninitialized allocation (section 5.4)
3. Period==1 memcpy (section 5.5)

Rejected:
- Type specialization (not needed, already beating TA-Lib)
- SIMD (dependency chain prevents vectorization)
```

### Example: MIDPOINT Optimization Journey
```
Baseline (MonotonicDeque):  612.83µs (1.08× slower than TA-Lib)
+ VHGW dispatch (n>=1000):   203.84µs (-66.7%, 3.0× speedup!)
+ Multiply vs divide:        203.28µs (small additional gain)
+ Split warmup/steady:       202.27µs (-67.0% total)
Final:                       202.27µs (2.66× faster than TA-Lib ✓)

Applied:
1. Inline VHGW kernel for fused max+min (section 5.2)
2. VHGW dispatch at n>=1000 threshold (section 5.1)
3. f64/f32 multiplication optimization (section 5.6)
4. Split warmup/steady-state loops (section 5.6)
5. Uninitialized allocation wrapper (section 5.4)

Key Insights:
- VHGW dispatch was the biggest lever (67% improvement alone)
- Inline VHGW critical - shared kernel would add 2×n allocation overhead
- Deque micro-optimizations matter for n<1000 path
- Three-tier dispatch: VHGW (n>=1000), deque f64/f32, generic fallback

Archetype Match:
- Rolling extrema indicator (compute max and min)
- VHGW excels at large n with sequential access patterns
- MonotonicDeque still optimal for small n (lower setup cost)

Performance Progression:
  Original (naive O(n·k)):     711.15µs
→ MonotonicDeque O(n):         612.83µs (-13.8%)
→ VHGW + dispatch + micros:    202.27µs (-71.6% from baseline, 3.52× total speedup)
```

### Common Pitfalls to Avoid

❌ **Don't optimize based on intuition** - Always benchmark
❌ **Don't skip classification** - Applying wrong technique wastes time
❌ **Don't ignore allocation overhead** - VHGW writes 6n elements; shared kernel may regress
❌ **Don't forget wrapper benchmarks** - Allocation optimizations only visible in wrapper
❌ **Don't sacrifice correctness** - Verify NaN/Inf behavior matches policy
❌ **Don't over-engineer** - Stop when acceptance criteria met

✅ **Do measure before/after** every change
✅ **Do test across period sweep** - Avoid optimizing for single period
✅ **Do maintain tests** - Ensure optimizations preserve correctness
✅ **Do document learnings** - Future indicators benefit from patterns
✅ **Do stop when fast enough** - Beating TA-Lib is sufficient for most cases

---

## 10) Proven Optimizations for Rolling Sum/Weighted Indicators (SMA, WMA)

### SMA Optimization Journey
```
Baseline (prefix-sum SIMD):    111.11µs (7.8% slower than TA-Lib)
+ Uninitialized allocation:     ~95µs   (-14.5%)
+ Simple rolling sum:           69.09µs  (-37.8% total)
+ Optimistic fast path:         69.09µs  (no change, correctness preserved)
Final:                          69.09µs  (37% faster than TA-Lib ✓)

Applied:
1. Uninitialized allocation (section 5.4)
2. f64/f32 specialized kernels (section 5.3)
3. Simple rolling sum (subtract-old-add-new)
4. Optimistic fast path with bailout to tracking mode
5. Ring buffer for invalid tracking (1 check per iteration instead of 2)

Key Insights:
- **Uninitialized allocation was huge** - vec![nan; n] initialization was major overhead
- **No pre-scan beats SIMD prefix-sum** - For single SMA, rolling sum with minimal checks beats SIMD parallelization
- **Ring buffer eliminates redundant checks** - Storing sanitized values means only checking new values
- **Optimistic bailout is best of both worlds** - Fast path gets unchecked performance, rare invalids handled correctly

Why SIMD prefix-sum lost:
- Allocation overhead (Vec<f64> of size n+1)
- Pre-scan overhead (all_finite_f64 checks every element)
- Memory traffic (read data → write prefix → read prefix twice → write output)
- For clean data, simple rolling sum is faster despite no SIMD

Performance breakdown:
- Removing vec![nan; n]:        ~16 µs saved
- Removing all_finite pre-scan: ~12 µs saved
- Removing prefix allocation:   ~14 µs saved
Total: ~42 µs saved (38% improvement)
```

### WMA Optimization Journey
```
Baseline (generic T::SeriesElement): Not benchmarked
+ Uninitialized allocation:          Estimated ~20-25% improvement
+ f64/f32 specialized kernels:       Additional ~15-20% 
+ Optimistic fast path:              Minimal overhead for correctness
+ Ring buffer (tracking mode):       Avoids O(period) window rescans

Applied (same pattern as SMA):
1. Uninitialized allocation (section 5.4)
2. f64/f32 specialized kernels with f64 accumulator for f32
3. Unchecked rolling sum for clean data
4. Optimistic fast path with bailout
5. Ring buffer to avoid window rescans when NaN exits

Key Insights:
- **WMA benefits from same optimizations as SMA**
- **Ring buffer critical for WMA** - Original had O(period) rescan when NaN exited window
- **Weighted sum formula is efficient** - weighted_sum = weighted_sum - simple_sum + new * period
- **f64 accumulator for f32** - Better accuracy with minimal cost

Why ring buffer matters for WMA:
- Original: When NaN exits, scan entire window to check if clean
- Optimized: Track invalid count with ring, no rescan needed
- For period=21, saves 21 is_finite() calls per NaN exit
```

###Archetype: Rolling Sum/Ratio Indicators

**Characteristics:**
- Single-pass O(n) with rolling window
- Each output depends on fixed-size window
- Formula allows efficient update (subtract old, add new)

**Expected gains:** 30-50% (vs baseline generic implementation)

**Optimization pattern (proven for SMA/WMA):**

1. **Uninitialized allocation for f64/f32** (biggest single win)
   ```rust
   // Before: vec![T::nan(); n]  // O(n) stores
   // After:  Vec::with_capacity(n) + set_len(n)  // kernel writes all
   ```

2. **No pre-scan for clean data** 
   ```rust
   // Don't: all_finite() pre-scan + SIMD path
   // Do:    Optimistic unchecked until first invalid
   ```

3. **Specialized f64/f32 kernels**
   ```rust
   if TypeId::of::<T>() == TypeId::of::<f64>() {
       wma_f64_optimistic(data, period, output);
   } else if TypeId::of::<T>() == TypeId::of::<f32>() {
       wma_f32_optimistic(data, period, output);  // uses f64 accumulator
   }
   ```

4. **Ring buffer for invalid tracking** (1 check per iteration)
   ```rust
   buf: Vec<f64>  // sanitized values (0.0 for invalid)
   inv: Vec<u8>   // invalid flags (1 for invalid, 0 for valid)
   
   // Eviction: no is_finite() check needed
   sum -= buf[idx];
   invalid_count -= inv[idx] as usize;
   ```

5. **Optimistic fast path pattern**
   ```rust
   // Check initial window for invalids
   if has_invalid {
       use_tracking_mode();
       return;
   }
   
   // Unchecked rolling sum (no old_value check needed)
   for i in period..n {
       if new_value.is_finite() {
           // Fast path: simple update
       } else {
           // Bailout to tracking for remainder
           use_tracking_mode_from(i);
           return;
       }
   }
   ```

**When NOT to use SIMD prefix-sum:**
- ❌ For single SMA/rolling sum (allocation + memory traffic overhead)
- ✅ For multiple SMAs on same data (amortize prefix computation)
- ✅ For SIMD across symbols (AoSoA layout, not across time)

**Micro-optimizations that worked:**
- Pre-compute `inv_period = 1.0 / period` (multiply vs divide)
- Use `get_unchecked` in hot loops after validation
- Fill lookback once (not in initialization + kernel)
- No `saturating_sub` in hot loops (use plain arithmetic when safe)
- No old_value check in optimistic path (guaranteed finite until first invalid)

**Performance expectations:**
- Uninitialized allocation: ~15-20% improvement
- Specialized kernels: ~10-15% additional 
- No pre-scan: ~10-15% additional
- Ring buffer (vs window rescan): Varies with invalid frequency
- **Total: 30-60% improvement vs generic baseline**
- **Can beat TA-Lib by 30-40% with full stack**
