//! Benchmark rate index definitions.
//!
//! This module provides rate index types for financial instruments.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::{RateIndex, Currency, IndexMetadata, CompoundingMethod};
//! use infra_domain::time::{DayCounter, Tenor, CalendarId};
//!
//! let sofr = RateIndex::Sofr;
//! assert_eq!(sofr.currency(), Currency::USD);
//! assert_eq!(sofr.tenor(), Tenor::Overnight);
//! assert_eq!(sofr.day_counter(), DayCounter::Actual360);
//!
//! // Access full metadata
//! let metadata = sofr.metadata();
//! assert_eq!(metadata.compounding_method, CompoundingMethod::Compounded);
//! assert!(sofr.is_overnight());
//! ```

use crate::{
    market::core::{CompoundingMethod, Currency},
    time::{CalendarId, DayCounter, Tenor},
};

/// Metadata for a rate index.
///
/// Contains all the market conventions associated with a benchmark rate index.
///
/// # Examples
///
/// ```
/// use infra_domain::market::{RateIndex, IndexMetadata, CompoundingMethod};
/// use infra_domain::time::{DayCounter, CalendarId};
///
/// let metadata = RateIndex::Sofr.metadata();
/// assert_eq!(metadata.compounding_method, CompoundingMethod::Compounded);
/// assert_eq!(metadata.fixing_lag, 0);
/// assert_eq!(metadata.settlement_lag, 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IndexMetadata {
    /// Compounding method for interest calculation.
    pub compounding_method: CompoundingMethod,
    /// Number of business days between observation and fixing.
    pub fixing_lag: u8,
    /// Number of business days for settlement.
    pub settlement_lag: u8,
    /// Day count convention for accrual calculation.
    pub day_counter: DayCounter,
    /// Holiday calendar for the index.
    pub calendar: CalendarId,
}

