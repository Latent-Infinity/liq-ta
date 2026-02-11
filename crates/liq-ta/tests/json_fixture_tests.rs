//! JSON-driven spec fixture tests - authoritative source of truth for indicator behavior.
//!
//! These tests load fixtures from JSON files and verify indicator behavior.
//! The JSON files in tests/fixtures/ are the canonical specification.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::manual_let_else)]

use liq_ta::indicators::{
    atr::{atr, true_range},
    bollinger::bollinger,
    ema::ema,
    macd::macd,
    rsi::rsi,
    stochastic::stochastic_fast,
};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const EPSILON: f64 = 1e-10;
const LOOSE_EPSILON: f64 = 1e-6;
const SPEC_VERSION: &str = "3.0";

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    (a - b).abs() < eps
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[derive(Debug, Deserialize)]
struct SpecFixture {
    spec_version: String,
    rationale: String,
    #[serde(default)]
    input: Value,
    #[serde(default)]
    params: Value,
    #[serde(default)]
    expected: Value,
}

fn load_fixture(path: &Path) -> Option<SpecFixture> {
    let content = fs::read_to_string(path).expect("Failed to read fixture file");
    let value: Value = serde_json::from_str(&content).expect("Failed to parse fixture JSON");
    value.get("spec_version")?;
    let fixture: SpecFixture = serde_json::from_value(value).expect("Invalid fixture schema");
    assert_eq!(
        fixture.spec_version, SPEC_VERSION,
        "Fixture spec_version must match current PRD version"
    );
    Some(fixture)
}

fn parse_vec_f64(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("Expected array")
        .iter()
        .map(|v| v.as_f64().expect("Expected f64"))
        .collect()
}

fn parse_opt_vec_f64(value: &Value) -> Vec<Option<f64>> {
    value
        .as_array()
        .expect("Expected array")
        .iter()
        .map(serde_json::Value::as_f64)
        .collect()
}

fn parse_input_series(input: &Value) -> Vec<f64> {
    parse_vec_f64(input)
}

fn parse_input_ohlc(input: &Value) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let obj = input.as_object().expect("Expected input object");
    let high = parse_vec_f64(obj.get("high").expect("Missing high"));
    let low = parse_vec_f64(obj.get("low").expect("Missing low"));
    let close = parse_vec_f64(obj.get("close").expect("Missing close"));
    (high, low, close)
}

fn parse_length(input: &Value) -> usize {
    let obj = input.as_object().expect("Expected input object");
    obj.get("length")
        .and_then(Value::as_u64)
        .expect("Missing length") as usize
}

fn assert_expected_vec(actual: &[f64], expected: &[Option<f64>], eps: f64, label: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{label}: output length mismatch"
    );
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        match e {
            None => assert!(a.is_nan(), "{label}[{i}] expected NaN, got {a}"),
            Some(exp) => assert!(
                approx_eq(*a, *exp, eps),
                "{label}[{i}] expected {exp}, got {a}"
            ),
        }
    }
}

