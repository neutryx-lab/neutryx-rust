//! Instrument error types.
//!
//! This module provides structured error handling for instrument
//! construction and payoff computation operations.

use pricer_core::types::PricingError;
use thiserror::Error;

/// Instrument-related errors.
///
/// Provides structured error handling for instrument construction
/// and payoff computation with descriptive context for each failure mode.
///
/// # Variants
/// - `InvalidStrike`: Strike price is non-positive
/// - `InvalidExpiry`: Expiry time is non-positive
/// - `InvalidNotional`: Notional amount is invalid
/// - `PayoffError`: Payoff computation failed
/// - `InvalidParameter`: General parameter validation failure
///
/// # Examples
/// ```
/// use pricer_models::instruments::InstrumentError;
///
/// let err = InstrumentError::InvalidStrike { strike: -100.0 };
/// assert!(format!("{}", err).contains("-100"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum InstrumentError {
    /// Invalid strike price (non-positive).
    #[error("Invalid strike: K = {strike}")]
    InvalidStrike {
        /// The invalid strike value
        strike: f64,
    },

    /// Invalid expiry time (non-positive).
    #[error("Invalid expiry: T = {expiry}")]
    InvalidExpiry {
        /// The invalid expiry value
        expiry: f64,
    },

    /// Invalid notional amount.
    #[error("Invalid notional: N = {notional}")]
    InvalidNotional {
        /// The invalid notional value
        notional: f64,
    },

    /// Payoff computation error.
    #[error("Payoff computation error: {message}")]
    PayoffError {
        /// Description of the payoff error
        message: String,
    },

    /// Invalid parameter (general validation failure).
    #[error("Invalid parameter: {message}")]
    InvalidParameter {
        /// Description of the parameter error
        message: String,
    },
}

impl From<InstrumentError> for PricingError {
    fn from(err: InstrumentError) -> Self {
        match err {
            InstrumentError::InvalidStrike { strike } => {
                PricingError::InvalidInput(format!("Invalid strike: K = {}", strike))
            }
            InstrumentError::InvalidExpiry { expiry } => {
                PricingError::InvalidInput(format!("Invalid expiry: T = {}", expiry))
            }
            InstrumentError::InvalidNotional { notional } => {
                PricingError::InvalidInput(format!("Invalid notional: N = {}", notional))
            }
            InstrumentError::PayoffError { message } => PricingError::ModelFailure(message),
            InstrumentError::InvalidParameter { message } => PricingError::InvalidInput(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_strike_display() {
        let err = InstrumentError::InvalidStrike { strike: -100.0 };
        assert_eq!(format!("{}", err), "Invalid strike: K = -100");
    }

    #[test]
    fn test_invalid_expiry_display() {
        let err = InstrumentError::InvalidExpiry { expiry: -0.5 };
        assert_eq!(format!("{}", err), "Invalid expiry: T = -0.5");
    }

    #[test]
    fn test_invalid_notional_display() {
        let err = InstrumentError::InvalidNotional { notional: -1000.0 };
        assert_eq!(format!("{}", err), "Invalid notional: N = -1000");
    }

    #[test]
    fn test_payoff_error_display() {
        let err = InstrumentError::PayoffError {
            message: "Negative spot price".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Payoff computation error: Negative spot price"
        );
    }

    #[test]
    fn test_invalid_parameter_display() {
        let err = InstrumentError::InvalidParameter {
            message: "Epsilon must be positive".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Invalid parameter: Epsilon must be positive"
        );
    }

    #[test]
    fn test_from_instrument_error_to_pricing_error_invalid_strike() {
        let instrument_err = InstrumentError::InvalidStrike { strike: -100.0 };
        let pricing_err: PricingError = instrument_err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => assert!(msg.contains("-100")),
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_from_instrument_error_to_pricing_error_payoff() {
        let instrument_err = InstrumentError::PayoffError {
            message: "Test error".to_string(),
        };
        let pricing_err: PricingError = instrument_err.into();
        match pricing_err {
            PricingError::ModelFailure(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected ModelFailure variant"),
        }
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = InstrumentError::InvalidStrike { strike: -100.0 };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = InstrumentError::InvalidExpiry { expiry: -0.5 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}
