//! Cashflow types for financial instruments.

use super::{payoff::Payoff, sub_schedule::SubSchedule};
use crate::{market::Currency, time::Date};

/// Compounding method for multi-period coupons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum CompoundType {
    /// No compounding (single-period coupon).
    #[default]
    None,
    /// Arithmetic average of sub-period rates.
    Average,
    /// Standard compounding: (1+r1*d1)(1+r2*d2)...-1.
    Straight,
    /// Flat compounding (spread compounds on notional only).
    Flat,
    /// Spread-exclusive compounding.
    SpreadExclusive,
    /// Linear product of rates.
    LinearProduct,
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
    /// Premium payment (e.g., option premium).
    Premium,
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

    /// Fixing date for floating rate observation.
    pub fixing_date: Option<Date>,

    /// Compounding method for multi-period coupons.
    pub compound_type: CompoundType,

    /// Sub-schedule details for compounding/averaging.
    pub sub_schedules: Option<Vec<SubSchedule>>,
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
            fixing_date: None,
            compound_type: CompoundType::None,
            sub_schedules: None,
        }
    }

    /// Creates a cashflow with sub-schedules for compounding/averaging.
    #[must_use]
    pub fn new_with_sub_schedules(
        cf_type: CashflowType,
        payment_date: Date,
        accrual_start: Date,
        accrual_end: Date,
        year_fraction: f64,
        notional: f64,
        payoff: Payoff,
        currency: Currency,
        compound_type: CompoundType,
        sub_schedules: Vec<SubSchedule>,
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
            fixing_date: None,
            compound_type,
            sub_schedules: Some(sub_schedules),
        }
    }

    /// Sets the fixing date (builder pattern).
    #[must_use]
    pub fn with_fixing_date(mut self, date: Date) -> Self {
        self.fixing_date = Some(date);
        self
    }

    /// Returns the fixing date if set.
    #[must_use]
    pub fn fixing_date(&self) -> Option<Date> { self.fixing_date }

    /// Returns true if this cashflow has sub-schedule details.
    #[must_use]
    pub fn has_sub_schedules(&self) -> bool { self.sub_schedules.is_some() }

    /// Returns the sub-schedule details if present.
    #[must_use]
    pub fn sub_schedules(&self) -> Option<&[SubSchedule]> { self.sub_schedules.as_deref() }

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
        assert!(cf.fixing_date.is_none());
        assert_eq!(cf.compound_type, CompoundType::None);
        assert!(!cf.has_sub_schedules());
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

    #[test]
    fn test_cashflow_with_fixing_date() {
        let fixing = Date::from_ymd(2025, 6, 29).unwrap();
        let cf = make_floating_cashflow().with_fixing_date(fixing);
        assert_eq!(cf.fixing_date(), Some(fixing));
    }

    #[test]
    fn test_cashflow_compound_type_default() {
        let cf = make_fixed_cashflow();
        assert_eq!(cf.compound_type, CompoundType::None);
    }

    #[test]
    fn test_cashflow_with_sub_schedules() {
        let sub = SubSchedule::new(
            Date::from_ymd(2025, 1, 1).unwrap(),
            Date::from_ymd(2025, 1, 1).unwrap(),
            Date::from_ymd(2025, 1, 2).unwrap(),
            1.0 / 360.0,
            Payoff::floating(IndexType::Rate(RateIndex::Sofr)),
        );
        let cf = Cashflow::new_with_sub_schedules(
            CashflowType::Coupon,
            Date::from_ymd(2025, 7, 1).unwrap(),
            Date::from_ymd(2025, 1, 1).unwrap(),
            Date::from_ymd(2025, 7, 1).unwrap(),
            0.5,
            1_000_000.0,
            Payoff::floating(IndexType::Rate(RateIndex::Sofr)),
            Currency::USD,
            CompoundType::Straight,
            vec![sub],
        );
        assert!(cf.has_sub_schedules());
        assert_eq!(cf.sub_schedules().unwrap().len(), 1);
        assert_eq!(cf.compound_type, CompoundType::Straight);
    }
}
