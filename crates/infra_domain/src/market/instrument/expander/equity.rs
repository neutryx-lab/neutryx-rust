//! Equity instrument expansion implementations.

use super::{coupon_swap_trade, settlement_trade, InstrumentExpander};
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
    trade::{Direction, LegType, TradeType},
};

impl InstrumentExpander for EquityForward {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        Ok(settlement_trade(
            trade_id,
            self.settlement_date,
            self.notional,
            self.forward_price,
            self.currency,
            Direction::Receiver,
            TradeType::FxForward,
        ))
    }
}

impl InstrumentExpander for EquityVanillaOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        Ok(settlement_trade(
            trade_id,
            self.expiry,
            self.notional,
            self.strike,
            self.currency,
            Direction::Receiver,
            TradeType::Generic,
        ))
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
        Ok(settlement_trade(
            trade_id,
            self.expiry,
            self.notional,
            self.strike,
            self.currency,
            Direction::Receiver,
            TradeType::Generic,
        ))
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
        Ok(settlement_trade(
            trade_id,
            self.expiry,
            self.notional,
            strike,
            self.currency,
            Direction::Receiver,
            TradeType::Generic,
        ))
    }
}

impl InstrumentExpander for EquitySwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        Ok(coupon_swap_trade(
            trade_id,
            self.start_date,
            self.maturity,
            self.notional,
            self.funding_spread,
            0.0,
            self.currency,
            LegType::Floating,
            LegType::Floating,
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
        Ok(settlement_trade(
            trade_id,
            self.expiry,
            self.notional,
            self.strike,
            self.currency,
            Direction::Receiver,
            TradeType::Generic,
        ))
    }
}

use crate::trade::Trade;
