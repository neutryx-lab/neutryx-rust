//! Bond convention definitions.
//!
//! This module provides types for bond conventions.

use crate::time::{BusinessDayConvention, CalendarId, DayCounter, Frequency};

/// Convention for a bond.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BondConvention {
    /// Day count convention.
    pub day_count: DayCounter,
    /// Coupon payment frequency.
    pub coupon_frequency: Frequency,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Number of settlement days.
    pub settlement_days: u32,
}

impl BondConvention {
    /// Creates a new bond convention.
    #[must_use]
    pub fn new(
        day_count: DayCounter,
        coupon_frequency: Frequency,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        settlement_days: u32,
    ) -> Self {
        Self {
            day_count,
            coupon_frequency,
            calendar,
            business_day_convention,
            settlement_days,
        }
    }

    /// Returns the US Treasury bond convention.
    #[must_use]
    pub fn us_treasury() -> Self {
        Self {
            day_count: DayCounter::ActualActualIsda,
            coupon_frequency: Frequency::SemiAnnual,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::Following,
            settlement_days: 1,
        }
    }

    /// Returns the UK Gilt convention.
    #[must_use]
    pub fn uk_gilt() -> Self {
        Self {
            day_count: DayCounter::ActualActualIsda,
            coupon_frequency: Frequency::SemiAnnual,
            calendar: CalendarId::London,
            business_day_convention: BusinessDayConvention::Following,
            settlement_days: 1,
        }
    }

    /// Returns the German Bund convention.
    #[must_use]
    pub fn german_bund() -> Self {
        Self {
            day_count: DayCounter::ActualActualIsda,
            coupon_frequency: Frequency::Annual,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::Following,
            settlement_days: 2,
        }
    }

    /// Returns the JGB (Japanese Government Bond) convention.
    #[must_use]
    pub fn jgb() -> Self {
        Self {
            day_count: DayCounter::Actual365Fixed,
            coupon_frequency: Frequency::SemiAnnual,
            calendar: CalendarId::Tokyo,
            business_day_convention: BusinessDayConvention::Following,
            settlement_days: 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bond_convention_new() {
        let conv = BondConvention::new(
            DayCounter::Thirty360Bond,
            Frequency::SemiAnnual,
            CalendarId::NewYork,
            BusinessDayConvention::Following,
            2,
        );
        assert_eq!(conv.settlement_days, 2);
    }

    #[test]
    fn test_us_treasury_convention() {
        let conv = BondConvention::us_treasury();
        assert_eq!(conv.coupon_frequency, Frequency::SemiAnnual);
        assert_eq!(conv.settlement_days, 1);
    }

    #[test]
    fn test_uk_gilt_convention() {
        let conv = BondConvention::uk_gilt();
        assert_eq!(conv.calendar, CalendarId::London);
    }

    #[test]
    fn test_german_bund_convention() {
        let conv = BondConvention::german_bund();
        assert_eq!(conv.coupon_frequency, Frequency::Annual);
        assert_eq!(conv.settlement_days, 2);
    }

    #[test]
    fn test_jgb_convention() {
        let conv = BondConvention::jgb();
        assert_eq!(conv.calendar, CalendarId::Tokyo);
        assert_eq!(conv.settlement_days, 3);
    }
}
