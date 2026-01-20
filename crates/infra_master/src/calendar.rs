//! Holiday calendar definitions.
//!
//! This module provides holiday calendars for business day calculations.
//!
//! # Examples
//!
//! ```
//! use infra_master::{Calendar, CalendarId, Date, BusinessDayConvention};
//!
//! let calendar = Calendar::get(CalendarId::Target);
//! let date = Date::from_ymd(2026, 1, 5).unwrap(); // Monday
//! assert!(calendar.is_business_day_date(date));
//! ```

use chrono::{Datelike, NaiveDate, Weekday};

use crate::{BusinessDayConvention, Date};

/// Calendar identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CalendarId {
    /// TARGET (Trans-European Automated Real-time Gross Settlement Express
    /// Transfer)
    Target,
    /// New York
    NewYork,
    /// Tokyo
    Tokyo,
    /// London
    London,
    /// Weekend only (Saturday/Sunday)
    WeekendOnly,
}

/// Holiday calendar for business day calculations.
#[derive(Debug, Clone)]
pub struct Calendar {
    id: CalendarId,
}

impl Calendar {
    /// Get a calendar by identifier.
    #[must_use]
    pub fn get(id: CalendarId) -> Self { Self { id } }

    /// Check if a date is a business day.
    #[must_use]
    pub fn is_business_day(&self, date: NaiveDate) -> bool {
        // Check weekend
        if date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun {
            return false;
        }

        // Check holidays based on calendar
        !self.is_holiday(date)
    }

    /// Check if a date is a holiday (excluding weekends).
    #[must_use]
    pub fn is_holiday(&self, date: NaiveDate) -> bool {
        match self.id {
            CalendarId::WeekendOnly => false,
            CalendarId::Target => Self::is_target_holiday(date),
            CalendarId::NewYork => Self::is_ny_holiday(date),
            CalendarId::Tokyo => Self::is_tokyo_holiday(date),
            CalendarId::London => Self::is_london_holiday(date),
        }
    }

    /// Get the next business day on or after the given date.
    #[must_use]
    pub fn next_business_day(&self, mut date: NaiveDate) -> NaiveDate {
        while !self.is_business_day(date) {
            date = date.succ_opt().unwrap_or(date);
        }
        date
    }

    /// Get the previous business day on or before the given date.
    #[must_use]
    pub fn prev_business_day(&self, mut date: NaiveDate) -> NaiveDate {
        while !self.is_business_day(date) {
            date = date.pred_opt().unwrap_or(date);
        }
        date
    }

