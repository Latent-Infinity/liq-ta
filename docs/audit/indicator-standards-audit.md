## Indicator Standards Audit (rust-code-standards.md)

Scope: indicator modules under `crates/liq-ta/src/indicators`, reviewed against `docs/rust-code-standards.md` with emphasis on ownership, allocations, iterator usage, error patterns, and hot-path discipline.

### Compliance highlights
- **Borrowing-first APIs**: Indicator entrypoints accept slices (`&[T]`) and provide `_into` variants for buffer reuse.
- **Preallocation in hot paths**: Most indicator implementations size output `Vec`s with `with_capacity` or `len`-exact allocation.
- **Typed errors**: Validation failures consistently return `Error` variants instead of `Result<T, String>`.
- **Iterator pipelines**: In hot code, loops are generally explicit and allocation-free; iterator chains are mostly in tests.

### Findings (by priority)
#### High
- None observed.

#### Medium
- None observed.

#### Low
- **Unused/experimental kernels trigger warnings** (violates "avoid dead code in hot modules"):
  - SIMD and unchecked SMA variants are defined but not called, leaving unused import/constants. `crates/liq-ta/src/indicators/sma.rs:50`
  - Unchecked WMA kernels are defined but unused; they introduce unused locals and warnings. `crates/liq-ta/src/indicators/wma.rs:65`
  - Alternate MFI branchless kernel and helper types are unused. `crates/liq-ta/src/indicators/mfi.rs:94`
- **Unsafe blocks without explicit SAFETY notes**: Many unsafe slices/transmutes lack adjacent `// SAFETY:` comments, making safety invariants harder to audit. Examples include type punning in `stochastic`, `kama`, `statistics`. `crates/liq-ta/src/indicators/stochastic.rs:404`
- **TODO left in production code**: `williams_r` contains a performance TODO without a tracking reference. `crates/liq-ta/src/indicators/williams_r.rs:573`
- **Test-only allocations without capacity**: A few tests build vectors with `Vec::new()` even when size is known. `crates/liq-ta/src/indicators/atr.rs:1458`

### Static analysis summary
- `cargo udeps --all-targets --all-features` reports unused dev-deps in `liq-ta-cli`: `rand`, `rand_chacha`. Consider removing or documenting as false positives. `crates/liq-ta-cli/Cargo.toml`
- `cargo test` warnings (subset):
  - `sma.rs`: unused SIMD import, unused constant, unused kernels.
  - `wma.rs`: unused locals in unchecked kernels.
  - `mfi.rs`: unused structs/functions for branchless eviction.
  - `kernels/simd.rs` test: `sum_qd` unused.

### Recommended cleanups (if desired)
- Remove or feature-gate unused SMA/WMA/MFI kernels to eliminate warnings and align with "keep hot paths lean."
- Add `// SAFETY:` comments around unsafe blocks, especially where `transmute` or `set_len` is used.
- Convert the `williams_r` TODO into a tracked issue or remove it.
- Remove unused CLI dev-deps (`rand`, `rand_chacha`) if not needed.

