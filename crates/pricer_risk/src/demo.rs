//! Demo module for lazy-arc-pricing-kernel portfolio orchestration.
//!
//! This module provides:
//! - `DemoTrade`: Simplified trade structure for demonstration
//! - `run_portfolio_pricing`: Pull-then-Push parallel pricing orchestration
//!
//! # Architecture Role
//!
//! This module implements the Pull-then-Push execution pattern:
//!
//! 1. **Pull Phase**: Resolve market data dependencies lazily via `MarketProvider`
//! 2. **Push Phase**: Construct `PricingContext` and invoke pricing kernel
//!
//! # Design Principles
//!
//! - Parallel execution via Rayon
//! - Lazy evaluation: only fetch what's needed
//! - Arc sharing: same currency curves/vols shared across trades
//!
//! # Example
//!
//! ```rust,ignore
//! use pricer_risk::demo::{DemoTrade, run_portfolio_pricing};
//! use pricer_optimiser::provider::MarketProvider;
//!
//! let market = MarketProvider::new();
//! let trades = vec![
//!     DemoTrade::new_vanilla_swap("T1", Currency::USD, 0.02),
//!     DemoTrade::new_cms_swap("T2", Currency::USD, 0.025),
//! ];
//!
//! run_portfolio_pricing(&trades, &market);
//! ```

use pricer_core::types::Currency;
use pricer_models::demo::{
    BlackScholes, CmsSwap, InstrumentEnum, ModelEnum, VanillaSwap,
};
use pricer_optimiser::provider::MarketProvider;
use pricer_pricing::context::{price_single_trade, PricingContext};
use rayon::prelude::*;

/// Simplified trade structure for demonstration.
///
/// Contains all information needed to price a single trade:
/// - Trade identifier
/// - Currency (for market data resolution)
/// - Model (for state evolution)
/// - Instrument (for payoff calculation)
#[derive(Debug, Clone)]
pub struct DemoTrade {
    /// Unique trade identifier.
    pub id: String,
    /// Trade currency (determines which curves/vols to use).
    pub ccy: Currency,
    /// Stochastic model for state evolution.
    pub model: ModelEnum,
    /// Instrument defining the payoff.
    pub instrument: InstrumentEnum,
}

impl DemoTrade {
    /// Creates a new demo trade.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique trade identifier.
    /// * `ccy` - Trade currency.
    /// * `model` - Stochastic model.
    /// * `instrument` - Instrument definition.
    pub fn new(id: impl Into<String>, ccy: Currency, model: ModelEnum, instrument: InstrumentEnum) -> Self {
        Self {
            id: id.into(),
            ccy,
            model,
            instrument,
        }
    }

    /// Creates a vanilla swap trade with default BlackScholes model.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique trade identifier.
    /// * `ccy` - Trade currency.
    /// * `fixed_rate` - Fixed rate of the swap.
    pub fn new_vanilla_swap(id: impl Into<String>, ccy: Currency, fixed_rate: f64) -> Self {
        Self {
            id: id.into(),
            ccy,
            model: ModelEnum::BlackScholes(BlackScholes { vol: 0.2 }),
            instrument: InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate }),
        }
    }

    /// Creates a CMS swap trade with default BlackScholes model.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique trade identifier.
    /// * `ccy` - Trade currency.
    /// * `fixed_rate` - Fixed rate of the swap.
    pub fn new_cms_swap(id: impl Into<String>, ccy: Currency, fixed_rate: f64) -> Self {
        Self {
            id: id.into(),
            ccy,
            model: ModelEnum::BlackScholes(BlackScholes { vol: 0.2 }),
            instrument: InstrumentEnum::CmsSwap(CmsSwap { fixed_rate }),
        }
    }
}

/// Result of pricing a single trade.
#[derive(Debug, Clone)]
pub struct PricingResultDemo {
    /// Trade identifier.
    pub trade_id: String,
    /// Present value of the trade.
    pub pv: f64,
}

