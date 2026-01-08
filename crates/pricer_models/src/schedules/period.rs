//! Period definition for scheduled instruments.

use pricer_core::types::time::{Date, DayCountConvention};
use std::fmt;

/// A single accrual period in a schedule.
///
/// Represents one payment period with:
/// - Accrual start and end dates
/// - Payment date
/// - Day count convention for year fraction calculation
///
/// # Examples
///
/// ```
/// use pricer_models::schedules::Period;
/// use pricer_core::types::time::{Date, DayCountConvention};
///
/// let period = Period::new(
///     Date::from_ymd(2024, 1, 15).unwrap(),
///     Date::from_ymd(2024, 7, 15).unwrap(),
///     Date::from_ymd(2024, 7, 17).unwrap(), // Payment 2 days after end
///     DayCountConvention::ActualActual360,
/// );
///
/// assert!((period.year_fraction() - 0.5056).abs() < 0.001); // ~182/360
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Period {
    /// Start date of the accrual period.
    start: Date,
    /// End date of the accrual period.
    end: Date,
    /// Payment date (may differ from end date due to business day adjustments).
    payment: Date,
    /// Day count convention for year fraction calculation.
    day_count: DayCountConvention,
}

impl Period {
    /// Creates a new period with the specified dates and day count convention.
    ///
    /// # Arguments
    ///
    /// * `start` - Start date of the accrual period
    /// * `end` - End date of the accrual period
    /// * `payment` - Payment date
    /// * `day_count` - Day count convention for year fraction calculation
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::Period;
    /// use pricer_core::types::time::{Date, DayCountConvention};
    ///
    /// let period = Period::new(
    ///     Date::from_ymd(2024, 1, 15).unwrap(),
    ///     Date::from_ymd(2024, 7, 15).unwrap(),
    ///     Date::from_ymd(2024, 7, 15).unwrap(),
    ///     DayCountConvention::Thirty360,
    /// );
    /// ```
    #[inline]
    pub fn new(start: Date, end: Date, payment: Date, day_count: DayCountConvention) -> Self {
        Self {
            start,
            end,
            payment,
            day_count,
        }
    }

    /// Creates a new period where payment date equals end date.
    ///
    /// This is a convenience constructor for the common case where
    /// payment occurs on the accrual end date.
    ///
    /// # Arguments
    ///
    /// * `start` - Start date of the accrual period
    /// * `end` - End date of the accrual period (also payment date)
    /// * `day_count` - Day count convention for year fraction calculation
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::Period;
    /// use pricer_core::types::time::{Date, DayCountConvention};
    ///
    /// let period = Period::with_payment_on_end(
    ///     Date::from_ymd(2024, 1, 15).unwrap(),
    ///     Date::from_ymd(2024, 7, 15).unwrap(),
    ///     DayCountConvention::ActualActual365,
    /// );
    ///
    /// assert_eq!(period.end(), period.payment());
    /// ```
    #[inline]
    pub fn with_payment_on_end(start: Date, end: Date, day_count: DayCountConvention) -> Self {
        Self {
            start,
            end,
            payment: end,
            day_count,
        }
    }

    /// Returns the start date of the accrual period.
    #[inline]
    pub fn start(&self) -> Date {
        self.start
    }

    /// Returns the end date of the accrual period.
    #[inline]
    pub fn end(&self) -> Date {
        self.end
    }

    /// Returns the payment date.
    #[inline]
    pub fn payment(&self) -> Date {
        self.payment
    }

    /// Returns the day count convention.
    #[inline]
    pub fn day_count(&self) -> DayCountConvention {
        self.day_count
    }

    /// Calculates the year fraction for this period.
    ///
    /// Uses the period's day count convention to calculate the
    /// year fraction between start and end dates.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::Period;
    /// use pricer_core::types::time::{Date, DayCountConvention};
    ///
    /// // 6 months in 30/360 convention
    /// let period = Period::with_payment_on_end(
    ///     Date::from_ymd(2024, 1, 15).unwrap(),
    ///     Date::from_ymd(2024, 7, 15).unwrap(),
    ///     DayCountConvention::Thirty360,
    /// );
    ///
    /// assert!((period.year_fraction() - 0.5).abs() < 0.001);
    /// ```
    #[inline]
    pub fn year_fraction(&self) -> f64 {
        self.day_count.year_fraction_dates(self.start, self.end)
    }

    /// Returns the number of days in this period.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::Period;
    /// use pricer_core::types::time::{Date, DayCountConvention};
    ///
    /// let period = Period::with_payment_on_end(
    ///     Date::from_ymd(2024, 1, 1).unwrap(),
    ///     Date::from_ymd(2024, 1, 31).unwrap(),
    ///     DayCountConvention::ActualActual365,
    /// );
    ///
    /// assert_eq!(period.days(), 30);
    /// ```
    #[inline]
    pub fn days(&self) -> i64 {
        self.end - self.start
    }

