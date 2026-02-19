//! FX instrument expansion implementations.

use super::{
    fx_exchange_trade, generate_floating_leg_cashflows, rates::generate_payment_dates,
    InstrumentExpander,
};
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
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        Ok(fx_exchange_trade(
            trade_id,
            self.settlement_date,
            self.notional,
            self.spot_rate,
            self.notional_currency,
            &self.currency_pair,
            TradeType::FxForward,
        ))
    }
}

impl InstrumentExpander for FxForward {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        Ok(fx_exchange_trade(
            trade_id,
            self.settlement_date,
            self.notional,
            self.forward_rate,
            self.notional_currency,
            &self.currency_pair,
            TradeType::FxForward,
        ))
    }
}

impl InstrumentExpander for FxVanillaOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        use crate::trade::IndexType;

        let fx_index = IndexType::Fx {
            base: self.currency_pair.base.to_string(),
            quote: self.currency_pair.quote.to_string(),
        };

        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.delivery_date,
            self.expiry,
            self.delivery_date,
            0.0,
            self.notional,
            Payoff::VanillaOption {
                index: fx_index,
                strike: self.strike,
                option_type: self.option_type,
            },
            self.notional_currency,
        );

        let settlement_leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.notional_currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![settlement_leg],
            TradeType::FxOption {
                option_type: self.option_type,
                strike: self.strike,
                exercise_type: self.exercise_style,
                settlement_type: crate::trade::SettlementType::Cash,
                expiry_date: self.expiry,
            },
        ))
    }
}

impl InstrumentExpander for FxBarrierOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        use crate::trade::IndexType;

        let v = &self.vanilla;
        let fx_index = IndexType::Fx {
            base: v.currency_pair.base.to_string(),
            quote: v.currency_pair.quote.to_string(),
        };

        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            v.delivery_date,
            v.expiry,
            v.delivery_date,
            0.0,
            v.notional,
            Payoff::VanillaOption {
                index: fx_index,
                strike: v.strike,
                option_type: v.option_type,
            },
            v.notional_currency,
        );

        let settlement_leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            v.notional_currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![settlement_leg],
            TradeType::FxBarrierOption {
                option_type: v.option_type,
                strike: v.strike,
                barrier: self.barrier_level,
                barrier_type: self.barrier_type,
                barrier_direction: self.barrier_direction,
                exercise_type: v.exercise_style,
                expiry_date: v.expiry,
            },
        ))
    }
}

impl InstrumentExpander for FxSwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let other_currency = if self.notional_currency == self.currency_pair.base {
            self.currency_pair.quote
        } else {
            self.currency_pair.base
        };

        let (near_receive_amount, far_pay_amount) =
            if self.notional_currency == self.currency_pair.base {
                (
                    self.notional * self.near_rate,
                    self.notional * self.far_rate,
                )
            } else {
                (
                    self.notional / self.near_rate,
                    self.notional / self.far_rate,
                )
            };

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
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        self.validate()
            .map_err(|e| InstrumentError::invalid_parameter(e.to_string()))?;

        let domestic_dates = generate_payment_dates(
            self.start_date,
            self.maturity,
            self.domestic_leg.payment_frequency,
        );
        let foreign_dates = generate_payment_dates(
            self.start_date,
            self.maturity,
            self.foreign_leg.payment_frequency,
        );

        let domestic_cashflows = generate_floating_leg_cashflows(
            &domestic_dates,
            self.domestic_leg.rate_index,
            self.notional,
            self.domestic_currency,
        );
        let foreign_cashflows = generate_floating_leg_cashflows(
            &foreign_dates,
            self.foreign_leg.rate_index,
            self.notional,
            self.foreign_currency,
        );

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
