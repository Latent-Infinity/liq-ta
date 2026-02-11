//! Rolling extrema using monotonic deque for O(n) rolling max/min.
//!
//! This module provides efficient implementations for computing rolling maximum
//! and minimum values over a sliding window using monotonic deques.
//!
//! # Algorithm
//!
//! The monotonic deque algorithm maintains a double-ended queue of indices such that:
//! - For rolling max: values at those indices are in decreasing order
//! - For rolling min: values at those indices are in increasing order
//!
//! This allows O(1) amortized time per element instead of O(k) for naive scans,
//! where k is the window size.
//!
//! # Complexity
//!
//! - Time: O(n) for n elements (amortized O(1) per element)
//! - Space: O(k) for the deque, where k is the period
//!
//! # Example
//!
//! ```
//! use liq_ta::kernels::rolling_extrema::{rolling_max, rolling_min, RollingExtremaOutput};
//!
//! let data = vec![3.0_f64, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
//! let period = 3;
//!
//! // Compute rolling maximum
//! let max_result = rolling_max(&data, period).unwrap();
//! assert!((max_result[2] - 4.0).abs() < 1e-10); // max of [3, 1, 4]
//! assert!((max_result[5] - 9.0).abs() < 1e-10); // max of [1, 5, 9]
//!
//! // Compute rolling minimum
//! let min_result = rolling_min(&data, period).unwrap();
//! assert!((min_result[2] - 1.0).abs() < 1e-10); // min of [3, 1, 4]
//! assert!((min_result[5] - 1.0).abs() < 1e-10); // min of [1, 5, 9]
//! ```
//!
//! # References
//!
//! - The monotonic deque algorithm is also known as the "sliding window maximum" algorithm
//! - It can be used for efficient computation of Stochastic Oscillator (%K calculation)
//!   which requires finding highest high and lowest low over a lookback period

use std::collections::VecDeque;

use crate::error::{Error, Result};
use crate::traits::SeriesElement;
use crate::utils::is_invalid;

/// Returns the lookback period for rolling max/min.
///
/// The lookback is the number of NaN values at the start of the output.
/// For rolling extrema, this is `period - 1`.
///
/// # Example
///
/// ```
/// use liq_ta::kernels::rolling_extrema::rolling_extrema_lookback;
///
/// assert_eq!(rolling_extrema_lookback(5), 4);
/// assert_eq!(rolling_extrema_lookback(14), 13);
/// ```
#[inline]
#[must_use]
pub const fn rolling_extrema_lookback(period: usize) -> usize {
    if period == 0 { 0 } else { period - 1 }
}

/// Returns the minimum input length required for rolling max/min.
///
/// This is the smallest input size that will produce at least one valid output.
/// For rolling extrema, this equals the period.
///
/// # Example
///
/// ```
/// use liq_ta::kernels::rolling_extrema::rolling_extrema_min_len;
///
/// assert_eq!(rolling_extrema_min_len(5), 5);
/// assert_eq!(rolling_extrema_min_len(14), 14);
/// ```
#[inline]
#[must_use]
pub const fn rolling_extrema_min_len(period: usize) -> usize {
    period
}

/// A monotonic deque for efficiently tracking rolling extrema.
///
/// This structure maintains a deque of indices where the values at those indices
/// are monotonically ordered (decreasing for max, increasing for min).
///
/// # Type Parameters
///
/// - `T`: The numeric type (typically `f32` or `f64`)
#[derive(Debug, Clone)]
pub struct MonotonicDeque<T> {
    /// The deque stores indices into the data array
    deque: VecDeque<usize>,
    /// Tracks indices with invalid (NaN/Infinity) values for NaN propagation
    invalid_indices: VecDeque<usize>,
    /// The window size
    period: usize,
    /// Phantom marker for the element type
    _phantom: std::marker::PhantomData<T>,
}

impl<T: SeriesElement> MonotonicDeque<T> {
    /// Creates a new monotonic deque with the specified window size.
    ///
    /// # Arguments
    ///
    /// * `period` - The window size for rolling calculations
    ///
    /// # Example
    ///
    /// ```
    /// use liq_ta::kernels::rolling_extrema::MonotonicDeque;
    ///
    /// let deque: MonotonicDeque<f64> = MonotonicDeque::new(5);
    /// ```
    #[must_use]
    pub fn new(period: usize) -> Self {
        Self {
            deque: VecDeque::with_capacity(period),
            invalid_indices: VecDeque::with_capacity(period),
            period,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns the window size.
    #[must_use]
    pub const fn period(&self) -> usize {
        self.period
    }

    /// Returns true if the deque is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.deque.is_empty()
    }

    /// Returns the number of indices currently in the deque.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.deque.len()
    }

    /// Clears the deque.
    #[inline]
    pub fn clear(&mut self) {
        self.deque.clear();
        self.invalid_indices.clear();
    }

    /// Pushes a new value for computing rolling maximum.
    ///
    /// This maintains the invariant that values at indices in the deque are
    /// in decreasing order, so the front always contains the maximum value's index.
    ///
    /// # Arguments
    ///
    /// * `index` - The current index in the data array
    /// * `data` - The data array
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds for `data`.
    #[inline]
    pub fn push_max(&mut self, index: usize, data: &[T]) {
        let value = data[index];

        // Track invalid values for NaN propagation
        if is_invalid(value) {
            self.invalid_indices.push_back(index);
            self.remove_expired(index);
            return;
        }

        // Remove elements from the back that are smaller than or equal to current value
        while let Some(&back_idx) = self.deque.back() {
            let back_val = data[back_idx];
            if is_invalid(back_val) || value >= back_val {
                self.deque.pop_back();
            } else {
                break;
            }
        }

        // Add current index
        self.deque.push_back(index);

        // Remove elements that are outside the window
        self.remove_expired(index);
    }

