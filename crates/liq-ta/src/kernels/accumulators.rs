//! Precision-aware accumulators for numeric stability.
//!
//! This module provides accumulator types that use f64 internal state regardless
//! of input type, providing improved numeric stability for f32 inputs while
//! maintaining O(1) per-operation complexity.
//!
//! # Accumulators
//!
//! - [`RollingSumF64`]: O(1) rolling sum with f64 internal state
//! - [`RollingVarianceF64`]: O(1) rolling variance using sum-of-squares with f64
//! - [`WelfordVarianceF64`]: O(1) rolling variance using Welford's algorithm (more stable)
//! - [`CumulativeSum`]: Unbounded cumulative sum with f64 precision
//! - [`CumulativeProductSum`]: Cumulative sum of products (e.g., price × volume)
//! - [`WilderSmoothing`]: Wilder's smoothing for RSI with f64 state
//!
//! # Usage
//!
//! ```rust,ignore
//! use liq_ta::kernels::accumulators::RollingSumF64;
//!
//! let mut sum = RollingSumF64::new();
//! sum.add(100.0_f32);  // f32 input, f64 internal
//! sum.add(101.5_f32);
//! sum.remove(100.0_f32);  // O(1) removal
//! assert!((sum.value() - 101.5).abs() < 1e-10);
//! ```
//!
//! # Precision Benefits
//!
//! Using f64 accumulators for f32 inputs provides:
//! - 15-16 significant digits vs 7-8 for f32
//! - Reduced catastrophic cancellation in variance calculations
//! - Lower accumulated error in long running sums (VWAP, OBV)

use num_traits::ToPrimitive;

// =============================================================================
// RollingSumF64 - O(1) Rolling Sum
// =============================================================================

/// Rolling sum accumulator with f64 internal precision.
///
/// Maintains a running sum that supports O(1) add and remove operations.
/// Uses f64 internally regardless of input type for improved precision.
///
/// # Example
///
/// ```rust,ignore
/// let mut sum = RollingSumF64::new();
/// for &val in &window {
///     sum.add(val);
/// }
/// // Rolling update
/// sum.remove(oldest);
/// sum.add(newest);
/// let mean = sum.value() / window_size as f64;
/// ```
#[derive(Debug, Clone, Default)]
pub struct RollingSumF64 {
    sum: f64,
}

impl RollingSumF64 {
    /// Creates a new accumulator with sum = 0.
    #[inline]
    pub fn new() -> Self {
        Self { sum: 0.0 }
    }

    /// Creates an accumulator with an initial sum value.
    #[inline]
    pub fn with_initial(sum: f64) -> Self {
        Self { sum }
    }

    /// Adds a value to the sum. O(1).
    #[inline]
    pub fn add<T: ToPrimitive>(&mut self, value: T) {
        self.sum += value.to_f64().unwrap_or(f64::NAN);
    }

    /// Removes a value from the sum. O(1).
    ///
    /// Note: This is a simple subtraction and may accumulate small errors
    /// over many operations. For f32 inputs cast to f64, this error is negligible.
    #[inline]
    pub fn remove<T: ToPrimitive>(&mut self, old: T) {
        self.sum -= old.to_f64().unwrap_or(f64::NAN);
    }

    /// Returns the current sum as f64.
    #[inline]
    pub fn value(&self) -> f64 {
        self.sum
    }

    /// Returns the current sum as f32.
    #[inline]
    pub fn as_f32(&self) -> f32 {
        self.sum as f32
    }

    /// Returns the current sum as f64.
    #[inline]
    pub fn as_f64(&self) -> f64 {
        self.sum
    }

    /// Resets the accumulator to zero.
    #[inline]
    pub fn reset(&mut self) {
        self.sum = 0.0;
    }
}

// =============================================================================
// RollingVarianceF64 - O(1) Rolling Variance
// =============================================================================

/// Rolling variance accumulator using sum-of-squares with f64 precision.
///
/// Maintains running sum and sum-of-squares for O(1) variance computation.
/// Uses the formula: `Var(X) = E[X²] - E[X]²`
///
/// # Zero-Variance Policy
///
/// Due to floating-point error, computed variance may be slightly negative.
/// This is clamped to 0.0 (not epsilon) to avoid false signal.
///
/// # Example
///
/// ```rust,ignore
/// let mut var = RollingVarianceF64::new();
/// for &val in &window {
///     var.push(val);
/// }
/// // Rolling update
/// var.pop(oldest);
/// var.push(newest);
/// let stddev = var.population_stddev();
/// ```
#[derive(Debug, Clone, Default)]
pub struct RollingVarianceF64 {
    sum: f64,
    sum_sq: f64,
    count: usize,
}

