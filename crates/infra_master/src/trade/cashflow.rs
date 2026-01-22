//! Cashflow types for financial instruments.
//!
//! This module provides the fundamental unit of financial calculations:
//! the cashflow.

use crate::{Currency, Date};

use super::payoff::Payoff;

/// Type of cashflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CashflowType {
    /// Interest payment (coupon).
    Coupon,
    /// Principal exchange or redemption.
    Principal,
    /// Fee payment.
    Fee,
    /// Settlement payment (e.g., option exercise).
    Settlement,
}

impl CashflowType {
    /// Returns true if this is a coupon payment.
    #[must_use]
    pub fn is_coupon(&self) -> bool {
        matches!(self, CashflowType::Coupon)
    }

    /// Returns true if this is a principal payment.
    #[must_use]
    pub fn is_principal(&self) -> bool {
        matches!(self, CashflowType::Principal)
    }
}

/// A single cashflow in a financial instrument.
///
/// Represents the smallest unit of a financial trade: a single payment
/// at a specific date with a defined calculation formula.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cashflow {
    /// Type of this cashflow.
    pub cf_type: CashflowType,

    /// Date when payment occurs.
    pub payment_date: Date,

    /// Start of accrual period.
    pub accrual_start: Date,

    /// End of accrual period.
    pub accrual_end: Date,

    /// Year fraction for the accrual period.
    pub year_fraction: f64,

    /// Notional amount.
    pub notional: f64,

    /// Payoff formula.
    pub payoff: Payoff,

    /// Currency of the cashflow.
    pub currency: Currency,
}

impl Cashflow {
    /// Creates a new cashflow.
    #[must_use]
    pub fn new(
        cf_type: CashflowType,
        payment_date: Date,
        accrual_start: Date,
        accrual_end: Date,
        year_fraction: f64,
        notional: f64,
        payoff: Payoff,
        currency: Currency,
    ) -> Self {
        Self {
            cf_type,
            payment_date,
            accrual_start,
            accrual_end,
            year_fraction,
            notional,
            payoff,
            currency,
        }
    }

    /// Returns true if this cashflow has a fixed rate (no index dependency).
    #[must_use]
    pub fn is_fixed(&self) -> bool {
        self.payoff.is_fixed()
    }

    /// Returns true if this cashflow's payment date is in the future.
    #[must_use]
    pub fn is_future(&self, ref_date: Date) -> bool {
        self.payment_date > ref_date
    }

    /// Returns the number of accrual days.
    #[must_use]
    pub fn accrual_days(&self) -> i64 {
        self.accrual_end - self.accrual_start
    }

    /// Returns the cashflow type.
    #[must_use]
    pub fn cashflow_type(&self) -> CashflowType {
        self.cf_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::{IndexType, Payoff};
    use crate::RateIndex;

    fn make_fixed_cashflow() -> Cashflow {
        Cashflow::new(
            CashflowType::Coupon,
            Date::from_ymd(2025, 7, 1).unwrap(),
            Date::from_ymd(2025, 1, 1).unwrap(),
            Date::from_ymd(2025, 7, 1).unwrap(),
            0.5,
            1_000_000.0,
            Payoff::fixed(0.05),
            Currency::USD,
        )
    }

    fn make_floating_cashflow() -> Cashflow {
        Cashflow::new(
            CashflowType::Coupon,
            Date::from_ymd(2025, 7, 1).unwrap(),
            Date::from_ymd(2025, 1, 1).unwrap(),
            Date::from_ymd(2025, 7, 1).unwrap(),
            0.5,
            1_000_000.0,
            Payoff::floating(IndexType::Rate(RateIndex::Sofr)),
            Currency::USD,
        )
    }

    #[test]
    fn test_cashflow_type_is_coupon() {
        assert!(CashflowType::Coupon.is_coupon());
        assert!(!CashflowType::Principal.is_coupon());
        assert!(!CashflowType::Fee.is_coupon());
        assert!(!CashflowType::Settlement.is_coupon());
    }

    #[test]
    fn test_cashflow_type_is_principal() {
        assert!(!CashflowType::Coupon.is_principal());
        assert!(CashflowType::Principal.is_principal());
        assert!(!CashflowType::Fee.is_principal());
        assert!(!CashflowType::Settlement.is_principal());
    }

    #[test]
    fn test_cashflow_new() {
        let cf = make_fixed_cashflow();

        assert_eq!(cf.cf_type, CashflowType::Coupon);
        assert_eq!(cf.payment_date, Date::from_ymd(2025, 7, 1).unwrap());
        assert_eq!(cf.notional, 1_000_000.0);
        assert_eq!(cf.currency, Currency::USD);
        assert_eq!(cf.year_fraction, 0.5);
    }

    #[test]
    fn test_cashflow_is_fixed_true() {
        let cf = make_fixed_cashflow();
        assert!(cf.is_fixed());
    }

    #[test]
    fn test_cashflow_is_fixed_false() {
        let cf = make_floating_cashflow();
        assert!(!cf.is_fixed());
    }

    #[test]
    fn test_cashflow_is_future_true() {
        let cf = make_fixed_cashflow();
        let ref_date = Date::from_ymd(2025, 1, 1).unwrap();
        assert!(cf.is_future(ref_date));
    }

    #[test]
    fn test_cashflow_is_future_false() {
        let cf = make_fixed_cashflow();
        let ref_date = Date::from_ymd(2025, 12, 1).unwrap();
        assert!(!cf.is_future(ref_date));
    }

    #[test]
    fn test_cashflow_is_future_same_date() {
        let cf = make_fixed_cashflow();
        let ref_date = Date::from_ymd(2025, 7, 1).unwrap();
        assert!(!cf.is_future(ref_date)); // Same date is NOT future
    }

    #[test]
    fn test_cashflow_accrual_days() {
        let cf = make_fixed_cashflow();
        // From 2025-01-01 to 2025-07-01 = 181 days
        assert_eq!(cf.accrual_days(), 181);
    }

    #[test]
    fn test_cashflow_type_accessor() {
        let cf = make_fixed_cashflow();
        assert_eq!(cf.cashflow_type(), CashflowType::Coupon);
    }

    #[test]
    fn test_cashflow_principal() {
        let cf = Cashflow::new(
            CashflowType::Principal,
            Date::from_ymd(2030, 1, 1).unwrap(),
            Date::from_ymd(2030, 1, 1).unwrap(),
            Date::from_ymd(2030, 1, 1).unwrap(),
            0.0,
            100_000_000.0,
            Payoff::fixed(1.0), // Principal uses rate=1.0
            Currency::EUR,
        );

        assert!(cf.cf_type.is_principal());
        assert_eq!(cf.currency, Currency::EUR);
    }

    #[test]
    fn test_cashflow_clone() {
        let cf = make_fixed_cashflow();
        let cloned = cf.clone();
        assert_eq!(cf, cloned);
    }

    #[test]
    fn test_cashflow_debug() {
        let cf = make_fixed_cashflow();
        let debug = format!("{:?}", cf);
        assert!(debug.contains("Cashflow"));
        assert!(debug.contains("Coupon"));
    }

    #[test]
    fn test_cashflow_type_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(CashflowType::Coupon);
        set.insert(CashflowType::Principal);
        set.insert(CashflowType::Coupon); // Duplicate
        assert_eq!(set.len(), 2);
    }
}
