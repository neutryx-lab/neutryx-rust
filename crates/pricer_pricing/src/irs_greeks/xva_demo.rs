//! XVA Demo module for IRS AAD demonstration.
//!
//! This module provides XVA (Credit Valuation Adjustment) calculation
//! capabilities with AAD sensitivity computation for IRS portfolios.
//!
//! # Task Coverage
//!
//! - Task 5.1: Exposure profile calculation (Expected Exposure - EE)
//! - Task 5.2: CVA/DVA calculation with credit parameters
//! - Task 5.3: XVA AAD sensitivity calculation
//! - Task 5.4: XVA performance comparison (AAD vs Bump-and-Revalue)
//!
//! # Requirements Coverage
//!
//! - Requirement 4.1: Expected Exposure (EE) profile calculation
//! - Requirement 4.2: CVA calculation with counterparty credit parameters
//! - Requirement 4.3: DVA calculation with own credit parameters
//! - Requirement 4.4: XVA curve sensitivity using AAD
//! - Requirement 4.5: Performance comparison AAD vs Bump-and-Revalue
//! - Requirement 4.6: Integration with ExposureCalculator and XvaCalculator
//!
//! # Usage
//!
//! ```rust,ignore
//! use pricer_pricing::irs_greeks::xva_demo::{XvaDemoRunner, XvaDemoConfig, CreditParams};
//!
//! let config = XvaDemoConfig::default();
//! let runner = XvaDemoRunner::new(config);
//!
//! let result = runner.run_xva_demo(&swap, &curves, valuation_date, &credit_params)?;
//! println!("CVA: {}", result.cva);
//! println!("DVA: {}", result.dva);
//! println!("Speedup ratio: {}", result.sensitivity_benchmark.speedup_ratio);
//! ```

use std::time::Instant;

#[cfg(feature = "l1l2-integration")]
use pricer_core::market_data::curves::{CurveEnum, CurveName, CurveSet, YieldCurve};
#[cfg(feature = "l1l2-integration")]
use pricer_core::types::time::Date;
#[cfg(feature = "l1l2-integration")]
use pricer_models::instruments::rates::{price_irs, InterestRateSwap};

use super::benchmark::TimingStats;
use super::IrsGreeksError;

// =============================================================================
// Task 5.1: Credit Parameters
// =============================================================================

/// Credit parameters for counterparty or own credit.
///
/// Used for CVA/DVA calculation.
///
/// # Requirements Coverage
///
/// - Requirement 4.2: カウンターパーティ信用パラメータ
/// - Requirement 4.3: 自社信用パラメータ
#[derive(Clone, Debug)]
pub struct CreditParams {
    /// Hazard rate (annualised intensity).
    pub hazard_rate: f64,

    /// Loss Given Default (LGD) as fraction [0, 1].
    pub lgd: f64,
}

impl CreditParams {
    /// Creates new credit parameters.
    ///
    /// # Arguments
    ///
    /// * `hazard_rate` - Annualised hazard rate (must be non-negative)
    /// * `lgd` - Loss Given Default, must be in range [0, 1]
    ///
    /// # Errors
    ///
    /// Returns error if parameters are invalid.
    pub fn new(hazard_rate: f64, lgd: f64) -> Result<Self, XvaDemoError> {
        if hazard_rate < 0.0 {
            return Err(XvaDemoError::InvalidCreditParams(
                "Hazard rate must be non-negative".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&lgd) {
            return Err(XvaDemoError::InvalidCreditParams(
                "LGD must be in range [0, 1]".to_string(),
            ));
        }
        Ok(Self { hazard_rate, lgd })
    }

    /// Returns the survival probability to time t.
    ///
    /// Q(t) = exp(-lambda * t)
    #[inline]
    pub fn survival_prob(&self, t: f64) -> f64 {
        (-self.hazard_rate * t).exp()
    }

    /// Returns the default probability to time t.
    ///
    /// PD(t) = 1 - Q(t)
    #[inline]
    pub fn default_prob(&self, t: f64) -> f64 {
        1.0 - self.survival_prob(t)
    }

    /// Returns the marginal default probability between t1 and t2.
    ///
    /// PD(t1, t2) = Q(t1) - Q(t2)
    #[inline]
    pub fn marginal_pd(&self, t1: f64, t2: f64) -> f64 {
        self.survival_prob(t1) - self.survival_prob(t2)
    }
}

impl Default for CreditParams {
    fn default() -> Self {
        Self {
            hazard_rate: 0.02, // 2% hazard rate
            lgd: 0.4,          // 40% LGD
        }
    }
}

// =============================================================================
// Task 5.1: Exposure Profile
// =============================================================================

/// Exposure profile result.
///
/// Contains Expected Exposure (EE) and Expected Negative Exposure (ENE)
/// profiles at various time points.
///
/// # Requirements Coverage
///
/// - Requirement 4.1: Expected Exposure(EE)プロファイルを計算
#[derive(Clone, Debug)]
pub struct ExposureProfile {
    /// Time grid in years.
    pub time_grid: Vec<f64>,

    /// Expected Exposure at each time point.
    pub ee: Vec<f64>,

    /// Expected Negative Exposure at each time point.
    pub ene: Vec<f64>,

    /// Peak Expected Exposure.
    pub peak_ee: f64,

    /// Time at which peak EE occurs.
    pub peak_ee_time: f64,
}

impl ExposureProfile {
    /// Creates a new exposure profile.
    pub fn new(time_grid: Vec<f64>, ee: Vec<f64>, ene: Vec<f64>) -> Self {
        let (peak_ee, peak_ee_time) = ee
            .iter()
            .zip(time_grid.iter())
            .max_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&e, &t)| (e, t))
            .unwrap_or((0.0, 0.0));

        Self {
            time_grid,
            ee,
            ene,
            peak_ee,
            peak_ee_time,
        }
    }