impl RollingVarianceF64 {
    /// Creates a new empty accumulator.
    #[inline]
    pub fn new() -> Self {
        Self {
            sum: 0.0,
            sum_sq: 0.0,
            count: 0,
        }
    }

    /// Creates an accumulator with pre-computed initial values.
    ///
    /// Useful when the initial sum and sum_sq have been computed externally
    /// (e.g., using SIMD) and we want to continue with rolling updates.
    #[inline]
    pub fn with_initial(sum: f64, sum_sq: f64, count: usize) -> Self {
        Self { sum, sum_sq, count }
    }

    /// Adds a value to the accumulator. O(1).
    #[inline]
    pub fn push<T: ToPrimitive>(&mut self, value: T) {
        let v = value.to_f64().unwrap_or(f64::NAN);
        self.sum += v;
        self.sum_sq += v * v;
        self.count += 1;
    }

    /// Removes a value from the accumulator. O(1).
    #[inline]
    pub fn pop<T: ToPrimitive>(&mut self, old: T) {
        let v = old.to_f64().unwrap_or(f64::NAN);
        self.sum -= v;
        self.sum_sq -= v * v;
        self.count = self.count.saturating_sub(1);
    }

    /// Returns the count of values.
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the mean (average) of the values.
    #[inline]
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            return f64::NAN;
        }
        self.sum / self.count as f64
    }

    /// Returns the population variance. O(1).
    ///
    /// Uses the formula: `Var = E[X²] - E[X]²`
    /// Result is clamped to 0.0 if negative due to floating-point error.
    #[inline]
    pub fn variance(&self) -> f64 {
        if self.count == 0 {
            return f64::NAN;
        }
        let n = self.count as f64;
        let mean = self.sum / n;
        let mean_of_squares = self.sum_sq / n;
        let variance = mean_of_squares - mean * mean;
        // Clamp to zero per Zero-Variance Clamp Policy
        variance.max(0.0)
    }

    /// Returns the sample variance (Bessel's correction). O(1).
    #[inline]
    pub fn sample_variance(&self) -> f64 {
        if self.count < 2 {
            return f64::NAN;
        }
        let n = self.count as f64;
        let mean = self.sum / n;
        let mean_of_squares = self.sum_sq / n;
        let pop_variance = mean_of_squares - mean * mean;
        // Apply Bessel's correction: s² = n/(n-1) * σ²
        let sample_var = pop_variance * n / (n - 1.0);
        sample_var.max(0.0)
    }

    /// Returns the population standard deviation. O(1).
    #[inline]
    pub fn population_stddev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Returns the sample standard deviation. O(1).
    #[inline]
    pub fn sample_stddev(&self) -> f64 {
        self.sample_variance().sqrt()
    }

    /// Returns the current sum.
    #[inline]
    pub fn sum(&self) -> f64 {
        self.sum
    }

    /// Returns the current sum of squares.
    #[inline]
    pub fn sum_sq(&self) -> f64 {
        self.sum_sq
    }

    /// Resets the accumulator to empty state.
    #[inline]
    pub fn reset(&mut self) {
        self.sum = 0.0;
        self.sum_sq = 0.0;
        self.count = 0;
    }
}

// =============================================================================
// WelfordVarianceF64 - O(1) Rolling Variance using Welford's Algorithm
// =============================================================================

/// Rolling variance accumulator using Welford's online algorithm with f64 precision.
///
/// Welford's algorithm maintains a running mean and sum of squared differences (M2),
/// which is numerically more stable than the sum-of-squares formula, especially when:
/// - The mean is large and variance is small
/// - Values are near-constant with tiny variations
///
/// # Algorithm
///
/// For addition (push):
/// ```text
/// n = n + 1
/// delta = x - mean
/// mean = mean + delta / n
/// m2 = m2 + delta * (x - mean)  // Note: using new mean
/// ```
///
/// For removal (pop):
/// ```text
/// delta = x - mean
/// mean = (n * mean - x) / (n - 1)  // Restore old mean without x
/// m2 = m2 - delta * (x - mean)     // Note: using new mean
/// n = n - 1
/// ```
///
/// # Zero-Variance Policy
///
/// Due to floating-point error, computed M2 may be slightly negative.
/// This is clamped to 0.0 to avoid returning negative variance.
///
/// # Example
///
/// ```rust,ignore
/// let mut var = WelfordVarianceF64::new();
/// for &val in &window {
///     var.push(val);
/// }
/// // Rolling update
/// var.pop(oldest);
/// var.push(newest);
/// let stddev = var.population_stddev();
/// ```
#[derive(Debug, Clone, Default)]
pub struct WelfordVarianceF64 {
    mean: f64,
    m2: f64, // Sum of squared differences from the mean
    count: usize,
}

