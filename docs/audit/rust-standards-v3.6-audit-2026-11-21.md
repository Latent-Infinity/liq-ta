# liq-ta Rust standards audit and remediation, 2026-11-21 (follow-up 2026-11-22)

**Standard:** High-Performance Rust Coding Standards v3.6, reviewed 2026-11-21 (`Rust Code Standards.md` in the Software-Standards library; index `GUIDES.md`, last reviewed 2026-11-21).
**Scope:** the `liq-ta`, `liq-ta-cli` and `liq-ta-python` crates (Rust sources, manifests, CI); Python sources are out of scope. The unbuilt `talib_comparison.rs` benchmark, which held the repository's last `unsafe`, was deleted in the 2026-11-22 cleanup.
**Baseline revision:** `42e4828`. **Toolchain:** pin 1.98.1, MSRV 1.95, aarch64-unknown-linux-gnu.

## Findings and dispositions (ordered by impact)

| # | Rule | Evidence at baseline | Verdict at baseline | Remediation | Now |
|---|---|---|---|---|---|
| 1 | `RPS-232`, `RPS-236`, `RPS-099`, `RPS-230`, `RPS-239` | About 290 `unsafe` uses in `crates/liq-ta/src`: 192 `transmute`, 44 `from_raw_parts`, 50 `get_unchecked`. Only 10 `// SAFETY:` comments. No measured benefit recorded. | FAIL | **First pass (`df89e57`):** unchecked indexing became plain indexing, and `TypeId` slice views were confined to two documented `unsafe` blocks in `cast.rs`. **Follow-up:** the f32/f64 fast kernels are now generic over `T` and convert through the safe `cast::to_f64`/`from_f64` (`as` casts in num-traits). The `TypeId` check still picks the path, but no memory is reinterpreted. `unsafe_code = "forbid"` is set in the workspace lint table. Kernel pairs that were identical after the rewrite were merged (`ad`, `kama`, `midpoint`, `aroon`, `ema`, `wma`, `trima`, the stochastic VHGW kernels). The public `compute_stochastic_*_vhgw_{f64,f32}` and `kernels::simd::*_f64` names are kept as wrappers. | PASS: no `unsafe` anywhere in the compiled workspace, enforced by `forbid` |
| 2 | `RPS-234` | No Miri coverage for unsafe code | FAIL | Moot: no unsafe code remains (the interim Miri job was removed) | N/A |
| 3 | `RPS-128`, `RPS-129`, `RPS-247` | Workspace release profile set `panic = "abort"`, which also applied to the PyO3 extension, so a panic aborted the host Python process | FAIL | Removed. PyO3 converts panics into Python exceptions again. | PASS |
| 4 | `RPS-255` | `strip = true` removed the symbol table from shipped CLI and wheel artifacts | FAIL | `strip = "debuginfo"` | PASS |
| 5 | `RPS-263` | `overflow-checks` inherited, not decided | FAIL | `overflow-checks = false` set explicitly, with the reason | PARTIAL: decided, not benchmarked separately |
| 6 | `RPS-318`, `RPS-233`, `RPS-298` | No workspace lint table. 245 bare `#[allow]`; 36 of them suppressed allow-by-default groups (`pedantic`, `nursery`, `restriction`) to no effect. | FAIL | Section 22 lint table added, plus `clippy::allow_attributes = "deny"`, with `[lints] workspace = true` in every member. No-op and unfulfilled suppressions deleted; the rest converted to `#[expect(lint, reason = "...")]`. Lints the removed allows had hidden (`or_fun_call` ×30, `needless_collect` ×2, `redundant_clone` ×15, `format_push_string` ×2) fixed in code. | PASS: 0 `#[allow]` in compiled code |
| 7 | `RPS-313` | No coverage gate | FAIL | CI step `cargo llvm-cov --locked --features parallel,serde --fail-under-lines 90` | PASS: 94.99% lines locally |
| 8 | `RPS-122`, `RPS-123` | 10 `unwrap()` in non-test library code (`wma` ×2, `ema`, `aroon` ×2, `ao` ×2, `traits` ×3) | PARTIAL: invariants, but no message | `wma` now propagates with `?`; the others use `expect` with the invariant | PASS |
| 9 | `RPS-290` | CI Clippy covered only the default members, so `liq-ta-python` was not linted | PARTIAL | `cargo clippy --workspace` | PASS |
| 10 | `RPS-006`, `RPS-013`, `RPS-294` | No newer-compiler job and no release-mode tests | PARTIAL | Release-mode test step, plus non-blocking stable and beta release-mode jobs | PASS |
| 11 | `RPS-004`, `RPS-293` | The MSRV job ran `test --workspace`. That links the PyO3 test binary against libpython, which fails without it (reproduced on this host). | UNVERIFIED in CI | MSRV job tests the default members and checks `liq-ta-python` | PASS locally |

## Coverage of the remaining rule areas

