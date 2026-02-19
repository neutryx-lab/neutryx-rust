//! Credit instrument expansion implementations.

use super::{credit_premium_leg, InstrumentExpander};
use crate::{
    ids::TradeId,
    market::{
        convention::ConventionSet,
        instrument::{Cds, CdsIndex, CdsOption, InstrumentError, NtdBasket},
    },
    time::Date,
    trade::{
        Cashflow, CashflowType, Direction, Leg, LegType, OptionType, Payoff, ProtectionSide,
        Trade, TradeType,
    },
};

impl InstrumentExpander for Cds {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _cds_conv = conventions.get_cds()?;

        let premium_leg = credit_premium_leg(
            self.start_date,
            self.maturity,
            self.notional,
            self.spread,
            self.currency,
        );

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
            LegType::Protection,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![premium_leg, protection_leg],
            TradeType::CreditDefaultSwap {
                reference_entity: self.reference_entity.clone(),
                entity_id: None,
                protection_side: ProtectionSide::Buyer,
            },
        ))
    }
}

impl InstrumentExpander for CdsIndex {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _cds_conv = conventions.get_cds()?;

        let premium_leg = credit_premium_leg(
            self.start_date,
            self.maturity,
            self.notional,
            self.spread,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![premium_leg],
            TradeType::CreditDefaultSwapIndex {
                index_name: self.index_name.clone(),
                series: self.series,
                version: Some(self.version),
                protection_side: ProtectionSide::Buyer,
            },
        ))
    }
}

impl InstrumentExpander for CdsOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let premium_leg = credit_premium_leg(
            self.exercise_date,
            self.underlying_maturity,
            self.notional,
            self.strike_spread,
            self.currency,
        );

        let protection_cf = Cashflow::new(
            CashflowType::Settlement,
            self.underlying_maturity,
            self.exercise_date,
            self.underlying_maturity,
            0.0,
            self.notional * 0.6,
            Payoff::fixed(1.0),
            self.currency,
        );
        let protection_leg = Leg::new(
            vec![protection_cf],
            Direction::Receiver,
            LegType::Protection,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![premium_leg, protection_leg],
            TradeType::CreditDefaultSwapOption {
                reference_entity: self.reference_entity.clone(),
                option_type: if self.is_payer {
                    OptionType::Call
                } else {
                    OptionType::Put
                },
                exercise_type: crate::trade::ExerciseType::European,
                expiry_date: self.exercise_date,
            },
        ))
    }
}

impl InstrumentExpander for NtdBasket {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _vd: Date,
        _conv: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let premium_leg = credit_premium_leg(
            self.start_date,
            self.maturity,
            self.notional,
            self.spread,
            self.currency,
        );

        let protection_cf = Cashflow::new(
            CashflowType::Settlement,
            self.maturity,
            self.start_date,
            self.maturity,
            0.0,
            self.notional * 0.6,
            Payoff::fixed(1.0),
            self.currency,
        );
        let protection_leg = Leg::new(
            vec![protection_cf],
            Direction::Receiver,
            LegType::Protection,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![premium_leg, protection_leg],
            TradeType::NtdBasket {
                constituents: self.constituents.clone(),
                nth_to_default: self.nth_to_default,
                protection_side: ProtectionSide::Buyer,
            },
        ))
    }
}
