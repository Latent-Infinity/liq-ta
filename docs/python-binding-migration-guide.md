# Python Binding Migration Guide (Stage 5)

This guide is the operational path for adding indicators via the registry-first Python
binding architecture and for safely migrating legacy/manual wrappers.

## Scope

- Rust core indicator implementation in `crates/liq-ta`
- Python binding registration in `crates/liq-ta-python/src`
- Python metadata/stub alignment in `crates/liq-ta-python/python/liq_ta`
- Rust + Python test coverage for behavior and diagnostics

## New indicator onboarding (registry-first path)

1. Implement Rust indicator surface in `crates/liq-ta/src/indicators/`:
   - `fn`
   - `_into`
   - `_lookback`
   - `_min_len`
   - input validation + actionable `liq_ta::Error` paths
2. Add descriptor in `crates/liq-ta-python/src/registry.rs`:
   - `name`, `category`, `input_shape`, `inputs`, `params`, `outputs`, `supports_out`, `callable_target`
3. Add Python wrapper in `crates/liq-ta-python/src/lib.rs`:
   - Prefer `single_output_series_indicator!` for series single-output cases
   - Map core errors through `PyValueError`
4. Register/export symbol in `_liq_ta` module initialization.
5. Update Python package surface:
   - `python/liq_ta/__init__.py` metadata (`INDICATORS`) and `__all__`
   - `python/liq_ta/_liq_ta.pyi` type stub signature
6. Add validation tests:
   - Rust indicator tests + error paths
   - Python shape/smoke/error tests
   - Metadata/stub alignment checks

## Minimal example (registry + wrapper)

```rust
// registry.rs
IndicatorBindingDescriptor {
    name: "my_indicator",
    category: "momentum",
    input_shape: "Series<f64>",
    inputs: &["data"],
    params: &["period"],
    outputs: &["my_indicator"],
    supports_out: true,
    callable_target: "my_indicator",
}

// lib.rs
single_output_series_indicator!(
    my_indicator,
    liq_ta::indicators::my_indicator,
    liq_ta::indicators::my_indicator_into,
    "My Indicator."
);
```

## Legacy manual-wrapper migration path

1. Keep legacy function exported during migration.
2. Add matching registry descriptor and metadata entry first.
3. Port manual wrapper to macro/shared path when possible.
4. Keep Python symbol name and signature stable unless versioned change is intentional.
5. Add/expand tests before removing old path.

Compatibility rule:
- do not break existing manual wrapper call sites during migration;
- migrate incrementally with behavior parity checks.

## Required validation commands

From `liq-ta/`:

```bash
cargo test --workspace
cargo llvm-cov --workspace --json --summary-only --output-path /tmp/stage5_cov.json
```

From `liq-ta/crates/liq-ta-python/`:

```bash
PYTHONPATH=python uv run pytest -q tests/test_indicators.py tests/test_stage3_surface_parity.py tests/test_stage4_hardening.py
```

## Hardening checks

- No unresolved binding TODO/FIXME debt in active binding surfaces.
- Deterministic error classes/messages for invalid indicator selection and arg-shape issues.
- Metadata drift detection remains green.
