//! Period and tenor definitions.

use std::{fmt, ops::Add, str::FromStr};

use chrono::{Datelike, Months, NaiveDate};

use super::{day_counters::DayCounter, types::Date};

/// Time unit for period calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TimeUnit {
    /// Days.
    #[strum(serialize = "D")]
    Days,
    /// Weeks (7 days).
    #[strum(serialize = "W")]
    Weeks,
    /// Months.
    #[strum(serialize = "M")]
    Months,
    /// Years (12 months).
    #[strum(serialize = "Y")]
    Years,
}

/// A generic time period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Period {
    /// Number of units (can be negative).
    pub length: i32,
    /// Unit of time.
    pub units: TimeUnit,
}

impl Period {
    /// Create a new period.
    #[must_use]
    pub fn new(length: i32, units: TimeUnit) -> Self { Self { length, units } }

    /// Create a period in days.
    #[must_use]
    pub fn days(n: i32) -> Self { Self::new(n, TimeUnit::Days) }

    /// Create a period in weeks.
    #[must_use]
    pub fn weeks(n: i32) -> Self { Self::new(n, TimeUnit::Weeks) }

    /// Create a period in months.
    #[must_use]
    pub fn months(n: i32) -> Self { Self::new(n, TimeUnit::Months) }

    /// Create a period in years.
    #[must_use]
    pub fn years(n: i32) -> Self { Self::new(n, TimeUnit::Years) }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.length, self.units)
    }
}

impl Add<Period> for Date {
    type Output = Date;

    /// Add a period to a date.
    fn add(self, period: Period) -> Date {
        let naive = self.into_inner();
        let result = match period.units {
            TimeUnit::Days => {
                if period.length >= 0 {
                    naive.checked_add_days(chrono::Days::new(period.length as u64))
                } else {
                    naive.checked_sub_days(chrono::Days::new((-period.length) as u64))
                }
            }
            TimeUnit::Weeks => {
                let days = period.length * 7;
                if days >= 0 {
                    naive.checked_add_days(chrono::Days::new(days as u64))
                } else {
                    naive.checked_sub_days(chrono::Days::new((-days) as u64))
                }
            }
            TimeUnit::Months => {
                if period.length >= 0 {
                    naive.checked_add_months(Months::new(period.length as u32))
                } else {
                    naive.checked_sub_months(Months::new((-period.length) as u32))
                }
            }
            TimeUnit::Years => {
                let months = period.length * 12;
                if months >= 0 {
                    naive.checked_add_months(Months::new(months as u32))
                } else {
                    naive.checked_sub_months(Months::new((-months) as u32))
                }
            }
        };
        Date::from_naive(result.unwrap_or(naive))
    }
}

/// End of month handling rule for tenor calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EndOfMonthRule {
    /// Month-end to month-end adjustment.
    #[default]
    Adjust,

    /// Try to preserve the original day.
    Preserve,

    /// No special month-end handling.
    None,
}

