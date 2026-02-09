//! Equity instrument expansion implementations.
//!
//! Covers: EquityForward, EquityVanillaOption, EquityBarrierOption,
//! AsianOption, LookbackOption, EquitySwap, BasketOption.

use super::InstrumentExpander;
use crate::{
    ids::TradeId,
    market::{
        convention::ConventionSet,
        instrument::{
            AsianOption, BasketOption, EquityBarrierOption, EquityForward, EquitySwap,
            EquityVanillaOption, InstrumentError, LookbackOption,
        },
    },
    time::Date,
    trade::{Cashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade, TradeType},
};

impl InstrumentExpander for EquityForward {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Equity forward: pay fixed price, receive equity value at settlement
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.settlement_date,
            self.settlement_date,
            self.settlement_date,
            0.0,
            self.notional,
            Payoff::fixed(self.forward_price),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::FxForward))
    }
}

impl InstrumentExpander for EquityVanillaOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::fixed(self.strike),
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

impl InstrumentExpander for EquityBarrierOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        self.vanilla
            .expand_to_trade(trade_id, valuation_date, conventions)
    }
}

impl InstrumentExpander for AsianOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::fixed(self.strike),
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

impl InstrumentExpander for LookbackOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let strike = self.strike.unwrap_or(0.0);
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::fixed(strike),
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

impl InstrumentExpander for EquitySwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Equity leg
        let equity_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(0.0), // Equity return
            self.currency,
        );

        let equity_leg = Leg::new(
            vec![equity_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        // Funding leg (fixed spread over funding index)
        let funding_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            self.funding_spread,
            self.notional,
            Payoff::fixed(self.funding_spread),
            self.currency,
        );

        let funding_leg = Leg::new(
            vec![funding_cf],
            Direction::Payer,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![equity_leg, funding_leg],
            TradeType::Swap,
        ))
    }
}

impl InstrumentExpander for BasketOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::fixed(self.strike),
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
