//! Market rate error types.
//!
//! This module provides structured error types for market rate operations
//! using the `thiserror` crate.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::{MarketRateError, RateType};
//!
//! let error = MarketRateError::InvalidRate {
//!     value: f64::NAN,
//!     reason: "Value is NaN".to_string(),
//! };
//!
//! assert!(error.to_string().contains("Invalid rate"));
//! ```

use thiserror::Error;

use super::rate_type::RateType;

/// Errors that can occur during market rate operations.
///
/// This error type covers validation failures, missing data,
/// and mapping errors.
///
/// # Variants
///
/// - `InvalidRate`: The rate value is invalid (NaN, Infinite, or out of bounds)
/// - `StaleData`: The rate data is older than the acceptable threshold
/// - `MissingRate`: A required rate was not found
/// - `MappingFailed`: Failed to map a rate to an instrument
/// - `ValidationFailed`: Custom validation failure
///
/// # Examples
///
/// ```
/// use infra_master::market::{MarketRateError, RateType};
///
/// // Invalid rate error
/// let error = MarketRateError::InvalidRate {
///     value: 150.0,
///     reason: "Interest rate exceeds 100%".to_string(),
/// };
/// println!("{}", error);
///
/// // Mapping failed error
/// let error = MarketRateError::MappingFailed {
///     rate_type: RateType::Vol,
///     reason: "Volatility mapping not supported".to_string(),
/// };
/// println!("{}", error);
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MarketRateError {
    /// The rate value is invalid.
    ///
    /// This error is returned when:
    /// - The value is NaN or Infinite
    /// - The value exceeds reasonable bounds for the rate type
    #[error("Invalid rate value: {value} ({reason})")]
    InvalidRate {
        /// The invalid value.
        value: f64,
        /// Reason for invalidity.
        reason: String,
    },

    /// The rate data is stale.
    ///
    /// This error is returned when the timestamp of a rate
    /// is older than the acceptable threshold.
    #[error("Stale data: rate is older than {threshold_ms}ms (description: {description})")]
    StaleData {
        /// The staleness threshold in milliseconds.
        threshold_ms: i64,
        /// Description of the stale rate.
        description: String,
    },

    /// A required rate was not found.
    #[error("Missing rate: {description}")]
    MissingRate {
        /// Description of the missing rate.
        description: String,
    },

    /// Failed to map a rate to an instrument.
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

impl MarketRateError {
    /// Creates an `InvalidRate` error for a NaN value.
    #[must_use]
    pub fn nan() -> Self {
        Self::InvalidRate {
            value: f64::NAN,
            reason: "Value is NaN".to_string(),
        }
    }

    /// Creates an `InvalidRate` error for an infinite value.
    #[must_use]
    pub fn infinite(value: f64) -> Self {
        Self::InvalidRate {
            value,
            reason: "Value is infinite".to_string(),
        }
    }

    /// Creates an `InvalidRate` error for a value out of bounds.
    #[must_use]
    pub fn out_of_bounds(value: f64, min: f64, max: f64) -> Self {
        Self::InvalidRate {
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
    fn test_invalid_rate_error() {
        let error = MarketRateError::InvalidRate {
            value: f64::NAN,
            reason: "Value is NaN".to_string(),
        };

        let msg = error.to_string();
        assert!(msg.contains("Invalid rate"));
        assert!(msg.contains("NaN"));
    }

    #[test]
    fn test_stale_data_error() {
        let error = MarketRateError::StaleData {
            threshold_ms: 60000,
            description: "USD 3M SOFR".to_string(),
        };

        let msg = error.to_string();
        assert!(msg.contains("Stale data"));
        assert!(msg.contains("60000"));
        assert!(msg.contains("USD 3M SOFR"));
    }

    #[test]
    fn test_missing_rate_error() {
        let error = MarketRateError::MissingRate {
            description: "EUR 5Y Swap".to_string(),
        };

        let msg = error.to_string();
        assert!(msg.contains("Missing rate"));
        assert!(msg.contains("EUR 5Y Swap"));
    }

    #[test]
    fn test_mapping_failed_error() {
        let error = MarketRateError::MappingFailed {
            rate_type: RateType::Vol,
            reason: "Volatility mapping not supported".to_string(),
        };

        let msg = error.to_string();
        assert!(msg.contains("Mapping failed"));
        assert!(msg.contains("Vol"));
    }

    #[test]
    fn test_validation_failed_error() {
        let error = MarketRateError::ValidationFailed("Custom validation message".to_string());

        let msg = error.to_string();
        assert!(msg.contains("Validation failed"));
        assert!(msg.contains("Custom validation message"));
    }

    #[test]
    fn test_nan_helper() {
        let error = MarketRateError::nan();

        match error {
            MarketRateError::InvalidRate { reason, .. } => {
                assert!(reason.contains("NaN"));
            }
            _ => panic!("Expected InvalidRate"),
        }
    }

    #[test]
    fn test_infinite_helper() {
        let error = MarketRateError::infinite(f64::INFINITY);

        match error {
            MarketRateError::InvalidRate { value, reason } => {
                assert!(value.is_infinite());
                assert!(reason.contains("infinite"));
            }
            _ => panic!("Expected InvalidRate"),
        }
    }

    #[test]
    fn test_out_of_bounds_helper() {
        let error = MarketRateError::out_of_bounds(1.5, -0.1, 1.0);

        match error {
            MarketRateError::InvalidRate { value, reason } => {
                assert!((value - 1.5).abs() < f64::EPSILON);
                assert!(reason.contains("-0.1"));
                assert!(reason.contains("1"));
            }
            _ => panic!("Expected InvalidRate"),
        }
    }

    #[test]
    fn test_unsupported_rate_type_helper() {
        let error = MarketRateError::unsupported_rate_type(RateType::BasisSwap);

        match error {
            MarketRateError::MappingFailed { rate_type, .. } => {
                assert_eq!(rate_type, RateType::BasisSwap);
            }
            _ => panic!("Expected MappingFailed"),
        }
    }

    #[test]
    fn test_error_clone() {
        let error = MarketRateError::ValidationFailed("test".to_string());
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_error_eq() {
        let error1 = MarketRateError::ValidationFailed("test".to_string());
        let error2 = MarketRateError::ValidationFailed("test".to_string());
        let error3 = MarketRateError::ValidationFailed("other".to_string());

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[test]
    fn test_error_debug() {
        let error = MarketRateError::InvalidRate {
            value: 0.5,
            reason: "test".to_string(),
        };

        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InvalidRate"));
        assert!(debug_str.contains("0.5"));
    }
}
