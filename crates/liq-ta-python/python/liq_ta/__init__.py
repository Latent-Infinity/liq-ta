"""liq-ta: High-performance technical analysis library.

This library provides fast implementations of common technical analysis
indicators, powered by Rust with NumPy array support.

All functions support an optional `out=` parameter for zero-copy output:
    # Allocating (convenient)
    result = liq_ta.sma(prices, 20)

    # Zero-copy (for performance-critical code)
    out = np.empty(len(prices))
    liq_ta.sma(prices, 20, out=out)

Example:
    >>> import numpy as np
    >>> import liq_ta
    >>>
    >>> prices = np.array([44.0, 44.5, 43.5, 44.0, 44.5, 45.0, 45.5, 46.0])
    >>> sma = liq_ta.sma(prices, 5)
    >>> print(sma)  # First 4 values are NaN
"""

from liq_ta._liq_ta import (
    # Moving Averages
    sma,
    ema,
    ema_wilder,
    wma,
    dema,
    tema,
    trima,
    midpoint,
    midprice,
    kama,
    t3,
    sar,
    sarext,
    ht_trendline,
    mama,
    mavp,
    hma,
    gaussian_filter,
    gaussian_channel,
    # Momentum
    rsi,
    macd,
    qqe,
    ao,
    bulls_power,
    bears_power,
    demarker,
    osma,
    vortex,
    rvi,
    dpo,
    connors_rsi,
    stc,
    laguerre_rsi,
    dss_bressert,
    supertrend,
    mom,
    roc,
    rocp,
    rocr,
    rocr100,
    apo,
    ppo,
    bop,
    aroon,
    aroonosc,
    cci,
    cmo,
    mfi,
    stochrsi,
    trix,
    ultosc,
    adxr,
    dx,
    plus_dm,
    minus_dm,
    stochastic,
    stochastic_fast,
    stochastic_slow,
    williams_r,
    adx,
    ichimoku,
    # Volatility
    atr,
    true_range,
    bollinger,
    donchian,
    keltner_channel,
    rolling_stddev,
    chop,
    ulcer_index,
    hurst,
    autocorr,
    hma_atr_bands,
    hma_bollinger_bands,
    vwap_atr_bands,
    vwap_bollinger_bands,
    # Volume
    obv,
    vwap,
    ad,
    adosc,
    # Cycle (Hilbert Transform)
    ht_dcperiod,
    ht_dcphase,
    ht_phasor,
    ht_sine,
    ht_trendmode,
    # Price Transform
    avgprice,
    medprice,
    typprice,
    wclprice,
    # Statistics
    var,
    correl,
    beta,
    linearreg,
    linearreg_slope,
    linearreg_intercept,
    linearreg_angle,
    tsf,
    # Candlestick Patterns - Single-candle
    cdl_doji,
    cdl_dragonfly_doji,
    cdl_gravestone_doji,
    cdl_longleg_doji,
    cdl_rickshaw_man,
    cdl_marubozu,
    cdl_closing_marubozu,
    cdl_spinning_top,
    cdl_high_wave,
    cdl_long_line,
    cdl_short_line,
    cdl_hammer,
    cdl_hanging_man,
    cdl_inverted_hammer,
    cdl_shooting_star,
    cdl_takuri,
    cdl_belt_hold,
    # Candlestick Patterns - Two-candle
    cdl_engulfing,
    cdl_harami,
    cdl_harami_cross,
    cdl_piercing,
    cdl_dark_cloud_cover,
    cdl_doji_star,
    cdl_kicking,
    cdl_kicking_by_length,
    cdl_matching_low,
    cdl_homing_pigeon,
    cdl_in_neck,
    cdl_on_neck,
    cdl_thrusting,
    cdl_separating_lines,
    cdl_counter_attack,
    cdl_2crows,
    cdl_hikkake,
    cdl_hikkake_mod,
    # Candlestick Patterns - Three-candle and complex
    cdl_morning_star,
    cdl_evening_star,
    cdl_morning_doji_star,
    cdl_evening_doji_star,
    cdl_abandoned_baby,
    cdl_3white_soldiers,
    cdl_3black_crows,
    cdl_3inside,
    cdl_3outside,
    cdl_3line_strike,
    cdl_3stars_in_south,
    cdl_tristar,
    cdl_identical_3crows,
    cdl_stick_sandwich,
    cdl_unique_3river,
    cdl_advance_block,
    cdl_stalled_pattern,
    cdl_tasuki_gap,
    cdl_upside_gap_2crows,
    cdl_gap_side_side_white,
    cdl_breakaway,
    cdl_ladder_bottom,
    cdl_mat_hold,
    cdl_rise_fall_3methods,
    cdl_concealing_baby_swallow,
    cdl_xside_gap_3methods,
    get_indicator_registry,
)

