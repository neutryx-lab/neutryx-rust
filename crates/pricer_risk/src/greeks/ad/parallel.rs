//! Parallel adjoint aggregation for Monte Carlo Greeks computation.

use std::sync::atomic::{AtomicUsize, Ordering};

use num_traits::Float;
use rayon::prelude::*;

use super::loops::AdjointAccumulator;

/// Configuration for parallel Greeks computation.
#[derive(Clone, Copy, Debug)]
pub struct ParallelGreeksConfig {
    /// Minimum paths per thread.
    pub min_paths_per_thread: usize,
    /// Whether to use thread-local storage.
    pub use_thread_local: bool,
    /// Chunk size for parallel iteration.
    pub chunk_size: usize,
}

impl Default for ParallelGreeksConfig {
    fn default() -> Self {
        Self {
            min_paths_per_thread: 100,
            use_thread_local: true,
            chunk_size: 0,
        }
    }
}

impl ParallelGreeksConfig {
    /// Creates a new configuration with the given minimum paths per thread.
    #[inline]
    pub fn with_min_paths(min_paths_per_thread: usize) -> Self {
        Self {
            min_paths_per_thread,
            ..Default::default()
        }
    }

    /// Sets the chunk size for parallel iteration.
    #[inline]
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    /// Determines if parallelisation should be used for the given path count.
    #[inline]
    pub fn should_parallelise(&self, n_paths: usize) -> bool {
        let n_threads = rayon::current_num_threads();
        n_paths >= self.min_paths_per_thread * n_threads
    }

    /// Computes the effective chunk size for the given path count.
    #[inline]
    pub fn effective_chunk_size(&self, n_paths: usize) -> usize {
        if self.chunk_size > 0 {
            self.chunk_size
        } else {
            let n_threads = rayon::current_num_threads();
            n_paths.div_ceil(n_threads)
        }
    }
}

/// Parallel Greeks computer using thread-local accumulators.
#[derive(Debug)]
pub struct ParallelGreeksComputer {
    config: ParallelGreeksConfig,
    paths_processed: AtomicUsize,
}

impl Clone for ParallelGreeksComputer {
    fn clone(&self) -> Self {
        Self {
            config: self.config,
            paths_processed: AtomicUsize::new(self.paths_processed.load(Ordering::Relaxed)),
        }
    }
}

impl ParallelGreeksComputer {
    /// Creates a new parallel Greeks computer.
    #[inline]
    pub fn new(config: ParallelGreeksConfig) -> Self {
        Self {
            config,
            paths_processed: AtomicUsize::new(0),
        }
    }

    /// Creates a new computer with default configuration.
    #[inline]
    pub fn default_config() -> Self { Self::new(ParallelGreeksConfig::default()) }

    /// Returns the configuration.
    #[inline]
    pub fn config(&self) -> &ParallelGreeksConfig { &self.config }

    /// Returns the number of paths processed.
    #[inline]
    pub fn paths_processed(&self) -> usize { self.paths_processed.load(Ordering::Relaxed) }

    /// Resets the paths processed counter.
    #[inline]
    pub fn reset(&self) { self.paths_processed.store(0, Ordering::Relaxed); }

    /// Computes Greeks in parallel using the provided path function.
    pub fn compute_parallel<F>(&self, n_paths: usize, path_fn: F) -> AdjointAccumulator<f64>
    where
        F: Fn(usize) -> AdjointAccumulator<f64> + Send + Sync,
    {
        self.paths_processed.store(0, Ordering::Relaxed);

        if !self.config.should_parallelise(n_paths) {
            return self.compute_sequential(n_paths, path_fn);
        }

        let result = (0..n_paths)
            .into_par_iter()
            .fold(AdjointAccumulator::new, |mut acc, path_idx| {
                let contribution = path_fn(path_idx);
                acc.merge(&contribution);
                acc
            })
            .reduce(AdjointAccumulator::new, |mut a, b| {
                a.merge(&b);
                a
            });

        self.paths_processed.store(n_paths, Ordering::Relaxed);
        result
    }

    fn compute_sequential<F>(&self, n_paths: usize, path_fn: F) -> AdjointAccumulator<f64>
    where
        F: Fn(usize) -> AdjointAccumulator<f64>,
    {
        let mut accumulator = AdjointAccumulator::new();

        for path_idx in 0..n_paths {
            let contribution = path_fn(path_idx);
            accumulator.merge(&contribution);
        }

        self.paths_processed.store(n_paths, Ordering::Relaxed);
        accumulator
    }

    /// Computes Greeks in parallel with chunked processing for large path
    /// counts.
    pub fn compute_parallel_chunked<F>(&self, n_paths: usize, path_fn: F) -> AdjointAccumulator<f64>
    where
        F: Fn(usize) -> AdjointAccumulator<f64> + Send + Sync,
    {
        let chunk_size = self.config.effective_chunk_size(n_paths);

        self.paths_processed.store(0, Ordering::Relaxed);

        let result = (0..n_paths)
            .into_par_iter()
            .with_min_len(chunk_size)
            .fold(AdjointAccumulator::new, |mut acc, path_idx| {
                let contribution = path_fn(path_idx);
                acc.merge(&contribution);
                acc
            })
            .reduce(AdjointAccumulator::new, |mut a, b| {
                a.merge(&b);
                a
            });

        self.paths_processed.store(n_paths, Ordering::Relaxed);
        result
    }
}

