# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added (2026-02-25: SQX parity + Python binding architecture)

- Completed SQX parity/core extension delivery in `liq-ta`:
  - P1: `keltner_channel`, `ichimoku`, `qqe`
  - P2+: `hma`, `supertrend`, `ao`, `bulls_power`, `bears_power`, `demarker`, `osma`,
    `vortex`, `rvi`, `dpo`, `connors_rsi`, `stc`, `laguerre_rsi`, `dss_bressert`,
    `chop`, `ulcer_index`, `hurst`, `autocorr`
  - Gaussian strategy path: `gaussian_filter`, `gaussian_channel`
  - Composite bands: `hma_atr_bands`, `hma_bollinger_bands`, `vwap_atr_bands`, `vwap_bollinger_bands`
- Added Python hardening API helpers:
  - `compute_indicator(name, *args, **kwargs)`
  - `require_indicator_info(name)`
  - `validate_indicator_metadata(raise_on_error=...)`
- Added explicit Python error taxonomy for deterministic failure handling:
  - `LiqTaError`, `IndicatorNotFoundError`, `IndicatorArgumentError`, `IndicatorMetadataError`
- Added migration/onboarding documentation:
  - `docs/python-binding-migration-guide.md`

### Changed (2026-02-25: compatibility and diagnostics)

- Standardized CLI error-class output with stable tags:
  - `io_error`, `csv_parse_error`, `indicator_error`, `invalid_argument`
- Added optional CLI debug diagnostics:
  - `--debug-errors`
  - `LIQ_TA_DEBUG_ERRORS=1` environment toggle
- Preserved backward compatibility for existing manual Python wrappers while moving
  new additions to registry-first onboarding.

### Documentation (2026-02-25)

- Updated `docs/sqx-indicator-gap-analysis.md` with implementation status and
  remaining lower-priority gap scope.
- Expanded `docs/python-binding-architecture-stage0.md` with extension examples,
  deterministic error contracts, and migration guidance.

### Fixed

- **CRITICAL: ADX Wilder Smoothing Math Bug** (2025-12-28)
  - Fixed incorrect Wilder smoothing formula that caused unbounded ADX values (could exceed 100)
  - Corrected from `prev - prev/period + current` to `(prev*(period-1) + current)/period`
  - Added range validation test ensuring ADX ∈ [0, 100]
  - Impact: All ADX calculations now produce correct bounded values

- **WMA Infinity Handling Regression** (2025-12-28)
  - Restored `is_invalid()` helper that checks both NaN AND Infinity
  - Previous refactor only checked `.is_nan()`, missing ±Infinity propagation
  - Impact: Infinity now correctly propagates through WMA calculations

- **Precision Test Parameter Issues** (2025-12-28)
  - Fixed `precision_var_near_constant` test using unrepresentable f32 values
  - Changed from `base=1000.0, noise=1e-5` to `base=10.0, noise=1e-5`
  - f32 ULP at 1000.0 is 1.19e-4, noise 1e-5 is too small to represent
  - All 16 precision validation tests now pass (was 3 failing)

### Changed

- **IEEE 754 NaN Propagation Optimizations** (2025-12-28)
  - ADX: Added `nan_active` flag pattern for +8% to +41% throughput improvement
  - VAR: Implemented f64 shifted variance formula for High precision mode
  - Prevents catastrophic cancellation with near-constant data
  - Performance neutral or improved across all indicators

### Added

- **Precision Mode System** (`precision` module)
  - `PrecisionMode::High` (default): Uses f64 accumulators for f32 inputs
  - `PrecisionMode::Fast`: Uses native f32 accumulators for maximum throughput
  - `set_precision_mode()`: Runtime configuration
  - `with_precision_mode()`: Thread-local scoped override
  - `current_precision_mode()`: Query current mode
  - `LIQ_TA_PRECISION` environment variable support
  - `precision-fast` Cargo feature for compile-time default

