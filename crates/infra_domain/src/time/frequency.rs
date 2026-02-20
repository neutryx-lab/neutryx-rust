//! Payment frequency definitions.

use std::str::FromStr;

/// Payment frequency for financial instruments.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    strum::Display,
    strum::AsRefStr,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    /// No payments / zero coupon.
    #[strum(serialize = "None")]
    None,
    /// Daily payments (252 business days per year).
    Daily,
    /// Weekly payments (52 per year).
    Weekly,
    /// Monthly payments (12 per year).
    #[default]
    Monthly,
    /// Bi-monthly payments (6 per year).
    #[strum(serialize = "Bi-Monthly")]
    BiMonthly,
    /// Quarterly payments (4 per year).
    Quarterly,
    /// Tri-annual payments (3 per year).
    #[strum(serialize = "Tri-Annual")]
    TriAnnual,
    /// Semi-annual payments (2 per year).
    #[strum(serialize = "Semi-Annual")]
    SemiAnnual,
    /// Annual payments (1 per year).
    Annual,
}

impl Frequency {
    /// Returns the number of months per payment period.
    #[must_use]
    pub fn months_per_period(&self) -> u32 {
        match self {
            Frequency::None => 0,
            Frequency::Annual => 12,
            Frequency::SemiAnnual => 6,
            Frequency::TriAnnual => 4,
            Frequency::Quarterly => 3,
            Frequency::BiMonthly => 2,
            Frequency::Monthly => 1,
            Frequency::Weekly | Frequency::Daily => 0,
        }
    }

    /// Returns the number of payment periods per year.
    #[must_use]
    pub fn periods_per_year(&self) -> u32 {
        match self {
            Frequency::None => 0,
            Frequency::Annual => 1,
            Frequency::SemiAnnual => 2,
            Frequency::TriAnnual => 3,
            Frequency::Quarterly => 4,
            Frequency::BiMonthly => 6,
            Frequency::Monthly => 12,
            Frequency::Weekly => 52,
            Frequency::Daily => 365,
        }
    }

    /// Returns `true` if this frequency is monthly-based (divisible into
    /// months).
    #[must_use]
    pub fn is_monthly(&self) -> bool {
        matches!(
            self,
            Frequency::Monthly
                | Frequency::BiMonthly
                | Frequency::Quarterly
                | Frequency::TriAnnual
                | Frequency::SemiAnnual
                | Frequency::Annual
        )
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
            "none" | "zero" | "zerocoupon" => Ok(Frequency::None),
            "annual" | "1y" | "yearly" | "pa" => Ok(Frequency::Annual),
            "semiannual" | "6m" | "sa" => Ok(Frequency::SemiAnnual),
            "triannual" | "4m" | "ta" => Ok(Frequency::TriAnnual),
            "quarterly" | "3m" | "qa" => Ok(Frequency::Quarterly),
            "bimonthly" | "2m" => Ok(Frequency::BiMonthly),
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
    fn test_frequency_ord_full_chain() {
        assert!(Frequency::None < Frequency::Daily);
        assert!(Frequency::Daily < Frequency::Weekly);
        assert!(Frequency::Weekly < Frequency::Monthly);
        assert!(Frequency::Monthly < Frequency::BiMonthly);
        assert!(Frequency::BiMonthly < Frequency::Quarterly);
        assert!(Frequency::Quarterly < Frequency::TriAnnual);
        assert!(Frequency::TriAnnual < Frequency::SemiAnnual);
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
            Frequency::None,
            Frequency::BiMonthly,
            Frequency::TriAnnual,
        ];
        frequencies.sort();

        assert_eq!(
            frequencies,
            vec![
                Frequency::None,
                Frequency::Daily,
                Frequency::Weekly,
                Frequency::Monthly,
                Frequency::BiMonthly,
                Frequency::Quarterly,
                Frequency::TriAnnual,
                Frequency::SemiAnnual,
                Frequency::Annual,
            ]
        );
    }

    #[test]
    fn test_frequency_default() {
        assert_eq!(Frequency::default(), Frequency::Monthly);
    }

    #[test]
    fn test_months_per_period() {
        assert_eq!(Frequency::None.months_per_period(), 0);
        assert_eq!(Frequency::Annual.months_per_period(), 12);
        assert_eq!(Frequency::SemiAnnual.months_per_period(), 6);
        assert_eq!(Frequency::TriAnnual.months_per_period(), 4);
        assert_eq!(Frequency::Quarterly.months_per_period(), 3);
        assert_eq!(Frequency::BiMonthly.months_per_period(), 2);
        assert_eq!(Frequency::Monthly.months_per_period(), 1);
        assert_eq!(Frequency::Weekly.months_per_period(), 0);
        assert_eq!(Frequency::Daily.months_per_period(), 0);
    }

    #[test]
    fn test_periods_per_year() {
        assert_eq!(Frequency::None.periods_per_year(), 0);
        assert_eq!(Frequency::Annual.periods_per_year(), 1);
        assert_eq!(Frequency::SemiAnnual.periods_per_year(), 2);
        assert_eq!(Frequency::TriAnnual.periods_per_year(), 3);
        assert_eq!(Frequency::Quarterly.periods_per_year(), 4);
        assert_eq!(Frequency::BiMonthly.periods_per_year(), 6);
        assert_eq!(Frequency::Monthly.periods_per_year(), 12);
        assert_eq!(Frequency::Weekly.periods_per_year(), 52);
        assert_eq!(Frequency::Daily.periods_per_year(), 365);
    }

    #[test]
    fn test_is_monthly() {
        assert!(Frequency::Monthly.is_monthly());
        assert!(Frequency::Quarterly.is_monthly());
        assert!(Frequency::Annual.is_monthly());
        assert!(!Frequency::Weekly.is_monthly());
        assert!(!Frequency::Daily.is_monthly());
        assert!(!Frequency::None.is_monthly());
    }

    #[test]
    fn test_from_str() {
        assert_eq!("Annual".parse::<Frequency>().unwrap(), Frequency::Annual);
        assert_eq!(
            "semi-annual".parse::<Frequency>().unwrap(),
            Frequency::SemiAnnual
        );
        assert_eq!(
            "tri-annual".parse::<Frequency>().unwrap(),
            Frequency::TriAnnual
        );
        assert_eq!(
            "QUARTERLY".parse::<Frequency>().unwrap(),
            Frequency::Quarterly
        );
        assert_eq!(
            "bi-monthly".parse::<Frequency>().unwrap(),
            Frequency::BiMonthly
        );
        assert_eq!("Monthly".parse::<Frequency>().unwrap(), Frequency::Monthly);
        assert_eq!("none".parse::<Frequency>().unwrap(), Frequency::None);
        assert_eq!("2m".parse::<Frequency>().unwrap(), Frequency::BiMonthly);
        assert_eq!("4m".parse::<Frequency>().unwrap(), Frequency::TriAnnual);
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
        set.insert(Frequency::Annual);
        assert_eq!(set.len(), 2);
    }
}
