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

    for (i, _) in data.iter().enumerate() {
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

    for (i, _) in data.iter().enumerate() {
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

    for (i, _) in data.iter().enumerate() {
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

    for (i, _) in data.iter().enumerate() {
        deque.push_min(i, data);

        if i >= period - 1 {
            output[i] = deque.get_extremum(data);
        }
    }

    Ok(n - period + 1)
}

/// Computes both rolling maximum and minimum in a single pass.
///
/// This function is more efficient than calling `rolling_max` and `rolling_min`
/// separately when both values are needed, as it only iterates over the data once.
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
/// - Space complexity: O(n) for outputs + O(2k) for the deques
///
/// # Example
///
/// ```
/// use fast_ta::kernels::rolling_extrema::rolling_extrema;
///
/// let data = vec![3.0_f64, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
/// let result = rolling_extrema(&data, 3).unwrap();
///
/// // First 2 values are NaN
/// assert!(result.max[0].is_nan());
/// assert!(result.min[0].is_nan());
///
/// // At index 2: window [3, 1, 4]
/// assert!((result.max[2] - 4.0).abs() < 1e-10); // max = 4
/// assert!((result.min[2] - 1.0).abs() < 1e-10); // min = 1
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
    let mut max_result = vec![T::nan(); n];
    let mut min_result = vec![T::nan(); n];

    let mut max_deque: MonotonicDeque<T> = MonotonicDeque::new(period);
    let mut min_deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for (i, _) in data.iter().enumerate() {
        max_deque.push_max(i, data);
        min_deque.push_min(i, data);

        if i >= period - 1 {
            max_result[i] = max_deque.get_extremum(data);
            min_result[i] = min_deque.get_extremum(data);
        }
    }

    Ok(RollingExtremaOutput {
        max: max_result,
        min: min_result,
    })
}

/// Computes both rolling maximum and minimum into pre-allocated output buffers.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
/// * `output` - Pre-allocated output structure
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
/// - Any output buffer is shorter than the input data
pub fn rolling_extrema_into<T: SeriesElement>(
    data: &[T],
    period: usize,
    output: &mut RollingExtremaOutput<T>,
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

    if output.max.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.max.len(),
            indicator: "rolling_extrema",
        });
    }

    if output.min.len() < data.len() {
        return Err(Error::BufferTooSmall {
            required: data.len(),
            actual: output.min.len(),
            indicator: "rolling_extrema",
        });
    }

    let n = data.len();

    // Initialize with NaN
    for (max_value, min_value) in output
        .max
        .iter_mut()
        .zip(output.min.iter_mut())
        .take(period - 1)
    {
        *max_value = T::nan();
        *min_value = T::nan();
    }

    let mut max_deque: MonotonicDeque<T> = MonotonicDeque::new(period);
    let mut min_deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for (i, _) in data.iter().enumerate() {
        max_deque.push_max(i, data);
        min_deque.push_min(i, data);

        if i >= period - 1 {
            output.max[i] = max_deque.get_extremum(data);
            output.min[i] = min_deque.get_extremum(data);
        }
    }

    Ok(n - period + 1)
}

/// Computes rolling maximum using the naive O(n×k) scan approach.
///
/// This function is provided for comparison and testing purposes.
/// It is NOT recommended for production use - use `rolling_max` instead.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
///
/// # Returns
///
/// A `Result` containing a vector of rolling maximum values.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
pub fn rolling_max_naive<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
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
            indicator: "rolling_max_naive",
        });
    }

    let n = data.len();
    let mut result = vec![T::nan(); n];

    for (i, out) in result.iter_mut().enumerate().skip(period - 1) {
        let window_start = i + 1 - period;
        let mut max_val = data[window_start];

        for &value in data.iter().take(i + 1).skip(window_start + 1) {
            if value > max_val || is_invalid(max_val) {
                max_val = value;
            }
        }

        *out = max_val;
    }

    Ok(result)
}

/// Computes rolling minimum using the naive O(n×k) scan approach.
///
/// This function is provided for comparison and testing purposes.
/// It is NOT recommended for production use - use `rolling_min` instead.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
///
/// # Returns
///
/// A `Result` containing a vector of rolling minimum values.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
pub fn rolling_min_naive<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
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
            indicator: "rolling_min_naive",
        });
    }

    let n = data.len();
    let mut result = vec![T::nan(); n];

    for (i, out) in result.iter_mut().enumerate().skip(period - 1) {
        let window_start = i + 1 - period;
        let mut min_val = data[window_start];

        for &value in data.iter().take(i + 1).skip(window_start + 1) {
            if value < min_val || is_invalid(min_val) {
                min_val = value;
            }
        }

        *out = min_val;
    }

    Ok(result)
}

// ==================== NaN-Propagating Variants ====================
//
// These functions propagate NaN through the window: if any value in the
// current window is NaN, the output is NaN. This is required for indicators
// like Stochastic that need strict NaN propagation semantics.

