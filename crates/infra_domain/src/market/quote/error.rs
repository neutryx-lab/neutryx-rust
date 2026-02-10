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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_variants_display() {
        assert!(MarketQuoteError::nan().to_string().contains("NaN"));
        assert!(MarketQuoteError::infinite(f64::INFINITY).to_string().contains("infinite"));
        assert!(MarketQuoteError::out_of_bounds(1.5, 0.0, 1.0).to_string().contains("between"));
        assert!(MarketQuoteError::unsupported_rate_type(RateType::Vol).to_string().contains("Vol"));
        assert!(MarketQuoteError::ValidationFailed("msg".into()).to_string().contains("msg"));

        let stale = MarketQuoteError::StaleData { threshold_ms: 60000, description: "USD".into() };
        assert!(stale.to_string().contains("60000"));

        let missing = MarketQuoteError::MissingQuote { description: "EUR 5Y".into() };
        assert!(missing.to_string().contains("EUR 5Y"));
    }

    #[test]
    fn test_helper_constructors() {
        assert!(matches!(MarketQuoteError::nan(), MarketQuoteError::InvalidQuote { .. }));
        assert!(matches!(
            MarketQuoteError::infinite(f64::INFINITY),
            MarketQuoteError::InvalidQuote { value, .. } if value.is_infinite()
        ));
        assert!(matches!(
            MarketQuoteError::out_of_bounds(1.5, 0.0, 1.0),
            MarketQuoteError::InvalidQuote { value, .. } if (value - 1.5).abs() < f64::EPSILON
        ));
        assert!(matches!(
            MarketQuoteError::unsupported_rate_type(RateType::BasisSwap),
            MarketQuoteError::MappingFailed { rate_type, .. } if rate_type == RateType::BasisSwap
        ));
    }
}
