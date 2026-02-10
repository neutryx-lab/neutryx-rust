//! Ticker mapping for external data sources.
//!
//! This module provides the [`TickerMapping`] type for mapping external
//! tickers (Reuters RIC, Bloomberg ticker) to internal [`QuoteId`] identifiers.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::{TickerMapping, QuoteId, RateType, Currency};
//! use infra_domain::time::Tenor;
//!
//! let mut mapping = TickerMapping::new();
//!
//! let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
//! mapping.register("USD3MD=", quote_id);
//!
//! assert!(mapping.contains("USD3MD="));
//! assert!(mapping.lookup("USD3MD=").is_some());
//! ```

use std::collections::HashMap;

use crate::{
    market::{
        core::{Currency, RateType},
        quote::QuoteId,
    },
    time::Tenor,
};

/// Mapping from external tickers to internal rate identifiers.
///
/// Provides a bidirectional mapping between external data provider tickers
/// (such as Reuters RICs or Bloomberg tickers) and internal [`QuoteId`]
/// identifiers.
///
/// # Examples
///
/// ```
/// use infra_domain::market::{TickerMapping, QuoteId, RateType, Currency};
/// use infra_domain::time::Tenor;
///
/// // Create with default mappings for major currencies
/// let mapping = TickerMapping::with_defaults();
///
/// // Or create an empty mapping and register custom tickers
/// let mut custom = TickerMapping::new();
/// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
/// custom.register("USSW3M Curncy", quote_id);
/// ```
#[derive(Debug, Clone, Default)]
pub struct TickerMapping {
    /// Mapping from ticker string to QuoteId.
    mapping: HashMap<String, QuoteId>,
}

impl TickerMapping {
    /// Creates a new empty `TickerMapping`.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::TickerMapping;
    ///
    /// let mapping = TickerMapping::new();
    /// assert!(mapping.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }

    /// Creates a `TickerMapping` with default mappings for major currencies.
    ///
    /// Includes standard mappings for USD, EUR, GBP, JPY, and CHF
    /// for common rate types (deposits, swaps).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::TickerMapping;
    ///
    /// let mapping = TickerMapping::with_defaults();
    /// assert!(!mapping.is_empty());
    /// ```
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut mapping = Self::new();

