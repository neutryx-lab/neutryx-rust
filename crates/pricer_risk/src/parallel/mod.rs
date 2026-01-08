//! Rayon-based parallelisation utilities.
//!
//! This module provides helpers for parallel processing of portfolio
//! data using Rayon.
//!
//! # Performance Targets
//!
//! - >80% parallel efficiency on 8+ cores
//! - Minimal memory allocation in hot paths
//! - Batch processing for optimal cache utilisation

use rayon::prelude::*;

/// Batch size for parallel processing.
///
/// Empirically tuned for cache efficiency.
pub const DEFAULT_BATCH_SIZE: usize = 64;

/// Processes items in parallel batches.
///
/// # Arguments
///
/// * `items` - Slice of items to process
/// * `batch_size` - Number of items per batch
/// * `processor` - Function to apply to each batch
///
/// # Returns
///
/// Vector of results from each batch.
pub fn process_in_batches<T, R, F>(items: &[T], batch_size: usize, processor: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&[T]) -> R + Sync + Send,
{
    items.par_chunks(batch_size.max(1)).map(processor).collect()
}

/// Parallel map with specified batch size.
///
/// Maps each item through a function in parallel.
///
/// # Arguments
///
/// * `items` - Slice of items to process
/// * `mapper` - Function to apply to each item
///
/// # Returns
///
/// Vector of mapped results.
pub fn parallel_map<T, R, F>(items: &[T], mapper: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync + Send,
{
    items.par_iter().map(mapper).collect()
}

/// Parallel reduce over items.
///
/// # Arguments
///
/// * `items` - Slice of items to reduce
/// * `identity` - Identity value for reduction
/// * `mapper` - Function to extract value from item
/// * `reducer` - Associative reduction function
///
/// # Returns
///
/// Reduced value.
pub fn parallel_reduce<T, R, M, Red>(items: &[T], identity: R, mapper: M, reducer: Red) -> R
where
    T: Sync,
    R: Send + Sync + Copy,
    M: Fn(&T) -> R + Sync + Send,
    Red: Fn(R, R) -> R + Sync + Send,
{
    items.par_iter().map(mapper).reduce(|| identity, reducer)
}

/// Parallel sum of values.
pub fn parallel_sum<T, F>(items: &[T], extractor: F) -> f64
where
    T: Sync,
    F: Fn(&T) -> f64 + Sync + Send,
{
    items.par_iter().map(extractor).sum()
}

/// Configuration for parallel execution.
#[derive(Clone, Debug)]
pub struct ParallelConfig {
    /// Batch size for chunked processing
    pub batch_size: usize,
    /// Minimum items before using parallelism
    pub parallel_threshold: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            batch_size: DEFAULT_BATCH_SIZE,
            parallel_threshold: 100,
        }
    }
}

impl ParallelConfig {
    /// Creates a new parallel configuration.
    pub fn new(batch_size: usize, parallel_threshold: usize) -> Self {
        Self {
            batch_size: batch_size.max(1),
            parallel_threshold,
        }
    }

    /// Returns whether to use parallel processing for the given item count.
    #[inline]
    pub fn should_parallelize(&self, n_items: usize) -> bool {
        n_items >= self.parallel_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_in_batches() {
        let items: Vec<i32> = (0..100).collect();
        let sums: Vec<i32> = process_in_batches(&items, 10, |batch| batch.iter().sum());

        assert_eq!(sums.len(), 10);
        assert_eq!(sums.iter().sum::<i32>(), (0..100).sum());
    }

    #[test]
    fn test_parallel_map() {
        let items: Vec<i32> = (0..100).collect();
        let doubled: Vec<i32> = parallel_map(&items, |&x| x * 2);

        assert_eq!(doubled.len(), 100);
        assert_eq!(doubled[50], 100);
    }

    #[test]
    fn test_parallel_reduce() {
        let items: Vec<i32> = (1..=10).collect();
        let sum = parallel_reduce(&items, 0, |&x| x, |a, b| a + b);
        assert_eq!(sum, 55);
    }

    #[test]
    fn test_parallel_sum() {
        let items: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let sum = parallel_sum(&items, |&x| x);
        assert_eq!(sum, 15.0);
    }

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelConfig::default();
        assert_eq!(config.batch_size, DEFAULT_BATCH_SIZE);
        assert_eq!(config.parallel_threshold, 100);
    }

    #[test]
    fn test_should_parallelize() {
        let config = ParallelConfig::default();
        assert!(!config.should_parallelize(50));
        assert!(config.should_parallelize(100));
        assert!(config.should_parallelize(1000));
    }
}
