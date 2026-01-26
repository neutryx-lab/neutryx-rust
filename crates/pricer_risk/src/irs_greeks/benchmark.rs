//! Benchmark utilities for IRS Greeks calculation.
//!
//! Provides tools for measuring and comparing AAD vs Bump-and-Revalue
//! performance.

use thiserror::Error;

/// Benchmark error types.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum BenchmarkError {
    /// Invalid configuration.
    #[error("Invalid benchmark configuration: {0}")]
    InvalidConfig(String),
    /// Benchmark execution failed.
    #[error("Benchmark failed: {0}")]
    ExecutionFailed(String),
}

/// Benchmark configuration.
#[derive(Clone, Debug)]
pub struct BenchmarkConfig {
    /// Number of iterations for timing measurements.
    pub iterations: usize,
    /// Number of warmup iterations.
    pub warmup_iterations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 100,
            warmup_iterations: 10,
        }
    }
}

impl BenchmarkConfig {
    /// Create a new benchmark configuration.
    pub fn new() -> Self { Self::default() }

    /// Set the number of iterations.
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Set the number of warmup iterations.
    pub fn with_warmup_iterations(mut self, warmup: usize) -> Self {
        self.warmup_iterations = warmup;
        self
    }
}

/// Benchmark runner for IRS Greeks calculations.
#[derive(Clone, Debug)]
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner.
    pub fn new(config: BenchmarkConfig) -> Self { Self { config } }

    /// Get the configuration.
    pub fn config(&self) -> &BenchmarkConfig { &self.config }
}

impl Default for BenchmarkRunner {
    fn default() -> Self { Self::new(BenchmarkConfig::default()) }
}

/// Timing statistics for benchmark results.
#[derive(Clone, Debug, Default)]
pub struct TimingStats {
    /// Mean time in nanoseconds.
    pub mean_ns: f64,
    /// Standard deviation in nanoseconds.
    pub std_dev_ns: f64,
    /// Minimum time in nanoseconds.
    pub min_ns: u64,
    /// Maximum time in nanoseconds.
    pub max_ns: u64,
}

/// PV benchmark result.
#[derive(Clone, Debug, Default)]
pub struct PvBenchmarkResult {
    /// NPV value.
    pub npv: f64,
    /// Timing statistics.
    pub timing: TimingStats,
}

/// Single Delta benchmark result.
#[derive(Clone, Debug, Default)]
pub struct SingleDeltaBenchmarkResult {
    /// Delta value.
    pub delta: f64,
    /// Timing statistics.
    pub timing: TimingStats,
}

/// Delta benchmark result comparing AAD vs Bump.
#[derive(Clone, Debug, Default)]
pub struct DeltaBenchmarkResult {
    /// AAD timing statistics.
    pub aad_timing: TimingStats,
    /// Bump timing statistics.
    pub bump_timing: TimingStats,
    /// Speedup ratio (bump_time / aad_time).
    pub speedup_ratio: f64,
}

/// Full benchmark result.
#[derive(Clone, Debug, Default)]
pub struct FullBenchmarkResult {
    /// PV benchmark result.
    pub pv: PvBenchmarkResult,
    /// Single Delta benchmark result.
    pub single_delta: SingleDeltaBenchmarkResult,
    /// Delta benchmark result.
    pub deltas: DeltaBenchmarkResult,
}

/// Scalability benchmark result.
#[derive(Clone, Debug, Default)]
pub struct ScalabilityResult {
    /// Number of tenor points.
    pub tenor_counts: Vec<usize>,
    /// AAD times for each tenor count.
    pub aad_times_ns: Vec<f64>,
    /// Bump times for each tenor count.
    pub bump_times_ns: Vec<f64>,
}

/// Swap parameters for benchmark generation.
#[derive(Clone, Debug)]
pub struct SwapParams {
    /// Notional amount.
    pub notional: f64,
    /// Fixed rate.
    pub fixed_rate: f64,
    /// Maturity in years.
    pub maturity_years: f64,
}

impl Default for SwapParams {
    fn default() -> Self {
        Self {
            notional: 1_000_000.0,
            fixed_rate: 0.03,
            maturity_years: 5.0,
        }
    }
}