        // USD Deposits
        mapping.register(
            "USD1MD=",
            QuoteId::new(Currency::USD, Tenor::OneMonth, RateType::Deposit),
        );
        mapping.register(
            "USD3MD=",
            QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit),
        );
        mapping.register(
            "USD6MD=",
            QuoteId::new(Currency::USD, Tenor::SixMonths, RateType::Deposit),
        );

        // USD Swaps
        mapping.register(
            "USSW1 Curncy",
            QuoteId::new(Currency::USD, Tenor::OneYear, RateType::Swap),
        );
        mapping.register(
            "USSW2 Curncy",
            QuoteId::new(Currency::USD, Tenor::TwoYears, RateType::Swap),
        );
        mapping.register(
            "USSW5 Curncy",
            QuoteId::new(Currency::USD, Tenor::FiveYears, RateType::Swap),
        );
        mapping.register(
            "USSW10 Curncy",
            QuoteId::new(Currency::USD, Tenor::TenYears, RateType::Swap),
        );

        // EUR Deposits
        mapping.register(
            "EUR1MD=",
            QuoteId::new(Currency::EUR, Tenor::OneMonth, RateType::Deposit),
        );
        mapping.register(
            "EUR3MD=",
            QuoteId::new(Currency::EUR, Tenor::ThreeMonths, RateType::Deposit),
        );
        mapping.register(
            "EUR6MD=",
            QuoteId::new(Currency::EUR, Tenor::SixMonths, RateType::Deposit),
        );

        // EUR Swaps
        mapping.register(
            "EUSW1 Curncy",
            QuoteId::new(Currency::EUR, Tenor::OneYear, RateType::Swap),
        );
        mapping.register(
            "EUSW2 Curncy",
            QuoteId::new(Currency::EUR, Tenor::TwoYears, RateType::Swap),
        );
        mapping.register(
            "EUSW5 Curncy",
            QuoteId::new(Currency::EUR, Tenor::FiveYears, RateType::Swap),
        );
        mapping.register(
            "EUSW10 Curncy",
            QuoteId::new(Currency::EUR, Tenor::TenYears, RateType::Swap),
        );

        // GBP Deposits
        mapping.register(
            "GBP1MD=",
            QuoteId::new(Currency::GBP, Tenor::OneMonth, RateType::Deposit),
        );
        mapping.register(
            "GBP3MD=",
            QuoteId::new(Currency::GBP, Tenor::ThreeMonths, RateType::Deposit),
        );

        // GBP Swaps
        mapping.register(
            "BPSW1 Curncy",
            QuoteId::new(Currency::GBP, Tenor::OneYear, RateType::Swap),
        );
        mapping.register(
            "BPSW5 Curncy",
            QuoteId::new(Currency::GBP, Tenor::FiveYears, RateType::Swap),
        );

        // JPY Deposits
        mapping.register(
            "JPY1MD=",
            QuoteId::new(Currency::JPY, Tenor::OneMonth, RateType::Deposit),
        );
        mapping.register(
            "JPY3MD=",
            QuoteId::new(Currency::JPY, Tenor::ThreeMonths, RateType::Deposit),
        );

        // JPY Swaps
        mapping.register(
            "JYSW1 Curncy",
            QuoteId::new(Currency::JPY, Tenor::OneYear, RateType::Swap),
        );
        mapping.register(
            "JYSW5 Curncy",
            QuoteId::new(Currency::JPY, Tenor::FiveYears, RateType::Swap),
        );

        // CHF Deposits
        mapping.register(
            "CHF1MD=",
            QuoteId::new(Currency::CHF, Tenor::OneMonth, RateType::Deposit),
        );
        mapping.register(
            "CHF3MD=",
            QuoteId::new(Currency::CHF, Tenor::ThreeMonths, RateType::Deposit),
        );

        // CHF Swaps
        mapping.register(
            "SFSW1 Curncy",
            QuoteId::new(Currency::CHF, Tenor::OneYear, RateType::Swap),
        );
        mapping.register(
            "SFSW5 Curncy",
            QuoteId::new(Currency::CHF, Tenor::FiveYears, RateType::Swap),
        );

        mapping
    }

    /// Registers a ticker mapping.
    ///
    /// # Arguments
    ///
    /// * `ticker` - The external ticker string
    /// * `quote_id` - The internal rate identifier
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{TickerMapping, QuoteId, RateType, Currency};
    /// use infra_domain::time::Tenor;
    ///
    /// let mut mapping = TickerMapping::new();
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// mapping.register("USD3MD=", quote_id);
    /// ```
    pub fn register(&mut self, ticker: impl Into<String>, quote_id: QuoteId) {
        self.mapping.insert(ticker.into(), quote_id);
    }

    /// Looks up a rate ID by ticker.
    ///
    /// Returns `None` if the ticker is not found.
    ///
    /// # Arguments
    ///
    /// * `ticker` - The external ticker string to look up
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::TickerMapping;
    ///
    /// let mapping = TickerMapping::with_defaults();
    ///
    /// // Existing ticker
    /// assert!(mapping.lookup("USD3MD=").is_some());
    ///
    /// // Unknown ticker
    /// assert!(mapping.lookup("UNKNOWN").is_none());
    /// ```
    #[must_use]
    pub fn lookup(&self, ticker: &str) -> Option<&QuoteId> { self.mapping.get(ticker) }

    /// Checks if a ticker is registered.
    ///
    /// # Arguments
    ///
    /// * `ticker` - The external ticker string to check
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::TickerMapping;
    ///
    /// let mapping = TickerMapping::with_defaults();
    ///
    /// assert!(mapping.contains("USD3MD="));
    /// assert!(!mapping.contains("UNKNOWN"));
    /// ```
    #[must_use]
    pub fn contains(&self, ticker: &str) -> bool { self.mapping.contains_key(ticker) }

    /// Returns the number of registered tickers.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::TickerMapping;
    ///
    /// let mapping = TickerMapping::new();
    /// assert_eq!(mapping.len(), 0);
    ///
    /// let defaults = TickerMapping::with_defaults();
    /// assert!(defaults.len() > 0);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize { self.mapping.len() }

    /// Returns `true` if no tickers are registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::TickerMapping;
    ///
    /// let mapping = TickerMapping::new();
    /// assert!(mapping.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool { self.mapping.is_empty() }

    /// Returns an iterator over all registered tickers.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::TickerMapping;
    ///
    /// let mapping = TickerMapping::with_defaults();
    /// for (ticker, quote_id) in mapping.iter() {
    ///     println!("{} -> {}", ticker, quote_id);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&String, &QuoteId)> { self.mapping.iter() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticker_mapping_operations() {
        // new + empty + default
        let empty = TickerMapping::new();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert_eq!(TickerMapping::default().len(), 0);
        assert!(format!("{:?}", empty).contains("TickerMapping"));

        // register + lookup + contains + len
        let mut m = TickerMapping::new();
        let q1 = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        m.register("TEST", q1.clone());
        assert!(m.contains("TEST"));
        assert!(!m.contains("NOPE"));
        assert_eq!(m.lookup("TEST"), Some(&q1));
        assert!(m.lookup("NOPE").is_none());
        assert_eq!(m.len(), 1);

        m.register("T2", QuoteId::new(Currency::EUR, Tenor::OneMonth, RateType::Deposit));
        assert_eq!(m.len(), 2);

        // overwrite
        let q2 = QuoteId::new(Currency::EUR, Tenor::OneMonth, RateType::Deposit);
        m.register("TEST", q2.clone());
        assert_eq!(m.len(), 2);
        assert_eq!(m.lookup("TEST"), Some(&q2));

        // clone
        let c = m.clone();
        assert_eq!(c.len(), m.len());
        for (t, q) in m.iter() { assert_eq!(c.lookup(t), Some(q)); }
    }

    #[test]
    fn test_ticker_mapping_defaults() {
        let m = TickerMapping::with_defaults();
        assert!(!m.is_empty());

        // iter consistency
        assert_eq!(m.iter().count(), m.len());
        for (t, q) in m.iter() { assert!(!t.is_empty()); assert_eq!(m.lookup(t), Some(q)); }

        // specific lookups
        let q = m.lookup("USD3MD=").unwrap();
        assert_eq!(q.currency, Currency::USD);
        assert_eq!(q.tenor, Tenor::ThreeMonths);
        assert_eq!(q.rate_type, RateType::Deposit);
        assert!(m.contains("EUR3MD="));
        assert!(m.contains("USSW5 Curncy"));
        assert!(m.contains("EUSW5 Curncy"));

        // multi-currency defaults
        for (dep, sw) in [("USD1MD=","USSW1 Curncy"),("EUR1MD=","EUSW1 Curncy"),
                           ("GBP1MD=","BPSW1 Curncy"),("JPY1MD=","JYSW1 Curncy"),("CHF1MD=","SFSW1 Curncy")] {
            assert!(m.lookup(dep).is_some(), "missing {dep}");
            assert!(m.lookup(sw).is_some(), "missing {sw}");
        }
    }
}
