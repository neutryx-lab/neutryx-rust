//! Schedule and ScheduleBuilder implementation.

use super::error::ScheduleError;
use super::frequency::Frequency;
use super::period::Period;
use chrono::{Datelike, Months};
use pricer_core::types::time::{Date, DayCountConvention};

/// A collection of payment periods for financial instruments.
///
/// Contains the complete schedule of periods for instruments like
/// interest rate swaps, bonds, or any scheduled cash flow instrument.
///
/// # Examples
///
/// ```
/// use pricer_models::schedules::{Schedule, ScheduleBuilder, Frequency};
/// use pricer_core::types::time::{Date, DayCountConvention};
///
/// let schedule = ScheduleBuilder::new()
///     .start(Date::from_ymd(2024, 1, 15).unwrap())
///     .end(Date::from_ymd(2026, 1, 15).unwrap())
///     .frequency(Frequency::SemiAnnual)
///     .day_count(DayCountConvention::ActualActual360)
///     .build()
///     .unwrap();
///
/// assert_eq!(schedule.periods().len(), 4);
/// assert_eq!(schedule.payment_dates().len(), 4);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Schedule {
    /// All periods in the schedule.
    periods: Vec<Period>,
    /// All payment dates (derived from periods).
    payment_dates: Vec<Date>,
    /// Accrual start dates for each period.
    accrual_start: Vec<Date>,
    /// Accrual end dates for each period.
    accrual_end: Vec<Date>,
}

impl Schedule {
    /// Creates a new schedule from a list of periods.
    ///
    /// Extracts payment dates and accrual dates from the periods.
    ///
    /// # Arguments
    ///
    /// * `periods` - The list of periods in the schedule
    ///
    /// # Panics
    ///
    /// Panics if periods is empty.
    pub fn new(periods: Vec<Period>) -> Self {
        assert!(
            !periods.is_empty(),
            "Schedule must have at least one period"
        );

        let payment_dates: Vec<Date> = periods.iter().map(|p| p.payment()).collect();
        let accrual_start: Vec<Date> = periods.iter().map(|p| p.start()).collect();
        let accrual_end: Vec<Date> = periods.iter().map(|p| p.end()).collect();

        Self {
            periods,
            payment_dates,
            accrual_start,
            accrual_end,
        }
    }

    /// Returns the periods in the schedule.
    #[inline]
    pub fn periods(&self) -> &[Period] {
        &self.periods
    }

    /// Returns the payment dates.
    #[inline]
    pub fn payment_dates(&self) -> &[Date] {
        &self.payment_dates
    }

    /// Returns the accrual start dates.
    #[inline]
    pub fn accrual_start_dates(&self) -> &[Date] {
        &self.accrual_start
    }

    /// Returns the accrual end dates.
    #[inline]
    pub fn accrual_end_dates(&self) -> &[Date] {
        &self.accrual_end
    }

    /// Returns the number of periods.
    #[inline]
    pub fn len(&self) -> usize {
        self.periods.len()
    }

