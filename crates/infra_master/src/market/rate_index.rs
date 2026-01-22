//! Benchmark rate index definitions.
//!
//! This module provides rate index types for financial instruments.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::{RateIndex, Currency};
//! use infra_master::time::{DayCounter, Tenor};
//!
//! let sofr = RateIndex::Sofr;
//! assert_eq!(sofr.currency(), Currency::USD);
//! assert_eq!(sofr.tenor(), Tenor::Overnight);
//! assert_eq!(sofr.day_counter(), DayCounter::Actual360);
//! ```

use crate::{
    market::Currency,
    time::{DayCounter, Tenor},
};

/// Benchmark rate index.
///
/// Represents standard benchmark interest rate indices used in financial
/// markets.
///
/// # Examples
///
/// ```
/// use infra_master::market::{RateIndex, Currency};
///
/// let euribor = RateIndex::Euribor3M;
/// assert_eq!(euribor.currency(), Currency::EUR);
/// assert_eq!(euribor.name(), "EURIBOR 3M");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum RateIndex {
    /// Secured Overnight Financing Rate (USD)
    Sofr,
    /// Tokyo Overnight Average Rate (JPY)
    Tonar,
    /// Euro Interbank Offered Rate 3 Month
    Euribor3M,
    /// Euro Interbank Offered Rate 6 Month
    Euribor6M,
    /// Sterling Overnight Index Average (GBP)
    Sonia,
    /// Swiss Average Rate Overnight (CHF)
    Saron,
}

impl RateIndex {
    /// Returns the currency associated with this rate index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{RateIndex, Currency};
    ///
    /// assert_eq!(RateIndex::Sofr.currency(), Currency::USD);
    /// assert_eq!(RateIndex::Tonar.currency(), Currency::JPY);
    /// assert_eq!(RateIndex::Sonia.currency(), Currency::GBP);
    /// ```
    #[must_use]
    pub const fn currency(&self) -> Currency {
        match self {
            Self::Sofr => Currency::USD,
            Self::Tonar => Currency::JPY,
            Self::Euribor3M | Self::Euribor6M => Currency::EUR,
            Self::Sonia => Currency::GBP,
            Self::Saron => Currency::CHF,
        }
    }

    /// Returns the standard fixing tenor for this rate index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::RateIndex;
    /// use infra_master::time::Tenor;
    ///
    /// assert_eq!(RateIndex::Sofr.tenor(), Tenor::Overnight);
    /// assert_eq!(RateIndex::Euribor3M.tenor(), Tenor::ThreeMonths);
    /// assert_eq!(RateIndex::Euribor6M.tenor(), Tenor::SixMonths);
    /// ```
    #[must_use]
    pub const fn tenor(&self) -> Tenor {
        match self {
            Self::Sofr | Self::Tonar | Self::Sonia | Self::Saron => Tenor::Overnight,
            Self::Euribor3M => Tenor::ThreeMonths,
            Self::Euribor6M => Tenor::SixMonths,
        }
    }

    /// Returns the day count convention for this rate index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::RateIndex;
    /// use infra_master::time::DayCounter;
    ///
    /// assert_eq!(RateIndex::Sofr.day_counter(), DayCounter::Actual360);
    /// assert_eq!(RateIndex::Sonia.day_counter(), DayCounter::Actual365Fixed);
    /// ```
    #[must_use]
    pub const fn day_counter(&self) -> DayCounter {
        match self {
            // USD, EUR, CHF use ACT/360
            Self::Sofr | Self::Euribor3M | Self::Euribor6M | Self::Saron => DayCounter::Actual360,
            // GBP uses ACT/365
            Self::Sonia => DayCounter::Actual365Fixed,
            // JPY uses ACT/365
            Self::Tonar => DayCounter::Actual365Fixed,
        }
    }

    /// Returns the human-readable name of this rate index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::RateIndex;
    ///
    /// assert_eq!(RateIndex::Sofr.name(), "SOFR");
    /// assert_eq!(RateIndex::Euribor3M.name(), "EURIBOR 3M");
    /// ```
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Sofr => "SOFR",
            Self::Tonar => "TONAR",
            Self::Euribor3M => "EURIBOR 3M",
            Self::Euribor6M => "EURIBOR 6M",
            Self::Sonia => "SONIA",
            Self::Saron => "SARON",
        }
    }

    /// Returns the short code for this rate index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::RateIndex;
    ///
    /// assert_eq!(RateIndex::Sofr.code(), "SOFR");
    /// assert_eq!(RateIndex::Euribor3M.code(), "EUR3M");
    /// ```
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Sofr => "SOFR",
            Self::Tonar => "TONAR",
            Self::Euribor3M => "EUR3M",
            Self::Euribor6M => "EUR6M",
            Self::Sonia => "SONIA",
            Self::Saron => "SARON",
        }
    }
}

