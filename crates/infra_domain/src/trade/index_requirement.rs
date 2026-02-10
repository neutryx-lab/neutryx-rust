//! Index requirement types for Market validation.
//!
//! This module provides types to represent what market indices a Trade
//! or Cashflow requires for pricing.
//!
//! # Examples
//!
//! ```
//! use infra_domain::trade::IndexRequirement;
//! use infra_domain::market::{RateIndex, CurrencyPair};
//! use infra_domain::market::Currency;
//!
//! // Rate curve requirement
//! let rate_req = IndexRequirement::RateCurve(RateIndex::Sofr);
//! assert!(rate_req.is_rate_curve());
//!
//! // FX curve requirement
//! let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
//! let fx_req = IndexRequirement::FxCurve(pair);
//! assert!(fx_req.is_fx_curve());
//! ```

use crate::market::{CurrencyPair, RateIndex};

/// Requirement for a market index needed by a Trade or Cashflow.
///
/// This enum represents the different types of market data that a pricing
/// operation may require. It is used by `TradeIndexRequirements` trait to
/// enumerate what indices are needed, and by `MarketValidator` to verify
/// that all required indices are available in the market.
///
/// # Variants
///
/// - `RateCurve`: Requires a yield curve for discount/projection (e.g., SOFR,
///   EURIBOR)
/// - `SwaptionVol`: Requires a swaption volatility cube for the given rate
///   index
/// - `FxCurve`: Requires an FX forward curve for the given currency pair
/// - `FxVol`: Requires an FX volatility surface for the given currency pair
///
/// # Examples
///
/// ```
/// use infra_domain::trade::IndexRequirement;
/// use infra_domain::market::RateIndex;
/// use infra_domain::market::CurrencyPair;
/// use infra_domain::market::Currency;
///
/// // A floating leg requires a rate curve
/// let sofr_curve = IndexRequirement::RateCurve(RateIndex::Sofr);
///
/// // A swaption requires both rate curve and vol cube
/// let sofr_vol = IndexRequirement::SwaptionVol(RateIndex::Sofr);
///
/// // An FX forward requires an FX curve
/// let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
/// let eurusd_curve = IndexRequirement::FxCurve(pair);
///
/// // An FX option requires both FX curve and vol surface
/// let eurusd_vol = IndexRequirement::FxVol(pair);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IndexRequirement {
    /// Rate index for discount/projection curves.
    ///
    /// Used by floating rate legs (OIS, IBOR) and fixed legs for discounting.
    RateCurve(RateIndex),

    /// Rate index for swaption volatility cube.
    ///
    /// Used by swaption, cap/floor, and other interest rate options.
    SwaptionVol(RateIndex),

    /// Currency pair for FX forward curve.
    ///
    /// Used by FX forwards, FX swaps, and cross-currency swaps.
    FxCurve(CurrencyPair),

    /// Currency pair for FX volatility surface.
    ///
    /// Used by FX vanilla options, FX barrier options, and FX exotics.
    FxVol(CurrencyPair),
}

impl IndexRequirement {
    /// Returns `true` if this is a `RateCurve` requirement.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::trade::IndexRequirement;
    /// use infra_domain::market::RateIndex;
    ///
    /// let req = IndexRequirement::RateCurve(RateIndex::Sofr);
    /// assert!(req.is_rate_curve());
    /// ```
    #[must_use]
    pub fn is_rate_curve(&self) -> bool { matches!(self, Self::RateCurve(_)) }

    /// Returns `true` if this is a `SwaptionVol` requirement.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::trade::IndexRequirement;
    /// use infra_domain::market::RateIndex;
    ///
    /// let req = IndexRequirement::SwaptionVol(RateIndex::Sofr);
    /// assert!(req.is_swaption_vol());
    /// ```
    #[must_use]
    pub fn is_swaption_vol(&self) -> bool { matches!(self, Self::SwaptionVol(_)) }

    /// Returns `true` if this is an `FxCurve` requirement.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::trade::IndexRequirement;
    /// use infra_domain::market::CurrencyPair;
    /// use infra_domain::market::Currency;
    ///
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
    /// let req = IndexRequirement::FxCurve(pair);
    /// assert!(req.is_fx_curve());
    /// ```
    #[must_use]
    pub fn is_fx_curve(&self) -> bool { matches!(self, Self::FxCurve(_)) }

    /// Returns `true` if this is an `FxVol` requirement.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::trade::IndexRequirement;
    /// use infra_domain::market::CurrencyPair;
    /// use infra_domain::market::Currency;
    ///
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
    /// let req = IndexRequirement::FxVol(pair);
    /// assert!(req.is_fx_vol());
    /// ```
    #[must_use]
    pub fn is_fx_vol(&self) -> bool { matches!(self, Self::FxVol(_)) }

