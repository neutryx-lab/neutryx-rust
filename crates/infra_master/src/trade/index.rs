//! Market index types for financial instruments.
//!
//! This module provides types for representing various market indices
//! used in floating-rate instruments.

use crate::RateIndex;

/// Type of market index.
///
/// Represents various market indices used for determining floating
/// rates in financial instruments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IndexType {
    /// Interest rate index (wraps existing RateIndex).
    Rate(RateIndex),

    /// Swap rate (e.g., 10Y swap rate).
    SwapRate {
        /// Currency of the swap.
        currency: String,
        /// Tenor of the swap (e.g., "10Y").
        tenor: String,
    },

    /// Foreign exchange rate.
    Fx {
        /// Base currency (e.g., "EUR" in EUR/USD).
        base: String,
        /// Quote currency (e.g., "USD" in EUR/USD).
        quote: String,
    },

    /// Equity index or single stock.
    Equity {
        /// Ticker symbol or index code.
        ticker: String,
    },

    /// Inflation index.
    Inflation {
        /// Index name (e.g., "CPI", "RPI").
        name: String,
        /// Region or country code.
        region: String,
    },

    /// Commodity index.
    Commodity {
        /// Commodity name (e.g., "WTI", "BRENT").
        name: String,
    },
}

impl From<RateIndex> for IndexType {
    fn from(rate_index: RateIndex) -> Self { IndexType::Rate(rate_index) }
}

impl IndexType {
    /// Returns true if this is an interest rate index.
    #[must_use]
    pub fn is_rate(&self) -> bool { matches!(self, IndexType::Rate(_)) }

    /// Returns true if this is an FX index.
    #[must_use]
    pub fn is_fx(&self) -> bool { matches!(self, IndexType::Fx { .. }) }

    /// Returns the rate index if this is a Rate variant.
    #[must_use]
    pub fn as_rate(&self) -> Option<&RateIndex> {
        match self {
            IndexType::Rate(r) => Some(r),
            _ => None,
        }
    }
}

/// Observation parameters for an index.
///
/// Defines how an index is observed for cashflow calculations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IndexObservation {
    /// The type of index being observed.
    pub index_type: IndexType,

    /// Number of days before the period start for fixing observation.
    /// Positive means fixing occurs before the period.
    pub observation_lag: i32,

    /// Source for the fixing (e.g., "ISDA", "Reuters", "Bloomberg").
    pub fixing_source: Option<String>,
}

impl IndexObservation {
    /// Creates a new index observation with default settings.
    #[must_use]
    pub fn new(index_type: IndexType) -> Self {
        Self {
            index_type,
            observation_lag: 0,
            fixing_source: None,
        }
    }

    /// Sets the observation lag in days.
    #[must_use]
    pub fn with_lag(mut self, lag: i32) -> Self {
        self.observation_lag = lag;
        self
    }

    /// Sets the fixing source.
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.fixing_source = Some(source.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_rate_index() {
        let rate_index = RateIndex::Sofr;
        let index_type: IndexType = rate_index.into();

        assert!(matches!(index_type, IndexType::Rate(RateIndex::Sofr)));
        assert!(index_type.is_rate());
        assert!(!index_type.is_fx());
    }

    #[test]
    fn test_as_rate() {
        let index_type = IndexType::Rate(RateIndex::Euribor3M);
        assert_eq!(index_type.as_rate(), Some(&RateIndex::Euribor3M));

        let fx_index = IndexType::Fx {
            base: "EUR".into(),
            quote: "USD".into(),
        };
        assert_eq!(fx_index.as_rate(), None);
    }

    #[test]
    fn test_swap_rate_index() {
        let index = IndexType::SwapRate {
            currency: "USD".into(),
            tenor: "10Y".into(),
        };

        assert!(!index.is_rate());
        assert!(!index.is_fx());
    }

    #[test]
    fn test_fx_index() {
        let index = IndexType::Fx {
            base: "EUR".into(),
            quote: "USD".into(),
        };

        assert!(index.is_fx());
        assert!(!index.is_rate());
    }

    #[test]
    fn test_equity_index() {
        let index = IndexType::Equity {
            ticker: "SPX".into(),
        };

        assert!(!index.is_rate());
        assert!(!index.is_fx());
    }

    #[test]
    fn test_inflation_index() {
        let index = IndexType::Inflation {
            name: "CPI".into(),
            region: "US".into(),
        };

        assert!(!index.is_rate());
    }

    #[test]
    fn test_commodity_index() {
        let index = IndexType::Commodity { name: "WTI".into() };

        assert!(!index.is_rate());
    }

    #[test]
    fn test_index_observation_new() {
        let obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr));

        assert!(matches!(obs.index_type, IndexType::Rate(RateIndex::Sofr)));
        assert_eq!(obs.observation_lag, 0);
        assert!(obs.fixing_source.is_none());
    }

    #[test]
    fn test_index_observation_with_lag() {
        let obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr)).with_lag(2);

        assert_eq!(obs.observation_lag, 2);
    }

    #[test]
    fn test_index_observation_with_source() {
        let obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr)).with_source("ISDA");

        assert_eq!(obs.fixing_source, Some("ISDA".to_string()));
    }

    #[test]
    fn test_index_observation_chained() {
        let obs = IndexObservation::new(IndexType::Rate(RateIndex::Euribor6M))
            .with_lag(-2)
            .with_source("Reuters");

        assert_eq!(obs.observation_lag, -2);
        assert_eq!(obs.fixing_source, Some("Reuters".to_string()));
    }

    #[test]
    fn test_index_type_clone() {
        let index = IndexType::Rate(RateIndex::Sonia);
        let cloned = index.clone();
        assert_eq!(index, cloned);
    }

    #[test]
    fn test_index_type_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(IndexType::Rate(RateIndex::Sofr));
        set.insert(IndexType::Rate(RateIndex::Tonar));
        set.insert(IndexType::Rate(RateIndex::Sofr)); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_index_type_debug() {
        let index = IndexType::Rate(RateIndex::Sofr);
        let debug = format!("{:?}", index);
        assert!(debug.contains("Sofr"));
    }
}