class LiqTaError(ValueError):
    """Base class for user-facing liq-ta API errors."""


class IndicatorNotFoundError(LiqTaError):
    """Raised when an indicator name cannot be resolved."""


class IndicatorArgumentError(LiqTaError):
    """Raised when indicator arguments or argument shapes are invalid."""


class IndicatorMetadataError(LiqTaError):
    """Raised when runtime indicator metadata is inconsistent."""


_RUNTIME_INDICATOR_METADATA = {
    name: {
        "name": name,
        "category": category,
        "input_shape": input_shape,
        "inputs": list(inputs),
        "params": list(params),
        "outputs": list(outputs),
        "supports_out": supports_out,
    }
    for name, category, input_shape, inputs, params, outputs, supports_out in get_indicator_registry()
}


# Indicator metadata registry
INDICATORS = {
    # Moving Averages
    "sma": {
        "name": "Simple Moving Average",
        "category": "moving_average",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["sma"],
        "supports_out": True,
    },
    "ema": {
        "name": "Exponential Moving Average",
        "category": "moving_average",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["ema"],
        "supports_out": True,
    },
    "ema_wilder": {
        "name": "Wilder's Exponential Moving Average",
        "category": "moving_average",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["ema_wilder"],
        "supports_out": True,
    },
    "wma": {
        "name": "Weighted Moving Average",
        "category": "moving_average",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["wma"],
        "supports_out": True,
    },
    "dema": {
        "name": "Double Exponential Moving Average",
        "category": "moving_average",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["dema"],
        "supports_out": True,
    },
    "tema": {
        "name": "Triple Exponential Moving Average",
        "category": "moving_average",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["tema"],
        "supports_out": True,
    },
    "trima": {
        "name": "Triangular Moving Average",
        "category": "moving_average",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["trima"],
        "supports_out": True,
    },
    "midpoint": {
        "name": "Midpoint Over Period",
        "category": "moving_average",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["midpoint"],
        "supports_out": True,
    },
    "midprice": {
        "name": "Midpoint Price Over Period",
        "category": "moving_average",
        "inputs": ["high", "low"],
        "params": ["period"],
        "outputs": ["midprice"],
        "supports_out": True,
    },
    "kama": {
        "name": "Kaufman Adaptive Moving Average",
        "category": "moving_average",
        "inputs": ["data"],
        "params": ["period", "fast_period", "slow_period"],
        "outputs": ["kama"],
        "supports_out": True,
    },
    "t3": {
        "name": "Tillson T3 Moving Average",
        "category": "moving_average",
        "inputs": ["data"],
        "params": ["period", "vfactor"],
        "outputs": ["t3"],
        "supports_out": True,
    },
    "sar": {
        "name": "Parabolic SAR",
        "category": "moving_average",
        "inputs": ["high", "low"],
        "params": ["af_start", "af_step", "af_max"],
        "outputs": ["sar"],
        "supports_out": True,
    },
    "sarext": {
        "name": "Extended Parabolic SAR",
        "category": "moving_average",
        "inputs": ["high", "low"],
        "params": ["start_value", "offset_on_reverse", "af_init_long", "af_long", "af_max_long", "af_init_short", "af_short", "af_max_short"],
        "outputs": ["sarext"],
        "supports_out": True,
    },
    "ht_trendline": {
        "name": "Hilbert Transform - Instantaneous Trendline",
        "category": "moving_average",
        "inputs": ["data"],
        "params": [],
        "outputs": ["trendline"],
        "supports_out": True,
    },
    "mama": {
        "name": "MESA Adaptive Moving Average",
        "category": "moving_average",
        "inputs": ["data"],
        "params": ["fast_limit", "slow_limit"],
        "outputs": ["mama", "fama"],
        "supports_out": False,
    },
    "mavp": {
        "name": "Moving Average Variable Period",
        "category": "moving_average",
        "inputs": ["data", "periods"],
        "params": ["min_period", "max_period"],
        "outputs": ["mavp"],
        "supports_out": True,
    },
    # Momentum
    "mom": {
        "name": "Momentum",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["mom"],
        "supports_out": True,
    },
    "roc": {
        "name": "Rate of Change",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["roc"],
        "supports_out": True,
    },
    "rocp": {
        "name": "Rate of Change Percentage",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["rocp"],
        "supports_out": True,
    },
    "rocr": {
        "name": "Rate of Change Ratio",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["rocr"],
        "supports_out": True,
    },
    "rocr100": {
        "name": "Rate of Change Ratio 100",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["rocr100"],
        "supports_out": True,
    },
    "apo": {
        "name": "Absolute Price Oscillator",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["fast_period", "slow_period"],
        "outputs": ["apo"],
        "supports_out": True,
    },
    "ppo": {
        "name": "Percentage Price Oscillator",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["fast_period", "slow_period"],
        "outputs": ["ppo"],
        "supports_out": True,
    },
    "bop": {
        "name": "Balance of Power",
        "category": "momentum",
        "inputs": ["open", "high", "low", "close"],
        "params": [],
        "outputs": ["bop"],
        "supports_out": True,
    },
    "aroon": {
        "name": "Aroon Indicator",
        "category": "momentum",
        "inputs": ["high", "low"],
        "params": ["period"],
        "outputs": ["aroon_up", "aroon_down"],
        "supports_out": False,
    },
    "aroonosc": {
        "name": "Aroon Oscillator",
        "category": "momentum",
        "inputs": ["high", "low"],
        "params": ["period"],
        "outputs": ["aroonosc"],
        "supports_out": True,
    },
    "cci": {
        "name": "Commodity Channel Index",
        "category": "momentum",
        "inputs": ["high", "low", "close"],
        "params": ["period"],
        "outputs": ["cci"],
        "supports_out": True,
    },
    "cmo": {
        "name": "Chande Momentum Oscillator",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["cmo"],
        "supports_out": True,
    },
    "mfi": {
        "name": "Money Flow Index",
        "category": "momentum",
        "inputs": ["high", "low", "close", "volume"],
        "params": ["period"],
        "outputs": ["mfi"],
        "supports_out": True,
    },
    "stochrsi": {
        "name": "Stochastic RSI",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["rsi_period", "stoch_period", "k_period", "d_period"],
        "outputs": ["fastk", "fastd"],
        "supports_out": False,
    },
    "trix": {
        "name": "Triple Exponential Average",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["trix"],
        "supports_out": True,
    },
    "ultosc": {
        "name": "Ultimate Oscillator",
        "category": "momentum",
        "inputs": ["high", "low", "close"],
        "params": ["period1", "period2", "period3"],
        "outputs": ["ultosc"],
        "supports_out": True,
    },
    "rsi": {
        "name": "Relative Strength Index",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["rsi"],
        "supports_out": True,
    },
    "macd": {
        "name": "Moving Average Convergence Divergence",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["fast_period", "slow_period", "signal_period"],
        "outputs": ["macd_line", "signal_line", "histogram"],
        "supports_out": False,
    },
    "qqe": {
        "name": "Quantitative Qualitative Estimation",
        "category": "momentum",
        "inputs": ["data"],
        "params": ["rsi_period", "smoothing_period", "wilders_period", "factor"],
        "outputs": ["qqe", "upper_band", "lower_band"],
        "supports_out": False,
    },
    "stochastic": {
        "name": "Stochastic Oscillator",
        "category": "momentum",
        "inputs": ["high", "low", "close"],
        "params": ["k_period", "d_period", "k_slowing"],
        "outputs": ["k", "d"],
        "supports_out": False,
    },
    "stochastic_fast": {
        "name": "Fast Stochastic Oscillator",
        "category": "momentum",
        "inputs": ["high", "low", "close"],
        "params": ["k_period", "d_period"],
        "outputs": ["k", "d"],
        "supports_out": False,
    },
    "stochastic_slow": {
        "name": "Slow Stochastic Oscillator",
        "category": "momentum",
        "inputs": ["high", "low", "close"],
        "params": ["k_period", "d_period"],
        "outputs": ["k", "d"],
        "supports_out": False,
    },
    "williams_r": {
        "name": "Williams %R",
        "category": "momentum",
        "inputs": ["high", "low", "close"],
        "params": ["period"],
        "outputs": ["williams_r"],
        "supports_out": True,
    },
    "adx": {
        "name": "Average Directional Index",
        "category": "momentum",
        "inputs": ["high", "low", "close"],
        "params": ["period"],
        "outputs": ["adx", "plus_di", "minus_di"],
        "supports_out": False,
    },
    "ichimoku": {
        "name": "Ichimoku Kinko Hyo",
        "category": "trend",
        "inputs": ["high", "low", "close"],
        "params": ["tenkan_period", "kijun_period", "senkou_b_period", "displacement"],
        "outputs": ["tenkan", "kijun", "senkou_a", "senkou_b", "chikou"],
        "supports_out": False,
    },
    "adxr": {
        "name": "Average Directional Movement Index Rating",
        "category": "momentum",
        "inputs": ["high", "low", "close"],
        "params": ["period"],
        "outputs": ["adxr"],
        "supports_out": True,
    },
    "dx": {
        "name": "Directional Movement Index",
        "category": "momentum",
        "inputs": ["high", "low", "close"],
        "params": ["period"],
        "outputs": ["dx"],
        "supports_out": True,
    },
    "plus_dm": {
        "name": "Plus Directional Movement",
        "category": "momentum",
        "inputs": ["high", "low"],
        "params": ["period"],
        "outputs": ["plus_dm"],
        "supports_out": True,
    },
    "minus_dm": {
        "name": "Minus Directional Movement",
        "category": "momentum",
        "inputs": ["high", "low"],
        "params": ["period"],
        "outputs": ["minus_dm"],
        "supports_out": True,
    },
    # Volatility
    "atr": {
        "name": "Average True Range",
        "category": "volatility",
        "inputs": ["high", "low", "close"],
        "params": ["period"],
        "outputs": ["atr"],
        "supports_out": True,
    },
    "true_range": {
        "name": "True Range",
        "category": "volatility",
        "inputs": ["high", "low", "close"],
        "params": [],
        "outputs": ["true_range"],
        "supports_out": True,
    },
    "bollinger": {
        "name": "Bollinger Bands",
        "category": "volatility",
        "inputs": ["data"],
        "params": ["period", "std_dev"],
        "outputs": ["upper", "middle", "lower"],
        "supports_out": False,
    },
    "donchian": {
        "name": "Donchian Channels",
        "category": "volatility",
        "inputs": ["high", "low"],
        "params": ["period"],
        "outputs": ["upper", "middle", "lower"],
        "supports_out": False,
    },
    "keltner_channel": {
        "name": "Keltner Channel",
        "category": "volatility",
        "inputs": ["high", "low", "close"],
        "params": ["period", "atr_multiplier"],
        "outputs": ["upper", "middle", "lower"],
        "supports_out": False,
    },
    "rolling_stddev": {
        "name": "Rolling Standard Deviation",
        "category": "volatility",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["stddev"],
        "supports_out": True,
    },
    # Volume
    "obv": {
        "name": "On-Balance Volume",
        "category": "volume",
        "inputs": ["close", "volume"],
        "params": [],
        "outputs": ["obv"],
        "supports_out": True,
    },
    "vwap": {
        "name": "Volume Weighted Average Price",
        "category": "volume",
        "inputs": ["high", "low", "close", "volume"],
        "params": [],
        "outputs": ["vwap"],
        "supports_out": True,
    },
    "ad": {
        "name": "Chaikin A/D Line",
        "category": "volume",
        "inputs": ["high", "low", "close", "volume"],
        "params": [],
        "outputs": ["ad"],
        "supports_out": True,
    },
    "adosc": {
        "name": "Chaikin A/D Oscillator",
        "category": "volume",
        "inputs": ["high", "low", "close", "volume"],
        "params": ["fast_period", "slow_period"],
        "outputs": ["adosc"],
        "supports_out": True,
    },
    # Cycle Indicators (Hilbert Transform)
    "ht_dcperiod": {
        "name": "Hilbert Transform - Dominant Cycle Period",
        "category": "cycle",
        "inputs": ["data"],
        "params": [],
        "outputs": ["dcperiod"],
        "supports_out": True,
    },
    "ht_dcphase": {
        "name": "Hilbert Transform - Dominant Cycle Phase",
        "category": "cycle",
        "inputs": ["data"],
        "params": [],
        "outputs": ["dcphase"],
        "supports_out": True,
    },
    "ht_phasor": {
        "name": "Hilbert Transform - Phasor Components",
        "category": "cycle",
        "inputs": ["data"],
        "params": [],
        "outputs": ["inphase", "quadrature"],
        "supports_out": False,
    },
    "ht_sine": {
        "name": "Hilbert Transform - SineWave",
        "category": "cycle",
        "inputs": ["data"],
        "params": [],
        "outputs": ["sine", "lead_sine"],
        "supports_out": False,
    },
    "ht_trendmode": {
        "name": "Hilbert Transform - Trend vs Cycle Mode",
        "category": "cycle",
        "inputs": ["data"],
        "params": [],
        "outputs": ["trendmode"],
        "supports_out": True,
    },
    # Price Transform
    "avgprice": {
        "name": "Average Price",
        "category": "price_transform",
        "inputs": ["open", "high", "low", "close"],
        "params": [],
        "outputs": ["avgprice"],
        "supports_out": True,
    },
    "medprice": {
        "name": "Median Price",
        "category": "price_transform",
        "inputs": ["high", "low"],
        "params": [],
        "outputs": ["medprice"],
        "supports_out": True,
    },
    "typprice": {
        "name": "Typical Price",
        "category": "price_transform",
        "inputs": ["high", "low", "close"],
        "params": [],
        "outputs": ["typprice"],
        "supports_out": True,
    },
    "wclprice": {
        "name": "Weighted Close Price",
        "category": "price_transform",
        "inputs": ["high", "low", "close"],
        "params": [],
        "outputs": ["wclprice"],
        "supports_out": True,
    },
    # Statistics
    "var": {
        "name": "Variance",
        "category": "statistic",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["var"],
        "supports_out": True,
    },
    "correl": {
        "name": "Pearson's Correlation Coefficient",
        "category": "statistic",
        "inputs": ["data0", "data1"],
        "params": ["period"],
        "outputs": ["correl"],
        "supports_out": True,
    },
    "beta": {
        "name": "Beta",
        "category": "statistic",
        "inputs": ["data0", "data1"],
        "params": ["period"],
        "outputs": ["beta"],
        "supports_out": True,
    },
    "linearreg": {
        "name": "Linear Regression",
        "category": "statistic",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["linearreg"],
        "supports_out": True,
    },
    "linearreg_slope": {
        "name": "Linear Regression Slope",
        "category": "statistic",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["slope"],
        "supports_out": True,
    },
    "linearreg_intercept": {
        "name": "Linear Regression Intercept",
        "category": "statistic",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["intercept"],
        "supports_out": True,
    },
    "linearreg_angle": {
        "name": "Linear Regression Angle",
        "category": "statistic",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["angle"],
        "supports_out": True,
    },
    "tsf": {
        "name": "Time Series Forecast",
        "category": "statistic",
        "inputs": ["data"],
        "params": ["period"],
        "outputs": ["tsf"],
        "supports_out": True,
    },
}

