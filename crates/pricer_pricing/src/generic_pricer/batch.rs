//! Batch pricing for multiple trades.
//!
//! This module provides parallel batch pricing capabilities using Rayon.
//! Features:
//! - Parallel pricing of multiple trades
//! - Partial error continuation (failed trades don't stop processing)
//! - Arc-cached market data sharing
//! - Processing statistics
//! - Portfolio aggregations (by currency, netting set, book)

use std::collections::HashMap;
use std::{sync::Arc, time::Instant};

use infra_config::PricingConfig;
use infra_domain::trade::Trade;
use pricer_models::market::MarketProvider;
use rayon::prelude::*;

use super::{
    config::{ModelConfig, PricerConfig},
    error::PricingError,
};
use super::{pricer::GenericPricer, result::PricingResult};

// Type alias for batch pricing result type
type BatchResultType = PricingResult;

/// Unique identifier for a trade.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TradeId(pub String);

impl TradeId {
    /// Creates a new trade ID.
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for TradeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

impl From<&str> for TradeId {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

impl From<String> for TradeId {
    fn from(s: String) -> Self { Self(s) }
}

/// Statistics for batch pricing operation.
#[derive(Debug, Clone)]
pub struct BatchStats {
    /// Total number of trades processed.
    pub total_count: usize,
    /// Number of successfully priced trades.
    pub success_count: usize,
    /// Number of failed trades.
    pub failure_count: usize,
    /// Total elapsed time in milliseconds.
    pub elapsed_ms: u64,
}

impl BatchStats {
    /// Creates new batch statistics.
    pub fn new(total: usize, success: usize, failure: usize, elapsed_ms: u64) -> Self {
        Self {
            total_count: total,
            success_count: success,
            failure_count: failure,
            elapsed_ms,
        }
    }

    /// Returns the success rate as a percentage (0.0 - 100.0).
    pub fn success_rate(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            (self.success_count as f64 / self.total_count as f64) * 100.0
        }
    }

    /// Returns the average time per trade in milliseconds.
    pub fn avg_time_per_trade_ms(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.elapsed_ms as f64 / self.total_count as f64
        }
    }
}

/// Result of batch pricing operation.
#[derive(Debug)]
pub struct BatchPricingResult {
    /// Successfully priced trades.
    pub successes: Vec<(TradeId, BatchResultType)>,
    /// Failed trades with their errors.
    pub failures: Vec<(TradeId, PricingError)>,
    /// Processing statistics.
    pub stats: BatchStats,
}

impl BatchPricingResult {
    /// Creates a new batch pricing result.
    pub fn new(
        successes: Vec<(TradeId, BatchResultType)>,
        failures: Vec<(TradeId, PricingError)>,
        elapsed_ms: u64,
    ) -> Self {
        let total = successes.len() + failures.len();
        let stats = BatchStats::new(total, successes.len(), failures.len(), elapsed_ms);

        Self {
            successes,
            failures,
            stats,
        }
    }

    /// Returns true if all trades were priced successfully.
    pub fn all_succeeded(&self) -> bool { self.failures.is_empty() }

    /// Returns true if all trades failed.
    pub fn all_failed(&self) -> bool { self.successes.is_empty() }

    /// Returns the total PV across all successful trades.
    pub fn total_pv(&self) -> f64 { self.successes.iter().map(|(_, r)| r.total_pv).sum() }

    /// Gets the result for a specific trade ID.
    pub fn get(&self, trade_id: &TradeId) -> Option<Result<&BatchResultType, &PricingError>> {
        // Check successes first
        if let Some((_, result)) = self.successes.iter().find(|(id, _)| id == trade_id) {
            return Some(Ok(result));
        }

        // Check failures
        if let Some((_, error)) = self.failures.iter().find(|(id, _)| id == trade_id) {
            return Some(Err(error));
        }

        None
    }
}

/// Batch pricer for processing multiple trades in parallel.
///
/// Uses Rayon for parallel processing and shares market data via Arc.
#[derive(Debug, Clone)]
pub struct BatchPricer {
    /// Model configuration.
    model_config: ModelConfig,
    /// Pricer configuration.
    pricer_config: PricerConfig,
}

impl BatchPricer {
    /// Creates a new batch pricer.
    pub fn new(model_config: ModelConfig, pricer_config: PricerConfig) -> Self {
        Self {
            model_config,
            pricer_config,
        }
    }

    /// Returns the model configuration.
    pub fn model_config(&self) -> &ModelConfig { &self.model_config }

    /// Returns the pricer configuration.
    pub fn pricer_config(&self) -> &PricerConfig { &self.pricer_config }
}

// =============================================================================
// PortfolioPricer - Config-driven portfolio pricing with aggregations
// =============================================================================