    /// Returns whether this period is valid (end > start).
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::Period;
    /// use pricer_core::types::time::{Date, DayCountConvention};
    ///
    /// let valid = Period::with_payment_on_end(
    ///     Date::from_ymd(2024, 1, 1).unwrap(),
    ///     Date::from_ymd(2024, 7, 1).unwrap(),
    ///     DayCountConvention::ActualActual365,
    /// );
    /// assert!(valid.is_valid());
    ///
    /// let invalid = Period::with_payment_on_end(
    ///     Date::from_ymd(2024, 7, 1).unwrap(),
    ///     Date::from_ymd(2024, 1, 1).unwrap(),
    ///     DayCountConvention::ActualActual365,
    /// );
    /// assert!(!invalid.is_valid());
    /// ```
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.end > self.start
    }

    /// Returns whether this period contains the given date.
    ///
    /// The date is contained if start <= date < end.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::Period;
    /// use pricer_core::types::time::{Date, DayCountConvention};
    ///
    /// let period = Period::with_payment_on_end(
    ///     Date::from_ymd(2024, 1, 1).unwrap(),
    ///     Date::from_ymd(2024, 7, 1).unwrap(),
    ///     DayCountConvention::ActualActual365,
    /// );
    ///
    /// assert!(period.contains(Date::from_ymd(2024, 3, 15).unwrap()));
    /// assert!(period.contains(Date::from_ymd(2024, 1, 1).unwrap())); // Start is included
    /// assert!(!period.contains(Date::from_ymd(2024, 7, 1).unwrap())); // End is excluded
    /// ```
    #[inline]
    pub fn contains(&self, date: Date) -> bool {
        date >= self.start && date < self.end
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Period({} to {}, pay {}, {})",
            self.start, self.end, self.payment, self.day_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_period() -> Period {
        Period::new(
            Date::from_ymd(2024, 1, 15).unwrap(),
            Date::from_ymd(2024, 7, 15).unwrap(),
            Date::from_ymd(2024, 7, 17).unwrap(),
            DayCountConvention::ActualActual360,
        )
    }

    #[test]
    fn test_new() {
        let period = sample_period();
        assert_eq!(period.start(), Date::from_ymd(2024, 1, 15).unwrap());
        assert_eq!(period.end(), Date::from_ymd(2024, 7, 15).unwrap());
        assert_eq!(period.payment(), Date::from_ymd(2024, 7, 17).unwrap());
        assert_eq!(period.day_count(), DayCountConvention::ActualActual360);
    }

    #[test]
    fn test_with_payment_on_end() {
        let period = Period::with_payment_on_end(
            Date::from_ymd(2024, 1, 15).unwrap(),
            Date::from_ymd(2024, 7, 15).unwrap(),
            DayCountConvention::Thirty360,
        );
        assert_eq!(period.end(), period.payment());
    }

    #[test]
    fn test_year_fraction_act_360() {
        let period = Period::with_payment_on_end(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 7, 1).unwrap(),
            DayCountConvention::ActualActual360,
        );
        // 182 days / 360
        let expected = 182.0 / 360.0;
        assert!((period.year_fraction() - expected).abs() < 1e-10);
    }

    #[test]
    fn test_year_fraction_act_365() {
        let period = Period::with_payment_on_end(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 7, 1).unwrap(),
            DayCountConvention::ActualActual365,
        );
        // 182 days / 365
        let expected = 182.0 / 365.0;
        assert!((period.year_fraction() - expected).abs() < 1e-10);
    }

    #[test]
    fn test_year_fraction_thirty_360() {
        let period = Period::with_payment_on_end(
            Date::from_ymd(2024, 1, 15).unwrap(),
            Date::from_ymd(2024, 7, 15).unwrap(),
            DayCountConvention::Thirty360,
        );
        // 6 months = 180/360 = 0.5
        assert!((period.year_fraction() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_days() {
        let period = Period::with_payment_on_end(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 1, 31).unwrap(),
            DayCountConvention::ActualActual365,
        );
        assert_eq!(period.days(), 30);
    }

    #[test]
    fn test_is_valid() {
        let valid = Period::with_payment_on_end(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 7, 1).unwrap(),
            DayCountConvention::ActualActual365,
        );
        assert!(valid.is_valid());

        let same_day = Period::with_payment_on_end(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 1, 1).unwrap(),
            DayCountConvention::ActualActual365,
        );
        assert!(!same_day.is_valid());

        let reversed = Period::with_payment_on_end(
            Date::from_ymd(2024, 7, 1).unwrap(),
            Date::from_ymd(2024, 1, 1).unwrap(),
            DayCountConvention::ActualActual365,
        );
        assert!(!reversed.is_valid());
    }

    #[test]
    fn test_contains() {
        let period = Period::with_payment_on_end(
            Date::from_ymd(2024, 1, 1).unwrap(),
            Date::from_ymd(2024, 7, 1).unwrap(),
            DayCountConvention::ActualActual365,
        );

        // Start is included
        assert!(period.contains(Date::from_ymd(2024, 1, 1).unwrap()));
        // Middle is included
        assert!(period.contains(Date::from_ymd(2024, 3, 15).unwrap()));
        // Day before end is included
        assert!(period.contains(Date::from_ymd(2024, 6, 30).unwrap()));
        // End is excluded
        assert!(!period.contains(Date::from_ymd(2024, 7, 1).unwrap()));
        // Before start is excluded
        assert!(!period.contains(Date::from_ymd(2023, 12, 31).unwrap()));
        // After end is excluded
        assert!(!period.contains(Date::from_ymd(2024, 7, 2).unwrap()));
    }

    #[test]
    fn test_display() {
        let period = sample_period();
        let display = format!("{}", period);
        assert!(display.contains("2024-01-15"));
        assert!(display.contains("2024-07-15"));
        assert!(display.contains("2024-07-17"));
        assert!(display.contains("ACT/360"));
    }

    #[test]
    fn test_clone_and_copy() {
        let period1 = sample_period();
        let period2 = period1; // Copy
        let period3 = period1.clone(); // Clone

        assert_eq!(period1, period2);
        assert_eq!(period1, period3);
    }

    #[test]
    fn test_debug() {
        let period = sample_period();
        let debug_str = format!("{:?}", period);
        assert!(debug_str.contains("Period"));
    }
}
