//! Roll schedule generation for financial instruments (IRS, bonds, etc.).

use chrono::{Datelike, Months, NaiveDate};

use super::{error::TimeError, frequency::Frequency, types::Date};

/// Roll convention for schedule generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RollConvention {
    /// Standard: use the day from the effective date.
    Standard,
    /// Roll on the 29th (or month end if fewer days).
    Day29th,
    /// Roll on the 30th (or month end if fewer days).
    Day30th,
    /// End of month.
    EndOfMonth,
}

/// Stub period type for schedule generation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum StubType {
    /// Short stub period (default).
    #[default]
    Short,
    /// Long stub period.
    Long,
    /// No stub (schedule must divide evenly).
    None,
}

/// Calculate the n-th roll date from a first roll date.
///
/// For monthly-based frequencies, advances by the appropriate number of months
/// and applies the roll convention. For weekly/daily frequencies, advances by
/// the corresponding number of days.
#[must_use]
pub fn get_roll_date(
    first_roll: Date,
    frequency: Frequency,
    roll_convention: RollConvention,
    n: i32,
) -> Date {
    if frequency == Frequency::None {
        return first_roll;
    }

    if frequency == Frequency::Weekly {
        return first_roll + (i64::from(n) * 7);
    }

    if frequency == Frequency::Daily {
        return first_roll + i64::from(n);
    }

    // Monthly-based frequencies
    let months_per_period = frequency.months_per_period();
    let total_months = i64::from(n) * i64::from(months_per_period);

    let naive = first_roll.into_inner();
    let shifted = if total_months >= 0 {
        naive
            .checked_add_months(Months::new(total_months as u32))
            .unwrap_or(naive)
    } else {
        naive
            .checked_sub_months(Months::new((-total_months) as u32))
            .unwrap_or(naive)
    };

    // Apply roll convention
    let adjusted = match roll_convention {
        RollConvention::Standard => {
            let target_day = first_roll.day();
            let last_day = last_day_of_month_naive(shifted.year(), shifted.month());
            let day = target_day.min(last_day);
            NaiveDate::from_ymd_opt(shifted.year(), shifted.month(), day).unwrap_or(shifted)
        }
        RollConvention::Day29th => {
            let last_day = last_day_of_month_naive(shifted.year(), shifted.month());
            let day = 29u32.min(last_day);
            NaiveDate::from_ymd_opt(shifted.year(), shifted.month(), day).unwrap_or(shifted)
        }
        RollConvention::Day30th => {
            let last_day = last_day_of_month_naive(shifted.year(), shifted.month());
            let day = 30u32.min(last_day);
            NaiveDate::from_ymd_opt(shifted.year(), shifted.month(), day).unwrap_or(shifted)
        }
        RollConvention::EndOfMonth => {
            let last_day = last_day_of_month_naive(shifted.year(), shifted.month());
            NaiveDate::from_ymd_opt(shifted.year(), shifted.month(), last_day).unwrap_or(shifted)
        }
    };

    Date::from_naive(adjusted)
}

/// Generate a roll schedule from effective to termination date.
///
/// Returns a vector of dates starting with `effective` and ending with
/// `termination`. Intermediate dates are determined by the frequency and
/// roll convention. Stub periods at the start are controlled by `first_stub`.
pub fn create_roll_schedule(
    effective: Date,
    termination: Date,
    frequency: Frequency,
    roll_convention: RollConvention,
    first_stub: StubType,
    _last_stub: StubType,
) -> Result<Vec<Date>, TimeError> {
    if effective >= termination {
        return Err(TimeError::ScheduleError(
            "Effective date must be before termination date".into(),
        ));
    }
    if frequency == Frequency::None {
        return Ok(vec![effective, termination]);
    }

    // Generate backward from termination
    let mut dates = vec![termination];
    let mut n = -1i32;
    loop {
        let date = get_roll_date(termination, frequency, roll_convention, n);
        if date <= effective {
            break;
        }
        dates.push(date);
        n -= 1;
    }

    // Add effective at front
    dates.push(effective);
    dates.reverse();
    dates.dedup();

    // Handle first stub
    if dates.len() > 2 && first_stub == StubType::Long {
        // Merge the first stub: remove the second date
        dates.remove(1);
    }

    Ok(dates)
}

