//! Quote identifier for market quotes.
//!
//! This module provides the [`QuoteId`] type for uniquely identifying
//! market quotes by their currency, tenor, rate type, and optional index.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::{QuoteId, RateType, Currency, RateIndex};
//! use infra_domain::time::Tenor;
//!
//! let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
//! assert_eq!(quote_id.currency, Currency::USD);
//! assert_eq!(quote_id.tenor, Tenor::ThreeMonths);
//! assert_eq!(quote_id.rate_type, RateType::Swap);
//! ```

use std::fmt;

use crate::{
    market::{
        core::{Currency, RateType},
        index::RateIndex,
    },
    time::Tenor,
};

/// Unique identifier for a market quote.
///
/// A `QuoteId` uniquely identifies a market quote by combining:
/// - Currency (e.g., USD, EUR)
/// - Tenor (e.g., 3M, 1Y)
/// - Rate type (e.g., Swap, Deposit)
/// - Optional rate index (e.g., SOFR, EURIBOR)
///
/// # Examples
///
/// ```
/// use infra_domain::market::{QuoteId, RateType, Currency, RateIndex};
/// use infra_domain::time::Tenor;
///
/// // Create a quote ID
/// let quote_id = QuoteId::new(Currency::USD, Tenor::OneYear, RateType::Ois);
///
/// // With a rate index
/// let quote_id_with_index = QuoteId::new(Currency::USD, Tenor::OneYear, RateType::Ois)
///     .with_index(RateIndex::Sofr);
/// assert_eq!(quote_id_with_index.rate_index, Some(RateIndex::Sofr));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QuoteId {
    /// Currency of the quote.
    pub currency: Currency,
    /// Tenor of the quote.
    pub tenor: Tenor,
    /// Type of the rate (Deposit, Swap, etc.).
    pub rate_type: RateType,
    /// Optional rate index (SOFR, EURIBOR, etc.).
    pub rate_index: Option<RateIndex>,
}

impl QuoteId {
    /// Creates a new `QuoteId`.
    ///
    /// # Arguments
    ///
    /// * `currency` - The currency of the quote
    /// * `tenor` - The tenor of the quote
    /// * `rate_type` - The type of the rate
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{QuoteId, RateType, Currency};
    /// use infra_domain::time::Tenor;
    ///
    /// let quote_id = QuoteId::new(Currency::EUR, Tenor::FiveYears, RateType::Swap);
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

    /// Adds a rate index to this `QuoteId`.
    ///
    /// Returns a new `QuoteId` with the specified rate index.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index to add
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{QuoteId, RateType, Currency, RateIndex};
    /// use infra_domain::time::Tenor;
    ///
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Ois)
    ///     .with_index(RateIndex::Sofr);
    ///
    /// assert_eq!(quote_id.rate_index, Some(RateIndex::Sofr));
    /// ```
    #[must_use]
    pub fn with_index(mut self, index: RateIndex) -> Self {
        self.rate_index = Some(index);
        self
    }

    /// Returns a description string for this quote ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{QuoteId, RateType, Currency};
    /// use infra_domain::time::Tenor;
    ///
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
    /// assert_eq!(quote_id.description(), "USD 3M SWAP");
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

impl fmt::Display for QuoteId {
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
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_construction_and_display() {
        let id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
        assert_eq!(id.currency, Currency::USD);
        assert_eq!(id.rate_index, None);
        assert_eq!(id.description(), "USD 3M SWAP");
        assert_eq!(format!("{}", id), "USD 3M SWAP");

        let with_idx = QuoteId::new(Currency::USD, Tenor::OneYear, RateType::Ois).with_index(RateIndex::Sofr);
        assert_eq!(with_idx.rate_index, Some(RateIndex::Sofr));
        assert_eq!(format!("{}", with_idx), "USD 1Y OIS (SOFR)");
    }

    #[test]
    fn test_equality_and_hash() {
        let id1 = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
        let id2 = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
        let id3 = QuoteId::new(Currency::EUR, Tenor::ThreeMonths, RateType::Swap);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);

        // Index affects equality
        let with_idx = id1.clone().with_index(RateIndex::Sofr);
        assert_ne!(id1, with_idx);

        let mut set = HashSet::new();
        set.insert(id1);
        set.insert(id2); // duplicate
        set.insert(id3);
        assert_eq!(set.len(), 2);
    }
}