    /// Pushes a new value for computing rolling minimum.
    ///
    /// This maintains the invariant that values at indices in the deque are
    /// in increasing order, so the front always contains the minimum value's index.
    ///
    /// # Arguments
    ///
    /// * `index` - The current index in the data array
    /// * `data` - The data array
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds for `data`.
    #[inline]
    pub fn push_min(&mut self, index: usize, data: &[T]) {
        let value = data[index];

        // Track invalid values for NaN propagation
        if is_invalid(value) {
            self.invalid_indices.push_back(index);
            self.remove_expired(index);
            return;
        }

        // Remove elements from the back that are larger than or equal to current value
        while let Some(&back_idx) = self.deque.back() {
            let back_val = data[back_idx];
            if is_invalid(back_val) || value <= back_val {
                self.deque.pop_back();
            } else {
                break;
            }
        }

        // Add current index
        self.deque.push_back(index);

        // Remove elements that are outside the window
        self.remove_expired(index);
    }

    /// Removes indices that are outside the current window.
    #[inline]
    fn remove_expired(&mut self, current_index: usize) {
        // Only remove if we've seen at least `period` elements
        if current_index >= self.period {
            let window_start = current_index + 1 - self.period;

            // Remove expired valid indices
            while let Some(&front_idx) = self.deque.front() {
                if front_idx < window_start {
                    self.deque.pop_front();
                } else {
                    break;
                }
            }

            // Remove expired invalid indices
            while let Some(&front_idx) = self.invalid_indices.front() {
                if front_idx < window_start {
                    self.invalid_indices.pop_front();
                } else {
                    break;
                }
            }
        }
    }

    /// Returns the index of the current extremum (max or min) value.
    ///
    /// Returns `None` if the deque is empty.
    #[inline]
    #[must_use]
    pub fn front_index(&self) -> Option<usize> {
        self.deque.front().copied()
    }

    /// Returns the current extremum value from the data array.
    ///
    /// Returns `NaN` if the window contains any invalid (NaN/Infinity) values.
    ///
    /// This implements strict NaN propagation per PRD §4.3:
    /// "NaN in window → Output NaN for that position"
    ///
    /// This ensures mathematical correctness and prevents silent data corruption.
    #[inline]
    pub fn get_extremum(&self, data: &[T]) -> T {
        // Strict NaN propagation: if window contains ANY invalid value, return NaN
        if !self.invalid_indices.is_empty() {
            return T::nan();
        }

        // Return NaN if all values are invalid (deque is empty)
        self.front_index().map_or_else(T::nan, |idx| data[idx])
    }
}

/// Output structure containing both rolling maximum and minimum.
///
/// Each vector has the same length as the input data. The first `period - 1`
/// values are NaN due to insufficient lookback data.
#[derive(Debug, Clone)]
pub struct RollingExtremaOutput<T> {
    /// The rolling maximum values.
    pub max: Vec<T>,
    /// The rolling minimum values.
    pub min: Vec<T>,
}

/// Computes the rolling maximum using a monotonic deque.
///
/// This algorithm runs in O(n) time for n elements, with amortized O(1)
/// per element, compared to O(n×k) for the naive approach.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
///
/// # Returns
///
/// A `Result` containing a vector of rolling maximum values,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for output + O(k) for the deque
///
/// # Example
///
/// ```
/// use liq_ta::kernels::rolling_extrema::rolling_max;
///
/// let data = vec![1.0_f64, 3.0, 2.0, 5.0, 4.0];
/// let result = rolling_max(&data, 3).unwrap();
///
/// // First 2 values are NaN
/// assert!(result[0].is_nan());
/// assert!(result[1].is_nan());
///
/// // max of [1,3,2] = 3
/// assert!((result[2] - 3.0).abs() < 1e-10);
///
/// // max of [3,2,5] = 5
/// assert!((result[3] - 5.0).abs() < 1e-10);
/// ```
pub fn rolling_max<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "rolling_max",
        });
    }

    let n = data.len();
    let mut result = vec![T::nan(); n];

    let mut deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for i in 0..n {
        deque.push_max(i, data);

        if i >= period - 1 {
            result[i] = deque.get_extremum(data);
        }
    }

    Ok(result)
}

/// Computes the rolling maximum into a pre-allocated output buffer.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid values computed,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
/// - The output buffer is shorter than the input data
pub fn rolling_max_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "rolling_max",
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "rolling_max",
        });
    }

    let n = data.len();

    // Initialize with NaN
    for value in output.iter_mut().take(period - 1) {
        *value = T::nan();
    }

    let mut deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for i in 0..n {
        deque.push_max(i, data);

        if i >= period - 1 {
            output[i] = deque.get_extremum(data);
        }
    }

    Ok(n - period + 1)
}

