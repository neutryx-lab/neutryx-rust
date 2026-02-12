//! Index requirement types for Market validation.

use crate::market::{CurrencyPair, RateIndex};

/// Requirement for a market index needed by a Trade or Cashflow.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum IndexRequirement {
    /// Rate index for discount/projection curves.
    RateCurve(RateIndex),

    /// Rate index for swaption volatility cube.
    SwaptionVol(RateIndex),

    /// Currency pair for FX forward curve.
    FxCurve(CurrencyPair),

    /// Currency pair for FX volatility surface.
    FxVol(CurrencyPair),
}

impl IndexRequirement {
    /// Returns `true` if this is a `RateCurve` requirement.
    #[must_use]
    pub fn is_rate_curve(&self) -> bool { matches!(self, Self::RateCurve(_)) }

    /// Returns `true` if this is a `SwaptionVol` requirement.
    #[must_use]
    pub fn is_swaption_vol(&self) -> bool { matches!(self, Self::SwaptionVol(_)) }

    /// Returns `true` if this is an `FxCurve` requirement.
    #[must_use]
    pub fn is_fx_curve(&self) -> bool { matches!(self, Self::FxCurve(_)) }

    /// Returns `true` if this is an `FxVol` requirement.
    #[must_use]
    pub fn is_fx_vol(&self) -> bool { matches!(self, Self::FxVol(_)) }

    /// Returns the `RateIndex` if this is a rate-based requirement.
    #[must_use]
    pub fn as_rate_index(&self) -> Option<&RateIndex> {
        match self {
            Self::RateCurve(idx) | Self::SwaptionVol(idx) => Some(idx),
            _ => None,
        }
    }

    /// Returns the `CurrencyPair` if this is an FX-based requirement.
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
        match (self, other) {
            (Self::RateCurve(a), Self::RateCurve(b)) => a.name().cmp(b.name()),
            (Self::SwaptionVol(a), Self::SwaptionVol(b)) => a.name().cmp(b.name()),
            (Self::FxCurve(a), Self::FxCurve(b)) => a.to_string().cmp(&b.to_string()),
            (Self::FxVol(a), Self::FxVol(b)) => a.to_string().cmp(&b.to_string()),
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
            IndexRequirement::RateCurve(RateIndex::Sofr),
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
