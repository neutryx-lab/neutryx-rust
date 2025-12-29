//! Error types for structured error handling.
//!
//! This module provides:
//! - `PricingError`: Errors from pricing operations
//! - `DateError`: Errors from date construction and parsing
//! - `CurrencyError`: Errors from currency parsing
//! - `InterpolationError`: Errors from interpolation operations
//! - `SolverError`: Errors from root-finding solvers

use std::fmt;
use thiserror::Error;

/// Categorised pricing errors.
///
/// Provides structured error handling for pricing operations with
/// descriptive context for each failure mode.
///
/// # Variants
/// - `InvalidInput`: Invalid market data or parameters
/// - `NumericalInstability`: Computation failed to converge
/// - `ModelFailure`: Model assumptions violated
/// - `UnsupportedInstrument`: Instrument type not supported by model
///
/// # Examples
/// ```
/// use pricer_core::types::PricingError;
///
/// let err = PricingError::InvalidInput("Negative spot price".to_string());
/// assert_eq!(format!("{}", err), "Invalid input: Negative spot price");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PricingError {
    /// Invalid input data or parameters
    InvalidInput(String),

    /// Numerical instability during computation
    NumericalInstability(String),

    /// Model failed to produce valid result
    ModelFailure(String),

    /// Instrument type not supported
    UnsupportedInstrument(String),
}

impl fmt::Display for PricingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PricingError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            PricingError::NumericalInstability(msg) => {
                write!(f, "Numerical instability: {}", msg)
            }
            PricingError::ModelFailure(msg) => write!(f, "Model failure: {}", msg),
            PricingError::UnsupportedInstrument(msg) => {
                write!(f, "Unsupported instrument: {}", msg)
            }
        }
    }
}

impl std::error::Error for PricingError {}

/// Date-related errors.
///
/// Provides structured error handling for date construction and parsing
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `InvalidDate`: Invalid date components (e.g., February 30th)
/// - `ParseError`: Failed to parse date string
///
/// # Examples
/// ```
/// use pricer_core::types::DateError;
///
/// let err = DateError::InvalidDate { year: 2024, month: 2, day: 30 };
/// assert_eq!(format!("{}", err), "Invalid date: 2024-2-30");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateError {
    /// Invalid date components (e.g., February 30th).
    InvalidDate {
        /// Year component
        year: i32,
        /// Month component (1-12)
        month: u32,
        /// Day component (1-31)
        day: u32,
    },

    /// Failed to parse date string.
    ParseError(String),
}

impl fmt::Display for DateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DateError::InvalidDate { year, month, day } => {
                write!(f, "Invalid date: {}-{}-{}", year, month, day)
            }
            DateError::ParseError(msg) => write!(f, "Date parse error: {}", msg),
        }
    }
}

impl std::error::Error for DateError {}

/// Currency-related errors.
///
/// Provides structured error handling for currency parsing
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `UnknownCurrency`: Unknown currency code
/// - `ParseError`: Failed to parse currency string
///
/// # Examples
/// ```
/// use pricer_core::types::CurrencyError;
///
/// let err = CurrencyError::UnknownCurrency("XYZ".to_string());
/// assert_eq!(format!("{}", err), "Unknown currency: XYZ");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrencyError {
    /// Unknown currency code.
    UnknownCurrency(String),

    /// Failed to parse currency string.
    ParseError(String),
}

impl fmt::Display for CurrencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CurrencyError::UnknownCurrency(code) => write!(f, "Unknown currency: {}", code),
            CurrencyError::ParseError(msg) => write!(f, "Currency parse error: {}", msg),
        }
    }
}

impl std::error::Error for CurrencyError {}

