//! Error types for optimisation operations.

use thiserror::Error;

/// Errors that can occur during optimisation.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum OptimisationError {
    /// Optimisation did not converge within maximum iterations.
    #[error("Optimisation did not converge after {iterations} iterations")]
    NotConverged {
        /// Number of iterations attempted.
        iterations: usize,
    },

    /// Invalid input parameters.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Numerical error during optimisation.
    #[error("Numerical error: {0}")]
    NumericalError(String),

    /// Dimension mismatch.
    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch {
        /// Expected dimension.
        expected: usize,
        /// Actual dimension.
        got: usize,
    },

    /// Bounds error.
    #[error("Bounds error: {0}")]
    BoundsError(String),

    /// Gradient computation failed.
    #[error("Gradient computation failed: {0}")]
    GradientError(String),

    /// Line search failed.
    #[error("Line search failed: {0}")]
    LineSearchError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_converged_error() {
        let err = OptimisationError::NotConverged { iterations: 100 };
        assert!(format!("{}", err).contains("100"));
        assert!(format!("{}", err).contains("converge"));
    }

    #[test]
    fn test_invalid_input_error() {
        let err = OptimisationError::InvalidInput("negative value".to_string());
        assert!(format!("{}", err).contains("negative"));
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let err = OptimisationError::DimensionMismatch {
            expected: 3,
            got: 5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("3"));
        assert!(msg.contains("5"));
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = OptimisationError::NotConverged { iterations: 50 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}
