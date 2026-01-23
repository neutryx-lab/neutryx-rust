//! Date calculation utilities for yield curve bootstrapping.
//!
//! This module provides helper functions and types for date calculations
//! in the context of yield curve construction, integrating with the
//! `infra_master` calendar functionality.
//!
//! ## Features
//!
//! - Business day adjustments using `infra_master::BusinessDayConvention`
//! - Spot date calculation (T+n settlement)
//! - Year fraction calculations using `infra_master::DayCounter`
//! - Integration with `infra_master` calendars
//!
//! ## Example
//!
//! ```rust,ignore
//! use pricer_models::market::calibration::bootstrapping::{DateCalculator, SpotDateConvention};
//! use chrono::NaiveDate;
//!
//! let calc = DateCalculator::new();
//! let trade_date = NaiveDate::from_ymd_opt(2026, 1, 14).unwrap();
//! let spot_date = calc.spot_date(trade_date, SpotDateConvention::T2);
//! ```

use chrono::{Datelike, NaiveDate, Weekday};
use infra_master::{BusinessDayConvention, Date, DayCounter};
use num_traits::Float;

/// Spot date convention for settlement.
///
/// Defines the number of business days from trade date to settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SpotDateConvention {
    /// Same day settlement (T+0)
    T0,
    /// Next business day settlement (T+1)
    T1,
    /// Standard spot settlement (T+2)
    #[default]
    T2,
    /// T+3 settlement
    T3,
}

impl SpotDateConvention {
    /// Get the number of business days to settlement.
    pub fn business_days(&self) -> i32 {
        match self {
            SpotDateConvention::T0 => 0,
            SpotDateConvention::T1 => 1,
            SpotDateConvention::T2 => 2,
            SpotDateConvention::T3 => 3,
        }
    }

    /// Get the convention name.
    pub fn name(&self) -> &'static str {
        match self {
            SpotDateConvention::T0 => "T+0",
            SpotDateConvention::T1 => "T+1",
            SpotDateConvention::T2 => "T+2",
            SpotDateConvention::T3 => "T+3",
        }
    }
}

impl std::fmt::Display for SpotDateConvention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Date calculator for bootstrapping operations.
///
/// Provides methods for business day adjustments, spot date calculation,
/// and year fraction computations.
#[derive(Debug, Clone)]
pub struct DateCalculator {
    /// Default spot date convention
    spot_convention: SpotDateConvention,
    /// Default business day adjustment
    business_day_convention: BusinessDayConvention,
    /// Default day count convention
    day_counter: DayCounter,
}

impl Default for DateCalculator {
    fn default() -> Self {
        Self {
            spot_convention: SpotDateConvention::T2,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            day_counter: DayCounter::Actual365Fixed,
        }
    }
}

impl DateCalculator {
    /// Create a new date calculator with default settings.
    pub fn new() -> Self { Self::default() }

    /// Create a builder for customised configuration.
    pub fn builder() -> DateCalculatorBuilder { DateCalculatorBuilder::default() }

    /// Check if a date is a weekend.
    pub fn is_weekend(&self, date: NaiveDate) -> bool {
        date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun
    }

    /// Check if a date is a business day (weekends only check).
    ///
    /// For more comprehensive holiday checking, use `infra_master::Calendar`.
    pub fn is_business_day(&self, date: NaiveDate) -> bool { !self.is_weekend(date) }

    /// Get the next business day on or after the given date.
    pub fn following(&self, mut date: NaiveDate) -> NaiveDate {
        while !self.is_business_day(date) {
            date = date.succ_opt().unwrap_or(date);
        }
        date
    }

    /// Get the previous business day on or before the given date.
    pub fn preceding(&self, mut date: NaiveDate) -> NaiveDate {
        while !self.is_business_day(date) {
            date = date.pred_opt().unwrap_or(date);
        }
        date
    }

    /// Apply modified following adjustment.
    ///
    /// Moves to next business day, but if that crosses a month boundary,
    /// moves to the previous business day instead.
    pub fn modified_following(&self, date: NaiveDate) -> NaiveDate {
        let following = self.following(date);
        if following.month() == date.month() {
            following
        } else {
            self.preceding(date)
        }
    }

    /// Apply modified preceding adjustment.
    ///
    /// Moves to previous business day, but if that crosses a month boundary,
    /// moves to the next business day instead.
    pub fn modified_preceding(&self, date: NaiveDate) -> NaiveDate {
        let preceding = self.preceding(date);
        if preceding.month() == date.month() {
            preceding
        } else {
            self.following(date)
        }
    }

