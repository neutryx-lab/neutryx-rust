//! Day count convention definitions.

use std::str::FromStr;

use chrono::{Datelike, NaiveDate};

use super::{
    calendars::{Calendar, CalendarEnum, CalendarId, ConcreteCalendar},
    frequency::Frequency,
    types::Date,
};

/// Day count convention for interest calculations.
#[non_exhaustive]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    strum::Display,
    strum::AsRefStr,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum DayCounter {
    /// Actual/360.
    #[strum(serialize = "ACT/360")]
    Actual360,

    /// Actual/365 Fixed.
    #[default]
    #[strum(serialize = "ACT/365")]
    Actual365Fixed,

    /// Actual/365.25.
    #[strum(serialize = "ACT/365.25")]
    Actual36525,

    /// Actual/Actual (ISDA).
    #[strum(serialize = "ACT/ACT ISDA")]
    ActualActualIsda,

    /// Actual/Actual (ICMA).
    #[strum(serialize = "ACT/ACT ICMA")]
    ActualActualIcma,

    /// 30/360 (Bond Basis).
    #[strum(serialize = "30/360")]
    Thirty360Bond,

    /// 30/360 (European).
    #[strum(serialize = "30E/360")]
    Thirty360European,

    /// 30E/360 (ISDA).
    #[strum(serialize = "30E/360 ISDA")]
    ThirtyE360Isda,

    /// Business/252.
    #[strum(serialize = "BUS/252")]
    Bus252,
}

