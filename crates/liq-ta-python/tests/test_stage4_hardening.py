import numpy as np
import pytest

import liq_ta


def test_compute_indicator_rejects_unknown_indicator():
    with pytest.raises(liq_ta.IndicatorNotFoundError, match="unknown indicator"):
        liq_ta.compute_indicator("definitely_not_real", np.array([1.0, 2.0, 3.0]), 5)


def test_compute_indicator_maps_arg_shape_errors():
    prices = np.linspace(100.0, 110.0, 32)

    with pytest.raises(liq_ta.IndicatorArgumentError, match="invalid argument shape"):
        # Missing required `period` argument for SMA.
        liq_ta.compute_indicator("sma", prices)


def test_compute_indicator_debug_mode_includes_expected_signature_context():
    prices = np.linspace(100.0, 110.0, 32)

    with pytest.raises(liq_ta.IndicatorArgumentError) as exc_info:
        liq_ta.compute_indicator("sma", prices, debug=True)

    msg = str(exc_info.value)
    assert "expected_inputs=" in msg
    assert "expected_params=" in msg


def test_require_indicator_info_rejects_blank_name():
    with pytest.raises(liq_ta.IndicatorArgumentError, match="must not be empty"):
        liq_ta.require_indicator_info("   ")


def test_list_indicators_rejects_unknown_category_with_actionable_error():
    with pytest.raises(liq_ta.IndicatorArgumentError, match="valid categories"):
        liq_ta.list_indicators(category="nonexistent")


def test_validate_indicator_metadata_returns_clean_state():
    diagnostics = liq_ta.validate_indicator_metadata(raise_on_error=False)
    assert diagnostics == []


def test_validate_indicator_metadata_rejects_missing_runtime_input_shape():
    if not liq_ta._RUNTIME_INDICATOR_METADATA:
        pytest.skip("runtime registry metadata not populated")

    runtime_name = next(iter(liq_ta._RUNTIME_INDICATOR_METADATA))
    baseline = liq_ta._RUNTIME_INDICATOR_METADATA[runtime_name]
    mutated = dict(baseline)
    mutated.pop("input_shape", None)

    original_runtime = dict(liq_ta._RUNTIME_INDICATOR_METADATA)
    try:
        liq_ta._RUNTIME_INDICATOR_METADATA[runtime_name] = mutated
        with pytest.raises(
            liq_ta.IndicatorMetadataError,
            match=f"indicator '{runtime_name}' missing required key 'input_shape'",
        ):
            liq_ta.validate_indicator_metadata()
    finally:
        liq_ta._RUNTIME_INDICATOR_METADATA.clear()
        liq_ta._RUNTIME_INDICATOR_METADATA.update(original_runtime)
