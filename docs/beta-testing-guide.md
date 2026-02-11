# liq-ta Beta Testing Guide

**Version**: 0.1.0-beta
**Date**: 2025-12-23

This guide walks beta testers through installing and validating liq-ta from source.

---

## Prerequisites

- **Python**: 3.12 or newer (3.14 supported)
- **Rust**: nightly toolchain (1.90+) - required for portable SIMD
- **Git**: for cloning the repository

### Install Rust (if needed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default nightly  # liq-ta requires nightly for portable SIMD
rustc --version  # Should show 1.90+
```

---

## Step 1: Create Test Project

```bash
# Create a new directory for testing
mkdir liq-ta-beta-test
cd liq-ta-beta-test

# Create and activate virtual environment
python3 -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Verify Python version
python --version  # Should be 3.12+
```

---

## Step 2: Clone and Install liq-ta

```bash
# Clone the repository
git clone https://github.com/anthropics/liq-ta.git
cd liq-ta/crates/liq-ta-python

# Install build dependencies
pip install maturin numpy

# Build and install liq-ta (development mode)
maturin develop --release

# Verify installation
python -c "import liq_ta; print(f'liq-ta {liq_ta.__version__} installed successfully')"
```

Expected output:
```
liq-ta 0.1.0 installed successfully
```

---

## Step 3: Run Validation Script

Run the bundled validator (uses the indicator registry for discovery):

```bash
liq-ta-validate
# or
python -m liq_ta.validate
```

Run the validation:

```bash
python -m liq_ta.validate
```

Expected output:
```
============================================================
liq-ta Beta Validation
============================================================

[OK] liq-ta version: 0.1.0

--- Moving Averages ---
  [OK] sma(close, 20) - first valid: 99.8234
  [OK] sma(close, 20, out=out) - zero-copy verified
  [OK] ema(close, 20) - first valid: 99.8234
  [OK] ema_wilder(close, 20) - first valid: 99.8234

--- Momentum Indicators ---
  [OK] rsi(close, 14) - range: [23.4, 76.8]
  [OK] macd(close, 12, 26, 9) - returns 3 arrays
  [OK] stochastic(high, low, close, 14, 3, 1) - returns %K, %D
  [OK] stochastic_fast(high, low, close, 14, 3)
  [OK] stochastic_slow(high, low, close, 14, 3)
  [OK] williams_r(high, low, close, 14) - range: [-95.2, -4.8]
  [OK] adx(high, low, close, 14) - returns ADX, +DI, -DI

--- Volatility Indicators ---
  [OK] atr(high, low, close, 14) - mean: 0.5234
  [OK] true_range(high, low, close)
  [OK] bollinger(close, 20, 2.0) - returns upper, middle, lower
  [OK] donchian(high, low, 20) - returns upper, middle, lower
  [OK] rolling_stddev(close, 20)

--- Volume Indicators ---
  [OK] obv(close, volume) - final: 12345
  [OK] vwap(high, low, close, volume) - final: 100.1234

============================================================
VALIDATION SUMMARY
============================================================

Tests passed: 18/18

All tests passed! liq-ta is working correctly.
```

---

## Step 4: Test with Your Data

Once validation passes, try liq-ta with your own data:

```python
import numpy as np
import pandas as pd
import liq_ta

# Load your data (example with pandas)
df = pd.read_csv('your_price_data.csv')

# Extract arrays (liq-ta requires numpy arrays)
close = df['close'].to_numpy()
high = df['high'].to_numpy()
low = df['low'].to_numpy()
volume = df['volume'].to_numpy()

# Calculate indicators
sma_20 = liq_ta.sma(close, 20)
rsi_14 = liq_ta.rsi(close, 14)
macd_line, signal, histogram = liq_ta.macd(close, 12, 26, 9)
atr_14 = liq_ta.atr(high, low, close, 14)

# Add back to DataFrame
df['sma_20'] = sma_20
df['rsi_14'] = rsi_14
df['atr_14'] = atr_14

print(df.tail())
```

### Zero-Copy for Performance

For performance-critical code, use the `out=` parameter to avoid allocations:

```python
import numpy as np
import liq_ta

# Pre-allocate output buffers
n = len(close)
sma_buffer = np.empty(n, dtype=np.float64)
ema_buffer = np.empty(n, dtype=np.float64)
rsi_buffer = np.empty(n, dtype=np.float64)

# Calculate indicators (writes directly to buffers)
liq_ta.sma(close, 20, out=sma_buffer)
liq_ta.ema(close, 20, out=ema_buffer)
liq_ta.rsi(close, 14, out=rsi_buffer)

# Buffers now contain the results - no extra allocations
```

### Polars Integration

liq-ta works seamlessly with Polars:

```python
import polars as pl
import liq_ta

df = pl.read_csv('your_data.csv')

# Polars .to_numpy() is essentially zero-copy due to Arrow backing
close = df['close'].to_numpy()
sma = liq_ta.sma(close, 20)

# Add back to DataFrame
df = df.with_columns(pl.Series('sma_20', sma))
```

---

## Providing Feedback

Please report any issues or feedback:

1. **GitHub Issues**: https://github.com/anthropics/liq-ta/issues
2. **Include**:
   - Python version (`python --version`)
   - OS and architecture (`uname -a` or `systeminfo`)
   - Error message and traceback
   - Minimal code to reproduce

### Feedback Topics

We're particularly interested in:

- [ ] **Installation issues**: Problems building or installing
- [ ] **Numerical accuracy**: Discrepancies vs TA-Lib or other libraries
- [ ] **Performance**: Benchmarks vs your current solution
- [ ] **API ergonomics**: Anything confusing or inconvenient
- [ ] **Missing indicators**: What else do you need?
- [ ] **Documentation**: What's unclear?

---

## Troubleshooting

### "maturin: command not found"

```bash
pip install maturin
```

### "error: linker `cc` not found" (Linux)

```bash
sudo apt-get install build-essential  # Debian/Ubuntu
sudo yum groupinstall "Development Tools"  # RHEL/CentOS
```

### "error: failed to run custom build command for `pyo3-ffi`"

Ensure Rust is installed and up to date:
```bash
rustup update stable
```

### Non-contiguous array error

liq-ta requires C-contiguous arrays. If you get a `TypeError` about contiguous arrays:

```python
# This may fail:
strided = data[::2]  # Every other element - not contiguous
liq_ta.sma(strided, 5)  # TypeError!

# Solution: make it contiguous
contiguous = np.ascontiguousarray(strided)
liq_ta.sma(contiguous, 5)  # Works!
```

### Import error after installation

Make sure you're using the same Python that maturin used:
```bash
which python  # Should be your venv Python
maturin develop  # Reinstall
```

---

## Thank You!

Thank you for beta testing liq-ta! Your feedback helps us build a better library.
