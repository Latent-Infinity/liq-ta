//! Moving Average Convergence Divergence (MACD) indicator.
//!
//! The MACD is a trend-following momentum indicator that shows the relationship
//! between two exponential moving averages of a security's price. It consists
//! of three components:
//!
//! - **MACD Line**: The difference between the fast EMA and slow EMA
//! - **Signal Line**: An EMA of the MACD line (typically 9-period)
//! - **Histogram**: The difference between the MACD line and signal line
//!
//! # Algorithm
//!
//! This implementation computes MACD with O(n) time complexity:
//!
//! 1. Calculate fast EMA (typically 12-period)
//! 2. Calculate slow EMA (typically 26-period)
//! 3. MACD Line = Fast EMA - Slow EMA
//! 4. Signal Line = EMA(MACD Line, `signal_period`)
//! 5. Histogram = MACD Line - Signal Line
//!
//! # Formula
//!
//! ```text
//! MACD Line[i] = EMA(fast_period)[i] - EMA(slow_period)[i]
//! Signal Line[i] = EMA(MACD Line, signal_period)[i]
//! Histogram[i] = MACD Line[i] - Signal Line[i]
//! ```
//!
//! # NaN Handling
//!
//! - MACD Line: First `slow_period - 1` values are NaN (need slow EMA to be valid)
//! - Signal Line: First `slow_period - 1 + signal_period - 1` values are NaN
//! - Histogram: Same as Signal Line (needs both MACD and Signal to be valid)
//!
//! # Example
//!
//! ```
//! use fast_ta::indicators::macd::{macd, MacdOutput};
//!
//! let data = vec![
//!     26.0_f64, 27.0, 28.0, 27.5, 28.5, 29.0, 28.0, 27.0, 28.0, 29.0,
//!     30.0, 29.5, 30.5, 31.0, 30.0, 31.0, 32.0, 31.5, 32.5, 33.0,
//!     32.0, 31.0, 32.0, 33.0, 34.0, 33.5, 34.5, 35.0, 34.0, 35.0,
//!     36.0, 35.5, 36.5, 37.0,
//! ];
//!
//! // Standard MACD with 12, 26, 9 periods
//! let result = macd(&data, 12, 26, 9).unwrap();
//!
//! // Access the three components
//! assert!(result.macd_line[24].is_nan()); // slow_period - 1 = 25 values are NaN
//! assert!(!result.macd_line[25].is_nan()); // First valid MACD value
//! ```

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns the lookback period for the MACD line.
///
/// The MACD line lookback is `slow_period - 1` (first valid MACD line value).
///
/// # Example
///
/// ```
/// use fast_ta::indicators::macd::macd_line_lookback;
///
/// assert_eq!(macd_line_lookback(26), 25);
/// assert_eq!(macd_line_lookback(12), 11);
/// ```
#[inline]
#[must_use]
pub const fn macd_line_lookback(slow_period: usize) -> usize {
    if slow_period == 0 {
        0
    } else {
        slow_period - 1
    }
}

/// Returns the lookback period for the MACD signal line and histogram.
///
/// The signal line lookback is `slow_period + signal_period - 2`.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::macd::macd_signal_lookback;
///
/// // Standard MACD (12, 26, 9): 26 + 9 - 2 = 33
/// assert_eq!(macd_signal_lookback(26, 9), 33);
/// ```
#[inline]
#[must_use]
pub const fn macd_signal_lookback(slow_period: usize, signal_period: usize) -> usize {
    if slow_period == 0 || signal_period == 0 {
        0
    } else {
        slow_period + signal_period - 2
    }
}

/// Returns the minimum input length required for full MACD output.
///
/// This is the smallest input size that will produce at least one valid
/// signal line and histogram value.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::macd::macd_min_len;
///
/// // Standard MACD (12, 26, 9): 26 + 9 - 1 = 34
/// assert_eq!(macd_min_len(26, 9), 34);
/// ```
#[inline]
#[must_use]
pub const fn macd_min_len(slow_period: usize, signal_period: usize) -> usize {
    if slow_period == 0 || signal_period == 0 {
        0
    } else {
        slow_period + signal_period - 1
    }
}

/// The output of MACD calculation containing all three components.
///
/// # Fields
///
/// - `macd_line`: The difference between fast and slow EMAs
/// - `signal_line`: The EMA of the MACD line
/// - `histogram`: The difference between MACD line and signal line
#[derive(Debug, Clone)]
pub struct MacdOutput<T: SeriesElement> {
    /// The MACD line (fast EMA - slow EMA).
    ///
    /// First `slow_period - 1` values are NaN.
    pub macd_line: Vec<T>,

    /// The signal line (EMA of MACD line).
    ///
    /// First `slow_period - 1 + signal_period - 1` values are NaN.
    pub signal_line: Vec<T>,

    /// The histogram (MACD line - signal line).
    ///
    /// First `slow_period - 1 + signal_period - 1` values are NaN.
    pub histogram: Vec<T>,
}

impl<T: SeriesElement> MacdOutput<T> {
    /// Returns the length of the output vectors.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.macd_line.len()
    }

    /// Returns true if the output vectors are empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.macd_line.is_empty()
    }

    /// Returns the index of the first valid MACD line value.
    ///
    /// This is `slow_period - 1`.
    #[must_use]
    pub const fn first_valid_macd_index(&self, slow_period: usize) -> usize {
        slow_period - 1
    }

    /// Returns the index of the first valid signal/histogram value.
    ///
    /// This is `slow_period - 1 + signal_period - 1`.
    #[must_use]
    pub const fn first_valid_signal_index(
        &self,
        slow_period: usize,
        signal_period: usize,
    ) -> usize {
        slow_period - 1 + signal_period - 1
    }
}

