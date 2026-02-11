//! Time utilities and Day Count Conventions for financial calculations.
//!
//! This module provides:
//! - `DayCountConvention`: A simplified subset of day count conventions
//! - `time_to_maturity`, `time_to_maturity_dates`: Year fraction utilities
//!
//! # Note
//!
//! For core time types (`Date`, `BusinessDayConvention`, `DayCounter`),
//! import directly from `infra_domain`.
//!
//! # Examples
//!
//! ```
//! use infra_domain::time::Date;
//! use pricer_core::types::time::DayCountConvention;
//!
//! let start = Date::from_ymd(2024, 1, 1).unwrap();
//! let end = Date::from_ymd(2024, 7, 1).unwrap();
//!
//! // Calculate year fraction using ACT/365
//! let yf = DayCountConvention::ActualActual365.year_fraction_dates(start, end);
//! assert!((yf - 0.4986).abs() < 0.001);
//! ```

use std::str::FromStr;

use infra_domain::time::{Date, DayCounter};

/// Day Count Convention (year fraction convention).
///
/// A simplified subset of day count conventions commonly used in derivatives.
/// For the full set of conventions, use `infra_domain::DayCounter` directly.
///
/// # Variants
/// - `ActualActual365`: Actual days / 365 (equivalent to
///   `DayCounter::Actual365Fixed`)
/// - `ActualActual360`: Actual days / 360 (equivalent to
///   `DayCounter::Actual360`)
/// - `Thirty360`: 30/360 Bond Basis (equivalent to `DayCounter::Thirty360Bond`)
///
/// # Usage
///
/// ```
/// use pricer_core::types::time::DayCountConvention;
///
/// let act_365 = DayCountConvention::ActualActual365;
/// assert_eq!(act_365.name(), "ACT/365");
/// ```
#[non_exhaustive]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash,
    strum::Display, strum::AsRefStr,
)]
pub enum DayCountConvention {
    /// Actual/365 Fixed: actual_days / 365.0
    ///
    /// Used in:
    /// - Most derivatives markets
    /// - UK gilts
    /// - Japanese government bonds
    #[strum(serialize = "ACT/365")]
    ActualActual365,

    /// Actual/360: actual_days / 360.0
    ///
    /// Used in:
    /// - Money market instruments
    /// - US Treasury bills
    /// - LIBOR-based instruments
    #[strum(serialize = "ACT/360")]
    ActualActual360,

    /// 30/360 US Bond Basis
    ///
    /// Used in:
    /// - US corporate bonds
    /// - US agency bonds
    /// - Some municipal bonds
    ///
    /// Each month is treated as having 30 days, and the year as 360 days.
    #[strum(serialize = "30/360")]
    Thirty360,
}

impl DayCountConvention {
    /// Returns the standard convention name.
    ///
    /// Returns industry-standard convention names for serialisation
    /// and display purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::DayCountConvention;
    ///
    /// assert_eq!(DayCountConvention::ActualActual365.name(), "ACT/365");
    /// assert_eq!(DayCountConvention::ActualActual360.name(), "ACT/360");
    /// assert_eq!(DayCountConvention::Thirty360.name(), "30/360");
    /// ```
    pub fn name(&self) -> &str { self.as_ref() }

    /// Convert to the underlying `infra_domain::DayCounter`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::DayCountConvention;
    /// use infra_domain::time::DayCounter;
    ///
    /// let dcc = DayCountConvention::ActualActual365;
    /// let dc: DayCounter = dcc.into();
    /// assert_eq!(dc, DayCounter::Actual365Fixed);
    /// ```
    #[must_use]
    pub fn to_day_counter(self) -> DayCounter {
        match self {
            DayCountConvention::ActualActual365 => DayCounter::Actual365Fixed,
            DayCountConvention::ActualActual360 => DayCounter::Actual360,
            DayCountConvention::Thirty360 => DayCounter::Thirty360Bond,
        }
    }

    /// Calculates year fraction using Date type.
    ///
    /// Returns negative values when start > end instead of panicking.
    ///
    /// # Arguments
    /// * `start` - Start date
    /// * `end` - End date
    ///
    /// # Returns
    /// Year fraction as f64. Negative if start > end.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::DayCountConvention;
    /// use infra_domain::time::Date;
    ///
    /// let start = Date::from_ymd(2024, 1, 1).unwrap();
    /// let end = Date::from_ymd(2024, 7, 1).unwrap();
    ///
    /// let yf = DayCountConvention::ActualActual365.year_fraction_dates(start, end);
    /// assert!((yf - 0.4986).abs() < 0.001);
    ///
    /// // Reversed dates return negative value
    /// let yf_neg = DayCountConvention::ActualActual365.year_fraction_dates(end, start);
    /// assert!((yf_neg + 0.4986).abs() < 0.001);
    /// ```
    pub fn year_fraction_dates(&self, start: Date, end: Date) -> f64 {
        self.to_day_counter().year_fraction(start, end)
    }

