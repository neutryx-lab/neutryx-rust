//! Schedule generation utilities for trade expansion.
//!
//! This module provides functions to generate payment schedules from
//! start date, tenor, and payment frequency.
//!
//! # Task Coverage
//!
//! - Task 2.1: Schedule generation logic
//! - Task 2.2: Schedule generation unit tests
//!
//! # Requirements Coverage
//!
//! - Requirement 3.2: CF展開ロジック（スケジュール生成）

use std::str::FromStr;

/// Schedule generation error.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleError {
    /// Error message describing what went wrong.
    pub message: String,
}

impl ScheduleError {
    /// Creates a new schedule error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Schedule error: {}", self.message)
    }
}

impl std::error::Error for ScheduleError {}

/// A single accrual period in a payment schedule.
#[derive(Debug, Clone, PartialEq)]
pub struct SchedulePeriod {
    /// Start date of the period
    pub start_date: String,
    /// End date of the period
    pub end_date: String,
    /// Payment date (usually same as end date or adjusted for business days)
    pub payment_date: String,
    /// Year fraction for the period (approximate, based on Act/360 or Act/365)
    pub year_fraction: f64,
}

/// Payment frequency enum for schedule generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaymentFrequency {
    /// Annual payments (once per year).
    Annual,
    /// Semi-annual payments (twice per year).
    SemiAnnual,
    /// Quarterly payments (four times per year).
    Quarterly,
    /// Monthly payments (twelve times per year).
    Monthly,
}

impl PaymentFrequency {
    /// Returns the number of months between payments.
    pub fn months_per_period(&self) -> u32 {
        match self {
            Self::Annual => 12,
            Self::SemiAnnual => 6,
            Self::Quarterly => 3,
            Self::Monthly => 1,
        }
    }
}

impl FromStr for PaymentFrequency {
    type Err = ScheduleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "annual" | "yearly" | "1y" => Ok(Self::Annual),
            "semiannual" | "6m" => Ok(Self::SemiAnnual),
            "quarterly" | "3m" => Ok(Self::Quarterly),
            "monthly" | "1m" => Ok(Self::Monthly),
            _ => Err(ScheduleError::new(format!(
                "Invalid payment frequency: {}",
                s
            ))),
        }
    }
}

/// Day count convention for year fraction calculation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DayCountConvention {
    /// Actual/360 day count convention.
    #[default]
    Act360,
    /// Actual/365 day count convention.
    Act365,
    /// 30/360 day count convention.
    Thirty360,
}

impl FromStr for DayCountConvention {
    type Err = ScheduleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(['/', '_', ' '], "").as_str() {
            "act360" | "actual360" => Ok(Self::Act360),
            "act365" | "actual365" | "act365fixed" => Ok(Self::Act365),
            "thirty360" | "30360" | "bond" => Ok(Self::Thirty360),
            _ => Err(ScheduleError::new(format!(
                "Invalid day count convention: {}",
                s
            ))),
        }
    }
}

/// Tenor representation for schedule generation.
#[derive(Debug, Clone, PartialEq)]
pub struct Tenor {
    /// Number of months
    pub months: u32,
    /// Original string representation
    pub code: String,
}

impl FromStr for Tenor {
    type Err = ScheduleError;

    /// Parses tenor from strings like "3M", "1Y", "5Y", "6M".
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_uppercase();

        if s.ends_with('Y') {
            let years: u32 = s[..s.len() - 1]
                .parse()
                .map_err(|_| ScheduleError::new(format!("Invalid year tenor: {}", s)))?;
            Ok(Self {
                months: years * 12,
                code: s,
            })
        } else if s.ends_with('M') {
            let months: u32 = s[..s.len() - 1]
                .parse()
                .map_err(|_| ScheduleError::new(format!("Invalid month tenor: {}", s)))?;
            Ok(Self { months, code: s })
        } else if s.ends_with('W') {
            // Weeks - approximate to months (4 weeks ≈ 1 month)
            let weeks: u32 = s[..s.len() - 1]
                .parse()
                .map_err(|_| ScheduleError::new(format!("Invalid week tenor: {}", s)))?;
            // For simplicity, treat weeks as fraction of a month
            // 1W ≈ 0 months (handled as special case)
            let months = if weeks >= 4 { weeks / 4 } else { 0 };
            Ok(Self { months, code: s })
        } else if s == "ON" || s == "O/N" {
            // Overnight - special case
            Ok(Self { months: 0, code: s })
        } else {
            Err(ScheduleError::new(format!("Invalid tenor format: {}", s)))
        }
    }
}