# Auto-merge indicators from Rust registry not already in INDICATORS
for _name, _meta in _RUNTIME_INDICATOR_METADATA.items():
    if _name not in INDICATORS:
        INDICATORS[_name] = {
            "name": _meta["name"],
            "category": _meta["category"],
            "inputs": _meta["inputs"],
            "params": _meta["params"],
            "outputs": _meta["outputs"],
            "supports_out": _meta["supports_out"],
        }


def _combined_indicator_metadata():
    merged = dict(INDICATORS)
    for name, meta in _RUNTIME_INDICATOR_METADATA.items():
        base = merged.pop(name, None)
        if base is None:
            merged[name] = meta
        else:
            merged[name] = {**base, **meta}
    return merged


def _normalize_indicator_name(name):
    if not isinstance(name, str):
        raise IndicatorArgumentError(
            f"indicator name must be a string, got {type(name).__name__}"
        )
    normalized = name.strip()
    if not normalized:
        raise IndicatorArgumentError("indicator name must not be empty")
    return normalized


def _known_categories(metadata):
    return sorted({meta["category"] for meta in metadata.values()})


def list_indicators(category=None):
    """List available indicators with metadata.

    Args:
        category: Optional filter by category. One of:
            'moving_average', 'momentum', 'volatility', 'volume'

    Returns:
        List of indicator info dicts with keys:
            - function: The indicator function name
            - name: Human-readable name
            - category: Indicator category
            - inputs: Required input arrays
            - params: Configuration parameters
            - outputs: Output array names
            - supports_out: Whether out= parameter is supported

    Example:
        >>> import liq_ta
        >>> # List all indicators
        >>> for ind in liq_ta.list_indicators():
        ...     print(f"{ind['function']}: {ind['name']}")
        ...
        >>> # List only momentum indicators
        >>> momentum = liq_ta.list_indicators(category='momentum')
    """
    result = []
    combined = _combined_indicator_metadata()
    normalized_category = None

    if category is not None:
        if not isinstance(category, str):
            raise IndicatorArgumentError(
                f"category must be a string, got {type(category).__name__}"
            )
        normalized_category = category.strip().lower()
        categories = _known_categories(combined)
        if normalized_category not in categories:
            raise IndicatorArgumentError(
                f"unknown category '{category}'. valid categories: {', '.join(categories)}"
            )

    for func_name, meta in combined.items():
        if normalized_category is None or meta["category"] == normalized_category:
            result.append({"function": func_name, **meta})
    return result


