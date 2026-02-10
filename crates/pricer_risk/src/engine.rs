//! Risk engine facade.
//!
//! Provides [`RiskEngine`] as the unified entry point for risk calculations,
//! scenario analysis, and portfolio operations.
//!
//! # Overview
//!
//! The `RiskEngine` serves as the **single facade** for all risk-related
//! computations:
//!
//! - **Greeks calculation**: Single trade and portfolio-level Greeks
//! - **Scenario analysis**: Stress testing and P&L computation
//! - **Portfolio operations**: Pricing, aggregation, XVA
//!
//! # Requirements
//!
//! - Requirement 5.1: RiskEngine facade
//! - Requirement 5.2: compute_greeks() method
//! - Requirement 5.3: AAD/Bump mode selection
//! - Requirement 5.4: Risk factor identification

use std::{collections::HashMap, time::Instant};

use infra_config::{GreekType, GreeksMethod, RiskConfig, SecondOrderMode};
use rayon::prelude::*;

use crate::{
    error::RiskError,
    greeks::{GreeksConfig, GreeksMode, GreeksResult},
    portfolio::{NettingSetId, Portfolio, Trade, TradeId},
    result::{
        ComputedGreeks, FailedCalculation, PerformanceMetrics, PortfolioRiskResult, RiskResult,
    },
    scenarios::{Scenario, ScenarioEngine, ScenarioResult},
};

/// Configuration for the RiskEngine.
#[derive(Debug, Clone)]
pub struct RiskEngineConfig {
    /// Base risk configuration.
    pub risk_config: RiskConfig,
    /// Parallel threshold (number of trades to trigger parallel processing).
    pub parallel_threshold: usize,
    /// Batch size for parallel processing.
    pub batch_size: usize,
    /// Continue processing on individual trade failures.
    pub continue_on_error: bool,
}

impl Default for RiskEngineConfig {
    fn default() -> Self {
        Self {
            risk_config: RiskConfig::default(),
            parallel_threshold: 100,
            batch_size: 64,
            continue_on_error: true,
        }
    }
}

impl RiskEngineConfig {
    /// Creates a new RiskEngineConfig with the given RiskConfig.
    pub fn new(risk_config: RiskConfig) -> Self {
        Self {
            risk_config,
            ..Default::default()
        }
    }

    /// Sets the parallel threshold.
    pub fn with_parallel_threshold(mut self, threshold: usize) -> Self {
        self.parallel_threshold = threshold;
        self
    }

    /// Sets the batch size.
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.max(1);
        self
    }

    /// Sets whether to continue on error.
    pub fn with_continue_on_error(mut self, continue_on_error: bool) -> Self {
        self.continue_on_error = continue_on_error;
        self
    }
}

/// Risk calculation engine facade.
///
/// Provides unified access to all risk operations:
/// - **Greeks computation**: AAD (Enzyme) or Bump-and-Revalue
/// - **Scenario analysis**: Stress testing and P&L computation
/// - **Portfolio operations**: Pricing, aggregation by netting set
///
/// # Architecture
///
/// `RiskEngine` follows the **Facade pattern**, internally delegating to:
/// - `ScenarioEngine` for scenario execution
/// - `Portfolio` methods for portfolio-level operations
///
/// # Example
///
/// ```rust,ignore
/// use pricer_risk::{RiskEngine, RiskEngineConfig, Portfolio};
/// use infra_config::RiskConfig;
///
/// let config = RiskEngineConfig::new(RiskConfig::default());
/// let engine = RiskEngine::new(config);
///
/// // Greeks calculation
/// let result = engine.compute_greeks("T001", || Ok(greeks_result))?;
///
/// // Scenario analysis
/// engine.add_scenario(scenario);
/// let scenario_results = engine.run_all_scenarios(&portfolio, base_pricer)?;
///
/// // Portfolio pricing
/// let prices = engine.price_portfolio(&portfolio, |trade| price_fn(trade));
/// ```
#[derive(Debug, Clone)]
pub struct RiskEngine {
    config: RiskEngineConfig,
    /// Internal scenario engine for stress testing.
    scenario_engine: ScenarioEngine<f64>,
}

#[allow(clippy::result_large_err)]
impl RiskEngine {
    /// Creates a new RiskEngine with the given configuration.
    pub fn new(config: RiskEngineConfig) -> Self {
        Self {
            config,
            scenario_engine: ScenarioEngine::new(),
        }
    }

    /// Creates a RiskEngine with default configuration.
    pub fn with_defaults() -> Self { Self::new(RiskEngineConfig::default()) }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &RiskEngineConfig { &self.config }

    /// Returns the active Greeks calculation method.
    pub fn greeks_method(&self) -> GreeksMethod { self.config.risk_config.greeks_method }

    /// Checks if AAD is available.
    ///
    /// Returns true if the `enzyme-ad` feature is enabled.
    #[cfg(feature = "enzyme-ad")]
    pub fn is_aad_available() -> bool { true }

