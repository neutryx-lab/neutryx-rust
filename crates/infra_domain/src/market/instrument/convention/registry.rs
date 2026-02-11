//! Convention registry for looking up market conventions.
//!
//! This module provides the [`ConventionRegistry`] type for managing
//! and looking up market conventions by currency and rate type.

use std::collections::HashMap;

use super::MarketConvention;
use crate::market::{Currency, RateType};

/// A key for looking up conventions in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConventionKey {
    /// Currency of the convention.
    pub currency: Currency,
    /// Rate type of the convention.
    pub rate_type: RateType,
}

impl ConventionKey {
    /// Creates a new convention key.
    #[must_use]
    pub fn new(currency: Currency, rate_type: RateType) -> Self {
        Self {
            currency,
            rate_type,
        }
    }
}

impl std::fmt::Display for ConventionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.currency.code(), self.rate_type.code())
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
///
/// Provides O(1) lookup of conventions by (Currency, RateType) pairs.
/// Can be populated programmatically or from JSON configuration.
///
/// # Example
///
/// ```rust
/// use infra_domain::market::convention::{
///     ConventionRegistry, MarketConvention, DepositConvention,
/// };
/// use infra_domain::market::{Currency, RateType};
///
/// let mut registry = ConventionRegistry::new();
/// registry.register(
///     Currency::USD,
///     RateType::Deposit,
///     MarketConvention::Deposit(DepositConvention::usd()),
/// );
///
/// let convention = registry.get(Currency::USD, RateType::Deposit);
/// assert!(convention.is_some());
/// ```
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
    ///
    /// Includes conventions for:
    /// - USD, EUR, GBP, JPY, CHF
    /// - Deposit, Swap, OIS, FRA, Futures rate types
    ///
    /// # Example
    ///
    /// ```rust
    /// use infra_domain::market::convention::ConventionRegistry;
    /// use infra_domain::market::{Currency, RateType};
    ///
    /// let registry = ConventionRegistry::with_defaults();
    /// assert!(registry.get(Currency::USD, RateType::Deposit).is_some());
    /// ```
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Register all default conventions using MarketConvention::for_rate_id
        let currencies = [
            Currency::USD,
            Currency::EUR,
            Currency::GBP,
            Currency::JPY,
            Currency::CHF,
        ];
        let rate_types = [
            RateType::Deposit,
            RateType::Swap,
            RateType::Ois,
            RateType::Fra,
            RateType::Futures,
            RateType::FxForward,
        ];

        for currency in currencies {
            for rate_type in rate_types {
                let quote_id = crate::market::QuoteId::new(
                    currency,
                    crate::time::Tenor::OneYear, // Tenor doesn't affect convention selection
                    rate_type,
                );
                if let Some(convention) = MarketConvention::for_quote_id(&quote_id) {
                    registry.register(currency, rate_type, convention);
                }
            }
        }

        registry
    }

    /// Registers a convention in the registry.
    ///
    /// If a convention already exists for the given key, it is replaced.
    ///
    /// # Arguments
    ///
    /// * `currency` - The currency
    /// * `rate_type` - The rate type
    /// * `convention` - The convention to register
    pub fn register(
        &mut self,
        currency: Currency,
        rate_type: RateType,
        convention: MarketConvention,
    ) {
        let key = ConventionKey::new(currency, rate_type);
        self.conventions.insert(key, convention);
    }

    /// Gets a convention from the registry.
    ///
    /// Returns `None` if no convention is registered for the given key.
    ///
    /// # Arguments
    ///
    /// * `currency` - The currency
    /// * `rate_type` - The rate type
    ///
    /// # Example
    ///
    /// ```rust
    /// use infra_domain::market::convention::ConventionRegistry;
    /// use infra_domain::market::{Currency, RateType};
    ///
    /// let registry = ConventionRegistry::with_defaults();
    ///
    /// // USD Deposit exists
    /// assert!(registry.get(Currency::USD, RateType::Deposit).is_some());
    ///
    /// // Vol doesn't have a convention
    /// assert!(registry.get(Currency::USD, RateType::Vol).is_none());
    /// ```
    #[must_use]
    pub fn get(&self, currency: Currency, rate_type: RateType) -> Option<&MarketConvention> {
        let key = ConventionKey::new(currency, rate_type);
        self.conventions.get(&key)
    }

    /// Gets a convention using a key.
    #[must_use]
    pub fn get_by_key(&self, key: &ConventionKey) -> Option<&MarketConvention> {
        self.conventions.get(key)
    }

    /// Returns true if the registry contains a convention for the given key.
    #[must_use]
    pub fn contains(&self, currency: Currency, rate_type: RateType) -> bool {
        let key = ConventionKey::new(currency, rate_type);
        self.conventions.contains_key(&key)
    }

    /// Returns an iterator over all registered keys.
    ///
    /// # Example
    ///
    /// ```rust
    /// use infra_domain::market::convention::ConventionRegistry;
    ///
    /// let registry = ConventionRegistry::with_defaults();
    /// for key in registry.keys() {
    ///     println!("{}", key);
    /// }
    /// ```
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
    ///
    /// Returns the removed convention, or `None` if it wasn't registered.
    pub fn remove(&mut self, currency: Currency, rate_type: RateType) -> Option<MarketConvention> {
        let key = ConventionKey::new(currency, rate_type);
        self.conventions.remove(&key)
    }

    /// Clears all conventions from the registry.
    pub fn clear(&mut self) { self.conventions.clear(); }

    /// Returns all currencies that have at least one registered convention.
    #[must_use]
    pub fn currencies(&self) -> Vec<Currency> {
        let mut currencies: Vec<Currency> = self.conventions.keys().map(|k| k.currency).collect();
        currencies.sort_by_key(|c| c.code());
        currencies.dedup();
        currencies
    }

    /// Returns all rate types that have at least one registered convention.
    #[must_use]
    pub fn rate_types(&self) -> Vec<RateType> {
        let mut rate_types: Vec<RateType> = self.conventions.keys().map(|k| k.rate_type).collect();
        rate_types.sort_by_key(|rt| rt.code());
        rate_types.dedup();
        rate_types
    }

    /// Returns all conventions for a given currency.
    pub fn conventions_for_currency(
        &self,
        currency: Currency,
    ) -> impl Iterator<Item = (&RateType, &MarketConvention)> {
        self.conventions
            .iter()
            .filter(move |(k, _)| k.currency == currency)
            .map(|(k, v)| (&k.rate_type, v))
    }

    /// Returns all conventions for a given rate type.
    pub fn conventions_for_rate_type(
        &self,
        rate_type: RateType,
    ) -> impl Iterator<Item = (&Currency, &MarketConvention)> {
        self.conventions
            .iter()
            .filter(move |(k, _)| k.rate_type == rate_type)
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
        registry.register(Currency::USD, RateType::Deposit, conv.clone());

        assert_eq!(registry.len(), 1);
        assert!(registry.contains(Currency::USD, RateType::Deposit));
        assert!(!registry.contains(Currency::EUR, RateType::Deposit));
        assert_eq!(
            registry.get(Currency::USD, RateType::Deposit).unwrap(),
            &conv
        );
    }

    #[test]
    fn test_with_defaults() {
        let registry = ConventionRegistry::with_defaults();
        assert!(registry.len() > 10);
        assert!(registry.get(Currency::USD, RateType::Deposit).is_some());
        assert!(registry.get(Currency::EUR, RateType::Swap).is_some());
        assert!(registry.get(Currency::USD, RateType::Vol).is_none());

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

        let removed = registry.remove(Currency::USD, RateType::Deposit);
        assert!(removed.is_some());
        assert_eq!(registry.len(), initial_len - 1);

        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_convention_key() {
        let key1 = ConventionKey::new(Currency::USD, RateType::Swap);
        let key2 = ConventionKey::new(Currency::USD, RateType::Swap);
        let key3 = ConventionKey::new(Currency::EUR, RateType::Deposit);
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert_eq!(key3.to_string(), "EUR DEPO");
    }
}
