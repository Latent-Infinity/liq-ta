//! TA-Lib Reference Comparison Tests
//!
//! These tests compare liq-ta outputs to TA-Lib reference values stored in golden files.
//! Per PRD §7.4, these are informational only and do not block releases.
//!
//! Use `--features reference-checks-strict` to treat divergences as test failures.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::manual_let_else)]

use liq_ta::indicators::{
    atr::atr, bollinger::bollinger, ema::ema, macd::macd, rsi::rsi, sma::sma,
    stochastic::stochastic_fast,
};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const EPSILON: f64 = 1e-10;
const LOOSE_EPSILON: f64 = 1e-6;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    (a - b).abs() < eps
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// Report a reference divergence. Fails test only with reference-checks-strict feature.
macro_rules! reference_divergence {
    ($($arg:tt)*) => {
        #[cfg(feature = "reference-checks-strict")]
        panic!($($arg)*);
        #[cfg(not(feature = "reference-checks-strict"))]
        {
            eprintln!("[WARN] {}", format!($($arg)*));
            return;
        }
    };
}

#[derive(Debug, Deserialize)]
struct GoldenCase {
    input: Value,
    params: Value,
    expected: Value,
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

fn compare_vec(
    indicator: &str,
    case_name: &str,
    actual: &[f64],
    expected: &[Option<f64>],
    tolerance: f64,
) {
    if actual.len() != expected.len() {
        reference_divergence!(
            "{indicator}/{case_name}: length mismatch - actual {} vs expected {}",
            actual.len(),
            expected.len()
        );
    }

    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        match e {
            None => {
                if !a.is_nan() {
                    reference_divergence!(
                        "{indicator}/{case_name}: index {i} expected NaN, got {a}"
                    );
                }
            }
            Some(exp) => {
                if !approx_eq(*a, *exp, tolerance) {
                    reference_divergence!(
                        "{indicator}/{case_name}: index {i} expected {exp}, got {a} (diff: {})",
                        (a - exp).abs()
                    );
                }
            }
        }
    }
}

fn load_golden_case(path: &Path) -> GoldenCase {
    let content = fs::read_to_string(path).expect("Failed to read golden file");
    serde_json::from_str(&content).expect("Failed to parse golden file")
}

