//! Market index types for financial instruments.
//!
//! This module provides types for representing various market indices
//! used in floating-rate instruments.

use crate::{Date, RateIndex};

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
///
/// # Examples
///
/// ```
/// use infra_master::trade::{IndexObservation, IndexType};
/// use infra_master::{RateIndex, Date};
///
/// // Simple observation with lag
/// let obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr))
///     .with_lag(2);
///
/// // Observation with period for compound indices (OIS)
/// let start = Date::from_ymd(2025, 1, 1).unwrap();
/// let end = Date::from_ymd(2025, 3, 31).unwrap();
/// let compound_obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr))
///     .with_observation_period(start, end);
///
/// assert!(compound_obs.is_compound_observation());
/// ```
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

    /// Observation date (optional, populated during evaluation).
    ///
    /// For term indices, this is the single fixing date.
    /// For compound indices, this may be the period end date or left as None.
    pub observation_date: Option<Date>,

    /// Observation period for compound indices (start, end).
    ///
    /// For OIS indices, this defines the period over which overnight
    /// rates are compounded. For IBOR indices, this is typically None.
    pub observation_period: Option<(Date, Date)>,
}

impl IndexObservation {
    /// Creates a new index observation with default settings.
    #[must_use]
    pub fn new(index_type: IndexType) -> Self {
        Self {
            index_type,
            observation_lag: 0,
            fixing_source: None,
            observation_date: None,
            observation_period: None,
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

    /// Sets the observation date.
    ///
    /// This is typically populated during cashflow evaluation.
    #[must_use]
    pub fn with_observation_date(mut self, date: Date) -> Self {
        self.observation_date = Some(date);
        self
    }

    /// Sets the observation period for compound indices.
    ///
    /// Used for OIS legs where overnight rates are compounded
    /// over a period (start_date to end_date).
    #[must_use]
    pub fn with_observation_period(mut self, start: Date, end: Date) -> Self {
        self.observation_period = Some((start, end));
        self
    }

    /// Returns true if this observation has a compound period.
    ///
    /// Compound observations are used for OIS indices where the
    /// rate is compounded over a period rather than fixed at a single point.
    #[must_use]
    pub fn is_compound_observation(&self) -> bool { self.observation_period.is_some() }

    /// Returns the observation period if set.
    #[must_use]
    pub fn period(&self) -> Option<(Date, Date)> { self.observation_period }

    /// Returns true if this observation requires daily compounding.
    ///
    /// This checks if the underlying index is an overnight index
    /// and has an observation period defined.
    #[must_use]
    pub fn requires_daily_compounding(&self) -> bool {
        if let Some(rate_index) = self.index_type.as_rate() {
            rate_index.is_overnight() && self.observation_period.is_some()
        } else {
            false
        }
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
        assert!(obs.observation_date.is_none());
        assert!(obs.observation_period.is_none());
        assert!(!obs.is_compound_observation());
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

    // ========================================
    // IndexObservation Extension Tests
    // ========================================

    #[test]
    fn test_index_observation_with_observation_date() {
        let date = Date::from_ymd(2025, 3, 15).unwrap();
        let obs =
            IndexObservation::new(IndexType::Rate(RateIndex::Sofr)).with_observation_date(date);

        assert_eq!(obs.observation_date, Some(date));
    }

    #[test]
    fn test_index_observation_with_period() {
        let start = Date::from_ymd(2025, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 3, 31).unwrap();
        let obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr))
            .with_observation_period(start, end);

        assert!(obs.is_compound_observation());
        assert_eq!(obs.period(), Some((start, end)));
    }

    #[test]
    fn test_requires_daily_compounding_ois() {
        let start = Date::from_ymd(2025, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 3, 31).unwrap();

        // OIS index with period -> requires daily compounding
        let ois_obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr))
            .with_observation_period(start, end);
        assert!(ois_obs.requires_daily_compounding());

        // OIS index without period -> no daily compounding needed
        let ois_no_period = IndexObservation::new(IndexType::Rate(RateIndex::Sofr));
        assert!(!ois_no_period.requires_daily_compounding());
    }

    #[test]
    fn test_requires_daily_compounding_ibor() {
        let start = Date::from_ymd(2025, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 3, 31).unwrap();

        // IBOR index never requires daily compounding (even with period)
        let ibor_obs = IndexObservation::new(IndexType::Rate(RateIndex::Euribor3M))
            .with_observation_period(start, end);
        // Note: Euribor3M is a term index, so it doesn't require daily compounding
        // even if a period is set (though typically it wouldn't have a period)
        assert!(!ibor_obs.requires_daily_compounding());
    }

    #[test]
    fn test_requires_daily_compounding_non_rate_index() {
        let start = Date::from_ymd(2025, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 3, 31).unwrap();

        // FX index -> never requires daily compounding
        let fx_obs = IndexObservation::new(IndexType::Fx {
            base: "EUR".into(),
            quote: "USD".into(),
        })
        .with_observation_period(start, end);
        assert!(!fx_obs.requires_daily_compounding());
    }

    #[test]
    fn test_index_observation_full_chain() {
        let date = Date::from_ymd(2025, 3, 31).unwrap();
        let start = Date::from_ymd(2025, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 3, 31).unwrap();

        let obs = IndexObservation::new(IndexType::Rate(RateIndex::Sonia))
            .with_lag(0)
            .with_source("ISDA")
            .with_observation_date(date)
            .with_observation_period(start, end);

        assert_eq!(obs.observation_lag, 0);
        assert_eq!(obs.fixing_source, Some("ISDA".to_string()));
        assert_eq!(obs.observation_date, Some(date));
        assert_eq!(obs.observation_period, Some((start, end)));
        assert!(obs.is_compound_observation());
        assert!(obs.requires_daily_compounding());
    }
}
