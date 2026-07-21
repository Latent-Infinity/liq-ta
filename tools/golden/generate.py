#!/usr/bin/env python3
"""Generate golden files with TA-Lib reference outputs.

This script generates reference outputs from TA-Lib for all 7 baseline
indicators. The outputs are stored as JSON golden files for comparison
with liq-ta implementations.

Requirements:
    pip install talib numpy

Usage:
    python tools/generate_golden.py

Output:
    benches/golden/*.json - Golden files for each indicator
    benches/golden/metadata.json - Generation metadata
"""

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

try:
    import numpy as np
    import talib
except ImportError:
    print("Error: This script requires numpy and TA-Lib.")
    print("Install with: pip install talib numpy")
    print("")
    print("Note: TA-Lib requires the C library to be installed first.")
    print("See: https://github.com/ta-lib/ta-lib-python#dependencies")
    sys.exit(1)


# Configuration
GOLDEN_DIR = Path("crates/liq-ta/tests/golden")
DATA_SIZES = [1_000, 10_000, 100_000]  # Skip 1M for faster generation
SEEDS = [42, 123, 456]


def generate_random_walk(n: int, seed: int) -> np.ndarray:
    """Generate a random walk price series (matches Rust implementation)."""
    np.random.seed(seed)

    initial_price = 100.0
    drift = 0.0001
    volatility = 0.02

    prices = np.zeros(n)
    prices[0] = initial_price

    for i in range(1, n):
        z = np.random.standard_normal()
        return_pct = drift + volatility * z
        prices[i] = max(0.01, prices[i-1] * (1 + return_pct))

    return prices


def generate_ohlcv(n: int, seed: int) -> dict:
    """Generate OHLCV data (matches Rust implementation structure)."""
    np.random.seed(seed)

    initial_price = 100.0
    drift = 0.0001
    volatility = 0.02

    open_prices = np.zeros(n)
    high_prices = np.zeros(n)
    low_prices = np.zeros(n)
    close_prices = np.zeros(n)
    volumes = np.zeros(n)

    prev_close = initial_price

    for i in range(n):
        # Generate random values
        u1, u2, u3, u4, u5 = np.random.random(5)

        # Open with small gap from previous close
        gap_factor = (u3 - 0.5) * volatility * 0.5
        open_price = max(0.01, prev_close * (1 + gap_factor))

        # Close based on drift + volatility
        z = np.random.standard_normal()
        return_pct = drift + volatility * z
        close_price = max(0.01, open_price * (1 + return_pct))

        # High and Low
        intraday_vol = volatility * (0.5 + u4 * 0.5)
        max_oc = max(open_price, close_price)
        min_oc = min(open_price, close_price)

        high_price = max_oc + max_oc * intraday_vol * u4
        low_price = max(0.01, min_oc - min_oc * intraday_vol * (1 - u4))

        # Volume
        volume = 1_000_000.0 * (0.5 + u5)

        open_prices[i] = open_price
        high_prices[i] = high_price
        low_prices[i] = low_price
        close_prices[i] = close_price
        volumes[i] = volume

        prev_close = close_price

    return {
        "open": open_prices,
        "high": high_prices,
        "low": low_prices,
        "close": close_prices,
        "volume": volumes,
    }


def array_to_json(arr: np.ndarray) -> list:
    """Convert numpy array to JSON-compatible list (NaN -> null)."""
    return [None if np.isnan(x) else float(x) for x in arr]


def generate_sma_golden() -> dict:
    """Generate SMA golden file."""
    period = 14
    test_cases = []

    for size in DATA_SIZES:
        for seed in SEEDS:
            prices = generate_random_walk(size, seed)
            output = talib.SMA(prices, timeperiod=period)

            test_cases.append({
                "name": f"random_walk_{size//1000}k_seed{seed}",
                "input_seed": seed,
                "input_length": size,
                "output": array_to_json(output),
            })

    return {
        "indicator": "SMA",
        "parameters": {"period": period},
        "talib_version": talib.__version__,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "test_cases": test_cases,
    }


def generate_ema_golden() -> dict:
    """Generate EMA golden file."""
    period = 14
    test_cases = []

    for size in DATA_SIZES:
        for seed in SEEDS:
            prices = generate_random_walk(size, seed)
            output = talib.EMA(prices, timeperiod=period)

            test_cases.append({
                "name": f"random_walk_{size//1000}k_seed{seed}",
                "input_seed": seed,
                "input_length": size,
                "output": array_to_json(output),
            })

    return {
        "indicator": "EMA",
        "parameters": {"period": period},
        "talib_version": talib.__version__,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "test_cases": test_cases,
    }


def generate_rsi_golden() -> dict:
    """Generate RSI golden file."""
    period = 14
    test_cases = []

    for size in DATA_SIZES:
        for seed in SEEDS:
            prices = generate_random_walk(size, seed)
            output = talib.RSI(prices, timeperiod=period)

            test_cases.append({
                "name": f"random_walk_{size//1000}k_seed{seed}",
                "input_seed": seed,
                "input_length": size,
                "output": array_to_json(output),
            })

    return {
        "indicator": "RSI",
        "parameters": {"period": period},
        "talib_version": talib.__version__,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "test_cases": test_cases,
    }


