//! IRS AAD Demo Workflow implementation.
//!
//! This module provides the IRS AAD (Adjoint Algorithmic Differentiation) demo
//! workflow for demonstrating the performance advantages of AAD over traditional
//! Bump-and-Revalue methods.
//!
//! # Task Coverage
//!
//! - Task 6.1: IrsAadWorkflow implementation
//!   - DemoWorkflow trait implementation
//!   - IRS parameter input processing
//!   - Calculation mode selection (AAD/Bump-and-Revalue)
//!   - Single calculation and benchmark execution modes
//!
//! # Requirements Coverage
//!
//! - Requirement 6.1: DemoWorkflow trait integration
//! - Requirement 6.2: IRS parameter input processing
//! - Requirement 6.5: Execution mode selection
//!
//! # Usage
//!
//! ```rust,ignore
//! use frictional_bank::workflow::IrsAadWorkflow;
//! use frictional_bank::config::DemoConfig;
//! use pricer_pricing::greeks::GreeksMode;
//!
//! let workflow = IrsAadWorkflow::new(IrsAadConfig::default());
//!
//! // Single calculation
//! let result = workflow.compute_single(&params, GreeksMode::BumpRevalue).await?;
//!
//! // Benchmark execution
//! let benchmark = workflow.run_benchmark(&params).await?;
//! ```

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::config::DemoConfig;
use crate::error::DemoError;
use crate::workflow::{DemoWorkflow, ProgressCallback, WorkflowResult, WorkflowStep};

#[cfg(feature = "l1l2-integration")]
use pricer_core::market_data::curves::{CurveEnum, CurveName, CurveSet};
#[cfg(feature = "l1l2-integration")]
use pricer_core::types::time::{Date, DayCountConvention};
#[cfg(feature = "l1l2-integration")]
use pricer_core::types::Currency;
#[cfg(feature = "l1l2-integration")]
use pricer_models::instruments::rates::{
    FixedLeg, FloatingLeg, InterestRateSwap, RateIndex, SwapDirection,
};
#[cfg(feature = "l1l2-integration")]
use pricer_models::schedules::{Frequency, ScheduleBuilder};

#[cfg(feature = "l1l2-integration")]
use pricer_pricing::greeks::GreeksMode;
#[cfg(feature = "l1l2-integration")]
use pricer_pricing::irs_greeks::xva_demo::{CreditParams, XvaDemoConfig, XvaDemoRunner};
#[cfg(feature = "l1l2-integration")]
use pricer_pricing::irs_greeks::{
    BenchmarkConfig, BenchmarkRunner, DeltaBenchmarkResult, IrsGreeksCalculator, IrsGreeksConfig,
    IrsLazyEvaluator,
};

// =============================================================================
// Stub types for non-l1l2-integration builds
// =============================================================================

#[cfg(not(feature = "l1l2-integration"))]
#[allow(dead_code)]
mod stub_types {
    //! Stub types when pricer_pricing is not available.

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub enum GreeksMode {
        #[default]
        BumpRevalue,
        EnzymeAAD,
    }

    #[derive(Clone, Debug)]
    pub struct BenchmarkConfig {
        pub iterations: usize,
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

    #[derive(Clone, Debug)]
    pub struct IrsGreeksConfig {
        pub bump_size: f64,
    }