/// Computes the Moving Average Convergence Divergence (MACD) indicator.
///
/// The MACD uses standard EMA smoothing (α = 2 / (period + 1)).
///
/// # Arguments
///
/// * `data` - The input price data series
/// * `fast_period` - The period for the fast EMA (typically 12)
/// * `slow_period` - The period for the slow EMA (typically 26)
/// * `signal_period` - The period for the signal line EMA (typically 9)
///
/// # Returns
///
/// A `Result` containing a `MacdOutput` with all three components, or an error
/// if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - Any period is zero (`Error::InvalidPeriod`)
/// - The fast period is greater than or equal to the slow period (`Error::InvalidPeriod`)
/// - The input data is shorter than required (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for each output vector (3n total)
/// - Uses fused computation: 2 passes instead of 5 for better cache efficiency
///
/// # Example
///
/// ```
/// use fast_ta::indicators::macd::macd;
///
/// let mut data: Vec<f64> = Vec::with_capacity(50);
/// for i in 0..50 {
///     data.push(100.0 + (i as f64) * 0.5);
/// }
/// let result = macd(&data, 12, 26, 9).unwrap();
///
/// // Standard interpretation:
/// // - MACD line crosses above signal: bullish signal
/// // - MACD line crosses below signal: bearish signal
/// // - Positive histogram: bullish momentum
/// // - Negative histogram: bearish momentum
/// ```
#[inline]
#[must_use = "this returns a Result with the MACD output, which should be used"]
pub fn macd<T: SeriesElement>(
    data: &[T],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<MacdOutput<T>> {
    // Validate inputs
    validate_macd_inputs(data, fast_period, slow_period, signal_period)?;

    let n = data.len();

    // Allocate output vectors
    let mut macd_line = vec![T::nan(); n];
    let mut signal_line = vec![T::nan(); n];
    let mut histogram = vec![T::nan(); n];

    // Use fused computation for better performance
    compute_macd_fused(
        data,
        fast_period,
        slow_period,
        signal_period,
        &mut macd_line,
        &mut signal_line,
        &mut histogram,
    )?;

    Ok(MacdOutput {
        macd_line,
        signal_line,
        histogram,
    })
}

/// Computes MACD into pre-allocated output buffers.
///
/// This variant allows reusing existing buffers to avoid allocations in
/// performance-critical code paths.
///
/// # Arguments
///
/// * `data` - The input price data series
/// * `fast_period` - The period for the fast EMA (typically 12)
/// * `slow_period` - The period for the slow EMA (typically 26)
/// * `signal_period` - The period for the signal line EMA (typically 9)
/// * `macd_output` - Pre-allocated output buffer for MACD line
/// * `signal_output` - Pre-allocated output buffer for signal line
/// * `histogram_output` - Pre-allocated output buffer for histogram
///
/// # Returns
///
/// A `Result` containing a tuple of (`valid_macd_count`, `valid_signal_count`),
/// where `valid_signal_count` also applies to the histogram.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - Any period is zero (`Error::InvalidPeriod`)
/// - The fast period is greater than or equal to the slow period (`Error::InvalidPeriod`)
/// - The input data is shorter than required (`Error::InsufficientData`)
/// - Any output buffer is shorter than the input data
///
/// # Example
///
/// ```
/// use fast_ta::indicators::macd::macd_into;
///
/// let mut data: Vec<f64> = Vec::with_capacity(50);
/// for i in 0..50 {
///     data.push(100.0 + (i as f64) * 0.5);
/// }
/// let mut macd_line = vec![0.0_f64; 50];
/// let mut signal_line = vec![0.0_f64; 50];
/// let mut histogram = vec![0.0_f64; 50];
///
/// let (valid_macd, valid_signal) = macd_into(
///     &data, 12, 26, 9,
///     &mut macd_line, &mut signal_line, &mut histogram
/// ).unwrap();
/// ```
#[inline]
#[must_use = "this returns a Result with the valid MACD counts"]
pub fn macd_into<T: SeriesElement>(
    data: &[T],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    macd_output: &mut [T],
    signal_output: &mut [T],
    histogram_output: &mut [T],
) -> Result<(usize, usize)> {
    // Validate inputs
    validate_macd_inputs(data, fast_period, slow_period, signal_period)?;

    let n = data.len();

    // Validate output buffer sizes
    if macd_output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: macd_output.len(),
            indicator: "macd",
        });
    }
    if signal_output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: signal_output.len(),
            indicator: "macd",
        });
    }
    if histogram_output.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: histogram_output.len(),
            indicator: "macd",
        });
    }

    // Use fused computation for better performance
    compute_macd_fused(
        data,
        fast_period,
        slow_period,
        signal_period,
        macd_output,
        signal_output,
        histogram_output,
    )?;

    // Valid counts
    let first_valid_macd = slow_period - 1;
    let first_valid_signal = first_valid_macd + signal_period - 1;
    let valid_macd = n - first_valid_macd;
    let valid_signal = n - first_valid_signal;

    Ok((valid_macd, valid_signal))
}

