//! Error types for structured error handling.
//!
//! This module provides:
//! - `PricingError`: Errors from pricing operations
//! - `DateError`: Errors from date construction and parsing
//! - `CurrencyError`: Errors from currency parsing

use std::fmt;

/// Categorised pricing errors.
///
/// Provides structured error handling for pricing operations with
/// descriptive context for each failure mode.
///
/// # Variants
/// - `InvalidInput`: Invalid market data or parameters
/// - `NumericalInstability`: Computation failed to converge
/// - `ModelFailure`: Model assumptions violated
/// - `UnsupportedInstrument`: Instrument type not supported by model
///
/// # Examples
/// ```
/// use pricer_core::types::PricingError;
///
/// let err = PricingError::InvalidInput("Negative spot price".to_string());
/// assert_eq!(format!("{}", err), "Invalid input: Negative spot price");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PricingError {
    /// Invalid input data or parameters
    InvalidInput(String),

    /// Numerical instability during computation
    NumericalInstability(String),

    /// Model failed to produce valid result
    ModelFailure(String),

    /// Instrument type not supported
    UnsupportedInstrument(String),
}

impl fmt::Display for PricingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PricingError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            PricingError::NumericalInstability(msg) => {
                write!(f, "Numerical instability: {}", msg)
            }
            PricingError::ModelFailure(msg) => write!(f, "Model failure: {}", msg),
            PricingError::UnsupportedInstrument(msg) => {
                write!(f, "Unsupported instrument: {}", msg)
            }
        }
    }
}

impl std::error::Error for PricingError {}

/// Date-related errors.
///
/// Provides structured error handling for date construction and parsing
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `InvalidDate`: Invalid date components (e.g., February 30th)
/// - `ParseError`: Failed to parse date string
///
/// # Examples
/// ```
/// use pricer_core::types::DateError;
///
/// let err = DateError::InvalidDate { year: 2024, month: 2, day: 30 };
/// assert_eq!(format!("{}", err), "Invalid date: 2024-2-30");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateError {
    /// Invalid date components (e.g., February 30th).
    InvalidDate {
        /// Year component
        year: i32,
        /// Month component (1-12)
        month: u32,
        /// Day component (1-31)
        day: u32,
    },

    /// Failed to parse date string.
    ParseError(String),
}

impl fmt::Display for DateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DateError::InvalidDate { year, month, day } => {
                write!(f, "Invalid date: {}-{}-{}", year, month, day)
            }
            DateError::ParseError(msg) => write!(f, "Date parse error: {}", msg),
        }
    }
}

impl std::error::Error for DateError {}

/// Currency-related errors.
///
/// Provides structured error handling for currency parsing
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `UnknownCurrency`: Unknown currency code
/// - `ParseError`: Failed to parse currency string
///
/// # Examples
/// ```
/// use pricer_core::types::CurrencyError;
///
/// let err = CurrencyError::UnknownCurrency("XYZ".to_string());
/// assert_eq!(format!("{}", err), "Unknown currency: XYZ");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrencyError {
    /// Unknown currency code.
    UnknownCurrency(String),

    /// Failed to parse currency string.
    ParseError(String),
}

impl fmt::Display for CurrencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CurrencyError::UnknownCurrency(code) => write!(f, "Unknown currency: {}", code),
            CurrencyError::ParseError(msg) => write!(f, "Currency parse error: {}", msg),
        }
    }
}

impl std::error::Error for CurrencyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_input_display() {
        let err = PricingError::InvalidInput("Test error".to_string());
        assert_eq!(format!("{}", err), "Invalid input: Test error");
    }

    #[test]
    fn test_numerical_instability_display() {
        let err = PricingError::NumericalInstability("Failed to converge".to_string());
        assert_eq!(format!("{}", err), "Numerical instability: Failed to converge");
    }

    #[test]
    fn test_model_failure_display() {
        let err = PricingError::ModelFailure("Volatility out of range".to_string());
        assert_eq!(format!("{}", err), "Model failure: Volatility out of range");
    }

    #[test]
    fn test_unsupported_instrument_display() {
        let err = PricingError::UnsupportedInstrument("Asian option".to_string());
        assert_eq!(format!("{}", err), "Unsupported instrument: Asian option");
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = PricingError::InvalidInput("Test".to_string());
        let _: &dyn std::error::Error = &err; // Verify Error trait is implemented
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = PricingError::InvalidInput("Test".to_string());
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // DateError tests

    #[test]
    fn test_date_error_invalid_date_display() {
        let err = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        assert_eq!(format!("{}", err), "Invalid date: 2024-2-30");
    }

    #[test]
    fn test_date_error_parse_error_display() {
        let err = DateError::ParseError("invalid format".to_string());
        assert_eq!(format!("{}", err), "Date parse error: invalid format");
    }

    #[test]
    fn test_date_error_trait_implementation() {
        let err = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_date_error_clone_and_equality() {
        let err1 = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // CurrencyError tests

    #[test]
    fn test_currency_error_unknown_currency_display() {
        let err = CurrencyError::UnknownCurrency("XYZ".to_string());
        assert_eq!(format!("{}", err), "Unknown currency: XYZ");
    }

    #[test]
    fn test_currency_error_parse_error_display() {
        let err = CurrencyError::ParseError("invalid input".to_string());
        assert_eq!(format!("{}", err), "Currency parse error: invalid input");
    }

    #[test]
    fn test_currency_error_trait_implementation() {
        let err = CurrencyError::UnknownCurrency("XYZ".to_string());
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_currency_error_clone_and_equality() {
        let err1 = CurrencyError::UnknownCurrency("XYZ".to_string());
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}
