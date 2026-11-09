# Python 3.15 binding verification

The Python bindings were checked on macOS ARM64 using an isolated CPython
3.15.2 interpreter. The official interpreter archive was verified against
its release SHA-256,
`1b1b03fca8aa351698403f3d100ee83db7a5d99748fc7707681ac31a6a846ad9`.
No system interpreter or project dependency was changed.

The tested source revision was `d6871e9b39f95a54fe87deda24e202f07a5b7b20`,
with Rust 1.98.1 (`48a229ceaefd4985c50990b14116b6d856af0985`) and the
workspace lock SHA-256
`5625da6d3fe04eabb31f10360d67881bb913c770e92acc99d1301c5d858b09f0`.
The Python lock SHA-256 was
`e4308ec447f65886dd16eab1d9217fe990dcf2e17ae08776511ff05a3d2d00ae`.
The isolated environment used the lock's NumPy 2.5.4, pytest 9.2.0 and
maturin 1.16.0; each downloaded wheel matched its locked SHA-256.

`maturin build --release --locked --offline` produced a CPython 3.15 wheel
without an ABI compatibility override. The wheel SHA-256 is
`a332852ff25241840e0a047343de578d207b31766fa5d8156eb52166e1df7542`;
its extracted native extension SHA-256 is
`0ea18cae13d33e5487502dbd890783bbded30d4c68cff780f931f8b52ab75a3d`.
The complete three-module Python binding suite imported that extracted wheel
and completed 58 tests, with two optional Polars tests skipped. The build and
test log SHA-256 values are
`8a2660cb2a7ddc32adc7923c84dde7d54b384c1121f3325c454ba92ad1cc249d` and
`c3eb489529d1241f3842612933faaed0c67eb51f7ed8cf56a2dd15db368cc1e4`.

These are software regression checks on one platform, not financial or
cross-platform evidence. The wheel and logs were retained in the local
temporary directory at review time; the hashes identify them if archived.
