//! Period and tenor definitions.
//!
//! This module provides type-safe representations of financial periods,
//! tenors, and accrual periods.
//!
//! # Examples
//!
//! ```
//! use infra_domain::time::{Date, Period, TimeUnit, Tenor, EndOfMonthRule};
//!
//! // Generic period
//! let period = Period::months(3);
//! let date = Date::from_ymd(2024, 1, 15).unwrap();
//! let future = date + period;
//! assert_eq!(future, Date::from_ymd(2024, 4, 15).unwrap());
//!
//! // Standard tenor
//! let tenor = Tenor::ThreeMonths;
//! assert_eq!(tenor.code(), "3M");
//! ```

use std::{fmt, ops::Add, str::FromStr};

use chrono::{Datelike, Months, NaiveDate};

use super::{day_counters::DayCounter, types::Date};

/// Time unit for period calculations.
///
/// # Examples
///
/// ```
/// use infra_domain::time::TimeUnit;
///
/// let unit = TimeUnit::Months;
/// assert_eq!(format!("{}", unit), "M");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TimeUnit {
    /// Days
    Days,
    /// Weeks (7 days)
    Weeks,
    /// Months
    Months,
    /// Years (12 months)
    Years,
}

impl fmt::Display for TimeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeUnit::Days => write!(f, "D"),
            TimeUnit::Weeks => write!(f, "W"),
            TimeUnit::Months => write!(f, "M"),
            TimeUnit::Years => write!(f, "Y"),
        }
    }
}

/// A generic time period.
///
/// Represents a period of time with a length and unit.
/// Can be positive or negative.
///
/// # Examples
///
/// ```
/// use infra_domain::time::{Period, TimeUnit, Date};
///
/// let period = Period::new(3, TimeUnit::Months);
/// assert_eq!(format!("{}", period), "3M");
///
/// let date = Date::from_ymd(2024, 1, 15).unwrap();
/// let future = date + period;
/// assert_eq!(future, Date::from_ymd(2024, 4, 15).unwrap());
/// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::{Period, TimeUnit};
    ///
    /// let period = Period::days(7);
    /// assert_eq!(period.length, 7);
    /// assert_eq!(period.units, TimeUnit::Days);
    /// ```
    #[must_use]
    pub fn days(n: i32) -> Self { Self::new(n, TimeUnit::Days) }

    /// Create a period in weeks.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::{Period, TimeUnit};
    ///
    /// let period = Period::weeks(2);
    /// assert_eq!(period.length, 2);
    /// assert_eq!(period.units, TimeUnit::Weeks);
    /// ```
    #[must_use]
    pub fn weeks(n: i32) -> Self { Self::new(n, TimeUnit::Weeks) }

    /// Create a period in months.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::{Period, TimeUnit};
    ///
    /// let period = Period::months(3);
    /// assert_eq!(period.length, 3);
    /// assert_eq!(period.units, TimeUnit::Months);
    /// ```
    #[must_use]
    pub fn months(n: i32) -> Self { Self::new(n, TimeUnit::Months) }

    /// Create a period in years.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::{Period, TimeUnit};
    ///
    /// let period = Period::years(1);
    /// assert_eq!(period.length, 1);
    /// assert_eq!(period.units, TimeUnit::Years);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::{Date, Period};
    ///
    /// let date = Date::from_ymd(2024, 1, 31).unwrap();
    /// let future = date + Period::months(1);
    /// // 1 month after Jan 31 = Feb 29 (2024 is leap year)
    /// assert_eq!(future, Date::from_ymd(2024, 2, 29).unwrap());
    /// ```
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
///
/// Defines how to handle month-end dates when adding tenors.
///
/// # Examples
///
/// ```
/// use infra_domain::time::{Date, Tenor, EndOfMonthRule};
///
/// let date = Date::from_ymd(2024, 1, 31).unwrap();
///
/// // Adjust: month-end to month-end
/// let adjusted = Tenor::OneMonth.add_to_date(date, EndOfMonthRule::Adjust);
/// assert_eq!(adjusted, Date::from_ymd(2024, 2, 29).unwrap()); // Feb 29 (leap year)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EndOfMonthRule {
    /// Month-end to month-end adjustment.
    ///
    /// If the start date is the last day of the month, the result date
    /// will also be the last day of the resulting month.
    /// E.g., 2024-01-31 + 1M = 2024-02-29 (leap year)
    #[default]
    Adjust,

    /// Try to preserve the original day.
    ///
    /// If the day doesn't exist in the target month, fall back to
    /// the last day of that month.
    /// E.g., 2024-01-31 + 1M = 2024-02-29 (day 31 doesn't exist)
    Preserve,

    /// No special month-end handling.
    ///
    /// Simple month addition with fallback to month-end if invalid.
    /// Same behaviour as Preserve for most cases.
    None,
}

