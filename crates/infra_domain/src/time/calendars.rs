//! Holiday calendar definitions and abstractions.

use std::{fmt, str::FromStr};

use chrono::{Datelike, NaiveDate, Weekday};

use super::types::Date;

/// Business day adjustment convention.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::AsRefStr)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BusinessDayConvention {
    /// Move to the next business day.
    Following,

    /// Move to the next business day, unless it crosses a month boundary.
    #[strum(serialize = "Modified Following")]
    ModifiedFollowing,

    /// Move to the previous business day.
    Preceding,

    /// Move to the previous business day, unless it crosses a month boundary.
    #[strum(serialize = "Modified Preceding")]
    ModifiedPreceding,

    /// Do not adjust the date.
    Unadjusted,
}

impl BusinessDayConvention {
    /// Returns the standard name for this convention.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str { self.as_ref() }

    /// Returns a short code for this convention.
    #[inline]
    #[must_use]
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

impl FromStr for BusinessDayConvention {
    type Err = String;

    /// Parses business day convention from string (case-insensitive).
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

/// Calendar trait for business day calculations.
pub trait Calendar: Send + Sync {
    /// Check if a date is a business day.
    fn is_business_day(&self, date: Date) -> bool;

    /// Check if a date is a holiday (non-business day).
    fn is_holiday(&self, date: Date) -> bool { !self.is_business_day(date) }

    /// Get the next business day on or after the given date.
    fn next_business_day(&self, mut date: Date) -> Date {
        while !self.is_business_day(date) {
            date = date + 1;
        }
        date
    }

    /// Get the previous business day on or before the given date.
    fn prev_business_day(&self, mut date: Date) -> Date {
        while !self.is_business_day(date) {
            date = date + (-1);
        }
        date
    }

    /// Add business days to a date.
    fn add_business_days(&self, mut date: Date, days: i32) -> Date {
        let step = if days >= 0 { 1i64 } else { -1i64 };
        let mut remaining = days.abs();

        while remaining > 0 {
            date = date + step;
            if self.is_business_day(date) {
                remaining -= 1;
            }
        }
        date
    }

    /// Adjust a date according to a business day convention.
    fn adjust(&self, date: Date, convention: BusinessDayConvention) -> Date {
        match convention {
            BusinessDayConvention::Unadjusted => date,
            BusinessDayConvention::Following => self.next_business_day(date),
            BusinessDayConvention::Preceding => self.prev_business_day(date),
            BusinessDayConvention::ModifiedFollowing => {
                let adjusted = self.next_business_day(date);
                if adjusted.month() != date.month() {
                    self.prev_business_day(date)
                } else {
                    adjusted
                }
            }
            BusinessDayConvention::ModifiedPreceding => {
                let adjusted = self.prev_business_day(date);
                if adjusted.month() != date.month() {
                    self.next_business_day(date)
                } else {
                    adjusted
                }
            }
        }
    }
}

/// Calendar identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CalendarId {
    /// TARGET (Trans-European Automated Real-time Gross Settlement Express.
    Target,
    /// New York.
    NewYork,
    /// Tokyo.
    Tokyo,
    /// London.
    London,
    /// Weekend only (Saturday/Sunday).
    WeekendOnly,
}

/// Concrete calendar implementation.
#[derive(Debug, Clone)]
pub struct ConcreteCalendar {
    id: CalendarId,
}

impl ConcreteCalendar {
    /// Create a calendar by identifier.
    #[must_use]
    pub fn new(id: CalendarId) -> Self { Self { id } }

    /// Get a calendar by identifier (convenience method).
    #[must_use]
    pub fn get(id: CalendarId) -> Self { Self::new(id) }

    /// Returns the calendar identifier.
    #[must_use]
    pub fn id(&self) -> CalendarId { self.id }

    fn is_holiday_internal(&self, date: NaiveDate) -> bool {
        match self.id {
            CalendarId::WeekendOnly => false,
            CalendarId::Target => Self::is_target_holiday(date),
            CalendarId::NewYork => Self::is_ny_holiday(date),
            CalendarId::Tokyo => Self::is_tokyo_holiday(date),
            CalendarId::London => Self::is_london_holiday(date),
        }
    }

