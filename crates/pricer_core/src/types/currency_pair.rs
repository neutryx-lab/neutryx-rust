//! FX rate types for foreign exchange calculations.
//!
//! This module provides FX rate structures for foreign exchange
//! derivative pricing and risk management.
//!
//! # Note
//!
//! `FxRate<T>` is distinct from `infra_master::CurrencyPair`:
//! - `infra_master::CurrencyPair`: Instrument definition (no spot rate, no AD
//!   support)
//! - `FxRate<T>`: Pricing type (includes spot rate, supports AD via generic
//!   `T`)
//!
//! # Examples
//!
//! ```
//! use infra_master::Currency;
//! use pricer_core::types::FxRate;
//!
//! // Create a USD/JPY FX rate
//! let rate = FxRate::new(Currency::USD, Currency::JPY, 150.0).unwrap();
//! assert_eq!(rate.base(), Currency::USD);
//! assert_eq!(rate.quote(), Currency::JPY);
//! assert_eq!(rate.spot(), 150.0);
//!
//! // Invert the rate (JPY/USD)
//! let inverted = rate.invert();
//! assert_eq!(inverted.base(), Currency::JPY);
//! assert_eq!(inverted.quote(), Currency::USD);
//! ```

use std::fmt;

use infra_master::{Currency, CurrencyError};
use num_traits::Float;

/// An FX rate for foreign exchange calculations.
///
/// Represents a pair of currencies with a spot exchange rate.
/// The convention is BASE/QUOTE, meaning 1 unit of BASE = spot units of QUOTE.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Distinction from `infra_master::CurrencyPair`
///
/// - `infra_master::CurrencyPair`: Instrument definition (static, no spot rate)
/// - `FxRate<T>`: Pricing type (dynamic spot rate, AD-compatible)
///
/// # Examples
///
/// ```
/// use infra_master::Currency;
/// use pricer_core::types::FxRate;
///
/// // EUR/USD = 1.10 means 1 EUR = 1.10 USD
/// let eurusd = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
/// assert_eq!(eurusd.base(), Currency::EUR);
/// assert_eq!(eurusd.quote(), Currency::USD);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct FxRate<T: Float> {
    /// Base currency (the numerator in the exchange rate)
    base: Currency,
    /// Quote currency (the denominator in the exchange rate)
    quote: Currency,
    /// Spot exchange rate: 1 unit of base = spot units of quote
    spot: T,
}

impl<T: Float> FxRate<T> {
    /// Creates a new FX rate.
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
    /// use infra_master::Currency;
    /// use pricer_core::types::FxRate;
    ///
    /// let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert_eq!(rate.spot(), 1.10);
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
    /// use infra_master::Currency;
    /// use pricer_core::types::FxRate;
    ///
    /// let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert_eq!(rate.base(), Currency::EUR);
    /// ```
    #[inline]
    pub fn base(&self) -> Currency { self.base }

    /// Returns the quote currency.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::Currency;
    /// use pricer_core::types::FxRate;
    ///
    /// let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert_eq!(rate.quote(), Currency::USD);
    /// ```
    #[inline]
    pub fn quote(&self) -> Currency { self.quote }

    /// Returns the spot exchange rate.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::Currency;
    /// use pricer_core::types::FxRate;
    ///
    /// let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert_eq!(rate.spot(), 1.10);
    /// ```
    #[inline]
    pub fn spot(&self) -> T { self.spot }

    /// Returns the currency pair code in standard format (BASE/QUOTE).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::Currency;
    /// use pricer_core::types::FxRate;
    ///
    /// let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert_eq!(rate.code(), "EUR/USD");
    /// ```
    pub fn code(&self) -> String { format!("{}/{}", self.base.code(), self.quote.code()) }

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
    /// use infra_master::Currency;
    /// use pricer_core::types::FxRate;
    ///
    /// let mut rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// rate.set_spot(1.15).unwrap();
    /// assert_eq!(rate.spot(), 1.15);
    /// ```
    pub fn set_spot(&mut self, new_spot: T) -> Result<(), CurrencyError> {
        if new_spot <= T::zero() {
            return Err(CurrencyError::InvalidSpotRate);
        }
        self.spot = new_spot;
        Ok(())
    }

