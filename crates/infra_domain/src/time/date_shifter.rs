//! Date shifter for single- and multi-calendar adjustments.

use super::{
    calendars::{BusinessDayConvention, Calendar, CalendarEnum},
    types::Date,
};

/// Date shifter supporting single or multiple calendars.
#[derive(Debug, Clone)]
pub enum DateShifter {
    /// Single calendar: a date is a business day only on this calendar.
    All(CalendarEnum),
    /// Multiple calendars: a date is a business day if it is a business day
    /// on ALL of them.
    Any(Vec<CalendarEnum>),
}

impl DateShifter {
    /// Shift a date forward by the given number of business days.
    #[must_use]
    pub fn shift(&self, date: Date, days: i32) -> Date {
        match self {
            Self::All(cal) => cal.add_business_days(date, days),
            Self::Any(cals) => {
                let step: i64 = if days >= 0 { 1 } else { -1 };
                let mut remaining = days.abs();
                let mut current = date;
                while remaining > 0 {
                    current = current + step;
                    if cals.iter().all(|c| c.is_business_day(current)) {
                        remaining -= 1;
                    }
                }
                current
            }
        }
    }

    /// Adjust a date to the nearest business day according to a convention.
    #[must_use]
    pub fn adjust(&self, date: Date, convention: BusinessDayConvention) -> Date {
        match self {
            Self::All(cal) => cal.adjust(date, convention),
            Self::Any(cals) => {
                // Adjust using first calendar, then verify all agree.
                // If not, step further in the same direction.
                match convention {
                    BusinessDayConvention::Unadjusted => date,
                    BusinessDayConvention::Following | BusinessDayConvention::ModifiedFollowing => {
                        let mut d = date;
                        while !cals.iter().all(|c| c.is_business_day(d)) {
                            d = d + 1;
                        }
                        if convention == BusinessDayConvention::ModifiedFollowing
                            && d.month() != date.month()
                        {
                            // Fall back to preceding
                            d = date;
                            while !cals.iter().all(|c| c.is_business_day(d)) {
                                d = d + (-1);
                            }
                        }
                        d
                    }
                    BusinessDayConvention::Preceding | BusinessDayConvention::ModifiedPreceding => {
                        let mut d = date;
                        while !cals.iter().all(|c| c.is_business_day(d)) {
                            d = d + (-1);
                        }
                        if convention == BusinessDayConvention::ModifiedPreceding
                            && d.month() != date.month()
                        {
                            d = date;
                            while !cals.iter().all(|c| c.is_business_day(d)) {
                                d = d + 1;
                            }
                        }
                        d
                    }
                }
            }
        }
    }

    /// Check if a date is a business day on all calendars in this shifter.
    #[must_use]
    pub fn is_business_day(&self, date: Date) -> bool {
        match self {
            Self::All(cal) => cal.is_business_day(date),
            Self::Any(cals) => cals.iter().all(|c| c.is_business_day(date)),
        }
    }

    /// Check if a date is the last business day of its month.
    #[must_use]
    pub fn is_last_business_day_of_month(&self, date: Date) -> bool {
        if !self.is_business_day(date) {
            return false;
        }
        let next = self.shift(date, 1);
        next.month() != date.month()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::{CalendarId, ConcreteCalendar};

    fn weekend_cal() -> CalendarEnum {
        CalendarEnum::Concrete(ConcreteCalendar::get(CalendarId::WeekendOnly))
    }

    fn ny_cal() -> CalendarEnum {
        CalendarEnum::Concrete(ConcreteCalendar::get(CalendarId::NewYork))
    }

    #[test]
    fn test_shift_single() {
        let shifter = DateShifter::All(weekend_cal());
        let fri = Date::from_ymd(2026, 1, 9).unwrap();
        assert_eq!(shifter.shift(fri, 1), Date::from_ymd(2026, 1, 12).unwrap());
    }

    #[test]
    fn test_shift_multi() {
        let shifter = DateShifter::Any(vec![weekend_cal(), ny_cal()]);
        let fri = Date::from_ymd(2026, 1, 9).unwrap();
        assert_eq!(shifter.shift(fri, 1), Date::from_ymd(2026, 1, 12).unwrap());
    }

    #[test]
    fn test_adjust_following() {
        let shifter = DateShifter::All(weekend_cal());
        let sat = Date::from_ymd(2026, 1, 10).unwrap();
        assert_eq!(
            shifter.adjust(sat, BusinessDayConvention::Following),
            Date::from_ymd(2026, 1, 12).unwrap()
        );
    }

    #[test]
    fn test_is_business_day_single() {
        let shifter = DateShifter::All(weekend_cal());
        assert!(shifter.is_business_day(Date::from_ymd(2026, 1, 5).unwrap()));
        assert!(!shifter.is_business_day(Date::from_ymd(2026, 1, 10).unwrap()));
    }

    #[test]
    fn test_is_last_business_day_of_month() {
        let shifter = DateShifter::All(weekend_cal());
        // Jan 30 2026 is Friday — last BD of Jan
        assert!(shifter.is_last_business_day_of_month(Date::from_ymd(2026, 1, 30).unwrap()));
        assert!(!shifter.is_last_business_day_of_month(Date::from_ymd(2026, 1, 29).unwrap()));
    }
}
