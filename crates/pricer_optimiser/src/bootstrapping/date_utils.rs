//! Date calculation utilities for yield curve bootstrapping.
//!
//! This module provides helper functions and types for date calculations
//! in the context of yield curve construction, integrating with the
//! `infra_master` calendar functionality and `pricer_core` day count conventions.
//!
//! ## Features
//!
//! - Business day adjustments (Following, Modified Following, Preceding)
//! - Spot date calculation (T+n settlement)
//! - Year fraction calculations using various day count conventions
//! - Integration with `infra_master` calendars
//!
//! ## Example
//!
//! ```rust,ignore
//! use pricer_optimiser::bootstrapping::{DateCalculator, SpotDateConvention};
//! use chrono::NaiveDate;
//!
//! let calc = DateCalculator::new();
//! let trade_date = NaiveDate::from_ymd_opt(2026, 1, 14).unwrap();
//! let spot_date = calc.spot_date(trade_date, SpotDateConvention::T2);
//! ```

use chrono::{Datelike, NaiveDate, Weekday};
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

/// Business day adjustment convention.
///
/// Defines how dates falling on non-business days are adjusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BusinessDayAdjustment {
    /// Move to next business day
    #[default]
    Following,
    /// Move to next business day, unless it crosses month boundary
    ModifiedFollowing,
    /// Move to previous business day
    Preceding,
    /// Move to previous business day, unless it crosses month boundary
    ModifiedPreceding,
    /// No adjustment
    Unadjusted,
}

impl BusinessDayAdjustment {
    /// Get the adjustment name.
    pub fn name(&self) -> &'static str {
        match self {
            BusinessDayAdjustment::Following => "Following",
            BusinessDayAdjustment::ModifiedFollowing => "Modified Following",
            BusinessDayAdjustment::Preceding => "Preceding",
            BusinessDayAdjustment::ModifiedPreceding => "Modified Preceding",
            BusinessDayAdjustment::Unadjusted => "Unadjusted",
        }
    }
}

impl std::fmt::Display for BusinessDayAdjustment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Day count convention for year fraction calculations.
///
/// Mirrors `pricer_core::types::time::DayCountConvention` for convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DayCount {
    /// Actual/360 - actual days divided by 360
    Act360,
    /// Actual/365 Fixed - actual days divided by 365
    #[default]
    Act365Fixed,
    /// 30/360 Bond Basis
    Thirty360,
}

impl DayCount {
    /// Calculate year fraction between two dates.
    pub fn year_fraction(&self, start: NaiveDate, end: NaiveDate) -> f64 {
        let days = (end - start).num_days() as f64;

        match self {
            DayCount::Act360 => days / 360.0,
            DayCount::Act365Fixed => days / 365.0,
            DayCount::Thirty360 => self.thirty_360_fraction(start, end),
        }
    }

    /// Calculate year fraction as generic float type.
    pub fn year_fraction_generic<T: Float>(&self, start: NaiveDate, end: NaiveDate) -> T {
        T::from(self.year_fraction(start, end)).unwrap_or_else(T::zero)
    }

    /// Get the convention name.
    pub fn name(&self) -> &'static str {
        match self {
            DayCount::Act360 => "ACT/360",
            DayCount::Act365Fixed => "ACT/365",
            DayCount::Thirty360 => "30/360",
        }
    }

    fn thirty_360_fraction(&self, start: NaiveDate, end: NaiveDate) -> f64 {
        let (y1, m1, d1) = (start.year(), start.month() as i32, start.day() as i32);
        let (y2, m2, d2) = (end.year(), end.month() as i32, end.day() as i32);

        let d1_adj = d1.min(30);
        let d2_adj = if d1_adj == 30 { d2.min(30) } else { d2 };

        let days = 360 * (y2 - y1) + 30 * (m2 - m1) + (d2_adj - d1_adj);
        days as f64 / 360.0
    }
}

impl std::fmt::Display for DayCount {
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
    business_day_adjustment: BusinessDayAdjustment,
    /// Default day count convention
    day_count: DayCount,
}