/// Computes the rolling minimum using a monotonic deque.
///
/// This algorithm runs in O(n) time for n elements, with amortized O(1)
/// per element, compared to O(n×k) for the naive approach.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
///
/// # Returns
///
/// A `Result` containing a vector of rolling minimum values,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for output + O(k) for the deque
///
/// # Example
///
/// ```
/// use liq_ta::kernels::rolling_extrema::rolling_min;
///
/// let data = vec![5.0_f64, 3.0, 4.0, 1.0, 2.0];
/// let result = rolling_min(&data, 3).unwrap();
///
/// // First 2 values are NaN
/// assert!(result[0].is_nan());
/// assert!(result[1].is_nan());
///
/// // min of [5,3,4] = 3
/// assert!((result[2] - 3.0).abs() < 1e-10);
///
/// // min of [3,4,1] = 1
/// assert!((result[3] - 1.0).abs() < 1e-10);
/// ```
pub fn rolling_min<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "rolling_min",
        });
    }

    let n = data.len();
    let mut result = vec![T::nan(); n];

    let mut deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for i in 0..n {
        deque.push_min(i, data);

        if i >= period - 1 {
            result[i] = deque.get_extremum(data);
        }
    }

    Ok(result)
}

/// Computes the rolling minimum into a pre-allocated output buffer.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
/// * `output` - Pre-allocated output buffer (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid values computed,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
/// - The output buffer is shorter than the input data
pub fn rolling_min_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut [T],
) -> Result<usize> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "rolling_min",
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "rolling_min",
        });
    }

    let n = data.len();

    // Initialize with NaN
    for value in output.iter_mut().take(period - 1) {
        *value = T::nan();
    }

    let mut deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for i in 0..n {
        deque.push_min(i, data);

        if i >= period - 1 {
            output[i] = deque.get_extremum(data);
        }
    }

    Ok(n - period + 1)
}

/// Computes both rolling maximum and minimum using monotonic deques.
///
/// This is more efficient than calling `rolling_max` and `rolling_min` separately,
/// as it processes the data only once.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
///
/// # Returns
///
/// A `Result` containing a `RollingExtremaOutput` with both max and min vectors,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
///
/// # Performance
///
/// - Time complexity: O(n) where n is the length of the input data
/// - Space complexity: O(n) for output + O(k) for both deques
///
/// # Example
///
/// ```
/// use liq_ta::kernels::rolling_extrema::rolling_extrema;
///
/// let data = vec![1.0_f64, 3.0, 2.0, 5.0, 4.0];
/// let result = rolling_extrema(&data, 3).unwrap();
///
/// // First 2 values are NaN
/// assert!(result.max[0].is_nan());
/// assert!(result.min[0].is_nan());
///
/// // max of [1,3,2] = 3, min = 1
/// assert!((result.max[2] - 3.0).abs() < 1e-10);
/// assert!((result.min[2] - 1.0).abs() < 1e-10);
/// ```
pub fn rolling_extrema<T: SeriesElement>(
    data: &[T],
    period: usize,
) -> Result<RollingExtremaOutput<T>> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "rolling_extrema",
        });
    }

    let n = data.len();
    let mut max = vec![T::nan(); n];
    let mut min = vec![T::nan(); n];

    let mut max_deque: MonotonicDeque<T> = MonotonicDeque::new(period);
    let mut min_deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for i in 0..n {
        max_deque.push_max(i, data);
        min_deque.push_min(i, data);

        if i >= period - 1 {
            max[i] = max_deque.get_extremum(data);
            min[i] = min_deque.get_extremum(data);
        }
    }

    Ok(RollingExtremaOutput { max, min })
}

/// Computes both rolling maximum and minimum into pre-allocated output buffers.
///
/// This is more efficient than calling `rolling_max_into` and `rolling_min_into` separately.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
/// * `max_output` - Pre-allocated output buffer for max (must be at least as long as input)
/// * `min_output` - Pre-allocated output buffer for min (must be at least as long as input)
///
/// # Returns
///
/// A `Result` containing the number of valid values computed,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
/// - Either output buffer is shorter than the input data
pub fn rolling_extrema_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    max_output: &mut [T],
    min_output: &mut [T],
) -> Result<usize> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "rolling_extrema",
        });
    }

    if max_output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: max_output.len(),
            indicator: "rolling_extrema_max",
        });
    }

    if min_output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: min_output.len(),
            indicator: "rolling_extrema_min",
        });
    }

    let n = data.len();

    // Initialize with NaN
    for value in max_output.iter_mut().take(period - 1) {
        *value = T::nan();
    }
    for value in min_output.iter_mut().take(period - 1) {
        *value = T::nan();
    }

    let mut max_deque: MonotonicDeque<T> = MonotonicDeque::new(period);
    let mut min_deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for i in 0..n {
        max_deque.push_max(i, data);
        min_deque.push_min(i, data);

        if i >= period - 1 {
            max_output[i] = max_deque.get_extremum(data);
            min_output[i] = min_deque.get_extremum(data);
        }
    }

    Ok(n - period + 1)
}

/// Computes the rolling maximum with strict NaN propagation.
///
/// Unlike `rolling_max`, this function propagates NaN values: if any value
/// in the window is NaN, the result for that window will be NaN.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
///
/// # Returns
///
/// A `Result` containing a vector of rolling maximum values,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
pub fn rolling_max_nan_propagating<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "rolling_max_nan_propagating",
        });
    }

    let n = data.len();
    let mut result = vec![T::nan(); n];

    for i in (period - 1)..n {
        let window_start = i + 1 - period;
        let window = &data[window_start..=i];

        // Check for NaN in window
        let has_nan = window.iter().any(|&v| is_invalid(v));

        if has_nan {
            result[i] = T::nan();
        } else {
            // Find max in window
            let mut max_val = window[0];
            for &val in &window[1..] {
                if val > max_val {
                    max_val = val;
                }
            }
            result[i] = max_val;
        }
    }

    Ok(result)
}

