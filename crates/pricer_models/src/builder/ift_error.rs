//! IFT (Implicit Function Theorem) sensitivity error types.
//!
//! # Requirement: 3.4
//!
//! This module defines errors that can occur during IFT-based sensitivity
//! computation, which requires a cached Jacobian inverse from calibration.

use thiserror::Error;

// =============================================================================
// IFT Sensitivity Error
// =============================================================================

/// Errors that can occur during IFT (Implicit Function Theorem) sensitivity
/// computation.
///
/// IFT-based sensitivities require a cached Jacobian inverse from calibration.
/// These errors indicate when IFT computation cannot proceed.
///
/// # Requirement: 3.4
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum IftError {
    /// Jacobian inverse is not cached.
    ///
    /// IFT sensitivity requires J⁻¹ to be stored during calibration.
    /// Recalibrate with `store_jacobian_inverse=true`.
    #[error("Jacobian逆行列がキャッシュされていません。store_jacobian_inverse=trueで再キャリブレーションしてください")]
    NoJacobianInverse,

    /// Input vector dimension does not match expected size.
    ///
    /// The ∂F/∂m vector must have length equal to the number of instruments.
    #[error("次元不整合: 期待値 {expected}、実際値 {got}")]
    DimensionMismatch {
        /// Expected dimension (number of instruments/pillars)
        expected: usize,
        /// Actual dimension provided
        got: usize,
    },

    /// Batch input matrix has wrong dimensions.
    ///
    /// For batch sensitivity, the matrix must have n_instruments rows.
    #[error("バッチ入力の次元不整合: 行数 {expected}、実際値 {got}")]
    BatchDimensionMismatch {
        /// Expected number of rows (n_instruments)
        expected: usize,
        /// Actual number of rows provided
        got: usize,
    },

    /// Numerical error during IFT computation.
    ///
    /// Matrix-vector multiplication or other numerical operation failed.
    #[error("IFT計算中の数値エラー: {message}")]
    NumericalError {
        /// Description of the numerical issue
        message: String,
    },
}

impl IftError {
    /// Create a no Jacobian inverse error.
    pub fn no_jacobian_inverse() -> Self { IftError::NoJacobianInverse }

    /// Create a dimension mismatch error.
    pub fn dimension_mismatch(expected: usize, got: usize) -> Self {
        IftError::DimensionMismatch { expected, got }
    }

    /// Create a batch dimension mismatch error.
    pub fn batch_dimension_mismatch(expected: usize, got: usize) -> Self {
        IftError::BatchDimensionMismatch { expected, got }
    }

    /// Create a numerical error.
    pub fn numerical_error(message: impl Into<String>) -> Self {
        IftError::NumericalError {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // IftError Tests (Requirement 3.4)
    // =========================================================================

    #[test]
    fn test_ift_error_no_jacobian_inverse() {
        let err = IftError::no_jacobian_inverse();
        let msg = format!("{err}");
        assert!(msg.contains("Jacobian"));
        assert!(msg.contains("store_jacobian_inverse"));
        assert!(matches!(err, IftError::NoJacobianInverse));
    }

    #[test]
    fn test_ift_error_dimension_mismatch() {
        let err = IftError::dimension_mismatch(10, 5);
        let msg = format!("{err}");
        assert!(msg.contains("10"));
        assert!(msg.contains("5"));
        if let IftError::DimensionMismatch { expected, got } = err {
            assert_eq!(expected, 10);
            assert_eq!(got, 5);
        } else {
            panic!("Expected DimensionMismatch error");
        }
    }

    #[test]
    fn test_ift_error_batch_dimension_mismatch() {
        let err = IftError::batch_dimension_mismatch(20, 15);
        let msg = format!("{err}");
        assert!(msg.contains("20"));
        assert!(msg.contains("15"));
        if let IftError::BatchDimensionMismatch { expected, got } = err {
            assert_eq!(expected, 20);
            assert_eq!(got, 15);
        } else {
            panic!("Expected BatchDimensionMismatch error");
        }
    }

    #[test]
    fn test_ift_error_numerical_error() {
        let err = IftError::numerical_error("NaN detected");
        let msg = format!("{err}");
        assert!(msg.contains("NaN"));
        assert!(msg.contains("数値エラー"));
        if let IftError::NumericalError { message } = err {
            assert!(message.contains("NaN"));
        } else {
            panic!("Expected NumericalError error");
        }
    }

    #[test]
    fn test_ift_error_equality() {
        // IftError derives PartialEq and Eq
        let err1 = IftError::no_jacobian_inverse();
        let err2 = IftError::no_jacobian_inverse();
        assert_eq!(err1, err2);

        let err3 = IftError::dimension_mismatch(10, 5);
        let err4 = IftError::dimension_mismatch(10, 5);
        assert_eq!(err3, err4);

        let err5 = IftError::dimension_mismatch(10, 5);
        let err6 = IftError::dimension_mismatch(10, 6);
        assert_ne!(err5, err6);
    }
}