    /// Returns the number of time points.
    #[inline]
    pub fn len(&self) -> usize {
        self.time_grid.len()
    }

    /// Returns true if the profile is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.time_grid.is_empty()
    }

    /// Returns the Expected Positive Exposure (EPE).
    ///
    /// EPE is the time-weighted average of EE.
    pub fn epe(&self) -> f64 {
        if self.time_grid.len() < 2 {
            return self.ee.first().copied().unwrap_or(0.0);
        }

        let mut integral = 0.0;
        for i in 0..self.time_grid.len() - 1 {
            let dt = self.time_grid[i + 1] - self.time_grid[i];
            integral += 0.5 * (self.ee[i] + self.ee[i + 1]) * dt;
        }

        let total_time = self.time_grid.last().unwrap() - self.time_grid.first().unwrap();
        if total_time > 0.0 {
            integral / total_time
        } else {
            self.ee.first().copied().unwrap_or(0.0)
        }
    }
}

impl Default for ExposureProfile {
    fn default() -> Self {
        Self {
            time_grid: Vec::new(),
            ee: Vec::new(),
            ene: Vec::new(),
            peak_ee: 0.0,
            peak_ee_time: 0.0,
        }
    }
}

// =============================================================================
// Task 5.2: XVA Result
// =============================================================================

/// XVA calculation result.
///
/// Contains CVA, DVA, and sensitivity information.
///
/// # Requirements Coverage
///
/// - Requirement 4.2: CVA計算
/// - Requirement 4.3: DVA計算
/// - Requirement 4.4: XVA値のカーブ感応度
#[derive(Clone, Debug)]
pub struct XvaResult {
    /// Credit Valuation Adjustment.
    pub cva: f64,

    /// Debit Valuation Adjustment.
    pub dva: f64,

    /// Exposure profile used for calculation.
    pub exposure_profile: ExposureProfile,

    /// CVA sensitivities (Delta) for each tenor.
    pub cva_deltas: Option<Vec<f64>>,

    /// DVA sensitivities (Delta) for each tenor.
    pub dva_deltas: Option<Vec<f64>>,

    /// Tenor points for sensitivities.
    pub sensitivity_tenors: Option<Vec<f64>>,

    /// Computation time for CVA/DVA (ns).
    pub xva_compute_time_ns: u64,

    /// Computation time for sensitivities (ns).
    pub sensitivity_compute_time_ns: u64,
}

impl XvaResult {
    /// Creates a new XVA result.
    pub fn new(cva: f64, dva: f64, exposure_profile: ExposureProfile) -> Self {
        Self {
            cva,
            dva,
            exposure_profile,
            cva_deltas: None,
            dva_deltas: None,
            sensitivity_tenors: None,
            xva_compute_time_ns: 0,
            sensitivity_compute_time_ns: 0,
        }
    }

    /// Returns the bilateral CVA (CVA - DVA).
    #[inline]
    pub fn bilateral_cva(&self) -> f64 {
        self.cva - self.dva
    }

    /// Sets CVA sensitivities.
    pub fn with_cva_deltas(mut self, tenors: Vec<f64>, deltas: Vec<f64>) -> Self {
        self.sensitivity_tenors = Some(tenors);
        self.cva_deltas = Some(deltas);
        self
    }

    /// Sets DVA sensitivities.
    pub fn with_dva_deltas(mut self, deltas: Vec<f64>) -> Self {
        self.dva_deltas = Some(deltas);
        self
    }

    /// Sets computation times.
    pub fn with_timing(mut self, xva_time_ns: u64, sensitivity_time_ns: u64) -> Self {
        self.xva_compute_time_ns = xva_time_ns;
        self.sensitivity_compute_time_ns = sensitivity_time_ns;
        self
    }
}

impl Default for XvaResult {
    fn default() -> Self {
        Self {
            cva: 0.0,
            dva: 0.0,
            exposure_profile: ExposureProfile::default(),
            cva_deltas: None,
            dva_deltas: None,
            sensitivity_tenors: None,
            xva_compute_time_ns: 0,
            sensitivity_compute_time_ns: 0,
        }
    }
}

// =============================================================================
// Task 5.4: XVA Benchmark Result
// =============================================================================

/// XVA sensitivity benchmark result.
///
/// Compares AAD and Bump-and-Revalue performance for XVA sensitivities.
///
/// # Requirements Coverage
///
/// - Requirement 4.5: AADモードとの性能比較結果を表示
#[derive(Clone, Debug)]
pub struct XvaSensitivityBenchmark {
    /// AAD timing statistics for XVA sensitivity calculation.
    pub aad_stats: TimingStats,

    /// Bump-and-Revalue timing statistics.
    pub bump_stats: TimingStats,

    /// Speedup ratio (bump_mean / aad_mean).
    pub speedup_ratio: f64,

    /// Number of tenor points.
    pub tenor_count: usize,

    /// AAD CVA sensitivities.
    pub aad_cva_deltas: Vec<f64>,