    /// Add business days to a date.
    #[must_use]
    pub fn add_business_days(&self, mut date: NaiveDate, days: i32) -> NaiveDate {
        let step = if days >= 0 { 1 } else { -1 };
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

    // =====================================================================
    // Date-based API (type-safe wrappers)
    // =====================================================================

    /// Check if a date is a business day (using Date type).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::{Calendar, CalendarId, Date};
    ///
    /// let calendar = Calendar::get(CalendarId::Target);
    /// let monday = Date::from_ymd(2026, 1, 5).unwrap();
    /// assert!(calendar.is_business_day_date(monday));
    /// ```
    #[must_use]
    pub fn is_business_day_date(&self, date: Date) -> bool {
        self.is_business_day(date.into_inner())
    }

    /// Get the next business day on or after the given date (using Date type).
    #[must_use]
    pub fn next_business_day_date(&self, date: Date) -> Date {
        Date::from_naive(self.next_business_day(date.into_inner()))
    }

    /// Get the previous business day on or before the given date (using Date
    /// type).
    #[must_use]
    pub fn prev_business_day_date(&self, date: Date) -> Date {
        Date::from_naive(self.prev_business_day(date.into_inner()))
    }

    /// Add business days to a date (using Date type).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::{Calendar, CalendarId, Date};
    ///
    /// let calendar = Calendar::get(CalendarId::WeekendOnly);
    /// let friday = Date::from_ymd(2026, 1, 9).unwrap();
    /// let monday = calendar.add_business_days_date(friday, 1);
    /// assert_eq!(monday, Date::from_ymd(2026, 1, 12).unwrap());
    /// ```
    #[must_use]
    pub fn add_business_days_date(&self, date: Date, days: i32) -> Date {
        Date::from_naive(self.add_business_days(date.into_inner(), days))
    }

    /// Adjust a date according to a business day convention.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::{Calendar, CalendarId, Date, BusinessDayConvention};
    ///
    /// let calendar = Calendar::get(CalendarId::WeekendOnly);
    /// let saturday = Date::from_ymd(2026, 1, 10).unwrap();
    ///
    /// // Following: move to next business day (Monday)
    /// let adjusted = calendar.adjust(saturday, BusinessDayConvention::Following);
    /// assert_eq!(adjusted, Date::from_ymd(2026, 1, 12).unwrap());
    ///
    /// // Preceding: move to previous business day (Friday)
    /// let adjusted = calendar.adjust(saturday, BusinessDayConvention::Preceding);
    /// assert_eq!(adjusted, Date::from_ymd(2026, 1, 9).unwrap());
    /// ```
    #[must_use]
    pub fn adjust(&self, date: Date, convention: BusinessDayConvention) -> Date {
        match convention {
            BusinessDayConvention::Unadjusted => date,
            BusinessDayConvention::Following => self.next_business_day_date(date),
            BusinessDayConvention::Preceding => self.prev_business_day_date(date),
            BusinessDayConvention::ModifiedFollowing => {
                let adjusted = self.next_business_day_date(date);
                // If month changed, use preceding instead
                if adjusted.month() != date.month() {
                    self.prev_business_day_date(date)
                } else {
                    adjusted
                }
            }
            BusinessDayConvention::ModifiedPreceding => {
                let adjusted = self.prev_business_day_date(date);
                // If month changed, use following instead
                if adjusted.month() != date.month() {
                    self.next_business_day_date(date)
                } else {
                    adjusted
                }
            }
        }
    }

    // TARGET calendar holidays (simplified)
    fn is_target_holiday(date: NaiveDate) -> bool {
        let month = date.month();
        let day = date.day();

        // Fixed holidays: New Year's Day, Labour Day, Christmas Day, Boxing Day
        matches!((month, day), (1 | 5, 1) | (12, 25 | 26))
    }

    // New York calendar holidays (simplified)
    fn is_ny_holiday(date: NaiveDate) -> bool {
        let month = date.month();
        let day = date.day();

        // New Year's Day, Independence Day, Christmas Day
        matches!((month, day), (1, 1) | (7, 4) | (12, 25))
    }

    // Tokyo calendar holidays (simplified)
    fn is_tokyo_holiday(date: NaiveDate) -> bool {
        let month = date.month();
        let day = date.day();

        // Fixed holidays: New Year's Day, Bank Holidays (days 1-3 of January)
        matches!((month, day), (1, 1..=3))
    }

    // London calendar holidays (simplified)
    fn is_london_holiday(date: NaiveDate) -> bool {
        let month = date.month();
        let day = date.day();

        // Fixed holidays: New Year's Day, Christmas Day, Boxing Day
        matches!((month, day), (1, 1) | (12, 25 | 26))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weekend_not_business_day() {
        let calendar = Calendar::get(CalendarId::WeekendOnly);
        // Saturday
        let saturday = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        assert!(!calendar.is_business_day(saturday));
        // Monday
        let monday = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        assert!(calendar.is_business_day(monday));
    }

    #[test]
    fn test_add_business_days() {
        let calendar = Calendar::get(CalendarId::WeekendOnly);
        let friday = NaiveDate::from_ymd_opt(2026, 1, 9).unwrap();
        let monday = calendar.add_business_days(friday, 1);
        assert_eq!(monday, NaiveDate::from_ymd_opt(2026, 1, 12).unwrap());
    }

    #[test]
    fn test_is_business_day_date() {
        let calendar = Calendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let monday = Date::from_ymd(2026, 1, 5).unwrap();

        assert!(!calendar.is_business_day_date(saturday));
        assert!(calendar.is_business_day_date(monday));
    }

    #[test]
    fn test_add_business_days_date() {
        let calendar = Calendar::get(CalendarId::WeekendOnly);
        let friday = Date::from_ymd(2026, 1, 9).unwrap();
        let monday = calendar.add_business_days_date(friday, 1);
        assert_eq!(monday, Date::from_ymd(2026, 1, 12).unwrap());
    }

    #[test]
    fn test_adjust_following() {
        let calendar = Calendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::Following);
        assert_eq!(adjusted, Date::from_ymd(2026, 1, 12).unwrap()); // Monday
    }

    #[test]
    fn test_adjust_preceding() {
        let calendar = Calendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::Preceding);
        assert_eq!(adjusted, Date::from_ymd(2026, 1, 9).unwrap()); // Friday
    }

    #[test]
    fn test_adjust_unadjusted() {
        let calendar = Calendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::Unadjusted);
        assert_eq!(adjusted, saturday);
    }

    #[test]
    fn test_adjust_modified_following_same_month() {
        let calendar = Calendar::get(CalendarId::WeekendOnly);
        // Saturday Jan 10 -> Monday Jan 12 (same month)
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::ModifiedFollowing);
        assert_eq!(adjusted, Date::from_ymd(2026, 1, 12).unwrap());
    }

    #[test]
    fn test_adjust_modified_following_month_end() {
        let calendar = Calendar::get(CalendarId::WeekendOnly);
        // Saturday Jan 31, 2026 - next business day is Feb 2 (crosses month)
        // Should use preceding instead -> Friday Jan 30
        let saturday = Date::from_ymd(2026, 1, 31).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::ModifiedFollowing);
        assert_eq!(adjusted, Date::from_ymd(2026, 1, 30).unwrap());
    }

    #[test]
    fn test_adjust_modified_preceding_same_month() {
        let calendar = Calendar::get(CalendarId::WeekendOnly);
        // Saturday Jan 10 -> Friday Jan 9 (same month)
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::ModifiedPreceding);
        assert_eq!(adjusted, Date::from_ymd(2026, 1, 9).unwrap());
    }

    #[test]
    fn test_adjust_modified_preceding_month_start() {
        let calendar = Calendar::get(CalendarId::WeekendOnly);
        // Sunday Feb 1, 2026 - prev business day is Jan 30 (crosses month)
        // Should use following instead -> Monday Feb 2
        let sunday = Date::from_ymd(2026, 2, 1).unwrap();
        let adjusted = calendar.adjust(sunday, BusinessDayConvention::ModifiedPreceding);
        assert_eq!(adjusted, Date::from_ymd(2026, 2, 2).unwrap());
    }
}
