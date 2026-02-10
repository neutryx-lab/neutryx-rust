//! FX instrument expansion implementations.
//!
//! Covers: FxSpot, FxForward, FxVanillaOption, FxBarrierOption, FxSwap,
//! CrossCurrencyBasisSwap.

use super::{fx_exchange_trade, rates::generate_payment_dates, settlement_trade, InstrumentExpander};
use crate::{
    ids::TradeId,
    market::{
        convention::ConventionSet,
        instrument::{
            CrossCurrencyBasisSwap, FxBarrierOption, FxForward, FxSpot, FxSwap, FxVanillaOption,
            InstrumentError,
        },
    },
    time::Date,
    trade::{Cashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade, TradeType},
};

impl InstrumentExpander for FxSpot {
    fn expand_to_trade(&self, trade_id: impl Into<TradeId>, _vd: Date, _conv: &ConventionSet) -> Result<Trade, InstrumentError> {
        Ok(fx_exchange_trade(trade_id, self.settlement_date, self.notional, self.spot_rate, self.notional_currency, &self.currency_pair, TradeType::FxForward))
    }
}

impl InstrumentExpander for FxForward {
    fn expand_to_trade(&self, trade_id: impl Into<TradeId>, _vd: Date, _conv: &ConventionSet) -> Result<Trade, InstrumentError> {
        Ok(fx_exchange_trade(trade_id, self.settlement_date, self.notional, self.forward_rate, self.notional_currency, &self.currency_pair, TradeType::FxForward))
    }
}

impl InstrumentExpander for FxVanillaOption {
    fn expand_to_trade(&self, trade_id: impl Into<TradeId>, _vd: Date, _conv: &ConventionSet) -> Result<Trade, InstrumentError> {
        Ok(settlement_trade(trade_id, self.delivery_date, self.notional, self.strike, self.notional_currency, Direction::Receiver, TradeType::Generic))
    }
}

impl InstrumentExpander for FxBarrierOption {
    fn expand_to_trade(&self, trade_id: impl Into<TradeId>, vd: Date, conv: &ConventionSet) -> Result<Trade, InstrumentError> {
        self.vanilla.expand_to_trade(trade_id, vd, conv)
    }
}

impl InstrumentExpander for FxSwap {
    fn expand_to_trade(&self, trade_id: impl Into<TradeId>, _vd: Date, _conv: &ConventionSet) -> Result<Trade, InstrumentError> {
        let other_currency = if self.notional_currency == self.currency_pair.base {
            self.currency_pair.quote
        } else {
            self.currency_pair.base
        };

        let (near_receive_amount, far_pay_amount) = if self.notional_currency == self.currency_pair.base {
            (self.notional * self.near_rate, self.notional * self.far_rate)
        } else {
            (self.notional / self.near_rate, self.notional / self.far_rate)
        };

        // Near leg: pay notional_currency, receive other_currency
        let near_pay_cf = Cashflow::new(
            CashflowType::Principal, self.near_leg_date, self.near_leg_date, self.near_leg_date,
            0.0, self.notional, Payoff::fixed(1.0), self.notional_currency,
        );
        let near_receive_cf = Cashflow::new(
            CashflowType::Principal, self.near_leg_date, self.near_leg_date, self.near_leg_date,
            0.0, near_receive_amount, Payoff::fixed(1.0), other_currency,
        );

        // Far leg: receive notional_currency, pay other_currency
        let far_receive_cf = Cashflow::new(
            CashflowType::Principal, self.far_leg_date, self.far_leg_date, self.far_leg_date,
            0.0, self.notional, Payoff::fixed(1.0), self.notional_currency,
        );
        let far_pay_cf = Cashflow::new(
            CashflowType::Principal, self.far_leg_date, self.far_leg_date, self.far_leg_date,
            0.0, far_pay_amount, Payoff::fixed(1.0), other_currency,
        );

        let near_pay_leg = Leg::new(vec![near_pay_cf], Direction::Payer, LegType::Principal, self.notional_currency);
        let near_receive_leg = Leg::new(vec![near_receive_cf], Direction::Receiver, LegType::Principal, other_currency);
        let far_receive_leg = Leg::new(vec![far_receive_cf], Direction::Receiver, LegType::Principal, self.notional_currency);
        let far_pay_leg = Leg::new(vec![far_pay_cf], Direction::Payer, LegType::Principal, other_currency);

        Ok(Trade::new(trade_id, vec![near_pay_leg, near_receive_leg, far_receive_leg, far_pay_leg], TradeType::Swap))
    }
}

impl InstrumentExpander for CrossCurrencyBasisSwap {
    fn expand_to_trade(&self, trade_id: impl Into<TradeId>, _vd: Date, _conv: &ConventionSet) -> Result<Trade, InstrumentError> {
        use crate::trade::IndexType;

        self.validate().map_err(|e| InstrumentError::invalid_parameter(e.to_string()))?;

        let domestic_dates = generate_payment_dates(self.start_date, self.maturity, self.domestic_leg.payment_frequency);
        let foreign_dates = generate_payment_dates(self.start_date, self.maturity, self.foreign_leg.payment_frequency);

        let domestic_cashflows: Vec<_> = (0..domestic_dates.len().saturating_sub(1)).map(|i| {
            let (start, end) = (domestic_dates[i], domestic_dates[i + 1]);
            Cashflow::new(
                CashflowType::Coupon, end, start, end,
                (end - start) as f64 / 360.0, self.notional,
                Payoff::floating(IndexType::Rate(self.domestic_leg.rate_index)), self.domestic_currency,
            )
        }).collect();

        let foreign_cashflows: Vec<_> = (0..foreign_dates.len().saturating_sub(1)).map(|i| {
            let (start, end) = (foreign_dates[i], foreign_dates[i + 1]);
            Cashflow::new(
                CashflowType::Coupon, end, start, end,
                (end - start) as f64 / 360.0, self.notional,
                Payoff::floating(IndexType::Rate(self.foreign_leg.rate_index)), self.foreign_currency,
            )
        }).collect();

        let domestic_leg = Leg::new(domestic_cashflows, Direction::Payer, LegType::Floating, self.domestic_currency);
        let foreign_leg = Leg::new(foreign_cashflows, Direction::Receiver, LegType::Floating, self.foreign_currency);

        Ok(Trade::new(trade_id, vec![domestic_leg, foreign_leg], TradeType::Swap))
    }
}
