//! Day count convention definitions.

/// Day count convention for interest calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DayCountConvention {
    /// Actual/360
    Actual360,
    /// Actual/365 Fixed
    #[default]
    Actual365Fixed,
    /// Actual/365.25
    Actual36525,
    /// Actual/Actual (ISDA)
    ActualActualIsda,
    /// 30/360 (Bond Basis)
    Thirty360Bond,
    /// 30/360 (European)
    Thirty360European,
    /// 30E/360 (ISDA)
    ThirtyE360Isda,
}

impl DayCountConvention {
    /// Calculate the year fraction between two dates.
    pub fn year_fraction(&self, start: chrono::NaiveDate, end: chrono::NaiveDate) -> f64 {
        let days = (end - start).num_days() as f64;

        match self {
            DayCountConvention::Actual360 => days / 360.0,
            DayCountConvention::Actual365Fixed => days / 365.0,
            DayCountConvention::Actual36525 => days / 365.25,
            DayCountConvention::ActualActualIsda => {
                // Simplified: use 365.25 as average
                days / 365.25
            }
            DayCountConvention::Thirty360Bond
            | DayCountConvention::Thirty360European
            | DayCountConvention::ThirtyE360Isda => self.thirty_360_days(start, end) / 360.0,
        }
    }

    /// Calculate 30/360 day count.
    fn thirty_360_days(&self, start: chrono::NaiveDate, end: chrono::NaiveDate) -> f64 {
        use chrono::Datelike;

        let (y1, m1, d1) = (start.year(), start.month() as i32, start.day() as i32);
        let (y2, m2, d2) = (end.year(), end.month() as i32, end.day() as i32);

        let (d1_adj, d2_adj) = match self {
            DayCountConvention::Thirty360Bond => {
                let d1_adj = d1.min(30);
                let d2_adj = if d1_adj == 30 { d2.min(30) } else { d2 };
                (d1_adj, d2_adj)
            }
            DayCountConvention::Thirty360European | DayCountConvention::ThirtyE360Isda => {
                (d1.min(30), d2.min(30))
            }
            _ => (d1, d2),
        };

        (360 * (y2 - y1) + 30 * (m2 - m1) + (d2_adj - d1_adj)) as f64
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn test_actual_360() {
        let dcc = DayCountConvention::Actual360;
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let yf = dcc.year_fraction(start, end);
        assert!((yf - 0.25).abs() < 0.01); // ~90 days / 360
    }

    #[test]
    fn test_actual_365() {
        let dcc = DayCountConvention::Actual365Fixed;
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        let yf = dcc.year_fraction(start, end);
        assert!((yf - 1.0).abs() < 0.01);
    }
}
