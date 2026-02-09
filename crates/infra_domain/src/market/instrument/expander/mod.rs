//! Instrument expansion to Trade (cashflow generation).
//!
//! This module provides the `InstrumentExpander` trait for converting
//! `InstrumentDefinition` into `Trade` with generated cashflows.
//!
//! # Example
//!
//! ```rust,ignore
//! use infra_domain::market::instrument::{InstrumentDefinition, InstrumentExpander, FxSpot};
//! use infra_domain::market::convention::ConventionSet;
//! use infra_domain::time::Date;
//!
//! let fx_spot = FxSpot { /* ... */ };
//! let instrument = InstrumentDefinition::FxSpot(fx_spot);
//! let conventions = ConventionSet::usd_standard();
//! let valuation_date = Date::from_ymd(2025, 1, 1).unwrap();
//!
//! let trade = instrument.expand_to_trade("TRADE-001", valuation_date, &conventions)?;
//! ```

mod commodity;
mod credit;
mod equity;
mod fx;
pub(crate) mod rates;
#[cfg(test)]
mod tests;

use super::{InstrumentDefinition, InstrumentError};
use crate::{ids::TradeId, market::convention::ConventionSet, time::Date, trade::Trade};

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
            InstrumentDefinition::Deposit(d) => {
                d.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::Fra(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::Futures(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::InterestRateSwap(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::BasisSwap(b) => {
                b.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::Ois(o) => {
                o.expand_to_trade(trade_id, valuation_date, conventions)
            }
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
            InstrumentDefinition::CrossCurrencyBasisSwap(x) => {
                x.expand_to_trade(trade_id, valuation_date, conventions)
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
