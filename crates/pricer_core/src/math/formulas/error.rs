//! Error types for pricing formula calculations.
//!
//! This module provides error types and shared validation helpers
//! for closed-form pricing formulas.

use num_traits::Float;
use thiserror::Error;

/// Formula calculation errors.
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
    InvalidVolatility { volatility: f64 },

    /// Invalid spot price (non-positive).
    #[error("Invalid spot price: S = {spot}")]
    InvalidSpot { spot: f64 },

    /// Invalid strike price (non-positive).
    #[error("Invalid strike price: K = {strike}")]
    InvalidStrike { strike: f64 },

    /// Invalid expiry (non-positive).
    #[error("Invalid expiry: T = {expiry}")]
    InvalidExpiry { expiry: f64 },

    /// Numerical instability during computation.
    #[error("Numerical instability: {message}")]
    NumericalInstability { message: String },
}

// ── Shared validation helpers ────────────────────────────────────

/// Validate that spot > 0.
pub fn require_positive_spot<T: Float>(spot: T) -> Result<(), FormulaError> {
    if spot > T::zero() { Ok(()) }
    else { Err(FormulaError::InvalidSpot { spot: spot.to_f64().unwrap_or(0.0) }) }
}

/// Validate that strike > 0.
pub fn require_positive_strike<T: Float>(strike: T) -> Result<(), FormulaError> {
    if strike > T::zero() { Ok(()) }
    else { Err(FormulaError::InvalidStrike { strike: strike.to_f64().unwrap_or(0.0) }) }
}

/// Validate that volatility > 0.
pub fn require_positive_vol<T: Float>(volatility: T) -> Result<(), FormulaError> {
    if volatility > T::zero() { Ok(()) }
    else { Err(FormulaError::InvalidVolatility { volatility: volatility.to_f64().unwrap_or(0.0) }) }
}

/// Validate that expiry > 0.
pub fn require_positive_expiry<T: Float>(expiry: T) -> Result<(), FormulaError> {
    if expiry > T::zero() { Ok(()) }
    else { Err(FormulaError::InvalidExpiry { expiry: expiry.to_f64().unwrap_or(0.0) }) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        assert_eq!(format!("{}", FormulaError::InvalidVolatility { volatility: -0.2 }),
            "Invalid volatility: σ = -0.2");
        assert_eq!(format!("{}", FormulaError::InvalidSpot { spot: -100.0 }),
            "Invalid spot price: S = -100");
        assert_eq!(format!("{}", FormulaError::InvalidStrike { strike: -50.0 }),
            "Invalid strike price: K = -50");
        assert_eq!(format!("{}", FormulaError::InvalidExpiry { expiry: -1.0 }),
            "Invalid expiry: T = -1");
        assert_eq!(format!("{}", FormulaError::NumericalInstability { message: "Division by zero".into() }),
            "Numerical instability: Division by zero");
    }

    #[test]
    fn clone_and_equality() {
        let err = FormulaError::InvalidVolatility { volatility: 0.1 };
        assert_eq!(err.clone(), err);
    }

    #[test]
    fn validation_helpers() {
        assert!(require_positive_spot(1.0_f64).is_ok());
        assert!(require_positive_spot(-1.0_f64).is_err());
        assert!(require_positive_spot(0.0_f64).is_err());
        assert!(require_positive_strike(1.0_f64).is_ok());
        assert!(require_positive_strike(-1.0_f64).is_err());
        assert!(require_positive_vol(0.2_f64).is_ok());
        assert!(require_positive_vol(0.0_f64).is_err());
        assert!(require_positive_expiry(1.0_f64).is_ok());
        assert!(require_positive_expiry(-0.5_f64).is_err());
    }
}