/// Benchmark rate index.
///
/// Represents standard benchmark interest rate indices used in financial
/// markets.
///
/// # Examples
///
/// ```
/// use infra_domain::market::{RateIndex, Currency};
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
    /// Euro Short-Term Rate (EUR overnight)
    Estr,
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
    /// Returns all supported rate indices.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::RateIndex;
    ///
    /// let indices = RateIndex::all();
    /// assert_eq!(indices.len(), 7);
    /// assert!(indices.contains(&RateIndex::Sofr));
    /// ```
    #[must_use]
    pub const fn all() -> [RateIndex; 7] {
        [
            RateIndex::Sofr,
            RateIndex::Tonar,
            RateIndex::Estr,
            RateIndex::Euribor3M,
            RateIndex::Euribor6M,
            RateIndex::Sonia,
            RateIndex::Saron,
        ]
    }

    /// Returns all rate index codes for API validation.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::RateIndex;
    ///
    /// let codes = RateIndex::all_codes();
    /// assert!(codes.contains(&"SOFR"));
    /// assert!(codes.contains(&"EURIBOR3M"));
    /// ```
    #[must_use]
    pub const fn all_codes() -> [&'static str; 7] {
        [
            "SOFR",
            "TONAR",
            "ESTR",
            "EURIBOR3M",
            "EURIBOR6M",
            "SONIA",
            "SARON",
        ]
    }

    /// Returns the API code for this rate index (no spaces, suitable for JSON).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::RateIndex;
    ///
    /// assert_eq!(RateIndex::Sofr.api_code(), "SOFR");
    /// assert_eq!(RateIndex::Euribor3M.api_code(), "EURIBOR3M");
    /// ```
    #[must_use]
    pub const fn api_code(&self) -> &'static str {
        match self {
            Self::Sofr => "SOFR",
            Self::Tonar => "TONAR",
            Self::Estr => "ESTR",
            Self::Euribor3M => "EURIBOR3M",
            Self::Euribor6M => "EURIBOR6M",
            Self::Sonia => "SONIA",
            Self::Saron => "SARON",
        }
    }

    /// Returns the default rate index for a given currency.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{RateIndex, Currency};
    ///
    /// assert_eq!(RateIndex::default_for_currency(Currency::USD), RateIndex::Sofr);
    /// assert_eq!(RateIndex::default_for_currency(Currency::EUR), RateIndex::Euribor3M);
    /// ```
    #[must_use]
    pub const fn default_for_currency(currency: Currency) -> Self {
        match currency {
            Currency::USD => Self::Sofr,
            Currency::EUR => Self::Euribor3M,
            Currency::GBP => Self::Sonia,
            Currency::JPY => Self::Tonar,
            Currency::CHF => Self::Saron,
        }
    }

    /// Returns the currency associated with this rate index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{RateIndex, Currency};
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
            Self::Estr | Self::Euribor3M | Self::Euribor6M => Currency::EUR,
            Self::Sonia => Currency::GBP,
            Self::Saron => Currency::CHF,
        }
    }

    /// Returns the standard fixing tenor for this rate index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::RateIndex;
    /// use infra_domain::time::Tenor;
    ///
    /// assert_eq!(RateIndex::Sofr.tenor(), Tenor::Overnight);
    /// assert_eq!(RateIndex::Euribor3M.tenor(), Tenor::ThreeMonths);
    /// assert_eq!(RateIndex::Euribor6M.tenor(), Tenor::SixMonths);
    /// ```
    #[must_use]
    pub const fn tenor(&self) -> Tenor {
        match self {
            Self::Sofr | Self::Tonar | Self::Estr | Self::Sonia | Self::Saron => Tenor::Overnight,
            Self::Euribor3M => Tenor::ThreeMonths,
            Self::Euribor6M => Tenor::SixMonths,
        }
    }

    /// Returns the day count convention for this rate index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::RateIndex;
    /// use infra_domain::time::DayCounter;
    ///
    /// assert_eq!(RateIndex::Sofr.day_counter(), DayCounter::Actual360);
    /// assert_eq!(RateIndex::Sonia.day_counter(), DayCounter::Actual365Fixed);
    /// ```
    #[must_use]
    pub const fn day_counter(&self) -> DayCounter {
        match self {
            // USD, EUR, CHF use ACT/360
            Self::Sofr | Self::Estr | Self::Euribor3M | Self::Euribor6M | Self::Saron => {
                DayCounter::Actual360
            }
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
    /// use infra_domain::market::RateIndex;
    ///
    /// assert_eq!(RateIndex::Sofr.name(), "SOFR");
    /// assert_eq!(RateIndex::Euribor3M.name(), "EURIBOR 3M");
    /// ```
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Sofr => "SOFR",
            Self::Tonar => "TONAR",
            Self::Estr => "ESTR",
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
    /// use infra_domain::market::RateIndex;
    ///
    /// assert_eq!(RateIndex::Sofr.code(), "SOFR");
    /// assert_eq!(RateIndex::Euribor3M.code(), "EUR3M");
    /// ```
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Sofr => "SOFR",
            Self::Tonar => "TONAR",
            Self::Estr => "ESTR",
            Self::Euribor3M => "EUR3M",
            Self::Euribor6M => "EUR6M",
            Self::Sonia => "SONIA",
            Self::Saron => "SARON",
        }
    }

    /// Returns the full metadata for this rate index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{RateIndex, IndexMetadata, CompoundingMethod};
    /// use infra_domain::time::{DayCounter, CalendarId};
    ///
    /// let metadata = RateIndex::Sofr.metadata();
    /// assert_eq!(metadata.compounding_method, CompoundingMethod::Compounded);
    /// assert_eq!(metadata.day_counter, DayCounter::Actual360);
    /// ```
    #[must_use]
    pub const fn metadata(&self) -> IndexMetadata {
        match self {
            Self::Sofr => IndexMetadata {
                compounding_method: CompoundingMethod::Compounded,
                fixing_lag: 0,
                settlement_lag: 2,
                day_counter: DayCounter::Actual360,
                calendar: CalendarId::NewYork,
            },
            Self::Tonar => IndexMetadata {
                compounding_method: CompoundingMethod::Compounded,
                fixing_lag: 0,
                settlement_lag: 2,
                day_counter: DayCounter::Actual365Fixed,
                calendar: CalendarId::Tokyo,
            },
            Self::Estr => IndexMetadata {
                compounding_method: CompoundingMethod::Compounded,
                fixing_lag: 0,
                settlement_lag: 2,
                day_counter: DayCounter::Actual360,
                calendar: CalendarId::Target,
            },
            Self::Sonia => IndexMetadata {
                compounding_method: CompoundingMethod::Compounded,
                fixing_lag: 0,
                settlement_lag: 0,
                day_counter: DayCounter::Actual365Fixed,
                calendar: CalendarId::London,
            },
            Self::Saron => IndexMetadata {
                compounding_method: CompoundingMethod::Compounded,
                fixing_lag: 0,
                settlement_lag: 2,
                day_counter: DayCounter::Actual360,
                calendar: CalendarId::Target, // Swiss calendar not yet defined
            },
            Self::Euribor3M | Self::Euribor6M => IndexMetadata {
                compounding_method: CompoundingMethod::Simple,
                fixing_lag: 2,
                settlement_lag: 2,
                day_counter: DayCounter::Actual360,
                calendar: CalendarId::Target,
            },
        }
    }

    /// Returns true if this is an overnight index (RFR).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::RateIndex;
    ///
    /// assert!(RateIndex::Sofr.is_overnight());
    /// assert!(RateIndex::Sonia.is_overnight());
    /// assert!(!RateIndex::Euribor3M.is_overnight());
    /// ```
    #[must_use]
    pub const fn is_overnight(&self) -> bool {
        matches!(
            self,
            Self::Sofr | Self::Tonar | Self::Estr | Self::Sonia | Self::Saron
        )
    }

    /// Returns true if this is a term index (IBOR-style).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::RateIndex;
    ///
    /// assert!(!RateIndex::Sofr.is_term_index());
    /// assert!(RateIndex::Euribor3M.is_term_index());
    /// ```
    #[must_use]
    pub const fn is_term_index(&self) -> bool { !self.is_overnight() }
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
            "ESTR" | "€STR" | "ESTER" => Ok(Self::Estr),
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
        assert_eq!(RateIndex::Estr.currency(), Currency::EUR);
        assert_eq!(RateIndex::Euribor3M.currency(), Currency::EUR);
        assert_eq!(RateIndex::Euribor6M.currency(), Currency::EUR);
        assert_eq!(RateIndex::Sonia.currency(), Currency::GBP);
        assert_eq!(RateIndex::Saron.currency(), Currency::CHF);
    }

    #[test]
    fn test_tenor() {
        assert_eq!(RateIndex::Sofr.tenor(), Tenor::Overnight);
        assert_eq!(RateIndex::Tonar.tenor(), Tenor::Overnight);
        assert_eq!(RateIndex::Estr.tenor(), Tenor::Overnight);
        assert_eq!(RateIndex::Euribor3M.tenor(), Tenor::ThreeMonths);
        assert_eq!(RateIndex::Euribor6M.tenor(), Tenor::SixMonths);
        assert_eq!(RateIndex::Sonia.tenor(), Tenor::Overnight);
        assert_eq!(RateIndex::Saron.tenor(), Tenor::Overnight);
    }

    #[test]
    fn test_day_counter() {
        assert_eq!(RateIndex::Sofr.day_counter(), DayCounter::Actual360);
        assert_eq!(RateIndex::Tonar.day_counter(), DayCounter::Actual365Fixed);
        assert_eq!(RateIndex::Estr.day_counter(), DayCounter::Actual360);
        assert_eq!(RateIndex::Euribor3M.day_counter(), DayCounter::Actual360);
        assert_eq!(RateIndex::Euribor6M.day_counter(), DayCounter::Actual360);
        assert_eq!(RateIndex::Sonia.day_counter(), DayCounter::Actual365Fixed);
        assert_eq!(RateIndex::Saron.day_counter(), DayCounter::Actual360);
    }

    #[test]
    fn test_name() {
        assert_eq!(RateIndex::Sofr.name(), "SOFR");
        assert_eq!(RateIndex::Tonar.name(), "TONAR");
        assert_eq!(RateIndex::Estr.name(), "ESTR");
        assert_eq!(RateIndex::Euribor3M.name(), "EURIBOR 3M");
        assert_eq!(RateIndex::Euribor6M.name(), "EURIBOR 6M");
        assert_eq!(RateIndex::Sonia.name(), "SONIA");
        assert_eq!(RateIndex::Saron.name(), "SARON");
    }

    #[test]
    fn test_code() {
        assert_eq!(RateIndex::Sofr.code(), "SOFR");
        assert_eq!(RateIndex::Tonar.code(), "TONAR");
        assert_eq!(RateIndex::Estr.code(), "ESTR");
        assert_eq!(RateIndex::Euribor3M.code(), "EUR3M");
        assert_eq!(RateIndex::Euribor6M.code(), "EUR6M");
        assert_eq!(RateIndex::Sonia.code(), "SONIA");
        assert_eq!(RateIndex::Saron.code(), "SARON");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", RateIndex::Sofr), "SOFR");
        assert_eq!(format!("{}", RateIndex::Estr), "ESTR");
        assert_eq!(format!("{}", RateIndex::Euribor3M), "EURIBOR 3M");
    }

    #[test]
    fn test_from_str() {
        assert_eq!("SOFR".parse::<RateIndex>().unwrap(), RateIndex::Sofr);
        assert_eq!("sofr".parse::<RateIndex>().unwrap(), RateIndex::Sofr);
        assert_eq!("TONAR".parse::<RateIndex>().unwrap(), RateIndex::Tonar);
        assert_eq!("ESTR".parse::<RateIndex>().unwrap(), RateIndex::Estr);
        assert_eq!("estr".parse::<RateIndex>().unwrap(), RateIndex::Estr);
        assert_eq!("ESTER".parse::<RateIndex>().unwrap(), RateIndex::Estr);
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
        set.insert(RateIndex::Estr);
        set.insert(RateIndex::Sofr); // Duplicate
        assert_eq!(set.len(), 3);
    }

    // ========================================
    // IndexMetadata Tests
    // ========================================

    #[test]
    fn test_metadata_sofr() {
        let metadata = RateIndex::Sofr.metadata();
        assert_eq!(metadata.compounding_method, CompoundingMethod::Compounded);
        assert_eq!(metadata.fixing_lag, 0);
        assert_eq!(metadata.settlement_lag, 2);
        assert_eq!(metadata.day_counter, DayCounter::Actual360);
        assert_eq!(metadata.calendar, CalendarId::NewYork);
    }

    #[test]
    fn test_metadata_sonia() {
        let metadata = RateIndex::Sonia.metadata();
        assert_eq!(metadata.compounding_method, CompoundingMethod::Compounded);
        assert_eq!(metadata.fixing_lag, 0);
        assert_eq!(metadata.settlement_lag, 0); // SONIA has T+0 settlement
        assert_eq!(metadata.day_counter, DayCounter::Actual365Fixed);
        assert_eq!(metadata.calendar, CalendarId::London);
    }

    #[test]
    fn test_metadata_euribor() {
        let metadata = RateIndex::Euribor3M.metadata();
        assert_eq!(metadata.compounding_method, CompoundingMethod::Simple);
        assert_eq!(metadata.fixing_lag, 2); // IBOR has T-2 fixing
        assert_eq!(metadata.settlement_lag, 2);
        assert_eq!(metadata.day_counter, DayCounter::Actual360);
        assert_eq!(metadata.calendar, CalendarId::Target);
    }

    #[test]
    fn test_metadata_estr() {
        let metadata = RateIndex::Estr.metadata();
        assert_eq!(metadata.compounding_method, CompoundingMethod::Compounded);
        assert_eq!(metadata.fixing_lag, 0);
        assert_eq!(metadata.day_counter, DayCounter::Actual360);
        assert_eq!(metadata.calendar, CalendarId::Target);
    }

    // ========================================
    // is_overnight / is_term_index Tests
    // ========================================

    #[test]
    fn test_is_overnight() {
        assert!(RateIndex::Sofr.is_overnight());
        assert!(RateIndex::Tonar.is_overnight());
        assert!(RateIndex::Estr.is_overnight());
        assert!(RateIndex::Sonia.is_overnight());
        assert!(RateIndex::Saron.is_overnight());
        assert!(!RateIndex::Euribor3M.is_overnight());
        assert!(!RateIndex::Euribor6M.is_overnight());
    }

    #[test]
    fn test_is_term_index() {
        assert!(!RateIndex::Sofr.is_term_index());
        assert!(!RateIndex::Tonar.is_term_index());
        assert!(!RateIndex::Estr.is_term_index());
        assert!(!RateIndex::Sonia.is_term_index());
        assert!(!RateIndex::Saron.is_term_index());
        assert!(RateIndex::Euribor3M.is_term_index());
        assert!(RateIndex::Euribor6M.is_term_index());
    }

    #[test]
    fn test_metadata_clone() {
        let metadata = RateIndex::Sofr.metadata();
        let cloned = metadata;
        assert_eq!(metadata, cloned);
    }

    #[test]
    fn test_metadata_debug() {
        let metadata = RateIndex::Sofr.metadata();
        let debug = format!("{:?}", metadata);
        assert!(debug.contains("IndexMetadata"));
        assert!(debug.contains("Compounded"));
    }
}
