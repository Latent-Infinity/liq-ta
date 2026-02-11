# liq-ta Benchmark Scripts

Modern Python CLI for running and comparing liq-ta vs ta-lib benchmarks with beautiful output.

## Installation

```bash
# Install Python dependencies
pip install -r scripts/requirements.txt
```

## Quick Start

```bash
# List all available indicators
./scripts/benchmark.py list

# Run a quick benchmark comparing liq-ta vs ta-lib
./scripts/benchmark.py run stochastic

# Run benchmarks for multiple indicators
./scripts/benchmark.py run sma ema rsi

# View previous results (auto-detects latest baseline)
./scripts/benchmark.py results
```

## Usage Examples

```bash
# Run specific indicators with verbose output
./scripts/benchmark.py run stochastic stochastic_fast -v

# Sort results by absolute gap
./scripts/benchmark.py results --sort gap

# Test with different data size
./scripts/benchmark.py run sma --size 10000

# Skip build step if already built
./scripts/benchmark.py run --skip-build
```

## Understanding Output

The script shows a color-coded comparison table:
- 🟢 **Green**: liq-ta is faster (ratio < 0.98)  
- 🟡 **Yellow**: Competitive, within ±2%
- 🔴 **Red**: liq-ta is slower (ratio > 1.02)

**Ratio** = liq-ta time / ta-lib time  
- < 1.0 means liq-ta is faster
- > 1.0 means ta-lib is faster

For your stochastic benchmark question, just run:
```bash
./scripts/benchmark.py run stochastic stochastic_fast
```

This will show you exactly how much faster or slower each is compared to ta-lib!
