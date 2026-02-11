"""Tests for liq-ta Python bindings."""

import numpy as np
import pytest

import liq_ta


class TestSMA:
    """Simple Moving Average tests."""

    def test_basic_sma(self):
        prices = np.array([1.0, 2.0, 3.0, 4.0, 5.0])
        result = liq_ta.sma(prices, 3)
        assert len(result) == 5
        assert np.isnan(result[0])
        assert np.isnan(result[1])
        assert result[2] == pytest.approx(2.0)
        assert result[3] == pytest.approx(3.0)
        assert result[4] == pytest.approx(4.0)

    def test_sma_returns_numpy_array(self):
        prices = np.array([1.0, 2.0, 3.0, 4.0, 5.0])
        result = liq_ta.sma(prices, 2)
        assert isinstance(result, np.ndarray)
        assert result.dtype == np.float64

    def test_sma_invalid_period(self):
        prices = np.array([1.0, 2.0, 3.0])
        with pytest.raises(ValueError, match="period"):
            liq_ta.sma(prices, 10)


class TestEMA:
    """Exponential Moving Average tests."""

    def test_basic_ema(self):
        prices = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])
        result = liq_ta.ema(prices, 5)
        assert len(result) == 10
        # First 4 values should be NaN
        assert all(np.isnan(result[:4]))
        # Remaining values should be valid
        assert all(~np.isnan(result[4:]))

    def test_ema_wilder(self):
        prices = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])
        result = liq_ta.ema_wilder(prices, 5)
        assert len(result) == 10


class TestRSI:
    """Relative Strength Index tests."""

    def test_rsi_range(self):
        np.random.seed(42)
        prices = 100 + np.cumsum(np.random.randn(100) * 0.5)
        result = liq_ta.rsi(prices, 14)

        # Valid values should be between 0 and 100
        valid = result[~np.isnan(result)]
        assert all(valid >= 0)
        assert all(valid <= 100)


class TestMACD:
    """MACD tests."""

    def test_macd_output_structure(self):
        np.random.seed(42)
        prices = 100 + np.cumsum(np.random.randn(50) * 0.5)
        line, signal, histogram = liq_ta.macd(prices)

        assert len(line) == 50
        assert len(signal) == 50
        assert len(histogram) == 50

    def test_macd_custom_periods(self):
        np.random.seed(42)
        prices = 100 + np.cumsum(np.random.randn(50) * 0.5)
        line, signal, histogram = liq_ta.macd(prices, fast_period=8, slow_period=17, signal_period=9)

        # Should compute without error
        assert len(line) == 50


class TestBollinger:
    """Bollinger Bands tests."""

    def test_bollinger_bands_structure(self):
        prices = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 4.0, 5.0, 6.0])
        upper, middle, lower = liq_ta.bollinger(prices, period=5)

        assert len(upper) == 10
        assert len(middle) == 10
        assert len(lower) == 10

        # Upper should be >= middle >= lower (for valid values)
        valid_idx = ~np.isnan(upper)
        assert all(upper[valid_idx] >= middle[valid_idx])
        assert all(middle[valid_idx] >= lower[valid_idx])


class TestStochastic:
    """Stochastic Oscillator tests."""

    def test_stochastic_range(self):
        high = np.array([10.0, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5])
        low = np.array([9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5])
        close = np.array([9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0])

        k, d = liq_ta.stochastic(high, low, close, 5, 3, 1)

        # Valid values should be between 0 and 100
        valid_k = k[~np.isnan(k)]
        valid_d = d[~np.isnan(d)]
        assert all(valid_k >= 0) and all(valid_k <= 100)
        assert all(valid_d >= 0) and all(valid_d <= 100)

    def test_stochastic_fast(self):
        # Need 14+ elements for default k_period=14
        np.random.seed(42)
        high = 100 + np.cumsum(np.random.randn(20) * 0.5) + np.abs(np.random.randn(20) * 0.3)
        low = high - np.abs(np.random.randn(20) * 0.6)
        close = (high + low) / 2

        k, d = liq_ta.stochastic_fast(high, low, close)
        assert len(k) == 20

    def test_stochastic_slow(self):
        # Need 14+ elements for default k_period=14
        np.random.seed(42)
        high = 100 + np.cumsum(np.random.randn(20) * 0.5) + np.abs(np.random.randn(20) * 0.3)
        low = high - np.abs(np.random.randn(20) * 0.6)
        close = (high + low) / 2

        k, d = liq_ta.stochastic_slow(high, low, close)
        assert len(k) == 20


