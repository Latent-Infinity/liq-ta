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
| 5 | `RPS-263` | `overflow-checks` inherited, not decided | FAIL | `overflow-checks = false` set explicitly, with the reason | PASS: decided and benchmarked; checks on cost up to +7% (`ad`, `sma/100000`); see the overflow-checks record |
| 6 | `RPS-318`, `RPS-233`, `RPS-298` | No workspace lint table. 245 bare `#[allow]`; 36 of them suppressed allow-by-default groups (`pedantic`, `nursery`, `restriction`) to no effect. | FAIL | Section 22 lint table added, plus `clippy::allow_attributes = "deny"`, with `[lints] workspace = true` in every member. No-op and unfulfilled suppressions deleted; the rest converted to `#[expect(lint, reason = "...")]`. Lints the removed allows had hidden (`or_fun_call` ×30, `needless_collect` ×2, `redundant_clone` ×15, `format_push_string` ×2) fixed in code. | PASS: 0 `#[allow]` in compiled code |
| 7 | `RPS-313` | No coverage gate | FAIL | CI step `cargo llvm-cov --locked --features parallel,serde --fail-under-lines 90` | PASS: 94.99% lines locally |
| 8 | `RPS-122`, `RPS-123` | 10 `unwrap()` in non-test library code (`wma` ×2, `ema`, `aroon` ×2, `ao` ×2, `traits` ×3) | PARTIAL: invariants, but no message | `wma` now propagates with `?`; the others use `expect` with the invariant | PASS |
| 9 | `RPS-290` | CI Clippy covered only the default members, so `liq-ta-python` was not linted | PARTIAL | `cargo clippy --workspace` | PASS |
| 10 | `RPS-006`, `RPS-013`, `RPS-294` | No newer-compiler job and no release-mode tests | PARTIAL | Release-mode test step, plus non-blocking stable and beta release-mode jobs | PASS |
| 11 | `RPS-004`, `RPS-293` | The MSRV job ran `test --workspace`. That links the PyO3 test binary against libpython, which fails without it (reproduced on this host). | UNVERIFIED in CI | MSRV job tests the default members and checks `liq-ta-python` | PASS: observed green on GitHub (run 45046639206) |
| 12 | CI | `Build wheels (ubuntu-latest)` had failed on every push since `d6871e9`: writing `rustc-version.txt` into the container-owned `dist/` was denied | FAIL (observed) | The runner creates `dist/` and records the compiler version before maturin runs | PASS: all 17 CI jobs green on GitHub at `3ab9127`, including wheels on Linux, macOS and Windows, and `cargo-deny` |
| 13 | `_into` contract | `midpoint_into` on the `n >= 1000` fast path left the caller's buffer contents in windows containing invalid values | FAIL (reproduced) | The kernel writes NaN. A new test checks all 166 public `*_into` functions on pre-dirtied buffers against their allocating twins (f32/f64, n = 50 and 1500, with and without holes). | PASS |

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
| Dependency audit | `RPS-296` | PASS | `deny.toml` and a CI `cargo-deny` job. Three advisories were resolved by lockfile updates: crossbeam-epoch 0.9.21 (RUSTSEC-2026-0204), rand 0.8.6 and 0.9.5 (RUSTSEC-2026-0097). `liq-ta-python` is `publish = false`. |
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
- **`aroon` (measured 2026-11-22):** hardware counters were unavailable (see Not verified), so instruction counts come from the disassembly of the fat-LTO bench binaries, where the kernel is inlined into `Bencher::iter`. In the van Herk kernel the prefix and suffix inner loops are the same length at both revisions (32 and 33 instructions per element). Pass 3 (combine and look up) grows from 38–40 to 46–48 instructions per output. Both `.min(period)` (cmp and csel, twice) and the lookup bounds check (cmp and branch, twice) remain: LLVM does not use the `min` to drop the check in this function. Per element that is about 103 → 111 instructions (+8%), so HEAD runs more instructions; the cause is not layout. A scratch build with plain `aroon_lookup[ps]` (no `.min`) runs 44–46 instructions per output. Pinned timings over three rounds were: `aroon/1000` 4.37–4.51 µs (base), 4.67–4.92 µs (HEAD), 4.55–4.68 µs (plain index); `aroon/10000` 43.2–44.5, 45.9–47.7 and 44.7–47.3 µs. The plain-index gain of 1–3% is within run-to-run spread. The remaining cost is the safe bounds check itself, which cannot be removed without `unsafe` or a different index proof.
- **`obv` (measured 2026-11-22):** the hot loop is instruction-for-instruction the same. The difference is one extra `malloc` and `free` per call. `obv()` creates `Vec::with_capacity(n)`, and `compute_obv_core` then replaces it with `*output = vec![T::zero(); n]`. At `42e4828`, LLVM elided the equivalent dead allocation; at HEAD it does not. A scratch build using `output.resize(close.len(), T::zero())` removes the extra allocation. Pinned timings: `obv/100` 114.1–116.7 ns (base), 121.2–122.4 ns (HEAD, +5% to +7%), 92.4–93.3 ns (fix, −19% vs base); `obv/1000` 874.7–913.0 ns, 925.2–928.0 ns and 871.0–890.1 ns. **Proposed source fix:** that one-line change in `compute_obv_core` (`crates/liq-ta/src/indicators/obv.rs`, the f64 branch). It is not applied here.
- **Method for both:** a clean `git archive` of HEAD (its aroon and obv codegen matches the working-tree build except for addresses) against `42e4828`, built with `CARGO_PROFILE_BENCH_STRIP=debuginfo` so symbols exist. Runs were base, HEAD, fix ×3, with `taskset -c 9`, `--warm-up-time 2 --measurement-time 5`, and load average 13–16. The round-1 base `obv` samples overlapped another pinned job and were discarded.
- **Earlier unpinned run:** the regressions it showed for `sma`, `cci/100`, `ema/100` and `mom/100` did not reproduce once pinned, or were fixed by restoring the zeroed allocation.
- **Unpinned A/A noise:** single 100,000-point runs moved by up to 25% between identical builds.