    /// Creates an inverted FX rate (swaps base and quote).
    ///
    /// The spot rate is inverted: new_spot = 1 / old_spot.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::Currency;
    /// use pricer_core::types::FxRate;
    ///
    /// let eurusd: FxRate<f64> = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// let usdeur = eurusd.invert();
    ///
    /// assert_eq!(usdeur.base(), Currency::USD);
    /// assert_eq!(usdeur.quote(), Currency::EUR);
    /// assert!((usdeur.spot() - 1.0_f64 / 1.10_f64).abs() < 1e-10);
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
    /// use infra_master::Currency;
    /// use pricer_core::types::FxRate;
    ///
    /// // EUR/USD = 1.10
    /// let rate: FxRate<f64> = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    ///
    /// // 100 EUR = 110 USD
    /// let usd_amount = rate.convert_to_quote(100.0_f64);
    /// assert!((usd_amount - 110.0_f64).abs() < 1e-10);
    /// ```
    #[inline]
    pub fn convert_to_quote(&self, base_amount: T) -> T { base_amount * self.spot }

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
    /// use infra_master::Currency;
    /// use pricer_core::types::FxRate;
    ///
    /// // EUR/USD = 1.10
    /// let rate: FxRate<f64> = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    ///
    /// // 110 USD = 100 EUR
    /// let eur_amount = rate.convert_to_base(110.0_f64);
    /// assert!((eur_amount - 100.0_f64).abs() < 1e-10);
    /// ```
    #[inline]
    pub fn convert_to_base(&self, quote_amount: T) -> T { quote_amount / self.spot }

    /// Checks if this rate contains the given currency.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::Currency;
    /// use pricer_core::types::FxRate;
    ///
    /// let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    /// assert!(rate.contains(Currency::EUR));
    /// assert!(rate.contains(Currency::USD));
    /// assert!(!rate.contains(Currency::JPY));
    /// ```
    #[inline]
    pub fn contains(&self, currency: Currency) -> bool {
        self.base == currency || self.quote == currency
    }
}

impl<T: Float + std::fmt::Display> fmt::Display for FxRate<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} = {}", self.code(), self.spot)
    }
}

impl<T: Float> PartialEq for FxRate<T> {
    fn eq(&self, other: &Self) -> bool { self.base == other.base && self.quote == other.quote }
}

impl<T: Float> Eq for FxRate<T> {}

impl<T: Float> std::hash::Hash for FxRate<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.hash(state);
        self.quote.hash(state);
    }
}

/// Deprecated type alias for backward compatibility.
#[deprecated(since = "0.9.0", note = "renamed to FxRate")]
pub type CurrencyPair<T> = FxRate<T>;

/// A simple FX pair identifier (without spot rate).
///
/// Used as a key for FX index mapping in `IndexMapper`. Unlike `FxRate<T>`,
/// this type doesn't carry a spot rate, making it suitable for use as
/// a HashMap key or index identifier.
///
/// # Convention
///
/// BASE/QUOTE means converting from BASE to QUOTE currency.
/// For example, EUR/USD means 1 EUR = X USD.
///
/// # Examples
///
/// ```
/// use infra_master::Currency;
/// use pricer_core::types::FxPair;
///
/// let eurusd = FxPair::new(Currency::EUR, Currency::USD);
/// assert_eq!(eurusd.base(), Currency::EUR);
/// assert_eq!(eurusd.quote(), Currency::USD);
/// assert_eq!(eurusd.code(), "EUR/USD");
///
/// // EUR/USD and USD/EUR are different pairs
/// let usdeur = FxPair::new(Currency::USD, Currency::EUR);
/// assert_ne!(eurusd, usdeur);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FxPair {
    /// Base currency (the numerator in the exchange rate)
    base: Currency,
    /// Quote currency (the denominator in the exchange rate)
    quote: Currency,
}

impl FxPair {
    /// Creates a new FX pair.
    ///
    /// # Arguments
    ///
    /// * `base` - The base currency
    /// * `quote` - The quote currency
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::Currency;
    /// use pricer_core::types::FxPair;
    ///
    /// let pair = FxPair::new(Currency::EUR, Currency::USD);
    /// assert_eq!(pair.base(), Currency::EUR);
    /// ```
    #[must_use]
    pub fn new(base: Currency, quote: Currency) -> Self {
        Self { base, quote }
    }

    /// Returns the base currency.
    #[inline]
    #[must_use]
    pub fn base(&self) -> Currency {
        self.base
    }

    /// Returns the quote currency.
    #[inline]
    #[must_use]
    pub fn quote(&self) -> Currency {
        self.quote
    }

    /// Returns the currency pair code in standard format (BASE/QUOTE).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::Currency;
    /// use pricer_core::types::FxPair;
    ///
    /// let pair = FxPair::new(Currency::EUR, Currency::USD);
    /// assert_eq!(pair.code(), "EUR/USD");
    /// ```
    #[must_use]
    pub fn code(&self) -> String {
        format!("{}/{}", self.base.code(), self.quote.code())
    }