/// Computes rolling maximum with NaN propagation using a monotonic deque.
///
/// Unlike `rolling_max` which skips NaN values, this function propagates NaN:
/// if any value in the current window is NaN, the output is NaN.
///
/// This is required for indicators like Stochastic that need strict NaN
/// propagation semantics per PRD §4.8.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
///
/// # Returns
///
/// A `Result` containing a vector of rolling maximum values with NaN propagation.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
pub fn rolling_max_nan_propagating<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
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

    // Track NaN positions in a circular buffer style using a count
    let mut nan_count = 0usize;
    let mut deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for i in 0..n {
        // Track NaN entering window
        if is_invalid(data[i]) {
            nan_count += 1;
        }

        // Track NaN leaving window (after first full window)
        if i >= period && is_invalid(data[i - period]) {
            nan_count -= 1;
        }

        // Always update deque (it handles NaN internally by skipping)
        deque.push_max(i, data);

        if i >= period - 1 {
            if nan_count > 0 {
                result[i] = T::nan();
            } else {
                result[i] = deque.get_extremum(data);
            }
        }
    }

    Ok(result)
}

/// Computes rolling minimum with NaN propagation using a monotonic deque.
///
/// Unlike `rolling_min` which skips NaN values, this function propagates NaN:
/// if any value in the current window is NaN, the output is NaN.
///
/// # Arguments
///
/// * `data` - The input data series
/// * `period` - The window size for rolling calculations
///
/// # Returns
///
/// A `Result` containing a vector of rolling minimum values with NaN propagation.
///
/// # Errors
///
/// Returns an error if:
/// - The input data is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - The input data is shorter than the period (`Error::InsufficientData`)
pub fn rolling_min_nan_propagating<T: SeriesElement>(data: &[T], period: usize) -> Result<Vec<T>> {
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

    let mut nan_count = 0usize;
    let mut deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for i in 0..n {
        if is_invalid(data[i]) {
            nan_count += 1;
        }

        if i >= period && is_invalid(data[i - period]) {
            nan_count -= 1;
        }

        deque.push_min(i, data);

        if i >= period - 1 {
            if nan_count > 0 {
                result[i] = T::nan();
            } else {
                result[i] = deque.get_extremum(data);
            }
        }
    }

    Ok(result)
}

/// Computes rolling max of `high` and rolling min of `low` with NaN propagation.
///
/// This fused version is optimized for Stochastic Oscillator which needs:
/// - Highest high from the `high` array
/// - Lowest low from the `low` array
/// - NaN propagation if either array has NaN in the window
///
/// # Arguments
///
/// * `high` - The high price data series
/// * `low` - The low price data series
/// * `period` - The window size for rolling calculations
///
/// # Returns
///
/// A `Result` containing a `RollingExtremaOutput` where:
/// - `max` contains the rolling maximum of `high`
/// - `min` contains the rolling minimum of `low`
///
/// # Errors
///
/// Returns an error if:
/// - Either input is empty (`Error::EmptyInput`)
/// - The period is zero (`Error::InvalidPeriod`)
/// - Input lengths don't match
/// - Input data is shorter than the period (`Error::InsufficientData`)
pub fn rolling_extrema_fused_nan_propagating<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
) -> Result<RollingExtremaOutput<T>> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if high.is_empty() || low.is_empty() {
        return Err(Error::EmptyInput);
    }

    if high.len() != low.len() {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", high.len(), low.len()),
        });
    }

    if high.len() < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: high.len(),
            indicator: "rolling_extrema_fused_nan_propagating",
        });
    }

    let n = high.len();
    let mut max_result = vec![T::nan(); n];
    let mut min_result = vec![T::nan(); n];

    // Track NaN count from both arrays combined
    let mut nan_count = 0usize;
    let mut max_deque: MonotonicDeque<T> = MonotonicDeque::new(period);
    let mut min_deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for i in 0..n {
        // Track NaN entering window (from either high or low)
        if is_invalid(high[i]) {
            nan_count += 1;
        }
        if is_invalid(low[i]) {
            nan_count += 1;
        }

        // Track NaN leaving window
        if i >= period {
            if is_invalid(high[i - period]) {
                nan_count -= 1;
            }
            if is_invalid(low[i - period]) {
                nan_count -= 1;
            }
        }

        // Update deques
        max_deque.push_max(i, high);
        min_deque.push_min(i, low);

        if i >= period - 1 {
            if nan_count > 0 {
                max_result[i] = T::nan();
                min_result[i] = T::nan();
            } else {
                max_result[i] = max_deque.get_extremum(high);
                min_result[i] = min_deque.get_extremum(low);
            }
        }
    }

    Ok(RollingExtremaOutput {
        max: max_result,
        min: min_result,
    })
}

