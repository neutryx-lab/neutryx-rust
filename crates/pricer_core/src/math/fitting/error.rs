//! Error types for fitting operations.

use thiserror::Error;

/// Errors that can occur during fitting operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum FittingError {
    /// Not enough data points for fitting.
    #[error("Insufficient data: need at least {needed}, got {got}")]
    InsufficientData {
        /// Minimum number of points needed.
        needed: usize,
        /// Actual number of points provided.
        got: usize,
    },

    /// Dimensions of input arrays don't match.
    #[error("Dimension mismatch: {0}")]
    DimensionMismatch(String),

    /// The fitting problem is ill-conditioned or singular.
    #[error("Fitting failed: {0}")]
    FittingFailed(String),

    /// Invalid input data.
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Numerical issue during computation.
    #[error("Numerical error: {0}")]
    NumericalError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insufficient_data_error() {
        let err = FittingError::InsufficientData { needed: 3, got: 2 };
        assert_eq!(
            format!("{err}"),
            "Insufficient data: need at least 3, got 2"
        );
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let err = FittingError::DimensionMismatch("x and y have different lengths".to_string());
        assert_eq!(
            format!("{err}"),
            "Dimension mismatch: x and y have different lengths"
        );
    }

    #[test]
    fn test_fitting_failed_error() {
        let err = FittingError::FittingFailed("matrix is singular".to_string());
        assert_eq!(format!("{err}"), "Fitting failed: matrix is singular");
    }

    #[test]
    fn test_invalid_data_error() {
        let err = FittingError::InvalidData("negative weights".to_string());
        assert_eq!(format!("{err}"), "Invalid data: negative weights");
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = FittingError::InsufficientData { needed: 3, got: 2 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}