/// Computes the rolling minimum with strict NaN propagation.
///
/// Unlike `rolling_min`, this function propagates NaN values: if any value
/// in the window is NaN, the result for that window will be NaN.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
///
/// # Returns
///
/// A `Result` containing a vector of rolling minimum values,
/// or an error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
pub fn rolling_min_nan_propagating<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
    // Validate inputs
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "rolling_min_nan_propagating",
        });
    }

    let n = data.len();
    let mut result = vec![T::nan(); n];

    for i in (period - 1)..n {
        let window_start = i + 1 - period;
        let window = &data[window_start..=i];

        // Check for NaN in window
        let has_nan = window.iter().any(|&v| is_invalid(v));

        if has_nan {
            result[i] = T::nan();
        } else {
            // Find min in window
            let mut min_val = window[0];
            for &val in &window[1..] {
                if val < min_val {
                    min_val = val;
                }
            }
            result[i] = min_val;
        }
    }

    Ok(result)
}
/// Van Herk/Gil-Werman algorithm for fused rolling extrema on separate high/low series.
///
/// This algorithm computes both rolling max (on high) and rolling min (on low) in O(3n) time
/// with O(6n) space using prefix-suffix decomposition. The memory access patterns are
/// SIMD-friendly and cache-optimal for large datasets.
///
/// # Algorithm
///
/// 1. **Divide**: Split data into blocks of size `period`
/// 2. **Forward pass**: Compute prefix max/min within each block
/// 3. **Backward pass**: Compute suffix max/min within each block
/// 4. **Combine**: For window `i-period+1..=i`, result is
///    `max(suffix[i-period+1], prefix[i])`
///
/// # Arguments
///
/// * `high` - High price series for maximum computation
/// * `low` - Low price series for minimum computation
/// * `period` - The window size for rolling calculations
///
/// # Returns
///
/// A `Result` containing `RollingExtremaOutput` with both max and min vectors,
/// or an error if validation fails.
///
/// # NaN Handling
///
/// This function uses **NaN propagation mode**: any NaN/Inf in the window causes NaN output.
/// Validity is tracked using prefix/suffix AND operations on finite checks.
///
/// # Performance
///
/// - **Time**: O(3n) - three passes over the data
/// - **Space**: O(6n) - six working buffers plus output
/// - **Best for**: Large datasets (n >= 1000) where SIMD benefits outweigh overhead
///
/// # Errors
///
/// Returns an error if:
/// - Either input is empty (`Error::EmptyInput`)
/// - The inputs have different lengths (`Error::LengthMismatch`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - Either input is shorter than the period (`Error::InsufficientData`)
#[inline]
pub fn rolling_extrema_fused_vhgw(
    high: &[f64],
    low: &[f64],
    period: usize,
) -> Result<RollingExtremaOutput<f64>> {
    // Validate inputs
    if high.is_empty() || low.is_empty() {
        return Err(Error::EmptyInput);
    }

    if high.len() != low.len() {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", high.len(), low.len()),
        });
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if high.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: high.len(),
            indicator: "rolling_extrema_fused_vhgw",
        });
    }

    let n = high.len();
    let lookback = period - 1;

    // Allocate output vectors
    let mut max = vec![f64::NAN; n];
    let mut min = vec![f64::NAN; n];

    // Allocate working buffers for prefix/suffix extrema
    let mut left_max_high = vec![f64::NEG_INFINITY; n];
    let mut right_max_high = vec![f64::NEG_INFINITY; n];
    let mut left_min_low = vec![f64::INFINITY; n];
    let mut right_min_low = vec![f64::INFINITY; n];

    // Track validity with prefix/suffix AND
    let mut left_valid = vec![true; n];
    let mut right_valid = vec![true; n];

    // Pass 1: Forward scan (prefix blocks)
    let mut block_start = 0;
    while block_start < n {
        let block_end = (block_start + period).min(n);

        // Reset for this block
        left_max_high[block_start] = high[block_start];
        left_min_low[block_start] = low[block_start];
        left_valid[block_start] = high[block_start].is_finite() && low[block_start].is_finite();

        // Extend prefix within block
        for i in (block_start + 1)..block_end {
            left_max_high[i] = left_max_high[i - 1].max(high[i]);
            left_min_low[i] = left_min_low[i - 1].min(low[i]);
            left_valid[i] = left_valid[i - 1] && high[i].is_finite() && low[i].is_finite();
        }

        block_start = block_end;
    }

    // Pass 2: Backward scan (suffix blocks)
    let mut block_end = n;
    while block_end > 0 {
        let block_start = block_end.saturating_sub(period);

        // Reset for this block (from end)
        let last_idx = block_end - 1;
        right_max_high[last_idx] = high[last_idx];
        right_min_low[last_idx] = low[last_idx];
        right_valid[last_idx] = high[last_idx].is_finite() && low[last_idx].is_finite();

        // Extend suffix within block (going backward)
        if last_idx > block_start {
            for i in (block_start..last_idx).rev() {
                right_max_high[i] = right_max_high[i + 1].max(high[i]);
                right_min_low[i] = right_min_low[i + 1].min(low[i]);
                right_valid[i] = right_valid[i + 1] && high[i].is_finite() && low[i].is_finite();
            }
        }

        block_end = block_start;
    }

    // Pass 3: Combine prefix/suffix to get rolling extrema
    for j in 0..(n - lookback) {
        let start = j;
        let end = j + lookback;

        // Combine prefix/suffix to get window extrema
        let hh = right_max_high[start].max(left_max_high[end]);
        let ll = right_min_low[start].min(left_min_low[end]);

        // Combine validity
        let window_ok = right_valid[start] && left_valid[end];

        if window_ok {
            max[end] = hh;
            min[end] = ll;
        }
        // else: already initialized to NaN
    }

    Ok(RollingExtremaOutput { max, min })
}

