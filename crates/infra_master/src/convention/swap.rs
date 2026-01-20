//! Swap convention definitions.
//!
//! This module provides types for representing interest rate swap conventions.

use crate::{BusinessDayConvention, CalendarId, DayCountConvention, Frequency, RateIndex};

/// Convention for a single leg of a swap.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwapLegConvention {
    /// Day count convention for this leg.
    pub day_count: DayCountConvention,
    /// Payment frequency.
    pub payment_frequency: Frequency,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Number of days between end of accrual and payment.
    pub payment_lag: u32,
}

impl SwapLegConvention {
    /// Creates a new swap leg convention.
    #[must_use]
    pub fn new(
        day_count: DayCountConvention,
        payment_frequency: Frequency,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        payment_lag: u32,
    ) -> Self {
        Self {
            day_count,
            payment_frequency,
            calendar,
            business_day_convention,
            payment_lag,
        }
    }
}

/// Convention for an interest rate swap.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwapConvention {
    /// Convention for the fixed leg.
    pub fixed_leg: SwapLegConvention,
    /// Convention for the floating leg.
    pub float_leg: SwapLegConvention,
    /// Rate index for the floating leg.
    pub float_index: RateIndex,
    /// Number of spot days from trade date to start date.
    pub spot_lag: u32,
}

impl SwapConvention {
    /// Creates a new swap convention.
    #[must_use]
    pub fn new(
        fixed_leg: SwapLegConvention,
        float_leg: SwapLegConvention,
        float_index: RateIndex,
        spot_lag: u32,
    ) -> Self {
        Self {
            fixed_leg,
            float_leg,
            float_index,
            spot_lag,
        }
    }

    /// Returns the USD SOFR swap convention.
    ///
    /// - Fixed leg: Annual, ACT/360, NY calendar, Modified Following
    /// - Float leg: Annual, ACT/360, NY calendar, Modified Following (SOFR compounded)
    /// - Spot lag: 2 days
    #[must_use]
    pub fn usd_sofr() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual360,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual360,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_index: RateIndex::Sofr,
            spot_lag: 2,
        }
    }

    /// Returns the EUR EURIBOR 6M swap convention.
    ///
    /// - Fixed leg: Annual, 30/360, TARGET calendar, Modified Following
    /// - Float leg: Semi-Annual, ACT/360, TARGET calendar, Modified Following
    /// - Spot lag: 2 days
    #[must_use]
    pub fn eur_euribor_6m() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCountConvention::Thirty360Bond,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual360,
                payment_frequency: Frequency::SemiAnnual,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_index: RateIndex::Euribor6M,
            spot_lag: 2,
        }
    }

    /// Returns the JPY TONAR swap convention.
    ///
    /// - Fixed leg: Annual, ACT/365, Tokyo calendar, Modified Following
    /// - Float leg: Annual, ACT/365, Tokyo calendar, Modified Following
    /// - Spot lag: 2 days
    #[must_use]
    pub fn jpy_tonar() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::Tokyo,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::Tokyo,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_index: RateIndex::Tonar,
            spot_lag: 2,
        }
    }

    /// Returns the GBP SONIA swap convention.
    ///
    /// - Fixed leg: Annual, ACT/365, London calendar, Modified Following
    /// - Float leg: Annual, ACT/365, London calendar, Modified Following
    /// - Spot lag: 0 days (same day)
    #[must_use]
    pub fn gbp_sonia() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::London,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 0,
            },
            float_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::London,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 0,
            },
            float_index: RateIndex::Sonia,
            spot_lag: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_leg_convention_new() {
        let leg = SwapLegConvention::new(
            DayCountConvention::Actual360,
            Frequency::Annual,
            CalendarId::NewYork,
            BusinessDayConvention::ModifiedFollowing,
            2,
        );

        assert_eq!(leg.day_count, DayCountConvention::Actual360);
        assert_eq!(leg.payment_frequency, Frequency::Annual);
        assert_eq!(leg.calendar, CalendarId::NewYork);
        assert_eq!(
            leg.business_day_convention,
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(leg.payment_lag, 2);
    }

    #[test]
    fn test_swap_convention_new() {
        let fixed_leg = SwapLegConvention::new(
            DayCountConvention::Thirty360Bond,
            Frequency::SemiAnnual,
            CalendarId::Target,
            BusinessDayConvention::ModifiedFollowing,
            2,
        );
        let float_leg = SwapLegConvention::new(
            DayCountConvention::Actual360,
            Frequency::Quarterly,
            CalendarId::Target,
            BusinessDayConvention::ModifiedFollowing,
            2,
        );
        let conv = SwapConvention::new(fixed_leg, float_leg, RateIndex::Euribor3M, 2);

        assert_eq!(conv.float_index, RateIndex::Euribor3M);
        assert_eq!(conv.spot_lag, 2);
    }

    #[test]
    fn test_usd_sofr_convention() {
        let conv = SwapConvention::usd_sofr();

        assert_eq!(conv.float_index, RateIndex::Sofr);
        assert_eq!(conv.spot_lag, 2);
        assert_eq!(conv.fixed_leg.day_count, DayCountConvention::Actual360);
        assert_eq!(conv.fixed_leg.payment_frequency, Frequency::Annual);
        assert_eq!(conv.fixed_leg.calendar, CalendarId::NewYork);
    }

    #[test]
    fn test_eur_euribor_6m_convention() {
        let conv = SwapConvention::eur_euribor_6m();

        assert_eq!(conv.float_index, RateIndex::Euribor6M);
        assert_eq!(conv.spot_lag, 2);
        assert_eq!(conv.fixed_leg.day_count, DayCountConvention::Thirty360Bond);
        assert_eq!(conv.fixed_leg.payment_frequency, Frequency::Annual);
        assert_eq!(conv.fixed_leg.calendar, CalendarId::Target);
        assert_eq!(conv.float_leg.payment_frequency, Frequency::SemiAnnual);
    }

    #[test]
    fn test_jpy_tonar_convention() {
        let conv = SwapConvention::jpy_tonar();

        assert_eq!(conv.float_index, RateIndex::Tonar);
        assert_eq!(conv.spot_lag, 2);
        assert_eq!(conv.fixed_leg.day_count, DayCountConvention::Actual365Fixed);
        assert_eq!(conv.fixed_leg.calendar, CalendarId::Tokyo);
    }

    #[test]
    fn test_gbp_sonia_convention() {
        let conv = SwapConvention::gbp_sonia();

        assert_eq!(conv.float_index, RateIndex::Sonia);
        assert_eq!(conv.spot_lag, 0); // SONIA swaps start same day
        assert_eq!(conv.fixed_leg.day_count, DayCountConvention::Actual365Fixed);
        assert_eq!(conv.fixed_leg.calendar, CalendarId::London);
    }

    #[test]
    fn test_swap_convention_clone() {
        let conv = SwapConvention::usd_sofr();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }
}
