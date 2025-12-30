//! Portfolio and trade structures for XVA calculations.
//!
//! This module provides:
//! - Trade structures with instrument references
//! - Counterparty definitions with credit parameters
//! - Netting sets for exposure aggregation
//! - Portfolio container with parallel iteration support
//!
//! # Architecture
//!
//! The portfolio module is organised around three core entities:
//!
//! - **Trade**: Individual financial instrument with metadata
//! - **Counterparty**: Credit entity with default risk parameters
//! - **NettingSet**: Group of trades for exposure netting
//!
//! # Examples
//!
//! ```
//! use pricer_xva::portfolio::{
//!     PortfolioBuilder, Trade, TradeId, Counterparty, CounterpartyId,
//!     NettingSet, NettingSetId, CreditParams,
//! };
//! use pricer_core::types::Currency;
//! use pricer_models::instruments::{
//!     Instrument, VanillaOption, InstrumentParams, PayoffType, ExerciseStyle,
//! };
//!
//! // Create counterparty with credit parameters
//! let credit = CreditParams::new(0.02, 0.4).unwrap();
//! let counterparty = Counterparty::new(CounterpartyId::new("CP001"), credit)
//!     .with_name("Acme Corp");
//!
//! // Create netting set
//! let mut netting_set = NettingSet::new(
//!     NettingSetId::new("NS001"),
//!     CounterpartyId::new("CP001"),
//! );
//!
//! // Create trade
//! let params = InstrumentParams::new(100.0, 1.0, 1_000_000.0).unwrap();
//! let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
//! let instrument = Instrument::Vanilla(call);
//!
//! let trade = Trade::new(
//!     TradeId::new("T001"),
//!     instrument,
//!     Currency::USD,
//!     CounterpartyId::new("CP001"),
//!     NettingSetId::new("NS001"),
//!     1.0,
//! );
//!
//! netting_set.add_trade(TradeId::new("T001"));
//!
//! // Build portfolio
//! let portfolio = PortfolioBuilder::new()
//!     .add_counterparty(counterparty)
//!     .add_netting_set(netting_set)
//!     .add_trade(trade)
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(portfolio.trade_count(), 1);
//! ```

mod builder;
mod counterparty;
mod error;
mod ids;
mod netting_set;
mod trade;

// Re-export public types
pub use builder::PortfolioBuilder;
pub use counterparty::{Counterparty, CreditParams, CreditRating};
pub use error::PortfolioError;
pub use ids::{CounterpartyId, NettingSetId, TradeId};
pub use netting_set::{CollateralAgreement, NettingSet};
pub use trade::{Trade, TradeBuilder};

use std::collections::HashMap;

use rayon::prelude::*;

/// Portfolio container for trades, counterparties, and netting sets.
///
/// Provides O(1) lookup by ID and supports parallel iteration via Rayon.
///
/// # Examples
///
/// ```
/// use pricer_xva::portfolio::{
///     Portfolio, PortfolioBuilder, Trade, TradeId, Counterparty, CounterpartyId,
///     NettingSet, NettingSetId, CreditParams,
/// };
/// use pricer_core::types::Currency;
/// use pricer_models::instruments::{
///     Instrument, VanillaOption, InstrumentParams, PayoffType, ExerciseStyle,
/// };
///
/// let credit = CreditParams::new(0.02, 0.4).unwrap();
/// let counterparty = Counterparty::new(CounterpartyId::new("CP001"), credit);
///
/// let netting_set = NettingSet::new(
///     NettingSetId::new("NS001"),
///     CounterpartyId::new("CP001"),
/// );
///
/// let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
/// let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
///
/// let trade = Trade::new(
///     TradeId::new("T001"),
///     Instrument::Vanilla(call),
///     Currency::USD,
///     CounterpartyId::new("CP001"),
///     NettingSetId::new("NS001"),
///     1_000_000.0,
/// );
///
/// let portfolio = PortfolioBuilder::new()
///     .add_counterparty(counterparty)
///     .add_netting_set(netting_set)
///     .add_trade(trade)
///     .build()
///     .unwrap();
///
/// // Access by ID
/// let t = portfolio.trade(&TradeId::new("T001"));
/// assert!(t.is_some());
/// ```
#[derive(Debug)]
pub struct Portfolio {
    trades: HashMap<TradeId, Trade>,
    counterparties: HashMap<CounterpartyId, Counterparty>,
    netting_sets: HashMap<NettingSetId, NettingSet>,
}

