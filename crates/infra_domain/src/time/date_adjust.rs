//! Date adjustment methods for financial instruments.

use super::{
    calendars::{BusinessDayConvention, Calendar, CalendarEnum},
    types::Date,
};

/// Lag type for date adjustments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LagType {
    /// Business day lag (skip non-business days).
    Business,
    /// Calendar day lag (count all days).
    Calendar,
}

/// Date adjustment method.
#[derive(Debug, Clone)]
pub enum DateAdjustMethod {
    /// Adjust to the nearest business day using a convention.
    BusinessDay {
        /// Calendar for business day determination.
        calendar: CalendarEnum,
        /// Convention for adjustment direction.
        convention: BusinessDayConvention,
    },
    /// Shift by a fixed number of days.
    LagDays {
        /// Calendar for business day counting.
        calendar: CalendarEnum,
        /// Number of days to lag (positive = forward).
        lag: i32,
        /// Whether to count business or calendar days.
        lag_type: LagType,
    },
    /// Apply base adjustment, then a follow-up adjustment.
    Composite {
        /// First adjustment to apply.
        base: Box<DateAdjustMethod>,
        /// Second adjustment to apply to the result.
        adjust: Box<DateAdjustMethod>,
    },
}

impl DateAdjustMethod {
    /// Create a business-day adjustment.
    #[must_use]
    pub fn business_day(calendar: CalendarEnum, convention: BusinessDayConvention) -> Self {
        Self::BusinessDay {
            calendar,
            convention,
        }
    }

    /// Create a lag-days adjustment.
    #[must_use]
    pub fn lag_days(calendar: CalendarEnum, lag: i32, lag_type: LagType) -> Self {
        Self::LagDays {
            calendar,
            lag,
            lag_type,
        }
    }

    /// Create a composite adjustment (base then adjust).
    #[must_use]
    pub fn composite(base: DateAdjustMethod, adjust: DateAdjustMethod) -> Self {
        Self::Composite {
            base: Box::new(base),
            adjust: Box::new(adjust),
        }
    }

    /// Apply this adjustment to a date.
    #[must_use]
    pub fn apply(&self, date: Date) -> Date {
        match self {
            Self::BusinessDay {
                calendar,
                convention,
            } => calendar.adjust(date, *convention),
            Self::LagDays {
                calendar,
                lag,
                lag_type,
            } => match lag_type {
                LagType::Calendar => {
                    let shifted = date + i64::from(*lag);
                    calendar.adjust(shifted, BusinessDayConvention::Following)
                }
                LagType::Business => calendar.add_business_days(date, *lag),
            },
            Self::Composite { base, adjust } => {
                let intermediate = base.apply(date);
                adjust.apply(intermediate)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::{CalendarId, ConcreteCalendar};

    fn weekend_cal() -> CalendarEnum {
        CalendarEnum::Concrete(ConcreteCalendar::get(CalendarId::WeekendOnly))
    }

    #[test]
    fn test_business_day_adjust() {
        let adj = DateAdjustMethod::business_day(weekend_cal(), BusinessDayConvention::Following);
        // Saturday -> Monday
        let sat = Date::from_ymd(2026, 1, 10).unwrap();
        assert_eq!(adj.apply(sat), Date::from_ymd(2026, 1, 12).unwrap());
    }

    #[test]
    fn test_lag_days_calendar() {
        let adj = DateAdjustMethod::lag_days(weekend_cal(), 2, LagType::Calendar);
        // Friday + 2 cal days = Sunday -> adjusted to Monday
        let fri = Date::from_ymd(2026, 1, 9).unwrap();
        assert_eq!(adj.apply(fri), Date::from_ymd(2026, 1, 12).unwrap());
    }

    #[test]
    fn test_lag_days_business() {
        let adj = DateAdjustMethod::lag_days(weekend_cal(), 2, LagType::Business);
        // Friday + 2 BD = Tuesday
        let fri = Date::from_ymd(2026, 1, 9).unwrap();
        assert_eq!(adj.apply(fri), Date::from_ymd(2026, 1, 13).unwrap());
    }

    #[test]
    fn test_composite() {
        let base = DateAdjustMethod::lag_days(weekend_cal(), 1, LagType::Calendar);
        let follow =
            DateAdjustMethod::business_day(weekend_cal(), BusinessDayConvention::Following);
        let composite = DateAdjustMethod::composite(base, follow);
        // Friday + 1 cal day = Saturday -> Following = Monday
        let fri = Date::from_ymd(2026, 1, 9).unwrap();
        assert_eq!(composite.apply(fri), Date::from_ymd(2026, 1, 12).unwrap());
    }
}
