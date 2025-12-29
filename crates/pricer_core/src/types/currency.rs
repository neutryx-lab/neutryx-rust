//! Currency types for financial calculations.
//!
//! This module provides ISO 4217 currency codes with metadata
//! for decimal precision and serialisation support.
//!
//! # Examples
//!
//! ```
//! use pricer_core::types::currency::Currency;
//!
//! let usd = Currency::USD;
//! assert_eq!(usd.code(), "USD");
//! assert_eq!(usd.decimal_places(), 2);
//!
//! let jpy = Currency::JPY;
//! assert_eq!(jpy.decimal_places(), 0);  // Yen has no decimal places
//! ```

use std::fmt;
use std::str::FromStr;

use super::error::CurrencyError;

/// ISO 4217 currency codes with decimal precision metadata.
///
/// Designed for static dispatch (enum-based) for Enzyme AD compatibility.
/// This enum provides the major trading currencies with their standard
/// decimal precision for financial calculations.
///
/// # Variants
/// - `USD`: United States Dollar (2 decimal places)
/// - `EUR`: Euro (2 decimal places)
/// - `GBP`: British Pound Sterling (2 decimal places)
/// - `JPY`: Japanese Yen (0 decimal places)
/// - `CHF`: Swiss Franc (2 decimal places)
///
/// # Examples
///
/// ```
/// use pricer_core::types::currency::Currency;
///
/// // Get currency code
/// assert_eq!(Currency::USD.code(), "USD");
///
/// // Get decimal places
/// assert_eq!(Currency::USD.decimal_places(), 2);
/// assert_eq!(Currency::JPY.decimal_places(), 0);
///
/// // Parse from string (case-insensitive)
/// let eur: Currency = "eur".parse().unwrap();
/// assert_eq!(eur, Currency::EUR);
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Currency {
    /// United States Dollar
    ///
    /// ISO 4217 code: USD
    /// Standard decimal places: 2
    USD,

    /// Euro
    ///
    /// ISO 4217 code: EUR
    /// Standard decimal places: 2
    EUR,

    /// British Pound Sterling
    ///
    /// ISO 4217 code: GBP
    /// Standard decimal places: 2
    GBP,

    /// Japanese Yen
    ///
    /// ISO 4217 code: JPY
    /// Standard decimal places: 0 (no minor units)
    JPY,

    /// Swiss Franc
    ///
    /// ISO 4217 code: CHF
    /// Standard decimal places: 2
    CHF,
}

impl Currency {
    /// Returns the ISO 4217 three-letter currency code.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::currency::Currency;
    ///
    /// assert_eq!(Currency::USD.code(), "USD");
    /// assert_eq!(Currency::EUR.code(), "EUR");
    /// assert_eq!(Currency::GBP.code(), "GBP");
    /// assert_eq!(Currency::JPY.code(), "JPY");
    /// assert_eq!(Currency::CHF.code(), "CHF");
    /// ```
    pub fn code(&self) -> &'static str {
        match self {
            Currency::USD => "USD",
            Currency::EUR => "EUR",
            Currency::GBP => "GBP",
            Currency::JPY => "JPY",
            Currency::CHF => "CHF",
        }
    }

    /// Returns the standard number of decimal places for this currency.
    ///
    /// Most currencies use 2 decimal places, but some (like JPY) use 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::currency::Currency;
    ///
    /// assert_eq!(Currency::USD.decimal_places(), 2);
    /// assert_eq!(Currency::EUR.decimal_places(), 2);
    /// assert_eq!(Currency::GBP.decimal_places(), 2);
    /// assert_eq!(Currency::JPY.decimal_places(), 0);
    /// assert_eq!(Currency::CHF.decimal_places(), 2);
    /// ```
    pub fn decimal_places(&self) -> u8 {
        match self {
            Currency::USD => 2,
            Currency::EUR => 2,
            Currency::GBP => 2,
            Currency::JPY => 0,
            Currency::CHF => 2,
        }
    }
}

impl FromStr for Currency {
    type Err = CurrencyError;

