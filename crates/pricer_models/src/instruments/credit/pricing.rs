//! CDS pricing module.
//!
//! This module provides pricing functionality for Credit Default Swaps using
//! survival probabilities from hazard rate curves.
//!
//! # Pricing Model
//!
//! The CDS value is computed as:
//! ```text
//! V = PV(ProtectionLeg) - PV(PremiumLeg)
//! ```
//!
//! ## Premium Leg
//!
//! The present value of premium payments:
//! ```text
//! PV_premium = Notional × Spread × Σ(DF(tᵢ) × S(tᵢ) × ΔT_i)
//! ```
//!
//! ## Protection Leg
//!
//! The present value of the protection payment:
//! ```text
//! PV_protection = Notional × (1 - R) × Σ(DF(tᵢ) × (S(tᵢ₋₁) - S(tᵢ)))
//! ```
//!
//! # Example
//!
//! ```
//! use pricer_models::instruments::credit::{CreditDefaultSwap, CdsDirection, CdsPricer};
//! use pricer_models::schedules::{ScheduleBuilder, Frequency};
//! use pricer_core::types::{Currency, time::{Date, DayCountConvention}};
//! use pricer_core::market_data::curves::{FlatCurve, FlatHazardRateCurve};
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
//! // Create flat curves for pricing
//! let discount_curve = FlatCurve::new(0.03);  // 3% flat rate
//! let credit_curve = FlatHazardRateCurve::new(0.01);  // 100bp hazard rate
//!
//! let pricer = CdsPricer::new(&discount_curve, &credit_curve);
//! let pv = pricer.price(&cds).unwrap();
//! ```

use num_traits::Float;
use pricer_core::market_data::curves::{CreditCurve, YieldCurve};
use pricer_core::market_data::error::MarketDataError;
use pricer_core::types::time::DayCountConvention;

use super::cds::{CdsDirection, CreditDefaultSwap};

/// CDS pricing result containing leg values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CdsPriceResult<T: Float> {
    /// Present value of the protection leg.
    pub protection_leg_pv: T,
    /// Present value of the premium leg.
    pub premium_leg_pv: T,
    /// Net present value (protection - premium for buyer).
    pub npv: T,
    /// Par spread (spread that makes NPV = 0).
    pub par_spread: T,
    /// Risky PV01 (sensitivity to 1bp spread change).
    pub risky_pv01: T,
}

impl<T: Float> CdsPriceResult<T> {
    /// Create a new CDS price result.
    pub fn new(
        protection_leg_pv: T,
        premium_leg_pv: T,
        direction: CdsDirection,
        spread: T,
    ) -> Self {
        let npv = match direction {
            CdsDirection::BuyProtection => protection_leg_pv - premium_leg_pv,
            CdsDirection::SellProtection => premium_leg_pv - protection_leg_pv,
        };

        // Par spread = (Protection PV) / (Risky Annuity)
        // Risky Annuity = Premium PV / Spread
        let risky_pv01 = if spread != T::zero() {
            premium_leg_pv / spread / T::from(10000.0).unwrap()
        } else {
            T::zero()
        };

        let par_spread = if premium_leg_pv != T::zero() {
            protection_leg_pv * spread / premium_leg_pv
        } else {
            T::zero()
        };

        Self {
            protection_leg_pv,
            premium_leg_pv,
            npv,
            par_spread,
            risky_pv01,
        }
    }
}

