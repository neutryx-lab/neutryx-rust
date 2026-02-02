//! FX swap convention definitions.
//!
//! This module provides types for representing FX swap conventions.

use crate::market::Currency;
use crate::time::{BusinessDayConvention, CalendarId};

/// Settlement type for FX transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FxSettlementType {
    /// Deliverable (physical exchange of currencies).
    Deliverable,
    /// Non-deliverable (cash settlement in reference currency).
    NonDeliverable,
}

/// Near leg type for FX swaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NearLegType {
    /// Today (T+0).
    Today,
    /// Tomorrow (T+1).
    Tomorrow,
    /// Spot (T+spot_days, typically T+2).
    Spot,
}

/// Convention for an FX swap.
///
/// Represents the market conventions for pricing and settling FX swaps
/// where two FX transactions (near and far legs) are executed simultaneously.
///
/// # Example
///
/// ```rust
/// use infra_master::market::convention::{FxSwapConvention, NearLegType};
///
/// let conv = FxSwapConvention::usd_jpy();
/// assert_eq!(conv.near_leg_type, NearLegType::Spot);
/// assert_eq!(conv.spot_days, 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxSwapConvention {
    /// Base currency (first in pair).
    pub base_currency: Currency,
    /// Quote currency (second in pair).
    pub quote_currency: Currency,
    /// Type of near leg (Today, Tomorrow, or Spot).
    pub near_leg_type: NearLegType,
    /// Number of spot days.
    pub spot_days: u32,
    /// Calendar for base currency.
    pub base_calendar: CalendarId,
    /// Calendar for quote currency.
    pub quote_calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Settlement type.
    pub settlement_type: FxSettlementType,
}

