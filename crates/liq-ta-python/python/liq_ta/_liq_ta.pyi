"""Type stubs for liq-ta native module.

All functions support an optional `out` parameter for zero-copy output.
When `out` is provided, results are written directly to the array (zero-copy).
When `out` is None (default), a new array is allocated and returned.
"""

from typing import Tuple
import numpy as np
from numpy.typing import NDArray

def get_indicator_registry() -> list[
    tuple[
        str,
        str,
        str,
        list[str],
        list[str],
        list[str],
        bool,
    ]
]:
    """Runtime indicator metadata source used by Python wrapper helpers.

    Returns:
        A list of tuples:
        (name, category, input_shape, inputs, params, outputs, supports_out)
    """
    ...

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

def qqe(
    data: NDArray[np.float64],
    rsi_period: int = 14,
    smoothing_period: int = 5,
    wilders_period: int = 14,
    factor: float = 4.236,
) -> Tuple[NDArray[np.float64], NDArray[np.float64], NDArray[np.float64]]:
    """Quantitative Qualitative Estimation.

    Args:
        data: Input price array
        rsi_period: RSI period
        smoothing_period: RSI smoothing EMA period
        wilders_period: Volatility smoothing period
        factor: Band multiplier

    Returns:
        Tuple of (qqe, upper_band, lower_band) arrays
    """
    ...

def ao(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
) -> NDArray[np.float64]:
    """Awesome Oscillator."""
    ...

def bulls_power(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Bulls Power."""
    ...

def bears_power(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Bears Power."""
    ...

def demarker(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    period: int = 14,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """DeMarker."""
    ...

def osma(
    data: NDArray[np.float64],
    fast_period: int = 12,
    slow_period: int = 26,
    signal_period: int = 9,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """OSMA."""
    ...

def vortex(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    period: int = 14,
) -> Tuple[NDArray[np.float64], NDArray[np.float64]]:
    """Vortex indicator."""
    ...

def rvi(
    open: NDArray[np.float64],
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    period: int = 10,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Relative Vigor Index."""
    ...

def dpo(
    data: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Detrended Price Oscillator."""
    ...

def connors_rsi(
    data: NDArray[np.float64],
    rsi_period: int = 3,
    streak_period: int = 2,
    rank_period: int = 100,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Connors RSI."""
    ...

def stc(
    data: NDArray[np.float64],
    fast_period: int = 23,
    slow_period: int = 50,
    cycle_period: int = 10,
    smooth_period: int = 3,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Schaff Trend Cycle."""
    ...

def laguerre_rsi(
    data: NDArray[np.float64],
    gamma: float = 0.5,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Laguerre RSI."""
    ...

def dss_bressert(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    stochastic_period: int = 14,
    ema_period: int = 5,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Double Smoothed Stochastic Bressert."""
    ...

def supertrend(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    period: int = 10,
    multiplier: float = 3.0,
) -> Tuple[
    NDArray[np.float64],
    NDArray[np.float64],
    NDArray[np.float64],
    NDArray[np.float64],
]:
    """Super Trend."""
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

def ichimoku(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    tenkan_period: int = 9,
    kijun_period: int = 26,
    senkou_b_period: int = 52,
    displacement: int = 26,
) -> Tuple[
    NDArray[np.float64],
    NDArray[np.float64],
    NDArray[np.float64],
    NDArray[np.float64],
    NDArray[np.float64],
]:
    """Ichimoku Kinko Hyo.

    Returns:
        Tuple of (tenkan, kijun, senkou_a, senkou_b, chikou) arrays
    """
    ...

def hma(
    data: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Hull Moving Average."""
    ...

def gaussian_filter(
    data: NDArray[np.float64],
    period: int = 20,
    sigma: float = 0.5,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Gaussian smoothing filter."""
    ...

def gaussian_channel(
    data: NDArray[np.float64],
    period: int = 20,
    sigma: float = 0.5,
    multiplier: float = 2.0,
) -> Tuple[
    NDArray[np.float64],
    NDArray[np.float64],
    NDArray[np.float64],
    NDArray[np.float64],
]:
    """Gaussian Channel."""
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

def chop(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Choppiness Index."""
    ...

def ulcer_index(
    data: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Ulcer Index."""
    ...

def hurst(
    data: NDArray[np.float64],
    period: int,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Hurst Exponent."""
    ...

def autocorr(
    data: NDArray[np.float64],
    period: int = 32,
    lag: int = 1,
    out: NDArray[np.float64] | None = None,
) -> NDArray[np.float64]:
    """Autocorrelation."""
    ...

def hma_atr_bands(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    hma_period: int = 21,
    atr_period: int = 14,
    atr_multiplier: float = 2.0,
) -> Tuple[NDArray[np.float64], NDArray[np.float64], NDArray[np.float64]]:
    """HMA ATR Bands."""
    ...

def hma_bollinger_bands(
    data: NDArray[np.float64],
    hma_period: int = 21,
    std_period: int = 20,
    std_multiplier: float = 2.0,
) -> Tuple[NDArray[np.float64], NDArray[np.float64], NDArray[np.float64]]:
    """HMA Bollinger Bands."""
    ...

def vwap_atr_bands(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    volume: NDArray[np.float64],
    atr_period: int = 14,
    atr_multiplier: float = 2.0,
) -> Tuple[NDArray[np.float64], NDArray[np.float64], NDArray[np.float64]]:
    """VWAP ATR Bands."""
    ...

def vwap_bollinger_bands(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    volume: NDArray[np.float64],
    std_period: int = 20,
    std_multiplier: float = 2.0,
) -> Tuple[NDArray[np.float64], NDArray[np.float64], NDArray[np.float64]]:
    """VWAP Bollinger Bands."""
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

def keltner_channel(
    high: NDArray[np.float64],
    low: NDArray[np.float64],
    close: NDArray[np.float64],
    period: int = 20,
    atr_multiplier: float = 2.0,
) -> Tuple[NDArray[np.float64], NDArray[np.float64], NDArray[np.float64]]:
    """Keltner Channel.

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
