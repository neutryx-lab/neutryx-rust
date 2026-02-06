//! FX fixing index definitions.
//!
//! This module provides FX fixing index types used for FX derivatives.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::{FxIndex, Currency};
//! use infra_domain::time::CalendarId;
//!
//! let ecb_eurusd = FxIndex::EcbEurUsd;
//! assert_eq!(ecb_eurusd.base_currency(), Currency::EUR);
//! assert_eq!(ecb_eurusd.quote_currency(), Currency::USD);
//! ```

use crate::{market::core::Currency, time::CalendarId};

/// FX fixing source.
///
/// Identifies the data provider or central bank publishing the FX fixing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FxFixingSource {
    /// European Central Bank
    Ecb,
    /// WM/Refinitiv (formerly WM/Reuters)
    WmReuters,
    /// Bank of Japan
    Boj,
    /// Bank of England
    Boe,
    /// Swiss National Bank
    Snb,
}

impl FxFixingSource {
    /// Returns the human-readable name of this fixing source.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Ecb => "ECB",
            Self::WmReuters => "WM/Reuters",
            Self::Boj => "BOJ",
            Self::Boe => "BOE",
            Self::Snb => "SNB",
        }
    }
}

impl std::fmt::Display for FxFixingSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Metadata for an FX fixing index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxIndexMetadata {
    /// Fixing source (ECB, WM/Reuters, etc.)
    pub source: FxFixingSource,
    /// Number of business days between trade date and fixing.
    pub fixing_lag: u8,
    /// Number of business days for settlement.
    pub settlement_lag: u8,
    /// Holiday calendar for the fixing.
    pub calendar: CalendarId,
    /// Fixing time in format "HH:MM" (local time of the source).
    pub fixing_time: &'static str,
}

/// FX fixing index.
///
/// Represents standard FX fixing indices used in FX derivatives.
///
/// # Examples
///
/// ```
/// use infra_domain::market::{FxIndex, Currency, FxFixingSource};
///
/// let wmr = FxIndex::WmrUsdJpy;
/// assert_eq!(wmr.base_currency(), Currency::USD);
/// assert_eq!(wmr.quote_currency(), Currency::JPY);
/// assert_eq!(wmr.source(), FxFixingSource::WmReuters);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum FxIndex {
    // ECB fixings (EUR-based)
    /// ECB EUR/USD fixing
    EcbEurUsd,
    /// ECB EUR/GBP fixing
    EcbEurGbp,
    /// ECB EUR/JPY fixing
    EcbEurJpy,
    /// ECB EUR/CHF fixing
    EcbEurChf,

    // WM/Reuters fixings (4pm London)
    /// WM/Reuters USD/JPY fixing
    WmrUsdJpy,
    /// WM/Reuters EUR/USD fixing
    WmrEurUsd,
    /// WM/Reuters GBP/USD fixing
    WmrGbpUsd,
    /// WM/Reuters USD/CHF fixing
    WmrUsdChf,

    // BOJ fixings
    /// Bank of Japan USD/JPY fixing
    BojUsdJpy,
}

impl FxIndex {
    /// Returns all supported FX indices.
    #[must_use]
    pub const fn all() -> [FxIndex; 9] {
        [
            FxIndex::EcbEurUsd,
            FxIndex::EcbEurGbp,
            FxIndex::EcbEurJpy,
            FxIndex::EcbEurChf,
            FxIndex::WmrUsdJpy,
            FxIndex::WmrEurUsd,
            FxIndex::WmrGbpUsd,
            FxIndex::WmrUsdChf,
            FxIndex::BojUsdJpy,
        ]
    }

