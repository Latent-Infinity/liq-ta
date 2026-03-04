//! Registry declarations used by the new Python binding foundation.
//!
//! This keeps indicator onboarding metadata in one place and makes pilot
//! indicators explicit before we add more binding entry points.

#[derive(Debug)]
pub struct IndicatorBindingDescriptor {
    pub name: &'static str,
    pub category: &'static str,
    pub input_shape: &'static str,
    pub inputs: &'static [&'static str],
    pub params: &'static [&'static str],
    pub outputs: &'static [&'static str],
    pub supports_out: bool,
    pub callable_target: &'static str,
}

pub const FOUNDATION_INDICATORS: &[IndicatorBindingDescriptor] = &[
    IndicatorBindingDescriptor {
        name: "sma",
        category: "moving_average",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["period"],
        outputs: &["sma"],
        supports_out: true,
        callable_target: "sma",
    },
    IndicatorBindingDescriptor {
        name: "ema",
        category: "moving_average",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["period"],
        outputs: &["ema"],
        supports_out: true,
        callable_target: "ema",
    },
    IndicatorBindingDescriptor {
        name: "ema_wilder",
        category: "moving_average",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["period"],
        outputs: &["ema_wilder"],
        supports_out: true,
        callable_target: "ema_wilder",
    },
    IndicatorBindingDescriptor {
        name: "rsi",
        category: "momentum",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["period"],
        outputs: &["rsi"],
        supports_out: true,
        callable_target: "rsi",
    },
    IndicatorBindingDescriptor {
        name: "macd",
        category: "momentum",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["fast_period", "slow_period", "signal_period"],
        outputs: &["macd_line", "signal_line", "histogram"],
        supports_out: false,
        callable_target: "macd",
    },
    IndicatorBindingDescriptor {
        name: "keltner_channel",
        category: "volatility",
        input_shape: "OHLC",
        inputs: &["high", "low", "close"],
        params: &["period", "atr_multiplier"],
        outputs: &["upper", "middle", "lower"],
        supports_out: false,
        callable_target: "keltner_channel",
    },
    IndicatorBindingDescriptor {
        name: "ichimoku",
        category: "trend",
        input_shape: "OHLC",
        inputs: &["high", "low", "close"],
        params: &[
            "tenkan_period",
            "kijun_period",
            "senkou_b_period",
            "displacement",
        ],
        outputs: &["tenkan", "kijun", "senkou_a", "senkou_b", "chikou"],
        supports_out: false,
        callable_target: "ichimoku",
    },
    IndicatorBindingDescriptor {
        name: "qqe",
        category: "momentum",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["rsi_period", "smoothing_period", "wilders_period", "factor"],
        outputs: &["qqe", "upper_band", "lower_band"],
        supports_out: false,
        callable_target: "qqe",
    },
    IndicatorBindingDescriptor {
        name: "hma",
        category: "trend",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["period"],
        outputs: &["hma"],
        supports_out: true,
        callable_target: "hma",
    },
    IndicatorBindingDescriptor {
        name: "gaussian_filter",
        category: "trend",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["period", "sigma"],
        outputs: &["gaussian_filter"],
        supports_out: true,
        callable_target: "gaussian_filter",
    },
    IndicatorBindingDescriptor {
        name: "gaussian_channel",
        category: "volatility",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["period", "sigma", "multiplier"],
        outputs: &["center", "upper", "lower", "trend"],
        supports_out: false,
        callable_target: "gaussian_channel",
    },
    IndicatorBindingDescriptor {
        name: "supertrend",
        category: "trend",
        input_shape: "OHLC",
        inputs: &["high", "low", "close"],
        params: &["period", "multiplier"],
        outputs: &["supertrend", "upper_band", "lower_band", "trend"],
        supports_out: false,
        callable_target: "supertrend",
    },
    IndicatorBindingDescriptor {
        name: "ao",
        category: "momentum",
        input_shape: "OHLC",
        inputs: &["high", "low"],
        params: &[],
        outputs: &["ao"],
        supports_out: false,
        callable_target: "ao",
    },
    IndicatorBindingDescriptor {
        name: "bulls_power",
        category: "momentum",
        input_shape: "OHLC",
        inputs: &["high", "low", "close"],
        params: &["period"],
        outputs: &["bulls_power"],
        supports_out: true,
        callable_target: "bulls_power",
    },
    IndicatorBindingDescriptor {
        name: "bears_power",
        category: "momentum",
        input_shape: "OHLC",
        inputs: &["high", "low", "close"],
        params: &["period"],
        outputs: &["bears_power"],
        supports_out: true,
        callable_target: "bears_power",
    },
    IndicatorBindingDescriptor {
        name: "demarker",
        category: "momentum",
        input_shape: "OHLC",
        inputs: &["high", "low"],
        params: &["period"],
        outputs: &["demarker"],
        supports_out: true,
        callable_target: "demarker",
    },
    IndicatorBindingDescriptor {
        name: "osma",
        category: "momentum",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["fast_period", "slow_period", "signal_period"],
        outputs: &["osma"],
        supports_out: true,
        callable_target: "osma",
    },
    IndicatorBindingDescriptor {
        name: "vortex",
        category: "trend",
        input_shape: "OHLC",
        inputs: &["high", "low", "close"],
        params: &["period"],
        outputs: &["plus_vi", "minus_vi"],
        supports_out: false,
        callable_target: "vortex",
    },
    IndicatorBindingDescriptor {
        name: "rvi",
        category: "momentum",
        input_shape: "OHLC",
        inputs: &["open", "high", "low", "close"],
        params: &["period"],
        outputs: &["rvi"],
        supports_out: true,
        callable_target: "rvi",
    },
    IndicatorBindingDescriptor {
        name: "dpo",
        category: "momentum",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["period"],
        outputs: &["dpo"],
        supports_out: true,
        callable_target: "dpo",
    },
    IndicatorBindingDescriptor {
        name: "connors_rsi",
        category: "momentum",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["rsi_period", "streak_period", "rank_period"],
        outputs: &["connors_rsi"],
        supports_out: true,
        callable_target: "connors_rsi",
    },
    IndicatorBindingDescriptor {
        name: "stc",
        category: "momentum",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &[
            "fast_period",
            "slow_period",
            "cycle_period",
            "smooth_period",
        ],
        outputs: &["stc"],
        supports_out: true,
        callable_target: "stc",
    },
    IndicatorBindingDescriptor {
        name: "laguerre_rsi",
        category: "momentum",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["gamma"],
        outputs: &["laguerre_rsi"],
        supports_out: true,
        callable_target: "laguerre_rsi",
    },
    IndicatorBindingDescriptor {
        name: "dss_bressert",
        category: "momentum",
        input_shape: "OHLC",
        inputs: &["high", "low", "close"],
        params: &["stochastic_period", "ema_period"],
        outputs: &["dss_bressert"],
        supports_out: true,
        callable_target: "dss_bressert",
    },
    IndicatorBindingDescriptor {
        name: "chop",
        category: "volatility",
        input_shape: "OHLC",
        inputs: &["high", "low", "close"],
        params: &["period"],
        outputs: &["chop"],
        supports_out: true,
        callable_target: "chop",
    },
    IndicatorBindingDescriptor {
        name: "ulcer_index",
        category: "volatility",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["period"],
        outputs: &["ulcer_index"],
        supports_out: true,
        callable_target: "ulcer_index",
    },
    IndicatorBindingDescriptor {
        name: "hurst",
        category: "regime",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["period"],
        outputs: &["hurst"],
        supports_out: true,
        callable_target: "hurst",
    },
    IndicatorBindingDescriptor {
        name: "autocorr",
        category: "regime",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["period", "lag"],
        outputs: &["autocorr"],
        supports_out: true,
        callable_target: "autocorr",
    },
    IndicatorBindingDescriptor {
        name: "hma_atr_bands",
        category: "volatility",
        input_shape: "OHLC",
        inputs: &["high", "low", "close"],
        params: &["hma_period", "atr_period", "atr_multiplier"],
        outputs: &["upper", "middle", "lower"],
        supports_out: false,
        callable_target: "hma_atr_bands",
    },
    IndicatorBindingDescriptor {
        name: "hma_bollinger_bands",
        category: "volatility",
        input_shape: "Series<f64>",
        inputs: &["data"],
        params: &["hma_period", "std_period", "std_multiplier"],
        outputs: &["upper", "middle", "lower"],
        supports_out: false,
        callable_target: "hma_bollinger_bands",
    },
    IndicatorBindingDescriptor {
        name: "vwap_atr_bands",
        category: "volatility",
        input_shape: "OHLCV",
        inputs: &["high", "low", "close", "volume"],
        params: &["atr_period", "atr_multiplier"],
        outputs: &["upper", "middle", "lower"],
        supports_out: false,
        callable_target: "vwap_atr_bands",
    },
    IndicatorBindingDescriptor {
        name: "vwap_bollinger_bands",
        category: "volatility",
        input_shape: "OHLCV",
        inputs: &["high", "low", "close", "volume"],
        params: &["std_period", "std_multiplier"],
        outputs: &["upper", "middle", "lower"],
        supports_out: false,
        callable_target: "vwap_bollinger_bands",
    },
];

