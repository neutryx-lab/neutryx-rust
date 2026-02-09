//! Commodity instrument expansion implementations.
//!
//! Covers: CommodityForward, CommoditySwap, CommodityVanillaOption,
//! CommodityAsianOption, SpreadOption.

use super::InstrumentExpander;
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
    trade::{Cashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade, TradeType},
};

impl InstrumentExpander for CommodityForward {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Pay fixed price, receive commodity
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.delivery_date,
            self.delivery_date,
            self.delivery_date,
            0.0,
            self.quantity * self.forward_price,
            Payoff::fixed(1.0),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Payer,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::FxForward))
    }
}

impl InstrumentExpander for CommoditySwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Fixed leg
        let notional = self.quantity_per_period * self.fixed_price;
        let fixed_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            notional,
            Payoff::fixed(self.fixed_price),
            self.currency,
        );

        let fixed_leg = Leg::new(
            vec![fixed_cf],
            Direction::Payer,
            LegType::Fixed,
            self.currency,
        );

        // Floating leg (commodity price)
        let floating_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            notional,
            Payoff::fixed(0.0), // Commodity index reference
            self.currency,
        );

        let floating_leg = Leg::new(
            vec![floating_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![fixed_leg, floating_leg],
            TradeType::Swap,
        ))
    }
}

impl InstrumentExpander for CommodityVanillaOption {
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
            self.quantity * self.strike,
            Payoff::fixed(1.0),
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

impl InstrumentExpander for CommodityAsianOption {
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
            self.quantity * self.strike,
            Payoff::fixed(1.0),
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

impl InstrumentExpander for SpreadOption {
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
            self.quantity,
            Payoff::fixed(self.spread_strike),
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
