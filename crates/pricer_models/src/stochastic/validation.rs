//! Common parameter validation utilities for stochastic models.
//!
//! This module provides:
//! - [`ParamValidationError`]: Unified parameter validation error type
//! - [`ComputationError`]: Common numerical computation errors
//! - Validation helper functions to reduce boilerplate
//!
//! # Design Philosophy
//!
//! Instead of each model defining its own `InvalidSpot`, `InvalidRho`, etc.,
//! this module provides a generic `ParamValidationError` that captures:
//! - Parameter name
//! - Invalid value
//! - Validation reason (e.g., "must be positive", "must be in range [-1, 1]")
//!
//! Models can use these common types directly or convert from them to
//! model-specific errors for backwards compatibility.
//!
//! # Example
//!
//! ```
//! use pricer_models::stochastic::validation::{ParamValidationError, validate_positive};
//!
//! // Validate a parameter
//! let result = validate_positive("spot", -100.0);
//! assert!(result.is_err());
//!
//! // Use in model construction
//! fn new_model(spot: f64) -> Result<(), ParamValidationError> {
//!     validate_positive("spot", spot)?;
//!     Ok(())
//! }
//! ```

use std::fmt;

use thiserror::Error;

// =============================================================================
// ParamValidationError - Unified parameter validation error
// =============================================================================

/// Unified parameter validation error.
///
/// Provides structured error information for parameter validation failures
/// with consistent formatting across all models.
///
/// # Fields
///
/// - `param`: Name of the invalid parameter (e.g., "spot", "rho", "kappa")
/// - `value`: The invalid value that was provided
/// - `constraint`: Human-readable description of the constraint
///
/// # Example
///
/// ```
/// use pricer_models::stochastic::validation::ParamValidationError;
///
/// let err = ParamValidationError::new("spot", -100.0, "must be positive");
/// assert!(err.to_string().contains("spot"));
/// assert!(err.to_string().contains("-100"));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ParamValidationError {
    /// Parameter name
    pub param: &'static str,
    /// Invalid value
    pub value: f64,
    /// Constraint description
    pub constraint: &'static str,
}

impl ParamValidationError {
    /// Create a new parameter validation error.
    #[must_use]
    pub const fn new(param: &'static str, value: f64, constraint: &'static str) -> Self {
        Self {
            param,
            value,
            constraint,
        }
    }

    /// Create error for a parameter that must be positive.
    #[must_use]
    pub const fn must_be_positive(param: &'static str, value: f64) -> Self {
        Self::new(param, value, "must be positive")
    }

    /// Create error for a parameter that must be non-negative.
    #[must_use]
    pub const fn must_be_non_negative(param: &'static str, value: f64) -> Self {
        Self::new(param, value, "must be non-negative")
    }

    /// Create error for a parameter that must be in range [-1, 1].
    #[must_use]
    pub const fn must_be_correlation(param: &'static str, value: f64) -> Self {
        Self::new(param, value, "must be in range [-1, 1]")
    }

    /// Create error for a parameter that must be in open interval (-1, 1).
    #[must_use]
    pub const fn must_be_strict_correlation(param: &'static str, value: f64) -> Self {
        Self::new(param, value, "must be in range (-1, 1)")
    }

    /// Create error for a parameter that must be in range [0, 1].
    #[must_use]
    pub const fn must_be_in_unit_interval(param: &'static str, value: f64) -> Self {
        Self::new(param, value, "must be in range [0, 1]")
    }
}

impl fmt::Display for ParamValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid {}: {} ({})",
            self.param, self.value, self.constraint
        )
    }
}

impl std::error::Error for ParamValidationError {}

// =============================================================================
// ComputationError - Common numerical computation errors
// =============================================================================

/// Common numerical computation errors.
///
/// Provides unified error handling for numerical issues that can occur
/// across different models during computation.
///
/// # Variants
///
/// - `NumericalInstability`: General numerical issues (overflow, underflow,
///   etc.)
/// - `NonFinite`: NaN or infinity detected in computation
/// - `ConvergenceFailure`: Iterative algorithm failed to converge
///
/// # Example
///
/// ```
/// use pricer_models::stochastic::validation::ComputationError;
///
/// let err = ComputationError::non_finite("variance calculation");
/// assert!(err.to_string().contains("NaN"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ComputationError {
    /// Numerical instability detected during computation.
    #[error("Numerical instability: {0}")]
    NumericalInstability(String),

    /// NaN or infinity detected in computation result.
    #[error("Non-finite value (NaN/Inf) detected in {0}")]
    NonFinite(String),

    /// Iterative algorithm failed to converge.
    #[error("Convergence failure after {iterations} iterations (residual: {residual})")]
    ConvergenceFailure {
        /// Number of iterations attempted
        iterations: usize,
        /// Final residual value
        residual: f64,
    },
}