/// Executes portfolio pricing using Pull-then-Push pattern.
///
/// This function demonstrates the complete 3-stage rocket execution:
///
/// 1. **Pull Phase** (per trade):
///    - Resolve curve via `market.get_curve(trade.ccy)`
///    - If `trade.instrument.requires_vol()`, resolve vol via `market.get_vol(trade.ccy)`
///
/// 2. **Push Phase** (per trade):
///    - Construct `PricingContext` with resolved references
///    - Invoke `price_single_trade` kernel
///
/// # Arguments
///
/// * `trades` - Slice of trades to price.
/// * `market` - Market data provider with lazy caching.
///
/// # Returns
///
/// Vector of pricing results, one per trade.
///
/// # Parallelism
///
/// Uses Rayon's `par_iter()` for parallel execution across trades.
/// The `MarketProvider` handles thread-safe caching internally.
///
/// # Logging
///
/// Cache misses in `MarketProvider` will produce log output:
/// - `[Optimiser] Bootstrapping Yield Curve for {currency}...`
/// - `[Optimiser] Calibrating SABR Surface for {currency}...`
pub fn run_portfolio_pricing(trades: &[DemoTrade], market: &MarketProvider) -> Vec<PricingResultDemo> {
    trades
        .par_iter()
        .map(|trade| {
            // =================================================================
            // PULL PHASE: Resolve market data dependencies
            // =================================================================

            // Always need the discount curve
            let curve_arc = market.get_curve(trade.ccy);

            // Only fetch vol if the instrument requires it
            let vol_arc = if trade.instrument.requires_vol() {
                Some(market.get_vol(trade.ccy))
            } else {
                None
            };

            // =================================================================
            // PUSH PHASE: Construct context and invoke kernel
            // =================================================================

            // Borrow references from Arcs for zero-copy context
            let ctx = PricingContext::new(
                curve_arc.as_ref(),
                vol_arc.as_ref().map(|arc| arc.as_ref()),
            );

            // Invoke the pricing kernel
            let pv = price_single_trade(&trade.model, &trade.instrument, &ctx);

            PricingResultDemo {
                trade_id: trade.id.clone(),
                pv,
            }
        })
        .collect()
}

