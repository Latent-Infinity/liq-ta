# Integrated backend smoke comparison

Status: `DEV_SMOKE_ONLY`. This supplements the immutable true-range smoke receipt; it does not change its scope or make a research claim.

- The integrated backend was built with Rust 1.98.1 using the same release profile and unchanged authorized fixture conversion described in `true-range-smoke-receipt.md`.
- `crates/liq-ta/src/kernels/simd.rs` SHA-256: `6922d196b1e4242e46fffe2375bceb2fc1ca32fe76c14cd3156b1b94c185fb45`.
- `crates/liq-ta/src/kernels/simd/scalar_backend.rs` SHA-256: `2621bf3ca58dcc91eafb24380e69ac579a405f1385758db51f46c40a13e8b93b`.
- All 448 calculated values matched the recorded nightly baseline byte for byte. The integrated output SHA-256 is `e6566a6a891241c0d52dfb2e80cdd75e56049b53575858c0bfce19682021a272`.
- The integrated backend also matched both recorded 396-case arithmetic parity logs, after canonicalization of NaNs by the drivers. Non-NaN bit patterns and NaN classification matched; payloads and signs of NaNs were not compared.
- After these checks, one explanatory comment was clarified without changing executable code. The resulting `simd.rs` SHA-256 is `4d3912aeecc71c327674a9dfce222d88275654e6869dfd8a9db6ca99919b199c`.