    /// Checks if AAD is available.
    ///
    /// Returns false when `enzyme-ad` feature is not enabled.
    #[cfg(not(feature = "enzyme-ad"))]
    pub fn is_aad_available() -> bool { false }

    /// Converts RiskConfig's GreeksMethod to GreeksMode.
    #[allow(dead_code)]
    fn to_greeks_mode(&self) -> Result<GreeksMode, RiskError> {
        match self.config.risk_config.greeks_method {
            GreeksMethod::Aad => {
                if Self::is_aad_available() {
                    // When enzyme-ad is available, use it
                    #[cfg(feature = "enzyme-ad")]
                    {
                        Ok(GreeksMode::EnzymeAAD)
                    }
                    #[cfg(not(feature = "enzyme-ad"))]
                    {
                        Err(RiskError::AadNotAvailable)
                    }
                } else {
                    Err(RiskError::AadNotAvailable)
                }
            }
            GreeksMethod::Bump => Ok(GreeksMode::BumpRevalue),
        }
    }

    /// Creates GreeksConfig from RiskConfig.
    #[allow(dead_code)]
    fn create_greeks_config(&self) -> Result<GreeksConfig, RiskError> {
        let mode = self.to_greeks_mode()?;
        let bump_sizes = &self.config.risk_config.bump_sizes;

        let mut builder = GreeksConfig::builder().mode(mode);

        // Apply bump sizes
        builder = builder
            .spot_bump_relative(bump_sizes.spot)
            .vol_bump_absolute(bump_sizes.vol)
            .rate_bump_absolute(bump_sizes.rate);

        builder
            .build()
            .map_err(|e| RiskError::Config(e.to_string()))
    }

    /// Determines whether to use parallel processing.
    fn should_parallelize(&self, n_trades: usize) -> bool {
        n_trades >= self.config.parallel_threshold
    }

    /// Computes Greeks for a trade using the internal pricing function.
    ///
    /// This is a placeholder that will be connected to actual pricing.
    fn compute_trade_greeks_internal<F>(
        &self,
        trade_id: &str,
        pricing_fn: F,
    ) -> Result<RiskResult, RiskError>
    where
        F: Fn() -> Result<GreeksResult<f64>, RiskError>,
    {
        let start = Instant::now();

        let greeks_result = pricing_fn()?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        let computed = ComputedGreeks::from_greeks_result(&greeks_result);

        Ok(RiskResult {
            trade_id: trade_id.to_string(),
            pv: greeks_result.price,
            greeks: computed,
            method: self.config.risk_config.greeks_method,
            metrics: PerformanceMetrics::new(elapsed_ms),
        })
    }

    /// Computes Greeks for a single trade.
    ///
    /// This method accepts a closure that performs the actual pricing,
    /// allowing flexibility in how trades are priced.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Trade identifier
    /// * `pricing_fn` - Closure that computes the Greeks
    ///
    /// # Returns
    ///
    /// `RiskResult` containing computed Greeks and metrics.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = engine.compute_greeks("T001", || {
    ///     // Your pricing logic here
    ///     Ok(GreeksResult::new(100.0, 0.01).with_delta(0.5))
    /// })?;
    /// ```
    pub fn compute_greeks<F>(&self, trade_id: &str, pricing_fn: F) -> Result<RiskResult, RiskError>
    where
        F: Fn() -> Result<GreeksResult<f64>, RiskError>,
    {
        // Validate AAD availability if requested
        if self.config.risk_config.greeks_method == GreeksMethod::Aad && !Self::is_aad_available() {
            return Err(RiskError::AadNotAvailable);
        }

        self.compute_trade_greeks_internal(trade_id, pricing_fn)
    }