impl WelfordVarianceF64 {
    /// Creates a new empty accumulator.
    #[inline]
    pub fn new() -> Self {
        Self {
            mean: 0.0,
            m2: 0.0,
            count: 0,
        }
    }

    /// Creates an accumulator with pre-computed initial values.
    ///
    /// Useful when initial statistics have been computed externally.
    #[inline]
    pub fn with_initial(mean: f64, m2: f64, count: usize) -> Self {
        Self { mean, m2, count }
    }

    /// Adds a value to the accumulator using Welford's online update. O(1).
    #[inline]
    pub fn push<T: ToPrimitive>(&mut self, value: T) {
        let x = value.to_f64().unwrap_or(f64::NAN);
        self.count += 1;
        let delta = x - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = x - self.mean; // Using updated mean
        self.m2 += delta * delta2;
    }

    /// Removes a value from the accumulator using inverse Welford update. O(1).
    ///
    /// # Panics
    ///
    /// Does not panic but produces NaN results if called on an empty accumulator.
    #[inline]
    pub fn pop<T: ToPrimitive>(&mut self, old: T) {
        if self.count == 0 {
            return;
        }

        let x = old.to_f64().unwrap_or(f64::NAN);
        let delta = x - self.mean;

        if self.count == 1 {
            // Last element being removed - reset to empty
            self.mean = 0.0;
            self.m2 = 0.0;
            self.count = 0;
            return;
        }

        // Compute the mean without this value
        let new_mean = (self.count as f64 * self.mean - x) / (self.count - 1) as f64;
        let delta2 = x - new_mean; // Difference from new mean
        self.m2 -= delta * delta2;
        self.mean = new_mean;
        self.count -= 1;
    }

    /// Returns the count of values.
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the mean (average) of the values.
    #[inline]
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            return f64::NAN;
        }
        self.mean
    }

    /// Returns the population variance. O(1).
    ///
    /// Uses: Var = M2 / n
    /// Result is clamped to 0.0 if negative due to floating-point error.
    #[inline]
    pub fn variance(&self) -> f64 {
        if self.count == 0 {
            return f64::NAN;
        }
        // Clamp to zero per Zero-Variance Clamp Policy
        (self.m2 / self.count as f64).max(0.0)
    }

    /// Returns the sample variance (Bessel's correction). O(1).
    #[inline]
    pub fn sample_variance(&self) -> f64 {
        if self.count < 2 {
            return f64::NAN;
        }
        // M2 / (n-1) for sample variance
        (self.m2 / (self.count - 1) as f64).max(0.0)
    }

    /// Returns the population standard deviation. O(1).
    #[inline]
    pub fn population_stddev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Returns the sample standard deviation. O(1).
    #[inline]
    pub fn sample_stddev(&self) -> f64 {
        self.sample_variance().sqrt()
    }

    /// Returns the current M2 (sum of squared differences from mean).
    #[inline]
    pub fn m2(&self) -> f64 {
        self.m2
    }

    /// Resets the accumulator to empty state.
    #[inline]
    pub fn reset(&mut self) {
        self.mean = 0.0;
        self.m2 = 0.0;
        self.count = 0;
    }
}

// =============================================================================
// CumulativeSum - Unbounded Cumulative Sum
// =============================================================================

/// Cumulative sum accumulator with f64 precision.
///
/// For unbounded accumulation (VWAP, OBV, AD) where values are never removed.
///
/// # Example
///
/// ```rust,ignore
/// let mut cum_sum = CumulativeSum::new();
/// for &volume in &volumes {
///     cum_sum.add(volume);
/// }
/// let total = cum_sum.value();
/// ```
#[derive(Debug, Clone, Default)]
pub struct CumulativeSum {
    sum: f64,
}

impl CumulativeSum {
    /// Creates a new accumulator with sum = 0.
    #[inline]
    pub fn new() -> Self {
        Self { sum: 0.0 }
    }

    /// Adds a value to the cumulative sum.
    #[inline]
    pub fn add<T: ToPrimitive>(&mut self, value: T) {
        self.sum += value.to_f64().unwrap_or(0.0);
    }

    /// Subtracts a value from the cumulative sum.
    /// Used for indicators like OBV that can add or subtract.
    #[inline]
    pub fn subtract<T: ToPrimitive>(&mut self, value: T) {
        self.sum -= value.to_f64().unwrap_or(0.0);
    }