/// Execution statistics for portfolio pricing.
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    /// Total number of trades processed.
    pub total_count: usize,
    /// Number of successfully priced trades.
    pub success_count: usize,
    /// Number of failed trades.
    pub failure_count: usize,
    /// Total elapsed time in milliseconds.
    pub elapsed_ms: u64,
}

impl ExecutionStats {
    /// Creates new execution statistics.
    pub fn new(total: usize, success: usize, failure: usize, elapsed_ms: u64) -> Self {
        Self {
            total_count: total,
            success_count: success,
            failure_count: failure,
            elapsed_ms,
        }
    }

    /// Returns the success rate as a percentage (0.0 - 100.0).
    pub fn success_rate(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            (self.success_count as f64 / self.total_count as f64) * 100.0
        }
    }

    /// Returns the average time per trade in milliseconds.
    pub fn avg_time_per_trade_ms(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.elapsed_ms as f64 / self.total_count as f64
        }
    }
}

/// Portfolio aggregations by various dimensions.
#[derive(Debug, Clone, Default)]
pub struct PortfolioAggregations {
    /// Total PV by currency.
    pub by_currency: HashMap<String, f64>,
    /// Total PV by netting set ID.
    pub by_netting_set: HashMap<String, f64>,
    /// Total PV by book.
    pub by_book: HashMap<String, f64>,
}

impl PortfolioAggregations {
    /// Creates a new empty aggregation.
    pub fn new() -> Self { Self::default() }

    /// Adds a PV to the currency bucket.
    pub fn add_by_currency(&mut self, currency: &str, pv: f64) {
        *self.by_currency.entry(currency.to_string()).or_insert(0.0) += pv;
    }

    /// Adds a PV to the netting set bucket.
    pub fn add_by_netting_set(&mut self, netting_set: &str, pv: f64) {
        *self
            .by_netting_set
            .entry(netting_set.to_string())
            .or_insert(0.0) += pv;
    }

    /// Adds a PV to the book bucket.
    pub fn add_by_book(&mut self, book: &str, pv: f64) {
        *self.by_book.entry(book.to_string()).or_insert(0.0) += pv;
    }
}

/// Result of portfolio pricing operation.
#[derive(Debug)]
pub struct PortfolioPricingResult {
    /// Successfully priced trades.
    pub successes: Vec<(TradeId, PricingResult)>,
    /// Failed trades with their errors.
    pub failures: Vec<(TradeId, PricingError)>,
    /// Execution statistics.
    pub stats: ExecutionStats,
    /// Aggregated PVs.
    pub aggregations: PortfolioAggregations,
}

impl PortfolioPricingResult {
    /// Creates a new portfolio pricing result.
    pub fn new(
        successes: Vec<(TradeId, PricingResult)>,
        failures: Vec<(TradeId, PricingError)>,
        elapsed_ms: u64,
        aggregations: PortfolioAggregations,
    ) -> Self {
        let total = successes.len() + failures.len();
        let stats = ExecutionStats::new(total, successes.len(), failures.len(), elapsed_ms);

        Self {
            successes,
            failures,
            stats,
            aggregations,
        }
    }

    /// Returns true if all trades were priced successfully.
    pub fn all_succeeded(&self) -> bool { self.failures.is_empty() }

    /// Returns true if all trades failed.
    pub fn all_failed(&self) -> bool { self.successes.is_empty() }

    /// Returns the total PV across all successful trades.
    pub fn total_pv(&self) -> f64 { self.successes.iter().map(|(_, r)| r.total_pv).sum() }

    /// Gets the result for a specific trade ID.
    pub fn get(&self, trade_id: &TradeId) -> Option<Result<&PricingResult, &PricingError>> {
        if let Some((_, result)) = self.successes.iter().find(|(id, _)| id == trade_id) {
            return Some(Ok(result));
        }
        if let Some((_, error)) = self.failures.iter().find(|(id, _)| id == trade_id) {
            return Some(Err(error));
        }
        None
    }
}

/// Portfolio pricer for config-driven portfolio pricing.
///
/// Provides portfolio-level pricing with:
/// - Parallel or sequential processing based on configuration
/// - Aggregations by currency, netting set, and book
/// - Partial failure handling (failed trades don't stop processing)
///
/// # Example
///
/// ```rust,ignore
/// use pricer_pricing::generic_pricer::PortfolioPricer;
/// use infra_config::PricingConfig;
///
/// let config = PricingConfig::from_toml_str(toml)?;
/// let pricer = PortfolioPricer::new(market, config)?;
/// let result = pricer.price_portfolio(&trades)?;
/// ```
#[derive(Debug)]
pub struct PortfolioPricer {
    /// Generic pricer instance.
    pricer: GenericPricer,
    /// Pricing configuration.
    config: PricingConfig,
}