impl Default for DateCalculator {
    fn default() -> Self {
        Self {
            spot_convention: SpotDateConvention::T2,
            business_day_adjustment: BusinessDayAdjustment::ModifiedFollowing,
            day_count: DayCount::Act365Fixed,
        }
    }
}

impl DateCalculator {
    /// Create a new date calculator with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for customised configuration.
    pub fn builder() -> DateCalculatorBuilder {
        DateCalculatorBuilder::default()
    }

    /// Check if a date is a weekend.
    pub fn is_weekend(&self, date: NaiveDate) -> bool {
        date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun
    }

    /// Check if a date is a business day (weekends only check).
    ///
    /// For more comprehensive holiday checking, use `infra_master::Calendar`.
    pub fn is_business_day(&self, date: NaiveDate) -> bool {
        !self.is_weekend(date)
    }

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
        if following.month() != date.month() {
            self.preceding(date)
        } else {
            following
        }
    }

    /// Apply modified preceding adjustment.
    ///
    /// Moves to previous business day, but if that crosses a month boundary,
    /// moves to the next business day instead.
    pub fn modified_preceding(&self, date: NaiveDate) -> NaiveDate {
        let preceding = self.preceding(date);
        if preceding.month() != date.month() {
            self.following(date)
        } else {
            preceding
        }
    }

    /// Adjust a date according to the specified business day adjustment.
    pub fn adjust(&self, date: NaiveDate, adjustment: BusinessDayAdjustment) -> NaiveDate {
        match adjustment {
            BusinessDayAdjustment::Following => self.following(date),
            BusinessDayAdjustment::ModifiedFollowing => self.modified_following(date),
            BusinessDayAdjustment::Preceding => self.preceding(date),
            BusinessDayAdjustment::ModifiedPreceding => self.modified_preceding(date),
            BusinessDayAdjustment::Unadjusted => date,
        }
    }

    /// Adjust a date using the default business day adjustment.
    pub fn adjust_default(&self, date: NaiveDate) -> NaiveDate {
        self.adjust(date, self.business_day_adjustment)
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
    pub fn year_fraction(&self, start: NaiveDate, end: NaiveDate, day_count: DayCount) -> f64 {
        day_count.year_fraction(start, end)
    }

    /// Calculate year fraction using the default day count convention.
    pub fn year_fraction_default(&self, start: NaiveDate, end: NaiveDate) -> f64 {
        self.year_fraction(start, end, self.day_count)
    }

    /// Calculate maturity in years from a start date.
    ///
    /// Converts a NaiveDate maturity to a year fraction.
    pub fn maturity_years(&self, start: NaiveDate, maturity: NaiveDate) -> f64 {
        self.day_count.year_fraction(start, maturity)
    }

    /// Calculate maturity in years as generic float type.
    pub fn maturity_years_generic<T: Float>(&self, start: NaiveDate, maturity: NaiveDate) -> T {
        self.day_count.year_fraction_generic(start, maturity)
    }

    /// Get the configured spot date convention.
    pub fn spot_convention(&self) -> SpotDateConvention {
        self.spot_convention
    }

    /// Get the configured business day adjustment.
    pub fn business_day_adjustment(&self) -> BusinessDayAdjustment {
        self.business_day_adjustment
    }

    /// Get the configured day count convention.
    pub fn day_count(&self) -> DayCount {
        self.day_count
    }
}

/// Builder for `DateCalculator`.
#[derive(Debug, Clone, Default)]
pub struct DateCalculatorBuilder {
    spot_convention: SpotDateConvention,
    business_day_adjustment: BusinessDayAdjustment,
    day_count: DayCount,
}

impl DateCalculatorBuilder {
    /// Set the spot date convention.
    pub fn spot_convention(mut self, convention: SpotDateConvention) -> Self {
        self.spot_convention = convention;
        self
    }

    /// Set the business day adjustment.
    pub fn business_day_adjustment(mut self, adjustment: BusinessDayAdjustment) -> Self {
        self.business_day_adjustment = adjustment;
        self
    }

    /// Set the day count convention.
    pub fn day_count(mut self, day_count: DayCount) -> Self {
        self.day_count = day_count;
        self
    }