def get_indicator_info(name):
    """Get metadata for a specific indicator.

    Args:
        name: Indicator function name (e.g., 'sma', 'rsi')

    Returns:
        Dict with indicator metadata, or None if not found.

    Example:
        >>> import liq_ta
        >>> info = liq_ta.get_indicator_info('macd')
        >>> print(info['outputs'])  # ['macd_line', 'signal_line', 'histogram']
    """
    if not isinstance(name, str):
        return None
    normalized = name.strip()
    if not normalized:
        return None

    combined = _combined_indicator_metadata()
    if normalized in combined:
        return {"function": normalized, **combined[normalized]}
    return None


def require_indicator_info(name):
    """Get indicator metadata or raise a deterministic user-facing error."""
    normalized = _normalize_indicator_name(name)
    info = get_indicator_info(normalized)
    if info is not None:
        return info

    available = ", ".join(sorted(_combined_indicator_metadata().keys()))
    raise IndicatorNotFoundError(
        f"unknown indicator '{normalized}'. available indicators: {available}"
    )


def validate_indicator_metadata(raise_on_error=True):
    """Validate merged indicator metadata registry integrity.

    Args:
        raise_on_error: If True, raise `IndicatorMetadataError` on first failure set.

    Returns:
        A list of validation error messages. Empty list means metadata is valid.
    """
    combined = _combined_indicator_metadata()
    diagnostics = []

    runtime_names = set(_RUNTIME_INDICATOR_METADATA.keys())

    for indicator_name, meta in combined.items():
        for required_key in (
            "category",
            "inputs",
            "params",
            "outputs",
            "supports_out",
        ):
            if required_key not in meta:
                diagnostics.append(
                    f"indicator '{indicator_name}' missing required key '{required_key}'"
                )

        inputs = meta.get("inputs")
        params = meta.get("params")
        outputs = meta.get("outputs")
        input_shape = meta.get("input_shape")
        supports_out = meta.get("supports_out")

        if indicator_name in runtime_names:
            if "input_shape" not in meta:
                diagnostics.append(
                    f"indicator '{indicator_name}' missing required key 'input_shape'"
                )
            elif not isinstance(input_shape, str) or not input_shape:
                diagnostics.append(
                    f"indicator '{indicator_name}' has invalid input_shape metadata"
                )

        if not isinstance(inputs, list):
            diagnostics.append(f"indicator '{indicator_name}' has non-list inputs metadata")
        if not isinstance(params, list):
            diagnostics.append(f"indicator '{indicator_name}' has non-list params metadata")
        if not isinstance(outputs, list) or len(outputs) == 0:
            diagnostics.append(
                f"indicator '{indicator_name}' has invalid outputs metadata (must be non-empty list)"
            )
        if not isinstance(supports_out, bool):
            diagnostics.append(
                f"indicator '{indicator_name}' has non-boolean supports_out metadata"
            )

        function_target = globals().get(indicator_name)
        if not callable(function_target):
            diagnostics.append(
                f"indicator '{indicator_name}' metadata exists but callable is not exported"
            )

    if diagnostics and raise_on_error:
        raise IndicatorMetadataError("; ".join(diagnostics))
    return diagnostics