/// Result of parallel Greeks computation with averaged values and statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct ParallelGreeksResult<T: Float> {
    /// Delta sensitivity.
    pub delta: T,
    /// Gamma sensitivity.
    pub gamma: T,
    /// Vega sensitivity.
    pub vega: T,
    /// Theta sensitivity.
    pub theta: T,
    /// Rho sensitivity.
    pub rho: T,
    /// Number of paths used.
    pub n_paths: usize,
    /// Standard error of the delta estimate.
    pub delta_std_error: T,
}

impl<T: Float> ParallelGreeksResult<T> {
    /// Creates a new result from an accumulator.
    pub fn from_accumulator(acc: &AdjointAccumulator<T>) -> Self
    where
        T: std::ops::Div<Output = T>,
    {
        let (delta, gamma, vega, theta, rho) = acc.averaged();
        Self {
            delta,
            gamma,
            vega,
            theta,
            rho,
            n_paths: acc.count(),
            delta_std_error: T::zero(),
        }
    }
}

/// Thread-safe Greeks aggregator using atomic operations (CAS-based).
#[derive(Debug)]
pub struct AtomicGreeksAggregator {
    delta_bits: AtomicUsize,
    gamma_bits: AtomicUsize,
    vega_bits: AtomicUsize,
    theta_bits: AtomicUsize,
    rho_bits: AtomicUsize,
    count: AtomicUsize,
}

impl Default for AtomicGreeksAggregator {
    fn default() -> Self { Self::new() }
}

impl AtomicGreeksAggregator {
    /// Creates a new atomic aggregator with zeroed values.
    pub fn new() -> Self {
        Self {
            delta_bits: AtomicUsize::new(0),
            gamma_bits: AtomicUsize::new(0),
            vega_bits: AtomicUsize::new(0),
            theta_bits: AtomicUsize::new(0),
            rho_bits: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
        }
    }

