#!/usr/bin/env python3
"""
liq-ta Validation Script

This script validates all indicators exposed by the liq_ta Python package using
its indicator registry. It verifies that each indicator can be called with
standard inputs and that output shapes are correct.

Usage:
    python -m liq_ta.validate

Exit codes:
    0 - All tests passed
    1 - One or more tests failed
"""

from __future__ import annotations

import sys
from typing import Any, Dict, List, Tuple

import numpy as np

DEFAULT_PARAMS: Dict[str, Any] = {
    "period": 14,
    "period1": 7,
    "period2": 14,
    "period3": 28,
    "fast_period": 12,
    "slow_period": 26,
    "signal_period": 9,
    "d_period": 3,
    "k_period": 14,
    "k_slowing": 1,
    "stoch_period": 14,
    "rsi_period": 14,
    "min_period": 2,
    "max_period": 30,
    "fast_limit": 0.5,
    "slow_limit": 0.05,
    "vfactor": 0.7,
    "std_dev": 2.0,
    "af_start": 0.02,
    "af_step": 0.02,
    "af_max": 0.2,
    "af_init_long": 0.02,
    "af_init_short": 0.02,
    "af_long": 0.02,
    "af_short": 0.02,
    "af_max_long": 0.2,
    "af_max_short": 0.2,
    "offset_on_reverse": 0.0,
    "start_value": 0.0,
}


def build_inputs(n: int) -> Dict[str, np.ndarray]:
    np.random.seed(42)

    close = 100 + np.cumsum(np.random.randn(n) * 0.5)
    high = close + np.abs(np.random.randn(n) * 0.3)
    low = close - np.abs(np.random.randn(n) * 0.3)
    open_ = close + np.random.randn(n) * 0.1
    volume = np.abs(np.random.randn(n) * 1000) + 1000

    data1 = close * 1.01
    data0 = close

    periods = np.linspace(2, 30, n, dtype=np.float64)

    return {
        "data": close,
        "data0": data0,
        "data1": data1,
        "close": close,
        "high": high,
        "low": low,
        "open": open_,
        "volume": volume,
        "periods": periods,
    }


def build_params(param_names: List[str]) -> Tuple[Dict[str, Any] | None, str | None]:
    params: Dict[str, Any] = {}
    for name in param_names:
        if name in DEFAULT_PARAMS:
            params[name] = DEFAULT_PARAMS[name]
        else:
            return None, f"unsupported param '{name}'"
    return params, None


def resolve_inputs(inputs: List[str], data: Dict[str, np.ndarray]) -> Tuple[List[np.ndarray] | None, str | None]:
    args: List[np.ndarray] = []
    for name in inputs:
        if name not in data:
            return None, f"unsupported input '{name}'"
        args.append(data[name])
    return args, None


def validate_output(name: str, outputs: List[str], result: Any, expected_len: int) -> Tuple[bool, str | None]:
    if len(outputs) == 1:
        if isinstance(result, tuple):
            return False, "expected single output, got tuple"
        try:
            if len(result) != expected_len:
                return False, f"length mismatch: {len(result)} != {expected_len}"
        except TypeError:
            return False, "output is not sequence-like"
        return True, None

    if not isinstance(result, tuple):
        return False, "expected tuple output"
    if len(result) != len(outputs):
        return False, f"tuple size mismatch: {len(result)} != {len(outputs)}"
    for series in result:
        if len(series) != expected_len:
            return False, f"length mismatch: {len(series)} != {expected_len}"
    return True, None


def validate_indicator(name: str, meta: Dict[str, Any], data: Dict[str, np.ndarray]) -> Tuple[str, str | None]:
    import liq_ta

    func = getattr(liq_ta, name, None)
    if func is None:
        return "SKIP", "function not found"

    inputs = meta.get("inputs", [])
    params = meta.get("params", [])
    outputs = meta.get("outputs", [name])
    supports_out = bool(meta.get("supports_out", False))

    args, err = resolve_inputs(inputs, data)
    if err:
        return "SKIP", err

    kwargs, err = build_params(params)
    if err:
        return "SKIP", err

    try:
        result = func(*args, **kwargs)
    except Exception as exc:  # pylint: disable=broad-except
        return "FAIL", str(exc)

    ok, err = validate_output(name, outputs, result, len(data["close"]))
    if not ok:
        return "FAIL", err

    if supports_out and len(outputs) == 1:
        try:
            out = np.empty(len(data["close"]), dtype=np.float64)
            out_result = func(*args, **kwargs, out=out)
            if out_result is not out:
                return "FAIL", "out= did not return the provided buffer"
        except Exception as exc:  # pylint: disable=broad-except
            return "FAIL", f"out= failed: {exc}"

    return "PASS", None


def main() -> None:
    print("=" * 60)
    print("liq-ta Validation")
    print("=" * 60)

    try:
        import liq_ta
        print(f"\n[OK] liq-ta version: {liq_ta.__version__}")
    except ImportError as exc:
        print(f"\n[FAIL] Could not import liq_ta: {exc}")
        print("\nMake sure liq-ta is installed:")
        print("  cd crates/liq-ta-python && maturin develop")
        sys.exit(1)

    data = build_inputs(100)

    results: Dict[str, str] = {}
    errors: List[Tuple[str, str]] = []
    skips: List[Tuple[str, str]] = []

    indicators = liq_ta.INDICATORS
    by_category: Dict[str, List[str]] = {}
    for name, meta in indicators.items():
        by_category.setdefault(meta.get("category", "other"), []).append(name)

    for category in sorted(by_category):
        print(f"\n--- {category.replace('_', ' ').title()} ---")
        for name in sorted(by_category[category]):
            status, detail = validate_indicator(name, indicators[name], data)
            results[name] = status
            if status == "PASS":
                print(f"  [OK] {name}")
            elif status == "SKIP":
                skips.append((name, detail or "skipped"))
                print(f"  [SKIP] {name} - {detail}")
            else:
                errors.append((name, detail or "failed"))
                print(f"  [FAIL] {name} - {detail}")

    print("\n" + "=" * 60)
    print("VALIDATION SUMMARY")
    print("=" * 60)

    passed = sum(1 for v in results.values() if v == "PASS")
    total = len(results)
    print(f"\nTests passed: {passed}/{total}")

    if skips:
        print(f"Skipped: {len(skips)}")

    if errors:
        print("\nFailed tests:")
        for name, error in errors:
            print(f"  - {name}: {error}")
        print("\nPlease report these errors to the liq-ta team.")
        sys.exit(1)

    print("\nAll tests passed! liq-ta is working correctly.")
    print("\nYou can now use liq-ta in your projects:")
    print(
        """
import numpy as np
import liq_ta

prices = np.array([...])  # Your price data

# Calculate indicators
sma = liq_ta.sma(prices, 20)
rsi = liq_ta.rsi(prices, 14)

# Zero-copy for performance
out = np.empty(len(prices))
liq_ta.sma(prices, 20, out=out)
"""
    )


if __name__ == "__main__":
    main()