impl Portfolio {
    /// Returns the number of trades in the portfolio.
    #[inline]
    pub fn trade_count(&self) -> usize {
        self.trades.len()
    }

    /// Returns the number of counterparties in the portfolio.
    #[inline]
    pub fn counterparty_count(&self) -> usize {
        self.counterparties.len()
    }

    /// Returns the number of netting sets in the portfolio.
    #[inline]
    pub fn netting_set_count(&self) -> usize {
        self.netting_sets.len()
    }

    /// Returns whether the portfolio is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.trades.is_empty()
    }

    /// Gets a trade by ID.
    #[inline]
    pub fn trade(&self, id: &TradeId) -> Option<&Trade> {
        self.trades.get(id)
    }

    /// Gets a counterparty by ID.
    #[inline]
    pub fn counterparty(&self, id: &CounterpartyId) -> Option<&Counterparty> {
        self.counterparties.get(id)
    }

    /// Gets a netting set by ID.
    #[inline]
    pub fn netting_set(&self, id: &NettingSetId) -> Option<&NettingSet> {
        self.netting_sets.get(id)
    }

    /// Returns an iterator over all trades.
    #[inline]
    pub fn trades(&self) -> impl Iterator<Item = &Trade> {
        self.trades.values()
    }

    /// Returns an iterator over all counterparties.
    #[inline]
    pub fn counterparties(&self) -> impl Iterator<Item = &Counterparty> {
        self.counterparties.values()
    }

    /// Returns an iterator over all netting sets.
    #[inline]
    pub fn netting_sets(&self) -> impl Iterator<Item = &NettingSet> {
        self.netting_sets.values()
    }

    /// Returns an iterator over trade IDs.
    #[inline]
    pub fn trade_ids(&self) -> impl Iterator<Item = &TradeId> {
        self.trades.keys()
    }

    /// Returns an iterator over counterparty IDs.
    #[inline]
    pub fn counterparty_ids(&self) -> impl Iterator<Item = &CounterpartyId> {
        self.counterparties.keys()
    }

    /// Returns an iterator over netting set IDs.
    #[inline]
    pub fn netting_set_ids(&self) -> impl Iterator<Item = &NettingSetId> {
        self.netting_sets.keys()
    }

    /// Gets all trades in a netting set.
    ///
    /// # Arguments
    ///
    /// * `ns_id` - Netting set ID
    ///
    /// # Returns
    ///
    /// Vector of trade references in the netting set.
    pub fn trades_in_netting_set(&self, ns_id: &NettingSetId) -> Vec<&Trade> {
        self.netting_sets
            .get(ns_id)
            .map(|ns| {
                ns.trade_ids()
                    .iter()
                    .filter_map(|tid| self.trades.get(tid))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets all trades for a counterparty.
    ///
    /// # Arguments
    ///
    /// * `cp_id` - Counterparty ID
    ///
    /// # Returns
    ///
    /// Vector of trade references for the counterparty.
    pub fn trades_for_counterparty(&self, cp_id: &CounterpartyId) -> Vec<&Trade> {
        self.trades
            .values()
            .filter(|t| t.counterparty_id() == cp_id)
            .collect()
    }

    /// Gets all netting sets for a counterparty.
    pub fn netting_sets_for_counterparty(&self, cp_id: &CounterpartyId) -> Vec<&NettingSet> {
        self.netting_sets
            .values()
            .filter(|ns| ns.counterparty_id() == cp_id)
            .collect()
    }

    /// Returns a parallel iterator over trades.
    ///
    /// Uses Rayon for parallel iteration across multiple threads.
    #[inline]
    pub fn trades_par_iter(&self) -> impl ParallelIterator<Item = (&TradeId, &Trade)> {
        self.trades.par_iter()
    }

    /// Returns a parallel iterator over netting sets.
    #[inline]
    pub fn netting_sets_par_iter(
        &self,
    ) -> impl ParallelIterator<Item = (&NettingSetId, &NettingSet)> {
        self.netting_sets.par_iter()
    }

    /// Returns a parallel iterator over counterparties.
    #[inline]
    pub fn counterparties_par_iter(
        &self,
    ) -> impl ParallelIterator<Item = (&CounterpartyId, &Counterparty)> {
        self.counterparties.par_iter()
    }

    /// Prices all trades in parallel using the provided pricing function.
    ///
    /// # Arguments
    ///
    /// * `pricer_fn` - Function that takes a trade reference and returns a price
    ///
    /// # Returns
    ///
    /// HashMap mapping trade IDs to prices.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let prices = portfolio.price_all_trades(|trade| {
    ///     // Use MC pricer or analytical formula
    ///     10.0 // Placeholder price
    /// });
    /// ```
    pub fn price_all_trades<F>(&self, pricer_fn: F) -> HashMap<TradeId, f64>
    where
        F: Fn(&Trade) -> f64 + Sync,
    {
        self.trades
            .par_iter()
            .map(|(id, trade)| (id.clone(), pricer_fn(trade)))
            .collect()
    }

    /// Aggregates values by netting set in parallel.
    ///
    /// # Arguments
    ///
    /// * `agg_fn` - Function that takes a slice of trades and returns an aggregated value
    ///
    /// # Returns
    ///
    /// HashMap mapping netting set IDs to aggregated values.
    pub fn aggregate_by_netting_set<F>(&self, agg_fn: F) -> HashMap<NettingSetId, f64>
    where
        F: Fn(&[&Trade]) -> f64 + Sync,
    {
        self.netting_sets
            .par_iter()
            .map(|(ns_id, ns)| {
                let trades: Vec<&Trade> = ns
                    .trade_ids()
                    .iter()
                    .filter_map(|tid| self.trades.get(tid))
                    .collect();
                (ns_id.clone(), agg_fn(&trades))
            })
            .collect()
    }

    /// Computes total notional by currency.
    pub fn notional_by_currency(&self) -> HashMap<pricer_core::types::Currency, f64> {
        use pricer_core::types::Currency;
        let mut result: HashMap<Currency, f64> = HashMap::new();
        for trade in self.trades.values() {
            *result.entry(trade.currency()).or_insert(0.0) += trade.notional();
        }
        result
    }

    /// Computes total notional for the portfolio.
    pub fn total_notional(&self) -> f64 {
        self.trades.values().map(|t| t.notional()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pricer_core::types::Currency;
    use pricer_models::instruments::{
        ExerciseStyle, Instrument, InstrumentParams, PayoffType, VanillaOption,
    };

    fn create_test_instrument() -> Instrument<f64> {
        let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
        let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
        Instrument::Vanilla(call)
    }

    fn create_test_portfolio() -> Portfolio {
        let credit1 = CreditParams::new(0.02, 0.4).unwrap();
        let credit2 = CreditParams::new(0.03, 0.5).unwrap();

        let cp1 = Counterparty::new(CounterpartyId::new("CP001"), credit1);
        let cp2 = Counterparty::new(CounterpartyId::new("CP002"), credit2);

        let mut ns1 = NettingSet::new(NettingSetId::new("NS001"), CounterpartyId::new("CP001"));
        let mut ns2 = NettingSet::new(NettingSetId::new("NS002"), CounterpartyId::new("CP002"));

        ns1.add_trade(TradeId::new("T001"));
        ns1.add_trade(TradeId::new("T002"));
        ns2.add_trade(TradeId::new("T003"));

        let t1 = Trade::new(
            TradeId::new("T001"),
            create_test_instrument(),
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            1_000_000.0,
        );
        let t2 = Trade::new(
            TradeId::new("T002"),
            create_test_instrument(),
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            2_000_000.0,
        );
        let t3 = Trade::new(
            TradeId::new("T003"),
            create_test_instrument(),
            Currency::EUR,
            CounterpartyId::new("CP002"),
            NettingSetId::new("NS002"),
            500_000.0,
        );

        PortfolioBuilder::new()
            .add_counterparties(vec![cp1, cp2])
            .add_netting_sets(vec![ns1, ns2])
            .add_trades(vec![t1, t2, t3])
            .build()
            .unwrap()
    }

    #[test]
    fn test_portfolio_counts() {
        let portfolio = create_test_portfolio();
        assert_eq!(portfolio.trade_count(), 3);
        assert_eq!(portfolio.counterparty_count(), 2);
        assert_eq!(portfolio.netting_set_count(), 2);
    }

    #[test]
    fn test_portfolio_lookup() {
        let portfolio = create_test_portfolio();

        let trade = portfolio.trade(&TradeId::new("T001"));
        assert!(trade.is_some());
        assert_eq!(trade.unwrap().notional(), 1_000_000.0);

        let cp = portfolio.counterparty(&CounterpartyId::new("CP001"));
        assert!(cp.is_some());

        let ns = portfolio.netting_set(&NettingSetId::new("NS001"));
        assert!(ns.is_some());
        assert_eq!(ns.unwrap().trade_count(), 2);
    }

    #[test]
    fn test_portfolio_lookup_not_found() {
        let portfolio = create_test_portfolio();

        assert!(portfolio.trade(&TradeId::new("T999")).is_none());
        assert!(portfolio
            .counterparty(&CounterpartyId::new("CP999"))
            .is_none());
        assert!(portfolio.netting_set(&NettingSetId::new("NS999")).is_none());
    }

    #[test]
    fn test_trades_in_netting_set() {
        let portfolio = create_test_portfolio();
        let trades = portfolio.trades_in_netting_set(&NettingSetId::new("NS001"));
        assert_eq!(trades.len(), 2);
    }

    #[test]
    fn test_trades_for_counterparty() {
        let portfolio = create_test_portfolio();
        let trades = portfolio.trades_for_counterparty(&CounterpartyId::new("CP001"));
        assert_eq!(trades.len(), 2);
    }

    #[test]
    fn test_netting_sets_for_counterparty() {
        let portfolio = create_test_portfolio();
        let nss = portfolio.netting_sets_for_counterparty(&CounterpartyId::new("CP001"));
        assert_eq!(nss.len(), 1);
    }

    #[test]
    fn test_price_all_trades() {
        let portfolio = create_test_portfolio();

        let prices = portfolio.price_all_trades(|trade| trade.notional() * 0.01);

        assert_eq!(prices.len(), 3);
        assert_eq!(prices.get(&TradeId::new("T001")), Some(&10_000.0));
    }

    #[test]
    fn test_aggregate_by_netting_set() {
        let portfolio = create_test_portfolio();

        let agg = portfolio.aggregate_by_netting_set(|trades| {
            trades.iter().map(|t| t.notional()).sum()
        });

        assert_eq!(agg.get(&NettingSetId::new("NS001")), Some(&3_000_000.0));
        assert_eq!(agg.get(&NettingSetId::new("NS002")), Some(&500_000.0));
    }

    #[test]
    fn test_notional_by_currency() {
        let portfolio = create_test_portfolio();
        let notionals = portfolio.notional_by_currency();

        assert_eq!(notionals.get(&Currency::USD), Some(&3_000_000.0));
        assert_eq!(notionals.get(&Currency::EUR), Some(&500_000.0));
    }

    #[test]
    fn test_total_notional() {
        let portfolio = create_test_portfolio();
        assert_eq!(portfolio.total_notional(), 3_500_000.0);
    }

    #[test]
    fn test_iterators() {
        let portfolio = create_test_portfolio();

        assert_eq!(portfolio.trades().count(), 3);
        assert_eq!(portfolio.counterparties().count(), 2);
        assert_eq!(portfolio.netting_sets().count(), 2);
        assert_eq!(portfolio.trade_ids().count(), 3);
    }

    #[test]
    fn test_parallel_iteration() {
        let portfolio = create_test_portfolio();

        // Ensure parallel iteration works
        let total: f64 = portfolio
            .trades_par_iter()
            .map(|(_, t)| t.notional())
            .sum();

        assert_eq!(total, 3_500_000.0);
    }

    #[test]
    fn test_is_empty() {
        let empty = PortfolioBuilder::new().build().unwrap();
        assert!(empty.is_empty());

        let non_empty = create_test_portfolio();
        assert!(!non_empty.is_empty());
    }
}
