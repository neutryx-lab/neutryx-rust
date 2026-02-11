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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub const fn new(length: i32, units: TimeUnit) -> Self { Self { length, units } }

    /// Create a period in days.
    #[must_use]
    pub const fn days(n: i32) -> Self { Self::new(n, TimeUnit::Days) }

    /// Create a period in weeks.
    #[must_use]
    pub const fn weeks(n: i32) -> Self { Self::new(n, TimeUnit::Weeks) }

    /// Create a period in months.
    #[must_use]
    pub const fn months(n: i32) -> Self { Self::new(n, TimeUnit::Months) }

    /// Create a period in years.
    #[must_use]
    pub const fn years(n: i32) -> Self { Self::new(n, TimeUnit::Years) }
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
///
/// A newtype wrapper around [`Period`] that adds financial tenor semantics:
/// - Special "ON" (Overnight) display and parsing for 1D periods
/// - Predefined constants for standard market tenors
/// - Serde as human-readable strings ("ON", "3M", "1Y", "42D", etc.)
/// - Support for arbitrary tenors (e.g., `Tenor::months(18)`, `Tenor::days(42)`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tenor(pub Period);

/// Standard tenor constants (PascalCase for backward compatibility with former enum variants).
#[allow(non_upper_case_globals)]
impl Tenor {
    /// Overnight (1D).
    pub const Overnight: Self = Self::new(Period::days(1));
    /// One week (1W).
    pub const OneWeek: Self = Self::new(Period::weeks(1));
    /// Two weeks (2W).
    pub const TwoWeeks: Self = Self::new(Period::weeks(2));
    /// One month (1M).
    pub const OneMonth: Self = Self::new(Period::months(1));
    /// Two months (2M).
    pub const TwoMonths: Self = Self::new(Period::months(2));
    /// Three months (3M).
    pub const ThreeMonths: Self = Self::new(Period::months(3));
    /// Six months (6M).
    pub const SixMonths: Self = Self::new(Period::months(6));
    /// Nine months (9M).
    pub const NineMonths: Self = Self::new(Period::months(9));
    /// One year (1Y).
    pub const OneYear: Self = Self::new(Period::years(1));
    /// Two years (2Y).
    pub const TwoYears: Self = Self::new(Period::years(2));
    /// Three years (3Y).
    pub const ThreeYears: Self = Self::new(Period::years(3));
    /// Five years (5Y).
    pub const FiveYears: Self = Self::new(Period::years(5));
    /// Seven years (7Y).
    pub const SevenYears: Self = Self::new(Period::years(7));
    /// Ten years (10Y).
    pub const TenYears: Self = Self::new(Period::years(10));
    /// Fifteen years (15Y).
    pub const FifteenYears: Self = Self::new(Period::years(15));
    /// Twenty years (20Y).
    pub const TwentyYears: Self = Self::new(Period::years(20));
    /// Thirty years (30Y).
    pub const ThirtyYears: Self = Self::new(Period::years(30));
}

impl Tenor {
    /// Normalise a period to its canonical unit.
    ///
    /// - Days divisible by 7 are promoted to weeks (7D → 1W).
    /// - Months divisible by 12 are promoted to years (12M → 1Y).
    #[must_use]
    const fn normalize(period: Period) -> Period {
        match period.units {
            TimeUnit::Days if period.length > 0 && period.length % 7 == 0 => {
                Period::weeks(period.length / 7)
            }
            TimeUnit::Months if period.length > 0 && period.length % 12 == 0 => {
                Period::years(period.length / 12)
            }
            _ => period,
        }
    }

    /// Create a tenor from a [`Period`], normalising to canonical units.
    #[must_use]
    pub const fn new(period: Period) -> Self { Self(Self::normalize(period)) }

    /// Create a tenor in days.
    #[must_use]
    pub const fn days(n: i32) -> Self { Self::new(Period::days(n)) }

    /// Create a tenor in weeks.
    #[must_use]
    pub const fn weeks(n: i32) -> Self { Self::new(Period::weeks(n)) }

    /// Create a tenor in months.
    #[must_use]
    pub const fn months(n: i32) -> Self { Self::new(Period::months(n)) }

    /// Create a tenor in years.
    #[must_use]
    pub const fn years(n: i32) -> Self { Self::new(Period::years(n)) }

    /// Returns the underlying [`Period`].
    #[must_use]
    pub const fn period(&self) -> Period { self.0 }

    /// Returns all standard tenors in their canonical order.
    #[must_use]
    pub const fn all() -> [Tenor; 17] {
        [
            Self::Overnight,
            Self::OneWeek,
            Self::TwoWeeks,
            Self::OneMonth,
            Self::TwoMonths,
            Self::ThreeMonths,
            Self::SixMonths,
            Self::NineMonths,
            Self::OneYear,
            Self::TwoYears,
            Self::ThreeYears,
            Self::FiveYears,
            Self::SevenYears,
            Self::TenYears,
            Self::FifteenYears,
            Self::TwentyYears,
            Self::ThirtyYears,
        ]
    }

    /// Returns all standard tenor codes in their canonical order (including FRA codes).
    #[must_use]
    pub const fn all_codes() -> [&'static str; 21] {
        [
            "ON", "1W", "2W", "1M", "2M", "3M", "1x4", "6M", "3x6", "9M", "6x9", "9x12", "1Y",
            "2Y", "3Y", "5Y", "7Y", "10Y", "15Y", "20Y", "30Y",
        ]
    }

