//! Error types for pricing formula calculations.
//!
//! This module provides error types for closed-form pricing formulas.
//! These are pure mathematical errors without dependencies on higher-level
//! pricing infrastructure.

use thiserror::Error;

/// Formula calculation errors.
///
/// Provides structured error handling for closed-form pricing formulas
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `InvalidVolatility`: Non-positive volatility
/// - `InvalidSpot`: Non-positive spot price
/// - `InvalidExpiry`: Non-positive time to expiry
/// - `NumericalInstability`: Computation encountered numerical issues
///
/// # Examples
/// ```
/// use pricer_core::math::formulas::FormulaError;
///
/// let err = FormulaError::InvalidVolatility { volatility: -0.2 };
/// assert!(format!("{}", err).contains("volatility"));
/// ```
#[derive(Debug, Clone, Error, PartialEq)]
pub enum FormulaError {
    /// Invalid volatility (non-positive).
    #[error("Invalid volatility: σ = {volatility}")]
    InvalidVolatility {
        /// The invalid volatility value
        volatility: f64,
    },

    /// Invalid spot price (non-positive).
    #[error("Invalid spot price: S = {spot}")]
    InvalidSpot {
        /// The invalid spot price value
        spot: f64,
    },

    /// Invalid expiry (non-positive).
    #[error("Invalid expiry: T = {expiry}")]
    InvalidExpiry {
        /// The invalid expiry value
        expiry: f64,
    },

    /// Numerical instability during computation.
    #[error("Numerical instability: {message}")]
    NumericalInstability {
        /// Description of the numerical issue
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_volatility_display() {
        let err = FormulaError::InvalidVolatility { volatility: -0.2 };
        assert_eq!(format!("{}", err), "Invalid volatility: σ = -0.2");
    }

    #[test]
    fn test_invalid_spot_display() {
        let err = FormulaError::InvalidSpot { spot: -100.0 };
        assert_eq!(format!("{}", err), "Invalid spot price: S = -100");
    }

    #[test]
    fn test_invalid_expiry_display() {
        let err = FormulaError::InvalidExpiry { expiry: -1.0 };
        assert_eq!(format!("{}", err), "Invalid expiry: T = -1");
    }

    #[test]
    fn test_numerical_instability_display() {
        let err = FormulaError::NumericalInstability {
            message: "Division by zero".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Numerical instability: Division by zero"
        );
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = FormulaError::InvalidVolatility { volatility: 0.1 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}