/// Returns `true` if the given year is a leap year.
fn is_leap_year(year: i32) -> bool { (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 }

/// Days in year (365 or 366).
fn days_in_year(year: i32) -> f64 {
    if is_leap_year(year) {
        366.0
    } else {
        365.0
    }
}

impl DayCounter {
    /// Returns the standard convention name.
    #[must_use]
    pub fn name(&self) -> &str { self.as_ref() }

    /// Returns the year fraction for a given number of calendar days.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn year_fraction_from_days(self, days: i64) -> f64 {
        match self {
            Self::Actual360 | Self::Bus252 => days as f64 / 360.0,
            Self::Actual365Fixed => days as f64 / 365.0,
            Self::Actual36525 => days as f64 / 365.25,
            Self::ActualActualIsda | Self::ActualActualIcma => days as f64 / 365.25,
            Self::Thirty360Bond | Self::Thirty360European | Self::ThirtyE360Isda => {
                days as f64 / 360.0
            }
        }
    }

    /// Calculate the year fraction between two dates.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn year_fraction(self, start: Date, end: Date) -> f64 {
        let days = end - start;

        match self {
            Self::Actual360 => days as f64 / 360.0,
            Self::Actual365Fixed => days as f64 / 365.0,
            Self::Actual36525 => days as f64 / 365.25,
            Self::ActualActualIsda => Self::act_act_isda(start, end),
            Self::ActualActualIcma => {
                // Without reference period info, fall back to ISDA.
                Self::act_act_isda(start, end)
            }
            Self::Bus252 => {
                // Without calendar, use WeekendOnly as fallback.
                let cal = CalendarEnum::Concrete(ConcreteCalendar::get(CalendarId::WeekendOnly));
                self.year_fraction_with_calendar(start, end, &cal)
            }
            Self::Thirty360Bond | Self::Thirty360European | Self::ThirtyE360Isda => {
                let (start_inner, end_inner, sign) = if start <= end {
                    (start.into_inner(), end.into_inner(), 1.0)
                } else {
                    (end.into_inner(), start.into_inner(), -1.0)
                };
                sign * Self::thirty_360_days(self, start_inner, end_inner) / 360.0
            }
        }
    }

    /// Calculate the number of days between two dates.
    #[must_use]
    pub fn day_count(self, start: Date, end: Date) -> i64 { end - start }

    /// Year fraction for ACT/ACT ICMA with reference period.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn year_fraction_icma(
        self,
        start: Date,
        end: Date,
        ref_start: Date,
        ref_end: Date,
        frequency: Frequency,
    ) -> f64 {
        let periods = frequency.periods_per_year();
        if periods == 0 {
            return Self::act_act_isda(start, end);
        }
        let ref_days = ref_end - ref_start;
        if ref_days <= 0 {
            return Self::act_act_isda(start, end);
        }
        let actual_days = end - start;
        actual_days as f64 / (ref_days as f64 * periods as f64)
    }

    /// Year fraction for BUS/252 with a specific calendar.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn year_fraction_with_calendar(
        self,
        start: Date,
        end: Date,
        calendar: &CalendarEnum,
    ) -> f64 {
        if start >= end {
            return 0.0;
        }
        let bd = calendar.count_business_days(start, end);
        bd as f64 / 252.0
    }

    /// Year fraction for 30E/360 ISDA with maturity flag.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub fn year_fraction_e360_isda(self, start: Date, end: Date, is_maturity: bool) -> f64 {
        let (s, e, sign) = if start <= end {
            (start.into_inner(), end.into_inner(), 1.0)
        } else {
            (end.into_inner(), start.into_inner(), -1.0)
        };
        let (y1, m1, mut d1) = (s.year(), s.month() as i32, s.day() as i32);
        let (y2, m2, mut d2) = (e.year(), e.month() as i32, e.day() as i32);

        if d1 == last_day(y1, m1 as u32) as i32 {
            d1 = 30;
        }
        if is_maturity && d2 == last_day(y2, m2 as u32) as i32 {
            d2 = 30;
        } else {
            d2 = d2.min(30);
        }

        sign * f64::from(360 * (y2 - y1) + 30 * (m2 - m1) + (d2 - d1)) / 360.0
    }

    /// Correct ACT/ACT ISDA implementation: split at year boundaries.
    #[allow(clippy::cast_precision_loss)]
    fn act_act_isda(start: Date, end: Date) -> f64 {
        if start == end {
            return 0.0;
        }
        let (s, e, sign) = if start <= end {
            (start, end, 1.0)
        } else {
            (end, start, -1.0)
        };

        let y1 = s.year();
        let y2 = e.year();

        if y1 == y2 {
            return sign * (e - s) as f64 / days_in_year(y1);
        }

        // Fraction in first year
        let jan1_next = Date::from_ymd(y1 + 1, 1, 1).unwrap_or(s);
        let mut total = (jan1_next - s) as f64 / days_in_year(y1);

        // Full intermediate years
        for y in (y1 + 1)..y2 {
            total += 1.0;
            let _ = y; // each full year contributes 1.0
        }

        // Fraction in last year
        let jan1_last = Date::from_ymd(y2, 1, 1).unwrap_or(e);
        total += (e - jan1_last) as f64 / days_in_year(y2);

        sign * total
    }

    /// Calculate 30/360 day count.
    #[allow(clippy::cast_possible_wrap)]
    fn thirty_360_days(self, start: NaiveDate, end: NaiveDate) -> f64 {
        let (y1, m1, d1) = (start.year(), start.month() as i32, start.day() as i32);
        let (y2, m2, d2) = (end.year(), end.month() as i32, end.day() as i32);

        let (d1_adj, d2_adj) = match self {
            Self::Thirty360Bond => {
                let d1_adj = d1.min(30);
                let d2_adj = if d1_adj == 30 { d2.min(30) } else { d2 };
                (d1_adj, d2_adj)
            }
            Self::Thirty360European | Self::ThirtyE360Isda => (d1.min(30), d2.min(30)),
            _ => (d1, d2),
        };

        f64::from(360 * (y2 - y1) + 30 * (m2 - m1) + (d2_adj - d1_adj))
    }
}

/// Last day of month for given year/month.
fn last_day(year: i32, month: u32) -> u32 {
    if month == 12 {
        31
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .and_then(|d| d.pred_opt())
            .map_or(31, |d| d.day())
    }
}

impl FromStr for DayCounter {
    type Err = String;

