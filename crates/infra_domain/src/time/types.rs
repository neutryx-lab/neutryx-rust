//! Date types for financial calculations.
//!
//! This module provides a type-safe date wrapper around chrono::NaiveDate
//! with Excel serial date conversion support.
//!
//! # Examples
//!
//! ```
//! use infra_master::time::Date;
//!
//! let date = Date::from_ymd(2024, 6, 15).unwrap();
//! assert_eq!(date.year(), 2024);
//! assert_eq!(date.month(), 6);
//! assert_eq!(date.day(), 15);
//!
//! // Excel serial conversion
//! let serial = date.to_serial();
//! let restored = Date::from_serial(serial).unwrap();
//! assert_eq!(date, restored);
//! ```

use std::{
    fmt,
    ops::{Add, Sub},
    str::FromStr,
};

use chrono::{Datelike, Days, Local, NaiveDate};

use super::error::TimeError;

/// Type-safe date wrapper around chrono::NaiveDate.
///
/// Provides ISO 8601 serialisation, standard date arithmetic,
/// and Excel serial date conversion. This wrapper ensures type safety
/// and provides a consistent API for date operations in financial calculations.
///
/// # Examples
///
/// ```
/// use infra_master::time::Date;
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
///
/// // Excel serial conversion
/// let serial = date.to_serial();
/// assert!(serial > 0);
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
    /// `Ok(Date)` if the date is valid, `Err(TimeError::InvalidDate)`
    /// otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Date;
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
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Result<Self, TimeError> {
        NaiveDate::from_ymd_opt(year, month, day)
            .map(Date)
            .ok_or(TimeError::InvalidDate { year, month, day })
    }

    /// Returns today's date based on local system time.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Date;
    ///
    /// let today = Date::today();
    /// // today is the current local date
    /// ```
    #[must_use]
    pub fn today() -> Self { Date(Local::now().date_naive()) }

    /// Parses a date from ISO 8601 format string (YYYY-MM-DD).
    ///
    /// # Arguments
    /// * `s` - Date string in ISO 8601 format
    ///
    /// # Returns
    /// `Ok(Date)` if parsing succeeds, `Err(TimeError::ParseError)` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Date;
    ///
    /// let date = Date::parse("2024-06-15").unwrap();
    /// assert_eq!(date.year(), 2024);
    ///
    /// let invalid = Date::parse("not-a-date");
    /// assert!(invalid.is_err());
    /// ```
    pub fn parse(s: &str) -> Result<Self, TimeError> {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Date)
            .map_err(|e| TimeError::ParseError(e.to_string()))
    }

    /// Returns the underlying NaiveDate.
    ///
    /// Use this method when you need access to chrono's full API.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Date;
    /// use chrono::Datelike;
    ///
    /// let date = Date::from_ymd(2024, 6, 15).unwrap();
    /// let naive = date.into_inner();
    /// assert_eq!(naive.weekday(), chrono::Weekday::Sat);
    /// ```
    #[must_use]
    pub fn into_inner(self) -> NaiveDate { self.0 }

    /// Returns the year component.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Date;
    ///
    /// let date = Date::from_ymd(2024, 6, 15).unwrap();
    /// assert_eq!(date.year(), 2024);
    /// ```
    #[must_use]
    pub fn year(&self) -> i32 { self.0.year() }

    /// Returns the month component (1-12).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Date;
    ///
    /// let date = Date::from_ymd(2024, 6, 15).unwrap();
    /// assert_eq!(date.month(), 6);
    /// ```
    #[must_use]
    pub fn month(&self) -> u32 { self.0.month() }

    /// Returns the day component (1-31).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Date;
    ///
    /// let date = Date::from_ymd(2024, 6, 15).unwrap();
    /// assert_eq!(date.day(), 15);
    /// ```
    #[must_use]
    pub fn day(&self) -> u32 { self.0.day() }

    /// Creates a Date from a NaiveDate.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Date;
    /// use chrono::NaiveDate;
    ///
    /// let naive = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    /// let date = Date::from_naive(naive);
    /// assert_eq!(date.year(), 2024);
    /// ```
    #[must_use]
    pub fn from_naive(date: NaiveDate) -> Self { Date(date) }

    /// Convert to Excel serial date (1900-01-01 = 1).
    ///
    /// Accounts for Excel's leap year bug where 1900 is incorrectly
    /// treated as a leap year. Serial 60 would represent the non-existent
    /// date 1900-02-29, so dates after 1900-02-28 are incremented by 1.
    ///
    /// # Reference
    /// - 1900-01-01 = 1
    /// - 1900-02-28 = 59
    /// - 1900-03-01 = 61 (skipping 60 due to the bug)
    /// - 2024-01-01 = 45292
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Date;
    ///
    /// let date = Date::from_ymd(2024, 1, 1).unwrap();
    /// assert_eq!(date.to_serial(), 45292);
    ///
    /// // Epoch
    /// let epoch = Date::from_ymd(1900, 1, 1).unwrap();
    /// assert_eq!(epoch.to_serial(), 1);
    /// ```
    #[must_use]
    #[allow(clippy::unwrap_used)] // Constant date (1899-12-31) is guaranteed valid
    pub fn to_serial(&self) -> i64 {
        // Excel epoch: 1899-12-31 (day 0), so 1900-01-01 = 1
        let epoch = NaiveDate::from_ymd_opt(1899, 12, 31).unwrap();
        let days = (self.0 - epoch).num_days();

        // Excel leap year bug: dates after 1900-02-28 are +1
        // Serial 59 = 1900-02-28, Serial 60 = 1900-02-29 (invalid)
        if days > 59 {
            days + 1
        } else {
            days
        }
    }

    /// Create a Date from Excel serial number.
    ///
    /// # Arguments
    /// * `serial` - Excel serial date (1 = 1900-01-01)
    ///
    /// # Errors
    /// Returns `TimeError::CalculationError` if:
    /// - Serial is less than 1
    /// - Serial is 60 (represents the non-existent date 1900-02-29)
    /// - Serial is out of valid date range
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Date;
    ///
    /// let date = Date::from_serial(45292).unwrap();
    /// assert_eq!(date, Date::from_ymd(2024, 1, 1).unwrap());
    ///
    /// // Invalid serial numbers
    /// assert!(Date::from_serial(0).is_err());
    /// assert!(Date::from_serial(-1).is_err());
    /// assert!(Date::from_serial(60).is_err()); // 1900-02-29 doesn't exist
    /// ```
    #[allow(clippy::unwrap_used)] // Constant date (1899-12-31) is guaranteed valid
    pub fn from_serial(serial: i64) -> Result<Self, TimeError> {
        if serial < 1 {
            return Err(TimeError::CalculationError(format!(
                "Serial number must be >= 1, got {}",
                serial
            )));
        }

        // Excel leap year bug: serial 60 = 1900-02-29 (invalid)
        if serial == 60 {
            return Err(TimeError::CalculationError(
                "Serial 60 represents invalid date 1900-02-29".into(),
            ));
        }

        let epoch = NaiveDate::from_ymd_opt(1899, 12, 31).unwrap();

        // Adjust for Excel leap year bug
        let adjusted = if serial > 60 { serial - 1 } else { serial };

        epoch
            .checked_add_days(Days::new(adjusted as u64))
            .map(Date::from_naive)
            .ok_or_else(|| {
                TimeError::CalculationError(format!("Serial {} out of valid date range", serial))
            })
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
    /// use infra_master::time::Date;
    ///
    /// let start = Date::from_ymd(2024, 1, 1).unwrap();
    /// let end = Date::from_ymd(2024, 1, 11).unwrap();
    ///
    /// assert_eq!(end - start, 10);
    /// assert_eq!(start - end, -10);
    /// ```
    fn sub(self, other: Self) -> i64 { (self.0 - other.0).num_days() }
}

