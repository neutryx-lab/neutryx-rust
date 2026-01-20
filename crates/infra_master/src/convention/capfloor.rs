//! Cap/Floor convention definitions.
//!
//! This module provides types for interest rate cap and floor conventions.

use crate::{BusinessDayConvention, CalendarId, DayCountConvention, Frequency, RateIndex};

/// Convention for an interest rate cap or floor.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CapFloorConvention {
    /// Day count convention.
    pub day_count: DayCountConvention,
    /// Payment frequency.
    pub payment_frequency: Frequency,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Reference rate index.
    pub index: RateIndex,
}

impl CapFloorConvention {
    /// Creates a new cap/floor convention.
    #[must_use]
    pub fn new(
        day_count: DayCountConvention,
        payment_frequency: Frequency,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        index: RateIndex,
    ) -> Self {
        Self {
            day_count,
            payment_frequency,
            calendar,
            business_day_convention,
            index,
        }
    }

    /// Returns the USD SOFR cap/floor convention.
    #[must_use]
    pub fn usd_sofr() -> Self {
        Self {
            day_count: DayCountConvention::Actual360,
            payment_frequency: Frequency::Quarterly,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            index: RateIndex::Sofr,
        }
    }

    /// Returns the EUR EURIBOR 3M cap/floor convention.
    #[must_use]
    pub fn eur_euribor_3m() -> Self {
        Self {
            day_count: DayCountConvention::Actual360,
            payment_frequency: Frequency::Quarterly,
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
    fn test_capfloor_convention_new() {
        let conv = CapFloorConvention::new(
            DayCountConvention::Actual360,
            Frequency::Quarterly,
            CalendarId::NewYork,
            BusinessDayConvention::ModifiedFollowing,
            RateIndex::Sofr,
        );

        assert_eq!(conv.payment_frequency, Frequency::Quarterly);
        assert_eq!(conv.index, RateIndex::Sofr);
    }

    #[test]
    fn test_usd_sofr_capfloor_convention() {
        let conv = CapFloorConvention::usd_sofr();
        assert_eq!(conv.index, RateIndex::Sofr);
        assert_eq!(conv.payment_frequency, Frequency::Quarterly);
    }

    #[test]
    fn test_eur_euribor_3m_capfloor_convention() {
        let conv = CapFloorConvention::eur_euribor_3m();
        assert_eq!(conv.index, RateIndex::Euribor3M);
        assert_eq!(conv.calendar, CalendarId::Target);
    }

    #[test]
    fn test_capfloor_convention_clone() {
        let conv = CapFloorConvention::usd_sofr();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }
}
