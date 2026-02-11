//! Quote identifier for market quotes.

use std::fmt;

use crate::{
    market::{
        core::{Currency, QuoteCategory},
        index::RateIndex,
    },
    time::Tenor,
};

/// Unique identifier for a market quote.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QuoteId {
    /// Currency of the quote.
    pub currency: Currency,
    /// Tenor of the quote.
    pub tenor: Tenor,
    /// Quote category (Deposit, Swap, etc.).
    pub quote_category: QuoteCategory,
    /// Optional rate index (SOFR, EURIBOR, etc.).
    pub rate_index: Option<RateIndex>,
}

impl QuoteId {
    /// Creates a new `QuoteId`.
    #[must_use]
    pub fn new(currency: Currency, tenor: Tenor, quote_category: QuoteCategory) -> Self {
        Self {
            currency,
            tenor,
            quote_category,
            rate_index: None,
        }
    }

    /// Adds a rate index to this `QuoteId`.
    #[must_use]
    pub fn with_index(mut self, index: RateIndex) -> Self {
        self.rate_index = Some(index);
        self
    }

    /// Returns a description string for this quote ID.
    #[must_use]
    pub fn description(&self) -> String {
        format!(
            "{} {} {}",
            self.currency.code(),
            self.tenor.code(),
            self.quote_category.code()
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
                self.quote_category.code(),
                index.code()
            )
        } else {
            write!(
                f,
                "{} {} {}",
                self.currency.code(),
                self.tenor.code(),
                self.quote_category.code()
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
        let id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Swap);
        assert_eq!(id.currency, Currency::USD);
        assert_eq!(id.rate_index, None);
        assert_eq!(id.description(), "USD 3M SWAP");
        assert_eq!(format!("{}", id), "USD 3M SWAP");

        let with_idx =
            QuoteId::new(Currency::USD, Tenor::OneYear, QuoteCategory::Ois).with_index(RateIndex::Sofr);
        assert_eq!(with_idx.rate_index, Some(RateIndex::Sofr));
        assert_eq!(format!("{}", with_idx), "USD 1Y OIS (SOFR)");
    }

    #[test]
    fn test_equality_and_hash() {
        let id1 = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Swap);
        let id2 = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Swap);
        let id3 = QuoteId::new(Currency::EUR, Tenor::ThreeMonths, QuoteCategory::Swap);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);

        let with_idx = id1.clone().with_index(RateIndex::Sofr);
        assert_ne!(id1, with_idx);

        let mut set = HashSet::new();
        set.insert(id1);
        set.insert(id2);
        set.insert(id3);
        assert_eq!(set.len(), 2);
    }
}
