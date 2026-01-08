//! Time types and Day Count Conventions for financial calculations.
//!
//! This module provides:
//! - `Date`: Type-safe date wrapper around chrono::NaiveDate
//! - `DayCountConvention`: Industry-standard day count conventions
//! - Year fraction calculations for financial instruments
//!
//! # Examples
//!
//! ```
//! use pricer_core::types::time::{Date, DayCountConvention};
//!
//! let start = Date::from_ymd(2024, 1, 1).unwrap();
//! let end = Date::from_ymd(2024, 7, 1).unwrap();
//!
//! // Calculate year fraction using ACT/365
//! let yf = DayCountConvention::ActualActual365.year_fraction_dates(start, end);
//! assert!((yf - 0.4986).abs() < 0.001);
//! ```

use chrono::{Datelike, Local, NaiveDate};
use std::fmt;
use std::ops::Sub;
use std::str::FromStr;

use super::error::DateError;

/// Type-safe date wrapper around chrono::NaiveDate.
///
/// Provides ISO 8601 serialisation and standard date arithmetic.
/// This wrapper ensures type safety and provides a consistent API
/// for date operations in financial calculations.
///
/// # Examples
///
/// ```
/// use pricer_core::types::time::Date;
///
/// // Create from year, month, day
/// let date = Date::from_ymd(2024, 6, 15).unwrap();
/// assert_eq!(date.year(), 2024);
/// assert_eq!(date.month(), 6);
/// assert_eq!(date.day(), 15);
///
/// // Parse from ISO 8601 string
/// let parsed: Date = "2024-06-15".parse().unwrap();
/// assert_eq!(date, parsed);
///
/// // Calculate days between dates
/// let start = Date::from_ymd(2024, 1, 1).unwrap();
/// let end = Date::from_ymd(2024, 1, 11).unwrap();
/// assert_eq!(end - start, 10);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Date(NaiveDate);

impl Date {
    /// Creates a Date from year, month, and day components.
    ///
    /// # Arguments
    /// * `year` - Year (e.g., 2024)
    /// * `month` - Month (1-12)
    /// * `day` - Day (1-31, depending on month)
    ///
    /// # Returns
    /// `Ok(Date)` if the date is valid, `Err(DateError::InvalidDate)` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::Date;
    ///
    /// // Valid date
    /// let date = Date::from_ymd(2024, 6, 15).unwrap();
    ///
    /// // Leap year February 29th
    /// let leap = Date::from_ymd(2024, 2, 29).unwrap();
    ///
    /// // Invalid date returns error
    /// let invalid = Date::from_ymd(2024, 2, 30);
    /// assert!(invalid.is_err());
    /// ```
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Result<Self, DateError> {
        NaiveDate::from_ymd_opt(year, month, day)
            .map(Date)
            .ok_or(DateError::InvalidDate { year, month, day })
    }

    /// Returns today's date based on local system time.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::Date;
    ///
    /// let today = Date::today();
    /// // today is the current local date
    /// ```
    pub fn today() -> Self {
        Date(Local::now().date_naive())
    }

    /// Parses a date from ISO 8601 format string (YYYY-MM-DD).
    ///
    /// # Arguments
    /// * `s` - Date string in ISO 8601 format
    ///
    /// # Returns
    /// `Ok(Date)` if parsing succeeds, `Err(DateError::ParseError)` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::Date;
    ///
    /// let date = Date::parse("2024-06-15").unwrap();
    /// assert_eq!(date.year(), 2024);
    ///
    /// let invalid = Date::parse("not-a-date");
    /// assert!(invalid.is_err());
    /// ```
    pub fn parse(s: &str) -> Result<Self, DateError> {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Date)
            .map_err(|e| DateError::ParseError(e.to_string()))
    }

    /// Returns the underlying NaiveDate.
    ///
    /// Use this method when you need access to chrono's full API.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::Date;
    /// use chrono::Datelike;
    ///
    /// let date = Date::from_ymd(2024, 6, 15).unwrap();
    /// let naive = date.into_inner();
    /// assert_eq!(naive.weekday(), chrono::Weekday::Sat);
    /// ```
    pub fn into_inner(self) -> NaiveDate {
        self.0
    }

    /// Returns the year component.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::Date;
    ///
    /// let date = Date::from_ymd(2024, 6, 15).unwrap();
    /// assert_eq!(date.year(), 2024);
    /// ```
    pub fn year(&self) -> i32 {
        self.0.year()
    }

    /// Returns the month component (1-12).
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::Date;
    ///
    /// let date = Date::from_ymd(2024, 6, 15).unwrap();
    /// assert_eq!(date.month(), 6);
    /// ```
    pub fn month(&self) -> u32 {
        self.0.month()
    }

    /// Returns the day component (1-31).
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::Date;
    ///
    /// let date = Date::from_ymd(2024, 6, 15).unwrap();
    /// assert_eq!(date.day(), 15);
    /// ```
    pub fn day(&self) -> u32 {
        self.0.day()
    }
}

