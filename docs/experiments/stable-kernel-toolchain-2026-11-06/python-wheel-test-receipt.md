# Python wheel software-regression receipt

Status: `DEV_SMOKE_ONLY`. The legacy Python tests were run under the scoped Oracle decision `liq-docs/plans/rust-legacy-engineering-suite-oracle-decision-2026-11-06.md`. Their generated examples are software checks, not financial acceptance evidence.

All three wheels were built from the integrated source using Rust 1.98.1, extracted into separate temporary directories, and selected explicitly with `PYTHONPATH` when running the existing tests with `--import-mode=importlib`. Imports were checked independently after the tests.

- Python 3.12: `PYTHONPATH=/tmp/liq-ta-wheel-import-py312-20261106`, interpreter `liq-experiments/.venv/bin/python`; 50 tests succeeded. Wheel SHA-256 `80ef6753e863847440d814e6f01b3dd6a80c1ca90f009ce674c24d41eb5a89f3`; extracted native module SHA-256 `2ae792661df3cc75f09a1b69e31964dec52df9ddde9635aff311c66bc8349b2f`. Test output: `/tmp/liq-ta-verification-20261106/run-cpkv4b45/`.
- Python 3.13: `PYTHONPATH=/tmp/liq-ta-wheel-import-py313-20261106`, interpreter `/tmp/liq-ta-py313-test-20261106/bin/python`; 48 tests succeeded and two optional Polars tests were skipped because Polars was unavailable offline. Wheel SHA-256 `faaf514ad81cdbcbb4929cb89892785319c511f2910bc5781395d1da84055185`; extracted native module SHA-256 `0e44163efb400e1c695c75abbda4e7148c666c13ef6a26563e11f16baa6c0f05`.
- Python 3.14: `PYTHONPATH=/tmp/liq-ta-wheel-import-20261106`, interpreter `/tmp/liq-ta-py314-test-20261106/bin/python`; 48 tests succeeded and two optional Polars tests were skipped because Polars was unavailable offline. Wheel SHA-256 `dc5b9c843079bc1079fceee9e9de9ad9fea5e3c0cc00226cc7bb1509c1a4d517`; extracted native module SHA-256 `ef6029ee423165851f9ba99dab8a9a97d901441c950abb760c7c911c3329a598`. Test output: `/tmp/liq-ta-verification-20261106/run-g3durhry/`.

The verification wrapper records command arguments but not environment variables; the explicit `PYTHONPATH` values above document which rebuilt extensions the test processes imported. These checks cover the local macOS ARM64 host, not the complete CI platform matrix.
