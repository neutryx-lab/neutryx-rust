//! Credit Default Swap (CDS) implementation.
//!
//! This module provides the [`CreditDefaultSwap`] structure for single-name CDS
//! with protection and premium legs.
//!
//! # Structure
//!
//! A CDS consists of:
//! - **Protection Leg**: Receives payment on credit event (default)
//! - **Premium Leg**: Pays periodic premium (spread) until default or maturity
//!
//! # Example
//!
//! ```
//! use pricer_models::instruments::credit::{CreditDefaultSwap, CdsDirection};
//! use pricer_models::schedules::{ScheduleBuilder, Frequency};
//! use pricer_core::types::{Currency, time::{Date, DayCountConvention}};
//!
//! let start = Date::from_ymd(2024, 3, 20).unwrap();
//! let end = Date::from_ymd(2029, 3, 20).unwrap();
//!
//! let schedule = ScheduleBuilder::new()
//!     .start(start)
//!     .end(end)
//!     .frequency(Frequency::Quarterly)
//!     .day_count(DayCountConvention::ActualActual360)
//!     .build()
//!     .unwrap();
//!
//! let cds: CreditDefaultSwap<f64> = CreditDefaultSwap::new(
//!     "ACME Corp".to_string(),
//!     10_000_000.0,
//!     0.01,  // 100bp spread
//!     0.4,   // 40% recovery rate
//!     schedule,
//!     Currency::USD,
//!     CdsDirection::BuyProtection,
//! );
//!
//! assert_eq!(cds.notional(), 10_000_000.0);
//! assert!((cds.spread() - 0.01).abs() < 1e-10);
//! ```

use num_traits::Float;
use pricer_core::types::time::DayCountConvention;
use pricer_core::types::Currency;
use std::fmt;

use crate::instruments::traits::{Cashflow, CashflowInstrument, InstrumentTrait};
use crate::schedules::Schedule;

/// CDS direction (buyer or seller of protection).
///
/// In CDS market convention:
/// - **BuyProtection**: Buy protection (pay premium, receive payment on default)
/// - **SellProtection**: Sell protection (receive premium, pay on default)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CdsDirection {
    /// Buy protection: pay premium, receive payment on default.
    BuyProtection,
    /// Sell protection: receive premium, pay on default.
    SellProtection,
}

impl CdsDirection {
    /// Returns the multiplier for premium leg cashflows.
    ///
    /// - BuyProtection: -1 (negative = pay)
    /// - SellProtection: +1 (positive = receive)
    #[inline]
    pub fn premium_multiplier<T: Float>(&self) -> T {
        match self {
            CdsDirection::BuyProtection => -T::one(),
            CdsDirection::SellProtection => T::one(),
        }
    }

    /// Returns the multiplier for protection leg cashflows.
    ///
    /// - BuyProtection: +1 (positive = receive on default)
    /// - SellProtection: -1 (negative = pay on default)
    #[inline]
    pub fn protection_multiplier<T: Float>(&self) -> T {
        match self {
            CdsDirection::BuyProtection => T::one(),
            CdsDirection::SellProtection => -T::one(),
        }
    }
}

impl fmt::Display for CdsDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CdsDirection::BuyProtection => write!(f, "Buy Protection"),
            CdsDirection::SellProtection => write!(f, "Sell Protection"),
        }
    }
}