    /// Parses day count convention from string (case-insensitive).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().replace(['/', ' ', '.', '_'], "").as_str() {
            "ACT360" | "ACTUAL360" | "A360" => Ok(DayCounter::Actual360),
            "ACT365" | "ACTUAL365" | "A365" | "ACT365FIXED" | "ACTUAL365FIXED" => {
                Ok(DayCounter::Actual365Fixed)
            }
            "ACT36525" | "ACTUAL36525" => Ok(DayCounter::Actual36525),
            "ACTACTISDA" | "ACTUALACTUALISDA" | "ACTACT" => Ok(DayCounter::ActualActualIsda),
            "ACTACTICMA" | "ACTUALACTUALICMA" => Ok(DayCounter::ActualActualIcma),
            "30360" | "THIRTY360" | "30360BOND" => Ok(DayCounter::Thirty360Bond),
            "30E360" | "30360EUROPEAN" => Ok(DayCounter::Thirty360European),
            "30E360ISDA" => Ok(DayCounter::ThirtyE360Isda),
            "BUS252" | "BUSINESS252" => Ok(DayCounter::Bus252),
            _ => Err(format!("Unknown day count convention: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actual_360() {
        let start = Date::from_ymd(2026, 1, 1).unwrap();
        let end = Date::from_ymd(2026, 4, 1).unwrap();
        let yf = DayCounter::Actual360.year_fraction(start, end);
        assert!((yf - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_actual_365() {
        let start = Date::from_ymd(2026, 1, 1).unwrap();
        let end = Date::from_ymd(2027, 1, 1).unwrap();
        let yf = DayCounter::Actual365Fixed.year_fraction(start, end);
        assert!((yf - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_name() {
        assert_eq!(DayCounter::Actual360.name(), "ACT/360");
        assert_eq!(DayCounter::Actual365Fixed.name(), "ACT/365");
        assert_eq!(DayCounter::Actual36525.name(), "ACT/365.25");
        assert_eq!(DayCounter::ActualActualIsda.name(), "ACT/ACT ISDA");
        assert_eq!(DayCounter::ActualActualIcma.name(), "ACT/ACT ICMA");
        assert_eq!(DayCounter::Thirty360Bond.name(), "30/360");
        assert_eq!(DayCounter::Thirty360European.name(), "30E/360");
        assert_eq!(DayCounter::ThirtyE360Isda.name(), "30E/360 ISDA");
        assert_eq!(DayCounter::Bus252.name(), "BUS/252");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", DayCounter::Actual365Fixed), "ACT/365");
        assert_eq!(format!("{}", DayCounter::Thirty360Bond), "30/360");
        assert_eq!(format!("{}", DayCounter::Bus252), "BUS/252");
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            "ACT/365".parse::<DayCounter>().unwrap(),
            DayCounter::Actual365Fixed
        );
        assert_eq!(
            "act/360".parse::<DayCounter>().unwrap(),
            DayCounter::Actual360
        );
        assert_eq!(
            "30/360".parse::<DayCounter>().unwrap(),
            DayCounter::Thirty360Bond
        );
        assert_eq!(
            "Thirty360".parse::<DayCounter>().unwrap(),
            DayCounter::Thirty360Bond
        );
        assert_eq!(
            "30E/360".parse::<DayCounter>().unwrap(),
            DayCounter::Thirty360European
        );
        assert_eq!(
            "ACT/ACT ICMA".parse::<DayCounter>().unwrap(),
            DayCounter::ActualActualIcma
        );
        assert_eq!("BUS/252".parse::<DayCounter>().unwrap(), DayCounter::Bus252);
    }

    #[test]
    fn test_from_str_invalid() {
        let result = "INVALID".parse::<DayCounter>();
        assert!(result.is_err());
    }

    #[test]
    fn test_year_fraction_dates() {
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = Date::from_ymd(2024, 7, 1).unwrap();

        let yf = DayCounter::Actual365Fixed.year_fraction(start, end);
        assert!((yf - 182.0 / 365.0).abs() < 1e-10);
    }

    #[test]
    fn test_year_fraction_dates_negative() {
        let start = Date::from_ymd(2024, 7, 1).unwrap();
        let end = Date::from_ymd(2024, 1, 1).unwrap();

        let yf = DayCounter::Actual365Fixed.year_fraction(start, end);
        assert!(yf < 0.0);
        assert!((yf + 182.0 / 365.0).abs() < 1e-10);
    }

    #[test]
    fn test_same_date_returns_zero() {
        let date = Date::from_ymd(2024, 6, 15).unwrap();

        for dcc in [
            DayCounter::Actual360,
            DayCounter::Actual365Fixed,
            DayCounter::Thirty360Bond,
            DayCounter::ActualActualIsda,
        ] {
            assert_eq!(dcc.year_fraction(date, date), 0.0);
        }
    }

    #[test]
    fn test_thirty_360_bond() {
        let start = Date::from_ymd(2024, 1, 31).unwrap();
        let end = Date::from_ymd(2024, 3, 31).unwrap();

        let yf = DayCounter::Thirty360Bond.year_fraction(start, end);
        assert!((yf - 60.0 / 360.0).abs() < 1e-10);
    }

    #[test]
    fn test_thirty_360_european() {
        let start = Date::from_ymd(2024, 1, 31).unwrap();
        let end = Date::from_ymd(2024, 3, 31).unwrap();

        let yf = DayCounter::Thirty360European.year_fraction(start, end);
        assert!((yf - 60.0 / 360.0).abs() < 1e-10);
    }

    #[test]
    fn test_thirty_360_bond_vs_european() {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2024, 3, 31).unwrap();

        let bond_yf = DayCounter::Thirty360Bond.year_fraction(start, end);
        let euro_yf = DayCounter::Thirty360European.year_fraction(start, end);

        assert!((bond_yf - 76.0 / 360.0).abs() < 1e-10);
        assert!((euro_yf - 75.0 / 360.0).abs() < 1e-10);
    }

    #[test]
    fn test_day_count() {
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = Date::from_ymd(2024, 1, 11).unwrap();

        assert_eq!(DayCounter::Actual365Fixed.day_count(start, end), 10);
        assert_eq!(DayCounter::Actual365Fixed.day_count(end, start), -10);
    }

    #[test]
    fn test_year_fraction_from_days() {
        assert!((DayCounter::Actual365Fixed.year_fraction_from_days(365) - 1.0).abs() < 1e-10);
        assert!((DayCounter::Actual360.year_fraction_from_days(360) - 1.0).abs() < 1e-10);
        assert!((DayCounter::Actual365Fixed.year_fraction_from_days(0)).abs() < 1e-15);
        assert!((DayCounter::Actual365Fixed.year_fraction_from_days(-365) + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_default() {
        let dcc = DayCounter::default();
        assert_eq!(dcc, DayCounter::Actual365Fixed);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(DayCounter::Actual365Fixed);
        set.insert(DayCounter::Actual360);
        set.insert(DayCounter::Actual365Fixed);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_act_act_isda_same_year() {
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = Date::from_ymd(2024, 7, 1).unwrap();
        let yf = DayCounter::ActualActualIsda.year_fraction(start, end);
        // 2024 is leap year (366 days), Jan1 to Jul1 = 182 days
        assert!((yf - 182.0 / 366.0).abs() < 1e-10);
    }

    #[test]
    fn test_act_act_isda_cross_year() {
        let start = Date::from_ymd(2024, 7, 1).unwrap();
        let end = Date::from_ymd(2025, 7, 1).unwrap();
        let yf = DayCounter::ActualActualIsda.year_fraction(start, end);
        // 2024: Jul1..Dec31 = 184 days / 366
        // 2025: Jan1..Jul1 = 181 days / 365
        let expected = 184.0 / 366.0 + 181.0 / 365.0;
        assert!((yf - expected).abs() < 1e-10);
    }

    #[test]
    fn test_act_act_isda_full_year() {
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 1, 1).unwrap();
        let yf = DayCounter::ActualActualIsda.year_fraction(start, end);
        assert!((yf - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_act_act_isda_multi_year() {
        let start = Date::from_ymd(2022, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 1, 1).unwrap();
        let yf = DayCounter::ActualActualIsda.year_fraction(start, end);
        assert!((yf - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_bus252() {
        let start = Date::from_ymd(2026, 1, 5).unwrap(); // Monday
        let end = Date::from_ymd(2026, 1, 9).unwrap(); // Friday
        let yf = DayCounter::Bus252.year_fraction(start, end);
        // 4 business days / 252
        assert!((yf - 4.0 / 252.0).abs() < 1e-10);
    }

    #[test]
    fn test_year_fraction_icma() {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2024, 7, 15).unwrap();
        let ref_start = Date::from_ymd(2024, 1, 15).unwrap();
        let ref_end = Date::from_ymd(2024, 7, 15).unwrap();
        let yf = DayCounter::ActualActualIcma.year_fraction_icma(
            start,
            end,
            ref_start,
            ref_end,
            Frequency::SemiAnnual,
        );
        // 182 / (182 * 2) = 0.5
        assert!((yf - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_year_fraction_e360_isda_with_maturity() {
        let start = Date::from_ymd(2024, 1, 31).unwrap();
        let end = Date::from_ymd(2024, 3, 31).unwrap();
        let yf_mat = DayCounter::ThirtyE360Isda.year_fraction_e360_isda(start, end, true);
        let yf_no = DayCounter::ThirtyE360Isda.year_fraction_e360_isda(start, end, false);
        // With is_maturity=true: d1=30, d2=30 → 60/360
        // With is_maturity=false: d1=30, d2=min(31,30)=30 → 60/360
        assert!((yf_mat - 60.0 / 360.0).abs() < 1e-10);
        assert!((yf_no - 60.0 / 360.0).abs() < 1e-10);
    }
}