**Decision:** no `unsafe` code is retained. The `obv` residual is fixed: `obv()` now resizes its preallocated buffer instead of allocating a second one; bit-equivalence was re-run and is identical. The `aroon` residual (about +5%) is the cost of the safe lookup bounds check.

### Overflow checks (`RPS-263`), measured 2026-11-22

- **Builds:** the HEAD working tree (`748a2dc` plus another agent's uncommitted `midpoint.rs`/`rolling_extrema.rs` edits, the same in both builds), as `cargo bench --locked -p liq-ta --bench indicators --no-run` with `CARGO_PROFILE_BENCH_OVERFLOW_CHECKS=false` and `=true`, in separate `CARGO_TARGET_DIR`s. The `true` binary contains 1,098 calls to `panic_const_{add,sub,mul}_overflow`; the `false` binary contains 0.
- **Run:** the 21 changed groups (84 benchmarks), `taskset -c 9`, `--warm-up-time 2 --measurement-time 5 --noplot`, order off/on/off (A/B/A). Load average was 13–24 (other agents were compiling). Then a confirmation run of the 17 outlier benchmarks, order off/on/off/on, load 13–22.
- **Full run:** on vs mean(off): mean −0.9%, median −0.15%. A/A: mean −3.5%, median −0.7%; single benchmarks moved by up to 57% under load.
- **Confirmed effects (on vs off, best-of-two each):**
  - `ad/100` +3.8%, `ad/1000` +6.8%, `ad/10000` +7.0%, `sma/100000` +6.7%.
  - `bollinger` is the exception: checks on is faster, by −36.9% (100), −40.3% (1,000), −42.6% (10,000) and −10.6% (100,000).
  - The rest are within ±3%. A `roc/100000` +140% in the first on-run did not reproduce (−2.1%).
- **`bollinger` diagnosis (disassembly):** with checks off, LLVM if-converts the finite/non-finite branch of the rolling update into `fcsel`. That puts the selects on the loop-carried `sum`/`sum_sq` dependency chain. With checks on, the checked `nan_count += 1` keeps a predictable branch instead. This is a codegen sensitivity of the `bollinger` kernel, not an overflow-semantics benefit. It also applies at `42e4828` (same default). Not fixed here.
- **Decision:** keep `overflow-checks = false`. Turning checks on costs up to 7% on `ad` and `sma/100000` and buys no safety the up-front index validation does not already give. The `bollinger` result is a kernel-level optimisation opportunity.
- Raw output: `/tmp/rps-meas/ofc_{a1,b,a2}.txt`, `/tmp/rps-meas/ofc2_{c1,d1,c2,d2}.txt`, `/tmp/rps-meas/ao_*.txt` (not archived).

### Artifact size and symbols (`RPS-255`, `RPS-128`), measured 2026-11-22

Builds: `maturin build --locked --release` (venv `/tmp/liqta-venv`, CPython 3.12) and `cargo build --locked --release -p liq-ta-cli`. Each tree used its own release profile and a separate `CARGO_TARGET_DIR`. HEAD is the working tree at 09:34 (see above).

| Artifact | `42e4828` (`panic = "abort"`, `strip = true`) | HEAD (unwind, `strip = "debuginfo"`) |
|---|---:|---:|
| Wheel file | 446,760 B | 612,213 B (+37%) |
| `_liq_ta…so` | 1,189,848 B, stripped, `nm`: no symbols | 1,765,488 B, `.symtab` + `.strtab` = 313,352 B, `nm`: 2,277 symbols |
| `.so` `nm -D` | 133 (1 defined: `PyInit__liq_ta`) | 135 (1 defined) |
| `.so` `.text` / `.gcc_except_table` / `.eh_frame` | 872,680 / 4,060 / 67,112 B | 1,073,832 / 68,596 / 80,240 B |
| CLI `liq-ta` | 1,316,904 B, `nm`: no symbols, `nm -D` 73 | 2,007,704 B (+52%), `nm`: 2,596 symbols, `.symtab` + `.strtab` = 428,504 B, `nm -D` 75 |

- The HEAD wheel keeps its symbol table.
- **Attribution control:** HEAD built with the old profile (`CARGO_PROFILE_RELEASE_PANIC=abort CARGO_PROFILE_RELEASE_STRIP=true`) produces a 1,316,904 B CLI (`.text` 1,029,688 B), the same file size as `42e4828`. All of the CLI growth therefore comes from the profile change: 428,504 B of symbols, and for unwinding `.text` +162,640 B, `.gcc_except_table` +41,876 B and `.eh_frame` +17,680 B. The source changes add 3,792 B of `.text`.

## Not verified

- CI: all 17 jobs passed on GitHub at `3ab9127`: Rust tests, MSRV 1.95, stable and beta, Cargo Deny, wheels on 3 OSes, and Python 3.12–3.14 on 3 OSes.
- Hardware counters: `perf stat` (including `-e task-clock`) is refused on this host (`perf_event_paranoid` = 4, no `CAP_PERFMON`, `sudo` needs a password), and neither valgrind nor qemu TCG plugins are installed. The `aroon`/`obv` instruction counts above are static path lengths from disassembly, not dynamic counts.
- The `aroon` alternative (plain indexing, 1–3% within noise) was measured in a scratch copy only and not applied.
- Artifact sizes were measured on aarch64 only. The x86_64 and macOS wheels are unmeasured.
- The 2026-11-21 interim measurement (unpinned, shared host) is superseded by the pinned run above.