class TestATR:
    """Average True Range tests."""

    def test_atr_positive(self):
        high = np.array([10.0, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5])
        low = np.array([9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5])
        close = np.array([9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0])

        result = liq_ta.atr(high, low, close, 5)

        # ATR should be positive
        valid = result[~np.isnan(result)]
        assert all(valid > 0)

    def test_true_range(self):
        high = np.array([10.0, 11.0, 12.0])
        low = np.array([9.0, 10.0, 11.0])
        close = np.array([9.5, 10.5, 11.5])

        result = liq_ta.true_range(high, low, close)
        assert len(result) == 3


class TestADX:
    """Average Directional Index tests."""

    def test_adx_output(self):
        np.random.seed(42)
        high = 100 + np.cumsum(np.random.randn(30) * 0.5) + np.abs(np.random.randn(30) * 0.3)
        low = high - np.abs(np.random.randn(30) * 0.6)
        close = (high + low) / 2

        adx, plus_di, minus_di = liq_ta.adx(high, low, close, 14)

        assert len(adx) == 30
        assert len(plus_di) == 30
        assert len(minus_di) == 30


class TestWilliamsR:
    """Williams %R tests."""

    def test_williams_r_range(self):
        high = np.array([10.0, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5])
        low = np.array([9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5])
        close = np.array([9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0])

        result = liq_ta.williams_r(high, low, close, 5)

        # Williams %R should be between -100 and 0
        valid = result[~np.isnan(result)]
        assert all(valid >= -100) and all(valid <= 0)


class TestDonchian:
    """Donchian Channels tests."""

    def test_donchian_structure(self):
        high = np.array([10.0, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5])
        low = np.array([9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5])

        upper, middle, lower = liq_ta.donchian(high, low, 5)

        assert len(upper) == 10
        assert len(middle) == 10
        assert len(lower) == 10


class TestOBV:
    """On-Balance Volume tests."""

    def test_obv_basic(self):
        close = np.array([10.0, 10.5, 10.2, 10.8, 10.5])
        volume = np.array([1000.0, 1500.0, 1200.0, 1800.0, 1100.0])

        result = liq_ta.obv(close, volume)

        assert len(result) == 5
        assert result[0] == 1000.0  # First value is first volume


class TestVWAP:
    """Volume Weighted Average Price tests."""

    def test_vwap_basic(self):
        high = np.array([10.0, 11.0, 12.0, 11.5, 12.5])
        low = np.array([9.0, 10.0, 11.0, 10.5, 11.5])
        close = np.array([9.5, 10.5, 11.5, 11.0, 12.0])
        volume = np.array([1000.0, 1500.0, 1200.0, 1800.0, 1100.0])

        result = liq_ta.vwap(high, low, close, volume)

        assert len(result) == 5


class TestRollingStddev:
    """Rolling Standard Deviation tests."""

    def test_rolling_stddev(self):
        data = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 1.0, 2.0])
        result = liq_ta.rolling_stddev(data, 3)

        assert len(result) == 10
        # Stddev should be non-negative
        valid = result[~np.isnan(result)]
        assert all(valid >= 0)


class TestPolarsIntegration:
    """Polars DataFrame integration tests."""

    def test_polars_series_via_numpy(self):
        """Test that Polars Series works via to_numpy()."""
        pytest.importorskip("polars")
        import polars as pl

        prices = pl.Series("price", [44.0, 44.5, 43.5, 44.0, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0])
        result = liq_ta.sma(prices.to_numpy(), 5)

        assert len(result) == 10
        assert result[4] == pytest.approx(44.1)

    def test_polars_dataframe_workflow(self):
        """Test typical Polars DataFrame workflow."""
        pytest.importorskip("polars")
        import polars as pl

        df = pl.DataFrame({
            "high": [10.0, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 11.0, 10.5, 11.5],
            "low": [9.0, 10.0, 11.0, 10.5, 11.5, 12.0, 11.0, 10.0, 9.5, 10.5],
            "close": [9.5, 10.5, 11.5, 11.0, 12.0, 12.5, 11.5, 10.5, 10.0, 11.0],
        })

        # Calculate ATR
        atr = liq_ta.atr(
            df["high"].to_numpy(),
            df["low"].to_numpy(),
            df["close"].to_numpy(),
            5,
        )

        # Add back to DataFrame
        df = df.with_columns(pl.Series("atr", atr))
        assert "atr" in df.columns
        assert len(df) == 10


