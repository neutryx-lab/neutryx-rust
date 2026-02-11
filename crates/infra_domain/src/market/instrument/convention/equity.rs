//! Equity convention definitions.
//!
//! This module provides types for representing equity market conventions.

use crate::time::CalendarId;

/// Dividend handling convention for equity derivatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DividendConvention {
    /// Continuous dividend yield assumption.
    ContinuousYield,
    /// Discrete dividend payments (absolute amounts).
    DiscreteDividends,
    /// Proportional dividend yield (percentage of spot).
    ProportionalDividends,
    /// No dividend adjustment (total return).
    None,
}

/// Settlement type for equity transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EquitySettlementType {
    /// Cash settlement (pay/receive cash based on price).
    Cash,
    /// Physical delivery of shares.
    Physical,
}

/// Convention for equity derivatives.
///
/// Represents the market conventions for pricing and settling equity
/// derivatives.
///
/// # Example
///
/// ```rust
/// use infra_domain::market::convention::{
///     EquityConvention, DividendConvention, EquitySettlementType,
/// };
/// use infra_domain::time::CalendarId;
///
/// let conv = EquityConvention::us_equity();
/// assert_eq!(conv.settlement_days, 2);
/// assert_eq!(conv.dividend_convention, DividendConvention::DiscreteDividends);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EquityConvention {
    /// Number of business days to settlement.
    pub settlement_days: u32,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Dividend handling convention.
    pub dividend_convention: DividendConvention,
    /// Default settlement type.
    pub settlement_type: EquitySettlementType,
    /// Whether options use premium-adjusted delta.
    pub premium_adjusted_delta: bool,
    /// Borrow rate spread (for short selling).
    pub borrow_spread: f64,
}

impl EquityConvention {
    /// Creates a new equity convention.
    #[must_use]
    pub fn new(
        settlement_days: u32,
        calendar: CalendarId,
        dividend_convention: DividendConvention,
        settlement_type: EquitySettlementType,
        premium_adjusted_delta: bool,
        borrow_spread: f64,
    ) -> Self {
        Self {
            settlement_days,
            calendar,
            dividend_convention,
            settlement_type,
            premium_adjusted_delta,
            borrow_spread,
        }
    }

}

super::define_convention_factories! {
    for EquityConvention;
    /// Returns the standard US equity convention (T+2, NY, Discrete, Cash).
    us_equity => {
        settlement_days: 2, calendar: CalendarId::NewYork,
        dividend_convention: DividendConvention::DiscreteDividends,
        settlement_type: EquitySettlementType::Cash,
        premium_adjusted_delta: false, borrow_spread: 0.0,
    };
    /// Returns the standard European equity convention (T+2, TARGET, Discrete, Cash).
    eu_equity => {
        settlement_days: 2, calendar: CalendarId::Target,
        dividend_convention: DividendConvention::DiscreteDividends,
        settlement_type: EquitySettlementType::Cash,
        premium_adjusted_delta: false, borrow_spread: 0.0,
    };
    /// Returns the standard UK equity convention (T+2, London, Discrete, Cash).
    uk_equity => {
        settlement_days: 2, calendar: CalendarId::London,
        dividend_convention: DividendConvention::DiscreteDividends,
        settlement_type: EquitySettlementType::Cash,
        premium_adjusted_delta: false, borrow_spread: 0.0,
    };
    /// Returns the standard Japanese equity convention (T+2, Tokyo, Discrete, Cash).
    jp_equity => {
        settlement_days: 2, calendar: CalendarId::Tokyo,
        dividend_convention: DividendConvention::DiscreteDividends,
        settlement_type: EquitySettlementType::Cash,
        premium_adjusted_delta: false, borrow_spread: 0.0,
    };
    /// Returns the equity index convention (T+1, total return, no dividends).
    index_total_return => {
        settlement_days: 1, calendar: CalendarId::NewYork,
        dividend_convention: DividendConvention::None,
        settlement_type: EquitySettlementType::Cash,
        premium_adjusted_delta: false, borrow_spread: 0.0,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equity_presets() {
        let us = EquityConvention::us_equity();
        assert_eq!(us.settlement_days, 2);
        assert_eq!(us.calendar, CalendarId::NewYork);
        assert_eq!(
            us.dividend_convention,
            DividendConvention::DiscreteDividends
        );
        assert_eq!(us.settlement_type, EquitySettlementType::Cash);

        assert_eq!(EquityConvention::eu_equity().calendar, CalendarId::Target);
        assert_eq!(EquityConvention::uk_equity().calendar, CalendarId::London);
        assert_eq!(EquityConvention::jp_equity().calendar, CalendarId::Tokyo);

        let idx = EquityConvention::index_total_return();
        assert_eq!(idx.settlement_days, 1);
        assert_eq!(idx.dividend_convention, DividendConvention::None);
    }
}