/// Financial tenor (period) representation.
///
/// Standard tenors used in financial markets for interest rates,
/// swaps, and other instruments.
///
/// # Examples
///
/// ```
/// use infra_domain::time::Tenor;
///
/// let tenor = Tenor::ThreeMonths;
/// assert_eq!(tenor.code(), "3M");
/// assert_eq!(tenor.to_months(), 3);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tenor {
    /// Overnight (O/N)
    #[cfg_attr(feature = "serde", serde(rename = "O/N"))]
    Overnight,
    /// One week (1W)
    #[cfg_attr(feature = "serde", serde(rename = "1W"))]
    OneWeek,
    /// Two weeks (2W)
    #[cfg_attr(feature = "serde", serde(rename = "2W"))]
    TwoWeeks,
    /// One month (1M)
    #[cfg_attr(feature = "serde", serde(rename = "1M"))]
    OneMonth,
    /// Two months (2M)
    #[cfg_attr(feature = "serde", serde(rename = "2M"))]
    TwoMonths,
    /// Three months (3M)
    #[cfg_attr(feature = "serde", serde(rename = "3M"))]
    ThreeMonths,
    /// Six months (6M)
    #[cfg_attr(feature = "serde", serde(rename = "6M"))]
    SixMonths,
    /// Nine months (9M)
    #[cfg_attr(feature = "serde", serde(rename = "9M"))]
    NineMonths,
    /// One year (1Y)
    #[cfg_attr(feature = "serde", serde(rename = "1Y"))]
    OneYear,
    /// Two years (2Y)
    #[cfg_attr(feature = "serde", serde(rename = "2Y"))]
    TwoYears,
    /// Three years (3Y)
    #[cfg_attr(feature = "serde", serde(rename = "3Y"))]
    ThreeYears,
    /// Five years (5Y)
    #[cfg_attr(feature = "serde", serde(rename = "5Y"))]
    FiveYears,
    /// Seven years (7Y)
    #[cfg_attr(feature = "serde", serde(rename = "7Y"))]
    SevenYears,
    /// Ten years (10Y)
    #[cfg_attr(feature = "serde", serde(rename = "10Y"))]
    TenYears,
    /// Fifteen years (15Y)
    #[cfg_attr(feature = "serde", serde(rename = "15Y"))]
    FifteenYears,
    /// Twenty years (20Y)
    #[cfg_attr(feature = "serde", serde(rename = "20Y"))]
    TwentyYears,
    /// Thirty years (30Y)
    #[cfg_attr(feature = "serde", serde(rename = "30Y"))]
    ThirtyYears,
}

