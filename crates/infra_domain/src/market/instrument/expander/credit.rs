//! Credit instrument expansion implementations.
//!
//! Covers: Cds, CdsIndex, CdsOption, NtdBasket.

use super::{settlement_trade, InstrumentExpander};
use crate::{
    ids::TradeId,
    market::{
        convention::ConventionSet,
        instrument::{Cds, CdsIndex, CdsOption, InstrumentError, NtdBasket},
    },
    time::Date,
    trade::{Cashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade, TradeType},
};

impl InstrumentExpander for Cds {
    fn expand_to_trade(&self, trade_id: impl Into<TradeId>, _vd: Date, conventions: &ConventionSet) -> Result<Trade, InstrumentError> {
        let _cds_conv = conventions.get_cds()?;

        // Premium leg: periodic spread payments
        let premium_cf = Cashflow::new(
            CashflowType::Coupon, self.maturity, self.start_date, self.maturity,
            1.0, self.notional, Payoff::fixed(self.spread), self.currency,
        );
        let premium_leg = Leg::new(vec![premium_cf], Direction::Payer, LegType::Fixed, self.currency);

        // Protection leg: contingent payment on default
        let protection_cf = Cashflow::new(
            CashflowType::Settlement, self.maturity, self.start_date, self.maturity,
            0.0, self.notional * (1.0 - self.recovery_rate.unwrap_or(0.4)), Payoff::fixed(1.0), self.currency,
        );
        let protection_leg = Leg::new(vec![protection_cf], Direction::Receiver, LegType::Generic, self.currency);

        Ok(Trade::new(trade_id, vec![premium_leg, protection_leg], TradeType::Swap))
    }
}

impl InstrumentExpander for CdsIndex {
    fn expand_to_trade(&self, trade_id: impl Into<TradeId>, _vd: Date, conventions: &ConventionSet) -> Result<Trade, InstrumentError> {
        let _cds_conv = conventions.get_cds()?;

        let premium_cf = Cashflow::new(
            CashflowType::Coupon, self.maturity, self.start_date, self.maturity,
            1.0, self.notional, Payoff::fixed(self.spread), self.currency,
        );
        let premium_leg = Leg::new(vec![premium_cf], Direction::Payer, LegType::Fixed, self.currency);

        Ok(Trade::new(trade_id, vec![premium_leg], TradeType::Swap))
    }
}

impl InstrumentExpander for CdsOption {
    fn expand_to_trade(&self, trade_id: impl Into<TradeId>, _vd: Date, _conv: &ConventionSet) -> Result<Trade, InstrumentError> {
        Ok(settlement_trade(trade_id, self.exercise_date, self.notional, self.strike_spread, self.currency, Direction::Receiver, TradeType::Generic))
    }
}

impl InstrumentExpander for NtdBasket {
    fn expand_to_trade(&self, trade_id: impl Into<TradeId>, _vd: Date, _conv: &ConventionSet) -> Result<Trade, InstrumentError> {
        let premium_cf = Cashflow::new(
            CashflowType::Coupon, self.maturity, self.start_date, self.maturity,
            1.0, self.notional, Payoff::fixed(self.spread), self.currency,
        );
        let premium_leg = Leg::new(vec![premium_cf], Direction::Payer, LegType::Fixed, self.currency);

        Ok(Trade::new(trade_id, vec![premium_leg], TradeType::Swap))
    }
}
