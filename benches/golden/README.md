# Golden Files Directory

This directory contains pre-computed TA-Lib reference outputs for validating
liq-ta indicator implementations.

## Generating Golden Files

Golden files are generated using the Python script `tools/generate_golden.py`:

```bash
# Install dependencies
pip install talib numpy

# Generate golden files
python tools/generate_golden.py
```

**Note:** TA-Lib requires the C library to be installed first. See the
[TA-Lib Python documentation](https://github.com/ta-lib/ta-lib-python#dependencies)
for installation instructions.

## File Format

Each indicator has a JSON file with the following structure:

```json
{
  "indicator": "SMA",
  "parameters": { "period": 14 },
  "talib_version": "0.4.32",
  "generated_at": "2024-12-20T00:00:00Z",
  "test_cases": [
    {
      "name": "random_walk_1k_seed42",
      "input_seed": 42,
      "input_length": 1000,
      "output": [null, null, ..., 100.234, 100.456, ...]
    }
  ]
}
```

- `null` values represent NaN (lookback period or missing data)
- Input data is generated using the same seeded RNG as liq-ta-experiments

## Indicators

The following indicators have golden files:

| File | Indicator | Parameters |
|------|-----------|------------|
| `sma.json` | Simple Moving Average | period=14 |
| `ema.json` | Exponential Moving Average | period=14 |
| `rsi.json` | Relative Strength Index | period=14 |
| `macd.json` | MACD | fast=12, slow=26, signal=9 |
| `atr.json` | Average True Range | period=14 |
| `bollinger.json` | Bollinger Bands | period=20, std=2.0 |
| `stochastic.json` | Stochastic Oscillator | k=14, d=3 |

## Usage in Tests

```rust
use liq_ta_experiments::talib_baseline::{load_golden_file, compare_outputs, DEFAULT_TOLERANCE};

let golden = load_golden_file("benches/golden/sma.json")?;
let test_case = golden.find_test_case("random_walk_1k_seed42").unwrap();

// Generate input data with matching seed
let input = generate_random_walk(test_case.input_length, test_case.input_seed);

// Compute liq-ta output
let output = liq_ta_core::indicators::sma(&input, 14)?;

// Compare against golden reference
let result = compare_outputs(&output, &test_case.output, DEFAULT_TOLERANCE)?;
assert!(result.passed, "Output should match TA-Lib reference");
```

## Regenerating Golden Files

Golden files should be regenerated when:

1. Test data generation algorithm changes
2. TA-Lib version is updated
3. New test cases are added

After regenerating, verify that all liq-ta tests still pass to ensure
there are no regressions.
