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
    /// Convergence failure - optimiser did not converge
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
    /// Failed to compute gradients for optimisation.
    #[error("勾配計算失敗: {message}")]
    GradientError {
        /// Description of the gradient computation failure
        message: String,
    },

    // --- Global Solver Errors (Requirement 2.5, 9.1, 9.2, 9.4) ---
    /// No instruments provided for calibration.
    ///
    /// Requirement 2.5: The Calibration Problem shall return this error
    /// if the instrument list is empty.
    #[error("キャリブレーション商品が指定されていません")]
    NoInstruments,

    /// Jacobian matrix is singular.
    ///
    /// Requirement 9.1: If Jacobian matrix is singular, the Global Solver
    /// shall return this error with condition number information.
    #[error("Jacobian行列が特異です (condition number: {condition_number:.2e})")]
    SingularJacobian {
        /// Condition number of the matrix (estimate)
        condition_number: f64,
    },

    /// Solver diverged during iteration.
    ///
    /// Requirement 9.2: If the solver diverges (residual increases),
    /// the Global Solver shall return this error.
    #[error("ソルバーが発散しました (iteration: {iteration}, residual: {residual:.6e})")]
    Divergence {
        /// Iteration at which divergence was detected
        iteration: usize,
        /// Residual value at divergence
        residual: f64,
    },

    /// Instrument evaluation failed.
    ///
    /// Requirement 9.4: If computing the theoretical price for an instrument
    /// fails, the Calibration Problem shall return this error identifying the
    /// specific instrument.
    #[error("商品 {instrument_index} の評価に失敗しました: {message}")]
    InstrumentEvaluationFailed {
        /// Index of the failed instrument (0-based)
        instrument_index: usize,
        /// Description of the failure
        message: String,
    },

    /// Dimension mismatch between instruments and parameters.
    ///
    /// Requirement 2.6: The Calibration Problem shall verify that
    /// instrument count matches parameter count.
    #[error(
        "商品数とパラメータ数が一致しません (instruments: {instruments}, parameters: {parameters})"
    )]
    DimensionMismatch {
        /// Number of instruments
        instruments: usize,
        /// Number of parameters
        parameters: usize,
    },

    /// Solver error from pricer_core.
    ///
    /// Wraps errors from the underlying numerical solver.
    #[error("ソルバーエラー: {message}")]
    SolverError {
        /// Error message from the solver
        message: String,
    },

    /// Missing required input.
    ///
    /// A required field was not provided to the builder.
    #[error("必須入力が不足しています: {field}")]
    MissingInput {
        /// Name of the missing field
        field: String,
    },

    // --- Jump Calibration Errors (Requirement 6.4) ---
    /// Jump calibration failed.
    ///
    /// The jump-aware calibration did not converge or produced invalid results.
    /// This may trigger fallback to non-jump calibration if enabled.
    #[error("ジャンプキャリブレーションに失敗しました: {message} (iterations: {iterations}, residual: {residual:.6e})")]
    JumpCalibrationFailed {
        /// Description of the failure
        message: String,
        /// Final residual at failure
        residual: f64,
        /// Number of iterations performed
        iterations: usize,
    },

    /// Invalid jump parameter.
    ///
    /// A jump pillar has invalid parameters (e.g., out of range, invalid date).
    #[error("無効なジャンプパラメータ: 日付 {date}, 値 {value:.4}bps - {reason}")]
    InvalidJumpParameter {
        /// Jump date in years or date string
        date: String,
        /// Jump value in basis points
        value: f64,
        /// Reason for invalidity
        reason: String,
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

    /// Create a no instruments error.
    pub fn no_instruments() -> Self { CalibrationError::NoInstruments }

    /// Create a singular Jacobian error.
    pub fn singular_jacobian(condition_number: f64) -> Self {
        CalibrationError::SingularJacobian { condition_number }
    }

    /// Create a divergence error.
    pub fn divergence(iteration: usize, residual: f64) -> Self {
        CalibrationError::Divergence {
            iteration,
            residual,
        }
    }

    /// Create an instrument evaluation failed error.
    pub fn instrument_evaluation_failed(
        instrument_index: usize,
        message: impl Into<String>,
    ) -> Self {
        CalibrationError::InstrumentEvaluationFailed {
            instrument_index,
            message: message.into(),
        }
    }

    /// Create a dimension mismatch error.
    pub fn dimension_mismatch(instruments: usize, parameters: usize) -> Self {
        CalibrationError::DimensionMismatch {
            instruments,
            parameters,
        }
    }

    /// Create a solver error.
    pub fn solver_error(message: impl Into<String>) -> Self {
        CalibrationError::SolverError {
            message: message.into(),
        }
    }

    /// Create a jump calibration failed error.
    pub fn jump_calibration_failed(
        message: impl Into<String>,
        residual: f64,
        iterations: usize,
    ) -> Self {
        CalibrationError::JumpCalibrationFailed {
            message: message.into(),
            residual,
            iterations,
        }
    }

    /// Create an invalid jump parameter error.
    pub fn invalid_jump_parameter(
        date: impl Into<String>,
        value: f64,
        reason: impl Into<String>,
    ) -> Self {
        CalibrationError::InvalidJumpParameter {
            date: date.into(),
            value,
            reason: reason.into(),
        }
    }

    /// Check if this is a recoverable error.
    ///
    /// Recoverable errors might succeed with different initial parameters
    /// or optimiser settings.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            CalibrationError::ConvergenceFailure { .. }
                | CalibrationError::NumericalInstability { .. }
                | CalibrationError::JumpCalibrationFailed { .. }
        )
    }
}

