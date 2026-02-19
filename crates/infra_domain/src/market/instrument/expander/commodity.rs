//! Commodity instrument expansion implementations.

use super::{coupon_swap_trade, InstrumentExpander};
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
    trade::{Cashflow, CashflowType, Direction, IndexType, Leg, LegType, Payoff, Trade, TradeType},
};

impl InstrumentExpander for CommodityForward {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let cf = Cashflow::new(
            CashflowType::Settlement,
            self.delivery_date,
            self.delivery_date,
            self.delivery_date,
            0.0,
            self.quantity * self.forward_price,
            Payoff::fixed(1.0),
            self.currency,
        );

        let leg = Leg::new(vec![cf], Direction::Payer, LegType::Generic, self.currency);

        Ok(Trade::new(
            trade_id,
            vec![leg],
            TradeType::CommodityForward {
                commodity: self.commodity.to_string(),
                delivery_date: self.delivery_date,
                forward_price: self.forward_price,
                quantity: self.quantity,
                quantity_unit: format!("{:?}", self.unit),
            },
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
        let comm_index = IndexType::Commodity {
            name: self.commodity.to_string(),
        };

        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.quantity,
            Payoff::VanillaOption {
                index: comm_index,
                strike: self.strike,
                option_type: self.option_type,
            },
            self.currency,
        );

        let settlement_leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![settlement_leg],
            TradeType::CommodityOption {
                commodity: self.commodity.to_string(),
                option_type: self.option_type,
                strike: self.strike,
                exercise_type: self.exercise_style,
                expiry_date: self.expiry,
                quantity: self.quantity,
                quantity_unit: format!("{:?}", self.unit),
            },
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
        let comm_index = IndexType::Commodity {
            name: self.commodity.to_string(),
        };

        let observation_dates = super::rates::generate_payment_dates(
            self.averaging_start,
            self.averaging_end,
            self.observation_frequency,
        );

        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.quantity,
            Payoff::VanillaOption {
                index: comm_index,
                strike: self.strike,
                option_type: self.option_type,
            },
            self.currency,
        );

        let settlement_leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![settlement_leg],
            TradeType::CommodityAsianOption {
                commodity: self.commodity.to_string(),
                option_type: self.option_type,
                strike: self.strike,
                observation_dates,
                expiry_date: self.expiry,
                quantity: self.quantity,
                quantity_unit: format!("{:?}", self.unit),
            },
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
        let comm_index = IndexType::Commodity {
            name: self.commodity_1.to_string(),
        };

        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.quantity,
            Payoff::VanillaOption {
                index: comm_index,
                strike: self.spread_strike,
                option_type: self.option_type,
            },
            self.currency,
        );

        let settlement_leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![settlement_leg],
            TradeType::SpreadOption {
                commodity_1: self.commodity_1.to_string(),
                commodity_2: self.commodity_2.to_string(),
                option_type: self.option_type,
                spread_strike: self.spread_strike,
                expiry_date: self.expiry,
                quantity: self.quantity,
            },
        ))
    }
}
