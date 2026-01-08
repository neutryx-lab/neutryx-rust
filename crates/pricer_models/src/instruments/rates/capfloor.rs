//! Interest rate Cap and Floor implementation.
//!
//! This module provides:
//! - [`Cap`]: Series of caplets that pay when rates exceed a strike
//! - [`Floor`]: Series of floorlets that pay when rates fall below a strike
//! - [`Collar`]: Combination of a cap and floor
//!
//! # Structure
//!
//! A cap/floor consists of a series of options (caplets/floorlets) on
//! a floating rate index. Each caplet/floorlet is an option on a
//! single forward rate for one period.
//!
//! # Payoff
//!
//! For each period i:
//! - **Caplet**: max(ForwardRate_i - Strike, 0) × YearFraction × Notional
//! - **Floorlet**: max(Strike - ForwardRate_i, 0) × YearFraction × Notional
//!
//! # Example
//!
//! ```
//! use pricer_models::instruments::rates::{Cap, Floor, Collar, RateIndex};
//! use pricer_models::schedules::{ScheduleBuilder, Frequency};
//! use pricer_core::types::{Currency, time::{Date, DayCountConvention}};
//!
//! let start = Date::from_ymd(2024, 1, 15).unwrap();
//! let end = Date::from_ymd(2027, 1, 15).unwrap();
//!
//! let schedule = ScheduleBuilder::new()
//!     .start(start)
//!     .end(end)
//!     .frequency(Frequency::Quarterly)
//!     .day_count(DayCountConvention::ActualActual360)
//!     .build()
//!     .unwrap();
//!
//! let cap = Cap::new(
//!     1_000_000.0,
//!     schedule,
//!     0.04, // 4% strike
//!     RateIndex::Sofr,
//!     Currency::USD,
//! );
//!
//! assert_eq!(cap.strike(), 0.04);
//! ```

use num_traits::Float;
use pricer_core::types::time::DayCountConvention;
use pricer_core::types::Currency;

use super::RateIndex;
use crate::schedules::Schedule;

/// Interest rate cap.
///
/// A cap is a series of caplets, each of which pays
/// max(ForwardRate - Strike, 0) × YearFraction × Notional
/// for a given period.
///
/// Caps are used to hedge against rising interest rates.
#[derive(Debug, Clone)]
pub struct Cap<T: Float> {
    /// Notional principal amount.
    notional: T,
    /// Payment schedule (defines caplet periods).
    schedule: Schedule,
    /// Strike rate.
    strike: T,
    /// Reference rate index.
    index: RateIndex,
    /// Day count convention.
    day_count: DayCountConvention,
    /// Settlement currency.
    currency: Currency,
}

impl<T: Float> Cap<T> {
    /// Create a new interest rate cap.
    ///
    /// # Arguments
    ///
    /// * `notional` - Notional principal amount
    /// * `schedule` - Payment schedule (defines caplet periods)
    /// * `strike` - Cap strike rate
    /// * `index` - Reference rate index
    /// * `currency` - Settlement currency
    pub fn new(
        notional: T,
        schedule: Schedule,
        strike: T,
        index: RateIndex,
        currency: Currency,
    ) -> Self {
        let day_count = index.default_day_count();
        Self {
            notional,
            schedule,
            strike,
            index,
            day_count,
            currency,
        }
    }

    /// Create a new cap with custom day count convention.
    pub fn with_day_count(
        notional: T,
        schedule: Schedule,
        strike: T,
        index: RateIndex,
        day_count: DayCountConvention,
        currency: Currency,
    ) -> Self {
        Self {
            notional,
            schedule,
            strike,
            index,
            day_count,
            currency,
        }
    }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> T {
        self.notional
    }

    /// Returns the payment schedule.
    #[inline]
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Returns the strike rate.
    #[inline]
    pub fn strike(&self) -> T {
        self.strike
    }

    /// Returns the reference rate index.
    #[inline]
    pub fn index(&self) -> RateIndex {
        self.index
    }