/// CDS pricer using discount and credit curves.
///
/// Prices CDS using the ISDA standard model approach with survival
/// probabilities derived from hazard rate curves.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float`
/// * `D` - Discount curve implementing `YieldCurve<T>`
/// * `C` - Credit curve implementing `CreditCurve<T>`
pub struct CdsPricer<'a, T: Float, D: YieldCurve<T>, C: CreditCurve<T>> {
    /// Discount curve for present value calculations.
    discount_curve: &'a D,
    /// Credit curve for survival probability calculations.
    credit_curve: &'a C,
    /// Phantom data for type parameter T.
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: Float, D: YieldCurve<T>, C: CreditCurve<T>> CdsPricer<'a, T, D, C> {
    /// Create a new CDS pricer.
    ///
    /// # Arguments
    ///
    /// * `discount_curve` - Discount curve for discounting
    /// * `credit_curve` - Credit curve for survival probabilities
    pub fn new(discount_curve: &'a D, credit_curve: &'a C) -> Self {
        Self {
            discount_curve,
            credit_curve,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Price a CDS and return detailed pricing result.
    ///
    /// # Arguments
    ///
    /// * `cds` - Credit Default Swap to price
    ///
    /// # Returns
    ///
    /// * `Ok(CdsPriceResult)` - Pricing result with leg values
    /// * `Err(MarketDataError)` - If pricing fails
    pub fn price(&self, cds: &CreditDefaultSwap<T>) -> Result<CdsPriceResult<T>, MarketDataError> {
        let protection_pv = self.protection_leg_pv(cds)?;
        let premium_pv = self.premium_leg_pv(cds)?;

        Ok(CdsPriceResult::new(
            protection_pv,
            premium_pv,
            cds.direction(),
            cds.spread(),
        ))
    }

    /// Calculate the present value of the protection leg.
    ///
    /// The protection leg pays (1 - Recovery) × Notional upon default.
    /// The PV is computed by integrating over the default time distribution:
    ///
    /// ```text
    /// PV = LGD × Notional × Σ(DF(tᵢ) × (S(tᵢ₋₁) - S(tᵢ)))
    /// ```
    ///
    /// where LGD = 1 - Recovery, DF is discount factor, S is survival probability.
    pub fn protection_leg_pv(&self, cds: &CreditDefaultSwap<T>) -> Result<T, MarketDataError> {
        let lgd = cds.loss_given_default();
        let notional = cds.notional();
        let schedule = cds.schedule();

        let mut pv = T::zero();
        let start_date = schedule.start_date();
        let mut prev_survival = T::one();

        for period in schedule.periods() {
            // Time to payment date in years
            let t = T::from(
                DayCountConvention::ActualActual365
                    .year_fraction_dates(start_date, period.payment())
            ).unwrap_or_else(T::zero);

            // Discount factor to mid-point of period (approximation)
            let df = self.discount_curve.discount_factor(t)?;

            // Survival probability at end of period
            let survival = self.credit_curve.survival_probability(t)?;

            // Default probability during period
            let default_prob = prev_survival - survival;

            // PV contribution: DF × Default_prob × LGD × Notional
            pv = pv + df * default_prob * lgd * notional;

            prev_survival = survival;
        }

        Ok(pv)
    }

    /// Calculate the present value of the premium leg.
    ///
    /// The premium leg pays Spread × Notional × DayCount at each payment date
    /// conditional on survival:
    ///
    /// ```text
    /// PV = Spread × Notional × Σ(DF(tᵢ) × S(tᵢ) × ΔTᵢ)
    /// ```
    ///
    /// Note: This simplified version does not include accrued premium on default.
    pub fn premium_leg_pv(&self, cds: &CreditDefaultSwap<T>) -> Result<T, MarketDataError> {
        let spread = cds.spread();
        let notional = cds.notional();
        let schedule = cds.schedule();

        let mut pv = T::zero();
        let start_date = schedule.start_date();

        for period in schedule.periods() {
            // Time to payment date in years
            let t = T::from(
                DayCountConvention::ActualActual365
                    .year_fraction_dates(start_date, period.payment())
            ).unwrap_or_else(T::zero);

            // Year fraction for accrual
            let year_frac = T::from(period.year_fraction()).unwrap_or_else(T::zero);

            // Discount factor
            let df = self.discount_curve.discount_factor(t)?;

            // Survival probability
            let survival = self.credit_curve.survival_probability(t)?;

            // PV contribution: DF × Survival × Spread × Notional × YearFrac
            pv = pv + df * survival * spread * notional * year_frac;
        }

        Ok(pv)
    }

    /// Calculate the risky annuity (risky PV01).
    ///
    /// The risky annuity is the present value of receiving 1bp per year
    /// conditional on survival:
    ///
    /// ```text
    /// RiskyAnnuity = Notional × Σ(DF(tᵢ) × S(tᵢ) × ΔTᵢ) / 10000
    /// ```
    pub fn risky_annuity(&self, cds: &CreditDefaultSwap<T>) -> Result<T, MarketDataError> {
        let notional = cds.notional();
        let schedule = cds.schedule();

        let mut pv = T::zero();
        let start_date = schedule.start_date();
        let bp = T::from(0.0001).unwrap(); // 1 basis point

        for period in schedule.periods() {
            let t = T::from(
                DayCountConvention::ActualActual365
                    .year_fraction_dates(start_date, period.payment())
            ).unwrap_or_else(T::zero);

            let year_frac = T::from(period.year_fraction()).unwrap_or_else(T::zero);
            let df = self.discount_curve.discount_factor(t)?;
            let survival = self.credit_curve.survival_probability(t)?;

            pv = pv + df * survival * bp * notional * year_frac;
        }

        Ok(pv)
    }

    /// Calculate the par spread (fair spread).
    ///
    /// The par spread is the spread that makes NPV = 0:
    ///
    /// ```text
    /// ParSpread = ProtectionLegPV / RiskyAnnuity
    /// ```
    pub fn par_spread(&self, cds: &CreditDefaultSwap<T>) -> Result<T, MarketDataError> {
        let protection_pv = self.protection_leg_pv(cds)?;

        // Compute risky annuity per 1 unit spread
        let notional = cds.notional();
        let schedule = cds.schedule();
        let start_date = schedule.start_date();

        let mut annuity = T::zero();
        for period in schedule.periods() {
            let t = T::from(
                DayCountConvention::ActualActual365
                    .year_fraction_dates(start_date, period.payment())
            ).unwrap_or_else(T::zero);

            let year_frac = T::from(period.year_fraction()).unwrap_or_else(T::zero);
            let df = self.discount_curve.discount_factor(t)?;
            let survival = self.credit_curve.survival_probability(t)?;

            annuity = annuity + df * survival * notional * year_frac;
        }

        if annuity == T::zero() {
            return Ok(T::zero());
        }

        Ok(protection_pv / annuity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::credit::CreditDefaultSwap;
    use crate::schedules::{Frequency, ScheduleBuilder};
    use pricer_core::market_data::curves::{FlatCurve, FlatHazardRateCurve};
    use pricer_core::types::time::Date;
    use pricer_core::types::Currency;

    fn create_test_cds() -> CreditDefaultSwap<f64> {
        let start = Date::from_ymd(2024, 3, 20).unwrap();
        let end = Date::from_ymd(2029, 3, 20).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        CreditDefaultSwap::new(
            "ACME Corp".to_string(),
            10_000_000.0,
            0.01, // 100bp spread
            0.4,  // 40% recovery
            schedule,
            Currency::USD,
            CdsDirection::BuyProtection,
        )
    }

    fn create_at_the_money_cds(par_spread: f64) -> CreditDefaultSwap<f64> {
        let start = Date::from_ymd(2024, 3, 20).unwrap();
        let end = Date::from_ymd(2029, 3, 20).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        CreditDefaultSwap::new(
            "ACME Corp".to_string(),
            10_000_000.0,
            par_spread,
            0.4,
            schedule,
            Currency::USD,
            CdsDirection::BuyProtection,
        )
    }

    // ========================================
    // CdsPriceResult Tests
    // ========================================

    #[test]
    fn test_price_result_buyer() {
        let result = CdsPriceResult::new(
            100_000.0,
            80_000.0,
            CdsDirection::BuyProtection,
            0.01,
        );

        // NPV = Protection - Premium for buyer
        assert!((result.npv - 20_000.0).abs() < 1e-6);
        assert!((result.protection_leg_pv - 100_000.0).abs() < 1e-6);
        assert!((result.premium_leg_pv - 80_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_price_result_seller() {
        let result = CdsPriceResult::new(
            100_000.0,
            80_000.0,
            CdsDirection::SellProtection,
            0.01,
        );

        // NPV = Premium - Protection for seller
        assert!((result.npv - (-20_000.0)).abs() < 1e-6);
    }

    #[test]
    fn test_price_result_par_spread() {
        let result = CdsPriceResult::new(
            100_000.0,
            100_000.0,
            CdsDirection::BuyProtection,
            0.01,
        );

        // At par, par_spread should equal the contract spread
        assert!((result.par_spread - 0.01).abs() < 1e-10);
    }

    // ========================================
    // CdsPricer Tests
    // ========================================

    #[test]
    fn test_pricer_new() {
        let discount_curve = FlatCurve::new(0.03);
        let credit_curve = FlatHazardRateCurve::new(0.01);

        let _pricer = CdsPricer::new(&discount_curve, &credit_curve);
    }

    #[test]
    fn test_protection_leg_pv_positive() {
        let cds = create_test_cds();
        let discount_curve = FlatCurve::new(0.03);
        let credit_curve = FlatHazardRateCurve::new(0.01);

        let pricer = CdsPricer::new(&discount_curve, &credit_curve);
        let protection_pv = pricer.protection_leg_pv(&cds).unwrap();

        // Protection PV should be positive (receiving payment on default)
        assert!(protection_pv > 0.0, "Protection PV should be positive");
    }

    #[test]
    fn test_premium_leg_pv_positive() {
        let cds = create_test_cds();
        let discount_curve = FlatCurve::new(0.03);
        let credit_curve = FlatHazardRateCurve::new(0.01);

        let pricer = CdsPricer::new(&discount_curve, &credit_curve);
        let premium_pv = pricer.premium_leg_pv(&cds).unwrap();

        // Premium PV should be positive (absolute value of payments)
        assert!(premium_pv > 0.0, "Premium PV should be positive");
    }

    #[test]
    fn test_protection_leg_increases_with_hazard_rate() {
        let cds = create_test_cds();
        let discount_curve = FlatCurve::new(0.03);

        let credit_curve_low = FlatHazardRateCurve::new(0.005);
        let credit_curve_high = FlatHazardRateCurve::new(0.02);

        let pricer_low = CdsPricer::new(&discount_curve, &credit_curve_low);
        let pricer_high = CdsPricer::new(&discount_curve, &credit_curve_high);

        let pv_low = pricer_low.protection_leg_pv(&cds).unwrap();
        let pv_high = pricer_high.protection_leg_pv(&cds).unwrap();

        // Higher hazard rate = higher default prob = higher protection PV
        assert!(
            pv_high > pv_low,
            "Protection PV should increase with hazard rate"
        );
    }

    #[test]
    fn test_premium_leg_decreases_with_hazard_rate() {
        let cds = create_test_cds();
        let discount_curve = FlatCurve::new(0.03);

        let credit_curve_low = FlatHazardRateCurve::new(0.005);
        let credit_curve_high = FlatHazardRateCurve::new(0.02);

        let pricer_low = CdsPricer::new(&discount_curve, &credit_curve_low);
        let pricer_high = CdsPricer::new(&discount_curve, &credit_curve_high);

        let pv_low = pricer_low.premium_leg_pv(&cds).unwrap();
        let pv_high = pricer_high.premium_leg_pv(&cds).unwrap();

        // Higher hazard rate = lower survival = lower premium PV
        assert!(
            pv_high < pv_low,
            "Premium PV should decrease with hazard rate"
        );
    }

    #[test]
    fn test_par_spread_calculation() {
        let cds = create_test_cds();
        let discount_curve = FlatCurve::new(0.03);
        let credit_curve = FlatHazardRateCurve::new(0.01);

        let pricer = CdsPricer::new(&discount_curve, &credit_curve);
        let par_spread = pricer.par_spread(&cds).unwrap();

        // Par spread should be positive and reasonable
        assert!(par_spread > 0.0, "Par spread should be positive");
        assert!(par_spread < 0.1, "Par spread should be less than 10%");
    }

    #[test]
    fn test_at_par_npv_near_zero() {
        let discount_curve = FlatCurve::new(0.03);
        let credit_curve = FlatHazardRateCurve::new(0.01);

        // First get the par spread
        let test_cds = create_test_cds();
        let pricer = CdsPricer::new(&discount_curve, &credit_curve);
        let par_spread = pricer.par_spread(&test_cds).unwrap();

        // Create CDS at par spread
        let cds_at_par = create_at_the_money_cds(par_spread);
        let result = pricer.price(&cds_at_par).unwrap();

        // NPV should be close to zero at par
        assert!(
            result.npv.abs() < 1.0, // Less than $1 on 10M notional
            "NPV should be near zero at par spread, got {}",
            result.npv
        );
    }

    #[test]
    fn test_risky_annuity() {
        let cds = create_test_cds();
        let discount_curve = FlatCurve::new(0.03);
        let credit_curve = FlatHazardRateCurve::new(0.01);

        let pricer = CdsPricer::new(&discount_curve, &credit_curve);
        let risky_pv01 = pricer.risky_annuity(&cds).unwrap();

        // Risky PV01 should be positive
        assert!(risky_pv01 > 0.0, "Risky PV01 should be positive");

        // For 10M notional, 5Y CDS, risky PV01 should be around 4000-5000
        // (approximately 4-5 years × 10M × 1bp)
        assert!(
            risky_pv01 > 3000.0 && risky_pv01 < 6000.0,
            "Risky PV01 should be reasonable, got {}",
            risky_pv01
        );
    }

    #[test]
    fn test_price_complete() {
        let cds = create_test_cds();
        let discount_curve = FlatCurve::new(0.03);
        let credit_curve = FlatHazardRateCurve::new(0.01);

        let pricer = CdsPricer::new(&discount_curve, &credit_curve);
        let result = pricer.price(&cds).unwrap();

        // All values should be computed
        assert!(result.protection_leg_pv > 0.0);
        assert!(result.premium_leg_pv > 0.0);
        assert!(result.par_spread > 0.0);
        assert!(result.risky_pv01 > 0.0);
    }

    #[test]
    fn test_higher_recovery_lower_protection_pv() {
        let discount_curve = FlatCurve::new(0.03);
        let credit_curve = FlatHazardRateCurve::new(0.01);
        let pricer = CdsPricer::new(&discount_curve, &credit_curve);

        let start = Date::from_ymd(2024, 3, 20).unwrap();
        let end = Date::from_ymd(2029, 3, 20).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        let cds_low_recovery = CreditDefaultSwap::new(
            "Test".to_string(),
            10_000_000.0,
            0.01,
            0.2, // 20% recovery
            schedule.clone(),
            Currency::USD,
            CdsDirection::BuyProtection,
        );

        let cds_high_recovery = CreditDefaultSwap::new(
            "Test".to_string(),
            10_000_000.0,
            0.01,
            0.6, // 60% recovery
            schedule,
            Currency::USD,
            CdsDirection::BuyProtection,
        );

        let pv_low = pricer.protection_leg_pv(&cds_low_recovery).unwrap();
        let pv_high = pricer.protection_leg_pv(&cds_high_recovery).unwrap();

        // Higher recovery = lower LGD = lower protection PV
        assert!(
            pv_low > pv_high,
            "Higher recovery should give lower protection PV"
        );
    }

    #[test]
    fn test_seller_npv_opposite_sign() {
        let discount_curve = FlatCurve::new(0.03);
        let credit_curve = FlatHazardRateCurve::new(0.01);

        let start = Date::from_ymd(2024, 3, 20).unwrap();
        let end = Date::from_ymd(2029, 3, 20).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        let cds_buyer = CreditDefaultSwap::new(
            "Test".to_string(),
            10_000_000.0,
            0.01,
            0.4,
            schedule.clone(),
            Currency::USD,
            CdsDirection::BuyProtection,
        );

        let cds_seller = CreditDefaultSwap::new(
            "Test".to_string(),
            10_000_000.0,
            0.01,
            0.4,
            schedule,
            Currency::USD,
            CdsDirection::SellProtection,
        );

        let pricer = CdsPricer::new(&discount_curve, &credit_curve);

        let result_buyer = pricer.price(&cds_buyer).unwrap();
        let result_seller = pricer.price(&cds_seller).unwrap();

        // Buyer and seller NPVs should be opposite
        assert!(
            (result_buyer.npv + result_seller.npv).abs() < 1e-6,
            "Buyer and seller NPVs should be opposite"
        );
    }

    // ========================================
    // Edge Cases
    // ========================================

    #[test]
    fn test_zero_hazard_rate() {
        let cds = create_test_cds();
        let discount_curve = FlatCurve::new(0.03);
        let credit_curve = FlatHazardRateCurve::new(0.0); // Zero hazard rate

        let pricer = CdsPricer::new(&discount_curve, &credit_curve);
        let protection_pv = pricer.protection_leg_pv(&cds).unwrap();

        // With zero hazard rate, no default occurs, protection PV = 0
        assert!(
            protection_pv.abs() < 1e-6,
            "Protection PV should be zero with no default risk"
        );
    }

    #[test]
    fn test_zero_discount_rate() {
        let cds = create_test_cds();
        let discount_curve = FlatCurve::new(0.0); // Zero discount rate
        let credit_curve = FlatHazardRateCurve::new(0.01);

        let pricer = CdsPricer::new(&discount_curve, &credit_curve);
        let result = pricer.price(&cds).unwrap();

        // Should still compute valid results
        assert!(result.protection_leg_pv > 0.0);
        assert!(result.premium_leg_pv > 0.0);
    }

    #[test]
    fn test_with_f32() {
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
            "Test".to_string(),
            10_000_000.0_f32,
            0.01_f32,
            0.4_f32,
            schedule,
            Currency::USD,
            CdsDirection::BuyProtection,
        );

        let discount_curve = FlatCurve::new(0.03_f32);
        let credit_curve = FlatHazardRateCurve::new(0.01_f32);

        let pricer = CdsPricer::new(&discount_curve, &credit_curve);
        let result = pricer.price(&cds).unwrap();

        assert!(result.protection_leg_pv > 0.0);
        assert!(result.premium_leg_pv > 0.0);
    }
}