    /// Returns the `RateIndex` if this is a rate-based requirement.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::trade::IndexRequirement;
    /// use infra_domain::market::RateIndex;
    ///
    /// let req = IndexRequirement::RateCurve(RateIndex::Sofr);
    /// assert_eq!(req.as_rate_index(), Some(&RateIndex::Sofr));
    ///
    /// let req = IndexRequirement::SwaptionVol(RateIndex::Euribor3M);
    /// assert_eq!(req.as_rate_index(), Some(&RateIndex::Euribor3M));
    /// ```
    #[must_use]
    pub fn as_rate_index(&self) -> Option<&RateIndex> {
        match self {
            Self::RateCurve(idx) | Self::SwaptionVol(idx) => Some(idx),
            _ => None,
        }
    }

    /// Returns the `CurrencyPair` if this is an FX-based requirement.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::trade::IndexRequirement;
    /// use infra_domain::market::CurrencyPair;
    /// use infra_domain::market::Currency;
    ///
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
    /// let req = IndexRequirement::FxCurve(pair);
    /// assert_eq!(req.as_currency_pair(), Some(&pair));
    /// ```
    #[must_use]
    pub fn as_currency_pair(&self) -> Option<&CurrencyPair> {
        match self {
            Self::FxCurve(pair) | Self::FxVol(pair) => Some(pair),
            _ => None,
        }
    }
}

impl std::fmt::Display for IndexRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateCurve(idx) => write!(f, "RateCurve({})", idx),
            Self::SwaptionVol(idx) => write!(f, "SwaptionVol({})", idx),
            Self::FxCurve(pair) => write!(f, "FxCurve({})", pair),
            Self::FxVol(pair) => write!(f, "FxVol({})", pair),
        }
    }
}

impl PartialOrd for IndexRequirement {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

impl Ord for IndexRequirement {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by variant order, then by inner value
        match (self, other) {
            (Self::RateCurve(a), Self::RateCurve(b)) => a.name().cmp(b.name()),
            (Self::SwaptionVol(a), Self::SwaptionVol(b)) => a.name().cmp(b.name()),
            (Self::FxCurve(a), Self::FxCurve(b)) => a.to_string().cmp(&b.to_string()),
            (Self::FxVol(a), Self::FxVol(b)) => a.to_string().cmp(&b.to_string()),
            // Variant ordering: RateCurve < SwaptionVol < FxCurve < FxVol
            (Self::RateCurve(_), _) => std::cmp::Ordering::Less,
            (_, Self::RateCurve(_)) => std::cmp::Ordering::Greater,
            (Self::SwaptionVol(_), _) => std::cmp::Ordering::Less,
            (_, Self::SwaptionVol(_)) => std::cmp::Ordering::Greater,
            (Self::FxCurve(_), Self::FxVol(_)) => std::cmp::Ordering::Less,
            (Self::FxVol(_), Self::FxCurve(_)) => std::cmp::Ordering::Greater,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::Currency;

    #[test]
    fn test_variant_type_checks() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);

        assert!(IndexRequirement::RateCurve(RateIndex::Sofr).is_rate_curve());
        assert!(IndexRequirement::SwaptionVol(RateIndex::Euribor3M).is_swaption_vol());
        assert!(IndexRequirement::FxCurve(pair).is_fx_curve());
        assert!(IndexRequirement::FxVol(pair).is_fx_vol());
    }

    #[test]
    fn test_accessors() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);

        assert_eq!(
            IndexRequirement::RateCurve(RateIndex::Sofr).as_rate_index(),
            Some(&RateIndex::Sofr)
        );
        assert_eq!(
            IndexRequirement::SwaptionVol(RateIndex::Sonia).as_rate_index(),
            Some(&RateIndex::Sonia)
        );
        assert_eq!(IndexRequirement::FxCurve(pair).as_rate_index(), None);

        assert_eq!(
            IndexRequirement::FxCurve(pair).as_currency_pair(),
            Some(&pair)
        );
        assert_eq!(
            IndexRequirement::FxVol(pair).as_currency_pair(),
            Some(&pair)
        );
        assert_eq!(
            IndexRequirement::RateCurve(RateIndex::Sofr).as_currency_pair(),
            None
        );
    }

    #[test]
    fn test_ordering_and_dedup() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let mut reqs = vec![
            IndexRequirement::FxVol(pair),
            IndexRequirement::RateCurve(RateIndex::Sofr),
            IndexRequirement::RateCurve(RateIndex::Sofr), // Duplicate
            IndexRequirement::FxCurve(pair),
            IndexRequirement::SwaptionVol(RateIndex::Sofr),
        ];

        reqs.sort();
        reqs.dedup();

        assert_eq!(reqs.len(), 4);
        assert!(reqs[0].is_rate_curve());
        assert!(reqs[1].is_swaption_vol());
        assert!(reqs[2].is_fx_curve());
        assert!(reqs[3].is_fx_vol());
    }

    #[test]
    fn test_display() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        assert!(format!("{}", IndexRequirement::RateCurve(RateIndex::Sofr)).contains("RateCurve"));
        assert!(format!("{}", IndexRequirement::FxCurve(pair)).contains("EUR/USD"));
    }
}