    /// Adds delta contribution (using compare-and-swap).
    pub fn add_delta(&self, value: f64) {
        self.atomic_add(&self.delta_bits, value);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Adds gamma contribution.
    pub fn add_gamma(&self, value: f64) { self.atomic_add(&self.gamma_bits, value); }

    /// Adds vega contribution.
    pub fn add_vega(&self, value: f64) { self.atomic_add(&self.vega_bits, value); }

    /// Adds all Greeks from an accumulator.
    pub fn add_from_accumulator(&self, acc: &AdjointAccumulator<f64>) {
        self.atomic_add(&self.delta_bits, acc.delta());
        self.atomic_add(&self.gamma_bits, acc.gamma());
        self.atomic_add(&self.vega_bits, acc.vega());
        self.atomic_add(&self.theta_bits, acc.theta());
        self.atomic_add(&self.rho_bits, acc.rho());
        self.count.fetch_add(acc.count(), Ordering::Relaxed);
    }

    /// Returns the accumulated delta.
    pub fn delta(&self) -> f64 { f64::from_bits(self.delta_bits.load(Ordering::Relaxed) as u64) }

    /// Returns the accumulated gamma.
    pub fn gamma(&self) -> f64 { f64::from_bits(self.gamma_bits.load(Ordering::Relaxed) as u64) }

    /// Returns the accumulated vega.
    pub fn vega(&self) -> f64 { f64::from_bits(self.vega_bits.load(Ordering::Relaxed) as u64) }

    /// Returns the count of contributions.
    pub fn count(&self) -> usize { self.count.load(Ordering::Relaxed) }

    /// Returns averaged Greeks.
    pub fn averaged(&self) -> (f64, f64, f64, f64, f64) {
        let count = self.count() as f64;
        if count == 0.0 {
            return (0.0, 0.0, 0.0, 0.0, 0.0);
        }

        (
            self.delta() / count,
            self.gamma() / count,
            self.vega() / count,
            f64::from_bits(self.theta_bits.load(Ordering::Relaxed) as u64) / count,
            f64::from_bits(self.rho_bits.load(Ordering::Relaxed) as u64) / count,
        )
    }

    /// Resets all values to zero.
    pub fn reset(&self) {
        self.delta_bits.store(0, Ordering::Relaxed);
        self.gamma_bits.store(0, Ordering::Relaxed);
        self.vega_bits.store(0, Ordering::Relaxed);
        self.theta_bits.store(0, Ordering::Relaxed);
        self.rho_bits.store(0, Ordering::Relaxed);
        self.count.store(0, Ordering::Relaxed);
    }

    #[inline]
    fn atomic_add(&self, atomic: &AtomicUsize, value: f64) {
        let mut current = atomic.load(Ordering::Relaxed);
        loop {
            let current_f64 = f64::from_bits(current as u64);
            let new_f64 = current_f64 + value;
            let new = new_f64.to_bits() as usize;

            match atomic.compare_exchange_weak(current, new, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(x) => current = x,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_greeks_config_default() {
        let config = ParallelGreeksConfig::default();
        assert_eq!(config.min_paths_per_thread, 100);
        assert!(config.use_thread_local);
        assert_eq!(config.chunk_size, 0);
    }

    #[test]
    fn test_parallel_greeks_config_should_parallelise() {
        let config = ParallelGreeksConfig::with_min_paths(100);

        let n_threads = rayon::current_num_threads();
        let threshold = 100 * n_threads;

        assert!(!config.should_parallelise(threshold - 1));
        assert!(config.should_parallelise(threshold));
    }

    #[test]
    fn test_parallel_greeks_computer_sequential() {
        let computer = ParallelGreeksComputer::default_config();

        let result = computer.compute_sequential(100, |_| {
            let mut acc = AdjointAccumulator::new();
            acc.add_delta(1.0);
            acc
        });

        assert_eq!(result.count(), 100);
        assert!((result.delta() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_parallel_greeks_computer_parallel() {
        let computer = ParallelGreeksComputer::new(ParallelGreeksConfig::with_min_paths(10));

        let result = computer.compute_parallel(1000, |path_idx| {
            let mut acc = AdjointAccumulator::new();
            acc.add_delta(path_idx as f64);
            acc
        });

        let expected_sum = (999 * 1000) / 2;
        assert_eq!(result.count(), 1000);
        assert!((result.delta() - expected_sum as f64).abs() < 1e-6);
    }

    #[test]
    fn test_parallel_greeks_computer_chunked() {
        let config = ParallelGreeksConfig::default().with_chunk_size(100);
        let computer = ParallelGreeksComputer::new(config);

        let result = computer.compute_parallel_chunked(1000, |_| {
            let mut acc = AdjointAccumulator::new();
            acc.add_delta(1.0);
            acc.add_gamma(0.1);
            acc
        });

        assert_eq!(result.count(), 1000);
        assert!((result.delta() - 1000.0).abs() < 1e-10);
        assert!((result.gamma() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_parallel_greeks_result_from_accumulator() {
        let mut acc = AdjointAccumulator::new();
        for _ in 0..100 {
            acc.add_delta(1.0);
            acc.add_gamma(0.1);
        }

        let result = ParallelGreeksResult::from_accumulator(&acc);

        assert_eq!(result.n_paths, 100);
        assert!((result.delta - 1.0).abs() < 1e-10);
        assert!((result.gamma - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_atomic_greeks_aggregator_basic() {
        let agg = AtomicGreeksAggregator::new();

        agg.add_delta(1.0);
        agg.add_delta(2.0);
        agg.add_gamma(0.5);

        assert_eq!(agg.count(), 2);
        assert!((agg.delta() - 3.0).abs() < 1e-10);
        assert!((agg.gamma() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_atomic_greeks_aggregator_parallel() {
        let agg = AtomicGreeksAggregator::new();

        (0..1000_usize).into_par_iter().for_each(|_| {
            agg.add_delta(1.0);
        });

        assert_eq!(agg.count(), 1000);
        assert!((agg.delta() - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn test_atomic_greeks_aggregator_averaged() {
        let agg = AtomicGreeksAggregator::new();

        for i in 0..100 {
            agg.add_delta(i as f64);
        }

        let (avg_delta, _, _, _, _) = agg.averaged();

        assert!((avg_delta - 49.5).abs() < 1e-10);
    }

    #[test]
    fn test_atomic_greeks_aggregator_reset() {
        let agg = AtomicGreeksAggregator::new();

        agg.add_delta(100.0);
        assert_eq!(agg.count(), 1);

        agg.reset();
        assert_eq!(agg.count(), 0);
        assert_eq!(agg.delta(), 0.0);
    }

    #[test]
    fn test_atomic_greeks_aggregator_from_accumulator() {
        let agg = AtomicGreeksAggregator::new();

        let mut acc = AdjointAccumulator::new();
        acc.add_delta(5.0);
        acc.add_gamma(1.0);
        acc.add_vega(10.0);

        agg.add_from_accumulator(&acc);

        assert_eq!(agg.count(), 1);
        assert!((agg.delta() - 5.0).abs() < 1e-10);
        assert!((agg.gamma() - 1.0).abs() < 1e-10);
        assert!((agg.vega() - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_paths_processed_tracking() {
        let computer = ParallelGreeksComputer::default_config();

        assert_eq!(computer.paths_processed(), 0);

        computer.compute_sequential(50, |_| AdjointAccumulator::new());

        assert_eq!(computer.paths_processed(), 50);

        computer.reset();
        assert_eq!(computer.paths_processed(), 0);
    }
}
