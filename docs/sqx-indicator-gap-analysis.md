# StrategyQuantX Indicator Gap Analysis

Gap analysis comparing indicators available in StrategyQuantX (SQX) against those
currently implemented in `liq-ta`.

## Implementation status update (2026-02-25)

The gap identified in this document has been materially reduced across Phases 1-4 of
`liq-docs/plans/liq-ta-sqx-indicator-gap-and-python-binding-plan.md`.

### Delivered since initial analysis

- **P1 core parity delivered**:
  - `keltner_channel`
  - `ichimoku`
  - `qqe`
- **P2 high-value extension set delivered**:
  - `hma`, `supertrend`, `ao`, `bulls_power`, `bears_power`, `demarker`, `osma`
  - `vortex`, `rvi`, `dpo`, `connors_rsi`, `stc`, `laguerre_rsi`, `dss_bressert`
  - `chop`, `ulcer_index`, `hurst`, `autocorr`
- **Gaussian Channel strategy support delivered**:
  - `gaussian_filter`
  - `gaussian_channel` (center/upper/lower/trend outputs)
- **Composite envelope/band indicators delivered**:
  - `hma_atr_bands`, `hma_bollinger_bands`
  - `vwap_atr_bands`, `vwap_bollinger_bands`
- **Python and CLI parity surfaces expanded** with registry-backed metadata validation,
  deterministic error taxonomy, and strategy-oriented coverage tests.

### Remaining work after Stage 5

Remaining gaps are primarily lower-priority/community-style (`P3`) items such as
Waddah Attar Explosion, TTM Squeeze, DiDi Index, and other niche/proprietary variants.

## Research Sources