    /// Returns whether the schedule is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.periods.is_empty()
    }

    /// Returns the start date of the schedule.
    #[inline]
    pub fn start_date(&self) -> Date {
        self.periods[0].start()
    }

    /// Returns the end date of the schedule.
    #[inline]
    pub fn end_date(&self) -> Date {
        self.periods.last().unwrap().end()
    }

    /// Returns the first payment date.
    #[inline]
    pub fn first_payment_date(&self) -> Date {
        self.payment_dates[0]
    }

    /// Returns the last payment date.
    #[inline]
    pub fn last_payment_date(&self) -> Date {
        *self.payment_dates.last().unwrap()
    }

    /// Returns an iterator over the periods.
    pub fn iter(&self) -> impl Iterator<Item = &Period> {
        self.periods.iter()
    }

    /// Calculates the total year fraction of all periods.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::{Schedule, ScheduleBuilder, Frequency};
    /// use pricer_core::types::time::{Date, DayCountConvention};
    ///
    /// let schedule = ScheduleBuilder::new()
    ///     .start(Date::from_ymd(2024, 1, 15).unwrap())
    ///     .end(Date::from_ymd(2026, 1, 15).unwrap())
    ///     .frequency(Frequency::Annual)
    ///     .day_count(DayCountConvention::Thirty360)
    ///     .build()
    ///     .unwrap();
    ///
    /// // 2 years in 30/360
    /// assert!((schedule.total_year_fraction() - 2.0).abs() < 0.01);
    /// ```
    pub fn total_year_fraction(&self) -> f64 {
        self.periods.iter().map(|p| p.year_fraction()).sum()
    }

    /// Returns the period containing the given date, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::{Schedule, ScheduleBuilder, Frequency};
    /// use pricer_core::types::time::{Date, DayCountConvention};
    ///
    /// let schedule = ScheduleBuilder::new()
    ///     .start(Date::from_ymd(2024, 1, 1).unwrap())
    ///     .end(Date::from_ymd(2025, 1, 1).unwrap())
    ///     .frequency(Frequency::Quarterly)
    ///     .day_count(DayCountConvention::ActualActual365)
    ///     .build()
    ///     .unwrap();
    ///
    /// let period = schedule.period_containing(Date::from_ymd(2024, 5, 15).unwrap());
    /// assert!(period.is_some());
    /// ```
    pub fn period_containing(&self, date: Date) -> Option<&Period> {
        self.periods.iter().find(|p| p.contains(date))
    }

    /// Returns the index of the period containing the given date, if any.
    pub fn period_index_containing(&self, date: Date) -> Option<usize> {
        self.periods.iter().position(|p| p.contains(date))
    }
}

/// Builder for constructing schedules with flexible configuration.
///
/// # Examples
///
/// ```
/// use pricer_models::schedules::{ScheduleBuilder, Frequency};
/// use pricer_core::types::time::{Date, DayCountConvention};
///
/// // Simple schedule
/// let schedule = ScheduleBuilder::new()
///     .start(Date::from_ymd(2024, 1, 15).unwrap())
///     .end(Date::from_ymd(2026, 1, 15).unwrap())
///     .frequency(Frequency::Quarterly)
///     .build()
///     .unwrap();
///
/// // Custom day count
/// let schedule_custom = ScheduleBuilder::new()
///     .start(Date::from_ymd(2024, 3, 1).unwrap())
///     .end(Date::from_ymd(2027, 3, 1).unwrap())
///     .frequency(Frequency::SemiAnnual)
///     .day_count(DayCountConvention::ActualActual360)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct ScheduleBuilder {
    start_date: Option<Date>,
    end_date: Option<Date>,
    frequency: Option<Frequency>,
    day_count: DayCountConvention,
}

impl Default for ScheduleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScheduleBuilder {
    /// Creates a new schedule builder with default settings.
    ///
    /// Default day count convention is ACT/365.
    pub fn new() -> Self {
        Self {
            start_date: None,
            end_date: None,
            frequency: None,
            day_count: DayCountConvention::ActualActual365,
        }
    }

    /// Sets the start date of the schedule.
    pub fn start(mut self, date: Date) -> Self {
        self.start_date = Some(date);
        self
    }

    /// Sets the end date of the schedule.
    pub fn end(mut self, date: Date) -> Self {
        self.end_date = Some(date);
        self
    }

    /// Sets the payment frequency.
    pub fn frequency(mut self, freq: Frequency) -> Self {
        self.frequency = Some(freq);
        self
    }

    /// Sets the day count convention.
    pub fn day_count(mut self, dc: DayCountConvention) -> Self {
        self.day_count = dc;
        self
    }

    /// Builds the schedule.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Start date is missing
    /// - End date is missing
    /// - Frequency is missing
    /// - Start date is not before end date
    /// - No periods can be generated
    pub fn build(self) -> Result<Schedule, ScheduleError> {
        let start = self
            .start_date
            .ok_or(ScheduleError::MissingField { field: "start" })?;
        let end = self
            .end_date
            .ok_or(ScheduleError::MissingField { field: "end" })?;
        let frequency = self
            .frequency
            .ok_or(ScheduleError::MissingField { field: "frequency" })?;

        if start >= end {
            return Err(ScheduleError::InvalidDateRange { start, end });
        }

        let periods = self.generate_periods(start, end, frequency)?;

        if periods.is_empty() {
            return Err(ScheduleError::NoPeriods { start, end });
        }

        Ok(Schedule::new(periods))
    }