/// Financial tenor (period) representation.
#[non_exhaustive]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::AsRefStr, strum::EnumString,
)]
#[strum(ascii_case_insensitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tenor {
    /// Overnight (O/N).
    #[cfg_attr(feature = "serde", serde(rename = "O/N"))]
    #[strum(to_string = "ON", serialize = "O/N")]
    Overnight,
    /// One week (1W).
    #[cfg_attr(feature = "serde", serde(rename = "1W"))]
    #[strum(to_string = "1W")]
    OneWeek,
    /// Two weeks (2W).
    #[cfg_attr(feature = "serde", serde(rename = "2W"))]
    #[strum(to_string = "2W")]
    TwoWeeks,
    /// One month (1M).
    #[cfg_attr(feature = "serde", serde(rename = "1M"))]
    #[strum(to_string = "1M")]
    OneMonth,
    /// Two months (2M).
    #[cfg_attr(feature = "serde", serde(rename = "2M"))]
    #[strum(to_string = "2M")]
    TwoMonths,
    /// Three months (3M).
    #[cfg_attr(feature = "serde", serde(rename = "3M"))]
    #[strum(to_string = "3M")]
    ThreeMonths,
    /// Six months (6M).
    #[cfg_attr(feature = "serde", serde(rename = "6M"))]
    #[strum(to_string = "6M")]
    SixMonths,
    /// Nine months (9M).
    #[cfg_attr(feature = "serde", serde(rename = "9M"))]
    #[strum(to_string = "9M")]
    NineMonths,
    /// One year (1Y).
    #[cfg_attr(feature = "serde", serde(rename = "1Y"))]
    #[strum(to_string = "1Y")]
    OneYear,
    /// Two years (2Y).
    #[cfg_attr(feature = "serde", serde(rename = "2Y"))]
    #[strum(to_string = "2Y")]
    TwoYears,
    /// Three years (3Y).
    #[cfg_attr(feature = "serde", serde(rename = "3Y"))]
    #[strum(to_string = "3Y")]
    ThreeYears,
    /// Five years (5Y).
    #[cfg_attr(feature = "serde", serde(rename = "5Y"))]
    #[strum(to_string = "5Y")]
    FiveYears,
    /// Seven years (7Y).
    #[cfg_attr(feature = "serde", serde(rename = "7Y"))]
    #[strum(to_string = "7Y")]
    SevenYears,
    /// Ten years (10Y).
    #[cfg_attr(feature = "serde", serde(rename = "10Y"))]
    #[strum(to_string = "10Y")]
    TenYears,
    /// Fifteen years (15Y).
    #[cfg_attr(feature = "serde", serde(rename = "15Y"))]
    #[strum(to_string = "15Y")]
    FifteenYears,
    /// Twenty years (20Y).
    #[cfg_attr(feature = "serde", serde(rename = "20Y"))]
    #[strum(to_string = "20Y")]
    TwentyYears,
    /// Thirty years (30Y).
    #[cfg_attr(feature = "serde", serde(rename = "30Y"))]
    #[strum(to_string = "30Y")]
    ThirtyYears,
}

impl Tenor {
    /// Returns all tenors in their canonical order.
    #[must_use]
    pub const fn all() -> [Tenor; 17] {
        [
            Tenor::Overnight,
            Tenor::OneWeek,
            Tenor::TwoWeeks,
            Tenor::OneMonth,
            Tenor::TwoMonths,
            Tenor::ThreeMonths,
            Tenor::SixMonths,
            Tenor::NineMonths,
            Tenor::OneYear,
            Tenor::TwoYears,
            Tenor::ThreeYears,
            Tenor::FiveYears,
            Tenor::SevenYears,
            Tenor::TenYears,
            Tenor::FifteenYears,
            Tenor::TwentyYears,
            Tenor::ThirtyYears,
        ]
    }

    /// Returns all tenor codes in their canonical order.
    #[must_use]
    pub const fn all_codes() -> [&'static str; 21] {
        [
            "ON", "1W", "2W", "1M", "2M", "3M", "1x4", "6M", "3x6", "9M", "6x9", "9x12", "1Y",
            "2Y", "3Y", "5Y", "7Y", "10Y", "15Y", "20Y", "30Y",
        ]
    }

    /// Returns the standard code for this tenor.
    #[must_use]
    pub fn code(&self) -> &str { self.as_ref() }

    /// Returns the number of months for this tenor.
    #[must_use]
    pub fn to_months(&self) -> u32 {
        match self {
            Tenor::Overnight | Tenor::OneWeek | Tenor::TwoWeeks => 0,
            Tenor::OneMonth => 1,
            Tenor::TwoMonths => 2,
            Tenor::ThreeMonths => 3,
            Tenor::SixMonths => 6,
            Tenor::NineMonths => 9,
            Tenor::OneYear => 12,
            Tenor::TwoYears => 24,
            Tenor::ThreeYears => 36,
            Tenor::FiveYears => 60,
            Tenor::SevenYears => 84,
            Tenor::TenYears => 120,
            Tenor::FifteenYears => 180,
            Tenor::TwentyYears => 240,
            Tenor::ThirtyYears => 360,
        }
    }

