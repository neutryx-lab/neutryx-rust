//! Date types for financial calculations.

use std::{
    fmt,
    ops::{Add, Sub},
    str::FromStr,
};

use chrono::{Datelike, Days, Local, NaiveDate};

use super::error::TimeError;

/// Type-safe date wrapper around chrono::NaiveDate.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Date(NaiveDate);

impl Date {
    /// Creates a Date from year, month, and day components.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Result<Self, TimeError> {
        NaiveDate::from_ymd_opt(year, month, day)
            .map(Date)
            .ok_or(TimeError::InvalidDate { year, month, day })
    }

    /// Returns today's date based on local system time.
    #[must_use]
    pub fn today() -> Self { Date(Local::now().date_naive()) }

    /// Parses a date from ISO 8601 format string (YYYY-MM-DD).
    pub fn parse(s: &str) -> Result<Self, TimeError> {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Date)
            .map_err(|e| TimeError::ParseError(e.to_string()))
    }

    /// Returns the underlying NaiveDate.
    #[must_use]
    pub fn into_inner(self) -> NaiveDate { self.0 }

    /// Returns the year component.
    #[must_use]
    pub fn year(&self) -> i32 { self.0.year() }

    /// Returns the month component (1-12).
    #[must_use]
    pub fn month(&self) -> u32 { self.0.month() }

    /// Returns the day component (1-31).
    #[must_use]
    pub fn day(&self) -> u32 { self.0.day() }

    /// Creates a Date from a NaiveDate.
    #[must_use]
    pub fn from_naive(date: NaiveDate) -> Self { Date(date) }

    /// Convert to Excel serial date (1900-01-01 = 1).
    #[must_use]
    #[allow(clippy::unwrap_used)]
    pub fn to_serial(&self) -> i64 {
        let epoch = NaiveDate::from_ymd_opt(1899, 12, 31).unwrap();
        let days = (self.0 - epoch).num_days();

        if days > 59 {
            days + 1
        } else {
            days
        }
    }

    /// Create a Date from Excel serial number.
    #[allow(clippy::unwrap_used)]
    pub fn from_serial(serial: i64) -> Result<Self, TimeError> {
        if serial < 1 {
            return Err(TimeError::CalculationError(format!(
                "Serial number must be >= 1, got {}",
                serial
            )));
        }

        if serial == 60 {
            return Err(TimeError::CalculationError(
                "Serial 60 represents invalid date 1900-02-29".into(),
            ));
        }

        let epoch = NaiveDate::from_ymd_opt(1899, 12, 31).unwrap();

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
    fn sub(self, other: Self) -> i64 { (self.0 - other.0).num_days() }
}

impl Add<i64> for Date {
    type Output = Date;

    /// Adds a number of days to a date.
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
        let date = Date::from_ymd(2024, 2, 29).unwrap();
        assert_eq!(date.month(), 2);
        assert_eq!(date.day(), 29);
    }

    #[test]
    fn test_date_from_ymd_invalid() {
        let result = Date::from_ymd(2024, 2, 30);
        assert!(result.is_err());

        let result = Date::from_ymd(2024, 13, 1);
        assert!(result.is_err());

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

        let result = Date::parse("2024/06/15");
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
        let d2 = d1;
        let d3 = d1.clone();
        assert_eq!(d1, d2);
        assert_eq!(d1, d3);
    }

    #[test]
    fn test_date_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Date::from_ymd(2024, 1, 1).unwrap());
        set.insert(Date::from_ymd(2024, 12, 31).unwrap());
        set.insert(Date::from_ymd(2024, 1, 1).unwrap());
        assert_eq!(set.len(), 2);
    }

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
        let date = Date::from_ymd(1900, 2, 28).unwrap();
        assert_eq!(date.to_serial(), 59);
    }

    #[test]
    fn test_excel_serial_mar_1_1900() {
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