/// Computes rolling max of `high` and rolling min of `low` into pre-allocated buffers.
///
/// This is the `_into` variant for buffer reuse.
///
/// # Arguments
///
/// * `high` - The high price data series
/// * `low` - The low price data series
/// * `period` - The window size for rolling calculations
/// * `output` - Pre-allocated output structure
///
/// # Returns
///
/// A `Result` containing the number of valid values computed.
pub fn rolling_extrema_fused_nan_propagating_into<T: SeriesElement>(
    high: &[T],
    low: &[T],
    period: usize,
    output: &mut RollingExtremaOutput<T>,
) -> Result<usize> {
    if period == 0 {
        return Err(Error::InvalidPeriod {
            period,
            reason: "period must be at least 1",
        });
    }

    if high.is_empty() || low.is_empty() {
        return Err(Error::EmptyInput);
    }

    if high.len() != low.len() {
        return Err(Error::LengthMismatch {
            description: format!("high has {} elements, low has {}", high.len(), low.len()),
        });
    }

    let n = high.len();

    if output.max.len() < n || output.min.len() < n {
        return Err(Error::BufferTooSmall {
            required: n,
            actual: output.max.len().min(output.min.len()),
            indicator: "rolling_extrema_fused_nan_propagating",
        });
    }

    if n < period {
        return Err(Error::InsufficientData {
            required: period,
            actual: n,
            indicator: "rolling_extrema_fused_nan_propagating",
        });
    }

    // Initialize with NaN
    for i in 0..(period - 1) {
        output.max[i] = T::nan();
        output.min[i] = T::nan();
    }

    let mut nan_count = 0usize;
    let mut max_deque: MonotonicDeque<T> = MonotonicDeque::new(period);
    let mut min_deque: MonotonicDeque<T> = MonotonicDeque::new(period);

    for i in 0..n {
        if is_invalid(high[i]) {
            nan_count += 1;
        }
        if is_invalid(low[i]) {
            nan_count += 1;
        }

        if i >= period {
            if is_invalid(high[i - period]) {
                nan_count -= 1;
            }
            if is_invalid(low[i - period]) {
                nan_count -= 1;
            }
        }

        max_deque.push_max(i, high);
        min_deque.push_min(i, low);

        if i >= period - 1 {
            if nan_count > 0 {
                output.max[i] = T::nan();
                output.min[i] = T::nan();
            } else {
                output.max[i] = max_deque.get_extremum(high);
                output.min[i] = min_deque.get_extremum(low);
            }
        }
    }

    Ok(n - period + 1)
}

#[cfg(test)]
mod tests {
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
    const EPSILON_F32: f32 = 1e-5;

    // ==================== MonotonicDeque Tests ====================

    #[test]
    fn test_monotonic_deque_new() {
        let deque: MonotonicDeque<f64> = MonotonicDeque::new(5);
        assert_eq!(deque.period(), 5);
        assert!(deque.is_empty());
        assert_eq!(deque.len(), 0);
    }

    #[test]
    fn test_monotonic_deque_push_max() {
        let data = vec![3.0_f64, 1.0, 4.0, 1.0, 5.0];
        let mut deque: MonotonicDeque<f64> = MonotonicDeque::new(3);

        deque.push_max(0, &data); // [3]
        assert_eq!(deque.front_index(), Some(0));

        deque.push_max(1, &data); // [3, 1] -> 3 is still max
        assert_eq!(deque.front_index(), Some(0));

        deque.push_max(2, &data); // [4] -> 4 is new max
        assert_eq!(deque.front_index(), Some(2));

        deque.push_max(3, &data); // [4, 1] -> 4 is still max
        assert_eq!(deque.front_index(), Some(2));

        deque.push_max(4, &data); // [5] -> 5 is new max, 4 expired
        assert_eq!(deque.front_index(), Some(4));
    }

    #[test]
    fn test_monotonic_deque_push_min() {
        let data = vec![3.0_f64, 5.0, 2.0, 4.0, 1.0];
        let mut deque: MonotonicDeque<f64> = MonotonicDeque::new(3);

        deque.push_min(0, &data); // [3]
        assert_eq!(deque.front_index(), Some(0));

        deque.push_min(1, &data); // [3, 5] -> 3 is still min
        assert_eq!(deque.front_index(), Some(0));

        deque.push_min(2, &data); // [2] -> 2 is new min
        assert_eq!(deque.front_index(), Some(2));

        deque.push_min(3, &data); // [2, 4] -> 2 is still min
        assert_eq!(deque.front_index(), Some(2));

        deque.push_min(4, &data); // [1] -> 1 is new min, 2 expired
        assert_eq!(deque.front_index(), Some(4));
    }

    #[test]
    fn test_monotonic_deque_clear() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let mut deque: MonotonicDeque<f64> = MonotonicDeque::new(3);

        deque.push_max(0, &data);
        deque.push_max(1, &data);
        assert!(!deque.is_empty());