impl Sub for Date {
    type Output = i64;

    /// Returns the number of days between two dates.
    ///
    /// The result is positive if `self` is after `other`, negative otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::Date;
    ///
    /// let start = Date::from_ymd(2024, 1, 1).unwrap();
    /// let end = Date::from_ymd(2024, 1, 11).unwrap();
    ///
    /// assert_eq!(end - start, 10);
    /// assert_eq!(start - end, -10);
    /// ```
    fn sub(self, other: Self) -> i64 {
        (self.0 - other.0).num_days()
    }
}

impl FromStr for Date {
    type Err = DateError;

    /// Parses a date from ISO 8601 format string (YYYY-MM-DD).
    fn from_str(s: &str) -> Result<Self, DateError> {
        Date::parse(s)
    }
}

impl fmt::Display for Date {
    /// Formats the date as ISO 8601 (YYYY-MM-DD).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d"))
    }
}

/// Day Count Convention (year fraction convention).
///
/// # Variants
/// - `ActualActual365`: Actual days / 365 (standard for derivatives and UK bonds)
/// - `ActualActual360`: Actual days / 360 (common in money market instruments)
/// - `Thirty360`: Each month treated as 30 days, year as 360 days (US corporate bonds)
///
/// # Usage
///
/// ```
/// use pricer_core::types::time::DayCountConvention;
/// use chrono::NaiveDate;
///
/// let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
/// let end = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
///
/// let act_365 = DayCountConvention::ActualActual365;
/// let year_fraction = act_365.year_fraction(start, end);
/// // 182 days / 365.0 ≈ 0.4986
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DayCountConvention {
    /// Actual/365 Fixed: actual_days / 365.0
    ///
    /// Used in:
    /// - Most derivatives markets
    /// - UK gilts
    /// - Japanese government bonds
    ActualActual365,

    /// Actual/360: actual_days / 360.0
    ///
    /// Used in:
    /// - Money market instruments
    /// - US Treasury bills
    /// - LIBOR-based instruments
    ActualActual360,

    /// 30/360 US Bond Basis
    ///
    /// Used in:
    /// - US corporate bonds
    /// - US agency bonds
    /// - Some municipal bonds
    ///
    /// Each month is treated as having 30 days, and the year as 360 days.
    Thirty360,
}

impl DayCountConvention {
    /// Returns the standard convention name.
    ///
    /// Returns industry-standard convention names for serialisation
    /// and display purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::DayCountConvention;
    ///
    /// assert_eq!(DayCountConvention::ActualActual365.name(), "ACT/365");
    /// assert_eq!(DayCountConvention::ActualActual360.name(), "ACT/360");
    /// assert_eq!(DayCountConvention::Thirty360.name(), "30/360");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            DayCountConvention::ActualActual365 => "ACT/365",
            DayCountConvention::ActualActual360 => "ACT/360",
            DayCountConvention::Thirty360 => "30/360",
        }
    }

    /// Calculate year fraction between two dates.
    ///
    /// # Arguments
    /// * `start` - Start date
    /// * `end` - End date
    ///
    /// # Returns
    /// Year fraction as f64 (e.g., 0.5 for 6 months, 1.0 for 1 year)
    ///
    /// # Panics
    /// Panics if `start > end`
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::DayCountConvention;
    /// use chrono::NaiveDate;
    ///
    /// let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    /// let end = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
    ///
    /// // Act/365
    /// let act_365 = DayCountConvention::ActualActual365;
    /// let yf_365 = act_365.year_fraction(start, end);
    /// assert!((yf_365 - 0.4986).abs() < 0.001);
    ///
    /// // Act/360
    /// let act_360 = DayCountConvention::ActualActual360;
    /// let yf_360 = act_360.year_fraction(start, end);
    /// assert!((yf_360 - 0.5056).abs() < 0.001);
    /// ```
    pub fn year_fraction(&self, start: NaiveDate, end: NaiveDate) -> f64 {
        assert!(
            start <= end,
            "start date must be less than or equal to end date"
        );

        match self {
            DayCountConvention::ActualActual365 => {
                let days = (end - start).num_days();
                days as f64 / 365.0
            }
            DayCountConvention::ActualActual360 => {
                let days = (end - start).num_days();
                days as f64 / 360.0
            }
            DayCountConvention::Thirty360 => {
                // 30/360 US Bond Basis implementation
                let y1 = start.year();
                let m1 = start.month();
                let d1 = start.day();

                let y2 = end.year();
                let m2 = end.month();
                let d2 = end.day();

                // 30/360 US adjustments
                let d1_adj = if d1 == 31 { 30 } else { d1 };
                let d2_adj = if d2 == 31 && d1_adj == 30 { 30 } else { d2 };

                let days = 360 * (y2 - y1)
                    + 30 * (m2 as i32 - m1 as i32)
                    + (d2_adj as i32 - d1_adj as i32);
                days as f64 / 360.0
            }
        }
    }

    /// Calculates year fraction using Date type.
    ///
    /// Unlike `year_fraction`, this method returns negative values
    /// when start > end instead of panicking. This is useful for
    /// calculations where the sign indicates direction.
    ///
    /// # Arguments
    /// * `start` - Start date
    /// * `end` - End date
    ///
    /// # Returns
    /// Year fraction as f64. Negative if start > end.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::{Date, DayCountConvention};
    ///
    /// let start = Date::from_ymd(2024, 1, 1).unwrap();
    /// let end = Date::from_ymd(2024, 7, 1).unwrap();
    ///
    /// let yf = DayCountConvention::ActualActual365.year_fraction_dates(start, end);
    /// assert!((yf - 0.4986).abs() < 0.001);
    ///
    /// // Reversed dates return negative value
    /// let yf_neg = DayCountConvention::ActualActual365.year_fraction_dates(end, start);
    /// assert!((yf_neg + 0.4986).abs() < 0.001);
    /// ```
    pub fn year_fraction_dates(&self, start: Date, end: Date) -> f64 {
        let days = end - start; // Returns i64, can be negative

        match self {
            DayCountConvention::ActualActual365 => days as f64 / 365.0,
            DayCountConvention::ActualActual360 => days as f64 / 360.0,
            DayCountConvention::Thirty360 => {
                // For 30/360, we need to handle negative direction
                let (start_inner, end_inner, sign) = if start <= end {
                    (start.into_inner(), end.into_inner(), 1.0)
                } else {
                    (end.into_inner(), start.into_inner(), -1.0)
                };

                let y1 = start_inner.year();
                let m1 = start_inner.month();
                let d1 = start_inner.day();

                let y2 = end_inner.year();
                let m2 = end_inner.month();
                let d2 = end_inner.day();

                let d1_adj = if d1 == 31 { 30 } else { d1 };
                let d2_adj = if d2 == 31 && d1_adj == 30 { 30 } else { d2 };

                let days_30_360 = 360 * (y2 - y1)
                    + 30 * (m2 as i32 - m1 as i32)
                    + (d2_adj as i32 - d1_adj as i32);
                sign * days_30_360 as f64 / 360.0
            }
        }
    }
}