    /// Returns the API code for this FX index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::FxIndex;
    ///
    /// assert_eq!(FxIndex::EcbEurUsd.api_code(), "ECB-EURUSD");
    /// assert_eq!(FxIndex::WmrUsdJpy.api_code(), "WMR-USDJPY");
    /// ```
    #[must_use]
    pub const fn api_code(&self) -> &'static str {
        match self {
            Self::EcbEurUsd => "ECB-EURUSD",
            Self::EcbEurGbp => "ECB-EURGBP",
            Self::EcbEurJpy => "ECB-EURJPY",
            Self::EcbEurChf => "ECB-EURCHF",
            Self::WmrUsdJpy => "WMR-USDJPY",
            Self::WmrEurUsd => "WMR-EURUSD",
            Self::WmrGbpUsd => "WMR-GBPUSD",
            Self::WmrUsdChf => "WMR-USDCHF",
            Self::BojUsdJpy => "BOJ-USDJPY",
        }
    }

    /// Returns the base currency (numerator) of this FX pair.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{FxIndex, Currency};
    ///
    /// assert_eq!(FxIndex::EcbEurUsd.base_currency(), Currency::EUR);
    /// assert_eq!(FxIndex::WmrUsdJpy.base_currency(), Currency::USD);
    /// ```
    #[must_use]
    pub const fn base_currency(&self) -> Currency {
        match self {
            Self::EcbEurUsd | Self::EcbEurGbp | Self::EcbEurJpy | Self::EcbEurChf => Currency::EUR,
            Self::WmrUsdJpy | Self::WmrUsdChf => Currency::USD,
            Self::WmrEurUsd => Currency::EUR,
            Self::WmrGbpUsd => Currency::GBP,
            Self::BojUsdJpy => Currency::USD,
        }
    }

    /// Returns the quote currency (denominator) of this FX pair.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{FxIndex, Currency};
    ///
    /// assert_eq!(FxIndex::EcbEurUsd.quote_currency(), Currency::USD);
    /// assert_eq!(FxIndex::WmrUsdJpy.quote_currency(), Currency::JPY);
    /// ```
    #[must_use]
    pub const fn quote_currency(&self) -> Currency {
        match self {
            Self::EcbEurUsd | Self::WmrEurUsd => Currency::USD,
            Self::EcbEurGbp => Currency::GBP,
            Self::EcbEurJpy | Self::WmrUsdJpy | Self::BojUsdJpy => Currency::JPY,
            Self::EcbEurChf | Self::WmrUsdChf => Currency::CHF,
            Self::WmrGbpUsd => Currency::USD,
        }
    }

    /// Returns the fixing source for this index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{FxIndex, FxFixingSource};
    ///
    /// assert_eq!(FxIndex::EcbEurUsd.source(), FxFixingSource::Ecb);
    /// assert_eq!(FxIndex::WmrUsdJpy.source(), FxFixingSource::WmReuters);
    /// ```
    #[must_use]
    pub const fn source(&self) -> FxFixingSource {
        match self {
            Self::EcbEurUsd | Self::EcbEurGbp | Self::EcbEurJpy | Self::EcbEurChf => {
                FxFixingSource::Ecb
            }
            Self::WmrUsdJpy | Self::WmrEurUsd | Self::WmrGbpUsd | Self::WmrUsdChf => {
                FxFixingSource::WmReuters
            }
            Self::BojUsdJpy => FxFixingSource::Boj,
        }
    }

    /// Returns the human-readable name of this FX index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::FxIndex;
    ///
    /// assert_eq!(FxIndex::EcbEurUsd.name(), "ECB EUR/USD");
    /// assert_eq!(FxIndex::WmrUsdJpy.name(), "WMR USD/JPY");
    /// ```
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::EcbEurUsd => "ECB EUR/USD",
            Self::EcbEurGbp => "ECB EUR/GBP",
            Self::EcbEurJpy => "ECB EUR/JPY",
            Self::EcbEurChf => "ECB EUR/CHF",
            Self::WmrUsdJpy => "WMR USD/JPY",
            Self::WmrEurUsd => "WMR EUR/USD",
            Self::WmrGbpUsd => "WMR GBP/USD",
            Self::WmrUsdChf => "WMR USD/CHF",
            Self::BojUsdJpy => "BOJ USD/JPY",
        }
    }

    /// Returns the full metadata for this FX index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{FxIndex, FxFixingSource};
    /// use infra_domain::time::CalendarId;
    ///
    /// let metadata = FxIndex::EcbEurUsd.metadata();
    /// assert_eq!(metadata.source, FxFixingSource::Ecb);
    /// assert_eq!(metadata.fixing_time, "14:15");
    /// ```
    #[must_use]
    pub const fn metadata(&self) -> FxIndexMetadata {
        match self {
            Self::EcbEurUsd | Self::EcbEurGbp | Self::EcbEurJpy | Self::EcbEurChf => {
                FxIndexMetadata {
                    source: FxFixingSource::Ecb,
                    fixing_lag: 0,
                    settlement_lag: 2,
                    calendar: CalendarId::Target,
                    fixing_time: "14:15", // ECB publishes at 14:15 CET
                }
            }
            Self::WmrUsdJpy | Self::WmrEurUsd | Self::WmrGbpUsd | Self::WmrUsdChf => {
                FxIndexMetadata {
                    source: FxFixingSource::WmReuters,
                    fixing_lag: 0,
                    settlement_lag: 2,
                    calendar: CalendarId::London,
                    fixing_time: "16:00", // WM/Reuters 4pm London fix
                }
            }
            Self::BojUsdJpy => FxIndexMetadata {
                source: FxFixingSource::Boj,
                fixing_lag: 0,
                settlement_lag: 2,
                calendar: CalendarId::Tokyo,
                fixing_time: "09:55", // BOJ publishes around 9:55 JST
            },
        }
    }

    /// Returns the currency pair string (e.g., "EUR/USD").
    #[must_use]
    pub const fn pair(&self) -> &'static str {
        match self {
            Self::EcbEurUsd | Self::WmrEurUsd => "EUR/USD",
            Self::EcbEurGbp => "EUR/GBP",
            Self::EcbEurJpy => "EUR/JPY",
            Self::EcbEurChf => "EUR/CHF",
            Self::WmrUsdJpy | Self::BojUsdJpy => "USD/JPY",
            Self::WmrGbpUsd => "GBP/USD",
            Self::WmrUsdChf => "USD/CHF",
        }
    }
}

