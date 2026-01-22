//! Rate identifier for market rates.
//!
//! This module provides the [`RateId`] type for uniquely identifying
//! market rates by their currency, tenor, rate type, and optional index.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::{RateId, RateType, Currency, RateIndex};
//! use infra_master::time::Tenor;
//!
//! let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
//! assert_eq!(rate_id.currency, Currency::USD);
//! assert_eq!(rate_id.tenor, Tenor::ThreeMonths);
//! assert_eq!(rate_id.rate_type, RateType::Swap);
//! ```

use std::fmt;

use crate::market::{Currency, RateIndex};
use crate::time::Tenor;

use super::rate_type::RateType;

/// Unique identifier for a market rate.
///
/// A `RateId` uniquely identifies a market rate by combining:
/// - Currency (e.g., USD, EUR)
/// - Tenor (e.g., 3M, 1Y)
/// - Rate type (e.g., Swap, Deposit)
/// - Optional rate index (e.g., SOFR, EURIBOR)
///
/// # Examples
///
/// ```
/// use infra_master::market::{RateId, RateType, Currency, RateIndex};
/// use infra_master::time::Tenor;
///
/// // Create a rate ID
/// let rate_id = RateId::new(Currency::USD, Tenor::OneYear, RateType::Ois);
///
/// // With a rate index
/// let rate_id_with_index = RateId::new(Currency::USD, Tenor::OneYear, RateType::Ois)
///     .with_index(RateIndex::Sofr);
/// assert_eq!(rate_id_with_index.rate_index, Some(RateIndex::Sofr));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RateId {
    /// Currency of the rate.
    pub currency: Currency,
    /// Tenor of the rate.
    pub tenor: Tenor,
    /// Type of the rate (Deposit, Swap, etc.).
    pub rate_type: RateType,
    /// Optional rate index (SOFR, EURIBOR, etc.).
    pub rate_index: Option<RateIndex>,
}

impl RateId {
    /// Creates a new `RateId`.
    ///
    /// # Arguments
    ///
    /// * `currency` - The currency of the rate
    /// * `tenor` - The tenor of the rate
    /// * `rate_type` - The type of the rate
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{RateId, RateType, Currency};
    /// use infra_master::time::Tenor;
    ///
    /// let rate_id = RateId::new(Currency::EUR, Tenor::FiveYears, RateType::Swap);
    /// ```
    #[must_use]
    pub fn new(currency: Currency, tenor: Tenor, rate_type: RateType) -> Self {
        Self {
            currency,
            tenor,
            rate_type,
            rate_index: None,
        }
    }

    /// Adds a rate index to this `RateId`.
    ///
    /// Returns a new `RateId` with the specified rate index.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index to add
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{RateId, RateType, Currency, RateIndex};
    /// use infra_master::time::Tenor;
    ///
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Ois)
    ///     .with_index(RateIndex::Sofr);
    ///
    /// assert_eq!(rate_id.rate_index, Some(RateIndex::Sofr));
    /// ```
    #[must_use]
    pub fn with_index(mut self, index: RateIndex) -> Self {
        self.rate_index = Some(index);
        self
    }

    /// Returns a description string for this rate ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{RateId, RateType, Currency};
    /// use infra_master::time::Tenor;
    ///
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
    /// assert_eq!(rate_id.description(), "USD 3M SWAP");
    /// ```
    #[must_use]
    pub fn description(&self) -> String {
        format!(
            "{} {} {}",
            self.currency.code(),
            self.tenor.code(),
            self.rate_type.code()
        )
    }
}