/// Simple date representation (year, month, day).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimpleDate {
    /// Year component.
    pub year: i32,
    /// Month component (1-12).
    pub month: u32,
    /// Day component (1-31).
    pub day: u32,
}

impl SimpleDate {
    /// Creates a new date.
    pub fn new(year: i32, month: u32, day: u32) -> Result<Self, ScheduleError> {
        if month < 1 || month > 12 {
            return Err(ScheduleError::new(format!("Invalid month: {}", month)));
        }
        if day < 1 || day > 31 {
            return Err(ScheduleError::new(format!("Invalid day: {}", day)));
        }
        Ok(Self { year, month, day })
    }

    /// Parses a date from ISO 8601 format (YYYY-MM-DD).
    pub fn parse(s: &str) -> Result<Self, ScheduleError> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return Err(ScheduleError::new(format!("Invalid date format: {}", s)));
        }

        let year: i32 = parts[0]
            .parse()
            .map_err(|_| ScheduleError::new(format!("Invalid year: {}", parts[0])))?;
        let month: u32 = parts[1]
            .parse()
            .map_err(|_| ScheduleError::new(format!("Invalid month: {}", parts[1])))?;
        let day: u32 = parts[2]
            .parse()
            .map_err(|_| ScheduleError::new(format!("Invalid day: {}", parts[2])))?;

        Self::new(year, month, day)
    }

    /// Formats the date as ISO 8601 (YYYY-MM-DD).
    pub fn to_iso_string(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Returns the number of days in the given month.
    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            _ => 30, // Fallback
        }
    }

    /// Returns true if this date is the last day of its month.
    pub fn is_end_of_month(&self) -> bool { self.day == Self::days_in_month(self.year, self.month) }

    /// Adds months to this date.
    pub fn add_months(&self, months: u32) -> Self {
        let total_months = (self.year * 12 + self.month as i32 - 1) + months as i32;
        let new_year = total_months / 12;
        let new_month = (total_months % 12) + 1;

        let max_day = Self::days_in_month(new_year, new_month as u32);
        let new_day = self.day.min(max_day);

        Self {
            year: new_year,
            month: new_month as u32,
            day: new_day,
        }
    }

    /// Adds months with end-of-month rule.
    pub fn add_months_with_eom(&self, months: u32, preserve_eom: bool) -> Self {
        let is_eom = self.is_end_of_month();
        let result = self.add_months(months);

        if preserve_eom && is_eom {
            // Adjust to end of the new month
            let max_day = Self::days_in_month(result.year, result.month);
            Self {
                year: result.year,
                month: result.month,
                day: max_day,
            }
        } else {
            result
        }
    }

    /// Returns the number of days between two dates (approximate).
    pub fn days_between(&self, other: &Self) -> i64 {
        // Simple approximation: days since year 0
        let self_days = self.year as i64 * 365 + self.year as i64 / 4 - self.year as i64 / 100
            + self.year as i64 / 400
            + (self.month as i64 - 1) * 30
            + self.day as i64;

        let other_days = other.year as i64 * 365 + other.year as i64 / 4 - other.year as i64 / 100
            + other.year as i64 / 400
            + (other.month as i64 - 1) * 30
            + other.day as i64;

        other_days - self_days
    }
}

