//! Swap index definitions for CMS (Constant Maturity Swap).
//!
//! This module provides swap index types used for CMS products.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::{SwapIndex, Currency, RateIndex};
//! use infra_domain::time::Tenor;
//!
//! let cms10y = SwapIndex::UsdCms10Y;
//! assert_eq!(cms10y.currency(), Currency::USD);
//! assert_eq!(cms10y.tenor(), Tenor::TenYears);
//! assert_eq!(cms10y.underlying_index(), RateIndex::Sofr);
//! ```

use super::RateIndex;
use crate::{
    market::core::Currency,
    time::{CalendarId, DayCounter, Tenor},
};

/// Metadata for a swap index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwapIndexMetadata {
    /// Underlying rate index for the floating leg.
    pub underlying_index: RateIndex,
    /// Fixed leg frequency.
    pub fixed_frequency: Tenor,
    /// Floating leg frequency.
    pub float_frequency: Tenor,
    /// Day count convention for fixed leg.
    pub fixed_day_counter: DayCounter,
    /// Day count convention for floating leg.
    pub float_day_counter: DayCounter,
    /// Number of business days for settlement.
    pub settlement_lag: u8,
    /// Holiday calendar for the swap.
    pub calendar: CalendarId,
}

/// Swap index for CMS (Constant Maturity Swap) products.
///
/// Represents standard swap rate indices used in CMS swaps, caps, and floors.
///
/// # Examples
///
/// ```
/// use infra_domain::market::{SwapIndex, Currency};
/// use infra_domain::time::Tenor;
///
/// let eur_cms = SwapIndex::EurCms10Y;
/// assert_eq!(eur_cms.currency(), Currency::EUR);
/// assert_eq!(eur_cms.tenor(), Tenor::TenYears);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum SwapIndex {
    // USD CMS (SOFR-based)
    /// USD CMS 2Y
    UsdCms2Y,
    /// USD CMS 5Y
    UsdCms5Y,
    /// USD CMS 10Y
    UsdCms10Y,
    /// USD CMS 30Y
    UsdCms30Y,

    // EUR CMS (EURIBOR/ESTR-based)
    /// EUR CMS 2Y
    EurCms2Y,
    /// EUR CMS 5Y
    EurCms5Y,
    /// EUR CMS 10Y
    EurCms10Y,
    /// EUR CMS 30Y
    EurCms30Y,

    // GBP CMS (SONIA-based)
    /// GBP CMS 2Y
    GbpCms2Y,
    /// GBP CMS 5Y
    GbpCms5Y,
    /// GBP CMS 10Y
    GbpCms10Y,
    /// GBP CMS 30Y
    GbpCms30Y,

    // JPY CMS (TONAR-based)
    /// JPY CMS 2Y
    JpyCms2Y,
    /// JPY CMS 5Y
    JpyCms5Y,
    /// JPY CMS 10Y
    JpyCms10Y,
    /// JPY CMS 30Y
    JpyCms30Y,

    // CHF CMS (SARON-based)
    /// CHF CMS 2Y
    ChfCms2Y,
    /// CHF CMS 5Y
    ChfCms5Y,
    /// CHF CMS 10Y
    ChfCms10Y,
    /// CHF CMS 30Y
    ChfCms30Y,
}

impl SwapIndex {
    /// Returns all supported swap indices.
    #[must_use]
    pub const fn all() -> [SwapIndex; 20] {
        [
            SwapIndex::UsdCms2Y,
            SwapIndex::UsdCms5Y,
            SwapIndex::UsdCms10Y,
            SwapIndex::UsdCms30Y,
            SwapIndex::EurCms2Y,
            SwapIndex::EurCms5Y,
            SwapIndex::EurCms10Y,
            SwapIndex::EurCms30Y,
            SwapIndex::GbpCms2Y,
            SwapIndex::GbpCms5Y,
            SwapIndex::GbpCms10Y,
            SwapIndex::GbpCms30Y,
            SwapIndex::JpyCms2Y,
            SwapIndex::JpyCms5Y,
            SwapIndex::JpyCms10Y,
            SwapIndex::JpyCms30Y,
            SwapIndex::ChfCms2Y,
            SwapIndex::ChfCms5Y,
            SwapIndex::ChfCms10Y,
            SwapIndex::ChfCms30Y,
        ]
    }

