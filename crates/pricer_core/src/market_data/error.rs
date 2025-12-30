//! Market data error types.
//!
//! This module provides structured error handling for market data operations
//! including yield curve and volatility surface lookups.

use crate::types::{InterpolationError, PricingError};
use thiserror::Error;

/// Market data operation errors.
///
/// Provides structured error handling for yield curve and volatility surface
/// operations with descriptive context for each failure mode.
///
/// # Variants
///
/// - `InvalidMaturity`: Negative time to maturity
/// - `InvalidStrike`: Non-positive strike price
/// - `InvalidExpiry`: Non-positive time to expiry
/// - `OutOfBounds`: Query outside valid domain
/// - `Interpolation`: Wrapped interpolation error
/// - `InsufficientData`: Not enough data points for construction
///
/// # Examples
///
/// ```
/// use pricer_core::market_data::MarketDataError;
///
/// let err = MarketDataError::InvalidMaturity { t: -1.0 };
/// assert!(format!("{}", err).contains("-1"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MarketDataError {
    /// Invalid maturity (negative time).
    #[error("Invalid maturity: t = {t}")]
    InvalidMaturity {
        /// The invalid maturity value
        t: f64,
    },

    /// Invalid strike price (non-positive).
    #[error("Invalid strike: K = {strike}")]
    InvalidStrike {
        /// The invalid strike value
        strike: f64,
    },

    /// Invalid expiry (non-positive).
    #[error("Invalid expiry: T = {expiry}")]
    InvalidExpiry {
        /// The invalid expiry value
        expiry: f64,
    },

    /// Query point outside valid domain.
    #[error("Out of bounds: {x} not in [{min}, {max}]")]
    OutOfBounds {
        /// The query point that was out of bounds
        x: f64,
        /// Minimum valid value
        min: f64,
        /// Maximum valid value
        max: f64,
    },

    /// Interpolation error.
    #[error("Interpolation error: {0}")]
    Interpolation(#[from] InterpolationError),

    /// Insufficient data for construction.
    #[error("Insufficient data: got {got}, need {need}")]
    InsufficientData {
        /// Number of points provided
        got: usize,
        /// Minimum number of points required
        need: usize,
    },
}

impl From<MarketDataError> for PricingError {
    fn from(err: MarketDataError) -> Self {
        PricingError::InvalidInput(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_maturity_display() {
        let err = MarketDataError::InvalidMaturity { t: -1.5 };
        assert_eq!(format!("{}", err), "Invalid maturity: t = -1.5");
    }

    #[test]
    fn test_invalid_strike_display() {
        let err = MarketDataError::InvalidStrike { strike: -100.0 };
        assert_eq!(format!("{}", err), "Invalid strike: K = -100");
    }

    #[test]
    fn test_invalid_expiry_display() {
        let err = MarketDataError::InvalidExpiry { expiry: 0.0 };
        assert_eq!(format!("{}", err), "Invalid expiry: T = 0");
    }

    #[test]
    fn test_out_of_bounds_display() {
        let err = MarketDataError::OutOfBounds {
            x: 5.0,
            min: 0.0,
            max: 3.0,
        };
        assert_eq!(format!("{}", err), "Out of bounds: 5 not in [0, 3]");
    }

    #[test]
    fn test_insufficient_data_display() {
        let err = MarketDataError::InsufficientData { got: 1, need: 2 };
        assert_eq!(format!("{}", err), "Insufficient data: got 1, need 2");
    }

    #[test]
    fn test_from_interpolation_error() {
        let interp_err = InterpolationError::OutOfBounds {
            x: 5.0,
            min: 0.0,
            max: 3.0,
        };
        let mkt_err: MarketDataError = interp_err.into();
        match mkt_err {
            MarketDataError::Interpolation(_) => {}
            _ => panic!("Expected Interpolation variant"),
        }
    }

    #[test]
    fn test_into_pricing_error() {
        let mkt_err = MarketDataError::InvalidMaturity { t: -1.0 };
        let pricing_err: PricingError = mkt_err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => {
                assert!(msg.contains("-1"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = MarketDataError::InvalidMaturity { t: -1.0 };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = MarketDataError::InvalidStrike { strike: 0.0 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}
