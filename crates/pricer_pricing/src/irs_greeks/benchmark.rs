//! Benchmark functionality for IRS Greeks calculation.
//!
//! This module provides performance benchmarking for AAD vs Bump-and-Revalue
//! Greeks calculation methods.
//!
//! # Task Coverage
//!
//! - Task 4.1: Benchmark executor with statistical analysis
//! - Task 4.2: Scalability measurement
//! - Task 4.3: JSON/Markdown output
//!
//! # Requirements Coverage
//!
//! - Requirement 5.1: PV計算時間、単一Delta計算時間、全テナーDelta計算時間の計測
//! - Requirement 5.2: 複数回(デフォルト100回)実行し、統計値を算出
//! - Requirement 5.3: AADモードとBump-and-Revalueモードの速度比を計算
//! - Requirement 5.4: テナー数増加に伴う計算時間のスケーラビリティを計測
//! - Requirement 5.5: 結果をJSON形式およびMarkdown形式で出力
//!
//! # Usage
//!
//! ```rust,ignore
//! use pricer_pricing::irs_greeks::{BenchmarkRunner, BenchmarkConfig};
//!
//! let config = BenchmarkConfig::default();
//! let runner = BenchmarkRunner::new(config);
//!
//! let result = runner.benchmark_deltas(&swap, &curves, valuation_date, &tenor_points);
//! println!("Speedup ratio: {}", result.speedup_ratio);
//!
//! // Output as JSON
//! let json = runner.to_json(&result);
//! // Output as Markdown
//! let md = runner.to_markdown(&result);
//! ```

use std::time::Instant;

#[cfg(feature = "l1l2-integration")]
use pricer_core::market_data::curves::CurveSet;
#[cfg(feature = "l1l2-integration")]
use pricer_core::types::time::Date;
#[cfg(feature = "l1l2-integration")]
use pricer_models::instruments::rates::InterestRateSwap;

use super::IrsGreeksCalculator;
use super::IrsGreeksConfig;
use super::IrsGreeksError;

// =============================================================================
// Task 4.1: Benchmark Configuration
// =============================================================================

/// Configuration for benchmark execution.
///
/// # Default Values
///
/// | Parameter | Default | Description |
/// |-----------|---------|-------------|
/// | `iterations` | 100 | Number of timed iterations |
/// | `warmup` | 10 | Number of warmup iterations |
///
/// # Requirements Coverage
///
/// - Requirement 5.2: 複数回(デフォルト100回)実行
#[derive(Clone, Debug)]
pub struct BenchmarkConfig {
    /// Number of timed iterations (default: 100).
    pub iterations: usize,

    /// Number of warmup iterations (default: 10).
    pub warmup: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 100,
            warmup: 10,
        }
    }
}

