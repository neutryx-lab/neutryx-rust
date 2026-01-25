//! Error types for IRS Greeks calculation.
//!
//! # Migration Note
//!
//! [`IrsGreeksError`] is maintained for backward compatibility. New code
//! can use [`crate::greeks::GreeksError`] which provides a unified error
//! hierarchy for all Greeks operations.

use thiserror::Error;

/// IRS Greeks calculation error.
///
/// This error type handles IRS-specific Greeks calculation failures.
/// For a unified error type, see [`crate::greeks::GreeksError`].
#[derive(Debug, Error, Clone, PartialEq)]
pub enum IrsGreeksError {
    /// Invalid swap parameters.
    #[error("Invalid swap parameters: {0}")]
    InvalidSwap(String),

    /// Curve not found in curve set.
    #[error("Curve not found: {0}")]
    CurveNotFound(String),

    /// AAD computation failed.
    #[error("AAD computation failed: {0}")]
    AadFailed(String),

    /// Accuracy check failed between AAD and bump-and-revalue.
    #[error("Accuracy check failed: max relative error {0} exceeds tolerance {1}")]
    AccuracyCheckFailed(f64, f64),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

// =============================================================================
// Conversion to unified GreeksError
// =============================================================================

#[allow(deprecated)]
impl From<IrsGreeksError> for crate::greeks::GreeksError {
    fn from(err: IrsGreeksError) -> Self {
        match err {
            IrsGreeksError::InvalidSwap(msg) => Self::InvalidSwap(msg),
            IrsGreeksError::CurveNotFound(name) => Self::CurveNotFound(name),
            IrsGreeksError::AadFailed(msg) => Self::AadFailed(msg),
            IrsGreeksError::AccuracyCheckFailed(err, tol) => Self::AccuracyCheckFailed(err, tol),
            IrsGreeksError::InvalidConfig(msg) => Self::InvalidConfig(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(deprecated)]
    use crate::greeks::GreeksError;

    #[test]
    fn test_from_irs_greeks_error_to_greeks_error() {
        let err = IrsGreeksError::InvalidSwap("missing notional".to_string());
        let greeks_err: GreeksError = err.into();
        assert!(matches!(greeks_err, GreeksError::InvalidSwap(_)));
    }

    #[test]
    fn test_from_curve_not_found() {
        let err = IrsGreeksError::CurveNotFound("SOFR".to_string());
        let greeks_err: GreeksError = err.into();
        assert!(matches!(greeks_err, GreeksError::CurveNotFound(_)));
    }

    #[test]
    fn test_from_aad_failed() {
        let err = IrsGreeksError::AadFailed("tape overflow".to_string());
        let greeks_err: GreeksError = err.into();
        assert!(matches!(greeks_err, GreeksError::AadFailed(_)));
    }

    #[test]
    fn test_from_accuracy_check_failed() {
        let err = IrsGreeksError::AccuracyCheckFailed(0.01, 1e-6);
        let greeks_err: GreeksError = err.into();
        assert!(matches!(greeks_err, GreeksError::AccuracyCheckFailed(_, _)));
    }

    #[test]
    fn test_from_invalid_config() {
        let err = IrsGreeksError::InvalidConfig("bad mode".to_string());
        let greeks_err: GreeksError = err.into();
        assert!(matches!(greeks_err, GreeksError::InvalidConfig(_)));
    }
}
