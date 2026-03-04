# Property Tests Coverage Audit Report

**Audit Date:** 2026-01-16
**Auditor:** auto-claude
**Subtask:** 4-1 (Property Tests Coverage Audit)
**Standard:** `docs/indicator-standards.md` - Quality Gates Section

## Executive Summary

The property tests in `crates/liq-ta/tests/property_tests.rs` provide coverage for **8 out of 20 benchmarked indicators** (40% coverage) and **8 out of 47 total indicator modules** (17% coverage).

**Coverage Status: PARTIAL - Coverage gaps identified for core indicators**

### Key Findings

| Metric | Value | Assessment |
|--------|-------|------------|
| Property tests file | `property_tests.rs` | Single file, well-organized |
| Total property tests | 21 | Good depth for covered indicators |
| Indicators with coverage | 8 | SMA, EMA, RSI, MACD, Bollinger, ATR/TR, Stochastic, Rolling Extrema |
| Benchmarked indicators without property tests | 12 | ADX, Williams %R, Donchian, OBV, VWAP, price transforms, AD, ROC, MFI, VAR |
| Average tests per covered indicator | 2.6 | Good property variety |

## Current Property Test Coverage

### Covered Indicators (8 categories, 21 tests)

| Indicator | Tests | Properties Verified |
|-----------|-------|---------------------|
| **SMA** | 3 | output_length, nan_count, constant_input |
| **EMA** | 3 | output_length, nan_count, constant_input |
| **RSI** | 4 | output_length, bounded [0,100], all_gains=100, all_losses=0 |
| **MACD** | 2 | output_length, histogram = macd_line - signal_line |
| **Bollinger** | 2 | band_order (upper >= middle >= lower), symmetric |
| **ATR + True Range** | 2 | atr_non_negative, true_range_non_negative |
| **Stochastic** | 2 | k_bounded [0,100], d_bounded [0,100] |
| **Rolling Extrema (kernels)** | 3 | max >= min, max >= current, min <= current |

### Test Quality Assessment

The existing property tests demonstrate **good practices**:

1. **Output shape invariants**: All verify `output.len() == input.len()`
2. **NaN prefix counting**: SMA, EMA verify exact NaN count = period - 1
3. **Value bounds**: RSI, Stochastic verify bounded outputs [0, 100]
4. **Mathematical relationships**: MACD histogram definition, Bollinger symmetry
5. **Monotonicity**: Rolling extrema relationships
6. **Edge case behavior**: RSI all-gains/all-losses, constant input for MA

## Coverage Gaps Analysis

### Benchmarked Indicators Without Property Tests (12 indicators)

These indicators have benchmarks in `indicators.rs` but lack property tests:

| Indicator | Priority | Suggested Properties |
|-----------|----------|---------------------|
| **ADX** | HIGH | output_length, bounded [0,100], non-negative |
| **Williams %R** | HIGH | output_length, bounded [-100,0] |
| **Donchian** | HIGH | high >= close >= low, output_length |
| **OBV** | MEDIUM | output_length, no NaN after first element |
| **VWAP** | MEDIUM | output_length, always positive |
| **avgprice** | MEDIUM | output_length, (O+H+L+C)/4 range |
| **medprice** | MEDIUM | output_length, (H+L)/2 range |
| **typprice** | MEDIUM | output_length, (H+L+C)/3 range |
| **wclprice** | MEDIUM | output_length, (H+L+2C)/4 range |
| **AD** | MEDIUM | output_length, cumulative nature |
| **ROC** | MEDIUM | output_length, nan_count = period |
| **MFI** | MEDIUM | output_length, bounded [0,100] |
| **VAR** | LOW | output_length, non_negative |

### Other Indicators Without Property Tests (27 modules)

