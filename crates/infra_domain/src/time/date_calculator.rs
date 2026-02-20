//! Date calculator for rolling dates by various units.

use chrono::{Datelike, Months};

use super::calendars::Calendar;
use super::date_adjust::DateAdjustMethod;
use super::schedule::RollConvention;
use super::types::Date;

/// Date calculator: computes an adjusted target date from a base date.
#[derive(Debug, Clone)]
pub enum DateCalculator {
    /// Move by calendar days, then adjust.
    Day {
        /// Number of days to move.
        days: i32,
        /// Adjustment to apply after moving.
        adjust: DateAdjustMethod,
    },
    /// Move by weeks, then adjust.
    Week {
        /// Number of weeks to move.
        weeks: i32,
        /// Adjustment to apply after moving.
        adjust: DateAdjustMethod,
    },
    /// Move by months with roll convention, then adjust.
    Month {
        /// Number of months to move.
        months: i32,
        /// Adjustment to apply after moving.
        adjust: DateAdjustMethod,
        /// Roll convention for day-of-month handling.
        roll_convention: RollConvention,
    },
    /// Move by years with roll convention, then adjust.
    Year {
        /// Number of years to move.
        years: i32,
        /// Adjustment to apply after moving.
        adjust: DateAdjustMethod,
        /// Roll convention for day-of-month handling.
        roll_convention: RollConvention,
    },
    /// Move by business days, then adjust.
    BusinessDay {
        /// Number of business days to move.
        days: i32,
        /// Adjustment to apply after moving.
        adjust: DateAdjustMethod,
    },
}

impl DateCalculator {
    /// Compute the raw (unadjusted) rolled date.
    #[must_use]
    pub fn roll_date(&self, base: Date) -> Date {
        match self {
            Self::Day { days, .. } => base + i64::from(*days),
            Self::Week { weeks, .. } => base + (i64::from(*weeks) * 7),
            Self::Month {
                months,
                roll_convention,
                ..
            } => add_months_with_roll(base, *months, *roll_convention),
            Self::Year {
                years,
                roll_convention,
                ..
            } => add_months_with_roll(base, *years * 12, *roll_convention),
            Self::BusinessDay { days, adjust, .. } => {
                // For BD, extract the calendar from the adjust method.
                if let DateAdjustMethod::BusinessDay { calendar, .. }
                | DateAdjustMethod::LagDays { calendar, .. } = adjust
                {
                    calendar.add_business_days(base, *days)
                } else {
                    // Fallback: use calendar day shift.
                    base + i64::from(*days)
                }
            }
        }
    }

    /// Compute the adjusted date (roll then adjust).
    #[must_use]
    pub fn adjusted_date(&self, base: Date) -> Date {
        let rolled = self.roll_date(base);
        match self {
            Self::Day { adjust, .. }
            | Self::Week { adjust, .. }
            | Self::Month { adjust, .. }
            | Self::Year { adjust, .. }
            | Self::BusinessDay { adjust, .. } => adjust.apply(rolled),
        }
    }
}

/// Add months to a date respecting a roll convention.
fn add_months_with_roll(base: Date, months: i32, convention: RollConvention) -> Date {
    let naive = base.into_inner();
    let shifted = if months >= 0 {
        naive
            .checked_add_months(Months::new(months as u32))
            .unwrap_or(naive)
    } else {
        naive
            .checked_sub_months(Months::new((-months) as u32))
            .unwrap_or(naive)
    };

    let adjusted = match convention {
        RollConvention::Standard => {
            let target_day = base.day();
            let last = last_day(shifted.year(), shifted.month());
            let day = target_day.min(last);
            chrono::NaiveDate::from_ymd_opt(shifted.year(), shifted.month(), day)
                .unwrap_or(shifted)
        }
        RollConvention::Day29th => {
            let last = last_day(shifted.year(), shifted.month());
            let day = 29u32.min(last);
            chrono::NaiveDate::from_ymd_opt(shifted.year(), shifted.month(), day)
                .unwrap_or(shifted)
        }
        RollConvention::Day30th => {
            let last = last_day(shifted.year(), shifted.month());
            let day = 30u32.min(last);
            chrono::NaiveDate::from_ymd_opt(shifted.year(), shifted.month(), day)
                .unwrap_or(shifted)
        }
        RollConvention::EndOfMonth => {
            let last = last_day(shifted.year(), shifted.month());
            chrono::NaiveDate::from_ymd_opt(shifted.year(), shifted.month(), last)
                .unwrap_or(shifted)
        }
    };

    Date::from_naive(adjusted)
}

