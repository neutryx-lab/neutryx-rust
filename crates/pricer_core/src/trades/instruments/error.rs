//! Instrument error types.

use crate::types::PricingError;
use thiserror::Error;

/// Instrument-related errors.
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
