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
    market::{core::{Currency, RateType}, quote::QuoteId},
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
    fn test_ticker_mapping_new() {
        let mapping = TickerMapping::new();
        assert!(mapping.is_empty());
        assert_eq!(mapping.len(), 0);
    }

    #[test]
    fn test_ticker_mapping_with_defaults() {
        let mapping = TickerMapping::with_defaults();

        // Should have mappings for multiple currencies
        assert!(!mapping.is_empty());

        // Check some specific defaults
        assert!(mapping.contains("USD3MD="));
        assert!(mapping.contains("EUR3MD="));
        assert!(mapping.contains("USSW5 Curncy"));
        assert!(mapping.contains("EUSW5 Curncy"));
    }

    #[test]
    fn test_ticker_mapping_register() {
        let mut mapping = TickerMapping::new();

        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        mapping.register("TEST_TICKER", quote_id.clone());

        assert!(mapping.contains("TEST_TICKER"));
        assert_eq!(mapping.lookup("TEST_TICKER"), Some(&quote_id));
    }

    #[test]
    fn test_ticker_mapping_lookup_found() {
        let mapping = TickerMapping::with_defaults();

        let result = mapping.lookup("USD3MD=");
        assert!(result.is_some());

        let quote_id = result.unwrap();
        assert_eq!(quote_id.currency, Currency::USD);
        assert_eq!(quote_id.tenor, Tenor::ThreeMonths);
        assert_eq!(quote_id.rate_type, RateType::Deposit);
    }

    #[test]
    fn test_ticker_mapping_lookup_not_found() {
        let mapping = TickerMapping::with_defaults();

        let result = mapping.lookup("UNKNOWN_TICKER");
        assert!(result.is_none());
    }

    #[test]
    fn test_ticker_mapping_contains() {
        let mapping = TickerMapping::with_defaults();

        assert!(mapping.contains("USD3MD="));
        assert!(!mapping.contains("NONEXISTENT"));
    }

    #[test]
    fn test_ticker_mapping_len() {
        let mut mapping = TickerMapping::new();
        assert_eq!(mapping.len(), 0);

        mapping.register(
            "TICK1",
            QuoteId::new(Currency::USD, Tenor::OneMonth, RateType::Deposit),
        );
        assert_eq!(mapping.len(), 1);

        mapping.register(
            "TICK2",
            QuoteId::new(Currency::EUR, Tenor::OneMonth, RateType::Deposit),
        );
        assert_eq!(mapping.len(), 2);
    }

    #[test]
    fn test_ticker_mapping_is_empty() {
        let empty = TickerMapping::new();
        assert!(empty.is_empty());

        let defaults = TickerMapping::with_defaults();
        assert!(!defaults.is_empty());
    }

    #[test]
    fn test_ticker_mapping_iter() {
        let mapping = TickerMapping::with_defaults();

        let count = mapping.iter().count();
        assert_eq!(count, mapping.len());

        // Verify all entries are valid
        for (ticker, quote_id) in mapping.iter() {
            assert!(!ticker.is_empty());
            assert!(mapping.contains(ticker));
            assert_eq!(mapping.lookup(ticker), Some(quote_id));
        }
    }

    #[test]
    fn test_ticker_mapping_overwrite() {
        let mut mapping = TickerMapping::new();

        let quote_id1 = QuoteId::new(Currency::USD, Tenor::OneMonth, RateType::Deposit);
        let quote_id2 = QuoteId::new(Currency::EUR, Tenor::OneMonth, RateType::Deposit);

        mapping.register("SAME_TICKER", quote_id1);
        mapping.register("SAME_TICKER", quote_id2.clone());

        // Should have overwritten
        assert_eq!(mapping.len(), 1);
        assert_eq!(mapping.lookup("SAME_TICKER"), Some(&quote_id2));
    }

    #[test]
    fn test_ticker_mapping_clone() {
        let original = TickerMapping::with_defaults();
        let cloned = original.clone();

        assert_eq!(original.len(), cloned.len());

        for (ticker, quote_id) in original.iter() {
            assert_eq!(cloned.lookup(ticker), Some(quote_id));
        }
    }

    #[test]
    fn test_ticker_mapping_debug() {
        let mapping = TickerMapping::new();
        let debug_str = format!("{:?}", mapping);
        assert!(debug_str.contains("TickerMapping"));
    }

    #[test]
    fn test_ticker_mapping_default() {
        let mapping = TickerMapping::default();
        assert!(mapping.is_empty());
    }

    #[test]
    fn test_default_mappings_currencies() {
        let mapping = TickerMapping::with_defaults();

        // USD
        assert!(mapping.lookup("USD1MD=").is_some());
        assert!(mapping.lookup("USSW1 Curncy").is_some());

        // EUR
        assert!(mapping.lookup("EUR1MD=").is_some());
        assert!(mapping.lookup("EUSW1 Curncy").is_some());

        // GBP
        assert!(mapping.lookup("GBP1MD=").is_some());
        assert!(mapping.lookup("BPSW1 Curncy").is_some());

        // JPY
        assert!(mapping.lookup("JPY1MD=").is_some());
        assert!(mapping.lookup("JYSW1 Curncy").is_some());

        // CHF
        assert!(mapping.lookup("CHF1MD=").is_some());
        assert!(mapping.lookup("SFSW1 Curncy").is_some());
    }
}
