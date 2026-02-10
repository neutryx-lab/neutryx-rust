//! ISO 4217 currency codes with decimal precision and serialisation support.

use std::{fmt, str::FromStr};

use crate::error::CurrencyError;

/// ISO 4217 currency codes with decimal precision metadata.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Currency {
    /// United States Dollar (2 dp).
    #[default]
    USD,

    /// Euro (2 dp).
    EUR,

    /// British Pound Sterling (2 dp).
    GBP,

    /// Japanese Yen (0 dp).
    JPY,

    /// Swiss Franc (2 dp).
    CHF,
}

impl Currency {
    /// Returns all supported currencies.
    #[must_use]
    pub const fn all() -> [Currency; 5] {
        [
            Currency::USD,
            Currency::EUR,
            Currency::GBP,
            Currency::JPY,
            Currency::CHF,
        ]
    }

    /// Returns all currency codes.
    #[must_use]
    pub const fn all_codes() -> [&'static str; 5] { ["USD", "EUR", "GBP", "JPY", "CHF"] }

    /// Returns the ISO 4217 three-letter currency code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Currency::USD => "USD",
            Currency::EUR => "EUR",
            Currency::GBP => "GBP",
            Currency::JPY => "JPY",
            Currency::CHF => "CHF",
        }
    }

    /// Returns the full currency name.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::Currency;
    ///
    /// assert_eq!(Currency::USD.name(), "US Dollar");
    /// assert_eq!(Currency::JPY.name(), "Japanese Yen");
    /// ```
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Currency::USD => "US Dollar",
            Currency::EUR => "Euro",
            Currency::GBP => "British Pound",
            Currency::JPY => "Japanese Yen",
            Currency::CHF => "Swiss Franc",
        }
    }

    /// Returns the standard number of decimal places for this currency.
    ///
    /// Most currencies use 2 decimal places, but some (like JPY) use 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::Currency;
    ///
    /// assert_eq!(Currency::USD.decimal_places(), 2);
    /// assert_eq!(Currency::EUR.decimal_places(), 2);
    /// assert_eq!(Currency::GBP.decimal_places(), 2);
    /// assert_eq!(Currency::JPY.decimal_places(), 0);
    /// assert_eq!(Currency::CHF.decimal_places(), 2);
    /// ```
    #[must_use]
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
    /// use infra_domain::market::Currency;
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.code()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_unknown_error_variant() {
        match "XYZ".parse::<Currency>() {
            Err(CurrencyError::UnknownCurrency(code)) => assert_eq!(code, "XYZ"),
            other => panic!("Expected UnknownCurrency, got {:?}", other),
        }
    }

    #[test]
    fn test_roundtrip() {
        for currency in Currency::all() {
            let parsed: Currency = currency.code().parse().unwrap();
            assert_eq!(currency, parsed);
        }
    }
}
