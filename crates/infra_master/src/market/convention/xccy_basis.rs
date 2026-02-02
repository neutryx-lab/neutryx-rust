//! Cross-currency basis swap convention definitions.
//!
//! This module provides types for representing cross-currency basis swap conventions.

use crate::market::{Currency, RateIndex};
use crate::time::{BusinessDayConvention, CalendarId, DayCounter, Frequency};

/// Convention for a leg of a cross-currency basis swap.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XCcyLegConvention {
    /// Currency of this leg.
    pub currency: Currency,
    /// Day count convention.
    pub day_count: DayCounter,
    /// Payment frequency.
    pub payment_frequency: Frequency,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Reference rate index.
    pub index: RateIndex,
    /// Number of days between end of accrual and payment.
    pub payment_lag: u32,
}

impl XCcyLegConvention {
    /// Creates a new cross-currency leg convention.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        currency: Currency,
        day_count: DayCounter,
        payment_frequency: Frequency,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        index: RateIndex,
        payment_lag: u32,
    ) -> Self {
        Self {
            currency,
            day_count,
            payment_frequency,
            calendar,
            business_day_convention,
            index,
            payment_lag,
        }
    }
}

/// Specifies which leg receives the basis spread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BasisSpreadLeg {
    /// Basis spread is on the base currency leg.
    Base,
    /// Basis spread is on the quote currency leg.
    Quote,
}

/// Convention for a cross-currency basis swap.
///
/// Represents the market conventions for pricing and settling cross-currency
/// basis swaps where two floating rate legs in different currencies are exchanged.
///
/// # Example
///
/// ```rust
/// use infra_master::market::convention::XCcyBasisConvention;
///
/// let conv = XCcyBasisConvention::usd_jpy();
/// assert_eq!(conv.spot_lag, 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XCcyBasisConvention {
    /// Convention for the base currency leg (first in pair).
    pub base_leg: XCcyLegConvention,
    /// Convention for the quote currency leg (second in pair).
    pub quote_leg: XCcyLegConvention,
    /// Which leg receives the basis spread.
    pub spread_on: BasisSpreadLeg,
    /// Number of spot days from trade date to start date.
    pub spot_lag: u32,
    /// Whether notionals are exchanged at inception and maturity.
    pub exchange_notional: bool,
}

impl XCcyBasisConvention {
    /// Creates a new cross-currency basis swap convention.
    #[must_use]
    pub fn new(
        base_leg: XCcyLegConvention,
        quote_leg: XCcyLegConvention,
        spread_on: BasisSpreadLeg,
        spot_lag: u32,
        exchange_notional: bool,
    ) -> Self {
        Self {
            base_leg,
            quote_leg,
            spread_on,
            spot_lag,
            exchange_notional,
        }
    }

    /// Returns the USD/JPY cross-currency basis swap convention.
    ///
    /// - Base leg (USD): SOFR, Quarterly, ACT/360, NY calendar
    /// - Quote leg (JPY): TONAR, Quarterly, ACT/365, Tokyo calendar
    /// - Spread on: JPY leg
    /// - Spot lag: 2 days
    /// - Notional exchange: Yes
    #[must_use]
    pub fn usd_jpy() -> Self {
        Self {
            base_leg: XCcyLegConvention {
                currency: Currency::USD,
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Sofr,
                payment_lag: 2,
            },
            quote_leg: XCcyLegConvention {
                currency: Currency::JPY,
                day_count: DayCounter::Actual365Fixed,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::Tokyo,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Tonar,
                payment_lag: 2,
            },
            spread_on: BasisSpreadLeg::Quote,
            spot_lag: 2,
            exchange_notional: true,
        }
    }

    /// Returns the EUR/USD cross-currency basis swap convention.
    ///
    /// - Base leg (EUR): ESTR, Quarterly, ACT/360, TARGET calendar
    /// - Quote leg (USD): SOFR, Quarterly, ACT/360, NY calendar
    /// - Spread on: EUR leg
    /// - Spot lag: 2 days
    /// - Notional exchange: Yes
    #[must_use]
    pub fn eur_usd() -> Self {
        Self {
            base_leg: XCcyLegConvention {
                currency: Currency::EUR,
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Estr,
                payment_lag: 2,
            },
            quote_leg: XCcyLegConvention {
                currency: Currency::USD,
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Sofr,
                payment_lag: 2,
            },
            spread_on: BasisSpreadLeg::Base,
            spot_lag: 2,
            exchange_notional: true,
        }
    }

    /// Returns the GBP/USD cross-currency basis swap convention.
    ///
    /// - Base leg (GBP): SONIA, Quarterly, ACT/365, London calendar
    /// - Quote leg (USD): SOFR, Quarterly, ACT/360, NY calendar
    /// - Spread on: GBP leg
    /// - Spot lag: 2 days
    /// - Notional exchange: Yes
    #[must_use]
    pub fn gbp_usd() -> Self {
        Self {
            base_leg: XCcyLegConvention {
                currency: Currency::GBP,
                day_count: DayCounter::Actual365Fixed,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::London,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Sonia,
                payment_lag: 0,
            },
            quote_leg: XCcyLegConvention {
                currency: Currency::USD,
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Sofr,
                payment_lag: 2,
            },
            spread_on: BasisSpreadLeg::Base,
            spot_lag: 2,
            exchange_notional: true,
        }
    }