    /// Returns the API code for this swap index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::SwapIndex;
    ///
    /// assert_eq!(SwapIndex::UsdCms10Y.api_code(), "USD-CMS-10Y");
    /// assert_eq!(SwapIndex::EurCms5Y.api_code(), "EUR-CMS-5Y");
    /// ```
    #[must_use]
    pub const fn api_code(&self) -> &'static str {
        match self {
            Self::UsdCms2Y => "USD-CMS-2Y",
            Self::UsdCms5Y => "USD-CMS-5Y",
            Self::UsdCms10Y => "USD-CMS-10Y",
            Self::UsdCms30Y => "USD-CMS-30Y",
            Self::EurCms2Y => "EUR-CMS-2Y",
            Self::EurCms5Y => "EUR-CMS-5Y",
            Self::EurCms10Y => "EUR-CMS-10Y",
            Self::EurCms30Y => "EUR-CMS-30Y",
            Self::GbpCms2Y => "GBP-CMS-2Y",
            Self::GbpCms5Y => "GBP-CMS-5Y",
            Self::GbpCms10Y => "GBP-CMS-10Y",
            Self::GbpCms30Y => "GBP-CMS-30Y",
            Self::JpyCms2Y => "JPY-CMS-2Y",
            Self::JpyCms5Y => "JPY-CMS-5Y",
            Self::JpyCms10Y => "JPY-CMS-10Y",
            Self::JpyCms30Y => "JPY-CMS-30Y",
            Self::ChfCms2Y => "CHF-CMS-2Y",
            Self::ChfCms5Y => "CHF-CMS-5Y",
            Self::ChfCms10Y => "CHF-CMS-10Y",
            Self::ChfCms30Y => "CHF-CMS-30Y",
        }
    }

    /// Returns the currency of this swap index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{SwapIndex, Currency};
    ///
    /// assert_eq!(SwapIndex::UsdCms10Y.currency(), Currency::USD);
    /// assert_eq!(SwapIndex::EurCms10Y.currency(), Currency::EUR);
    /// ```
    #[must_use]
    pub const fn currency(&self) -> Currency {
        match self {
            Self::UsdCms2Y | Self::UsdCms5Y | Self::UsdCms10Y | Self::UsdCms30Y => Currency::USD,
            Self::EurCms2Y | Self::EurCms5Y | Self::EurCms10Y | Self::EurCms30Y => Currency::EUR,
            Self::GbpCms2Y | Self::GbpCms5Y | Self::GbpCms10Y | Self::GbpCms30Y => Currency::GBP,
            Self::JpyCms2Y | Self::JpyCms5Y | Self::JpyCms10Y | Self::JpyCms30Y => Currency::JPY,
            Self::ChfCms2Y | Self::ChfCms5Y | Self::ChfCms10Y | Self::ChfCms30Y => Currency::CHF,
        }
    }

    /// Returns the tenor (maturity) of this swap index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::SwapIndex;
    /// use infra_domain::time::Tenor;
    ///
    /// assert_eq!(SwapIndex::UsdCms2Y.tenor(), Tenor::TwoYears);
    /// assert_eq!(SwapIndex::UsdCms10Y.tenor(), Tenor::TenYears);
    /// assert_eq!(SwapIndex::EurCms30Y.tenor(), Tenor::ThirtyYears);
    /// ```
    #[must_use]
    pub const fn tenor(&self) -> Tenor {
        match self {
            Self::UsdCms2Y | Self::EurCms2Y | Self::GbpCms2Y | Self::JpyCms2Y | Self::ChfCms2Y => {
                Tenor::TwoYears
            }
            Self::UsdCms5Y | Self::EurCms5Y | Self::GbpCms5Y | Self::JpyCms5Y | Self::ChfCms5Y => {
                Tenor::FiveYears
            }
            Self::UsdCms10Y
            | Self::EurCms10Y
            | Self::GbpCms10Y
            | Self::JpyCms10Y
            | Self::ChfCms10Y => Tenor::TenYears,
            Self::UsdCms30Y
            | Self::EurCms30Y
            | Self::GbpCms30Y
            | Self::JpyCms30Y
            | Self::ChfCms30Y => Tenor::ThirtyYears,
        }
    }

    /// Returns the underlying rate index for the floating leg.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{SwapIndex, RateIndex};
    ///
    /// assert_eq!(SwapIndex::UsdCms10Y.underlying_index(), RateIndex::Sofr);
    /// assert_eq!(SwapIndex::EurCms10Y.underlying_index(), RateIndex::Estr);
    /// assert_eq!(SwapIndex::GbpCms10Y.underlying_index(), RateIndex::Sonia);
    /// ```
    #[must_use]
    pub const fn underlying_index(&self) -> RateIndex {
        match self {
            Self::UsdCms2Y | Self::UsdCms5Y | Self::UsdCms10Y | Self::UsdCms30Y => RateIndex::Sofr,
            Self::EurCms2Y | Self::EurCms5Y | Self::EurCms10Y | Self::EurCms30Y => RateIndex::Estr,
            Self::GbpCms2Y | Self::GbpCms5Y | Self::GbpCms10Y | Self::GbpCms30Y => RateIndex::Sonia,
            Self::JpyCms2Y | Self::JpyCms5Y | Self::JpyCms10Y | Self::JpyCms30Y => RateIndex::Tonar,
            Self::ChfCms2Y | Self::ChfCms5Y | Self::ChfCms10Y | Self::ChfCms30Y => RateIndex::Saron,
        }
    }

    /// Returns the human-readable name of this swap index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::SwapIndex;
    ///
    /// assert_eq!(SwapIndex::UsdCms10Y.name(), "USD CMS 10Y");
    /// assert_eq!(SwapIndex::EurCms5Y.name(), "EUR CMS 5Y");
    /// ```
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::UsdCms2Y => "USD CMS 2Y",
            Self::UsdCms5Y => "USD CMS 5Y",
            Self::UsdCms10Y => "USD CMS 10Y",
            Self::UsdCms30Y => "USD CMS 30Y",
            Self::EurCms2Y => "EUR CMS 2Y",
            Self::EurCms5Y => "EUR CMS 5Y",
            Self::EurCms10Y => "EUR CMS 10Y",
            Self::EurCms30Y => "EUR CMS 30Y",
            Self::GbpCms2Y => "GBP CMS 2Y",
            Self::GbpCms5Y => "GBP CMS 5Y",
            Self::GbpCms10Y => "GBP CMS 10Y",
            Self::GbpCms30Y => "GBP CMS 30Y",
            Self::JpyCms2Y => "JPY CMS 2Y",
            Self::JpyCms5Y => "JPY CMS 5Y",
            Self::JpyCms10Y => "JPY CMS 10Y",
            Self::JpyCms30Y => "JPY CMS 30Y",
            Self::ChfCms2Y => "CHF CMS 2Y",
            Self::ChfCms5Y => "CHF CMS 5Y",
            Self::ChfCms10Y => "CHF CMS 10Y",
            Self::ChfCms30Y => "CHF CMS 30Y",
        }
    }

    /// Returns the full metadata for this swap index.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{SwapIndex, RateIndex};
    /// use infra_domain::time::{Tenor, DayCounter};
    ///
    /// let metadata = SwapIndex::UsdCms10Y.metadata();
    /// assert_eq!(metadata.underlying_index, RateIndex::Sofr);
    /// assert_eq!(metadata.fixed_frequency, Tenor::SixMonths);
    /// ```
    #[must_use]
    pub const fn metadata(&self) -> SwapIndexMetadata {
        match self.currency() {
            Currency::USD => SwapIndexMetadata {
                underlying_index: RateIndex::Sofr,
                fixed_frequency: Tenor::SixMonths,
                float_frequency: Tenor::ThreeMonths,
                fixed_day_counter: DayCounter::Thirty360Bond,
                float_day_counter: DayCounter::Actual360,
                settlement_lag: 2,
                calendar: CalendarId::NewYork,
            },
            Currency::EUR => SwapIndexMetadata {
                underlying_index: RateIndex::Estr,
                fixed_frequency: Tenor::OneYear,
                float_frequency: Tenor::SixMonths,
                fixed_day_counter: DayCounter::Thirty360Bond,
                float_day_counter: DayCounter::Actual360,
                settlement_lag: 2,
                calendar: CalendarId::Target,
            },
            Currency::GBP => SwapIndexMetadata {
                underlying_index: RateIndex::Sonia,
                fixed_frequency: Tenor::OneYear,
                float_frequency: Tenor::SixMonths,
                fixed_day_counter: DayCounter::Actual365Fixed,
                float_day_counter: DayCounter::Actual365Fixed,
                settlement_lag: 0,
                calendar: CalendarId::London,
            },
            Currency::JPY => SwapIndexMetadata {
                underlying_index: RateIndex::Tonar,
                fixed_frequency: Tenor::SixMonths,
                float_frequency: Tenor::SixMonths,
                fixed_day_counter: DayCounter::Actual365Fixed,
                float_day_counter: DayCounter::Actual365Fixed,
                settlement_lag: 2,
                calendar: CalendarId::Tokyo,
            },
            Currency::CHF => SwapIndexMetadata {
                underlying_index: RateIndex::Saron,
                fixed_frequency: Tenor::OneYear,
                float_frequency: Tenor::SixMonths,
                fixed_day_counter: DayCounter::Thirty360Bond,
                float_day_counter: DayCounter::Actual360,
                settlement_lag: 2,
                calendar: CalendarId::Target, // Swiss calendar not yet defined
            },
        }
    }

    /// Creates a swap index from currency and tenor.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{SwapIndex, Currency};
    /// use infra_domain::time::Tenor;
    ///
    /// let index = SwapIndex::from_currency_tenor(Currency::USD, Tenor::TenYears);
    /// assert_eq!(index, Some(SwapIndex::UsdCms10Y));
    ///
    /// let unsupported = SwapIndex::from_currency_tenor(Currency::USD, Tenor::OneYear);
    /// assert_eq!(unsupported, None);
    /// ```
    #[must_use]
    pub const fn from_currency_tenor(currency: Currency, tenor: Tenor) -> Option<Self> {
        match (currency, tenor) {
            (Currency::USD, Tenor::TwoYears) => Some(Self::UsdCms2Y),
            (Currency::USD, Tenor::FiveYears) => Some(Self::UsdCms5Y),
            (Currency::USD, Tenor::TenYears) => Some(Self::UsdCms10Y),
            (Currency::USD, Tenor::ThirtyYears) => Some(Self::UsdCms30Y),
            (Currency::EUR, Tenor::TwoYears) => Some(Self::EurCms2Y),
            (Currency::EUR, Tenor::FiveYears) => Some(Self::EurCms5Y),
            (Currency::EUR, Tenor::TenYears) => Some(Self::EurCms10Y),
            (Currency::EUR, Tenor::ThirtyYears) => Some(Self::EurCms30Y),
            (Currency::GBP, Tenor::TwoYears) => Some(Self::GbpCms2Y),
            (Currency::GBP, Tenor::FiveYears) => Some(Self::GbpCms5Y),
            (Currency::GBP, Tenor::TenYears) => Some(Self::GbpCms10Y),
            (Currency::GBP, Tenor::ThirtyYears) => Some(Self::GbpCms30Y),
            (Currency::JPY, Tenor::TwoYears) => Some(Self::JpyCms2Y),
            (Currency::JPY, Tenor::FiveYears) => Some(Self::JpyCms5Y),
            (Currency::JPY, Tenor::TenYears) => Some(Self::JpyCms10Y),
            (Currency::JPY, Tenor::ThirtyYears) => Some(Self::JpyCms30Y),
            (Currency::CHF, Tenor::TwoYears) => Some(Self::ChfCms2Y),
            (Currency::CHF, Tenor::FiveYears) => Some(Self::ChfCms5Y),
            (Currency::CHF, Tenor::TenYears) => Some(Self::ChfCms10Y),
            (Currency::CHF, Tenor::ThirtyYears) => Some(Self::ChfCms30Y),
            _ => None,
        }
    }
}