    impl Default for IrsGreeksConfig {
        fn default() -> Self {
            Self { bump_size: 0.0001 }
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct TimingStats {
        pub mean_ns: f64,
    }

    #[derive(Clone, Debug, Default)]
    pub struct DeltaBenchmarkResult {
        pub speedup_ratio: f64,
        pub aad_stats: TimingStats,
        pub bump_stats: TimingStats,
    }

    pub struct IrsGreeksCalculator<T>(std::marker::PhantomData<T>);

    impl<T> IrsGreeksCalculator<T> {
        pub fn new(_config: IrsGreeksConfig) -> Self {
            Self(std::marker::PhantomData)
        }
    }

    pub struct IrsLazyEvaluator<T>(std::marker::PhantomData<T>);

    impl<T> IrsLazyEvaluator<T> {
        pub fn new() -> Self {
            Self(std::marker::PhantomData)
        }
    }

    impl<T> Default for IrsLazyEvaluator<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    pub struct BenchmarkRunner;

    impl BenchmarkRunner {
        pub fn new(_config: BenchmarkConfig) -> Self {
            Self
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct XvaDemoConfig;

    impl XvaDemoConfig {
        pub fn new() -> Self {
            Self
        }
        pub fn with_num_time_points(self, _n: usize) -> Self {
            self
        }
        pub fn with_benchmark_iterations(self, _n: usize) -> Self {
            self
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct CreditParams;

    pub struct XvaDemoRunner;

    impl XvaDemoRunner {
        pub fn new(_config: XvaDemoConfig) -> Self {
            Self
        }
    }
}

#[cfg(not(feature = "l1l2-integration"))]
use stub_types::*;

// =============================================================================
// IRS AAD Configuration
// =============================================================================

/// Configuration for IRS AAD workflow.
#[derive(Clone, Debug, Default)]
pub struct IrsAadConfig {
    /// Greeks calculator configuration.
    pub greeks_config: IrsGreeksConfig,
    /// Benchmark runner configuration.
    pub benchmark_config: BenchmarkConfig,
    /// XVA demo configuration.
    #[allow(clippy::default_constructed_unit_structs)]
    pub xva_config: XvaDemoConfig,
}

impl IrsAadConfig {
    /// Creates a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the Greeks calculator configuration.
    pub fn with_greeks_config(mut self, config: IrsGreeksConfig) -> Self {
        self.greeks_config = config;
        self
    }

    /// Sets the benchmark configuration.
    pub fn with_benchmark_config(mut self, config: BenchmarkConfig) -> Self {
        self.benchmark_config = config;
        self
    }

    /// Sets the XVA demo configuration.
    pub fn with_xva_config(mut self, config: XvaDemoConfig) -> Self {
        self.xva_config = config;
        self
    }
}

// =============================================================================
// IRS Parameters
// =============================================================================

/// Input parameters for IRS construction.
///
/// # Requirements Coverage
///
/// - Requirement 6.2: IRS parameter input processing
#[derive(Clone, Debug)]
pub struct IrsParams {
    /// Notional amount.
    pub notional: f64,
    /// Fixed rate (annualised).
    pub fixed_rate: f64,
    /// Start date.
    pub start_date_ymd: (i32, u32, u32),
    /// End date.
    pub end_date_ymd: (i32, u32, u32),
    /// Currency.
    pub currency: String,
    /// Swap direction (pay or receive fixed).
    pub pay_fixed: bool,
    /// Rate index name (e.g., "SOFR", "EURIBOR").
    pub rate_index: String,
}

impl Default for IrsParams {
    fn default() -> Self {
        Self {
            notional: 1_000_000.0,
            fixed_rate: 0.03,
            start_date_ymd: (2024, 1, 15),
            end_date_ymd: (2029, 1, 15),
            currency: "USD".to_string(),
            pay_fixed: true,
            rate_index: "SOFR".to_string(),
        }
    }
}

impl IrsParams {
    /// Creates new IRS parameters.
    pub fn new(
        notional: f64,
        fixed_rate: f64,
        start_date_ymd: (i32, u32, u32),
        end_date_ymd: (i32, u32, u32),
    ) -> Self {
        Self {
            notional,
            fixed_rate,
            start_date_ymd,
            end_date_ymd,
            ..Default::default()
        }
    }

    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), DemoError> {
        if self.notional <= 0.0 {
            return Err(DemoError::Validation(
                "Notional must be positive".to_string(),
            ));
        }

        let (sy, sm, sd) = self.start_date_ymd;
        let (ey, em, ed) = self.end_date_ymd;

        // Simple date comparison
        if (sy, sm, sd) >= (ey, em, ed) {
            return Err(DemoError::Validation(
                "Start date must be before end date".to_string(),
            ));
        }

        Ok(())
    }
}

// =============================================================================
// IRS Compute Result
// =============================================================================

/// Result of a single IRS Greeks computation.
///
/// # Requirements Coverage
///
/// - Requirement 6.5: Execution mode selection
#[derive(Clone, Debug)]
pub struct IrsComputeResult {
    /// Net Present Value.
    pub npv: f64,
    /// DV01 (1bp parallel shift sensitivity).
    pub dv01: f64,
    /// Tenor delta values.
    pub tenor_deltas: Vec<f64>,
    /// Tenor points.
    pub tenors: Vec<f64>,
    /// Computation time in nanoseconds.
    pub compute_time_ns: u64,
    /// Calculation mode used.
    pub mode: GreeksMode,
}

impl IrsComputeResult {
    /// Creates a new compute result.
    pub fn new(npv: f64, dv01: f64, mode: GreeksMode) -> Self {
        Self {
            npv,
            dv01,
            tenor_deltas: Vec::new(),
            tenors: Vec::new(),
            compute_time_ns: 0,
            mode,
        }
    }

    /// Sets the tenor deltas.
    pub fn with_deltas(mut self, tenors: Vec<f64>, deltas: Vec<f64>) -> Self {
        self.tenors = tenors;
        self.tenor_deltas = deltas;
        self
    }

    /// Sets the computation time.
    pub fn with_time(mut self, time_ns: u64) -> Self {
        self.compute_time_ns = time_ns;
        self
    }
}

// =============================================================================
// XVA Demo Result
// =============================================================================

/// Result of XVA demo computation.
#[derive(Clone, Debug)]
pub struct XvaDemoResult {
    /// Credit Valuation Adjustment.
    pub cva: f64,
    /// Debit Valuation Adjustment.
    pub dva: f64,
    /// CVA sensitivities (deltas).
    pub cva_deltas: Vec<f64>,
    /// AAD computation time in nanoseconds.
    pub aad_time_ns: u64,
    /// Bump-and-Revalue computation time in nanoseconds.
    pub bump_time_ns: u64,
    /// Speedup ratio (bump / AAD).
    pub speedup_ratio: f64,
}

impl Default for XvaDemoResult {
    fn default() -> Self {
        Self {
            cva: 0.0,
            dva: 0.0,
            cva_deltas: Vec::new(),
            aad_time_ns: 0,
            bump_time_ns: 0,
            speedup_ratio: 1.0,
        }
    }
}

// =============================================================================
// IRS AAD Workflow
// =============================================================================

/// IRS AAD Demo Workflow.
///
/// Implements the DemoWorkflow trait to integrate with FrictionalBank demo system.
/// Provides IRS Greeks calculation using AAD and Bump-and-Revalue methods
/// with performance benchmarking.
///
/// # Requirements Coverage
///
/// - Requirement 6.1: DemoWorkflow trait implementation
/// - Requirement 6.2: IRS parameter input processing
/// - Requirement 6.5: Execution mode selection (AAD/Bump-and-Revalue)
///
/// # Example
///
/// ```rust,ignore
/// use frictional_bank::workflow::IrsAadWorkflow;
///
/// let workflow = IrsAadWorkflow::new(IrsAadConfig::default());
///
/// // Run via DemoWorkflow trait
/// let result = workflow.run(&config, None).await?;
///
/// // Or run single calculation
/// let result = workflow.compute_single(&params, GreeksMode::BumpRevalue).await?;
/// ```
pub struct IrsAadWorkflow {
    /// Calculator for IRS Greeks.
    calculator: IrsGreeksCalculator<f64>,
    /// Lazy evaluator for caching.
    lazy_evaluator: IrsLazyEvaluator<f64>,
    /// Benchmark runner for performance comparison.
    benchmark_runner: BenchmarkRunner,
    /// XVA demo runner.
    #[allow(dead_code)]
    xva_runner: XvaDemoRunner,
    /// Cancellation flag.
    cancelled: Arc<AtomicBool>,
    /// Configuration.
    config: IrsAadConfig,
}

impl IrsAadWorkflow {
    /// Creates a new IRS AAD workflow.
    ///
    /// # Arguments
    ///
    /// * `config` - Workflow configuration
    ///
    /// # Returns
    ///
    /// New IrsAadWorkflow instance.
    pub fn new(config: IrsAadConfig) -> Self {
        Self {
            calculator: IrsGreeksCalculator::new(config.greeks_config.clone()),
            lazy_evaluator: IrsLazyEvaluator::new(),
            benchmark_runner: BenchmarkRunner::new(config.benchmark_config.clone()),
            xva_runner: XvaDemoRunner::new(config.xva_config.clone()),
            cancelled: Arc::new(AtomicBool::new(false)),
            config,
        }
    }

    /// Creates a workflow with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(IrsAadConfig::default())
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &IrsAadConfig {
        &self.config
    }

    /// Returns the calculator reference.
    pub fn calculator(&self) -> &IrsGreeksCalculator<f64> {
        &self.calculator
    }

    /// Returns the benchmark runner reference.
    pub fn benchmark_runner(&self) -> &BenchmarkRunner {
        &self.benchmark_runner
    }

    /// Returns the lazy evaluator reference.
    pub fn lazy_evaluator(&self) -> &IrsLazyEvaluator<f64> {
        &self.lazy_evaluator
    }

    /// Checks if the workflow has been cancelled.
    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Resets the cancellation flag.
    pub fn reset_cancellation(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }
}

#[cfg(feature = "l1l2-integration")]
impl IrsAadWorkflow {
    /// Builds an InterestRateSwap from parameters.
    ///
    /// # Arguments
    ///
    /// * `params` - IRS parameters
    ///
    /// # Returns
    ///
    /// InterestRateSwap instance or error.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 6.2: IRS parameter input processing
    pub fn build_swap(&self, params: &IrsParams) -> Result<InterestRateSwap<f64>, DemoError> {
        params.validate()?;

        let (sy, sm, sd) = params.start_date_ymd;
        let (ey, em, ed) = params.end_date_ymd;

        let start = Date::from_ymd(sy, sm, sd)
            .map_err(|e| DemoError::Validation(format!("Invalid start date: {}", e)))?;
        let end = Date::from_ymd(ey, em, ed)
            .map_err(|e| DemoError::Validation(format!("Invalid end date: {}", e)))?;

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::SemiAnnual)
            .day_count(DayCountConvention::Thirty360)
            .build()
            .map_err(|e| DemoError::Validation(format!("Schedule build error: {}", e)))?;

        let fixed_leg = FixedLeg::new(
            schedule.clone(),
            params.fixed_rate,
            DayCountConvention::Thirty360,
        );

        let rate_index = match params.rate_index.to_uppercase().as_str() {
            "SOFR" => RateIndex::Sofr,
            "EURIBOR" | "EURIBOR_6M" => RateIndex::Euribor6M,
            "EURIBOR_3M" => RateIndex::Euribor3M,
            "SONIA" => RateIndex::Sonia,
            "TONAR" => RateIndex::Tonar,
            _ => RateIndex::Sofr, // Default to SOFR
        };

        let floating_leg = FloatingLeg::new(
            schedule,
            0.0, // spread
            rate_index,
            DayCountConvention::ActualActual360,
        );

        let currency = match params.currency.to_uppercase().as_str() {
            "USD" => Currency::USD,
            "EUR" => Currency::EUR,
            "GBP" => Currency::GBP,
            "JPY" => Currency::JPY,
            _ => Currency::USD, // Default to USD
        };

        let direction = if params.pay_fixed {
            SwapDirection::PayFixed
        } else {
            SwapDirection::ReceiveFixed
        };

        Ok(InterestRateSwap::new(
            params.notional,
            fixed_leg,
            floating_leg,
            currency,
            direction,
        ))
    }

    /// Creates default curve set for demo.
    pub fn create_demo_curves(&self) -> CurveSet<f64> {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Discount, CurveEnum::flat(0.03));
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035));
        curves.set_discount_curve(CurveName::Discount);
        curves
    }

