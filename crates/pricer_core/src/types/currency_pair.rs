//! Currency pair types for FX calculations.
//!
//! This module provides currency pair structures for foreign exchange
//! derivative pricing and risk management.
//!
//! # Examples
//!
//! ```
//! use pricer_core::types::{Currency, CurrencyPair};
//!
//! // Create a USD/JPY currency pair
//! let pair = CurrencyPair::new(Currency::USD, Currency::JPY, 150.0).unwrap();
//! assert_eq!(pair.base(), Currency::USD);
//! assert_eq!(pair.quote(), Currency::JPY);
//! assert_eq!(pair.spot(), 150.0);
//!
//! // Invert the pair (JPY/USD)
//! let inverted = pair.invert();
//! assert_eq!(inverted.base(), Currency::JPY);
//! assert_eq!(inverted.quote(), Currency::USD);
//! ```

use num_traits::Float;
use std::fmt;

use super::currency::Currency;
use super::error::CurrencyError;

/// A currency pair for foreign exchange calculations.
///
/// Represents a pair of currencies with a spot exchange rate.
/// The convention is BASE/QUOTE, meaning 1 unit of BASE = spot units of QUOTE.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Examples
///
/// ```
/// use pricer_core::types::{Currency, CurrencyPair};
///
/// // EUR/USD = 1.10 means 1 EUR = 1.10 USD
/// let eurusd = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
/// assert_eq!(eurusd.base(), Currency::EUR);
/// assert_eq!(eurusd.quote(), Currency::USD);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct CurrencyPair<T: Float> {
    /// Base currency (the numerator in the exchange rate)
    base: Currency,
    /// Quote currency (the denominator in the exchange rate)
    quote: Currency,
    /// Spot exchange rate: 1 unit of base = spot units of quote
    spot: T,
}

impl<T: Float> CurrencyPair<T> {
    /// Creates a new currency pair.
    ///
    /// # Arguments
    ///
    /// * `base` - The base currency
    /// * `quote` - The quote currency
    /// * `spot` - The spot exchange rate (must be positive)
    ///
    /// # Errors
    ///
    /// Returns `CurrencyError::InvalidSpotRate` if spot is not positive.
    /// Returns `CurrencyError::SameCurrency` if base and quote are the same.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::{Currency, CurrencyPair};
    ///
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert_eq!(pair.spot(), 1.10);
    /// ```
    pub fn new(base: Currency, quote: Currency, spot: T) -> Result<Self, CurrencyError> {
        if base == quote {
            return Err(CurrencyError::SameCurrency(base.code().to_string()));
        }
        if spot <= T::zero() {
            return Err(CurrencyError::InvalidSpotRate);
        }
        Ok(Self { base, quote, spot })
    }

    /// Returns the base currency.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::{Currency, CurrencyPair};
    ///
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert_eq!(pair.base(), Currency::EUR);
    /// ```
    #[inline]
    pub fn base(&self) -> Currency {
        self.base
    }

    /// Returns the quote currency.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::{Currency, CurrencyPair};
    ///
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert_eq!(pair.quote(), Currency::USD);
    /// ```
    #[inline]
    pub fn quote(&self) -> Currency {
        self.quote
    }

    /// Returns the spot exchange rate.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::{Currency, CurrencyPair};
    ///
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert_eq!(pair.spot(), 1.10);
    /// ```
    #[inline]
    pub fn spot(&self) -> T {
        self.spot
    }

    /// Returns the currency pair code in standard format (BASE/QUOTE).
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::{Currency, CurrencyPair};
    ///
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert_eq!(pair.code(), "EUR/USD");
    /// ```
    pub fn code(&self) -> String {
        format!("{}/{}", self.base.code(), self.quote.code())
    }