/// Computes rolling MIDPOINT using fused VHGW algorithm.
///
/// This is a specialized VHGW kernel that computes both rolling max and min
/// in a single fused pass, then combines them to produce midpoint output.
///
/// # Algorithm
///
/// Uses the Van Herk-Gil-Werman algorithm with three sequential passes:
/// 1. Forward scan: compute prefix max/min for each block
/// 2. Backward scan: compute suffix max/min for each block
/// 3. Combine: merge prefix/suffix and compute (max+min)*0.5
///
/// # Performance
///
/// - Time: O(n) with 3 sequential passes (better vectorization than deque)
/// - Space: O(n) for prefix/suffix buffers
/// - Best for: large n (>1000) and/or large periods (>30)
///
/// # Arguments
///
/// * `data` - Input data series
/// * `period` - Rolling window size
/// * `output` - Pre-allocated output buffer
///
/// # Returns
///
/// Number of valid outputs computed, or error if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
/// - The output buffer is too small (`Error::BufferTooSmall`)
pub fn rolling_midpoint_vhgw_f64(data: &[f64], period: usize, output: &mut [f64]) -> Result<usize> {
    // Validate inputs
    if data.is_empty() {
        return Err(Error::EmptyInput);
    }

    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if data.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: data.len(),
            indicator: "rolling_midpoint_vhgw",
        });
    }

    if output.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.len(),
            indicator: "rolling_midpoint_vhgw",
        });
    }

    let n = data.len();
    let lookback = period - 1;

    // Initialize lookback period with NaN
    for i in 0..lookback {
        output[i] = f64::NAN;
    }

    // Handle period == 1 case
    if period == 1 {
        for i in 0..n {
            output[i] = if data[i].is_finite() {
                data[i]
            } else {
                f64::NAN
            };
        }
        return Ok(n);
    }

    // Allocate working buffers for prefix/suffix extrema
    let mut left_max = vec![f64::NEG_INFINITY; n];
    let mut right_max = vec![f64::NEG_INFINITY; n];
    let mut left_min = vec![f64::INFINITY; n];
    let mut right_min = vec![f64::INFINITY; n];

    // Track validity with prefix/suffix AND
    let mut left_valid = vec![true; n];
    let mut right_valid = vec![true; n];

    // Pass 1: Forward scan (prefix blocks)
    let mut block_start = 0;
    while block_start < n {
        let block_end = (block_start + period).min(n);

        // Reset for this block
        left_max[block_start] = data[block_start];
        left_min[block_start] = data[block_start];
        left_valid[block_start] = data[block_start].is_finite();

        // Extend prefix within block
        for i in (block_start + 1)..block_end {
            left_max[i] = left_max[i - 1].max(data[i]);
            left_min[i] = left_min[i - 1].min(data[i]);
            left_valid[i] = left_valid[i - 1] && data[i].is_finite();
        }

        block_start = block_end;
    }

    // Pass 2: Backward scan (suffix blocks)
    let mut block_end = n;
    while block_end > 0 {
        let block_start = block_end.saturating_sub(period);

        // Reset for this block (from end)
        let last_idx = block_end - 1;
        right_max[last_idx] = data[last_idx];
        right_min[last_idx] = data[last_idx];
        right_valid[last_idx] = data[last_idx].is_finite();

        // Extend suffix within block (going backward)
        if last_idx > block_start {
            for i in (block_start..last_idx).rev() {
                right_max[i] = right_max[i + 1].max(data[i]);
                right_min[i] = right_min[i + 1].min(data[i]);
                right_valid[i] = right_valid[i + 1] && data[i].is_finite();
            }
        }

        block_end = block_start;
    }

    // Pass 3: Combine prefix/suffix to get rolling midpoint
    for j in 0..(n - lookback) {
        let start = j;
        let end = j + lookback;

        // Combine prefix/suffix to get window extrema
        let highest = right_max[start].max(left_max[end]);
        let lowest = right_min[start].min(left_min[end]);

        // Combine validity
        let window_ok = right_valid[start] && left_valid[end];

        if window_ok {
            output[end] = (highest + lowest) * 0.5;
        }
        // else: already initialized to NaN
    }

    Ok(n - lookback)
}

