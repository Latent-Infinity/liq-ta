//! Python bindings for liq-ta technical analysis library.
//!
//! This module provides Python bindings via `PyO3` for all Tier 0 indicators
//! in the liq-ta library, with `NumPy` array support for zero-copy data transfer.
//!
//! All functions support an optional `out` parameter following `NumPy` conventions:
//! - If `out` is provided, results are written directly (zero-copy)
//! - If `out` is None, a new array is allocated and returned

// Pedantic lint suppressions for Python bindings
// These are intentional patterns for PyO3 FFI code
#![allow(clippy::too_many_arguments)] // Python functions mirror indicator signatures
#![allow(clippy::too_many_lines)] // Complex binding logic requires more lines
#![allow(clippy::type_complexity)] // PyO3 types are complex by nature
#![allow(clippy::needless_pass_by_value)] // PyO3 requires pass-by-value for Python interop

use numpy::{PyArray1, PyArrayMethods, PyReadonlyArray1};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Convert liq-ta error to Python `ValueError`
fn to_py_err(e: liq_ta::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

// =============================================================================
// Moving Averages
// =============================================================================

/// Simple Moving Average (SMA)
///
/// Calculates the arithmetic mean of prices over a rolling window.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The number of periods for the moving average
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with SMA values (first period-1 values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn sma<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        // SAFETY: We have exclusive access during this function call
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::sma_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::sma(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Exponential Moving Average (EMA)
///
/// Calculates an exponentially weighted average that gives more weight to recent prices.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The number of periods for calculating the smoothing factor
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with EMA values (first period-1 values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn ema<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::ema_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::ema(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Wilder's Exponential Moving Average
///
/// Uses smoothing factor α = 1/period (Wilder's method).
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The number of periods
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with Wilder's EMA values
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn ema_wilder<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::ema_wilder_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::ema_wilder(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Weighted Moving Average (WMA)
///
/// Calculates a linearly weighted average that gives more weight to recent prices.
/// The most recent price has weight n, second most recent has weight n-1, etc.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The number of periods for the moving average
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with WMA values (first period-1 values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn wma<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::wma_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::wma(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Double Exponential Moving Average (DEMA)
///
/// Reduces lag by applying a combination of single and double-smoothed EMAs.
/// Formula: DEMA = 2 * EMA(data) - EMA(EMA(data))
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The number of periods for EMA calculations
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with DEMA values (first 2*(period-1) values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn dema<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::dema_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::dema(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Triple Exponential Moving Average (TEMA)
///
/// Further reduces lag compared to DEMA by applying triple smoothing.
/// Formula: TEMA = 3 * EMA1 - 3 * EMA2 + EMA3
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The number of periods for EMA calculations
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with TEMA values (first 3*(period-1) values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn tema<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::tema_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::tema(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Triangular Moving Average (TRIMA)
///
/// Double-smoothed moving average that gives more weight to the middle of the data range.
/// Computed as SMA(SMA(data, period1), period2).
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The TRIMA period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with TRIMA values (first period-1 values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn trima<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::trima_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::trima(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// MIDPOINT - Midpoint over period
///
/// Calculates the midpoint of the price range: (highest + lowest) / 2.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The MIDPOINT period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with MIDPOINT values (first period-1 values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn midpoint<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::midpoint_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::midpoint(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// KAMA (Kaufman Adaptive Moving Average)
///
/// Adaptive moving average that adjusts smoothing based on market efficiency.
/// Responds quickly in trending markets and slowly in sideways markets.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The efficiency ratio period (typically 10)
///     `fast_period`: Fast EMA period (default 2)
///     `slow_period`: Slow EMA period (default 30)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with KAMA values (first period-1 values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, fast_period=2, slow_period=30, out=None))]
fn kama<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    fast_period: usize,
    slow_period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::kama_full_into(input, period, fast_period, slow_period, slice)
            .map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::kama_full(input, period, fast_period, slow_period)
            .map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// MIDPRICE - Midpoint price over period
///
/// Calculates the midpoint of the high-low price range: (`highest_high` + `lowest_low`) / 2.
///
/// Args:
///     high: High price array (`NumPy` array of f64)
///     low: Low price array (`NumPy` array of f64)
///     period: The MIDPRICE period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with MIDPRICE values (first period-1 values are NaN)
#[pyfunction]
#[pyo3(signature = (high, low, period, out=None))]
fn midprice<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high_slice = high.as_slice()?;
    let low_slice = low.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::midprice_into(high_slice, low_slice, period, slice)
            .map_err(to_py_err)?;
        Ok(output)
    } else {
        let result =
            liq_ta::indicators::midprice(high_slice, low_slice, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// T3 (Tillson T3 Moving Average)
///
/// Smoother moving average using six EMAs with a volume factor to reduce
/// lag while maintaining smoothness.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The EMA period
///     vfactor: Volume factor (typically 0.7)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with T3 values (first 6*(period-1) values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, vfactor=0.7, out=None))]
fn t3<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    vfactor: f64,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::t3_full_into(input, period, vfactor, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::t3_full(input, period, vfactor).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// SAR (Parabolic Stop and Reverse)
///
/// Trend-following indicator that provides potential entry and exit points.
/// SAR appears as dots above or below price depending on trend direction.
///
/// Args:
///     high: High price array (`NumPy` array of f64)
///     low: Low price array (`NumPy` array of f64)
///     `af_start`: Initial acceleration factor (default 0.02)
///     `af_step`: Acceleration factor increment (default 0.02)
///     `af_max`: Maximum acceleration factor (default 0.20)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with SAR values (first value is NaN)
#[pyfunction]
#[pyo3(signature = (high, low, af_start=0.02, af_step=0.02, af_max=0.20, out=None))]
fn sar<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    af_start: f64,
    af_step: f64,
    af_max: f64,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high_slice = high.as_slice()?;
    let low_slice = low.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::sar_full_into(high_slice, low_slice, af_start, af_step, af_max, slice)
            .map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::sar_full(high_slice, low_slice, af_start, af_step, af_max)
            .map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// SAREXT (Extended Parabolic SAR)
///
/// Extended version of Parabolic SAR with separate parameters for long/short.
/// Returns positive values for long positions, negative for short positions.
///
/// Args:
///     high: High price array (`NumPy` array of f64)
///     low: Low price array (`NumPy` array of f64)
///     `start_value`: Initial SAR value (0.0 = auto-detect)
///     `offset_on_reverse`: Offset on trend reversal
///     `af_init_long`: Initial AF for long positions
///     `af_long`: AF increment for long positions
///     `af_max_long`: Maximum AF for long positions
///     `af_init_short`: Initial AF for short positions
///     `af_short`: AF increment for short positions
///     `af_max_short`: Maximum AF for short positions
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with SAREXT values (positive=long, negative=short)
#[pyfunction]
#[pyo3(signature = (high, low, start_value=0.0, offset_on_reverse=0.0, af_init_long=0.02, af_long=0.02, af_max_long=0.20, af_init_short=0.02, af_short=0.02, af_max_short=0.20, out=None))]
fn sarext<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    start_value: f64,
    offset_on_reverse: f64,
    af_init_long: f64,
    af_long: f64,
    af_max_long: f64,
    af_init_short: f64,
    af_short: f64,
    af_max_short: f64,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high_slice = high.as_slice()?;
    let low_slice = low.as_slice()?;

    let params = liq_ta::indicators::SarExtParams {
        start_value,
        offset_on_reverse,
        af_init_long,
        af_long,
        af_max_long,
        af_init_short,
        af_short,
        af_max_short,
    };

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::sarext_full_into(high_slice, low_slice, params, slice)
            .map_err(to_py_err)?;
        Ok(output)
    } else {
        let result =
            liq_ta::indicators::sarext_full(high_slice, low_slice, params).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// `HT_TRENDLINE` (Hilbert Transform - Instantaneous Trendline)
///
/// Uses Hilbert Transform signal processing to compute an adaptive trendline
/// based on the dominant cycle period in the data.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with trendline values (first 63 values are NaN)
#[pyfunction]
#[pyo3(signature = (data, out=None))]
fn ht_trendline<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::ht_trendline_into(input, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::ht_trendline(input).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// MAMA (MESA Adaptive Moving Average)
///
/// Adaptive moving average using Hilbert Transform to measure rate of phase change.
/// Returns both MAMA and FAMA (Following Adaptive Moving Average).
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     `fast_limit`: Maximum alpha (default 0.5)
///     `slow_limit`: Minimum alpha (default 0.05)
///
/// Returns:
///     Tuple of (mama, fama) `NumPy` arrays (first 32 values are NaN)
#[pyfunction]
#[pyo3(signature = (data, fast_limit=0.5, slow_limit=0.05))]
fn mama<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    fast_limit: f64,
    slow_limit: f64,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let input = data.as_slice()?;
    let result = liq_ta::indicators::mama_full(input, fast_limit, slow_limit).map_err(to_py_err)?;
    Ok((
        PyArray1::from_vec(py, result.mama),
        PyArray1::from_vec(py, result.fama),
    ))
}

/// MAVP (Moving Average Variable Period)
///
/// Computes a simple moving average where the period can vary at each data point.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     periods: Period to use at each data point (`NumPy` array of f64)
///     `min_period`: Minimum allowed period (default: 2)
///     `max_period`: Maximum allowed period (default: 30)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with MAVP values
#[pyfunction]
#[pyo3(signature = (data, periods, min_period=2, max_period=30, out=None))]
fn mavp<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    periods: PyReadonlyArray1<'py, f64>,
    min_period: usize,
    max_period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;
    let periods_slice = periods.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::mavp_into(input, periods_slice, min_period, max_period, slice)
            .map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::mavp(input, periods_slice, min_period, max_period)
            .map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

// =============================================================================
// Momentum Indicators
// =============================================================================

/// Relative Strength Index (RSI)
///
/// Momentum oscillator measuring speed and magnitude of price changes.
/// Values range from 0 to 100.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The number of periods (typically 14)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with RSI values (0-100 range)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn rsi<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::rsi_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::rsi(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Moving Average Convergence Divergence (MACD)
///
/// Trend-following momentum indicator showing the relationship between two EMAs.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     `fast_period`: Fast EMA period (default: 12)
///     `slow_period`: Slow EMA period (default: 26)
///     `signal_period`: Signal line period (default: 9)
///
/// Returns:
///     Tuple of (`macd_line`, `signal_line`, histogram) `NumPy` arrays
#[pyfunction]
#[pyo3(signature = (data, fast_period=12, slow_period=26, signal_period=9))]
fn macd<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> PyResult<(
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
)> {
    let input = data.as_slice()?;
    let config = liq_ta::indicators::Macd::new()
        .with_fast_period(fast_period)
        .with_slow_period(slow_period)
        .with_signal_period(signal_period);
    let result = config.compute(input).map_err(to_py_err)?;
    Ok((
        PyArray1::from_vec(py, result.macd_line),
        PyArray1::from_vec(py, result.signal_line),
        PyArray1::from_vec(py, result.histogram),
    ))
}

/// Stochastic Oscillator
///
/// Momentum indicator comparing closing price to price range over a period.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     `k_period`: %K lookback period (default: 14)
///     `d_period`: %D smoothing period (default: 3)
///     `k_slowing`: %K smoothing (1=fast, 3=slow, default: 1)
///
/// Returns:
///     Tuple of (%K, %D) `NumPy` arrays (values 0-100)
#[pyfunction]
#[pyo3(signature = (high, low, close, k_period=14, d_period=3, k_slowing=1))]
fn stochastic<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    k_period: usize,
    d_period: usize,
    k_slowing: usize,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;
    let result = liq_ta::indicators::stochastic(h, l, c, k_period, d_period, k_slowing)
        .map_err(to_py_err)?;
    Ok((
        PyArray1::from_vec(py, result.k),
        PyArray1::from_vec(py, result.d),
    ))
}

/// Fast Stochastic Oscillator
///
/// Stochastic with `k_slowing=1` (no smoothing on %K).
#[pyfunction]
#[pyo3(signature = (high, low, close, k_period=14, d_period=3))]
fn stochastic_fast<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    k_period: usize,
    d_period: usize,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;
    let result =
        liq_ta::indicators::stochastic_fast(h, l, c, k_period, d_period).map_err(to_py_err)?;
    Ok((
        PyArray1::from_vec(py, result.k),
        PyArray1::from_vec(py, result.d),
    ))
}

/// Slow Stochastic Oscillator
///
/// Stochastic with `k_slowing=3` (traditional slow stochastic).
#[pyfunction]
#[pyo3(signature = (high, low, close, k_period=14, d_period=3))]
fn stochastic_slow<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    k_period: usize,
    d_period: usize,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;
    let result =
        liq_ta::indicators::stochastic_slow(h, l, c, k_period, d_period).map_err(to_py_err)?;
    Ok((
        PyArray1::from_vec(py, result.k),
        PyArray1::from_vec(py, result.d),
    ))
}

/// Average Directional Index (ADX)
///
/// Measures trend strength regardless of direction. Values 0-100.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     period: The number of periods (typically 14)
///
/// Returns:
///     Tuple of (adx, `plus_di`, `minus_di`) `NumPy` arrays
#[pyfunction]
fn adx<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    period: usize,
) -> PyResult<(
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
)> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;
    let result = liq_ta::indicators::adx(h, l, c, period).map_err(to_py_err)?;
    Ok((
        PyArray1::from_vec(py, result.adx),
        PyArray1::from_vec(py, result.plus_di),
        PyArray1::from_vec(py, result.minus_di),
    ))
}

/// Williams %R
///
/// Momentum oscillator similar to Stochastic but inverted scale (-100 to 0).
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     period: The lookback period (typically 14)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with Williams %R values (-100 to 0)
#[pyfunction]
#[pyo3(signature = (high, low, close, period, out=None))]
fn williams_r<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::williams_r_into(h, l, c, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::williams_r(h, l, c, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Momentum (MOM)
///
/// Measures the rate of change in price by comparing current price to price N periods ago.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: Lookback period (number of bars to look back)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with momentum values (first `period` values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn mom<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::mom_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::mom(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Rate of Change (ROC)
///
/// Measures the percentage change in price over a given period.
/// ROC = ((price - price[n]) / price[n]) * 100
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with ROC values (percentage, first `period` values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn roc<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::roc_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::roc(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Rate of Change Percentage (ROCP)
///
/// Measures the decimal percentage change in price over a given period.
/// ROCP = (price - price[n]) / price[n]
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with ROCP values (decimal, first `period` values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn rocp<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::rocp_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::rocp(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Rate of Change Ratio (ROCR)
///
/// Measures the ratio of current price to price N periods ago.
/// ROCR = price / price[n]
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with ROCR values (ratio, first `period` values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn rocr<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::rocr_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::rocr(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Rate of Change Ratio 100 (ROCR100)
///
/// Measures the ratio of current price to price N periods ago, scaled by 100.
/// ROCR100 = (price / price[n]) * 100
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with ROCR100 values (first `period` values are NaN)
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn rocr100<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::rocr100_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::rocr100(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Absolute Price Oscillator (APO)
///
/// Measures the difference between fast and slow EMAs.
/// APO = EMA(price, `fast_period`) - EMA(price, `slow_period`)
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     `fast_period`: Fast EMA period (default: 12)
///     `slow_period`: Slow EMA period (default: 26)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with APO values (first `slow_period-1` values are NaN)
#[pyfunction]
#[pyo3(signature = (data, fast_period=12, slow_period=26, out=None))]
fn apo<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    fast_period: usize,
    slow_period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::apo_into(input, fast_period, slow_period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::apo(input, fast_period, slow_period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Percentage Price Oscillator (PPO)
///
/// Measures the percentage difference between fast and slow EMAs.
/// PPO = ((EMA(price, `fast_period`) - EMA(price, `slow_period`)) / EMA(price, `slow_period`)) * 100
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     `fast_period`: Fast EMA period (default: 12)
///     `slow_period`: Slow EMA period (default: 26)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with PPO values (percentage, first `slow_period-1` values are NaN)
#[pyfunction]
#[pyo3(signature = (data, fast_period=12, slow_period=26, out=None))]
fn ppo<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    fast_period: usize,
    slow_period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::ppo_into(input, fast_period, slow_period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::ppo(input, fast_period, slow_period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Balance of Power (BOP)
///
/// Measures the strength of buyers vs sellers by comparing the closing price
/// position within the day's range.
/// BOP = (close - open) / (high - low)
///
/// Args:
///     open: Open prices (`NumPy` array of f64)
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with BOP values (range -1 to +1)
#[pyfunction]
#[pyo3(signature = (open, high, low, close, out=None))]
fn bop<'py>(
    py: Python<'py>,
    open: PyReadonlyArray1<'py, f64>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let o = open.as_slice()?;
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::bop_into(o, h, l, c, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::bop(o, h, l, c).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Aroon Indicator
///
/// Measures the time since a high or low occurred within a specified period.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     period: Lookback period (typically 25)
///
/// Returns:
///     Tuple of (`aroon_up`, `aroon_down`) arrays with values 0-100
#[pyfunction]
#[pyo3(signature = (high, low, period=25))]
fn aroon<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    period: usize,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;

    let result = liq_ta::indicators::aroon(h, l, period).map_err(to_py_err)?;
    Ok((
        PyArray1::from_vec(py, result.aroon_up),
        PyArray1::from_vec(py, result.aroon_down),
    ))
}

/// Aroon Oscillator (AROONOSC)
///
/// Measures the difference between Aroon Up and Aroon Down.
/// AROONOSC = Aroon Up - Aroon Down
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     period: Lookback period (typically 25)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with AROONOSC values (range -100 to +100)
#[pyfunction]
#[pyo3(signature = (high, low, period=25, out=None))]
fn aroonosc<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::aroonosc_into(h, l, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::aroonosc(h, l, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Commodity Channel Index (CCI)
///
/// Measures the deviation of an asset's price from its statistical mean.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     period: Lookback period (typically 20)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with CCI values
#[pyfunction]
#[pyo3(signature = (high, low, close, period=20, out=None))]
fn cci<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::cci_into(h, l, c, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::cci(h, l, c, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Chande Momentum Oscillator (CMO)
///
/// Measures momentum by comparing gains to losses over a period.
///
/// Args:
///     data: Price data (`NumPy` array of f64)
///     period: Lookback period (typically 14)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with CMO values (range -100 to +100)
#[pyfunction]
#[pyo3(signature = (data, period=14, out=None))]
fn cmo<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::cmo_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::cmo(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Money Flow Index (MFI)
///
/// Volume-weighted RSI that measures buying and selling pressure.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     volume: Volume data (`NumPy` array of f64)
///     period: Lookback period (typically 14)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with MFI values (range 0 to 100)
#[pyfunction]
#[pyo3(signature = (high, low, close, volume, period=14, out=None))]
fn mfi<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    volume: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;
    let v = volume.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::mfi_into(h, l, c, v, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::mfi(h, l, c, v, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Stochastic RSI (STOCHRSI)
///
/// Applies Stochastic oscillator formula to RSI values.
///
/// Args:
///     data: Price data (`NumPy` array of f64)
///     `rsi_period`: Period for RSI calculation (default 14)
///     `stoch_period`: Period for Stochastic calculation (default 14)
///     `k_period`: Smoothing period for K (default 1)
///     `d_period`: Period for D line (default 3)
///
/// Returns:
///     Tuple of (`FastK`, `FastD`) `NumPy` arrays (range 0 to 1)
#[pyfunction]
#[pyo3(signature = (data, rsi_period=14, stoch_period=14, k_period=1, d_period=3))]
fn stochrsi<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    rsi_period: usize,
    stoch_period: usize,
    k_period: usize,
    d_period: usize,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let input = data.as_slice()?;

    let result = liq_ta::indicators::stochrsi(input, rsi_period, stoch_period, k_period, d_period)
        .map_err(to_py_err)?;

    Ok((
        PyArray1::from_vec(py, result.fastk),
        PyArray1::from_vec(py, result.fastd),
    ))
}

/// TRIX (Triple Exponential Average)
///
/// Shows the percent rate of change of a triple exponentially smoothed EMA.
///
/// Args:
///     data: Price data (`NumPy` array of f64)
///     period: EMA period (default 15)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with TRIX values (percentage)
#[pyfunction]
#[pyo3(signature = (data, period=15, out=None))]
fn trix<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::trix_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::trix(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Ultimate Oscillator (ULTOSC)
///
/// Measures momentum across three different timeframes.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     period1: First (short) period (default 7)
///     period2: Second (medium) period (default 14)
///     period3: Third (long) period (default 28)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with ULTOSC values (range 0 to 100)
#[pyfunction]
#[pyo3(signature = (high, low, close, period1=7, period2=14, period3=28, out=None))]
fn ultosc<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    period1: usize,
    period2: usize,
    period3: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::ultosc_into(h, l, c, period1, period2, period3, slice)
            .map_err(to_py_err)?;
        Ok(output)
    } else {
        let result =
            liq_ta::indicators::ultosc(h, l, c, period1, period2, period3).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// ADX Rating (ADXR)
///
/// Smoothed ADX - average of current ADX and ADX from period bars ago.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     period: ADX period (default 14)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with ADXR values (range 0 to 100)
#[pyfunction]
#[pyo3(signature = (high, low, close, period=14, out=None))]
fn adxr<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::adxr_into(h, l, c, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::adxr(h, l, c, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Directional Movement Index (DX)
///
/// Measures the difference between +DI and -DI relative to their sum.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     period: Period for calculation (default 14)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with DX values (range 0 to 100)
#[pyfunction]
#[pyo3(signature = (high, low, close, period=14, out=None))]
fn dx<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::dx_into(h, l, c, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::dx(h, l, c, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Plus Directional Movement (`PLUS_DM`)
///
/// Smoothed positive directional movement.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     period: Period for smoothing (default 14)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with Plus DM values
#[pyfunction]
#[pyo3(signature = (high, low, period=14, out=None))]
fn plus_dm<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::plus_dm_into(h, l, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::plus_dm(h, l, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Minus Directional Movement (`MINUS_DM`)
///
/// Smoothed negative directional movement.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     period: Period for smoothing (default 14)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with Minus DM values
#[pyfunction]
#[pyo3(signature = (high, low, period=14, out=None))]
fn minus_dm<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::minus_dm_into(h, l, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::minus_dm(h, l, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

// =============================================================================
// Volatility Indicators
// =============================================================================

/// Average True Range (ATR)
///
/// Measures market volatility using the true range of price movements.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     period: The number of periods (typically 14)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with ATR values
#[pyfunction]
#[pyo3(signature = (high, low, close, period, out=None))]
fn atr<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::atr_into(h, l, c, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::atr(h, l, c, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// True Range
///
/// Single-period volatility measurement (max of high-low, high-prev_close, prev_close-low).
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with True Range values
#[pyfunction]
#[pyo3(signature = (high, low, close, out=None))]
fn true_range<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::true_range_into(h, l, c, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::true_range(h, l, c).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Bollinger Bands
///
/// Price envelope based on standard deviation around a moving average.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The number of periods (default: 20)
///     `std_dev`: Standard deviation multiplier (default: 2.0)
///
/// Returns:
///     Tuple of (`upper_band`, `middle_band`, `lower_band`) `NumPy` arrays
#[pyfunction]
#[pyo3(signature = (data, period=20, std_dev=2.0))]
fn bollinger<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    std_dev: f64,
) -> PyResult<(
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
)> {
    let input = data.as_slice()?;
    let config = liq_ta::indicators::Bollinger::new()
        .with_period(period)
        .with_std_dev(std_dev);
    let result = config.compute(input).map_err(to_py_err)?;
    Ok((
        PyArray1::from_vec(py, result.upper),
        PyArray1::from_vec(py, result.middle),
        PyArray1::from_vec(py, result.lower),
    ))
}

/// Donchian Channels
///
/// Price channels showing highest high and lowest low over a period.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     period: The lookback period (default: 20)
///
/// Returns:
///     Tuple of (upper, middle, lower) `NumPy` arrays
#[pyfunction]
#[pyo3(signature = (high, low, period=20))]
fn donchian<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    period: usize,
) -> PyResult<(
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
)> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let result = liq_ta::indicators::donchian(h, l, period).map_err(to_py_err)?;
    Ok((
        PyArray1::from_vec(py, result.upper),
        PyArray1::from_vec(py, result.middle),
        PyArray1::from_vec(py, result.lower),
    ))
}

/// Rolling Standard Deviation
///
/// Calculates the standard deviation over a rolling window.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     period: The window size
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with rolling standard deviation values
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn rolling_stddev<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::rolling_stddev_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::rolling_stddev(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

// =============================================================================
// Volume Indicators
// =============================================================================

/// On-Balance Volume (OBV)
///
/// Cumulative volume indicator that relates volume to price changes.
///
/// Args:
///     close: Close prices (`NumPy` array of f64)
///     volume: Volume values (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with OBV values
#[pyfunction]
#[pyo3(signature = (close, volume, out=None))]
fn obv<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    volume: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let c = close.as_slice()?;
    let v = volume.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::obv_into(c, v, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::obv(c, v).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Volume Weighted Average Price (VWAP)
///
/// Average price weighted by volume, commonly used as a trading benchmark.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     volume: Volume values (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with VWAP values
#[pyfunction]
#[pyo3(signature = (high, low, close, volume, out=None))]
fn vwap<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    volume: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;
    let v = volume.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::vwap_into(h, l, c, v, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::vwap(h, l, c, v).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Chaikin Accumulation/Distribution Line (AD)
///
/// Cumulative volume-based indicator that measures the flow of money into/out of a security.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     volume: Volume data (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with AD values
#[pyfunction]
#[pyo3(signature = (high, low, close, volume, out=None))]
fn ad<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    volume: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;
    let v = volume.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::ad_into(h, l, c, v, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::ad(h, l, c, v).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Chaikin A/D Oscillator (ADOSC)
///
/// Difference between fast and slow EMA of the Accumulation/Distribution Line.
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     volume: Volume data (`NumPy` array of f64)
///     `fast_period`: Fast EMA period (typically 3)
///     `slow_period`: Slow EMA period (typically 10)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with ADOSC values
#[pyfunction]
#[pyo3(signature = (high, low, close, volume, fast_period=3, slow_period=10, out=None))]
fn adosc<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    volume: PyReadonlyArray1<'py, f64>,
    fast_period: usize,
    slow_period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;
    let v = volume.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::adosc_into(h, l, c, v, fast_period, slow_period, slice)
            .map_err(to_py_err)?;
        Ok(output)
    } else {
        let result =
            liq_ta::indicators::adosc(h, l, c, v, fast_period, slow_period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

// =============================================================================
// Cycle Indicators (Hilbert Transform)
// =============================================================================

/// Hilbert Transform - Dominant Cycle Period (`HT_DCPERIOD`)
///
/// Measures the dominant cycle period in the price data.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with dominant cycle period values
#[pyfunction]
#[pyo3(signature = (data, out=None))]
fn ht_dcperiod<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::ht_dcperiod_into(input, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::ht_dcperiod(input).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Hilbert Transform - Dominant Cycle Phase (`HT_DCPHASE`)
///
/// Measures the instantaneous phase of the dominant cycle.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with phase values in degrees (0-360)
#[pyfunction]
#[pyo3(signature = (data, out=None))]
fn ht_dcphase<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::ht_dcphase_into(input, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::ht_dcphase(input).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Hilbert Transform - Phasor Components (`HT_PHASOR`)
///
/// Computes the in-phase and quadrature components.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///
/// Returns:
///     Tuple of (inphase, quadrature) `NumPy` arrays
#[pyfunction]
#[pyo3(signature = (data,))]
fn ht_phasor<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let input = data.as_slice()?;
    let result = liq_ta::indicators::ht_phasor(input).map_err(to_py_err)?;

    Ok((
        PyArray1::from_vec(py, result.inphase),
        PyArray1::from_vec(py, result.quadrature),
    ))
}

/// Hilbert Transform - `SineWave` (`HT_SINE`)
///
/// Computes sine wave based on the dominant cycle phase.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///
/// Returns:
///     Tuple of (sine, `lead_sine`) `NumPy` arrays
#[pyfunction]
#[pyo3(signature = (data,))]
fn ht_sine<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let input = data.as_slice()?;
    let result = liq_ta::indicators::ht_sine(input).map_err(to_py_err)?;

    Ok((
        PyArray1::from_vec(py, result.sine),
        PyArray1::from_vec(py, result.lead_sine),
    ))
}

/// Hilbert Transform - Trend vs Cycle Mode (`HT_TRENDMODE`)
///
/// Determines whether the market is in trend or cycle mode.
///
/// Args:
///     data: Input price array (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with mode values (1 for trend, 0 for cycle)
#[pyfunction]
#[pyo3(signature = (data, out=None))]
fn ht_trendmode<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::ht_trendmode_into(input, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::ht_trendmode(input).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

// =============================================================================
// Statistical Functions
// =============================================================================

/// VAR (Variance)
///
/// Calculates rolling population variance.
///
/// Args:
///     data: Input data array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with variance values
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn var<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::var_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::var(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// CORREL (Pearson Correlation Coefficient)
///
/// Calculates rolling Pearson correlation between two series.
///
/// Args:
///     data0: First data array (`NumPy` array of f64)
///     data1: Second data array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with correlation values (-1 to 1)
#[pyfunction]
#[pyo3(signature = (data0, data1, period, out=None))]
fn correl<'py>(
    py: Python<'py>,
    data0: PyReadonlyArray1<'py, f64>,
    data1: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let x = data0.as_slice()?;
    let y = data1.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::correl_into(x, y, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::correl(x, y, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// BETA
///
/// Calculates rolling beta coefficient (covariance / variance of market).
///
/// Args:
///     data0: Asset data array (`NumPy` array of f64)
///     data1: Market/benchmark data array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with beta values
#[pyfunction]
#[pyo3(signature = (data0, data1, period, out=None))]
fn beta<'py>(
    py: Python<'py>,
    data0: PyReadonlyArray1<'py, f64>,
    data1: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let asset = data0.as_slice()?;
    let market = data1.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::beta_into(asset, market, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::beta(asset, market, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// LINEARREG (Linear Regression)
///
/// Calculates rolling linear regression predicted value at end of period.
///
/// Args:
///     data: Input data array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with regression values
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn linearreg<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::linearreg_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::linearreg(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// `LINEARREG_SLOPE` (Linear Regression Slope)
///
/// Calculates rolling linear regression slope.
///
/// Args:
///     data: Input data array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with slope values
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn linearreg_slope<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::linearreg_slope_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::linearreg_slope(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// `LINEARREG_INTERCEPT` (Linear Regression Intercept)
///
/// Calculates rolling linear regression y-intercept.
///
/// Args:
///     data: Input data array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with intercept values
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn linearreg_intercept<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::linearreg_intercept_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::linearreg_intercept(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// `LINEARREG_ANGLE` (Linear Regression Angle)
///
/// Calculates rolling linear regression angle in degrees.
///
/// Args:
///     data: Input data array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with angle values in degrees
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn linearreg_angle<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::linearreg_angle_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::linearreg_angle(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// TSF (Time Series Forecast)
///
/// Calculates rolling time series forecast (one period ahead prediction).
///
/// Args:
///     data: Input data array (`NumPy` array of f64)
///     period: Lookback period
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with forecast values
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn tsf<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::tsf_into(input, period, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::tsf(input, period).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

// =============================================================================
// Price Transform Indicators
// =============================================================================

/// Average Price (AVGPRICE)
///
/// Calculates the average of OHLC prices: (Open + High + Low + Close) / 4
///
/// Args:
///     open: Open prices (`NumPy` array of f64)
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with average price values
#[pyfunction]
#[pyo3(signature = (open, high, low, close, out=None))]
fn avgprice<'py>(
    py: Python<'py>,
    open: PyReadonlyArray1<'py, f64>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let o = open.as_slice()?;
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::avgprice_into(o, h, l, c, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::avgprice(o, h, l, c).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Median Price (MEDPRICE)
///
/// Calculates the median of high and low prices: (High + Low) / 2
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with median price values
#[pyfunction]
#[pyo3(signature = (high, low, out=None))]
fn medprice<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::medprice_into(h, l, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::medprice(h, l).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Typical Price (TYPPRICE)
///
/// Calculates the typical price: (High + Low + Close) / 3
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with typical price values
#[pyfunction]
#[pyo3(signature = (high, low, close, out=None))]
fn typprice<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::typprice_into(h, l, c, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::typprice(h, l, c).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

/// Weighted Close Price (WCLPRICE)
///
/// Calculates the weighted close price: (High + Low + 2*Close) / 4
///
/// Args:
///     high: High prices (`NumPy` array of f64)
///     low: Low prices (`NumPy` array of f64)
///     close: Close prices (`NumPy` array of f64)
///     out: Optional pre-allocated output array for zero-copy writes
///
/// Returns:
///     `NumPy` array with weighted close price values
#[pyfunction]
#[pyo3(signature = (high, low, close, out=None))]
fn wclprice<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let h = high.as_slice()?;
    let l = low.as_slice()?;
    let c = close.as_slice()?;

    if let Some(output) = out {
        let slice = unsafe { output.as_slice_mut()? };
        liq_ta::indicators::wclprice_into(h, l, c, slice).map_err(to_py_err)?;
        Ok(output)
    } else {
        let result = liq_ta::indicators::wclprice(h, l, c).map_err(to_py_err)?;
        Ok(PyArray1::from_vec(py, result))
    }
}

// =============================================================================
// Candlestick Pattern Recognition
// =============================================================================

// Macro to generate candlestick pattern functions (all take OHLC and return i32)
macro_rules! candlestick_fn {
    ($name:ident, $rust_fn:path, $rust_fn_into:path, $doc:expr) => {
        #[doc = $doc]
        #[pyfunction]
        #[pyo3(signature = (open, high, low, close, out=None))]
        fn $name<'py>(
            py: Python<'py>,
            open: PyReadonlyArray1<'py, f64>,
            high: PyReadonlyArray1<'py, f64>,
            low: PyReadonlyArray1<'py, f64>,
            close: PyReadonlyArray1<'py, f64>,
            out: Option<Bound<'py, PyArray1<i32>>>,
        ) -> PyResult<Bound<'py, PyArray1<i32>>> {
            let o = open.as_slice()?;
            let h = high.as_slice()?;
            let l = low.as_slice()?;
            let c = close.as_slice()?;

            match out {
                Some(output) => {
                    let slice = unsafe { output.as_slice_mut()? };
                    $rust_fn_into(o, h, l, c, slice).map_err(to_py_err)?;
                    Ok(output)
                }
                None => {
                    let result = $rust_fn(o, h, l, c).map_err(to_py_err)?;
                    Ok(PyArray1::from_vec(py, result))
                }
            }
        }
    };
}

// Single-candle patterns
candlestick_fn!(
    cdl_doji,
    liq_ta::indicators::candlestick::cdl_doji,
    liq_ta::indicators::candlestick::cdl_doji_into,
    "Doji pattern recognition.\n\nReturns 100 for doji, 0 for no pattern."
);
candlestick_fn!(
    cdl_dragonfly_doji,
    liq_ta::indicators::candlestick::cdl_dragonfly_doji,
    liq_ta::indicators::candlestick::cdl_dragonfly_doji_into,
    "Dragonfly Doji pattern recognition."
);
candlestick_fn!(
    cdl_gravestone_doji,
    liq_ta::indicators::candlestick::cdl_gravestone_doji,
    liq_ta::indicators::candlestick::cdl_gravestone_doji_into,
    "Gravestone Doji pattern recognition."
);
candlestick_fn!(
    cdl_longleg_doji,
    liq_ta::indicators::candlestick::cdl_longleg_doji,
    liq_ta::indicators::candlestick::cdl_longleg_doji_into,
    "Long-Legged Doji pattern recognition."
);
candlestick_fn!(
    cdl_rickshaw_man,
    liq_ta::indicators::candlestick::cdl_rickshaw_man,
    liq_ta::indicators::candlestick::cdl_rickshaw_man_into,
    "Rickshaw Man pattern recognition."
);
candlestick_fn!(
    cdl_marubozu,
    liq_ta::indicators::candlestick::cdl_marubozu,
    liq_ta::indicators::candlestick::cdl_marubozu_into,
    "Marubozu pattern recognition.\n\nReturns 100 for bullish, -100 for bearish."
);
candlestick_fn!(
    cdl_closing_marubozu,
    liq_ta::indicators::candlestick::cdl_closing_marubozu,
    liq_ta::indicators::candlestick::cdl_closing_marubozu_into,
    "Closing Marubozu pattern recognition."
);
candlestick_fn!(
    cdl_spinning_top,
    liq_ta::indicators::candlestick::cdl_spinning_top,
    liq_ta::indicators::candlestick::cdl_spinning_top_into,
    "Spinning Top pattern recognition."
);
candlestick_fn!(
    cdl_high_wave,
    liq_ta::indicators::candlestick::cdl_high_wave,
    liq_ta::indicators::candlestick::cdl_high_wave_into,
    "High-Wave Candle pattern recognition."
);
candlestick_fn!(
    cdl_long_line,
    liq_ta::indicators::candlestick::cdl_long_line,
    liq_ta::indicators::candlestick::cdl_long_line_into,
    "Long Line Candle pattern recognition."
);
candlestick_fn!(
    cdl_short_line,
    liq_ta::indicators::candlestick::cdl_short_line,
    liq_ta::indicators::candlestick::cdl_short_line_into,
    "Short Line Candle pattern recognition."
);
candlestick_fn!(
    cdl_hammer,
    liq_ta::indicators::candlestick::cdl_hammer,
    liq_ta::indicators::candlestick::cdl_hammer_into,
    "Hammer pattern recognition (bullish reversal)."
);
candlestick_fn!(
    cdl_hanging_man,
    liq_ta::indicators::candlestick::cdl_hanging_man,
    liq_ta::indicators::candlestick::cdl_hanging_man_into,
    "Hanging Man pattern recognition (bearish reversal)."
);
candlestick_fn!(
    cdl_inverted_hammer,
    liq_ta::indicators::candlestick::cdl_inverted_hammer,
    liq_ta::indicators::candlestick::cdl_inverted_hammer_into,
    "Inverted Hammer pattern recognition."
);
candlestick_fn!(
    cdl_shooting_star,
    liq_ta::indicators::candlestick::cdl_shooting_star,
    liq_ta::indicators::candlestick::cdl_shooting_star_into,
    "Shooting Star pattern recognition (bearish reversal)."
);
candlestick_fn!(
    cdl_takuri,
    liq_ta::indicators::candlestick::cdl_takuri,
    liq_ta::indicators::candlestick::cdl_takuri_into,
    "Takuri (Dragonfly Doji with long lower shadow) pattern."
);
candlestick_fn!(
    cdl_belt_hold,
    liq_ta::indicators::candlestick::cdl_belt_hold,
    liq_ta::indicators::candlestick::cdl_belt_hold_into,
    "Belt-Hold pattern recognition."
);

// Two-candle patterns
candlestick_fn!(
    cdl_engulfing,
    liq_ta::indicators::candlestick::cdl_engulfing,
    liq_ta::indicators::candlestick::cdl_engulfing_into,
    "Engulfing pattern recognition.\n\nReturns 100 for bullish, -100 for bearish."
);
candlestick_fn!(
    cdl_harami,
    liq_ta::indicators::candlestick::cdl_harami,
    liq_ta::indicators::candlestick::cdl_harami_into,
    "Harami pattern recognition.\n\nReturns 100 for bullish, -100 for bearish."
);
candlestick_fn!(
    cdl_harami_cross,
    liq_ta::indicators::candlestick::cdl_harami_cross,
    liq_ta::indicators::candlestick::cdl_harami_cross_into,
    "Harami Cross pattern recognition."
);
candlestick_fn!(
    cdl_piercing,
    liq_ta::indicators::candlestick::cdl_piercing,
    liq_ta::indicators::candlestick::cdl_piercing_into,
    "Piercing pattern recognition (bullish reversal)."
);
candlestick_fn!(
    cdl_dark_cloud_cover,
    liq_ta::indicators::candlestick::cdl_dark_cloud_cover,
    liq_ta::indicators::candlestick::cdl_dark_cloud_cover_into,
    "Dark Cloud Cover pattern recognition (bearish reversal)."
);
candlestick_fn!(
    cdl_doji_star,
    liq_ta::indicators::candlestick::cdl_doji_star,
    liq_ta::indicators::candlestick::cdl_doji_star_into,
    "Doji Star pattern recognition."
);
candlestick_fn!(
    cdl_kicking,
    liq_ta::indicators::candlestick::cdl_kicking,
    liq_ta::indicators::candlestick::cdl_kicking_into,
    "Kicking pattern recognition."
);
candlestick_fn!(
    cdl_kicking_by_length,
    liq_ta::indicators::candlestick::cdl_kicking_by_length,
    liq_ta::indicators::candlestick::cdl_kicking_by_length_into,
    "Kicking pattern (determined by longer marubozu)."
);
candlestick_fn!(
    cdl_matching_low,
    liq_ta::indicators::candlestick::cdl_matching_low,
    liq_ta::indicators::candlestick::cdl_matching_low_into,
    "Matching Low pattern recognition."
);
candlestick_fn!(
    cdl_homing_pigeon,
    liq_ta::indicators::candlestick::cdl_homing_pigeon,
    liq_ta::indicators::candlestick::cdl_homing_pigeon_into,
    "Homing Pigeon pattern recognition."
);
candlestick_fn!(
    cdl_in_neck,
    liq_ta::indicators::candlestick::cdl_in_neck,
    liq_ta::indicators::candlestick::cdl_in_neck_into,
    "In-Neck pattern recognition."
);
candlestick_fn!(
    cdl_on_neck,
    liq_ta::indicators::candlestick::cdl_on_neck,
    liq_ta::indicators::candlestick::cdl_on_neck_into,
    "On-Neck pattern recognition."
);
candlestick_fn!(
    cdl_thrusting,
    liq_ta::indicators::candlestick::cdl_thrusting,
    liq_ta::indicators::candlestick::cdl_thrusting_into,
    "Thrusting pattern recognition."
);
candlestick_fn!(
    cdl_separating_lines,
    liq_ta::indicators::candlestick::cdl_separating_lines,
    liq_ta::indicators::candlestick::cdl_separating_lines_into,
    "Separating Lines pattern recognition."
);
candlestick_fn!(
    cdl_counter_attack,
    liq_ta::indicators::candlestick::cdl_counter_attack,
    liq_ta::indicators::candlestick::cdl_counter_attack_into,
    "Counter-Attack pattern recognition."
);
candlestick_fn!(
    cdl_2crows,
    liq_ta::indicators::candlestick::cdl_2crows,
    liq_ta::indicators::candlestick::cdl_2crows_into,
    "Two Crows pattern recognition."
);
candlestick_fn!(
    cdl_hikkake,
    liq_ta::indicators::candlestick::cdl_hikkake,
    liq_ta::indicators::candlestick::cdl_hikkake_into,
    "Hikkake pattern recognition."
);
candlestick_fn!(
    cdl_hikkake_mod,
    liq_ta::indicators::candlestick::cdl_hikkake_mod,
    liq_ta::indicators::candlestick::cdl_hikkake_mod_into,
    "Modified Hikkake pattern recognition."
);

// Three-candle patterns
candlestick_fn!(
    cdl_morning_star,
    liq_ta::indicators::candlestick::cdl_morning_star,
    liq_ta::indicators::candlestick::cdl_morning_star_into,
    "Morning Star pattern recognition (bullish reversal)."
);
candlestick_fn!(
    cdl_evening_star,
    liq_ta::indicators::candlestick::cdl_evening_star,
    liq_ta::indicators::candlestick::cdl_evening_star_into,
    "Evening Star pattern recognition (bearish reversal)."
);
candlestick_fn!(
    cdl_morning_doji_star,
    liq_ta::indicators::candlestick::cdl_morning_doji_star,
    liq_ta::indicators::candlestick::cdl_morning_doji_star_into,
    "Morning Doji Star pattern recognition."
);
candlestick_fn!(
    cdl_evening_doji_star,
    liq_ta::indicators::candlestick::cdl_evening_doji_star,
    liq_ta::indicators::candlestick::cdl_evening_doji_star_into,
    "Evening Doji Star pattern recognition."
);
candlestick_fn!(
    cdl_abandoned_baby,
    liq_ta::indicators::candlestick::cdl_abandoned_baby,
    liq_ta::indicators::candlestick::cdl_abandoned_baby_into,
    "Abandoned Baby pattern recognition."
);
candlestick_fn!(
    cdl_3white_soldiers,
    liq_ta::indicators::candlestick::cdl_3white_soldiers,
    liq_ta::indicators::candlestick::cdl_3white_soldiers_into,
    "Three White Soldiers pattern recognition."
);
candlestick_fn!(
    cdl_3black_crows,
    liq_ta::indicators::candlestick::cdl_3black_crows,
    liq_ta::indicators::candlestick::cdl_3black_crows_into,
    "Three Black Crows pattern recognition."
);
candlestick_fn!(
    cdl_3inside,
    liq_ta::indicators::candlestick::cdl_3inside,
    liq_ta::indicators::candlestick::cdl_3inside_into,
    "Three Inside Up/Down pattern recognition."
);
candlestick_fn!(
    cdl_3outside,
    liq_ta::indicators::candlestick::cdl_3outside,
    liq_ta::indicators::candlestick::cdl_3outside_into,
    "Three Outside Up/Down pattern recognition."
);
candlestick_fn!(
    cdl_3line_strike,
    liq_ta::indicators::candlestick::cdl_3line_strike,
    liq_ta::indicators::candlestick::cdl_3line_strike_into,
    "Three-Line Strike pattern recognition."
);
candlestick_fn!(
    cdl_3stars_in_south,
    liq_ta::indicators::candlestick::cdl_3stars_in_south,
    liq_ta::indicators::candlestick::cdl_3stars_in_south_into,
    "Three Stars in the South pattern recognition."
);
candlestick_fn!(
    cdl_tristar,
    liq_ta::indicators::candlestick::cdl_tristar,
    liq_ta::indicators::candlestick::cdl_tristar_into,
    "Tristar pattern recognition."
);
candlestick_fn!(
    cdl_identical_3crows,
    liq_ta::indicators::candlestick::cdl_identical_3crows,
    liq_ta::indicators::candlestick::cdl_identical_3crows_into,
    "Identical Three Crows pattern recognition."
);
candlestick_fn!(
    cdl_stick_sandwich,
    liq_ta::indicators::candlestick::cdl_stick_sandwich,
    liq_ta::indicators::candlestick::cdl_stick_sandwich_into,
    "Stick Sandwich pattern recognition."
);
candlestick_fn!(
    cdl_unique_3river,
    liq_ta::indicators::candlestick::cdl_unique_3river,
    liq_ta::indicators::candlestick::cdl_unique_3river_into,
    "Unique 3 River pattern recognition."
);
candlestick_fn!(
    cdl_advance_block,
    liq_ta::indicators::candlestick::cdl_advance_block,
    liq_ta::indicators::candlestick::cdl_advance_block_into,
    "Advance Block pattern recognition."
);
candlestick_fn!(
    cdl_stalled_pattern,
    liq_ta::indicators::candlestick::cdl_stalled_pattern,
    liq_ta::indicators::candlestick::cdl_stalled_pattern_into,
    "Stalled Pattern recognition."
);
candlestick_fn!(
    cdl_tasuki_gap,
    liq_ta::indicators::candlestick::cdl_tasuki_gap,
    liq_ta::indicators::candlestick::cdl_tasuki_gap_into,
    "Tasuki Gap pattern recognition."
);
candlestick_fn!(
    cdl_upside_gap_2crows,
    liq_ta::indicators::candlestick::cdl_upside_gap_2crows,
    liq_ta::indicators::candlestick::cdl_upside_gap_2crows_into,
    "Upside Gap Two Crows pattern recognition."
);
candlestick_fn!(
    cdl_gap_side_side_white,
    liq_ta::indicators::candlestick::cdl_gap_side_side_white,
    liq_ta::indicators::candlestick::cdl_gap_side_side_white_into,
    "Up/Down-Gap Side-by-Side White Lines pattern."
);
candlestick_fn!(
    cdl_breakaway,
    liq_ta::indicators::candlestick::cdl_breakaway,
    liq_ta::indicators::candlestick::cdl_breakaway_into,
    "Breakaway pattern recognition."
);
candlestick_fn!(
    cdl_ladder_bottom,
    liq_ta::indicators::candlestick::cdl_ladder_bottom,
    liq_ta::indicators::candlestick::cdl_ladder_bottom_into,
    "Ladder Bottom pattern recognition."
);
candlestick_fn!(
    cdl_mat_hold,
    liq_ta::indicators::candlestick::cdl_mat_hold,
    liq_ta::indicators::candlestick::cdl_mat_hold_into,
    "Mat Hold pattern recognition."
);
candlestick_fn!(
    cdl_rise_fall_3methods,
    liq_ta::indicators::candlestick::cdl_rise_fall_3methods,
    liq_ta::indicators::candlestick::cdl_rise_fall_3methods_into,
    "Rising/Falling Three Methods pattern recognition."
);
candlestick_fn!(
    cdl_concealing_baby_swallow,
    liq_ta::indicators::candlestick::cdl_concealing_baby_swallow,
    liq_ta::indicators::candlestick::cdl_concealing_baby_swallow_into,
    "Concealing Baby Swallow pattern recognition."
);
candlestick_fn!(
    cdl_xside_gap_3methods,
    liq_ta::indicators::candlestick::cdl_xside_gap_3methods,
    liq_ta::indicators::candlestick::cdl_xside_gap_3methods_into,
    "Up/Down-Gap Side-by-Side Three Methods pattern."
);

// =============================================================================
// Module Definition
// =============================================================================

/// liq-ta Python module
///
/// High-performance technical analysis library with `NumPy` support.
/// All functions support optional `out=` parameter for zero-copy output.
#[pymodule]
fn _liq_ta(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Moving averages
    m.add_function(wrap_pyfunction!(sma, m)?)?;
    m.add_function(wrap_pyfunction!(ema, m)?)?;
    m.add_function(wrap_pyfunction!(ema_wilder, m)?)?;
    m.add_function(wrap_pyfunction!(wma, m)?)?;
    m.add_function(wrap_pyfunction!(dema, m)?)?;
    m.add_function(wrap_pyfunction!(tema, m)?)?;
    m.add_function(wrap_pyfunction!(trima, m)?)?;
    m.add_function(wrap_pyfunction!(midpoint, m)?)?;
    m.add_function(wrap_pyfunction!(midprice, m)?)?;
    m.add_function(wrap_pyfunction!(kama, m)?)?;
    m.add_function(wrap_pyfunction!(t3, m)?)?;
    m.add_function(wrap_pyfunction!(sar, m)?)?;
    m.add_function(wrap_pyfunction!(sarext, m)?)?;
    m.add_function(wrap_pyfunction!(ht_trendline, m)?)?;
    m.add_function(wrap_pyfunction!(mama, m)?)?;
    m.add_function(wrap_pyfunction!(mavp, m)?)?;

    // Momentum indicators
    m.add_function(wrap_pyfunction!(rsi, m)?)?;
    m.add_function(wrap_pyfunction!(macd, m)?)?;
    m.add_function(wrap_pyfunction!(stochastic, m)?)?;
    m.add_function(wrap_pyfunction!(stochastic_fast, m)?)?;
    m.add_function(wrap_pyfunction!(stochastic_slow, m)?)?;
    m.add_function(wrap_pyfunction!(williams_r, m)?)?;
    m.add_function(wrap_pyfunction!(adx, m)?)?;
    m.add_function(wrap_pyfunction!(mom, m)?)?;
    m.add_function(wrap_pyfunction!(roc, m)?)?;
    m.add_function(wrap_pyfunction!(rocp, m)?)?;
    m.add_function(wrap_pyfunction!(rocr, m)?)?;
    m.add_function(wrap_pyfunction!(rocr100, m)?)?;
    m.add_function(wrap_pyfunction!(apo, m)?)?;
    m.add_function(wrap_pyfunction!(ppo, m)?)?;
    m.add_function(wrap_pyfunction!(bop, m)?)?;
    m.add_function(wrap_pyfunction!(aroon, m)?)?;
    m.add_function(wrap_pyfunction!(aroonosc, m)?)?;
    m.add_function(wrap_pyfunction!(cci, m)?)?;
    m.add_function(wrap_pyfunction!(cmo, m)?)?;
    m.add_function(wrap_pyfunction!(mfi, m)?)?;
    m.add_function(wrap_pyfunction!(stochrsi, m)?)?;
    m.add_function(wrap_pyfunction!(trix, m)?)?;
    m.add_function(wrap_pyfunction!(ultosc, m)?)?;
    m.add_function(wrap_pyfunction!(adxr, m)?)?;
    m.add_function(wrap_pyfunction!(dx, m)?)?;
    m.add_function(wrap_pyfunction!(plus_dm, m)?)?;
    m.add_function(wrap_pyfunction!(minus_dm, m)?)?;

    // Volatility indicators
    m.add_function(wrap_pyfunction!(atr, m)?)?;
    m.add_function(wrap_pyfunction!(true_range, m)?)?;
    m.add_function(wrap_pyfunction!(bollinger, m)?)?;
    m.add_function(wrap_pyfunction!(donchian, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_stddev, m)?)?;

    // Volume indicators
    m.add_function(wrap_pyfunction!(obv, m)?)?;
    m.add_function(wrap_pyfunction!(vwap, m)?)?;
    m.add_function(wrap_pyfunction!(ad, m)?)?;
    m.add_function(wrap_pyfunction!(adosc, m)?)?;

    // Cycle indicators (Hilbert Transform)
    m.add_function(wrap_pyfunction!(ht_dcperiod, m)?)?;
    m.add_function(wrap_pyfunction!(ht_dcphase, m)?)?;
    m.add_function(wrap_pyfunction!(ht_phasor, m)?)?;
    m.add_function(wrap_pyfunction!(ht_sine, m)?)?;
    m.add_function(wrap_pyfunction!(ht_trendmode, m)?)?;

    // Price transform indicators
    m.add_function(wrap_pyfunction!(avgprice, m)?)?;
    m.add_function(wrap_pyfunction!(medprice, m)?)?;
    m.add_function(wrap_pyfunction!(typprice, m)?)?;
    m.add_function(wrap_pyfunction!(wclprice, m)?)?;

    // Statistical functions
    m.add_function(wrap_pyfunction!(var, m)?)?;
    m.add_function(wrap_pyfunction!(correl, m)?)?;
    m.add_function(wrap_pyfunction!(beta, m)?)?;
    m.add_function(wrap_pyfunction!(linearreg, m)?)?;
    m.add_function(wrap_pyfunction!(linearreg_slope, m)?)?;
    m.add_function(wrap_pyfunction!(linearreg_intercept, m)?)?;
    m.add_function(wrap_pyfunction!(linearreg_angle, m)?)?;
    m.add_function(wrap_pyfunction!(tsf, m)?)?;

    // Candlestick pattern recognition
    // Single-candle patterns
    m.add_function(wrap_pyfunction!(cdl_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_dragonfly_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_gravestone_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_longleg_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_rickshaw_man, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_marubozu, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_closing_marubozu, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_spinning_top, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_high_wave, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_long_line, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_short_line, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_hammer, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_hanging_man, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_inverted_hammer, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_shooting_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_takuri, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_belt_hold, m)?)?;
    // Two-candle patterns
    m.add_function(wrap_pyfunction!(cdl_engulfing, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_harami, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_harami_cross, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_piercing, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_dark_cloud_cover, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_doji_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_kicking, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_kicking_by_length, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_matching_low, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_homing_pigeon, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_in_neck, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_on_neck, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_thrusting, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_separating_lines, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_counter_attack, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_2crows, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_hikkake, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_hikkake_mod, m)?)?;
    // Three-candle and complex patterns
    m.add_function(wrap_pyfunction!(cdl_morning_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_evening_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_morning_doji_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_evening_doji_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_abandoned_baby, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_3white_soldiers, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_3black_crows, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_3inside, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_3outside, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_3line_strike, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_3stars_in_south, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_tristar, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_identical_3crows, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_stick_sandwich, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_unique_3river, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_advance_block, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_stalled_pattern, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_tasuki_gap, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_upside_gap_2crows, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_gap_side_side_white, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_breakaway, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_ladder_bottom, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_mat_hold, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_rise_fall_3methods, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_concealing_baby_swallow, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_xside_gap_3methods, m)?)?;

    Ok(())
}