impl FxSwapConvention {
    /// Creates a new FX swap convention.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_currency: Currency,
        quote_currency: Currency,
        near_leg_type: NearLegType,
        spot_days: u32,
        base_calendar: CalendarId,
        quote_calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        settlement_type: FxSettlementType,
    ) -> Self {
        Self {
            base_currency,
            quote_currency,
            near_leg_type,
            spot_days,
            base_calendar,
            quote_calendar,
            business_day_convention,
            settlement_type,
        }
    }

    /// Returns the USD/JPY FX swap convention.
    ///
    /// - Near leg: Spot
    /// - Spot days: 2
    /// - Base calendar: New York
    /// - Quote calendar: Tokyo
    /// - Settlement: Deliverable
    #[must_use]
    pub fn usd_jpy() -> Self {
        Self {
            base_currency: Currency::USD,
            quote_currency: Currency::JPY,
            near_leg_type: NearLegType::Spot,
            spot_days: 2,
            base_calendar: CalendarId::NewYork,
            quote_calendar: CalendarId::Tokyo,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            settlement_type: FxSettlementType::Deliverable,
        }
    }

    /// Returns the EUR/USD FX swap convention.
    ///
    /// - Near leg: Spot
    /// - Spot days: 2
    /// - Base calendar: TARGET
    /// - Quote calendar: New York
    /// - Settlement: Deliverable
    #[must_use]
    pub fn eur_usd() -> Self {
        Self {
            base_currency: Currency::EUR,
            quote_currency: Currency::USD,
            near_leg_type: NearLegType::Spot,
            spot_days: 2,
            base_calendar: CalendarId::Target,
            quote_calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            settlement_type: FxSettlementType::Deliverable,
        }
    }

    /// Returns the GBP/USD FX swap convention.
    ///
    /// - Near leg: Spot
    /// - Spot days: 2
    /// - Base calendar: London
    /// - Quote calendar: New York
    /// - Settlement: Deliverable
    #[must_use]
    pub fn gbp_usd() -> Self {
        Self {
            base_currency: Currency::GBP,
            quote_currency: Currency::USD,
            near_leg_type: NearLegType::Spot,
            spot_days: 2,
            base_calendar: CalendarId::London,
            quote_calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            settlement_type: FxSettlementType::Deliverable,
        }
    }

    /// Returns the EUR/JPY FX swap convention.
    ///
    /// - Near leg: Spot
    /// - Spot days: 2
    /// - Base calendar: TARGET
    /// - Quote calendar: Tokyo
    /// - Settlement: Deliverable
    #[must_use]
    pub fn eur_jpy() -> Self {
        Self {
            base_currency: Currency::EUR,
            quote_currency: Currency::JPY,
            near_leg_type: NearLegType::Spot,
            spot_days: 2,
            base_calendar: CalendarId::Target,
            quote_calendar: CalendarId::Tokyo,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            settlement_type: FxSettlementType::Deliverable,
        }
    }

    /// Creates a tom/next FX swap convention based on an existing convention.
    ///
    /// Tom/next swaps have near leg on T+1 (tomorrow) and far leg on T+2 (spot).
    #[must_use]
    pub fn as_tom_next(&self) -> Self {
        Self {
            near_leg_type: NearLegType::Tomorrow,
            ..self.clone()
        }
    }

    /// Creates an overnight FX swap convention based on an existing convention.
    ///
    /// Overnight swaps have near leg on T+0 (today) and far leg on T+1 (tomorrow).
    #[must_use]
    pub fn as_overnight(&self) -> Self {
        Self {
            near_leg_type: NearLegType::Today,
            ..self.clone()
        }
    }

    /// Returns whether this is a deliverable FX swap.
    #[must_use]
    pub fn is_deliverable(&self) -> bool {
        self.settlement_type == FxSettlementType::Deliverable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fx_swap_convention_new() {
        let conv = FxSwapConvention::new(
            Currency::USD,
            Currency::JPY,
            NearLegType::Spot,
            2,
            CalendarId::NewYork,
            CalendarId::Tokyo,
            BusinessDayConvention::ModifiedFollowing,
            FxSettlementType::Deliverable,
        );

        assert_eq!(conv.base_currency, Currency::USD);
        assert_eq!(conv.quote_currency, Currency::JPY);
        assert_eq!(conv.near_leg_type, NearLegType::Spot);
        assert_eq!(conv.spot_days, 2);
        assert!(conv.is_deliverable());
    }

    #[test]
    fn test_usd_jpy_convention() {
        let conv = FxSwapConvention::usd_jpy();

        assert_eq!(conv.base_currency, Currency::USD);
        assert_eq!(conv.quote_currency, Currency::JPY);
        assert_eq!(conv.near_leg_type, NearLegType::Spot);
        assert_eq!(conv.spot_days, 2);
        assert_eq!(conv.base_calendar, CalendarId::NewYork);
        assert_eq!(conv.quote_calendar, CalendarId::Tokyo);
        assert!(conv.is_deliverable());
    }

    #[test]
    fn test_eur_usd_convention() {
        let conv = FxSwapConvention::eur_usd();

        assert_eq!(conv.base_currency, Currency::EUR);
        assert_eq!(conv.quote_currency, Currency::USD);
        assert_eq!(conv.base_calendar, CalendarId::Target);
        assert_eq!(conv.quote_calendar, CalendarId::NewYork);
        assert!(conv.is_deliverable());
    }

    #[test]
    fn test_gbp_usd_convention() {
        let conv = FxSwapConvention::gbp_usd();

        assert_eq!(conv.base_currency, Currency::GBP);
        assert_eq!(conv.quote_currency, Currency::USD);
        assert_eq!(conv.base_calendar, CalendarId::London);
    }

    #[test]
    fn test_eur_jpy_convention() {
        let conv = FxSwapConvention::eur_jpy();

        assert_eq!(conv.base_currency, Currency::EUR);
        assert_eq!(conv.quote_currency, Currency::JPY);
        assert_eq!(conv.base_calendar, CalendarId::Target);
        assert_eq!(conv.quote_calendar, CalendarId::Tokyo);
    }

    #[test]
    fn test_as_tom_next() {
        let spot_conv = FxSwapConvention::usd_jpy();
        let tom_next_conv = spot_conv.as_tom_next();

        assert_eq!(tom_next_conv.near_leg_type, NearLegType::Tomorrow);
        assert_eq!(tom_next_conv.base_currency, Currency::USD);
        assert_eq!(tom_next_conv.quote_currency, Currency::JPY);
    }

    #[test]
    fn test_as_overnight() {
        let spot_conv = FxSwapConvention::eur_usd();
        let overnight_conv = spot_conv.as_overnight();

        assert_eq!(overnight_conv.near_leg_type, NearLegType::Today);
        assert_eq!(overnight_conv.base_currency, Currency::EUR);
        assert_eq!(overnight_conv.quote_currency, Currency::USD);
    }

    #[test]
    fn test_near_leg_type_equality() {
        assert_eq!(NearLegType::Spot, NearLegType::Spot);
        assert_ne!(NearLegType::Spot, NearLegType::Tomorrow);
        assert_ne!(NearLegType::Tomorrow, NearLegType::Today);
    }

    #[test]
    fn test_fx_settlement_type_equality() {
        assert_eq!(FxSettlementType::Deliverable, FxSettlementType::Deliverable);
        assert_ne!(
            FxSettlementType::Deliverable,
            FxSettlementType::NonDeliverable
        );
    }

    #[test]
    fn test_fx_swap_convention_clone() {
        let conv = FxSwapConvention::usd_jpy();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }

    #[test]
    fn test_fx_swap_convention_debug() {
        let conv = FxSwapConvention::usd_jpy();
        let debug_str = format!("{:?}", conv);
        assert!(debug_str.contains("FxSwapConvention"));
        assert!(debug_str.contains("USD"));
        assert!(debug_str.contains("JPY"));
    }
}
