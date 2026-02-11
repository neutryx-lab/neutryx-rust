//! Market quote error types.

use thiserror::Error;

use crate::market::core::RateType;

/// Errors that can occur during market quote operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MarketQuoteError {
    /// The quote value is invalid.
    #[error("Invalid quote value: {value} ({reason})")]
    InvalidQuote {
        /// The invalid value.
        value: f64,
        /// Reason for invalidity.
        reason: String,
    },

    /// The quote data is stale.
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
        assert!(MarketQuoteError::infinite(f64::INFINITY)
            .to_string()
            .contains("infinite"));
        assert!(MarketQuoteError::out_of_bounds(1.5, 0.0, 1.0)
            .to_string()
            .contains("between"));
        assert!(MarketQuoteError::unsupported_rate_type(RateType::Vol)
            .to_string()
            .contains("Vol"));
        assert!(MarketQuoteError::ValidationFailed("msg".into())
            .to_string()
            .contains("msg"));

        let stale = MarketQuoteError::StaleData {
            threshold_ms: 60000,
            description: "USD".into(),
        };
        assert!(stale.to_string().contains("60000"));

        let missing = MarketQuoteError::MissingQuote {
            description: "EUR 5Y".into(),
        };
        assert!(missing.to_string().contains("EUR 5Y"));
    }

    #[test]
    fn test_helper_constructors() {
        assert!(matches!(
            MarketQuoteError::nan(),
            MarketQuoteError::InvalidQuote { .. }
        ));
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