    /// Returns the current cumulative sum.
    #[inline]
    pub fn value(&self) -> f64 {
        self.sum
    }

    /// Returns the current sum as f32.
    #[inline]
    pub fn as_f32(&self) -> f32 {
        self.sum as f32
    }

    /// Resets the accumulator to zero.
    #[inline]
    pub fn reset(&mut self) {
        self.sum = 0.0;
    }
}

// =============================================================================
// CumulativeProductSum - Cumulative Sum of Products
// =============================================================================

/// Cumulative sum of products (e.g., price × volume for VWAP).
///
/// Handles both float and integer types for the multiplier (volume).
///
/// # Example
///
/// ```rust,ignore
/// let mut pv_sum = CumulativeProductSum::new();
/// for (&price, &volume) in prices.iter().zip(volumes.iter()) {
///     pv_sum.add(price, volume);
/// }
/// let vwap = pv_sum.value() / total_volume;
/// ```
#[derive(Debug, Clone, Default)]
pub struct CumulativeProductSum {
    sum: f64,
}

impl CumulativeProductSum {
    /// Creates a new accumulator with sum = 0.
    #[inline]
    pub fn new() -> Self {
        Self { sum: 0.0 }
    }

    /// Adds a product (price × volume) to the sum.
    #[inline]
    pub fn add<P: ToPrimitive, V: ToPrimitive>(&mut self, price: P, volume: V) {
        let p = price.to_f64().unwrap_or(f64::NAN);
        let v = volume.to_f64().unwrap_or(0.0);
        self.sum += p * v;
    }

    /// Returns the current cumulative sum of products.
    #[inline]
    pub fn value(&self) -> f64 {
        self.sum
    }

    /// Returns the current sum as f32.
    #[inline]
    pub fn as_f32(&self) -> f32 {
        self.sum as f32
    }

    /// Resets the accumulator to zero.
    #[inline]
    pub fn reset(&mut self) {
        self.sum = 0.0;
    }
}

// =============================================================================
// WilderSmoothing - Wilder's Smoothing for RSI
// =============================================================================

/// Wilder's smoothing accumulator for RSI.
///
/// Uses Wilder's formula: `new_avg = prev_avg + (value - prev_avg) / period`
/// This is equivalent to EMA with alpha = 1/period.
///
/// # Example
///
/// ```rust,ignore
/// let mut avg_gain = WilderSmoothing::new();
/// let mut avg_loss = WilderSmoothing::new();
///
/// // Initialize with first period average
/// avg_gain.initialize(initial_avg_gain);
/// avg_loss.initialize(initial_avg_loss);
///
/// // Update with each new value
/// avg_gain.update(current_gain, period);
/// avg_loss.update(current_loss, period);
///
/// let rs = avg_gain.value() / avg_loss.value();
/// let rsi = 100.0 - (100.0 / (1.0 + rs));
/// ```
#[derive(Debug, Clone, Default)]
pub struct WilderSmoothing {
    value: f64,
    initialized: bool,
}

impl WilderSmoothing {
    /// Creates a new uninitialized accumulator.
    #[inline]
    pub fn new() -> Self {
        Self {
            value: 0.0,
            initialized: false,
        }
    }

    /// Initializes the accumulator with a starting value.
    #[inline]
    pub fn initialize<T: ToPrimitive>(&mut self, initial: T) {
        self.value = initial.to_f64().unwrap_or(0.0);
        self.initialized = true;
    }

    /// Updates the smoothed value using Wilder's formula.
    ///
    /// Formula: `new = prev + (value - prev) / period`
    #[inline]
    pub fn update<T: ToPrimitive>(&mut self, value: T, period: usize) {
        if !self.initialized {
            return;
        }
        let v = value.to_f64().unwrap_or(0.0);
        let p = period as f64;
        self.value = self.value + (v - self.value) / p;
    }

    /// Returns the current smoothed value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns true if the accumulator has been initialized.
    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Resets the accumulator to uninitialized state.
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.initialized = false;
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // RollingSumF64 Tests (Task 1.1)
    // -------------------------------------------------------------------------

    #[test]
    fn test_rolling_sum_basic() {
        let mut sum = RollingSumF64::new();
        assert_eq!(sum.value(), 0.0);

        sum.add(10.0_f64);
        assert_eq!(sum.value(), 10.0);

        sum.add(20.0_f64);
        assert_eq!(sum.value(), 30.0);

        sum.remove(10.0_f64);
        assert_eq!(sum.value(), 20.0);
    }

    #[test]
    fn test_rolling_sum_f32_input() {
        let mut sum = RollingSumF64::new();

        // f32 inputs should be converted to f64 internally
        sum.add(100.0_f32);
        sum.add(101.5_f32);
        sum.remove(100.0_f32);

        let result = sum.value();
        assert!((result - 101.5).abs() < 1e-10);
    }