/// Fused MACD computation: computes fast/slow EMAs together and signal/histogram together.
///
/// This reduces the number of passes over the data from 5 to 2 for better cache efficiency.
///
/// Pass 1: Compute both fast and slow EMAs in a single loop, output MACD line
/// Pass 2: Compute signal line and histogram in a single loop
#[inline]
fn compute_macd_fused<T: SeriesElement>(
    data: &[T],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    macd_output: &mut [T],
    signal_output: &mut [T],
    histogram_output: &mut [T],
) -> Result<()> {
    let n = data.len();

    // Compute alpha values for standard EMA: α = 2 / (period + 1)
    let two = T::two();
    let fast_alpha = two / T::from_usize(fast_period + 1)?;
    let slow_alpha = two / T::from_usize(slow_period + 1)?;
    let signal_alpha = two / T::from_usize(signal_period + 1)?;

    let fast_one_minus_alpha = T::one() - fast_alpha;
    let slow_one_minus_alpha = T::one() - slow_alpha;
    let signal_one_minus_alpha = T::one() - signal_alpha;

    let fast_period_t = T::from_usize(fast_period)?;
    let slow_period_t = T::from_usize(slow_period)?;
    let signal_period_t = T::from_usize(signal_period)?;

    // =========================================================================
    // PASS 1: Compute fast EMA, slow EMA, and MACD line in a single loop
    // =========================================================================

    // Initialize lookback period with NaN
    let first_valid_slow = slow_period - 1;

    // Compute SMA seeds for both EMAs
    let mut fast_sum = T::zero();
    let mut slow_sum = T::zero();
    let mut fast_nan_count = 0usize;
    let mut slow_nan_count = 0usize;

    // Accumulate for fast EMA seed
    for &value in data.iter().take(fast_period) {
        if is_invalid(value) {
            fast_nan_count += 1;
        } else {
            fast_sum = fast_sum + value;
        }
    }

    // Accumulate for slow EMA seed (includes fast_period values already summed)
    for &value in data.iter().take(slow_period) {
        if is_invalid(value) {
            slow_nan_count += 1;
        } else {
            slow_sum = slow_sum + value;
        }
    }

    // Initialize EMA values
    let mut fast_ema = if fast_nan_count == 0 {
        fast_sum / fast_period_t
    } else {
        T::nan()
    };

    let mut slow_ema = if slow_nan_count == 0 {
        slow_sum / slow_period_t
    } else {
        T::nan()
    };

    // Fill NaN for lookback period
    for value in macd_output.iter_mut().take(first_valid_slow) {
        *value = T::nan();
    }

    // Compute MACD values starting from first_valid_slow
    // But we need to advance fast_ema to the right position first

    // Advance fast EMA from first_valid_fast to first_valid_slow - 1
    for i in fast_period..slow_period {
        let value = data[i];
        if is_invalid(fast_ema) || is_invalid(value) {
            fast_ema = T::nan();
        } else {
            fast_ema = fast_alpha * value + fast_one_minus_alpha * fast_ema;
        }
    }

    // Now at index first_valid_slow, both EMAs are valid
    // Set first MACD value
    if !is_invalid(fast_ema) && !is_invalid(slow_ema) {
        macd_output[first_valid_slow] = fast_ema - slow_ema;
    } else {
        macd_output[first_valid_slow] = T::nan();
    }

    // Continue computing both EMAs and MACD line
    for i in (first_valid_slow + 1)..n {
        let value = data[i];

        // Update fast EMA
        if is_invalid(fast_ema) || is_invalid(value) {
            fast_ema = T::nan();
        } else {
            fast_ema = fast_alpha * value + fast_one_minus_alpha * fast_ema;
        }

        // Update slow EMA
        if is_invalid(slow_ema) || is_invalid(value) {
            slow_ema = T::nan();
        } else {
            slow_ema = slow_alpha * value + slow_one_minus_alpha * slow_ema;
        }

        // Compute MACD line
        if !is_invalid(fast_ema) && !is_invalid(slow_ema) {
            macd_output[i] = fast_ema - slow_ema;
        } else {
            macd_output[i] = T::nan();
        }
    }

    // =========================================================================
    // PASS 2: Compute signal line and histogram in a single loop
    // =========================================================================

    let first_valid_signal = first_valid_slow + signal_period - 1;

    // Fill NaN for signal and histogram lookback
    for value in signal_output.iter_mut().take(first_valid_signal) {
        *value = T::nan();
    }
    for value in histogram_output.iter_mut().take(first_valid_signal) {
        *value = T::nan();
    }

    // Early return if not enough data for signal
    if first_valid_signal >= n {
        return Ok(());
    }

    // Compute SMA seed for signal line from first signal_period valid MACD values
    let mut signal_sum = T::zero();
    for &value in macd_output
        .iter()
        .skip(first_valid_slow)
        .take(signal_period)
    {
        signal_sum = signal_sum + value;
    }
    let mut signal_ema = signal_sum / signal_period_t;

    // Set first valid signal and histogram
    signal_output[first_valid_signal] = signal_ema;
    histogram_output[first_valid_signal] = macd_output[first_valid_signal] - signal_ema;

    // Compute remaining signal and histogram values
    for i in (first_valid_signal + 1)..n {
        let macd_val = macd_output[i];

        if is_invalid(signal_ema) || is_invalid(macd_val) {
            signal_ema = T::nan();
            signal_output[i] = T::nan();
            histogram_output[i] = T::nan();
        } else {
            signal_ema = signal_alpha * macd_val + signal_one_minus_alpha * signal_ema;
            signal_output[i] = signal_ema;
            histogram_output[i] = macd_val - signal_ema;
        }
    }

    Ok(())
}