    fn is_target_holiday(date: NaiveDate) -> bool {
        let month = date.month();
        let day = date.day();

        matches!((month, day), (1 | 5, 1) | (12, 25 | 26))
    }

    fn is_ny_holiday(date: NaiveDate) -> bool {
        let month = date.month();
        let day = date.day();

        matches!((month, day), (1, 1) | (7, 4) | (12, 25))
    }

    fn is_tokyo_holiday(date: NaiveDate) -> bool {
        let month = date.month();
        let day = date.day();

        matches!((month, day), (1, 1..=3))
    }

    fn is_london_holiday(date: NaiveDate) -> bool {
        let month = date.month();
        let day = date.day();

        matches!((month, day), (1, 1) | (12, 25 | 26))
    }
}

impl Calendar for ConcreteCalendar {
    fn is_business_day(&self, date: Date) -> bool {
        let naive = date.into_inner();

        if matches!(naive.weekday(), Weekday::Sat | Weekday::Sun) {
            return false;
        }

        !self.is_holiday_internal(naive)
    }
}

/// Rule for combining multiple calendars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum JointCalendarRule {
    /// A date is a business day only if ALL calendars agree.
    JoinHolidays,
    /// A date is a business day if ANY calendar says so.
    JoinBusinessDays,
}

/// A calendar that combines multiple calendars.
pub struct JointCalendar {
    calendars: Vec<Box<dyn Calendar>>,
    rule: JointCalendarRule,
}

impl JointCalendar {
    /// Create a new joint calendar.
    pub fn new(calendars: Vec<Box<dyn Calendar>>, rule: JointCalendarRule) -> Self {
        Self { calendars, rule }
    }

    /// Returns the rule used for combining calendars.
    #[must_use]
    pub fn rule(&self) -> JointCalendarRule { self.rule }
}

impl Calendar for JointCalendar {
    fn is_business_day(&self, date: Date) -> bool {
        match self.rule {
            JointCalendarRule::JoinHolidays => {
                self.calendars.iter().all(|c| c.is_business_day(date))
            }
            JointCalendarRule::JoinBusinessDays => {
                self.calendars.iter().any(|c| c.is_business_day(date))
            }
        }
    }
}