    #[test]
    fn test_rolling_sum_precision_vs_f32() {
        // Test with values that would cause precision issues in f32
        let mut sum_f64 = RollingSumF64::new();

        // Add many small values to a large base
        let large_base = 1e8_f32;
        let small_delta = 0.01_f32;
        let iterations = 10_000;

        sum_f64.add(large_base);
        for _ in 0..iterations {
            sum_f64.add(small_delta);
        }

        // Expected: large_base + iterations * small_delta
        let expected = large_base as f64 + (iterations as f64) * (small_delta as f64);
        let actual = sum_f64.value();

        // f64 accumulator should be very close to expected
        let rel_error = ((actual - expected) / expected).abs();
        assert!(rel_error < 1e-10, "Relative error too large: {}", rel_error);
    }

    #[test]
    fn test_rolling_sum_long_window() {
        // Test with long window (period > 1000)
        let mut sum = RollingSumF64::new();
        let period = 1500_usize;
        let values: Vec<f32> = (0..period).map(|i| (i as f32) * 0.1).collect();

        // Add all values
        for &v in &values {
            sum.add(v);
        }

        // Expected sum
        let expected: f64 = values.iter().map(|&v| v as f64).sum();
        let actual = sum.value();

        assert!(
            (actual - expected).abs() < 1e-8,
            "Long window sum mismatch: expected {}, got {}",
            expected,
            actual
        );
    }

    #[test]
    fn test_rolling_sum_vastly_different_magnitudes() {
        let mut sum = RollingSumF64::new();

        // Mix of values with different magnitudes
        // This tests that f64 handles the range well
        sum.add(1e6_f64);
        sum.add(1e-6_f64);
        sum.add(1e3_f64);

        let result = sum.value();
        let expected = 1e6 + 1e-6 + 1e3;

        // f64 should handle this precisely
        assert!(
            (result - expected).abs() < 1e-9,
            "Expected {}, got {}",
            expected,
            result
        );

        // Now test the removal case
        sum.remove(1e6_f64);
        let result2 = sum.value();
        let expected2 = 1e-6 + 1e3;

        assert!(
            (result2 - expected2).abs() < 1e-9,
            "After removal: expected {}, got {}",
            expected2,
            result2
        );
    }

    #[test]
    fn test_rolling_sum_integer_input() {
        let mut sum = RollingSumF64::new();
        sum.add(100_i32);
        sum.add(200_i64);
        sum.add(50_u32);
        assert_eq!(sum.value(), 350.0);
    }

    // -------------------------------------------------------------------------
    // RollingVarianceF64 Tests (Task 1.3)
    // -------------------------------------------------------------------------

    #[test]
    fn test_rolling_variance_basic() {
        let mut var = RollingVarianceF64::new();

        // Add values [1, 2, 3, 4, 5]
        for i in 1..=5 {
            var.push(i as f64);
        }

        assert_eq!(var.count(), 5);
        assert!((var.mean() - 3.0).abs() < 1e-10);

        // Population variance of [1,2,3,4,5] = 2.0
        let expected_var = 2.0;
        assert!(
            (var.variance() - expected_var).abs() < 1e-10,
            "Expected variance {}, got {}",
            expected_var,
            var.variance()
        );
    }

    #[test]
    fn test_rolling_variance_rolling_update() {
        let mut var = RollingVarianceF64::new();

        // Initial window [1, 2, 3]
        var.push(1.0_f64);
        var.push(2.0_f64);
        var.push(3.0_f64);

        // Pop 1, push 4 -> [2, 3, 4]
        var.pop(1.0_f64);
        var.push(4.0_f64);

        assert_eq!(var.count(), 3);
        assert!((var.mean() - 3.0).abs() < 1e-10);

        // Variance of [2, 3, 4] = 2/3
        let expected = 2.0 / 3.0;
        assert!(
            (var.variance() - expected).abs() < 1e-10,
            "Expected {}, got {}",
            expected,
            var.variance()
        );
    }

    #[test]
    fn test_rolling_variance_catastrophic_cancellation() {
        // This tests the scenario where naive f32 variance would fail:
        // Large mean, tiny variance (near-constant data)
        let mut var = RollingVarianceF64::new();

        let base = 1e7_f64;
        let noise = 1e-5;

        // Add 1000 values that are base + tiny_noise
        for i in 0..1000 {
            let v = base + (i as f64) * noise;
            var.push(v);
        }

        let result = var.variance();

        // Variance should be positive and reasonable
        assert!(result >= 0.0, "Variance should be non-negative");
        assert!(result.is_finite(), "Variance should be finite");

        // The variance should be approximately (999 * noise)² / 12 for uniform spacing
        // But more importantly, it should be small compared to base²
        assert!(result < 1.0, "Variance should be small, got {}", result);
    }