    /// Parses ISO 4217 currency code (case-insensitive).
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::types::currency::Currency;
    ///
    /// let usd: Currency = "USD".parse().unwrap();
    /// assert_eq!(usd, Currency::USD);
    ///
    /// // Case-insensitive
    /// let eur: Currency = "eur".parse().unwrap();
    /// assert_eq!(eur, Currency::EUR);
    ///
    /// // Unknown currency returns error
    /// let result: Result<Currency, _> = "XYZ".parse();
    /// assert!(result.is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, CurrencyError> {
        match s.to_uppercase().as_str() {
            "USD" => Ok(Currency::USD),
            "EUR" => Ok(Currency::EUR),
            "GBP" => Ok(Currency::GBP),
            "JPY" => Ok(Currency::JPY),
            "CHF" => Ok(Currency::CHF),
            _ => Err(CurrencyError::UnknownCurrency(s.to_string())),
        }
    }
}

impl fmt::Display for Currency {
    /// Formats as ISO 4217 code.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_code() {
        assert_eq!(Currency::USD.code(), "USD");
        assert_eq!(Currency::EUR.code(), "EUR");
        assert_eq!(Currency::GBP.code(), "GBP");
        assert_eq!(Currency::JPY.code(), "JPY");
        assert_eq!(Currency::CHF.code(), "CHF");
    }

    #[test]
    fn test_currency_decimal_places() {
        assert_eq!(Currency::USD.decimal_places(), 2);
        assert_eq!(Currency::EUR.decimal_places(), 2);
        assert_eq!(Currency::GBP.decimal_places(), 2);
        assert_eq!(Currency::JPY.decimal_places(), 0);
        assert_eq!(Currency::CHF.decimal_places(), 2);
    }

    #[test]
    fn test_currency_from_str() {
        assert_eq!("USD".parse::<Currency>().unwrap(), Currency::USD);
        assert_eq!("EUR".parse::<Currency>().unwrap(), Currency::EUR);
        assert_eq!("GBP".parse::<Currency>().unwrap(), Currency::GBP);
        assert_eq!("JPY".parse::<Currency>().unwrap(), Currency::JPY);
        assert_eq!("CHF".parse::<Currency>().unwrap(), Currency::CHF);
    }

    #[test]
    fn test_currency_from_str_case_insensitive() {
        assert_eq!("usd".parse::<Currency>().unwrap(), Currency::USD);
        assert_eq!("Eur".parse::<Currency>().unwrap(), Currency::EUR);
        assert_eq!("gbP".parse::<Currency>().unwrap(), Currency::GBP);
    }

    #[test]
    fn test_currency_from_str_unknown() {
        let result = "XYZ".parse::<Currency>();
        assert!(result.is_err());
        match result {
            Err(CurrencyError::UnknownCurrency(code)) => assert_eq!(code, "XYZ"),
            _ => panic!("Expected UnknownCurrency error"),
        }
    }

    #[test]
    fn test_currency_display() {
        assert_eq!(format!("{}", Currency::USD), "USD");
        assert_eq!(format!("{}", Currency::EUR), "EUR");
        assert_eq!(format!("{}", Currency::JPY), "JPY");
    }

    #[test]
    fn test_currency_roundtrip() {
        for currency in [
            Currency::USD,
            Currency::EUR,
            Currency::GBP,
            Currency::JPY,
            Currency::CHF,
        ] {
            let code = currency.code();
            let parsed: Currency = code.parse().unwrap();
            assert_eq!(currency, parsed);
        }
    }

    #[test]
    fn test_currency_copy_clone() {
        let c1 = Currency::USD;
        let c2 = c1; // Copy
        let c3 = c1.clone(); // Clone
        assert_eq!(c1, c2);
        assert_eq!(c1, c3);
    }

    #[test]
    fn test_currency_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Currency::USD);
        set.insert(Currency::EUR);
        set.insert(Currency::USD); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[cfg(feature = "serde")]
    mod serde_tests {
        use super::*;

        #[test]
        fn test_currency_serde_roundtrip() {
            let currency = Currency::USD;
            let json = serde_json::to_string(&currency).unwrap();
            assert_eq!(json, "\"USD\"");

            let parsed: Currency = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, currency);
        }

        #[test]
        fn test_all_currencies_serde_roundtrip() {
            for currency in [
                Currency::USD,
                Currency::EUR,
                Currency::GBP,
                Currency::JPY,
                Currency::CHF,
            ] {
                let json = serde_json::to_string(&currency).unwrap();
                let parsed: Currency = serde_json::from_str(&json).unwrap();
                assert_eq!(parsed, currency);
            }
        }
    }
}
