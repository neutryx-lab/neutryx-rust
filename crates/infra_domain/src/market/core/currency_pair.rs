//! Currency pair representation for FX markets.

use super::currency::Currency;

/// Currency pair representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