- [StrategyQuant Features](https://strategyquant.com/features/)
- [AutoTradingAcademy - StrategyQuant](https://www.autotradingacademy.com/strategyquant)
- [SQX New Indicators Blog (2022)](https://strategyquant.com/blog/new-indicators-has-been-implemented-for-strategyquant-x/)
- [SQX v130 Indicators](https://strategyquant.com/blog/new-indicators-in-strategyquant-x-130/)
- [SQX Signal Indicator Classification](https://strategyquant.com/shared/signal-indicator-classification/)
- [No Nonsense Trader - SQX Indicator Hub](https://nononsensetrader.com/the-ultimate-indicator-hub-for-strategyquant-users-all-in-one-downloadable-package/)
- [SQX 2025 Plans](https://strategyquant.com/blog/isgreater-isloweradaptive-block-autocorrelation-indicator-and-plans-for-2025/)

---

## Current liq-ta Indicator Inventory

### Overlap Studies / Moving Averages (10)

| Indicator | Module | Description |
|-----------|--------|-------------|
| SMA | `sma.rs` | Simple Moving Average |
| EMA | `ema.rs` | Exponential Moving Average |
| WMA | `wma.rs` | Weighted Moving Average |
| DEMA | `dema.rs` | Double Exponential Moving Average |
| TEMA | `tema.rs` | Triple Exponential Moving Average |
| TRIMA | `trima.rs` | Triangular Moving Average |
| KAMA | `kama.rs` | Kaufman Adaptive Moving Average |
| MAMA | `mama.rs` | MESA Adaptive Moving Average |
| T3 | `t3.rs` | Triple Exponential Moving Average (T3) |
| MAVP | `mavp.rs` | Moving Average with Variable Period |

### Trend (5)

| Indicator | Module | Description |
|-----------|--------|-------------|
| ADX | `adx.rs` | Average Directional Movement Index |
| DX | `dx.rs` | Directional Movement Index |
| Aroon | `aroon.rs` | Aroon Up/Down |
| SAR | `sar.rs` | Parabolic SAR |
| SAREXT | `sarext.rs` | Parabolic SAR Extended |

### Momentum / Oscillators (14)

| Indicator | Module | Description |
|-----------|--------|-------------|
| RSI | `rsi.rs` | Relative Strength Index |
| MACD | `macd.rs` | Moving Average Convergence/Divergence |
| Stochastic | `stochastic.rs` | Stochastic Oscillator (Fast/Slow) |
| StochRSI | `stochrsi.rs` | Stochastic RSI |
| CCI | `cci.rs` | Commodity Channel Index |
| CMO | `cmo.rs` | Chande Momentum Oscillator |
| MOM | `mom.rs` | Momentum |
| ROC | `roc.rs` | Rate of Change |
| APO | `apo.rs` | Absolute Price Oscillator |
| TRIX | `trix.rs` | Triple Smooth EMA Rate of Change |
| Williams %R | `williams_r.rs` | Williams Percent Range |
| BOP | `bop.rs` | Balance of Power |
| UltOsc | `ultosc.rs` | Ultimate Oscillator |
| MFI | `mfi.rs` | Money Flow Index |

### Volatility (3)

| Indicator | Module | Description |
|-----------|--------|-------------|
| ATR | `atr.rs` | Average True Range |
| Bollinger Bands | `bollinger.rs` | Bollinger Bands (upper/mid/lower) |
| Donchian Channels | `donchian.rs` | Donchian Channel (high/low) |

### Volume (4)

| Indicator | Module | Description |
|-----------|--------|-------------|
| OBV | `obv.rs` | On Balance Volume |
| AD | `ad.rs` | Accumulation/Distribution Line |
| ADOSC | `adosc.rs` | Accumulation/Distribution Oscillator |
| VWAP | `vwap.rs` | Volume Weighted Average Price |

### Hilbert Transform (6)

| Indicator | Module | Description |
|-----------|--------|-------------|
| HT Trendline | `ht_trendline.rs` | Hilbert Transform - Instantaneous Trendline |
| HT DC Period | `ht_dcperiod.rs` | Hilbert Transform - Dominant Cycle Period |
| HT DC Phase | `ht_dcphase.rs` | Hilbert Transform - Dominant Cycle Phase |
| HT Phasor | `ht_phasor.rs` | Hilbert Transform - Phasor Components |
| HT Sine | `ht_sine.rs` | Hilbert Transform - SineWave |
| HT Trend Mode | `ht_trendmode.rs` | Hilbert Transform - Trend vs Cycle Mode |

### Price Transforms (4)

| Indicator | Module | Description |
|-----------|--------|-------------|
| AvgPrice | `price_transform.rs` | Average Price (O+H+L+C)/4 |
| MedPrice | `price_transform.rs` | Median Price (H+L)/2 |
| TypPrice | `price_transform.rs` | Typical Price (H+L+C)/3 |
| WclPrice | `price_transform.rs` | Weighted Close Price (H+L+2C)/4 |

### Midpoint (2)

| Indicator | Module | Description |
|-----------|--------|-------------|
| Midpoint | `midpoint.rs` | Midpoint over period |
| MidPrice | `midprice.rs` | Midpoint Price over period |

### Statistical Functions (15)

| Indicator | Module | Description |
|-----------|--------|-------------|
| Variance | `statistics.rs` | Population/Sample Variance |
| StdDev | `statistics.rs` | Standard Deviation |
| Skewness | `statistics.rs` | Skewness |
| Kurtosis | `statistics.rs` | Kurtosis |
| Covariance | `statistics.rs` | Covariance |
| Z-Score | `statistics.rs` | Z-Score |
| MAD | `statistics.rs` | Mean Absolute Deviation |
| SEM | `statistics.rs` | Standard Error of the Mean |
| Correlation | `statistics.rs` | Pearson Correlation Coefficient |
| Beta | `statistics.rs` | Beta (regression slope) |
| Linear Regression | `statistics.rs` | Linear Regression |
| LinReg Slope | `statistics.rs` | Linear Regression Slope |
| LinReg Intercept | `statistics.rs` | Linear Regression Intercept |
| LinReg Angle | `statistics.rs` | Linear Regression Angle |
| TSF | `statistics.rs` | Time Series Forecast |

### Candlestick Patterns (47)

**Single Candle (17):** Doji, Dragonfly Doji, Gravestone Doji, Long-Legged Doji,
Rickshaw Man, Marubozu, Closing Marubozu, Spinning Top, High Wave, Long Line,
Short Line, Hammer, Hanging Man, Inverted Hammer, Shooting Star, Takuri, Belt Hold

**Two Candle (17):** Engulfing, Harami, Harami Cross, Piercing, Dark Cloud Cover,
Doji Star, Kicking, Kicking by Length, Matching Low, Homing Pigeon, In Neck,
On Neck, Thrusting, Separating Lines, Counter Attack, Two Crows, Hikkake,
Hikkake Modified

**Three Candle (13):** Morning Star, Evening Star, Morning Doji Star, Evening Doji Star,
Abandoned Baby, Three White Soldiers, Three Black Crows, Three Inside, Three Outside,
Three Line Strike, Three Stars in South, Tri-Star, Identical Three Crows

**Total implemented: ~110 indicators/patterns**

---

## StrategyQuantX Indicator Inventory

SQX advertises 40+ core technical indicators and 250+ building blocks (indicators +
signals + comparison operators). Below is the consolidated list from official sources.

### Core Built-in Indicators

**Trend:**
SMA, EMA, WMA, TEMA, ADX, Parabolic SAR, Ichimoku Kinko Hyo, Keltner Channel,
Highest/Lowest, QQE

**Momentum/Oscillators:**
CCI, RSI, Stochastic, MACD, Momentum, Williams %R, True Range, Price Difference

**Volatility:**
ATR, Bollinger Bands

### Extended Indicators (added via updates)

| Indicator | Version/Date | Category |
|-----------|-------------|----------|
| Vortex Indicator | v130 | Trend |
| Choppiness Index (CHOP) | v130 | Volatility |
| Detrended Price Oscillator (DPO) | v130 | Momentum |
| ROC (Rate of Change) | Apr 2022 | Momentum |
| Disparity Index | Apr 2022 | Momentum |
| Relative Vigor Index (RVI) | Apr 2022 | Momentum |
| Double Smoothed Stochastic Bressert | Apr 2022 | Momentum |
| TMA Centered Bands | 2022 | Volatility |
| RCI 3 Lines | 2022 | Momentum |
| Waddah Attar Explosion (WAE) | 2022 | Momentum |
| Connors RSI (CRSI) | 2022 | Momentum |
| Z-Score | Jan 2023 | Statistical |
| DEMA | Community | Overlap |
| Money Flow Index | Community | Volume |
| Slope Direction Line | Community | Trend |

### Community / Add-on Indicators (from No Nonsense Trader Hub)

**Momentum Engine:**
Waddah Attar Explosion, ROC, RVI, HMA ATR Bands, HMA Bollinger Bands,
VWAP ATR Bands, VWAP Bollinger Bands, DSS Bressert, David Varati Oscillator,
Disparity Index, ATR Percent, ATR Percent Rank, Z-Score, Donchian Channels

**Reversion Core:**
Smoothed RSI, Connors RSI, Close Minus MA, Z-Score, DPO, VWAP, Casey Percent,
BH Ergodic, DiDi Index, Disparity Index, TTM Squeeze, DSS Bressert

**Filter Matrix:**
Entropy Math, Choppiness Index, CCA Market Regime, ATR Trailing Stops,
Ehlers Hilbert Transform, Ehlers Moving of All Moving Averages, KAMA OHLC,
Semaphore Signal Level Channel, Logarithmic ATR, Hurst Exponent

### SQX Signal/Classification Categories

**Trend:** ADX, Aroon, Gann Hi-Lo, HMA, Ichimoku, KAMA, KAMA Efficiency Ratio,
Linear Regression, Moving Average, Parabolic SAR, Super Trend

**Momentum:** Awesome Oscillator, Bears Power, Bulls Power, CCI, De Marker,
Laguerre RSI, MACD, Momentum, OSMA, QQE, Reflex, ROC, RSI, Schaff Trend Cycle,
SR Percent Rank, Stochastic, Vortex, Williams %R

**Volatility:** ATR, Bollinger Bands, Keltner Channel, StdDev, Ulcer Index

---

## Gap Analysis

### Legend

- **P1** = High priority (core SQX built-in, commonly used)
- **P2** = Medium priority (SQX extended/classification indicator)
- **P3** = Lower priority (community add-on, niche)

### Missing: Trend Indicators

| Indicator | Priority | Complexity | Notes |
|-----------|----------|------------|-------|
| Ichimoku Kinko Hyo | **P1** | Medium | Core SQX. 5 lines: Tenkan-sen, Kijun-sen, Senkou A/B, Chikou. Uses Highest/Lowest over periods. |
| Keltner Channel | **P1** | Low | Core SQX. EMA center + ATR-based bands. Depends on existing EMA + ATR. |
| Super Trend | **P2** | Low | ATR-based trend following. Depends on existing ATR. |
| Hull Moving Average (HMA) | **P2** | Low | WMA-based smoothed MA. Depends on existing WMA. |
| Gann Hi-Lo | **P2** | Low | SMA of highs/lows with trend switch logic. |
| Slope Direction Line | **P3** | Low | Linear regression slope visualization. LinReg already exists. |

### Missing: Momentum / Oscillators

| Indicator | Priority | Complexity | Notes |
|-----------|----------|------------|-------|
| QQE | **P1** | Medium | Core SQX. Smoothed RSI with dynamic volatility bands. Depends on RSI + EMA. |
| Awesome Oscillator (AO) | **P2** | Low | Difference of 5-period and 34-period SMA of median price. |
| Bears Power | **P2** | Low | Low minus EMA. |
| Bulls Power | **P2** | Low | High minus EMA. |
| De Marker | **P2** | Low | Compares high/low to previous high/low. |
| OSMA | **P2** | Low | MACD histogram smoothed. Depends on existing MACD. |
| Connors RSI (CRSI) | **P2** | Medium | Composite of RSI, up/down streak RSI, and ROC percentile. |
| Relative Vigor Index (RVI) | **P2** | Low | Close-Open vs High-Low ratio smoothed. |
| Schaff Trend Cycle (STC) | **P2** | Medium | Double-smoothed stochastic of MACD. |
| Vortex Indicator | **P2** | Low | Positive/negative trend movement. Uses True Range. |
| Detrended Price Oscillator (DPO) | **P2** | Low | Price minus shifted SMA. |
| Laguerre RSI | **P2** | Medium | RSI using Laguerre filter instead of EMA. |
| Reflex Indicator | **P2** | Medium | Ehlers cycle indicator. |
| DSS Bressert | **P2** | Medium | Double smoothed stochastic. |
| Disparity Index | **P3** | Low | Percent difference from MA. |
| SR Percent Rank | **P3** | Low | Percentile rank of support/resistance. |
| RCI 3 Lines | **P3** | Medium | Rank Correlation Index across 3 timeframes. |
| Waddah Attar Explosion (WAE) | **P3** | Medium | Combines Bollinger Bands and MACD for volatility breakout. |
| David Varati Oscillator | **P3** | Low | Proprietary oscillator. |
| BH Ergodic | **P3** | Medium | True Strength Index variant. |
| DiDi Index | **P3** | Low | Divergence detection index. |
| Casey Percent | **P3** | Low | Proprietary percent oscillator. |
| TTM Squeeze | **P3** | Medium | Bollinger inside Keltner detection + momentum histogram. |
| Smoothed RSI | **P3** | Low | RSI with additional smoothing. Trivial given existing RSI + EMA. |
| Close Minus Moving Average | **P3** | Low | Trivial: close - MA. |

### Missing: Volatility

| Indicator | Priority | Complexity | Notes |
|-----------|----------|------------|-------|
| Ulcer Index | **P2** | Low | Measures downside volatility. |
| Choppiness Index (CHOP) | **P2** | Low | Uses ATR sum / Highest-Lowest range. |
| TMA Centered Bands | **P3** | Low | TRIMA + ATR bands. Depends on existing TRIMA + ATR. |
| ATR Percent | **P3** | Low | ATR as percentage of price. Trivial. |
| ATR Trailing Stops | **P3** | Low | Price +/- ATR multiple with trend logic. |
| Logarithmic ATR | **P3** | Low | ATR on log-transformed prices. |

### Missing: Advanced / Statistical

| Indicator | Priority | Complexity | Notes |
|-----------|----------|------------|-------|
| Hurst Exponent | **P2** | Medium | Measures long-range dependence / mean-reversion tendency. |
| Autocorrelation | **P2** | Medium | Correlation of series with lagged version of itself. |
| Entropy | **P3** | Medium | Shannon entropy of price distribution. |
| CCA Market Regime | **P3** | High | Market regime classification. |
| KAMA Efficiency Ratio | **P3** | Low | Already partially in KAMA; just expose the ratio. |
| CUSUM Monitoring | **P3** | Medium | Cumulative sum change detection. Planned for SQX 2025. |
| EWMA Drift Detection | **P3** | Low | Exponentially weighted MA for drift. Planned for SQX 2025. |

### Missing: Composite / Band Indicators

| Indicator | Priority | Complexity | Notes |
|-----------|----------|------------|-------|
| HMA ATR Bands | **P3** | Low | HMA + ATR envelope. Requires HMA first. |
| HMA Bollinger Bands | **P3** | Low | HMA + StdDev envelope. Requires HMA first. |
| VWAP ATR Bands | **P3** | Low | VWAP + ATR envelope. Trivial given existing VWAP + ATR. |
| VWAP Bollinger Bands | **P3** | Low | VWAP + StdDev envelope. Trivial given existing VWAP + StdDev. |
| Semaphore Signal Level Channel | **P3** | Medium | Multi-level signal channel. |

---

## Summary

| Category | liq-ta Has | SQX Has (est.) | Gap Count |
|----------|-----------|----------------|-----------|
| Overlap / MA | 10 | 13 | 3 (Ichimoku*, HMA, Gann Hi-Lo) |
| Trend | 5 | 8 | 3 (Ichimoku, Super Trend, Keltner) |
| Momentum | 14 | 32 | 18 |
| Volatility | 3 | 8 | 5 |
| Volume | 4 | 4 | 0 |
| Hilbert Transform | 6 | 2 | 0 (liq-ta ahead) |
| Statistical | 15 | 5 | 0 (liq-ta ahead) |
| Price Transforms | 4 | 2 | 0 (liq-ta ahead) |
| Advanced / Regime | 0 | 6 | 6 |
| Candlestick | 47 | ~10 | 0 (liq-ta ahead) |
| **Total** | **~110** | **~90 core+ext** | **~35 missing** |

*Ichimoku spans both Overlap and Trend categories.

---

## Recommended Implementation Order

### Stage 1 - Core Parity (P1)

These are core SQX built-in indicators that are broadly expected in any TA library.

1. **Keltner Channel** - Low complexity, depends on existing EMA + ATR
2. **Ichimoku Kinko Hyo** - Medium complexity, uses rolling max/min (already have)
3. **QQE** - Medium complexity, depends on RSI + EMA

### Stage 2 - Extended Parity (P2)

Common indicators from SQX's extended set and classification system.

4. **Hull Moving Average (HMA)** - Low complexity
5. **Super Trend** - Low complexity
6. **Awesome Oscillator** - Low complexity
7. **Bears Power / Bulls Power** - Low complexity (implement together)
8. **De Marker** - Low complexity
9. **OSMA** - Low complexity
10. **Vortex Indicator** - Low complexity
11. **Relative Vigor Index (RVI)** - Low complexity
12. **Detrended Price Oscillator (DPO)** - Low complexity
13. **Connors RSI (CRSI)** - Medium complexity
14. **Schaff Trend Cycle (STC)** - Medium complexity
15. **Choppiness Index (CHOP)** - Low complexity
16. **Ulcer Index** - Low complexity
17. **Hurst Exponent** - Medium complexity
18. **Autocorrelation** - Medium complexity
19. **Laguerre RSI** - Medium complexity
20. **Reflex** - Medium complexity
21. **DSS Bressert** - Medium complexity
22. **Gann Hi-Lo** - Low complexity

### Stage 3 - Community / Niche (P3)

Lower priority indicators, mostly community add-ons or composite indicators.

23-35. Remaining P3 indicators as needed.

---

## Notes

- Many P3 indicators are composites of existing primitives (e.g., VWAP ATR Bands =
  VWAP + ATR arithmetic) and could be left as user-space compositions rather than
  first-class library indicators.
- SQX's "250+ building blocks" count includes signal conditions (e.g., "CCI crossed
  above 0"), comparison operators, and exit methods -- not just raw indicators.
- liq-ta is already ahead of SQX in Hilbert Transform, statistical functions,
  candlestick patterns, and price transforms.
- Some SQX community indicators (Casey Percent, David Varati Oscillator, DiDi Index)
  have limited public documentation, making accurate implementation difficult without
  reference source code.