impl ComputationError {
    /// Create a numerical instability error.
    #[must_use]
    pub fn numerical_instability(message: impl Into<String>) -> Self {
        Self::NumericalInstability(message.into())
    }

    /// Create a non-finite value error.
    #[must_use]
    pub fn non_finite(context: impl Into<String>) -> Self { Self::NonFinite(context.into()) }

    /// Create a convergence failure error.
    #[must_use]
    pub const fn convergence_failure(iterations: usize, residual: f64) -> Self {
        Self::ConvergenceFailure {
            iterations,
            residual,
        }
    }
}

// =============================================================================
// Validation helper functions
// =============================================================================

/// Validate that a parameter is positive (> 0).
///
/// # Example
///
/// ```
/// use pricer_models::stochastic::validation::validate_positive;
///
/// assert!(validate_positive("spot", 100.0).is_ok());
/// assert!(validate_positive("spot", 0.0).is_err());
/// assert!(validate_positive("spot", -1.0).is_err());
/// ```
#[inline]
pub fn validate_positive(param: &'static str, value: f64) -> Result<(), ParamValidationError> {
    if value > 0.0 {
        Ok(())
    } else {
        Err(ParamValidationError::must_be_positive(param, value))
    }
}

/// Validate that a parameter is non-negative (>= 0).
///
/// # Example
///
/// ```
/// use pricer_models::stochastic::validation::validate_non_negative;
///
/// assert!(validate_non_negative("nu", 0.0).is_ok());
/// assert!(validate_non_negative("nu", 0.5).is_ok());
/// assert!(validate_non_negative("nu", -0.1).is_err());
/// ```
#[inline]
pub fn validate_non_negative(param: &'static str, value: f64) -> Result<(), ParamValidationError> {
    if value >= 0.0 {
        Ok(())
    } else {
        Err(ParamValidationError::must_be_non_negative(param, value))
    }
}

/// Validate that a parameter is in the closed interval [-1, 1].
///
/// # Example
///
/// ```
/// use pricer_models::stochastic::validation::validate_correlation;
///
/// assert!(validate_correlation("rho", 0.5).is_ok());
/// assert!(validate_correlation("rho", -1.0).is_ok());
/// assert!(validate_correlation("rho", 1.0).is_ok());
/// assert!(validate_correlation("rho", 1.1).is_err());
/// ```
#[inline]
pub fn validate_correlation(param: &'static str, value: f64) -> Result<(), ParamValidationError> {
    if (-1.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ParamValidationError::must_be_correlation(param, value))
    }
}

/// Validate that a parameter is in the open interval (-1, 1).
///
/// Used for parameters like SABR rho where boundary values cause singularities.
///
/// # Example
///
/// ```
/// use pricer_models::stochastic::validation::validate_strict_correlation;
///
/// assert!(validate_strict_correlation("rho", 0.5).is_ok());
/// assert!(validate_strict_correlation("rho", -0.99).is_ok());
/// assert!(validate_strict_correlation("rho", -1.0).is_err());
/// assert!(validate_strict_correlation("rho", 1.0).is_err());
/// ```
#[inline]
pub fn validate_strict_correlation(
    param: &'static str,
    value: f64,
) -> Result<(), ParamValidationError> {
    if value > -1.0 && value < 1.0 {
        Ok(())
    } else {
        Err(ParamValidationError::must_be_strict_correlation(
            param, value,
        ))
    }
}

/// Validate that a parameter is in the closed interval [0, 1].
///
/// # Example
///
/// ```
/// use pricer_models::stochastic::validation::validate_unit_interval;
///
/// assert!(validate_unit_interval("beta", 0.5).is_ok());
/// assert!(validate_unit_interval("beta", 0.0).is_ok());
/// assert!(validate_unit_interval("beta", 1.0).is_ok());
/// assert!(validate_unit_interval("beta", 1.1).is_err());
/// ```
#[inline]
pub fn validate_unit_interval(param: &'static str, value: f64) -> Result<(), ParamValidationError> {
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ParamValidationError::must_be_in_unit_interval(param, value))
    }
}

