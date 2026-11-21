#!/bin/bash
# Run the registered indicator benchmarks sequentially from the workspace root.
set -euo pipefail

cd "$(dirname "$0")/.."
cargo bench --locked -p liq-ta --bench indicators -- "$@"