    /// Returns the day count convention.
    #[inline]
    pub fn day_count(&self) -> DayCountConvention {
        self.day_count
    }

    /// Returns the settlement currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns the maturity time in years (from start to end of last period).
    pub fn maturity(&self) -> T {
        let periods = self.schedule.periods();
        match (periods.first(), periods.last()) {
            (Some(first), Some(last)) => {
                let year_frac = DayCountConvention::ActualActual365
                    .year_fraction_dates(first.start(), last.end());
                T::from(year_frac).unwrap_or_else(T::zero)
            }
            _ => T::zero(),
        }
    }

    /// Returns the number of caplets.
    #[inline]
    pub fn num_caplets(&self) -> usize {
        self.schedule.periods().len()
    }
}

/// Interest rate floor.
///
/// A floor is a series of floorlets, each of which pays
/// max(Strike - ForwardRate, 0) × YearFraction × Notional
/// for a given period.
///
/// Floors are used to hedge against falling interest rates.
#[derive(Debug, Clone)]
pub struct Floor<T: Float> {
    /// Notional principal amount.
    notional: T,
    /// Payment schedule (defines floorlet periods).
    schedule: Schedule,
    /// Strike rate.
    strike: T,
    /// Reference rate index.
    index: RateIndex,
    /// Day count convention.
    day_count: DayCountConvention,
    /// Settlement currency.
    currency: Currency,
}

impl<T: Float> Floor<T> {
    /// Create a new interest rate floor.
    ///
    /// # Arguments
    ///
    /// * `notional` - Notional principal amount
    /// * `schedule` - Payment schedule (defines floorlet periods)
    /// * `strike` - Floor strike rate
    /// * `index` - Reference rate index
    /// * `currency` - Settlement currency
    pub fn new(
        notional: T,
        schedule: Schedule,
        strike: T,
        index: RateIndex,
        currency: Currency,
    ) -> Self {
        let day_count = index.default_day_count();
        Self {
            notional,
            schedule,
            strike,
            index,
            day_count,
            currency,
        }
    }

    /// Create a new floor with custom day count convention.
    pub fn with_day_count(
        notional: T,
        schedule: Schedule,
        strike: T,
        index: RateIndex,
        day_count: DayCountConvention,
        currency: Currency,
    ) -> Self {
        Self {
            notional,
            schedule,
            strike,
            index,
            day_count,
            currency,
        }
    }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> T {
        self.notional
    }

    /// Returns the payment schedule.
    #[inline]
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Returns the strike rate.
    #[inline]
    pub fn strike(&self) -> T {
        self.strike
    }

    /// Returns the reference rate index.
    #[inline]
    pub fn index(&self) -> RateIndex {
        self.index
    }

    /// Returns the day count convention.
    #[inline]
    pub fn day_count(&self) -> DayCountConvention {
        self.day_count
    }

    /// Returns the settlement currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns the maturity time in years (from start to end of last period).
    pub fn maturity(&self) -> T {
        let periods = self.schedule.periods();
        match (periods.first(), periods.last()) {
            (Some(first), Some(last)) => {
                let year_frac = DayCountConvention::ActualActual365
                    .year_fraction_dates(first.start(), last.end());
                T::from(year_frac).unwrap_or_else(T::zero)
            }
            _ => T::zero(),
        }
    }

    /// Returns the number of floorlets.
    #[inline]
    pub fn num_floorlets(&self) -> usize {
        self.schedule.periods().len()
    }
}

/// Interest rate collar.
///
/// A collar is a combination of:
/// - Buying a cap at the cap strike
/// - Selling a floor at the floor strike
///
/// This limits both upside and downside interest rate exposure.
/// Often structured as a zero-cost collar where cap and floor
/// premiums offset each other.
#[derive(Debug, Clone)]
pub struct Collar<T: Float> {
    /// The cap component.
    cap: Cap<T>,
    /// The floor component.
    floor: Floor<T>,
}