#[test]
fn reference_golden_files() {
    let dir = golden_dir();
    let entries = fs::read_dir(&dir).expect("Failed to read golden directory");

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // Skip metadata and *_reference.json files (they have test_cases format, not direct input/params/expected)
        if file_name == "metadata.json" || file_name.ends_with("_reference.json") {
            continue;
        }

        let indicator = if let Some((prefix, _)) = file_name.split_once('_') {
            prefix
        } else {
            continue;
        };

        if !matches!(
            indicator,
            "sma" | "ema" | "rsi" | "macd" | "atr" | "bollinger" | "stochastic"
        ) {
            continue;
        }

        let case = load_golden_case(&path);

        match indicator {
            "sma" => {
                let input = parse_input_series(&case.input);
                let period = case
                    .params
                    .get("period")
                    .and_then(Value::as_u64)
                    .expect("Missing period") as usize;
                let expected = parse_opt_vec_f64(&case.expected);
                let result = sma(&input, period).expect("SMA failed");
                compare_vec("SMA", file_name, &result, &expected, EPSILON);
            }
            "ema" => {
                let input = parse_input_series(&case.input);
                let period = case
                    .params
                    .get("period")
                    .and_then(Value::as_u64)
                    .expect("Missing period") as usize;
                let expected = parse_opt_vec_f64(&case.expected);
                let result = ema(&input, period).expect("EMA failed");
                compare_vec("EMA", file_name, &result, &expected, EPSILON);
            }
            "rsi" => {
                let input = parse_input_series(&case.input);
                let period = case
                    .params
                    .get("period")
                    .and_then(Value::as_u64)
                    .expect("Missing period") as usize;
                let expected = parse_opt_vec_f64(&case.expected);
                let result = rsi(&input, period).expect("RSI failed");
                compare_vec("RSI", file_name, &result, &expected, LOOSE_EPSILON);
            }
            "bollinger" => {
                let input = parse_input_series(&case.input);
                let period = case
                    .params
                    .get("period")
                    .and_then(Value::as_u64)
                    .expect("Missing period") as usize;
                let num_std_dev = case
                    .params
                    .get("num_std_dev")
                    .and_then(Value::as_f64)
                    .expect("Missing num_std_dev");
                let expected = case.expected.as_object().expect("Expected object");
                let middle = parse_opt_vec_f64(expected.get("middle").expect("Missing middle"));
                let upper = parse_opt_vec_f64(expected.get("upper").expect("Missing upper"));
                let lower = parse_opt_vec_f64(expected.get("lower").expect("Missing lower"));
                let result = bollinger(&input, period, num_std_dev).expect("Bollinger failed");
                compare_vec(
                    "BOLL",
                    &format!("{file_name}:middle"),
                    &result.middle,
                    &middle,
                    EPSILON,
                );
                compare_vec(
                    "BOLL",
                    &format!("{file_name}:upper"),
                    &result.upper,
                    &upper,
                    EPSILON,
                );
                compare_vec(
                    "BOLL",
                    &format!("{file_name}:lower"),
                    &result.lower,
                    &lower,
                    EPSILON,
                );
            }
            "macd" => {
                let input = parse_input_series(&case.input);
                let fast = case
                    .params
                    .get("fast_period")
                    .and_then(Value::as_u64)
                    .expect("Missing fast_period") as usize;
                let slow = case
                    .params
                    .get("slow_period")
                    .and_then(Value::as_u64)
                    .expect("Missing slow_period") as usize;
                let signal = case
                    .params
                    .get("signal_period")
                    .and_then(Value::as_u64)
                    .expect("Missing signal_period") as usize;
                let expected = case.expected.as_object().expect("Expected object");
                let macd_line =
                    parse_opt_vec_f64(expected.get("macd_line").expect("Missing macd_line"));
                let signal_line =
                    parse_opt_vec_f64(expected.get("signal_line").expect("Missing signal_line"));
                let histogram =
                    parse_opt_vec_f64(expected.get("histogram").expect("Missing histogram"));
                let result = macd(&input, fast, slow, signal).expect("MACD failed");
                compare_vec(
                    "MACD",
                    &format!("{file_name}:macd"),
                    &result.macd_line,
                    &macd_line,
                    EPSILON,
                );
                compare_vec(
                    "MACD",
                    &format!("{file_name}:signal"),
                    &result.signal_line,
                    &signal_line,
                    EPSILON,
                );
                compare_vec(
                    "MACD",
                    &format!("{file_name}:hist"),
                    &result.histogram,
                    &histogram,
                    EPSILON,
                );
            }
            "atr" => {
                let (high, low, close) = parse_input_ohlc(&case.input);
                let period = case
                    .params
                    .get("period")
                    .and_then(Value::as_u64)
                    .expect("Missing period") as usize;
                let expected = parse_opt_vec_f64(&case.expected);
                let result = atr(&high, &low, &close, period).expect("ATR failed");
                compare_vec("ATR", file_name, &result, &expected, EPSILON);
            }
            "stochastic" => {
                let (high, low, close) = parse_input_ohlc(&case.input);
                let k_period = case
                    .params
                    .get("k_period")
                    .and_then(Value::as_u64)
                    .expect("Missing k_period") as usize;
                let d_period = case
                    .params
                    .get("d_period")
                    .and_then(Value::as_u64)
                    .expect("Missing d_period") as usize;
                let expected = parse_opt_vec_f64(&case.expected);
                let result = stochastic_fast(&high, &low, &close, k_period, d_period)
                    .expect("Stochastic failed");
                compare_vec("STOCH", file_name, &result.k, &expected, LOOSE_EPSILON);
            }
            _ => {}
        }
    }
}
