//! Batch processing utilities for parallel indicator computation.
//!
//! This module provides utilities for computing indicators on multiple time series
//! in parallel using Rayon when the `parallel` feature is enabled.
//!
//! # Feature Flag
//!
//! This module requires the `parallel` feature to be enabled:
//!
//! ```toml
//! [dependencies]
//! liq-ta = { version = "0.1", features = ["parallel"] }
//! ```
//!
//! # Example
//!
//! ```ignore
//! use liq_ta::batch::BatchProcessor;
//! use liq_ta::indicators::sma::sma;
//!
//! let series = vec![
//!     vec![1.0, 2.0, 3.0, 4.0, 5.0],
//!     vec![5.0, 4.0, 3.0, 2.0, 1.0],
//!     vec![2.0, 4.0, 6.0, 8.0, 10.0],
//! ];
//!
//! // Process all series in parallel
//! let results: Vec<Vec<f64>> = BatchProcessor::new()
//!     .process(&series, |s| sma(s, 3))
//!     .unwrap();
//! ```

use crate::error::Result;
use crate::traits::SeriesElement;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Batch processor for parallel indicator computation.
///
/// Provides methods to compute indicators on multiple time series in parallel
/// when the `parallel` feature is enabled.
#[derive(Debug, Default, Clone)]
pub struct BatchProcessor {
    /// Minimum number of elements per series to use parallel processing.
    /// Series below this threshold will be processed sequentially.
    min_parallel_threshold: usize,
}

impl BatchProcessor {
    /// Creates a new batch processor with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            min_parallel_threshold: 1000,
        }
    }

    /// Sets the minimum number of series required to use parallel processing.
    ///
    /// If the number of series is below this threshold, sequential processing
    /// is used instead to avoid parallel overhead.
    #[must_use]
    pub const fn min_parallel_threshold(mut self, threshold: usize) -> Self {
        self.min_parallel_threshold = threshold;
        self
    }

    /// Processes multiple time series in parallel, applying the given indicator function.
    ///
    /// When the `parallel` feature is enabled and there are enough series,
    /// this uses Rayon's parallel iterator for concurrent processing.
    ///
    /// # Arguments
    ///
    /// * `series` - A slice of time series to process
    /// * `indicator_fn` - A function that computes an indicator on a single series
    ///
    /// # Returns
    ///
    /// A vector of results, one for each input series.
    ///
    /// # Errors
    ///
    /// Returns an error if any indicator computation fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use liq_ta::batch::BatchProcessor;
    /// use liq_ta::indicators::ema::ema;
    ///
    /// let series = vec![
    ///     vec![1.0, 2.0, 3.0, 4.0, 5.0],
    ///     vec![5.0, 4.0, 3.0, 2.0, 1.0],
    /// ];
    ///
    /// let results = BatchProcessor::new()
    ///     .process(&series, |s| ema(s, 3))
    ///     .unwrap();
    /// ```
    #[cfg(feature = "parallel")]
    pub fn process<T, F, R>(&self, series: &[Vec<T>], indicator_fn: F) -> Result<Vec<R>>
    where
        T: SeriesElement + Send + Sync,
        F: Fn(&[T]) -> Result<R> + Send + Sync,
        R: Send,
    {
        if series.len() < self.min_parallel_threshold {
            // Sequential fallback for small batches
            let mut results = Vec::with_capacity(series.len());
            for s in series {
                results.push(indicator_fn(s)?);
            }
            Ok(results)
        } else {
            // Parallel processing
            series
                .par_iter()
                .map(|s| indicator_fn(s))
                .collect::<Result<Vec<R>>>()
        }
    }

    /// Sequential version when parallel feature is disabled.
    ///
    /// # Errors
    ///
    /// Returns an error if any indicator computation fails.
    #[cfg(not(feature = "parallel"))]
    pub fn process<T, F, R>(&self, series: &[Vec<T>], indicator_fn: F) -> Result<Vec<R>>
    where
        T: SeriesElement,
        F: Fn(&[T]) -> Result<R>,
    {
        let mut results = Vec::with_capacity(series.len());
        for s in series {
            results.push(indicator_fn(s)?);
        }
        Ok(results)
    }

    /// Processes multiple time series in parallel with references to slices.
    ///
    /// Similar to [`process`](Self::process), but accepts slices instead of owned vectors.
    ///
    /// # Errors
    ///
    /// Returns an error if any indicator computation fails.
    #[cfg(feature = "parallel")]
    pub fn process_refs<T, F, R>(&self, series: &[&[T]], indicator_fn: F) -> Result<Vec<R>>
    where
        T: SeriesElement + Send + Sync,
        F: Fn(&[T]) -> Result<R> + Send + Sync,
        R: Send,
    {
        if series.len() < self.min_parallel_threshold {
            let mut results = Vec::with_capacity(series.len());
            for s in series {
                results.push(indicator_fn(s)?);
            }
            Ok(results)
        } else {
            series
                .par_iter()
                .map(|s| indicator_fn(s))
                .collect::<Result<Vec<R>>>()
        }
    }

    /// Sequential version when parallel feature is disabled.
    ///
    /// # Errors
    ///
    /// Returns an error if any indicator computation fails.
    #[cfg(not(feature = "parallel"))]
    pub fn process_refs<T, F, R>(&self, series: &[&[T]], indicator_fn: F) -> Result<Vec<R>>
    where
        T: SeriesElement,
        F: Fn(&[T]) -> Result<R>,
    {
        let mut results = Vec::with_capacity(series.len());
        for s in series {
            results.push(indicator_fn(s)?);
        }
        Ok(results)
    }
}