pub fn entries() -> &'static [IndicatorBindingDescriptor] {
    FOUNDATION_INDICATORS
}

#[cfg(test)]
pub fn lookup(name: &str) -> Option<&'static IndicatorBindingDescriptor> {
    FOUNDATION_INDICATORS
        .iter()
        .find(|entry| entry.name == name)
}

pub fn validate_entries(entries: &[IndicatorBindingDescriptor]) -> Result<(), String> {
    let mut index = 0;
    while index < entries.len() {
        let entry = &entries[index];

        if entry.name.is_empty() {
            return Err("Indicator name must not be empty".to_string());
        }
        if entry.category.is_empty() {
            return Err(format!("Indicator '{}' missing category", entry.name));
        }
        if entry.inputs.is_empty() {
            return Err(format!(
                "Indicator '{}' must define at least one input",
                entry.name
            ));
        }
        if entry.outputs.is_empty() {
            return Err(format!(
                "Indicator '{}' must define at least one output",
                entry.name
            ));
        }
        if entry.input_shape != "Series<f64>"
            && entry.input_shape != "OHLC"
            && entry.input_shape != "OHLCV"
        {
            return Err(format!(
                "Indicator '{}' uses unsupported input_shape '{}'",
                entry.name, entry.input_shape
            ));
        }
        if entry.callable_target.is_empty() {
            return Err(format!(
                "Indicator '{}' missing callable target",
                entry.name
            ));
        }

        let mut inner = index + 1;
        while inner < entries.len() {
            if entries[inner].name == entry.name {
                return Err(format!(
                    "Duplicate indicator registration for name '{}'",
                    entry.name
                ));
            }
            inner += 1;
        }

        index += 1;
    }

    Ok(())
}