/// Computes Stochastic Fast (%K and %D) using fused VHGW algorithm.
///
/// This specialized VHGW kernel computes rolling max of high and rolling min of low,
/// then combines them with close to produce %K, then computes %D as SMA of %K.
///
/// # Algorithm
///
/// 1. VHGW pass on high array → rolling max
/// 2. VHGW pass on low array → rolling min
/// 3. Combine with close: %K = 100 * (close - min) / (max - min)
/// 4. Rolling SMA on %K → %D
///
/// # Performance
///
/// - Time: O(n) with better vectorization than deque
/// - Space: O(n) for VHGW buffers
/// - Best for: large n (>1000)
///
/// # Arguments
///
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `k_period` - Rolling window size for %K
/// * `d_period` - SMA period for %D
/// * `k_out` - Pre-allocated output buffer for %K
/// * `d_out` - Pre-allocated output buffer for %D
///
/// # Returns
///
/// Ok(()) on success, or error if validation fails.
pub fn compute_stochastic_fast_vhgw_f64(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    d_period: usize,
    k_out: &mut [f64],
    d_out: &mut [f64],
) -> Result<()> {
    let n = close.len();
    if n < k_period {
        return Err(Error::InsufficientData {
            indicator: "stochastic_fast_vhgw",
            required: k_period,
            actual: n,
        });
    }

    let k_lookback = k_period - 1;
    let d_start = k_lookback + d_period - 1;

    // Fill lookback NaNs
    k_out[..k_lookback].fill(f64::NAN);
    d_out[..d_start.min(n)].fill(f64::NAN);

    // Pass 1 & 2: VHGW on high (forward + backward for rolling max)
    let mut left_max_high = vec![f64::NEG_INFINITY; n];
    let mut right_max_high = vec![f64::NEG_INFINITY; n];

    // Forward scan for max_high
    left_max_high[0] = high[0];
    for i in 1..n {
        if i % k_period == 0 {
            left_max_high[i] = high[i];
        } else {
            left_max_high[i] = left_max_high[i - 1].max(high[i]);
        }
    }

    // Backward scan for max_high
    right_max_high[n - 1] = high[n - 1];
    for i in (0..n - 1).rev() {
        if (i + 1) % k_period == 0 {
            right_max_high[i] = high[i];
        } else {
            right_max_high[i] = right_max_high[i + 1].max(high[i]);
        }
    }

    // Pass 3 & 4: VHGW on low (forward + backward for rolling min)
    let mut left_min_low = vec![f64::INFINITY; n];
    let mut right_min_low = vec![f64::INFINITY; n];

    // Forward scan for min_low
    left_min_low[0] = low[0];
    for i in 1..n {
        if i % k_period == 0 {
            left_min_low[i] = low[i];
        } else {
            left_min_low[i] = left_min_low[i - 1].min(low[i]);
        }
    }

    // Backward scan for min_low
    right_min_low[n - 1] = low[n - 1];
    for i in (0..n - 1).rev() {
        if (i + 1) % k_period == 0 {
            right_min_low[i] = low[i];
        } else {
            right_min_low[i] = right_min_low[i + 1].min(low[i]);
        }
    }

    // Pass 5: Combine to compute %K
    for j in 0..(n - k_lookback) {
        let start = j;
        let end = j + k_lookback;

        let highest_high = right_max_high[start].max(left_max_high[end]);
        let lowest_low = right_min_low[start].min(left_min_low[end]);

        let range = highest_high - lowest_low;
        k_out[end] = if range > 0.0 {
            100.0 * (close[end] - lowest_low) / range
        } else {
            50.0 // Flat range
        };
    }

    // Pass 6: Compute %D as SMA of %K
    if n >= d_start + 1 {
        // Initialize first %D window
        let mut sum = 0.0;
        for i in k_lookback..(k_lookback + d_period) {
            sum += k_out[i];
        }
        d_out[d_start] = sum / d_period as f64;

        // Rolling SMA for remaining %D values
        for i in (d_start + 1)..n {
            let old_idx = i - d_period;
            sum = sum - k_out[old_idx] + k_out[i];
            d_out[i] = sum / d_period as f64;
        }
    }

    Ok(())
}

/// Computes Fast Stochastic %K and %D using Van Herk/Gil-Werman (VHGW) algorithm.
///
/// This is an f32-specialized version optimized for single-precision data.
///
/// # Algorithm
///
/// Uses VHGW for O(n) rolling max/min:
/// 1. Forward scan: Compute local max/min in blocks of size k_period
/// 2. Backward scan: Compute local max/min in blocks of size k_period
/// 3. Combine: `rolling_extrema = max(left[i], right[i-k_period+1])`
/// 4. Compute %K from rolling high/low extrema
/// 5. Compute %D as SMA of %K
///
/// # Arguments
///
/// * `high` - High prices (f32)
/// * `low` - Low prices (f32)
/// * `close` - Closing prices (f32)
/// * `k_period` - Lookback period for %K
/// * `d_period` - Smoothing period for %D (SMA of %K)
/// * `k_out` - Output buffer for %K values
/// * `d_out` - Output buffer for %D values
///
/// # Returns
///
/// Ok(()) on success, or error if validation fails.
pub fn compute_stochastic_fast_vhgw_f32(
    high: &[f32],
    low: &[f32],
    close: &[f32],
    k_period: usize,
    d_period: usize,
    k_out: &mut [f32],
    d_out: &mut [f32],
) -> Result<()> {
    let n = close.len();
    if n < k_period {
        return Err(Error::InsufficientData {
            indicator: "stochastic_fast_vhgw",
            required: k_period,
            actual: n,
        });
    }

    let k_lookback = k_period - 1;
    let d_start = k_lookback + d_period - 1;

    // Fill lookback NaNs
    k_out[..k_lookback].fill(f32::NAN);
    d_out[..d_start.min(n)].fill(f32::NAN);

    // Pass 1 & 2: VHGW on high (forward + backward for rolling max)
    let mut left_max_high = vec![f32::NEG_INFINITY; n];
    let mut right_max_high = vec![f32::NEG_INFINITY; n];

    // Forward scan for max_high
    left_max_high[0] = high[0];
    for i in 1..n {
        if i % k_period == 0 {
            left_max_high[i] = high[i];
        } else {
            left_max_high[i] = left_max_high[i - 1].max(high[i]);
        }
    }

    // Backward scan for max_high
    right_max_high[n - 1] = high[n - 1];
    for i in (0..n - 1).rev() {
        if (i + 1) % k_period == 0 {
            right_max_high[i] = high[i];
        } else {
            right_max_high[i] = right_max_high[i + 1].max(high[i]);
        }
    }

    // Pass 3 & 4: VHGW on low (forward + backward for rolling min)
    let mut left_min_low = vec![f32::INFINITY; n];
    let mut right_min_low = vec![f32::INFINITY; n];

    // Forward scan for min_low
    left_min_low[0] = low[0];
    for i in 1..n {
        if i % k_period == 0 {
            left_min_low[i] = low[i];
        } else {
            left_min_low[i] = left_min_low[i - 1].min(low[i]);
        }
    }

    // Backward scan for min_low
    right_min_low[n - 1] = low[n - 1];
    for i in (0..n - 1).rev() {
        if (i + 1) % k_period == 0 {
            right_min_low[i] = low[i];
        } else {
            right_min_low[i] = right_min_low[i + 1].min(low[i]);
        }
    }

    // Pass 5: Combine to compute %K
    for j in 0..(n - k_lookback) {
        let start = j;
        let end = j + k_lookback;

        let highest_high = right_max_high[start].max(left_max_high[end]);
        let lowest_low = right_min_low[start].min(left_min_low[end]);

        let range = highest_high - lowest_low;
        k_out[end] = if range > 0.0 {
            100.0 * (close[end] - lowest_low) / range
        } else {
            50.0 // Flat range
        };
    }

    // Pass 6: Compute %D as SMA of %K
    if n >= d_start + 1 {
        // Initialize first %D window
        let mut sum = 0.0_f32;
        for i in k_lookback..(k_lookback + d_period) {
            sum += k_out[i];
        }
        d_out[d_start] = sum / d_period as f32;

        // Rolling SMA for remaining %D values
        for i in (d_start + 1)..n {
            let old_idx = i - d_period;
            sum = sum - k_out[old_idx] + k_out[i];
            d_out[i] = sum / d_period as f32;
        }
    }

    Ok(())
}