    /// Computes a single IRS Greeks calculation.
    ///
    /// # Arguments
    ///
    /// * `params` - IRS parameters
    /// * `mode` - Calculation mode (AAD or Bump-and-Revalue)
    ///
    /// # Returns
    ///
    /// IrsComputeResult with NPV, DV01, and deltas.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 6.5: Execution mode selection
    pub async fn compute_single(
        &self,
        params: &IrsParams,
        mode: GreeksMode,
    ) -> Result<IrsComputeResult, DemoError> {
        if self.is_cancelled() {
            return Err(DemoError::Cancelled);
        }

        let swap = self.build_swap(params)?;
        let curves = self.create_demo_curves();
        let (sy, sm, sd) = params.start_date_ymd;
        let valuation_date = Date::from_ymd(sy, sm, sd)
            .map_err(|e| DemoError::Validation(format!("Invalid valuation date: {}", e)))?;

        let start_time = Instant::now();

        // Compute NPV
        let npv = self
            .calculator
            .compute_npv(&swap, &curves, valuation_date)
            .map_err(|e| DemoError::Computation(format!("NPV calculation error: {}", e)))?;

        // Compute DV01
        let dv01 = self
            .calculator
            .compute_dv01(&swap, &curves, valuation_date)
            .map_err(|e| DemoError::Computation(format!("DV01 calculation error: {}", e)))?;

        // Compute tenor deltas
        let tenor_points = vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
        let bump_size = self.config.greeks_config.bump_size;
        let delta_result = match mode {
            GreeksMode::BumpRevalue => self
                .calculator
                .compute_tenor_deltas_bump(&swap, &curves, valuation_date, &tenor_points, bump_size)
                .map_err(|e| DemoError::Computation(format!("Delta calculation error: {}", e)))?,
            #[cfg(feature = "enzyme-ad")]
            GreeksMode::EnzymeAAD => self
                .calculator
                .compute_tenor_deltas_aad(&swap, &curves, valuation_date, &tenor_points)
                .map_err(|e| DemoError::Computation(format!("AAD calculation error: {}", e)))?,
            _ => self
                .calculator
                .compute_tenor_deltas_bump(&swap, &curves, valuation_date, &tenor_points, bump_size)
                .map_err(|e| DemoError::Computation(format!("Delta calculation error: {}", e)))?,
        };

        let compute_time = start_time.elapsed().as_nanos() as u64;

        Ok(IrsComputeResult::new(npv, dv01, mode)
            .with_deltas(delta_result.tenors, delta_result.deltas)
            .with_time(compute_time))
    }