    /// Build the date calculator.
    pub fn build(self) -> DateCalculator {
        DateCalculator {
            spot_convention: self.spot_convention,
            business_day_adjustment: self.business_day_adjustment,
            day_count: self.day_count,
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
    // BusinessDayAdjustment Tests
    // ========================================

    #[test]
    fn test_bda_name() {
        assert_eq!(BusinessDayAdjustment::Following.name(), "Following");
        assert_eq!(
            BusinessDayAdjustment::ModifiedFollowing.name(),
            "Modified Following"
        );
        assert_eq!(BusinessDayAdjustment::Preceding.name(), "Preceding");
        assert_eq!(
            BusinessDayAdjustment::ModifiedPreceding.name(),
            "Modified Preceding"
        );
        assert_eq!(BusinessDayAdjustment::Unadjusted.name(), "Unadjusted");
    }

    #[test]
    fn test_bda_default() {
        let bda: BusinessDayAdjustment = Default::default();
        assert_eq!(bda, BusinessDayAdjustment::Following);
    }

    #[test]
    fn test_bda_display() {
        assert_eq!(
            format!("{}", BusinessDayAdjustment::ModifiedFollowing),
            "Modified Following"
        );
    }

    // ========================================
    // DayCount Tests
    // ========================================

    #[test]
    fn test_day_count_act360() {
        let dc = DayCount::Act360;
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(); // 90 days
        let yf = dc.year_fraction(start, end);
        assert!((yf - 90.0 / 360.0).abs() < 1e-10);
    }

    #[test]
    fn test_day_count_act365() {
        let dc = DayCount::Act365Fixed;
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(); // 365 days
        let yf = dc.year_fraction(start, end);
        assert!((yf - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_day_count_thirty360() {
        let dc = DayCount::Thirty360;
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(); // 6 months
        let yf = dc.year_fraction(start, end);
        assert!((yf - 0.5).abs() < 1e-10); // 180/360 = 0.5
    }

    #[test]
    fn test_day_count_generic() {
        let dc = DayCount::Act365Fixed;
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        let yf: f64 = dc.year_fraction_generic(start, end);
        assert!((yf - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_day_count_name() {
        assert_eq!(DayCount::Act360.name(), "ACT/360");
        assert_eq!(DayCount::Act365Fixed.name(), "ACT/365");
        assert_eq!(DayCount::Thirty360.name(), "30/360");
    }

    #[test]
    fn test_day_count_default() {
        let dc: DayCount = Default::default();
        assert_eq!(dc, DayCount::Act365Fixed);
    }

    // ========================================
    // DateCalculator Tests
    // ========================================

    fn calc() -> DateCalculator {
        DateCalculator::new()
    }

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
            calc.adjust(saturday, BusinessDayAdjustment::Following),
            monday
        );
        assert_eq!(
            calc.adjust(saturday, BusinessDayAdjustment::Preceding),
            friday
        );
        assert_eq!(
            calc.adjust(saturday, BusinessDayAdjustment::Unadjusted),
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

        let yf_365 = calc.year_fraction(start, end, DayCount::Act365Fixed);
        assert!((yf_365 - 1.0).abs() < 1e-10);

        let yf_360 = calc.year_fraction(start, end, DayCount::Act360);
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
            calc.business_day_adjustment(),
            BusinessDayAdjustment::Following
        );
        assert_eq!(calc.day_count(), DayCount::Act365Fixed);
    }

    #[test]
    fn test_builder_custom() {
        let calc = DateCalculator::builder()
            .spot_convention(SpotDateConvention::T1)
            .business_day_adjustment(BusinessDayAdjustment::ModifiedFollowing)
            .day_count(DayCount::Act360)
            .build();

        assert_eq!(calc.spot_convention(), SpotDateConvention::T1);
        assert_eq!(
            calc.business_day_adjustment(),
            BusinessDayAdjustment::ModifiedFollowing
        );
        assert_eq!(calc.day_count(), DayCount::Act360);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_clone() {
        let calc1 = DateCalculator::new();
        let calc2 = calc1.clone();
        assert_eq!(calc1.spot_convention(), calc2.spot_convention());
        assert_eq!(calc1.day_count(), calc2.day_count());
    }
}