impl FromStr for DayCountConvention {
    type Err = String;

    /// Parses day count convention from string (case-insensitive).
    ///
    /// Supports multiple aliases for each convention:
    /// - ACT/365: "ACT/365", "Actual/365", "Act365", "A365"
    /// - ACT/360: "ACT/360", "Actual/360", "Act360", "A360"
    /// - 30/360: "30/360", "Thirty360", "30360"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().replace(['/', ' '], "").as_str() {
            "ACT365" | "ACTUAL365" | "A365" => Ok(DayCountConvention::ActualActual365),
            "ACT360" | "ACTUAL360" | "A360" => Ok(DayCountConvention::ActualActual360),
            "30360" | "THIRTY360" => Ok(DayCountConvention::Thirty360),
            _ => Err(format!("Unknown day count convention: {}", s)),
        }
    }
}

impl fmt::Display for DayCountConvention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::DayCountConvention;
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use std::str::FromStr;

    impl Serialize for DayCountConvention {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(self.name())
        }
    }

    impl<'de> Deserialize<'de> for DayCountConvention {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            DayCountConvention::from_str(&s).map_err(de::Error::custom)
        }
    }
}

/// Business Day Convention for date adjustments.
///
/// Defines how to adjust dates that fall on non-business days (weekends, holidays).
///
/// # Variants
///
/// - `Following`: Move to the next business day
/// - `ModifiedFollowing`: Move to the next business day, unless it crosses a month boundary
/// - `Preceding`: Move to the previous business day
/// - `ModifiedPreceding`: Move to the previous business day, unless it crosses a month boundary
/// - `Unadjusted`: Do not adjust the date
///
/// # Examples
///
/// ```
/// use pricer_core::types::time::BusinessDayConvention;
///
/// let conv = BusinessDayConvention::ModifiedFollowing;
/// assert_eq!(conv.name(), "Modified Following");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BusinessDayConvention {
    /// Move to the next business day.
    ///
    /// If a date falls on a weekend or holiday, move forward to the
    /// next valid business day.
    Following,

    /// Move to the next business day, unless it crosses a month boundary.
    ///
    /// If moving forward would cross into a new month, move backward
    /// to the previous business day instead. This is the most common
    /// convention for money market instruments.
    ModifiedFollowing,

    /// Move to the previous business day.
    ///
    /// If a date falls on a weekend or holiday, move backward to the
    /// previous valid business day.
    Preceding,

    /// Move to the previous business day, unless it crosses a month boundary.
    ///
    /// If moving backward would cross into a previous month, move forward
    /// to the next business day instead.
    ModifiedPreceding,

    /// Do not adjust the date.
    ///
    /// The date is used as-is, even if it falls on a weekend or holiday.
    Unadjusted,
}

