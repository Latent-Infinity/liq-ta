# Python output-array safety verification

The output borrow repair was checked on macOS ARM64 with the pinned Rust 1.98.1
toolchain and the Python 3.12, 3.13, and 3.14 wheel builds. These results are
software regression evidence only; they supersede neither the earlier numerical
comparison nor its recorded artifact identities.

The workspace `Cargo.lock` SHA-256 is
`5625da6d3fe04eabb31f10360d67881bb913c770e92acc99d1301c5d858b09f0`.
The binding source SHA-256 is
`605856f7d7cc8df5b2b7d7a93924368dccf1e43fd70a13bb749b0f0c836ef2ab`;
the binding test SHA-256 is
`b419ecd327ca911cc465e1bec457b0f010f7b7128d98675fb8b20caf7fdc6c1a`.

| Rebuilt wheel | SHA-256 |
| --- | --- |
| CPython 3.12 | `53a1195b6d41860cc528bccc44e64b2b3b150cc009c07f3a3827c7952ad1faab` |
| CPython 3.13 | `a9325c2ec71b6db90fb6838a3fbc32104678ad49ce49e085a94e2db3baebe0f7` |
| CPython 3.14 | `60ca2844b59385bac48c4bd5b0833209290895a04625aaf57dd4a9209bae1b30` |

Each wheel was built using `maturin build --release --locked --offline`,
extracted into a separate temporary directory, and tested using that directory
as `PYTHONPATH`. Each complete Python suite had 58 successful tests and two
optional Polars tests skipped. The three test-log SHA-256 values are,
respectively, `4af3c88b0a09ec3226e68f6fa8ec6de105af02e0e4eacb0a760c371c174b123a`,
`8017828ad99902d76876dbe0f34190284f3d56637136285538236521f864714a`, and
`24284cf0f25fcf9c2cfbae89fd8fc2f96632c696114b95b92811a9441d806054`.
The build-log SHA-256 is
`e95bf4d45da6d218f26344e214db85dc269b4c0b178e31efe8bfab1a0badca9f`.

`cargo llvm-cov --workspace --summary-only --locked --offline` completed with
94.28% line and 93.18% region coverage. Its log SHA-256 is
`fa4ada9c79b22c5ae4dc791a3214781553f2b0509d848e8afc46d3e551fc0cf2`;
the previously investigated warning about 24 mismatched LLVM mappings remains.
The complete Rust 1.95 workspace test and the configured CI feature suite
completed; their log SHA-256 values are
`395e852767cd887679e405f26445a407276a06b8ab5df98e99d2124a7a3c03e9` and
`ad96305b2a17752be03ab2fdfe9925fc94223fac47db4c2011efa4d1a813e8fc`.
Formatting and Clippy checks completed. The AddressSanitizer binding test suite
completed 11 tests; its log SHA-256 is
`872fb190b1cfe8a95dcc32d9006bacc50a769c824e770b0423f17943271f4361`.
Miri could not execute CPython's `Py_IsInitialized` foreign call on macOS; its
log SHA-256 is `4a78fd8b8cfb362299a8905e71c45f667bdfec73f139268ea091424d26795874`.
The wheels and full command logs were retained in the local temporary directory
at the time of review; their hashes above identify them if separately archived.
