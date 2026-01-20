//! Futures convention definitions.
//!
//! This module provides types for interest rate futures conventions.

use crate::{CalendarId, DayCountConvention};

/// Convention for an interest rate future.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FuturesConvention {
    /// Contract size (notional per contract).
    pub contract_size: f64,
    /// Tick size (minimum price movement).
    pub tick_size: f64,
    /// Day count convention.
    pub day_count: DayCountConvention,
    /// Calendar for settlement.
    pub calendar: CalendarId,
}

impl FuturesConvention {
    /// Creates a new futures convention.
    #[must_use]
    pub fn new(
        contract_size: f64,
        tick_size: f64,
        day_count: DayCountConvention,
        calendar: CalendarId,
    ) -> Self {
        Self {
            contract_size,
            tick_size,
            day_count,
            calendar,
        }
    }

    /// Returns the CME Eurodollar futures convention.
    #[must_use]
    pub fn cme_eurodollar() -> Self {
        Self {
            contract_size: 1_000_000.0,
            tick_size: 0.0025, // 0.25 basis points
            day_count: DayCountConvention::Actual360,
            calendar: CalendarId::NewYork,
        }
    }

    /// Returns the CME SOFR futures convention.
    #[must_use]
    pub fn cme_sofr() -> Self {
        Self {
            contract_size: 1_000_000.0,
            tick_size: 0.0025,
            day_count: DayCountConvention::Actual360,
            calendar: CalendarId::NewYork,
        }
    }

    /// Returns the Eurex EURIBOR futures convention.
    #[must_use]
    pub fn eurex_euribor() -> Self {
        Self {
            contract_size: 1_000_000.0,
            tick_size: 0.005, // 0.5 basis points
            day_count: DayCountConvention::Actual360,
            calendar: CalendarId::Target,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_futures_convention_new() {
        let conv = FuturesConvention::new(
            1_000_000.0,
            0.0025,
            DayCountConvention::Actual360,
            CalendarId::NewYork,
        );

        assert_eq!(conv.contract_size, 1_000_000.0);
        assert_eq!(conv.tick_size, 0.0025);
    }

    #[test]
    fn test_cme_eurodollar_convention() {
        let conv = FuturesConvention::cme_eurodollar();
        assert_eq!(conv.contract_size, 1_000_000.0);
        assert_eq!(conv.calendar, CalendarId::NewYork);
    }

    #[test]
    fn test_cme_sofr_convention() {
        let conv = FuturesConvention::cme_sofr();
        assert_eq!(conv.contract_size, 1_000_000.0);
        assert_eq!(conv.day_count, DayCountConvention::Actual360);
    }

    #[test]
    fn test_eurex_euribor_convention() {
        let conv = FuturesConvention::eurex_euribor();
        assert_eq!(conv.calendar, CalendarId::Target);
    }

    #[test]
    fn test_futures_convention_clone() {
        let conv = FuturesConvention::cme_sofr();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }
}
