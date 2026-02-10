//! Currency pair representation for FX markets.
//!
//! This module provides the fundamental [`CurrencyPair`] type used throughout
//! the system for representing FX currency pairs.
//!
//! # Example
//!
//! ```rust
//! use infra_domain::market::{CurrencyPair, Currency};
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
    pub fn new(base: Currency, quote: Currency) -> Self { Self { base, quote } }

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
    pub fn to_string_pair(&self) -> String { format!("{}/{}", self.base.code(), self.quote.code()) }
}

impl std::fmt::Display for CurrencyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.base.code(), self.quote.code())
    }
}
