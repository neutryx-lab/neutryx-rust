//! Market quote error types.
//!
//! This module provides structured error types for market quote operations
//! using the `thiserror` crate.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::{MarketQuoteError, RateType};
//!
//! let error = MarketQuoteError::InvalidQuote {
//!     value: f64::NAN,
//!     reason: "Value is NaN".to_string(),
//! };
//!
//! assert!(error.to_string().contains("Invalid quote"));
//! ```

use thiserror::Error;

use crate::market::core::RateType;

/// Errors that can occur during market quote operations.
///
/// This error type covers validation failures, missing data,
/// and mapping errors.
///
/// # Variants
///
/// - `InvalidQuote`: The quote value is invalid (NaN, Infinite, or out of
///   bounds)
/// - `StaleData`: The quote data is older than the acceptable threshold
/// - `MissingQuote`: A required quote was not found
/// - `MappingFailed`: Failed to map a quote to an instrument
/// - `ValidationFailed`: Custom validation failure
///
/// # Examples
///
/// ```
/// use infra_domain::market::{MarketQuoteError, RateType};
///
/// // Invalid quote error
/// let error = MarketQuoteError::InvalidQuote {
///     value: 150.0,
///     reason: "Interest rate exceeds 100%".to_string(),
/// };
/// println!("{}", error);
///
/// // Mapping failed error
/// let error = MarketQuoteError::MappingFailed {
///     rate_type: RateType::Vol,
///     reason: "Volatility mapping not supported".to_string(),
/// };
/// println!("{}", error);
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MarketQuoteError {
    /// The quote value is invalid.
    ///
    /// This error is returned when:
    /// - The value is NaN or Infinite
    /// - The value exceeds reasonable bounds for the rate type
    #[error("Invalid quote value: {value} ({reason})")]
    InvalidQuote {
        /// The invalid value.
        value: f64,
        /// Reason for invalidity.
        reason: String,
    },

    /// The quote data is stale.
    ///
    /// This error is returned when the timestamp of a quote
    /// is older than the acceptable threshold.
    #[error("Stale data: quote is older than {threshold_ms}ms (description: {description})")]
    StaleData {
        /// The staleness threshold in milliseconds.
        threshold_ms: i64,
        /// Description of the stale quote.
        description: String,
    },

    /// A required quote was not found.
    #[error("Missing quote: {description}")]
    MissingQuote {
        /// Description of the missing quote.
        description: String,
    },

    /// Failed to map a quote to an instrument.
    ///
    /// This error is returned when an unsupported rate type
    /// is used with the instrument mapper.
    #[error("Mapping failed: cannot convert {rate_type:?} to Instrument ({reason})")]
    MappingFailed {
        /// The rate type that could not be mapped.
        rate_type: RateType,
        /// Reason for the mapping failure.
        reason: String,
    },

    /// Custom validation failure.
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

impl MarketQuoteError {
    /// Creates an `InvalidQuote` error for a NaN value.
    #[must_use]
    pub fn nan() -> Self {
        Self::InvalidQuote {
            value: f64::NAN,
            reason: "Value is NaN".to_string(),
        }
    }

    /// Creates an `InvalidQuote` error for an infinite value.
    #[must_use]
    pub fn infinite(value: f64) -> Self {
        Self::InvalidQuote {
            value,
            reason: "Value is infinite".to_string(),
        }
    }

    /// Creates an `InvalidQuote` error for a value out of bounds.
    #[must_use]
    pub fn out_of_bounds(value: f64, min: f64, max: f64) -> Self {
        Self::InvalidQuote {
            value,
            reason: format!("Value must be between {} and {}", min, max),
        }
    }

    /// Creates a `MappingFailed` error for an unsupported rate type.
    #[must_use]
    pub fn unsupported_rate_type(rate_type: RateType) -> Self {
        Self::MappingFailed {
            rate_type,
            reason: "Unsupported rate type for instrument mapping".to_string(),
        }
    }
}

/// Type alias for backward compatibility.
#[deprecated(since = "0.2.0", note = "Use MarketQuoteError instead")]
pub type MarketRateError = MarketQuoteError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_quote_error() {
        let error = MarketQuoteError::InvalidQuote {
            value: f64::NAN,
            reason: "Value is NaN".to_string(),
        };

        let msg = error.to_string();
        assert!(msg.contains("Invalid quote"));
        assert!(msg.contains("NaN"));
    }

    #[test]
    fn test_stale_data_error() {
        let error = MarketQuoteError::StaleData {
            threshold_ms: 60000,
            description: "USD 3M SOFR".to_string(),
        };

        let msg = error.to_string();
        assert!(msg.contains("Stale data"));
        assert!(msg.contains("60000"));
        assert!(msg.contains("USD 3M SOFR"));
    }

    #[test]
    fn test_missing_quote_error() {
        let error = MarketQuoteError::MissingQuote {
            description: "EUR 5Y Swap".to_string(),
        };

        let msg = error.to_string();
        assert!(msg.contains("Missing quote"));
        assert!(msg.contains("EUR 5Y Swap"));
    }

    #[test]
    fn test_mapping_failed_error() {
        let error = MarketQuoteError::MappingFailed {
            rate_type: RateType::Vol,
            reason: "Volatility mapping not supported".to_string(),
        };

        let msg = error.to_string();
        assert!(msg.contains("Mapping failed"));
        assert!(msg.contains("Vol"));
    }

    #[test]
    fn test_validation_failed_error() {
        let error = MarketQuoteError::ValidationFailed("Custom validation message".to_string());

        let msg = error.to_string();
        assert!(msg.contains("Validation failed"));
        assert!(msg.contains("Custom validation message"));
    }

    #[test]
    fn test_nan_helper() {
        let error = MarketQuoteError::nan();

        match error {
            MarketQuoteError::InvalidQuote { reason, .. } => {
                assert!(reason.contains("NaN"));
            }
            _ => panic!("Expected InvalidQuote"),
        }
    }

    #[test]
    fn test_infinite_helper() {
        let error = MarketQuoteError::infinite(f64::INFINITY);

        match error {
            MarketQuoteError::InvalidQuote { value, reason } => {
                assert!(value.is_infinite());
                assert!(reason.contains("infinite"));
            }
            _ => panic!("Expected InvalidQuote"),
        }
    }

    #[test]
    fn test_out_of_bounds_helper() {
        let error = MarketQuoteError::out_of_bounds(1.5, -0.1, 1.0);

        match error {
            MarketQuoteError::InvalidQuote { value, reason } => {
                assert!((value - 1.5).abs() < f64::EPSILON);
                assert!(reason.contains("-0.1"));
                assert!(reason.contains("1"));
            }
            _ => panic!("Expected InvalidQuote"),
        }
    }

    #[test]
    fn test_unsupported_rate_type_helper() {
        let error = MarketQuoteError::unsupported_rate_type(RateType::BasisSwap);

        match error {
            MarketQuoteError::MappingFailed { rate_type, .. } => {
                assert_eq!(rate_type, RateType::BasisSwap);
            }
            _ => panic!("Expected MappingFailed"),
        }
    }

    #[test]
    fn test_error_clone() {
        let error = MarketQuoteError::ValidationFailed("test".to_string());
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_error_eq() {
        let error1 = MarketQuoteError::ValidationFailed("test".to_string());
        let error2 = MarketQuoteError::ValidationFailed("test".to_string());
        let error3 = MarketQuoteError::ValidationFailed("other".to_string());

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[test]
    fn test_error_debug() {
        let error = MarketQuoteError::InvalidQuote {
            value: 0.5,
            reason: "test".to_string(),
        };

        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InvalidQuote"));
        assert!(debug_str.contains("0.5"));
    }
}