| Area | Rules | Verdict | Note |
|---|---|---|---|
| Toolchain | `RPS-001`–`RPS-005`, `RPS-009`–`RPS-011` | PASS | Edition 2024, `rust-version = "1.95"`, pin 1.98.1 (carries the 1.98.1 fix), compiler recorded in CI and wheel metadata |
| Pin age | `RPS-262`, `RPS-307` | N/A | Pin unchanged; a move to 1.100.0 is a separate, benchmarked upgrade |
| Cargo 1.100 controls | `RPS-314`, `RPS-315` | N/A | Pinned Cargo is 1.98.1 |
| Errors | `RPS-116`, `RPS-121` | PASS | `thiserror` enum; no `Result<_, String>` |
| Async, locks, atomics, hashing, I/O services | Sections 9, 13, 14 | N/A | No async, shared state or hash maps on hot paths |
| CPU target | `RPS-224`–`RPS-228` | PASS | No `target_feature`, `std::arch` or `target-cpu=native` |
| Allocation on untrusted input | `RPS-028`, `RPS-049`, `RPS-203` | PASS (CLI) | The CLI bounds CSV fields (16 KiB) and row resources (commit `e99e02a`) |
| Benchmarks | `RPS-281`, `RPS-282` | PASS | Criterion with `black_box` on generated random-walk data |
| Dependency audit | `RPS-296` | PARTIAL | `cargo-deny` is installed locally but has no project config or CI job |
| Performance claims | `RPS-014`–`RPS-017`, `RPS-021` | See below | |

## Checks run (2026-11-22, final tree)

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` (pin 1.98.1): clean.
- `cargo test --locked --features parallel,serde,reference-checks-strict,precision-fast`: 2131 passed, and the same count in `--release` (without `reference-checks-strict`).
- `cargo +1.95.0 test --locked`: 2130 passed. `cargo +1.95.0 check -p liq-ta-python --all-targets`: ok.
- Bit-equivalence harness (throwaway, `/tmp/liqta-equiv`). It ran 30 indicators and their variants (sma, wma, ema, trima, t3, trix, kama, mom, roc ×4, midpoint, midprice, aroon, atr, cci, williams_r, mfi, bollinger, stochastic ×3, the 12 statistics functions, ad, obv) on `42e4828` and on the follow-up tree. Coverage: periods 2/5/14/30/200, f64 and f32, High and Fast precision, and inputs with and without NaN/±∞ holes. The 896 lines of `Debug` output (65 MB) were byte-identical.
- `cargo llvm-cov --locked --features parallel,serde --fail-under-lines 90`: 95.42% lines.
- `cargo doc --locked --no-deps --all-features`: no warnings.
- Python bindings (maturin release build, Python 3.12): 60 passed, 2 skipped.

## Performance record

**Objective:** no material throughput loss from removing `unsafe`. No project latency budget exists, so this records the effect; it is not a gate.

**Method (final, 2026-11-22):**
- Criterion `indicators` suite, limited to the 21 groups whose code changed, at sizes 100, 1,000, 10,000 and 100,000.
- Baseline: a `git archive` of `42e4828`. Candidate: the unsafe-free tree. Toolchain 1.98.1, each tree's own bench profile, `--warm-up-time 2 --measurement-time 5`.
- Pinned with `taskset -c 9` to one Cortex-X925 core (the host mixes X925 and A725 cores; the earlier unpinned runs could migrate between them).
- Runs were A (baseline), B (candidate), then A again, all compared with the first baseline. The repeated baseline (A/A) measures noise. Other jobs raised the load average to 6–19 during the runs.
- Raw output: `/tmp/b3_{pre,post,aa}.txt` (not archived).

| Result (84 benchmarks) | Count | Notable |
|---|---:|---|
| Clearly faster (beyond A/A noise) | 10 | `mom` −46% to −76% (the generic `lagged_sub_sanitize` loop now auto-vectorizes; results are bit-identical), `ad` −5% to −8%, `roc` 1,000–10,000 −7% to −8% |
| Within noise or no change | 66 | All `sma` sizes are now within ±1.1%: zeroed allocation restored on the f32/f64 paths. Mean change over all 84: −3.2% (A/A mean −0.4%). |
| Possibly slower | 8 | `aroon` 1,000–10,000 +4.7% to +6.2%, reproduced in four runs; `obv` 100–1,000 +5%, reproduced twice; `bollinger/100000` +4.3%, `stochastic/100000` +5.9% (A/A +3.4%), `trima/100` +2.3%, `wma/100` +2.1% |

**Controls and diagnosis:**
- **`aroon`:** the benchmark (period 25) takes the van Herk kernel, which differs from baseline only in how it reads the lookup table (baseline: `get_unchecked`). `.min(period)` and plain checked indexing measured the same. Precomputing a lookup in the lazy-rescan path also changed nothing, so both experiments were reverted. Bounds-check cost is therefore an unlikely cause; code layout under fat LTO is plausible [inference, not measured].
- **`obv`:** its fast kernel is the same code made generic, and only sizes under 10,000 move. Same likely cause [inference].
- **Earlier unpinned run:** the regressions it showed for `sma`, `cci/100`, `ema/100` and `mom/100` did not reproduce once pinned, or were fixed by restoring the zeroed allocation.
- **Unpinned A/A noise:** single 100,000-point runs moved by up to 25% between identical builds.

**Decision:** no `unsafe` code is retained. The residual differences are at most about 6% on two indicators and have no identified source-level cause.

## Not verified

- CI itself: the workflow changes were validated as YAML and their commands run locally, but no GitHub run was observed.
- Binary-size effect of the release-profile change (unwinding kept, symbols kept) is unmeasured.
- The 2026-11-21 interim measurement (unpinned, shared host) is superseded by the pinned run above.
