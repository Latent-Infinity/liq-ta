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
//! use fast_ta::kernels::rolling_extrema::{rolling_max, rolling_min, RollingExtremaOutput};
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
/// use fast_ta::kernels::rolling_extrema::rolling_extrema_lookback;
///
/// assert_eq!(rolling_extrema_lookback(5), 4);
/// assert_eq!(rolling_extrema_lookback(14), 13);
/// ```
#[inline]
#[must_use]
pub const fn rolling_extrema_lookback(period: usize) -> usize {
    if period == 0 {
        0
    } else {
        period - 1
    }
}

/// Returns the minimum input length required for rolling max/min.
///
/// This is the smallest input size that will produce at least one valid output.
/// For rolling extrema, this equals the period.
///
/// # Example
///
/// ```
/// use fast_ta::kernels::rolling_extrema::rolling_extrema_min_len;
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
    /// use fast_ta::kernels::rolling_extrema::MonotonicDeque;
    ///
    /// let deque: MonotonicDeque<f64> = MonotonicDeque::new(5);
    /// ```
    #[must_use]
    pub fn new(period: usize) -> Self {
        Self {
            deque: VecDeque::with_capacity(period),
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

        // Handle invalid values: invalid inputs should not be considered as max
        if is_invalid(value) {
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

        // Handle invalid values: invalid inputs should not be considered as min
        if is_invalid(value) {
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
            while let Some(&front_idx) = self.deque.front() {
                if front_idx < window_start {
                    self.deque.pop_front();
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
    /// Returns `NaN` if the deque is empty.
    #[inline]
    pub fn get_extremum(&self, data: &[T]) -> T {
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
/// use fast_ta::kernels::rolling_extrema::rolling_max;
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
/// use fast_ta::kernels::rolling_extrema::rolling_min;
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
/// use fast_ta::kernels::rolling_extrema::rolling_extrema;
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
pub fn rolling_extrema<T: SeriesElement>(data: &[T], period: usize) -> Result<RollingExtremaOutput<T>> {
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