impl PortfolioPricer {
    /// Creates a new portfolio pricer from configuration.
    ///
    /// # Arguments
    ///
    /// * `market` - Arc-shared market data provider
    /// * `config` - Pricing configuration
    ///
    /// # Errors
    ///
    /// Returns `PricingError` if configuration is invalid.
    pub fn new(market: Arc<MarketProvider>, config: PricingConfig) -> Result<Self, PricingError> {
        let pricer = GenericPricer::from_config(market, &config)?;
        Ok(Self { pricer, config })
    }

    /// Creates a portfolio pricer with existing GenericPricer.
    pub fn with_pricer(pricer: GenericPricer, config: PricingConfig) -> Self {
        Self { pricer, config }
    }

    /// Returns a reference to the pricing configuration.
    pub fn config(&self) -> &PricingConfig { &self.config }

    /// Returns a reference to the underlying GenericPricer.
    pub fn pricer(&self) -> &GenericPricer { &self.pricer }

    /// Prices a portfolio of trades.
    ///
    /// Uses parallel processing if `parallel_enabled` is true in config.
    /// Failed trades are recorded but don't abort the entire portfolio.
    ///
    /// # Arguments
    ///
    /// * `trades` - Slice of trades to price (with their IDs)
    ///
    /// # Returns
    ///
    /// `PortfolioPricingResult` containing successes, failures, stats, and
    /// aggregations.
    pub fn price_portfolio(
        &self,
        trades: &[(TradeId, Trade)],
    ) -> Result<PortfolioPricingResult, PricingError> {
        let start = Instant::now();

        let results: Vec<(TradeId, Result<PricingResult, PricingError>)> =
            if self.config.parallel_enabled {
                self.price_parallel(trades)
            } else {
                self.price_sequential(trades)
            };

        // Partition into successes and failures
        let mut successes = Vec::new();
        let mut failures = Vec::new();
        let mut aggregations = PortfolioAggregations::new();

        for (id, result) in results {
            match result {
                Ok(pricing_result) => {
                    // Update aggregations
                    aggregations.add_by_currency(
                        pricing_result.reporting_currency.code(),
                        pricing_result.total_pv,
                    );

                    // TODO: Add netting_set and book from trade metadata when available
                    // For now, use "default" as placeholder
                    aggregations.add_by_netting_set("default", pricing_result.total_pv);
                    aggregations.add_by_book("default", pricing_result.total_pv);

                    successes.push((id, pricing_result));
                }
                Err(error) => failures.push((id, error)),
            }
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;
        Ok(PortfolioPricingResult::new(
            successes,
            failures,
            elapsed_ms,
            aggregations,
        ))
    }

    /// Prices trades in parallel using Rayon.
    fn price_parallel(
        &self,
        trades: &[(TradeId, Trade)],
    ) -> Vec<(TradeId, Result<PricingResult, PricingError>)> {
        trades
            .par_iter()
            .map(|(id, trade)| {
                let result = self.pricer.price_with_config(trade, &self.config);
                (id.clone(), result)
            })
            .collect()
    }

    /// Prices trades sequentially.
    fn price_sequential(
        &self,
        trades: &[(TradeId, Trade)],
    ) -> Vec<(TradeId, Result<PricingResult, PricingError>)> {
        trades
            .iter()
            .map(|(id, trade)| {
                let result = self.pricer.price_with_config(trade, &self.config);
                (id.clone(), result)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generic_pricer::config::{ModelConfigBuilder, PricerConfigBuilder};

    #[test]
    fn test_trade_id() {
        let id = TradeId::new("TRADE-001");
        assert_eq!(id.as_str(), "TRADE-001");
        assert_eq!(format!("{}", id), "TRADE-001");

        let id2: TradeId = "TRADE-002".into();
        assert_eq!(id2.as_str(), "TRADE-002");
    }

    #[test]
    fn test_batch_stats() {
        let stats = BatchStats::new(100, 95, 5, 1000);

        assert_eq!(stats.total_count, 100);
        assert_eq!(stats.success_count, 95);
        assert_eq!(stats.failure_count, 5);
        assert_eq!(stats.elapsed_ms, 1000);
        assert!((stats.success_rate() - 95.0).abs() < 0.01);
        assert!((stats.avg_time_per_trade_ms() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_batch_stats_empty() {
        let stats = BatchStats::new(0, 0, 0, 0);

        assert!((stats.success_rate() - 0.0).abs() < 0.01);
        assert!((stats.avg_time_per_trade_ms() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_batch_pricer_creation() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let batch_pricer = BatchPricer::new(model_config.clone(), pricer_config);

        assert_eq!(
            batch_pricer.model_config().num_paths,
            model_config.num_paths
        );
    }

    #[test]
    fn test_batch_pricing_result_empty() {
        let result = BatchPricingResult::new(vec![], vec![], 0);

        assert!(result.all_succeeded());
        assert!(result.all_failed());
        assert!((result.total_pv() - 0.0).abs() < 1e-10);
    }
}