impl Tenor {
    /// Returns all tenors in their canonical order.
    ///
    /// The order matches the enum definition order, which represents
    /// the standard financial ordering from shortest to longest maturity.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::Tenor;
    ///
    /// let tenors = Tenor::all();
    /// assert_eq!(tenors[0], Tenor::Overnight);
    /// assert_eq!(tenors[1], Tenor::OneWeek);
    /// assert_eq!(tenors.last(), Some(&Tenor::ThirtyYears));
    /// ```
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
    ///
    /// # Examples
    /// Returns all tenor codes in canonical order, including common FRA tenors.
    ///
    /// FRA tenors (NxM format) are sorted by their end month (maturity),
    /// placed after the standard tenor of the same maturity:
    /// - 1x4 (ends at 4M) → between 3M and 6M
    /// - 3x6 (ends at 6M) → after 6M
    /// - 6x9 (ends at 9M) → after 9M
    /// - 9x12 (ends at 12M) → before 1Y
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::Tenor;
    ///
    /// let codes = Tenor::all_codes();
    /// assert_eq!(codes[0], "ON");
    /// assert!(codes.contains(&"1x4"));
    /// assert!(codes.contains(&"3x6"));
    /// ```
    #[must_use]
    pub const fn all_codes() -> [&'static str; 21] {
        [
            "ON", "1W", "2W", "1M", "2M", "3M", "1x4", "6M", "3x6", "9M", "6x9", "9x12", "1Y",
            "2Y", "3Y", "5Y", "7Y", "10Y", "15Y", "20Y", "30Y",
        ]
    }

