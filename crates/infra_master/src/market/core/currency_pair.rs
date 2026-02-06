//! Currency pair representation for FX markets.
//!
//! This module provides the fundamental [`CurrencyPair`] type used throughout
//! the system for representing FX currency pairs.
//!
//! # Example
//!
//! ```rust
//! use infra_master::market::{CurrencyPair, Currency};
//!
//! let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
//! assert_eq!(pair.to_string(), "EUR/USD");
//!
//! let inverse = pair.inverse();
//! assert_eq!(inverse.to_string(), "USD/EUR");
//! ```

use super::currency::Currency;

/// Currency pair representation.
///
/// Represents a pair of currencies for FX transactions.
/// Convention: Base/Quote, e.g., EUR/USD means EUR is base, USD is quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CurrencyPair {
    /// Base currency (first in the pair).
    pub base: Currency,
    /// Quote currency (second in the pair).
    pub quote: Currency,
}

impl CurrencyPair {
    /// Creates a new currency pair.
    #[must_use]
    pub fn new(base: Currency, quote: Currency) -> Self {
        Self { base, quote }
    }

    /// Returns the inverse currency pair.
    #[must_use]
    pub fn inverse(&self) -> Self {
        Self {
            base: self.quote,
            quote: self.base,
        }
    }

    /// Returns the pair as a string (e.g., "EUR/USD").
    #[must_use]
    pub fn to_string_pair(&self) -> String {
        format!("{}/{}", self.base.code(), self.quote.code())
    }
}

impl std::fmt::Display for CurrencyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.base.code(), self.quote.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_currency_pair() -> CurrencyPair {
        CurrencyPair::new(Currency::EUR, Currency::USD)
    }

    #[test]
    fn test_currency_pair_new() {
        let pair = make_test_currency_pair();
        assert_eq!(pair.base, Currency::EUR);
        assert_eq!(pair.quote, Currency::USD);
    }

    #[test]
    fn test_currency_pair_inverse() {
        let pair = make_test_currency_pair();
        let inverse = pair.inverse();
        assert_eq!(inverse.base, Currency::USD);
        assert_eq!(inverse.quote, Currency::EUR);
    }

    #[test]
    fn test_currency_pair_display() {
        let pair = make_test_currency_pair();
        assert_eq!(pair.to_string(), "EUR/USD");
    }

    #[test]
    fn test_currency_pair_to_string_pair() {
        let pair = make_test_currency_pair();
        assert_eq!(pair.to_string_pair(), "EUR/USD");
    }

    #[test]
    fn test_currency_pair_equality() {
        let pair1 = CurrencyPair::new(Currency::EUR, Currency::USD);
        let pair2 = CurrencyPair::new(Currency::EUR, Currency::USD);
        let pair3 = CurrencyPair::new(Currency::USD, Currency::EUR);

        assert_eq!(pair1, pair2);
        assert_ne!(pair1, pair3);
    }

    #[test]
    fn test_currency_pair_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(CurrencyPair::new(Currency::EUR, Currency::USD));
        set.insert(CurrencyPair::new(Currency::EUR, Currency::USD)); // duplicate
        set.insert(CurrencyPair::new(Currency::USD, Currency::JPY));

        assert_eq!(set.len(), 2);
    }
}
