# PRD: Advanced API Expansion — Zero-Alloc Chaining for `liq-ta`

**Status:** Draft (Proposed)
**Owner:** liq-ta maintainers
**Target Release:** vNext (minor)
**Audience:** Library maintainers + performance-focused users

---

## 1. Executive Summary

This PRD defines an advanced API expansion for `liq-ta` to support **zero-alloc chaining** of technical indicators. The feature introduces a reusable `Workspace` with preallocated buffers and a handle-based series model (`SeriesId` / `SeriesRef`) to enable high-throughput multi-indicator workflows without heap allocation during computation.

The existing **Simple API** and **Buffer API** remain unchanged and authoritative for current usage. The new API is explicitly targeted at advanced users and high-performance pipelines (batch backtests, large-scale feature generation).

---

## 2. Problem Statement

Current indicator workflows typically require users to:

* allocate output vectors per indicator,
* create intermediate owned buffers when chaining indicators,
* incur unpredictable peak memory usage and additional allocations in multi-indicator workflows.

These costs compound in backtesting and feature pipelines where dozens to hundreds of indicators are computed across long series.

`liq-ta` needs a deterministic, high-performance workflow for composing indicators:

* **No additional allocations during compute**
* **Reusable buffers** across indicators and runs
* **Zero-alloc chaining** (workspace-backed intermediates)

---

## 3. Goals and Non-Goals

### 3.1 Goals

1. **Zero-alloc chaining**

   * No heap allocation/reallocation during indicator computation and chaining once the workspace is constructed.

2. **Deterministic memory**

   * Peak memory is bounded by workspace configuration: fixed series buffer pool + fixed scratch capacity.

3. **Preserve current semantics**

   * Full-length outputs with NaN prefix per `*_lookback` policy.
   * Multi-output indicators aligned and share lookback rules.
   * Error and `_into` semantics remain unchanged for current APIs.

4. **Support chaining and composition**

   * `Indicator A(close) -> intermediate -> Indicator B(intermediate)` without allocating a new `Vec` per step.

5. **Support multi-input and multi-output indicators**

   * Multi-input (OHLC, etc.) and multi-output (Stochastic K/D, etc.) supported without runtime hashing or heap allocation.

6. **Idiomatic Rust resource handling**

   * RAII-scoped reclamation of intermediate buffers via a `scope()` API.

### 3.2 Non-Goals

* Replace, deprecate, or weaken Simple/Buffer APIs.
* Add streaming/incremental update semantics (separate effort).
* Provide TA-Lib compatibility guarantees beyond the `liq-ta` spec.
* Guarantee “zero-copy” in the literal sense (we compute and write outputs). The goal is zero *additional allocation*.

---

## 4. Key Concepts and Terminology

* **Zero-alloc chaining:** once a workspace is built, indicator computations do not allocate on the heap.
* **Workspace:** an owning container of preallocated series buffers and scratch memory.
* **SeriesId:** a lightweight handle to a workspace-owned buffer.
* **SeriesRef:** handle + metadata (lookback, length).
* **Materialization:** explicit copy-out from workspace buffer into user-owned memory (opt-in).

---

## 5. Design Principles / Constraints

1. **No borrowed output slices in compute**

   * `compute()` must not return a borrowed slice tied to a mutable workspace borrow (prevents chaining).
   * Compute returns **handles** (`SeriesRef`).

2. **No hidden allocations in hot path**

   * No `HashMap`, no string lookups, no implicit `Vec` growth inside compute/run.
   * Any allocations required for planning are done outside the hot path.

3. **Deterministic buffer ownership**

   * Workspace buffers do not alias across live outputs (unless explicitly in-place, future optimization).
   * Intermediates are released automatically at scope end unless kept.

4. **Preserve lookback semantics**

   * Outputs remain full-length; NaN prefix rules remain as today.

---

## 6. Proposed API Surface

### 6.1 Core Types

```rust
pub trait SeriesElement: Copy + Default {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct SeriesId(u32);

#[derive(Copy, Clone, Debug)]
pub struct SeriesMeta {
    pub len: usize,
    pub lookback: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct SeriesRef {
    pub id: SeriesId,
    pub meta: SeriesMeta,
}

pub enum Input<'a, T> {
    Slice { data: &'a [T], lookback: usize },
    Series(SeriesRef),
}
```

### 6.2 Workspace

Workspace is built once, with deterministic capacity:

* `len`: series length
* `buffers`: number of fixed-length series buffers in the pool
* `scratch_bytes`: preallocated scratch arena for temp memory

```rust
pub struct Workspace<T> {
    // fixed-length buffers + free list
    // scratch arena (bump allocator)
}

impl<T: SeriesElement> Workspace<T> {
    pub fn builder(len: usize) -> WorkspaceBuilder;

    pub fn len(&self) -> usize;

    pub fn input<'a>(&self, data: &'a [T]) -> Result<Input<'a, T>, Error>;

    pub fn view(&self, s: SeriesRef) -> &[T];
    pub fn view_mut(&mut self, s: SeriesRef) -> &mut [T];

    pub fn scope<R>(&mut self, f: impl FnOnce(&mut Scope<'_, T>) -> Result<R, Error>)
        -> Result<R, Error>;
}
```

### 6.3 Scope (RAII for intermediates)

A scope tracks allocated intermediates and releases them automatically at scope end unless explicitly kept.

```rust
pub struct Scope<'ws, T> { /* ws + tracking */ }

impl<'ws, T: SeriesElement> Scope<'ws, T> {
    pub fn keep(&mut self, id: SeriesId);
}
```