pub fn validate() -> Result<(), String> {
    validate_entries(FOUNDATION_INDICATORS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_entry(name: &'static str) -> IndicatorBindingDescriptor {
        IndicatorBindingDescriptor {
            name,
            category: "momentum",
            input_shape: "Series<f64>",
            inputs: &["data"],
            params: &["period"],
            outputs: &["value"],
            supports_out: true,
            callable_target: "callable",
        }
    }

    #[test]
    fn test_entries_returns_foundation_indicators() {
        assert_eq!(entries().len(), FOUNDATION_INDICATORS.len());
        let actual: Vec<&str> = entries().iter().map(|entry| entry.name).collect();
        let expected: Vec<&str> = FOUNDATION_INDICATORS
            .iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_validate_entries_rejects_empty_name() {
        let invalid = [IndicatorBindingDescriptor {
            name: "",
            ..base_entry("fallback")
        }];
        let err = validate_entries(&invalid).expect_err("empty name should fail");
        assert_eq!(err, "Indicator name must not be empty");
    }

    #[test]
    fn test_validate_entries_rejects_empty_category() {
        let invalid = [IndicatorBindingDescriptor {
            category: "",
            ..base_entry("missing_category")
        }];
        let err = validate_entries(&invalid).expect_err("empty category should fail");
        assert_eq!(err, "Indicator 'missing_category' missing category");
    }

    #[test]
    fn test_validate_entries_rejects_empty_inputs() {
        let invalid = [IndicatorBindingDescriptor {
            inputs: &[],
            ..base_entry("missing_inputs")
        }];
        let err = validate_entries(&invalid).expect_err("missing inputs should fail");
        assert_eq!(
            err,
            "Indicator 'missing_inputs' must define at least one input"
        );
    }

    #[test]
    fn test_validate_entries_rejects_empty_outputs() {
        let invalid = [IndicatorBindingDescriptor {
            outputs: &[],
            ..base_entry("missing_outputs")
        }];
        let err = validate_entries(&invalid).expect_err("missing outputs should fail");
        assert_eq!(
            err,
            "Indicator 'missing_outputs' must define at least one output"
        );
    }

    #[test]
    fn test_validate_entries_rejects_empty_callable_target() {
        let invalid = [IndicatorBindingDescriptor {
            callable_target: "",
            ..base_entry("missing_callable")
        }];
        let err = validate_entries(&invalid).expect_err("missing callable target should fail");
        assert_eq!(err, "Indicator 'missing_callable' missing callable target");
    }
}
