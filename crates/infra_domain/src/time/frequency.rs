//! Payment frequency definitions.
//!
//! This module provides payment frequency representations for
//! financial instruments.
//!
//! # Examples
//!
//! ```
//! use infra_domain::time::Frequency;
//!
//! let freq = Frequency::Quarterly;
//! assert_eq!(freq.periods_per_year(), 4);
//! assert_eq!(freq.months_per_period(), 3);
//! ```

use std::str::FromStr;

/// Payment frequency for financial instruments.
///
/// Represents how often payments are made (e.g., coupons, interest).
///
/// # Examples
///
/// ```
/// use infra_domain::time::Frequency;
///
/// let freq = Frequency::SemiAnnual;
/// assert_eq!(freq.periods_per_year(), 2);
/// assert_eq!(freq.months_per_period(), 6);
/// ```
/// Payment frequency ordered from highest (Daily) to lowest (Annual).
///
/// Ordering rationale: Financial schedules typically progress from
/// higher frequency to lower frequency when iterating payment dates.
/// The `Ord` implementation ensures `Daily < Weekly < Monthly < Quarterly <
/// SemiAnnual < Annual`.
///
/// # Adding New Variants
///
/// When adding new frequency variants, place them according to their
/// payment frequency (higher frequency = earlier in declaration order).
///
/// # Examples
///
/// ```
/// use infra_domain::time::Frequency;
///
/// // Ordering: Daily is "less than" Weekly (higher frequency first)
/// assert!(Frequency::Daily < Frequency::Weekly);
/// assert!(Frequency::Monthly < Frequency::Annual);
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default,
    strum::Display, strum::AsRefStr,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Frequency {
    /// Daily payments (252 business days per year)
    Daily,
    /// Weekly payments (52 per year)
    Weekly,
    /// Monthly payments (12 per year)
    #[default]
    Monthly,
    /// Quarterly payments (4 per year)
    Quarterly,
    /// Semi-annual payments (2 per year)
    #[strum(serialize = "Semi-Annual")]
    SemiAnnual,
    /// Annual payments (1 per year)
    Annual,
}

impl Frequency {
    /// Returns the number of months per payment period.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::Frequency;
    ///
    /// assert_eq!(Frequency::Annual.months_per_period(), 12);
    /// assert_eq!(Frequency::SemiAnnual.months_per_period(), 6);
    /// assert_eq!(Frequency::Quarterly.months_per_period(), 3);
    /// assert_eq!(Frequency::Monthly.months_per_period(), 1);
    /// assert_eq!(Frequency::Weekly.months_per_period(), 0);
    /// assert_eq!(Frequency::Daily.months_per_period(), 0);
    /// ```
    #[must_use]
    pub fn months_per_period(&self) -> u32 {
        match self {
            Frequency::Annual => 12,
            Frequency::SemiAnnual => 6,
            Frequency::Quarterly => 3,
            Frequency::Monthly => 1,
            Frequency::Weekly | Frequency::Daily => 0,
        }
    }

    /// Returns the number of payment periods per year.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::Frequency;
    ///
    /// assert_eq!(Frequency::Annual.periods_per_year(), 1);
    /// assert_eq!(Frequency::SemiAnnual.periods_per_year(), 2);
    /// assert_eq!(Frequency::Quarterly.periods_per_year(), 4);
    /// assert_eq!(Frequency::Monthly.periods_per_year(), 12);
    /// assert_eq!(Frequency::Weekly.periods_per_year(), 52);
    /// assert_eq!(Frequency::Daily.periods_per_year(), 365);
    /// ```
    #[must_use]
    pub fn periods_per_year(&self) -> u32 {
        match self {
            Frequency::Annual => 1,
            Frequency::SemiAnnual => 2,
            Frequency::Quarterly => 4,
            Frequency::Monthly => 12,
            Frequency::Weekly => 52,
            Frequency::Daily => 365,
        }
    }

    /// Returns the standard name for this frequency.
    #[must_use]
    pub fn name(&self) -> &str { self.as_ref() }
}

impl FromStr for Frequency {
    type Err = String;