/// Convenience function to process multiple series in parallel.
///
/// This is a shorthand for creating a `BatchProcessor` with default settings.
///
/// # Example
///
/// ```ignore
/// use liq_ta::batch::process_batch;
/// use liq_ta::indicators::sma::sma;
///
/// let series = vec![
///     vec![1.0, 2.0, 3.0, 4.0, 5.0],
///     vec![5.0, 4.0, 3.0, 2.0, 1.0],
/// ];
///
/// let results = process_batch(&series, |s| sma(s, 3)).unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if any indicator computation fails.
#[cfg(feature = "parallel")]
pub fn process_batch<T, F, R>(series: &[Vec<T>], indicator_fn: F) -> Result<Vec<R>>
where
    T: SeriesElement + Send + Sync,
    F: Fn(&[T]) -> Result<R> + Send + Sync,
    R: Send,
{
    BatchProcessor::new().process(series, indicator_fn)
}

/// Sequential version when parallel feature is disabled.
///
/// # Errors
///
/// Returns an error if any indicator computation fails.
#[cfg(not(feature = "parallel"))]
pub fn process_batch<T, F, R>(series: &[Vec<T>], indicator_fn: F) -> Result<Vec<R>>
where
    T: SeriesElement,
    F: Fn(&[T]) -> Result<R>,
{
    BatchProcessor::new().process(series, indicator_fn)
}

