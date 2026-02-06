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

    /// Returns the standard US equity convention.
    ///
    /// - Settlement: T+2
    /// - Calendar: New York
    /// - Dividends: Discrete
    /// - Settlement type: Cash
    #[must_use]
    pub fn us_equity() -> Self {
        Self {
            settlement_days: 2,
            calendar: CalendarId::NewYork,
            dividend_convention: DividendConvention::DiscreteDividends,
            settlement_type: EquitySettlementType::Cash,
            premium_adjusted_delta: false,
            borrow_spread: 0.0,
        }
    }

    /// Returns the standard European equity convention.
    ///
    /// - Settlement: T+2
    /// - Calendar: TARGET
    /// - Dividends: Discrete
    /// - Settlement type: Cash
    #[must_use]
    pub fn eu_equity() -> Self {
        Self {
            settlement_days: 2,
            calendar: CalendarId::Target,
            dividend_convention: DividendConvention::DiscreteDividends,
            settlement_type: EquitySettlementType::Cash,
            premium_adjusted_delta: false,
            borrow_spread: 0.0,
        }
    }

    /// Returns the standard UK equity convention.
    ///
    /// - Settlement: T+2
    /// - Calendar: London
    /// - Dividends: Discrete
    /// - Settlement type: Cash
    #[must_use]
    pub fn uk_equity() -> Self {
        Self {
            settlement_days: 2,
            calendar: CalendarId::London,
            dividend_convention: DividendConvention::DiscreteDividends,
            settlement_type: EquitySettlementType::Cash,
            premium_adjusted_delta: false,
            borrow_spread: 0.0,
        }
    }

    /// Returns the standard Japanese equity convention.
    ///
    /// - Settlement: T+2
    /// - Calendar: Tokyo
    /// - Dividends: Discrete
    /// - Settlement type: Cash
    #[must_use]
    pub fn jp_equity() -> Self {
        Self {
            settlement_days: 2,
            calendar: CalendarId::Tokyo,
            dividend_convention: DividendConvention::DiscreteDividends,
            settlement_type: EquitySettlementType::Cash,
            premium_adjusted_delta: false,
            borrow_spread: 0.0,
        }
    }

    /// Returns the equity index convention (no dividends in index total
    /// return).
    ///
    /// - Settlement: T+1
    /// - Dividends: None (total return index)
    #[must_use]
    pub fn index_total_return() -> Self {
        Self {
            settlement_days: 1,
            calendar: CalendarId::NewYork,
            dividend_convention: DividendConvention::None,
            settlement_type: EquitySettlementType::Cash,
            premium_adjusted_delta: false,
            borrow_spread: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equity_convention_new() {
        let conv = EquityConvention::new(
            3,
            CalendarId::London,
            DividendConvention::ContinuousYield,
            EquitySettlementType::Physical,
            true,
            0.005,
        );

        assert_eq!(conv.settlement_days, 3);
        assert_eq!(conv.calendar, CalendarId::London);
        assert_eq!(
            conv.dividend_convention,
            DividendConvention::ContinuousYield
        );
        assert_eq!(conv.settlement_type, EquitySettlementType::Physical);
        assert!(conv.premium_adjusted_delta);
        assert!((conv.borrow_spread - 0.005).abs() < 1e-10);
    }

    #[test]
    fn test_us_equity_convention() {
        let conv = EquityConvention::us_equity();

        assert_eq!(conv.settlement_days, 2);
        assert_eq!(conv.calendar, CalendarId::NewYork);
        assert_eq!(
            conv.dividend_convention,
            DividendConvention::DiscreteDividends
        );
        assert_eq!(conv.settlement_type, EquitySettlementType::Cash);
    }

    #[test]
    fn test_eu_equity_convention() {
        let conv = EquityConvention::eu_equity();

        assert_eq!(conv.settlement_days, 2);
        assert_eq!(conv.calendar, CalendarId::Target);
    }

    #[test]
    fn test_uk_equity_convention() {
        let conv = EquityConvention::uk_equity();

        assert_eq!(conv.calendar, CalendarId::London);
    }

    #[test]
    fn test_jp_equity_convention() {
        let conv = EquityConvention::jp_equity();

        assert_eq!(conv.calendar, CalendarId::Tokyo);
    }

    #[test]
    fn test_index_total_return_convention() {
        let conv = EquityConvention::index_total_return();

        assert_eq!(conv.settlement_days, 1);
        assert_eq!(conv.dividend_convention, DividendConvention::None);
    }

    #[test]
    fn test_dividend_convention_equality() {
        assert_eq!(
            DividendConvention::ContinuousYield,
            DividendConvention::ContinuousYield
        );
        assert_ne!(
            DividendConvention::ContinuousYield,
            DividendConvention::DiscreteDividends
        );
    }

    #[test]
    fn test_settlement_type_equality() {
        assert_eq!(EquitySettlementType::Cash, EquitySettlementType::Cash);
        assert_ne!(EquitySettlementType::Cash, EquitySettlementType::Physical);
    }

    #[test]
    fn test_equity_convention_clone() {
        let conv = EquityConvention::us_equity();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }
}
