//! FRA convention definitions.
//!
//! This module provides types for Forward Rate Agreement conventions.

use crate::{BusinessDayConvention, CalendarId, DayCountConvention, RateIndex};

/// Convention for a Forward Rate Agreement.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FraConvention {
    /// Day count convention.
    pub day_count: DayCountConvention,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Reference rate index.
    pub index: RateIndex,
}

impl FraConvention {
    /// Creates a new FRA convention.
    #[must_use]
    pub fn new(
        day_count: DayCountConvention,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        index: RateIndex,
    ) -> Self {
        Self {
            day_count,
            calendar,
            business_day_convention,
            index,
        }
    }

    /// Returns the USD SOFR FRA convention.
    #[must_use]
    pub fn usd_sofr() -> Self {
        Self {
            day_count: DayCountConvention::Actual360,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            index: RateIndex::Sofr,
        }
    }

    /// Returns the EUR EURIBOR 3M FRA convention.
    #[must_use]
    pub fn eur_euribor_3m() -> Self {
        Self {
            day_count: DayCountConvention::Actual360,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            index: RateIndex::Euribor3M,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fra_convention_new() {
        let conv = FraConvention::new(
            DayCountConvention::Actual360,
            CalendarId::NewYork,
            BusinessDayConvention::ModifiedFollowing,
            RateIndex::Sofr,
        );

        assert_eq!(conv.day_count, DayCountConvention::Actual360);
        assert_eq!(conv.calendar, CalendarId::NewYork);
        assert_eq!(conv.index, RateIndex::Sofr);
    }

    #[test]
    fn test_usd_sofr_fra_convention() {
        let conv = FraConvention::usd_sofr();
        assert_eq!(conv.index, RateIndex::Sofr);
        assert_eq!(conv.calendar, CalendarId::NewYork);
    }

    #[test]
    fn test_eur_euribor_3m_fra_convention() {
        let conv = FraConvention::eur_euribor_3m();
        assert_eq!(conv.index, RateIndex::Euribor3M);
        assert_eq!(conv.calendar, CalendarId::Target);
    }

    #[test]
    fn test_fra_convention_clone() {
        let conv = FraConvention::usd_sofr();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }
}