    /// Calculate year fraction between two NaiveDate values.
    ///
    /// # Arguments
    /// * `start` - Start date
    /// * `end` - End date
    ///
    /// # Returns
    /// Year fraction as f64 (e.g., 0.5 for 6 months, 1.0 for 1 year)
    ///
    /// # Panics
    /// Panics if `start > end`
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::time::DayCountConvention;
    /// use chrono::NaiveDate;
    ///
    /// let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    /// let end = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
    ///
    /// // Act/365
    /// let act_365 = DayCountConvention::ActualActual365;
    /// let yf_365 = act_365.year_fraction(start, end);
    /// assert!((yf_365 - 0.4986).abs() < 0.001);
    ///
    /// // Act/360
    /// let act_360 = DayCountConvention::ActualActual360;
    /// let yf_360 = act_360.year_fraction(start, end);
    /// assert!((yf_360 - 0.5056).abs() < 0.001);
    /// ```
    pub fn year_fraction(&self, start: chrono::NaiveDate, end: chrono::NaiveDate) -> f64 {
        assert!(
            start <= end,
            "start date must be less than or equal to end date"
        );

        let start_date = Date::from_naive(start);
        let end_date = Date::from_naive(end);
        self.to_day_counter().year_fraction(start_date, end_date)
    }
}

impl From<DayCountConvention> for DayCounter {
    fn from(dcc: DayCountConvention) -> Self { dcc.to_day_counter() }
}

impl FromStr for DayCountConvention {
    type Err = String;

    /// Parses day count convention from string (case-insensitive).
    ///
    /// Supports multiple aliases for each convention:
    /// - ACT/365: "ACT/365", "Actual/365", "Act365", "A365"
    /// - ACT/360: "ACT/360", "Actual/360", "Act360", "A360"
    /// - 30/360: "30/360", "Thirty360", "30360"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().replace(['/', ' '], "").as_str() {
            "ACT365" | "ACTUAL365" | "A365" => Ok(DayCountConvention::ActualActual365),
            "ACT360" | "ACTUAL360" | "A360" => Ok(DayCountConvention::ActualActual360),
            "30360" | "THIRTY360" => Ok(DayCountConvention::Thirty360),
            _ => Err(format!("Unknown day count convention: {}", s)),
        }
    }
}

mod serde_impl {
    use std::str::FromStr;

    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

    use super::DayCountConvention;

    impl Serialize for DayCountConvention {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(self.name())
        }
    }

    impl<'de> Deserialize<'de> for DayCountConvention {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            DayCountConvention::from_str(&s).map_err(de::Error::custom)
        }
    }
}

/// Calculate time to maturity using default convention (Act/365).
///
/// # Arguments
/// * `start` - Valuation date
/// * `end` - Maturity date
///
/// # Returns
/// Time to maturity in years (Act/365 convention)
///
/// # Panics
/// Panics if `start > end`
///
/// # Examples
///
/// ```
/// use pricer_core::types::time::time_to_maturity;
/// use chrono::NaiveDate;
///
/// let valuation_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
/// let maturity_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
///
/// let ttm = time_to_maturity(valuation_date, maturity_date);
/// assert!((ttm - 1.0027).abs() < 0.001); // ~1 year (366 days in 2024 leap year)
/// ```
pub fn time_to_maturity(start: chrono::NaiveDate, end: chrono::NaiveDate) -> f64 {
    DayCountConvention::ActualActual365.year_fraction(start, end)
}

