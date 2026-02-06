//! XVA error types.

use thiserror::Error;

/// Errors that can occur during XVA calculations.
#[derive(Debug, Error)]
pub enum XvaError {
    /// Missing exposure profile for a netting set.
    #[error("Missing exposure profile for netting set: {0}")]
    MissingExposureProfile(String),

    /// Missing credit parameters for a counterparty.
    #[error("Missing credit parameters for counterparty: {0}")]
    MissingCreditParams(String),

    /// Missing own credit parameters for DVA calculation.
    #[error("Missing own credit parameters")]
    MissingOwnCreditParams,

    /// Invalid funding spread value.
    #[error("Invalid funding spread: {0}")]
    InvalidFundingSpread(String),

    /// Invalid credit parameter value.
    #[error("Invalid credit parameter: {0}")]
    InvalidCreditParam(String),

    /// Numerical integration error.
    #[error("Integration error: {0}")]
    IntegrationError(String),

    /// Time grid mismatch between exposure profiles.
    #[error("Time grid mismatch: expected {expected} points, got {actual}")]
    TimeGridMismatch {
        /// Expected number of time points.
        expected: usize,
        /// Actual number of time points.
        actual: usize,
    },

    /// Empty time grid.
    #[error("Time grid is empty")]
    EmptyTimeGrid,

    /// Discount factor mismatch with time grid.
    #[error("Discount factor count ({actual}) doesn't match time grid ({expected})")]
    DiscountFactorMismatch {
        /// Expected count.
        expected: usize,
        /// Actual count.
        actual: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_missing_exposure() {
        let err = XvaError::MissingExposureProfile("NS001".to_string());
        assert_eq!(
            format!("{}", err),
            "Missing exposure profile for netting set: NS001"
        );
    }

    #[test]
    fn test_error_display_time_grid_mismatch() {
        let err = XvaError::TimeGridMismatch {
            expected: 100,
            actual: 50,
        };
        assert_eq!(
            format!("{}", err),
            "Time grid mismatch: expected 100 points, got 50"
        );
    }

    #[test]
    fn test_error_is_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(XvaError::MissingOwnCreditParams);
        assert!(err.to_string().contains("own credit"));
    }
}