impl fmt::Debug for JointCalendar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JointCalendar")
            .field("rule", &self.rule)
            .field("calendar_count", &self.calendars.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_bdc_from_str() {
        assert_eq!(
            "Following".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Following
        );
        assert_eq!(
            "MF".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(
            "modified following"
                .parse::<BusinessDayConvention>()
                .unwrap(),
            BusinessDayConvention::ModifiedFollowing
        );
    }

    #[test]
    fn test_bdc_from_str_invalid() {
        assert!("invalid".parse::<BusinessDayConvention>().is_err());
    }

    #[test]
    fn test_weekend_not_business_day() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        assert!(!calendar.is_business_day(saturday));
        let monday = Date::from_ymd(2026, 1, 5).unwrap();
        assert!(calendar.is_business_day(monday));
    }

    #[test]
    fn test_is_holiday() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        assert!(calendar.is_holiday(saturday));
    }

    #[test]
    fn test_next_business_day() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let monday = calendar.next_business_day(saturday);
        assert_eq!(monday, Date::from_ymd(2026, 1, 12).unwrap());
    }

    #[test]
    fn test_prev_business_day() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let friday = calendar.prev_business_day(saturday);
        assert_eq!(friday, Date::from_ymd(2026, 1, 9).unwrap());
    }

    #[test]
    fn test_add_business_days() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let friday = Date::from_ymd(2026, 1, 9).unwrap();
        let monday = calendar.add_business_days(friday, 1);
        assert_eq!(monday, Date::from_ymd(2026, 1, 12).unwrap());
    }

    #[test]
    fn test_adjust_following() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::Following);
        assert_eq!(adjusted, Date::from_ymd(2026, 1, 12).unwrap());
    }

    #[test]
    fn test_adjust_preceding() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::Preceding);
        assert_eq!(adjusted, Date::from_ymd(2026, 1, 9).unwrap());
    }

    #[test]
    fn test_adjust_unadjusted() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::Unadjusted);
        assert_eq!(adjusted, saturday);
    }

    #[test]
    fn test_adjust_modified_following_same_month() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::ModifiedFollowing);
        assert_eq!(adjusted, Date::from_ymd(2026, 1, 12).unwrap());
    }

    #[test]
    fn test_adjust_modified_following_month_end() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 31).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::ModifiedFollowing);
        assert_eq!(adjusted, Date::from_ymd(2026, 1, 30).unwrap());
    }

    #[test]
    fn test_adjust_modified_preceding_same_month() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let saturday = Date::from_ymd(2026, 1, 10).unwrap();
        let adjusted = calendar.adjust(saturday, BusinessDayConvention::ModifiedPreceding);
        assert_eq!(adjusted, Date::from_ymd(2026, 1, 9).unwrap());
    }

    #[test]
    fn test_adjust_modified_preceding_month_start() {
        let calendar = ConcreteCalendar::get(CalendarId::WeekendOnly);
        let sunday = Date::from_ymd(2026, 2, 1).unwrap();
        let adjusted = calendar.adjust(sunday, BusinessDayConvention::ModifiedPreceding);
        assert_eq!(adjusted, Date::from_ymd(2026, 2, 2).unwrap());
    }

    #[test]
    fn test_target_holiday() {
        let calendar = ConcreteCalendar::get(CalendarId::Target);
        let christmas = Date::from_ymd(2026, 12, 25).unwrap();
        assert!(!calendar.is_business_day(christmas));
    }

    #[test]
    fn test_ny_holiday() {
        let calendar = ConcreteCalendar::get(CalendarId::NewYork);
        let independence = Date::from_ymd(2025, 7, 4).unwrap();
        assert!(!calendar.is_business_day(independence));
    }

    #[test]
    fn test_joint_calendar_join_holidays() {
        let ny = Box::new(ConcreteCalendar::get(CalendarId::NewYork));
        let london = Box::new(ConcreteCalendar::get(CalendarId::London));
        let joint = JointCalendar::new(vec![ny, london], JointCalendarRule::JoinHolidays);

        let monday = Date::from_ymd(2026, 1, 5).unwrap();
        assert!(joint.is_business_day(monday));

        let new_year = Date::from_ymd(2026, 1, 1).unwrap();
        assert!(!joint.is_business_day(new_year));
    }

    #[test]
    fn test_joint_calendar_join_business_days() {
        let weekend = Box::new(ConcreteCalendar::get(CalendarId::WeekendOnly));
        let joint = JointCalendar::new(vec![weekend.clone()], JointCalendarRule::JoinBusinessDays);

        let monday = Date::from_ymd(2026, 1, 5).unwrap();
        assert!(joint.is_business_day(monday));
    }

    #[test]
    fn test_joint_calendar_all_must_agree_for_join_holidays() {
        let target = Box::new(ConcreteCalendar::get(CalendarId::Target));
        let weekend_only = Box::new(ConcreteCalendar::get(CalendarId::WeekendOnly));
        let joint = JointCalendar::new(vec![target, weekend_only], JointCalendarRule::JoinHolidays);

        let labour_day = Date::from_ymd(2026, 5, 1).unwrap();

        assert!(!joint.is_business_day(labour_day));
    }

    #[test]
    fn test_joint_calendar_rule() {
        let ny = Box::new(ConcreteCalendar::get(CalendarId::NewYork));
        let joint = JointCalendar::new(vec![ny], JointCalendarRule::JoinHolidays);
        assert_eq!(joint.rule(), JointCalendarRule::JoinHolidays);
    }

    #[test]
    fn test_joint_calendar_debug() {
        let ny = Box::new(ConcreteCalendar::get(CalendarId::NewYork));
        let joint = JointCalendar::new(vec![ny], JointCalendarRule::JoinHolidays);
        let debug_str = format!("{:?}", joint);
        assert!(debug_str.contains("JointCalendar"));
        assert!(debug_str.contains("JoinHolidays"));
    }
}