def compute_indicator(name, *args, debug=False, **kwargs):
    """Compute an indicator dynamically by name.

    This helper provides a single indicator-selection surface that maps:
    - unknown indicator selection -> `IndicatorNotFoundError`
    - malformed arg counts/shapes -> `IndicatorArgumentError`
    """
    info = require_indicator_info(name)
    function_name = info["function"]
    indicator_fn = globals().get(function_name)

    if not callable(indicator_fn):
        raise IndicatorMetadataError(
            f"indicator '{function_name}' is registered but not callable"
        )

    try:
        return indicator_fn(*args, **kwargs)
    except TypeError as exc:
        message = f"invalid argument shape for indicator '{function_name}': {exc}"
        if debug:
            message += (
                f" | expected_inputs={info.get('inputs', [])}"
                f" expected_params={info.get('params', [])}"
            )
        raise IndicatorArgumentError(message) from exc


__all__ = [
    # Moving Averages
    "sma",
    "ema",
    "ema_wilder",
    "wma",
    "dema",
    "tema",
    "trima",
    "midpoint",
    "midprice",
    "kama",
    "t3",
    "sar",
    "sarext",
    "ht_trendline",
    "mama",
    "mavp",
    "hma",
    "gaussian_filter",
    "gaussian_channel",
    # Momentum
    "ao",
    "bulls_power",
    "bears_power",
    "demarker",
    "osma",
    "vortex",
    "rvi",
    "dpo",
    "connors_rsi",
    "stc",
    "laguerre_rsi",
    "dss_bressert",
    "supertrend",
    "mom",
    "roc",
    "rocp",
    "rocr",
    "rocr100",
    "apo",
    "ppo",
    "bop",
    "aroon",
    "aroonosc",
    "cci",
    "cmo",
    "mfi",
    "stochrsi",
    "trix",
    "ultosc",
    "rsi",
    "macd",
    "qqe",
    "stochastic",
    "stochastic_fast",
    "stochastic_slow",
    "williams_r",
    "adx",
    "ichimoku",
    "adxr",
    "dx",
    "plus_dm",
    "minus_dm",
    # Volatility
    "atr",
    "true_range",
    "bollinger",
    "donchian",
    "keltner_channel",
    "rolling_stddev",
    "chop",
    "ulcer_index",
    "hurst",
    "autocorr",
    "hma_atr_bands",
    "hma_bollinger_bands",
    "vwap_atr_bands",
    "vwap_bollinger_bands",
    # Volume
    "obv",
    "vwap",
    "ad",
    "adosc",
    # Cycle (Hilbert Transform)
    "ht_dcperiod",
    "ht_dcphase",
    "ht_phasor",
    "ht_sine",
    "ht_trendmode",
    # Price Transform
    "avgprice",
    "medprice",
    "typprice",
    "wclprice",
    # Statistics
    "var",
    "correl",
    "beta",
    "linearreg",
    "linearreg_slope",
    "linearreg_intercept",
    "linearreg_angle",
    "tsf",
    # Candlestick Patterns - Single-candle
    "cdl_doji",
    "cdl_dragonfly_doji",
    "cdl_gravestone_doji",
    "cdl_longleg_doji",
    "cdl_rickshaw_man",
    "cdl_marubozu",
    "cdl_closing_marubozu",
    "cdl_spinning_top",
    "cdl_high_wave",
    "cdl_long_line",
    "cdl_short_line",
    "cdl_hammer",
    "cdl_hanging_man",
    "cdl_inverted_hammer",
    "cdl_shooting_star",
    "cdl_takuri",
    "cdl_belt_hold",
    # Candlestick Patterns - Two-candle
    "cdl_engulfing",
    "cdl_harami",
    "cdl_harami_cross",
    "cdl_piercing",
    "cdl_dark_cloud_cover",
    "cdl_doji_star",
    "cdl_kicking",
    "cdl_kicking_by_length",
    "cdl_matching_low",
    "cdl_homing_pigeon",
    "cdl_in_neck",
    "cdl_on_neck",
    "cdl_thrusting",
    "cdl_separating_lines",
    "cdl_counter_attack",
    "cdl_2crows",
    "cdl_hikkake",
    "cdl_hikkake_mod",
    # Candlestick Patterns - Three-candle and complex
    "cdl_morning_star",
    "cdl_evening_star",
    "cdl_morning_doji_star",
    "cdl_evening_doji_star",
    "cdl_abandoned_baby",
    "cdl_3white_soldiers",
    "cdl_3black_crows",
    "cdl_3inside",
    "cdl_3outside",
    "cdl_3line_strike",
    "cdl_3stars_in_south",
    "cdl_tristar",
    "cdl_identical_3crows",
    "cdl_stick_sandwich",
    "cdl_unique_3river",
    "cdl_advance_block",
    "cdl_stalled_pattern",
    "cdl_tasuki_gap",
    "cdl_upside_gap_2crows",
    "cdl_gap_side_side_white",
    "cdl_breakaway",
    "cdl_ladder_bottom",
    "cdl_mat_hold",
    "cdl_rise_fall_3methods",
    "cdl_concealing_baby_swallow",
    "cdl_xside_gap_3methods",
    # Metadata
    "LiqTaError",
    "IndicatorNotFoundError",
    "IndicatorArgumentError",
    "IndicatorMetadataError",
    "INDICATORS",
    "list_indicators",
    "get_indicator_info",
    "require_indicator_info",
    "validate_indicator_metadata",
    "compute_indicator",
    "get_indicator_registry",
]
