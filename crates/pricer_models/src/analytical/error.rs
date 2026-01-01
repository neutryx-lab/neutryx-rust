//! Error types for analytical pricing operations.
//!
//! This module provides:
//! - `AnalyticalError`: Errors specific to analytical pricing models

use pricer_core::types::PricingError;
use thiserror::Error;

/// Analytical pricing errors.
///
/// Provides structured error handling for analytical pricing operations
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `InvalidVolatility`: Non-positive volatility
/// - `InvalidSpot`: Non-positive spot price (for Black-Scholes)
/// - `UnsupportedExerciseStyle`: Exercise style not supported by model
/// - `NumericalInstability`: Computation encountered numerical issues
///
/// # Examples
/// ```
/// use pricer_models::analytical::AnalyticalError;
///
/// let err = AnalyticalError::InvalidVolatility { volatility: -0.2 };
/// assert!(format!("{}", err).contains("volatility"));
/// ```
#[derive(Debug, Clone, Error, PartialEq)]
pub enum AnalyticalError {
    /// Invalid volatility (non-positive).
    #[error("Invalid volatility: σ = {volatility}")]
    InvalidVolatility {
        /// The invalid volatility value
        volatility: f64,
    },

    /// Invalid spot price (non-positive for Black-Scholes).
    #[error("Invalid spot price: S = {spot}")]
    InvalidSpot {
        /// The invalid spot price value
        spot: f64,
    },

    /// Unsupported exercise style.
    #[error("Unsupported exercise style: {style}")]
    UnsupportedExerciseStyle {
        /// Description of the unsupported exercise style
        style: String,
    },

    /// Numerical instability during computation.
    #[error("Numerical instability: {message}")]
    NumericalInstability {
        /// Description of the numerical issue
        message: String,
    },
}

impl From<AnalyticalError> for PricingError {
    fn from(err: AnalyticalError) -> Self {
        match err {
            AnalyticalError::InvalidVolatility { .. } | AnalyticalError::InvalidSpot { .. } => {
                PricingError::InvalidInput(err.to_string())
            }
            AnalyticalError::UnsupportedExerciseStyle { .. } => {
                PricingError::UnsupportedInstrument(err.to_string())
            }
            AnalyticalError::NumericalInstability { .. } => {
                PricingError::NumericalInstability(err.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================
    // RED Phase Tests - These tests verify AnalyticalError behaviour
    // ==========================================================

    #[test]
    fn test_invalid_volatility_display() {
        let err = AnalyticalError::InvalidVolatility { volatility: -0.2 };
        assert_eq!(format!("{}", err), "Invalid volatility: σ = -0.2");
    }

    #[test]
    fn test_invalid_spot_display() {
        let err = AnalyticalError::InvalidSpot { spot: -100.0 };
        assert_eq!(format!("{}", err), "Invalid spot price: S = -100");
    }

    #[test]
    fn test_unsupported_exercise_style_display() {
        let err = AnalyticalError::UnsupportedExerciseStyle {
            style: "American".to_string(),
        };
        assert_eq!(format!("{}", err), "Unsupported exercise style: American");
    }

    #[test]
    fn test_numerical_instability_display() {
        let err = AnalyticalError::NumericalInstability {
            message: "Division by zero in d1 calculation".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Numerical instability: Division by zero in d1 calculation"
        );
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = AnalyticalError::InvalidVolatility { volatility: 0.0 };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = AnalyticalError::InvalidVolatility { volatility: 0.1 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // ==========================================================
    // From<AnalyticalError> for PricingError tests
    // ==========================================================

    #[test]
    fn test_invalid_volatility_to_pricing_error() {
        let err = AnalyticalError::InvalidVolatility { volatility: -0.1 };
        let pricing_err: PricingError = err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => {
                assert!(msg.contains("volatility"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_invalid_spot_to_pricing_error() {
        let err = AnalyticalError::InvalidSpot { spot: -50.0 };
        let pricing_err: PricingError = err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => {
                assert!(msg.contains("spot"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_unsupported_exercise_to_pricing_error() {
        let err = AnalyticalError::UnsupportedExerciseStyle {
            style: "Bermudan".to_string(),
        };
        let pricing_err: PricingError = err.into();
        match pricing_err {
            PricingError::UnsupportedInstrument(msg) => {
                assert!(msg.contains("Bermudan"));
            }
            _ => panic!("Expected UnsupportedInstrument variant"),
        }
    }

    #[test]
    fn test_numerical_instability_to_pricing_error() {
        let err = AnalyticalError::NumericalInstability {
            message: "Overflow".to_string(),
        };
        let pricing_err: PricingError = err.into();
        match pricing_err {
            PricingError::NumericalInstability(msg) => {
                assert!(msg.contains("Overflow"));
            }
            _ => panic!("Expected NumericalInstability variant"),
        }
    }
}