class TestZeroCopy:
    """Zero-copy behavior validation tests."""

    def test_contiguous_array_works(self):
        """Contiguous arrays should work (zero-copy input)."""
        data = np.array([1.0, 2.0, 3.0, 4.0, 5.0], dtype=np.float64)
        assert data.flags["C_CONTIGUOUS"]
        result = liq_ta.sma(data, 2)
        assert len(result) == 5

    def test_sma_out_zero_copy_output(self):
        """sma with out= should write to pre-allocated array without copying."""
        data = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])
        out = np.empty(len(data), dtype=np.float64)
        original_ptr = out.ctypes.data

        result = liq_ta.sma(data, 3, out=out)

        # Pointer should be unchanged (same memory)
        assert out.ctypes.data == original_ptr
        # Result should be the same array
        assert result is out
        assert out[2] == pytest.approx(2.0)
        assert out[9] == pytest.approx(9.0)

    def test_ema_out_zero_copy_output(self):
        """ema with out= should write to pre-allocated array."""
        data = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])
        out = np.empty(len(data), dtype=np.float64)

        result = liq_ta.ema(data, 3, out=out)

        assert result is out
        assert not np.isnan(out[2])

    def test_rsi_out_zero_copy_output(self):
        """rsi with out= should write to pre-allocated array."""
        np.random.seed(42)
        data = 100 + np.cumsum(np.random.randn(50) * 0.5)
        out = np.empty(len(data), dtype=np.float64)

        result = liq_ta.rsi(data, 14, out=out)

        assert result is out
        valid = out[~np.isnan(out)]
        assert all(valid >= 0) and all(valid <= 100)

    def test_atr_out_zero_copy_output(self):
        """atr with out= should write to pre-allocated array."""
        np.random.seed(42)
        high = 100 + np.cumsum(np.random.randn(30) * 0.5) + np.abs(np.random.randn(30) * 0.3)
        low = high - np.abs(np.random.randn(30) * 0.6)
        close = (high + low) / 2
        out = np.empty(30, dtype=np.float64)

        result = liq_ta.atr(high, low, close, 14, out=out)

        assert result is out
        valid = out[~np.isnan(out)]
        assert all(valid > 0)

    def test_out_matches_allocating(self):
        """out= variants should produce identical results to allocating versions."""
        data = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])

        # SMA
        expected = liq_ta.sma(data, 3)
        out = np.empty(len(data), dtype=np.float64)
        liq_ta.sma(data, 3, out=out)
        assert np.allclose(expected, out, equal_nan=True)

        # EMA
        expected = liq_ta.ema(data, 3)
        liq_ta.ema(data, 3, out=out)
        assert np.allclose(expected, out, equal_nan=True)

    def test_non_contiguous_array_fails(self):
        """Non-contiguous arrays should raise an error."""
        data = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], dtype=np.float64)
        strided = data[::2]  # Every other element - not contiguous
        assert not strided.flags["C_CONTIGUOUS"]

        # TypeError is raised for non-contiguous arrays
        with pytest.raises(TypeError, match="contiguous"):
            liq_ta.sma(strided, 2)

    def test_non_contiguous_2d_fails(self):
        """Non-contiguous 2D slice should fail."""
        data = np.array([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]], dtype=np.float64)
        # Take every other row - not contiguous
        strided = data[::2, 0]
        assert not strided.flags["C_CONTIGUOUS"]

        with pytest.raises(TypeError, match="contiguous"):
            liq_ta.sma(strided, 2)

    def test_contiguous_copy_works(self):
        """Non-contiguous arrays work after np.ascontiguousarray()."""
        data = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], dtype=np.float64)
        strided = data[::2]
        contiguous = np.ascontiguousarray(strided)

        assert contiguous.flags["C_CONTIGUOUS"]
        result = liq_ta.sma(contiguous, 2)
        assert len(result) == 4


class TestEdgeCases:
    """Edge case tests."""

    def test_empty_array_raises(self):
        with pytest.raises(ValueError):
            liq_ta.sma(np.array([]), 5)

    def test_period_zero_raises(self):
        with pytest.raises(ValueError):
            liq_ta.sma(np.array([1.0, 2.0, 3.0]), 0)

    def test_mismatched_lengths_raises(self):
        high = np.array([10.0, 11.0, 12.0])
        low = np.array([9.0, 10.0])  # Different length
        close = np.array([9.5, 10.5, 11.5])

        with pytest.raises(ValueError):
            liq_ta.atr(high, low, close, 2)
