//! Accrual period definitions.
//!
//! This module provides the Period struct for representing
//! a single accrual period in financial instruments.
//!
//! # Examples
//!
//! ```
//! use infra_master::{Date, Period, DayCountConvention};
//!
//! let start = Date::from_ymd(2024, 1, 1).unwrap();
//! let end = Date::from_ymd(2024, 4, 1).unwrap();
//! let payment = Date::from_ymd(2024, 4, 3).unwrap();
//!
//! let period = Period::new(start, end, payment);
//! let yf = period.year_fraction(DayCountConvention::Actual365Fixed);
//! assert!((yf - 0.2493).abs() < 0.001);
//! ```

use crate::{Date, DayCountConvention};

/// A single accrual period.
///
/// Represents a period between two dates with an associated payment date.
/// Used for calculating interest accruals in bonds, swaps, and other
/// fixed income instruments.
///
/// # Examples
///
/// ```
/// use infra_master::{Date, Period, DayCountConvention};
///
/// let start = Date::from_ymd(2024, 1, 1).unwrap();
/// let end = Date::from_ymd(2024, 7, 1).unwrap();
/// let payment = Date::from_ymd(2024, 7, 3).unwrap();
///
/// let period = Period::new(start, end, payment);
/// assert_eq!(period.accrual_days(), 182);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Period {
    /// Start date of the accrual period
    pub start: Date,
    /// End date of the accrual period
    pub end: Date,
    /// Payment date for this period
    pub payment: Date,
}

impl Period {
    /// Creates a new Period.
    ///
    /// # Arguments
    /// * `start` - Start date of the accrual period
    /// * `end` - End date of the accrual period
    /// * `payment` - Payment date for this period
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::{Date, Period};
    ///
    /// let period = Period::new(
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
    /// use infra_master::{Date, Period};
    ///
    /// let period = Period::new(
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
    /// use infra_master::{Date, Period, DayCountConvention};
    ///
    /// let period = Period::new(
    ///     Date::from_ymd(2024, 1, 1).unwrap(),
    ///     Date::from_ymd(2024, 7, 1).unwrap(),
    ///     Date::from_ymd(2024, 7, 3).unwrap(),
    /// );
    ///
    /// let yf = period.year_fraction(DayCountConvention::Actual365Fixed);
    /// assert!((yf - 182.0 / 365.0).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn year_fraction(&self, day_count: DayCountConvention) -> f64 {
        day_count.year_fraction_dates(self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = Date::from_ymd(2024, 7, 1).unwrap();
        let payment = Date::from_ymd(2024, 7, 3).unwrap();

        let period = Period::new(start, end, payment);
        assert_eq!(period.start, start);
        assert_eq!(period.end, end);
        assert_eq!(period.payment, payment);
    }

    #[test]
    fn test_accrual_days() {
        let period = Period::new(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 1, 11).unwrap(),
            Date::from_ymd(2024, 1, 13).unwrap(),
        );
        assert_eq!(period.accrual_days(), 10);
    }

    #[test]
    fn test_year_fraction_act365() {
        let period = Period::new(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 7, 1).unwrap(),
            Date::from_ymd(2024, 7, 3).unwrap(),
        );
        let yf = period.year_fraction(DayCountConvention::Actual365Fixed);
        assert!((yf - 182.0 / 365.0).abs() < 1e-10);
    }

    #[test]
    fn test_year_fraction_act360() {
        let period = Period::new(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 7, 1).unwrap(),
            Date::from_ymd(2024, 7, 3).unwrap(),
        );
        let yf = period.year_fraction(DayCountConvention::Actual360);
        assert!((yf - 182.0 / 360.0).abs() < 1e-10);
    }

    #[test]
    fn test_clone() {
        let period = Period::new(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 7, 1).unwrap(),
            Date::from_ymd(2024, 7, 3).unwrap(),
        );
        let cloned = period.clone();
        assert_eq!(period, cloned);
    }
}