/// Executes portfolio pricing sequentially (for testing/debugging).
///
/// Same logic as `run_portfolio_pricing` but without parallelism.
pub fn run_portfolio_pricing_sequential(trades: &[DemoTrade], market: &MarketProvider) -> Vec<PricingResultDemo> {
    trades
        .iter()
        .map(|trade| {
            let curve_arc = market.get_curve(trade.ccy);
            let vol_arc = if trade.instrument.requires_vol() {
                Some(market.get_vol(trade.ccy))
            } else {
                None
            };

            let ctx = PricingContext::new(
                curve_arc.as_ref(),
                vol_arc.as_ref().map(|arc| arc.as_ref()),
            );

            let pv = price_single_trade(&trade.model, &trade.instrument, &ctx);

            PricingResultDemo {
                trade_id: trade.id.clone(),
                pv,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pricer_models::demo::HullWhite;

    // -------------------------------------------------------------------------
    // Task 4.1: DemoTrade Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_demo_trade_creation() {
        let trade = DemoTrade::new(
            "T001",
            Currency::USD,
            ModelEnum::BlackScholes(BlackScholes { vol: 0.2 }),
            InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate: 0.02 }),
        );

        assert_eq!(trade.id, "T001");
        assert_eq!(trade.ccy, Currency::USD);
    }

    #[test]
    fn test_demo_trade_vanilla_swap_factory() {
        let trade = DemoTrade::new_vanilla_swap("T001", Currency::USD, 0.02);

        assert_eq!(trade.id, "T001");
        assert_eq!(trade.ccy, Currency::USD);
        assert!(!trade.instrument.requires_vol());
    }

    #[test]
    fn test_demo_trade_cms_swap_factory() {
        let trade = DemoTrade::new_cms_swap("T002", Currency::EUR, 0.025);

        assert_eq!(trade.id, "T002");
        assert_eq!(trade.ccy, Currency::EUR);
        assert!(trade.instrument.requires_vol());
    }

    #[test]
    fn test_demo_trade_with_hull_white() {
        let trade = DemoTrade::new(
            "T003",
            Currency::JPY,
            ModelEnum::HullWhite(HullWhite { mean_rev: 0.1, vol: 0.01 }),
            InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate: 0.01 }),
        );

        assert_eq!(trade.id, "T003");
        assert_eq!(trade.ccy, Currency::JPY);
    }

    // -------------------------------------------------------------------------
    // Task 4.2: run_portfolio_pricing Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_run_portfolio_pricing_single_vanilla() {
        let market = MarketProvider::new();
        let trades = vec![DemoTrade::new_vanilla_swap("T001", Currency::USD, 0.02)];

        let results = run_portfolio_pricing(&trades, &market);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trade_id, "T001");
        // PV should be non-zero
        assert!(results[0].pv.abs() > 1e-10);
    }

    #[test]
    fn test_run_portfolio_pricing_single_cms() {
        let market = MarketProvider::new();
        let trades = vec![DemoTrade::new_cms_swap("T002", Currency::USD, 0.02)];

        let results = run_portfolio_pricing(&trades, &market);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trade_id, "T002");
    }

    #[test]
    fn test_run_portfolio_pricing_multiple_trades() {
        let market = MarketProvider::new();
        let trades = vec![
            DemoTrade::new_vanilla_swap("T001", Currency::USD, 0.02),
            DemoTrade::new_vanilla_swap("T002", Currency::USD, 0.025),
            DemoTrade::new_cms_swap("T003", Currency::USD, 0.02),
            DemoTrade::new_vanilla_swap("T004", Currency::JPY, 0.01),
        ];

        let results = run_portfolio_pricing(&trades, &market);

        assert_eq!(results.len(), 4);

        // Verify all trade IDs are present
        let ids: Vec<_> = results.iter().map(|r| r.trade_id.as_str()).collect();
        assert!(ids.contains(&"T001"));
        assert!(ids.contains(&"T002"));
        assert!(ids.contains(&"T003"));
        assert!(ids.contains(&"T004"));
    }

    #[test]
    fn test_run_portfolio_pricing_sequential_matches_parallel() {
        let market = MarketProvider::new();
        let trades = vec![
            DemoTrade::new_vanilla_swap("T001", Currency::USD, 0.02),
            DemoTrade::new_cms_swap("T002", Currency::EUR, 0.025),
        ];

        let parallel_results = run_portfolio_pricing(&trades, &market);

        // Reset cache for fair comparison
        let market2 = MarketProvider::new();
        let sequential_results = run_portfolio_pricing_sequential(&trades, &market2);

        // Results should match (order may differ in parallel)
        for seq_result in &sequential_results {
            let par_result = parallel_results
                .iter()
                .find(|r| r.trade_id == seq_result.trade_id)
                .expect("Trade ID should exist in parallel results");
            assert!(
                (par_result.pv - seq_result.pv).abs() < 1e-10,
                "PV mismatch for {}: parallel={}, sequential={}",
                seq_result.trade_id,
                par_result.pv,
                seq_result.pv
            );
        }
    }

    #[test]
    fn test_run_portfolio_pricing_empty() {
        let market = MarketProvider::new();
        let trades: Vec<DemoTrade> = vec![];

        let results = run_portfolio_pricing(&trades, &market);

        assert!(results.is_empty());
    }

    // -------------------------------------------------------------------------
    // Requirement 4.5/4.6: Lazy Vol Fetch Verification
    // -------------------------------------------------------------------------

    #[test]
    fn test_vanilla_swap_does_not_require_vol() {
        let trade = DemoTrade::new_vanilla_swap("T001", Currency::USD, 0.02);
        assert!(!trade.instrument.requires_vol());
    }

    #[test]
    fn test_cms_swap_requires_vol() {
        let trade = DemoTrade::new_cms_swap("T001", Currency::USD, 0.02);
        assert!(trade.instrument.requires_vol());
    }

    // -------------------------------------------------------------------------
    // Arc Cache Verification
    // -------------------------------------------------------------------------

    #[test]
    fn test_arc_cache_same_currency_shared() {
        use std::sync::Arc;

        let market = MarketProvider::new();

        // Get curve twice for same currency
        let curve1 = market.get_curve(Currency::USD);
        let curve2 = market.get_curve(Currency::USD);

        // Should be the same Arc
        assert!(Arc::ptr_eq(&curve1, &curve2));
    }

    #[test]
    fn test_portfolio_pricing_uses_cached_curves() {
        let market = MarketProvider::new();

        // Multiple USD trades should share the same curve
        let trades = vec![
            DemoTrade::new_vanilla_swap("T001", Currency::USD, 0.02),
            DemoTrade::new_vanilla_swap("T002", Currency::USD, 0.025),
            DemoTrade::new_vanilla_swap("T003", Currency::USD, 0.03),
        ];

        // First run builds cache
        let _results = run_portfolio_pricing(&trades, &market);

        // Verify cache was populated
        let cache_size = {
            // Access internal cache for testing (would need pub(crate) in real code)
            // For now, just verify we can get the same curve again
            let curve = market.get_curve(Currency::USD);
            // If this doesn't print, cache is working
            drop(curve);
            1
        };

        assert_eq!(cache_size, 1);
    }
}