/// Convert BootstrapError to CalibrationError for seamless error propagation.
impl From<super::BootstrapError> for CalibrationError {
    fn from(err: super::BootstrapError) -> Self {
        use super::BootstrapError;
        match err {
            BootstrapError::ConvergenceFailure {
                maturity: _,
                residual,
                iterations,
            } => CalibrationError::ConvergenceFailure {
                iterations,
                residual,
            },
            BootstrapError::InsufficientData { required, provided } => {
                CalibrationError::InsufficientData { required, provided }
            }
            BootstrapError::NegativeRate { maturity, rate } => {
                CalibrationError::numerical_instability(format!(
                    "Negative rate {rate} at maturity {maturity}"
                ))
            }
            BootstrapError::ArbitrageDetected { maturity } => {
                CalibrationError::arbitrage_violation(format!(
                    "Arbitrage detected at maturity {maturity}"
                ))
            }
            BootstrapError::DuplicateMaturity { maturity } => {
                CalibrationError::invalid_market_data(format!("Duplicate maturity: {maturity}"))
            }
            BootstrapError::Solver(solver_err) => {
                CalibrationError::numerical_instability(solver_err.to_string())
            }
            BootstrapError::MarketData(mkt_err) => {
                CalibrationError::invalid_market_data(mkt_err.to_string())
            }
            BootstrapError::InvalidInput(msg) => CalibrationError::invalid_market_data(msg),
            BootstrapError::InvalidMaturity {
                maturity,
                max_maturity,
            } => CalibrationError::invalid_market_data(format!(
                "Invalid maturity {maturity} (max: {max_maturity})"
            )),
        }
    }
}