        deque.clear();
        assert!(deque.is_empty());
        assert_eq!(deque.len(), 0);
    }

    #[test]
    fn test_monotonic_deque_nan_handling() {
        let data = vec![1.0_f64, f64::NAN, 3.0];
        let mut deque: MonotonicDeque<f64> = MonotonicDeque::new(3);

        deque.push_max(0, &data);
        deque.push_max(1, &data); // NaN should be skipped
        deque.push_max(2, &data);

        // 3 should be the max (NaN was skipped)
        assert_eq!(deque.front_index(), Some(2));
        assert!(approx_eq(deque.get_extremum(&data), 3.0, EPSILON));
    }

    // ==================== rolling_max Tests ====================

    #[test]
    fn test_rolling_max_basic() {
        let data = vec![1.0_f64, 3.0, 2.0, 5.0, 4.0];
        let result = rolling_max(&data, 3).unwrap();

        assert_eq!(result.len(), 5);

        // First 2 values are NaN
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());

        // max of [1,3,2] = 3
        assert!(approx_eq(result[2], 3.0, EPSILON));

        // max of [3,2,5] = 5
        assert!(approx_eq(result[3], 5.0, EPSILON));

        // max of [2,5,4] = 5
        assert!(approx_eq(result[4], 5.0, EPSILON));
    }

    #[test]
    fn test_rolling_max_period_one() {
        let data = vec![1.0_f64, 3.0, 2.0, 5.0, 4.0];
        let result = rolling_max(&data, 1).unwrap();

        // With period 1, max equals input
        for i in 0..5 {
            assert!(approx_eq(result[i], data[i], EPSILON));
        }
    }

    #[test]
    fn test_rolling_max_period_equals_length() {
        let data = vec![1.0_f64, 3.0, 2.0, 5.0, 4.0];
        let result = rolling_max(&data, 5).unwrap();

        // Only last value is valid
        for value in result.iter().take(4) {
            assert!(value.is_nan());
        }
        assert!(approx_eq(result[4], 5.0, EPSILON)); // max of all
    }

    #[test]
    fn test_rolling_max_constant_values() {
        let data = vec![5.0_f64; 10];
        let result = rolling_max(&data, 3).unwrap();

        for value in result.iter().skip(2) {
            assert!(approx_eq(*value, 5.0, EPSILON));
        }
    }

    #[test]
    fn test_rolling_max_f32() {
        let data = vec![1.0_f32, 3.0, 2.0, 5.0, 4.0];
        let result = rolling_max(&data, 3).unwrap();

        assert!(approx_eq(result[2], 3.0_f32, EPSILON_F32));
        assert!(approx_eq(result[3], 5.0_f32, EPSILON_F32));
    }

    #[test]
    fn test_rolling_max_matches_naive() {
        let data: Vec<f64> = (0..100)
            .map(|i| (f64::from(i) * 7.0).sin().mul_add(10.0, 50.0))
            .collect();

        for period in [2, 5, 10, 20, 50] {
            let optimized = rolling_max(&data, period).unwrap();
            let naive = rolling_max_naive(&data, period).unwrap();

            for i in 0..data.len() {
                assert!(
                    approx_eq(optimized[i], naive[i], EPSILON),
                    "Mismatch at index {i} for period {period}: optimized={}, naive={}",
                    optimized[i],
                    naive[i]
                );
            }
        }
    }

    // ==================== rolling_min Tests ====================

    #[test]
    fn test_rolling_min_basic() {
        let data = vec![5.0_f64, 3.0, 4.0, 1.0, 2.0];
        let result = rolling_min(&data, 3).unwrap();

        assert_eq!(result.len(), 5);

        // First 2 values are NaN
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());

        // min of [5,3,4] = 3
        assert!(approx_eq(result[2], 3.0, EPSILON));

        // min of [3,4,1] = 1
        assert!(approx_eq(result[3], 1.0, EPSILON));

        // min of [4,1,2] = 1
        assert!(approx_eq(result[4], 1.0, EPSILON));
    }

    #[test]
    fn test_rolling_min_period_one() {
        let data = vec![5.0_f64, 3.0, 4.0, 1.0, 2.0];
        let result = rolling_min(&data, 1).unwrap();

        // With period 1, min equals input
        for i in 0..5 {
            assert!(approx_eq(result[i], data[i], EPSILON));
        }
    }

    #[test]
    fn test_rolling_min_period_equals_length() {
        let data = vec![5.0_f64, 3.0, 4.0, 1.0, 2.0];
        let result = rolling_min(&data, 5).unwrap();

        // Only last value is valid
        for value in result.iter().take(4) {
            assert!(value.is_nan());
        }
        assert!(approx_eq(result[4], 1.0, EPSILON)); // min of all
    }

    #[test]
    fn test_rolling_min_constant_values() {
        let data = vec![5.0_f64; 10];
        let result = rolling_min(&data, 3).unwrap();

        for value in result.iter().skip(2) {
            assert!(approx_eq(*value, 5.0, EPSILON));
        }
    }

    #[test]
    fn test_rolling_min_f32() {
        let data = vec![5.0_f32, 3.0, 4.0, 1.0, 2.0];
        let result = rolling_min(&data, 3).unwrap();

        assert!(approx_eq(result[2], 3.0_f32, EPSILON_F32));
        assert!(approx_eq(result[3], 1.0_f32, EPSILON_F32));
    }

    #[test]
    fn test_rolling_min_matches_naive() {
        let data: Vec<f64> = (0..100)
            .map(|i| (f64::from(i) * 7.0).sin().mul_add(10.0, 50.0))
            .collect();

        for period in [2, 5, 10, 20, 50] {
            let optimized = rolling_min(&data, period).unwrap();
            let naive = rolling_min_naive(&data, period).unwrap();

            for i in 0..data.len() {
                assert!(
                    approx_eq(optimized[i], naive[i], EPSILON),
                    "Mismatch at index {i} for period {period}: optimized={}, naive={}",
                    optimized[i],
                    naive[i]
                );
            }
        }
    }

    // ==================== rolling_extrema Tests ====================

    #[test]
    fn test_rolling_extrema_basic() {
        let data = vec![3.0_f64, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let result = rolling_extrema(&data, 3).unwrap();

        assert_eq!(result.max.len(), 8);
        assert_eq!(result.min.len(), 8);

        // First 2 values are NaN
        assert!(result.max[0].is_nan());
        assert!(result.max[1].is_nan());
        assert!(result.min[0].is_nan());
        assert!(result.min[1].is_nan());

        // [3, 1, 4]: max=4, min=1
        assert!(approx_eq(result.max[2], 4.0, EPSILON));
        assert!(approx_eq(result.min[2], 1.0, EPSILON));

        // [1, 4, 1]: max=4, min=1
        assert!(approx_eq(result.max[3], 4.0, EPSILON));
        assert!(approx_eq(result.min[3], 1.0, EPSILON));

        // [1, 5, 9]: max=9, min=1
        assert!(approx_eq(result.max[5], 9.0, EPSILON));
        assert!(approx_eq(result.min[5], 1.0, EPSILON));
    }

    #[test]
    fn test_rolling_extrema_matches_separate() {
        let data: Vec<f64> = (0..50)
            .map(|i| (f64::from(i) * 0.1).sin().mul_add(10.0, 50.0))
            .collect();

        for period in [2, 5, 10, 20] {
            let fused = rolling_extrema(&data, period).unwrap();
            let max_separate = rolling_max(&data, period).unwrap();
            let min_separate = rolling_min(&data, period).unwrap();

            for i in 0..data.len() {
                assert!(
                    approx_eq(fused.max[i], max_separate[i], EPSILON),
                    "Max mismatch at index {i} for period {period}"
                );
                assert!(
                    approx_eq(fused.min[i], min_separate[i], EPSILON),
                    "Min mismatch at index {i} for period {period}"
                );
            }
        }
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_rolling_max_empty_input() {
        let data: Vec<f64> = vec![];
        let result = rolling_max(&data, 3);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_rolling_max_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = rolling_max(&data, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_rolling_max_period_exceeds_length() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = rolling_max(&data, 5);
        assert!(matches!(
            result,
            Err(Error::InsufficientData {
                required: 5,
                actual: 3,
                ..
            })
        ));
    }

    #[test]
    fn test_rolling_min_empty_input() {
        let data: Vec<f64> = vec![];
        let result = rolling_min(&data, 3);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    #[test]
    fn test_rolling_min_zero_period() {
        let data = vec![1.0_f64, 2.0, 3.0];
        let result = rolling_min(&data, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidPeriod { period: 0, .. })
        ));
    }

    #[test]
    fn test_rolling_extrema_empty_input() {
        let data: Vec<f64> = vec![];
        let result = rolling_extrema(&data, 3);
        assert!(matches!(result, Err(Error::EmptyInput)));
    }

    // ==================== Into Variant Tests ====================

    #[test]
    fn test_rolling_max_into_basic() {
        let data = vec![1.0_f64, 3.0, 2.0, 5.0, 4.0];
        let mut output = vec![0.0_f64; 5];

        let valid_count = rolling_max_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!(approx_eq(output[2], 3.0, EPSILON));
        assert!(approx_eq(output[3], 5.0, EPSILON));
    }

    #[test]
    fn test_rolling_min_into_basic() {
        let data = vec![5.0_f64, 3.0, 4.0, 1.0, 2.0];
        let mut output = vec![0.0_f64; 5];

        let valid_count = rolling_min_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!(approx_eq(output[2], 3.0, EPSILON));
        assert!(approx_eq(output[3], 1.0, EPSILON));
    }

    #[test]
    fn test_rolling_extrema_into_basic() {
        let data = vec![3.0_f64, 1.0, 4.0, 1.0, 5.0];
        let mut output = RollingExtremaOutput {
            max: vec![0.0_f64; 5],
            min: vec![0.0_f64; 5],
        };

        let valid_count = rolling_extrema_into(&data, 3, &mut output).unwrap();

        assert_eq!(valid_count, 3);
        assert!(approx_eq(output.max[2], 4.0, EPSILON));
        assert!(approx_eq(output.min[2], 1.0, EPSILON));
    }

    #[test]
    fn test_rolling_max_into_insufficient_output() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = vec![0.0_f64; 3]; // Too short

        let result = rolling_max_into(&data, 3, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_rolling_extrema_into_insufficient_output() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let mut output = RollingExtremaOutput {
            max: vec![0.0_f64; 3], // Too short
            min: vec![0.0_f64; 5],
        };

        let result = rolling_extrema_into(&data, 3, &mut output);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    // ==================== NaN Handling Tests ====================

    #[test]
    fn test_rolling_max_with_nan() {
        let data = vec![1.0_f64, f64::NAN, 3.0, 4.0, 5.0];
        let result = rolling_max(&data, 3).unwrap();

        // Windows containing NaN:
        // [1, NaN, 3]: max should be 3 (NaN skipped)
        assert!(approx_eq(result[2], 3.0, EPSILON));

        // [NaN, 3, 4]: max should be 4
        assert!(approx_eq(result[3], 4.0, EPSILON));

        // [3, 4, 5]: max should be 5
        assert!(approx_eq(result[4], 5.0, EPSILON));
    }

    #[test]
    fn test_rolling_min_with_nan() {
        let data = vec![5.0_f64, f64::NAN, 3.0, 2.0, 1.0];
        let result = rolling_min(&data, 3).unwrap();

        // [5, NaN, 3]: min should be 3 (NaN skipped)
        assert!(approx_eq(result[2], 3.0, EPSILON));

        // [NaN, 3, 2]: min should be 2
        assert!(approx_eq(result[3], 2.0, EPSILON));

        // [3, 2, 1]: min should be 1
        assert!(approx_eq(result[4], 1.0, EPSILON));
    }

    #[test]
    fn test_rolling_max_all_nan() {
        let data = vec![f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN];
        let result = rolling_max(&data, 3).unwrap();

        // All values are NaN, so all results should be NaN
        for &val in &result {
            assert!(val.is_nan());
        }
    }

    // ==================== Property Tests ====================

    #[test]
    fn test_rolling_max_always_gte_input() {
        let data: Vec<f64> = (0..50)
            .map(|i| (f64::from(i) * 0.1).sin().mul_add(10.0, 50.0))
            .collect();
        let result = rolling_max(&data, 5).unwrap();

        for i in 4..data.len() {
            assert!(
                result[i] >= data[i],
                "Max at {i} should be >= current value"
            );
        }
    }

    #[test]
    fn test_rolling_min_always_lte_input() {
        let data: Vec<f64> = (0..50)
            .map(|i| (f64::from(i) * 0.1).sin().mul_add(10.0, 50.0))
            .collect();
        let result = rolling_min(&data, 5).unwrap();

        for i in 4..data.len() {
            assert!(
                result[i] <= data[i],
                "Min at {i} should be <= current value"
            );
        }
    }

    #[test]
    fn test_rolling_max_gte_rolling_min() {
        let data: Vec<f64> = (0..50)
            .map(|i| (f64::from(i) * 0.1).sin().mul_add(10.0, 50.0))
            .collect();
        let extrema = rolling_extrema(&data, 5).unwrap();

        for i in 4..data.len() {
            assert!(
                extrema.max[i] >= extrema.min[i],
                "Max should be >= min at index {i}"
            );
        }
    }

    #[test]
    fn test_rolling_nan_count() {
        let data = vec![1.0_f64; 100];

        for period in [1, 2, 5, 10, 50] {
            let max_result = rolling_max(&data, period).unwrap();
            let min_result = rolling_min(&data, period).unwrap();

            let max_nan_count = max_result.iter().filter(|x| x.is_nan()).count();
            let min_nan_count = min_result.iter().filter(|x| x.is_nan()).count();

            assert_eq!(
                max_nan_count,
                period - 1,
                "Max NaN count for period {period}"
            );
            assert_eq!(
                min_nan_count,
                period - 1,
                "Min NaN count for period {period}"
            );
        }
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_rolling_extrema_ascending() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let result = rolling_extrema(&data, 3).unwrap();

        // For ascending data, min is always the oldest in window
        // max is always the newest
        assert!(approx_eq(result.max[2], 3.0, EPSILON));
        assert!(approx_eq(result.max[3], 4.0, EPSILON));
        assert!(approx_eq(result.max[4], 5.0, EPSILON));

        assert!(approx_eq(result.min[2], 1.0, EPSILON));
        assert!(approx_eq(result.min[3], 2.0, EPSILON));
        assert!(approx_eq(result.min[4], 3.0, EPSILON));
    }

    #[test]
    fn test_rolling_extrema_descending() {
        let data = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
        let result = rolling_extrema(&data, 3).unwrap();

        // For descending data, max is always the oldest in window
        // min is always the newest
        assert!(approx_eq(result.max[2], 5.0, EPSILON));
        assert!(approx_eq(result.max[3], 4.0, EPSILON));
        assert!(approx_eq(result.max[4], 3.0, EPSILON));

        assert!(approx_eq(result.min[2], 3.0, EPSILON));
        assert!(approx_eq(result.min[3], 2.0, EPSILON));
        assert!(approx_eq(result.min[4], 1.0, EPSILON));
    }

    #[test]
    fn test_rolling_extrema_large_period() {
        let data: Vec<f64> = (0..100).map(f64::from).collect();
        let result = rolling_extrema(&data, 50).unwrap();

        // First 49 values should be NaN
        for i in 0..49 {
            assert!(result.max[i].is_nan());
            assert!(result.min[i].is_nan());
        }

        // At index 49: window is 0..49, max=49, min=0
        assert!(approx_eq(result.max[49], 49.0, EPSILON));
        assert!(approx_eq(result.min[49], 0.0, EPSILON));

        // At index 99: window is 50..99, max=99, min=50
        assert!(approx_eq(result.max[99], 99.0, EPSILON));
        assert!(approx_eq(result.min[99], 50.0, EPSILON));
    }

    #[test]
    fn test_rolling_extrema_negative_values() {
        let data = vec![-5.0_f64, -3.0, -1.0, -4.0, -2.0];
        let result = rolling_extrema(&data, 3).unwrap();

        // [-5, -3, -1]: max=-1, min=-5
        assert!(approx_eq(result.max[2], -1.0, EPSILON));
        assert!(approx_eq(result.min[2], -5.0, EPSILON));

        // [-3, -1, -4]: max=-1, min=-4
        assert!(approx_eq(result.max[3], -1.0, EPSILON));
        assert!(approx_eq(result.min[3], -4.0, EPSILON));
    }

    #[test]
    fn test_rolling_extrema_very_large_values() {
        let data = vec![1e15_f64, 1e16_f64, 1e14_f64, 1e15_f64, 1e17_f64];
        let result = rolling_extrema(&data, 3).unwrap();

        assert!(approx_eq(result.max[2], 1e16, 1e6));
        assert!(approx_eq(result.min[2], 1e14, 1e6));
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_rolling_max_and_max_into_produce_same_result() {
        let data = vec![1.0_f64, 3.0, 2.0, 5.0, 4.0, 6.0, 3.0, 8.0];
        let result1 = rolling_max(&data, 3).unwrap();

        let mut result2 = vec![0.0_f64; data.len()];
        rolling_max_into(&data, 3, &mut result2).unwrap();

        for i in 0..data.len() {
            assert!(
                approx_eq(result1[i], result2[i], EPSILON),
                "Mismatch at {i}"
            );
        }
    }

    #[test]
    fn test_rolling_min_and_min_into_produce_same_result() {
        let data = vec![5.0_f64, 3.0, 4.0, 1.0, 2.0, 6.0, 3.0, 0.0];
        let result1 = rolling_min(&data, 3).unwrap();

        let mut result2 = vec![0.0_f64; data.len()];
        rolling_min_into(&data, 3, &mut result2).unwrap();

        for i in 0..data.len() {
            assert!(
                approx_eq(result1[i], result2[i], EPSILON),
                "Mismatch at {i}"
            );
        }
    }

    #[test]
    fn test_rolling_extrema_and_extrema_into_produce_same_result() {
        let data = vec![3.0_f64, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let result1 = rolling_extrema(&data, 3).unwrap();

        let mut result2 = RollingExtremaOutput {
            max: vec![0.0_f64; data.len()],
            min: vec![0.0_f64; data.len()],
        };
        rolling_extrema_into(&data, 3, &mut result2).unwrap();

        for i in 0..data.len() {
            assert!(
                approx_eq(result1.max[i], result2.max[i], EPSILON),
                "Max mismatch at {i}"
            );
            assert!(
                approx_eq(result1.min[i], result2.min[i], EPSILON),
                "Min mismatch at {i}"
            );
        }
    }

    // ==================== Clone Tests ====================

    #[test]
    fn test_monotonic_deque_clone() {
        let data = vec![3.0_f64, 1.0, 4.0];
        let mut deque: MonotonicDeque<f64> = MonotonicDeque::new(3);
        deque.push_max(0, &data);
        deque.push_max(1, &data);

        let cloned = deque.clone();
        assert_eq!(cloned.len(), deque.len());
        assert_eq!(cloned.period(), deque.period());
    }

    #[test]
    fn test_rolling_extrema_output_clone() {
        let data = vec![3.0_f64, 1.0, 4.0, 1.0, 5.0];
        let result = rolling_extrema(&data, 3).unwrap();
        let cloned = result.clone();

        for i in 0..result.max.len() {
            assert!(approx_eq(result.max[i], cloned.max[i], EPSILON));
            assert!(approx_eq(result.min[i], cloned.min[i], EPSILON));
        }
    }

    // ==================== NaN Propagating Tests ====================

    #[test]
    fn test_rolling_max_nan_propagating_basic() {
        let data = vec![1.0_f64, 3.0, 2.0, 5.0, 4.0];
        let result = rolling_max_nan_propagating(&data, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(approx_eq(result[2], 3.0, EPSILON));
        assert!(approx_eq(result[3], 5.0, EPSILON));
        assert!(approx_eq(result[4], 5.0, EPSILON));
    }

    #[test]
    fn test_rolling_max_nan_propagating_with_nan() {
        let data = vec![1.0_f64, f64::NAN, 3.0, 4.0, 5.0];
        let result = rolling_max_nan_propagating(&data, 3).unwrap();

        // Window [1, NaN, 3] should be NaN (NaN propagates)
        assert!(result[2].is_nan());
        // Window [NaN, 3, 4] should be NaN
        assert!(result[3].is_nan());
        // Window [3, 4, 5] should be 5
        assert!(approx_eq(result[4], 5.0, EPSILON));
    }

    #[test]
    fn test_rolling_min_nan_propagating_with_nan() {
        let data = vec![5.0_f64, f64::NAN, 3.0, 2.0, 1.0];
        let result = rolling_min_nan_propagating(&data, 3).unwrap();

        // Window [5, NaN, 3] should be NaN
        assert!(result[2].is_nan());
        // Window [NaN, 3, 2] should be NaN
        assert!(result[3].is_nan());
        // Window [3, 2, 1] should be 1
        assert!(approx_eq(result[4], 1.0, EPSILON));
    }

    #[test]
    fn test_rolling_extrema_fused_nan_propagating_basic() {
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5];
        let low = vec![9.0_f64, 10.0, 11.0, 10.5, 11.5];

        let result = rolling_extrema_fused_nan_propagating(&high, &low, 3).unwrap();

        assert!(result.max[0].is_nan());
        assert!(result.max[1].is_nan());
        // max of high[0..3] = 12
        assert!(approx_eq(result.max[2], 12.0, EPSILON));
        // min of low[0..3] = 9
        assert!(approx_eq(result.min[2], 9.0, EPSILON));
    }

    #[test]
    fn test_rolling_extrema_fused_nan_propagating_with_nan_in_high() {
        // NaN at index 2, period 3
        // Window at index 2: [0,1,2] - contains NaN
        // Window at index 3: [1,2,3] - contains NaN
        // Window at index 4: [2,3,4] - contains NaN
        // Window at index 5: [3,4,5] - first without NaN
        let high = vec![10.0_f64, 11.0, f64::NAN, 11.5, 12.5, 13.0];
        let low = vec![9.0_f64, 10.0, 11.0, 10.5, 11.5, 12.0];

        let result = rolling_extrema_fused_nan_propagating(&high, &low, 3).unwrap();

        // Window contains NaN in high at index 2
        assert!(result.max[2].is_nan());
        assert!(result.min[2].is_nan());
        // Window [11.0, NaN, 11.5] still has NaN
        assert!(result.max[3].is_nan());
        assert!(result.min[3].is_nan());
        // Window [NaN, 11.5, 12.5] still has NaN
        assert!(result.max[4].is_nan());
        assert!(result.min[4].is_nan());
        // Window [11.5, 12.5, 13.0] - first without NaN
        assert!(approx_eq(result.max[5], 13.0, EPSILON));
        assert!(approx_eq(result.min[5], 10.5, EPSILON));
    }

    #[test]
    fn test_rolling_extrema_fused_nan_propagating_with_nan_in_low() {
        // NaN at index 2 in low, period 3
        // Window at index 5: [3,4,5] - first without NaN
        let high = vec![10.0_f64, 11.0, 12.0, 11.5, 12.5, 13.0];
        let low = vec![9.0_f64, 10.0, f64::NAN, 10.5, 11.5, 12.0];

        let result = rolling_extrema_fused_nan_propagating(&high, &low, 3).unwrap();

        // Window contains NaN in low at index 2
        assert!(result.max[2].is_nan());
        assert!(result.min[2].is_nan());
        // Window [10.0, NaN, 10.5] still has NaN
        assert!(result.max[3].is_nan());
        assert!(result.min[3].is_nan());
        // Window [NaN, 10.5, 11.5] still has NaN
        assert!(result.max[4].is_nan());
        assert!(result.min[4].is_nan());
        // Window [10.5, 11.5, 12.0] - now clean
        assert!(approx_eq(result.max[5], 13.0, EPSILON));
        assert!(approx_eq(result.min[5], 10.5, EPSILON));
    }
}