impl std::fmt::Display for FxIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for FxIndex {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().replace(['/', '-', ' '], "").as_str() {
            "ECBEURUSD" => Ok(Self::EcbEurUsd),
            "ECBEURGBP" => Ok(Self::EcbEurGbp),
            "ECBEURJPY" => Ok(Self::EcbEurJpy),
            "ECBEURCHF" => Ok(Self::EcbEurChf),
            "WMRUSDJPY" | "WMRJPY" => Ok(Self::WmrUsdJpy),
            "WMREURUSD" => Ok(Self::WmrEurUsd),
            "WMRGBPUSD" => Ok(Self::WmrGbpUsd),
            "WMRUSDCHF" => Ok(Self::WmrUsdChf),
            "BOJUSDJPY" | "BOJJPY" => Ok(Self::BojUsdJpy),
            _ => Err(format!("Unknown FX index: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_quote_currency() {
        assert_eq!(FxIndex::EcbEurUsd.base_currency(), Currency::EUR);
        assert_eq!(FxIndex::EcbEurUsd.quote_currency(), Currency::USD);

        assert_eq!(FxIndex::WmrUsdJpy.base_currency(), Currency::USD);
        assert_eq!(FxIndex::WmrUsdJpy.quote_currency(), Currency::JPY);

        assert_eq!(FxIndex::WmrGbpUsd.base_currency(), Currency::GBP);
        assert_eq!(FxIndex::WmrGbpUsd.quote_currency(), Currency::USD);
    }

    #[test]
    fn test_source() {
        assert_eq!(FxIndex::EcbEurUsd.source(), FxFixingSource::Ecb);
        assert_eq!(FxIndex::EcbEurGbp.source(), FxFixingSource::Ecb);
        assert_eq!(FxIndex::WmrUsdJpy.source(), FxFixingSource::WmReuters);
        assert_eq!(FxIndex::BojUsdJpy.source(), FxFixingSource::Boj);
    }

    #[test]
    fn test_metadata() {
        let ecb = FxIndex::EcbEurUsd.metadata();
        assert_eq!(ecb.source, FxFixingSource::Ecb);
        assert_eq!(ecb.fixing_time, "14:15");
        assert_eq!(ecb.calendar, CalendarId::Target);

        let wmr = FxIndex::WmrUsdJpy.metadata();
        assert_eq!(wmr.source, FxFixingSource::WmReuters);
        assert_eq!(wmr.fixing_time, "16:00");
        assert_eq!(wmr.calendar, CalendarId::London);

        let boj = FxIndex::BojUsdJpy.metadata();
        assert_eq!(boj.source, FxFixingSource::Boj);
        assert_eq!(boj.fixing_time, "09:55");
        assert_eq!(boj.calendar, CalendarId::Tokyo);
    }

    #[test]
    fn test_name() {
        assert_eq!(FxIndex::EcbEurUsd.name(), "ECB EUR/USD");
        assert_eq!(FxIndex::WmrUsdJpy.name(), "WMR USD/JPY");
        assert_eq!(FxIndex::BojUsdJpy.name(), "BOJ USD/JPY");
    }

    #[test]
    fn test_api_code() {
        assert_eq!(FxIndex::EcbEurUsd.api_code(), "ECB-EURUSD");
        assert_eq!(FxIndex::WmrUsdJpy.api_code(), "WMR-USDJPY");
        assert_eq!(FxIndex::BojUsdJpy.api_code(), "BOJ-USDJPY");
    }

    #[test]
    fn test_pair() {
        assert_eq!(FxIndex::EcbEurUsd.pair(), "EUR/USD");
        assert_eq!(FxIndex::WmrUsdJpy.pair(), "USD/JPY");
        assert_eq!(FxIndex::WmrGbpUsd.pair(), "GBP/USD");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", FxIndex::EcbEurUsd), "ECB EUR/USD");
        assert_eq!(format!("{}", FxIndex::WmrUsdJpy), "WMR USD/JPY");
    }

    #[test]
    fn test_from_str() {
        assert_eq!("ECB-EURUSD".parse::<FxIndex>().unwrap(), FxIndex::EcbEurUsd);
        assert_eq!(
            "ecb-eur-usd".parse::<FxIndex>().unwrap(),
            FxIndex::EcbEurUsd
        );
        assert_eq!("WMR-USDJPY".parse::<FxIndex>().unwrap(), FxIndex::WmrUsdJpy);
        assert_eq!("BOJ-USDJPY".parse::<FxIndex>().unwrap(), FxIndex::BojUsdJpy);
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("UNKNOWN".parse::<FxIndex>().is_err());
        assert!("LIBOR".parse::<FxIndex>().is_err());
    }

    #[test]
    fn test_all() {
        let all = FxIndex::all();
        assert_eq!(all.len(), 9);
        assert!(all.contains(&FxIndex::EcbEurUsd));
        assert!(all.contains(&FxIndex::WmrUsdJpy));
        assert!(all.contains(&FxIndex::BojUsdJpy));
    }

    #[test]
    fn test_fixing_source_display() {
        assert_eq!(format!("{}", FxFixingSource::Ecb), "ECB");
        assert_eq!(format!("{}", FxFixingSource::WmReuters), "WM/Reuters");
        assert_eq!(format!("{}", FxFixingSource::Boj), "BOJ");
    }
}