    /// Runs the benchmark comparing AAD and Bump-and-Revalue.
    ///
    /// # Arguments
    ///
    /// * `params` - IRS parameters
    ///
    /// # Returns
    ///
    /// DeltaBenchmarkResult with timing statistics and speedup ratio.
    pub async fn run_benchmark(
        &self,
        params: &IrsParams,
    ) -> Result<DeltaBenchmarkResult, DemoError> {
        if self.is_cancelled() {
            return Err(DemoError::Cancelled);
        }

        let swap = self.build_swap(params)?;
        let curves = self.create_demo_curves();
        let (sy, sm, sd) = params.start_date_ymd;
        let valuation_date = Date::from_ymd(sy, sm, sd)
            .map_err(|e| DemoError::Validation(format!("Invalid valuation date: {}", e)))?;

        let tenor_points = vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];

        let result = self
            .benchmark_runner
            .benchmark_deltas(&swap, &curves, valuation_date, &tenor_points)
            .map_err(|e| DemoError::Computation(format!("Benchmark error: {}", e)))?;

        Ok(result)
    }

    /// Runs the XVA demo with CVA/DVA calculation.
    ///
    /// # Arguments
    ///
    /// * `params` - IRS parameters
    /// * `credit_params` - Credit parameters for counterparty
    ///
    /// # Returns
    ///
    /// XvaDemoResult with CVA, DVA, and sensitivity information.
    pub async fn run_xva_demo(
        &self,
        params: &IrsParams,
        credit_params: &CreditParams,
    ) -> Result<XvaDemoResult, DemoError> {
        if self.is_cancelled() {
            return Err(DemoError::Cancelled);
        }

        let swap = self.build_swap(params)?;
        let curves = self.create_demo_curves();
        let (sy, sm, sd) = params.start_date_ymd;
        let valuation_date = Date::from_ymd(sy, sm, sd)
            .map_err(|e| DemoError::Validation(format!("Invalid valuation date: {}", e)))?;

        let xva_result = self
            .xva_runner
            .run_xva_demo(&swap, &curves, valuation_date, credit_params, None)
            .map_err(|e| DemoError::Computation(format!("XVA calculation error: {}", e)))?;

        // Run sensitivity benchmark
        let tenor_points = vec![1.0, 2.0, 5.0];
        let benchmark = self
            .xva_runner
            .benchmark_xva_sensitivities(
                &swap,
                &curves,
                valuation_date,
                credit_params,
                &tenor_points,
            )
            .map_err(|e| DemoError::Computation(format!("XVA benchmark error: {}", e)))?;

        Ok(XvaDemoResult {
            cva: xva_result.cva,
            dva: xva_result.dva,
            cva_deltas: benchmark.aad_cva_deltas,
            aad_time_ns: benchmark.aad_stats.mean_ns as u64,
            bump_time_ns: benchmark.bump_stats.mean_ns as u64,
            speedup_ratio: benchmark.speedup_ratio,
        })
    }
}

// =============================================================================
// DemoWorkflow Implementation
// =============================================================================

#[async_trait]
impl DemoWorkflow for IrsAadWorkflow {
    /// Returns the workflow name.
    fn name(&self) -> &str {
        "IRS AAD Demo"
    }