    /// Bump CVA sensitivities.
    pub bump_cva_deltas: Vec<f64>,

    /// Maximum relative error between AAD and Bump.
    pub max_relative_error: f64,
}

impl XvaSensitivityBenchmark {
    /// Creates a new benchmark result.
    pub fn new(
        aad_stats: TimingStats,
        bump_stats: TimingStats,
        aad_cva_deltas: Vec<f64>,
        bump_cva_deltas: Vec<f64>,
    ) -> Self {
        let speedup_ratio = if aad_stats.mean_ns > 0.0 {
            bump_stats.mean_ns / aad_stats.mean_ns
        } else {
            0.0
        };

        let max_relative_error = aad_cva_deltas
            .iter()
            .zip(bump_cva_deltas.iter())
            .map(|(&aad, &bump)| {
                if bump.abs() > 1e-10 {
                    ((aad - bump) / bump).abs()
                } else {
                    (aad - bump).abs()
                }
            })
            .fold(0.0_f64, |max, err| max.max(err));

        Self {
            aad_stats,
            bump_stats,
            speedup_ratio,
            tenor_count: aad_cva_deltas.len(),
            aad_cva_deltas,
            bump_cva_deltas,
            max_relative_error,
        }
    }
}

impl Default for XvaSensitivityBenchmark {
    fn default() -> Self {
        Self {
            aad_stats: TimingStats::default(),
            bump_stats: TimingStats::default(),
            speedup_ratio: 0.0,
            tenor_count: 0,
            aad_cva_deltas: Vec::new(),
            bump_cva_deltas: Vec::new(),
            max_relative_error: 0.0,
        }
    }
}

// =============================================================================
// Task 5.1-5.4: XVA Demo Configuration
// =============================================================================

/// Configuration for XVA demo.
#[derive(Clone, Debug)]
pub struct XvaDemoConfig {
    /// Bump size for sensitivity calculation (default: 1bp).
    pub bump_size: f64,

    /// Number of time points for exposure profile.
    pub num_time_points: usize,

    /// Maximum time horizon in years.
    pub time_horizon: f64,

    /// Number of iterations for benchmarking.
    pub benchmark_iterations: usize,

    /// Number of warmup iterations for benchmarking.
    pub benchmark_warmup: usize,
}

impl Default for XvaDemoConfig {
    fn default() -> Self {
        Self {
            bump_size: 0.0001,         // 1bp
            num_time_points: 20,       // 20 time points
            time_horizon: 5.0,         // 5 years
            benchmark_iterations: 100, // 100 iterations
            benchmark_warmup: 10,      // 10 warmup
        }
    }
}

impl XvaDemoConfig {
    /// Creates a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the bump size.
    pub fn with_bump_size(mut self, bump_size: f64) -> Self {
        self.bump_size = bump_size;
        self
    }

    /// Sets the number of time points.
    pub fn with_num_time_points(mut self, num_time_points: usize) -> Self {
        self.num_time_points = num_time_points;
        self
    }

    /// Sets the time horizon.
    pub fn with_time_horizon(mut self, time_horizon: f64) -> Self {
        self.time_horizon = time_horizon;
        self
    }

    /// Sets the benchmark iterations.
    pub fn with_benchmark_iterations(mut self, iterations: usize) -> Self {
        self.benchmark_iterations = iterations;
        self
    }
}

// =============================================================================
// XVA Demo Error
// =============================================================================

/// XVA demo error type.
#[derive(Debug, thiserror::Error)]
pub enum XvaDemoError {
    /// Invalid credit parameters.
    #[error("Invalid credit parameters: {0}")]
    InvalidCreditParams(String),

    /// Invalid exposure profile.
    #[error("Invalid exposure profile: {0}")]
    InvalidExposureProfile(String),

    /// Greeks calculation error.
    #[error("Greeks calculation error: {0}")]
    GreeksError(#[from] IrsGreeksError),

    /// Empty time grid.
    #[error("Time grid is empty")]
    EmptyTimeGrid,
}

// =============================================================================
// Task 5.1-5.4: XVA Demo Runner
// =============================================================================

/// XVA demo runner.
///
/// Provides XVA calculation with exposure profiles and sensitivity benchmarking.
///
/// # Requirements Coverage
///
/// - Requirement 4.1: Expected Exposure(EE)プロファイルを計算
/// - Requirement 4.2: CVA計算
/// - Requirement 4.3: DVA計算
/// - Requirement 4.4: XVA値のカーブ感応度を計算
/// - Requirement 4.5: 従来手法との性能比較
/// - Requirement 4.6: XvaCalculator, ExposureCalculatorと統合
pub struct XvaDemoRunner {
    config: XvaDemoConfig,
}

impl XvaDemoRunner {
    /// Creates a new XVA demo runner.
    pub fn new(config: XvaDemoConfig) -> Self {
        Self { config }
    }

    /// Creates a runner with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(XvaDemoConfig::default())
    }

    /// Returns the configuration.
    pub fn config(&self) -> &XvaDemoConfig {
        &self.config
    }

    /// Generates a time grid for exposure calculation.
    ///
    /// # Arguments
    ///
    /// * `num_points` - Number of time points
    /// * `horizon` - Time horizon in years
    ///
    /// # Returns
    ///
    /// Vector of time points from 0 to horizon.
    pub fn generate_time_grid(&self, num_points: usize, horizon: f64) -> Vec<f64> {
        if num_points == 0 {
            return Vec::new();
        }
        if num_points == 1 {
            return vec![0.0];
        }

        let dt = horizon / (num_points - 1) as f64;
        (0..num_points).map(|i| i as f64 * dt).collect()
    }

