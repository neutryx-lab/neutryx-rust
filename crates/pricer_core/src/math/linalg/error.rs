//! Error types for linear algebra operations.

use thiserror::Error;

/// Errors that can occur during linear algebra operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum LinearAlgebraError {
    /// Matrix is not positive definite (required for Cholesky decomposition).
    #[error("Matrix is not positive definite")]
    NotPositiveDefinite,

    /// Matrix is singular (no inverse exists).
    #[error("Matrix is singular")]
    SingularMatrix,

    /// Matrix dimensions do not match for the operation.
    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch {
        /// Expected dimension description.
        expected: String,
        /// Actual dimension description.
        got: String,
    },

    /// Matrix is not square (required for some operations).
    #[error("Matrix is not square: {rows}x{cols}")]
    NotSquare {
        /// Number of rows.
        rows: usize,
        /// Number of columns.
        cols: usize,
    },

    /// Decomposition algorithm failed.
    #[error("Decomposition failed: {0}")]
    DecompositionFailed(String),

    /// Invalid input provided.
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_positive_definite_error() {
        let err = LinearAlgebraError::NotPositiveDefinite;
        assert_eq!(format!("{err}"), "Matrix is not positive definite");
    }

    #[test]
    fn test_singular_matrix_error() {
        let err = LinearAlgebraError::SingularMatrix;
        assert_eq!(format!("{err}"), "Matrix is singular");
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let err = LinearAlgebraError::DimensionMismatch {
            expected: "3x3".to_string(),
            got: "2x3".to_string(),
        };
        assert_eq!(
            format!("{err}"),
            "Dimension mismatch: expected 3x3, got 2x3"
        );
    }

    #[test]
    fn test_not_square_error() {
        let err = LinearAlgebraError::NotSquare { rows: 2, cols: 3 };
        assert_eq!(format!("{err}"), "Matrix is not square: 2x3");
    }

    #[test]
    fn test_decomposition_failed_error() {
        let err = LinearAlgebraError::DecompositionFailed("QR failed".to_string());
        assert_eq!(format!("{err}"), "Decomposition failed: QR failed");
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = LinearAlgebraError::NotPositiveDefinite;
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}