    /// Generates periods between start and end dates based on frequency.
    fn generate_periods(
        &self,
        start: Date,
        end: Date,
        frequency: Frequency,
    ) -> Result<Vec<Period>, ScheduleError> {
        let mut periods = Vec::new();
        let mut current_start = start;

        while current_start < end {
            let current_end = self.advance_date(current_start, frequency)?;

            // Cap at end date
            let period_end = if current_end > end { end } else { current_end };

            // Payment on end date (simplified - no business day adjustment for now)
            let payment = period_end;

            let period = Period::new(current_start, period_end, payment, self.day_count);
            periods.push(period);

            current_start = period_end;
        }

        Ok(periods)
    }

    /// Advances a date by the frequency interval.
    fn advance_date(&self, date: Date, frequency: Frequency) -> Result<Date, ScheduleError> {
        let inner = date.into_inner();

        let new_date = match frequency {
            Frequency::Annual => inner.checked_add_months(Months::new(12)).ok_or_else(|| {
                ScheduleError::DateOverflow {
                    reason: "Adding 12 months overflowed".to_string(),
                }
            })?,
            Frequency::SemiAnnual => inner.checked_add_months(Months::new(6)).ok_or_else(|| {
                ScheduleError::DateOverflow {
                    reason: "Adding 6 months overflowed".to_string(),
                }
            })?,
            Frequency::Quarterly => inner.checked_add_months(Months::new(3)).ok_or_else(|| {
                ScheduleError::DateOverflow {
                    reason: "Adding 3 months overflowed".to_string(),
                }
            })?,
            Frequency::Monthly => inner.checked_add_months(Months::new(1)).ok_or_else(|| {
                ScheduleError::DateOverflow {
                    reason: "Adding 1 month overflowed".to_string(),
                }
            })?,
            Frequency::Weekly => inner + chrono::Days::new(7),
            Frequency::Daily => inner + chrono::Days::new(1),
        };

        Date::from_ymd(new_date.year(), new_date.month(), new_date.day()).map_err(|_| {
            ScheduleError::DateOverflow {
                reason: format!("Invalid date after advancing: {:?}", new_date),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_new() {
        let periods = vec![
            Period::with_payment_on_end(
                Date::from_ymd(2024, 1, 1).unwrap(),
                Date::from_ymd(2024, 7, 1).unwrap(),
                DayCountConvention::ActualActual365,
            ),
            Period::with_payment_on_end(
                Date::from_ymd(2024, 7, 1).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                DayCountConvention::ActualActual365,
            ),
        ];

        let schedule = Schedule::new(periods);
        assert_eq!(schedule.len(), 2);
        assert!(!schedule.is_empty());
    }

    #[test]
    fn test_schedule_dates() {
        let periods = vec![
            Period::with_payment_on_end(
                Date::from_ymd(2024, 1, 1).unwrap(),
                Date::from_ymd(2024, 7, 1).unwrap(),
                DayCountConvention::ActualActual365,
            ),
            Period::with_payment_on_end(
                Date::from_ymd(2024, 7, 1).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                DayCountConvention::ActualActual365,
            ),
        ];

        let schedule = Schedule::new(periods);

        assert_eq!(schedule.start_date(), Date::from_ymd(2024, 1, 1).unwrap());
        assert_eq!(schedule.end_date(), Date::from_ymd(2025, 1, 1).unwrap());
        assert_eq!(
            schedule.first_payment_date(),
            Date::from_ymd(2024, 7, 1).unwrap()
        );
        assert_eq!(
            schedule.last_payment_date(),
            Date::from_ymd(2025, 1, 1).unwrap()
        );
    }

    #[test]
    fn test_schedule_total_year_fraction() {
        let periods = vec![
            Period::with_payment_on_end(
                Date::from_ymd(2024, 1, 15).unwrap(),
                Date::from_ymd(2024, 7, 15).unwrap(),
                DayCountConvention::Thirty360,
            ),
            Period::with_payment_on_end(
                Date::from_ymd(2024, 7, 15).unwrap(),
                Date::from_ymd(2025, 1, 15).unwrap(),
                DayCountConvention::Thirty360,
            ),
        ];

        let schedule = Schedule::new(periods);
        // 2 x 0.5 year = 1.0 year
        assert!((schedule.total_year_fraction() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_schedule_period_containing() {
        let periods = vec![
            Period::with_payment_on_end(
                Date::from_ymd(2024, 1, 1).unwrap(),
                Date::from_ymd(2024, 4, 1).unwrap(),
                DayCountConvention::ActualActual365,
            ),
            Period::with_payment_on_end(
                Date::from_ymd(2024, 4, 1).unwrap(),
                Date::from_ymd(2024, 7, 1).unwrap(),
                DayCountConvention::ActualActual365,
            ),
        ];

        let schedule = Schedule::new(periods);

        // Date in first period
        let period = schedule.period_containing(Date::from_ymd(2024, 2, 15).unwrap());
        assert!(period.is_some());
        assert_eq!(period.unwrap().start(), Date::from_ymd(2024, 1, 1).unwrap());

        // Date in second period
        let period = schedule.period_containing(Date::from_ymd(2024, 5, 15).unwrap());
        assert!(period.is_some());
        assert_eq!(period.unwrap().start(), Date::from_ymd(2024, 4, 1).unwrap());

        // Date outside schedule
        let period = schedule.period_containing(Date::from_ymd(2024, 8, 1).unwrap());
        assert!(period.is_none());
    }

    #[test]
    fn test_schedule_period_index() {
        let periods = vec![
            Period::with_payment_on_end(
                Date::from_ymd(2024, 1, 1).unwrap(),
                Date::from_ymd(2024, 4, 1).unwrap(),
                DayCountConvention::ActualActual365,
            ),
            Period::with_payment_on_end(
                Date::from_ymd(2024, 4, 1).unwrap(),
                Date::from_ymd(2024, 7, 1).unwrap(),
                DayCountConvention::ActualActual365,
            ),
        ];

        let schedule = Schedule::new(periods);

        assert_eq!(
            schedule.period_index_containing(Date::from_ymd(2024, 2, 15).unwrap()),
            Some(0)
        );
        assert_eq!(
            schedule.period_index_containing(Date::from_ymd(2024, 5, 15).unwrap()),
            Some(1)
        );
        assert_eq!(
            schedule.period_index_containing(Date::from_ymd(2024, 8, 1).unwrap()),
            None
        );
    }

    // ScheduleBuilder tests

    #[test]
    fn test_builder_quarterly() {
        let schedule = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 1).unwrap())
            .end(Date::from_ymd(2025, 1, 1).unwrap())
            .frequency(Frequency::Quarterly)
            .build()
            .unwrap();

        assert_eq!(schedule.len(), 4);
        assert_eq!(schedule.start_date(), Date::from_ymd(2024, 1, 1).unwrap());
        assert_eq!(schedule.end_date(), Date::from_ymd(2025, 1, 1).unwrap());
    }

    #[test]
    fn test_builder_semi_annual() {
        let schedule = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 15).unwrap())
            .end(Date::from_ymd(2026, 1, 15).unwrap())
            .frequency(Frequency::SemiAnnual)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        assert_eq!(schedule.len(), 4);
    }

    #[test]
    fn test_builder_annual() {
        let schedule = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 3, 1).unwrap())
            .end(Date::from_ymd(2027, 3, 1).unwrap())
            .frequency(Frequency::Annual)
            .build()
            .unwrap();

        assert_eq!(schedule.len(), 3);
    }

    #[test]
    fn test_builder_monthly() {
        let schedule = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 1).unwrap())
            .end(Date::from_ymd(2024, 7, 1).unwrap())
            .frequency(Frequency::Monthly)
            .build()
            .unwrap();