    /// Adjust a date according to the specified business day convention.
    pub fn adjust(&self, date: NaiveDate, convention: BusinessDayConvention) -> NaiveDate {
        match convention {
            BusinessDayConvention::Following => self.following(date),
            BusinessDayConvention::ModifiedFollowing => self.modified_following(date),
            BusinessDayConvention::Preceding => self.preceding(date),
            BusinessDayConvention::ModifiedPreceding => self.modified_preceding(date),
            BusinessDayConvention::Unadjusted => date,
        }
    }

    /// Adjust a date using the default business day convention.
    pub fn adjust_default(&self, date: NaiveDate) -> NaiveDate {
        self.adjust(date, self.business_day_convention)
    }

    /// Add business days to a date.
    pub fn add_business_days(&self, mut date: NaiveDate, days: i32) -> NaiveDate {
        if days == 0 {
            return self.following(date);
        }

        let step = if days > 0 { 1 } else { -1 };
        let mut remaining = days.abs();

        while remaining > 0 {
            date = if step > 0 {
                date.succ_opt().unwrap_or(date)
            } else {
                date.pred_opt().unwrap_or(date)
            };
            if self.is_business_day(date) {
                remaining -= 1;
            }
        }

        date
    }

    /// Calculate the spot date from a trade date.
    pub fn spot_date(&self, trade_date: NaiveDate, convention: SpotDateConvention) -> NaiveDate {
        self.add_business_days(trade_date, convention.business_days())
    }

    /// Calculate the spot date using the default convention.
    pub fn spot_date_default(&self, trade_date: NaiveDate) -> NaiveDate {
        self.spot_date(trade_date, self.spot_convention)
    }

    /// Calculate year fraction between two dates.
    pub fn year_fraction(&self, start: NaiveDate, end: NaiveDate, day_counter: DayCounter) -> f64 {
        let start_date = Date::from_naive(start);
        let end_date = Date::from_naive(end);
        day_counter.year_fraction(start_date, end_date)
    }

    /// Calculate year fraction using the default day count convention.
    pub fn year_fraction_default(&self, start: NaiveDate, end: NaiveDate) -> f64 {
        self.year_fraction(start, end, self.day_counter)
    }

    /// Calculate year fraction as generic float type.
    pub fn year_fraction_generic<T: Float>(
        &self,
        start: NaiveDate,
        end: NaiveDate,
        day_counter: DayCounter,
    ) -> T {
        T::from(self.year_fraction(start, end, day_counter)).unwrap_or_else(T::zero)
    }

    /// Calculate maturity in years from a start date.
    ///
    /// Converts a NaiveDate maturity to a year fraction.
    pub fn maturity_years(&self, start: NaiveDate, maturity: NaiveDate) -> f64 {
        self.year_fraction(start, maturity, self.day_counter)
    }

    /// Calculate maturity in years as generic float type.
    pub fn maturity_years_generic<T: Float>(&self, start: NaiveDate, maturity: NaiveDate) -> T {
        T::from(self.maturity_years(start, maturity)).unwrap_or_else(T::zero)
    }

    /// Get the configured spot date convention.
    pub fn spot_convention(&self) -> SpotDateConvention { self.spot_convention }

    /// Get the configured business day convention.
    pub fn business_day_convention(&self) -> BusinessDayConvention { self.business_day_convention }

    /// Get the configured day counter.
    pub fn day_counter(&self) -> DayCounter { self.day_counter }
}

/// Builder for `DateCalculator`.
#[derive(Debug, Clone)]
pub struct DateCalculatorBuilder {
    spot_convention: SpotDateConvention,
    business_day_convention: BusinessDayConvention,
    day_counter: DayCounter,
}

impl Default for DateCalculatorBuilder {
    fn default() -> Self {
        Self {
            spot_convention: SpotDateConvention::T2,
            business_day_convention: BusinessDayConvention::Following,
            day_counter: DayCounter::Actual365Fixed,
        }
    }
}

impl DateCalculatorBuilder {
    /// Set the spot date convention.
    pub fn spot_convention(mut self, convention: SpotDateConvention) -> Self {
        self.spot_convention = convention;
        self
    }

    /// Set the business day convention.
    pub fn business_day_convention(mut self, convention: BusinessDayConvention) -> Self {
        self.business_day_convention = convention;
        self
    }

    /// Set the day counter.
    pub fn day_counter(mut self, day_counter: DayCounter) -> Self {
        self.day_counter = day_counter;
        self
    }

