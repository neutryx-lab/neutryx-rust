//! FX instrument expansion implementations.
//!
//! Covers: FxSpot, FxForward, FxVanillaOption, FxBarrierOption, FxSwap,
//! CrossCurrencyBasisSwap.

use super::InstrumentExpander;
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

use super::rates::generate_payment_dates;

impl InstrumentExpander for FxSpot {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // FX spot has two principal exchanges
        // Pay notional in notional currency, receive converted amount in other currency

        let pay_cf = Cashflow::new(
            CashflowType::Principal,
            self.settlement_date,
            self.settlement_date,
            self.settlement_date,
            0.0,
            self.notional,
            Payoff::fixed(1.0),
            self.notional_currency,
        );

        let receive_amount = if self.notional_currency == self.currency_pair.base {
            self.notional * self.spot_rate
        } else {
            self.notional / self.spot_rate
        };

        let receive_currency = if self.notional_currency == self.currency_pair.base {
            self.currency_pair.quote
        } else {
            self.currency_pair.base
        };

        let receive_cf = Cashflow::new(
            CashflowType::Principal,
            self.settlement_date,
            self.settlement_date,
            self.settlement_date,
            0.0,
            receive_amount,
            Payoff::fixed(1.0),
            receive_currency,
        );

        let pay_leg = Leg::new(
            vec![pay_cf],
            Direction::Payer,
            LegType::Principal,
            self.notional_currency,
        );

        let receive_leg = Leg::new(
            vec![receive_cf],
            Direction::Receiver,
            LegType::Principal,
            receive_currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![pay_leg, receive_leg],
            TradeType::FxForward,
        ))
    }
}

impl InstrumentExpander for FxForward {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Similar to FX spot but with forward rate
        let pay_cf = Cashflow::new(
            CashflowType::Principal,
            self.settlement_date,
            self.settlement_date,
            self.settlement_date,
            0.0,
            self.notional,
            Payoff::fixed(1.0),
            self.notional_currency,
        );

        let receive_amount = if self.notional_currency == self.currency_pair.base {
            self.notional * self.forward_rate
        } else {
            self.notional / self.forward_rate
        };

        let receive_currency = if self.notional_currency == self.currency_pair.base {
            self.currency_pair.quote
        } else {
            self.currency_pair.base
        };

        let receive_cf = Cashflow::new(
            CashflowType::Principal,
            self.settlement_date,
            self.settlement_date,
            self.settlement_date,
            0.0,
            receive_amount,
            Payoff::fixed(1.0),
            receive_currency,
        );

        let pay_leg = Leg::new(
            vec![pay_cf],
            Direction::Payer,
            LegType::Principal,
            self.notional_currency,
        );

        let receive_leg = Leg::new(
            vec![receive_cf],
            Direction::Receiver,
            LegType::Principal,
            receive_currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![pay_leg, receive_leg],
            TradeType::FxForward,
        ))
    }
}

impl InstrumentExpander for FxVanillaOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // FX option has conditional payoff at delivery
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.delivery_date,
            self.expiry,
            self.delivery_date,
            0.0,
            self.notional,
            Payoff::fixed(self.strike),
            self.notional_currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.notional_currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Generic))
    }
}

impl InstrumentExpander for FxBarrierOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Barrier option is based on vanilla with barrier monitoring
        self.vanilla
            .expand_to_trade(trade_id, valuation_date, conventions)
    }
}

