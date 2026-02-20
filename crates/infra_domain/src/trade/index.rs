//! Market index types for financial instruments.

use crate::{
    market::{CompoundingMethod, RateIndex},
    time::{Date, Frequency},
};

/// Type of market index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

    /// Interest rate future price index.
    IrFuture {
        /// Exchange code (e.g., "CME", "ICE").
        exchange: String,
        /// Contract code (e.g., "ED", "SR3").
        contract: String,
    },

    /// Bond future price index.
    BondFuture {
        /// Exchange code (e.g., "CME", "EUREX").
        exchange: String,
        /// Contract code (e.g., "TY", "RX").
        contract: String,
    },

    /// Credit spread index (single-name CDS spread).
    Credit {
        /// Reference entity name.
        reference_entity: String,
        /// Seniority (e.g., "SNRFOR", "SUBLT2").
        seniority: Option<String>,
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

    /// Returns true if this is an equity index.
    #[must_use]
    pub fn is_equity(&self) -> bool { matches!(self, IndexType::Equity { .. }) }

    /// Returns true if this is an inflation index.
    #[must_use]
    pub fn is_inflation(&self) -> bool { matches!(self, IndexType::Inflation { .. }) }

    /// Returns true if this is a commodity index.
    #[must_use]
    pub fn is_commodity(&self) -> bool { matches!(self, IndexType::Commodity { .. }) }

    /// Returns true if this is an IR future index.
    #[must_use]
    pub fn is_ir_future(&self) -> bool { matches!(self, IndexType::IrFuture { .. }) }

    /// Returns true if this is a bond future index.
    #[must_use]
    pub fn is_bond_future(&self) -> bool { matches!(self, IndexType::BondFuture { .. }) }

    /// Returns true if this is a credit index.
    #[must_use]
    pub fn is_credit(&self) -> bool { matches!(self, IndexType::Credit { .. }) }

    /// Returns true if this is a swap rate index.
    #[must_use]
    pub fn is_swap_rate(&self) -> bool { matches!(self, IndexType::SwapRate { .. }) }
}

