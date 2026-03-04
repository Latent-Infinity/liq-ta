import numpy as np

import liq_ta


def _sample_data(n: int = 260):
    close = np.linspace(100.0, 150.0, n)
    open_ = close - 0.1
    high = close + 0.8
    low = close - 0.8
    return open_, high, low, close


def test_stage3_surface_parity_shapes_and_nan_prefixes():
    n = 260
    open_, high, low, close = _sample_data(n)

    # MA
    hma = liq_ta.hma(close, 21)
    assert len(hma) == n
    assert np.isnan(hma[:10]).any()

    # Trend
    supertrend, upper, lower, trend = liq_ta.supertrend(high, low, close, 10, 3.0)
    assert len(supertrend) == n
    assert len(upper) == n
    assert len(lower) == n
    assert len(trend) == n
    assert np.isnan(supertrend[:10]).all()

    # Momentum
    osma = liq_ta.osma(close, 12, 26, 9)
    assert len(osma) == n
    assert np.isnan(osma[:20]).any()

    # Volatility
    chop = liq_ta.chop(high, low, close, 14)
    assert len(chop) == n
    assert np.isnan(chop[:14]).all()

    # Regime
    center, g_upper, g_lower, g_trend = liq_ta.gaussian_channel(close, 20, 0.5, 2.0)
    assert len(center) == n
    assert len(g_upper) == n
    assert len(g_lower) == n
    assert len(g_trend) == n
    assert np.isnan(center[:19]).all()
    finite_trend = g_trend[~np.isnan(g_trend)]
    assert finite_trend.size > 0
    assert set(np.unique(finite_trend)).issubset({-1.0, 0.0, 1.0})