impl BusinessDayConvention {
    /// Returns the standard name for this convention.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::BusinessDayConvention;
    ///
    /// assert_eq!(BusinessDayConvention::Following.name(), "Following");
    /// assert_eq!(BusinessDayConvention::ModifiedFollowing.name(), "Modified Following");
    /// ```
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            BusinessDayConvention::Following => "Following",
            BusinessDayConvention::ModifiedFollowing => "Modified Following",
            BusinessDayConvention::Preceding => "Preceding",
            BusinessDayConvention::ModifiedPreceding => "Modified Preceding",
            BusinessDayConvention::Unadjusted => "Unadjusted",
        }
    }

    /// Returns a short code for this convention.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::BusinessDayConvention;
    ///
    /// assert_eq!(BusinessDayConvention::Following.code(), "F");
    /// assert_eq!(BusinessDayConvention::ModifiedFollowing.code(), "MF");
    /// ```
    #[inline]
    pub fn code(&self) -> &'static str {
        match self {
            BusinessDayConvention::Following => "F",
            BusinessDayConvention::ModifiedFollowing => "MF",
            BusinessDayConvention::Preceding => "P",
            BusinessDayConvention::ModifiedPreceding => "MP",
            BusinessDayConvention::Unadjusted => "U",
        }
    }
}

impl fmt::Display for BusinessDayConvention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for BusinessDayConvention {
    type Err = String;

    /// Parses business day convention from string (case-insensitive).
    ///
    /// Supports full names and short codes:
    /// - Following: "following", "f"
    /// - ModifiedFollowing: "modified following", "modifiedfollowing", "mf"
    /// - Preceding: "preceding", "p"
    /// - ModifiedPreceding: "modified preceding", "modifiedpreceding", "mp"
    /// - Unadjusted: "unadjusted", "u", "none"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace([' ', '_', '-'], "").as_str() {
            "following" | "f" => Ok(BusinessDayConvention::Following),
            "modifiedfollowing" | "mf" => Ok(BusinessDayConvention::ModifiedFollowing),
            "preceding" | "p" => Ok(BusinessDayConvention::Preceding),
            "modifiedpreceding" | "mp" => Ok(BusinessDayConvention::ModifiedPreceding),
            "unadjusted" | "u" | "none" => Ok(BusinessDayConvention::Unadjusted),
            _ => Err(format!("Unknown business day convention: {}", s)),
        }
    }
}

#[cfg(feature = "serde")]
mod serde_bdc_impl {
    use super::BusinessDayConvention;
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use std::str::FromStr;

    impl Serialize for BusinessDayConvention {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(self.name())
        }
    }

    impl<'de> Deserialize<'de> for BusinessDayConvention {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            BusinessDayConvention::from_str(&s).map_err(de::Error::custom)
        }
    }
}

/// Calculate time to maturity using default convention (Act/365).
///
/// # Arguments
/// * `start` - Valuation date
/// * `end` - Maturity date
///
/// # Returns
/// Time to maturity in years (Act/365 convention)
///
/// # Panics
/// Panics if `start > end`
///
/// # Examples
///
/// ```
/// use pricer_core::types::time::time_to_maturity;
/// use chrono::NaiveDate;
///
/// let valuation_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
/// let maturity_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
///
/// let ttm = time_to_maturity(valuation_date, maturity_date);
/// assert!((ttm - 1.0027).abs() < 0.001); // ~1 year (366 days in 2024 leap year)
/// ```
pub fn time_to_maturity(start: NaiveDate, end: NaiveDate) -> f64 {
    DayCountConvention::ActualActual365.year_fraction(start, end)
}