**Contract:** Within a scope, indicator computation is allocation-free.

### 6.4 Indicator Operation Trait

Indicators can be executed with workspace-backed inputs and produce workspace-backed outputs.

```rust
pub trait IndicatorOp<T: SeriesElement> {
    fn name(&self) -> &'static str;

    // Planning hooks (for future pipeline compilation)
    fn scratch_bytes(&self) -> usize { 0 }
    fn supports_in_place(&self) -> bool { false }

    fn compute<'a>(&self, scope: &mut Scope<'_, T>, input: Input<'a, T>)
        -> Result<SeriesRef, Error>;
}
```

### 6.5 Materialization APIs

Materialization is explicit and outside the “zero-alloc chaining” guarantee.

```rust
impl<T: SeriesElement> Workspace<T> {
    pub fn materialize_into(&self, s: SeriesRef, out: &mut [T]) -> Result<(), Error>;
    pub fn materialize_vec(&self, s: SeriesRef) -> Vec<T>; // explicit allocation
}
```

---

## 7. Multi-Input and Multi-Output

### 7.1 Multi-Input

Use typed input structs rather than string maps.

```rust
pub struct OhlcInput<'a, T> {
    pub open:  Input<'a, T>,
    pub high:  Input<'a, T>,
    pub low:   Input<'a, T>,
    pub close: Input<'a, T>,
}
```

### 7.2 Multi-Output

Use fixed structs for outputs in the low-level API (allocation-free).

```rust
pub struct StochOut {
    pub k: SeriesRef,
    pub d: SeriesRef,
}
```

---

## 8. Example Workflow

```rust
let mut ws = Workspace::<f64>::builder(close.len())
    .buffers(8)
    .scratch_bytes(64 * 1024)
    .build();

let (rsi_ema, sma) = ws.scope(|s| {
    let close_in = ws.input(&close)?;

    let rsi = ops::Rsi::new(14).compute(s, close_in)?;
    let rsi_ema = ops::Ema::new(5).compute(s, Input::Series(rsi))?;

    let sma = ops::Sma::new(20).compute(s, ws.input(&close)?)?;

    s.keep(rsi_ema.id);
    s.keep(sma.id);

    Ok((rsi_ema, sma))
})?;

let rsi_ema_slice = ws.view(rsi_ema);
let sma_slice = ws.view(sma);
```

---

## 9. Optional Future Layer: Compiled Pipeline (DAG Builder)

A pipeline builder may be introduced later, but only as a layer that compiles down to numeric IDs and a fixed execution plan that:

* computes peak live buffers needed,
* computes scratch requirements,
* performs allocation during `compile()`,
* performs **zero allocations during `run()`**.

**Not required** for initial zero-alloc chaining delivery.

---

## 10. Error Handling

### New errors introduced

* `WorkspaceExhausted { requested, available }`
* `ScratchExhausted { requested, available }`
* `LengthMismatch`

Existing errors remain unchanged for Simple/Buffer APIs.

---

## 11. Performance Requirements

### Hard requirements

* **No heap allocation** during `IndicatorOp::compute()` execution after workspace creation.
* Workspace buffer pool and scratch arena are the only memory used during compute.
* No runtime hashing, no string-based lookups in the hot path.

### Soft requirements / future improvements

* Optional in-place compute for select indicators where safe.
* Optional “plan compilation” to compute minimal peak buffers.

---

## 12. Compatibility and Migration

* Simple API: unchanged.
* Buffer API: unchanged.
* Advanced API is additive; no breaking changes required.

---

## 13. Implementation Plan (High Level)

### Phase 1: Workspace Core

* Implement `WorkspaceBuilder`, series buffer pool, scratch arena.
* Implement `SeriesId`, `SeriesRef`, `Input`.
* Implement `Scope` with deterministic, allocation-free tracking (fixed capacity / pre-reserve).

### Phase 2: Indicator Integration

* Add `IndicatorOp<T>` implementations for a small set of core indicators first (SMA/EMA/RSI).
* Validate semantics match existing `_into` outputs and lookback policies.

### Phase 3: Expand Coverage

* Add multi-input examples (e.g., ATR from OHLC).
* Add multi-output example (e.g., Stochastic K/D).

### Phase 4: Benchmarks + Gates

* Microbench:

  * baseline: Buffer API with user-managed allocations
  * advanced: Workspace API with chaining
* Confirm:

  * allocation count = 0 during compute
  * stable wall time improvements and reduced peak RSS

---

## 14. Acceptance Criteria

1. **Correctness**

* Advanced API outputs match existing `_into` results for supported indicators (within established floating tolerance).
* Lookback and NaN prefix rules identical to current behavior.

2. **Allocation**

* With a constructed workspace, indicator chaining performs **0 heap allocations** during compute.
* Tests include allocation guards (feature-gated) in CI.

3. **Determinism**

* Peak memory bounded by configured `buffers` and `scratch_bytes`.
* Exhaustion produces clear actionable errors.

4. **Ergonomics**

* `scope()` enables common chaining workflows without manual `release()`.

---

## 15. Open Questions

1. **Scope tracking without allocation**

* Preferred approach: fixed-capacity tracking stored in workspace or `scope_with_capacity(max_live)`.

2. **Scratch sizing strategy**

* Expose scratch sizing explicitly only, or provide a conservative default?

3. **In-place support**

* Do we introduce in-place capability now (opt-in per op), or later?

4. **Multi-input API**

* Standardize OHLC containers early or keep it per-indicator initially?

5. **Pipeline layer**

* When/if introduced, define compile-time vs run-time guarantees precisely (especially no allocation during `run()`).


