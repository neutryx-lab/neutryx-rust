//! Period and tenor definitions.
//!
//! This module provides type-safe representations of financial periods,
//! tenors, and accrual periods.
//!
//! # Examples
//!
//! ```
//! use infra_master::time::{Date, Period, TimeUnit, Tenor, EndOfMonthRule};
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
/// use infra_master::time::TimeUnit;
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
/// use infra_master::time::{Period, TimeUnit, Date};
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
    /// use infra_master::time::{Period, TimeUnit};
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
    /// use infra_master::time::{Period, TimeUnit};
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
    /// use infra_master::time::{Period, TimeUnit};
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
    /// use infra_master::time::{Period, TimeUnit};
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
    /// use infra_master::time::{Date, Period};
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
/// use infra_master::time::{Date, Tenor, EndOfMonthRule};
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
/// use infra_master::time::Tenor;
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
    Overnight,
    /// One week (1W)
    OneWeek,
    /// Two weeks (2W)
    TwoWeeks,
    /// One month (1M)
    OneMonth,
    /// Two months (2M)
    TwoMonths,
    /// Three months (3M)
    ThreeMonths,
    /// Six months (6M)
    SixMonths,
    /// Nine months (9M)
    NineMonths,
    /// One year (1Y)
    OneYear,
    /// Two years (2Y)
    TwoYears,
    /// Three years (3Y)
    ThreeYears,
    /// Five years (5Y)
    FiveYears,
    /// Seven years (7Y)
    SevenYears,
    /// Ten years (10Y)
    TenYears,
    /// Fifteen years (15Y)
    FifteenYears,
    /// Twenty years (20Y)
    TwentyYears,
    /// Thirty years (30Y)
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
    /// use infra_master::time::Tenor;
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
    ///
    /// ```
    /// use infra_master::time::Tenor;
    ///
    /// let codes = Tenor::all_codes();
    /// assert_eq!(codes[0], "ON");
    /// assert_eq!(codes[3], "1M");
    /// assert_eq!(codes[8], "1Y");
    /// ```
    #[must_use]
    pub const fn all_codes() -> [&'static str; 17] {
        [
            "ON", "1W", "2W", "1M", "2M", "3M", "6M", "9M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y",
            "15Y", "20Y", "30Y",
        ]
    }

    /// Returns the standard code for this tenor.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Tenor;
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
    /// use infra_master::time::Tenor;
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

    /// Returns the approximate number of days for this tenor.
    ///
    /// This is an approximation using 30 days per month and 365 days per year.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::Tenor;
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
    /// use infra_master::time::{Tenor, Period, TimeUnit};
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
    /// use infra_master::time::{Date, Tenor, EndOfMonthRule};
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
    /// use infra_master::time::Tenor;
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
/// use infra_master::time::{Date, AccrualPeriod, DayCounter};
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
    /// use infra_master::time::{Date, AccrualPeriod};
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
    /// use infra_master::time::{Date, AccrualPeriod};
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
    /// use infra_master::time::{Date, AccrualPeriod, DayCounter};
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
