//! Unified error types for stochastic models.

use pricer_core::types::PricingError;
use thiserror::Error;

use super::{
    correlated::CorrelationError, gibson_schwartz::GibsonSchwartzError, heston::HestonError,
    jarrow_yildirim::JarrowYildirimError,
};

/// Unified error type for stochastic model operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ModelError {
    /// Heston model validation error.
    #[error("Heston: {0}")]
    Heston(#[from] HestonError),

    /// Correlation matrix error.
    #[error("Correlation: {0}")]
    Correlation(#[from] CorrelationError),

    /// Jarrow-Yildirim model error.
    #[error("JarrowYildirim: {0}")]
    JarrowYildirim(#[from] JarrowYildirimError),

    /// Gibson-Schwartz commodity model error.
    #[error("GibsonSchwartz: {0}")]
    GibsonSchwartz(#[from] GibsonSchwartzError),

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
            ModelError::Heston(e) => PricingError::ModelFailure(e.to_string()),
            ModelError::JarrowYildirim(e) => PricingError::ModelFailure(e.to_string()),
            ModelError::Correlation(e) => PricingError::ModelFailure(e.to_string()),
            ModelError::GibsonSchwartz(e) => PricingError::ModelFailure(e.to_string()),
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

    #[test]
    fn test_heston_error_to_model_error() {
        use super::super::validation::ParamValidationError;
        let heston_err = HestonError::Param(ParamValidationError::must_be_positive("spot", -100.0));
        let model_err: ModelError = heston_err.into();
        assert!(matches!(model_err, ModelError::Heston(_)));
    }

    #[test]
    fn test_heston_to_pricing_error() {
        use super::super::validation::ParamValidationError;
        let heston_err = HestonError::Param(ParamValidationError::must_be_positive("v0", -0.1));
        let model_err: ModelError = heston_err.into();
        let pricing_err: PricingError = model_err.into();
        assert!(matches!(pricing_err, PricingError::ModelFailure(_)));
    }

    #[test]
    fn test_correlation_error_to_model_error() {
        let corr_err = CorrelationError::NotPositiveDefinite;
        let model_err: ModelError = corr_err.into();
        assert!(matches!(model_err, ModelError::Correlation(_)));
    }
}