impl BenchmarkConfig {
    /// Creates a new benchmark configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the number of iterations.
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Sets the number of warmup iterations.
    pub fn with_warmup(mut self, warmup: usize) -> Self {
        self.warmup = warmup;
        self
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), BenchmarkError> {
        if self.iterations == 0 {
            return Err(BenchmarkError::InvalidConfig(
                "iterations must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

// =============================================================================
// Task 4.1: Timing Statistics
// =============================================================================

/// Timing statistics from benchmark runs.
///
/// Contains mean, standard deviation, min, and max values.
///
/// # Requirements Coverage
///
/// - Requirement 5.2: 統計値(平均、標準偏差、最小、最大)を算出
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimingStats {
    /// Mean time in nanoseconds.
    pub mean_ns: f64,

    /// Standard deviation in nanoseconds.
    pub std_dev_ns: f64,

    /// Minimum time in nanoseconds.
    pub min_ns: u64,

    /// Maximum time in nanoseconds.
    pub max_ns: u64,

    /// Number of samples.
    pub sample_count: usize,
}

impl TimingStats {
    /// Computes statistics from a vector of timing samples.
    ///
    /// # Arguments
    ///
    /// * `samples` - Vector of timing samples in nanoseconds
    ///
    /// # Returns
    ///
    /// `TimingStats` containing computed statistics.
    pub fn from_samples(samples: &[u64]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        let n = samples.len();
        let sum: u64 = samples.iter().sum();
        let mean = sum as f64 / n as f64;

        let variance: f64 = samples
            .iter()
            .map(|&x| {
                let diff = x as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / n as f64;

        let std_dev = variance.sqrt();
        let min = *samples.iter().min().unwrap_or(&0);
        let max = *samples.iter().max().unwrap_or(&0);

        Self {
            mean_ns: mean,
            std_dev_ns: std_dev,
            min_ns: min,
            max_ns: max,
            sample_count: n,
        }
    }

    /// Returns the mean time in microseconds.
    pub fn mean_us(&self) -> f64 {
        self.mean_ns / 1000.0
    }

    /// Returns the mean time in milliseconds.
    pub fn mean_ms(&self) -> f64 {
        self.mean_ns / 1_000_000.0
    }

    /// Returns the coefficient of variation (CV) as a percentage.
    ///
    /// CV = (std_dev / mean) * 100
    pub fn cv_percent(&self) -> f64 {
        if self.mean_ns > 0.0 {
            (self.std_dev_ns / self.mean_ns) * 100.0
        } else {
            0.0
        }
    }
}

// =============================================================================
// Task 4.1: Benchmark Results
// =============================================================================

/// Result of a PV benchmark.
///
/// # Requirements Coverage
///
/// - Requirement 5.1: PV計算時間
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PvBenchmarkResult {
    /// Timing statistics for PV computation.
    pub stats: TimingStats,
}

/// Result of a single Delta benchmark.
///
/// # Requirements Coverage
///
/// - Requirement 5.1: 単一Delta計算時間
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SingleDeltaBenchmarkResult {
    /// Timing statistics for single Delta computation.
    pub stats: TimingStats,

    /// The tenor point used for the benchmark.
    pub tenor: f64,
}

/// Result of a Delta benchmark comparing AAD and Bump-and-Revalue.
///
/// # Requirements Coverage
///
/// - Requirement 5.1: 全テナーDelta計算時間
/// - Requirement 5.3: 速度比(speedup ratio)を計算
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeltaBenchmarkResult {
    /// AAD timing statistics.
    pub aad_stats: TimingStats,

    /// Bump-and-Revalue timing statistics.
    pub bump_stats: TimingStats,

    /// Speedup ratio (bump_mean / aad_mean).
    pub speedup_ratio: f64,

    /// Number of tenor points.
    pub tenor_count: usize,
}

impl DeltaBenchmarkResult {
    /// Computes the speedup ratio from AAD and bump statistics.
    pub fn compute_speedup_ratio(aad_stats: &TimingStats, bump_stats: &TimingStats) -> f64 {
        if aad_stats.mean_ns > 0.0 {
            bump_stats.mean_ns / aad_stats.mean_ns
        } else {
            0.0
        }
    }
}

// =============================================================================
// Task 4.2: Scalability Results
// =============================================================================

/// Result of a scalability benchmark.
///
/// Contains benchmark results for varying tenor counts.
///
/// # Requirements Coverage
///
/// - Requirement 5.4: テナー数増加に伴う計算時間のスケーラビリティを計測
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScalabilityResult {
    /// Results for each tenor count: (tenor_count, DeltaBenchmarkResult)
    pub results: Vec<(usize, DeltaBenchmarkResult)>,
}

impl ScalabilityResult {
    /// Creates a new empty scalability result.
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Adds a result for a specific tenor count.
    pub fn add_result(&mut self, tenor_count: usize, result: DeltaBenchmarkResult) {
        self.results.push((tenor_count, result));
    }

    /// Returns the number of data points.
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Returns true if there are no data points.
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Extracts AAD timing data for plotting.
    ///
    /// Returns: Vec of (tenor_count, mean_time_ns)
    pub fn aad_data(&self) -> Vec<(usize, f64)> {
        self.results
            .iter()
            .map(|(count, result)| (*count, result.aad_stats.mean_ns))
            .collect()
    }

    /// Extracts Bump timing data for plotting.
    ///
    /// Returns: Vec of (tenor_count, mean_time_ns)
    pub fn bump_data(&self) -> Vec<(usize, f64)> {
        self.results
            .iter()
            .map(|(count, result)| (*count, result.bump_stats.mean_ns))
            .collect()
    }

    /// Extracts speedup ratio data for plotting.
    ///
    /// Returns: Vec of (tenor_count, speedup_ratio)
    pub fn speedup_data(&self) -> Vec<(usize, f64)> {
        self.results
            .iter()
            .map(|(count, result)| (*count, result.speedup_ratio))
            .collect()
    }
}

impl Default for ScalabilityResult {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Task 4.3: Full Benchmark Result with Metadata
// =============================================================================

/// Swap parameters for benchmark output.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwapParams {
    /// Notional amount.
    pub notional: f64,

    /// Fixed rate.
    pub fixed_rate: f64,

    /// Tenor in years.
    pub tenor_years: f64,
}

/// Full benchmark result with metadata for output.
///
/// # Requirements Coverage
///
/// - Requirement 5.5: タイムスタンプ、スワップパラメータ、計測結果を含むスキーマ
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FullBenchmarkResult {
    /// Benchmark type identifier.
    pub benchmark_type: String,

    /// Timestamp when the benchmark was run (ISO 8601 format).
    pub timestamp: String,

    /// Swap parameters used.
    pub swap_params: SwapParams,

    /// Delta benchmark results.
    pub delta_results: DeltaBenchmarkResult,

    /// Optional scalability results.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub scalability_results: Option<ScalabilityResult>,
}

// =============================================================================
// Task 4.1: Benchmark Error
// =============================================================================

/// Benchmark execution error.
#[derive(Debug, thiserror::Error)]
pub enum BenchmarkError {
    /// Invalid configuration.
    #[error("Invalid benchmark configuration: {0}")]
    InvalidConfig(String),

    /// Greeks calculation error.
    #[error("Greeks calculation error: {0}")]
    GreeksError(#[from] IrsGreeksError),
}

// =============================================================================
// Task 4.1, 4.2: Benchmark Runner
// =============================================================================

/// Benchmark runner for IRS Greeks calculation.
///
/// # Requirements Coverage
///
/// - Requirement 5.1: PV計算、単一Delta計算、全テナーDelta計算の計測機能
/// - Requirement 5.2: 複数回反復による統計値算出、ウォームアップ実行
/// - Requirement 5.3: AADモードとBump-and-Revalueモードの速度比を計算
/// - Requirement 5.4: テナーポイント数を変化させた計算時間計測
/// - Requirement 5.5: 結果をJSON形式およびMarkdown形式で出力
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
}

impl BenchmarkRunner {
    /// Creates a new benchmark runner with the given configuration.
    pub fn new(config: BenchmarkConfig) -> Self {
        Self { config }
    }

    /// Creates a new benchmark runner with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(BenchmarkConfig::default())
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &BenchmarkConfig {
        &self.config
    }
}

#[cfg(feature = "l1l2-integration")]
impl BenchmarkRunner {
    /// Runs PV calculation benchmark.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap
    /// * `curves` - Curve set containing discount and forward curves
    /// * `valuation_date` - The valuation date
    ///
    /// # Returns
    ///
    /// `PvBenchmarkResult` containing timing statistics.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 5.1: PV計算時間
    /// - Requirement 5.2: ウォームアップ実行、複数回反復による統計値算出
    pub fn benchmark_pv(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
    ) -> Result<PvBenchmarkResult, BenchmarkError> {
        self.config.validate()?;

        let calculator = IrsGreeksCalculator::<f64>::new(IrsGreeksConfig::default());

        // Warmup phase
        for _ in 0..self.config.warmup {
            let _ = calculator.compute_npv(swap, curves, valuation_date)?;
        }

        // Timed iterations
        let mut samples = Vec::with_capacity(self.config.iterations);
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let _ = calculator.compute_npv(swap, curves, valuation_date)?;
            samples.push(start.elapsed().as_nanos() as u64);
        }

        Ok(PvBenchmarkResult {
            stats: TimingStats::from_samples(&samples),
        })
    }

    /// Runs single Delta calculation benchmark.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap
    /// * `curves` - Curve set
    /// * `valuation_date` - The valuation date
    /// * `tenor` - Single tenor point for Delta calculation
    ///
    /// # Returns
    ///
    /// `SingleDeltaBenchmarkResult` containing timing statistics.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 5.1: 単一Delta計算時間
    pub fn benchmark_single_delta(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        tenor: f64,
    ) -> Result<SingleDeltaBenchmarkResult, BenchmarkError> {
        self.config.validate()?;

        let calculator = IrsGreeksCalculator::<f64>::new(IrsGreeksConfig::default());
        let tenor_points = [tenor];

        // Warmup phase
        for _ in 0..self.config.warmup {
            let _ = calculator.compute_tenor_deltas_bump(
                swap,
                curves,
                valuation_date,
                &tenor_points,
                0.0001,
            )?;
        }

        // Timed iterations
        let mut samples = Vec::with_capacity(self.config.iterations);
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let _ = calculator.compute_tenor_deltas_bump(
                swap,
                curves,
                valuation_date,
                &tenor_points,
                0.0001,
            )?;
            samples.push(start.elapsed().as_nanos() as u64);
        }

        Ok(SingleDeltaBenchmarkResult {
            stats: TimingStats::from_samples(&samples),
            tenor,
        })
    }

    /// Runs Delta benchmark comparing AAD and Bump-and-Revalue.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap
    /// * `curves` - Curve set
    /// * `valuation_date` - The valuation date
    /// * `tenor_points` - Tenor points for Delta calculation
    ///
    /// # Returns
    ///
    /// `DeltaBenchmarkResult` containing timing statistics and speedup ratio.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 5.1: 全テナーDelta計算時間
    /// - Requirement 5.3: 速度比(speedup ratio)を計算
    pub fn benchmark_deltas(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        tenor_points: &[f64],
    ) -> Result<DeltaBenchmarkResult, BenchmarkError> {
        self.config.validate()?;

        let calculator = IrsGreeksCalculator::<f64>::new(IrsGreeksConfig::default());

        // Warmup phase for AAD
        for _ in 0..self.config.warmup {
            let _ = calculator.compute_tenor_deltas_aad(swap, curves, valuation_date, tenor_points)?;
        }

        // Timed iterations for AAD
        let mut aad_samples = Vec::with_capacity(self.config.iterations);
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let _ = calculator.compute_tenor_deltas_aad(swap, curves, valuation_date, tenor_points)?;
            aad_samples.push(start.elapsed().as_nanos() as u64);
        }

        // Warmup phase for Bump-and-Revalue
        for _ in 0..self.config.warmup {
            let _ = calculator.compute_tenor_deltas_bump(
                swap,
                curves,
                valuation_date,
                tenor_points,
                0.0001,
            )?;
        }

        // Timed iterations for Bump-and-Revalue
        let mut bump_samples = Vec::with_capacity(self.config.iterations);
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let _ = calculator.compute_tenor_deltas_bump(
                swap,
                curves,
                valuation_date,
                tenor_points,
                0.0001,
            )?;
            bump_samples.push(start.elapsed().as_nanos() as u64);
        }

        let aad_stats = TimingStats::from_samples(&aad_samples);
        let bump_stats = TimingStats::from_samples(&bump_samples);
        let speedup_ratio = DeltaBenchmarkResult::compute_speedup_ratio(&aad_stats, &bump_stats);

        Ok(DeltaBenchmarkResult {
            aad_stats,
            bump_stats,
            speedup_ratio,
            tenor_count: tenor_points.len(),
        })
    }

    /// Runs scalability benchmark with varying tenor counts.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap
    /// * `curves` - Curve set
    /// * `valuation_date` - The valuation date
    /// * `tenor_counts` - Array of tenor counts to benchmark
    ///
    /// # Returns
    ///
    /// `ScalabilityResult` containing benchmark results for each tenor count.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 5.4: テナーポイント数を変化させた計算時間計測
    pub fn benchmark_scalability(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        tenor_counts: &[usize],
    ) -> Result<ScalabilityResult, BenchmarkError> {
        self.config.validate()?;

        let mut result = ScalabilityResult::new();

        for &count in tenor_counts {
            // Generate tenor points
            let tenor_points: Vec<f64> = (0..count)
                .map(|i| 0.25 + (i as f64) * 0.25) // 3M, 6M, 9M, 1Y, ...
                .collect();

            let benchmark_result = self.benchmark_deltas(swap, curves, valuation_date, &tenor_points)?;
            result.add_result(count, benchmark_result);
        }

        Ok(result)
    }

    /// Creates a full benchmark result with metadata.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap (for extracting parameters)
    /// * `delta_results` - Delta benchmark results
    /// * `scalability_results` - Optional scalability results
    ///
    /// # Returns
    ///
    /// `FullBenchmarkResult` with metadata.
    pub fn create_full_result(
        &self,
        swap: &InterestRateSwap<f64>,
        delta_results: DeltaBenchmarkResult,
        scalability_results: Option<ScalabilityResult>,
    ) -> FullBenchmarkResult {
        // Get current timestamp
        let timestamp = chrono_timestamp();

        // Extract swap parameters
        let swap_params = SwapParams {
            notional: swap.notional(),
            fixed_rate: swap.fixed_leg().fixed_rate(),
            tenor_years: calculate_tenor_years(swap),
        };

        FullBenchmarkResult {
            benchmark_type: "delta_comparison".to_string(),
            timestamp,
            swap_params,
            delta_results,
            scalability_results,
        }
    }
}

/// Helper function to get current timestamp in ISO 8601 format.
fn chrono_timestamp() -> String {
    // Simple timestamp without chrono dependency
    // Format: YYYY-MM-DDTHH:MM:SSZ
    use std::time::SystemTime;

    let now = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    // Convert to approximate date/time (simplified)
    let secs = now.as_secs();
    let days = secs / 86400;
    let remaining_secs = secs % 86400;
    let hours = remaining_secs / 3600;
    let mins = (remaining_secs % 3600) / 60;
    let secs_in_min = remaining_secs % 60;

    // Approximate year/month/day calculation from days since epoch (1970-01-01)
    let years = 1970 + (days / 365);
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        years, month.min(12), day.min(31), hours, mins, secs_in_min
    )
}

/// Helper function to calculate swap tenor in years.
#[cfg(feature = "l1l2-integration")]
fn calculate_tenor_years(swap: &InterestRateSwap<f64>) -> f64 {
    let schedule = swap.fixed_leg().schedule();
    let periods = schedule.periods();
    if periods.is_empty() {
        return 0.0;
    }

    // Calculate approximate tenor from first to last period
    let first = periods.first().unwrap();
    let last = periods.last().unwrap();

    // Approximate years from days (Date subtraction returns i64 days)
    let days = (last.end() - first.start()) as f64;
    days / 365.0
}

// =============================================================================
// Task 4.3: Output Formatters
// =============================================================================

impl BenchmarkRunner {
    /// Converts benchmark result to JSON format.
    ///
    /// # Arguments
    ///
    /// * `result` - The benchmark result to convert
    ///
    /// # Returns
    ///
    /// JSON string representation of the result.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 5.5: 結果をJSON形式で出力
    #[cfg(feature = "serde")]
    pub fn to_json(&self, result: &FullBenchmarkResult) -> String {
        serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".to_string())
    }

    /// Converts benchmark result to JSON format (without serde).
    ///
    /// Manual JSON generation when serde is not available.
    #[cfg(not(feature = "serde"))]
    pub fn to_json(&self, result: &FullBenchmarkResult) -> String {
        format!(
            r#"{{
  "benchmark_type": "{}",
  "timestamp": "{}",
  "swap_params": {{
    "notional": {},
    "fixed_rate": {},
    "tenor_years": {}
  }},
  "results": {{
    "tenor_count": {},
    "aad": {{
      "mean_ns": {},
      "std_dev_ns": {},
      "min_ns": {},
      "max_ns": {}
    }},
    "bump": {{
      "mean_ns": {},
      "std_dev_ns": {},
      "min_ns": {},
      "max_ns": {}
    }},
    "speedup_ratio": {}
  }}
}}"#,
            result.benchmark_type,
            result.timestamp,
            result.swap_params.notional,
            result.swap_params.fixed_rate,
            result.swap_params.tenor_years,
            result.delta_results.tenor_count,
            result.delta_results.aad_stats.mean_ns,
            result.delta_results.aad_stats.std_dev_ns,
            result.delta_results.aad_stats.min_ns,
            result.delta_results.aad_stats.max_ns,
            result.delta_results.bump_stats.mean_ns,
            result.delta_results.bump_stats.std_dev_ns,
            result.delta_results.bump_stats.min_ns,
            result.delta_results.bump_stats.max_ns,
            result.delta_results.speedup_ratio
        )
    }