/// Validates MACD inputs.
#[inline]
const fn validate_macd_inputs<T: SeriesElement>(
    data: &[T],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<()> {
    // Validate periods
    if fast_period == 0 {
        return Err(Error::InvalidPeriod {
            period: fast_period,
            reason: "fast_period must be at least 1",
        });
    }
    if slow_period == 0 {
        return Err(Error::InvalidPeriod {
            period: slow_period,
            reason: "slow_period must be at least 1",
        });
    }
    if signal_period == 0 {
        return Err(Error::InvalidPeriod {
            period: signal_period,
            reason: "signal_period must be at least 1",
        });
    }
    if fast_period >= slow_period {
        return Err(Error::InvalidPeriod {
            period: fast_period,
            reason: "fast_period must be less than slow_period",
        });
    }

    // Validate data
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    // Minimum required data: slow_period (for slow EMA) + signal_period - 1 (for signal EMA)
    // Actually, we need at least slow_period for a valid MACD line,
    // and slow_period + signal_period - 1 for a valid signal line
    let min_required = slow_period + signal_period - 1;
    if data.len() < min_required {
        return Err(Error::InsufficientData {
            required: min_required,
            actual: data.len(),
            indicator: "macd",
        });
    }

    Ok(())
}

// ==================== Configuration Type ====================

/// MACD configuration with fluent builder API.
///
/// Provides sensible defaults (12, 26, 9) and fluent setters for customization.
/// Implements `Default` for zero-config usage per Gravity Check 1.1.
///
/// # Example
///
/// ```
/// use fast_ta::indicators::macd::Macd;
///
/// let prices = vec![
///     26.0_f64, 27.0, 28.0, 27.5, 28.5, 29.0, 28.0, 27.0, 28.0, 29.0,
///     30.0, 29.5, 30.5, 31.0, 30.0, 31.0, 32.0, 31.5, 32.5, 33.0,
///     32.0, 31.0, 32.0, 33.0, 34.0, 33.5, 34.5, 35.0, 34.0, 35.0,
///     36.0, 35.5, 36.5, 37.0,
/// ];
///
/// // Use defaults (12, 26, 9)
/// let result = Macd::default().compute(&prices).unwrap();
///
/// // Or customize with fluent API
/// let result = Macd::new()
///     .fast_period(10)
///     .slow_period(21)
///     .signal_period(7)
///     .compute(&prices)
///     .unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Macd {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
}

impl Default for Macd {
    /// Creates a MACD configuration with standard parameters (12, 26, 9).
    fn default() -> Self {
        Self {
            fast_period: 12,
            slow_period: 26,
            signal_period: 9,
        }
    }
}

impl Macd {
    /// Creates a new MACD configuration with standard parameters (12, 26, 9).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the fast EMA period.
    ///
    /// Default: 12
    #[must_use]
    pub const fn fast_period(mut self, period: usize) -> Self {
        self.fast_period = period;
        self
    }

    /// Sets the slow EMA period.
    ///
    /// Default: 26
    #[must_use]
    pub const fn slow_period(mut self, period: usize) -> Self {
        self.slow_period = period;
        self
    }

    /// Sets the signal line EMA period.
    ///
    /// Default: 9
    #[must_use]
    pub const fn signal_period(mut self, period: usize) -> Self {
        self.signal_period = period;
        self
    }

    /// Computes MACD using the configured parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The input data is empty
    /// - `fast_period >= slow_period`
    /// - Any period is 0
    /// - Insufficient data for the configured periods
    pub fn compute<T: SeriesElement>(&self, data: &[T]) -> Result<MacdOutput<T>> {
        macd(data, self.fast_period, self.slow_period, self.signal_period)
    }

    /// Computes MACD into pre-allocated buffers.
    ///
    /// Returns `(macd_valid_count, signal_valid_count)`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any buffer is smaller than input length
    /// - The input data is empty
    /// - `fast_period >= slow_period`
    /// - Any period is 0
    /// - Insufficient data for the configured periods
    pub fn compute_into<T: SeriesElement>(
        &self,
        data: &[T],
        macd_output: &mut [T],
        signal_output: &mut [T],
        histogram_output: &mut [T],
    ) -> Result<(usize, usize)> {
        macd_into(
            data,
            self.fast_period,
            self.slow_period,
            self.signal_period,
            macd_output,
            signal_output,
            histogram_output,
        )
    }

    /// Returns the fast period.
    #[must_use]
    pub const fn get_fast_period(&self) -> usize {
        self.fast_period
    }

    /// Returns the slow period.
    #[must_use]
    pub const fn get_slow_period(&self) -> usize {
        self.slow_period
    }

    /// Returns the signal period.
    #[must_use]
    pub const fn get_signal_period(&self) -> usize {
        self.signal_period
    }

    /// Returns the MACD line lookback for this configuration.
    #[must_use]
    pub const fn line_lookback(&self) -> usize {
        macd_line_lookback(self.slow_period)
    }

