//! Index requirement types for Market validation.
//!
//! This module provides types to represent what market indices a Trade
//! or Cashflow requires for pricing.
//!
//! # Examples
//!
//! ```
//! use infra_master::trade::IndexRequirement;
//! use infra_master::market::{RateIndex, CurrencyPair};
//! use infra_master::Currency;
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
/// use infra_master::trade::IndexRequirement;
/// use infra_master::market::RateIndex;
/// use infra_master::market::CurrencyPair;
/// use infra_master::Currency;
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
    /// use infra_master::trade::IndexRequirement;
    /// use infra_master::market::RateIndex;
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
    /// use infra_master::trade::IndexRequirement;
    /// use infra_master::market::RateIndex;
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
    /// use infra_master::trade::IndexRequirement;
    /// use infra_master::market::CurrencyPair;
    /// use infra_master::Currency;
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
    /// use infra_master::trade::IndexRequirement;
    /// use infra_master::market::CurrencyPair;
    /// use infra_master::Currency;
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
    /// use infra_master::trade::IndexRequirement;
    /// use infra_master::market::RateIndex;
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
    /// use infra_master::trade::IndexRequirement;
    /// use infra_master::market::CurrencyPair;
    /// use infra_master::Currency;
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
    use crate::Currency;

    // ========================================
    // Variant Creation Tests
    // ========================================

    #[test]
    fn test_rate_curve_creation() {
        let req = IndexRequirement::RateCurve(RateIndex::Sofr);
        assert!(req.is_rate_curve());
        assert!(!req.is_swaption_vol());
        assert!(!req.is_fx_curve());
        assert!(!req.is_fx_vol());
    }

    #[test]
    fn test_swaption_vol_creation() {
        let req = IndexRequirement::SwaptionVol(RateIndex::Euribor3M);
        assert!(!req.is_rate_curve());
        assert!(req.is_swaption_vol());
        assert!(!req.is_fx_curve());
        assert!(!req.is_fx_vol());
    }

    #[test]
    fn test_fx_curve_creation() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let req = IndexRequirement::FxCurve(pair);
        assert!(!req.is_rate_curve());
        assert!(!req.is_swaption_vol());
        assert!(req.is_fx_curve());
        assert!(!req.is_fx_vol());
    }

    #[test]
    fn test_fx_vol_creation() {
        let pair = CurrencyPair::new(Currency::USD, Currency::JPY);
        let req = IndexRequirement::FxVol(pair);
        assert!(!req.is_rate_curve());
        assert!(!req.is_swaption_vol());
        assert!(!req.is_fx_curve());
        assert!(req.is_fx_vol());
    }

    // ========================================
    // Accessor Tests
    // ========================================

    #[test]
    fn test_as_rate_index() {
        let req = IndexRequirement::RateCurve(RateIndex::Sofr);
        assert_eq!(req.as_rate_index(), Some(&RateIndex::Sofr));

        let req = IndexRequirement::SwaptionVol(RateIndex::Sonia);
        assert_eq!(req.as_rate_index(), Some(&RateIndex::Sonia));

        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let req = IndexRequirement::FxCurve(pair);
        assert_eq!(req.as_rate_index(), None);
    }

    #[test]
    fn test_as_currency_pair() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let req = IndexRequirement::FxCurve(pair);
        assert_eq!(req.as_currency_pair(), Some(&pair));

        let req = IndexRequirement::FxVol(pair);
        assert_eq!(req.as_currency_pair(), Some(&pair));

        let req = IndexRequirement::RateCurve(RateIndex::Sofr);
        assert_eq!(req.as_currency_pair(), None);
    }

    // ========================================
    // Hash and Eq Tests (HashMap key usability)
    // ========================================

    #[test]
    fn test_hash_map_key() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(IndexRequirement::RateCurve(RateIndex::Sofr));
        set.insert(IndexRequirement::RateCurve(RateIndex::Tonar));
        set.insert(IndexRequirement::RateCurve(RateIndex::Sofr)); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&IndexRequirement::RateCurve(RateIndex::Sofr)));
    }

    #[test]
    fn test_equality() {
        let req1 = IndexRequirement::RateCurve(RateIndex::Sofr);
        let req2 = IndexRequirement::RateCurve(RateIndex::Sofr);
        let req3 = IndexRequirement::RateCurve(RateIndex::Sonia);

        assert_eq!(req1, req2);
        assert_ne!(req1, req3);
    }

    // ========================================
    // Clone and Debug Tests
    // ========================================

    #[test]
    fn test_clone() {
        let req1 = IndexRequirement::SwaptionVol(RateIndex::Estr);
        let req2 = req1.clone();
        assert_eq!(req1, req2);
    }

    #[test]
    fn test_debug() {
        let req = IndexRequirement::RateCurve(RateIndex::Sofr);
        let debug = format!("{:?}", req);
        assert!(debug.contains("RateCurve"));
        assert!(debug.contains("Sofr"));
    }

    // ========================================
    // Display Tests
    // ========================================

    #[test]
    fn test_display_rate_curve() {
        let req = IndexRequirement::RateCurve(RateIndex::Sofr);
        let display = format!("{}", req);
        assert!(display.contains("RateCurve"));
        assert!(display.contains("SOFR"));
    }

    #[test]
    fn test_display_swaption_vol() {
        let req = IndexRequirement::SwaptionVol(RateIndex::Euribor3M);
        let display = format!("{}", req);
        assert!(display.contains("SwaptionVol"));
    }

    #[test]
    fn test_display_fx_curve() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let req = IndexRequirement::FxCurve(pair);
        let display = format!("{}", req);
        assert!(display.contains("FxCurve"));
        assert!(display.contains("EUR/USD"));
    }

    #[test]
    fn test_display_fx_vol() {
        let pair = CurrencyPair::new(Currency::USD, Currency::JPY);
        let req = IndexRequirement::FxVol(pair);
        let display = format!("{}", req);
        assert!(display.contains("FxVol"));
        assert!(display.contains("USD/JPY"));
    }

    // ========================================
    // Ord Tests (for sorting and dedup)
    // ========================================

    #[test]
    fn test_ordering() {
        let rate_curve = IndexRequirement::RateCurve(RateIndex::Sofr);
        let swaption_vol = IndexRequirement::SwaptionVol(RateIndex::Sofr);
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fx_curve = IndexRequirement::FxCurve(pair);
        let fx_vol = IndexRequirement::FxVol(pair);

        // RateCurve < SwaptionVol < FxCurve < FxVol
        assert!(rate_curve < swaption_vol);
        assert!(swaption_vol < fx_curve);
        assert!(fx_curve < fx_vol);
    }

    #[test]
    fn test_sort() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let mut reqs = vec![
            IndexRequirement::FxVol(pair),
            IndexRequirement::RateCurve(RateIndex::Sofr),
            IndexRequirement::FxCurve(pair),
            IndexRequirement::SwaptionVol(RateIndex::Sofr),
        ];

        reqs.sort();

        assert!(reqs[0].is_rate_curve());
        assert!(reqs[1].is_swaption_vol());
        assert!(reqs[2].is_fx_curve());
        assert!(reqs[3].is_fx_vol());
    }

    #[test]
    fn test_dedup() {
        let mut reqs = vec![
            IndexRequirement::RateCurve(RateIndex::Sofr),
            IndexRequirement::RateCurve(RateIndex::Sonia),
            IndexRequirement::RateCurve(RateIndex::Sofr), // Duplicate
        ];

        reqs.sort();
        reqs.dedup();

        assert_eq!(reqs.len(), 2);
    }
}
