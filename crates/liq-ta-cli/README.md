# liq-ta-cli

Command-line interface for the liq-ta technical analysis library.

## Installation

```bash
cargo install liq-ta-cli
```

Or build from source:

```bash
cargo build --release -p liq-ta-cli
```

## Usage

```bash
liq-ta <COMMAND> <INPUT> [PARAMS] [-o OUTPUT]
```

- `COMMAND`: The indicator to compute
- `INPUT`: Path to input CSV file
- `PARAMS`: Indicator parameters (optional, uses defaults)
- `-o OUTPUT`: Output file path (optional, defaults to stdout)

## Indicators

### Moving Averages

#### SMA - Simple Moving Average

```bash
# Default period (20)
liq-ta sma prices.csv

# Custom period
liq-ta sma prices.csv 50

# With output file
liq-ta sma prices.csv 20 -o sma_output.csv
```

#### EMA - Exponential Moving Average

```bash
liq-ta ema prices.csv 20
liq-ta ema prices.csv 12 -o ema12.csv
```

### Momentum Indicators

#### RSI - Relative Strength Index

```bash
# Default period (14)
liq-ta rsi prices.csv

# Custom period
liq-ta rsi prices.csv 7
```

#### MACD - Moving Average Convergence Divergence

Parameters: `fast,slow,signal`

```bash
# Default (12,26,9)
liq-ta macd prices.csv

# Custom parameters
liq-ta macd prices.csv 8,17,9
```

Outputs three columns: `macd`, `signal`, `histogram`

#### Stochastic Oscillator

Parameters: `k_period,d_period[,k_slowing]`

```bash
# Fast Stochastic (14,3)
liq-ta stochastic prices.csv 14,3

# Slow Stochastic (14,3,3)
liq-ta stochastic prices.csv 14,3,3
```

Outputs two columns: `k`, `d`

#### Williams %R

```bash
# Default period (14)
liq-ta williams-r prices.csv

# Custom period
liq-ta williams-r prices.csv 10
```

#### ADX - Average Directional Index

```bash
# Default period (14)
liq-ta adx prices.csv

# Custom period
liq-ta adx prices.csv 20
```

Outputs three columns: `adx`, `plus_di`, `minus_di`

### Volatility Indicators

#### ATR - Average True Range

Requires OHLC data.

```bash
# Default period (14)
liq-ta atr ohlc.csv

# Custom period
liq-ta atr ohlc.csv 20
```

#### Bollinger Bands

Parameters: `period,std_dev`

```bash
# Default (20,2.0)
liq-ta bollinger prices.csv

# Custom parameters
liq-ta bollinger prices.csv 20,2.5
```

Outputs three columns: `upper`, `middle`, `lower`

#### Donchian Channels

```bash
# Default period (20)
liq-ta donchian ohlc.csv

# Custom period
liq-ta donchian ohlc.csv 55
```

Outputs three columns: `upper`, `middle`, `lower`

### Volume Indicators

#### OBV - On-Balance Volume

Requires close and volume columns.

```bash
liq-ta obv ohlcv.csv
```

#### VWAP - Volume Weighted Average Price

Requires OHLCV data.

```bash
liq-ta vwap ohlcv.csv
```

## Input CSV Format

The CLI auto-detects columns based on headers. Supported column names:

- **Close prices**: `close`, `price`, `adj_close`, `adjusted_close`
- **High prices**: `high`
- **Low prices**: `low`
- **Open prices**: `open`
- **Volume**: `volume`, `vol`
- **Date/Time**: `date`, `datetime`, `time`, `timestamp` (preserved in output)

Example input:

```csv
date,open,high,low,close,volume
2024-01-01,100.0,102.5,99.0,101.5,1000000
2024-01-02,101.5,103.0,100.0,102.0,1200000
```

## Output Format

- **Date column**: Preserved from input, aligned to first valid output row
- **Lookback rows**: Dropped (rows with NaN prefix are excluded)
- **Internal NaN**: Preserved as empty cells
- **Multi-output indicators**: Multiple columns (e.g., `macd,signal,histogram`)

Example output for MACD:

```csv
date,macd,signal,histogram
2024-02-03,0.523,0.412,0.111
2024-02-04,0.587,0.447,0.140
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Argument error (invalid parameters, unknown command) |
| 2 | Data error (file not found, CSV parse error) |
| 3 | Computation error (insufficient data) |

## Error Messages

The CLI provides actionable error messages:

```bash
$ liq-ta sma missing.csv
Error: File not found: missing.csv
  Hint: Check that the file path is correct

$ liq-ta macd prices.csv 12,26
Error: MACD requires 3 parameters, got 2
  Hint: Use format: fast,slow,signal (e.g., 12,26,9)

$ liq-ta sma short.csv 50
Error: Insufficient data for SMA with period 50
  Hint: Input has 10 rows, but period requires at least 50
```

## Examples

```bash
# Basic SMA
liq-ta sma prices.csv 20

# RSI with output file
liq-ta rsi prices.csv 14 -o rsi.csv

# MACD to stdout (for piping)
liq-ta macd prices.csv 12,26,9

# Bollinger Bands with custom std dev
liq-ta bollinger prices.csv 20,2.5 -o bands.csv

# Full stochastic
liq-ta stochastic ohlc.csv 14,3,3

# Pipeline with other tools
liq-ta sma prices.csv 20 | head -5
cat prices.csv | liq-ta rsi - 14
```

## Performance

All indicators run in O(n) time with high throughput. Processing 100K rows typically takes under 1ms.

## License

MIT OR Apache-2.0