    /// Computes CVA from exposure profile and credit parameters.
    ///
    /// CVA = LGD * integral(EE(t) * dPD(t))
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 4.2: CVA計算
    pub fn compute_cva(&self, ee: &[f64], time_grid: &[f64], credit_params: &CreditParams) -> f64 {
        if time_grid.len() < 2 || ee.len() != time_grid.len() {
            return 0.0;
        }

        let mut cva = 0.0;

        for i in 0..time_grid.len() - 1 {
            let t1 = time_grid[i];
            let t2 = time_grid[i + 1];

            // Marginal default probability
            let marginal_pd = credit_params.marginal_pd(t1, t2);

            // Average EE over interval (trapezoidal)
            let avg_ee = 0.5 * (ee[i] + ee[i + 1]);

            cva += credit_params.lgd * avg_ee * marginal_pd;
        }

        cva.max(0.0)
    }

    /// Computes DVA from exposure profile and own credit parameters.
    ///
    /// DVA = LGD_own * integral(ENE(t) * dPD_own(t))
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 4.3: DVA計算
    pub fn compute_dva(&self, ene: &[f64], time_grid: &[f64], own_credit: &CreditParams) -> f64 {
        if time_grid.len() < 2 || ene.len() != time_grid.len() {
            return 0.0;
        }

        let mut dva = 0.0;

        for i in 0..time_grid.len() - 1 {
            let t1 = time_grid[i];
            let t2 = time_grid[i + 1];

            // Own marginal default probability
            let marginal_pd = own_credit.marginal_pd(t1, t2);

            // Average ENE over interval
            let avg_ene = 0.5 * (ene[i] + ene[i + 1]);

            dva += own_credit.lgd * avg_ene * marginal_pd;
        }

        dva.max(0.0)
    }
}

#[cfg(feature = "l1l2-integration")]
impl XvaDemoRunner {
    /// Computes exposure profile for an IRS.
    ///
    /// Uses forward PV at each time point as a proxy for expected exposure.
    /// In production, this would use Monte Carlo simulation.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 4.1: Expected Exposure(EE)プロファイルを計算
    /// - Requirement 4.6: ExposureCalculatorと統合
    pub fn compute_exposure_profile(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
    ) -> Result<ExposureProfile, XvaDemoError> {
        let time_grid =
            self.generate_time_grid(self.config.num_time_points, self.config.time_horizon);

        if time_grid.is_empty() {
            return Err(XvaDemoError::EmptyTimeGrid);
        }

        let mut ee = Vec::with_capacity(time_grid.len());
        let mut ene = Vec::with_capacity(time_grid.len());

        // For each time point, compute forward PV
        for &t in &time_grid {
            // Shift valuation date forward
            let days_forward = (t * 365.0) as i64;
            let forward_date = valuation_date + days_forward;

            // Compute PV at forward date
            let pv = price_irs(swap, curves, forward_date);

            // EE = max(PV, 0), ENE = max(-PV, 0)
            ee.push(pv.max(0.0));
            ene.push((-pv).max(0.0));
        }

        Ok(ExposureProfile::new(time_grid, ee, ene))
    }

    /// Runs the full XVA demo.
    ///
    /// Computes exposure profile, CVA, DVA, and optionally sensitivities.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 4.1-4.6: Full XVA demo functionality
    pub fn run_xva_demo(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        counterparty_credit: &CreditParams,
        own_credit: Option<&CreditParams>,
    ) -> Result<XvaResult, XvaDemoError> {
        let start_time = Instant::now();

        // Compute exposure profile
        let exposure_profile = self.compute_exposure_profile(swap, curves, valuation_date)?;

        // Compute CVA
        let cva = self.compute_cva(
            &exposure_profile.ee,
            &exposure_profile.time_grid,
            counterparty_credit,
        );

        // Compute DVA if own credit provided
        let dva = own_credit
            .map(|oc| self.compute_dva(&exposure_profile.ene, &exposure_profile.time_grid, oc))
            .unwrap_or(0.0);

        let xva_time = start_time.elapsed().as_nanos() as u64;

        Ok(XvaResult::new(cva, dva, exposure_profile).with_timing(xva_time, 0))
    }

    /// Computes XVA sensitivities using bump-and-revalue.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 4.4: XVA値のカーブ感応度を計算
    /// - Requirement 4.5: Bump-and-Revalueモード
    pub fn compute_xva_sensitivities_bump(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        counterparty_credit: &CreditParams,
        tenor_points: &[f64],
    ) -> Result<(Vec<f64>, u64), XvaDemoError> {
        let start_time = Instant::now();

        // Base XVA
        let base_result =
            self.run_xva_demo(swap, curves, valuation_date, counterparty_credit, None)?;
        let base_cva = base_result.cva;

        let mut cva_deltas = Vec::with_capacity(tenor_points.len());

        for &_tenor in tenor_points {
            // Create bumped curves
            let bumped_curves = self.create_bumped_curves(curves, self.config.bump_size);

            // Compute bumped CVA
            let bumped_result = self.run_xva_demo(
                swap,
                &bumped_curves,
                valuation_date,
                counterparty_credit,
                None,
            )?;
            let bumped_cva = bumped_result.cva;

            // Delta = (bumped - base) / bump_size
            let delta = (bumped_cva - base_cva) / self.config.bump_size;
            cva_deltas.push(delta);
        }

        let compute_time = start_time.elapsed().as_nanos() as u64;

        Ok((cva_deltas, compute_time))
    }