/// Generates a payment schedule from start date, tenor, and frequency.
///
/// # Arguments
///
/// * `start_date` - Start date in ISO 8601 format (YYYY-MM-DD)
/// * `tenor` - Tenor string (e.g., "5Y", "3M", "10Y")
/// * `frequency` - Payment frequency (e.g., "Quarterly", "SemiAnnual")
/// * `day_count` - Day count convention (e.g., "Act360", "Act365")
///
/// # Returns
///
/// A vector of schedule periods, or an error if parsing fails.
///
/// # Example
///
/// ```ignore
/// let schedule = generate_schedule("2024-01-15", "5Y", "Quarterly", "Act360")?;
/// assert_eq!(schedule.len(), 20); // 5 years * 4 quarters
/// ```
pub fn generate_schedule(
    start_date: &str,
    tenor: &str,
    frequency: &str,
    day_count: &str,
) -> Result<Vec<SchedulePeriod>, ScheduleError> {
    let start = SimpleDate::parse(start_date)?;
    let tenor = Tenor::from_str(tenor)?;
    let frequency = PaymentFrequency::from_str(frequency)?;
    let day_count = DayCountConvention::from_str(day_count)?;

    // Handle short tenors (< 1 month)
    if tenor.months == 0 {
        // Single period for overnight/week tenors
        return Ok(vec![SchedulePeriod {
            start_date: start.to_iso_string(),
            end_date: start.add_months(1).to_iso_string(),
            payment_date: start.add_months(1).to_iso_string(),
            year_fraction: calculate_year_fraction(1, day_count),
        }]);
    }

    let months_per_period = frequency.months_per_period();

    // Calculate number of periods
    let num_periods = if tenor.months >= months_per_period {
        tenor.months / months_per_period
    } else {
        1 // At least one period
    };

    let mut schedule = Vec::with_capacity(num_periods as usize);
    let mut current_start = start;

    for _i in 0..num_periods {
        let current_end = current_start.add_months_with_eom(months_per_period, true);

        // Year fraction calculation
        let days = current_start.days_between(&current_end);
        let year_fraction = calculate_year_fraction_from_days(days, day_count);

        schedule.push(SchedulePeriod {
            start_date: current_start.to_iso_string(),
            end_date: current_end.to_iso_string(),
            payment_date: current_end.to_iso_string(),
            year_fraction,
        });

        current_start = current_end;
    }

    Ok(schedule)
}

/// Calculates year fraction based on months and day count convention.
fn calculate_year_fraction(months: u32, day_count: DayCountConvention) -> f64 {
    let days = months * 30; // Approximate
    calculate_year_fraction_from_days(days as i64, day_count)
}

