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

        if file_name.starts_with("spec_keltner_") {
            use liq_ta::indicators::keltner::keltner_channel;
            let (high, low, close) = parse_input_ohlc(&fixture.input);
            let period = fixture
                .params
                .get("period")
                .and_then(Value::as_u64)
                .expect("Missing period") as usize;
            let atr_multiplier = fixture
                .params
                .get("atr_multiplier")
                .and_then(Value::as_f64)
                .expect("Missing atr_multiplier");

            let result = keltner_channel(&high, &low, &close, period, atr_multiplier)
                .expect("Keltner failed");

            let first_valid = fixture
                .expected
                .get("expected_first_valid_index")
                .and_then(Value::as_u64)
                .expect("Missing expected_first_valid_index")
                as usize;

            for i in 0..result.middle.len() {
                if i < first_valid {
                    assert!(
                        result.upper[i].is_nan()
                            && result.middle[i].is_nan()
                            && result.lower[i].is_nan(),
                        "{file_name}[{i}] expected NaN lookback region"
                    );
                } else if result.upper[i].is_finite()
                    && result.middle[i].is_finite()
                    && result.lower[i].is_finite()
                {
                    assert!(
                        result.upper[i] >= result.middle[i] && result.middle[i] >= result.lower[i],
                        "{file_name}[{i}] expected upper >= middle >= lower"
                    );
                }
            }
            continue;
        }

        if file_name.starts_with("spec_ichimoku_") {
            use liq_ta::indicators::ichimoku::ichimoku;
            let (high, low, close) = parse_input_ohlc(&fixture.input);
            let tenkan_period = fixture
                .params
                .get("tenkan_period")
                .and_then(Value::as_u64)
                .expect("Missing tenkan_period") as usize;
            let kijun_period = fixture
                .params
                .get("kijun_period")
                .and_then(Value::as_u64)
                .expect("Missing kijun_period") as usize;
            let senkou_b_period = fixture
                .params
                .get("senkou_b_period")
                .and_then(Value::as_u64)
                .expect("Missing senkou_b_period") as usize;
            let displacement = fixture
                .params
                .get("displacement")
                .and_then(Value::as_u64)
                .expect("Missing displacement") as usize;

            let result = ichimoku(
                &high,
                &low,
                &close,
                tenkan_period,
                kijun_period,
                senkou_b_period,
                displacement,
            )
            .expect("Ichimoku failed");

            let tenkan_first = fixture
                .expected
                .get("expected_tenkan_first_valid_index")
                .and_then(Value::as_u64)
                .expect("Missing expected_tenkan_first_valid_index")
                as usize;
            let kijun_first = fixture
                .expected
                .get("expected_kijun_first_valid_index")
                .and_then(Value::as_u64)
                .expect("Missing expected_kijun_first_valid_index")
                as usize;
            let senkou_first = fixture
                .expected
                .get("expected_senkou_first_valid_index")
                .and_then(Value::as_u64)
                .expect("Missing expected_senkou_first_valid_index")
                as usize;

            assert!(result.tenkan[..tenkan_first].iter().all(|v| v.is_nan()));
            assert!(result.kijun[..kijun_first].iter().all(|v| v.is_nan()));
            assert!(result.senkou_b[..senkou_first].iter().all(|v| v.is_nan()));

            if displacement > 0 {
                let start = result.chikou.len().saturating_sub(displacement);
                let tail = &result.chikou[start..];
                assert!(
                    tail.iter().all(|v| v.is_nan()),
                    "{file_name}: trailing chikou values should be NaN"
                );
            }
            continue;
        }

        if file_name.starts_with("spec_qqe_") {
            use liq_ta::indicators::qqe::qqe;
            let input = parse_input_series(&fixture.input);
            let rsi_period = fixture
                .params
                .get("rsi_period")
                .and_then(Value::as_u64)
                .expect("Missing rsi_period") as usize;
            let smoothing_period = fixture
                .params
                .get("smoothing_period")
                .and_then(Value::as_u64)
                .expect("Missing smoothing_period") as usize;
            let wilders_period = fixture
                .params
                .get("wilders_period")
                .and_then(Value::as_u64)
                .expect("Missing wilders_period") as usize;
            let factor = fixture
                .params
                .get("factor")
                .and_then(Value::as_f64)
                .expect("Missing factor");

            let result = qqe(&input, rsi_period, smoothing_period, wilders_period, factor)
                .expect("QQE failed");

            let first_valid = fixture
                .expected
                .get("expected_first_valid_index")
                .and_then(Value::as_u64)
                .expect("Missing expected_first_valid_index")
                as usize;

            assert!(
                result.upper_band[..first_valid].iter().all(|v| v.is_nan())
                    && result.lower_band[..first_valid].iter().all(|v| v.is_nan()),
                "{file_name}: expected QQE bands to remain NaN before first valid index"
            );
            for i in first_valid..result.qqe.len() {
                if result.upper_band[i].is_finite()
                    && result.qqe[i].is_finite()
                    && result.lower_band[i].is_finite()
                {
                    assert!(
                        result.upper_band[i] >= result.qqe[i]
                            && result.qqe[i] >= result.lower_band[i],
                        "{file_name}[{i}] expected upper >= qqe >= lower"
                    );
                }
            }
            continue;
        }

        if file_name.starts_with("spec_stage2_indicator_matrix_") {
            use liq_ta::indicators::{
                ao, autocorr, bears_power, bulls_power, chop, connors_rsi, demarker, dpo,
                dss_bressert, gaussian_channel, gaussian_filter, hma, hma_atr_bands,
                hma_bollinger_bands, hurst, laguerre_rsi, osma, rvi, stc, supertrend, ulcer_index,
                vortex, vwap_atr_bands, vwap_bollinger_bands,
            };

            let n = parse_length(&fixture.input);
            let base: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.2).collect();
            let open: Vec<f64> = base.iter().map(|v| v - 0.1).collect();
            let high: Vec<f64> = base.iter().map(|v| v + 0.8).collect();
            let low: Vec<f64> = base.iter().map(|v| v - 0.8).collect();
            let close: Vec<f64> = base.iter().map(|v| v + 0.2).collect();
            let volume: Vec<f64> = (0..n).map(|i| 1_000.0 + (i % 20) as f64 * 25.0).collect();

            assert_eq!(hma(&close, 21).expect("HMA failed").len(), n, "{file_name}");
            assert_eq!(
                gaussian_filter(&close, 20, 0.5)
                    .expect("Gaussian Filter failed")
                    .len(),
                n,
                "{file_name}"
            );
            assert_eq!(ao(&high, &low).expect("AO failed").len(), n, "{file_name}");
            assert_eq!(
                bulls_power(&high, &low, &close, 13)
                    .expect("Bulls Power failed")
                    .len(),
                n,
                "{file_name}"
            );
            assert_eq!(
                bears_power(&high, &low, &close, 13)
                    .expect("Bears Power failed")
                    .len(),
                n,
                "{file_name}"
            );
            assert_eq!(
                demarker(&high, &low, 14).expect("DeMarker failed").len(),
                n,
                "{file_name}"
            );
            assert_eq!(
                osma(&close, 12, 26, 9).expect("OSMA failed").len(),
                n,
                "{file_name}"
            );
            assert_eq!(
                rvi(&open, &high, &low, &close, 10)
                    .expect("RVI failed")
                    .len(),
                n,
                "{file_name}"
            );
            assert_eq!(dpo(&close, 20).expect("DPO failed").len(), n, "{file_name}");
            assert_eq!(
                connors_rsi(&close, 3, 2, 100)
                    .expect("Connors RSI failed")
                    .len(),
                n,
                "{file_name}"
            );
            assert_eq!(
                stc(&close, 23, 50, 10, 3).expect("STC failed").len(),
                n,
                "{file_name}"
            );
            assert_eq!(
                laguerre_rsi(&close, 0.5)
                    .expect("Laguerre RSI failed")
                    .len(),
                n,
                "{file_name}"
            );
            assert_eq!(
                dss_bressert(&high, &low, &close, 14, 5)
                    .expect("DSS Bressert failed")
                    .len(),
                n,
                "{file_name}"
            );
            assert_eq!(
                chop(&high, &low, &close, 14).expect("CHOP failed").len(),
                n,
                "{file_name}"
            );
            assert_eq!(
                ulcer_index(&close, 14).expect("Ulcer Index failed").len(),
                n,
                "{file_name}"
            );
            assert_eq!(
                hurst(&close, 64).expect("Hurst failed").len(),
                n,
                "{file_name}"
            );
            assert_eq!(
                autocorr(&close, 32, 1)
                    .expect("Autocorrelation failed")
                    .len(),
                n,
                "{file_name}"
            );

            let vortex_out = vortex(&high, &low, &close, 14).expect("Vortex failed");
            assert_eq!(vortex_out.plus_vi.len(), n, "{file_name}");
            assert_eq!(vortex_out.minus_vi.len(), n, "{file_name}");

            let supertrend_out =
                supertrend(&high, &low, &close, 10, 3.0).expect("SuperTrend failed");
            assert_eq!(supertrend_out.supertrend.len(), n, "{file_name}");
            assert_eq!(supertrend_out.upper_band.len(), n, "{file_name}");
            assert_eq!(supertrend_out.lower_band.len(), n, "{file_name}");
            assert_eq!(supertrend_out.trend.len(), n, "{file_name}");

            let gaussian_out =
                gaussian_channel(&close, 20, 0.5, 2.0).expect("Gaussian Channel failed");
            assert_eq!(gaussian_out.center.len(), n, "{file_name}");
            assert_eq!(gaussian_out.upper.len(), n, "{file_name}");
            assert_eq!(gaussian_out.lower.len(), n, "{file_name}");
            assert_eq!(gaussian_out.trend.len(), n, "{file_name}");

            let hma_atr_out =
                hma_atr_bands(&high, &low, &close, 21, 14, 2.0).expect("HMA ATR bands failed");
            assert_eq!(hma_atr_out.upper.len(), n, "{file_name}");
            assert_eq!(hma_atr_out.middle.len(), n, "{file_name}");
            assert_eq!(hma_atr_out.lower.len(), n, "{file_name}");

            let hma_bollinger_out =
                hma_bollinger_bands(&close, 21, 20, 2.0).expect("HMA Bollinger bands failed");
            assert_eq!(hma_bollinger_out.upper.len(), n, "{file_name}");
            assert_eq!(hma_bollinger_out.middle.len(), n, "{file_name}");
            assert_eq!(hma_bollinger_out.lower.len(), n, "{file_name}");

            let vwap_atr_out = vwap_atr_bands(&high, &low, &close, &volume, 14, 2.0)
                .expect("VWAP ATR bands failed");
            assert_eq!(vwap_atr_out.upper.len(), n, "{file_name}");
            assert_eq!(vwap_atr_out.middle.len(), n, "{file_name}");
            assert_eq!(vwap_atr_out.lower.len(), n, "{file_name}");

            let vwap_bollinger_out = vwap_bollinger_bands(&high, &low, &close, &volume, 20, 2.0)
                .expect("VWAP Bollinger bands failed");
            assert_eq!(vwap_bollinger_out.upper.len(), n, "{file_name}");
            assert_eq!(vwap_bollinger_out.middle.len(), n, "{file_name}");
            assert_eq!(vwap_bollinger_out.lower.len(), n, "{file_name}");
            continue;
        }

        if file_name.starts_with("spec_gaussian_channel_") {
            use liq_ta::indicators::gaussian_channel::{
                gaussian_channel, gaussian_channel_lookback,
            };
            let n = parse_length(&fixture.input);
            let period = fixture
                .params
                .get("period")
                .and_then(Value::as_u64)
                .expect("Missing period") as usize;
            let sigma = fixture
                .params
                .get("sigma")
                .and_then(Value::as_f64)
                .expect("Missing sigma");
            let multiplier = fixture
                .params
                .get("multiplier")
                .and_then(Value::as_f64)
                .expect("Missing multiplier");
            let scenario = fixture
                .params
                .get("scenario")
                .and_then(Value::as_str)
                .expect("Missing scenario");

            let data: Vec<f64> = match scenario {
                "bullish" => (0..n).map(|i| 100.0 + i as f64 * 0.35).collect(),
                "bearish" => (0..n).map(|i| 180.0 - i as f64 * 0.35).collect(),
                "transition" => {
                    let pivot = n / 2;
                    (0..n)
                        .map(|i| {
                            if i < pivot {
                                160.0 - i as f64 * 0.45
                            } else {
                                100.0 + (i - pivot) as f64 * 0.55
                            }
                        })
                        .collect()
                }
                other => panic!("{file_name}: unsupported scenario '{other}'"),
            };

            let out = gaussian_channel(&data, period, sigma, multiplier)
                .expect("Gaussian Channel failed");
            let lookback = gaussian_channel_lookback(period);
            for i in 0..lookback.min(n) {
                assert!(
                    out.center[i].is_nan()
                        && out.upper[i].is_nan()
                        && out.lower[i].is_nan()
                        && out.trend[i].is_nan(),
                    "{file_name}[{i}] expected NaN lookback region"
                );
            }

            let mut finite_trend = Vec::new();
            for i in lookback..n {
                if out.center[i].is_finite() && out.upper[i].is_finite() && out.lower[i].is_finite()
                {
                    assert!(
                        out.upper[i] >= out.center[i] && out.center[i] >= out.lower[i],
                        "{file_name}[{i}] expected upper >= center >= lower"
                    );
                }
                if out.trend[i].is_finite() {
                    finite_trend.push(out.trend[i]);
                }
            }
            assert!(
                !finite_trend.is_empty(),
                "{file_name}: expected at least one finite trend value"
            );

            if let Some(expected_final) = fixture
                .expected
                .get("expected_final_trend")
                .and_then(Value::as_f64)
            {
                let actual_final = *finite_trend.last().expect("non-empty trend");
                assert!(
                    approx_eq(actual_final, expected_final, EPSILON),
                    "{file_name}: expected final trend {expected_final}, got {actual_final}"
                );
            }

            if fixture
                .expected
                .get("expect_transition")
                .and_then(Value::as_bool)
                == Some(true)
            {
                let transitions = finite_trend
                    .windows(2)
                    .filter(|pair| {
                        pair[0] != 0.0 && pair[1] != 0.0 && pair[0].signum() != pair[1].signum()
                    })
                    .count();
                assert!(
                    transitions > 0,
                    "{file_name}: expected at least one bullish/bearish transition"
                );
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
                } else if file_name.contains("keltner") {
                    use liq_ta::indicators::keltner::{
                        keltner_channel_lookback, keltner_channel_min_len,
                    };
                    assert_eq!(
                        keltner_channel_lookback(period),
                        fixture
                            .expected
                            .get("expected_lookback")
                            .and_then(Value::as_u64)
                            .unwrap() as usize,
                        "{file_name}"
                    );
                    assert_eq!(
                        keltner_channel_min_len(period),
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
            } else if file_name.contains("ichimoku") {
                use liq_ta::indicators::ichimoku::{ichimoku_lookback, ichimoku_min_len};
                let tenkan = fixture
                    .params
                    .get("tenkan_period")
                    .and_then(Value::as_u64)
                    .unwrap() as usize;
                let kijun = fixture
                    .params
                    .get("kijun_period")
                    .and_then(Value::as_u64)
                    .unwrap() as usize;
                let senkou_b = fixture
                    .params
                    .get("senkou_b_period")
                    .and_then(Value::as_u64)
                    .unwrap() as usize;
                assert_eq!(
                    ichimoku_lookback(tenkan, kijun, senkou_b),
                    fixture
                        .expected
                        .get("expected_lookback")
                        .and_then(Value::as_u64)
                        .unwrap() as usize,
                    "{file_name}"
                );
                assert_eq!(
                    ichimoku_min_len(tenkan, kijun, senkou_b),
                    fixture
                        .expected
                        .get("expected_min_len")
                        .and_then(Value::as_u64)
                        .unwrap() as usize,
                    "{file_name}"
                );
            } else if file_name.contains("qqe") {
                use liq_ta::indicators::qqe::{qqe_lookback, qqe_min_len};
                let rsi_period = fixture
                    .params
                    .get("rsi_period")
                    .and_then(Value::as_u64)
                    .unwrap() as usize;
                let smoothing_period = fixture
                    .params
                    .get("smoothing_period")
                    .and_then(Value::as_u64)
                    .unwrap() as usize;
                let wilders_period = fixture
                    .params
                    .get("wilders_period")
                    .and_then(Value::as_u64)
                    .unwrap() as usize;
                assert_eq!(
                    qqe_lookback(rsi_period, smoothing_period, wilders_period),
                    fixture
                        .expected
                        .get("expected_lookback")
                        .and_then(Value::as_u64)
                        .unwrap() as usize,
                    "{file_name}"
                );
                assert_eq!(
                    qqe_min_len(rsi_period, smoothing_period, wilders_period),
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