/// Computes Full/Slow Stochastic %K and %D using Van Herk/Gil-Werman (VHGW) algorithm.
///
/// This is the f64-specialized version for full stochastic with K slowing.
///
/// # Algorithm
///
/// Uses VHGW for O(n) rolling max/min:
/// 1. Forward/backward scans: Compute local max/min in blocks of size k_period
/// 2. Combine: Compute raw %K from rolling extrema
/// 3. Apply SMA(raw %K, slow_k_period) to get slow %K
/// 4. Apply SMA(slow %K, d_period) to get %D
///
/// # Arguments
///
/// * `high` - High prices (f64)
/// * `low` - Low prices (f64)
/// * `close` - Closing prices (f64)
/// * `k_period` - Lookback period for raw %K
/// * `slow_k_period` - Smoothing period for slow %K (SMA of raw %K)
/// * `d_period` - Smoothing period for %D (SMA of slow %K)
/// * `k_out` - Output buffer for slow %K values
/// * `d_out` - Output buffer for %D values
///
/// # Returns
///
/// Ok(()) on success, or error if validation fails.
pub fn compute_stochastic_full_vhgw_f64(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    slow_k_period: usize,
    d_period: usize,
    k_out: &mut [f64],
    d_out: &mut [f64],
) -> Result<()> {
    let n = close.len();
    if n < k_period {
        return Err(Error::InsufficientData {
            indicator: "stochastic_full_vhgw",
            required: k_period,
            actual: n,
        });
    }

    let k_lookback = k_period - 1;
    let slow_k_start = k_lookback + slow_k_period - 1;
    let d_start = slow_k_start + d_period - 1;

    // Fill lookback NaNs
    k_out[..slow_k_start.min(n)].fill(f64::NAN);
    d_out[..d_start.min(n)].fill(f64::NAN);

    // Pass 1 & 2: VHGW on high (forward + backward for rolling max)
    let mut left_max_high = vec![f64::NEG_INFINITY; n];
    let mut right_max_high = vec![f64::NEG_INFINITY; n];

    // Forward scan for max_high
    left_max_high[0] = high[0];
    for i in 1..n {
        if i % k_period == 0 {
            left_max_high[i] = high[i];
        } else {
            left_max_high[i] = left_max_high[i - 1].max(high[i]);
        }
    }

    // Backward scan for max_high
    right_max_high[n - 1] = high[n - 1];
    for i in (0..n - 1).rev() {
        if (i + 1) % k_period == 0 {
            right_max_high[i] = high[i];
        } else {
            right_max_high[i] = right_max_high[i + 1].max(high[i]);
        }
    }

    // Pass 3 & 4: VHGW on low (forward + backward for rolling min)
    let mut left_min_low = vec![f64::INFINITY; n];
    let mut right_min_low = vec![f64::INFINITY; n];

    // Forward scan for min_low
    left_min_low[0] = low[0];
    for i in 1..n {
        if i % k_period == 0 {
            left_min_low[i] = low[i];
        } else {
            left_min_low[i] = left_min_low[i - 1].min(low[i]);
        }
    }

    // Backward scan for min_low
    right_min_low[n - 1] = low[n - 1];
    for i in (0..n - 1).rev() {
        if (i + 1) % k_period == 0 {
            right_min_low[i] = low[i];
        } else {
            right_min_low[i] = right_min_low[i + 1].min(low[i]);
        }
    }

    // Pass 5: Combine to compute raw %K (not output yet, need to smooth)
    let mut raw_k = vec![0.0_f64; n];
    for j in 0..(n - k_lookback) {
        let start = j;
        let end = j + k_lookback;

        let highest_high = right_max_high[start].max(left_max_high[end]);
        let lowest_low = right_min_low[start].min(left_min_low[end]);

        let range = highest_high - lowest_low;
        raw_k[end] = if range > 0.0 {
            100.0 * (close[end] - lowest_low) / range
        } else {
            50.0 // Flat range
        };
    }

    // Pass 6: Compute slow %K as SMA of raw %K
    if n >= slow_k_start + 1 {
        // Initialize first slow %K window
        let mut sum = 0.0;
        for i in k_lookback..(k_lookback + slow_k_period) {
            sum += raw_k[i];
        }
        k_out[slow_k_start] = sum / slow_k_period as f64;

        // Rolling SMA for remaining slow %K values
        for i in (slow_k_start + 1)..n {
            let old_idx = i - slow_k_period;
            sum = sum - raw_k[old_idx] + raw_k[i];
            k_out[i] = sum / slow_k_period as f64;
        }
    }

    // Pass 7: Compute %D as SMA of slow %K
    if n >= d_start + 1 {
        // Initialize first %D window
        let mut sum = 0.0;
        for i in slow_k_start..(slow_k_start + d_period) {
            sum += k_out[i];
        }
        d_out[d_start] = sum / d_period as f64;

        // Rolling SMA for remaining %D values
        for i in (d_start + 1)..n {
            let old_idx = i - d_period;
            sum = sum - k_out[old_idx] + k_out[i];
            d_out[i] = sum / d_period as f64;
        }
    }

    Ok(())
}

