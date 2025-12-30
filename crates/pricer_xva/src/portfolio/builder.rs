//! Portfolio builder for constructing validated portfolios.
//!
//! This module provides a builder pattern for creating portfolios
//! with validation of references between entities.

use std::collections::{HashMap, HashSet};

use super::counterparty::Counterparty;
use super::error::PortfolioError;
use super::ids::{CounterpartyId, NettingSetId, TradeId};
use super::netting_set::NettingSet;
use super::trade::Trade;
use super::Portfolio;

/// Builder for constructing portfolios with validation.
///
/// The builder collects trades, counterparties, and netting sets,
/// then validates all references on `build()`.
///
/// # Examples
///
/// ```
/// use pricer_xva::portfolio::{
///     PortfolioBuilder, Trade, TradeId, Counterparty, CounterpartyId,
///     NettingSet, NettingSetId, CreditParams,
/// };
/// use pricer_core::types::Currency;
/// use pricer_models::instruments::{
///     Instrument, VanillaOption, InstrumentParams, PayoffType, ExerciseStyle,
/// };
///
/// // Create entities
/// let credit = CreditParams::new(0.02, 0.4).unwrap();
/// let counterparty = Counterparty::new(CounterpartyId::new("CP001"), credit);
///
/// let mut netting_set = NettingSet::new(
///     NettingSetId::new("NS001"),
///     CounterpartyId::new("CP001"),
/// );
///
/// let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
/// let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
/// let instrument = Instrument::Vanilla(call);
///
/// let trade = Trade::new(
///     TradeId::new("T001"),
///     instrument,
///     Currency::USD,
///     CounterpartyId::new("CP001"),
///     NettingSetId::new("NS001"),
///     1_000_000.0,
/// );
///
/// netting_set.add_trade(TradeId::new("T001"));
///
/// // Build portfolio
/// let portfolio = PortfolioBuilder::new()
///     .add_counterparty(counterparty)
///     .add_netting_set(netting_set)
///     .add_trade(trade)
///     .build()
///     .unwrap();
///
/// assert_eq!(portfolio.trade_count(), 1);
/// ```
#[derive(Default)]
pub struct PortfolioBuilder {
    trades: Vec<Trade>,
    counterparties: Vec<Counterparty>,
    netting_sets: Vec<NettingSet>,
}

impl PortfolioBuilder {
    /// Creates a new portfolio builder.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a trade to the portfolio.
    pub fn add_trade(mut self, trade: Trade) -> Self {
        self.trades.push(trade);
        self
    }

    /// Adds multiple trades to the portfolio.
    pub fn add_trades(mut self, trades: impl IntoIterator<Item = Trade>) -> Self {
        self.trades.extend(trades);
        self
    }

    /// Adds a counterparty to the portfolio.
    pub fn add_counterparty(mut self, counterparty: Counterparty) -> Self {
        self.counterparties.push(counterparty);
        self
    }

    /// Adds multiple counterparties to the portfolio.
    pub fn add_counterparties(
        mut self,
        counterparties: impl IntoIterator<Item = Counterparty>,
    ) -> Self {
        self.counterparties.extend(counterparties);
        self
    }

    /// Adds a netting set to the portfolio.
    pub fn add_netting_set(mut self, netting_set: NettingSet) -> Self {
        self.netting_sets.push(netting_set);
        self
    }

    /// Adds multiple netting sets to the portfolio.
    pub fn add_netting_sets(mut self, netting_sets: impl IntoIterator<Item = NettingSet>) -> Self {
        self.netting_sets.extend(netting_sets);
        self
    }