fn last_day(year: i32, month: u32) -> u32 {
    if month == 12 {
        31
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
            .and_then(|d| d.pred_opt())
            .map_or(31, |d| d.day())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::{BusinessDayConvention, CalendarEnum, CalendarId, ConcreteCalendar};

    fn weekend_cal() -> CalendarEnum {
        CalendarEnum::Concrete(ConcreteCalendar::get(CalendarId::WeekendOnly))
    }

    fn following_adjust() -> DateAdjustMethod {
        DateAdjustMethod::business_day(weekend_cal(), BusinessDayConvention::Following)
    }

    #[test]
    fn test_day_calculator() {
        let calc = DateCalculator::Day {
            days: 5,
            adjust: following_adjust(),
        };
        let base = Date::from_ymd(2026, 1, 5).unwrap(); // Monday
        // +5 days = Saturday -> adjusted to Monday Jan 12
        assert_eq!(
            calc.adjusted_date(base),
            Date::from_ymd(2026, 1, 12).unwrap()
        );
    }

    #[test]
    fn test_week_calculator() {
        let calc = DateCalculator::Week {
            weeks: 1,
            adjust: following_adjust(),
        };
        let base = Date::from_ymd(2026, 1, 5).unwrap(); // Monday
        // +1 week = Monday Jan 12
        assert_eq!(
            calc.adjusted_date(base),
            Date::from_ymd(2026, 1, 12).unwrap()
        );
    }

    #[test]
    fn test_month_calculator_standard() {
        let calc = DateCalculator::Month {
            months: 1,
            adjust: following_adjust(),
            roll_convention: RollConvention::Standard,
        };
        let base = Date::from_ymd(2024, 1, 31).unwrap();
        // +1 month standard = Feb 29 (2024 leap), which is Thursday (BD)
        let result = calc.adjusted_date(base);
        assert_eq!(result, Date::from_ymd(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_month_calculator_eom() {
        let calc = DateCalculator::Month {
            months: 1,
            adjust: following_adjust(),
            roll_convention: RollConvention::EndOfMonth,
        };
        let base = Date::from_ymd(2024, 1, 15).unwrap();
        // +1 month EOM = Feb 29
        let result = calc.adjusted_date(base);
        assert_eq!(result, Date::from_ymd(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_year_calculator() {
        let calc = DateCalculator::Year {
            years: 1,
            adjust: following_adjust(),
            roll_convention: RollConvention::Standard,
        };
        let base = Date::from_ymd(2024, 2, 29).unwrap();
        // +1 year standard = Feb 28 2025 (Friday, BD)
        assert_eq!(
            calc.adjusted_date(base),
            Date::from_ymd(2025, 2, 28).unwrap()
        );
    }

    #[test]
    fn test_business_day_calculator() {
        let calc = DateCalculator::BusinessDay {
            days: 2,
            adjust: following_adjust(),
        };
        let fri = Date::from_ymd(2026, 1, 9).unwrap();
        // +2 BD from Friday = Tuesday Jan 13
        assert_eq!(
            calc.adjusted_date(fri),
            Date::from_ymd(2026, 1, 13).unwrap()
        );
    }

    #[test]
    fn test_roll_date_vs_adjusted() {
        let calc = DateCalculator::Day {
            days: 5,
            adjust: following_adjust(),
        };
        let base = Date::from_ymd(2026, 1, 5).unwrap();
        let rolled = calc.roll_date(base); // Saturday Jan 10
        let adjusted = calc.adjusted_date(base); // Monday Jan 12
        assert_eq!(rolled, Date::from_ymd(2026, 1, 10).unwrap());
        assert_eq!(adjusted, Date::from_ymd(2026, 1, 12).unwrap());
    }
}
