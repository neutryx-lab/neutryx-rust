//! Credit instrument expansion implementations.
//!
//! Covers: Cds, CdsIndex, CdsOption, NtdBasket.

use super::InstrumentExpander;
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
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _cds_conv = conventions.get_cds()?;

        // Premium leg: periodic spread payments
        let premium_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.spread),
            self.currency,
        );

        let premium_leg = Leg::new(
            vec![premium_cf],
            Direction::Payer,
            LegType::Fixed,
            self.currency,
        );

        // Protection leg: contingent payment on default
        let protection_cf = Cashflow::new(
            CashflowType::Settlement,
            self.maturity,
            self.start_date,
            self.maturity,
            0.0,
            self.notional * (1.0 - self.recovery_rate.unwrap_or(0.4)),
            Payoff::fixed(1.0),
            self.currency,
        );

        let protection_leg = Leg::new(
            vec![protection_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![premium_leg, protection_leg],
            TradeType::Swap,
        ))
    }
}

impl InstrumentExpander for CdsIndex {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _cds_conv = conventions.get_cds()?;

        // Similar to single-name CDS but on index
        let premium_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.spread),
            self.currency,
        );

        let premium_leg = Leg::new(
            vec![premium_cf],
            Direction::Payer,
            LegType::Fixed,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![premium_leg], TradeType::Swap))
    }
}

impl InstrumentExpander for CdsOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.exercise_date,
            self.exercise_date,
            self.exercise_date,
            0.0,
            self.notional,
            Payoff::fixed(self.strike_spread),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Generic))
    }
}

impl InstrumentExpander for NtdBasket {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Nth-to-default basket similar to CDS
        let premium_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.spread),
            self.currency,
        );

        let premium_leg = Leg::new(
            vec![premium_cf],
            Direction::Payer,
            LegType::Fixed,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![premium_leg], TradeType::Swap))
    }
}
