//! Batch pricing for multiple trades.
//!
//! This module provides parallel batch pricing capabilities using Rayon.
//! Features:
//! - Parallel pricing of multiple trades
//! - Partial error continuation (failed trades don't stop processing)
//! - Arc-cached market data sharing
//! - Processing statistics

use std::time::Instant;

use rayon::prelude::*;

use super::config::{ModelConfig, PricerConfig};
use super::error::PricingError;
use super::pricer::GenericPricer;
use super::result::PricingResult;

#[cfg(not(feature = "l1l2-integration"))]
use super::config::DefaultCurrency as Currency;

#[cfg(not(feature = "l1l2-integration"))]
use super::pricer::SimpleLeg;

#[cfg(not(feature = "l1l2-integration"))]
use super::result::Date;

/// Unique identifier for a trade.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TradeId(pub String);

impl TradeId {
    /// Creates a new trade ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TradeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TradeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for TradeId {
    fn from(s: String) -> Self {
        Self(s)
    }
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
    pub successes: Vec<(TradeId, PricingResult)>,
    /// Failed trades with their errors.
    pub failures: Vec<(TradeId, PricingError)>,
    /// Processing statistics.
    pub stats: BatchStats,
}

impl BatchPricingResult {
    /// Creates a new batch pricing result.
    pub fn new(
        successes: Vec<(TradeId, PricingResult)>,
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
    pub fn all_succeeded(&self) -> bool {
        self.failures.is_empty()
    }

    /// Returns true if all trades failed.
    pub fn all_failed(&self) -> bool {
        self.successes.is_empty()
    }

    /// Returns the total PV across all successful trades.
    pub fn total_pv(&self) -> f64 {
        self.successes.iter().map(|(_, r)| r.total_pv).sum()
    }

    /// Gets the result for a specific trade ID.
    pub fn get(&self, trade_id: &TradeId) -> Option<Result<&PricingResult, &PricingError>> {
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
    pub fn model_config(&self) -> &ModelConfig {
        &self.model_config
    }

    /// Returns the pricer configuration.
    pub fn pricer_config(&self) -> &PricerConfig {
        &self.pricer_config
    }
}

/// Simple trade wrapper for standalone mode.
#[cfg(not(feature = "l1l2-integration"))]
#[derive(Debug, Clone)]
pub struct SimpleTrade {
    /// Trade identifier.
    pub id: TradeId,
    /// Legs of the trade.
    pub legs: Vec<SimpleLeg>,
}

#[cfg(not(feature = "l1l2-integration"))]
impl SimpleTrade {
    /// Creates a new simple trade.
    pub fn new(id: impl Into<TradeId>, legs: Vec<SimpleLeg>) -> Self {
        Self {
            id: id.into(),
            legs,
        }
    }
}

#[cfg(not(feature = "l1l2-integration"))]
impl BatchPricer {
    /// Prices a batch of trades in parallel.
    ///
    /// Uses Rayon's parallel iterator for concurrent processing.
    /// Failed trades don't stop processing of other trades.
    ///
    /// # Arguments
    ///
    /// * `trades` - Slice of trades to price
    /// * `valuation_date` - Common valuation date for all trades
    /// * `reporting_currency` - Common reporting currency for all trades
    ///
    /// # Returns
    ///
    /// `BatchPricingResult` containing successes, failures, and statistics.
    pub fn price_batch(
        &self,
        trades: &[SimpleTrade],
        valuation_date: Date,
        reporting_currency: Currency,
    ) -> BatchPricingResult {
        let start = Instant::now();

        // Create pricer for each trade (they share the same config)
        let pricer = GenericPricer::new(self.model_config.clone(), self.pricer_config.clone());

        // Process trades in parallel
        let results: Vec<(TradeId, Result<PricingResult, PricingError>)> = trades
            .par_iter()
            .map(|trade| {
                let result = pricer.get_pv_simple(
                    trade.legs.clone(),
                    valuation_date,
                    reporting_currency,
                );
                (trade.id.clone(), result)
            })
            .collect();

        // Partition into successes and failures
        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for (id, result) in results {
            match result {
                Ok(pricing_result) => successes.push((id, pricing_result)),
                Err(error) => failures.push((id, error)),
            }
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;
        BatchPricingResult::new(successes, failures, elapsed_ms)
    }

    /// Prices a batch of trades sequentially (for comparison/debugging).
    pub fn price_batch_sequential(
        &self,
        trades: &[SimpleTrade],
        valuation_date: Date,
        reporting_currency: Currency,
    ) -> BatchPricingResult {
        let start = Instant::now();

        let pricer = GenericPricer::new(self.model_config.clone(), self.pricer_config.clone());

        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for trade in trades {
            let result = pricer.get_pv_simple(
                trade.legs.clone(),
                valuation_date,
                reporting_currency,
            );

            match result {
                Ok(pricing_result) => successes.push((trade.id.clone(), pricing_result)),
                Err(error) => failures.push((trade.id.clone(), error)),
            }
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;
        BatchPricingResult::new(successes, failures, elapsed_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generic_pricer::config::{ModelConfigBuilder, PricerConfigBuilder};

    #[cfg(not(feature = "l1l2-integration"))]
    use crate::generic_pricer::pricer::SimpleCashflow;

    #[cfg(not(feature = "l1l2-integration"))]
    use crate::generic_pricer::result::Direction;

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

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_batch_pricer_creation() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let batch_pricer = BatchPricer::new(model_config.clone(), pricer_config);

        assert_eq!(batch_pricer.model_config().num_paths, model_config.num_paths);
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_batch_pricing_parallel() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let batch_pricer = BatchPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(0);
        let payment_date = Date::from_days(365);

        // Create multiple trades
        let trades: Vec<SimpleTrade> = (0..10)
            .map(|i| {
                let leg = SimpleLeg {
                    currency: Currency::USD,
                    direction: Direction::Receiver,
                    cashflows: vec![SimpleCashflow {
                        payment_date,
                        amount: 100_000.0 * (i + 1) as f64,
                    }],
                };
                SimpleTrade::new(format!("TRADE-{:03}", i), vec![leg])
            })
            .collect();

        let result = batch_pricer.price_batch(&trades, valuation_date, Currency::USD);

        assert!(result.all_succeeded());
        assert_eq!(result.stats.total_count, 10);
        assert_eq!(result.stats.success_count, 10);
        assert_eq!(result.stats.failure_count, 0);
        assert!((result.stats.success_rate() - 100.0).abs() < 0.01);
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_batch_pricing_with_failures() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let batch_pricer = BatchPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(0);
        let payment_date = Date::from_days(365);

        // Create trades - some with unsupported currency
        let trades: Vec<SimpleTrade> = vec![
            SimpleTrade::new(
                "TRADE-001",
                vec![SimpleLeg {
                    currency: Currency::USD,
                    direction: Direction::Receiver,
                    cashflows: vec![SimpleCashflow {
                        payment_date,
                        amount: 100_000.0,
                    }],
                }],
            ),
            SimpleTrade::new(
                "TRADE-002",
                vec![SimpleLeg {
                    currency: Currency::CHF, // Will fail - no FX rate
                    direction: Direction::Receiver,
                    cashflows: vec![SimpleCashflow {
                        payment_date,
                        amount: 100_000.0,
                    }],
                }],
            ),
            SimpleTrade::new(
                "TRADE-003",
                vec![SimpleLeg {
                    currency: Currency::EUR,
                    direction: Direction::Receiver,
                    cashflows: vec![SimpleCashflow {
                        payment_date,
                        amount: 100_000.0,
                    }],
                }],
            ),
        ];

        let result = batch_pricer.price_batch(&trades, valuation_date, Currency::USD);

        // Should have partial success
        assert!(!result.all_succeeded());
        assert!(!result.all_failed());
        assert_eq!(result.stats.total_count, 3);
        assert_eq!(result.stats.success_count, 2);
        assert_eq!(result.stats.failure_count, 1);

        // Check specific results
        assert!(result.get(&TradeId::new("TRADE-001")).unwrap().is_ok());
        assert!(result.get(&TradeId::new("TRADE-002")).unwrap().is_err());
        assert!(result.get(&TradeId::new("TRADE-003")).unwrap().is_ok());
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_batch_pricing_sequential() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let batch_pricer = BatchPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(0);
        let payment_date = Date::from_days(365);

        let trades: Vec<SimpleTrade> = (0..5)
            .map(|i| {
                let leg = SimpleLeg {
                    currency: Currency::USD,
                    direction: Direction::Receiver,
                    cashflows: vec![SimpleCashflow {
                        payment_date,
                        amount: 100_000.0,
                    }],
                };
                SimpleTrade::new(format!("TRADE-{:03}", i), vec![leg])
            })
            .collect();

        let result = batch_pricer.price_batch_sequential(&trades, valuation_date, Currency::USD);

        assert!(result.all_succeeded());
        assert_eq!(result.stats.total_count, 5);
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_batch_pricing_total_pv() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let batch_pricer = BatchPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(0);
        let payment_date = Date::from_days(365);

        // Create two trades with same cashflow
        let trades: Vec<SimpleTrade> = vec![
            SimpleTrade::new(
                "TRADE-001",
                vec![SimpleLeg {
                    currency: Currency::USD,
                    direction: Direction::Receiver,
                    cashflows: vec![SimpleCashflow {
                        payment_date,
                        amount: 100_000.0,
                    }],
                }],
            ),
            SimpleTrade::new(
                "TRADE-002",
                vec![SimpleLeg {
                    currency: Currency::USD,
                    direction: Direction::Receiver,
                    cashflows: vec![SimpleCashflow {
                        payment_date,
                        amount: 100_000.0,
                    }],
                }],
            ),
        ];

        let result = batch_pricer.price_batch(&trades, valuation_date, Currency::USD);

        // Total PV should be roughly 2 * 95,123 ≈ 190,246
        assert!(result.total_pv() > 190_000.0 && result.total_pv() < 192_000.0);
    }

    #[test]
    fn test_batch_pricing_result_empty() {
        let result = BatchPricingResult::new(vec![], vec![], 0);

        assert!(result.all_succeeded());
        assert!(result.all_failed());
        assert!((result.total_pv() - 0.0).abs() < 1e-10);
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_batch_pricing_result_get() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let batch_pricer = BatchPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(0);
        let payment_date = Date::from_days(365);

        let trades: Vec<SimpleTrade> = vec![SimpleTrade::new(
            "TRADE-001",
            vec![SimpleLeg {
                currency: Currency::USD,
                direction: Direction::Receiver,
                cashflows: vec![SimpleCashflow {
                    payment_date,
                    amount: 100_000.0,
                }],
            }],
        )];

        let result = batch_pricer.price_batch(&trades, valuation_date, Currency::USD);

        // Check existing trade
        let trade_result = result.get(&TradeId::new("TRADE-001"));
        assert!(trade_result.is_some());
        assert!(trade_result.unwrap().is_ok());

        // Check non-existing trade
        let missing = result.get(&TradeId::new("TRADE-999"));
        assert!(missing.is_none());
    }
}