/// Observation parameters for an index.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IndexObservation {
    /// The type of index being observed.
    pub index_type: IndexType,

    /// Number of days before the period start for fixing observation.
    pub observation_lag: i32,

    /// Source for the fixing (e.g., "ISDA", "Reuters", "Bloomberg").
    pub fixing_source: Option<String>,

    /// Observation date (optional, populated during evaluation).
    pub observation_date: Option<Date>,

    /// Observation period for compound indices (start, end).
    pub observation_period: Option<(Date, Date)>,

    /// Reset frequency for the index observation.
    pub reset_frequency: Frequency,

    /// Compounding method for rate calculation.
    pub compounding_method: CompoundingMethod,

    /// Lookback period in business days.
    pub lookback_period: Option<i32>,

    /// Lockout period in business days.
    pub lockout_period: Option<i32>,
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
            reset_frequency: Frequency::default(),
            compounding_method: CompoundingMethod::default(),
            lookback_period: None,
            lockout_period: None,
        }
    }

    /// Creates an index observation from a `RateIndex` with appropriate.
    #[must_use]
    pub fn from_rate_index(rate_index: RateIndex) -> Self {
        let metadata = rate_index.metadata();

        let reset_frequency = if rate_index.is_overnight() {
            Frequency::Daily
        } else {
            match rate_index.tenor() {
                crate::time::Tenor::OneMonth => Frequency::Monthly,
                crate::time::Tenor::ThreeMonths => Frequency::Quarterly,
                crate::time::Tenor::SixMonths => Frequency::SemiAnnual,
                crate::time::Tenor::OneYear => Frequency::Annual,
                _ => Frequency::Quarterly,
            }
        };

        Self {
            index_type: IndexType::Rate(rate_index),
            observation_lag: i32::from(metadata.fixing_lag),
            fixing_source: None,
            observation_date: None,
            observation_period: None,
            reset_frequency,
            compounding_method: metadata.compounding_method,
            lookback_period: None,
            lockout_period: None,
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
    #[must_use]
    pub fn with_observation_date(mut self, date: Date) -> Self {
        self.observation_date = Some(date);
        self
    }

    /// Sets the observation period for compound indices.
    #[must_use]
    pub fn with_observation_period(mut self, start: Date, end: Date) -> Self {
        self.observation_period = Some((start, end));
        self
    }

    /// Sets the reset frequency for the observation.
    #[must_use]
    pub fn with_reset_frequency(mut self, frequency: Frequency) -> Self {
        self.reset_frequency = frequency;
        self
    }

    /// Sets the compounding method for rate calculation.
    #[must_use]
    pub fn with_compounding_method(mut self, method: CompoundingMethod) -> Self {
        self.compounding_method = method;
        self
    }

    /// Sets the lookback period in business days.
    #[must_use]
    pub fn with_lookback_period(mut self, days: i32) -> Self {
        self.lookback_period = Some(days);
        self
    }

    /// Sets the lockout period in business days.
    #[must_use]
    pub fn with_lockout_period(mut self, days: i32) -> Self {
        self.lockout_period = Some(days);
        self
    }

    /// Returns true if this observation has a compound period.
    #[must_use]
    pub fn is_compound_observation(&self) -> bool { self.observation_period.is_some() }

    /// Returns the observation period if set.
    #[must_use]
    pub fn period(&self) -> Option<(Date, Date)> { self.observation_period }

    /// Returns true if this observation requires daily compounding.
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
        set.insert(IndexType::Rate(RateIndex::Sofr));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_index_type_debug() {
        let index = IndexType::Rate(RateIndex::Sofr);
        let debug = format!("{:?}", index);
        assert!(debug.contains("Sofr"));
    }

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

        let ois_obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr))
            .with_observation_period(start, end);
        assert!(ois_obs.requires_daily_compounding());

        let ois_no_period = IndexObservation::new(IndexType::Rate(RateIndex::Sofr));
        assert!(!ois_no_period.requires_daily_compounding());
    }

    #[test]
    fn test_requires_daily_compounding_ibor() {
        let start = Date::from_ymd(2025, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 3, 31).unwrap();

        let ibor_obs = IndexObservation::new(IndexType::Rate(RateIndex::Euribor3M))
            .with_observation_period(start, end);
        assert!(!ibor_obs.requires_daily_compounding());
    }

    #[test]
    fn test_requires_daily_compounding_non_rate_index() {
        let start = Date::from_ymd(2025, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 3, 31).unwrap();

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

    #[test]
    fn test_from_rate_index_sofr() {
        use crate::{market::CompoundingMethod, time::Frequency};

        let obs = IndexObservation::from_rate_index(RateIndex::Sofr);

        assert!(matches!(obs.index_type, IndexType::Rate(RateIndex::Sofr)));
        assert_eq!(obs.reset_frequency, Frequency::Daily);
        assert_eq!(obs.compounding_method, CompoundingMethod::Compounded);
        assert!(obs.lookback_period.is_none());
        assert!(obs.lockout_period.is_none());
        assert_eq!(obs.observation_lag, 0);
    }

    #[test]
    fn test_from_rate_index_euribor() {
        use crate::{market::CompoundingMethod, time::Frequency};

        let obs = IndexObservation::from_rate_index(RateIndex::Euribor3M);

        assert!(matches!(
            obs.index_type,
            IndexType::Rate(RateIndex::Euribor3M)
        ));
        assert_eq!(obs.reset_frequency, Frequency::Quarterly);
        assert_eq!(obs.compounding_method, CompoundingMethod::Simple);
    }

    #[test]
    fn test_from_rate_index_sonia() {
        use crate::{market::CompoundingMethod, time::Frequency};

        let obs = IndexObservation::from_rate_index(RateIndex::Sonia);

        assert_eq!(obs.reset_frequency, Frequency::Daily);
        assert_eq!(obs.compounding_method, CompoundingMethod::Compounded);
    }

    #[test]
    fn test_index_observation_with_lookback() {
        let obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr)).with_lookback_period(5);

        assert_eq!(obs.lookback_period, Some(5));
    }

    #[test]
    fn test_index_observation_with_lockout() {
        let obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr)).with_lockout_period(2);

        assert_eq!(obs.lockout_period, Some(2));
    }

    #[test]
    fn test_index_observation_with_reset_frequency() {
        use crate::time::Frequency;

        let obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr))
            .with_reset_frequency(Frequency::Daily);

        assert_eq!(obs.reset_frequency, Frequency::Daily);
    }

    #[test]
    fn test_index_observation_with_compounding_method() {
        use crate::market::CompoundingMethod;

        let obs = IndexObservation::new(IndexType::Rate(RateIndex::Sofr))
            .with_compounding_method(CompoundingMethod::Averaged);

        assert_eq!(obs.compounding_method, CompoundingMethod::Averaged);
    }

    #[test]
    fn test_from_rate_index_preserves_metadata() {
        let sofr_obs = IndexObservation::from_rate_index(RateIndex::Sofr);
        let sofr_metadata = RateIndex::Sofr.metadata();
        assert_eq!(
            sofr_obs.compounding_method,
            sofr_metadata.compounding_method
        );

        let euribor_obs = IndexObservation::from_rate_index(RateIndex::Euribor3M);
        let euribor_metadata = RateIndex::Euribor3M.metadata();
        assert_eq!(
            euribor_obs.compounding_method,
            euribor_metadata.compounding_method
        );
    }

    #[test]
    fn test_ir_future_index() {
        let index = IndexType::IrFuture {
            exchange: "CME".into(),
            contract: "SR3".into(),
        };

        assert!(index.is_ir_future());
        assert!(!index.is_rate());
        assert!(!index.is_fx());
        assert!(!index.is_equity());
        assert!(!index.is_bond_future());
        assert!(!index.is_credit());
    }

    #[test]
    fn test_bond_future_index() {
        let index = IndexType::BondFuture {
            exchange: "EUREX".into(),
            contract: "RX".into(),
        };

        assert!(index.is_bond_future());
        assert!(!index.is_rate());
        assert!(!index.is_ir_future());
        assert!(!index.is_credit());
    }

    #[test]
    fn test_credit_index() {
        let index = IndexType::Credit {
            reference_entity: "ACME Corp".into(),
            seniority: Some("SNRFOR".into()),
        };

        assert!(index.is_credit());
        assert!(!index.is_rate());
        assert!(!index.is_fx());
        assert!(!index.is_equity());
    }

    #[test]
    fn test_credit_index_without_seniority() {
        let index = IndexType::Credit {
            reference_entity: "ACME Corp".into(),
            seniority: None,
        };

        assert!(index.is_credit());
    }

    #[test]
    fn test_existing_classifier_methods() {
        let equity = IndexType::Equity {
            ticker: "SPX".into(),
        };
        assert!(equity.is_equity());
        assert!(!equity.is_inflation());
        assert!(!equity.is_commodity());

        let inflation = IndexType::Inflation {
            name: "CPI".into(),
            region: "US".into(),
        };
        assert!(inflation.is_inflation());
        assert!(!inflation.is_equity());

        let commodity = IndexType::Commodity { name: "WTI".into() };
        assert!(commodity.is_commodity());
        assert!(!commodity.is_equity());

        let swap_rate = IndexType::SwapRate {
            currency: "USD".into(),
            tenor: "10Y".into(),
        };
        assert!(swap_rate.is_swap_rate());
        assert!(!swap_rate.is_rate());
    }

    #[test]
    fn test_ir_future_hash_and_eq() {
        use std::collections::HashSet;

        let idx1 = IndexType::IrFuture {
            exchange: "CME".into(),
            contract: "SR3".into(),
        };
        let idx2 = IndexType::IrFuture {
            exchange: "CME".into(),
            contract: "SR3".into(),
        };
        let idx3 = IndexType::IrFuture {
            exchange: "ICE".into(),
            contract: "SR3".into(),
        };

        assert_eq!(idx1, idx2);
        assert_ne!(idx1, idx3);

        let mut set = HashSet::new();
        set.insert(idx1);
        set.insert(idx2);
        set.insert(idx3);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_new_index_types_debug() {
        let ir_future = IndexType::IrFuture {
            exchange: "CME".into(),
            contract: "ED".into(),
        };
        let debug = format!("{:?}", ir_future);
        assert!(debug.contains("IrFuture"));
        assert!(debug.contains("CME"));

        let credit = IndexType::Credit {
            reference_entity: "ACME".into(),
            seniority: Some("SNRFOR".into()),
        };
        let debug = format!("{:?}", credit);
        assert!(debug.contains("Credit"));
        assert!(debug.contains("ACME"));
    }
}