    /// Creates an inverted FX pair (swaps base and quote).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::Currency;
    /// use pricer_core::types::FxPair;
    ///
    /// let eurusd = FxPair::new(Currency::EUR, Currency::USD);
    /// let usdeur = eurusd.invert();
    /// assert_eq!(usdeur.base(), Currency::USD);
    /// assert_eq!(usdeur.quote(), Currency::EUR);
    /// ```
    #[must_use]
    pub fn invert(&self) -> Self {
        Self {
            base: self.quote,
            quote: self.base,
        }
    }

    /// Checks if this pair contains the given currency.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::Currency;
    /// use pricer_core::types::FxPair;
    ///
    /// let pair = FxPair::new(Currency::EUR, Currency::USD);
    /// assert!(pair.contains(Currency::EUR));
    /// assert!(pair.contains(Currency::USD));
    /// assert!(!pair.contains(Currency::JPY));
    /// ```
    #[inline]
    #[must_use]
    pub fn contains(&self, currency: Currency) -> bool {
        self.base == currency || self.quote == currency
    }

    /// Checks if this is a same-currency pair (base == quote).
    ///
    /// Note: Such pairs are typically invalid for FX operations
    /// but can be useful for validation.
    #[inline]
    #[must_use]
    pub fn is_same_currency(&self) -> bool {
        self.base == self.quote
    }
}

impl fmt::Display for FxPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.base.code(), self.quote.code())
    }
}