    /// Returns the tenor as a fraction of a year.
    #[must_use]
    pub fn to_years(&self) -> f64 {
        match self {
            Tenor::Overnight => 1.0 / 365.0,
            Tenor::OneWeek => 7.0 / 365.0,
            Tenor::TwoWeeks => 14.0 / 365.0,
            Tenor::OneMonth => 1.0 / 12.0,
            Tenor::TwoMonths => 2.0 / 12.0,
            Tenor::ThreeMonths => 3.0 / 12.0,
            Tenor::SixMonths => 6.0 / 12.0,
            Tenor::NineMonths => 9.0 / 12.0,
            Tenor::OneYear => 1.0,
            Tenor::TwoYears => 2.0,
            Tenor::ThreeYears => 3.0,
            Tenor::FiveYears => 5.0,
            Tenor::SevenYears => 7.0,
            Tenor::TenYears => 10.0,
            Tenor::FifteenYears => 15.0,
            Tenor::TwentyYears => 20.0,
            Tenor::ThirtyYears => 30.0,
        }
    }

    /// Returns the approximate number of days for this tenor.
    #[must_use]
    pub fn to_days(&self) -> u32 {
        match self {
            Tenor::Overnight => 1,
            Tenor::OneWeek => 7,
            Tenor::TwoWeeks => 14,
            Tenor::OneMonth => 30,
            Tenor::TwoMonths => 60,
            Tenor::ThreeMonths => 90,
            Tenor::SixMonths => 180,
            Tenor::NineMonths => 270,
            Tenor::OneYear => 365,
            Tenor::TwoYears => 730,
            Tenor::ThreeYears => 1095,
            Tenor::FiveYears => 1825,
            Tenor::SevenYears => 2555,
            Tenor::TenYears => 3650,
            Tenor::FifteenYears => 5475,
            Tenor::TwentyYears => 7300,
            Tenor::ThirtyYears => 10950,
        }
    }

    /// Convert to a generic Period.
    #[must_use]
    pub fn to_period(&self) -> Period {
        match self {
            Tenor::Overnight => Period::days(1),
            Tenor::OneWeek => Period::weeks(1),
            Tenor::TwoWeeks => Period::weeks(2),
            Tenor::OneMonth => Period::months(1),
            Tenor::TwoMonths => Period::months(2),
            Tenor::ThreeMonths => Period::months(3),
            Tenor::SixMonths => Period::months(6),
            Tenor::NineMonths => Period::months(9),
            Tenor::OneYear => Period::years(1),
            Tenor::TwoYears => Period::years(2),
            Tenor::ThreeYears => Period::years(3),
            Tenor::FiveYears => Period::years(5),
            Tenor::SevenYears => Period::years(7),
            Tenor::TenYears => Period::years(10),
            Tenor::FifteenYears => Period::years(15),
            Tenor::TwentyYears => Period::years(20),
            Tenor::ThirtyYears => Period::years(30),
        }
    }

    /// Adds this tenor to a date.
    #[must_use]
    pub fn add_to_date(&self, date: Date, eom_rule: EndOfMonthRule) -> Date {
        let naive = date.into_inner();

        let result = match self {
            Tenor::Overnight => naive.succ_opt().unwrap_or(naive),
            Tenor::OneWeek => add_days(naive, 7),
            Tenor::TwoWeeks => add_days(naive, 14),
            _ => {
                let months = self.to_months();
                add_months_with_eom(naive, months, eom_rule)
            }
        };

        Date::from_naive(result)
    }
}

