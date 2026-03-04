# Python Binding Foundation (Stage 0) — Registry and Macro-Onboarding

## Goal

Create a predictable, AI-readable onboarding path for Python bindings so new indicators are added by editing declarative metadata plus a shared wrapper pattern.

## Design decision

- Use one registry entry per exposed indicator in Rust (`registry.rs`) for metadata and mapping intent.
- Keep existing manual wrappers temporarily for compatibility while new indicators are added through the registry-backed path.
- Use macros for common signature patterns (`single_output_series_indicator!`) to keep boilerplate low and behavior consistent.
- Preserve current Python runtime behavior (`out` zero-copy + contiguous-array errors + `ValueError` mapping).

## Foundation schema

Each registry entry includes:

- `name` — Python function name
- `category` — indicator group (`moving_average`, `momentum`, ...)
- `input_shape` — one of `Series<f64>`, `OHLC`, `OHLCV`
- `inputs` — required input argument names
- `params` — optional tuning parameters
- `outputs` — return values
- `supports_out` — whether `out=` is supported
- `callable_target` — canonical Rust binding name

## Onboarding checklist

1. Add registry descriptor to `liq-ta/crates/liq-ta-python/src/registry.rs`.
2. Add wrapper implementation using existing `*_series_indicator` style (macro path for new indicators).
3. Add wrapper registration in foundation registration helper.
4. Export symbol via module init or a registration list.
5. Add/update stub metadata and Python metadata alignment checks.
6. Add a small test covering:
   - registry lookup + schema validation
   - runtime function export
   - stub and metadata alignment

## Why this is AI-readable

The onboarding format is intentionally small and declarative:

- One-line indicator descriptor in Rust registry.
- One macro invocation for the Python wrapper.
- One registration entry in the foundation list.
- Metadata is derived from the same descriptor source.

This keeps search-and-edit behavior predictable for agents: to onboard a new indicator,
the agent edits the registry and one wrapper definition without rediscovering dozens of repetitive patterns.

## Why this is the pit of success

This approach minimizes accidental divergence between:

- runtime behavior
- metadata
- Python API

while still allowing manual legacy bindings to stay stable during incremental migration.

## Stage 5 status (2026-02-25)

The architecture is now used for completed SQX parity and extension indicators, with
hardening in place for error determinism and metadata diagnostics.

### Deterministic user-facing error classes

Python package-level API now exposes stable, explicit error classes:

- `LiqTaError`
- `IndicatorNotFoundError`
- `IndicatorArgumentError`
- `IndicatorMetadataError`

These are raised by helper APIs such as `compute_indicator()`,
`require_indicator_info()`, and `validate_indicator_metadata()`.

## Extension pattern with concrete example

### 1) Add Rust registry descriptor (`src/registry.rs`)

```rust
IndicatorBindingDescriptor {
    name: "my_indicator",
    category: "momentum",
    input_shape: "Series<f64>",
    inputs: &["data"],
    params: &["period"],
    outputs: &["my_indicator"],
    supports_out: true,
    callable_target: "my_indicator",
},
```

### 2) Add binding wrapper (`src/lib.rs`)

Use the shared macro path for single-series outputs:

```rust
single_output_series_indicator!(
    my_indicator,
    liq_ta::indicators::my_indicator,
    liq_ta::indicators::my_indicator_into,
    "My Indicator."
);
```

Then ensure registration includes `my_indicator` in the module init list.

### 3) Sync package metadata/stubs

- Add metadata entry in `python/liq_ta/__init__.py` (or validate it from runtime registry).
- Ensure function appears in `python/liq_ta/_liq_ta.pyi`.
- Include it in `__all__`.

### 4) Add tests

- Rust: indicator behavior + validation/error paths.
- Python: shape/smoke test, metadata alignment, and error mapping.

## Legacy wrapper migration contract

- Existing manual `#[pyfunction]` wrappers remain supported for compatibility.
- New indicators should follow the registry/macro onboarding path by default.
- Legacy wrappers can be migrated incrementally when touched, without forced breakage.