    /// Updates the spot rate.
    ///
    /// # Arguments
    ///
    /// * `new_spot` - The new spot exchange rate (must be positive)
    ///
    /// # Errors
    ///
    /// Returns `CurrencyError::InvalidSpotRate` if new_spot is not positive.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::{Currency, CurrencyPair};
    ///
    /// let mut pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// pair.set_spot(1.15).unwrap();
    /// assert_eq!(pair.spot(), 1.15);
    /// ```
    pub fn set_spot(&mut self, new_spot: T) -> Result<(), CurrencyError> {
        if new_spot <= T::zero() {
            return Err(CurrencyError::InvalidSpotRate);
        }
        self.spot = new_spot;
        Ok(())
    }

    /// Creates an inverted currency pair (swaps base and quote).
    ///
    /// The spot rate is inverted: new_spot = 1 / old_spot.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::{Currency, CurrencyPair};
    ///
    /// let eurusd = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// let usdeur = eurusd.invert();
    ///
    /// assert_eq!(usdeur.base(), Currency::USD);
    /// assert_eq!(usdeur.quote(), Currency::EUR);
    /// assert!((usdeur.spot() - 1.0 / 1.10).abs() < 1e-10);
    /// ```
    pub fn invert(&self) -> Self {
        Self {
            base: self.quote,
            quote: self.base,
            spot: T::one() / self.spot,
        }
    }

    /// Converts an amount from base currency to quote currency.
    ///
    /// # Arguments
    ///
    /// * `base_amount` - Amount in base currency
    ///
    /// # Returns
    ///
    /// Amount in quote currency.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::{Currency, CurrencyPair};
    ///
    /// // EUR/USD = 1.10
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    ///
    /// // 100 EUR = 110 USD
    /// let usd_amount = pair.convert_to_quote(100.0);
    /// assert!((usd_amount - 110.0).abs() < 1e-10);
    /// ```
    #[inline]
    pub fn convert_to_quote(&self, base_amount: T) -> T {
        base_amount * self.spot
    }

    /// Converts an amount from quote currency to base currency.
    ///
    /// # Arguments
    ///
    /// * `quote_amount` - Amount in quote currency
    ///
    /// # Returns
    ///
    /// Amount in base currency.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::{Currency, CurrencyPair};
    ///
    /// // EUR/USD = 1.10
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    ///
    /// // 110 USD = 100 EUR
    /// let eur_amount = pair.convert_to_base(110.0);
    /// assert!((eur_amount - 100.0).abs() < 1e-10);
    /// ```
    #[inline]
    pub fn convert_to_base(&self, quote_amount: T) -> T {
        quote_amount / self.spot
    }

    /// Checks if this pair contains the given currency.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::{Currency, CurrencyPair};
    ///
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert!(pair.contains(Currency::EUR));
    /// assert!(pair.contains(Currency::USD));
    /// assert!(!pair.contains(Currency::JPY));
    /// ```
    #[inline]
    pub fn contains(&self, currency: Currency) -> bool {
        self.base == currency || self.quote == currency
    }
}

impl<T: Float + std::fmt::Display> fmt::Display for CurrencyPair<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} = {}", self.code(), self.spot)
    }
}

impl<T: Float> PartialEq for CurrencyPair<T> {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base && self.quote == other.quote
    }
}

impl<T: Float> Eq for CurrencyPair<T> {}

