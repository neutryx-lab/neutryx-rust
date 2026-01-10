//! Calibration error types.
//!
//! # Requirement: 6.1
//!
//! This module defines comprehensive error types for model calibration,
//! supporting multiple error scenarios and diagnostic information.

use thiserror::Error;

/// Calibration error type.
///
/// # Requirement: 6.1
///
/// Comprehensive error type for model calibration failures.
/// Supports multiple error scenarios with diagnostic information.
#[derive(Error, Debug, Clone)]
pub enum CalibrationError {
    /// Convergence failure - optimizer did not converge
    ///
    /// Contains iteration count and final residual for diagnostics.
    #[error(
        "キャリブレーションが収束しませんでした (iterations: {iterations}, residual: {residual:.6e})"
    )]
    ConvergenceFailure {
        /// Number of iterations performed
        iterations: usize,
        /// Final residual (sum of squared errors)
        residual: f64,
    },

    /// Invalid parameter bounds
    ///
    /// Parameter constraints were violated during calibration.
    #[error("パラメータ境界違反: {param_name} = {value:.6} (bounds: [{lower:.6}, {upper:.6}])")]
    BoundsViolation {
        /// Name of the parameter
        param_name: String,
        /// Attempted value
        value: f64,
        /// Lower bound
        lower: f64,
        /// Upper bound
        upper: f64,
    },

    /// Insufficient market data
    ///
    /// Not enough data points for reliable calibration.
    #[error("市場データが不足しています (required: {required}, provided: {provided})")]
    InsufficientData {
        /// Minimum required data points
        required: usize,
        /// Actual data points provided
        provided: usize,
    },

    /// Numerical instability during calibration
    ///
    /// NaN, Inf, or other numerical issues encountered.
    #[error("数値不安定性: {message}")]
    NumericalInstability {
        /// Description of the numerical issue
        message: String,
    },

    /// Invalid market data
    ///
    /// Market data failed validation (e.g., negative prices, invalid strikes).
    #[error("無効な市場データ: {message}")]
    InvalidMarketData {
        /// Description of the validation failure
        message: String,
    },

    /// Model-specific error
    ///
    /// Error specific to a particular model type.
    #[error("{model_name} モデルエラー: {message}")]
    ModelError {
        /// Name of the model
        model_name: String,
        /// Error description
        message: String,
    },

    /// Arbitrage violation detected
    ///
    /// Calibrated parameters would produce arbitrage opportunities.
    #[error("アービトラージ違反: {message}")]
    ArbitrageViolation {
        /// Description of the arbitrage condition
        message: String,
    },

    /// Gradient computation failed
    ///
    /// Failed to compute gradients for optimization.
    #[error("勾配計算失敗: {message}")]
    GradientError {
        /// Description of the gradient computation failure
        message: String,
    },
}

impl CalibrationError {
    /// Create a convergence failure error.
    pub fn convergence_failure(iterations: usize, residual: f64) -> Self {
        CalibrationError::ConvergenceFailure {
            iterations,
            residual,
        }
    }

    /// Create a bounds violation error.
    pub fn bounds_violation(param_name: &str, value: f64, lower: f64, upper: f64) -> Self {
        CalibrationError::BoundsViolation {
            param_name: param_name.to_string(),
            value,
            lower,
            upper,
        }
    }

    /// Create an insufficient data error.
    pub fn insufficient_data(required: usize, provided: usize) -> Self {
        CalibrationError::InsufficientData { required, provided }
    }

    /// Create a numerical instability error.
    pub fn numerical_instability(message: impl Into<String>) -> Self {
        CalibrationError::NumericalInstability {
            message: message.into(),
        }
    }

    /// Create an invalid market data error.
    pub fn invalid_market_data(message: impl Into<String>) -> Self {
        CalibrationError::InvalidMarketData {
            message: message.into(),
        }
    }

    /// Create a model-specific error.
    pub fn model_error(model_name: &str, message: impl Into<String>) -> Self {
        CalibrationError::ModelError {
            model_name: model_name.to_string(),
            message: message.into(),
        }
    }

    /// Create an arbitrage violation error.
    pub fn arbitrage_violation(message: impl Into<String>) -> Self {
        CalibrationError::ArbitrageViolation {
            message: message.into(),
        }
    }

    /// Create a gradient error.
    pub fn gradient_error(message: impl Into<String>) -> Self {
        CalibrationError::GradientError {
            message: message.into(),
        }
    }

    /// Check if this is a recoverable error.
    ///
    /// Recoverable errors might succeed with different initial parameters
    /// or optimizer settings.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            CalibrationError::ConvergenceFailure { .. }
                | CalibrationError::NumericalInstability { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convergence_failure_error() {
        let err = CalibrationError::convergence_failure(1000, 1e-4);
        let msg = format!("{}", err);
        assert!(msg.contains("1000"));
        assert!(msg.contains("収束"));
    }

    #[test]
    fn test_bounds_violation_error() {
        let err = CalibrationError::bounds_violation("alpha", 1.5, 0.0, 1.0);
        let msg = format!("{}", err);
        assert!(msg.contains("alpha"));
        assert!(msg.contains("1.5"));
    }

    #[test]
    fn test_insufficient_data_error() {
        let err = CalibrationError::insufficient_data(5, 3);
        let msg = format!("{}", err);
        assert!(msg.contains("5"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn test_is_recoverable() {
        assert!(CalibrationError::convergence_failure(100, 0.1).is_recoverable());
        assert!(CalibrationError::numerical_instability("NaN").is_recoverable());
        assert!(!CalibrationError::insufficient_data(5, 3).is_recoverable());
    }
}