    /// Returns the standard code for this tenor.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::Tenor;
    ///
    /// assert_eq!(Tenor::Overnight.code(), "ON");
    /// assert_eq!(Tenor::ThreeMonths.code(), "3M");
    /// assert_eq!(Tenor::OneYear.code(), "1Y");
    /// ```
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Tenor::Overnight => "ON",
            Tenor::OneWeek => "1W",
            Tenor::TwoWeeks => "2W",
            Tenor::OneMonth => "1M",
            Tenor::TwoMonths => "2M",
            Tenor::ThreeMonths => "3M",
            Tenor::SixMonths => "6M",
            Tenor::NineMonths => "9M",
            Tenor::OneYear => "1Y",
            Tenor::TwoYears => "2Y",
            Tenor::ThreeYears => "3Y",
            Tenor::FiveYears => "5Y",
            Tenor::SevenYears => "7Y",
            Tenor::TenYears => "10Y",
            Tenor::FifteenYears => "15Y",
            Tenor::TwentyYears => "20Y",
            Tenor::ThirtyYears => "30Y",
        }
    }

    /// Returns the number of months for this tenor.
    ///
    /// For tenors shorter than a month, returns 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::Tenor;
    ///
    /// assert_eq!(Tenor::Overnight.to_months(), 0);
    /// assert_eq!(Tenor::ThreeMonths.to_months(), 3);
    /// assert_eq!(Tenor::OneYear.to_months(), 12);
    /// assert_eq!(Tenor::TenYears.to_months(), 120);
    /// ```
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
    ///
    /// Uses 365 days per year for day-based tenors and 12 months per year.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::Tenor;
    ///
    /// assert!((Tenor::Overnight.to_years() - 1.0/365.0).abs() < 1e-10);
    /// assert!((Tenor::ThreeMonths.to_years() - 0.25).abs() < 1e-10);
    /// assert!((Tenor::OneYear.to_years() - 1.0).abs() < 1e-10);
    /// ```
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
    ///
    /// This is an approximation using 30 days per month and 365 days per year.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::Tenor;
    ///
    /// assert_eq!(Tenor::Overnight.to_days(), 1);
    /// assert_eq!(Tenor::OneWeek.to_days(), 7);
    /// assert_eq!(Tenor::ThreeMonths.to_days(), 90);
    /// assert_eq!(Tenor::OneYear.to_days(), 365);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::{Tenor, Period, TimeUnit};
    ///
    /// let period = Tenor::ThreeMonths.to_period();
    /// assert_eq!(period.length, 3);
    /// assert_eq!(period.units, TimeUnit::Months);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::{Date, Tenor, EndOfMonthRule};
    ///
    /// let date = Date::from_ymd(2024, 1, 15).unwrap();
    ///
    /// // Add 3 months
    /// let future = Tenor::ThreeMonths.add_to_date(date, EndOfMonthRule::Adjust);
    /// assert_eq!(future, Date::from_ymd(2024, 4, 15).unwrap());
    ///
    /// // Add overnight
    /// let tomorrow = Tenor::Overnight.add_to_date(date, EndOfMonthRule::Adjust);
    /// assert_eq!(tomorrow, Date::from_ymd(2024, 1, 16).unwrap());
    /// ```
    #[must_use]
    pub fn add_to_date(&self, date: Date, eom_rule: EndOfMonthRule) -> Date {
        let naive = date.into_inner();

        let result = match self {
            // Day-based tenors
            Tenor::Overnight => naive.succ_opt().unwrap_or(naive),
            Tenor::OneWeek => add_days(naive, 7),
            Tenor::TwoWeeks => add_days(naive, 14),
            // Month-based tenors
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

    // Add months
    let result = date.checked_add_months(Months::new(months)).unwrap_or(date);

    match eom_rule {
        EndOfMonthRule::Adjust if is_eom => {
            // Adjust to end of resulting month
            end_of_month(result)
        }
        EndOfMonthRule::Preserve | EndOfMonthRule::None => {
            // Try to preserve day, fall back to end of month if invalid
            let target_year = result.year();
            let target_month = result.month();
            let original_day = date.day();

            NaiveDate::from_ymd_opt(target_year, target_month, original_day).unwrap_or_else(|| {
                // Day doesn't exist in target month, use last day
                end_of_month(result)
            })
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
    // Get first day of next month, then subtract one day
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

impl FromStr for Tenor {
    type Err = String;

    /// Parses a tenor from string (case-insensitive).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::Tenor;
    ///
    /// assert_eq!("3M".parse::<Tenor>().unwrap(), Tenor::ThreeMonths);
    /// assert_eq!("1Y".parse::<Tenor>().unwrap(), Tenor::OneYear);
    /// assert_eq!("ON".parse::<Tenor>().unwrap(), Tenor::Overnight);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ON" | "O/N" | "OVERNIGHT" => Ok(Tenor::Overnight),
            "1W" => Ok(Tenor::OneWeek),
            "2W" => Ok(Tenor::TwoWeeks),
            "1M" => Ok(Tenor::OneMonth),
            "2M" => Ok(Tenor::TwoMonths),
            "3M" => Ok(Tenor::ThreeMonths),
            "6M" => Ok(Tenor::SixMonths),
            "9M" => Ok(Tenor::NineMonths),
            "1Y" | "12M" => Ok(Tenor::OneYear),
            "2Y" | "24M" => Ok(Tenor::TwoYears),
            "3Y" | "36M" => Ok(Tenor::ThreeYears),
            "5Y" | "60M" => Ok(Tenor::FiveYears),
            "7Y" | "84M" => Ok(Tenor::SevenYears),
            "10Y" | "120M" => Ok(Tenor::TenYears),
            "15Y" | "180M" => Ok(Tenor::FifteenYears),
            "20Y" | "240M" => Ok(Tenor::TwentyYears),
            "30Y" | "360M" => Ok(Tenor::ThirtyYears),
            _ => Err(format!("Unknown tenor: {}", s)),
        }
    }
}

/// Parses any tenor string to years.
///
/// This function handles:
/// - Standard tenors: ON, 1W, 1M, 1Y, etc.
/// - Arbitrary tenors: 4M, 18M, 15Y, etc.
/// - Special tenors: O/N, T/N, S/N, SPOT
/// - Day-based: 1D, 30D, etc.
///
/// # Arguments
///
/// * `s` - Tenor string to parse
///
/// # Returns
///
/// Tenor value in years, or error if parsing fails.
///
/// # Examples
///
/// ```
/// use infra_domain::time::parse_tenor_to_years;
///
/// assert!((parse_tenor_to_years("ON").unwrap() - 1.0/365.0).abs() < 1e-10);
/// assert!((parse_tenor_to_years("3M").unwrap() - 0.25).abs() < 1e-10);
/// assert!((parse_tenor_to_years("1Y").unwrap() - 1.0).abs() < 1e-10);
/// assert!((parse_tenor_to_years("18M").unwrap() - 1.5).abs() < 1e-10);
/// ```
pub fn parse_tenor_to_years(s: &str) -> Result<f64, String> {
    let s = s.trim().to_uppercase();

    // Try standard Tenor first
    if let Ok(tenor) = s.parse::<Tenor>() {
        return Ok(tenor.to_years());
    }

    // Handle special tenors not in Tenor enum
    match s.as_str() {
        "T/N" | "TN" => return Ok(2.0 / 365.0),
        "S/N" | "SN" | "SPOT" => return Ok(2.0 / 365.0),
        _ => {}
    }

    // Parse arbitrary tenors: NxY, NxM, NxW, NxD
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

    // Try parsing as a plain number (years)
    s.parse::<f64>()
        .map_err(|_| format!("Invalid tenor format: {}", s))
}

/// Formats a year fraction as a human-readable tenor string.
///
/// This is the reverse operation of [`parse_tenor_to_years`].
///
/// The function attempts to find the most natural representation:
/// - 0.0 → "0"
/// - 1/365 → "O/N" (overnight)
/// - 1/52 → "1W" (1 week)
/// - 1/12 → "1M" (1 month)
/// - 0.5 → "6M"
/// - 1.0 → "1Y"
/// - 2.5 → "2.50Y"
///
/// # Arguments
///
/// * `years` - Tenor in decimal years
///
/// # Returns
///
/// Human-readable tenor string.
///
/// # Examples
///
/// ```
/// use infra_domain::time::format_years_as_tenor;
///
/// assert_eq!(format_years_as_tenor(0.0), "0");
/// assert_eq!(format_years_as_tenor(1.0 / 52.0), "1W");
/// assert_eq!(format_years_as_tenor(0.25), "3M");
/// assert_eq!(format_years_as_tenor(1.0), "1Y");
/// assert_eq!(format_years_as_tenor(10.0), "10Y");
/// ```
#[must_use]
pub fn format_years_as_tenor(years: f64) -> String {
    // Handle zero and near-zero values
    if years.abs() < 0.001 {
        return "0".to_string();
    }

    // Check for exact year values (integers)
    if years >= 1.0 && (years - years.round()).abs() < 0.001 {
        return format!("{}Y", years.round() as i32);
    }

    // Convert to weeks for very short tenors (up to 4 weeks)
    let weeks = years * 52.0;
    let rounded_weeks = weeks.round();
    if rounded_weeks >= 1.0 && rounded_weeks <= 4.0 && (weeks - rounded_weeks).abs() < 0.5 {
        return format!("{}W", rounded_weeks as i32);
    }

    // Convert to months
    let months = years * 12.0;
    let rounded_months = months.round();

    // Check if it's a clean month value (within 0.1 month tolerance)
    if rounded_months > 0.0 && (months - rounded_months).abs() < 0.1 {
        if rounded_months >= 12.0 && rounded_months as i32 % 12 == 0 {
            return format!("{}Y", rounded_months as i32 / 12);
        }
        return format!("{}M", rounded_months as i32);
    }

    // Convert to days for overnight/short tenors
    let days = years * 365.0;
    if days < 7.0 && days > 0.0 {
        let rounded_days = days.round() as i32;
        if rounded_days == 1 {
            return "O/N".to_string();
        }
        if rounded_days > 0 {
            return format!("{}D", rounded_days);
        }
    }

    // Fallback: show years with reasonable precision
    if years < 1.0 {
        format!("{:.1}M", years * 12.0)
    } else {
        format!("{:.2}Y", years)
    }
}

/// Parse FRA tenor string in "NxM" format (e.g., "3x6", "3X6M", "3Mx6M").
///
/// FRA tenors represent forward rate agreements with a start and end period.
/// Common formats include:
/// - "3x6" - 3 months to 6 months
/// - "6x12" - 6 months to 12 months
/// - "3Mx6M" - 3 months to 6 months (explicit month suffix)
///
/// # Arguments
///
/// * `tenor` - FRA tenor string
///
/// # Returns
///
/// `Some((start_years, end_years))` if successful, `None` otherwise.
///
/// # Examples
///
/// ```
/// use infra_domain::time::parse_fra_tenor;
///
/// let result = parse_fra_tenor("3x6");
/// assert!(result.is_some());
/// let (start, end) = result.unwrap();
/// assert!((start - 0.25).abs() < 1e-10); // 3M = 0.25Y
/// assert!((end - 0.5).abs() < 1e-10);    // 6M = 0.5Y
/// ```
pub fn parse_fra_tenor(tenor: &str) -> Option<(f64, f64)> {
    let tenor = tenor.trim().to_uppercase();

    // Find the 'X' separator
    let x_pos = tenor.find('X')?;
    if x_pos == 0 || x_pos == tenor.len() - 1 {
        return None;
    }

    let start_part = &tenor[..x_pos];
    let end_part = &tenor[x_pos + 1..];

    // Parse start period
    let start_months = parse_fra_period(start_part)?;

    // Parse end period
    let end_months = parse_fra_period(end_part)?;

    if end_months <= start_months {
        return None;
    }

    Some((start_months / 12.0, end_months / 12.0))
}

/// Parse a single FRA period part (e.g., "3", "3M", "12M").
///
/// # Arguments
///
/// * `s` - Period string (with or without 'M' suffix)
///
/// # Returns
///
/// Period in months, or `None` if parsing fails.
fn parse_fra_period(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // If ends with 'M', strip it and parse as months
    if let Some(stripped) = s.strip_suffix('M') {
        stripped.parse::<f64>().ok()
    } else {
        // Assume it's already in months
        s.parse::<f64>().ok()
    }
}

/// Parse expiry string to Date.
///
/// Supports:
/// - ISO date format: "2027-01-25"
/// - Tenor from as_of_date: "1Y", "6M", etc.
///
/// # Arguments
///
/// * `expiry_str` - Expiry string
/// * `as_of_date` - Reference date for tenor-based expiry
///
/// # Examples
///
/// ```
/// use infra_domain::time::{Date, parse_expiry_to_date};
///
/// let as_of = Date::from_ymd(2024, 1, 1).unwrap();
/// let date = parse_expiry_to_date("2024-06-01", as_of).unwrap();
/// assert_eq!(date, Date::from_ymd(2024, 6, 1).unwrap());
///
/// let date = parse_expiry_to_date("1Y", as_of).unwrap();
/// // Note: Uses 365 days/year approximation, so leap year 2024 results in Dec 31
/// assert_eq!(date, Date::from_ymd(2024, 12, 31).unwrap());
/// ```
pub fn parse_expiry_to_date(expiry_str: &str, as_of_date: Date) -> Result<Date, String> {
    // Try ISO date format first
    if let Ok(date) = Date::from_str(expiry_str) {
        return Ok(date);
    }

    // Try tenor format
    let years = parse_tenor_to_years(expiry_str)?;
    let days = (years * 365.0).round() as i64;

    as_of_date
        .into_inner()
        .checked_add_signed(chrono::Duration::days(days))
        .map(Date::from)
        .ok_or_else(|| format!("Date overflow for expiry: {}", expiry_str))
}

impl fmt::Display for Tenor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.code()) }
}

