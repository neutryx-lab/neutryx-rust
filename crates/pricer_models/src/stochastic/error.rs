//! Unified error types for stochastic models.

use pricer_core::types::PricingError;
use thiserror::Error;

#[cfg(feature = "exotic")]
use super::correlated::CorrelationError;
#[cfg(feature = "equity")]
use super::heston::HestonError;

/// Unified error type for stochastic model operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ModelError {
    /// Heston model validation error.
    #[cfg(feature = "equity")]
    #[error("Heston: {0}")]
    Heston(#[from] HestonError),

    /// Correlation matrix error.
    #[cfg(feature = "exotic")]
    #[error("Correlation: {0}")]
    Correlation(#[from] CorrelationError),

    /// Generic model error.
    #[error("{model_name}: {message}")]
    Generic {
        /// Model name.
        model_name: String,
        /// Error message.
        message: String,
    },

    /// Numerical instability during computation.
    #[error("Numerical instability: {0}")]
    NumericalInstability(String),
}

impl From<ModelError> for PricingError {
    fn from(err: ModelError) -> Self {
        match err {
            ModelError::NumericalInstability(msg) => PricingError::NumericalInstability(msg),
            ModelError::Generic {
                model_name,
                message,
            } => PricingError::ModelFailure(format!("{model_name}: {message}")),
            #[cfg(feature = "equity")]
            ModelError::Heston(e) => PricingError::ModelFailure(e.to_string()),
            #[cfg(feature = "exotic")]
            ModelError::Correlation(e) => PricingError::ModelFailure(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_error_generic() {
        let err = ModelError::Generic {
            model_name: "TestModel".to_string(),
            message: "test error".to_string(),
        };
        assert!(format!("{err}").contains("TestModel"));
        assert!(format!("{err}").contains("test error"));
    }

    #[test]
    fn test_model_error_numerical_instability() {
        let err = ModelError::NumericalInstability("overflow".to_string());
        assert!(format!("{err}").contains("overflow"));
    }

    #[test]
    fn test_model_error_to_pricing_error_numerical() {
        let err = ModelError::NumericalInstability("NaN detected".to_string());
        let pricing_err: PricingError = err.into();
        assert!(matches!(pricing_err, PricingError::NumericalInstability(_)));
    }

    #[test]
    fn test_model_error_to_pricing_error_generic() {
        let err = ModelError::Generic {
            model_name: "TestModel".to_string(),
            message: "failed".to_string(),
        };
        let pricing_err: PricingError = err.into();
        assert!(matches!(pricing_err, PricingError::ModelFailure(_)));
    }

    #[cfg(feature = "equity")]
    #[test]
    fn test_heston_error_to_model_error() {
        let heston_err = HestonError::InvalidSpot(-100.0);
        let model_err: ModelError = heston_err.into();
        assert!(matches!(model_err, ModelError::Heston(_)));
    }

    #[cfg(feature = "equity")]
    #[test]
    fn test_heston_to_pricing_error() {
        let heston_err = HestonError::InvalidV0(-0.1);
        let model_err: ModelError = heston_err.into();
        let pricing_err: PricingError = model_err.into();
        assert!(matches!(pricing_err, PricingError::ModelFailure(_)));
    }

    #[cfg(feature = "exotic")]
    #[test]
    fn test_correlation_error_to_model_error() {
        let corr_err = CorrelationError::NotPositiveDefinite;
        let model_err: ModelError = corr_err.into();
        assert!(matches!(model_err, ModelError::Correlation(_)));
    }
}