/// Calculate time to maturity using Date type and default convention (Act/365).
///
/// Unlike `time_to_maturity`, this function does not panic when start > end,
/// instead returning a negative value.
///
/// # Arguments
/// * `start` - Valuation date
/// * `end` - Maturity date
///
/// # Returns
/// Time to maturity in years (Act/365 convention). Negative if start > end.
///
/// # Examples
///
/// ```
/// use pricer_core::types::time::time_to_maturity_dates;
/// use infra_domain::time::Date;
///
/// let valuation_date = Date::from_ymd(2024, 1, 1).unwrap();
/// let maturity_date = Date::from_ymd(2025, 1, 1).unwrap();
///
/// let ttm = time_to_maturity_dates(valuation_date, maturity_date);
/// assert!((ttm - 1.0027).abs() < 0.001); // ~1 year (366 days in 2024 leap year)
///
/// // Negative time to maturity (expired)
/// let ttm_neg = time_to_maturity_dates(maturity_date, valuation_date);
/// assert!(ttm_neg < 0.0);
/// ```
pub fn time_to_maturity_dates(start: Date, end: Date) -> f64 {
    DayCountConvention::ActualActual365.year_fraction_dates(start, end)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn test_act_365_known_dates() {
        // 2024-01-01 to 2024-07-01 is 182 days
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();

        let convention = DayCountConvention::ActualActual365;
        let result = convention.year_fraction(start, end);

        let expected = 182.0 / 365.0; // ≈ 0.4986
        assert_relative_eq!(result, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_act_360_known_dates() {
        // 2024-01-01 to 2024-07-01 is 182 days
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();

        let convention = DayCountConvention::ActualActual360;
        let result = convention.year_fraction(start, end);

        let expected = 182.0 / 360.0; // ≈ 0.5056
        assert_relative_eq!(result, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_thirty_360_known_dates() {
        // 2024-01-01 to 2024-07-01
        // Years: 0, Months: 6, Days: 0 (1st to 1st)
        // Total days in 30/360: 0*360 + 6*30 + 0 = 180
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();

        let convention = DayCountConvention::Thirty360;
        let result = convention.year_fraction(start, end);

        let expected = 180.0 / 360.0; // 0.5
        assert_relative_eq!(result, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_time_to_maturity_matches_act_365() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();

        let ttm = time_to_maturity(start, end);
        let act_365 = DayCountConvention::ActualActual365.year_fraction(start, end);

        assert_relative_eq!(ttm, act_365, epsilon = 1e-10);
    }

    #[test]
    fn test_same_date_returns_zero() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let act_365 = DayCountConvention::ActualActual365;
        assert_eq!(act_365.year_fraction(date, date), 0.0);

        let act_360 = DayCountConvention::ActualActual360;
        assert_eq!(act_360.year_fraction(date, date), 0.0);

        let thirty_360 = DayCountConvention::Thirty360;
        assert_eq!(thirty_360.year_fraction(date, date), 0.0);
    }

    #[test]
    fn test_one_year_period() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();

        // 2024 is a leap year, so 366 days
        let act_365 = DayCountConvention::ActualActual365;
        let result_365 = act_365.year_fraction(start, end);
        assert_relative_eq!(result_365, 366.0 / 365.0, epsilon = 1e-10);

        let act_360 = DayCountConvention::ActualActual360;
        let result_360 = act_360.year_fraction(start, end);
        assert_relative_eq!(result_360, 366.0 / 360.0, epsilon = 1e-10);

        let thirty_360 = DayCountConvention::Thirty360;
        let result_30_360 = thirty_360.year_fraction(start, end);
        assert_relative_eq!(result_30_360, 1.0, epsilon = 1e-10); // Exactly 1 year in 30/360
    }

    #[test]
    #[should_panic(expected = "start date must be less than or equal to end date")]
    fn test_year_fraction_panics_on_reverse_dates() {
        let start = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let convention = DayCountConvention::ActualActual365;
        convention.year_fraction(start, end);
    }

    #[test]
    #[should_panic(expected = "start date must be less than or equal to end date")]
    fn test_time_to_maturity_panics_on_reverse_dates() {
        let start = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        time_to_maturity(start, end);
    }

    // Date tests using infra_domain::Date

    #[test]
    fn test_date_from_ymd_valid() {
        let date = Date::from_ymd(2024, 6, 15).unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 6);
        assert_eq!(date.day(), 15);
    }

    #[test]
    fn test_date_from_ymd_invalid() {
        // February 30 is invalid
        let result = Date::from_ymd(2024, 2, 30);
        assert!(result.is_err());
    }

    #[test]
    fn test_date_subtraction() {
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = Date::from_ymd(2024, 1, 11).unwrap();

        assert_eq!(end - start, 10);
        assert_eq!(start - end, -10);
    }

    // DayCountConvention name() tests

    #[test]
    fn test_dcc_name() {
        assert_eq!(DayCountConvention::ActualActual365.name(), "ACT/365");
        assert_eq!(DayCountConvention::ActualActual360.name(), "ACT/360");
        assert_eq!(DayCountConvention::Thirty360.name(), "30/360");
    }

    #[test]
    fn test_dcc_display() {
        assert_eq!(
            format!("{}", DayCountConvention::ActualActual365),
            "ACT/365"
        );
        assert_eq!(
            format!("{}", DayCountConvention::ActualActual360),
            "ACT/360"
        );
        assert_eq!(format!("{}", DayCountConvention::Thirty360), "30/360");
    }

    #[test]
    fn test_dcc_from_str() {
        assert_eq!(
            "ACT/365".parse::<DayCountConvention>().unwrap(),
            DayCountConvention::ActualActual365
        );
        assert_eq!(
            "act/360".parse::<DayCountConvention>().unwrap(),
            DayCountConvention::ActualActual360
        );
        assert_eq!(
            "30/360".parse::<DayCountConvention>().unwrap(),
            DayCountConvention::Thirty360
        );
        assert_eq!(
            "Thirty360".parse::<DayCountConvention>().unwrap(),
            DayCountConvention::Thirty360
        );
    }

    #[test]
    fn test_dcc_from_str_invalid() {
        let result = "INVALID".parse::<DayCountConvention>();
        assert!(result.is_err());
    }

    // year_fraction_dates tests

    #[test]
    fn test_year_fraction_dates_matches_year_fraction() {
        let start_date = Date::from_ymd(2024, 1, 1).unwrap();
        let end_date = Date::from_ymd(2024, 7, 1).unwrap();
        let start_naive = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end_naive = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();

        for dcc in [
            DayCountConvention::ActualActual365,
            DayCountConvention::ActualActual360,
            DayCountConvention::Thirty360,
        ] {
            let yf_dates = dcc.year_fraction_dates(start_date, end_date);
            let yf_naive = dcc.year_fraction(start_naive, end_naive);
            assert_relative_eq!(yf_dates, yf_naive, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_year_fraction_dates_negative() {
        let start = Date::from_ymd(2024, 7, 1).unwrap();
        let end = Date::from_ymd(2024, 1, 1).unwrap();

        // Should NOT panic, should return negative
        let yf = DayCountConvention::ActualActual365.year_fraction_dates(start, end);
        assert!(yf < 0.0);
        assert_relative_eq!(yf, -182.0 / 365.0, epsilon = 1e-10);
    }

    #[test]
    fn test_time_to_maturity_dates() {
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 1, 1).unwrap();

        let ttm = time_to_maturity_dates(start, end);
        assert_relative_eq!(ttm, 366.0 / 365.0, epsilon = 1e-10);
    }

    #[test]
    fn test_time_to_maturity_dates_negative() {
        let start = Date::from_ymd(2025, 1, 1).unwrap();
        let end = Date::from_ymd(2024, 1, 1).unwrap();

        // Should NOT panic, should return negative
        let ttm = time_to_maturity_dates(start, end);
        assert!(ttm < 0.0);
    }

    // Conversion tests

    #[test]
    fn test_to_day_counter() {
        assert_eq!(
            DayCountConvention::ActualActual365.to_day_counter(),
            DayCounter::Actual365Fixed
        );
        assert_eq!(
            DayCountConvention::ActualActual360.to_day_counter(),
            DayCounter::Actual360
        );
        assert_eq!(
            DayCountConvention::Thirty360.to_day_counter(),
            DayCounter::Thirty360Bond
        );
    }

    #[test]
    fn test_from_dcc_to_day_counter() {
        let dcc = DayCountConvention::ActualActual365;
        let dc: DayCounter = dcc.into();
        assert_eq!(dc, DayCounter::Actual365Fixed);
    }

    mod serde_tests {
        use super::*;

        #[test]
        fn test_dcc_serde_roundtrip() {
            let dcc = DayCountConvention::ActualActual365;
            let json = serde_json::to_string(&dcc).unwrap();
            assert_eq!(json, "\"ACT/365\"");

            let parsed: DayCountConvention = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, dcc);
        }

        #[test]
        fn test_dcc_serde_all_variants() {
            for dcc in [
                DayCountConvention::ActualActual365,
                DayCountConvention::ActualActual360,
                DayCountConvention::Thirty360,
            ] {
                let json = serde_json::to_string(&dcc).unwrap();
                let parsed: DayCountConvention = serde_json::from_str(&json).unwrap();
                assert_eq!(parsed, dcc);
            }
        }

        #[test]
        fn test_dcc_serde_deserialize_alias() {
            // Test case-insensitive and alias parsing
            let parsed: DayCountConvention = serde_json::from_str("\"Actual/365\"").unwrap();
            assert_eq!(parsed, DayCountConvention::ActualActual365);

            let parsed: DayCountConvention = serde_json::from_str("\"30/360\"").unwrap();
            assert_eq!(parsed, DayCountConvention::Thirty360);
        }
    }
}
