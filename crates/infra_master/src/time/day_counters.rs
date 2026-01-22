//! Day count convention definitions.
//!
//! This module provides ISDA-standard day count conventions for
//! year fraction calculations in financial instruments.
//!
//! # Examples
//!
//! ```
//! use infra_master::time::{Date, DayCounter};
//!
//! let start = Date::from_ymd(2024, 1, 1).unwrap();
//! let end = Date::from_ymd(2024, 7, 1).unwrap();
//!
//! let yf = DayCounter::Actual365Fixed.year_fraction(start, end);
//! assert!((yf - 0.4986).abs() < 0.001);
//! ```

use std::{fmt, str::FromStr};

use chrono::{Datelike, NaiveDate};

use super::types::Date;

/// Day count convention for interest calculations.
///
/// Also known as day count fraction or accrual factor.
/// Provides ISDA-standard day count conventions for calculating
/// year fractions between dates in financial instruments.
///
/// Uses static dispatch (enum + match) for optimal performance,
/// avoiding VTable lookup costs in hot paths.
///
/// # Variants
///
/// - `Actual360`: Actual days / 360
/// - `Actual365Fixed`: Actual days / 365 (standard for derivatives)
/// - `Actual36525`: Actual days / 365.25
/// - `ActualActualIsda`: ISDA actual/actual
/// - `Thirty360Bond`: 30/360 US Bond Basis
/// - `Thirty360European`: 30/360 European
/// - `ThirtyE360Isda`: 30E/360 ISDA
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DayCounter {
    /// Actual/360
    ///
    /// Used in money market instruments, US Treasury bills,
    /// and LIBOR-based instruments.
    Actual360,

    /// Actual/365 Fixed
    ///
    /// Used in most derivatives markets, UK gilts,
    /// and Japanese government bonds.
    #[default]
    Actual365Fixed,

    /// Actual/365.25
    ///
    /// Uses 365.25 as the denominator to account for leap years.
    Actual36525,

    /// Actual/Actual (ISDA)
    ///
    /// ISDA standard actual/actual convention.
    ActualActualIsda,

    /// 30/360 (Bond Basis)
    ///
    /// US corporate and agency bonds convention.
    /// Each month is treated as having 30 days.
    Thirty360Bond,

    /// 30/360 (European)
    ///
    /// European convention where both start and end days
    /// are capped at 30.
    Thirty360European,

    /// 30E/360 (ISDA)
    ///
    /// ISDA variant of 30/360.
    ThirtyE360Isda,
}