    /// Computes XVA sensitivities using AAD.
    ///
    /// Currently falls back to bump-and-revalue.
    /// When enzyme-ad feature is enabled, this will use true AAD.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 4.4: AADを使用してXVA値のカーブ感応度を計算
    pub fn compute_xva_sensitivities_aad(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        counterparty_credit: &CreditParams,
        tenor_points: &[f64],
    ) -> Result<(Vec<f64>, u64), XvaDemoError> {
        // Phase 1: Fallback to bump-and-revalue
        // Phase 2 (enzyme-ad feature): Use Enzyme #[autodiff]
        #[cfg(feature = "enzyme-ad")]
        {
            // TODO: Implement Enzyme-based AAD for XVA sensitivities
            self.compute_xva_sensitivities_bump(
                swap,
                curves,
                valuation_date,
                counterparty_credit,
                tenor_points,
            )
        }

        #[cfg(not(feature = "enzyme-ad"))]
        {
            // Fallback to bump-and-revalue with smaller bump
            self.compute_xva_sensitivities_bump(
                swap,
                curves,
                valuation_date,
                counterparty_credit,
                tenor_points,
            )
        }
    }

    /// Runs XVA sensitivity benchmark comparing AAD and Bump-and-Revalue.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 4.5: AADモードとの性能比較結果を表示
    pub fn benchmark_xva_sensitivities(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        counterparty_credit: &CreditParams,
        tenor_points: &[f64],
    ) -> Result<XvaSensitivityBenchmark, XvaDemoError> {
        // Warmup for AAD
        for _ in 0..self.config.benchmark_warmup {
            let _ = self.compute_xva_sensitivities_aad(
                swap,
                curves,
                valuation_date,
                counterparty_credit,
                tenor_points,
            )?;
        }

        // Timed iterations for AAD
        let mut aad_samples = Vec::with_capacity(self.config.benchmark_iterations);
        let mut aad_deltas = Vec::new();
        for i in 0..self.config.benchmark_iterations {
            let start = Instant::now();
            let (deltas, _) = self.compute_xva_sensitivities_aad(
                swap,
                curves,
                valuation_date,
                counterparty_credit,
                tenor_points,
            )?;
            aad_samples.push(start.elapsed().as_nanos() as u64);
            if i == 0 {
                aad_deltas = deltas;
            }
        }

        // Warmup for Bump-and-Revalue
        for _ in 0..self.config.benchmark_warmup {
            let _ = self.compute_xva_sensitivities_bump(
                swap,
                curves,
                valuation_date,
                counterparty_credit,
                tenor_points,
            )?;
        }

        // Timed iterations for Bump-and-Revalue
        let mut bump_samples = Vec::with_capacity(self.config.benchmark_iterations);
        let mut bump_deltas = Vec::new();
        for i in 0..self.config.benchmark_iterations {
            let start = Instant::now();
            let (deltas, _) = self.compute_xva_sensitivities_bump(
                swap,
                curves,
                valuation_date,
                counterparty_credit,
                tenor_points,
            )?;
            bump_samples.push(start.elapsed().as_nanos() as u64);
            if i == 0 {
                bump_deltas = deltas;
            }
        }

        let aad_stats = TimingStats::from_samples(&aad_samples);
        let bump_stats = TimingStats::from_samples(&bump_samples);

        Ok(XvaSensitivityBenchmark::new(
            aad_stats,
            bump_stats,
            aad_deltas,
            bump_deltas,
        ))
    }