impl Default for FxPair {
    /// Returns a dummy FX pair (USD/USD) for use as a placeholder.
    ///
    /// Note: This creates a same-currency pair which is typically invalid
    /// for actual FX operations.
    fn default() -> Self {
        Self {
            base: Currency::USD,
            quote: Currency::USD,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fx_rate_new() {
        let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        assert_eq!(rate.base(), Currency::EUR);
        assert_eq!(rate.quote(), Currency::USD);
        assert!((rate.spot() - 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_fx_rate_code() {
        let rate = FxRate::new(Currency::USD, Currency::JPY, 150.0).unwrap();
        assert_eq!(rate.code(), "USD/JPY");
    }

    #[test]
    fn test_fx_rate_same_currency_error() {
        let result = FxRate::new(Currency::USD, Currency::USD, 1.0);
        assert!(result.is_err());
        match result {
            Err(CurrencyError::SameCurrency(code)) => assert_eq!(code, "USD"),
            _ => panic!("Expected SameCurrency error"),
        }
    }

    #[test]
    fn test_fx_rate_invalid_spot_error() {
        let result = FxRate::new(Currency::EUR, Currency::USD, 0.0);
        assert!(result.is_err());
        match result {
            Err(CurrencyError::InvalidSpotRate) => {}
            _ => panic!("Expected InvalidSpotRate error"),
        }

        let result = FxRate::new(Currency::EUR, Currency::USD, -1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_fx_rate_set_spot() {
        let mut rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        rate.set_spot(1.15).unwrap();
        assert!((rate.spot() - 1.15).abs() < 1e-10);
    }

    #[test]
    fn test_fx_rate_set_spot_invalid() {
        let mut rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let result = rate.set_spot(0.0);
        assert!(result.is_err());
        // Original spot should be unchanged
        assert!((rate.spot() - 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_fx_rate_invert() {
        let eurusd = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let usdeur = eurusd.invert();

        assert_eq!(usdeur.base(), Currency::USD);
        assert_eq!(usdeur.quote(), Currency::EUR);
        assert!((usdeur.spot() - 1.0 / 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_fx_rate_convert_to_quote() {
        let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let usd = rate.convert_to_quote(100.0);
        assert!((usd - 110.0).abs() < 1e-10);
    }

    #[test]
    fn test_fx_rate_convert_to_base() {
        let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let eur = rate.convert_to_base(110.0);
        assert!((eur - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_fx_rate_convert_roundtrip() {
        let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let original = 100.0;
        let converted = rate.convert_to_quote(original);
        let back = rate.convert_to_base(converted);
        assert!((back - original).abs() < 1e-10);
    }

    #[test]
    fn test_fx_rate_contains() {
        let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        assert!(rate.contains(Currency::EUR));
        assert!(rate.contains(Currency::USD));
        assert!(!rate.contains(Currency::JPY));
        assert!(!rate.contains(Currency::GBP));
    }

    #[test]
    fn test_fx_rate_equality() {
        let rate1 = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let rate2 = FxRate::new(Currency::EUR, Currency::USD, 1.20).unwrap();
        let rate3 = FxRate::new(Currency::USD, Currency::EUR, 0.91).unwrap();

        // Same currencies = equal (spot rate doesn't affect equality)
        assert_eq!(rate1, rate2);
        // Different order = not equal
        assert_ne!(rate1, rate3);
    }

    #[test]
    fn test_fx_rate_hash() {
        use std::collections::HashSet;
        let rate1 = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let rate2 = FxRate::new(Currency::EUR, Currency::USD, 1.20).unwrap();
        let rate3 = FxRate::new(Currency::USD, Currency::JPY, 150.0).unwrap();

        let mut set = HashSet::new();
        set.insert(rate1);
        set.insert(rate2); // Same as rate1
        set.insert(rate3);

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_fx_rate_clone() {
        let rate1 = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let rate2 = rate1;
        assert_eq!(rate1, rate2);
    }

    #[test]
    fn test_fx_rate_display() {
        let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let display = format!("{}", rate);
        assert!(display.contains("EUR/USD"));
        assert!(display.contains("1.1"));
    }

    #[test]
    fn test_fx_rate_debug() {
        let rate = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        let debug_str = format!("{:?}", rate);
        assert!(debug_str.contains("FxRate"));
        assert!(debug_str.contains("EUR"));
        assert!(debug_str.contains("USD"));
    }

    #[test]
    fn test_usdjpy_rate() {
        let rate = FxRate::new(Currency::USD, Currency::JPY, 150.0).unwrap();
        assert_eq!(rate.code(), "USD/JPY");

        // 1000 USD = 150,000 JPY
        let jpy = rate.convert_to_quote(1000.0);
        assert!((jpy - 150000.0).abs() < 1e-10);

        // 150,000 JPY = 1000 USD
        let usd = rate.convert_to_base(150000.0);
        assert!((usd - 1000.0).abs() < 1e-10);
    }

    #[test]
    #[allow(deprecated)]
    fn test_currency_pair_alias() {
        // Test that the deprecated alias still works
        let rate: CurrencyPair<f64> =
            CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
        assert_eq!(rate.base(), Currency::EUR);
    }

    // =========================================================================
    // FxPair Tests
    // =========================================================================

    #[test]
    fn test_fx_pair_new() {
        let pair = FxPair::new(Currency::EUR, Currency::USD);
        assert_eq!(pair.base(), Currency::EUR);
        assert_eq!(pair.quote(), Currency::USD);
    }

    #[test]
    fn test_fx_pair_code() {
        let pair = FxPair::new(Currency::USD, Currency::JPY);
        assert_eq!(pair.code(), "USD/JPY");
    }

    #[test]
    fn test_fx_pair_invert() {
        let eurusd = FxPair::new(Currency::EUR, Currency::USD);
        let usdeur = eurusd.invert();
        assert_eq!(usdeur.base(), Currency::USD);
        assert_eq!(usdeur.quote(), Currency::EUR);
    }

    #[test]
    fn test_fx_pair_contains() {
        let pair = FxPair::new(Currency::EUR, Currency::USD);
        assert!(pair.contains(Currency::EUR));
        assert!(pair.contains(Currency::USD));
        assert!(!pair.contains(Currency::JPY));
    }

    #[test]
    fn test_fx_pair_equality() {
        let pair1 = FxPair::new(Currency::EUR, Currency::USD);
        let pair2 = FxPair::new(Currency::EUR, Currency::USD);
        let pair3 = FxPair::new(Currency::USD, Currency::EUR);

        assert_eq!(pair1, pair2);
        assert_ne!(pair1, pair3);
    }

    #[test]
    fn test_fx_pair_hash() {
        use std::collections::HashSet;

        let pair1 = FxPair::new(Currency::EUR, Currency::USD);
        let pair2 = FxPair::new(Currency::EUR, Currency::USD);
        let pair3 = FxPair::new(Currency::USD, Currency::JPY);

        let mut set = HashSet::new();
        set.insert(pair1);
        set.insert(pair2); // Same as pair1
        set.insert(pair3);

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_fx_pair_display() {
        let pair = FxPair::new(Currency::EUR, Currency::USD);
        assert_eq!(format!("{}", pair), "EUR/USD");
    }

    #[test]
    fn test_fx_pair_is_same_currency() {
        let valid = FxPair::new(Currency::EUR, Currency::USD);
        let invalid = FxPair::new(Currency::USD, Currency::USD);

        assert!(!valid.is_same_currency());
        assert!(invalid.is_same_currency());
    }

    #[test]
    fn test_fx_pair_default() {
        let pair = FxPair::default();
        assert!(pair.is_same_currency());
    }
}