#[test]
fn json_spec_fixtures() {
    let dir = fixtures_dir();
    let entries = fs::read_dir(&dir).expect("Failed to read fixtures directory");

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let fixture = match load_fixture(&path) {
            Some(fixture) => fixture,
            None => continue,
        };

        if file_name.starts_with("spec_ema_sma_seed_") {
            let input = parse_input_series(&fixture.input);
            let period = fixture
                .params
                .get("period")
                .and_then(Value::as_u64)
                .expect("Missing period") as usize;
            let expected = parse_opt_vec_f64(&fixture.expected);
            let result = ema(&input, period).expect("EMA failed");
            assert_expected_vec(&result, &expected, EPSILON, file_name);
            continue;
        }

        if file_name.starts_with("spec_rsi_extremes_") {
            let input = parse_input_series(&fixture.input);
            let period = fixture
                .params
                .get("period")
                .and_then(Value::as_u64)
                .expect("Missing period") as usize;
            let expected = parse_opt_vec_f64(&fixture.expected);
            let result = rsi(&input, period).expect("RSI failed");
            assert_expected_vec(&result, &expected, LOOSE_EPSILON, file_name);
            continue;
        }

        if file_name.starts_with("spec_rsi_wilder_") {
            let input = parse_input_series(&fixture.input);
            let period = fixture
                .params
                .get("period")
                .and_then(Value::as_u64)
                .expect("Missing period") as usize;
            let range = fixture
                .expected
                .get("expected_range")
                .expect("Missing expected_range")
                .as_object()
                .expect("expected_range must be object");
            let index = range
                .get("index")
                .and_then(Value::as_u64)
                .expect("Missing range index") as usize;
            let min = range
                .get("min")
                .and_then(Value::as_f64)
                .expect("Missing range min");
            let max = range
                .get("max")
                .and_then(Value::as_f64)
                .expect("Missing range max");
            let result = rsi(&input, period).expect("RSI failed");
            let value = result[index];
            assert!(
                value >= min && value <= max,
                "{file_name}[{index}] expected in [{min}, {max}], got {value}"
            );
            continue;
        }

        if file_name.starts_with("spec_atr_gap_") {
            let (high, low, close) = parse_input_ohlc(&fixture.input);
            let expected = fixture
                .expected
                .get("expected_tr")
                .expect("Missing expected_tr");
            let expected = parse_opt_vec_f64(expected);
            let result = true_range(&high, &low, &close).expect("TR failed");
            assert_expected_vec(&result, &expected, EPSILON, file_name);
            continue;
        }

        if file_name.starts_with("spec_atr_initialization_") {
            let (high, low, close) = parse_input_ohlc(&fixture.input);
            let period = fixture
                .params
                .get("period")
                .and_then(Value::as_u64)
                .expect("Missing period") as usize;
            let expected_index = fixture
                .expected
                .get("expected_first_valid_index")
                .and_then(Value::as_u64)
                .expect("Missing expected_first_valid_index")
                as usize;
            let result = atr(&high, &low, &close, period).expect("ATR failed");
            let tr = true_range(&high, &low, &close).expect("TR failed");
            let mut sum = 0.0;
            for value in tr.iter().skip(1).take(period) {
                sum += *value;
            }
            let expected_atr = sum / period as f64;
            assert!(
                approx_eq(result[expected_index], expected_atr, EPSILON),
                "{file_name}: expected first ATR {expected_atr}, got {}",
                result[expected_index]
            );
            continue;
        }

        if file_name.starts_with("spec_bollinger_collapse_")
            || file_name.starts_with("spec_bollinger_width_")
        {
            let input = parse_input_series(&fixture.input);
            let period = fixture
                .params
                .get("period")
                .and_then(Value::as_u64)
                .expect("Missing period") as usize;
            let num_std_dev = fixture
                .params
                .get("num_std_dev")
                .and_then(Value::as_f64)
                .expect("Missing num_std_dev");
            let result = bollinger(&input, period, num_std_dev).expect("Bollinger failed");

            if let Some(expected_obj) = fixture.expected.as_object() {
                if expected_obj.contains_key("middle") {
                    let middle =
                        parse_opt_vec_f64(expected_obj.get("middle").expect("Missing middle"));
                    let upper =
                        parse_opt_vec_f64(expected_obj.get("upper").expect("Missing upper"));
                    let lower =
                        parse_opt_vec_f64(expected_obj.get("lower").expect("Missing lower"));
                    assert_expected_vec(&result.middle, &middle, EPSILON, file_name);
                    assert_expected_vec(&result.upper, &upper, EPSILON, file_name);
                    assert_expected_vec(&result.lower, &lower, EPSILON, file_name);
                } else if expected_obj.get("expected_property")
                    == Some(&Value::String(
                        "upper - middle == middle - lower".to_string(),
                    ))
                {
                    for i in 0..result.middle.len() {
                        let upper = result.upper[i];
                        let middle = result.middle[i];
                        let lower = result.lower[i];
                        if upper.is_nan() || middle.is_nan() || lower.is_nan() {
                            continue;
                        }
                        let left = upper - middle;
                        let right = middle - lower;
                        assert!(
                            approx_eq(left, right, EPSILON),
                            "{file_name}[{i}] symmetric bands expected, got {left} vs {right}"
                        );
                    }
                } else if expected_obj.get("property")
                    == Some(&Value::String(
                        "no NaN in output except lookback positions".to_string(),
                    ))
                {
                    let lookback = period.saturating_sub(1);
                    for (i, value) in result.middle.iter().enumerate() {
                        if i >= lookback {
                            assert!(
                                !value.is_nan(),
                                "{file_name}[{i}] unexpected NaN after lookback"
                            );
                        }
                    }
                }
            }
            continue;
        }

        if file_name.starts_with("spec_stochastic_midpoint_")
            || file_name.starts_with("spec_stochastic_boundary_")
        {
            let (high, low, close) = parse_input_ohlc(&fixture.input);
            let k_period = fixture
                .params
                .get("k_period")
                .and_then(Value::as_u64)
                .expect("Missing k_period") as usize;
            let d_period = fixture
                .params
                .get("d_period")
                .and_then(Value::as_u64)
                .expect("Missing d_period") as usize;
            let result = stochastic_fast(&high, &low, &close, k_period, d_period)
                .expect("Stochastic failed");

            if let Some(expected_k) = fixture.expected.get("expected_k") {
                let expected = parse_opt_vec_f64(expected_k);
                assert_expected_vec(&result.k, &expected, EPSILON, file_name);
            } else if let Some(expected_k) = fixture.expected.get("expected_k_approx") {
                let expected = parse_opt_vec_f64(expected_k);
                assert_expected_vec(&result.k, &expected, LOOSE_EPSILON, file_name);
            } else if let Some(expected_k) = fixture.expected.as_array() {
                let expected = expected_k
                    .iter()
                    .map(serde_json::Value::as_f64)
                    .collect::<Vec<_>>();
                assert_expected_vec(&result.k, &expected, EPSILON, file_name);
            } else if let (Some(expected_val), Some(indices)) = (
                fixture.expected.get("expected_k_at_flat"),
                fixture.expected.get("flat_indices"),
            ) {
                let expected_val = expected_val
                    .as_f64()
                    .expect("expected_k_at_flat must be f64");
                for index in indices.as_array().expect("flat_indices must be array") {
                    let idx = index.as_u64().expect("flat index must be u64") as usize;
                    assert!(
                        approx_eq(result.k[idx], expected_val, EPSILON),
                        "{file_name}[{idx}] expected {expected_val}, got {}",
                        result.k[idx]
                    );
                }
            }
            continue;
        }

        if file_name.starts_with("spec_macd_alignment_") {
            let length = parse_length(&fixture.input);
            let fast = fixture
                .params
                .get("fast")
                .and_then(Value::as_u64)
                .expect("Missing fast") as usize;
            let slow = fixture
                .params
                .get("slow")
                .and_then(Value::as_u64)
                .expect("Missing slow") as usize;
            let signal = fixture
                .params
                .get("signal")
                .and_then(Value::as_u64)
                .expect("Missing signal") as usize;
            let input: Vec<f64> = (0..length).map(|v| v as f64).collect();
            let result = macd(&input, fast, slow, signal).expect("MACD failed");

            if let Some(expected_len) = fixture.expected.get("expected_output_length") {
                let expected_len = expected_len.as_u64().unwrap() as usize;
                assert_eq!(result.macd_line.len(), expected_len, "{file_name}");
                assert_eq!(result.signal_line.len(), expected_len, "{file_name}");
                assert_eq!(result.histogram.len(), expected_len, "{file_name}");
            }
            if fixture
                .expected
                .get("properties")
                .and_then(|v| v.get("histogram_equals_macd_minus_signal"))
                == Some(&Value::Bool(true))
                || fixture.expected.get("property").is_some()
            {
                for i in 0..result.macd_line.len() {
                    if result.macd_line[i].is_nan() || result.signal_line[i].is_nan() {
                        continue;
                    }
                    let expected_hist = result.macd_line[i] - result.signal_line[i];
                    assert!(
                        approx_eq(result.histogram[i], expected_hist, EPSILON),
                        "{file_name}[{i}] histogram mismatch"
                    );
                }
            }
            continue;
        }

        if file_name.starts_with("spec_lookback_") {
            let params = fixture.params.as_object().expect("params must be object");
            if let Some(period) = params.get("period") {
                let period = period.as_u64().unwrap() as usize;
                if file_name.contains("sma") {
                    use liq_ta::indicators::sma::{sma_lookback, sma_min_len};
                    assert_eq!(
                        sma_lookback(period),
                        fixture
                            .expected
                            .get("expected_lookback")
                            .and_then(Value::as_u64)
                            .unwrap() as usize,
                        "{file_name}"
                    );
                    assert_eq!(
                        sma_min_len(period),
                        fixture
                            .expected
                            .get("expected_min_len")
                            .and_then(Value::as_u64)
                            .unwrap() as usize,
                        "{file_name}"
                    );
                } else if file_name.contains("ema") {
                    use liq_ta::indicators::ema::{ema_lookback, ema_min_len};
                    assert_eq!(
                        ema_lookback(period),
                        fixture
                            .expected
                            .get("expected_lookback")
                            .and_then(Value::as_u64)
                            .unwrap() as usize,
                        "{file_name}"
                    );
                    assert_eq!(
                        ema_min_len(period),
                        fixture
                            .expected
                            .get("expected_min_len")
                            .and_then(Value::as_u64)
                            .unwrap() as usize,
                        "{file_name}"
                    );
                } else if file_name.contains("rsi") {
                    use liq_ta::indicators::rsi::{rsi_lookback, rsi_min_len};
                    assert_eq!(
                        rsi_lookback(period),
                        fixture
                            .expected
                            .get("expected_lookback")
                            .and_then(Value::as_u64)
                            .unwrap() as usize,
                        "{file_name}"
                    );
                    assert_eq!(
                        rsi_min_len(period),
                        fixture
                            .expected
                            .get("expected_min_len")
                            .and_then(Value::as_u64)
                            .unwrap() as usize,
                        "{file_name}"
                    );
                } else if file_name.contains("atr") {
                    use liq_ta::indicators::atr::{atr_lookback, atr_min_len};
                    assert_eq!(
                        atr_lookback(period),
                        fixture
                            .expected
                            .get("expected_lookback")
                            .and_then(Value::as_u64)
                            .unwrap() as usize,
                        "{file_name}"
                    );
                    assert_eq!(
                        atr_min_len(period),
                        fixture
                            .expected
                            .get("expected_min_len")
                            .and_then(Value::as_u64)
                            .unwrap() as usize,
                        "{file_name}"
                    );
                } else if file_name.contains("bollinger") {
                    use liq_ta::indicators::bollinger::{bollinger_lookback, bollinger_min_len};
                    assert_eq!(
                        bollinger_lookback(period),
                        fixture
                            .expected
                            .get("expected_lookback")
                            .and_then(Value::as_u64)
                            .unwrap() as usize,
                        "{file_name}"
                    );
                    assert_eq!(
                        bollinger_min_len(period),
                        fixture
                            .expected
                            .get("expected_min_len")
                            .and_then(Value::as_u64)
                            .unwrap() as usize,
                        "{file_name}"
                    );
                }
            } else if file_name.contains("macd") {
                use liq_ta::indicators::macd::{
                    macd_line_lookback, macd_min_len, macd_signal_lookback,
                };
                let fast = fixture
                    .params
                    .get("fast_period")
                    .and_then(Value::as_u64)
                    .unwrap() as usize;
                let slow = fixture
                    .params
                    .get("slow_period")
                    .and_then(Value::as_u64)
                    .unwrap() as usize;
                let signal = fixture
                    .params
                    .get("signal_period")
                    .and_then(Value::as_u64)
                    .unwrap() as usize;
                assert_eq!(
                    macd_line_lookback(slow),
                    fixture
                        .expected
                        .get("expected_macd_lookback")
                        .and_then(Value::as_u64)
                        .unwrap() as usize,
                    "{file_name}"
                );
                assert_eq!(
                    macd_signal_lookback(slow, signal),
                    fixture
                        .expected
                        .get("expected_signal_lookback")
                        .and_then(Value::as_u64)
                        .unwrap() as usize,
                    "{file_name}"
                );
                assert_eq!(
                    macd_min_len(slow, signal),
                    fixture
                        .expected
                        .get("expected_min_len")
                        .and_then(Value::as_u64)
                        .unwrap() as usize,
                    "{file_name}"
                );
                let _ = fast; // included for completeness, not used in lookback formulas
            } else if file_name.contains("stochastic") {
                use liq_ta::indicators::stochastic::{
                    stochastic_d_lookback, stochastic_k_lookback, stochastic_min_len,
                };
                let k = fixture
                    .params
                    .get("k_period")
                    .and_then(Value::as_u64)
                    .unwrap() as usize;
                let d = fixture
                    .params
                    .get("d_period")
                    .and_then(Value::as_u64)
                    .unwrap() as usize;
                assert_eq!(
                    stochastic_k_lookback(k),
                    fixture
                        .expected
                        .get("expected_k_lookback")
                        .and_then(Value::as_u64)
                        .unwrap() as usize,
                    "{file_name}"
                );
                assert_eq!(
                    stochastic_d_lookback(k, d),
                    fixture
                        .expected
                        .get("expected_d_lookback")
                        .and_then(Value::as_u64)
                        .unwrap() as usize,
                    "{file_name}"
                );
                assert_eq!(
                    stochastic_min_len(k, d),
                    fixture
                        .expected
                        .get("expected_min_len")
                        .and_then(Value::as_u64)
                        .unwrap() as usize,
                    "{file_name}"
                );
            }
            continue;
        }

        // Donchian Channels fixture - validates rolling high/low bands
        if file_name.starts_with("spec_donchian_bands") {
            use liq_ta::indicators::donchian::{donchian, donchian_lookback, donchian_min_len};
            let input = fixture.input.as_object().expect("Expected input object");
            let high = parse_vec_f64(input.get("high").expect("Missing high"));
            let low = parse_vec_f64(input.get("low").expect("Missing low"));
            let period = fixture
                .params
                .get("period")
                .and_then(Value::as_u64)
                .expect("Missing period") as usize;

            let result = donchian(&high, &low, period).expect("Donchian failed");

            // Check specific indices from expected
            let expected_obj = fixture.expected.as_object().expect("Expected object");
            for (key, val) in expected_obj {
                if key.starts_with("at_index_") {
                    let idx: usize = key.strip_prefix("at_index_").unwrap().parse().unwrap();
                    let expected_vals = val.as_object().unwrap();
                    let exp_upper = expected_vals.get("upper").and_then(Value::as_f64).unwrap();
                    let exp_lower = expected_vals.get("lower").and_then(Value::as_f64).unwrap();
                    let exp_middle = expected_vals.get("middle").and_then(Value::as_f64).unwrap();

                    assert!(
                        approx_eq(result.upper[idx], exp_upper, EPSILON),
                        "{file_name}: upper[{idx}] expected {exp_upper}, got {}",
                        result.upper[idx]
                    );
                    assert!(
                        approx_eq(result.lower[idx], exp_lower, EPSILON),
                        "{file_name}: lower[{idx}] expected {exp_lower}, got {}",
                        result.lower[idx]
                    );
                    assert!(
                        approx_eq(result.middle[idx], exp_middle, EPSILON),
                        "{file_name}: middle[{idx}] expected {exp_middle}, got {}",
                        result.middle[idx]
                    );
                }
            }

            // Verify lookback
            assert_eq!(
                donchian_lookback(period),
                period - 1,
                "{file_name}: lookback"
            );
            assert_eq!(donchian_min_len(period), period, "{file_name}: min_len");
            continue;
        }

        // Williams %R extremes - validate boundary conditions
        if file_name.starts_with("spec_williams_r_extremes_") {
            use liq_ta::indicators::williams_r::williams_r;
            let input = fixture.input.as_object().expect("Expected input object");
            let high = parse_vec_f64(input.get("high").expect("Missing high"));
            let low = parse_vec_f64(input.get("low").expect("Missing low"));
            let close = parse_vec_f64(input.get("close").expect("Missing close"));
            let period = fixture
                .params
                .get("period")
                .and_then(Value::as_u64)
                .expect("Missing period") as usize;

            let result = williams_r(&high, &low, &close, period).expect("Williams %R failed");

            // Check expected value at specific index
            if let Some(exp_val) = fixture.expected.get("williams_r_at_index_2") {
                let expected = exp_val.as_f64().expect("Expected f64");
                assert!(
                    approx_eq(result[2], expected, EPSILON),
                    "{file_name}: williams_r[2] expected {expected}, got {}",
                    result[2]
                );
            }
            continue;
        }

        // OBV direction - validate volume flow calculation
        if file_name.starts_with("spec_obv_direction_") {
            use liq_ta::indicators::obv::obv;
            let input = fixture.input.as_object().expect("Expected input object");
            let close = parse_vec_f64(input.get("close").expect("Missing close"));
            let volume = parse_vec_f64(input.get("volume").expect("Missing volume"));

            let result = obv(&close, &volume).expect("OBV failed");
            let expected_obv = parse_vec_f64(fixture.expected.get("obv").expect("Missing obv"));

            for (i, (actual, expected)) in result.iter().zip(expected_obv.iter()).enumerate() {
                assert!(
                    approx_eq(*actual, *expected, EPSILON),
                    "{file_name}: obv[{i}] expected {expected}, got {actual}"
                );
            }
            continue;
        }

        // VWAP cumulative - validate cumulative calculation
        if file_name.starts_with("spec_vwap_") {
            use liq_ta::indicators::vwap::{vwap, vwap_lookback, vwap_min_len};
            let input = fixture.input.as_object().expect("Expected input object");
            let high = parse_vec_f64(input.get("high").expect("Missing high"));
            let low = parse_vec_f64(input.get("low").expect("Missing low"));
            let close = parse_vec_f64(input.get("close").expect("Missing close"));
            let volume = parse_vec_f64(input.get("volume").expect("Missing volume"));

            let result = vwap(&high, &low, &close, &volume).expect("VWAP failed");
            let expected_vwap = parse_vec_f64(fixture.expected.get("vwap").expect("Missing vwap"));

            for (i, (actual, expected)) in result.iter().zip(expected_vwap.iter()).enumerate() {
                assert!(
                    approx_eq(*actual, *expected, LOOSE_EPSILON),
                    "{file_name}: vwap[{i}] expected {expected}, got {actual}"
                );
            }

            // Verify lookback
            assert_eq!(vwap_lookback(), 0, "{file_name}: lookback");
            assert_eq!(vwap_min_len(), 1, "{file_name}: min_len");
            continue;
        }

        // ADX directional movement - uses test_cases format, skip in json_fixture_tests
        // (covered by spec_fixture_tests.rs)
        if file_name.starts_with("spec_adx_") {
            continue;
        }

        panic!(
            "Unhandled fixture file: {file_name} ({} )",
            fixture.rationale
        );
    }
}