    /// Build the date calculator.
    pub fn build(self) -> DateCalculator {
        DateCalculator {
            spot_convention: self.spot_convention,
            business_day_convention: self.business_day_convention,
            day_counter: self.day_counter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // SpotDateConvention Tests
    // ========================================

    #[test]
    fn test_spot_convention_business_days() {
        assert_eq!(SpotDateConvention::T0.business_days(), 0);
        assert_eq!(SpotDateConvention::T1.business_days(), 1);
        assert_eq!(SpotDateConvention::T2.business_days(), 2);
        assert_eq!(SpotDateConvention::T3.business_days(), 3);
    }

    #[test]
    fn test_spot_convention_name() {
        assert_eq!(SpotDateConvention::T0.name(), "T+0");
        assert_eq!(SpotDateConvention::T1.name(), "T+1");
        assert_eq!(SpotDateConvention::T2.name(), "T+2");
        assert_eq!(SpotDateConvention::T3.name(), "T+3");
    }

    #[test]
    fn test_spot_convention_default() {
        let conv: SpotDateConvention = Default::default();
        assert_eq!(conv, SpotDateConvention::T2);
    }

    #[test]
    fn test_spot_convention_display() {
        assert_eq!(format!("{}", SpotDateConvention::T2), "T+2");
    }

    // ========================================
    // DateCalculator Tests
    // ========================================

    fn calc() -> DateCalculator { DateCalculator::new() }

    #[test]
    fn test_is_weekend() {
        let calc = calc();
        // Saturday 2026-01-10
        let saturday = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        assert!(calc.is_weekend(saturday));
        // Sunday 2026-01-11
        let sunday = NaiveDate::from_ymd_opt(2026, 1, 11).unwrap();
        assert!(calc.is_weekend(sunday));
        // Monday 2026-01-12
        let monday = NaiveDate::from_ymd_opt(2026, 1, 12).unwrap();
        assert!(!calc.is_weekend(monday));
    }

    #[test]
    fn test_is_business_day() {
        let calc = calc();
        let monday = NaiveDate::from_ymd_opt(2026, 1, 12).unwrap();
        let saturday = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        assert!(calc.is_business_day(monday));
        assert!(!calc.is_business_day(saturday));
    }

    #[test]
    fn test_following() {
        let calc = calc();
        // Saturday -> Monday
        let saturday = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let monday = NaiveDate::from_ymd_opt(2026, 1, 12).unwrap();
        assert_eq!(calc.following(saturday), monday);

        // Monday stays Monday
        assert_eq!(calc.following(monday), monday);
    }

    #[test]
    fn test_preceding() {
        let calc = calc();
        // Saturday -> Friday
        let saturday = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let friday = NaiveDate::from_ymd_opt(2026, 1, 9).unwrap();
        assert_eq!(calc.preceding(saturday), friday);

        // Friday stays Friday
        assert_eq!(calc.preceding(friday), friday);
    }

    #[test]
    fn test_modified_following_no_month_cross() {
        let calc = calc();
        // Saturday Jan 10 -> Monday Jan 12 (no month cross)
        let saturday = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let monday = NaiveDate::from_ymd_opt(2026, 1, 12).unwrap();
        assert_eq!(calc.modified_following(saturday), monday);
    }

    #[test]
    fn test_modified_following_month_cross() {
        let calc = calc();
        // Saturday Jan 31 is a Saturday in 2026
        // Following would be Monday Feb 2 (crosses month)
        // Modified following should go back to Friday Jan 30
        let saturday = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        let friday = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        assert_eq!(calc.modified_following(saturday), friday);
    }

    #[test]
    fn test_modified_preceding_no_month_cross() {
        let calc = calc();
        // Sunday Jan 11 -> Friday Jan 9 (no month cross)
        let sunday = NaiveDate::from_ymd_opt(2026, 1, 11).unwrap();
        let friday = NaiveDate::from_ymd_opt(2026, 1, 9).unwrap();
        assert_eq!(calc.modified_preceding(sunday), friday);
    }

    #[test]
    fn test_modified_preceding_month_cross() {
        let calc = calc();
        // Sunday Feb 1, 2026 - preceding would be Friday Jan 30 (crosses month)
        // Modified preceding should go forward to Monday Feb 2
        let sunday = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let monday = NaiveDate::from_ymd_opt(2026, 2, 2).unwrap();
        assert_eq!(calc.modified_preceding(sunday), monday);
    }

    #[test]
    fn test_adjust() {
        let calc = calc();
        let saturday = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let monday = NaiveDate::from_ymd_opt(2026, 1, 12).unwrap();
        let friday = NaiveDate::from_ymd_opt(2026, 1, 9).unwrap();

        assert_eq!(
            calc.adjust(saturday, BusinessDayConvention::Following),
            monday
        );
        assert_eq!(
            calc.adjust(saturday, BusinessDayConvention::Preceding),
            friday
        );
        assert_eq!(
            calc.adjust(saturday, BusinessDayConvention::Unadjusted),
            saturday
        );
    }

    #[test]
    fn test_add_business_days() {
        let calc = calc();
        // Friday + 1 business day = Monday
        let friday = NaiveDate::from_ymd_opt(2026, 1, 9).unwrap();
        let monday = NaiveDate::from_ymd_opt(2026, 1, 12).unwrap();
        assert_eq!(calc.add_business_days(friday, 1), monday);

        // Friday + 2 business days = Tuesday
        let tuesday = NaiveDate::from_ymd_opt(2026, 1, 13).unwrap();
        assert_eq!(calc.add_business_days(friday, 2), tuesday);

        // Monday - 1 business day = Friday
        assert_eq!(calc.add_business_days(monday, -1), friday);
    }

    #[test]
    fn test_spot_date() {
        let calc = calc();
        // Monday Jan 12, 2026
        let trade_date = NaiveDate::from_ymd_opt(2026, 1, 12).unwrap();

        // T+0 = same day (Monday)
        assert_eq!(
            calc.spot_date(trade_date, SpotDateConvention::T0),
            trade_date
        );

        // T+1 = Tuesday
        let t1 = NaiveDate::from_ymd_opt(2026, 1, 13).unwrap();
        assert_eq!(calc.spot_date(trade_date, SpotDateConvention::T1), t1);

        // T+2 = Wednesday
        let t2 = NaiveDate::from_ymd_opt(2026, 1, 14).unwrap();
        assert_eq!(calc.spot_date(trade_date, SpotDateConvention::T2), t2);
    }

    #[test]
    fn test_spot_date_over_weekend() {
        let calc = calc();
        // Thursday Jan 8, 2026
        let thursday = NaiveDate::from_ymd_opt(2026, 1, 8).unwrap();

        // T+2 = Monday (skips weekend)
        let monday = NaiveDate::from_ymd_opt(2026, 1, 12).unwrap();
        assert_eq!(calc.spot_date(thursday, SpotDateConvention::T2), monday);
    }

    #[test]
    fn test_year_fraction() {
        let calc = calc();
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();

        let yf_365 = calc.year_fraction(start, end, DayCounter::Actual365Fixed);
        assert!((yf_365 - 1.0).abs() < 1e-10);

        let yf_360 = calc.year_fraction(start, end, DayCounter::Actual360);
        assert!((yf_360 - 365.0 / 360.0).abs() < 1e-10);
    }

    #[test]
    fn test_maturity_years() {
        let calc = calc();
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let maturity_1y = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        let maturity_6m = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();

        let yf_1y = calc.maturity_years(start, maturity_1y);
        assert!((yf_1y - 1.0).abs() < 0.01);

        let yf_6m = calc.maturity_years(start, maturity_6m);
        assert!((yf_6m - 0.5).abs() < 0.01);
    }

    // ========================================
    // Builder Tests
    // ========================================

    #[test]
    fn test_builder_default() {
        let calc = DateCalculator::builder().build();
        assert_eq!(calc.spot_convention(), SpotDateConvention::T2);
        assert_eq!(
            calc.business_day_convention(),
            BusinessDayConvention::Following
        );
        assert_eq!(calc.day_counter(), DayCounter::Actual365Fixed);
    }

    #[test]
    fn test_builder_custom() {
        let calc = DateCalculator::builder()
            .spot_convention(SpotDateConvention::T1)
            .business_day_convention(BusinessDayConvention::ModifiedFollowing)
            .day_counter(DayCounter::Actual360)
            .build();

        assert_eq!(calc.spot_convention(), SpotDateConvention::T1);
        assert_eq!(
            calc.business_day_convention(),
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(calc.day_counter(), DayCounter::Actual360);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_clone() {
        let calc1 = DateCalculator::new();
        let calc2 = calc1.clone();
        assert_eq!(calc1.spot_convention(), calc2.spot_convention());
        assert_eq!(calc1.day_counter(), calc2.day_counter());
    }
}