impl fmt::Display for RateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref index) = self.rate_index {
            write!(
                f,
                "{} {} {} ({})",
                self.currency.code(),
                self.tenor.code(),
                self.rate_type.code(),
                index.code()
            )
        } else {
            write!(
                f,
                "{} {} {}",
                self.currency.code(),
                self.tenor.code(),
                self.rate_type.code()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_rate_id_new() {
        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);

        assert_eq!(rate_id.currency, Currency::USD);
        assert_eq!(rate_id.tenor, Tenor::ThreeMonths);
        assert_eq!(rate_id.rate_type, RateType::Swap);
        assert_eq!(rate_id.rate_index, None);
    }

    #[test]
    fn test_rate_id_with_index() {
        let rate_id = RateId::new(Currency::USD, Tenor::OneYear, RateType::Ois)
            .with_index(RateIndex::Sofr);

        assert_eq!(rate_id.currency, Currency::USD);
        assert_eq!(rate_id.tenor, Tenor::OneYear);
        assert_eq!(rate_id.rate_type, RateType::Ois);
        assert_eq!(rate_id.rate_index, Some(RateIndex::Sofr));
    }

    #[test]
    fn test_rate_id_description() {
        let rate_id = RateId::new(Currency::EUR, Tenor::FiveYears, RateType::Swap);
        assert_eq!(rate_id.description(), "EUR 5Y SWAP");

        let rate_id_deposit = RateId::new(Currency::USD, Tenor::OneMonth, RateType::Deposit);
        assert_eq!(rate_id_deposit.description(), "USD 1M DEPO");
    }

    #[test]
    fn test_rate_id_display_without_index() {
        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
        assert_eq!(format!("{}", rate_id), "USD 3M SWAP");
    }

    #[test]
    fn test_rate_id_display_with_index() {
        let rate_id = RateId::new(Currency::USD, Tenor::OneYear, RateType::Ois)
            .with_index(RateIndex::Sofr);
        assert_eq!(format!("{}", rate_id), "USD 1Y OIS (SOFR)");
    }

    #[test]
    fn test_rate_id_clone() {
        let original = RateId::new(Currency::GBP, Tenor::TenYears, RateType::Swap);
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_rate_id_eq() {
        let id1 = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
        let id2 = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
        let id3 = RateId::new(Currency::EUR, Tenor::ThreeMonths, RateType::Swap);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_rate_id_eq_with_index() {
        let id1 = RateId::new(Currency::USD, Tenor::OneYear, RateType::Ois)
            .with_index(RateIndex::Sofr);
        let id2 = RateId::new(Currency::USD, Tenor::OneYear, RateType::Ois)
            .with_index(RateIndex::Sofr);
        let id3 = RateId::new(Currency::USD, Tenor::OneYear, RateType::Ois);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3); // Different because of index
    }

    #[test]
    fn test_rate_id_hash() {
        let mut set = HashSet::new();

        let id1 = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
        let id2 = RateId::new(Currency::EUR, Tenor::FiveYears, RateType::Swap);
        let id3 = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap); // Duplicate

        set.insert(id1);
        set.insert(id2);
        set.insert(id3);

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_rate_id_as_hashmap_key() {
        let mut map: HashMap<RateId, f64> = HashMap::new();

        let id1 = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let id2 = RateId::new(Currency::USD, Tenor::SixMonths, RateType::Deposit);

        map.insert(id1.clone(), 0.05);
        map.insert(id2.clone(), 0.055);

        assert_eq!(map.get(&id1), Some(&0.05));
        assert_eq!(map.get(&id2), Some(&0.055));
    }

    #[test]
    fn test_rate_id_debug() {
        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
        let debug_str = format!("{:?}", rate_id);

        assert!(debug_str.contains("RateId"));
        assert!(debug_str.contains("USD"));
        assert!(debug_str.contains("ThreeMonths"));
        assert!(debug_str.contains("Swap"));
    }

    #[test]
    fn test_rate_id_various_combinations() {
        // Test various currency/tenor/type combinations
        let combinations = vec![
            (Currency::USD, Tenor::Overnight, RateType::Deposit),
            (Currency::EUR, Tenor::OneWeek, RateType::Fra),
            (Currency::GBP, Tenor::OneMonth, RateType::Futures),
            (Currency::JPY, Tenor::ThreeMonths, RateType::Swap),
            (Currency::CHF, Tenor::SixMonths, RateType::Ois),
        ];

        for (currency, tenor, rate_type) in combinations {
            let rate_id = RateId::new(currency, tenor, rate_type);
            assert_eq!(rate_id.currency, currency);
            assert_eq!(rate_id.tenor, tenor);
            assert_eq!(rate_id.rate_type, rate_type);
        }
    }
}