/// Computes Full/Slow Stochastic %K and %D using Van Herk/Gil-Werman (VHGW) algorithm.
///
/// This is the f32-specialized version for full stochastic with K slowing.
pub fn compute_stochastic_full_vhgw_f32(
    high: &[f32],
    low: &[f32],
    close: &[f32],
    k_period: usize,
    slow_k_period: usize,
    d_period: usize,
    k_out: &mut [f32],
    d_out: &mut [f32],
) -> Result<()> {
    let n = close.len();
    if n < k_period {
        return Err(Error::InsufficientData {
            indicator: "stochastic_full_vhgw",
            required: k_period,
            actual: n,
        });
    }

    let k_lookback = k_period - 1;
    let slow_k_start = k_lookback + slow_k_period - 1;
    let d_start = slow_k_start + d_period - 1;

    // Fill lookback NaNs
    k_out[..slow_k_start.min(n)].fill(f32::NAN);
    d_out[..d_start.min(n)].fill(f32::NAN);

    // Pass 1 & 2: VHGW on high (forward + backward for rolling max)
    let mut left_max_high = vec![f32::NEG_INFINITY; n];
    let mut right_max_high = vec![f32::NEG_INFINITY; n];

    left_max_high[0] = high[0];
    for i in 1..n {
        if i % k_period == 0 {
            left_max_high[i] = high[i];
        } else {
            left_max_high[i] = left_max_high[i - 1].max(high[i]);
        }
    }

    right_max_high[n - 1] = high[n - 1];
    for i in (0..n - 1).rev() {
        if (i + 1) % k_period == 0 {
            right_max_high[i] = high[i];
        } else {
            right_max_high[i] = right_max_high[i + 1].max(high[i]);
        }
    }

    // Pass 3 & 4: VHGW on low (forward + backward for rolling min)
    let mut left_min_low = vec![f32::INFINITY; n];
    let mut right_min_low = vec![f32::INFINITY; n];

    left_min_low[0] = low[0];
    for i in 1..n {
        if i % k_period == 0 {
            left_min_low[i] = low[i];
        } else {
            left_min_low[i] = left_min_low[i - 1].min(low[i]);
        }
    }

    right_min_low[n - 1] = low[n - 1];
    for i in (0..n - 1).rev() {
        if (i + 1) % k_period == 0 {
            right_min_low[i] = low[i];
        } else {
            right_min_low[i] = right_min_low[i + 1].min(low[i]);
        }
    }

    // Pass 5: Combine to compute raw %K
    let mut raw_k = vec![0.0_f32; n];
    for j in 0..(n - k_lookback) {
        let start = j;
        let end = j + k_lookback;

        let highest_high = right_max_high[start].max(left_max_high[end]);
        let lowest_low = right_min_low[start].min(left_min_low[end]);

        let range = highest_high - lowest_low;
        raw_k[end] = if range > 0.0 {
            100.0 * (close[end] - lowest_low) / range
        } else {
            50.0
        };
    }

    // Pass 6: Compute slow %K as SMA of raw %K
    if n >= slow_k_start + 1 {
        let mut sum = 0.0_f32;
        for i in k_lookback..(k_lookback + slow_k_period) {
            sum += raw_k[i];
        }
        k_out[slow_k_start] = sum / slow_k_period as f32;

        for i in (slow_k_start + 1)..n {
            let old_idx = i - slow_k_period;
            sum = sum - raw_k[old_idx] + raw_k[i];
            k_out[i] = sum / slow_k_period as f32;
        }
    }

    // Pass 7: Compute %D as SMA of slow %K
    if n >= d_start + 1 {
        let mut sum = 0.0_f32;
        for i in slow_k_start..(slow_k_start + d_period) {
            sum += k_out[i];
        }
        d_out[d_start] = sum / d_period as f32;

        for i in (d_start + 1)..n {
            let old_idx = i - d_period;
            sum = sum - k_out[old_idx] + k_out[i];
            d_out[i] = sum / d_period as f32;
        }
    }

    Ok(())
}
