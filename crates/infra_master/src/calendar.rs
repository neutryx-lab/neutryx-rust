//! Holiday calendar definitions.

use chrono::{Datelike, NaiveDate, Weekday};

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
}