/// Credit Default Swap (CDS).
///
/// A CDS is a credit derivative that provides insurance against default
/// of a reference entity. The protection buyer pays a periodic premium
/// (spread) and receives a payment if the reference entity defaults.
///
/// # Structure
///
/// - **Protection Leg**: Contingent payment of (1 - Recovery) × Notional on default
/// - **Premium Leg**: Periodic payments of Spread × Notional × DayCountFraction
///
/// # Valuation
///
/// The CDS value is the difference between the present values of the
/// two legs, weighted by survival/default probabilities:
/// ```text
/// V = PV(ProtectionLeg) - PV(PremiumLeg)
/// ```
///
/// # Examples
///
/// ```
/// use pricer_models::instruments::credit::{CreditDefaultSwap, CdsDirection};
/// use pricer_models::schedules::{ScheduleBuilder, Frequency};
/// use pricer_core::types::{Currency, time::{Date, DayCountConvention}};
///
/// let start = Date::from_ymd(2024, 3, 20).unwrap();
/// let end = Date::from_ymd(2029, 3, 20).unwrap();
///
/// let schedule = ScheduleBuilder::new()
///     .start(start)
///     .end(end)
///     .frequency(Frequency::Quarterly)
///     .day_count(DayCountConvention::ActualActual360)
///     .build()
///     .unwrap();
///
/// let cds = CreditDefaultSwap::new(
///     "ACME Corp".to_string(),
///     10_000_000.0,
///     0.01,  // 100bp spread
///     0.4,   // 40% recovery rate
///     schedule,
///     Currency::USD,
///     CdsDirection::BuyProtection,
/// );
///
/// assert_eq!(cds.reference_entity(), "ACME Corp");
/// assert_eq!(cds.currency(), Currency::USD);
/// ```
#[derive(Debug, Clone)]
pub struct CreditDefaultSwap<T: Float> {
    /// Reference entity name (issuer/obligor).
    reference_entity: String,
    /// Notional principal amount.
    notional: T,
    /// CDS spread (annual, e.g., 0.01 for 100bp).
    spread: T,
    /// Recovery rate assumption (e.g., 0.4 for 40%).
    recovery_rate: T,
    /// Premium payment schedule.
    schedule: Schedule,
    /// Settlement currency.
    currency: Currency,
    /// CDS direction (buy or sell protection).
    direction: CdsDirection,
}

impl<T: Float> CreditDefaultSwap<T> {
    /// Create a new Credit Default Swap.
    ///
    /// # Arguments
    ///
    /// * `reference_entity` - Name of the reference entity (issuer)
    /// * `notional` - Notional principal amount
    /// * `spread` - Annual CDS spread (e.g., 0.01 for 100bp)
    /// * `recovery_rate` - Assumed recovery rate (e.g., 0.4 for 40%)
    /// * `schedule` - Premium payment schedule
    /// * `currency` - Settlement currency
    /// * `direction` - Buy or sell protection
    pub fn new(
        reference_entity: String,
        notional: T,
        spread: T,
        recovery_rate: T,
        schedule: Schedule,
        currency: Currency,
        direction: CdsDirection,
    ) -> Self {
        Self {
            reference_entity,
            notional,
            spread,
            recovery_rate,
            schedule,
            currency,
            direction,
        }
    }

    /// Returns the reference entity name.
    #[inline]
    pub fn reference_entity(&self) -> &str {
        &self.reference_entity
    }

    /// Returns the notional principal amount.
    #[inline]
    pub fn notional(&self) -> T {
        self.notional
    }

    /// Returns the CDS spread (annual).
    #[inline]
    pub fn spread(&self) -> T {
        self.spread
    }

    /// Returns the recovery rate assumption.
    #[inline]
    pub fn recovery_rate(&self) -> T {
        self.recovery_rate
    }

    /// Returns the loss given default (1 - recovery_rate).
    #[inline]
    pub fn loss_given_default(&self) -> T {
        T::one() - self.recovery_rate
    }

    /// Returns the premium payment schedule.
    #[inline]
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Returns the settlement currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns the CDS direction.
    #[inline]
    pub fn direction(&self) -> CdsDirection {
        self.direction
    }

    /// Returns the maturity time in years (end of final period).
    ///
    /// This calculates the time from the start of the CDS to the
    /// final protection period end date.
    pub fn maturity(&self) -> T {
        // Get the start date and end date from schedule
        let start_date = self.schedule.start_date();
        let end_date = self.schedule.end_date();

        let year_frac =
            DayCountConvention::ActualActual365.year_fraction_dates(start_date, end_date);
        T::from(year_frac).unwrap_or_else(T::zero)
    }

    /// Returns whether this is a protection buyer position.
    #[inline]
    pub fn is_protection_buyer(&self) -> bool {
        self.direction == CdsDirection::BuyProtection
    }

    /// Returns whether this is a protection seller position.
    #[inline]
    pub fn is_protection_seller(&self) -> bool {
        self.direction == CdsDirection::SellProtection
    }

    /// Returns the number of premium payment periods.
    #[inline]
    pub fn num_periods(&self) -> usize {
        self.schedule.len()
    }

