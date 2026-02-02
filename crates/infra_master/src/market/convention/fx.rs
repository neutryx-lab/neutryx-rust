//! FX convention definitions.
//!
//! This module provides types for foreign exchange conventions.

use crate::time::{BusinessDayConvention, CalendarId};

/// Convention for foreign exchange transactions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxConvention {
    /// Number of spot days.
    pub spot_days: u32,
    /// Calendar for settlement.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
}

impl FxConvention {
    /// Creates a new FX convention.
    #[must_use]
    pub fn new(
        spot_days: u32,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
    ) -> Self {
        Self {
            spot_days,
            calendar,
            business_day_convention,
        }
    }

    /// Returns the USD/JPY FX convention.
    #[must_use]
    pub fn usd_jpy() -> Self {
        Self {
            spot_days: 2,
            calendar: CalendarId::Tokyo,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
        }
    }

    /// Returns the EUR/USD FX convention.
    #[must_use]
    pub fn eur_usd() -> Self {
        Self {
            spot_days: 2,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
        }
    }

    /// Returns the GBP/USD FX convention.
    #[must_use]
    pub fn gbp_usd() -> Self {
        Self {
            spot_days: 2,
            calendar: CalendarId::London,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
        }
    }

    /// Returns the default USD FX convention.
    #[must_use]
    pub fn usd_default() -> Self {
        Self {
            spot_days: 2,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
        }
    }

    /// Returns the default EUR FX convention.
    #[must_use]
    pub fn eur_default() -> Self { Self::eur_usd() }

    /// Returns the default GBP FX convention.
    #[must_use]
    pub fn gbp_default() -> Self { Self::gbp_usd() }

    /// Returns the default JPY FX convention.
    #[must_use]
    pub fn jpy_default() -> Self { Self::usd_jpy() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fx_convention_new() {
        let conv = FxConvention::new(
            2,
            CalendarId::NewYork,
            BusinessDayConvention::ModifiedFollowing,
        );
        assert_eq!(conv.spot_days, 2);
    }

    #[test]
    fn test_eur_usd_convention() {
        let conv = FxConvention::eur_usd();
        assert_eq!(conv.spot_days, 2);
        assert_eq!(conv.calendar, CalendarId::Target);
    }

    #[test]
    fn test_usd_jpy_convention() {
        let conv = FxConvention::usd_jpy();
        assert_eq!(conv.calendar, CalendarId::Tokyo);
    }

    #[test]
    fn test_gbp_usd_convention() {
        let conv = FxConvention::gbp_usd();
        assert_eq!(conv.calendar, CalendarId::London);
    }
}
