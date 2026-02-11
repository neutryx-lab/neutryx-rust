//! Portfolio builder for constructing validated portfolios.

use std::collections::{HashMap, HashSet};

use super::{
    counterparty::Counterparty,
    error::PortfolioError,
    ids::{CounterpartyId, NettingSetId, TradeId},
    netting_set::NettingSet,
    trade::Trade,
    Portfolio,
};

/// Builder for constructing portfolios with reference validation on build().
#[derive(Default)]
pub struct PortfolioBuilder {
    trades: Vec<Trade>,
    counterparties: Vec<Counterparty>,
    netting_sets: Vec<NettingSet>,
}

impl PortfolioBuilder {
    /// Creates a new portfolio builder.
    #[inline]
    pub fn new() -> Self { Self::default() }

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

    /// Builds and validates the portfolio, checking for duplicate IDs and valid
    /// references.
    pub fn build(self) -> Result<Portfolio, PortfolioError> {
        let mut trade_ids = HashSet::new();
        for trade in &self.trades {
            if !trade_ids.insert(trade.id().clone()) {
                return Err(PortfolioError::DuplicateTrade(trade.id().to_string()));
            }
        }

        let mut cp_ids = HashSet::new();
        for cp in &self.counterparties {
            if !cp_ids.insert(cp.id().clone()) {
                return Err(PortfolioError::DuplicateCounterparty(cp.id().to_string()));
            }
        }

        let mut ns_ids = HashSet::new();
        for ns in &self.netting_sets {
            if !ns_ids.insert(ns.id().clone()) {
                return Err(PortfolioError::DuplicateNettingSet(ns.id().to_string()));
            }
        }

        for trade in &self.trades {
            if !cp_ids.contains(trade.counterparty_id()) {
                return Err(PortfolioError::UnknownCounterpartyReference(
                    trade.id().to_string(),
                    trade.counterparty_id().to_string(),
                ));
            }
        }

        for trade in &self.trades {
            if !ns_ids.contains(trade.netting_set_id()) {
                return Err(PortfolioError::UnknownNettingSetReference(
                    trade.id().to_string(),
                    trade.netting_set_id().to_string(),
                ));
            }
        }

        for ns in &self.netting_sets {
            if !cp_ids.contains(ns.counterparty_id()) {
                return Err(PortfolioError::NettingSetUnknownCounterparty(
                    ns.id().to_string(),
                    ns.counterparty_id().to_string(),
                ));
            }
        }

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
    pub fn trade_count(&self) -> usize { self.trades.len() }

    /// Returns the number of counterparties currently in the builder.
    #[inline]
    pub fn counterparty_count(&self) -> usize { self.counterparties.len() }

    /// Returns the number of netting sets currently in the builder.
    #[inline]
    pub fn netting_set_count(&self) -> usize { self.netting_sets.len() }
}

#[cfg(test)]
mod tests {
    use infra_domain::{
        market::Currency,
        trade::{ExerciseStyle, InstrumentParams, PayoffType, PricingInstrument, VanillaOption},
    };

    use super::*;
    use crate::portfolio::counterparty::CreditParams;

    fn create_test_instrument() -> PricingInstrument<f64> {
        let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
        let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
        PricingInstrument::Vanilla(call)
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
        let trade2 = create_test_trade("T001", "CP001", "NS001");

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
        let cp2 = create_test_counterparty("CP001");

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
        let ns2 = create_test_netting_set("NS001", "CP001");

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
        let trade = create_test_trade("T001", "CP999", "NS001");

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
        let trade = create_test_trade("T001", "CP001", "NS999");

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
        let netting_set = create_test_netting_set("NS001", "CP999");

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