/// Validate that a value is finite (not NaN or infinity).
///
/// # Example
///
/// ```
/// use pricer_models::stochastic::validation::validate_finite;
///
/// assert!(validate_finite("result", 1.0).is_ok());
/// assert!(validate_finite("result", f64::NAN).is_err());
/// assert!(validate_finite("result", f64::INFINITY).is_err());
/// ```
#[inline]
pub fn validate_finite(context: &'static str, value: f64) -> Result<(), ComputationError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ComputationError::non_finite(context))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // ParamValidationError tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_param_validation_error_display() {
        let err = ParamValidationError::new("spot", -100.0, "must be positive");
        let msg = err.to_string();
        assert!(msg.contains("spot"));
        assert!(msg.contains("-100"));
        assert!(msg.contains("must be positive"));
    }

    #[test]
    fn test_param_validation_error_must_be_positive() {
        let err = ParamValidationError::must_be_positive("v0", 0.0);
        assert_eq!(err.param, "v0");
        assert_eq!(err.value, 0.0);
        assert_eq!(err.constraint, "must be positive");
    }

    #[test]
    fn test_param_validation_error_must_be_correlation() {
        let err = ParamValidationError::must_be_correlation("rho", 1.5);
        assert!(err.to_string().contains("[-1, 1]"));
    }

    // -------------------------------------------------------------------------
    // ComputationError tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_computation_error_numerical_instability() {
        let err = ComputationError::numerical_instability("overflow in exp");
        assert!(err.to_string().contains("Numerical instability"));
        assert!(err.to_string().contains("overflow"));
    }

    #[test]
    fn test_computation_error_non_finite() {
        let err = ComputationError::non_finite("variance calculation");
        assert!(err.to_string().contains("NaN"));
        assert!(err.to_string().contains("variance"));
    }

    #[test]
    fn test_computation_error_convergence_failure() {
        let err = ComputationError::convergence_failure(100, 1e-6);
        let msg = err.to_string();
        assert!(msg.contains("100"));
        assert!(msg.contains("iterations"));
    }

    // -------------------------------------------------------------------------
    // Validation function tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_validate_positive() {
        assert!(validate_positive("x", 1.0).is_ok());
        assert!(validate_positive("x", 0.001).is_ok());
        assert!(validate_positive("x", 0.0).is_err());
        assert!(validate_positive("x", -1.0).is_err());
    }

    #[test]
    fn test_validate_non_negative() {
        assert!(validate_non_negative("x", 1.0).is_ok());
        assert!(validate_non_negative("x", 0.0).is_ok());
        assert!(validate_non_negative("x", -0.001).is_err());
    }

    #[test]
    fn test_validate_correlation() {
        assert!(validate_correlation("rho", 0.0).is_ok());
        assert!(validate_correlation("rho", 1.0).is_ok());
        assert!(validate_correlation("rho", -1.0).is_ok());
        assert!(validate_correlation("rho", 0.99).is_ok());
        assert!(validate_correlation("rho", -0.99).is_ok());
        assert!(validate_correlation("rho", 1.01).is_err());
        assert!(validate_correlation("rho", -1.01).is_err());
    }

    #[test]
    fn test_validate_strict_correlation() {
        assert!(validate_strict_correlation("rho", 0.0).is_ok());
        assert!(validate_strict_correlation("rho", 0.99).is_ok());
        assert!(validate_strict_correlation("rho", -0.99).is_ok());
        assert!(validate_strict_correlation("rho", 1.0).is_err());
        assert!(validate_strict_correlation("rho", -1.0).is_err());
    }

    #[test]
    fn test_validate_unit_interval() {
        assert!(validate_unit_interval("beta", 0.0).is_ok());
        assert!(validate_unit_interval("beta", 0.5).is_ok());
        assert!(validate_unit_interval("beta", 1.0).is_ok());
        assert!(validate_unit_interval("beta", -0.1).is_err());
        assert!(validate_unit_interval("beta", 1.1).is_err());
    }

    #[test]
    fn test_validate_finite() {
        assert!(validate_finite("result", 1.0).is_ok());
        assert!(validate_finite("result", 0.0).is_ok());
        assert!(validate_finite("result", -1e308).is_ok());
        assert!(validate_finite("result", f64::NAN).is_err());
        assert!(validate_finite("result", f64::INFINITY).is_err());
        assert!(validate_finite("result", f64::NEG_INFINITY).is_err());
    }

    // -------------------------------------------------------------------------
    // Error trait implementation tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_param_validation_error_is_error() {
        let err = ParamValidationError::must_be_positive("x", -1.0);
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_computation_error_is_error() {
        let err = ComputationError::non_finite("test");
        let _: &dyn std::error::Error = &err;
    }
}
