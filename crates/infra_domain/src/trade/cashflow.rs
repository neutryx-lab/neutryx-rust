//! Cashflow types for financial instruments.

use super::payoff::Payoff;
use crate::{market::Currency, time::Date};

/// Daily accrual detail for OIS (Overnight Index Swap) compounding.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DailyAccrual {
    /// The date for this daily accrual.
    pub date: Date,

    /// Overnight rate for this date (as decimal, e.g., 0.0425 for 4.25%).
    pub overnight_rate: f64,

    /// Day count fraction for this date (typically 1/360 or 1/365).
    pub day_fraction: f64,

    /// Cumulative compounded notional at end of this date.
    pub compounded_notional: f64,
}

impl DailyAccrual {
    /// Creates a new daily accrual.
    #[must_use]
    pub fn new(date: Date, overnight_rate: f64, day_fraction: f64, starting_notional: f64) -> Self {
        let compounded_notional = starting_notional * (1.0 + overnight_rate * day_fraction);
        Self {
            date,
            overnight_rate,
            day_fraction,
            compounded_notional,
        }
    }

    /// Creates a daily accrual with pre-calculated compounded notional.
    #[must_use]
    pub fn with_compounded_notional(
        date: Date,
        overnight_rate: f64,
        day_fraction: f64,
        compounded_notional: f64,
    ) -> Self {
        Self {
            date,
            overnight_rate,
            day_fraction,
            compounded_notional,
        }
    }

    /// Returns the daily interest earned.
    #[must_use]
    pub fn daily_interest(&self, starting_notional: f64) -> f64 {
        self.compounded_notional - starting_notional
    }
}

/// Type of cashflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    pub fn is_coupon(&self) -> bool { matches!(self, CashflowType::Coupon) }

    /// Returns true if this is a principal payment.
    #[must_use]
    pub fn is_principal(&self) -> bool { matches!(self, CashflowType::Principal) }
}

/// A single cashflow in a financial instrument.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

    /// Daily accrual details for OIS compounding.
    pub daily_accruals: Option<Vec<DailyAccrual>>,
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
            daily_accruals: None,
        }
    }

    /// Creates a new OIS cashflow with daily accrual details.
    #[must_use]
    pub fn new_with_daily_accruals(
        cf_type: CashflowType,
        payment_date: Date,
        accrual_start: Date,
        accrual_end: Date,
        year_fraction: f64,
        notional: f64,
        payoff: Payoff,
        currency: Currency,
        daily_accruals: Vec<DailyAccrual>,
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
            daily_accruals: Some(daily_accruals),
        }
    }

    /// Returns true if this cashflow has daily accrual details.
    #[must_use]
    pub fn has_daily_accruals(&self) -> bool { self.daily_accruals.is_some() }

    /// Returns the daily accrual details if present.
    #[must_use]
    pub fn daily_accruals(&self) -> Option<&[DailyAccrual]> { self.daily_accruals.as_deref() }

    /// Returns true if this cashflow has a fixed rate (no index dependency).
    #[must_use]
    pub fn is_fixed(&self) -> bool { self.payoff.is_fixed() }

    /// Returns true if this cashflow's payment date is in the future.
    #[must_use]
    pub fn is_future(&self, ref_date: Date) -> bool { self.payment_date > ref_date }

    /// Returns the number of accrual days.
    #[must_use]
    pub fn accrual_days(&self) -> i64 { self.accrual_end - self.accrual_start }

    /// Returns the cashflow type.
    #[must_use]
    pub fn cashflow_type(&self) -> CashflowType { self.cf_type }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        market::RateIndex,
        trade::{IndexType, Payoff},
    };

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
        assert!(!cf.is_future(ref_date));
    }

    #[test]
    fn test_cashflow_accrual_days() {
        let cf = make_fixed_cashflow();
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
            Payoff::fixed(1.0),
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
        set.insert(CashflowType::Coupon);
        assert_eq!(set.len(), 2);
    }
}