/// Calculates year fraction from actual days.
fn calculate_year_fraction_from_days(days: i64, day_count: DayCountConvention) -> f64 {
    match day_count {
        DayCountConvention::Act360 => days as f64 / 360.0,
        DayCountConvention::Act365 => days as f64 / 365.0,
        DayCountConvention::Thirty360 => days as f64 / 360.0,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 2.2: Schedule Generation Tests
    // =========================================================================

    mod tenor_tests {
        use super::*;

        #[test]
        fn test_tenor_parse_years() {
            let tenor = Tenor::from_str("5Y").unwrap();
            assert_eq!(tenor.months, 60);
            assert_eq!(tenor.code, "5Y");
        }

        #[test]
        fn test_tenor_parse_months() {
            let tenor = Tenor::from_str("3M").unwrap();
            assert_eq!(tenor.months, 3);
            assert_eq!(tenor.code, "3M");
        }

        #[test]
        fn test_tenor_parse_lowercase() {
            let tenor = Tenor::from_str("10y").unwrap();
            assert_eq!(tenor.months, 120);
        }

        #[test]
        fn test_tenor_parse_overnight() {
            let tenor = Tenor::from_str("ON").unwrap();
            assert_eq!(tenor.months, 0);
        }

        #[test]
        fn test_tenor_parse_invalid() {
            assert!(Tenor::from_str("INVALID").is_err());
            assert!(Tenor::from_str("5X").is_err());
        }
    }

    mod frequency_tests {
        use super::*;

        #[test]
        fn test_frequency_parse() {
            assert_eq!(
                PaymentFrequency::from_str("Quarterly").unwrap(),
                PaymentFrequency::Quarterly
            );
            assert_eq!(
                PaymentFrequency::from_str("semi-annual").unwrap(),
                PaymentFrequency::SemiAnnual
            );
            assert_eq!(
                PaymentFrequency::from_str("ANNUAL").unwrap(),
                PaymentFrequency::Annual
            );
            assert_eq!(
                PaymentFrequency::from_str("monthly").unwrap(),
                PaymentFrequency::Monthly
            );
        }

        #[test]
        fn test_frequency_months_per_period() {
            assert_eq!(PaymentFrequency::Annual.months_per_period(), 12);
            assert_eq!(PaymentFrequency::SemiAnnual.months_per_period(), 6);
            assert_eq!(PaymentFrequency::Quarterly.months_per_period(), 3);
            assert_eq!(PaymentFrequency::Monthly.months_per_period(), 1);
        }
    }

    mod day_count_tests {
        use super::*;

        #[test]
        fn test_day_count_parse() {
            assert_eq!(
                DayCountConvention::from_str("Act360").unwrap(),
                DayCountConvention::Act360
            );
            assert_eq!(
                DayCountConvention::from_str("act/365").unwrap(),
                DayCountConvention::Act365
            );
            assert_eq!(
                DayCountConvention::from_str("30/360").unwrap(),
                DayCountConvention::Thirty360
            );
        }
    }

    mod simple_date_tests {
        use super::*;

        #[test]
        fn test_date_parse() {
            let date = SimpleDate::parse("2024-01-15").unwrap();
            assert_eq!(date.year, 2024);
            assert_eq!(date.month, 1);
            assert_eq!(date.day, 15);
        }

        #[test]
        fn test_date_to_iso_string() {
            let date = SimpleDate::new(2024, 1, 5).unwrap();
            assert_eq!(date.to_iso_string(), "2024-01-05");
        }

        #[test]
        fn test_date_add_months() {
            let date = SimpleDate::new(2024, 1, 15).unwrap();
            let result = date.add_months(3);
            assert_eq!(result.year, 2024);
            assert_eq!(result.month, 4);
            assert_eq!(result.day, 15);
        }

        #[test]
        fn test_date_add_months_year_rollover() {
            let date = SimpleDate::new(2024, 11, 15).unwrap();
            let result = date.add_months(3);
            assert_eq!(result.year, 2025);
            assert_eq!(result.month, 2);
            assert_eq!(result.day, 15);
        }

        #[test]
        fn test_date_add_months_end_of_month() {
            // Jan 31 + 1 month = Feb 28/29
            let date = SimpleDate::new(2024, 1, 31).unwrap();
            let result = date.add_months(1);
            assert_eq!(result.year, 2024);
            assert_eq!(result.month, 2);
            assert_eq!(result.day, 29); // 2024 is leap year
        }

        #[test]
        fn test_date_is_end_of_month() {
            assert!(SimpleDate::new(2024, 1, 31).unwrap().is_end_of_month());
            assert!(SimpleDate::new(2024, 2, 29).unwrap().is_end_of_month()); // Leap year
            assert!(!SimpleDate::new(2024, 1, 15).unwrap().is_end_of_month());
        }

        #[test]
        fn test_date_add_months_with_eom() {
            let date = SimpleDate::new(2024, 1, 31).unwrap();
            let result = date.add_months_with_eom(1, true);
            // Should be end of Feb
            assert_eq!(result.year, 2024);
            assert_eq!(result.month, 2);
            assert_eq!(result.day, 29); // Leap year
        }

        #[test]
        fn test_leap_year() {
            assert!(SimpleDate::new(2024, 1, 1).unwrap().is_leap_year());
            assert!(!SimpleDate::new(2023, 1, 1).unwrap().is_leap_year());
            assert!(SimpleDate::new(2000, 1, 1).unwrap().is_leap_year());
            assert!(!SimpleDate::new(1900, 1, 1).unwrap().is_leap_year());
        }
    }

    mod schedule_generation_tests {
        use super::*;

        #[test]
        fn test_generate_schedule_5y_quarterly() {
            let schedule = generate_schedule("2024-01-15", "5Y", "Quarterly", "Act360").unwrap();
            assert_eq!(schedule.len(), 20); // 5 years * 4 quarters
        }

        #[test]
        fn test_generate_schedule_1y_semiannual() {
            let schedule = generate_schedule("2024-01-15", "1Y", "SemiAnnual", "Act365").unwrap();
            assert_eq!(schedule.len(), 2); // 2 periods per year
        }

        #[test]
        fn test_generate_schedule_3m_monthly() {
            let schedule = generate_schedule("2024-01-15", "3M", "Monthly", "Act360").unwrap();
            assert_eq!(schedule.len(), 3); // 3 months
        }

        #[test]
        fn test_generate_schedule_dates_correct() {
            let schedule = generate_schedule("2024-01-15", "1Y", "Quarterly", "Act360").unwrap();

            assert_eq!(schedule[0].start_date, "2024-01-15");
            assert_eq!(schedule[0].end_date, "2024-04-15");
            assert_eq!(schedule[0].payment_date, "2024-04-15");

            assert_eq!(schedule[1].start_date, "2024-04-15");
            assert_eq!(schedule[1].end_date, "2024-07-15");

            assert_eq!(schedule[2].start_date, "2024-07-15");
            assert_eq!(schedule[2].end_date, "2024-10-15");

            assert_eq!(schedule[3].start_date, "2024-10-15");
            assert_eq!(schedule[3].end_date, "2025-01-15");
        }

        #[test]
        fn test_generate_schedule_year_fraction() {
            let schedule = generate_schedule("2024-01-15", "1Y", "Quarterly", "Act360").unwrap();

            // Each quarter should be approximately 0.25 (90 days / 360)
            for period in &schedule {
                assert!(
                    period.year_fraction > 0.2 && period.year_fraction < 0.3,
                    "Year fraction {} is outside expected range",
                    period.year_fraction
                );
            }
        }

        #[test]
        fn test_generate_schedule_end_of_month() {
            // Start on Jan 31, end of month should be preserved
            let schedule = generate_schedule("2024-01-31", "1Y", "Quarterly", "Act360").unwrap();

            assert_eq!(schedule[0].start_date, "2024-01-31");
            // Apr 30 (end of April)
            assert_eq!(schedule[0].end_date, "2024-04-30");

            // Jul 31 (end of July)
            assert_eq!(schedule[1].end_date, "2024-07-31");
        }

        #[test]
        fn test_generate_schedule_overnight() {
            let schedule = generate_schedule("2024-01-15", "ON", "Monthly", "Act360").unwrap();
            // Should still produce a schedule with at least one period
            assert!(!schedule.is_empty());
        }

        #[test]
        fn test_generate_schedule_invalid_date() {
            let result = generate_schedule("invalid-date", "5Y", "Quarterly", "Act360");
            assert!(result.is_err());
        }

        #[test]
        fn test_generate_schedule_invalid_tenor() {
            let result = generate_schedule("2024-01-15", "INVALID", "Quarterly", "Act360");
            assert!(result.is_err());
        }

        #[test]
        fn test_generate_schedule_invalid_frequency() {
            let result = generate_schedule("2024-01-15", "5Y", "Invalid", "Act360");
            assert!(result.is_err());
        }

        #[test]
        fn test_generate_schedule_10y_annual() {
            let schedule = generate_schedule("2024-01-15", "10Y", "Annual", "Act365").unwrap();
            assert_eq!(schedule.len(), 10);
        }

        #[test]
        fn test_generate_schedule_30y_semiannual() {
            let schedule = generate_schedule("2024-01-15", "30Y", "SemiAnnual", "Act360").unwrap();
            assert_eq!(schedule.len(), 60); // 30 years * 2
        }
    }
}