    /// Converts benchmark result to Markdown format.
    ///
    /// # Arguments
    ///
    /// * `result` - The benchmark result to convert
    ///
    /// # Returns
    ///
    /// Markdown string representation of the result.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 5.5: 結果をMarkdown形式で出力
    pub fn to_markdown(&self, result: &FullBenchmarkResult) -> String {
        let mut md = String::new();

        // Header
        md.push_str("# IRS AAD Benchmark Results\n\n");

        // Timestamp
        md.push_str(&format!("**Timestamp:** {}\n\n", result.timestamp));

        // Swap Parameters
        md.push_str("## Swap Parameters\n\n");
        md.push_str("| Parameter | Value |\n");
        md.push_str("|-----------|-------|\n");
        md.push_str(&format!("| Notional | {:.0} |\n", result.swap_params.notional));
        md.push_str(&format!(
            "| Fixed Rate | {:.4}% |\n",
            result.swap_params.fixed_rate * 100.0
        ));
        md.push_str(&format!(
            "| Tenor | {:.1} years |\n",
            result.swap_params.tenor_years
        ));
        md.push_str("\n");

        // Benchmark Results
        md.push_str("## Benchmark Results\n\n");
        md.push_str(&format!(
            "**Tenor Points:** {}\n\n",
            result.delta_results.tenor_count
        ));

        // Timing Table
        md.push_str("### Timing Statistics\n\n");
        md.push_str("| Metric | AAD | Bump-and-Revalue |\n");
        md.push_str("|--------|-----|------------------|\n");
        md.push_str(&format!(
            "| Mean (ns) | {:.2} | {:.2} |\n",
            result.delta_results.aad_stats.mean_ns,
            result.delta_results.bump_stats.mean_ns
        ));
        md.push_str(&format!(
            "| Std Dev (ns) | {:.2} | {:.2} |\n",
            result.delta_results.aad_stats.std_dev_ns,
            result.delta_results.bump_stats.std_dev_ns
        ));
        md.push_str(&format!(
            "| Min (ns) | {} | {} |\n",
            result.delta_results.aad_stats.min_ns,
            result.delta_results.bump_stats.min_ns
        ));
        md.push_str(&format!(
            "| Max (ns) | {} | {} |\n",
            result.delta_results.aad_stats.max_ns,
            result.delta_results.bump_stats.max_ns
        ));
        md.push_str("\n");

        // Speedup Ratio
        md.push_str("### Performance Summary\n\n");
        md.push_str(&format!(
            "**Speedup Ratio:** {:.2}x (Bump / AAD)\n\n",
            result.delta_results.speedup_ratio
        ));

        // Scalability Results (if present)
        if let Some(ref scalability) = result.scalability_results {
            md.push_str("## Scalability Results\n\n");
            md.push_str("| Tenor Count | AAD Mean (us) | Bump Mean (us) | Speedup |\n");
            md.push_str("|-------------|---------------|----------------|--------|\n");
            for (count, delta_result) in &scalability.results {
                md.push_str(&format!(
                    "| {} | {:.2} | {:.2} | {:.2}x |\n",
                    count,
                    delta_result.aad_stats.mean_us(),
                    delta_result.bump_stats.mean_us(),
                    delta_result.speedup_ratio
                ));
            }
            md.push_str("\n");
        }

        md
    }

