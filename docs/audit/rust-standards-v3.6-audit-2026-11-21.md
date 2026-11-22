# liq-ta Rust standards audit and remediation, 2026-11-21

**Standard:** High-Performance Rust Coding Standards v3.6, reviewed 2026-11-21 (`Rust Code Standards.md` in the Software-Standards library; index `GUIDES.md`, last reviewed 2026-11-21).
**Scope:** the `liq-ta`, `liq-ta-cli` and `liq-ta-python` crates (Rust sources, manifests, CI). Not in scope: Python sources, and `crates/liq-ta/benches/talib_comparison.rs`, which is excluded from every build (`autobenches = false`, no `[[bench]]` entry), so no compiler or lint sees it.
**Baseline revision:** `42e4828`. **Toolchain:** pin 1.98.1, MSRV 1.95, Miri on nightly-2026-10-01, all on aarch64-unknown-linux-gnu.

## Findings and dispositions (ordered by impact)

| # | Rule | Evidence at baseline | Verdict at baseline | Remediation | Now |
|---|---|---|---|---|---|
| 1 | `RPS-232`, `RPS-236`, `RPS-099`, `RPS-230`, `RPS-239` | About 290 `unsafe` uses in `crates/liq-ta/src`: 192 `transmute`, 44 `from_raw_parts`, 50 `get_unchecked`. Only 10 `// SAFETY:` comments. No measured benefit recorded. | FAIL | `TypeId` specializations now go through `crate::cast` (`slice_as`, `slice_as_mut`, and the safe, allocation-free `vec_into`). Unchecked indexing is replaced by narrowed slices and plain indexing, with arithmetic order unchanged. Dead `ema_core_f64_unchecked` removed. | PASS: the only `unsafe` is two documented blocks in `cast.rs` |
| 2 | `RPS-234` | No Miri coverage for unsafe code | FAIL | `cast::tests` run under Miri; new CI `miri` job | PASS |
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

## Checks run (2026-11-21)

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` (pin 1.98.1): clean.
- `cargo test --locked --features parallel,serde,reference-checks-strict,precision-fast`: 2132 passed. The same in `--release` (without `reference-checks-strict`): 2132 passed.
- `cargo +1.95.0 test --locked`: 2131 passed. `cargo +1.95.0 check -p liq-ta-python --all-targets`: ok.
- `cargo +nightly miri test -p liq-ta --lib cast::tests`: 3 passed, no UB reported.
- `cargo llvm-cov --locked --features parallel,serde --fail-under-lines 90`: 94.99% lines.
- `cargo doc --locked --no-deps --all-features`: no warnings.
- Python bindings (maturin release build, Python 3.12): 60 passed, 2 skipped.

## Performance record

**Objective:** no material throughput loss from removing `unsafe`. No project latency budget exists, so this records the effect; it is not a gate.
**Method:** Criterion `indicators` suite, only the 21 groups whose code changed, sizes 100 to 100,000. The baseline is a `git archive` of `42e4828` and the candidate is the working tree. Both used the same target directory, toolchain 1.98.1, aarch64 (20 cores), `--warm-up-time 1 --measurement-time 3`, and the workspace bench profile of each tree. The host was shared (load average about 5), with no CPU pinning or frequency control, so `RPS-022`/`RPS-023` are not met. Raw output: `/tmp/bench_pre.txt` and `/tmp/bench_post.txt` (not archived).

| Result (84 benchmarks) | Count | Notable |
|---|---:|---|
| Significant improvement | 30 | `ad` −8% to −17%, `roc` −8% to −11%, `sma/100` −20%, `wma/100` −22% |
| Within noise or no change | 43 | |
| Significant regression | 11 | `stochastic/100000` +10.4%, `cci/100` +7.6%, `aroon` 100–10,000 +4.9% to +6.6%, `sma` 1,000–100,000 +3.4% to +4.5%, `ema/100` +3.5%, `wma/10000` +1.3%, `mom/100` +1.5% |

**Controls:**
- Rebuilding the baseline without `panic = "abort"` left `sma`/`stochastic` within ±0.9% of the baseline. The release-profile change is therefore not the cause of those regressions; the bounds checks on data-dependent indices (deque fronts in `stochastic`, the lookup table in `aroon`) are the likely cause [inference].
- Narrowing `sma` to zipped slices (`RPS-097`) gave identical timings, so it was reverted.
- `aroon/100000` moved from −30% to −16% between runs on the same code, so single-run differences of a few percent are within this host's variance.

**Decision:** keep the safe code. Restoring `unsafe` in `sma`, `stochastic` or `aroon` would need a controlled measurement on a dedicated host plus the `RPS-229`–`RPS-235` obligations (safe reference, `SAFETY` per operation, Miri coverage) before merge.

## Not verified

- CI itself: the workflow changes were validated as YAML and their commands run locally, but no GitHub run was observed.
- Binary-size effect of the release-profile change (unwinding kept, symbols kept) is unmeasured.