- **Mixed-Precision Arithmetic for f32 Inputs**
  - SMA: f64 rolling sum accumulator
  - Bollinger Bands: f64 rolling sum and sum-of-squares
  - RSI: f64 Wilder smoothing state (avg_gain, avg_loss)
  - Stochastic/%K/%D: f64 division and SMA smoothing
  - Williams %R: f64 range division
  - ROC family: f64 division operations
  - VWAP: f64 cumulative TP×Volume and Volume sums
  - OBV: f64 cumulative volume sum
  - AD: f64 cumulative Accumulation/Distribution sum
  - VAR: f64 rolling sum and sum-of-squares
  - CCI: f64 typical price, SMA, and mean deviation
  - MFI: f64 positive/negative money flow sums

- **Precision Validation Test Suite** (`tests/precision_validation.rs`)
  - 16 tests comparing f32 High mode against f64 reference
  - Tolerance helpers per Error Tolerance Specification
  - Synthetic data generators (random walk, near-constant, extreme values)
  - Error statistics reporting (max, mean, RMS)

- **NaN Propagation Tests** (`tests/nan_propagation_precision.rs`)
  - 23 tests verifying NaN/Infinity behavior identical in both modes
  - Edge cases: signed zeros, subnormals, consecutive NaNs

- **Precision Comparison Benchmarks** (`benches/precision_comparison.rs`)
  - Fast vs High mode performance comparison
  - Overhead verification against Performance Acceptance Criteria

- **Documentation**
  - `docs/numeric-precision.md`: Comprehensive precision guide
  - Precision behavior documented in each migrated indicator module

### Changed

- **Indicator Documentation**
  - Added "Precision Behavior" section to all migrated indicators
  - Documented tolerance expectations per indicator category
  - Added recommendations for maximum precision (f64 input)

### Performance

Precision mode overhead for f32 inputs (High mode vs Fast mode):

| Indicator Type | Overhead |
|----------------|----------|
| Simple (SMA, Stochastic, ROC) | 8-15% |
| Variance-based (Bollinger, VAR) | 3-20% |
| Cumulative (VWAP, OBV, AD) | 10-15% |
| Wilder smoothing (RSI, MFI) | 12-15% |

All indicators meet Performance Acceptance Criteria (15-20% max overhead).

### Precision Tolerances

When comparing f32 High mode against pure f64 reference:

| Indicator Category | Tolerance |
|--------------------|-----------|
| RSI, Stochastic, Williams %R, MFI | abs(0.01) |
| SMA, Bollinger, VWAP | hybrid(1e-5 rel, 1e-7 abs) |
| OBV, AD | hybrid(1e-4 rel, 1.0 abs) |
| VAR, STDDEV | hybrid(1e-5 rel, 1e-10 abs) |
| CCI | hybrid(1e-4 rel, 0.1 abs) |
| ROC, ROCR100 | hybrid(2e-4 rel, 2e-5 abs) |
| ROCP, ROCR | hybrid(1e-4 rel, 1e-6 abs) |

## [0.1.0] - 2025-12-24

### Added

- **129 TA-Lib Compatible Indicators**: Complete indicator suite
  - Moving Averages: SMA, EMA, WMA, DEMA, TEMA, TRIMA, KAMA, T3, MAVP
  - Momentum: RSI, MACD, MOM, ROC, CMO, APO, PPO, TRIX, ULTOSC, Stochastic, StochRSI
  - Trend: ADX, DX, AROON, CCI, SAR, MAMA
  - Volatility: ATR, TRANGE, Bollinger Bands, Donchian Channels
  - Volume: OBV, AD, ADOSC, MFI, VWAP
  - Statistics: VAR, STDDEV, LINEARREG, TSF, CORREL, BETA
  - Price Transforms: AVGPRICE, MEDPRICE, TYPPRICE, WCLPRICE
  - Hilbert Transform: HT_TRENDLINE, HT_SINE, HT_DCPERIOD, HT_DCPHASE, HT_PHASOR, HT_TRENDMODE
  - Candlestick Patterns: 61 patterns (Doji, Hammer, Engulfing, Morning Star, etc.)

