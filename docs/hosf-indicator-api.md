# HOSF Indicator API Draft

## Overview
High Order Statistical Features (HOSF) are rolling-window indicators that follow the standard liq-ta API:
- `indicator(...) -> Result<Vec<T>>`
- `indicator_into(...) -> Result<usize>`
- `indicator_lookback(...) -> usize`
- `indicator_min_len(...) -> usize`

All outputs are full-length with NaN prefix of length `lookback`.

## Proposed Modules
- `crates/liq-ta/src/indicators/skew.rs`
- `crates/liq-ta/src/indicators/kurtosis.rs`
- `crates/liq-ta/src/indicators/moment3.rs`
- `crates/liq-ta/src/indicators/moment4.rs`

## API Signatures

```rust
use liq_ta::traits::SeriesElement;
use liq_ta::error::Result;

// Rolling skewness (third standardized moment)
pub fn skew<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>>;
pub fn skew_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize>;
pub const fn skew_lookback(period: usize) -> usize;
pub const fn skew_min_len(period: usize) -> usize;

// Rolling excess kurtosis (fourth standardized moment minus 3)
pub fn kurtosis<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>>;
pub fn kurtosis_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize>;
pub const fn kurtosis_lookback(period: usize) -> usize;
pub const fn kurtosis_min_len(period: usize) -> usize;

// Rolling 3rd central moment
pub fn moment3<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>>;
pub fn moment3_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize>;
pub const fn moment3_lookback(period: usize) -> usize;
pub const fn moment3_min_len(period: usize) -> usize;

// Rolling 4th central moment
pub fn moment4<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>>;
pub fn moment4_into<T: SeriesElement>(data: &[T], period: usize, output: &mut [T]) -> Result<usize>;
pub const fn moment4_lookback(period: usize) -> usize;
pub const fn moment4_min_len(period: usize) -> usize;
```

## Behavior Notes
- **Lookback**: `period - 1` (rolling window).
- **Min length**: `period`.
- **NaN policy**: any NaN in the window -> NaN output for that position.
- **Variance = 0**:
  - `skew = 0`
  - `kurtosis = 0`
- **Population moments**: divide by `n` (not `n-1`).
- **Standardization**:
  - `skew = m3 / m2^(3/2)`
  - `kurtosis = m4 / m2^2 - 3`

## Exports
Add to `crates/liq-ta/src/indicators/mod.rs` and `crates/liq-ta/src/prelude.rs`.