    /// Returns the EUR/JPY cross-currency basis swap convention.
    ///
    /// - Base leg (EUR): ESTR, Quarterly, ACT/360, TARGET calendar
    /// - Quote leg (JPY): TONAR, Quarterly, ACT/365, Tokyo calendar
    /// - Spread on: EUR leg
    /// - Spot lag: 2 days
    /// - Notional exchange: Yes
    #[must_use]
    pub fn eur_jpy() -> Self {
        Self {
            base_leg: XCcyLegConvention {
                currency: Currency::EUR,
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Estr,
                payment_lag: 2,
            },
            quote_leg: XCcyLegConvention {
                currency: Currency::JPY,
                day_count: DayCounter::Actual365Fixed,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::Tokyo,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Tonar,
                payment_lag: 2,
            },
            spread_on: BasisSpreadLeg::Base,
            spot_lag: 2,
            exchange_notional: true,
        }
    }

    /// Returns the base currency of this swap.
    #[must_use]
    pub fn base_currency(&self) -> Currency {
        self.base_leg.currency
    }

    /// Returns the quote currency of this swap.
    #[must_use]
    pub fn quote_currency(&self) -> Currency {
        self.quote_leg.currency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xccy_leg_convention_new() {
        let leg = XCcyLegConvention::new(
            Currency::USD,
            DayCounter::Actual360,
            Frequency::Quarterly,
            CalendarId::NewYork,
            BusinessDayConvention::ModifiedFollowing,
            RateIndex::Sofr,
            2,
        );

        assert_eq!(leg.currency, Currency::USD);
        assert_eq!(leg.day_count, DayCounter::Actual360);
        assert_eq!(leg.payment_frequency, Frequency::Quarterly);
        assert_eq!(leg.calendar, CalendarId::NewYork);
        assert_eq!(leg.index, RateIndex::Sofr);
        assert_eq!(leg.payment_lag, 2);
    }

    #[test]
    fn test_xccy_basis_convention_new() {
        let base_leg = XCcyLegConvention::new(
            Currency::USD,
            DayCounter::Actual360,
            Frequency::Quarterly,
            CalendarId::NewYork,
            BusinessDayConvention::ModifiedFollowing,
            RateIndex::Sofr,
            2,
        );
        let quote_leg = XCcyLegConvention::new(
            Currency::JPY,
            DayCounter::Actual365Fixed,
            Frequency::Quarterly,
            CalendarId::Tokyo,
            BusinessDayConvention::ModifiedFollowing,
            RateIndex::Tonar,
            2,
        );
        let conv = XCcyBasisConvention::new(
            base_leg,
            quote_leg,
            BasisSpreadLeg::Quote,
            2,
            true,
        );

        assert_eq!(conv.base_currency(), Currency::USD);
        assert_eq!(conv.quote_currency(), Currency::JPY);
        assert_eq!(conv.spread_on, BasisSpreadLeg::Quote);
        assert_eq!(conv.spot_lag, 2);
        assert!(conv.exchange_notional);
    }

    #[test]
    fn test_usd_jpy_convention() {
        let conv = XCcyBasisConvention::usd_jpy();

        assert_eq!(conv.base_currency(), Currency::USD);
        assert_eq!(conv.quote_currency(), Currency::JPY);
        assert_eq!(conv.base_leg.index, RateIndex::Sofr);
        assert_eq!(conv.quote_leg.index, RateIndex::Tonar);
        assert_eq!(conv.spread_on, BasisSpreadLeg::Quote);
        assert_eq!(conv.spot_lag, 2);
        assert!(conv.exchange_notional);
    }

    #[test]
    fn test_eur_usd_convention() {
        let conv = XCcyBasisConvention::eur_usd();

        assert_eq!(conv.base_currency(), Currency::EUR);
        assert_eq!(conv.quote_currency(), Currency::USD);
        assert_eq!(conv.base_leg.index, RateIndex::Estr);
        assert_eq!(conv.quote_leg.index, RateIndex::Sofr);
        assert_eq!(conv.spread_on, BasisSpreadLeg::Base);
        assert_eq!(conv.spot_lag, 2);
        assert!(conv.exchange_notional);
    }

    #[test]
    fn test_gbp_usd_convention() {
        let conv = XCcyBasisConvention::gbp_usd();

        assert_eq!(conv.base_currency(), Currency::GBP);
        assert_eq!(conv.quote_currency(), Currency::USD);
        assert_eq!(conv.base_leg.index, RateIndex::Sonia);
        assert_eq!(conv.quote_leg.index, RateIndex::Sofr);
        assert_eq!(conv.spread_on, BasisSpreadLeg::Base);
        assert_eq!(conv.base_leg.payment_lag, 0); // SONIA has 0 payment lag
    }

    #[test]
    fn test_eur_jpy_convention() {
        let conv = XCcyBasisConvention::eur_jpy();

        assert_eq!(conv.base_currency(), Currency::EUR);
        assert_eq!(conv.quote_currency(), Currency::JPY);
        assert_eq!(conv.base_leg.index, RateIndex::Estr);
        assert_eq!(conv.quote_leg.index, RateIndex::Tonar);
        assert_eq!(conv.spread_on, BasisSpreadLeg::Base);
    }

    #[test]
    fn test_basis_spread_leg_equality() {
        assert_eq!(BasisSpreadLeg::Base, BasisSpreadLeg::Base);
        assert_ne!(BasisSpreadLeg::Base, BasisSpreadLeg::Quote);
    }

    #[test]
    fn test_xccy_basis_convention_clone() {
        let conv = XCcyBasisConvention::usd_jpy();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }

    #[test]
    fn test_xccy_basis_convention_debug() {
        let conv = XCcyBasisConvention::usd_jpy();
        let debug_str = format!("{:?}", conv);
        assert!(debug_str.contains("XCcyBasisConvention"));
        assert!(debug_str.contains("USD"));
        assert!(debug_str.contains("JPY"));
    }
}
