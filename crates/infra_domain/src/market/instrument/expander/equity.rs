//! Equity instrument expansion implementations.

use super::{coupon_swap_trade, InstrumentExpander};
use crate::{
    ids::TradeId,
    market::{
        convention::ConventionSet,
        instrument::{
            AsianOption, BasketOption, EquityBarrierOption, EquityForward, EquitySwap,
            EquityUnderlying, EquityVanillaOption, InstrumentError, LookbackOption,
        },
    },
    time::Date,
    trade::{
        Cashflow, CashflowType, Direction, IndexType, Leg, LegType, Payoff, SettlementType, Trade,
        TradeType,
    },
};

/// Extracts the underlyer name from an `EquityUnderlying`.
fn underlyer_name(underlying: &EquityUnderlying) -> String {
    match underlying {
        EquityUnderlying::SingleStock { ticker, .. } => ticker.clone(),
        EquityUnderlying::Index { name } => name.clone(),
    }
}

impl InstrumentExpander for EquityForward {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let underlyer = underlyer_name(&self.underlying);

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

        Ok(Trade::new(
            trade_id,
            vec![leg],
            TradeType::EquityForward {
                underlyer,
                forward_price: self.forward_price,
                settlement_date: self.settlement_date,
            },
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
        let underlyer = underlyer_name(&self.underlying);
        let eq_index = IndexType::Equity {
            ticker: underlyer.clone(),
        };

        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::VanillaOption {
                index: eq_index,
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
            TradeType::EquityOption {
                underlyer,
                option_type: self.option_type,
                strike: self.strike,
                exercise_type: self.exercise_style,
                settlement_type: SettlementType::Cash,
                expiry_date: self.expiry,
                contract_multiplier: 1.0,
            },
        ))
    }
}

impl InstrumentExpander for EquityBarrierOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let v = &self.vanilla;
        let underlyer = underlyer_name(&v.underlying);
        let eq_index = IndexType::Equity {
            ticker: underlyer.clone(),
        };

        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            v.expiry,
            v.expiry,
            v.expiry,
            0.0,
            v.notional,
            Payoff::VanillaOption {
                index: eq_index,
                strike: v.strike,
                option_type: v.option_type,
            },
            v.currency,
        );

        let settlement_leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            v.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![settlement_leg],
            TradeType::EquityBarrierOption {
                underlyer,
                option_type: v.option_type,
                strike: v.strike,
                barrier: self.barrier_level,
                barrier_type: self.barrier_type,
                barrier_direction: self.barrier_direction,
                monitoring_frequency: self.monitoring_frequency,
                expiry_date: v.expiry,
            },
        ))
    }
}

impl InstrumentExpander for AsianOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let underlyer = underlyer_name(&self.underlying);
        let eq_index = IndexType::Equity {
            ticker: underlyer.clone(),
        };

        let observation_dates = super::rates::generate_payment_dates(
            valuation_date,
            self.expiry,
            self.observation_frequency,
        );

        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::VanillaOption {
                index: eq_index,
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
            TradeType::AsianOption {
                underlyer,
                option_type: self.option_type,
                strike: self.strike,
                averaging_type: self.averaging_type,
                observation_dates,
                expiry_date: self.expiry,
            },
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
        let underlyer = underlyer_name(&self.underlying);
        let eq_index = IndexType::Equity {
            ticker: underlyer.clone(),
        };

        let strike = self.strike.unwrap_or(0.0);
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::VanillaOption {
                index: eq_index,
                strike,
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
            TradeType::LookbackOption {
                underlyer,
                option_type: self.option_type,
                lookback_type: self.lookback_type,
                strike: self.strike,
                observation_start: self.observation_start,
                expiry_date: self.expiry,
            },
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
        let first_underlyer = self
            .components
            .first()
            .map(|c| underlyer_name(&c.underlying))
            .unwrap_or_default();
        let eq_index = IndexType::Equity {
            ticker: first_underlyer,
        };

        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::VanillaOption {
                index: eq_index,
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

        let components: Vec<(String, f64)> = self
            .components
            .iter()
            .map(|c| (underlyer_name(&c.underlying), c.weight))
            .collect();

        Ok(Trade::new(
            trade_id,
            vec![settlement_leg],
            TradeType::BasketOption {
                components,
                option_type: self.option_type,
                strike: self.strike,
                expiry_date: self.expiry,
            },
        ))
    }
}
