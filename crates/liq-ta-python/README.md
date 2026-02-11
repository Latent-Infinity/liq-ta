# liq-ta

High-performance technical analysis library for Python, powered by Rust.

## Features

- **Fast**: Rust-powered O(n) algorithms with optimized memory access patterns
- **NumPy Native**: Zero-copy NumPy array support for seamless integration
- **Comprehensive**: All major technical indicators (SMA, EMA, RSI, MACD, Bollinger Bands, etc.)
- **Type Safe**: Full type hints for IDE support

## Installation

```bash
pip install liq-ta
```

## Quick Start

```python
import numpy as np
import liq_ta

# Generate sample price data
prices = np.array([44.0, 44.5, 43.5, 44.0, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0])

# Calculate Simple Moving Average
sma = liq_ta.sma(prices, period=5)
print(sma)  # First 4 values are NaN (lookback period)

# Calculate RSI
rsi = liq_ta.rsi(prices, period=5)
print(rsi)
```

## Zero-Copy API

All functions support an optional `out=` parameter for zero-copy output, following NumPy conventions:

```python
import numpy as np
import liq_ta

prices = np.random.randn(1_000_000)
output = np.empty(len(prices), dtype=np.float64)

# Zero-copy: ~2x faster than allocating version
liq_ta.sma(prices, 20, out=output)

# All single-output functions support out= parameter:
# sma, ema, ema_wilder, rsi, williams_r, atr, true_range, rolling_stddev, obv, vwap
```

**Performance comparison (1M elements):**
| Function | Time | Notes |
|----------|------|-------|
| `sma(data, 20)` | ~23ms | Allocates new array |
| `sma(data, 20, out=out)` | ~11ms | Zero-copy to pre-allocated |

## Available Indicators

### Moving Averages
- `sma(data, period)` - Simple Moving Average
- `ema(data, period)` - Exponential Moving Average
- `ema_wilder(data, period)` - Wilder's Smoothing (alpha = 1/n)

### Momentum
- `rsi(data, period)` - Relative Strength Index
- `macd(data, fast, slow, signal)` - MACD (returns line, signal, histogram)
- `stochastic(high, low, close, k, d, slowing)` - Stochastic Oscillator
- `stochastic_fast(high, low, close, k, d)` - Fast Stochastic
- `stochastic_slow(high, low, close, k, d)` - Slow Stochastic
- `williams_r(high, low, close, period)` - Williams %R
- `adx(high, low, close, period)` - Average Directional Index (returns adx, +DI, -DI)

### Volatility
- `atr(high, low, close, period)` - Average True Range
- `true_range(high, low, close)` - True Range
- `bollinger(data, period, std_dev)` - Bollinger Bands (returns upper, middle, lower)
- `donchian(high, low, period)` - Donchian Channels (returns upper, middle, lower)
- `rolling_stddev(data, period)` - Rolling Standard Deviation

### Volume
- `obv(close, volume)` - On-Balance Volume
- `vwap(high, low, close, volume)` - Volume Weighted Average Price

## Polars Integration

liq-ta works seamlessly with Polars via `.to_numpy()`:

```python
import polars as pl
import liq_ta

df = pl.DataFrame({
    'price': [44.0, 44.5, 43.5, 44.0, 44.5, 45.0, 45.5, 46.0]
})

# Convert Polars Series to NumPy (zero-copy due to Arrow backing)
sma = liq_ta.sma(df['price'].to_numpy(), period=5)

# Add result back to DataFrame
df = df.with_columns(pl.Series('sma_5', sma))
```

The `.to_numpy()` conversion is essentially zero-copy because Polars uses Arrow's contiguous memory layout internally.

## NaN Handling

All indicators return NaN values for the lookback period at the start of the output.
This ensures output arrays have the same length as input arrays.

```python
import numpy as np
import liq_ta

prices = np.array([1.0, 2.0, 3.0, 4.0, 5.0])
sma = liq_ta.sma(prices, period=3)
# sma = [nan, nan, 2.0, 3.0, 4.0]  # First 2 values are NaN
```

## Error Handling

All functions raise `ValueError` for invalid inputs:

```python
import numpy as np
import liq_ta

# Period too long for data
short_data = np.array([1.0, 2.0])
try:
    result = liq_ta.sma(short_data, period=10)
except ValueError as e:
    print(e)  # "Insufficient data: need at least 10 values, got 2"
```

## License

MIT OR Apache-2.0