/// A single accrual period for fixed income instruments.
///
/// Represents a period between two dates with an associated payment date.
/// Used for calculating interest accruals in bonds, swaps, and other
/// fixed income instruments.
///
/// # Examples
///
/// ```
/// use infra_domain::time::{Date, AccrualPeriod, DayCounter};
///
/// let start = Date::from_ymd(2024, 1, 1).unwrap();
/// let end = Date::from_ymd(2024, 7, 1).unwrap();
/// let payment = Date::from_ymd(2024, 7, 3).unwrap();
///
/// let period = AccrualPeriod::new(start, end, payment);
/// assert_eq!(period.accrual_days(), 182);
/// ```
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
    ///
    /// # Arguments
    /// * `start` - Start date of the accrual period
    /// * `end` - End date of the accrual period
    /// * `payment` - Payment date for this period
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::{Date, AccrualPeriod};
    ///
    /// let period = AccrualPeriod::new(
    ///     Date::from_ymd(2024, 1, 1).unwrap(),
    ///     Date::from_ymd(2024, 7, 1).unwrap(),
    ///     Date::from_ymd(2024, 7, 3).unwrap(),
    /// );
    /// ```
    #[must_use]
    pub fn new(start: Date, end: Date, payment: Date) -> Self {
        Self {
            start,
            end,
            payment,
        }
    }

    /// Returns the number of days in the accrual period.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::{Date, AccrualPeriod};
    ///
    /// let period = AccrualPeriod::new(
    ///     Date::from_ymd(2024, 1, 1).unwrap(),
    ///     Date::from_ymd(2024, 1, 11).unwrap(),
    ///     Date::from_ymd(2024, 1, 13).unwrap(),
    /// );
    /// assert_eq!(period.accrual_days(), 10);
    /// ```
    #[must_use]
    pub fn accrual_days(&self) -> i64 { self.end - self.start }

    /// Calculates the year fraction for this period.
    ///
    /// # Arguments
    /// * `day_count` - The day count convention to use
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::{Date, AccrualPeriod, DayCounter};
    ///
    /// let period = AccrualPeriod::new(
    ///     Date::from_ymd(2024, 1, 1).unwrap(),
    ///     Date::from_ymd(2024, 7, 1).unwrap(),
    ///     Date::from_ymd(2024, 7, 3).unwrap(),
    /// );
    ///
    /// let yf = period.year_fraction(DayCounter::Actual365Fixed);
    /// assert!((yf - 182.0 / 365.0).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn year_fraction(&self, day_count: DayCounter) -> f64 {
        day_count.year_fraction(self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TimeUnit tests

    #[test]
    fn test_time_unit_display() {
        assert_eq!(format!("{}", TimeUnit::Days), "D");
        assert_eq!(format!("{}", TimeUnit::Weeks), "W");
        assert_eq!(format!("{}", TimeUnit::Months), "M");
        assert_eq!(format!("{}", TimeUnit::Years), "Y");
    }

    // Period tests

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
        // Jan 31 + 1 month = Feb 29 (2024 is leap year)
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

    // Tenor tests

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
        // End of month to end of month
        let date = Date::from_ymd(2024, 1, 31).unwrap();
        let result = Tenor::OneMonth.add_to_date(date, EndOfMonthRule::Adjust);
        // Jan 31 + 1M with EOM adjust = Feb 29 (2024 is leap year)
        assert_eq!(result, Date::from_ymd(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_add_to_date_eom_adjust_non_eom() {
        // Non-end-of-month stays at same day
        let date = Date::from_ymd(2024, 1, 15).unwrap();
        let result = Tenor::OneMonth.add_to_date(date, EndOfMonthRule::Adjust);
        assert_eq!(result, Date::from_ymd(2024, 2, 15).unwrap());
    }

    #[test]
    fn test_add_to_date_eom_preserve() {
        // Try to preserve day 31, but Feb only has 29 days
        let date = Date::from_ymd(2024, 1, 31).unwrap();
        let result = Tenor::OneMonth.add_to_date(date, EndOfMonthRule::Preserve);
        // Falls back to last day of Feb
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
        // Feb 29 (leap year) + 1Y = Feb 28 (non-leap year)
        let date = Date::from_ymd(2024, 2, 29).unwrap();
        let result = Tenor::OneYear.add_to_date(date, EndOfMonthRule::Adjust);
        // EOM adjust: Feb 29 is EOM, so result should be Feb 28 (EOM in 2025)
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
        set.insert(Tenor::ThreeMonths); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // AccrualPeriod tests

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