impl InstrumentExpander for FxSwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Near leg: exchange at near rate
        let near_pay_cf = Cashflow::new(
            CashflowType::Principal,
            self.near_leg_date,
            self.near_leg_date,
            self.near_leg_date,
            0.0,
            self.notional,
            Payoff::fixed(1.0),
            self.notional_currency,
        );

        let near_receive_amount = if self.notional_currency == self.currency_pair.base {
            self.notional * self.near_rate
        } else {
            self.notional / self.near_rate
        };

        let other_currency = if self.notional_currency == self.currency_pair.base {
            self.currency_pair.quote
        } else {
            self.currency_pair.base
        };

        let near_receive_cf = Cashflow::new(
            CashflowType::Principal,
            self.near_leg_date,
            self.near_leg_date,
            self.near_leg_date,
            0.0,
            near_receive_amount,
            Payoff::fixed(1.0),
            other_currency,
        );

        // Far leg: reverse exchange at far rate
        let far_receive_cf = Cashflow::new(
            CashflowType::Principal,
            self.far_leg_date,
            self.far_leg_date,
            self.far_leg_date,
            0.0,
            self.notional,
            Payoff::fixed(1.0),
            self.notional_currency,
        );

        let far_pay_amount = if self.notional_currency == self.currency_pair.base {
            self.notional * self.far_rate
        } else {
            self.notional / self.far_rate
        };

        let far_pay_cf = Cashflow::new(
            CashflowType::Principal,
            self.far_leg_date,
            self.far_leg_date,
            self.far_leg_date,
            0.0,
            far_pay_amount,
            Payoff::fixed(1.0),
            other_currency,
        );

        let near_pay_leg = Leg::new(
            vec![near_pay_cf],
            Direction::Payer,
            LegType::Principal,
            self.notional_currency,
        );

        let near_receive_leg = Leg::new(
            vec![near_receive_cf],
            Direction::Receiver,
            LegType::Principal,
            other_currency,
        );

        let far_receive_leg = Leg::new(
            vec![far_receive_cf],
            Direction::Receiver,
            LegType::Principal,
            self.notional_currency,
        );

        let far_pay_leg = Leg::new(
            vec![far_pay_cf],
            Direction::Payer,
            LegType::Principal,
            other_currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![near_pay_leg, near_receive_leg, far_receive_leg, far_pay_leg],
            TradeType::Swap,
        ))
    }
}

impl InstrumentExpander for CrossCurrencyBasisSwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        use crate::trade::IndexType;

        // Validate the instrument first
        self.validate()
            .map_err(|e| InstrumentError::invalid_parameter(e.to_string()))?;

        // Generate payment dates for domestic leg
        let domestic_dates = generate_payment_dates(
            self.start_date,
            self.maturity,
            self.domestic_leg.payment_frequency,
        );

        // Generate payment dates for foreign leg
        let foreign_dates = generate_payment_dates(
            self.start_date,
            self.maturity,
            self.foreign_leg.payment_frequency,
        );

        // Generate domestic leg cashflows (floating)
        let mut domestic_cashflows = Vec::new();
        for i in 0..domestic_dates.len().saturating_sub(1) {
            let accrual_start = domestic_dates[i];
            let accrual_end = domestic_dates[i + 1];
            let year_fraction = (accrual_end - accrual_start) as f64 / 360.0;

            let cf = Cashflow::new(
                CashflowType::Coupon,
                accrual_end,
                accrual_start,
                accrual_end,
                year_fraction,
                self.notional,
                Payoff::floating(IndexType::Rate(self.domestic_leg.rate_index)),
                self.domestic_currency,
            );
            domestic_cashflows.push(cf);
        }

        // Generate foreign leg cashflows (floating with basis spread)
        let mut foreign_cashflows = Vec::new();
        for i in 0..foreign_dates.len().saturating_sub(1) {
            let accrual_start = foreign_dates[i];
            let accrual_end = foreign_dates[i + 1];
            let year_fraction = (accrual_end - accrual_start) as f64 / 360.0;

            // Note: basis spread would be applied here in practice
            let cf = Cashflow::new(
                CashflowType::Coupon,
                accrual_end,
                accrual_start,
                accrual_end,
                year_fraction,
                self.notional, // Foreign notional would be FX-adjusted in practice
                Payoff::floating(IndexType::Rate(self.foreign_leg.rate_index)),
                self.foreign_currency,
            );
            foreign_cashflows.push(cf);
        }

        let domestic_leg = Leg::new(
            domestic_cashflows,
            Direction::Payer,
            LegType::Floating,
            self.domestic_currency,
        );

        let foreign_leg = Leg::new(
            foreign_cashflows,
            Direction::Receiver,
            LegType::Floating,
            self.foreign_currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![domestic_leg, foreign_leg],
            TradeType::Swap,
        ))
    }
}
