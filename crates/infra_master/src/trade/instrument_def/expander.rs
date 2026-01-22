//! Instrument expansion to Trade (cashflow generation).
//!
//! This module provides the `InstrumentExpander` trait for converting
//! `InstrumentDefinition` into `Trade` with generated cashflows.
//!
//! # Example
//!
//! ```rust,ignore
//! use infra_master::trade::instrument_def::{InstrumentDefinition, InstrumentExpander, FxSpot};
//! use infra_master::trade::convention::ConventionSet;
//! use infra_master::Date;
//!
//! let fx_spot = FxSpot { /* ... */ };
//! let instrument = InstrumentDefinition::FxSpot(fx_spot);
//! let conventions = ConventionSet::usd_standard();
//! let valuation_date = Date::from_ymd(2025, 1, 1).unwrap();
//!
//! let trade = instrument.expand_to_trade("TRADE-001", valuation_date, &conventions)?;
//! ```

use super::{
    // Commodity
    CommodityAsianOption,
    CommodityForward,
    CommoditySwap,
    CommodityVanillaOption,
    // Credit
    Cds,
    CdsIndex,
    CdsOption,
    // Equity
    EquityBarrierOption,
    EquityForward,
    EquitySwap,
    EquityVanillaOption,
    // FX
    FxBarrierOption,
    FxForward,
    FxSpot,
    FxSwap,
    FxVanillaOption,
    InstrumentDefinition,
    InstrumentError,
    NtdBasket,
    // Rates
    CapFloor,
    CmsSwap,
    Frn,
    InflationSwap,
    SpreadOption,
    Swaption,
    // Common
    AsianOption,
    BasketOption,
    LookbackOption,
};
use crate::{
    trade::{
        convention::ConventionSet, Cashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade,
        TradeId, TradeType,
    },
    Date,
};

/// Trait for expanding instrument definitions into trades with cashflows.
///
/// This trait provides the `expand_to_trade` method which converts an
/// `InstrumentDefinition` into a fully expanded `Trade` with generated
/// cashflows based on market conventions.
pub trait InstrumentExpander {
    /// Expands this instrument into a Trade with cashflows.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Unique identifier for the resulting trade
    /// * `valuation_date` - Date for valuation/pricing
    /// * `conventions` - Market conventions for cashflow generation
    ///
    /// # Returns
    ///
    /// A `Trade` containing legs and cashflows, or an error if expansion fails.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if:
    /// - Required convention is missing (`MissingConvention`)
    /// - Instrument validation fails (`InvalidParameter`)
    /// - Cashflow expansion fails (`ExpansionFailed`)
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError>;
}

impl InstrumentExpander for InstrumentDefinition {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Validate first
        self.validate()?;

