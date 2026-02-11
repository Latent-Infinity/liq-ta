# Precision Test Datasets

This directory contains deterministic test datasets for precision validation tests.

## Reproducibility

All datasets are generated using seed `0xFA57_7A00` (mnemonic: "FAST-TA-00") with the ChaCha8 PRNG for consistent results across platforms.

## Files

| File | Description | Bars | Purpose |
|------|-------------|------|---------|
| `random_walk.json` | Random walk price series | 10,000 | General precision testing |
| `extreme_values.json` | Extreme values (+-1e6, +-1e-6) | 1,000 | Edge case testing |
| `near_constant.json` | Near-constant data (base=1000, noise<1e-4) | 10,000 | Variance stress test |
| `typical_ohlcv.json` | Realistic OHLCV with u64 volume | 10,000 | Volume indicator testing |

## Regeneration

To regenerate the datasets (run only when algorithm intentionally changes):

```sh
REGENERATE_TEST_DATA=1 cargo test -p liq-ta --test generate_test_data
```

## Schema

Each JSON file contains a `metadata` object with:
- `seed`: The random seed used (always `0xFA57_7A00`)
- `schema_version`: Schema version for forward compatibility
- `description`: Human-readable description
- `bars`: Number of data points

**Note**: Timestamps are intentionally excluded to ensure deterministic file contents.

## Usage in Tests

```rust
use serde_json;
use std::fs;

let json = fs::read_to_string("test-data/precision/random_walk.json")?;
let data: serde_json::Value = serde_json::from_str(&json)?;
let prices: Vec<f64> = data["data"]
    .as_array()
    .unwrap()
    .iter()
    .map(|v| v.as_f64().unwrap())
    .collect();
```
