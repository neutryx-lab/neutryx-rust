//! Commodity instrument expansion implementations.

use super::{coupon_swap_trade, settlement_trade, InstrumentExpander};
use crate::{
    ids::TradeId,
    market::{
        convention::ConventionSet,
        instrument::{
            CommodityAsianOption, CommodityForward, CommoditySwap, CommodityVanillaOption,
            InstrumentError, SpreadOption,
        },
    },
    time::Date,
    trade::{Direction, LegType, Trade, TradeType},
};

impl InstrumentExpander for CommodityForward {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        Ok(settlement_trade(
            trade_id,
            self.delivery_date,
            self.quantity * self.forward_price,
            1.0,
            self.currency,
            Direction::Payer,
            TradeType::FxForward,
        ))
    }
}

impl InstrumentExpander for CommoditySwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let notional = self.quantity_per_period * self.fixed_price;
        Ok(coupon_swap_trade(
            trade_id,
            self.start_date,
            self.maturity,
            notional,
            self.fixed_price,
            0.0,
            self.currency,
            LegType::Fixed,
            LegType::Floating,
        ))
    }
}

impl InstrumentExpander for CommodityVanillaOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        Ok(settlement_trade(
            trade_id,
            self.expiry,
            self.quantity * self.strike,
            1.0,
            self.currency,
            Direction::Receiver,
            TradeType::Generic,
        ))
    }
}

impl InstrumentExpander for CommodityAsianOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        Ok(settlement_trade(
            trade_id,
            self.expiry,
            self.quantity * self.strike,
            1.0,
            self.currency,
            Direction::Receiver,
            TradeType::Generic,
        ))
    }
}

impl InstrumentExpander for SpreadOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        Ok(settlement_trade(
            trade_id,
            self.expiry,
            self.quantity,
            self.spread_strike,
            self.currency,
            Direction::Receiver,
            TradeType::Generic,
        ))
    }
}