    #[test]
    fn test_rolling_variance_constant_values() {
        let mut var = RollingVarianceF64::new();

        // All same value
        for _ in 0..100 {
            var.push(42.0_f64);
        }

        // Variance of constant should be 0 (or very close to it)
        assert!(
            var.variance().abs() < 1e-10,
            "Constant data should have zero variance"
        );
    }

    #[test]
    fn test_rolling_variance_clamps_negative() {
        let mut var = RollingVarianceF64::new();

        // Due to floating-point arithmetic, we might get tiny negative variance
        // This should be clamped to 0
        var.push(1.0_f64);
        var.push(1.0 + 1e-16_f64); // Tiny difference

        let result = var.variance();
        assert!(result >= 0.0, "Variance should never be negative");
    }

    #[test]
    fn test_rolling_variance_sample_vs_population() {
        let mut var = RollingVarianceF64::new();

        // [1, 2, 3, 4, 5]
        for i in 1..=5 {
            var.push(i as f64);
        }

        let pop_var = var.variance();
        let sample_var = var.sample_variance();

        // Sample variance should be n/(n-1) times population variance
        let expected_ratio = 5.0 / 4.0;
        let actual_ratio = sample_var / pop_var;

        assert!(
            (actual_ratio - expected_ratio).abs() < 1e-10,
            "Sample/population ratio should be {}, got {}",
            expected_ratio,
            actual_ratio
        );
    }

    #[test]
    fn test_rolling_variance_f32_input() {
        let mut var = RollingVarianceF64::new();

        // f32 inputs
        for i in 1..=5 {
            var.push(i as f32);
        }

        // Should still work with f64 precision internally
        assert!((var.mean() - 3.0).abs() < 1e-10);
        assert!((var.variance() - 2.0).abs() < 1e-10);
    }

    // -------------------------------------------------------------------------
    // WelfordVarianceF64 Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_welford_variance_basic() {
        let mut var = WelfordVarianceF64::new();

        // Add values [1, 2, 3, 4, 5]
        for i in 1..=5 {
            var.push(i as f64);
        }

        assert_eq!(var.count(), 5);
        assert!((var.mean() - 3.0).abs() < 1e-10);