/// Interpolation-related errors.
///
/// Provides structured error handling for interpolation operations
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `OutOfBounds`: Query point outside valid interpolation domain
/// - `InsufficientData`: Not enough data points for interpolation
/// - `NonMonotonicData`: Data violates monotonicity requirement
/// - `InvalidInput`: General invalid input error
///
/// # Examples
/// ```
/// use pricer_core::types::InterpolationError;
///
/// let err = InterpolationError::OutOfBounds { x: 5.0, min: 0.0, max: 3.0 };
/// assert!(format!("{}", err).contains("outside valid domain"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InterpolationError {
    /// Query point outside valid interpolation domain.
    #[error("Query point {x} outside valid domain [{min}, {max}]")]
    OutOfBounds {
        /// The query point that was out of bounds
        x: f64,
        /// Minimum valid value
        min: f64,
        /// Maximum valid value
        max: f64,
    },

    /// Insufficient data points for interpolation.
    #[error("Insufficient data points: got {got}, need at least {need}")]
    InsufficientData {
        /// Number of points provided
        got: usize,
        /// Minimum number of points required
        need: usize,
    },

    /// Data is not monotonic when monotonicity is required.
    #[error("Data is not monotonic at index {index}")]
    NonMonotonicData {
        /// Index where monotonicity violation was detected
        index: usize,
    },

    /// Invalid input data or parameters.
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Root-finding solver errors.
///
/// Provides structured error handling for root-finding solver operations
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `MaxIterationsExceeded`: Solver failed to converge within iteration limit
/// - `DerivativeNearZero`: Derivative too small for Newton-Raphson
/// - `NoBracket`: Function values at bracket endpoints have same sign
/// - `NumericalInstability`: General numerical instability
///
/// # Examples
/// ```
/// use pricer_core::types::SolverError;
///
/// let err = SolverError::MaxIterationsExceeded { iterations: 100 };
/// assert!(format!("{}", err).contains("100 iterations"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SolverError {
    /// Solver failed to converge within maximum iterations.
    #[error("Failed to converge after {iterations} iterations")]
    MaxIterationsExceeded {
        /// Number of iterations attempted
        iterations: usize,
    },

    /// Derivative near zero (division by zero risk in Newton-Raphson).
    #[error("Derivative near zero at x = {x}")]
    DerivativeNearZero {
        /// The x value where derivative was near zero
        x: f64,
    },

    /// No valid bracket (function values at endpoints have same sign).
    #[error("No bracket: f({a}) and f({b}) have same sign")]
    NoBracket {
        /// Left bracket endpoint
        a: f64,
        /// Right bracket endpoint
        b: f64,
    },

    /// Numerical instability during computation.
    #[error("Numerical instability: {0}")]
    NumericalInstability(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_input_display() {
        let err = PricingError::InvalidInput("Test error".to_string());
        assert_eq!(format!("{}", err), "Invalid input: Test error");
    }

    #[test]
    fn test_numerical_instability_display() {
        let err = PricingError::NumericalInstability("Failed to converge".to_string());
        assert_eq!(
            format!("{}", err),
            "Numerical instability: Failed to converge"
        );
    }

    #[test]
    fn test_model_failure_display() {
        let err = PricingError::ModelFailure("Volatility out of range".to_string());
        assert_eq!(format!("{}", err), "Model failure: Volatility out of range");
    }

    #[test]
    fn test_unsupported_instrument_display() {
        let err = PricingError::UnsupportedInstrument("Asian option".to_string());
        assert_eq!(format!("{}", err), "Unsupported instrument: Asian option");
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = PricingError::InvalidInput("Test".to_string());
        let _: &dyn std::error::Error = &err; // Verify Error trait is implemented
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = PricingError::InvalidInput("Test".to_string());
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // DateError tests

    #[test]
    fn test_date_error_invalid_date_display() {
        let err = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        assert_eq!(format!("{}", err), "Invalid date: 2024-2-30");
    }

    #[test]
    fn test_date_error_parse_error_display() {
        let err = DateError::ParseError("invalid format".to_string());
        assert_eq!(format!("{}", err), "Date parse error: invalid format");
    }

    #[test]
    fn test_date_error_trait_implementation() {
        let err = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_date_error_clone_and_equality() {
        let err1 = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // CurrencyError tests

    #[test]
    fn test_currency_error_unknown_currency_display() {
        let err = CurrencyError::UnknownCurrency("XYZ".to_string());
        assert_eq!(format!("{}", err), "Unknown currency: XYZ");
    }

    #[test]
    fn test_currency_error_parse_error_display() {
        let err = CurrencyError::ParseError("invalid input".to_string());
        assert_eq!(format!("{}", err), "Currency parse error: invalid input");
    }

    #[test]
    fn test_currency_error_trait_implementation() {
        let err = CurrencyError::UnknownCurrency("XYZ".to_string());
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_currency_error_clone_and_equality() {
        let err1 = CurrencyError::UnknownCurrency("XYZ".to_string());
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // InterpolationError tests

    #[test]
    fn test_interpolation_error_out_of_bounds_display() {
        let err = InterpolationError::OutOfBounds {
            x: 5.0,
            min: 0.0,
            max: 3.0,
        };
        assert_eq!(
            format!("{}", err),
            "Query point 5 outside valid domain [0, 3]"
        );
    }

    #[test]
    fn test_interpolation_error_insufficient_data_display() {
        let err = InterpolationError::InsufficientData { got: 1, need: 2 };
        assert_eq!(
            format!("{}", err),
            "Insufficient data points: got 1, need at least 2"
        );
    }

    #[test]
    fn test_interpolation_error_non_monotonic_display() {
        let err = InterpolationError::NonMonotonicData { index: 3 };
        assert_eq!(format!("{}", err), "Data is not monotonic at index 3");
    }

    #[test]
    fn test_interpolation_error_invalid_input_display() {
        let err = InterpolationError::InvalidInput("empty array".to_string());
        assert_eq!(format!("{}", err), "Invalid input: empty array");
    }

    #[test]
    fn test_interpolation_error_trait_implementation() {
        let err = InterpolationError::OutOfBounds {
            x: 5.0,
            min: 0.0,
            max: 3.0,
        };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_interpolation_error_clone_and_equality() {
        let err1 = InterpolationError::InsufficientData { got: 1, need: 2 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // SolverError tests

    #[test]
    fn test_solver_error_max_iterations_display() {
        let err = SolverError::MaxIterationsExceeded { iterations: 100 };
        assert_eq!(
            format!("{}", err),
            "Failed to converge after 100 iterations"
        );
    }

    #[test]
    fn test_solver_error_derivative_near_zero_display() {
        let err = SolverError::DerivativeNearZero { x: 1.5 };
        assert_eq!(format!("{}", err), "Derivative near zero at x = 1.5");
    }

    #[test]
    fn test_solver_error_no_bracket_display() {
        let err = SolverError::NoBracket { a: 0.0, b: 1.0 };
        assert_eq!(
            format!("{}", err),
            "No bracket: f(0) and f(1) have same sign"
        );
    }

    #[test]
    fn test_solver_error_numerical_instability_display() {
        let err = SolverError::NumericalInstability("overflow detected".to_string());
        assert_eq!(
            format!("{}", err),
            "Numerical instability: overflow detected"
        );
    }

    #[test]
    fn test_solver_error_trait_implementation() {
        let err = SolverError::MaxIterationsExceeded { iterations: 100 };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_solver_error_clone_and_equality() {
        let err1 = SolverError::NoBracket { a: 0.0, b: 1.0 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // Serde tests (feature-gated)
    #[cfg(feature = "serde")]
    mod serde_tests {
        use super::*;

        #[test]
        fn test_interpolation_error_serde_roundtrip() {
            let err = InterpolationError::OutOfBounds {
                x: 5.0,
                min: 0.0,
                max: 3.0,
            };
            let json = serde_json::to_string(&err).unwrap();
            let deserialized: InterpolationError = serde_json::from_str(&json).unwrap();
            assert_eq!(err, deserialized);
        }

        #[test]
        fn test_solver_error_serde_roundtrip() {
            let err = SolverError::MaxIterationsExceeded { iterations: 100 };
            let json = serde_json::to_string(&err).unwrap();
            let deserialized: SolverError = serde_json::from_str(&json).unwrap();
            assert_eq!(err, deserialized);
        }
    }
}
