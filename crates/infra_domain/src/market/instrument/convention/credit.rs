//! Credit convention definitions.
//!
//! This module provides types for credit-related conventions including CDS.

use crate::time::{BusinessDayConvention, CalendarId, DayCounter, Frequency};

/// Convention for a credit default swap.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CdsConvention {
    /// Day count convention.
    pub day_count: DayCounter,
    /// Premium payment frequency.
    pub payment_frequency: Frequency,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Standard recovery rate (as decimal).
    pub recovery_rate: f64,
}

impl CdsConvention {
    /// Creates a new CDS convention.
    #[must_use]
    pub fn new(
        day_count: DayCounter,
        payment_frequency: Frequency,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        recovery_rate: f64,
    ) -> Self {
        Self {
            day_count,
            payment_frequency,
            calendar,
            business_day_convention,
            recovery_rate,
        }
    }

}

super::define_convention_factories! {
    for CdsConvention;
    /// Returns the ISDA standard CDS convention (North America, 40% recovery).
    isda_na => {
        day_count: DayCounter::Actual360, payment_frequency: Frequency::Quarterly,
        calendar: CalendarId::NewYork, business_day_convention: BusinessDayConvention::Following,
        recovery_rate: 0.40,
    };
    /// Returns the ISDA standard CDS convention (Europe, 40% recovery).
    isda_eu => {
        day_count: DayCounter::Actual360, payment_frequency: Frequency::Quarterly,
        calendar: CalendarId::Target, business_day_convention: BusinessDayConvention::Following,
        recovery_rate: 0.40,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cds_conventions() {
        let na = CdsConvention::isda_na();
        assert_eq!(na.day_count, DayCounter::Actual360);
        assert_eq!(na.payment_frequency, Frequency::Quarterly);
        assert_eq!(na.recovery_rate, 0.40);
        assert_eq!(na.calendar, CalendarId::NewYork);

        let eu = CdsConvention::isda_eu();
        assert_eq!(eu.calendar, CalendarId::Target);
        assert_eq!(eu.recovery_rate, 0.40);
    }
}