impl<T: Float> std::hash::Hash for CurrencyPair<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.hash(state);
        self.quote.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_pair_new() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        assert_eq!(pair.base(), Currency::EUR);
        assert_eq!(pair.quote(), Currency::USD);
        assert!((pair.spot() - 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_currency_pair_code() {
        let pair = CurrencyPair::new(Currency::USD, Currency::JPY, 150.0).unwrap();
        assert_eq!(pair.code(), "USD/JPY");
    }

    #[test]
    fn test_currency_pair_same_currency_error() {
        let result = CurrencyPair::new(Currency::USD, Currency::USD, 1.0);
        assert!(result.is_err());
        match result {
            Err(CurrencyError::SameCurrency(code)) => assert_eq!(code, "USD"),
            _ => panic!("Expected SameCurrency error"),
        }
    }

    #[test]
    fn test_currency_pair_invalid_spot_error() {
        let result = CurrencyPair::new(Currency::EUR, Currency::USD, 0.0);
        assert!(result.is_err());
        match result {
            Err(CurrencyError::InvalidSpotRate) => {}
            _ => panic!("Expected InvalidSpotRate error"),
        }

        let result = CurrencyPair::new(Currency::EUR, Currency::USD, -1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_currency_pair_set_spot() {
        let mut pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        pair.set_spot(1.15).unwrap();
        assert!((pair.spot() - 1.15).abs() < 1e-10);
    }

    #[test]
    fn test_currency_pair_set_spot_invalid() {
        let mut pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let result = pair.set_spot(0.0);
        assert!(result.is_err());
        // Original spot should be unchanged
        assert!((pair.spot() - 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_currency_pair_invert() {
        let eurusd = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let usdeur = eurusd.invert();

        assert_eq!(usdeur.base(), Currency::USD);
        assert_eq!(usdeur.quote(), Currency::EUR);
        assert!((usdeur.spot() - 1.0 / 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_currency_pair_convert_to_quote() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let usd = pair.convert_to_quote(100.0);
        assert!((usd - 110.0).abs() < 1e-10);
    }

    #[test]
    fn test_currency_pair_convert_to_base() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let eur = pair.convert_to_base(110.0);
        assert!((eur - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_currency_pair_convert_roundtrip() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let original = 100.0;
        let converted = pair.convert_to_quote(original);
        let back = pair.convert_to_base(converted);
        assert!((back - original).abs() < 1e-10);
    }

    #[test]
    fn test_currency_pair_contains() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        assert!(pair.contains(Currency::EUR));
        assert!(pair.contains(Currency::USD));
        assert!(!pair.contains(Currency::JPY));
        assert!(!pair.contains(Currency::GBP));
    }

    #[test]
    fn test_currency_pair_equality() {
        let pair1 = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let pair2 = CurrencyPair::new(Currency::EUR, Currency::USD, 1.20).unwrap();
        let pair3 = CurrencyPair::new(Currency::USD, Currency::EUR, 0.91).unwrap();

        // Same currencies = equal (spot rate doesn't affect equality)
        assert_eq!(pair1, pair2);
        // Different order = not equal
        assert_ne!(pair1, pair3);
    }

    #[test]
    fn test_currency_pair_hash() {
        use std::collections::HashSet;
        let pair1 = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let pair2 = CurrencyPair::new(Currency::EUR, Currency::USD, 1.20).unwrap();
        let pair3 = CurrencyPair::new(Currency::USD, Currency::JPY, 150.0).unwrap();

        let mut set = HashSet::new();
        set.insert(pair1);
        set.insert(pair2); // Same as pair1
        set.insert(pair3);

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_currency_pair_clone() {
        let pair1 = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let pair2 = pair1;
        assert_eq!(pair1, pair2);
    }

    #[test]
    fn test_currency_pair_display() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let display = format!("{}", pair);
        assert!(display.contains("EUR/USD"));
        assert!(display.contains("1.1"));
    }

    #[test]
    fn test_currency_pair_debug() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let debug_str = format!("{:?}", pair);
        assert!(debug_str.contains("CurrencyPair"));
        assert!(debug_str.contains("EUR"));
        assert!(debug_str.contains("USD"));
    }

    #[test]
    fn test_usdjpy_pair() {
        let pair = CurrencyPair::new(Currency::USD, Currency::JPY, 150.0).unwrap();
        assert_eq!(pair.code(), "USD/JPY");

        // 1000 USD = 150,000 JPY
        let jpy = pair.convert_to_quote(1000.0);
        assert!((jpy - 150000.0).abs() < 1e-10);

        // 150,000 JPY = 1000 USD
        let usd = pair.convert_to_base(150000.0);
        assert!((usd - 1000.0).abs() < 1e-10);
    }
}