impl std::fmt::Display for RateIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for RateIndex {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SOFR" => Ok(Self::Sofr),
            "TONAR" => Ok(Self::Tonar),
            "EURIBOR3M" | "EUR3M" | "EURIBOR 3M" => Ok(Self::Euribor3M),
            "EURIBOR6M" | "EUR6M" | "EURIBOR 6M" => Ok(Self::Euribor6M),
            "SONIA" => Ok(Self::Sonia),
            "SARON" => Ok(Self::Saron),
            _ => Err(format!("Unknown rate index: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency() {
        assert_eq!(RateIndex::Sofr.currency(), Currency::USD);
        assert_eq!(RateIndex::Tonar.currency(), Currency::JPY);
        assert_eq!(RateIndex::Euribor3M.currency(), Currency::EUR);
        assert_eq!(RateIndex::Euribor6M.currency(), Currency::EUR);
        assert_eq!(RateIndex::Sonia.currency(), Currency::GBP);
        assert_eq!(RateIndex::Saron.currency(), Currency::CHF);
    }

    #[test]
    fn test_tenor() {
        assert_eq!(RateIndex::Sofr.tenor(), Tenor::Overnight);
        assert_eq!(RateIndex::Tonar.tenor(), Tenor::Overnight);
        assert_eq!(RateIndex::Euribor3M.tenor(), Tenor::ThreeMonths);
        assert_eq!(RateIndex::Euribor6M.tenor(), Tenor::SixMonths);
        assert_eq!(RateIndex::Sonia.tenor(), Tenor::Overnight);
        assert_eq!(RateIndex::Saron.tenor(), Tenor::Overnight);
    }

    #[test]
    fn test_day_counter() {
        assert_eq!(RateIndex::Sofr.day_counter(), DayCounter::Actual360);
        assert_eq!(RateIndex::Tonar.day_counter(), DayCounter::Actual365Fixed);
        assert_eq!(RateIndex::Euribor3M.day_counter(), DayCounter::Actual360);
        assert_eq!(RateIndex::Euribor6M.day_counter(), DayCounter::Actual360);
        assert_eq!(RateIndex::Sonia.day_counter(), DayCounter::Actual365Fixed);
        assert_eq!(RateIndex::Saron.day_counter(), DayCounter::Actual360);
    }

    #[test]
    fn test_name() {
        assert_eq!(RateIndex::Sofr.name(), "SOFR");
        assert_eq!(RateIndex::Tonar.name(), "TONAR");
        assert_eq!(RateIndex::Euribor3M.name(), "EURIBOR 3M");
        assert_eq!(RateIndex::Euribor6M.name(), "EURIBOR 6M");
        assert_eq!(RateIndex::Sonia.name(), "SONIA");
        assert_eq!(RateIndex::Saron.name(), "SARON");
    }

    #[test]
    fn test_code() {
        assert_eq!(RateIndex::Sofr.code(), "SOFR");
        assert_eq!(RateIndex::Tonar.code(), "TONAR");
        assert_eq!(RateIndex::Euribor3M.code(), "EUR3M");
        assert_eq!(RateIndex::Euribor6M.code(), "EUR6M");
        assert_eq!(RateIndex::Sonia.code(), "SONIA");
        assert_eq!(RateIndex::Saron.code(), "SARON");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", RateIndex::Sofr), "SOFR");
        assert_eq!(format!("{}", RateIndex::Euribor3M), "EURIBOR 3M");
    }

    #[test]
    fn test_from_str() {
        assert_eq!("SOFR".parse::<RateIndex>().unwrap(), RateIndex::Sofr);
        assert_eq!("sofr".parse::<RateIndex>().unwrap(), RateIndex::Sofr);
        assert_eq!("TONAR".parse::<RateIndex>().unwrap(), RateIndex::Tonar);
        assert_eq!(
            "EURIBOR3M".parse::<RateIndex>().unwrap(),
            RateIndex::Euribor3M
        );
        assert_eq!("EUR3M".parse::<RateIndex>().unwrap(), RateIndex::Euribor3M);
        assert_eq!(
            "EURIBOR 3M".parse::<RateIndex>().unwrap(),
            RateIndex::Euribor3M
        );
        assert_eq!(
            "EURIBOR6M".parse::<RateIndex>().unwrap(),
            RateIndex::Euribor6M
        );
        assert_eq!("SONIA".parse::<RateIndex>().unwrap(), RateIndex::Sonia);
        assert_eq!("SARON".parse::<RateIndex>().unwrap(), RateIndex::Saron);
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("UNKNOWN".parse::<RateIndex>().is_err());
        assert!("LIBOR".parse::<RateIndex>().is_err());
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(RateIndex::Sofr);
        set.insert(RateIndex::Tonar);
        set.insert(RateIndex::Sofr); // Duplicate
        assert_eq!(set.len(), 2);
    }
}