    /// Returns the protection payment amount on default.
    ///
    /// This is (1 - RecoveryRate) × Notional.
    #[inline]
    pub fn protection_payment(&self) -> T {
        self.loss_given_default() * self.notional
    }

    /// Computes the premium payment for a single period.
    ///
    /// Premium = Notional × Spread × YearFraction
    ///
    /// # Arguments
    ///
    /// * `year_fraction` - The accrual period year fraction
    #[inline]
    pub fn period_premium(&self, year_fraction: T) -> T {
        self.notional * self.spread * year_fraction
    }
}

impl<T: Float> InstrumentTrait<T> for CreditDefaultSwap<T> {
    /// CDS payoff is not spot-based; returns zero.
    ///
    /// Actual CDS valuation requires credit curve and discount curve.
    fn payoff(&self, _spot: T) -> T {
        T::zero()
    }

    /// Returns the maturity time in years.
    fn expiry(&self) -> T {
        self.maturity()
    }

    /// Returns the settlement currency.
    fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns the notional amount.
    fn notional(&self) -> T {
        self.notional
    }

    /// CDS is not path-dependent in the traditional sense.
    fn is_path_dependent(&self) -> bool {
        false
    }

    /// Returns the instrument type name.
    fn type_name(&self) -> &'static str {
        "CreditDefaultSwap"
    }
}

