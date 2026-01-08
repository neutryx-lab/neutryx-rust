//! Payment frequency enumeration.

use std::fmt;
use std::str::FromStr;

/// Payment frequency for scheduled instruments.
///
/// Defines how often payments occur in a scheduled financial instrument
/// such as an interest rate swap or bond.
///
/// # Examples
///
/// ```
/// use pricer_models::schedules::Frequency;
///
/// let freq = Frequency::Quarterly;
/// assert_eq!(freq.periods_per_year(), 4);
/// assert_eq!(freq.months_between_payments(), 3);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frequency {
    /// Annual payments (once per year).
    Annual,
    /// Semi-annual payments (twice per year).
    SemiAnnual,
    /// Quarterly payments (four times per year).
    Quarterly,
    /// Monthly payments (twelve times per year).
    Monthly,
    /// Weekly payments (52 times per year).
    Weekly,
    /// Daily payments.
    Daily,
}

impl Frequency {
    /// Returns the number of payment periods per year.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::Frequency;
    ///
    /// assert_eq!(Frequency::Annual.periods_per_year(), 1);
    /// assert_eq!(Frequency::SemiAnnual.periods_per_year(), 2);
    /// assert_eq!(Frequency::Quarterly.periods_per_year(), 4);
    /// assert_eq!(Frequency::Monthly.periods_per_year(), 12);
    /// assert_eq!(Frequency::Weekly.periods_per_year(), 52);
    /// assert_eq!(Frequency::Daily.periods_per_year(), 365);
    /// ```
    #[inline]
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

    /// Returns the number of months between payment dates.
    ///
    /// Returns 0 for Weekly and Daily frequencies (use `days_between_payments` instead).
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::Frequency;
    ///
    /// assert_eq!(Frequency::Annual.months_between_payments(), 12);
    /// assert_eq!(Frequency::SemiAnnual.months_between_payments(), 6);
    /// assert_eq!(Frequency::Quarterly.months_between_payments(), 3);
    /// assert_eq!(Frequency::Monthly.months_between_payments(), 1);
    /// assert_eq!(Frequency::Weekly.months_between_payments(), 0);
    /// assert_eq!(Frequency::Daily.months_between_payments(), 0);
    /// ```
    #[inline]
    pub fn months_between_payments(&self) -> u32 {
        match self {
            Frequency::Annual => 12,
            Frequency::SemiAnnual => 6,
            Frequency::Quarterly => 3,
            Frequency::Monthly => 1,
            Frequency::Weekly => 0,
            Frequency::Daily => 0,
        }
    }

    /// Returns the approximate number of days between payment dates.
    ///
    /// For month-based frequencies, assumes 30 days per month.
    /// For Weekly and Daily, returns exact values.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::Frequency;
    ///
    /// assert_eq!(Frequency::Annual.days_between_payments(), 360);
    /// assert_eq!(Frequency::Weekly.days_between_payments(), 7);
    /// assert_eq!(Frequency::Daily.days_between_payments(), 1);
    /// ```
    #[inline]
    pub fn days_between_payments(&self) -> u32 {
        match self {
            Frequency::Annual => 360,
            Frequency::SemiAnnual => 180,
            Frequency::Quarterly => 90,
            Frequency::Monthly => 30,
            Frequency::Weekly => 7,
            Frequency::Daily => 1,
        }
    }

    /// Returns the standard name for this frequency.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::schedules::Frequency;
    ///
    /// assert_eq!(Frequency::Annual.name(), "Annual");
    /// assert_eq!(Frequency::SemiAnnual.name(), "Semi-Annual");
    /// ```
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Frequency::Annual => "Annual",
            Frequency::SemiAnnual => "Semi-Annual",
            Frequency::Quarterly => "Quarterly",
            Frequency::Monthly => "Monthly",
            Frequency::Weekly => "Weekly",
            Frequency::Daily => "Daily",
        }
    }
}

impl fmt::Display for Frequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for Frequency {
    type Err = String;