        assert_eq!(schedule.len(), 6);
    }

    #[test]
    fn test_builder_missing_start() {
        let result = ScheduleBuilder::new()
            .end(Date::from_ymd(2025, 1, 1).unwrap())
            .frequency(Frequency::Quarterly)
            .build();

        assert!(matches!(
            result,
            Err(ScheduleError::MissingField { field: "start" })
        ));
    }

    #[test]
    fn test_builder_missing_end() {
        let result = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 1).unwrap())
            .frequency(Frequency::Quarterly)
            .build();

        assert!(matches!(
            result,
            Err(ScheduleError::MissingField { field: "end" })
        ));
    }

    #[test]
    fn test_builder_missing_frequency() {
        let result = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 1).unwrap())
            .end(Date::from_ymd(2025, 1, 1).unwrap())
            .build();

        assert!(matches!(
            result,
            Err(ScheduleError::MissingField { field: "frequency" })
        ));
    }

    #[test]
    fn test_builder_invalid_date_range() {
        let result = ScheduleBuilder::new()
            .start(Date::from_ymd(2025, 1, 1).unwrap())
            .end(Date::from_ymd(2024, 1, 1).unwrap())
            .frequency(Frequency::Quarterly)
            .build();

        assert!(matches!(
            result,
            Err(ScheduleError::InvalidDateRange { .. })
        ));
    }

    #[test]
    fn test_builder_same_start_end() {
        let result = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 1).unwrap())
            .end(Date::from_ymd(2024, 1, 1).unwrap())
            .frequency(Frequency::Quarterly)
            .build();

        assert!(matches!(
            result,
            Err(ScheduleError::InvalidDateRange { .. })
        ));
    }

    #[test]
    fn test_builder_partial_period() {
        // 5 months with quarterly frequency = 1 full + 1 partial
        let schedule = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 1).unwrap())
            .end(Date::from_ymd(2024, 6, 1).unwrap())
            .frequency(Frequency::Quarterly)
            .build()
            .unwrap();

        assert_eq!(schedule.len(), 2);
        // Last period should end at the schedule end date
        assert_eq!(schedule.end_date(), Date::from_ymd(2024, 6, 1).unwrap());
    }

    #[test]
    fn test_builder_default_day_count() {
        let schedule = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 1).unwrap())
            .end(Date::from_ymd(2025, 1, 1).unwrap())
            .frequency(Frequency::Annual)
            .build()
            .unwrap();

        // Default is ACT/365
        let period = &schedule.periods()[0];
        assert_eq!(period.day_count(), DayCountConvention::ActualActual365);
    }

    #[test]
    fn test_builder_custom_day_count() {
        let schedule = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 1).unwrap())
            .end(Date::from_ymd(2025, 1, 1).unwrap())
            .frequency(Frequency::Annual)
            .day_count(DayCountConvention::Thirty360)
            .build()
            .unwrap();

        let period = &schedule.periods()[0];
        assert_eq!(period.day_count(), DayCountConvention::Thirty360);
    }

    #[test]
    fn test_schedule_iter() {
        let schedule = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 1).unwrap())
            .end(Date::from_ymd(2025, 1, 1).unwrap())
            .frequency(Frequency::Quarterly)
            .build()
            .unwrap();

        let mut count = 0;
        for _period in schedule.iter() {
            count += 1;
        }
        assert_eq!(count, 4);
    }

    #[test]
    fn test_schedule_clone() {
        let schedule1 = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 1).unwrap())
            .end(Date::from_ymd(2025, 1, 1).unwrap())
            .frequency(Frequency::Quarterly)
            .build()
            .unwrap();

        let schedule2 = schedule1.clone();
        assert_eq!(schedule1, schedule2);
    }

    #[test]
    fn test_schedule_debug() {
        let schedule = ScheduleBuilder::new()
            .start(Date::from_ymd(2024, 1, 1).unwrap())
            .end(Date::from_ymd(2025, 1, 1).unwrap())
            .frequency(Frequency::Quarterly)
            .build()
            .unwrap();

        let debug_str = format!("{:?}", schedule);
        assert!(debug_str.contains("Schedule"));
    }

    #[test]
    #[should_panic(expected = "Schedule must have at least one period")]
    fn test_schedule_new_empty_panics() {
        Schedule::new(vec![]);
    }
}