impl<T: Float> CashflowInstrument<T> for CreditDefaultSwap<T> {
    /// Returns the scheduled premium cashflows.
    ///
    /// Note: This returns only the premium leg cashflows assuming no default.
    /// Protection leg cashflows are contingent on default event.
    fn cashflows(&self) -> Vec<Cashflow<T>> {
        let multiplier: T = self.direction.premium_multiplier();

        self.schedule
            .periods()
            .iter()
            .map(|period| {
                let year_frac = T::from(period.year_fraction()).unwrap_or_else(T::zero);
                let amount = multiplier * self.period_premium(year_frac);

                // Convert payment date to year fraction from start
                let payment_time = T::from(
                    DayCountConvention::ActualActual365
                        .year_fraction_dates(self.schedule.start_date(), period.payment()),
                )
                .unwrap_or_else(T::zero);

                Cashflow::new(payment_time, amount, self.currency)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedules::{Frequency, ScheduleBuilder};
    use pricer_core::types::time::Date;

    // ========================================
    // CdsDirection Tests
    // ========================================

    #[test]
    fn test_cds_direction_multipliers() {
        assert_eq!(
            CdsDirection::BuyProtection.premium_multiplier::<f64>(),
            -1.0
        );
        assert_eq!(
            CdsDirection::BuyProtection.protection_multiplier::<f64>(),
            1.0
        );
        assert_eq!(
            CdsDirection::SellProtection.premium_multiplier::<f64>(),
            1.0
        );
        assert_eq!(
            CdsDirection::SellProtection.protection_multiplier::<f64>(),
            -1.0
        );
    }

    #[test]
    fn test_cds_direction_display() {
        assert_eq!(format!("{}", CdsDirection::BuyProtection), "Buy Protection");
        assert_eq!(format!("{}", CdsDirection::SellProtection), "Sell Protection");
    }

    #[test]
    fn test_cds_direction_equality() {
        assert_eq!(CdsDirection::BuyProtection, CdsDirection::BuyProtection);
        assert_ne!(CdsDirection::BuyProtection, CdsDirection::SellProtection);
    }

    // ========================================
    // CreditDefaultSwap Construction Tests
    // ========================================

    fn create_test_schedule() -> Schedule {
        let start = Date::from_ymd(2024, 3, 20).unwrap();
        let end = Date::from_ymd(2029, 3, 20).unwrap();

        ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap()
    }

    fn create_test_cds() -> CreditDefaultSwap<f64> {
        let schedule = create_test_schedule();

        CreditDefaultSwap::new(
            "ACME Corp".to_string(),
            10_000_000.0,
            0.01, // 100bp
            0.4,  // 40% recovery
            schedule,
            Currency::USD,
            CdsDirection::BuyProtection,
        )
    }

    #[test]
    fn test_cds_new() {
        let cds = create_test_cds();

        assert_eq!(cds.reference_entity(), "ACME Corp");
        assert!((cds.notional() - 10_000_000.0).abs() < 1e-6);
        assert!((cds.spread() - 0.01).abs() < 1e-10);
        assert!((cds.recovery_rate() - 0.4).abs() < 1e-10);
        assert_eq!(cds.currency(), Currency::USD);
        assert_eq!(cds.direction(), CdsDirection::BuyProtection);
    }

    #[test]
    fn test_cds_loss_given_default() {
        let cds = create_test_cds();
        // LGD = 1 - 0.4 = 0.6
        assert!((cds.loss_given_default() - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_cds_protection_payment() {
        let cds = create_test_cds();
        // Protection = LGD × Notional = 0.6 × 10M = 6M
        assert!((cds.protection_payment() - 6_000_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_cds_period_premium() {
        let cds = create_test_cds();
        // Premium for 0.25 year period = 10M × 0.01 × 0.25 = 25,000
        assert!((cds.period_premium(0.25) - 25_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_cds_maturity() {
        let cds = create_test_cds();
        // 5 years from schedule
        let maturity = cds.maturity();
        assert!(maturity > 4.0 && maturity < 6.0);
    }

    #[test]
    fn test_cds_num_periods() {
        let cds = create_test_cds();
        // 5 years quarterly = 20 periods
        assert_eq!(cds.num_periods(), 20);
    }

    #[test]
    fn test_cds_is_protection_buyer() {
        let cds = create_test_cds();
        assert!(cds.is_protection_buyer());
        assert!(!cds.is_protection_seller());
    }

    #[test]
    fn test_cds_protection_seller() {
        let schedule = create_test_schedule();
        let cds = CreditDefaultSwap::new(
            "ACME Corp".to_string(),
            10_000_000.0,
            0.01,
            0.4,
            schedule,
            Currency::USD,
            CdsDirection::SellProtection,
        );

        assert!(!cds.is_protection_buyer());
        assert!(cds.is_protection_seller());
    }

    // ========================================
    // InstrumentTrait Tests
    // ========================================

    #[test]
    fn test_cds_payoff() {
        let cds = create_test_cds();
        // CDS payoff is not spot-based
        assert_eq!(cds.payoff(100.0), 0.0);
    }

    #[test]
    fn test_cds_expiry() {
        let cds = create_test_cds();
        let expiry = <CreditDefaultSwap<f64> as InstrumentTrait<f64>>::expiry(&cds);
        assert!(expiry > 4.0 && expiry < 6.0);
    }

    #[test]
    fn test_cds_currency_trait() {
        let cds = create_test_cds();
        assert_eq!(
            <CreditDefaultSwap<f64> as InstrumentTrait<f64>>::currency(&cds),
            Currency::USD
        );
    }

    #[test]
    fn test_cds_notional_trait() {
        let cds = create_test_cds();
        let notional = <CreditDefaultSwap<f64> as InstrumentTrait<f64>>::notional(&cds);
        assert!((notional - 10_000_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_cds_is_not_path_dependent() {
        let cds = create_test_cds();
        assert!(!cds.is_path_dependent());
    }

    #[test]
    fn test_cds_type_name() {
        let cds = create_test_cds();
        assert_eq!(cds.type_name(), "CreditDefaultSwap");
    }

    // ========================================
    // CashflowInstrument Tests
    // ========================================

    #[test]
    fn test_cds_cashflows() {
        let cds = create_test_cds();
        let cashflows = cds.cashflows();

        // 20 quarterly periods
        assert_eq!(cashflows.len(), 20);

        // All cashflows should be negative for protection buyer (paying premium)
        for cf in &cashflows {
            assert!(
                cf.amount < 0.0,
                "Protection buyer should have negative premium cashflows"
            );
            assert_eq!(cf.currency, Currency::USD);
        }
    }

    #[test]
    fn test_cds_cashflows_seller() {
        let schedule = create_test_schedule();
        let cds = CreditDefaultSwap::new(
            "ACME Corp".to_string(),
            10_000_000.0,
            0.01,
            0.4,
            schedule,
            Currency::USD,
            CdsDirection::SellProtection,
        );

        let cashflows = cds.cashflows();

        // All cashflows should be positive for protection seller (receiving premium)
        for cf in &cashflows {
            assert!(
                cf.amount > 0.0,
                "Protection seller should have positive premium cashflows"
            );
        }
    }

    #[test]
    fn test_cds_num_cashflows() {
        let cds = create_test_cds();
        assert_eq!(cds.num_cashflows(), 20);
    }

    #[test]
    fn test_cds_total_cashflow() {
        let cds = create_test_cds();
        let total = cds.total_cashflow();

        // Total premium over 5 years ≈ 10M × 0.01 × 5 = 500,000 (negative for buyer)
        // May differ slightly due to day count conventions
        assert!(
            total < 0.0,
            "Protection buyer total cashflow should be negative"
        );
        assert!(total.abs() > 400_000.0 && total.abs() < 600_000.0);
    }

    // ========================================
    // Clone and Debug Tests
    // ========================================

    #[test]
    fn test_cds_clone() {
        let cds1 = create_test_cds();
        let cds2 = cds1.clone();

        assert_eq!(cds1.reference_entity(), cds2.reference_entity());
        assert!((cds1.notional() - cds2.notional()).abs() < 1e-10);
        assert!((cds1.spread() - cds2.spread()).abs() < 1e-10);
        assert_eq!(cds1.currency(), cds2.currency());
        assert_eq!(cds1.direction(), cds2.direction());
    }

    #[test]
    fn test_cds_debug() {
        let cds = create_test_cds();
        let debug_str = format!("{:?}", cds);
        assert!(debug_str.contains("CreditDefaultSwap"));
        assert!(debug_str.contains("ACME Corp"));
        assert!(debug_str.contains("notional"));
    }

    // ========================================
    // Edge Cases
    // ========================================

    #[test]
    fn test_cds_zero_spread() {
        let schedule = create_test_schedule();
        let cds = CreditDefaultSwap::new(
            "ACME Corp".to_string(),
            10_000_000.0,
            0.0, // zero spread
            0.4,
            schedule,
            Currency::USD,
            CdsDirection::BuyProtection,
        );

        assert_eq!(cds.period_premium(0.25), 0.0);
        let cashflows = cds.cashflows();
        for cf in &cashflows {
            assert_eq!(cf.amount, 0.0);
        }
    }

    #[test]
    fn test_cds_zero_recovery() {
        let schedule = create_test_schedule();
        let cds = CreditDefaultSwap::new(
            "ACME Corp".to_string(),
            10_000_000.0,
            0.01,
            0.0, // zero recovery = 100% LGD
            schedule,
            Currency::USD,
            CdsDirection::BuyProtection,
        );

        assert!((cds.loss_given_default() - 1.0).abs() < 1e-10);
        assert!((cds.protection_payment() - 10_000_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_cds_full_recovery() {
        let schedule = create_test_schedule();
        let cds = CreditDefaultSwap::new(
            "ACME Corp".to_string(),
            10_000_000.0,
            0.01,
            1.0, // 100% recovery = 0% LGD
            schedule,
            Currency::USD,
            CdsDirection::BuyProtection,
        );

        assert!(cds.loss_given_default().abs() < 1e-10);
        assert!(cds.protection_payment().abs() < 1e-6);
    }

    #[test]
    fn test_cds_different_currencies() {
        let schedule = create_test_schedule();

        for currency in [Currency::USD, Currency::EUR, Currency::JPY, Currency::GBP] {
            let cds = CreditDefaultSwap::new(
                "Test".to_string(),
                1_000_000.0,
                0.01,
                0.4,
                schedule.clone(),
                currency,
                CdsDirection::BuyProtection,
            );
            assert_eq!(cds.currency(), currency);
        }
    }

    // ========================================
    // Generic Type Tests
    // ========================================

    #[test]
    fn test_cds_with_f32() {
        let start = Date::from_ymd(2024, 3, 20).unwrap();
        let end = Date::from_ymd(2029, 3, 20).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        let cds: CreditDefaultSwap<f32> = CreditDefaultSwap::new(
            "ACME Corp".to_string(),
            10_000_000.0_f32,
            0.01_f32,
            0.4_f32,
            schedule,
            Currency::USD,
            CdsDirection::BuyProtection,
        );

        assert!((cds.notional() - 10_000_000.0_f32).abs() < 1e-3);
        assert!((cds.spread() - 0.01_f32).abs() < 1e-6);
    }
}
