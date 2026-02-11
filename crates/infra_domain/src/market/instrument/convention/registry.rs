//! Convention registry for looking up market conventions.

use std::collections::HashMap;

use super::MarketConvention;
use crate::market::{Currency, QuoteCategory};

/// A key for looking up conventions in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConventionKey {
    /// Currency of the convention.
    pub currency: Currency,
    /// Quote category of the convention.
    pub quote_category: QuoteCategory,
}

impl ConventionKey {
    /// Creates a new convention key.
    #[must_use]
    pub fn new(currency: Currency, quote_category: QuoteCategory) -> Self {
        Self {
            currency,
            quote_category,
        }
    }
}

impl std::fmt::Display for ConventionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.currency.code(), self.quote_category.code())
    }
}

/// Errors that can occur when working with the convention registry.
#[derive(Debug, Clone, PartialEq)]
pub enum RegistryError {
    /// JSON parsing failed.
    ParseError(String),
    /// Invalid convention data.
    InvalidConvention {
        /// Key that had the invalid convention.
        key: String,
        /// Reason for invalidity.
        reason: String,
    },
    /// Duplicate key in registry.
    DuplicateKey(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            RegistryError::InvalidConvention { key, reason } => {
                write!(f, "Invalid convention for {}: {}", key, reason)
            }
            RegistryError::DuplicateKey(key) => write!(f, "Duplicate key: {}", key),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Registry for market conventions.
#[derive(Debug, Clone, Default)]
pub struct ConventionRegistry {
    conventions: HashMap<ConventionKey, MarketConvention>,
}

impl ConventionRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            conventions: HashMap::new(),
        }
    }

    /// Creates a registry with standard conventions for major currencies.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        let currencies = [
            Currency::USD,
            Currency::EUR,
            Currency::GBP,
            Currency::JPY,
            Currency::CHF,
        ];
        let quote_categories = [
            QuoteCategory::Deposit,
            QuoteCategory::Swap,
            QuoteCategory::Ois,
            QuoteCategory::Fra,
            QuoteCategory::Futures,
            QuoteCategory::FxForward,
        ];

        for currency in currencies {
            for quote_category in quote_categories {
                let quote_id = crate::market::QuoteId::new(
                    currency,
                    crate::time::Tenor::OneYear,
                    quote_category,
                );
                if let Some(convention) = MarketConvention::for_quote_id(&quote_id) {
                    registry.register(currency, quote_category, convention);
                }
            }
        }

        registry
    }

    /// Registers a convention in the registry.
    pub fn register(
        &mut self,
        currency: Currency,
        quote_category: QuoteCategory,
        convention: MarketConvention,
    ) {
        let key = ConventionKey::new(currency, quote_category);
        self.conventions.insert(key, convention);
    }

    /// Gets a convention from the registry.
    #[must_use]
    pub fn get(
        &self,
        currency: Currency,
        quote_category: QuoteCategory,
    ) -> Option<&MarketConvention> {
        let key = ConventionKey::new(currency, quote_category);
        self.conventions.get(&key)
    }

    /// Gets a convention using a key.
    #[must_use]
    pub fn get_by_key(&self, key: &ConventionKey) -> Option<&MarketConvention> {
        self.conventions.get(key)
    }

    /// Returns true if the registry contains a convention for the given key.
    #[must_use]
    pub fn contains(&self, currency: Currency, quote_category: QuoteCategory) -> bool {
        let key = ConventionKey::new(currency, quote_category);
        self.conventions.contains_key(&key)
    }

    /// Returns an iterator over all registered keys.
    pub fn keys(&self) -> impl Iterator<Item = &ConventionKey> { self.conventions.keys() }

    /// Returns an iterator over all registered conventions.
    pub fn values(&self) -> impl Iterator<Item = &MarketConvention> { self.conventions.values() }

    /// Returns an iterator over all (key, convention) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&ConventionKey, &MarketConvention)> {
        self.conventions.iter()
    }

    /// Returns the number of registered conventions.
    #[must_use]
    pub fn len(&self) -> usize { self.conventions.len() }

    /// Returns true if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.conventions.is_empty() }

    /// Removes a convention from the registry.
    pub fn remove(
        &mut self,
        currency: Currency,
        quote_category: QuoteCategory,
    ) -> Option<MarketConvention> {
        let key = ConventionKey::new(currency, quote_category);
        self.conventions.remove(&key)
    }

    /// Clears all conventions from the registry.
    pub fn clear(&mut self) { self.conventions.clear(); }

    /// Returns all currencies that have at least one registered convention.
    #[must_use]
    pub fn currencies(&self) -> Vec<Currency> {
        let mut currencies: Vec<Currency> = self.conventions.keys().map(|k| k.currency).collect();
        currencies.sort_by(|a, b| a.code().cmp(b.code()));
        currencies.dedup();
        currencies
    }

    /// Returns all quote categories that have at least one registered
    /// convention.
    #[must_use]
    pub fn quote_categories(&self) -> Vec<QuoteCategory> {
        let mut quote_categories: Vec<QuoteCategory> =
            self.conventions.keys().map(|k| k.quote_category).collect();
        quote_categories.sort_by_key(|rt| rt.code());
        quote_categories.dedup();
        quote_categories
    }

    /// Returns all conventions for a given currency.
    pub fn conventions_for_currency(
        &self,
        currency: Currency,
    ) -> impl Iterator<Item = (&QuoteCategory, &MarketConvention)> {
        self.conventions
            .iter()
            .filter(move |(k, _)| k.currency == currency)
            .map(|(k, v)| (&k.quote_category, v))
    }

    /// Returns all conventions for a given quote category.
    pub fn conventions_for_quote_category(
        &self,
        quote_category: QuoteCategory,
    ) -> impl Iterator<Item = (&Currency, &MarketConvention)> {
        self.conventions
            .iter()
            .filter(move |(k, _)| k.quote_category == quote_category)
            .map(|(k, v)| (&k.currency, v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::convention::DepositConvention;

    #[test]
    fn test_register_and_get() {
        let mut registry = ConventionRegistry::new();
        assert!(registry.is_empty());

        let conv = MarketConvention::Deposit(DepositConvention::usd());
        registry.register(Currency::USD, QuoteCategory::Deposit, conv.clone());

        assert_eq!(registry.len(), 1);
        assert!(registry.contains(Currency::USD, QuoteCategory::Deposit));
        assert!(!registry.contains(Currency::EUR, QuoteCategory::Deposit));
        assert_eq!(
            registry.get(Currency::USD, QuoteCategory::Deposit).unwrap(),
            &conv
        );
    }

    #[test]
    fn test_with_defaults() {
        let registry = ConventionRegistry::with_defaults();
        assert!(registry.len() > 10);
        assert!(registry
            .get(Currency::USD, QuoteCategory::Deposit)
            .is_some());
        assert!(registry.get(Currency::EUR, QuoteCategory::Swap).is_some());
        assert!(registry.get(Currency::USD, QuoteCategory::Vol).is_none());

        let currencies = registry.currencies();
        assert!(currencies.contains(&Currency::USD));
        assert!(currencies.contains(&Currency::EUR));

        let usd_convs: Vec<_> = registry.conventions_for_currency(Currency::USD).collect();
        assert!(usd_convs.len() >= 3);
    }

    #[test]
    fn test_remove_and_clear() {
        let mut registry = ConventionRegistry::with_defaults();
        let initial_len = registry.len();

        let removed = registry.remove(Currency::USD, QuoteCategory::Deposit);
        assert!(removed.is_some());
        assert_eq!(registry.len(), initial_len - 1);

        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_convention_key() {
        let key1 = ConventionKey::new(Currency::USD, QuoteCategory::Swap);
        let key2 = ConventionKey::new(Currency::USD, QuoteCategory::Swap);
        let key3 = ConventionKey::new(Currency::EUR, QuoteCategory::Deposit);
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert_eq!(key3.to_string(), "EUR DEPO");
    }
}