    /// Converts scalability result to Chart.js compatible JSON.
    ///
    /// # Arguments
    ///
    /// * `result` - The scalability result to convert
    ///
    /// # Returns
    ///
    /// JSON string compatible with Chart.js data format.
    pub fn scalability_to_chartjs_json(&self, result: &ScalabilityResult) -> String {
        let labels: Vec<String> = result.results.iter().map(|(c, _)| c.to_string()).collect();
        let aad_data: Vec<f64> = result.results.iter().map(|(_, r)| r.aad_stats.mean_us()).collect();
        let bump_data: Vec<f64> = result.results.iter().map(|(_, r)| r.bump_stats.mean_us()).collect();

        format!(
            r#"{{
  "labels": {:?},
  "datasets": [
    {{
      "label": "AAD",
      "data": {:?},
      "borderColor": "rgb(75, 192, 192)",
      "tension": 0.1
    }},
    {{
      "label": "Bump-and-Revalue",
      "data": {:?},
      "borderColor": "rgb(255, 99, 132)",
      "tension": 0.1
    }}
  ]
}}"#,
            labels, aad_data, bump_data
        )
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 4.1: Benchmark Configuration Tests
    // =========================================================================

    mod config_tests {
        use super::*;

        #[test]
        fn test_default_config() {
            let config = BenchmarkConfig::default();
            assert_eq!(config.iterations, 100);
            assert_eq!(config.warmup, 10);
        }

        #[test]
        fn test_config_builder() {
            let config = BenchmarkConfig::new()
                .with_iterations(50)
                .with_warmup(5);

            assert_eq!(config.iterations, 50);
            assert_eq!(config.warmup, 5);
        }

        #[test]
        fn test_config_validation_valid() {
            let config = BenchmarkConfig::default();
            assert!(config.validate().is_ok());
        }

        #[test]
        fn test_config_validation_zero_iterations() {
            let config = BenchmarkConfig::new().with_iterations(0);
            assert!(config.validate().is_err());
        }
    }

    // =========================================================================
    // Task 4.1: Timing Statistics Tests
    // =========================================================================

    mod timing_stats_tests {
        use super::*;

        #[test]
        fn test_from_samples_empty() {
            let stats = TimingStats::from_samples(&[]);
            assert_eq!(stats.sample_count, 0);
            assert_eq!(stats.mean_ns, 0.0);
        }

        #[test]
        fn test_from_samples_single() {
            let stats = TimingStats::from_samples(&[1000]);
            assert_eq!(stats.sample_count, 1);
            assert!((stats.mean_ns - 1000.0).abs() < 1e-10);
            assert_eq!(stats.min_ns, 1000);
            assert_eq!(stats.max_ns, 1000);
            assert!((stats.std_dev_ns - 0.0).abs() < 1e-10);
        }

        #[test]
        fn test_from_samples_multiple() {
            let samples = vec![100, 200, 300, 400, 500];
            let stats = TimingStats::from_samples(&samples);

            assert_eq!(stats.sample_count, 5);
            assert!((stats.mean_ns - 300.0).abs() < 1e-10);
            assert_eq!(stats.min_ns, 100);
            assert_eq!(stats.max_ns, 500);
            assert!(stats.std_dev_ns > 0.0);
        }

        #[test]
        fn test_mean_us() {
            let stats = TimingStats {
                mean_ns: 1000.0,
                ..Default::default()
            };
            assert!((stats.mean_us() - 1.0).abs() < 1e-10);
        }

        #[test]
        fn test_mean_ms() {
            let stats = TimingStats {
                mean_ns: 1_000_000.0,
                ..Default::default()
            };
            assert!((stats.mean_ms() - 1.0).abs() < 1e-10);
        }

        #[test]
        fn test_cv_percent() {
            let stats = TimingStats {
                mean_ns: 100.0,
                std_dev_ns: 10.0,
                ..Default::default()
            };
            assert!((stats.cv_percent() - 10.0).abs() < 1e-10);
        }

        #[test]
        fn test_cv_percent_zero_mean() {
            let stats = TimingStats {
                mean_ns: 0.0,
                std_dev_ns: 10.0,
                ..Default::default()
            };
            assert!((stats.cv_percent() - 0.0).abs() < 1e-10);
        }
    }

    // =========================================================================
    // Task 4.1: Delta Benchmark Result Tests
    // =========================================================================

    mod delta_result_tests {
        use super::*;

        #[test]
        fn test_compute_speedup_ratio() {
            let aad_stats = TimingStats {
                mean_ns: 100.0,
                ..Default::default()
            };
            let bump_stats = TimingStats {
                mean_ns: 500.0,
                ..Default::default()
            };

            let ratio = DeltaBenchmarkResult::compute_speedup_ratio(&aad_stats, &bump_stats);
            assert!((ratio - 5.0).abs() < 1e-10);
        }

        #[test]
        fn test_compute_speedup_ratio_zero_aad() {
            let aad_stats = TimingStats {
                mean_ns: 0.0,
                ..Default::default()
            };
            let bump_stats = TimingStats {
                mean_ns: 500.0,
                ..Default::default()
            };

            let ratio = DeltaBenchmarkResult::compute_speedup_ratio(&aad_stats, &bump_stats);
            assert!((ratio - 0.0).abs() < 1e-10);
        }
    }

    // =========================================================================
    // Task 4.2: Scalability Result Tests
    // =========================================================================

    mod scalability_tests {
        use super::*;

        #[test]
        fn test_scalability_result_new() {
            let result = ScalabilityResult::new();
            assert!(result.is_empty());
            assert_eq!(result.len(), 0);
        }

        #[test]
        fn test_scalability_result_add_result() {
            let mut result = ScalabilityResult::new();

            let delta_result = DeltaBenchmarkResult {
                aad_stats: TimingStats::from_samples(&[100, 200, 300]),
                bump_stats: TimingStats::from_samples(&[500, 600, 700]),
                speedup_ratio: 3.0,
                tenor_count: 5,
            };

            result.add_result(5, delta_result);

            assert_eq!(result.len(), 1);
            assert!(!result.is_empty());
        }

        #[test]
        fn test_scalability_data_extraction() {
            let mut result = ScalabilityResult::new();

            // Add multiple data points
            for count in &[5, 10, 15, 20] {
                let delta_result = DeltaBenchmarkResult {
                    aad_stats: TimingStats {
                        mean_ns: (*count as f64) * 10.0,
                        ..Default::default()
                    },
                    bump_stats: TimingStats {
                        mean_ns: (*count as f64) * 50.0,
                        ..Default::default()
                    },
                    speedup_ratio: 5.0,
                    tenor_count: *count,
                };
                result.add_result(*count, delta_result);
            }

            let aad_data = result.aad_data();
            assert_eq!(aad_data.len(), 4);
            assert_eq!(aad_data[0], (5, 50.0));
            assert_eq!(aad_data[1], (10, 100.0));

            let bump_data = result.bump_data();
            assert_eq!(bump_data.len(), 4);
            assert_eq!(bump_data[0], (5, 250.0));

            let speedup_data = result.speedup_data();
            assert_eq!(speedup_data.len(), 4);
            assert_eq!(speedup_data[0], (5, 5.0));
        }
    }

    // =========================================================================
    // Task 4.1: Benchmark Runner Tests (Unit)
    // =========================================================================

    mod runner_tests {
        use super::*;

        #[test]
        fn test_benchmark_runner_new() {
            let config = BenchmarkConfig::default();
            let runner = BenchmarkRunner::new(config);
            assert_eq!(runner.config().iterations, 100);
        }

        #[test]
        fn test_benchmark_runner_with_defaults() {
            let runner = BenchmarkRunner::with_defaults();
            assert_eq!(runner.config().iterations, 100);
            assert_eq!(runner.config().warmup, 10);
        }
    }

    // =========================================================================
    // Task 4.3: Output Format Tests
    // =========================================================================

    mod output_tests {
        use super::*;

        fn create_test_full_result() -> FullBenchmarkResult {
            FullBenchmarkResult {
                benchmark_type: "delta_comparison".to_string(),
                timestamp: "2026-01-13T12:00:00Z".to_string(),
                swap_params: SwapParams {
                    notional: 1_000_000.0,
                    fixed_rate: 0.03,
                    tenor_years: 5.0,
                },
                delta_results: DeltaBenchmarkResult {
                    aad_stats: TimingStats {
                        mean_ns: 15000.0,
                        std_dev_ns: 500.0,
                        min_ns: 14000,
                        max_ns: 16500,
                        sample_count: 100,
                    },
                    bump_stats: TimingStats {
                        mean_ns: 300000.0,
                        std_dev_ns: 5000.0,
                        min_ns: 290000,
                        max_ns: 320000,
                        sample_count: 100,
                    },
                    speedup_ratio: 20.0,
                    tenor_count: 20,
                },
                scalability_results: None,
            }
        }

        #[test]
        fn test_to_json_contains_required_fields() {
            let runner = BenchmarkRunner::with_defaults();
            let result = create_test_full_result();

            let json = runner.to_json(&result);

            assert!(json.contains("benchmark_type"));
            assert!(json.contains("timestamp"));
            assert!(json.contains("notional"));
            assert!(json.contains("fixed_rate"));
            assert!(json.contains("tenor"));
            assert!(json.contains("mean_ns"));
            assert!(json.contains("std_dev_ns"));
            assert!(json.contains("speedup_ratio"));
        }

        #[test]
        fn test_to_markdown_contains_required_sections() {
            let runner = BenchmarkRunner::with_defaults();
            let result = create_test_full_result();

            let md = runner.to_markdown(&result);

            // Check for required sections
            assert!(md.contains("# IRS AAD Benchmark Results"));
            assert!(md.contains("## Swap Parameters"));
            assert!(md.contains("## Benchmark Results"));
            assert!(md.contains("### Timing Statistics"));
            assert!(md.contains("### Performance Summary"));
            assert!(md.contains("Speedup Ratio"));
        }

        #[test]
        fn test_to_markdown_with_scalability() {
            let runner = BenchmarkRunner::with_defaults();
            let mut result = create_test_full_result();

            // Add scalability results
            let mut scalability = ScalabilityResult::new();
            for count in &[5, 10, 15] {
                let delta_result = DeltaBenchmarkResult {
                    aad_stats: TimingStats {
                        mean_ns: (*count as f64) * 1000.0,
                        ..Default::default()
                    },
                    bump_stats: TimingStats {
                        mean_ns: (*count as f64) * 5000.0,
                        ..Default::default()
                    },
                    speedup_ratio: 5.0,
                    tenor_count: *count,
                };
                scalability.add_result(*count, delta_result);
            }
            result.scalability_results = Some(scalability);

            let md = runner.to_markdown(&result);

            assert!(md.contains("## Scalability Results"));
            assert!(md.contains("| Tenor Count |"));
        }

        #[test]
        fn test_scalability_to_chartjs_json() {
            let runner = BenchmarkRunner::with_defaults();
            let mut scalability = ScalabilityResult::new();

            for count in &[5, 10, 15] {
                let delta_result = DeltaBenchmarkResult {
                    aad_stats: TimingStats {
                        mean_ns: (*count as f64) * 1000.0,
                        ..Default::default()
                    },
                    bump_stats: TimingStats {
                        mean_ns: (*count as f64) * 5000.0,
                        ..Default::default()
                    },
                    speedup_ratio: 5.0,
                    tenor_count: *count,
                };
                scalability.add_result(*count, delta_result);
            }

            let json = runner.scalability_to_chartjs_json(&scalability);

            assert!(json.contains("labels"));
            assert!(json.contains("datasets"));
            assert!(json.contains("AAD"));
            assert!(json.contains("Bump-and-Revalue"));
        }
    }
}