/// Add days to a NaiveDate safely.
fn add_days(date: NaiveDate, days: u32) -> NaiveDate {
    date.checked_add_days(chrono::Days::new(u64::from(days)))
        .unwrap_or(date)
}

/// Add months to a NaiveDate with end-of-month rule handling.
fn add_months_with_eom(date: NaiveDate, months: u32, eom_rule: EndOfMonthRule) -> NaiveDate {
    let is_eom = is_end_of_month(date);

    let result = date.checked_add_months(Months::new(months)).unwrap_or(date);

    match eom_rule {
        EndOfMonthRule::Adjust if is_eom => end_of_month(result),
        EndOfMonthRule::Preserve | EndOfMonthRule::None => {
            let target_year = result.year();
            let target_month = result.month();
            let original_day = date.day();

            NaiveDate::from_ymd_opt(target_year, target_month, original_day)
                .unwrap_or_else(|| end_of_month(result))
        }
        EndOfMonthRule::Adjust => result,
    }
}

/// Check if a date is the last day of its month.
fn is_end_of_month(date: NaiveDate) -> bool {
    date.succ_opt()
        .map_or(true, |next| next.month() != date.month())
}

/// Get the last day of the month for a given date.
fn end_of_month(date: NaiveDate) -> NaiveDate {
    let year = date.year();
    let month = date.month();

    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .and_then(|d| d.pred_opt())
            .unwrap_or(date)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .and_then(|d| d.pred_opt())
            .unwrap_or(date)
    }
}

/// Parses any tenor string to years.
pub fn parse_tenor_to_years(s: &str) -> Result<f64, String> {
    let s = s.trim().to_uppercase();

    if let Ok(tenor) = s.parse::<Tenor>() {
        return Ok(tenor.to_years());
    }

    match s.as_str() {
        "T/N" | "TN" => return Ok(2.0 / 365.0),
        "S/N" | "SN" | "SPOT" => return Ok(2.0 / 365.0),
        _ => {}
    }

    if s.ends_with('Y') {
        let num_str = &s[..s.len() - 1];
        return num_str
            .parse::<f64>()
            .map_err(|_| format!("Invalid tenor format: {}", s));
    }

    if s.ends_with('M') {
        let num_str = &s[..s.len() - 1];
        return num_str
            .parse::<f64>()
            .map(|m| m / 12.0)
            .map_err(|_| format!("Invalid tenor format: {}", s));
    }

    if s.ends_with('W') {
        let num_str = &s[..s.len() - 1];
        return num_str
            .parse::<f64>()
            .map(|w| w / 52.0)
            .map_err(|_| format!("Invalid tenor format: {}", s));
    }

    if s.ends_with('D') {
        let num_str = &s[..s.len() - 1];
        return num_str
            .parse::<f64>()
            .map(|d| d / 365.0)
            .map_err(|_| format!("Invalid tenor format: {}", s));
    }

    s.parse::<f64>()
        .map_err(|_| format!("Invalid tenor format: {}", s))
}

/// Parse FRA tenor string in "NxM" format (e.g., "3x6", "3X6M", "3Mx6M").
pub fn parse_fra_tenor(tenor: &str) -> Option<(f64, f64)> {
    let tenor = tenor.trim().to_uppercase();

    let x_pos = tenor.find('X')?;
    if x_pos == 0 || x_pos == tenor.len() - 1 {
        return None;
    }

    let start_part = &tenor[..x_pos];
    let end_part = &tenor[x_pos + 1..];

    let start_months = parse_fra_period(start_part)?;

    let end_months = parse_fra_period(end_part)?;

    if end_months <= start_months {
        return None;
    }

    Some((start_months / 12.0, end_months / 12.0))
}

/// Parse a single FRA period part (e.g., "3", "3M", "12M").
fn parse_fra_period(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    if let Some(stripped) = s.strip_suffix('M') {
        stripped.parse::<f64>().ok()
    } else {
        s.parse::<f64>().ok()
    }
}