impl DayCounter {
    /// Returns the standard convention name.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::DayCounter;
    ///
    /// assert_eq!(DayCounter::Actual365Fixed.name(), "ACT/365");
    /// assert_eq!(DayCounter::Actual360.name(), "ACT/360");
    /// assert_eq!(DayCounter::Thirty360Bond.name(), "30/360");
    /// ```
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            DayCounter::Actual360 => "ACT/360",
            DayCounter::Actual365Fixed => "ACT/365",
            DayCounter::Actual36525 => "ACT/365.25",
            DayCounter::ActualActualIsda => "ACT/ACT ISDA",
            DayCounter::Thirty360Bond => "30/360",
            DayCounter::Thirty360European => "30E/360",
            DayCounter::ThirtyE360Isda => "30E/360 ISDA",
        }
    }

    /// Calculate the year fraction between two dates.
    ///
    /// Returns a negative value when start > end instead of panicking.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::{Date, DayCounter};
    ///
    /// let start = Date::from_ymd(2024, 1, 1).unwrap();
    /// let end = Date::from_ymd(2024, 7, 1).unwrap();
    ///
    /// let yf = DayCounter::Actual365Fixed.year_fraction(start, end);
    /// assert!((yf - 182.0 / 365.0).abs() < 1e-10);
    ///
    /// // Reversed dates return negative value
    /// let yf_neg = DayCounter::Actual365Fixed.year_fraction(end, start);
    /// assert!((yf_neg + 182.0 / 365.0).abs() < 1e-10);
    /// ```
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Day counts fit in f64 mantissa
    pub fn year_fraction(self, start: Date, end: Date) -> f64 {
        let days = end - start; // Returns i64, can be negative

        match self {
            Self::Actual360 => days as f64 / 360.0,
            Self::Actual365Fixed => days as f64 / 365.0,
            Self::Actual36525 => days as f64 / 365.25,
            Self::ActualActualIsda => days as f64 / 365.25,
            Self::Thirty360Bond | Self::Thirty360European | Self::ThirtyE360Isda => {
                // For 30/360, we need to handle negative direction
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
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::time::{Date, DayCounter};
    ///
    /// let start = Date::from_ymd(2024, 1, 1).unwrap();
    /// let end = Date::from_ymd(2024, 1, 11).unwrap();
    ///
    /// assert_eq!(DayCounter::Actual365Fixed.day_count(start, end), 10);
    /// ```
    #[must_use]
    pub fn day_count(self, start: Date, end: Date) -> i64 { end - start }

    /// Calculate 30/360 day count.
    #[allow(clippy::cast_possible_wrap)] // Month/day values are always small
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

impl FromStr for DayCounter {
    type Err = String;

    /// Parses day count convention from string (case-insensitive).
    ///
    /// Supports multiple aliases for each convention.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().replace(['/', ' ', '.'], "").as_str() {
            "ACT360" | "ACTUAL360" | "A360" => Ok(DayCounter::Actual360),
            "ACT365" | "ACTUAL365" | "A365" | "ACT365FIXED" | "ACTUAL365FIXED" => {
                Ok(DayCounter::Actual365Fixed)
            }
            "ACT36525" | "ACTUAL36525" => Ok(DayCounter::Actual36525),
            "ACTACTISDA" | "ACTUALACTUALISDA" | "ACTACT" => Ok(DayCounter::ActualActualIsda),
            "30360" | "THIRTY360" | "30360BOND" => Ok(DayCounter::Thirty360Bond),
            "30E360" | "30360EUROPEAN" => Ok(DayCounter::Thirty360European),
            "30E360ISDA" => Ok(DayCounter::ThirtyE360Isda),
            _ => Err(format!("Unknown day count convention: {}", s)),
        }
    }
}

impl fmt::Display for DayCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.name()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actual_360() {
        let start = Date::from_ymd(2026, 1, 1).unwrap();
        let end = Date::from_ymd(2026, 4, 1).unwrap();
        let yf = DayCounter::Actual360.year_fraction(start, end);
        assert!((yf - 0.25).abs() < 0.01); // ~90 days / 360
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
        assert_eq!(DayCounter::Thirty360Bond.name(), "30/360");
        assert_eq!(DayCounter::Thirty360European.name(), "30E/360");
        assert_eq!(DayCounter::ThirtyE360Isda.name(), "30E/360 ISDA");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", DayCounter::Actual365Fixed), "ACT/365");
        assert_eq!(format!("{}", DayCounter::Thirty360Bond), "30/360");
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
        // 182 days / 365 ≈ 0.4986
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
        ] {
            assert_eq!(dcc.year_fraction(date, date), 0.0);
        }
    }

    #[test]
    fn test_thirty_360_bond() {
        // 2024-01-31 to 2024-03-31
        // d1 = 31 -> 30, d2 = 31, d1_adj = 30 -> 30
        // Months: 2, Days: 0 => 60 days / 360 = 0.1667
        let start = Date::from_ymd(2024, 1, 31).unwrap();
        let end = Date::from_ymd(2024, 3, 31).unwrap();

        let yf = DayCounter::Thirty360Bond.year_fraction(start, end);
        assert!((yf - 60.0 / 360.0).abs() < 1e-10);
    }

    #[test]
    fn test_thirty_360_european() {
        // Both d1 and d2 are capped at 30
        let start = Date::from_ymd(2024, 1, 31).unwrap();
        let end = Date::from_ymd(2024, 3, 31).unwrap();

        let yf = DayCounter::Thirty360European.year_fraction(start, end);
        // d1 = 30, d2 = 30, 2 months = 60 days / 360
        assert!((yf - 60.0 / 360.0).abs() < 1e-10);
    }

    #[test]
    fn test_thirty_360_bond_vs_european() {
        // When d1 < 30, 30/360 Bond doesn't cap d2
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2024, 3, 31).unwrap();

        let bond_yf = DayCounter::Thirty360Bond.year_fraction(start, end);
        let euro_yf = DayCounter::Thirty360European.year_fraction(start, end);

        // Bond: d1=15, d2=31 -> days = 2*30 + (31-15) = 76
        // Euro: d1=15, d2=30 -> days = 2*30 + (30-15) = 75
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
        set.insert(DayCounter::Actual365Fixed); // Duplicate
        assert_eq!(set.len(), 2);
    }
}