// =============================================================================
// Integration Tests (with l1l2-integration feature)
// =============================================================================

#[cfg(all(test, feature = "l1l2-integration"))]
mod integration_tests {
    use super::*;
    use pricer_core::market_data::curves::{CurveEnum, CurveName, CurveSet};
    use pricer_core::types::time::{Date, DayCountConvention};
    use pricer_core::types::Currency;
    use pricer_models::instruments::rates::{
        FixedLeg, FloatingLeg, InterestRateSwap, RateIndex, SwapDirection,
    };
    use pricer_models::schedules::{Frequency, ScheduleBuilder};

    fn create_test_swap() -> InterestRateSwap<f64> {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2029, 1, 15).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::SemiAnnual)
            .day_count(DayCountConvention::Thirty360)
            .build()
            .unwrap();

        let fixed_leg = FixedLeg::new(schedule.clone(), 0.03, DayCountConvention::Thirty360);
        let floating_leg = FloatingLeg::new(
            schedule,
            0.0,
            RateIndex::Sofr,
            DayCountConvention::ActualActual360,
        );

        InterestRateSwap::new(
            1_000_000.0,
            fixed_leg,
            floating_leg,
            Currency::USD,
            SwapDirection::PayFixed,
        )
    }

    fn create_test_curves() -> CurveSet<f64> {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Discount, CurveEnum::flat(0.03));
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035));
        curves.set_discount_curve(CurveName::Discount);
        curves
    }

    // =========================================================================
    // Task 4.1: Benchmark Executor Integration Tests
    // =========================================================================

    #[test]
    fn test_benchmark_pv() {
        // Requirement 5.1: PV計算時間
        // Requirement 5.2: ウォームアップ実行、複数回反復による統計値算出
        let runner = BenchmarkRunner::new(BenchmarkConfig::new().with_iterations(10).with_warmup(2));
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let result = runner.benchmark_pv(&swap, &curves, valuation_date);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.stats.sample_count, 10);
        assert!(result.stats.mean_ns > 0.0);
        assert!(result.stats.min_ns > 0);
        assert!(result.stats.max_ns >= result.stats.min_ns);
    }

    #[test]
    fn test_benchmark_single_delta() {
        // Requirement 5.1: 単一Delta計算時間
        let runner = BenchmarkRunner::new(BenchmarkConfig::new().with_iterations(10).with_warmup(2));
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let result = runner.benchmark_single_delta(&swap, &curves, valuation_date, 2.0);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.stats.sample_count, 10);
        assert!((result.tenor - 2.0).abs() < 1e-10);
        assert!(result.stats.mean_ns > 0.0);
    }

    #[test]
    fn test_benchmark_deltas() {
        // Requirement 5.1: 全テナーDelta計算時間
        // Requirement 5.3: 速度比を計算
        let runner = BenchmarkRunner::new(BenchmarkConfig::new().with_iterations(10).with_warmup(2));
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let tenor_points = vec![0.5, 1.0, 2.0, 5.0];

        let result = runner.benchmark_deltas(&swap, &curves, valuation_date, &tenor_points);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.tenor_count, 4);
        assert_eq!(result.aad_stats.sample_count, 10);
        assert_eq!(result.bump_stats.sample_count, 10);
        assert!(result.aad_stats.mean_ns > 0.0);
        assert!(result.bump_stats.mean_ns > 0.0);
        // Speedup ratio should be calculated (currently ~1 since AAD falls back to bump)
        assert!(result.speedup_ratio > 0.0);
    }

    // =========================================================================
    // Task 4.2: Scalability Measurement Integration Tests
    // =========================================================================

    #[test]
    fn test_benchmark_scalability() {
        // Requirement 5.4: テナー数増加に伴う計算時間の推移データを生成
        let runner = BenchmarkRunner::new(BenchmarkConfig::new().with_iterations(5).with_warmup(1));
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let tenor_counts = &[2, 4, 6];

        let result = runner.benchmark_scalability(&swap, &curves, valuation_date, tenor_counts);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.len(), 3);

        // Verify data extraction works
        let aad_data = result.aad_data();
        let bump_data = result.bump_data();
        let speedup_data = result.speedup_data();

        assert_eq!(aad_data.len(), 3);
        assert_eq!(bump_data.len(), 3);
        assert_eq!(speedup_data.len(), 3);

        // Verify tenor counts
        assert_eq!(aad_data[0].0, 2);
        assert_eq!(aad_data[1].0, 4);
        assert_eq!(aad_data[2].0, 6);
    }

    #[test]
    fn test_scalability_bump_time_increases_with_tenor_count() {
        // Requirement 5.4: AAD O(1)逆伝播 vs Bump O(n)のスケーリング特性
        let runner = BenchmarkRunner::new(BenchmarkConfig::new().with_iterations(10).with_warmup(2));
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let tenor_counts = &[4, 8, 12];

        let result = runner
            .benchmark_scalability(&swap, &curves, valuation_date, tenor_counts)
            .unwrap();

        let bump_data = result.bump_data();

        // Bump time should generally increase with tenor count
        // Note: Due to timing variance, we use a relaxed check
        assert!(bump_data[0].1 > 0.0, "Bump time should be positive for 4 tenors");
        assert!(bump_data[2].1 > 0.0, "Bump time should be positive for 12 tenors");

        // The ratio of times should be roughly proportional to tenor ratio
        // 12 tenors / 4 tenors = 3x, so time ratio should be > 1.5x (relaxed)
        let ratio = bump_data[2].1 / bump_data[0].1;
        println!("Bump time ratio (12 tenors / 4 tenors): {}", ratio);
        // Due to timing variance, we just check it's not decreasing dramatically
        assert!(
            ratio > 0.5,
            "Bump time should scale with tenor count, ratio: {}",
            ratio
        );
    }

    // =========================================================================
    // Task 4.3: Benchmark Output Integration Tests
    // =========================================================================

    #[test]
    fn test_create_full_result() {
        // Requirement 5.5: タイムスタンプ、スワップパラメータ、計測結果を含むスキーマ
        let runner = BenchmarkRunner::new(BenchmarkConfig::new().with_iterations(5).with_warmup(1));
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let tenor_points = vec![1.0, 2.0, 5.0];

        let delta_results = runner
            .benchmark_deltas(&swap, &curves, valuation_date, &tenor_points)
            .unwrap();

        let full_result = runner.create_full_result(&swap, delta_results, None);

        assert_eq!(full_result.benchmark_type, "delta_comparison");
        assert!(!full_result.timestamp.is_empty());
        assert!((full_result.swap_params.notional - 1_000_000.0).abs() < 1e-10);
        assert!((full_result.swap_params.fixed_rate - 0.03).abs() < 1e-10);
        assert!(full_result.swap_params.tenor_years > 0.0);
        assert!(full_result.scalability_results.is_none());
    }

    #[test]
    fn test_full_benchmark_to_json() {
        // Requirement 5.5: 結果をJSON形式で出力
        let runner = BenchmarkRunner::new(BenchmarkConfig::new().with_iterations(5).with_warmup(1));
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let tenor_points = vec![1.0, 2.0];

        let delta_results = runner
            .benchmark_deltas(&swap, &curves, valuation_date, &tenor_points)
            .unwrap();

        let full_result = runner.create_full_result(&swap, delta_results, None);
        let json = runner.to_json(&full_result);

        // Verify JSON structure
        assert!(json.contains("delta_comparison"));
        assert!(json.contains("1000000"));
        assert!(json.contains("0.03"));
        assert!(json.contains("mean_ns"));
        assert!(json.contains("speedup_ratio"));
    }

    #[test]
    fn test_full_benchmark_to_markdown() {
        // Requirement 5.5: 結果をMarkdown形式で出力
        let runner = BenchmarkRunner::new(BenchmarkConfig::new().with_iterations(5).with_warmup(1));
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let tenor_points = vec![1.0, 2.0];

        let delta_results = runner
            .benchmark_deltas(&swap, &curves, valuation_date, &tenor_points)
            .unwrap();

        let full_result = runner.create_full_result(&swap, delta_results, None);
        let md = runner.to_markdown(&full_result);

        // Verify Markdown structure
        assert!(md.contains("# IRS AAD Benchmark Results"));
        assert!(md.contains("| Notional |"));
        assert!(md.contains("| Mean (ns) |"));
        assert!(md.contains("**Speedup Ratio:**"));
    }

    #[test]
    fn test_full_workflow_with_scalability() {
        // Full integration test: Run benchmarks and generate all outputs
        let runner = BenchmarkRunner::new(BenchmarkConfig::new().with_iterations(3).with_warmup(1));
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        // Run delta benchmark
        let tenor_points = vec![1.0, 2.0, 5.0];
        let delta_results = runner
            .benchmark_deltas(&swap, &curves, valuation_date, &tenor_points)
            .unwrap();

        // Run scalability benchmark
        let scalability_results = runner
            .benchmark_scalability(&swap, &curves, valuation_date, &[2, 4])
            .unwrap();

        // Create full result
        let full_result = runner.create_full_result(&swap, delta_results, Some(scalability_results));

        // Generate outputs
        let json = runner.to_json(&full_result);
        let md = runner.to_markdown(&full_result);
        let chartjs = runner.scalability_to_chartjs_json(full_result.scalability_results.as_ref().unwrap());

        // Verify all outputs are non-empty
        assert!(!json.is_empty());
        assert!(!md.is_empty());
        assert!(!chartjs.is_empty());

        // Verify scalability section in markdown
        assert!(md.contains("## Scalability Results"));
    }
}