impl std::fmt::Display for SwapIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for SwapIndex {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().replace([' ', '-'], "").as_str() {
            "USDCMS2Y" => Ok(Self::UsdCms2Y),
            "USDCMS5Y" => Ok(Self::UsdCms5Y),
            "USDCMS10Y" => Ok(Self::UsdCms10Y),
            "USDCMS30Y" => Ok(Self::UsdCms30Y),
            "EURCMS2Y" => Ok(Self::EurCms2Y),
            "EURCMS5Y" => Ok(Self::EurCms5Y),
            "EURCMS10Y" => Ok(Self::EurCms10Y),
            "EURCMS30Y" => Ok(Self::EurCms30Y),
            "GBPCMS2Y" => Ok(Self::GbpCms2Y),
            "GBPCMS5Y" => Ok(Self::GbpCms5Y),
            "GBPCMS10Y" => Ok(Self::GbpCms10Y),
            "GBPCMS30Y" => Ok(Self::GbpCms30Y),
            "JPYCMS2Y" => Ok(Self::JpyCms2Y),
            "JPYCMS5Y" => Ok(Self::JpyCms5Y),
            "JPYCMS10Y" => Ok(Self::JpyCms10Y),
            "JPYCMS30Y" => Ok(Self::JpyCms30Y),
            "CHFCMS2Y" => Ok(Self::ChfCms2Y),
            "CHFCMS5Y" => Ok(Self::ChfCms5Y),
            "CHFCMS10Y" => Ok(Self::ChfCms10Y),
            "CHFCMS30Y" => Ok(Self::ChfCms30Y),
            _ => Err(format!("Unknown swap index: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency() {
        assert_eq!(SwapIndex::UsdCms10Y.currency(), Currency::USD);
        assert_eq!(SwapIndex::EurCms10Y.currency(), Currency::EUR);
        assert_eq!(SwapIndex::GbpCms10Y.currency(), Currency::GBP);
        assert_eq!(SwapIndex::JpyCms10Y.currency(), Currency::JPY);
        assert_eq!(SwapIndex::ChfCms10Y.currency(), Currency::CHF);
    }

    #[test]
    fn test_tenor() {
        assert_eq!(SwapIndex::UsdCms2Y.tenor(), Tenor::TwoYears);
        assert_eq!(SwapIndex::UsdCms5Y.tenor(), Tenor::FiveYears);
        assert_eq!(SwapIndex::UsdCms10Y.tenor(), Tenor::TenYears);
        assert_eq!(SwapIndex::UsdCms30Y.tenor(), Tenor::ThirtyYears);
    }

    #[test]
    fn test_underlying_index() {
        assert_eq!(SwapIndex::UsdCms10Y.underlying_index(), RateIndex::Sofr);
        assert_eq!(SwapIndex::EurCms10Y.underlying_index(), RateIndex::Estr);
        assert_eq!(SwapIndex::GbpCms10Y.underlying_index(), RateIndex::Sonia);
        assert_eq!(SwapIndex::JpyCms10Y.underlying_index(), RateIndex::Tonar);
        assert_eq!(SwapIndex::ChfCms10Y.underlying_index(), RateIndex::Saron);
    }

    #[test]
    fn test_name() {
        assert_eq!(SwapIndex::UsdCms10Y.name(), "USD CMS 10Y");
        assert_eq!(SwapIndex::EurCms5Y.name(), "EUR CMS 5Y");
        assert_eq!(SwapIndex::GbpCms30Y.name(), "GBP CMS 30Y");
    }

    #[test]
    fn test_api_code() {
        assert_eq!(SwapIndex::UsdCms10Y.api_code(), "USD-CMS-10Y");
        assert_eq!(SwapIndex::EurCms5Y.api_code(), "EUR-CMS-5Y");
    }

    #[test]
    fn test_metadata_usd() {
        let metadata = SwapIndex::UsdCms10Y.metadata();
        assert_eq!(metadata.underlying_index, RateIndex::Sofr);
        assert_eq!(metadata.fixed_frequency, Tenor::SixMonths);
        assert_eq!(metadata.float_frequency, Tenor::ThreeMonths);
        assert_eq!(metadata.fixed_day_counter, DayCounter::Thirty360Bond);
        assert_eq!(metadata.float_day_counter, DayCounter::Actual360);
        assert_eq!(metadata.settlement_lag, 2);
        assert_eq!(metadata.calendar, CalendarId::NewYork);
    }

    #[test]
    fn test_metadata_eur() {
        let metadata = SwapIndex::EurCms10Y.metadata();
        assert_eq!(metadata.underlying_index, RateIndex::Estr);
        assert_eq!(metadata.fixed_frequency, Tenor::OneYear);
        assert_eq!(metadata.float_frequency, Tenor::SixMonths);
        assert_eq!(metadata.calendar, CalendarId::Target);
    }

    #[test]
    fn test_metadata_gbp() {
        let metadata = SwapIndex::GbpCms10Y.metadata();
        assert_eq!(metadata.underlying_index, RateIndex::Sonia);
        assert_eq!(metadata.settlement_lag, 0); // GBP has T+0
        assert_eq!(metadata.fixed_day_counter, DayCounter::Actual365Fixed);
    }

    #[test]
    fn test_from_currency_tenor() {
        assert_eq!(
            SwapIndex::from_currency_tenor(Currency::USD, Tenor::TenYears),
            Some(SwapIndex::UsdCms10Y)
        );
        assert_eq!(
            SwapIndex::from_currency_tenor(Currency::EUR, Tenor::FiveYears),
            Some(SwapIndex::EurCms5Y)
        );
        assert_eq!(
            SwapIndex::from_currency_tenor(Currency::USD, Tenor::OneYear),
            None
        );
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", SwapIndex::UsdCms10Y), "USD CMS 10Y");
        assert_eq!(format!("{}", SwapIndex::EurCms5Y), "EUR CMS 5Y");
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            "USD-CMS-10Y".parse::<SwapIndex>().unwrap(),
            SwapIndex::UsdCms10Y
        );
        assert_eq!(
            "usdcms10y".parse::<SwapIndex>().unwrap(),
            SwapIndex::UsdCms10Y
        );
        assert_eq!(
            "EUR CMS 5Y".parse::<SwapIndex>().unwrap(),
            SwapIndex::EurCms5Y
        );
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("UNKNOWN".parse::<SwapIndex>().is_err());
        assert!("USD-CMS-15Y".parse::<SwapIndex>().is_err());
    }

    #[test]
    fn test_all() {
        let all = SwapIndex::all();
        assert_eq!(all.len(), 20);
        assert!(all.contains(&SwapIndex::UsdCms10Y));
        assert!(all.contains(&SwapIndex::EurCms10Y));
        assert!(all.contains(&SwapIndex::JpyCms30Y));
    }
}
