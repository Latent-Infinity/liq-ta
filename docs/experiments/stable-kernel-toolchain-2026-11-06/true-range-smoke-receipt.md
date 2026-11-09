# True-range fixture equality smoke

Status: `DEV_SMOKE_ONLY`. This is a same-input implementation comparison, not signal or market-research evidence.

- Authorized fixture: `liq-experiments/tests/fixtures/approved/ecology/bars/khc_20230421-20230424.parquet`, SHA-256 `d4479cdb37c33a93fd445f4311c9b773de3fee95fd619903bb06fd90234afb30`, 449 rows from 2023-04-21 through 2023-04-24. Approved before row access by `liq-docs/plans/rust-true-range-fixture-smoke-oracle-decision-2026-11-06.md`.
- Reused the committed, previously guarded fixture; no new provider or upstream-store read. Read timestamp/high/low/close only; confirmed ascending timestamps, preserved all 449 rows including gaps and session boundary.
- PyArrow 23.0.1 converted each high/low/close scalar via Python `float` to IEEE-754 f64 and packed rows as three little-endian f64 values. Converted input SHA-256 `ec1c48354a02ad3578a137d61c10c17b425bc362b3ea51f953f591eaeace2478`. Input bytes are not included here.
- `true_range_smoke.rs` SHA-256 `7e18b614ff696343552f7f370e823c999930a78764b2ef15378a7df8305536cb`; full compiler, source, target and release-profile identities appear in the companion README.
- The nightly baseline and Rust 1.98.1 candidate each wrote the 448 calculated f64 values as little-endian bytes, in original row order. `cmp -s` reported byte equality; both output SHA-256 values are `e6566a6a891241c0d52dfb2e80cdd75e56049b53575858c0bfce19682021a272`. No raw output or market values are included here.
- This comparison does not establish an adjustment lineage, session-aware indicator validity, numerical equivalence on other inputs or a performance claim.