    /// Computes Greeks for a portfolio of trades.
    ///
    /// Automatically selects sequential or parallel processing based on
    /// portfolio size and configuration.
    ///
    /// # Arguments
    ///
    /// * `trades` - Iterator of (trade_id, pricing_fn) pairs
    ///
    /// # Returns
    ///
    /// `PortfolioRiskResult` containing individual results and aggregations.
    pub fn compute_portfolio_greeks<'a, I, F>(
        &self,
        trades: I,
    ) -> Result<PortfolioRiskResult, RiskError>
    where
        I: IntoIterator<Item = (&'a str, F)>,
        F: Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync,
    {
        // Validate AAD availability if requested
        if self.config.risk_config.greeks_method == GreeksMethod::Aad && !Self::is_aad_available() {
            return Err(RiskError::AadNotAvailable);
        }

        let trades_vec: Vec<_> = trades.into_iter().collect();

        if trades_vec.is_empty() {
            return Err(RiskError::EmptyPortfolio);
        }

        if self.should_parallelize(trades_vec.len()) {
            self.compute_portfolio_greeks_parallel(trades_vec)
        } else {
            self.compute_portfolio_greeks_sequential(trades_vec)
        }
    }

    /// Computes portfolio Greeks sequentially.
    fn compute_portfolio_greeks_sequential<F>(
        &self,
        trades: Vec<(&str, F)>,
    ) -> Result<PortfolioRiskResult, RiskError>
    where
        F: Fn() -> Result<GreeksResult<f64>, RiskError>,
    {
        let start = Instant::now();
        let mut results = Vec::with_capacity(trades.len());
        let mut failures = Vec::new();

        for (trade_id, pricing_fn) in trades {
            match self.compute_trade_greeks_internal(trade_id, pricing_fn) {
                Ok(result) => results.push(result),
                Err(e) => {
                    if self.config.continue_on_error {
                        failures.push(FailedCalculation::new(trade_id, e.to_string()));
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        Ok(PortfolioRiskResult::new(
            results, failures, elapsed_ms, false,
        ))
    }

    /// Computes portfolio Greeks in parallel.
    fn compute_portfolio_greeks_parallel<F>(
        &self,
        trades: Vec<(&str, F)>,
    ) -> Result<PortfolioRiskResult, RiskError>
    where
        F: Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync,
    {
        let start = Instant::now();

        let batch_results: Vec<_> = trades
            .par_iter()
            .map(|(trade_id, pricing_fn)| {
                match self.compute_trade_greeks_internal(trade_id, pricing_fn) {
                    Ok(result) => (Some(result), None),
                    Err(e) => (None, Some(FailedCalculation::new(*trade_id, e.to_string()))),
                }
            })
            .collect();

        let mut results = Vec::with_capacity(trades.len());
        let mut failures = Vec::new();

        for (result, failure) in batch_results {
            if let Some(r) = result {
                results.push(r);
            }
            if let Some(failed) = failure {
                if self.config.continue_on_error {
                    failures.push(failed);
                } else {
                    return Err(RiskError::CalculationFailed {
                        trade_id: failed.trade_id,
                        reason: failed.error_message,
                        partial_results: None,
                    });
                }
            }
        }

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        Ok(PortfolioRiskResult::new(
            results, failures, elapsed_ms, true,
        ))
    }

    /// Returns the target Greeks from configuration.
    pub fn target_greeks(&self) -> &[GreekType] { &self.config.risk_config.target_greeks }

    /// Returns true if second-order Greeks are requested.
    pub fn has_second_order_greeks(&self) -> bool {
        self.config.risk_config.has_second_order_greeks()
    }

    /// Returns the second-order calculation mode.
    pub fn second_order_mode(&self) -> SecondOrderMode { self.config.risk_config.second_order_mode }

    // =========================================================================
    // Task 7.10: CSA Conditions Support
    // =========================================================================

    /// Applies CSA (Credit Support Annex) conditions to an exposure value.
    ///
    /// This method adjusts the raw exposure based on collateral agreement
    /// terms:
    /// - Threshold: Amount below which no collateral is required
    /// - Independent Amount: Pre-agreed collateral amount
    ///
    /// # Arguments
    ///
    /// * `exposure` - Raw uncollateralized exposure
    /// * `collateral` - Collateral agreement parameters
    ///
    /// # Returns
    ///
    /// Collateralized exposure after applying CSA terms.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pricer_risk::portfolio::CollateralAgreement;
    ///
    /// let csa = CollateralAgreement::new(1_000_000.0, 0.0, 0.0, Currency::USD, 0.04)?;
    /// let collateralised_exp = engine.apply_csa_adjustment(2_500_000.0, &csa);
    /// assert_eq!(collateralised_exp, 1_500_000.0); // 2.5M - 1M threshold
    /// ```
    pub fn apply_csa_adjustment(
        &self,
        exposure: f64,
        collateral: &crate::portfolio::CollateralAgreement,
    ) -> f64 {
        collateral.collateralised_exposure(exposure)
    }

    /// Applies CSA conditions to a portfolio of exposures by netting set.
    ///
    /// # Arguments
    ///
    /// * `exposures` - Map of netting set ID to raw exposure
    /// * `netting_sets` - Netting sets with collateral agreements
    ///
    /// # Returns
    ///
    /// Map of netting set ID to collateralized exposure.
    pub fn apply_csa_to_portfolio(
        &self,
        exposures: &std::collections::HashMap<String, f64>,
        netting_sets: &[crate::portfolio::NettingSet],
    ) -> std::collections::HashMap<String, f64> {
        let mut collateralised = std::collections::HashMap::new();

        for ns in netting_sets {
            let ns_id = ns.id().as_str().to_string();
            if let Some(&exposure) = exposures.get(&ns_id) {
                let adj_exposure = if let Some(csa) = ns.collateral() {
                    self.apply_csa_adjustment(exposure, csa)
                } else {
                    exposure
                };
                collateralised.insert(ns_id, adj_exposure);
            }
        }

        collateralised
    }

    // =========================================================================
    // Task 7.11: Scenario-Based Greeks
    // =========================================================================

    /// Computes Greeks under a specified scenario.
    ///
    /// This method applies market shifts from a scenario before computing
    /// Greeks, allowing stress testing and scenario analysis.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Trade identifier
    /// * `scenario` - Market scenario to apply
    /// * `pricing_fn` - Closure that computes Greeks under the scenario
    ///
    /// # Returns
    ///
    /// `RiskResult` with Greeks computed under the scenario.
    pub fn compute_greeks_with_scenario<F>(
        &self,
        trade_id: &str,
        scenario_name: &str,
        pricing_fn: F,
    ) -> Result<ScenarioGreeksResult, RiskError>
    where
        F: Fn() -> Result<crate::greeks::GreeksResult<f64>, RiskError>,
    {
        let result = self.compute_greeks(trade_id, pricing_fn)?;

        Ok(ScenarioGreeksResult {
            scenario_name: scenario_name.to_string(),
            result,
        })
    }

    /// Computes Greeks for multiple scenarios.
    ///
    /// This is useful for stress testing where Greeks need to be computed
    /// under various market conditions.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Trade identifier
    /// * `scenarios` - Vector of (scenario_name, pricing_fn) pairs
    ///
    /// # Returns
    ///
    /// Vector of `ScenarioGreeksResult` for each scenario.
    pub fn compute_greeks_multi_scenario<'a, I, F>(
        &self,
        trade_id: &str,
        scenarios: I,
    ) -> Result<Vec<ScenarioGreeksResult>, RiskError>
    where
        I: IntoIterator<Item = (&'a str, F)>,
        F: Fn() -> Result<crate::greeks::GreeksResult<f64>, RiskError>,
    {
        let mut results = Vec::new();

        for (scenario_name, pricing_fn) in scenarios {
            let scenario_result =
                self.compute_greeks_with_scenario(trade_id, scenario_name, pricing_fn)?;
            results.push(scenario_result);
        }

        Ok(results)
    }

    /// Computes scenario-based portfolio Greeks.
    ///
    /// Applies a scenario to an entire portfolio and computes Greeks for each
    /// trade.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    /// * `trades` - Iterator of (trade_id, pricing_fn) pairs
    ///
    /// # Returns
    ///
    /// `ScenarioPortfolioResult` with aggregated Greeks under the scenario.
    pub fn compute_portfolio_greeks_with_scenario<'a, I, F>(
        &self,
        scenario_name: &str,
        trades: I,
    ) -> Result<ScenarioPortfolioResult, RiskError>
    where
        I: IntoIterator<Item = (&'a str, F)>,
        F: Fn() -> Result<crate::greeks::GreeksResult<f64>, RiskError> + Send + Sync,
    {
        let portfolio_result = self.compute_portfolio_greeks(trades)?;

        Ok(ScenarioPortfolioResult {
            scenario_name: scenario_name.to_string(),
            result: portfolio_result,
        })
    }

    // =========================================================================
    // Scenario Engine Delegation (Facade Pattern)
    // =========================================================================

    /// Adds a scenario to the internal scenario engine.
    ///
    /// # Arguments
    ///
    /// * `scenario` - The scenario to register
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pricer_risk::{RiskEngine, scenarios::{Scenario, BumpScenario, RiskFactorShift}};
    ///
    /// let mut engine = RiskEngine::with_defaults();
    /// let scenario = Scenario::named(
    ///     "IR +100bp",
    ///     BumpScenario::new().with_shift(RiskFactorShift::rate_parallel("*", 0.01)),
    /// );
    /// engine.add_scenario(scenario);
    /// ```
    pub fn add_scenario(&mut self, scenario: Scenario<f64>) {
        self.scenario_engine.add_scenario(scenario);
    }

    /// Adds multiple scenarios to the internal scenario engine.
    pub fn add_scenarios(&mut self, scenarios: impl IntoIterator<Item = Scenario<f64>>) {
        self.scenario_engine.add_scenarios(scenarios);
    }

    /// Returns the number of registered scenarios.
    pub fn scenario_count(&self) -> usize { self.scenario_engine.scenario_count() }

    /// Returns a reference to the registered scenarios.
    pub fn scenarios(&self) -> &[Scenario<f64>] { self.scenario_engine.scenarios() }

    /// Executes a single scenario against a portfolio.
    ///
    /// # Arguments
    ///
    /// * `scenario` - The scenario to execute
    /// * `base_value` - The base portfolio value (before stress)
    /// * `pricer` - Function that returns the stressed value given scenario
    ///   name
    ///
    /// # Returns
    ///
    /// `ScenarioResult` with P&L breakdown.
    pub fn run_scenario<F>(
        &mut self,
        scenario: &Scenario<f64>,
        base_value: f64,
        pricer: F,
    ) -> ScenarioResult<f64>
    where
        F: Fn(&str) -> f64,
    {
        self.scenario_engine
            .execute_scenario(scenario, base_value, pricer)
    }

    /// Executes all registered scenarios against a portfolio.
    ///
    /// # Arguments
    ///
    /// * `base_value` - The base portfolio value
    /// * `pricer` - Function that returns the stressed value given scenario
    ///   name
    ///
    /// # Returns
    ///
    /// Vector of `ScenarioResult` for all scenarios.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let results = engine.run_all_scenarios(1_000_000.0, |scenario_name| {
    ///     // Apply scenario and reprice portfolio
    ///     match scenario_name {
    ///         "IR +100bp" => 950_000.0,
    ///         "IR -100bp" => 1_050_000.0,
    ///         _ => 1_000_000.0,
    ///     }
    /// });
    /// ```
    pub fn run_all_scenarios<F>(&mut self, base_value: f64, pricer: F) -> Vec<ScenarioResult<f64>>
    where
        F: Fn(&str) -> f64,
    {
        self.scenario_engine.execute_all(base_value, pricer)
    }

    /// Returns the worst-case scenario result (largest loss).
    pub fn worst_case_scenario(&self) -> Option<&ScenarioResult<f64>> {
        self.scenario_engine.worst_case()
    }

    /// Returns all scenario results.
    pub fn scenario_results(&self) -> &[ScenarioResult<f64>] { self.scenario_engine.results() }

    /// Clears all scenario results (keeps scenarios registered).
    pub fn clear_scenario_results(&mut self) { self.scenario_engine.clear_results(); }

    /// Clears all scenarios and results.
    pub fn clear_scenarios(&mut self) { self.scenario_engine.clear(); }

    // =========================================================================
    // Portfolio Operations (Delegation)
    // =========================================================================

    /// Prices all trades in a portfolio using the provided pricing function.
    ///
    /// Leverages Rayon for parallel execution when portfolio size exceeds
    /// the parallel threshold.
    ///
    /// # Arguments
    ///
    /// * `portfolio` - The portfolio to price
    /// * `pricer_fn` - Function that takes a trade reference and returns a
    ///   price
    ///
    /// # Returns
    ///
    /// HashMap mapping trade IDs to prices.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let prices = engine.price_portfolio(&portfolio, |trade| {
    ///     // Monte Carlo or analytical pricing
    ///     mc_pricer.price(trade.instrument())
    /// });
    /// ```
    pub fn price_portfolio<F>(&self, portfolio: &Portfolio, pricer_fn: F) -> HashMap<TradeId, f64>
    where
        F: Fn(&Trade) -> f64 + Sync,
    {
        portfolio.price_all_trades(pricer_fn)
    }

    /// Aggregates values by netting set using the provided aggregation
    /// function.
    ///
    /// # Arguments
    ///
    /// * `portfolio` - The portfolio to aggregate
    /// * `agg_fn` - Function that takes a slice of trades and returns an
    ///   aggregated value
    ///
    /// # Returns
    ///
    /// HashMap mapping netting set IDs to aggregated values.
    pub fn aggregate_by_netting_set<F>(
        &self,
        portfolio: &Portfolio,
        agg_fn: F,
    ) -> HashMap<NettingSetId, f64>
    where
        F: Fn(&[&Trade]) -> f64 + Sync,
    {
        portfolio.aggregate_by_netting_set(agg_fn)
    }

    /// Computes total portfolio value using the provided pricing function.
    ///
    /// # Arguments
    ///
    /// * `portfolio` - The portfolio to price
    /// * `pricer_fn` - Function that takes a trade reference and returns a
    ///   price
    ///
    /// # Returns
    ///
    /// Total portfolio value (sum of all trade prices).
    pub fn total_portfolio_value<F>(&self, portfolio: &Portfolio, pricer_fn: F) -> f64
    where
        F: Fn(&Trade) -> f64 + Sync,
    {
        self.price_portfolio(portfolio, pricer_fn).values().sum()
    }

    /// Returns a mutable reference to the internal scenario engine.
    ///
    /// Use this for advanced scenario operations not exposed through
    /// the facade methods.
    pub fn scenario_engine_mut(&mut self) -> &mut ScenarioEngine<f64> { &mut self.scenario_engine }

    /// Returns a reference to the internal scenario engine.
    pub fn scenario_engine(&self) -> &ScenarioEngine<f64> { &self.scenario_engine }
}

/// Result of Greeks calculation under a specific scenario.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScenarioGreeksResult {
    /// Name of the scenario applied.
    pub scenario_name: String,
    /// Greeks result under this scenario.
    pub result: RiskResult,
}

impl ScenarioGreeksResult {
    /// Returns the scenario name.
    pub fn scenario_name(&self) -> &str { &self.scenario_name }

    /// Returns the PV under this scenario.
    pub fn pv(&self) -> f64 { self.result.pv }

    /// Returns the delta under this scenario.
    pub fn delta(&self) -> Option<f64> { self.result.greeks.delta }
}

/// Result of portfolio Greeks calculation under a specific scenario.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScenarioPortfolioResult {
    /// Name of the scenario applied.
    pub scenario_name: String,
    /// Portfolio result under this scenario.
    pub result: PortfolioRiskResult,
}

impl ScenarioPortfolioResult {
    /// Returns the scenario name.
    pub fn scenario_name(&self) -> &str { &self.scenario_name }

    /// Returns the total PV under this scenario.
    pub fn total_pv(&self) -> f64 { self.result.total_pv() }

    /// Returns the total delta under this scenario.
    pub fn total_delta(&self) -> Option<f64> { self.result.aggregations.total.delta }
}

#[cfg(test)]
mod tests {
    use infra_config::GreekType;

    use super::*;

    // =========================================================================
    // RiskEngineConfig Tests
    // =========================================================================

    #[test]
    fn test_risk_engine_config_default() {
        let config = RiskEngineConfig::default();
        assert_eq!(config.parallel_threshold, 100);
        assert_eq!(config.batch_size, 64);
        assert!(config.continue_on_error);
    }

    #[test]
    fn test_risk_engine_config_builder() {
        let risk_config = RiskConfig {
            greeks_method: GreeksMethod::Bump,
            target_greeks: vec![GreekType::Delta, GreekType::Vega],
            ..Default::default()
        };

        let config = RiskEngineConfig::new(risk_config)
            .with_parallel_threshold(50)
            .with_batch_size(32)
            .with_continue_on_error(false);

        assert_eq!(config.parallel_threshold, 50);
        assert_eq!(config.batch_size, 32);
        assert!(!config.continue_on_error);
    }

    #[test]
    fn test_risk_engine_config_batch_size_minimum() {
        let config = RiskEngineConfig::default().with_batch_size(0);
        assert_eq!(config.batch_size, 1);
    }

    // =========================================================================
    // RiskEngine Tests
    // =========================================================================

    #[test]
    fn test_risk_engine_new() {
        let engine = RiskEngine::with_defaults();
        assert_eq!(engine.greeks_method(), GreeksMethod::Bump);
    }

    #[test]
    fn test_risk_engine_is_aad_available() {
        // Without enzyme-ad feature, should return false
        #[cfg(not(feature = "enzyme-ad"))]
        assert!(!RiskEngine::is_aad_available());

        #[cfg(feature = "enzyme-ad")]
        assert!(RiskEngine::is_aad_available());
    }

    #[test]
    fn test_risk_engine_aad_not_available_error() {
        let risk_config = RiskConfig {
            greeks_method: GreeksMethod::Aad,
            ..Default::default()
        };
        let engine = RiskEngine::new(RiskEngineConfig::new(risk_config));

        let result = engine.compute_greeks("T001", || Ok(GreeksResult::new(100.0, 0.01)));

        #[cfg(not(feature = "enzyme-ad"))]
        assert!(matches!(result, Err(RiskError::AadNotAvailable)));

        #[cfg(feature = "enzyme-ad")]
        assert!(result.is_ok());
    }

    #[test]
    fn test_risk_engine_compute_greeks_bump() {
        let engine = RiskEngine::with_defaults();

        let result = engine.compute_greeks("T001", || {
            Ok(GreeksResult::new(100.0, 0.01)
                .with_delta(0.5)
                .with_gamma(0.02))
        });

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.trade_id, "T001");
        assert!((result.pv - 100.0).abs() < f64::EPSILON);
        assert!((result.greeks.delta.unwrap() - 0.5).abs() < f64::EPSILON);
        assert!((result.greeks.gamma.unwrap() - 0.02).abs() < f64::EPSILON);
        assert_eq!(result.method, GreeksMethod::Bump);
    }

    #[test]
    fn test_risk_engine_compute_greeks_error() {
        let engine = RiskEngine::with_defaults();

        let result = engine.compute_greeks("T001", || {
            Err(RiskError::MarketData("Curve not found".to_string()))
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(RiskError::MarketData(_))));
    }

    #[test]
    fn test_risk_engine_should_parallelize() {
        let engine = RiskEngine::with_defaults();

        assert!(!engine.should_parallelize(50));
        assert!(!engine.should_parallelize(99));
        assert!(engine.should_parallelize(100));
        assert!(engine.should_parallelize(1000));
    }

    #[test]
    fn test_risk_engine_compute_portfolio_greeks_empty() {
        let engine = RiskEngine::with_defaults();
        let trades: Vec<(&str, fn() -> Result<GreeksResult<f64>, RiskError>)> = vec![];

        let result = engine.compute_portfolio_greeks(trades);
        assert!(matches!(result, Err(RiskError::EmptyPortfolio)));
    }

    #[test]
    fn test_risk_engine_compute_portfolio_greeks_sequential() {
        let config = RiskEngineConfig::default().with_parallel_threshold(1000);
        let engine = RiskEngine::new(config);

        let trades: Vec<(
            &str,
            Box<dyn Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync>,
        )> = vec![
            (
                "T001",
                Box::new(|| Ok(GreeksResult::new(100.0, 0.01).with_delta(0.5))),
            ),
            (
                "T002",
                Box::new(|| Ok(GreeksResult::new(50.0, 0.005).with_delta(0.3))),
            ),
        ];

        let result = engine.compute_portfolio_greeks(trades).unwrap();

        assert_eq!(result.results.len(), 2);
        assert!(result.all_succeeded());
        assert!(!result.stats.used_parallel);
        assert!((result.total_pv() - 150.0).abs() < f64::EPSILON);
        assert!((result.aggregations.total.delta.unwrap() - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_risk_engine_compute_portfolio_greeks_parallel() {
        let config = RiskEngineConfig::default().with_parallel_threshold(1);
        let engine = RiskEngine::new(config);

        let trades: Vec<(
            &str,
            Box<dyn Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync>,
        )> = vec![
            (
                "T001",
                Box::new(|| Ok(GreeksResult::new(100.0, 0.01).with_delta(0.5))),
            ),
            (
                "T002",
                Box::new(|| Ok(GreeksResult::new(50.0, 0.005).with_delta(0.3))),
            ),
            (
                "T003",
                Box::new(|| Ok(GreeksResult::new(25.0, 0.002).with_delta(0.1))),
            ),
        ];

        let result = engine.compute_portfolio_greeks(trades).unwrap();

        assert_eq!(result.results.len(), 3);
        assert!(result.all_succeeded());
        assert!(result.stats.used_parallel);
    }

    #[test]
    fn test_risk_engine_compute_portfolio_greeks_with_failures() {
        let config = RiskEngineConfig::default()
            .with_parallel_threshold(1000)
            .with_continue_on_error(true);
        let engine = RiskEngine::new(config);

        let trades: Vec<(
            &str,
            Box<dyn Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync>,
        )> = vec![
            (
                "T001",
                Box::new(|| Ok(GreeksResult::new(100.0, 0.01).with_delta(0.5))),
            ),
            (
                "T002",
                Box::new(|| Err(RiskError::MarketData("Missing curve".to_string()))),
            ),
            (
                "T003",
                Box::new(|| Ok(GreeksResult::new(50.0, 0.005).with_delta(0.3))),
            ),
        ];

        let result = engine.compute_portfolio_greeks(trades).unwrap();

        assert_eq!(result.results.len(), 2);
        assert_eq!(result.failures.len(), 1);
        assert!(!result.all_succeeded());
        assert_eq!(result.stats.successful, 2);
        assert_eq!(result.stats.failed, 1);
    }

    #[test]
    fn test_risk_engine_target_greeks() {
        let risk_config = RiskConfig {
            target_greeks: vec![GreekType::Delta, GreekType::Gamma, GreekType::Vega],
            ..Default::default()
        };
        let engine = RiskEngine::new(RiskEngineConfig::new(risk_config));

        assert_eq!(engine.target_greeks().len(), 3);
        assert!(engine.has_second_order_greeks());
    }

    #[test]
    fn test_risk_engine_second_order_mode() {
        let risk_config = RiskConfig {
            second_order_mode: SecondOrderMode::Serial,
            ..Default::default()
        };
        let engine = RiskEngine::new(RiskEngineConfig::new(risk_config));

        assert_eq!(engine.second_order_mode(), SecondOrderMode::Serial);
    }

    // =========================================================================
    // Task 7.10: CSA Conditions Tests
    // =========================================================================

    #[test]
    fn test_apply_csa_adjustment_below_threshold() {
        use infra_domain::market::Currency;

        use crate::portfolio::CollateralAgreement;

        let engine = RiskEngine::with_defaults();
        let csa = CollateralAgreement::new(
            1_000_000.0, // threshold
            0.0,
            0.0,
            Currency::USD,
            CollateralAgreement::bilateral_mpor(),
        )
        .unwrap();

        // Exposure below threshold: should return 0
        let result = engine.apply_csa_adjustment(500_000.0, &csa);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_apply_csa_adjustment_above_threshold() {
        use infra_domain::market::Currency;

        use crate::portfolio::CollateralAgreement;

        let engine = RiskEngine::with_defaults();
        let csa = CollateralAgreement::new(
            1_000_000.0, // threshold
            0.0,
            0.0,
            Currency::USD,
            CollateralAgreement::bilateral_mpor(),
        )
        .unwrap();

        // Exposure above threshold: 2.5M - 1M = 1.5M
        let result = engine.apply_csa_adjustment(2_500_000.0, &csa);
        assert!((result - 1_500_000.0).abs() < 1e-10);
    }

    #[test]
    fn test_apply_csa_adjustment_with_independent_amount() {
        use infra_domain::market::Currency;

        use crate::portfolio::CollateralAgreement;

        let engine = RiskEngine::with_defaults();
        let csa = CollateralAgreement::new(
            1_000_000.0, // threshold
            0.0,
            200_000.0, // independent amount (we post)
            Currency::USD,
            CollateralAgreement::bilateral_mpor(),
        )
        .unwrap();

        // CE = max(E - Threshold - IA, 0) = max(2.5M - 1M - 0.2M, 0) = 1.3M
        let result = engine.apply_csa_adjustment(2_500_000.0, &csa);
        assert!((result - 1_300_000.0).abs() < 1e-10);
    }

    #[test]
    fn test_apply_csa_to_portfolio() {
        use std::collections::HashMap;

        use infra_domain::market::Currency;

        use crate::portfolio::{CollateralAgreement, CounterpartyId, NettingSet, NettingSetId};

        let engine = RiskEngine::with_defaults();

        // Create netting sets - one with CSA, one without
        let csa = CollateralAgreement::new(
            500_000.0,
            0.0,
            0.0,
            Currency::USD,
            CollateralAgreement::bilateral_mpor(),
        )
        .unwrap();

        let ns_collateralised = NettingSet::with_collateral(
            NettingSetId::new("NS001"),
            CounterpartyId::new("CP001"),
            csa,
        );
        let ns_uncollateralised =
            NettingSet::new(NettingSetId::new("NS002"), CounterpartyId::new("CP002"));

        let netting_sets = vec![ns_collateralised, ns_uncollateralised];

        let mut exposures = HashMap::new();
        exposures.insert("NS001".to_string(), 1_000_000.0);
        exposures.insert("NS002".to_string(), 500_000.0);

        let result = engine.apply_csa_to_portfolio(&exposures, &netting_sets);

        // NS001: 1M - 500K threshold = 500K collateralised exposure
        assert!((result.get("NS001").unwrap() - 500_000.0).abs() < 1e-10);
        // NS002: no CSA, so exposure remains unchanged
        assert!((result.get("NS002").unwrap() - 500_000.0).abs() < 1e-10);
    }

    // =========================================================================
    // Task 7.11: Scenario-Based Greeks Tests
    // =========================================================================

    #[test]
    fn test_compute_greeks_with_scenario() {
        let engine = RiskEngine::with_defaults();

        let result = engine.compute_greeks_with_scenario("T001", "IR +100bp", || {
            Ok(GreeksResult::new(95.0, 0.01).with_delta(0.45))
        });

        assert!(result.is_ok());
        let scenario_result = result.unwrap();
        assert_eq!(scenario_result.scenario_name(), "IR +100bp");
        assert!((scenario_result.pv() - 95.0).abs() < f64::EPSILON);
        assert!((scenario_result.delta().unwrap() - 0.45).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_greeks_multi_scenario() {
        let engine = RiskEngine::with_defaults();

        let scenarios: Vec<(&str, Box<dyn Fn() -> Result<GreeksResult<f64>, RiskError>>)> = vec![
            (
                "Base",
                Box::new(|| Ok(GreeksResult::new(100.0, 0.01).with_delta(0.5))),
            ),
            (
                "IR +50bp",
                Box::new(|| Ok(GreeksResult::new(98.0, 0.01).with_delta(0.48))),
            ),
            (
                "IR +100bp",
                Box::new(|| Ok(GreeksResult::new(95.0, 0.01).with_delta(0.45))),
            ),
        ];

        let results = engine.compute_greeks_multi_scenario("T001", scenarios);

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].scenario_name(), "Base");
        assert_eq!(results[1].scenario_name(), "IR +50bp");
        assert_eq!(results[2].scenario_name(), "IR +100bp");
    }

    #[test]
    fn test_compute_portfolio_greeks_with_scenario() {
        let config = RiskEngineConfig::default().with_parallel_threshold(1000);
        let engine = RiskEngine::new(config);

        let trades: Vec<(
            &str,
            Box<dyn Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync>,
        )> = vec![
            (
                "T001",
                Box::new(|| Ok(GreeksResult::new(95.0, 0.01).with_delta(0.45))),
            ),
            (
                "T002",
                Box::new(|| Ok(GreeksResult::new(48.0, 0.005).with_delta(0.28))),
            ),
        ];

        let result = engine.compute_portfolio_greeks_with_scenario("Stress Test", trades);

        assert!(result.is_ok());
        let scenario_result = result.unwrap();
        assert_eq!(scenario_result.scenario_name(), "Stress Test");
        assert!((scenario_result.total_pv() - 143.0).abs() < 1e-10);
        assert!((scenario_result.total_delta().unwrap() - 0.73).abs() < 1e-10);
    }

    #[test]
    fn test_scenario_greeks_result_accessors() {
        let greeks = ComputedGreeks {
            delta: Some(0.5),
            gamma: Some(0.02),
            ..Default::default()
        };
        let risk_result = RiskResult::new("T001", 100.0, greeks, GreeksMethod::Bump, 1.0);
        let scenario_result = ScenarioGreeksResult {
            scenario_name: "Test Scenario".to_string(),
            result: risk_result,
        };

        assert_eq!(scenario_result.scenario_name(), "Test Scenario");
        assert!((scenario_result.pv() - 100.0).abs() < f64::EPSILON);
        assert!((scenario_result.delta().unwrap() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scenario_portfolio_result_accessors() {
        let greeks = ComputedGreeks {
            delta: Some(0.5),
            ..Default::default()
        };
        let risk_result = RiskResult::new("T001", 100.0, greeks, GreeksMethod::Bump, 1.0);
        let portfolio_result = PortfolioRiskResult::new(vec![risk_result], vec![], 1.0, false);
        let scenario_result = ScenarioPortfolioResult {
            scenario_name: "Stress Test".to_string(),
            result: portfolio_result,
        };

        assert_eq!(scenario_result.scenario_name(), "Stress Test");
        assert!((scenario_result.total_pv() - 100.0).abs() < f64::EPSILON);
        assert!((scenario_result.total_delta().unwrap() - 0.5).abs() < f64::EPSILON);
    }
}