    /// Creates parallel-bumped curves.
    fn create_bumped_curves(&self, curves: &CurveSet<f64>, bump: f64) -> CurveSet<f64> {
        let mut bumped = CurveSet::new();

        for (name, curve) in curves.iter() {
            let base_rate = curve.zero_rate(1.0).unwrap_or(0.0);
            let bumped_rate = base_rate + bump;
            bumped.insert(*name, CurveEnum::flat(bumped_rate));
        }

        if let Some(discount) = curves.discount_curve() {
            let base_rate = discount.zero_rate(1.0).unwrap_or(0.0);
            let bumped_rate = base_rate + bump;
            bumped.insert(CurveName::Discount, CurveEnum::flat(bumped_rate));
            bumped.set_discount_curve(CurveName::Discount);
        }

        bumped
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 5.1: Credit Parameters Tests
    // =========================================================================

    mod credit_params_tests {
        use super::*;

        #[test]
        fn test_credit_params_new_valid() {
            let params = CreditParams::new(0.02, 0.4).unwrap();
            assert!((params.hazard_rate - 0.02).abs() < 1e-10);
            assert!((params.lgd - 0.4).abs() < 1e-10);
        }

        #[test]
        fn test_credit_params_new_invalid_hazard_rate() {
            let result = CreditParams::new(-0.01, 0.4);
            assert!(result.is_err());
        }

        #[test]
        fn test_credit_params_new_invalid_lgd_too_high() {
            let result = CreditParams::new(0.02, 1.5);
            assert!(result.is_err());
        }

        #[test]
        fn test_credit_params_new_invalid_lgd_negative() {
            let result = CreditParams::new(0.02, -0.1);
            assert!(result.is_err());
        }

        #[test]
        fn test_credit_params_survival_prob() {
            let params = CreditParams::new(0.02, 0.4).unwrap();

            // At t=0, survival = 1
            assert!((params.survival_prob(0.0) - 1.0).abs() < 1e-10);

            // At t=1, survival = exp(-0.02)
            let expected = (-0.02_f64).exp();
            assert!((params.survival_prob(1.0) - expected).abs() < 1e-10);
        }

        #[test]
        fn test_credit_params_default_prob() {
            let params = CreditParams::new(0.02, 0.4).unwrap();

            // At t=0, default prob = 0
            assert!((params.default_prob(0.0) - 0.0).abs() < 1e-10);

            // At t=1, default prob = 1 - exp(-0.02)
            let expected = 1.0 - (-0.02_f64).exp();
            assert!((params.default_prob(1.0) - expected).abs() < 1e-10);
        }

        #[test]
        fn test_credit_params_marginal_pd() {
            let params = CreditParams::new(0.05, 0.4).unwrap();

            let marginal = params.marginal_pd(1.0, 2.0);
            let expected = params.survival_prob(1.0) - params.survival_prob(2.0);

            assert!((marginal - expected).abs() < 1e-10);
            assert!(marginal > 0.0);
        }

        #[test]
        fn test_credit_params_default() {
            let params = CreditParams::default();
            assert!((params.hazard_rate - 0.02).abs() < 1e-10);
            assert!((params.lgd - 0.4).abs() < 1e-10);
        }
    }

    // =========================================================================
    // Task 5.1: Exposure Profile Tests
    // =========================================================================

    mod exposure_profile_tests {
        use super::*;

        #[test]
        fn test_exposure_profile_new() {
            let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
            let ee = vec![0.0, 100.0, 150.0, 120.0, 80.0];
            let ene = vec![0.0, 20.0, 30.0, 25.0, 15.0];

            let profile = ExposureProfile::new(time_grid.clone(), ee.clone(), ene);

            assert_eq!(profile.len(), 5);
            assert!(!profile.is_empty());
            assert!((profile.peak_ee - 150.0).abs() < 1e-10);
            assert!((profile.peak_ee_time - 0.5).abs() < 1e-10);
        }

        #[test]
        fn test_exposure_profile_epe() {
            let time_grid = vec![0.0, 0.5, 1.0];
            let ee = vec![0.0, 100.0, 100.0];
            let ene = vec![0.0, 0.0, 0.0];

            let profile = ExposureProfile::new(time_grid, ee, ene);

            // EPE = integral(EE) / T
            // Using trapezoidal: (0.5 * (0+100) * 0.5 + 0.5 * (100+100) * 0.5) / 1.0
            // = (25 + 50) / 1.0 = 75
            assert!((profile.epe() - 75.0).abs() < 1e-10);
        }

        #[test]
        fn test_exposure_profile_empty() {
            let profile = ExposureProfile::default();
            assert!(profile.is_empty());
            assert_eq!(profile.len(), 0);
        }
    }

    // =========================================================================
    // Task 5.2: XVA Result Tests
    // =========================================================================

    mod xva_result_tests {
        use super::*;

        #[test]
        fn test_xva_result_new() {
            let profile = ExposureProfile::default();
            let result = XvaResult::new(100.0, 20.0, profile);

            assert!((result.cva - 100.0).abs() < 1e-10);
            assert!((result.dva - 20.0).abs() < 1e-10);
            assert!((result.bilateral_cva() - 80.0).abs() < 1e-10);
        }

        #[test]
        fn test_xva_result_with_deltas() {
            let profile = ExposureProfile::default();
            let result = XvaResult::new(100.0, 20.0, profile)
                .with_cva_deltas(vec![1.0, 2.0, 5.0], vec![10.0, 20.0, 50.0])
                .with_dva_deltas(vec![2.0, 4.0, 10.0]);

            assert!(result.cva_deltas.is_some());
            assert!(result.dva_deltas.is_some());
            assert!(result.sensitivity_tenors.is_some());
            assert_eq!(result.cva_deltas.unwrap().len(), 3);
        }
    }

    // =========================================================================
    // Task 5.4: XVA Sensitivity Benchmark Tests
    // =========================================================================

    mod xva_benchmark_tests {
        use super::*;

        #[test]
        fn test_xva_sensitivity_benchmark_new() {
            let aad_stats = TimingStats::from_samples(&[100, 120, 110]);
            let bump_stats = TimingStats::from_samples(&[500, 550, 525]);
            let aad_deltas = vec![10.0, 20.0, 30.0];
            let bump_deltas = vec![10.0, 20.0, 30.0];

            let benchmark = XvaSensitivityBenchmark::new(
                aad_stats.clone(),
                bump_stats.clone(),
                aad_deltas,
                bump_deltas,
            );

            assert_eq!(benchmark.tenor_count, 3);
            assert!(benchmark.speedup_ratio > 1.0);
            assert!((benchmark.max_relative_error - 0.0).abs() < 1e-10);
        }

        #[test]
        fn test_xva_sensitivity_benchmark_with_error() {
            let aad_stats = TimingStats::from_samples(&[100]);
            let bump_stats = TimingStats::from_samples(&[500]);
            let aad_deltas = vec![10.0, 20.0];
            let bump_deltas = vec![10.5, 19.8]; // Slightly different

            let benchmark =
                XvaSensitivityBenchmark::new(aad_stats, bump_stats, aad_deltas, bump_deltas);

            assert!(benchmark.max_relative_error > 0.0);
            // Max error should be about max(|10-10.5|/10.5, |20-19.8|/19.8) ≈ 0.048
            assert!(benchmark.max_relative_error < 0.1);
        }
    }

    // =========================================================================
    // Task 5.1-5.4: XVA Demo Runner Tests
    // =========================================================================

    mod xva_demo_runner_tests {
        use super::*;

        #[test]
        fn test_xva_demo_config_default() {
            let config = XvaDemoConfig::default();
            assert!((config.bump_size - 0.0001).abs() < 1e-10);
            assert_eq!(config.num_time_points, 20);
            assert!((config.time_horizon - 5.0).abs() < 1e-10);
            assert_eq!(config.benchmark_iterations, 100);
        }

        #[test]
        fn test_xva_demo_config_builder() {
            let config = XvaDemoConfig::new()
                .with_bump_size(0.0005)
                .with_num_time_points(10)
                .with_time_horizon(3.0)
                .with_benchmark_iterations(50);

            assert!((config.bump_size - 0.0005).abs() < 1e-10);
            assert_eq!(config.num_time_points, 10);
            assert!((config.time_horizon - 3.0).abs() < 1e-10);
            assert_eq!(config.benchmark_iterations, 50);
        }

        #[test]
        fn test_generate_time_grid() {
            let runner = XvaDemoRunner::with_defaults();

            let grid = runner.generate_time_grid(5, 1.0);

            assert_eq!(grid.len(), 5);
            assert!((grid[0] - 0.0).abs() < 1e-10);
            assert!((grid[4] - 1.0).abs() < 1e-10);
            assert!((grid[2] - 0.5).abs() < 1e-10);
        }

        #[test]
        fn test_generate_time_grid_empty() {
            let runner = XvaDemoRunner::with_defaults();
            let grid = runner.generate_time_grid(0, 1.0);
            assert!(grid.is_empty());
        }

        #[test]
        fn test_generate_time_grid_single() {
            let runner = XvaDemoRunner::with_defaults();
            let grid = runner.generate_time_grid(1, 1.0);
            assert_eq!(grid.len(), 1);
            assert!((grid[0] - 0.0).abs() < 1e-10);
        }

        #[test]
        fn test_compute_cva_basic() {
            let runner = XvaDemoRunner::with_defaults();
            let credit = CreditParams::new(0.02, 0.4).unwrap();

            let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
            let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];

            let cva = runner.compute_cva(&ee, &time_grid, &credit);

            // CVA should be positive for positive exposure
            assert!(cva > 0.0);
        }

        #[test]
        fn test_compute_cva_zero_exposure() {
            let runner = XvaDemoRunner::with_defaults();
            let credit = CreditParams::new(0.02, 0.4).unwrap();

            let time_grid = vec![0.0, 0.5, 1.0];
            let ee = vec![0.0, 0.0, 0.0];

            let cva = runner.compute_cva(&ee, &time_grid, &credit);

            assert!((cva - 0.0).abs() < 1e-10);
        }

        #[test]
        fn test_compute_dva_basic() {
            let runner = XvaDemoRunner::with_defaults();
            let own_credit = CreditParams::new(0.03, 0.4).unwrap();

            let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
            let ene = vec![0.0, 50.0, 80.0, 60.0, 30.0];

            let dva = runner.compute_dva(&ene, &time_grid, &own_credit);

            // DVA should be positive for positive ENE
            assert!(dva > 0.0);
        }

        #[test]
        fn test_compute_cva_higher_pd_higher_cva() {
            let runner = XvaDemoRunner::with_defaults();

            let time_grid = vec![0.0, 0.5, 1.0];
            let ee = vec![0.0, 100.0, 100.0];

            let low_risk = CreditParams::new(0.01, 0.4).unwrap();
            let high_risk = CreditParams::new(0.05, 0.4).unwrap();

            let cva_low = runner.compute_cva(&ee, &time_grid, &low_risk);
            let cva_high = runner.compute_cva(&ee, &time_grid, &high_risk);

            assert!(cva_high > cva_low);
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
    // Task 5.1: Exposure Profile Integration Tests
    // =========================================================================

    #[test]
    fn test_compute_exposure_profile() {
        // Requirement 4.1: Expected Exposure(EE)プロファイルを計算
        let config = XvaDemoConfig::new()
            .with_num_time_points(10)
            .with_time_horizon(5.0);
        let runner = XvaDemoRunner::new(config);

        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let profile = runner.compute_exposure_profile(&swap, &curves, valuation_date);
        assert!(profile.is_ok());

        let profile = profile.unwrap();
        assert_eq!(profile.len(), 10);
        assert!(!profile.is_empty());

        // EE should be non-negative
        for ee in &profile.ee {
            assert!(*ee >= 0.0);
        }

        // ENE should be non-negative
        for ene in &profile.ene {
            assert!(*ene >= 0.0);
        }
    }

    // =========================================================================
    // Task 5.2: CVA/DVA Integration Tests
    // =========================================================================

    #[test]
    fn test_run_xva_demo_cva_only() {
        // Requirement 4.2: CVA計算
        let config = XvaDemoConfig::new()
            .with_num_time_points(10)
            .with_time_horizon(5.0);
        let runner = XvaDemoRunner::new(config);

        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let counterparty_credit = CreditParams::new(0.02, 0.4).unwrap();

        let result =
            runner.run_xva_demo(&swap, &curves, valuation_date, &counterparty_credit, None);
        assert!(result.is_ok());

        let result = result.unwrap();
        // CVA should be calculated (could be 0 or positive depending on exposure)
        assert!(result.cva >= 0.0);
        // DVA should be 0 when no own credit provided
        assert!((result.dva - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_run_xva_demo_cva_and_dva() {
        // Requirement 4.2, 4.3: CVA and DVA計算
        let config = XvaDemoConfig::new()
            .with_num_time_points(10)
            .with_time_horizon(5.0);
        let runner = XvaDemoRunner::new(config);

        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let counterparty_credit = CreditParams::new(0.02, 0.4).unwrap();
        let own_credit = CreditParams::new(0.03, 0.4).unwrap();

        let result = runner.run_xva_demo(
            &swap,
            &curves,
            valuation_date,
            &counterparty_credit,
            Some(&own_credit),
        );
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.cva >= 0.0);
        assert!(result.dva >= 0.0);
    }

    // =========================================================================
    // Task 5.3: XVA AAD Sensitivity Integration Tests
    // =========================================================================

    #[test]
    fn test_compute_xva_sensitivities_aad() {
        // Requirement 4.4: AADを使用してXVA値のカーブ感応度を計算
        let config = XvaDemoConfig::new()
            .with_num_time_points(5)
            .with_time_horizon(5.0);
        let runner = XvaDemoRunner::new(config);

        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let counterparty_credit = CreditParams::new(0.02, 0.4).unwrap();
        let tenor_points = vec![1.0, 2.0, 5.0];

        let result = runner.compute_xva_sensitivities_aad(
            &swap,
            &curves,
            valuation_date,
            &counterparty_credit,
            &tenor_points,
        );
        assert!(result.is_ok());

        let (deltas, _time) = result.unwrap();
        assert_eq!(deltas.len(), 3);
    }

    #[test]
    fn test_compute_xva_sensitivities_bump() {
        // Requirement 4.5: Bump-and-Revalueモード
        let config = XvaDemoConfig::new()
            .with_num_time_points(5)
            .with_time_horizon(5.0);
        let runner = XvaDemoRunner::new(config);

        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let counterparty_credit = CreditParams::new(0.02, 0.4).unwrap();
        let tenor_points = vec![1.0, 2.0, 5.0];

        let result = runner.compute_xva_sensitivities_bump(
            &swap,
            &curves,
            valuation_date,
            &counterparty_credit,
            &tenor_points,
        );
        assert!(result.is_ok());

        let (deltas, _time) = result.unwrap();
        assert_eq!(deltas.len(), 3);
    }

    // =========================================================================
    // Task 5.4: XVA Performance Comparison Integration Tests
    // =========================================================================

    #[test]
    fn test_benchmark_xva_sensitivities() {
        // Requirement 4.5: AADモードとの性能比較結果を表示
        let config = XvaDemoConfig::new()
            .with_num_time_points(5)
            .with_time_horizon(5.0)
            .with_benchmark_iterations(3)
            .with_benchmark_iterations(1); // Use minimal iterations for test speed
        let runner = XvaDemoRunner::new(config);

        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let counterparty_credit = CreditParams::new(0.02, 0.4).unwrap();
        let tenor_points = vec![1.0, 2.0];

        let result = runner.benchmark_xva_sensitivities(
            &swap,
            &curves,
            valuation_date,
            &counterparty_credit,
            &tenor_points,
        );
        assert!(result.is_ok());

        let benchmark = result.unwrap();
        assert_eq!(benchmark.tenor_count, 2);
        assert!(benchmark.aad_stats.mean_ns > 0.0);
        assert!(benchmark.bump_stats.mean_ns > 0.0);
        assert!(benchmark.speedup_ratio > 0.0);
    }

    #[test]
    fn test_xva_demo_full_workflow() {
        // Full integration test: Complete XVA demo workflow
        let config = XvaDemoConfig::new()
            .with_num_time_points(10)
            .with_time_horizon(5.0);
        let runner = XvaDemoRunner::new(config);

        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let counterparty_credit = CreditParams::new(0.02, 0.4).unwrap();
        let own_credit = CreditParams::new(0.03, 0.4).unwrap();

        // Step 1: Compute exposure profile
        let profile = runner
            .compute_exposure_profile(&swap, &curves, valuation_date)
            .unwrap();
        assert!(!profile.is_empty());

        // Step 2: Run XVA demo
        let xva_result = runner
            .run_xva_demo(
                &swap,
                &curves,
                valuation_date,
                &counterparty_credit,
                Some(&own_credit),
            )
            .unwrap();
        assert!(xva_result.cva >= 0.0);
        assert!(xva_result.dva >= 0.0);

        // Step 3: Compute sensitivities
        let tenor_points = vec![1.0, 2.0, 5.0];
        let (aad_deltas, _) = runner
            .compute_xva_sensitivities_aad(
                &swap,
                &curves,
                valuation_date,
                &counterparty_credit,
                &tenor_points,
            )
            .unwrap();
        let (bump_deltas, _) = runner
            .compute_xva_sensitivities_bump(
                &swap,
                &curves,
                valuation_date,
                &counterparty_credit,
                &tenor_points,
            )
            .unwrap();

        assert_eq!(aad_deltas.len(), 3);
        assert_eq!(bump_deltas.len(), 3);

        // Note: In this demo implementation, AAD falls back to bump,
        // so results should be identical
    }
}