impl Add<i64> for Date {
    type Output = Date;

    /// Adds a number of days to a date.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Date;
    ///
    /// let start = Date::from_ymd(2024, 1, 1).unwrap();
    /// let end = start + 10;
    /// assert_eq!(end, Date::from_ymd(2024, 1, 11).unwrap());
    ///
    /// // Negative days subtract
    /// let before = start + (-5);
    /// assert_eq!(before, Date::from_ymd(2023, 12, 27).unwrap());
    /// ```
    fn add(self, days: i64) -> Date {
        if days >= 0 {
            Date(self.0 + Days::new(days as u64))
        } else {
            Date(self.0 - Days::new((-days) as u64))
        }
    }
}

impl FromStr for Date {
    type Err = TimeError;

    /// Parses a date from ISO 8601 format string (YYYY-MM-DD).
    fn from_str(s: &str) -> Result<Self, TimeError> { Date::parse(s) }
}

impl fmt::Display for Date {
    /// Formats the date as ISO 8601 (YYYY-MM-DD).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d"))
    }
}

impl From<NaiveDate> for Date {
    fn from(date: NaiveDate) -> Self { Date(date) }
}

impl From<Date> for NaiveDate {
    fn from(date: Date) -> Self { date.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_date_addition() {
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = start + 10;
        assert_eq!(end, Date::from_ymd(2024, 1, 11).unwrap());

        // Negative days subtract
        let before = start + (-5);
        assert_eq!(before, Date::from_ymd(2023, 12, 27).unwrap());
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

    #[test]
    fn test_date_from_naive() {
        let naive = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let date = Date::from_naive(naive);
        assert_eq!(date.year(), 2024);
    }

    #[test]
    fn test_date_from_into_conversion() {
        let naive = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let date: Date = naive.into();
        let back: NaiveDate = date.into();
        assert_eq!(naive, back);
    }

    #[test]
    fn test_date_copy_clone() {
        let d1 = Date::from_ymd(2024, 6, 15).unwrap();
        let d2 = d1; // Copy
        let d3 = d1.clone(); // Clone
        assert_eq!(d1, d2);
        assert_eq!(d1, d3);
    }

    #[test]
    fn test_date_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Date::from_ymd(2024, 1, 1).unwrap());
        set.insert(Date::from_ymd(2024, 12, 31).unwrap());
        set.insert(Date::from_ymd(2024, 1, 1).unwrap()); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // Excel serial conversion tests

    #[test]
    fn test_excel_serial_epoch() {
        let date = Date::from_ymd(1900, 1, 1).unwrap();
        assert_eq!(date.to_serial(), 1);
    }

    #[test]
    fn test_excel_serial_jan_2() {
        let date = Date::from_ymd(1900, 1, 2).unwrap();
        assert_eq!(date.to_serial(), 2);
    }

    #[test]
    fn test_excel_serial_feb_28_1900() {
        // Last valid date before the leap year bug
        let date = Date::from_ymd(1900, 2, 28).unwrap();
        assert_eq!(date.to_serial(), 59);
    }

    #[test]
    fn test_excel_serial_mar_1_1900() {
        // First date after the leap year bug (skipping 60)
        let date = Date::from_ymd(1900, 3, 1).unwrap();
        assert_eq!(date.to_serial(), 61);
    }

    #[test]
    fn test_excel_serial_modern_date() {
        let date = Date::from_ymd(2024, 1, 1).unwrap();
        assert_eq!(date.to_serial(), 45292);
    }

    #[test]
    fn test_excel_serial_roundtrip() {
        let original = Date::from_ymd(2024, 1, 1).unwrap();
        let serial = original.to_serial();
        let restored = Date::from_serial(serial).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_excel_serial_roundtrip_various_dates() {
        let test_dates = [
            (1900, 1, 1),
            (1900, 2, 28),
            (1900, 3, 1),
            (2000, 1, 1),
            (2024, 6, 15),
            (2050, 12, 31),
        ];

        for (year, month, day) in test_dates {
            let original = Date::from_ymd(year, month, day).unwrap();
            let serial = original.to_serial();
            let restored = Date::from_serial(serial).unwrap();
            assert_eq!(
                original, restored,
                "Roundtrip failed for {}-{:02}-{:02}",
                year, month, day
            );
        }
    }

    #[test]
    fn test_from_serial_invalid_zero() {
        let result = Date::from_serial(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_serial_invalid_negative() {
        let result = Date::from_serial(-1);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_serial_invalid_60() {
        // Serial 60 represents the non-existent 1900-02-29
        let result = Date::from_serial(60);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_serial_valid_59() {
        let date = Date::from_serial(59).unwrap();
        assert_eq!(date, Date::from_ymd(1900, 2, 28).unwrap());
    }

    #[test]
    fn test_from_serial_valid_61() {
        let date = Date::from_serial(61).unwrap();
        assert_eq!(date, Date::from_ymd(1900, 3, 1).unwrap());
    }
}