/// Processes multiple OHLC datasets in parallel.
///
/// This is specialized for indicators that require OHLC data (high, low, close).
///
/// # Example
///
/// ```ignore
/// use liq_ta::batch::process_ohlc_batch;
/// use liq_ta::indicators::atr::atr;
///
/// let datasets: Vec<(Vec<f64>, Vec<f64>, Vec<f64>)> = vec![
///     // (high, low, close) for each symbol
///     (vec![10.0, 11.0, 12.0], vec![9.0, 10.0, 11.0], vec![9.5, 10.5, 11.5]),
///     (vec![20.0, 21.0, 22.0], vec![19.0, 20.0, 21.0], vec![19.5, 20.5, 21.5]),
/// ];
///
/// let results = process_ohlc_batch(&datasets, |h, l, c| atr(h, l, c, 2)).unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if any indicator computation fails.
#[cfg(feature = "parallel")]
pub fn process_ohlc_batch<T, F, R>(
    datasets: &[(Vec<T>, Vec<T>, Vec<T>)],
    indicator_fn: F,
) -> Result<Vec<R>>
where
    T: SeriesElement + Send + Sync,
    F: Fn(&[T], &[T], &[T]) -> Result<R> + Send + Sync,
    R: Send,
{
    if datasets.len() < 4 {
        // Sequential for small batches
        let mut results = Vec::with_capacity(datasets.len());
        for (h, l, c) in datasets {
            results.push(indicator_fn(h, l, c)?);
        }
        Ok(results)
    } else {
        datasets
            .par_iter()
            .map(|(h, l, c)| indicator_fn(h, l, c))
            .collect::<Result<Vec<R>>>()
    }
}

/// Sequential version when parallel feature is disabled.
///
/// # Errors
///
/// Returns an error if any indicator computation fails.
#[cfg(not(feature = "parallel"))]
pub fn process_ohlc_batch<T, F, R>(
    datasets: &[(Vec<T>, Vec<T>, Vec<T>)],
    indicator_fn: F,
) -> Result<Vec<R>>
where
    T: SeriesElement,
    F: Fn(&[T], &[T], &[T]) -> Result<R>,
{
    let mut results = Vec::with_capacity(datasets.len());
    for (h, l, c) in datasets {
        results.push(indicator_fn(h, l, c)?);
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::sma::sma;

    #[test]
    fn test_batch_processor_sequential() {
        let series = vec![
            vec![1.0_f64, 2.0, 3.0, 4.0, 5.0],
            vec![5.0, 4.0, 3.0, 2.0, 1.0],
            vec![2.0, 4.0, 6.0, 8.0, 10.0],
        ];

        let results = BatchProcessor::new()
            .min_parallel_threshold(100) // Force sequential
            .process(&series, |s| sma(s, 3))
            .unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].len(), 5);
        assert_eq!(results[1].len(), 5);
        assert_eq!(results[2].len(), 5);
    }

    #[test]
    fn test_process_batch_convenience() {
        let series = vec![
            vec![1.0_f64, 2.0, 3.0, 4.0, 5.0],
            vec![5.0, 4.0, 3.0, 2.0, 1.0],
        ];

        let results = process_batch(&series, |s| sma(s, 3)).unwrap();

        assert_eq!(results.len(), 2);

        // Verify SMA values
        // Series 1: SMA(3) of [1, 2, 3, 4, 5] at index 2 = (1+2+3)/3 = 2.0
        assert!((results[0][2] - 2.0).abs() < 1e-10);

        // Series 2: SMA(3) of [5, 4, 3, 2, 1] at index 2 = (5+4+3)/3 = 4.0
        assert!((results[1][2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_batch_processor_refs() {
        let s1 = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
        let s2 = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
        let series: Vec<&[f64]> = vec![&s1, &s2];

        let results = BatchProcessor::new()
            .process_refs(&series, |s| sma(s, 3))
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_batch_error_propagation() {
        let series = vec![
            vec![1.0_f64, 2.0], // Too short for period 3
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
        ];

        let result = process_batch(&series, |s| sma(s, 3));

        assert!(result.is_err());
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn test_batch_processor_parallel() {
        // Create many series to trigger parallel processing
        let series: Vec<Vec<f64>> = (0..100)
            .map(|i| (0..10).map(|j| (i * 10 + j) as f64).collect())
            .collect();

        let results = BatchProcessor::new()
            .min_parallel_threshold(10) // Force parallel
            .process(&series, |s| sma(s, 3))
            .unwrap();

        assert_eq!(results.len(), 100);
        for result in &results {
            assert_eq!(result.len(), 10);
        }
    }
}