| Category | Indicators | Priority |
|----------|------------|----------|
| **Moving Averages** | WMA, DEMA, TEMA, TRIMA, T3, KAMA | MEDIUM |
| **Momentum** | CMO, MOM, APO, StochRSI, TRIX | MEDIUM |
| **Directional** | DX, +DI, -DI, ADXR, Aroon | MEDIUM |
| **Volume** | ADOSC | LOW |
| **Price** | midpoint, midprice | LOW |
| **Hilbert** | HT_DCPERIOD, HT_DCPHASE, HT_PHASOR, HT_SINE, HT_TRENDLINE, HT_TRENDMODE | LOW |
| **SAR** | SAR, SAREXT | LOW |
| **Other** | CCI, BOP, ULTOSC, MAMA, MAVP | LOW |
| **Statistics** | beta, correl, linearreg family, tsf | LOW |

## Verification

```bash
# Run property tests (verification command)
cargo test -p liq-ta --test property_tests

# Count property tests
grep -c "fn prop_" crates/liq-ta/tests/property_tests.rs
# Result: 21 tests

# List covered indicators
grep "^use liq_ta" crates/liq-ta/tests/property_tests.rs
```

## Property Test Patterns Observed

### Pattern 1: Output Shape Properties
```rust
proptest! {
    fn prop_indicator_output_length(data in arb_price_series(5, 100), period in 1usize..=10) {
        let result = indicator(&data, period).unwrap();
        prop_assert_eq!(result.len(), data.len());
    }
}
```

### Pattern 2: NaN Count Properties
```rust
proptest! {
    fn prop_indicator_nan_count(data in arb_price_series(5, 100), period in 1usize..=10) {
        let result = indicator(&data, period).unwrap();
        let nan_count = result.iter().filter(|x| x.is_nan()).count();
        prop_assert_eq!(nan_count, indicator_lookback(period));
    }
}
```

### Pattern 3: Bounded Output Properties
```rust
proptest! {
    fn prop_indicator_bounded(data in arb_price_series(5, 100), period in 1usize..=10) {
        let result = indicator(&data, period).unwrap();
        for &val in result.iter().filter(|x| !x.is_nan()) {
            prop_assert!((LOWER..=UPPER).contains(&val));
        }
    }
}
```

### Pattern 4: Constant Input Properties
```rust
proptest! {
    fn prop_indicator_constant_input(constant in 1.0..1000.0_f64, len in 5usize..50, period in 1usize..=10) {
        let data = vec![constant; len];
        let result = indicator(&data, period).unwrap();
        // Verify expected behavior for constant input
    }
}
```

## Recommendations

### Immediate (subtask-4-2)

Add property tests for the **12 benchmarked indicators** without coverage:
1. ADX - bounded [0,100], non-negative
2. Williams %R - bounded [-100,0]
3. Donchian - channel containment
4. OBV - cumulative, no internal NaN
5. VWAP - positive values
6. Price transforms (avgprice, medprice, typprice, wclprice) - output shape
7. AD - cumulative properties
8. ROC - output shape, NaN count
9. MFI - bounded [0,100]
10. VAR - non-negative

### Future Improvements

1. **Expand WMA/DEMA/TEMA coverage** - Add tests similar to SMA/EMA
2. **Add momentum oscillator tests** - CMO, APO, StochRSI all bounded
3. **Add statistical function tests** - Correlation bounded [-1,1], VAR non-negative
4. **Consider adding NaN propagation property tests** - Verify NaN in input → NaN in output

## Compliance Status

| Category | Status |
|----------|--------|
| Core Moving Averages (SMA, EMA) | ✅ COVERED |
| Core Momentum (RSI, MACD, Stochastic) | ✅ COVERED |
| Core Volatility (ATR, Bollinger) | ✅ COVERED |
| Core Volume (OBV, VWAP) | ❌ MISSING |
| Benchmarked Indicators | ⚠️ PARTIAL (8/20 = 40%) |
| All Indicator Modules | ⚠️ PARTIAL (8/47 = 17%) |

**Overall Result: PARTIAL COVERAGE - Gaps identified for subtask-4-2**

## Files Referenced

- `crates/liq-ta/tests/property_tests.rs` - Property tests file
- `crates/liq-ta/benches/indicators.rs` - Benchmark indicators list
- `crates/liq-ta/src/indicators/mod.rs` - All indicator modules
- `docs/indicator-standards.md` - Quality Gates requirements
