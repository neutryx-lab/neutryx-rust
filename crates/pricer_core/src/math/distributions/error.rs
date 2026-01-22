//! Error types for probability distribution calculations.
//!
//! This module defines errors that can occur during probability distribution
//! computations, including invalid parameters and numerical failures.

use thiserror::Error;

/// Errors that can occur during probability distribution calculations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum DistributionError {
    /// Probability value is outside the valid range [0, 1].
    #[error("Probability {p} out of range [0, 1]")]
    InvalidProbability {
        /// The invalid probability value.
        p: f64,
    },

    /// Correlation coefficient is outside the valid range [-1, 1].
    #[error("Correlation coefficient {rho} out of range [-1, 1]")]
    InvalidCorrelation {
        /// The invalid correlation coefficient.
        rho: f64,
    },

    /// Degrees of freedom must be positive.
    #[error("Degrees of freedom must be positive: got {df}")]
    InvalidDegreesOfFreedom {
        /// The invalid degrees of freedom value.
        df: f64,
    },

    /// Non-centrality parameter must be non-negative.
    #[error("Non-centrality parameter must be non-negative: got {ncp}")]
    InvalidNonCentrality {
        /// The invalid non-centrality parameter.
        ncp: f64,
    },

    /// Correlation matrix is not positive definite.
    #[error("Correlation matrix is not positive definite")]
    NotPositiveDefinite,

    /// A numerical computation failed.
    #[error("Numerical computation failed: {0}")]
    NumericalError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_probability_display() {
        let err = DistributionError::InvalidProbability { p: 1.5 };
        assert_eq!(format!("{err}"), "Probability 1.5 out of range [0, 1]");
    }

    #[test]
    fn test_invalid_correlation_display() {
        let err = DistributionError::InvalidCorrelation { rho: 1.2 };
        assert_eq!(
            format!("{err}"),
            "Correlation coefficient 1.2 out of range [-1, 1]"
        );
    }

    #[test]
    fn test_invalid_degrees_of_freedom_display() {
        let err = DistributionError::InvalidDegreesOfFreedom { df: -1.0 };
        assert_eq!(
            format!("{err}"),
            "Degrees of freedom must be positive: got -1"
        );
    }

    #[test]
    fn test_invalid_non_centrality_display() {
        let err = DistributionError::InvalidNonCentrality { ncp: -0.5 };
        assert_eq!(
            format!("{err}"),
            "Non-centrality parameter must be non-negative: got -0.5"
        );
    }

    #[test]
    fn test_not_positive_definite_display() {
        let err = DistributionError::NotPositiveDefinite;
        assert_eq!(
            format!("{err}"),
            "Correlation matrix is not positive definite"
        );
    }

    #[test]
    fn test_numerical_error_display() {
        let err = DistributionError::NumericalError("overflow".to_string());
        assert_eq!(format!("{err}"), "Numerical computation failed: overflow");
    }

    #[test]
    fn test_error_equality() {
        let err1 = DistributionError::InvalidProbability { p: 0.5 };
        let err2 = DistributionError::InvalidProbability { p: 0.5 };
        let err3 = DistributionError::InvalidProbability { p: 0.6 };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }
}