/// Parse expiry string to Date.
pub fn parse_expiry_to_date(expiry_str: &str, as_of_date: Date) -> Result<Date, String> {
    if let Ok(date) = Date::from_str(expiry_str) {
        return Ok(date);
    }

    let years = parse_tenor_to_years(expiry_str)?;
    let days = (years * 365.0).round() as i64;

    as_of_date
        .into_inner()
        .checked_add_signed(chrono::Duration::days(days))
        .map(Date::from)
        .ok_or_else(|| format!("Date overflow for expiry: {}", expiry_str))
}

/// A single accrual period for fixed income instruments.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccrualPeriod {
    /// Start date of the accrual period.
    pub start: Date,
    /// End date of the accrual period.
    pub end: Date,
    /// Payment date for this period.
    pub payment: Date,
}

impl AccrualPeriod {
    /// Creates a new accrual period.
    #[must_use]
    pub fn new(start: Date, end: Date, payment: Date) -> Self {
        Self {
            start,
            end,
            payment,
        }
    }

    /// Returns the number of days in the accrual period.
    #[must_use]
    pub fn accrual_days(&self) -> i64 { self.end - self.start }

    /// Calculates the year fraction for this period.
    #[must_use]
    pub fn year_fraction(&self, day_count: DayCounter) -> f64 {
        day_count.year_fraction(self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_unit_display() {
        assert_eq!(format!("{}", TimeUnit::Days), "D");
        assert_eq!(format!("{}", TimeUnit::Weeks), "W");
        assert_eq!(format!("{}", TimeUnit::Months), "M");
        assert_eq!(format!("{}", TimeUnit::Years), "Y");
    }

    #[test]
    fn test_period_new() {
        let period = Period::new(3, TimeUnit::Months);
        assert_eq!(period.length, 3);
        assert_eq!(period.units, TimeUnit::Months);
    }

    #[test]
    fn test_period_convenience_constructors() {
        assert_eq!(Period::days(7).length, 7);
        assert_eq!(Period::days(7).units, TimeUnit::Days);

        assert_eq!(Period::weeks(2).length, 2);
        assert_eq!(Period::weeks(2).units, TimeUnit::Weeks);

        assert_eq!(Period::months(3).length, 3);
        assert_eq!(Period::months(3).units, TimeUnit::Months);

        assert_eq!(Period::years(1).length, 1);
        assert_eq!(Period::years(1).units, TimeUnit::Years);
    }

    #[test]
    fn test_period_display() {
        assert_eq!(format!("{}", Period::months(3)), "3M");
        assert_eq!(format!("{}", Period::years(1)), "1Y");
        assert_eq!(format!("{}", Period::weeks(-2)), "-2W");
    }

    #[test]
    fn test_date_add_period_days() {
        let date = Date::from_ymd(2024, 1, 15).unwrap();
        let future = date + Period::days(10);
        assert_eq!(future, Date::from_ymd(2024, 1, 25).unwrap());
    }

    #[test]
    fn test_date_add_period_weeks() {
        let date = Date::from_ymd(2024, 1, 15).unwrap();
        let future = date + Period::weeks(2);
        assert_eq!(future, Date::from_ymd(2024, 1, 29).unwrap());
    }

    #[test]
    fn test_date_add_period_months() {
        let date = Date::from_ymd(2024, 1, 15).unwrap();
        let future = date + Period::months(3);
        assert_eq!(future, Date::from_ymd(2024, 4, 15).unwrap());
    }

    #[test]
    fn test_date_add_period_years() {
        let date = Date::from_ymd(2024, 6, 15).unwrap();
        let future = date + Period::years(1);
        assert_eq!(future, Date::from_ymd(2025, 6, 15).unwrap());
    }

    #[test]
    fn test_date_add_period_month_end() {
        let date = Date::from_ymd(2024, 1, 31).unwrap();
        let future = date + Period::months(1);
        assert_eq!(future, Date::from_ymd(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_date_add_period_negative() {
        let date = Date::from_ymd(2024, 3, 15).unwrap();
        let past = date + Period::months(-1);
        assert_eq!(past, Date::from_ymd(2024, 2, 15).unwrap());
    }

    #[test]
    fn test_tenor_code() {
        assert_eq!(Tenor::Overnight.code(), "ON");
        assert_eq!(Tenor::OneWeek.code(), "1W");
        assert_eq!(Tenor::ThreeMonths.code(), "3M");
        assert_eq!(Tenor::SixMonths.code(), "6M");
        assert_eq!(Tenor::OneYear.code(), "1Y");
        assert_eq!(Tenor::TenYears.code(), "10Y");
    }

    #[test]
    fn test_tenor_to_months() {
        assert_eq!(Tenor::Overnight.to_months(), 0);
        assert_eq!(Tenor::OneWeek.to_months(), 0);
        assert_eq!(Tenor::OneMonth.to_months(), 1);
        assert_eq!(Tenor::ThreeMonths.to_months(), 3);
        assert_eq!(Tenor::OneYear.to_months(), 12);
        assert_eq!(Tenor::TenYears.to_months(), 120);
    }

    #[test]
    fn test_tenor_to_days() {
        assert_eq!(Tenor::Overnight.to_days(), 1);
        assert_eq!(Tenor::OneWeek.to_days(), 7);
        assert_eq!(Tenor::ThreeMonths.to_days(), 90);
        assert_eq!(Tenor::OneYear.to_days(), 365);
    }

    #[test]
    fn test_tenor_to_period() {
        let period = Tenor::ThreeMonths.to_period();
        assert_eq!(period.length, 3);
        assert_eq!(period.units, TimeUnit::Months);

        let period = Tenor::OneYear.to_period();
        assert_eq!(period.length, 1);
        assert_eq!(period.units, TimeUnit::Years);

        let period = Tenor::OneWeek.to_period();
        assert_eq!(period.length, 1);
        assert_eq!(period.units, TimeUnit::Weeks);
    }

    #[test]
    fn test_tenor_from_str() {
        assert_eq!("ON".parse::<Tenor>().unwrap(), Tenor::Overnight);
        assert_eq!("3M".parse::<Tenor>().unwrap(), Tenor::ThreeMonths);
        assert_eq!("1Y".parse::<Tenor>().unwrap(), Tenor::OneYear);
        assert_eq!("10Y".parse::<Tenor>().unwrap(), Tenor::TenYears);
    }

    #[test]
    fn test_tenor_from_str_invalid() {
        assert!("INVALID".parse::<Tenor>().is_err());
        assert!("18M".parse::<Tenor>().is_err());
        // Former month-based aliases are no longer accepted as Tenor variants
        assert!("12M".parse::<Tenor>().is_err());
        assert!("24M".parse::<Tenor>().is_err());
        assert!("OVERNIGHT".parse::<Tenor>().is_err());
    }

    #[test]
    fn test_tenor_display() {
        assert_eq!(format!("{}", Tenor::ThreeMonths), "3M");
        assert_eq!(format!("{}", Tenor::OneYear), "1Y");
    }

    #[test]
    fn test_add_to_date_overnight() {
        let date = Date::from_ymd(2024, 1, 15).unwrap();
        let result = Tenor::Overnight.add_to_date(date, EndOfMonthRule::Adjust);
        assert_eq!(result, Date::from_ymd(2024, 1, 16).unwrap());
    }

    #[test]
    fn test_add_to_date_week() {
        let date = Date::from_ymd(2024, 1, 15).unwrap();
        let result = Tenor::OneWeek.add_to_date(date, EndOfMonthRule::Adjust);
        assert_eq!(result, Date::from_ymd(2024, 1, 22).unwrap());
    }

    #[test]
    fn test_add_to_date_month() {
        let date = Date::from_ymd(2024, 1, 15).unwrap();
        let result = Tenor::ThreeMonths.add_to_date(date, EndOfMonthRule::Adjust);
        assert_eq!(result, Date::from_ymd(2024, 4, 15).unwrap());
    }

    #[test]
    fn test_add_to_date_year() {
        let date = Date::from_ymd(2024, 6, 15).unwrap();
        let result = Tenor::OneYear.add_to_date(date, EndOfMonthRule::Adjust);
        assert_eq!(result, Date::from_ymd(2025, 6, 15).unwrap());
    }

    #[test]
    fn test_add_to_date_eom_adjust() {
        let date = Date::from_ymd(2024, 1, 31).unwrap();
        let result = Tenor::OneMonth.add_to_date(date, EndOfMonthRule::Adjust);
        assert_eq!(result, Date::from_ymd(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_add_to_date_eom_adjust_non_eom() {
        let date = Date::from_ymd(2024, 1, 15).unwrap();
        let result = Tenor::OneMonth.add_to_date(date, EndOfMonthRule::Adjust);
        assert_eq!(result, Date::from_ymd(2024, 2, 15).unwrap());
    }

    #[test]
    fn test_add_to_date_eom_preserve() {
        let date = Date::from_ymd(2024, 1, 31).unwrap();
        let result = Tenor::OneMonth.add_to_date(date, EndOfMonthRule::Preserve);
        assert_eq!(result, Date::from_ymd(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_add_to_date_eom_none() {
        let date = Date::from_ymd(2024, 1, 31).unwrap();
        let result = Tenor::OneMonth.add_to_date(date, EndOfMonthRule::None);
        assert_eq!(result, Date::from_ymd(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_add_to_date_feb_29_leap() {
        let date = Date::from_ymd(2024, 2, 29).unwrap();
        let result = Tenor::OneYear.add_to_date(date, EndOfMonthRule::Adjust);
        assert_eq!(result, Date::from_ymd(2025, 2, 28).unwrap());
    }

    #[test]
    fn test_end_of_month_rule_default() {
        let rule = EndOfMonthRule::default();
        assert_eq!(rule, EndOfMonthRule::Adjust);
    }

    #[test]
    fn test_tenor_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Tenor::ThreeMonths);
        set.insert(Tenor::SixMonths);
        set.insert(Tenor::ThreeMonths);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_accrual_period_new() {
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = Date::from_ymd(2024, 7, 1).unwrap();
        let payment = Date::from_ymd(2024, 7, 3).unwrap();

        let period = AccrualPeriod::new(start, end, payment);
        assert_eq!(period.start, start);
        assert_eq!(period.end, end);
        assert_eq!(period.payment, payment);
    }

    #[test]
    fn test_accrual_period_accrual_days() {
        let period = AccrualPeriod::new(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 1, 11).unwrap(),
            Date::from_ymd(2024, 1, 13).unwrap(),
        );
        assert_eq!(period.accrual_days(), 10);
    }

    #[test]
    fn test_accrual_period_year_fraction_act365() {
        let period = AccrualPeriod::new(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 7, 1).unwrap(),
            Date::from_ymd(2024, 7, 3).unwrap(),
        );
        let yf = period.year_fraction(DayCounter::Actual365Fixed);
        assert!((yf - 182.0 / 365.0).abs() < 1e-10);
    }

    #[test]
    fn test_accrual_period_year_fraction_act360() {
        let period = AccrualPeriod::new(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 7, 1).unwrap(),
            Date::from_ymd(2024, 7, 3).unwrap(),
        );
        let yf = period.year_fraction(DayCounter::Actual360);
        assert!((yf - 182.0 / 360.0).abs() < 1e-10);
    }

    #[test]
    fn test_accrual_period_clone() {
        let period = AccrualPeriod::new(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 7, 1).unwrap(),
            Date::from_ymd(2024, 7, 3).unwrap(),
        );
        let cloned = period.clone();
        assert_eq!(period, cloned);
    }
}