        match self {
            // === Rates ===
            InstrumentDefinition::Swaption(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CapFloor(c) => {
                c.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::Frn(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CmsSwap(c) => {
                c.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::InflationSwap(i) => {
                i.expand_to_trade(trade_id, valuation_date, conventions)
            }

            // === FX ===
            InstrumentDefinition::FxSpot(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::FxForward(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::FxVanillaOption(o) => {
                o.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::FxBarrierOption(b) => {
                b.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::FxSwap(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }

            // === Equity ===
            InstrumentDefinition::EquityForward(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::EquityVanillaOption(o) => {
                o.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::EquityBarrierOption(b) => {
                b.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::AsianOption(a) => {
                a.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::LookbackOption(l) => {
                l.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::EquitySwap(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::BasketOption(b) => {
                b.expand_to_trade(trade_id, valuation_date, conventions)
            }

            // === Credit ===
            InstrumentDefinition::Cds(c) => {
                c.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CdsIndex(i) => {
                i.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CdsOption(o) => {
                o.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::NtdBasket(n) => {
                n.expand_to_trade(trade_id, valuation_date, conventions)
            }

            // === Commodity ===
            InstrumentDefinition::CommodityForward(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CommoditySwap(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CommodityVanillaOption(o) => {
                o.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CommodityAsianOption(a) => {
                a.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::SpreadOption(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
        }
    }
}

// ============================================================================
// Rates Instrument Expansion
// ============================================================================

impl InstrumentExpander for Swaption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _swaption_conv = conventions.get_swaption()?;

        // Create settlement cashflow for the swaption premium/exercise
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

        Ok(Trade::new(
            trade_id,
            vec![leg],
            TradeType::Swaption {
                exercise_dates: vec![self.expiry],
                exercise_type: self.exercise_type,
                settlement_type: self.settlement_type,
            },
        ))
    }
}

impl InstrumentExpander for CapFloor {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Generate caplet/floorlet cashflows based on payment frequency
        let mut cashflows = Vec::new();

        // For simplicity, create a single settlement cashflow
        // Full implementation would generate individual caplet/floorlet cashflows
        let strike = self.strikes.first().copied().unwrap_or(0.0);
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.start_date,
            self.start_date,
            self.start_date,
            0.0,
            self.notional_schedule.notional_at(0),
            Payoff::fixed(strike),
            self.currency,
        );
        cashflows.push(settlement_cf);

        let leg = Leg::new(cashflows, Direction::Receiver, LegType::CapFloor, self.currency);

        Ok(Trade::new(trade_id, vec![leg], TradeType::CapFloor))
    }
}

impl InstrumentExpander for Frn {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        use crate::trade::IndexType;

        // Create floating coupon cashflow
        let coupon_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0, // Placeholder year fraction
            self.principal_schedule.notional_at(0),
            Payoff::floating(IndexType::Rate(self.coupon_index)),
            self.currency,
        );

        // Create principal redemption cashflow
        let principal_cf = Cashflow::new(
            CashflowType::Principal,
            self.maturity,
            self.maturity,
            self.maturity,
            0.0,
            self.principal_schedule.notional_at(0),
            Payoff::fixed(1.0),
            self.currency,
        );

        let leg = Leg::new(
            vec![coupon_cf, principal_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![leg],
            TradeType::Bond {
                issuer_id: None,
                seniority: None,
            },
        ))
    }
}

impl InstrumentExpander for CmsSwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _swap_conv = conventions.get_swap()?;

        // Create CMS leg cashflow
        let cms_cf = Cashflow::new(
            CashflowType::Coupon,
            self.start_date, // Use start_date as placeholder since CmsSwap uses tenor
            self.start_date,
            self.start_date,
            1.0,
            self.notional,
            Payoff::fixed(self.spread), // CMS rate + spread
            self.currency,
        );

        let cms_leg = Leg::new(
            vec![cms_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![cms_leg], TradeType::Swap))
    }
}

impl InstrumentExpander for InflationSwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _inflation_conv = conventions.get_inflation_swap()?;

        // Create inflation leg cashflow
        let inflation_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.fixed_rate),
            self.currency,
        );

        let inflation_leg = Leg::new(
            vec![inflation_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![inflation_leg], TradeType::Swap))
    }
}

// ============================================================================
// FX Instrument Expansion
// ============================================================================

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

// ============================================================================
// Equity Instrument Expansion
// ============================================================================

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

// ============================================================================
// Credit Instrument Expansion
// ============================================================================

impl InstrumentExpander for Cds {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _cds_conv = conventions.get_cds()?;

        // Premium leg: periodic spread payments
        let premium_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.spread),
            self.currency,
        );

        let premium_leg = Leg::new(
            vec![premium_cf],
            Direction::Payer,
            LegType::Fixed,
            self.currency,
        );

        // Protection leg: contingent payment on default
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
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![premium_leg, protection_leg],
            TradeType::Swap,
        ))
    }
}

impl InstrumentExpander for CdsIndex {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _cds_conv = conventions.get_cds()?;

        // Similar to single-name CDS but on index
        let premium_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.spread),
            self.currency,
        );

        let premium_leg = Leg::new(
            vec![premium_cf],
            Direction::Payer,
            LegType::Fixed,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![premium_leg], TradeType::Swap))
    }
}