/// Convert CalibrationError to PricingError for top-level error handling.
impl From<CalibrationError> for pricer_core::types::PricingError {
    fn from(err: CalibrationError) -> Self {
        use pricer_core::types::PricingError;
        match err {
            CalibrationError::ConvergenceFailure {
                iterations,
                residual,
            } => PricingError::NumericalInstability(format!(
                "Calibration failed after {iterations} iterations (residual: {residual:.6e})"
            )),
            CalibrationError::NumericalInstability { message } => {
                PricingError::NumericalInstability(message)
            }
            CalibrationError::InvalidMarketData { message } => PricingError::InvalidInput(message),
            CalibrationError::InsufficientData { required, provided } => {
                PricingError::InvalidInput(format!(
                    "Insufficient data: need {required}, got {provided}"
                ))
            }
            CalibrationError::BoundsViolation {
                param_name, value, ..
            } => {
                PricingError::InvalidInput(format!("Parameter {param_name} out of bounds: {value}"))
            }
            CalibrationError::ModelError {
                model_name,
                message,
            } => PricingError::ModelFailure(format!("{model_name}: {message}")),
            CalibrationError::ArbitrageViolation { message } => {
                PricingError::ModelFailure(format!("Arbitrage violation: {message}"))
            }
            CalibrationError::GradientError { message } => {
                PricingError::NumericalInstability(format!("Gradient error: {message}"))
            }
            CalibrationError::NoInstruments => {
                PricingError::InvalidInput("No instruments provided for calibration".to_string())
            }
            CalibrationError::SingularJacobian { condition_number } => {
                PricingError::NumericalInstability(format!(
                    "Jacobian matrix is singular (condition number: {condition_number:.2e})"
                ))
            }
            CalibrationError::Divergence {
                iteration,
                residual,
            } => PricingError::NumericalInstability(format!(
                "Solver diverged at iteration {iteration} (residual: {residual:.6e})"
            )),
            CalibrationError::InstrumentEvaluationFailed {
                instrument_index,
                message,
            } => PricingError::NumericalInstability(format!(
                "Instrument {instrument_index} evaluation failed: {message}"
            )),
            CalibrationError::DimensionMismatch {
                instruments,
                parameters,
            } => PricingError::InvalidInput(format!(
                "Dimension mismatch: {instruments} instruments vs {parameters} parameters"
            )),
            CalibrationError::SolverError { message } => {
                PricingError::NumericalInstability(format!("Solver error: {message}"))
            }
            CalibrationError::MissingInput { field } => {
                PricingError::InvalidInput(format!("Missing required input: {field}"))
            }
            CalibrationError::JumpCalibrationFailed {
                message,
                residual,
                iterations,
            } => PricingError::NumericalInstability(format!(
                "Jump calibration failed: {message} (iterations: {iterations}, residual: {residual:.6e})"
            )),
            CalibrationError::InvalidJumpParameter {
                date,
                value,
                reason,
            } => PricingError::InvalidInput(format!(
                "Invalid jump parameter at {date}: {value:.4}bps - {reason}"
            )),
        }
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

    #[test]
    fn test_from_bootstrap_convergence() {
        use super::super::BootstrapError;
        let bootstrap_err = BootstrapError::convergence_failure(5.0, 0.001, 100);
        let calib_err: CalibrationError = bootstrap_err.into();
        assert!(matches!(
            calib_err,
            CalibrationError::ConvergenceFailure { .. }
        ));
    }

    #[test]
    fn test_from_bootstrap_insufficient_data() {
        use super::super::BootstrapError;
        let bootstrap_err = BootstrapError::insufficient_data(10, 3);
        let calib_err: CalibrationError = bootstrap_err.into();
        assert!(matches!(
            calib_err,
            CalibrationError::InsufficientData { .. }
        ));
    }

    #[test]
    fn test_calibration_to_pricing_error() {
        use pricer_core::types::PricingError;
        let calib_err = CalibrationError::convergence_failure(100, 0.01);
        let pricing_err: PricingError = calib_err.into();
        assert!(matches!(pricing_err, PricingError::NumericalInstability(_)));
    }

    #[test]
    fn test_calibration_to_pricing_invalid_data() {
        use pricer_core::types::PricingError;
        let calib_err = CalibrationError::invalid_market_data("negative price");
        let pricing_err: PricingError = calib_err.into();
        assert!(matches!(pricing_err, PricingError::InvalidInput(_)));
    }

    // --- Tests for new error types (Requirements 2.5, 9.1, 9.2, 9.4) ---

    #[test]
    fn test_no_instruments_error() {
        let err = CalibrationError::no_instruments();
        let msg = format!("{}", err);
        assert!(msg.contains("商品"));
        assert!(matches!(err, CalibrationError::NoInstruments));
    }

    #[test]
    fn test_singular_jacobian_error() {
        let err = CalibrationError::singular_jacobian(1e16);
        let msg = format!("{}", err);
        assert!(msg.contains("Jacobian"));
        assert!(msg.contains("1.00e16") || msg.contains("1e16"));
        if let CalibrationError::SingularJacobian { condition_number } = err {
            assert!((condition_number - 1e16).abs() < 1e10);
        } else {
            panic!("Expected SingularJacobian error");
        }
    }

    #[test]
    fn test_divergence_error() {
        let err = CalibrationError::divergence(50, 1.5e3);
        let msg = format!("{}", err);
        assert!(msg.contains("発散"));
        assert!(msg.contains("50"));
        if let CalibrationError::Divergence {
            iteration,
            residual,
        } = err
        {
            assert_eq!(iteration, 50);
            assert!((residual - 1.5e3).abs() < 1e-10);
        } else {
            panic!("Expected Divergence error");
        }
    }

    #[test]
    fn test_instrument_evaluation_failed_error() {
        let err = CalibrationError::instrument_evaluation_failed(3, "discount factor is NaN");
        let msg = format!("{}", err);
        assert!(msg.contains("3"));
        assert!(msg.contains("評価"));
        assert!(msg.contains("NaN"));
        if let CalibrationError::InstrumentEvaluationFailed {
            instrument_index,
            message,
        } = err
        {
            assert_eq!(instrument_index, 3);
            assert!(message.contains("NaN"));
        } else {
            panic!("Expected InstrumentEvaluationFailed error");
        }
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let err = CalibrationError::dimension_mismatch(5, 3);
        let msg = format!("{}", err);
        assert!(msg.contains("5"));
        assert!(msg.contains("3"));
        assert!(msg.contains("一致しません"));
        if let CalibrationError::DimensionMismatch {
            instruments,
            parameters,
        } = err
        {
            assert_eq!(instruments, 5);
            assert_eq!(parameters, 3);
        } else {
            panic!("Expected DimensionMismatch error");
        }
    }

    #[test]
    fn test_no_instruments_is_not_recoverable() {
        assert!(!CalibrationError::no_instruments().is_recoverable());
    }

    #[test]
    fn test_singular_jacobian_is_recoverable() {
        // Singular Jacobian might be recoverable with damping
        assert!(!CalibrationError::singular_jacobian(1e16).is_recoverable());
    }

    #[test]
    fn test_divergence_is_recoverable() {
        // Divergence might be recoverable with different initial values
        assert!(!CalibrationError::divergence(10, 100.0).is_recoverable());
    }

    // --- Tests for jump calibration errors (Requirement 6.4) ---

    #[test]
    fn test_jump_calibration_failed_error() {
        let err = CalibrationError::jump_calibration_failed("convergence failure", 1.5e-3, 50);
        let msg = format!("{}", err);
        assert!(msg.contains("ジャンプ"));
        assert!(msg.contains("50"));
        assert!(msg.contains("1.5"));
        if let CalibrationError::JumpCalibrationFailed {
            message,
            residual,
            iterations,
        } = err
        {
            assert_eq!(message, "convergence failure");
            assert!((residual - 1.5e-3).abs() < 1e-10);
            assert_eq!(iterations, 50);
        } else {
            panic!("Expected JumpCalibrationFailed error");
        }
    }

    #[test]
    fn test_invalid_jump_parameter_error() {
        let err = CalibrationError::invalid_jump_parameter("0.5Y", 150.0, "exceeds ±100bps limit");
        let msg = format!("{}", err);
        assert!(msg.contains("0.5Y"));
        assert!(msg.contains("150"));
        assert!(msg.contains("exceeds"));
        if let CalibrationError::InvalidJumpParameter {
            date,
            value,
            reason,
        } = err
        {
            assert_eq!(date, "0.5Y");
            assert!((value - 150.0).abs() < 1e-10);
            assert!(reason.contains("exceeds"));
        } else {
            panic!("Expected InvalidJumpParameter error");
        }
    }

    #[test]
    fn test_jump_calibration_failed_is_recoverable() {
        // Jump calibration failure is recoverable (can fallback to non-jump)
        assert!(CalibrationError::jump_calibration_failed("test", 0.01, 10).is_recoverable());
    }

    #[test]
    fn test_invalid_jump_parameter_is_not_recoverable() {
        // Invalid jump parameter is not recoverable
        assert!(
            !CalibrationError::invalid_jump_parameter("0.5Y", 150.0, "out of range")
                .is_recoverable()
        );
    }

    #[test]
    fn test_jump_calibration_to_pricing_error() {
        use pricer_core::types::PricingError;

        let err = CalibrationError::jump_calibration_failed("test failure", 0.01, 25);
        let pricing_err: PricingError = err.into();
        assert!(matches!(pricing_err, PricingError::NumericalInstability(_)));
        if let PricingError::NumericalInstability(msg) = pricing_err {
            assert!(msg.contains("Jump"));
        }
    }

    #[test]
    fn test_invalid_jump_to_pricing_error() {
        use pricer_core::types::PricingError;

        let err = CalibrationError::invalid_jump_parameter("2024-06-01", -120.0, "too large");
        let pricing_err: PricingError = err.into();
        assert!(matches!(pricing_err, PricingError::InvalidInput(_)));
        if let PricingError::InvalidInput(msg) = pricing_err {
            assert!(msg.contains("Invalid jump"));
        }
    }
}