    /// Parses frequency from string (case-insensitive).
    ///
    /// Supported formats:
    /// - Annual: "annual", "1y", "yearly"
    /// - SemiAnnual: "semi-annual", "semiannual", "6m", "2y"
    /// - Quarterly: "quarterly", "3m", "4y"
    /// - Monthly: "monthly", "1m", "12y"
    /// - Weekly: "weekly", "1w"
    /// - Daily: "daily", "1d"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "annual" | "1y" | "yearly" | "12m" => Ok(Frequency::Annual),
            "semiannual" | "6m" | "2y" => Ok(Frequency::SemiAnnual),
            "quarterly" | "3m" | "4y" => Ok(Frequency::Quarterly),
            "monthly" | "1m" => Ok(Frequency::Monthly),
            "weekly" | "1w" => Ok(Frequency::Weekly),
            "daily" | "1d" => Ok(Frequency::Daily),
            _ => Err(format!("Unknown frequency: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_months_between_payments() {
        assert_eq!(Frequency::Annual.months_between_payments(), 12);
        assert_eq!(Frequency::SemiAnnual.months_between_payments(), 6);
        assert_eq!(Frequency::Quarterly.months_between_payments(), 3);
        assert_eq!(Frequency::Monthly.months_between_payments(), 1);
        assert_eq!(Frequency::Weekly.months_between_payments(), 0);
        assert_eq!(Frequency::Daily.months_between_payments(), 0);
    }

    #[test]
    fn test_days_between_payments() {
        assert_eq!(Frequency::Annual.days_between_payments(), 360);
        assert_eq!(Frequency::SemiAnnual.days_between_payments(), 180);
        assert_eq!(Frequency::Quarterly.days_between_payments(), 90);
        assert_eq!(Frequency::Monthly.days_between_payments(), 30);
        assert_eq!(Frequency::Weekly.days_between_payments(), 7);
        assert_eq!(Frequency::Daily.days_between_payments(), 1);
    }

    #[test]
    fn test_name() {
        assert_eq!(Frequency::Annual.name(), "Annual");
        assert_eq!(Frequency::SemiAnnual.name(), "Semi-Annual");
        assert_eq!(Frequency::Quarterly.name(), "Quarterly");
        assert_eq!(Frequency::Monthly.name(), "Monthly");
        assert_eq!(Frequency::Weekly.name(), "Weekly");
        assert_eq!(Frequency::Daily.name(), "Daily");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Frequency::Annual), "Annual");
        assert_eq!(format!("{}", Frequency::SemiAnnual), "Semi-Annual");
    }

    #[test]
    fn test_from_str_valid() {
        assert_eq!("annual".parse::<Frequency>().unwrap(), Frequency::Annual);
        assert_eq!("Annual".parse::<Frequency>().unwrap(), Frequency::Annual);
        assert_eq!("1Y".parse::<Frequency>().unwrap(), Frequency::Annual);
        assert_eq!(
            "semi-annual".parse::<Frequency>().unwrap(),
            Frequency::SemiAnnual
        );
        assert_eq!(
            "SemiAnnual".parse::<Frequency>().unwrap(),
            Frequency::SemiAnnual
        );
        assert_eq!("6m".parse::<Frequency>().unwrap(), Frequency::SemiAnnual);
        assert_eq!(
            "quarterly".parse::<Frequency>().unwrap(),
            Frequency::Quarterly
        );
        assert_eq!("3m".parse::<Frequency>().unwrap(), Frequency::Quarterly);
        assert_eq!("monthly".parse::<Frequency>().unwrap(), Frequency::Monthly);
        assert_eq!("1m".parse::<Frequency>().unwrap(), Frequency::Monthly);
        assert_eq!("weekly".parse::<Frequency>().unwrap(), Frequency::Weekly);
        assert_eq!("daily".parse::<Frequency>().unwrap(), Frequency::Daily);
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("invalid".parse::<Frequency>().is_err());
        assert!("biweekly".parse::<Frequency>().is_err());
    }

    #[test]
    fn test_clone_and_copy() {
        let freq1 = Frequency::Quarterly;
        let freq2 = freq1; // Copy
        let freq3 = freq1.clone(); // Clone

        assert_eq!(freq1, freq2);
        assert_eq!(freq1, freq3);
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

    #[test]
    fn test_debug() {
        let debug_str = format!("{:?}", Frequency::Quarterly);
        assert!(debug_str.contains("Quarterly"));
    }
}
