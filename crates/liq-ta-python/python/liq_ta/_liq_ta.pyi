"""Type stubs for liq-ta native module.

All functions support an optional `out` parameter for zero-copy output.
When `out` is provided, results are written directly to the array (zero-copy).
When `out` is None (default), a new array is allocated and returned.
"""

from typing import Tuple
import numpy as np
from numpy.typing import NDArray

# Moving Averages

def sma(
    data: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Simple Moving Average.

    Args:
        data: Input price array
        period: Number of periods for the moving average
        out: Optional pre-allocated output array (zero-copy if provided)

    Returns:
        Array with SMA values (first period-1 values are NaN)
    """
    ...

def ema(
    data: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Exponential Moving Average.

    Args:
        data: Input price array
        period: Number of periods for calculating smoothing factor
        out: Optional pre-allocated output array (zero-copy if provided)

    Returns:
        Array with EMA values (first period-1 values are NaN)
    """
    ...

def ema_wilder(
    data: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Wilder's Exponential Moving Average (alpha = 1/period).

    Args:
        data: Input price array
        period: Number of periods
        out: Optional pre-allocated output array (zero-copy if provided)

    Returns:
        Array with Wilder's EMA values
    """
    ...

# Momentum Indicators

def rsi(
    data: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Relative Strength Index.

    Args:
        data: Input price array
        period: Number of periods (typically 14)
        out: Optional pre-allocated output array (zero-copy if provided)

    Returns:
        Array with RSI values (0-100 range)
    """
    ...

def macd(
    data: NDArray[np.float64],
    fast_period: int = 12,
    slow_period: int = 26,
    signal_period: int = 9,
) -> Tuple[NDArray[np.float64], NDArray[np.float64], NDArray[np.float64]]:
    """Moving Average Convergence Divergence.

    Args:
        data: Input price array
        fast_period: Fast EMA period (default: 12)
        slow_period: Slow EMA period (default: 26)
        signal_period: Signal line period (default: 9)

    Returns:
        Tuple of (macd_line, signal_line, histogram) arrays
    """
    ...

def stochastic(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    k_period: int = 14,
    d_period: int = 3,
    k_slowing: int = 1,
) -> Tuple[NDArray[np.float64], NDArray[np.float64]]:
    """Stochastic Oscillator.

    Args:
        high: High prices
        low: Low prices
        close: Close prices
        k_period: %K lookback period (default: 14)
        d_period: %D smoothing period (default: 3)
        k_slowing: %K smoothing (1=fast, 3=slow, default: 1)

    Returns:
        Tuple of (%K, %D) arrays (values 0-100)
    """
    ...

def stochastic_fast(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    k_period: int = 14,
    d_period: int = 3,
) -> Tuple[NDArray[np.float64], NDArray[np.float64]]:
    """Fast Stochastic Oscillator (k_slowing=1)."""
    ...

def stochastic_slow(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    k_period: int = 14,
    d_period: int = 3,
) -> Tuple[NDArray[np.float64], NDArray[np.float64]]:
    """Slow Stochastic Oscillator (k_slowing=3)."""
    ...

def williams_r(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Williams %R.

    Args:
        high: High prices
        low: Low prices
        close: Close prices
        period: Lookback period (typically 14)
        out: Optional pre-allocated output array (zero-copy if provided)

    Returns:
        Array with Williams %R values (-100 to 0)
    """
    ...

def adx(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    period: int,
) -> Tuple[NDArray[np.float64], NDArray[np.float64], NDArray[np.float64]]:
    """Average Directional Index.

    Args:
        high: High prices
        low: Low prices
        close: Close prices
        period: Number of periods (typically 14)

    Returns:
        Tuple of (adx, plus_di, minus_di) arrays
    """
    ...

# Volatility Indicators

def atr(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Average True Range.

    Args:
        high: High prices
        low: Low prices
        close: Close prices
        period: Number of periods (typically 14)
        out: Optional pre-allocated output array (zero-copy if provided)

    Returns:
        Array with ATR values
    """
    ...

def true_range(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """True Range.

    Args:
        high: High prices
        low: Low prices
        close: Close prices
        out: Optional pre-allocated output array (zero-copy if provided)

    Returns:
        Array with True Range values
    """
    ...

def bollinger(
    data: NDArray[np.float64],
    period: int = 20,
    std_dev: float = 2.0,
) -> Tuple[NDArray[np.float64], NDArray[np.float64], NDArray[np.float64]]:
    """Bollinger Bands.

    Args:
        data: Input price array
        period: Number of periods (default: 20)
        std_dev: Standard deviation multiplier (default: 2.0)

    Returns:
        Tuple of (upper_band, middle_band, lower_band) arrays
    """
    ...

def donchian(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    period: int = 20,
) -> Tuple[NDArray[np.float64], NDArray[np.float64], NDArray[np.float64]]:
    """Donchian Channels.

    Args:
        high: High prices
        low: Low prices
        period: Lookback period (default: 20)

    Returns:
        Tuple of (upper, middle, lower) arrays
    """
    ...

def rolling_stddev(
    data: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Rolling Standard Deviation.

    Args:
        data: Input array
        period: Window size
        out: Optional pre-allocated output array (zero-copy if provided)

    Returns:
        Array with rolling standard deviation values
    """
    ...

# Volume Indicators

def obv(
    close: NDArray[np.float64],
    volume: NDArray[np.float64],
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """On-Balance Volume.

    Args:
        close: Close prices
        volume: Volume values
        out: Optional pre-allocated output array (zero-copy if provided)

    Returns:
        Array with OBV values
    """
    ...

def vwap(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    volume: NDArray[np.float64],
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Volume Weighted Average Price.

    Args:
        high: High prices
        low: Low prices
        close: Close prices
        volume: Volume values
        out: Optional pre-allocated output array (zero-copy if provided)

    Returns:
        Array with VWAP values
    """
    ...