def generate_macd_golden() -> dict:
    """Generate MACD golden file."""
    fast_period = 12
    slow_period = 26
    signal_period = 9
    test_cases = []

    for size in DATA_SIZES:
        for seed in SEEDS:
            prices = generate_random_walk(size, seed)
            macd, signal, hist = talib.MACD(
                prices,
                fastperiod=fast_period,
                slowperiod=slow_period,
                signalperiod=signal_period,
            )

            test_cases.append({
                "name": f"random_walk_{size//1000}k_seed{seed}",
                "input_seed": seed,
                "input_length": size,
                "output": {
                    "macd": array_to_json(macd),
                    "signal": array_to_json(signal),
                    "histogram": array_to_json(hist),
                },
            })

    return {
        "indicator": "MACD",
        "parameters": {
            "fast_period": fast_period,
            "slow_period": slow_period,
            "signal_period": signal_period,
        },
        "talib_version": talib.__version__,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "test_cases": test_cases,
    }


def generate_atr_golden() -> dict:
    """Generate ATR golden file."""
    period = 14
    test_cases = []

    for size in DATA_SIZES:
        for seed in SEEDS:
            ohlcv = generate_ohlcv(size, seed)
            output = talib.ATR(
                ohlcv["high"],
                ohlcv["low"],
                ohlcv["close"],
                timeperiod=period,
            )

            test_cases.append({
                "name": f"ohlcv_{size//1000}k_seed{seed}",
                "input_seed": seed,
                "input_length": size,
                "output": array_to_json(output),
            })

    return {
        "indicator": "ATR",
        "parameters": {"period": period},
        "talib_version": talib.__version__,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "test_cases": test_cases,
    }


def generate_bollinger_golden() -> dict:
    """Generate Bollinger Bands golden file."""
    period = 20
    num_std_dev = 2.0
    test_cases = []

    for size in DATA_SIZES:
        for seed in SEEDS:
            prices = generate_random_walk(size, seed)
            upper, middle, lower = talib.BBANDS(
                prices,
                timeperiod=period,
                nbdevup=num_std_dev,
                nbdevdn=num_std_dev,
                matype=0,  # SMA
            )

            test_cases.append({
                "name": f"random_walk_{size//1000}k_seed{seed}",
                "input_seed": seed,
                "input_length": size,
                "output": {
                    "upper": array_to_json(upper),
                    "middle": array_to_json(middle),
                    "lower": array_to_json(lower),
                },
            })

    return {
        "indicator": "BBANDS",
        "parameters": {
            "period": period,
            "num_std_dev": num_std_dev,
        },
        "talib_version": talib.__version__,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "test_cases": test_cases,
    }


def generate_stochastic_golden() -> dict:
    """Generate Stochastic Oscillator golden file."""
    k_period = 14
    d_period = 3
    test_cases = []

    for size in DATA_SIZES:
        for seed in SEEDS:
            ohlcv = generate_ohlcv(size, seed)
            slowk, slowd = talib.STOCH(
                ohlcv["high"],
                ohlcv["low"],
                ohlcv["close"],
                fastk_period=k_period,
                slowk_period=d_period,
                slowk_matype=0,
                slowd_period=d_period,
                slowd_matype=0,
            )

            test_cases.append({
                "name": f"ohlcv_{size//1000}k_seed{seed}",
                "input_seed": seed,
                "input_length": size,
                "output": {
                    "k": array_to_json(slowk),
                    "d": array_to_json(slowd),
                },
            })

    return {
        "indicator": "STOCH",
        "parameters": {
            "k_period": k_period,
            "d_period": d_period,
        },
        "talib_version": talib.__version__,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "test_cases": test_cases,
    }


def generate_metadata() -> dict:
    """Generate metadata file with generation info."""
    return {
        "talib_version": talib.__version__,
        "numpy_version": np.__version__,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "data_sizes": DATA_SIZES,
        "seeds": SEEDS,
        "indicators": ["SMA", "EMA", "RSI", "MACD", "ATR", "BBANDS", "STOCH"],
        "generation_script": "tools/generate_golden.py",
    }


def main():
    """Generate all golden files."""
    # Create output directory
    GOLDEN_DIR.mkdir(parents=True, exist_ok=True)

    print(f"Generating golden files to {GOLDEN_DIR}/")
    print(f"TA-Lib version: {talib.__version__}")
    print(f"NumPy version: {np.__version__}")
    print()

    generators = [
        ("sma.json", generate_sma_golden),
        ("ema.json", generate_ema_golden),
        ("rsi.json", generate_rsi_golden),
        ("macd.json", generate_macd_golden),
        ("atr.json", generate_atr_golden),
        ("bollinger.json", generate_bollinger_golden),
        ("stochastic.json", generate_stochastic_golden),
    ]

    for filename, generator in generators:
        print(f"Generating {filename}...", end=" ", flush=True)
        data = generator()

        with open(GOLDEN_DIR / filename, "w") as f:
            json.dump(data, f, indent=2)

        test_count = len(data["test_cases"])
        print(f"done ({test_count} test cases)")

    # Generate metadata
    print("Generating metadata.json...", end=" ", flush=True)
    metadata = generate_metadata()
    with open(GOLDEN_DIR / "metadata.json", "w") as f:
        json.dump(metadata, f, indent=2)
    print("done")

    print()
    print("Golden file generation complete!")
    print(f"Files written to: {GOLDEN_DIR.absolute()}/")


if __name__ == "__main__":
    main()