    /// Builds and validates the portfolio.
    ///
    /// # Validation
    ///
    /// - No duplicate trade IDs
    /// - No duplicate counterparty IDs
    /// - No duplicate netting set IDs
    /// - All trades reference valid counterparties
    /// - All trades reference valid netting sets
    /// - All netting sets reference valid counterparties
    ///
    /// # Errors
    ///
    /// Returns `PortfolioError` if validation fails.
    pub fn build(self) -> Result<Portfolio, PortfolioError> {
        // Check for duplicate trade IDs
        let mut trade_ids = HashSet::new();
        for trade in &self.trades {
            if !trade_ids.insert(trade.id().clone()) {
                return Err(PortfolioError::DuplicateTrade(trade.id().to_string()));
            }
        }

        // Check for duplicate counterparty IDs
        let mut cp_ids = HashSet::new();
        for cp in &self.counterparties {
            if !cp_ids.insert(cp.id().clone()) {
                return Err(PortfolioError::DuplicateCounterparty(cp.id().to_string()));
            }
        }

        // Check for duplicate netting set IDs
        let mut ns_ids = HashSet::new();
        for ns in &self.netting_sets {
            if !ns_ids.insert(ns.id().clone()) {
                return Err(PortfolioError::DuplicateNettingSet(ns.id().to_string()));
            }
        }

        // Validate trade → counterparty references
        for trade in &self.trades {
            if !cp_ids.contains(trade.counterparty_id()) {
                return Err(PortfolioError::UnknownCounterpartyReference(
                    trade.id().to_string(),
                    trade.counterparty_id().to_string(),
                ));
            }
        }

        // Validate trade → netting set references
        for trade in &self.trades {
            if !ns_ids.contains(trade.netting_set_id()) {
                return Err(PortfolioError::UnknownNettingSetReference(
                    trade.id().to_string(),
                    trade.netting_set_id().to_string(),
                ));
            }
        }

        // Validate netting set → counterparty references
        for ns in &self.netting_sets {
            if !cp_ids.contains(ns.counterparty_id()) {
                return Err(PortfolioError::NettingSetUnknownCounterparty(
                    ns.id().to_string(),
                    ns.counterparty_id().to_string(),
                ));
            }
        }

        // Build HashMaps
        let trades: HashMap<TradeId, Trade> = self
            .trades
            .into_iter()
            .map(|t| (t.id().clone(), t))
            .collect();

        let counterparties: HashMap<CounterpartyId, Counterparty> = self
            .counterparties
            .into_iter()
            .map(|c| (c.id().clone(), c))
            .collect();

        let netting_sets: HashMap<NettingSetId, NettingSet> = self
            .netting_sets
            .into_iter()
            .map(|n| (n.id().clone(), n))
            .collect();

        Ok(Portfolio {
            trades,
            counterparties,
            netting_sets,
        })
    }

    /// Returns the number of trades currently in the builder.
    #[inline]
    pub fn trade_count(&self) -> usize {
        self.trades.len()
    }

    /// Returns the number of counterparties currently in the builder.
    #[inline]
    pub fn counterparty_count(&self) -> usize {
        self.counterparties.len()
    }