/// Calculate time to maturity using Date type and default convention (Act/365).
///
/// Unlike `time_to_maturity`, this function does not panic when start > end,
/// instead returning a negative value.
///
/// # Arguments
/// * `start` - Valuation date
/// * `end` - Maturity date
///
/// # Returns
/// Time to maturity in years (Act/365 convention). Negative if start > end.
///
/// # Examples
///
/// ```
/// use pricer_core::types::time::{Date, time_to_maturity_dates};
///
/// let valuation_date = Date::from_ymd(2024, 1, 1).unwrap();
/// let maturity_date = Date::from_ymd(2025, 1, 1).unwrap();
///
/// let ttm = time_to_maturity_dates(valuation_date, maturity_date);
/// assert!((ttm - 1.0027).abs() < 0.001); // ~1 year (366 days in 2024 leap year)
///
/// // Negative time to maturity (expired)
/// let ttm_neg = time_to_maturity_dates(maturity_date, valuation_date);
/// assert!(ttm_neg < 0.0);
/// ```
pub fn time_to_maturity_dates(start: Date, end: Date) -> f64 {
    DayCountConvention::ActualActual365.year_fraction_dates(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // Task 5.4: Day Count Convention unit tests

    #[test]
    fn test_act_365_known_dates() {
        // 2024-01-01 to 2024-07-01 is 182 days
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();

        let convention = DayCountConvention::ActualActual365;
        let result = convention.year_fraction(start, end);

        let expected = 182.0 / 365.0; // ≈ 0.4986
        assert_relative_eq!(result, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_act_360_known_dates() {
        // 2024-01-01 to 2024-07-01 is 182 days
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();

        let convention = DayCountConvention::ActualActual360;
        let result = convention.year_fraction(start, end);

        let expected = 182.0 / 360.0; // ≈ 0.5056
        assert_relative_eq!(result, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_thirty_360_known_dates() {
        // 2024-01-01 to 2024-07-01
        // Years: 0, Months: 6, Days: 0 (1st to 1st)
        // Total days in 30/360: 0*360 + 6*30 + 0 = 180
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();

        let convention = DayCountConvention::Thirty360;
        let result = convention.year_fraction(start, end);

        let expected = 180.0 / 360.0; // 0.5
        assert_relative_eq!(result, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_thirty_360_with_31st_days() {
        // Test 30/360 adjustments for 31st day of month
        // 2024-01-31 to 2024-03-31
        // d1 = 31 -> 30 (adjusted)
        // d2 = 31, d1_adj = 30 -> 30 (adjusted)
        // Months: 2, Days: 0 (after adjustment)
        // Total: 2*30 + 0 = 60 days
        let start = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 3, 31).unwrap();

        let convention = DayCountConvention::Thirty360;
        let result = convention.year_fraction(start, end);

        let expected = 60.0 / 360.0; // ≈ 0.1667
        assert_relative_eq!(result, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_time_to_maturity_matches_act_365() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();

        let ttm = time_to_maturity(start, end);
        let act_365 = DayCountConvention::ActualActual365.year_fraction(start, end);

        assert_relative_eq!(ttm, act_365, epsilon = 1e-10);
    }

    #[test]
    fn test_same_date_returns_zero() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let act_365 = DayCountConvention::ActualActual365;
        assert_eq!(act_365.year_fraction(date, date), 0.0);

        let act_360 = DayCountConvention::ActualActual360;
        assert_eq!(act_360.year_fraction(date, date), 0.0);

        let thirty_360 = DayCountConvention::Thirty360;
        assert_eq!(thirty_360.year_fraction(date, date), 0.0);
    }

    #[test]
    fn test_one_year_period() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();

        // 2024 is a leap year, so 366 days
        let act_365 = DayCountConvention::ActualActual365;
        let result_365 = act_365.year_fraction(start, end);
        assert_relative_eq!(result_365, 366.0 / 365.0, epsilon = 1e-10);

        let act_360 = DayCountConvention::ActualActual360;
        let result_360 = act_360.year_fraction(start, end);
        assert_relative_eq!(result_360, 366.0 / 360.0, epsilon = 1e-10);

        let thirty_360 = DayCountConvention::Thirty360;
        let result_30_360 = thirty_360.year_fraction(start, end);
        assert_relative_eq!(result_30_360, 1.0, epsilon = 1e-10); // Exactly 1 year in 30/360
    }

    #[test]
    #[should_panic(expected = "start date must be less than or equal to end date")]
    fn test_year_fraction_panics_on_reverse_dates() {
        let start = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let convention = DayCountConvention::ActualActual365;
        convention.year_fraction(start, end);
    }

    #[test]
    #[should_panic(expected = "start date must be less than or equal to end date")]
    fn test_time_to_maturity_panics_on_reverse_dates() {
        let start = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        time_to_maturity(start, end);
    }

    #[test]
    fn test_act_365_vs_act_360_ratio() {
        // Verify that Act/360 results are approximately 360/365 times Act/365
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

        let result_365 = DayCountConvention::ActualActual365.year_fraction(start, end);
        let result_360 = DayCountConvention::ActualActual360.year_fraction(start, end);

        // ratio should be close to 365/360
        let ratio = result_365 / result_360;
        assert_relative_eq!(ratio, 360.0 / 365.0, epsilon = 1e-10);
    }

    // Date tests

    #[test]
    fn test_date_from_ymd_valid() {
        let date = Date::from_ymd(2024, 6, 15).unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 6);
        assert_eq!(date.day(), 15);
    }

    #[test]
    fn test_date_from_ymd_leap_year() {
        // 2024 is a leap year
        let date = Date::from_ymd(2024, 2, 29).unwrap();
        assert_eq!(date.month(), 2);
        assert_eq!(date.day(), 29);
    }

    #[test]
    fn test_date_from_ymd_invalid() {
        // February 30 is invalid
        let result = Date::from_ymd(2024, 2, 30);
        assert!(result.is_err());

        // Month 13 is invalid
        let result = Date::from_ymd(2024, 13, 1);
        assert!(result.is_err());

        // Non-leap year February 29
        let result = Date::from_ymd(2023, 2, 29);
        assert!(result.is_err());
    }

    #[test]
    fn test_date_parse_valid() {
        let date = Date::parse("2024-06-15").unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 6);
        assert_eq!(date.day(), 15);
    }

    #[test]
    fn test_date_parse_invalid() {
        let result = Date::parse("not-a-date");
        assert!(result.is_err());

        let result = Date::parse("2024/06/15"); // Wrong format
        assert!(result.is_err());
    }

    #[test]
    fn test_date_from_str() {
        let date: Date = "2024-06-15".parse().unwrap();
        assert_eq!(date.year(), 2024);
    }

    #[test]
    fn test_date_display() {
        let date = Date::from_ymd(2024, 6, 15).unwrap();
        assert_eq!(format!("{}", date), "2024-06-15");
    }

    #[test]
    fn test_date_subtraction() {
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = Date::from_ymd(2024, 1, 11).unwrap();

        assert_eq!(end - start, 10);
        assert_eq!(start - end, -10);
    }

    #[test]
    fn test_date_ordering() {
        let earlier = Date::from_ymd(2024, 1, 1).unwrap();
        let later = Date::from_ymd(2024, 12, 31).unwrap();

        assert!(earlier < later);
        assert!(later > earlier);
        assert!(earlier <= earlier);
    }

    #[test]
    fn test_date_into_inner() {
        let date = Date::from_ymd(2024, 6, 15).unwrap();
        let naive = date.into_inner();
        assert_eq!(naive.year(), 2024);
    }

    // DayCountConvention name() tests

    #[test]
    fn test_dcc_name() {
        assert_eq!(DayCountConvention::ActualActual365.name(), "ACT/365");
        assert_eq!(DayCountConvention::ActualActual360.name(), "ACT/360");
        assert_eq!(DayCountConvention::Thirty360.name(), "30/360");
    }

    #[test]
    fn test_dcc_display() {
        assert_eq!(
            format!("{}", DayCountConvention::ActualActual365),
            "ACT/365"
        );
        assert_eq!(
            format!("{}", DayCountConvention::ActualActual360),
            "ACT/360"
        );
        assert_eq!(format!("{}", DayCountConvention::Thirty360), "30/360");
    }

    #[test]
    fn test_dcc_from_str() {
        assert_eq!(
            "ACT/365".parse::<DayCountConvention>().unwrap(),
            DayCountConvention::ActualActual365
        );
        assert_eq!(
            "act/360".parse::<DayCountConvention>().unwrap(),
            DayCountConvention::ActualActual360
        );
        assert_eq!(
            "30/360".parse::<DayCountConvention>().unwrap(),
            DayCountConvention::Thirty360
        );
        assert_eq!(
            "Thirty360".parse::<DayCountConvention>().unwrap(),
            DayCountConvention::Thirty360
        );
    }

    #[test]
    fn test_dcc_from_str_invalid() {
        let result = "INVALID".parse::<DayCountConvention>();
        assert!(result.is_err());
    }

    // year_fraction_dates tests

    #[test]
    fn test_year_fraction_dates_matches_year_fraction() {
        let start_date = Date::from_ymd(2024, 1, 1).unwrap();
        let end_date = Date::from_ymd(2024, 7, 1).unwrap();
        let start_naive = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end_naive = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();

        for dcc in [
            DayCountConvention::ActualActual365,
            DayCountConvention::ActualActual360,
            DayCountConvention::Thirty360,
        ] {
            let yf_dates = dcc.year_fraction_dates(start_date, end_date);
            let yf_naive = dcc.year_fraction(start_naive, end_naive);
            assert_relative_eq!(yf_dates, yf_naive, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_year_fraction_dates_negative() {
        let start = Date::from_ymd(2024, 7, 1).unwrap();
        let end = Date::from_ymd(2024, 1, 1).unwrap();

        // Should NOT panic, should return negative
        let yf = DayCountConvention::ActualActual365.year_fraction_dates(start, end);
        assert!(yf < 0.0);
        assert_relative_eq!(yf, -182.0 / 365.0, epsilon = 1e-10);
    }

    #[test]
    fn test_time_to_maturity_dates() {
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 1, 1).unwrap();

        let ttm = time_to_maturity_dates(start, end);
        assert_relative_eq!(ttm, 366.0 / 365.0, epsilon = 1e-10);
    }

    #[test]
    fn test_time_to_maturity_dates_negative() {
        let start = Date::from_ymd(2025, 1, 1).unwrap();
        let end = Date::from_ymd(2024, 1, 1).unwrap();

        // Should NOT panic, should return negative
        let ttm = time_to_maturity_dates(start, end);
        assert!(ttm < 0.0);
    }

    // BusinessDayConvention tests

    #[test]
    fn test_bdc_name() {
        assert_eq!(BusinessDayConvention::Following.name(), "Following");
        assert_eq!(
            BusinessDayConvention::ModifiedFollowing.name(),
            "Modified Following"
        );
        assert_eq!(BusinessDayConvention::Preceding.name(), "Preceding");
        assert_eq!(
            BusinessDayConvention::ModifiedPreceding.name(),
            "Modified Preceding"
        );
        assert_eq!(BusinessDayConvention::Unadjusted.name(), "Unadjusted");
    }

    #[test]
    fn test_bdc_code() {
        assert_eq!(BusinessDayConvention::Following.code(), "F");
        assert_eq!(BusinessDayConvention::ModifiedFollowing.code(), "MF");
        assert_eq!(BusinessDayConvention::Preceding.code(), "P");
        assert_eq!(BusinessDayConvention::ModifiedPreceding.code(), "MP");
        assert_eq!(BusinessDayConvention::Unadjusted.code(), "U");
    }

    #[test]
    fn test_bdc_display() {
        assert_eq!(format!("{}", BusinessDayConvention::Following), "Following");
        assert_eq!(
            format!("{}", BusinessDayConvention::ModifiedFollowing),
            "Modified Following"
        );
    }

    #[test]
    fn test_bdc_from_str_valid() {
        // Full names
        assert_eq!(
            "Following".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Following
        );
        assert_eq!(
            "modified following"
                .parse::<BusinessDayConvention>()
                .unwrap(),
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(
            "Modified_Following"
                .parse::<BusinessDayConvention>()
                .unwrap(),
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(
            "PRECEDING".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Preceding
        );
        assert_eq!(
            "unadjusted".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Unadjusted
        );

        // Short codes
        assert_eq!(
            "F".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Following
        );
        assert_eq!(
            "MF".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(
            "P".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Preceding
        );
        assert_eq!(
            "MP".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::ModifiedPreceding
        );
        assert_eq!(
            "U".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Unadjusted
        );
        assert_eq!(
            "none".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Unadjusted
        );
    }

    #[test]
    fn test_bdc_from_str_invalid() {
        assert!("invalid".parse::<BusinessDayConvention>().is_err());
        assert!("FFF".parse::<BusinessDayConvention>().is_err());
    }

    #[test]
    fn test_bdc_clone_and_copy() {
        let bdc1 = BusinessDayConvention::ModifiedFollowing;
        let bdc2 = bdc1; // Copy
        let bdc3 = bdc1.clone(); // Clone

        assert_eq!(bdc1, bdc2);
        assert_eq!(bdc1, bdc3);
    }

    #[test]
    fn test_bdc_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BusinessDayConvention::Following);
        set.insert(BusinessDayConvention::ModifiedFollowing);
        set.insert(BusinessDayConvention::Following); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_bdc_debug() {
        let debug_str = format!("{:?}", BusinessDayConvention::ModifiedFollowing);
        assert!(debug_str.contains("ModifiedFollowing"));
    }

    #[cfg(feature = "serde")]
    mod serde_tests {
        use super::*;

        #[test]
        fn test_date_serde_roundtrip() {
            let date = Date::from_ymd(2024, 6, 15).unwrap();
            let json = serde_json::to_string(&date).unwrap();
            assert_eq!(json, "\"2024-06-15\"");

            let parsed: Date = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, date);
        }

        #[test]
        fn test_dcc_serde_roundtrip() {
            let dcc = DayCountConvention::ActualActual365;
            let json = serde_json::to_string(&dcc).unwrap();
            assert_eq!(json, "\"ACT/365\"");

            let parsed: DayCountConvention = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, dcc);
        }

        #[test]
        fn test_dcc_serde_all_variants() {
            for dcc in [
                DayCountConvention::ActualActual365,
                DayCountConvention::ActualActual360,
                DayCountConvention::Thirty360,
            ] {
                let json = serde_json::to_string(&dcc).unwrap();
                let parsed: DayCountConvention = serde_json::from_str(&json).unwrap();
                assert_eq!(parsed, dcc);
            }
        }

        #[test]
        fn test_dcc_serde_deserialize_alias() {
            // Test case-insensitive and alias parsing
            let parsed: DayCountConvention = serde_json::from_str("\"Actual/365\"").unwrap();
            assert_eq!(parsed, DayCountConvention::ActualActual365);

            let parsed: DayCountConvention = serde_json::from_str("\"30/360\"").unwrap();
            assert_eq!(parsed, DayCountConvention::Thirty360);
        }

        // BusinessDayConvention serde tests

        #[test]
        fn test_bdc_serde_roundtrip() {
            let bdc = BusinessDayConvention::ModifiedFollowing;
            let json = serde_json::to_string(&bdc).unwrap();
            assert_eq!(json, "\"Modified Following\"");

            let parsed: BusinessDayConvention = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, bdc);
        }

        #[test]
        fn test_bdc_serde_all_variants() {
            for bdc in [
                BusinessDayConvention::Following,
                BusinessDayConvention::ModifiedFollowing,
                BusinessDayConvention::Preceding,
                BusinessDayConvention::ModifiedPreceding,
                BusinessDayConvention::Unadjusted,
            ] {
                let json = serde_json::to_string(&bdc).unwrap();
                let parsed: BusinessDayConvention = serde_json::from_str(&json).unwrap();
                assert_eq!(parsed, bdc);
            }
        }

        #[test]
        fn test_bdc_serde_deserialize_alias() {
            let parsed: BusinessDayConvention = serde_json::from_str("\"F\"").unwrap();
            assert_eq!(parsed, BusinessDayConvention::Following);

            let parsed: BusinessDayConvention = serde_json::from_str("\"MF\"").unwrap();
            assert_eq!(parsed, BusinessDayConvention::ModifiedFollowing);

            let parsed: BusinessDayConvention = serde_json::from_str("\"none\"").unwrap();
            assert_eq!(parsed, BusinessDayConvention::Unadjusted);
        }
    }

    // Task 6.2: Property-based tests for Day Count Convention
    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        // Generate valid NaiveDate values (avoiding edge cases)
        fn date_strategy() -> impl Strategy<Value = NaiveDate> {
            (2000i32..2100i32, 1u32..13u32, 1u32..29u32)
                .prop_filter_map("valid date", |(year, month, day)| {
                    NaiveDate::from_ymd_opt(year, month, day)
                })
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(1000))]

            #[test]
            fn test_year_fraction_non_negative(
                start in date_strategy(),
                end in date_strategy(),
            ) {
                // Only test when start <= end
                if start <= end {
                    let conventions = [
                        DayCountConvention::ActualActual365,
                        DayCountConvention::ActualActual360,
                        DayCountConvention::Thirty360,
                    ];

                    for convention in &conventions {
                        let result = convention.year_fraction(start, end);
                        assert!(
                            result >= 0.0,
                            "{:?}.year_fraction({}, {}) = {} should be non-negative",
                            convention, start, end, result
                        );
                    }
                }
            }

            #[test]
            fn test_act_365_vs_act_360_ratio_property(
                start in date_strategy(),
                end in date_strategy(),
            ) {
                // Only test when start < end (avoid division by zero)
                if start < end {
                    let result_365 = DayCountConvention::ActualActual365.year_fraction(start, end);
                    let result_360 = DayCountConvention::ActualActual360.year_fraction(start, end);

                    // Act/360 should be approximately 365/360 times Act/365
                    // result_360 = days / 360, result_365 = days / 365
                    // So result_365 / result_360 = 360 / 365
                    if result_360 > 0.0 {
                        let ratio = result_365 / result_360;
                        assert_relative_eq!(ratio, 360.0 / 365.0, epsilon = 1e-10);
                    }
                }
            }

            #[test]
            fn test_time_to_maturity_matches_act_365_property(
                start in date_strategy(),
                end in date_strategy(),
            ) {
                // Only test when start <= end
                if start <= end {
                    let ttm = time_to_maturity(start, end);
                    let act_365 = DayCountConvention::ActualActual365.year_fraction(start, end);

                    assert_relative_eq!(ttm, act_365, epsilon = 1e-10);
                }
            }

            #[test]
            fn test_year_fraction_is_monotonic(
                start in date_strategy(),
                mid in date_strategy(),
                end in date_strategy(),
            ) {
                // Sort dates to ensure start <= mid <= end
                let mut dates = [start, mid, end];
                dates.sort();
                let [d1, d2, d3] = dates;

                let conventions = [
                    DayCountConvention::ActualActual365,
                    DayCountConvention::ActualActual360,
                    DayCountConvention::Thirty360,
                ];

                for convention in &conventions {
                    let yf_1_2 = convention.year_fraction(d1, d2);
                    let yf_2_3 = convention.year_fraction(d2, d3);
                    let yf_1_3 = convention.year_fraction(d1, d3);

                    // Year fraction should be additive (or close to it)
                    // yf(d1, d3) ≈ yf(d1, d2) + yf(d2, d3)
                    assert_relative_eq!(
                        yf_1_3,
                        yf_1_2 + yf_2_3,
                        epsilon = 0.01, // Small tolerance for 30/360 rounding
                        max_relative = 0.01
                    );
                }
            }

            #[test]
            fn test_same_date_always_returns_zero_property(
                date in date_strategy(),
            ) {
                let conventions = [
                    DayCountConvention::ActualActual365,
                    DayCountConvention::ActualActual360,
                    DayCountConvention::Thirty360,
                ];

                for convention in &conventions {
                    let result = convention.year_fraction(date, date);
                    assert_eq!(
                        result, 0.0,
                        "{:?}.year_fraction({}, {}) should be 0.0",
                        convention, date, date
                    );
                }
            }

            #[test]
            fn test_year_fraction_finite(
                start in date_strategy(),
                end in date_strategy(),
            ) {
                // Only test when start <= end
                if start <= end {
                    let conventions = [
                        DayCountConvention::ActualActual365,
                        DayCountConvention::ActualActual360,
                        DayCountConvention::Thirty360,
                    ];

                    for convention in &conventions {
                        let result = convention.year_fraction(start, end);
                        assert!(
                            result.is_finite(),
                            "{:?}.year_fraction({}, {}) = {} should be finite",
                            convention, start, end, result
                        );
                    }
                }
            }
        }
    }
}
