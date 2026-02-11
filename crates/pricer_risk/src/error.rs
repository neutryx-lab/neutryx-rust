//! Risk engine error types.
//!
//! Provides [`RiskError`] for comprehensive error handling in risk
//! calculations.
//!
//! # Requirements
//!
//! - Requirement 8.2: RiskError with calculation failure details
//! - Requirement 8.4: AAD not available error
//! - Requirement 8.5: Numerical instability handling

use thiserror::Error;

use crate::greeks::GreeksError;

/// Error types for risk engine operations.
#[derive(Debug, Error)]
pub enum RiskError {
    /// Calculation failed for a specific trade.
    #[error("Calculation failed for trade '{trade_id}': {reason}")]
    CalculationFailed {
        /// Trade identifier.
        trade_id: String,
        /// Failure reason.
        reason: String,
        /// Partial results if any Greeks were computed.
        partial_results: Option<PartialGreeksResult>,
    },

    /// AAD (Automatic Adjoint Differentiation) is not available.
    ///
    /// This error occurs when `enzyme-ad` feature is not enabled
    /// but AAD method is requested.
    #[error("AAD is not available: enzyme-ad feature is not enabled")]
    AadNotAvailable,

    /// Numerical instability detected during calculation.
    #[error(
        "Numerical instability: {description} (value: {value}, suggestion: {suggested_mitigation})"
    )]
    NumericalInstability {
        /// Description of the instability.
        description: String,
        /// The problematic value.
        value: f64,
        /// Suggested mitigation action.
        suggested_mitigation: String,
    },

    /// Market data error.
    #[error("Market data error: {0}")]
    MarketData(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Greeks calculation error.
    #[error("Greeks calculation error: {0}")]
    Greeks(#[from] GreeksError),

    /// Empty portfolio error.
    #[error("Empty portfolio: no trades to process")]
    EmptyPortfolio,

    /// Invalid input error.
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Partial Greeks result when calculation partially succeeded.
#[derive(Debug, Clone)]
pub struct PartialGreeksResult {
    /// Computed delta if available.
    pub delta: Option<f64>,
    /// Computed gamma if available.
    pub gamma: Option<f64>,
    /// Computed vega if available.
    pub vega: Option<f64>,
    /// Computed theta if available.
    pub theta: Option<f64>,
    /// Computed rho if available.
    pub rho: Option<f64>,
}

impl PartialGreeksResult {
    /// Creates an empty partial result.
    pub fn empty() -> Self {
        Self {
            delta: None,
            gamma: None,
            vega: None,
            theta: None,
            rho: None,
        }
    }

    /// Returns true if any Greek was computed.
    pub fn has_any(&self) -> bool {
        self.delta.is_some()
            || self.gamma.is_some()
            || self.vega.is_some()
            || self.theta.is_some()
            || self.rho.is_some()
    }
}

impl Default for PartialGreeksResult {
    fn default() -> Self { Self::empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculation_failed_error() {
        let err = RiskError::CalculationFailed {
            trade_id: "T001".to_string(),
            reason: "Division by zero".to_string(),
            partial_results: None,
        };
        assert!(err.to_string().contains("T001"));
        assert!(err.to_string().contains("Division by zero"));
    }

    #[test]
    fn test_calculation_failed_with_partial_results() {
        let partial = PartialGreeksResult {
            delta: Some(0.5),
            gamma: None,
            vega: Some(0.1),
            theta: None,
            rho: None,
        };
        let err = RiskError::CalculationFailed {
            trade_id: "T002".to_string(),
            reason: "Vega calculation failed".to_string(),
            partial_results: Some(partial.clone()),
        };
        assert!(err.to_string().contains("T002"));
        assert!(partial.has_any());
    }

    #[test]
    fn test_aad_not_available_error() {
        let err = RiskError::AadNotAvailable;
        assert!(err.to_string().contains("AAD"));
        assert!(err.to_string().contains("enzyme-ad"));
    }

    #[test]
    fn test_numerical_instability_error() {
        let err = RiskError::NumericalInstability {
            description: "Gamma approaching infinity near ATM".to_string(),
            value: 1e15,
            suggested_mitigation: "Use wider bump size".to_string(),
        };
        assert!(err.to_string().contains("Gamma"));
        assert!(err.to_string().contains("infinity"));
    }

    #[test]
    fn test_market_data_error() {
        let err = RiskError::MarketData("Curve not found: USD-SOFR".to_string());
        assert!(err.to_string().contains("Market data"));
        assert!(err.to_string().contains("USD-SOFR"));
    }

    #[test]
    fn test_config_error() {
        let err = RiskError::Config("Invalid bump size".to_string());
        assert!(err.to_string().contains("Configuration"));
    }

    #[test]
    fn test_empty_portfolio_error() {
        let err = RiskError::EmptyPortfolio;
        assert!(err.to_string().contains("Empty portfolio"));
    }

    #[test]
    fn test_partial_greeks_result_empty() {
        let partial = PartialGreeksResult::empty();
        assert!(!partial.has_any());
        assert!(partial.delta.is_none());
    }

    #[test]
    fn test_partial_greeks_result_has_any() {
        let partial = PartialGreeksResult {
            delta: Some(0.5),
            ..Default::default()
        };
        assert!(partial.has_any());
    }
}