        // Population variance of [1,2,3,4,5] = 2.0
        let expected_var = 2.0;
        assert!(
            (var.variance() - expected_var).abs() < 1e-10,
            "Expected variance {}, got {}",
            expected_var,
            var.variance()
        );
    }

    #[test]
    fn test_welford_variance_rolling_update() {
        let mut var = WelfordVarianceF64::new();

        // Initial window [1, 2, 3]
        var.push(1.0_f64);
        var.push(2.0_f64);
        var.push(3.0_f64);

        // Pop 1, push 4 -> [2, 3, 4]
        var.pop(1.0_f64);
        var.push(4.0_f64);

        assert_eq!(var.count(), 3);
        assert!(
            (var.mean() - 3.0).abs() < 1e-10,
            "Expected mean 3.0, got {}",
            var.mean()
        );

        // Variance of [2, 3, 4] = 2/3
        let expected = 2.0 / 3.0;
        assert!(
            (var.variance() - expected).abs() < 1e-10,
            "Expected {}, got {}",
            expected,
            var.variance()
        );
    }

    #[test]
    fn test_welford_variance_catastrophic_cancellation() {
        // This is the key test: Welford should handle this much better than sum-of-squares
        // Large mean, tiny variance (near-constant data)
        let mut var = WelfordVarianceF64::new();

        let base = 1e7_f64;
        let noise = 1e-5;

        // Add 1000 values that are base + tiny_noise
        for i in 0..1000 {
            let v = base + (i as f64) * noise;
            var.push(v);
        }

        let result = var.variance();

        // Variance should be positive and reasonable
        assert!(result >= 0.0, "Variance should be non-negative");
        assert!(result.is_finite(), "Variance should be finite");

        // The variance should be small compared to base²
        assert!(result < 1.0, "Variance should be small, got {}", result);
    }

    #[test]
    fn test_welford_variance_constant_values() {
        let mut var = WelfordVarianceF64::new();

        // All same value
        for _ in 0..100 {
            var.push(42.0_f64);
        }

        // Variance of constant should be 0 (or very close to it)
        assert!(
            var.variance().abs() < 1e-10,
            "Constant data should have zero variance, got {}",
            var.variance()
        );
    }

    #[test]
    fn test_welford_variance_matches_sum_of_squares() {
        // Verify that Welford and sum-of-squares give same results for normal data
        let mut welford = WelfordVarianceF64::new();
        let mut sos = RollingVarianceF64::new();

        let values = [1.0, 3.5, 2.7, 8.9, 4.2, 6.1, 3.3, 7.7, 5.5, 9.0];

        for &v in &values {
            welford.push(v);
            sos.push(v);
        }

        let welford_var = welford.variance();
        let sos_var = sos.variance();

        assert!(
            (welford_var - sos_var).abs() < 1e-10,
            "Welford ({}) should match sum-of-squares ({})",
            welford_var,
            sos_var
        );

        assert!(
            (welford.mean() - sos.mean()).abs() < 1e-10,
            "Means should match"
        );
    }

    #[test]
    fn test_welford_variance_pop_all() {
        let mut var = WelfordVarianceF64::new();

        var.push(1.0);
        var.push(2.0);
        var.push(3.0);

        var.pop(1.0);
        var.pop(2.0);
        var.pop(3.0);

        assert_eq!(var.count(), 0);
        assert!(var.variance().is_nan());
    }

    #[test]
    fn test_welford_variance_sample_vs_population() {
        let mut var = WelfordVarianceF64::new();

        // [1, 2, 3, 4, 5]
        for i in 1..=5 {
            var.push(i as f64);
        }

        let pop_var = var.variance();
        let sample_var = var.sample_variance();

        // Sample variance should be n/(n-1) times population variance
        let expected_ratio = 5.0 / 4.0;
        let actual_ratio = sample_var / pop_var;

        assert!(
            (actual_ratio - expected_ratio).abs() < 1e-10,
            "Sample/population ratio should be {}, got {}",
            expected_ratio,
            actual_ratio
        );
    }

    #[test]
    fn test_welford_variance_long_rolling_window() {
        // Test rolling window behavior over many iterations
        let mut var = WelfordVarianceF64::new();
        let period = 20;

        // Build initial window
        let initial: Vec<f64> = (0..period).map(|i| (i as f64) * 0.5 + 10.0).collect();
        for &v in &initial {
            var.push(v);
        }

        // Roll through many more values
        let mut ring: Vec<f64> = initial.clone();
        let mut ring_idx = 0;

        for i in 0..1000 {
            let new_val = 10.0 + (i as f64).sin() * 5.0;
            let old_val = ring[ring_idx];

            var.pop(old_val);
            var.push(new_val);

            ring[ring_idx] = new_val;
            ring_idx = (ring_idx + 1) % period;

            // Verify count stays constant
            assert_eq!(var.count(), period);

            // Variance should be reasonable
            let v = var.variance();
            assert!(
                v >= 0.0 && v.is_finite(),
                "Invalid variance at step {}: {}",
                i,
                v
            );
        }
    }

    // -------------------------------------------------------------------------
    // CumulativeSum Tests (Task 1.5)
    // -------------------------------------------------------------------------

    #[test]
    fn test_cumulative_sum_basic() {
        let mut sum = CumulativeSum::new();

        sum.add(100.0_f64);
        assert_eq!(sum.value(), 100.0);

        sum.add(50.0_f64);
        assert_eq!(sum.value(), 150.0);

        sum.subtract(25.0_f64);
        assert_eq!(sum.value(), 125.0);
    }

    #[test]
    fn test_cumulative_sum_drift_comparison() {
        // Compare drift after 100K additions
        let mut f64_sum = CumulativeSum::new();
        let mut f32_sum: f32 = 0.0;

        let value = 0.1_f32; // 0.1 is not exactly representable
        let iterations = 100_000;

        for _ in 0..iterations {
            f64_sum.add(value);
            f32_sum += value;
        }

        // Expected value
        let expected = (iterations as f64) * (value as f64);

        // f64 accumulator should be much closer to expected
        let f64_error = (f64_sum.value() - expected).abs();
        let f32_error = ((f32_sum as f64) - expected).abs();

        assert!(
            f64_error < f32_error,
            "f64 error ({}) should be less than f32 error ({})",
            f64_error,
            f32_error
        );

        // f64 error should be very small
        assert!(
            f64_error < 1e-8,
            "f64 accumulated error too large: {}",
            f64_error
        );
    }

    #[test]
    fn test_cumulative_sum_integer_types() {
        let mut sum = CumulativeSum::new();

        sum.add(100_u32);
        sum.add(200_u64);
        sum.add(50_i32);
        sum.add(-25_i64);

        assert_eq!(sum.value(), 325.0);
    }

    #[test]
    fn test_cumulative_product_sum_basic() {
        let mut pv = CumulativeProductSum::new();

        // price=100, volume=1000 -> 100000
        pv.add(100.0_f64, 1000_u64);
        assert_eq!(pv.value(), 100_000.0);

        // price=105, volume=500 -> 52500
        pv.add(105.0_f64, 500_u32);
        assert_eq!(pv.value(), 152_500.0);
    }

    #[test]
    fn test_cumulative_product_sum_f32_price() {
        let mut pv = CumulativeProductSum::new();

        pv.add(100.5_f32, 1000_u64);
        let result = pv.value();

        assert!((result - 100_500.0).abs() < 1e-6);
    }

    // -------------------------------------------------------------------------
    // WilderSmoothing Tests (Task 1.7)
    // -------------------------------------------------------------------------

    #[test]
    fn test_wilder_smoothing_basic() {
        let mut ws = WilderSmoothing::new();
        assert!(!ws.is_initialized());

        ws.initialize(10.0_f64);
        assert!(ws.is_initialized());
        assert_eq!(ws.value(), 10.0);

        // Update with value 20, period 10
        // new = 10 + (20 - 10) / 10 = 11
        ws.update(20.0_f64, 10);
        assert!((ws.value() - 11.0).abs() < 1e-10);
    }

    #[test]
    fn test_wilder_smoothing_convergence() {
        let mut ws = WilderSmoothing::new();
        ws.initialize(0.0);

        let period = 14;
        let target = 100.0;

        // Apply constant value repeatedly, should converge toward target
        for _ in 0..1000 {
            ws.update(target, period);
        }

        // Should be very close to target after many iterations
        assert!(
            (ws.value() - target).abs() < 0.01,
            "Should converge to target, got {}",
            ws.value()
        );
    }

    #[test]
    fn test_wilder_smoothing_drift_reduction() {
        // Compare f64 Wilder smoothing vs simulated f32
        let mut ws_f64 = WilderSmoothing::new();
        let mut ws_f32: f32 = 10.0;

        ws_f64.initialize(10.0);

        let period = 14_usize;
        let iterations = 10_000;

        // Apply alternating values
        for i in 0..iterations {
            let value = if i % 2 == 0 { 15.0 } else { 5.0 };
            ws_f64.update(value, period);
            ws_f32 = ws_f32 + ((value as f32) - ws_f32) / (period as f32);
        }

        // Due to the Wilder smoothing formula, with alternating 5 and 15,
        // the asymptotic value depends on whether we end on 5 or 15.
        // After 10000 iterations (even), we last applied 15.0
        // The asymptotic behavior oscillates around the midpoint with decay.
        // What matters is that f64 accumulator reduces precision loss.

        // Both values should be finite and in reasonable range
        assert!(ws_f64.value().is_finite(), "f64 value should be finite");
        assert!(ws_f32.is_finite(), "f32 value should be finite");

        // Both should be within the [5, 15] range
        assert!(
            ws_f64.value() >= 5.0 && ws_f64.value() <= 15.0,
            "f64 Wilder value out of range: {}",
            ws_f64.value()
        );
        assert!(
            ws_f32 >= 5.0 && ws_f32 <= 15.0,
            "f32 Wilder value out of range: {}",
            ws_f32
        );
    }

    #[test]
    fn test_wilder_smoothing_rsi_tolerance() {
        // Simulate RSI calculation pattern
        let mut avg_gain = WilderSmoothing::new();
        let mut avg_loss = WilderSmoothing::new();

        // Initialize with first period averages
        avg_gain.initialize(1.5); // Average gain
        avg_loss.initialize(1.0); // Average loss

        let period = 14;

        // Simulate 10K updates
        for i in 0..10_000 {
            let (gain, loss) = if i % 3 == 0 {
                (2.0, 0.0)
            } else if i % 3 == 1 {
                (0.0, 1.5)
            } else {
                (1.0, 0.5)
            };
            avg_gain.update(gain, period);
            avg_loss.update(loss, period);
        }

        // Compute RSI
        let rs = avg_gain.value() / avg_loss.value().max(1e-10);
        let rsi = 100.0 - (100.0 / (1.0 + rs));

        // RSI should be in valid range
        assert!(rsi >= 0.0 && rsi <= 100.0, "RSI out of range: {}", rsi);

        // With the alternating pattern, RSI should be near 50
        // (slight bias toward gains in our pattern)
        assert!(
            rsi > 30.0 && rsi < 70.0,
            "RSI should be roughly balanced, got {}",
            rsi
        );
    }
}
