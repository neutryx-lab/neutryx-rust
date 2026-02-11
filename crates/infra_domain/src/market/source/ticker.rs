//! Ticker mapping for external data sources.

use std::collections::HashMap;

use crate::{
    market::{
        core::{Currency, QuoteCategory},
        quote::QuoteId,
    },
    time::Tenor,
};

/// Mapping from external tickers to internal rate identifiers.
#[derive(Debug, Clone, Default)]
pub struct TickerMapping {
    /// Mapping from ticker string to QuoteId.
    mapping: HashMap<String, QuoteId>,
}

impl TickerMapping {
    /// Creates a new empty `TickerMapping`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }

    /// Creates a `TickerMapping` with default mappings for major currencies.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut mapping = Self::new();

        mapping.register(
            "USD1MD=",
            QuoteId::new(Currency::USD, Tenor::OneMonth, QuoteCategory::Deposit),
        );
        mapping.register(
            "USD3MD=",
            QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit),
        );
        mapping.register(
            "USD6MD=",
            QuoteId::new(Currency::USD, Tenor::SixMonths, QuoteCategory::Deposit),
        );

        mapping.register(
            "USSW1 Curncy",
            QuoteId::new(Currency::USD, Tenor::OneYear, QuoteCategory::Swap),
        );
        mapping.register(
            "USSW2 Curncy",
            QuoteId::new(Currency::USD, Tenor::TwoYears, QuoteCategory::Swap),
        );
        mapping.register(
            "USSW5 Curncy",
            QuoteId::new(Currency::USD, Tenor::FiveYears, QuoteCategory::Swap),
        );
        mapping.register(
            "USSW10 Curncy",
            QuoteId::new(Currency::USD, Tenor::TenYears, QuoteCategory::Swap),
        );

        mapping.register(
            "EUR1MD=",
            QuoteId::new(Currency::EUR, Tenor::OneMonth, QuoteCategory::Deposit),
        );
        mapping.register(
            "EUR3MD=",
            QuoteId::new(Currency::EUR, Tenor::ThreeMonths, QuoteCategory::Deposit),
        );
        mapping.register(
            "EUR6MD=",
            QuoteId::new(Currency::EUR, Tenor::SixMonths, QuoteCategory::Deposit),
        );

        mapping.register(
            "EUSW1 Curncy",
            QuoteId::new(Currency::EUR, Tenor::OneYear, QuoteCategory::Swap),
        );
        mapping.register(
            "EUSW2 Curncy",
            QuoteId::new(Currency::EUR, Tenor::TwoYears, QuoteCategory::Swap),
        );
        mapping.register(
            "EUSW5 Curncy",
            QuoteId::new(Currency::EUR, Tenor::FiveYears, QuoteCategory::Swap),
        );
        mapping.register(
            "EUSW10 Curncy",
            QuoteId::new(Currency::EUR, Tenor::TenYears, QuoteCategory::Swap),
        );

        mapping.register(
            "GBP1MD=",
            QuoteId::new(Currency::GBP, Tenor::OneMonth, QuoteCategory::Deposit),
        );
        mapping.register(
            "GBP3MD=",
            QuoteId::new(Currency::GBP, Tenor::ThreeMonths, QuoteCategory::Deposit),
        );

        mapping.register(
            "BPSW1 Curncy",
            QuoteId::new(Currency::GBP, Tenor::OneYear, QuoteCategory::Swap),
        );
        mapping.register(
            "BPSW5 Curncy",
            QuoteId::new(Currency::GBP, Tenor::FiveYears, QuoteCategory::Swap),
        );

        mapping.register(
            "JPY1MD=",
            QuoteId::new(Currency::JPY, Tenor::OneMonth, QuoteCategory::Deposit),
        );
        mapping.register(
            "JPY3MD=",
            QuoteId::new(Currency::JPY, Tenor::ThreeMonths, QuoteCategory::Deposit),
        );

        mapping.register(
            "JYSW1 Curncy",
            QuoteId::new(Currency::JPY, Tenor::OneYear, QuoteCategory::Swap),
        );
        mapping.register(
            "JYSW5 Curncy",
            QuoteId::new(Currency::JPY, Tenor::FiveYears, QuoteCategory::Swap),
        );

        mapping.register(
            "CHF1MD=",
            QuoteId::new(Currency::CHF, Tenor::OneMonth, QuoteCategory::Deposit),
        );
        mapping.register(
            "CHF3MD=",
            QuoteId::new(Currency::CHF, Tenor::ThreeMonths, QuoteCategory::Deposit),
        );

        mapping.register(
            "SFSW1 Curncy",
            QuoteId::new(Currency::CHF, Tenor::OneYear, QuoteCategory::Swap),
        );
        mapping.register(
            "SFSW5 Curncy",
            QuoteId::new(Currency::CHF, Tenor::FiveYears, QuoteCategory::Swap),
        );

        mapping
    }

    /// Registers a ticker mapping.
    pub fn register(&mut self, ticker: impl Into<String>, quote_id: QuoteId) {
        self.mapping.insert(ticker.into(), quote_id);
    }

    /// Looks up a rate ID by ticker.
    #[must_use]
    pub fn lookup(&self, ticker: &str) -> Option<&QuoteId> { self.mapping.get(ticker) }

    /// Checks if a ticker is registered.
    #[must_use]
    pub fn contains(&self, ticker: &str) -> bool { self.mapping.contains_key(ticker) }

    /// Returns the number of registered tickers.
    #[must_use]
    pub fn len(&self) -> usize { self.mapping.len() }

    /// Returns `true` if no tickers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.mapping.is_empty() }

    /// Returns an iterator over all registered tickers.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &QuoteId)> { self.mapping.iter() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticker_mapping_operations() {
        let empty = TickerMapping::new();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert_eq!(TickerMapping::default().len(), 0);
        assert!(format!("{:?}", empty).contains("TickerMapping"));

        let mut m = TickerMapping::new();
        let q1 = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit);
        m.register("TEST", q1.clone());
        assert!(m.contains("TEST"));
        assert!(!m.contains("NOPE"));
        assert_eq!(m.lookup("TEST"), Some(&q1));
        assert!(m.lookup("NOPE").is_none());
        assert_eq!(m.len(), 1);

        m.register(
            "T2",
            QuoteId::new(Currency::EUR, Tenor::OneMonth, QuoteCategory::Deposit),
        );
        assert_eq!(m.len(), 2);

        let q2 = QuoteId::new(Currency::EUR, Tenor::OneMonth, QuoteCategory::Deposit);
        m.register("TEST", q2.clone());
        assert_eq!(m.len(), 2);
        assert_eq!(m.lookup("TEST"), Some(&q2));

        let c = m.clone();
        assert_eq!(c.len(), m.len());
        for (t, q) in m.iter() {
            assert_eq!(c.lookup(t), Some(q));
        }
    }

    #[test]
    fn test_ticker_mapping_defaults() {
        let m = TickerMapping::with_defaults();
        assert!(!m.is_empty());

        assert_eq!(m.iter().count(), m.len());
        for (t, q) in m.iter() {
            assert!(!t.is_empty());
            assert_eq!(m.lookup(t), Some(q));
        }

        let q = m.lookup("USD3MD=").unwrap();
        assert_eq!(q.currency, Currency::USD);
        assert_eq!(q.tenor, Tenor::ThreeMonths);
        assert_eq!(q.quote_category, QuoteCategory::Deposit);
        assert!(m.contains("EUR3MD="));
        assert!(m.contains("USSW5 Curncy"));
        assert!(m.contains("EUSW5 Curncy"));

        for (dep, sw) in [
            ("USD1MD=", "USSW1 Curncy"),
            ("EUR1MD=", "EUSW1 Curncy"),
            ("GBP1MD=", "BPSW1 Curncy"),
            ("JPY1MD=", "JYSW1 Curncy"),
            ("CHF1MD=", "SFSW1 Curncy"),
        ] {
            assert!(m.lookup(dep).is_some(), "missing {dep}");
            assert!(m.lookup(sw).is_some(), "missing {sw}");
        }
    }
}