    /// Returns the standard code for this tenor.
    #[must_use]
    pub fn code(&self) -> String { self.to_string() }

    /// Returns the number of months for this tenor (approximate for day/week tenors).
    #[must_use]
    pub const fn to_months(&self) -> u32 {
        match self.0.units {
            TimeUnit::Days | TimeUnit::Weeks => 0,
            TimeUnit::Months => self.0.length as u32,
            TimeUnit::Years => (self.0.length * 12) as u32,
        }
    }

    /// Returns the tenor as a fraction of a year.
    #[must_use]
    pub fn to_years(&self) -> f64 {
        match self.0.units {
            TimeUnit::Days => self.0.length as f64 / 365.0,
            TimeUnit::Weeks => (self.0.length * 7) as f64 / 365.0,
            TimeUnit::Months => self.0.length as f64 / 12.0,
            TimeUnit::Years => self.0.length as f64,
        }
    }

    /// Returns the approximate number of days for this tenor.
    #[must_use]
    pub const fn to_days(&self) -> u32 {
        match self.0.units {
            TimeUnit::Days => self.0.length as u32,
            TimeUnit::Weeks => (self.0.length * 7) as u32,
            TimeUnit::Months => (self.0.length * 30) as u32,
            TimeUnit::Years => (self.0.length * 365) as u32,
        }
    }

    /// Convert to a generic [`Period`].
    #[must_use]
    pub const fn to_period(&self) -> Period { self.0 }

    /// Adds this tenor to a date.
    #[must_use]
    pub fn add_to_date(&self, date: Date, eom_rule: EndOfMonthRule) -> Date {
        let naive = date.into_inner();

        let result = match self.0.units {
            TimeUnit::Days => add_days(naive, self.0.length as u32),
            TimeUnit::Weeks => add_days(naive, (self.0.length * 7) as u32),
            TimeUnit::Months | TimeUnit::Years => {
                let months = self.to_months();
                add_months_with_eom(naive, months, eom_rule)
            }
        };

        Date::from_naive(result)
    }
}

impl fmt::Display for Tenor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::Overnight {
            write!(f, "ON")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

impl FromStr for Tenor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("ON") {
            return Ok(Self::Overnight);
        }
        let upper = s.to_uppercase();
        if upper.len() < 2 {
            return Err(format!("Invalid tenor: {s}"));
        }
        let (num, unit) = upper.split_at(upper.len() - 1);
        let length: i32 = num
            .parse()
            .map_err(|_| format!("Invalid tenor: {s}"))?;
        let units = match unit {
            "D" => TimeUnit::Days,
            "W" => TimeUnit::Weeks,
            "M" => TimeUnit::Months,
            "Y" => TimeUnit::Years,
            _ => return Err(format!("Invalid tenor unit: {s}")),
        };
        Ok(Self::new(Period::new(length, units)))
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Tenor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Tenor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
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
        "TN" => return Ok(1.0 / 365.0),
        "SN" => return Ok(1.0 / 365.0),
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
    fn test_tenor_from_str_arbitrary() {
        // Arbitrary tenors are now supported
        let t18m = "18M".parse::<Tenor>().unwrap();
        assert_eq!(t18m, Tenor::months(18));
        assert_eq!(format!("{t18m}"), "18M");

        // 42D normalises to 6W (42 = 7 × 6)
        let t42d = "42D".parse::<Tenor>().unwrap();
        assert_eq!(t42d, Tenor::weeks(6));
        assert_eq!(format!("{t42d}"), "6W");

        // 10D stays as days (not divisible by 7)
        let t10d = "10D".parse::<Tenor>().unwrap();
        assert_eq!(t10d, Tenor::days(10));
        assert_eq!(format!("{t10d}"), "10D");
    }

    #[test]
    fn test_tenor_normalisation() {
        // 12M normalises to 1Y
        assert_eq!(Tenor::months(12), Tenor::OneYear);
        assert_eq!("12M".parse::<Tenor>().unwrap(), Tenor::OneYear);
        assert_eq!(format!("{}", Tenor::months(12)), "1Y");

        // 24M normalises to 2Y
        assert_eq!(Tenor::months(24), Tenor::TwoYears);

        // 7D normalises to 1W
        assert_eq!(Tenor::days(7), Tenor::OneWeek);
        assert_eq!(format!("{}", Tenor::days(7)), "1W");

        // 14D normalises to 2W
        assert_eq!(Tenor::days(14), Tenor::TwoWeeks);

        // Non-divisible values stay as-is
        assert_eq!(Tenor::months(18).0.units, TimeUnit::Months);
        assert_eq!(Tenor::days(10).0.units, TimeUnit::Days);
    }

    #[test]
    fn test_tenor_from_str_invalid() {
        assert!("INVALID".parse::<Tenor>().is_err());
        assert!("OVERNIGHT".parse::<Tenor>().is_err());
        assert!("O/N".parse::<Tenor>().is_err());
        assert!("".parse::<Tenor>().is_err());
        assert!("X".parse::<Tenor>().is_err());
        assert!("3X".parse::<Tenor>().is_err());
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