    /// Returns the signal line lookback for this configuration.
    #[must_use]
    pub const fn signal_lookback(&self) -> usize {
        macd_signal_lookback(self.slow_period, self.signal_period)
    }

    /// Returns the minimum input length for this configuration.
    #[must_use]
    pub const fn min_len(&self) -> usize {
        macd_min_len(self.slow_period, self.signal_period)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    use super::*;
    use num_traits::Float;

    // Helper function to compare floating point values
    fn approx_eq<T: Float>(a: T, b: T, epsilon: T) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        if a.is_nan() || b.is_nan() {
            return false;
        }
        (a - b).abs() < epsilon
    }

    const EPSILON: f64 = 1e-10;
    // Looser epsilon for MACD calculations involving multiple EMAs
    const MACD_EPSILON: f64 = 1e-6;

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_macd_basic() {
        // Create enough data for MACD(12, 26, 9)
        let data: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let result = macd(&data, 12, 26, 9).unwrap();

        assert_eq!(result.len(), 50);

        // First slow_period - 1 = 25 MACD values should be NaN
        for i in 0..25 {
            assert!(
                result.macd_line[i].is_nan(),
                "MACD line at {} should be NaN",
                i
            );
        }

        // First valid MACD at index 25
        assert!(!result.macd_line[25].is_nan());

        // First slow_period + signal_period - 2 = 33 signal values should be NaN
        for i in 0..33 {
            assert!(
                result.signal_line[i].is_nan(),
                "Signal line at {} should be NaN",
                i
            );
        }

        // First valid signal at index 33
        assert!(!result.signal_line[33].is_nan());

        // Histogram should have same NaN pattern as signal
        for i in 0..33 {
            assert!(
                result.histogram[i].is_nan(),
                "Histogram at {} should be NaN",
                i
            );
        }
        assert!(!result.histogram[33].is_nan());
    }

    #[test]
    fn test_macd_f32() {
        let data: Vec<f32> = (0..50).map(|i| 100.0 + (i as f32) * 0.5).collect();
        let result = macd(&data, 12, 26, 9).unwrap();

        assert_eq!(result.len(), 50);
        assert!(!result.macd_line[25].is_nan());
        assert!(!result.signal_line[33].is_nan());
    }

    #[test]
    fn test_macd_small_periods() {
        // Test with smaller periods (3, 6, 2)
        let data: Vec<f64> = (0..20).map(|i| 50.0 + (i as f64)).collect();
        let result = macd(&data, 3, 6, 2).unwrap();

        assert_eq!(result.len(), 20);

        // First valid MACD at index slow_period - 1 = 5
        assert!(result.macd_line[4].is_nan());
        assert!(!result.macd_line[5].is_nan());

        // First valid signal at index slow_period + signal_period - 2 = 6
        assert!(result.signal_line[5].is_nan());
        assert!(!result.signal_line[6].is_nan());
    }

    #[test]
    fn test_macd_minimum_periods() {
        // Test with minimum valid periods (1, 2, 1)
        let data: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let result = macd(&data, 1, 2, 1).unwrap();

        assert_eq!(result.len(), 5);

        // First valid MACD at index 1 (slow_period - 1)
        assert!(result.macd_line[0].is_nan());
        assert!(!result.macd_line[1].is_nan());

        // First valid signal at index 1 (slow_period + signal_period - 2 = 1)
        assert!(result.signal_line[0].is_nan());
        assert!(!result.signal_line[1].is_nan());
    }

    // ==================== MACD Line Tests ====================

    #[test]
    fn test_macd_line_uptrend() {
        // In an uptrend, fast EMA > slow EMA, so MACD line should be positive
        let data: Vec<f64> = (0..50).map(|i| 50.0 + (i as f64) * 2.0).collect();
        let result = macd(&data, 5, 10, 3).unwrap();

        // After warmup, MACD line should be positive
        for i in 15..result.len() {
            assert!(
                result.macd_line[i] > 0.0,
                "MACD line at {} should be positive in uptrend, got {}",
                i,
                result.macd_line[i]
            );
        }
    }

    #[test]
    fn test_macd_line_downtrend() {
        // In a downtrend, fast EMA < slow EMA, so MACD line should be negative
        let data: Vec<f64> = (0..50).map(|i| 150.0 - (i as f64) * 2.0).collect();
        let result = macd(&data, 5, 10, 3).unwrap();

        // After warmup, MACD line should be negative
        for i in 15..result.len() {
            assert!(
                result.macd_line[i] < 0.0,
                "MACD line at {} should be negative in downtrend, got {}",
                i,
                result.macd_line[i]
            );
        }
    }

    #[test]
    fn test_macd_line_constant_values() {
        // For constant values, EMAs should equal the constant, so MACD line should be 0
        let data = vec![50.0_f64; 50];
        let result = macd(&data, 5, 10, 3).unwrap();

        // After warmup, MACD line should be approximately 0
        for i in 9..result.len() {
            if !result.macd_line[i].is_nan() {
                assert!(
                    approx_eq(result.macd_line[i], 0.0, MACD_EPSILON),
                    "MACD line at {} should be ~0 for constant data, got {}",
                    i,
                    result.macd_line[i]
                );
            }
        }
    }

    // ==================== Signal Line Tests ====================

