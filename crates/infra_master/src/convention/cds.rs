//! CDS convention definitions.
//!
//! This module provides types for credit default swap conventions.

use crate::{BusinessDayConvention, CalendarId, DayCountConvention, Frequency};

/// Convention for a credit default swap.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CdsConvention {
    /// Day count convention.
    pub day_count: DayCountConvention,
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
        day_count: DayCountConvention,
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

    /// Returns the ISDA standard CDS convention (North America).
    #[must_use]
    pub fn isda_na() -> Self {
        Self {
            day_count: DayCountConvention::Actual360,
            payment_frequency: Frequency::Quarterly,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::Following,
            recovery_rate: 0.40, // 40% standard recovery
        }
    }

    /// Returns the ISDA standard CDS convention (Europe).
    #[must_use]
    pub fn isda_eu() -> Self {
        Self {
            day_count: DayCountConvention::Actual360,
            payment_frequency: Frequency::Quarterly,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::Following,
            recovery_rate: 0.40,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cds_convention_new() {
        let conv = CdsConvention::new(
            DayCountConvention::Actual360,
            Frequency::Quarterly,
            CalendarId::NewYork,
            BusinessDayConvention::Following,
            0.40,
        );
        assert_eq!(conv.recovery_rate, 0.40);
    }

    #[test]
    fn test_isda_na_convention() {
        let conv = CdsConvention::isda_na();
        assert_eq!(conv.day_count, DayCountConvention::Actual360);
        assert_eq!(conv.payment_frequency, Frequency::Quarterly);
        assert_eq!(conv.recovery_rate, 0.40);
    }

    #[test]
    fn test_isda_eu_convention() {
        let conv = CdsConvention::isda_eu();
        assert_eq!(conv.calendar, CalendarId::Target);
    }
}
