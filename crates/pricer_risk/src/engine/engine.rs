//! Risk engine facade.
//!
//! Provides [`RiskEngine`] as the unified entry point for risk calculations.
//!
//! # Requirements
//!
//! - Requirement 5.1: RiskEngine facade
//! - Requirement 5.2: compute_greeks() method
//! - Requirement 5.3: AAD/Bump mode selection
//! - Requirement 5.4: Risk factor identification

use std::time::Instant;

use infra_config::{GreeksMethod, GreekType, RiskConfig, SecondOrderMode};
use rayon::prelude::*;

use super::error::RiskError;
use super::result::{
    ComputedGreeks, FailedCalculation, PerformanceMetrics, PortfolioRiskResult, RiskResult,
};
use crate::greeks::{GreeksConfig, GreeksMode, GreeksResult};

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
/// Provides unified access to Greeks computation with support for:
/// - AAD (Automatic Adjoint Differentiation) via Enzyme
/// - Bump-and-Revalue (finite difference)
/// - Parallel portfolio processing
/// - Configuration-driven calculation
///
/// # Example
///
/// ```rust,ignore
/// use pricer_risk::engine::{RiskEngine, RiskEngineConfig};
/// use infra_config::RiskConfig;
///
/// let config = RiskEngineConfig::new(RiskConfig::default());
/// let engine = RiskEngine::new(config);
///
/// // Compute Greeks for a single trade
/// let result = engine.compute_greeks(&trade, &market, valuation_date)?;
/// println!("Delta: {:?}", result.greeks.delta);
/// ```
#[derive(Debug, Clone)]
pub struct RiskEngine {
    config: RiskEngineConfig,
}

impl RiskEngine {
    /// Creates a new RiskEngine with the given configuration.
    pub fn new(config: RiskEngineConfig) -> Self {
        Self { config }
    }

    /// Creates a RiskEngine with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(RiskEngineConfig::default())
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &RiskEngineConfig {
        &self.config
    }

    /// Returns the active Greeks calculation method.
    pub fn greeks_method(&self) -> GreeksMethod {
        self.config.risk_config.greeks_method
    }

    /// Checks if AAD is available.
    ///
    /// Returns true if the `enzyme-ad` feature is enabled.
    #[cfg(feature = "enzyme-ad")]
    pub fn is_aad_available() -> bool {
        true
    }

    /// Checks if AAD is available.
    ///
    /// Returns false when `enzyme-ad` feature is not enabled.
    #[cfg(not(feature = "enzyme-ad"))]
    pub fn is_aad_available() -> bool {
        false
    }

    /// Converts RiskConfig's GreeksMethod to GreeksMode.
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
    fn create_greeks_config(&self) -> Result<GreeksConfig, RiskError> {
        let mode = self.to_greeks_mode()?;
        let bump_sizes = &self.config.risk_config.bump_sizes;

        let mut builder = GreeksConfig::builder().mode(mode);

        // Apply bump sizes
        builder = builder
            .spot_bump_relative(bump_sizes.spot)
            .vol_bump_absolute(bump_sizes.vol)
            .rate_bump_absolute(bump_sizes.rate);

        builder.build().map_err(|e| RiskError::Config(e.to_string()))
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
        Ok(PortfolioRiskResult::new(results, failures, elapsed_ms, false))
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
        Ok(PortfolioRiskResult::new(results, failures, elapsed_ms, true))
    }

    /// Returns the target Greeks from configuration.
    pub fn target_greeks(&self) -> &[GreekType] {
        &self.config.risk_config.target_greeks
    }

    /// Returns true if second-order Greeks are requested.
    pub fn has_second_order_greeks(&self) -> bool {
        self.config.risk_config.has_second_order_greeks()
    }

    /// Returns the second-order calculation mode.
    pub fn second_order_mode(&self) -> SecondOrderMode {
        self.config.risk_config.second_order_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use infra_config::{BumpSizes, GreekType};

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

        let result = engine.compute_greeks("T001", || {
            Ok(GreeksResult::new(100.0, 0.01))
        });

        #[cfg(not(feature = "enzyme-ad"))]
        assert!(matches!(result, Err(RiskError::AadNotAvailable)));

        #[cfg(feature = "enzyme-ad")]
        assert!(result.is_ok());
    }

    #[test]
    fn test_risk_engine_compute_greeks_bump() {
        let engine = RiskEngine::with_defaults();

        let result = engine.compute_greeks("T001", || {
            Ok(GreeksResult::new(100.0, 0.01).with_delta(0.5).with_gamma(0.02))
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

        let trades: Vec<(&str, Box<dyn Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync>)> = vec![
            ("T001", Box::new(|| Ok(GreeksResult::new(100.0, 0.01).with_delta(0.5)))),
            ("T002", Box::new(|| Ok(GreeksResult::new(50.0, 0.005).with_delta(0.3)))),
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

        let trades: Vec<(&str, Box<dyn Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync>)> = vec![
            ("T001", Box::new(|| Ok(GreeksResult::new(100.0, 0.01).with_delta(0.5)))),
            ("T002", Box::new(|| Ok(GreeksResult::new(50.0, 0.005).with_delta(0.3)))),
            ("T003", Box::new(|| Ok(GreeksResult::new(25.0, 0.002).with_delta(0.1)))),
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

        let trades: Vec<(&str, Box<dyn Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync>)> = vec![
            ("T001", Box::new(|| Ok(GreeksResult::new(100.0, 0.01).with_delta(0.5)))),
            ("T002", Box::new(|| Err(RiskError::MarketData("Missing curve".to_string())))),
            ("T003", Box::new(|| Ok(GreeksResult::new(50.0, 0.005).with_delta(0.3)))),
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
}