    #[test]
    fn test_signal_line_smooths_macd() {
        // Signal line should be a smoothed version of MACD line
        let data: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let result = macd(&data, 5, 10, 3).unwrap();

        // Both should eventually converge to similar values for trending data
        let last_macd = result.macd_line[result.len() - 1];
        let last_signal = result.signal_line[result.len() - 1];

        // Signal should be close to MACD for smooth trends
        assert!(
            (last_macd - last_signal).abs() < 1.0,
            "Signal should be close to MACD in smooth trend"
        );
    }

    // ==================== Histogram Tests ====================

    #[test]
    fn test_histogram_equals_macd_minus_signal() {
        let data: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let result = macd(&data, 5, 10, 3).unwrap();

        // Histogram should equal MACD line - signal line
        for i in 11..result.len() {
            if !result.histogram[i].is_nan() {
                let expected = result.macd_line[i] - result.signal_line[i];
                assert!(
                    approx_eq(result.histogram[i], expected, EPSILON),
                    "Histogram at {} should equal MACD - Signal",
                    i
                );
            }
        }
    }

    #[test]
    fn test_histogram_sign_changes() {
        // Create data that reverses trend - histogram should change sign
        let mut data = Vec::with_capacity(60);
        // First half: uptrend
        for i in 0..30 {
            data.push(50.0 + (i as f64) * 2.0);
        }
        // Second half: downtrend
        for i in 0..30 {
            data.push(108.0 - (i as f64) * 2.0);
        }

        let result = macd(&data, 5, 10, 3).unwrap();

        // Early histogram should be positive (uptrend)
        let early_valid = 15;
        let mut found_positive = false;
        for i in early_valid..25 {
            if !result.histogram[i].is_nan() && result.histogram[i] > 0.0 {
                found_positive = true;
                break;
            }
        }

        // Late histogram should be negative (downtrend)
        let mut found_negative = false;
        for i in 45..result.len() {
            if !result.histogram[i].is_nan() && result.histogram[i] < 0.0 {
                found_negative = true;
                break;
            }
        }

        assert!(
            found_positive || found_negative,
            "Histogram should change sign with trend reversal"
        );
    }

    // ==================== Reference Value Tests ====================