    /// Parses frequency from string (case-insensitive).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "annual" | "1y" | "yearly" => Ok(Frequency::Annual),
            "semiannual" | "6m" | "2y" => Ok(Frequency::SemiAnnual),
            "quarterly" | "3m" | "4y" => Ok(Frequency::Quarterly),
            "monthly" | "1m" | "12y" => Ok(Frequency::Monthly),
            "weekly" | "1w" => Ok(Frequency::Weekly),
            "daily" | "1d" => Ok(Frequency::Daily),
            _ => Err(format!("Unknown frequency: {}", s)),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Ordering Tests (Requirement 1.2, 1.3)
    // ========================================

    #[test]
    fn test_frequency_ord_daily_less_than_weekly() {
        assert!(Frequency::Daily < Frequency::Weekly);
    }

    #[test]
    fn test_frequency_ord_weekly_less_than_monthly() {
        assert!(Frequency::Weekly < Frequency::Monthly);
    }

    #[test]
    fn test_frequency_ord_monthly_less_than_quarterly() {
        assert!(Frequency::Monthly < Frequency::Quarterly);
    }

    #[test]
    fn test_frequency_ord_quarterly_less_than_semiannual() {
        assert!(Frequency::Quarterly < Frequency::SemiAnnual);
    }

    #[test]
    fn test_frequency_ord_semiannual_less_than_annual() {
        assert!(Frequency::SemiAnnual < Frequency::Annual);
    }

    #[test]
    fn test_frequency_ord_full_chain() {
        // Verify complete ordering: Daily < Weekly < Monthly < Quarterly < SemiAnnual <
        // Annual
        assert!(Frequency::Daily < Frequency::Weekly);
        assert!(Frequency::Weekly < Frequency::Monthly);
        assert!(Frequency::Monthly < Frequency::Quarterly);
        assert!(Frequency::Quarterly < Frequency::SemiAnnual);
        assert!(Frequency::SemiAnnual < Frequency::Annual);
    }

    #[test]
    fn test_frequency_sort_vec() {
        let mut frequencies = vec![
            Frequency::Annual,
            Frequency::Daily,
            Frequency::Quarterly,
            Frequency::Monthly,
            Frequency::SemiAnnual,
            Frequency::Weekly,
        ];
        frequencies.sort();

        assert_eq!(
            frequencies,
            vec![
                Frequency::Daily,
                Frequency::Weekly,
                Frequency::Monthly,
                Frequency::Quarterly,
                Frequency::SemiAnnual,
                Frequency::Annual,
            ]
        );
    }

    #[test]
    fn test_frequency_default() {
        assert_eq!(Frequency::default(), Frequency::Monthly);
    }

    // ========================================
    // Existing Tests
    // ========================================

    #[test]
    fn test_months_per_period() {
        assert_eq!(Frequency::Annual.months_per_period(), 12);
        assert_eq!(Frequency::SemiAnnual.months_per_period(), 6);
        assert_eq!(Frequency::Quarterly.months_per_period(), 3);
        assert_eq!(Frequency::Monthly.months_per_period(), 1);
        assert_eq!(Frequency::Weekly.months_per_period(), 0);
        assert_eq!(Frequency::Daily.months_per_period(), 0);
    }

    #[test]
    fn test_periods_per_year() {
        assert_eq!(Frequency::Annual.periods_per_year(), 1);
        assert_eq!(Frequency::SemiAnnual.periods_per_year(), 2);
        assert_eq!(Frequency::Quarterly.periods_per_year(), 4);
        assert_eq!(Frequency::Monthly.periods_per_year(), 12);
        assert_eq!(Frequency::Weekly.periods_per_year(), 52);
        assert_eq!(Frequency::Daily.periods_per_year(), 365);
    }

    #[test]
    fn test_from_str() {
        assert_eq!("Annual".parse::<Frequency>().unwrap(), Frequency::Annual);
        assert_eq!(
            "semi-annual".parse::<Frequency>().unwrap(),
            Frequency::SemiAnnual
        );
        assert_eq!(
            "QUARTERLY".parse::<Frequency>().unwrap(),
            Frequency::Quarterly
        );
        assert_eq!("Monthly".parse::<Frequency>().unwrap(), Frequency::Monthly);
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("invalid".parse::<Frequency>().is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Frequency::Annual), "Annual");
        assert_eq!(format!("{}", Frequency::SemiAnnual), "Semi-Annual");
        assert_eq!(format!("{}", Frequency::Quarterly), "Quarterly");
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Frequency::Annual);
        set.insert(Frequency::Quarterly);
        set.insert(Frequency::Annual); // Duplicate
        assert_eq!(set.len(), 2);
    }
}