/// Return the last day number for the given year/month.
fn last_day_of_month_naive(year: i32, month: u32) -> u32 {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .and_then(|d| d.pred_opt())
            .map_or(31, |d| d.day())
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .and_then(|d| d.pred_opt())
            .map_or(31, |d| d.day())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roll_convention_standard() {
        let first_roll = Date::from_ymd(2024, 3, 15).unwrap();
        let d = get_roll_date(first_roll, Frequency::Monthly, RollConvention::Standard, 1);
        assert_eq!(d, Date::from_ymd(2024, 4, 15).unwrap());
    }

    #[test]
    fn test_roll_convention_eom() {
        let first_roll = Date::from_ymd(2024, 1, 31).unwrap();
        let d = get_roll_date(
            first_roll,
            Frequency::Monthly,
            RollConvention::EndOfMonth,
            1,
        );
        assert_eq!(d, Date::from_ymd(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_roll_convention_day30() {
        let first_roll = Date::from_ymd(2024, 1, 31).unwrap();
        let d = get_roll_date(first_roll, Frequency::Monthly, RollConvention::Day30th, 1);
        assert_eq!(d, Date::from_ymd(2024, 2, 29).unwrap());

        let d2 = get_roll_date(first_roll, Frequency::Monthly, RollConvention::Day30th, 2);
        assert_eq!(d2, Date::from_ymd(2024, 3, 30).unwrap());
    }

    #[test]
    fn test_roll_quarterly() {
        let first_roll = Date::from_ymd(2024, 3, 20).unwrap();
        let d = get_roll_date(
            first_roll,
            Frequency::Quarterly,
            RollConvention::Standard,
            1,
        );
        assert_eq!(d, Date::from_ymd(2024, 6, 20).unwrap());
    }

    #[test]
    fn test_roll_semiannual() {
        let first_roll = Date::from_ymd(2024, 1, 15).unwrap();
        let d = get_roll_date(
            first_roll,
            Frequency::SemiAnnual,
            RollConvention::Standard,
            1,
        );
        assert_eq!(d, Date::from_ymd(2024, 7, 15).unwrap());
    }

    #[test]
    fn test_roll_annual() {
        let first_roll = Date::from_ymd(2024, 6, 15).unwrap();
        let d = get_roll_date(first_roll, Frequency::Annual, RollConvention::Standard, 1);
        assert_eq!(d, Date::from_ymd(2025, 6, 15).unwrap());
    }

    #[test]
    fn test_roll_weekly() {
        let first_roll = Date::from_ymd(2024, 1, 15).unwrap();
        let d = get_roll_date(first_roll, Frequency::Weekly, RollConvention::Standard, 1);
        assert_eq!(d, Date::from_ymd(2024, 1, 22).unwrap());
    }

    #[test]
    fn test_roll_backward() {
        let first_roll = Date::from_ymd(2024, 6, 15).unwrap();
        let d = get_roll_date(first_roll, Frequency::Monthly, RollConvention::Standard, -1);
        assert_eq!(d, Date::from_ymd(2024, 5, 15).unwrap());
    }

    #[test]
    fn test_schedule_quarterly_1y() {
        let eff = Date::from_ymd(2024, 1, 15).unwrap();
        let term = Date::from_ymd(2025, 1, 15).unwrap();
        let schedule = create_roll_schedule(
            eff,
            term,
            Frequency::Quarterly,
            RollConvention::Standard,
            StubType::Short,
            StubType::Short,
        )
        .unwrap();
        assert_eq!(schedule.len(), 5); // eff + 4 quarterly dates
        assert_eq!(schedule[0], eff);
        assert_eq!(schedule[4], term);
    }

    #[test]
    fn test_schedule_semiannual_2y() {
        let eff = Date::from_ymd(2024, 3, 15).unwrap();
        let term = Date::from_ymd(2026, 3, 15).unwrap();
        let schedule = create_roll_schedule(
            eff,
            term,
            Frequency::SemiAnnual,
            RollConvention::Standard,
            StubType::Short,
            StubType::Short,
        )
        .unwrap();
        assert_eq!(schedule.len(), 5); // eff + 4 semi-annual dates
        assert_eq!(schedule[0], eff);
        assert_eq!(schedule[4], term);
    }

    #[test]
    fn test_schedule_invalid_dates() {
        let d1 = Date::from_ymd(2025, 1, 1).unwrap();
        let d2 = Date::from_ymd(2024, 1, 1).unwrap();
        let result = create_roll_schedule(
            d1,
            d2,
            Frequency::Monthly,
            RollConvention::Standard,
            StubType::Short,
            StubType::Short,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_schedule_zero_coupon() {
        let eff = Date::from_ymd(2024, 1, 1).unwrap();
        let term = Date::from_ymd(2025, 1, 1).unwrap();
        let schedule = create_roll_schedule(
            eff,
            term,
            Frequency::None,
            RollConvention::Standard,
            StubType::Short,
            StubType::Short,
        )
        .unwrap();
        assert_eq!(schedule, vec![eff, term]);
    }

    #[test]
    fn test_schedule_with_stub() {
        // 5M tenor with quarterly frequency -> short front stub
        let eff = Date::from_ymd(2024, 1, 15).unwrap();
        let term = Date::from_ymd(2024, 6, 15).unwrap();
        let schedule = create_roll_schedule(
            eff,
            term,
            Frequency::Quarterly,
            RollConvention::Standard,
            StubType::Short,
            StubType::Short,
        )
        .unwrap();
        // Should be: Jan15, Mar15, Jun15
        assert_eq!(schedule.len(), 3);
        assert_eq!(schedule[0], eff);
        assert_eq!(schedule[2], term);
    }

    #[test]
    fn test_schedule_eom() {
        let eff = Date::from_ymd(2024, 1, 31).unwrap();
        let term = Date::from_ymd(2024, 7, 31).unwrap();
        let schedule = create_roll_schedule(
            eff,
            term,
            Frequency::Monthly,
            RollConvention::EndOfMonth,
            StubType::Short,
            StubType::Short,
        )
        .unwrap();
        assert_eq!(schedule.len(), 7);
        assert_eq!(schedule[0], eff);
        assert_eq!(schedule[6], term);
        // February should be 29th (2024 is leap year)
        assert_eq!(schedule[1], Date::from_ymd(2024, 2, 29).unwrap());
    }
}