- **Three API Layers**
  - Simple API: `sma(&data, period)` - returns new Vec
  - Buffer API: `sma_into(&data, period, &mut output)` - writes to pre-allocated buffer
  - Configuration Types: `Macd::default().compute(&data)` - for complex indicators

- **Lookback Functions**
  - `*_lookback()` functions for all indicators (number of NaN values)
  - `*_min_len()` functions for all indicators (minimum input length)
  - Semver-stable contracts per PRD §4.11

- **CLI Tool** (`liq-ta-cli`)
  - CSV input/output with auto-detected columns
  - All 12 indicators supported
  - Exit codes (0=success, 1=argument, 2=data, 3=computation)
  - Actionable error messages with hints

- **Comprehensive Test Suite**
  - Spec fixtures (authoritative tests per PRD)
  - Property-based tests (proptest)
  - Input validation tests (53 tests)
  - Numeric stability tests (42 tests)
  - Allocation verification tests (16 tests)
  - Real-world regression tests with synthetic data

- **Documentation**
  - Complete rustdoc for all public items
  - README with quick start and examples
  - CLI README with usage guide
  - Benchmark baseline documentation

### Changed

- **Architecture Refactoring**
  - Removed DAG-based plan mode (E07 showed 1.4-2.2x overhead)
  - Removed fusion kernels that benchmarked slower (E02, E03)
  - Kept rolling_extrema kernel (E04: 4.3-24.4x faster)
  - Single library crate `liq-ta` + CLI crate `liq-ta-cli`

- **Numeric Policy**
  - Full-length output (NaN prefix for lookback)
  - NaN propagation (NaN in window produces NaN output)
  - Deterministic results (same inputs = identical outputs)

- **Initialization Rules**
  - EMA uses SMA seed (not first-value)
  - RSI uses Wilder smoothing (alpha = 1/period)
  - ATR first value = SMA of first period True Ranges
  - Stochastic: high == low returns %K = 50

### Removed

- `liq-ta-experiments` crate (benchmark/research code)
- `plan/` module (DAG-based execution)
- `kernels/running_stat.rs` (2.8x slower than separate passes)
- `kernels/ema_fusion.rs` (30% slower at scale)
- `petgraph` dependency

### Fixed

- Bollinger Bands uses population stddev (÷n) not sample (÷n-1)
- RSI extremes: all gains returns 100, all losses returns 0
- MACD validates fast_period < slow_period
- All indicators properly handle NaN/Infinity inputs

### Performance

Benchmark results (100K elements):

| Indicator | Throughput | vs TA-Lib |
|-----------|------------|-----------|
| MOM | 5.18 Gelem/s | 0.79× |
| ROC | 2.23 Gelem/s | **1.18×** |
| TRANGE | 2.17 Gelem/s | 0.65× |
| BOP | 1.79 Gelem/s | ~1.0× |
| AD | 1.25 Gelem/s | **1.15×** |
| WMA | 818 Melem/s | 0.97× |
| EMA | 712 Melem/s | 0.88× |
| APO | 670 Melem/s | **1.76×** |
| SMA | 425 Melem/s | 0.41× |
| OBV | 388 Melem/s | 0.28× |
| Bollinger | 372 Melem/s | **1.42×** |
| DEMA | 358 Melem/s | 0.94× |
| TEMA | 294 Melem/s | **1.13×** |
| ATR | 268 Melem/s | **1.06×** |
| RSI | 266 Melem/s | **1.14×** |
| MACD | 253 Melem/s | **1.06×** |
| KAMA | 249 Melem/s | 0.53× |
| ADX | 238 Melem/s | ~1.0× |
| MIDPOINT | 235 Melem/s | **1.40×** |

All indicators demonstrate O(n) linear time complexity. liq-ta outperforms TA-Lib
on 9/35 benchmarked indicators (TEMA, RSI, MACD, ROC, APO, ATR, Bollinger, AD, MIDPOINT).

## [0.0.1] - 2024-XX-XX

### Added

- Initial experimental implementation
- Benchmarking framework for E01-E07 experiments
- Proof of concept for rolling algorithms

---

*This changelog tracks the evolution from experimental benchmark framework to production-ready technical analysis library.*