    /// Returns the number of netting sets currently in the builder.
    #[inline]
    pub fn netting_set_count(&self) -> usize {
        self.netting_sets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::counterparty::CreditParams;
    use pricer_core::types::Currency;
    use pricer_models::instruments::{
        ExerciseStyle, Instrument, InstrumentParams, PayoffType, VanillaOption,
    };

    fn create_test_instrument() -> Instrument<f64> {
        let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
        let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
        Instrument::Vanilla(call)
    }

    fn create_test_counterparty(id: &str) -> Counterparty {
        let credit = CreditParams::new(0.02, 0.4).unwrap();
        Counterparty::new(CounterpartyId::new(id), credit)
    }

    fn create_test_netting_set(id: &str, cp_id: &str) -> NettingSet {
        NettingSet::new(NettingSetId::new(id), CounterpartyId::new(cp_id))
    }

    fn create_test_trade(id: &str, cp_id: &str, ns_id: &str) -> Trade {
        Trade::new(
            TradeId::new(id),
            create_test_instrument(),
            Currency::USD,
            CounterpartyId::new(cp_id),
            NettingSetId::new(ns_id),
            1_000_000.0,
        )
    }

    #[test]
    fn test_builder_empty() {
        let portfolio = PortfolioBuilder::new().build().unwrap();
        assert_eq!(portfolio.trade_count(), 0);
        assert_eq!(portfolio.counterparty_count(), 0);
        assert_eq!(portfolio.netting_set_count(), 0);
    }

    #[test]
    fn test_builder_valid_portfolio() {
        let counterparty = create_test_counterparty("CP001");
        let mut netting_set = create_test_netting_set("NS001", "CP001");
        netting_set.add_trade(TradeId::new("T001"));
        let trade = create_test_trade("T001", "CP001", "NS001");

        let portfolio = PortfolioBuilder::new()
            .add_counterparty(counterparty)
            .add_netting_set(netting_set)
            .add_trade(trade)
            .build()
            .unwrap();

        assert_eq!(portfolio.trade_count(), 1);
        assert_eq!(portfolio.counterparty_count(), 1);
        assert_eq!(portfolio.netting_set_count(), 1);
    }

    #[test]
    fn test_builder_duplicate_trade_id() {
        let counterparty = create_test_counterparty("CP001");
        let netting_set = create_test_netting_set("NS001", "CP001");
        let trade1 = create_test_trade("T001", "CP001", "NS001");
        let trade2 = create_test_trade("T001", "CP001", "NS001"); // Duplicate

        let result = PortfolioBuilder::new()
            .add_counterparty(counterparty)
            .add_netting_set(netting_set)
            .add_trade(trade1)
            .add_trade(trade2)
            .build();

        assert!(matches!(result, Err(PortfolioError::DuplicateTrade(_))));
    }

    #[test]
    fn test_builder_duplicate_counterparty_id() {
        let cp1 = create_test_counterparty("CP001");
        let cp2 = create_test_counterparty("CP001"); // Duplicate

        let result = PortfolioBuilder::new()
            .add_counterparty(cp1)
            .add_counterparty(cp2)
            .build();

        assert!(matches!(
            result,
            Err(PortfolioError::DuplicateCounterparty(_))
        ));
    }

    #[test]
    fn test_builder_duplicate_netting_set_id() {
        let counterparty = create_test_counterparty("CP001");
        let ns1 = create_test_netting_set("NS001", "CP001");
        let ns2 = create_test_netting_set("NS001", "CP001"); // Duplicate

        let result = PortfolioBuilder::new()
            .add_counterparty(counterparty)
            .add_netting_set(ns1)
            .add_netting_set(ns2)
            .build();

        assert!(matches!(
            result,
            Err(PortfolioError::DuplicateNettingSet(_))
        ));
    }

    #[test]
    fn test_builder_unknown_counterparty_reference() {
        let counterparty = create_test_counterparty("CP001");
        let netting_set = create_test_netting_set("NS001", "CP001");
        let trade = create_test_trade("T001", "CP999", "NS001"); // Unknown CP

        let result = PortfolioBuilder::new()
            .add_counterparty(counterparty)
            .add_netting_set(netting_set)
            .add_trade(trade)
            .build();

        assert!(matches!(
            result,
            Err(PortfolioError::UnknownCounterpartyReference(_, _))
        ));
    }

    #[test]
    fn test_builder_unknown_netting_set_reference() {
        let counterparty = create_test_counterparty("CP001");
        let netting_set = create_test_netting_set("NS001", "CP001");
        let trade = create_test_trade("T001", "CP001", "NS999"); // Unknown NS

        let result = PortfolioBuilder::new()
            .add_counterparty(counterparty)
            .add_netting_set(netting_set)
            .add_trade(trade)
            .build();

        assert!(matches!(
            result,
            Err(PortfolioError::UnknownNettingSetReference(_, _))
        ));
    }

    #[test]
    fn test_builder_netting_set_unknown_counterparty() {
        let counterparty = create_test_counterparty("CP001");
        let netting_set = create_test_netting_set("NS001", "CP999"); // Unknown CP

        let result = PortfolioBuilder::new()
            .add_counterparty(counterparty)
            .add_netting_set(netting_set)
            .build();

        assert!(matches!(
            result,
            Err(PortfolioError::NettingSetUnknownCounterparty(_, _))
        ));
    }

    #[test]
    fn test_builder_add_multiple() {
        let cps = vec![
            create_test_counterparty("CP001"),
            create_test_counterparty("CP002"),
        ];
        let nss = vec![
            create_test_netting_set("NS001", "CP001"),
            create_test_netting_set("NS002", "CP002"),
        ];
        let trades = vec![
            create_test_trade("T001", "CP001", "NS001"),
            create_test_trade("T002", "CP002", "NS002"),
        ];

        let portfolio = PortfolioBuilder::new()
            .add_counterparties(cps)
            .add_netting_sets(nss)
            .add_trades(trades)
            .build()
            .unwrap();

        assert_eq!(portfolio.trade_count(), 2);
        assert_eq!(portfolio.counterparty_count(), 2);
        assert_eq!(portfolio.netting_set_count(), 2);
    }

    #[test]
    fn test_builder_counts() {
        let builder = PortfolioBuilder::new()
            .add_counterparty(create_test_counterparty("CP001"))
            .add_netting_set(create_test_netting_set("NS001", "CP001"))
            .add_trade(create_test_trade("T001", "CP001", "NS001"));

        assert_eq!(builder.trade_count(), 1);
        assert_eq!(builder.counterparty_count(), 1);
        assert_eq!(builder.netting_set_count(), 1);
    }
}