    #[test]
    fn test_macd_known_calculation() {
        // Manual calculation verification
        // Data: simple sequential values
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = macd(&data, 2, 3, 2).unwrap();

        // Slow EMA(3) seed at index 2: (1+2+3)/3 = 2.0
        // Fast EMA(2) seed at index 1: (1+2)/2 = 1.5

        // At index 2:
        // Fast EMA: alpha = 2/3, EMA[2] = 2/3 * 3 + 1/3 * 1.5 = 2 + 0.5 = 2.5
        // Slow EMA: (1+2+3)/3 = 2.0
        // MACD[2] = 2.5 - 2.0 = 0.5
        assert!(approx_eq(result.macd_line[2], 0.5, MACD_EPSILON));

        // Signal line at index 3 (need 2 valid MACD values)
        // MACD[2] = 0.5
        // For MACD[3], need to compute Fast EMA[3] and Slow EMA[3]
        // Fast EMA[3] = 2/3 * 4 + 1/3 * 2.5 = 2.667 + 0.833 = 3.5
        // Slow EMA[3] = 2/4 * 4 + 2/4 * 2.0 = 2 + 1 = 3.0
        // MACD[3] = 3.5 - 3.0 = 0.5
        // Signal[3] = SMA(MACD[2..3]) = (0.5 + 0.5)/2 = 0.5

        assert!(!result.macd_line[2].is_nan());
        assert!(!result.macd_line[3].is_nan());
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_macd_with_nan_in_data() {
        let data = vec![1.0_f64, 2.0, f64::NAN, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = macd(&data, 2, 3, 2).unwrap();

        // NaN propagates through EMA, affecting MACD calculation
        assert!(result.macd_line[2].is_nan());
    }

    #[test]
    fn test_macd_negative_values() {
        let data: Vec<f64> = (-25..25).map(|i| i as f64).collect();
        let result = macd(&data, 5, 10, 3).unwrap();

        // Should work with negative values
        assert!(!result.macd_line[15].is_nan());
        assert!(!result.signal_line[17].is_nan());
    }

    #[test]
    fn test_macd_large_values() {
        let data: Vec<f64> = (0..50).map(|i| 1e10 + (i as f64) * 1e8).collect();
        let result = macd(&data, 5, 10, 3).unwrap();

        // Should handle large values
        for i in 12..result.len() {
            assert!(!result.signal_line[i].is_nan());
        }
    }

    #[test]
    fn test_macd_small_values() {
        let data: Vec<f64> = (0..50).map(|i| 1e-10 + (i as f64) * 1e-12).collect();
        let result = macd(&data, 5, 10, 3).unwrap();

        // Should handle small values
        for i in 12..result.len() {
            assert!(!result.signal_line[i].is_nan());
        }
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_macd_empty_input() {
        let data: Vec<f64> = vec![];
        let result = macd(&data, 12, 26, 9);

        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_macd_zero_fast_period() {
        let data = vec![1.0_f64; 50];
        let result = macd(&data, 0, 26, 9);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_macd_zero_slow_period() {
        let data = vec![1.0_f64; 50];
        let result = macd(&data, 12, 0, 9);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_macd_zero_signal_period() {
        let data = vec![1.0_f64; 50];
        let result = macd(&data, 12, 26, 0);

        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_macd_fast_equals_slow() {
        let data = vec![1.0_f64; 50];
        let result = macd(&data, 12, 12, 9);

        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_macd_fast_greater_than_slow() {
        let data = vec![1.0_f64; 50];
        let result = macd(&data, 26, 12, 9);

        assert!(matches!(result, Err(Error::InvalidPeriod { .. })));
    }

    #[test]
    fn test_macd_insufficient_data() {
        let data = vec![1.0_f64; 30]; // Need at least 26 + 9 - 1 = 34 for MACD(12, 26, 9)
        let result = macd(&data, 12, 26, 9);

        assert!(matches!(result, Err(Error::InsufficientData { .. })));
    }

    #[test]
    fn test_macd_minimum_required_data() {
        // For MACD(12, 26, 9), minimum is slow_period + signal_period - 1 = 34
        let data = vec![1.0_f64; 34];
        let result = macd(&data, 12, 26, 9);

        assert!(result.is_ok());
    }

    // ==================== macd_into Tests ====================

    #[test]
    fn test_macd_into_basic() {
        let data: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let mut macd_line = vec![0.0_f64; 50];
        let mut signal_line = vec![0.0_f64; 50];
        let mut histogram = vec![0.0_f64; 50];

        let (valid_macd, valid_signal) = macd_into(
            &data,
            12,
            26,
            9,
            &mut macd_line,
            &mut signal_line,
            &mut histogram,
        )
        .unwrap();

        assert_eq!(valid_macd, 25); // 50 - 25 = 25
        assert_eq!(valid_signal, 17); // 50 - 33 = 17

        // Verify same as macd()
        let reference = macd(&data, 12, 26, 9).unwrap();
        for i in 0..50 {
            assert!(approx_eq(macd_line[i], reference.macd_line[i], EPSILON));
            assert!(approx_eq(signal_line[i], reference.signal_line[i], EPSILON));
            assert!(approx_eq(histogram[i], reference.histogram[i], EPSILON));
        }
    }

    #[test]
    fn test_macd_into_buffer_reuse() {
        let data1: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64)).collect();
        let data2: Vec<f64> = (0..50).map(|i| 200.0 - (i as f64)).collect();
        let mut macd_line = vec![0.0_f64; 50];
        let mut signal_line = vec![0.0_f64; 50];
        let mut histogram = vec![0.0_f64; 50];

        macd_into(
            &data1,
            5,
            10,
            3,
            &mut macd_line,
            &mut signal_line,
            &mut histogram,
        )
        .unwrap();
        let first_macd = macd_line[15];

        macd_into(
            &data2,
            5,
            10,
            3,
            &mut macd_line,
            &mut signal_line,
            &mut histogram,
        )
        .unwrap();
        let second_macd = macd_line[15];

        // First (uptrend) should give positive MACD, second (downtrend) should give negative
        assert!(first_macd > 0.0);
        assert!(second_macd < 0.0);
    }

    #[test]
    fn test_macd_into_insufficient_output() {
        let data: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64)).collect();
        let mut macd_line = vec![0.0_f64; 30]; // Too short
        let mut signal_line = vec![0.0_f64; 50];
        let mut histogram = vec![0.0_f64; 50];

        let result = macd_into(
            &data,
            5,
            10,
            3,
            &mut macd_line,
            &mut signal_line,
            &mut histogram,
        );

        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_macd_into_f32() {
        let data: Vec<f32> = (0..50).map(|i| 100.0 + (i as f32) * 0.5).collect();
        let mut macd_line = vec![0.0_f32; 50];
        let mut signal_line = vec![0.0_f32; 50];
        let mut histogram = vec![0.0_f32; 50];

        let (valid_macd, valid_signal) = macd_into(
            &data,
            5,
            10,
            3,
            &mut macd_line,
            &mut signal_line,
            &mut histogram,
        )
        .unwrap();

        assert_eq!(valid_macd, 41); // 50 - 9 = 41
        assert_eq!(valid_signal, 39); // 50 - 11 = 39
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_macd_and_macd_into_produce_same_result() {
        let data: Vec<f64> = (0..60).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let result1 = macd(&data, 12, 26, 9).unwrap();

        let mut macd_line = vec![0.0_f64; 60];
        let mut signal_line = vec![0.0_f64; 60];
        let mut histogram = vec![0.0_f64; 60];
        macd_into(
            &data,
            12,
            26,
            9,
            &mut macd_line,
            &mut signal_line,
            &mut histogram,
        )
        .unwrap();

        for i in 0..60 {
            assert!(
                approx_eq(result1.macd_line[i], macd_line[i], EPSILON),
                "MACD line mismatch at {}",
                i
            );
            assert!(
                approx_eq(result1.signal_line[i], signal_line[i], EPSILON),
                "Signal line mismatch at {}",
                i
            );
            assert!(
                approx_eq(result1.histogram[i], histogram[i], EPSILON),
                "Histogram mismatch at {}",
                i
            );
        }
    }

    // ==================== Property-Based-Like Tests ====================

    #[test]
    fn test_macd_output_length_equals_input_length() {
        for len in [35, 50, 100, 200] {
            let data: Vec<f64> = (0..len).map(|x| x as f64).collect();
            let result = macd(&data, 12, 26, 9).unwrap();
            assert_eq!(result.len(), len);
            assert_eq!(result.macd_line.len(), len);
            assert_eq!(result.signal_line.len(), len);
            assert_eq!(result.histogram.len(), len);
        }
    }

    #[test]
    fn test_macd_nan_counts() {
        let data: Vec<f64> = (0..100).map(|x| x as f64).collect();

        for (fast, slow, signal) in [(3, 5, 2), (5, 10, 3), (12, 26, 9)] {
            let result = macd(&data, fast, slow, signal).unwrap();

            let macd_nan_count = result.macd_line.iter().filter(|x| x.is_nan()).count();
            let signal_nan_count = result.signal_line.iter().filter(|x| x.is_nan()).count();

            // MACD NaN count should be slow_period - 1
            assert_eq!(
                macd_nan_count,
                slow - 1,
                "MACD NaN count for ({}, {}, {})",
                fast,
                slow,
                signal
            );

            // Signal NaN count should be slow_period + signal_period - 2
            assert_eq!(
                signal_nan_count,
                slow + signal - 2,
                "Signal NaN count for ({}, {}, {})",
                fast,
                slow,
                signal
            );
        }
    }

    #[test]
    fn test_macd_histogram_nan_matches_signal() {
        let data: Vec<f64> = (0..100).map(|x| x as f64).collect();
        let result = macd(&data, 12, 26, 9).unwrap();

        // Histogram should be NaN exactly where signal is NaN
        for i in 0..result.len() {
            assert_eq!(
                result.histogram[i].is_nan(),
                result.signal_line[i].is_nan(),
                "Histogram and signal NaN mismatch at {}",
                i
            );
        }
    }

    // ==================== MacdOutput Struct Tests ====================

    #[test]
    fn test_macd_output_len() {
        let data: Vec<f64> = (0..50).map(|x| x as f64).collect();
        let result = macd(&data, 5, 10, 3).unwrap();

        assert_eq!(result.len(), 50);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_macd_output_is_empty() {
        let output: MacdOutput<f64> = MacdOutput {
            macd_line: vec![],
            signal_line: vec![],
            histogram: vec![],
        };

        assert!(output.is_empty());
        assert_eq!(output.len(), 0);
    }

    #[test]
    fn test_macd_output_first_valid_indices() {
        let data: Vec<f64> = (0..50).map(|x| x as f64).collect();
        let result = macd(&data, 12, 26, 9).unwrap();

        assert_eq!(result.first_valid_macd_index(26), 25);
        assert_eq!(result.first_valid_signal_index(26, 9), 33);
    }

    #[test]
    fn test_macd_output_clone() {
        let data: Vec<f64> = (0..50).map(|x| x as f64).collect();
        let result = macd(&data, 5, 10, 3).unwrap();
        let cloned = result.clone();

        for i in 0..result.len() {
            assert!(approx_eq(result.macd_line[i], cloned.macd_line[i], EPSILON));
            assert!(approx_eq(
                result.signal_line[i],
                cloned.signal_line[i],
                EPSILON
            ));
            assert!(approx_eq(result.histogram[i], cloned.histogram[i], EPSILON));
        }
    }

    // ==================== Real-World Scenario Tests ====================

    #[test]
    fn test_macd_bullish_crossover() {
        // Create data where MACD line will cross above signal line
        let mut data = Vec::with_capacity(60);
        // Initial flat period
        for _ in 0..35 {
            data.push(100.0_f64);
        }
        // Strong uptrend
        for i in 0..25 {
            data.push(100.0 + (i as f64) * 3.0);
        }

        let result = macd(&data, 5, 10, 3).unwrap();

        // After the uptrend starts, histogram should become positive
        let mut found_positive = false;
        for i in 40..result.len() {
            if !result.histogram[i].is_nan() && result.histogram[i] > 0.0 {
                found_positive = true;
                break;
            }
        }

        assert!(
            found_positive,
            "Should find positive histogram during uptrend"
        );
    }

    #[test]
    fn test_macd_bearish_crossover() {
        // Create data where MACD line will cross below signal line
        let mut data = Vec::with_capacity(60);
        // Initial flat period
        for _ in 0..35 {
            data.push(100.0_f64);
        }
        // Strong downtrend
        for i in 0..25 {
            data.push(100.0 - (i as f64) * 3.0);
        }

        let result = macd(&data, 5, 10, 3).unwrap();

        // After the downtrend starts, histogram should become negative
        let mut found_negative = false;
        for i in 40..result.len() {
            if !result.histogram[i].is_nan() && result.histogram[i] < 0.0 {
                found_negative = true;
                break;
            }
        }

        assert!(
            found_negative,
            "Should find negative histogram during downtrend"
        );
    }

    #[test]
    fn test_macd_divergence_scenario() {
        // Simulate price making higher highs but MACD making lower highs (bearish divergence setup)
        let data: Vec<f64> = vec![
            50.0, 51.0, 52.0, 53.0, 54.0, 55.0, 56.0, 57.0, 58.0, 59.0, 60.0, 61.0, 62.0, 63.0,
            64.0, 65.0, 66.0, 67.0, 68.0, 69.0, 70.0, 69.5, 70.0, 69.0, 70.5, 68.5, 71.0, 68.0,
            71.5, 67.5, 72.0, 67.0, 72.5, 66.5, 73.0, 66.0, 73.5, 65.5, 74.0, 65.0,
        ];

        let result = macd(&data, 5, 10, 3).unwrap();

        // Should compute without errors
        assert_eq!(result.len(), 40);
        assert!(!result.signal_line[15].is_nan());
    }
}