    /// Executes the workflow.
    ///
    /// # Arguments
    ///
    /// * `config` - Demo configuration
    /// * `progress` - Optional progress callback
    ///
    /// # Returns
    ///
    /// WorkflowResult with execution status.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 6.1: DemoWorkflow trait implementation
    #[cfg(feature = "l1l2-integration")]
    async fn run(
        &self,
        _config: &DemoConfig,
        progress: Option<ProgressCallback>,
    ) -> Result<WorkflowResult, DemoError> {
        self.reset_cancellation();
        let start_time = Instant::now();

        // Report progress: Loading
        if let Some(ref cb) = progress {
            cb(WorkflowStep::LoadingMarketData, 0.1);
        }

        // Create default parameters and curves
        let params = IrsParams::default();
        let swap = self.build_swap(&params)?;
        let curves = self.create_demo_curves();
        let (sy, sm, sd) = params.start_date_ymd;
        let valuation_date = Date::from_ymd(sy, sm, sd)
            .map_err(|e| DemoError::Validation(format!("Invalid valuation date: {}", e)))?;

        if self.is_cancelled() {
            return Err(DemoError::Cancelled);
        }

        // Report progress: Pricing
        if let Some(ref cb) = progress {
            cb(WorkflowStep::Pricing, 0.3);
        }

        // Compute NPV
        let npv = self
            .calculator
            .compute_npv(&swap, &curves, valuation_date)
            .map_err(|e| DemoError::Computation(format!("NPV calculation error: {}", e)))?;

        // Compute DV01
        let dv01 = self
            .calculator
            .compute_dv01(&swap, &curves, valuation_date)
            .map_err(|e| DemoError::Computation(format!("DV01 calculation error: {}", e)))?;

        if self.is_cancelled() {
            return Err(DemoError::Cancelled);
        }

        // Report progress: Running benchmark
        if let Some(ref cb) = progress {
            cb(WorkflowStep::CalculatingXva, 0.6);
        }

        // Run benchmark
        let tenor_points = vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
        let benchmark = self
            .benchmark_runner
            .benchmark_deltas(&swap, &curves, valuation_date, &tenor_points)
            .map_err(|e| DemoError::Computation(format!("Benchmark error: {}", e)))?;

        if self.is_cancelled() {
            return Err(DemoError::Cancelled);
        }

        // Report progress: Completed
        if let Some(ref cb) = progress {
            cb(WorkflowStep::Completed, 1.0);
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Log summary (in production, this would go to a proper logger)
        tracing::info!(
            "IRS AAD Demo completed: NPV={:.2}, DV01={:.4}, Speedup={:.2}x",
            npv,
            dv01,
            benchmark.speedup_ratio
        );

        Ok(WorkflowResult::success(duration_ms, 1))
    }

    /// Fallback run for non-l1l2-integration builds.
    #[cfg(not(feature = "l1l2-integration"))]
    async fn run(
        &self,
        _config: &DemoConfig,
        progress: Option<ProgressCallback>,
    ) -> Result<WorkflowResult, DemoError> {
        self.reset_cancellation();
        let start_time = Instant::now();

        if let Some(ref cb) = progress {
            cb(WorkflowStep::Completed, 1.0);
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(WorkflowResult::success(duration_ms, 0)
            .with_error("l1l2-integration feature not enabled"))
    }

    /// Cancels the workflow.
    async fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 6.1: IrsAadConfig Tests
    // =========================================================================

    mod config_tests {
        use super::*;

        #[test]
        fn test_irs_aad_config_default() {
            let config = IrsAadConfig::default();
            assert!((config.greeks_config.bump_size - 0.0001).abs() < 1e-10);
            assert_eq!(config.benchmark_config.iterations, 100);
        }

        #[test]
        fn test_irs_aad_config_builder() {
            let greeks_config = IrsGreeksConfig {
                bump_size: 0.0005,
                ..Default::default()
            };

            let config = IrsAadConfig::new().with_greeks_config(greeks_config.clone());

            assert!((config.greeks_config.bump_size - 0.0005).abs() < 1e-10);
        }
    }

    // =========================================================================
    // Task 6.1: IrsParams Tests
    // =========================================================================

    mod params_tests {
        use super::*;

        #[test]
        fn test_irs_params_default() {
            let params = IrsParams::default();
            assert!((params.notional - 1_000_000.0).abs() < 1e-10);
            assert!((params.fixed_rate - 0.03).abs() < 1e-10);
            assert!(params.pay_fixed);
        }

        #[test]
        fn test_irs_params_validate_success() {
            let params = IrsParams::default();
            assert!(params.validate().is_ok());
        }

        #[test]
        fn test_irs_params_validate_negative_notional() {
            let params = IrsParams {
                notional: -1000.0,
                ..Default::default()
            };
            let result = params.validate();
            assert!(result.is_err());
            if let Err(DemoError::Validation(msg)) = result {
                assert!(msg.contains("Notional"));
            }
        }

        #[test]
        fn test_irs_params_validate_invalid_dates() {
            let params = IrsParams {
                start_date_ymd: (2029, 1, 15),
                end_date_ymd: (2024, 1, 15),
                ..Default::default()
            };
            let result = params.validate();
            assert!(result.is_err());
            if let Err(DemoError::Validation(msg)) = result {
                assert!(msg.contains("date"));
            }
        }
    }

    // =========================================================================
    // Task 6.1: IrsComputeResult Tests
    // =========================================================================

    mod compute_result_tests {
        use super::*;

        #[test]
        fn test_irs_compute_result_new() {
            let result = IrsComputeResult::new(1000.0, 0.05, GreeksMode::BumpRevalue);
            assert!((result.npv - 1000.0).abs() < 1e-10);
            assert!((result.dv01 - 0.05).abs() < 1e-10);
            assert_eq!(result.mode, GreeksMode::BumpRevalue);
        }

        #[test]
        fn test_irs_compute_result_with_deltas() {
            let result = IrsComputeResult::new(1000.0, 0.05, GreeksMode::BumpRevalue)
                .with_deltas(vec![1.0, 2.0, 5.0], vec![100.0, 200.0, 500.0]);

            assert_eq!(result.tenors.len(), 3);
            assert_eq!(result.tenor_deltas.len(), 3);
            assert!((result.tenors[0] - 1.0).abs() < 1e-10);
            assert!((result.tenor_deltas[0] - 100.0).abs() < 1e-10);
        }

        #[test]
        fn test_irs_compute_result_with_time() {
            let result =
                IrsComputeResult::new(1000.0, 0.05, GreeksMode::BumpRevalue).with_time(12345678);

            assert_eq!(result.compute_time_ns, 12345678);
        }
    }

    // =========================================================================
    // Task 6.1: IrsAadWorkflow Tests
    // =========================================================================

    mod workflow_tests {
        use super::*;

        #[test]
        fn test_workflow_creation() {
            let workflow = IrsAadWorkflow::with_defaults();
            assert_eq!(workflow.name(), "IRS AAD Demo");
        }

        #[test]
        fn test_workflow_config_access() {
            let config = IrsAadConfig::default();
            let workflow = IrsAadWorkflow::new(config.clone());
            assert!((workflow.config().greeks_config.bump_size - 0.0001).abs() < 1e-10);
        }

        #[test]
        fn test_workflow_cancellation() {
            let workflow = IrsAadWorkflow::with_defaults();

            assert!(!workflow.is_cancelled());

            // Simulate cancellation
            workflow.cancelled.store(true, Ordering::SeqCst);
            assert!(workflow.is_cancelled());

            // Reset
            workflow.reset_cancellation();
            assert!(!workflow.is_cancelled());
        }

        #[test]
        fn test_workflow_component_access() {
            let workflow = IrsAadWorkflow::with_defaults();

            // Verify components are accessible
            let _calculator = workflow.calculator();
            let _benchmark_runner = workflow.benchmark_runner();
            let _lazy_evaluator = workflow.lazy_evaluator();
        }
    }

    // =========================================================================
    // Task 6.1: XvaDemoResult Tests
    // =========================================================================

    mod xva_result_tests {
        use super::*;

        #[test]
        fn test_xva_demo_result_default() {
            let result = XvaDemoResult::default();
            assert!((result.cva - 0.0).abs() < 1e-10);
            assert!((result.dva - 0.0).abs() < 1e-10);
            assert!(result.cva_deltas.is_empty());
            assert!((result.speedup_ratio - 1.0).abs() < 1e-10);
        }
    }

    // =========================================================================
    // Integration Tests (with l1l2-integration feature)
    // =========================================================================

    #[cfg(feature = "l1l2-integration")]
    mod integration_tests {
        use super::*;

        #[test]
        fn test_build_swap() {
            let workflow = IrsAadWorkflow::with_defaults();
            let params = IrsParams::default();

            let result = workflow.build_swap(&params);
            assert!(result.is_ok());

            let swap = result.unwrap();
            assert!((swap.notional() - 1_000_000.0).abs() < 1e-10);
        }

        #[test]
        fn test_build_swap_invalid_params() {
            let workflow = IrsAadWorkflow::with_defaults();
            let params = IrsParams {
                notional: -1000.0,
                ..Default::default()
            };

            let result = workflow.build_swap(&params);
            assert!(result.is_err());
        }

        #[test]
        fn test_create_demo_curves() {
            let workflow = IrsAadWorkflow::with_defaults();
            let curves = workflow.create_demo_curves();

            assert!(curves.discount_curve().is_some());
        }

        #[tokio::test]
        async fn test_compute_single_bump_mode() {
            let workflow = IrsAadWorkflow::with_defaults();
            let params = IrsParams::default();

            let result = workflow
                .compute_single(&params, GreeksMode::BumpRevalue)
                .await;
            assert!(result.is_ok());

            let compute_result = result.unwrap();
            assert_eq!(compute_result.mode, GreeksMode::BumpRevalue);
            assert!(compute_result.compute_time_ns > 0);
        }

        #[tokio::test]
        async fn test_run_benchmark() {
            let config = IrsAadConfig::default().with_benchmark_config(BenchmarkConfig {
                iterations: 3,
                warmup: 1,
            });
            let workflow = IrsAadWorkflow::new(config);
            let params = IrsParams::default();

            let result = workflow.run_benchmark(&params).await;
            assert!(result.is_ok());

            let benchmark = result.unwrap();
            assert!(benchmark.speedup_ratio > 0.0);
        }

        #[tokio::test]
        async fn test_workflow_run() {
            let workflow = IrsAadWorkflow::with_defaults();
            let config = DemoConfig::default();

            let result = workflow.run(&config, None).await;
            assert!(result.is_ok());

            let workflow_result = result.unwrap();
            assert!(workflow_result.success);
            assert_eq!(workflow_result.trades_processed, 1);
        }

        #[tokio::test]
        async fn test_workflow_cancellation_flag() {
            let workflow = IrsAadWorkflow::with_defaults();

            // Initially not cancelled
            assert!(!workflow.is_cancelled());

            // Cancel
            workflow.cancel().await;
            assert!(workflow.is_cancelled());

            // Reset
            workflow.reset_cancellation();
            assert!(!workflow.is_cancelled());
        }

        #[tokio::test]
        async fn test_run_xva_demo() {
            let config = IrsAadConfig::default().with_xva_config(
                XvaDemoConfig::new()
                    .with_num_time_points(5)
                    .with_benchmark_iterations(2),
            );
            let workflow = IrsAadWorkflow::new(config);
            let params = IrsParams::default();
            let credit_params = CreditParams::default();

            let result = workflow.run_xva_demo(&params, &credit_params).await;
            assert!(result.is_ok());

            let xva_result = result.unwrap();
            assert!(xva_result.cva >= 0.0);
        }
    }
}