impl<T: Float> Collar<T> {
    /// Create a new interest rate collar.
    ///
    /// # Arguments
    ///
    /// * `notional` - Notional principal amount
    /// * `schedule` - Payment schedule
    /// * `cap_strike` - Cap strike rate (upper bound)
    /// * `floor_strike` - Floor strike rate (lower bound)
    /// * `index` - Reference rate index
    /// * `currency` - Settlement currency
    ///
    /// # Panics
    ///
    /// Panics if floor_strike >= cap_strike.
    pub fn new(
        notional: T,
        schedule: Schedule,
        cap_strike: T,
        floor_strike: T,
        index: RateIndex,
        currency: Currency,
    ) -> Self {
        assert!(
            floor_strike < cap_strike,
            "Floor strike must be less than cap strike"
        );

        let cap = Cap::new(notional, schedule.clone(), cap_strike, index, currency);
        let floor = Floor::new(notional, schedule, floor_strike, index, currency);

        Self { cap, floor }
    }

    /// Returns the cap component.
    #[inline]
    pub fn cap(&self) -> &Cap<T> {
        &self.cap
    }

    /// Returns the floor component.
    #[inline]
    pub fn floor(&self) -> &Floor<T> {
        &self.floor
    }

    /// Returns the cap strike rate.
    #[inline]
    pub fn cap_strike(&self) -> T {
        self.cap.strike()
    }

    /// Returns the floor strike rate.
    #[inline]
    pub fn floor_strike(&self) -> T {
        self.floor.strike()
    }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> T {
        self.cap.notional()
    }

    /// Returns the reference rate index.
    #[inline]
    pub fn index(&self) -> RateIndex {
        self.cap.index()
    }