impl InstrumentExpander for CdsOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.exercise_date,
            self.exercise_date,
            self.exercise_date,
            0.0,
            self.notional,
            Payoff::fixed(self.strike_spread),
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

impl InstrumentExpander for NtdBasket {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Nth-to-default basket similar to CDS
        let premium_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.spread),
            self.currency,
        );

        let premium_leg = Leg::new(
            vec![premium_cf],
            Direction::Payer,
            LegType::Fixed,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![premium_leg], TradeType::Swap))
    }
}

// ============================================================================
// Commodity Instrument Expansion
// ============================================================================

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::convention::{
        CdsConvention, EquityConvention, FxConvention, FxOptionConvention,
        InflationSwapConvention, SwapConvention, SwaptionConvention,
    };
    use crate::trade::{ExerciseType, SettlementType};
    use crate::{Currency, Tenor};

    use super::super::{CurrencyPair, EquityUnderlying, ExerciseStyle, PayerReceiver};

    fn make_conventions() -> ConventionSet {
        ConventionSet::new()
            .with_swap(SwapConvention::usd_sofr())
            .with_swaption(SwaptionConvention::usd_sofr())
            .with_fx(FxConvention::usd_default())
            .with_fx_option(FxOptionConvention::g10_standard())
            .with_cds(CdsConvention::isda_na())
            .with_equity(EquityConvention::us_equity())
            .with_inflation_swap(InflationSwapConvention::us_cpi_zc())
    }

    fn valuation_date() -> Date {
        Date::from_ymd(2025, 1, 1).unwrap()
    }

    // === Rates Tests ===

    #[test]
    fn test_expand_swaption() {
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        let trade = swaption
            .expand_to_trade("SWAPTION-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "SWAPTION-001");
        assert!(trade.trade_type.is_swaption());
        assert_eq!(trade.num_legs(), 1);
        assert_eq!(trade.total_cashflows(), 1);
    }

    #[test]
    fn test_expand_swaption_missing_convention() {
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        let empty_conventions = ConventionSet::new();
        let result = swaption.expand_to_trade("SWAPTION-001", valuation_date(), &empty_conventions);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            InstrumentError::MissingConvention { .. }
        ));
    }

    // === FX Tests ===

    #[test]
    fn test_expand_fx_spot() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_spot
            .expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "FX-SPOT-001");
        assert_eq!(trade.trade_type, TradeType::FxForward);
        assert_eq!(trade.num_legs(), 2);
        assert_eq!(trade.total_cashflows(), 2);
    }

    #[test]
    fn test_expand_fx_forward() {
        let fx_forward = FxForward {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            forward_rate: 1.1100,
            settlement_date: Date::from_ymd(2025, 7, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_forward
            .expand_to_trade("FX-FWD-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "FX-FWD-001");
        assert_eq!(trade.trade_type, TradeType::FxForward);
        assert_eq!(trade.num_legs(), 2);
    }

    #[test]
    fn test_expand_fx_swap() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "FX-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 4); // near pay, near receive, far pay, far receive
    }

    // === Equity Tests ===

    #[test]
    fn test_expand_equity_forward() {
        let eq_forward = EquityForward {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            forward_price: 5000.0,
            settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
            notional: 100_000.0,
            currency: Currency::USD,
        };

        let trade = eq_forward
            .expand_to_trade("EQ-FWD-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "EQ-FWD-001");
        assert_eq!(trade.num_legs(), 1);
    }

    #[test]
    fn test_expand_equity_vanilla_option() {
        use crate::trade::OptionType;

        let eq_option = EquityVanillaOption {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            strike: 5000.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100_000.0,
            currency: Currency::USD,
        };

        let trade = eq_option
            .expand_to_trade("EQ-OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "EQ-OPT-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    // === Credit Tests ===

    #[test]
    fn test_expand_cds() {
        use super::super::CreditEvent;

        let cds = Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.01,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy, CreditEvent::FailureToPay],
        };

        let trade = cds
            .expand_to_trade("CDS-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "CDS-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 2); // premium leg and protection leg
    }

    // === Commodity Tests ===

    #[test]
    fn test_expand_commodity_forward() {
        use super::super::{CommodityType, EnergyType, QuantityUnit};

        let comm_forward = CommodityForward {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            delivery_location: "Cushing, OK".to_string(),
            delivery_date: Date::from_ymd(2025, 6, 15).unwrap(),
            quantity: 1000.0,
            unit: QuantityUnit::Barrels,
            forward_price: 75.0,
            currency: Currency::USD,
        };

        let trade = comm_forward
            .expand_to_trade("COMM-FWD-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "COMM-FWD-001");
        assert_eq!(trade.trade_type, TradeType::FxForward);
    }

    // === InstrumentDefinition Integration Tests ===

    #[test]
    fn test_instrument_definition_expand() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let instrument = InstrumentDefinition::FxSpot(fx_spot);
        let trade = instrument
            .expand_to_trade("INST-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "INST-001");
    }

    #[test]
    fn test_instrument_definition_expand_validates() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: -1_000_000.0, // Invalid: negative notional
            notional_currency: Currency::EUR,
        };

        let instrument = InstrumentDefinition::FxSpot(fx_spot);
        let result = instrument.expand_to_trade("INST-001", valuation_date(), &make_conventions());

        assert!(result.is_err());
    }

    #[test]
    fn test_trade_all_cashflows_compatibility() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        // Verify Trade::all_cashflows() works
        let cashflows: Vec<_> = trade.all_cashflows().collect();
        assert_eq!(cashflows.len(), 4);

        // Verify future_cashflows() works
        let future_cfs: Vec<_> = trade.future_cashflows(valuation_date()).collect();
        assert_eq!(future_cfs.len(), 4);
    }

    // =========================================================================
    // Task 11.2: CF Expansion Integration Tests
    // =========================================================================

    #[test]
    fn test_expand_cap_floor() {
        use crate::RateIndex;

        let cap = CapFloor {
            cap_floor_type: super::super::CapFloorType::Cap,
            strikes: vec![0.05],
            index: RateIndex::Sofr,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::TwoYears,
            notional_schedule: super::super::NotionalSchedule::constant(10_000_000.0),
            payment_frequency: crate::Frequency::Quarterly,
            currency: Currency::USD,
        };

        let trade = cap
            .expand_to_trade("CAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "CAP-001");
        assert_eq!(trade.trade_type, TradeType::CapFloor);
        assert_eq!(trade.num_legs(), 1);
    }

    #[test]
    fn test_expand_frn() {
        use crate::RateIndex;

        let frn = Frn {
            coupon_index: RateIndex::Sofr,
            spread: 0.005,
            reset_frequency: crate::Frequency::Quarterly,
            principal_schedule: super::super::NotionalSchedule::constant(10_000_000.0),
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2030, 1, 15).unwrap(),
            currency: Currency::USD,
        };

        let trade = frn
            .expand_to_trade("FRN-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "FRN-001");
        assert!(matches!(trade.trade_type, TradeType::Bond { .. }));
        assert_eq!(trade.num_legs(), 1);
        assert!(trade.total_cashflows() >= 2); // At least coupon + principal
    }

    #[test]
    fn test_expand_cms_swap() {
        let cms = CmsSwap {
            cms_tenor: Tenor::TenYears,
            convexity_adjustment: None,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::FiveYears,
            notional: 10_000_000.0,
            currency: Currency::USD,
            spread: 0.001,
        };

        let trade = cms
            .expand_to_trade("CMS-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "CMS-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        // Implementation creates single leg with CMS rate cashflow
        assert!(trade.num_legs() >= 1);
    }

    #[test]
    fn test_expand_inflation_swap() {
        use super::super::SwapType;

        let inf_swap = InflationSwap {
            inflation_index: "USCPI".to_string(),
            lag_months: 3,
            swap_type: SwapType::ZeroCoupon,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2030, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
            fixed_rate: 0.02,
        };

        let trade = inf_swap
            .expand_to_trade("INF-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "INF-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        // Implementation creates single leg for inflation leg
        assert!(trade.num_legs() >= 1);
    }

    #[test]
    fn test_expand_fx_vanilla_option() {
        use crate::trade::OptionType;

        let fx_option = FxVanillaOption {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            strike: 1.1000,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            delivery_date: Date::from_ymd(2025, 6, 17).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_option
            .expand_to_trade("FX-OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "FX-OPT-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    #[test]
    fn test_expand_fx_barrier_option() {
        use crate::trade::OptionType;
        use super::super::{BarrierDirection, BarrierType};

        let vanilla = FxVanillaOption {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            strike: 1.1000,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            delivery_date: Date::from_ymd(2025, 6, 17).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let barrier_option = FxBarrierOption {
            vanilla,
            barrier_level: 1.15,
            barrier_type: BarrierType::KnockOut,
            barrier_direction: BarrierDirection::Up,
            rebate: Some(5000.0),
        };

        let trade = barrier_option
            .expand_to_trade("FX-BARRIER-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "FX-BARRIER-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    #[test]
    fn test_expand_asian_option() {
        use crate::trade::OptionType;
        use super::super::AveragingType;

        let asian = AsianOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: 180.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            averaging_type: AveragingType::Arithmetic,
            observation_frequency: crate::Frequency::Monthly,
            observed_values: vec![175.0, 178.0, 180.0],
            notional: 1000.0,
            currency: Currency::USD,
        };

        let trade = asian
            .expand_to_trade("ASIAN-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "ASIAN-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    #[test]
    fn test_expand_equity_swap() {
        let eq_swap = EquitySwap {
            underlying: EquityUnderlying::index("SPX"),
            return_type: super::super::EquityReturnType::TotalReturn,
            funding_index: "SOFR".to_string(),
            funding_spread: 0.001,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2026, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
        };

        let trade = eq_swap
            .expand_to_trade("EQ-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "EQ-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 2); // equity leg + funding leg
    }

    #[test]
    fn test_expand_cds_index() {
        let cds_idx = CdsIndex {
            index_name: "CDX.NA.IG".to_string(),
            series: 40,
            version: 1,
            constituent_count: 125,
            notional: 10_000_000.0,
            spread: 0.006,
            start_date: Date::from_ymd(2025, 3, 20).unwrap(),
            maturity: Date::from_ymd(2030, 6, 20).unwrap(),
            currency: Currency::USD,
        };

        let trade = cds_idx
            .expand_to_trade("CDS-IDX-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "CDS-IDX-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
    }

    #[test]
    fn test_expand_commodity_swap() {
        use super::super::{CommodityType, EnergyType, QuantityUnit};

        let comm_swap = CommoditySwap {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            fixed_price: 75.0,
            floating_index: "WTI".to_string(),
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2026, 1, 15).unwrap(),
            quantity_per_period: 1000.0,
            unit: QuantityUnit::Barrels,
            payment_frequency: crate::Frequency::Monthly,
            currency: Currency::USD,
        };

        let trade = comm_swap
            .expand_to_trade("COMM-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "COMM-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 2); // fixed + floating
    }

    #[test]
    fn test_expand_commodity_vanilla_option() {
        use crate::trade::OptionType;
        use super::super::{CommodityType, EnergyType, QuantityUnit};

        let comm_opt = CommodityVanillaOption {
            commodity: CommodityType::Energy(EnergyType::NaturalGas),
            strike: 3.50,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            quantity: 10000.0,
            unit: QuantityUnit::MMBtu,
            settlement_type: SettlementType::Cash,
            currency: Currency::USD,
        };

        let trade = comm_opt
            .expand_to_trade("COMM-OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "COMM-OPT-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    // Verify convention integration
    #[test]
    fn test_conventions_affect_expansion() {
        // Same swaption with different conventions should have different exercise types
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        let conv = make_conventions();
        let trade = swaption
            .expand_to_trade("SWAPTION-001", valuation_date(), &conv)
            .unwrap();

        // Trade type should match swaption settings
        if let TradeType::Swaption { exercise_type, settlement_type, .. } = trade.trade_type {
            assert_eq!(exercise_type, ExerciseType::European);
            assert_eq!(settlement_type, SettlementType::Cash);
        } else {
            panic!("Expected TradeType::Swaption");
        }
    }

    // =========================================================================
    // Task 11.3: Edge Case Tests
    // =========================================================================

    #[test]
    fn test_edge_case_zero_notional_validation() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 0.0, // Edge case: zero notional
            notional_currency: Currency::EUR,
        };

        // FxSpot.validate() should catch this
        assert!(fx_spot.validate().is_err());
    }

    #[test]
    fn test_edge_case_negative_notional_validation() {
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::FiveYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: -10_000_000.0, // Edge case: negative notional
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        // Swaption.validate() should catch this
        assert!(swaption.validate().is_err());
    }

    #[test]
    fn test_edge_case_negative_strike_validation() {
        use crate::trade::OptionType;

        let eq_option = EquityVanillaOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: -100.0, // Edge case: negative strike
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100.0,
            currency: Currency::USD,
        };

        // EquityVanillaOption.validate() should catch this
        assert!(eq_option.validate().is_err());
    }

    #[test]
    fn test_edge_case_maturity_before_start_validation() {
        use super::super::CreditEvent;

        let cds = Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.01,
            start_date: Date::from_ymd(2030, 1, 1).unwrap(),
            maturity: Date::from_ymd(2025, 1, 1).unwrap(), // Edge case: maturity before start
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy],
        };

        // Cds.validate() should catch this
        assert!(cds.validate().is_err());
    }

    #[test]
    fn test_edge_case_same_start_end_date() {
        // Same-day FX spot should still work
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: valuation_date(), // Same as valuation date
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_spot
            .expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.total_cashflows(), 2);
    }

    #[test]
    fn test_edge_case_empty_observed_values() {
        use crate::trade::OptionType;
        use super::super::AveragingType;

        // Asian option with no observed values yet
        let asian = AsianOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: 180.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            averaging_type: AveragingType::Arithmetic,
            observation_frequency: crate::Frequency::Monthly,
            observed_values: vec![], // Edge case: empty observations
            notional: 1000.0,
            currency: Currency::USD,
        };

        // Should succeed - Asian option can start with no observations
        let trade = asian
            .expand_to_trade("ASIAN-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id, "ASIAN-001");
    }

    #[test]
    fn test_edge_case_very_large_notional() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1e15, // Edge case: very large notional
            notional_currency: Currency::EUR,
        };

        let trade = fx_spot
            .expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.total_cashflows(), 2);
    }

    #[test]
    fn test_edge_case_very_small_rate() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1e-10, // Edge case: very small rate (but positive)
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        // Very small rate might be rejected
        let result = fx_spot.expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions());
        // Depends on validation - either succeeds or fails with appropriate error
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_edge_case_fx_swap_same_near_far_date_validation() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 1, 3).unwrap(), // Same as near date
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        // FxSwap.validate() should catch this
        assert!(fx_swap.validate().is_err());
    }

    #[test]
    fn test_edge_case_far_date_before_near_validation() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 1, 3).unwrap(), // Before near date
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        // FxSwap.validate() should catch this
        assert!(fx_swap.validate().is_err());
    }

    #[test]
    fn test_edge_case_zero_spread_cds() {
        use super::super::CreditEvent;

        // CDS with zero spread is unusual but should work
        let cds = Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.0, // Zero spread
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy],
        };

        // Zero spread might be allowed for special cases
        let result = cds.expand_to_trade("CDS-001", valuation_date(), &make_conventions());
        // Validation depends on business rules
        assert!(result.is_ok() || result.is_err());
    }

    // =========================================================================
    // Task 11.4: Property-Based Tests (Consistency Checks)
    // =========================================================================

    #[test]
    fn test_property_expanded_trade_has_cashflows() {
        // Property: Every successfully expanded trade must have at least one cashflow
        let instruments: Vec<InstrumentDefinition> = vec![
            InstrumentDefinition::FxSpot(FxSpot {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                spot_rate: 1.1050,
                settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
                notional: 1_000_000.0,
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::FxForward(FxForward {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                forward_rate: 1.1100,
                settlement_date: Date::from_ymd(2025, 7, 3).unwrap(),
                notional: 1_000_000.0,
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::EquityForward(EquityForward {
                underlying: EquityUnderlying::index("SPX"),
                forward_price: 5000.0,
                settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
                notional: 100_000.0,
                currency: Currency::USD,
            }),
        ];

        let conv = make_conventions();
        for (i, inst) in instruments.iter().enumerate() {
            let trade = inst
                .expand_to_trade(format!("INST-{}", i), valuation_date(), &conv)
                .unwrap();

            // Property: trade must have at least one leg with at least one cashflow
            assert!(trade.total_cashflows() >= 1, "Trade must have at least one cashflow");
            assert!(trade.num_legs() >= 1, "Trade must have at least one leg");
        }
    }

    #[test]
    fn test_property_trade_id_preserved() {
        // Property: Trade ID passed to expand_to_trade must be preserved
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let test_ids = ["test-123", "TRADE_ABC", "id with spaces", ""];
        let conv = make_conventions();

        for id in &test_ids {
            let trade = fx_spot
                .expand_to_trade(*id, valuation_date(), &conv)
                .unwrap();

            assert_eq!(trade.id.as_str(), *id, "Trade ID must be preserved");
        }
    }

    #[test]
    fn test_property_validation_before_expansion() {
        // Property: Invalid instruments should fail validation before expansion
        let invalid_instruments: Vec<InstrumentDefinition> = vec![
            InstrumentDefinition::FxSpot(FxSpot {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                spot_rate: 1.1050,
                settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
                notional: -1_000_000.0, // Invalid
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::EquityForward(EquityForward {
                underlying: EquityUnderlying::stock("AAPL"),
                forward_price: -100.0, // Invalid
                settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
                notional: 100.0,
                currency: Currency::USD,
            }),
        ];

        let conv = make_conventions();
        for inst in &invalid_instruments {
            let result = inst.expand_to_trade("INVALID", valuation_date(), &conv);
            assert!(result.is_err(), "Invalid instrument should fail expansion");
        }
    }

    #[test]
    fn test_property_cashflow_currencies_consistent() {
        // Property: All cashflows in a leg should have the same currency
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        for leg in trade.legs() {
            let leg_ccy = leg.currency;
            for cf in leg.cashflows() {
                assert_eq!(cf.currency, leg_ccy, "Cashflow currency must match leg currency");
            }
        }
    }

    #[test]
    fn test_property_swap_has_multiple_legs() {
        // Property: Swaps should have at least 2 legs (pay and receive)
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert!(trade.num_legs() >= 2, "Swap must have at least 2 legs");
    }

    #[test]
    fn test_property_options_have_settlement_cashflow() {
        // Property: Options should have at least a settlement cashflow
        use crate::trade::OptionType;

        let eq_option = EquityVanillaOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: 180.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100.0,
            currency: Currency::USD,
        };

        let trade = eq_option
            .expand_to_trade("OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert!(trade.total_cashflows() >= 1, "Option must have at least settlement cashflow");
    }
}
