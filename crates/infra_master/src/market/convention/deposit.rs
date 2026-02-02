//! Deposit convention definitions.
//!
//! This module provides types for short-term deposit (money market) conventions.

use crate::time::{BusinessDayConvention, CalendarId, DayCounter};

/// Convention for a deposit (money market) instrument.
///
/// Represents the market conventions for pricing and settling deposit instruments.
///
/// # Example
///
/// ```rust
/// use infra_master::market::convention::DepositConvention;
///
/// let conv = DepositConvention::usd();
/// assert_eq!(conv.spot_lag, 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DepositConvention {
    /// Day count convention for accrual calculation.
    pub day_count: DayCounter,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention for date adjustments.
    pub business_day_convention: BusinessDayConvention,
    /// Number of business days from trade date to settlement (spot lag).
    pub spot_lag: u32,
}

impl DepositConvention {
    /// Creates a new deposit convention.
    #[must_use]
    pub fn new(
        day_count: DayCounter,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        spot_lag: u32,
    ) -> Self {
        Self {
            day_count,
            calendar,
            business_day_convention,
            spot_lag,
        }
    }

    /// Returns the USD deposit convention.
    ///
    /// - Day count: ACT/360
    /// - Calendar: New York
    /// - Business day convention: Modified Following
    /// - Spot lag: T+2
    #[must_use]
    pub fn usd() -> Self {
        Self {
            day_count: DayCounter::Actual360,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the EUR deposit convention.
    ///
    /// - Day count: ACT/360
    /// - Calendar: TARGET
    /// - Business day convention: Modified Following
    /// - Spot lag: T+2
    #[must_use]
    pub fn eur() -> Self {
        Self {
            day_count: DayCounter::Actual360,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the GBP deposit convention.
    ///
    /// - Day count: ACT/365 Fixed
    /// - Calendar: London
    /// - Business day convention: Modified Following
    /// - Spot lag: T+0 (same day settlement)
    #[must_use]
    pub fn gbp() -> Self {
        Self {
            day_count: DayCounter::Actual365Fixed,
            calendar: CalendarId::London,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 0,
        }
    }

    /// Returns the JPY deposit convention.
    ///
    /// - Day count: ACT/365 Fixed
    /// - Calendar: Tokyo
    /// - Business day convention: Modified Following
    /// - Spot lag: T+2
    #[must_use]
    pub fn jpy() -> Self {
        Self {
            day_count: DayCounter::Actual365Fixed,
            calendar: CalendarId::Tokyo,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the CHF deposit convention.
    ///
    /// - Day count: ACT/360
    /// - Calendar: TARGET (commonly used for CHF)
    /// - Business day convention: Modified Following
    /// - Spot lag: T+2
    #[must_use]
    pub fn chf() -> Self {
        Self {
            day_count: DayCounter::Actual360,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the AUD deposit convention.
    ///
    /// - Day count: ACT/365 Fixed
    /// - Calendar: WeekendOnly (placeholder for Sydney)
    /// - Business day convention: Modified Following
    /// - Spot lag: T+2
    #[must_use]
    pub fn aud() -> Self {
        Self {
            day_count: DayCounter::Actual365Fixed,
            calendar: CalendarId::WeekendOnly,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the CAD deposit convention.
    ///
    /// - Day count: ACT/365 Fixed
    /// - Calendar: WeekendOnly (placeholder for Toronto)
    /// - Business day convention: Modified Following
    /// - Spot lag: T+1
    #[must_use]
    pub fn cad() -> Self {
        Self {
            day_count: DayCounter::Actual365Fixed,
            calendar: CalendarId::WeekendOnly,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_convention_new() {
        let conv = DepositConvention::new(
            DayCounter::Actual360,
            CalendarId::NewYork,
            BusinessDayConvention::ModifiedFollowing,
            2,
        );

        assert_eq!(conv.day_count, DayCounter::Actual360);
        assert_eq!(conv.calendar, CalendarId::NewYork);
        assert_eq!(conv.business_day_convention, BusinessDayConvention::ModifiedFollowing);
        assert_eq!(conv.spot_lag, 2);
    }

    #[test]
    fn test_usd_deposit_convention() {
        let conv = DepositConvention::usd();

        assert_eq!(conv.day_count, DayCounter::Actual360);
        assert_eq!(conv.calendar, CalendarId::NewYork);
        assert_eq!(conv.business_day_convention, BusinessDayConvention::ModifiedFollowing);
        assert_eq!(conv.spot_lag, 2);
    }

    #[test]
    fn test_eur_deposit_convention() {
        let conv = DepositConvention::eur();

        assert_eq!(conv.day_count, DayCounter::Actual360);
        assert_eq!(conv.calendar, CalendarId::Target);
        assert_eq!(conv.business_day_convention, BusinessDayConvention::ModifiedFollowing);
        assert_eq!(conv.spot_lag, 2);
    }

    #[test]
    fn test_gbp_deposit_convention() {
        let conv = DepositConvention::gbp();

        assert_eq!(conv.day_count, DayCounter::Actual365Fixed);
        assert_eq!(conv.calendar, CalendarId::London);
        assert_eq!(conv.business_day_convention, BusinessDayConvention::ModifiedFollowing);
        assert_eq!(conv.spot_lag, 0); // Same day settlement
    }

    #[test]
    fn test_jpy_deposit_convention() {
        let conv = DepositConvention::jpy();

        assert_eq!(conv.day_count, DayCounter::Actual365Fixed);
        assert_eq!(conv.calendar, CalendarId::Tokyo);
        assert_eq!(conv.business_day_convention, BusinessDayConvention::ModifiedFollowing);
        assert_eq!(conv.spot_lag, 2);
    }

    #[test]
    fn test_chf_deposit_convention() {
        let conv = DepositConvention::chf();

        assert_eq!(conv.day_count, DayCounter::Actual360);
        assert_eq!(conv.calendar, CalendarId::Target);
        assert_eq!(conv.spot_lag, 2);
    }

    #[test]
    fn test_aud_deposit_convention() {
        let conv = DepositConvention::aud();

        assert_eq!(conv.day_count, DayCounter::Actual365Fixed);
        assert_eq!(conv.calendar, CalendarId::WeekendOnly);
        assert_eq!(conv.spot_lag, 2);
    }

    #[test]
    fn test_cad_deposit_convention() {
        let conv = DepositConvention::cad();

        assert_eq!(conv.day_count, DayCounter::Actual365Fixed);
        assert_eq!(conv.calendar, CalendarId::WeekendOnly);
        assert_eq!(conv.spot_lag, 1); // T+1 for CAD
    }

    #[test]
    fn test_deposit_convention_clone() {
        let conv = DepositConvention::usd();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }

    #[test]
    fn test_deposit_convention_debug() {
        let conv = DepositConvention::usd();
        let debug_str = format!("{:?}", conv);
        assert!(debug_str.contains("DepositConvention"));
        assert!(debug_str.contains("Actual360"));
    }
}