    /// Returns the settlement currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        self.cap.currency()
    }

    /// Returns the maturity.
    #[inline]
    pub fn maturity(&self) -> T {
        self.cap.maturity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedules::{Frequency, ScheduleBuilder};
    use pricer_core::types::time::Date;

    fn create_test_schedule() -> Schedule {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2027, 1, 15).unwrap();

        ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap()
    }

    // ========================================
    // Cap Tests
    // ========================================

    #[test]
    fn test_cap_new() {
        let schedule = create_test_schedule();
        let cap = Cap::new(1_000_000.0, schedule, 0.04, RateIndex::Sofr, Currency::USD);

        assert!((cap.notional() - 1_000_000.0).abs() < 1e-6);
        assert!((cap.strike() - 0.04).abs() < 1e-10);
        assert_eq!(cap.index(), RateIndex::Sofr);
        assert_eq!(cap.currency(), Currency::USD);
    }

    #[test]
    fn test_cap_with_day_count() {
        let schedule = create_test_schedule();
        let cap = Cap::with_day_count(
            1_000_000.0,
            schedule,
            0.04,
            RateIndex::Sofr,
            DayCountConvention::ActualActual365,
            Currency::USD,
        );

        assert_eq!(cap.day_count(), DayCountConvention::ActualActual365);
    }

    #[test]
    fn test_cap_num_caplets() {
        let schedule = create_test_schedule();
        let cap = Cap::new(1_000_000.0, schedule, 0.04, RateIndex::Sofr, Currency::USD);

        // 3 years, quarterly = 12 periods
        assert_eq!(cap.num_caplets(), 12);
    }

    #[test]
    fn test_cap_maturity() {
        let schedule = create_test_schedule();
        let cap = Cap::new(1_000_000.0, schedule, 0.04, RateIndex::Sofr, Currency::USD);

        let maturity = cap.maturity();
        assert!(maturity > 2.0 && maturity < 4.0);
    }

    #[test]
    fn test_cap_clone() {
        let schedule = create_test_schedule();
        let cap1 = Cap::new(1_000_000.0, schedule, 0.04, RateIndex::Sofr, Currency::USD);
        let cap2 = cap1.clone();

        assert!((cap1.strike() - cap2.strike()).abs() < 1e-10);
        assert_eq!(cap1.index(), cap2.index());
    }

    // ========================================
    // Floor Tests
    // ========================================

    #[test]
    fn test_floor_new() {
        let schedule = create_test_schedule();
        let floor = Floor::new(1_000_000.0, schedule, 0.02, RateIndex::Sofr, Currency::USD);

        assert!((floor.notional() - 1_000_000.0).abs() < 1e-6);
        assert!((floor.strike() - 0.02).abs() < 1e-10);
        assert_eq!(floor.index(), RateIndex::Sofr);
        assert_eq!(floor.currency(), Currency::USD);
    }

    #[test]
    fn test_floor_with_day_count() {
        let schedule = create_test_schedule();
        let floor = Floor::with_day_count(
            1_000_000.0,
            schedule,
            0.02,
            RateIndex::Euribor3M,
            DayCountConvention::Thirty360,
            Currency::EUR,
        );

        assert_eq!(floor.day_count(), DayCountConvention::Thirty360);
        assert_eq!(floor.index(), RateIndex::Euribor3M);
    }

    #[test]
    fn test_floor_num_floorlets() {
        let schedule = create_test_schedule();
        let floor = Floor::new(1_000_000.0, schedule, 0.02, RateIndex::Sofr, Currency::USD);

        assert_eq!(floor.num_floorlets(), 12);
    }

    #[test]
    fn test_floor_maturity() {
        let schedule = create_test_schedule();
        let floor = Floor::new(1_000_000.0, schedule, 0.02, RateIndex::Sofr, Currency::USD);

        let maturity = floor.maturity();
        assert!(maturity > 2.0 && maturity < 4.0);
    }

    // ========================================
    // Collar Tests
    // ========================================

    #[test]
    fn test_collar_new() {
        let schedule = create_test_schedule();
        let collar = Collar::new(
            1_000_000.0,
            schedule,
            0.05, // cap strike
            0.02, // floor strike
            RateIndex::Sofr,
            Currency::USD,
        );

        assert!((collar.cap_strike() - 0.05).abs() < 1e-10);
        assert!((collar.floor_strike() - 0.02).abs() < 1e-10);
        assert!((collar.notional() - 1_000_000.0).abs() < 1e-6);
    }

    #[test]
    #[should_panic(expected = "Floor strike must be less than cap strike")]
    fn test_collar_invalid_strikes() {
        let schedule = create_test_schedule();
        Collar::new(
            1_000_000.0,
            schedule,
            0.02, // cap strike (invalid: lower than floor)
            0.05, // floor strike
            RateIndex::Sofr,
            Currency::USD,
        );
    }

    #[test]
    fn test_collar_accessors() {
        let schedule = create_test_schedule();
        let collar = Collar::new(
            1_000_000.0,
            schedule,
            0.05,
            0.02,
            RateIndex::Sofr,
            Currency::USD,
        );

        assert_eq!(collar.index(), RateIndex::Sofr);
        assert_eq!(collar.currency(), Currency::USD);
        assert!(collar.cap().strike() > collar.floor().strike());
    }

    #[test]
    fn test_collar_maturity() {
        let schedule = create_test_schedule();
        let collar = Collar::new(
            1_000_000.0,
            schedule,
            0.05,
            0.02,
            RateIndex::Sofr,
            Currency::USD,
        );

        let maturity = collar.maturity();
        assert!(maturity > 2.0 && maturity < 4.0);
    }

    #[test]
    fn test_collar_clone() {
        let schedule = create_test_schedule();
        let collar1 = Collar::new(
            1_000_000.0,
            schedule,
            0.05,
            0.02,
            RateIndex::Sofr,
            Currency::USD,
        );
        let collar2 = collar1.clone();

        assert!((collar1.cap_strike() - collar2.cap_strike()).abs() < 1e-10);
        assert!((collar1.floor_strike() - collar2.floor_strike()).abs() < 1e-10);
    }
}
