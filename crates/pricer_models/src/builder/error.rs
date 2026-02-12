//! Calibration error types and numerical diagnostics.
//!
//! # Requirement: 5.3, 5.5, 6.1
//!
//! This module defines comprehensive error types for model calibration,
//! supporting multiple error scenarios and diagnostic information.
//!
//! ## Numerical Stability (Requirement 5)
//!
//! - [`JacobianQuality`]: Classification of Jacobian matrix quality
//!   (Good/Warning/Poor)
//! - [`NumericalDiagnostics`]: Comprehensive diagnostic information for
//!   calibration
//! - [`RegularisationType`]: Type of regularisation applied (None/Tikhonov/LM)

use num_traits::Float;
use pricer_core::math::numeric::from_f64;
use thiserror::Error;

/// Quality classification for Jacobian matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JacobianQuality {
    /// Jacobian is well-conditioned with no issues detected.
    Good,

    /// Jacobian has potential issues but calibration may proceed.
    ///
    /// Contains a reason describing the warning condition.
    Warning {
        /// Reason for the warning (e.g., "near-zero diagonal element")
        reason: &'static str,
    },

    /// Jacobian is poorly conditioned and may cause numerical issues.
    ///
    /// Contains a reason describing the poor quality.
    Poor {
        /// Reason for poor quality (e.g., "NaN detected", "Inf detected")
        reason: &'static str,
    },
}

impl JacobianQuality {
    /// Check if the quality is acceptable (Good or Warning).
    pub fn is_acceptable(&self) -> bool { !matches!(self, JacobianQuality::Poor { .. }) }

    /// Check if the quality is Good.
    pub fn is_good(&self) -> bool { matches!(self, JacobianQuality::Good) }

    /// Get the reason string if this is a Warning or Poor quality.
    pub fn reason(&self) -> Option<&'static str> {
        match self {
            JacobianQuality::Good => None,
            JacobianQuality::Warning { reason } | JacobianQuality::Poor { reason } => Some(reason),
        }
    }
}

impl std::fmt::Display for JacobianQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JacobianQuality::Good => write!(f, "Good"),
            JacobianQuality::Warning { reason } => write!(f, "Warning: {reason}"),
            JacobianQuality::Poor { reason } => write!(f, "Poor: {reason}"),
        }
    }
}

/// Type of regularisation applied during calibration.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RegularisationType<T: Float> {
    /// No regularisation applied.
    #[default]
    None,

    /// Tikhonov regularisation (ridge regression).
    ///
    /// Adds λI to the Hessian/Jacobian to improve conditioning.
    Tikhonov {
        /// Damping parameter λ
        damping: T,
    },

    /// Levenberg-Marquardt regularisation.
    ///
    /// Adds λ·diag(J^T·J) to blend between Gauss-Newton and gradient descent.
    LevenbergMarquardt {
        /// Damping parameter λ
        lambda: T,
    },
}

impl<T: Float> RegularisationType<T> {
    /// Check if any regularisation is applied.
    pub fn is_regularised(&self) -> bool { !matches!(self, RegularisationType::None) }

    /// Get the damping parameter if regularisation is applied.
    pub fn damping(&self) -> Option<T> {
        match self {
            RegularisationType::None => None,
            RegularisationType::Tikhonov { damping } => Some(*damping),
            RegularisationType::LevenbergMarquardt { lambda } => Some(*lambda),
        }
    }
}

impl<T: Float + std::fmt::Display> std::fmt::Display for RegularisationType<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegularisationType::None => write!(f, "None"),
            RegularisationType::Tikhonov { damping } => write!(f, "Tikhonov(λ={damping})"),
            RegularisationType::LevenbergMarquardt { lambda } => write!(f, "LM(λ={lambda})"),
        }
    }
}

/// Numerical diagnostics for calibration (condition numbers, residuals, regularisation).
#[derive(Debug, Clone)]
pub struct NumericalDiagnostics<T: Float> {
    /// Estimated condition number of the Jacobian matrix.
    ///
    /// Higher values indicate ill-conditioning. Values > 1e10 typically
    /// require regularisation.
    pub condition_number: Option<T>,

    /// Residual norm history at each iteration.
    ///
    /// Useful for diagnosing convergence behaviour.
    pub residual_history: Vec<T>,

    /// Type of regularisation applied during calibration.
    pub regularisation_applied: RegularisationType<T>,

    /// Quality assessment of the final Jacobian matrix.
    pub jacobian_quality: JacobianQuality,

    /// Number of NaN values detected (and handled) during calibration.
    pub nan_count: usize,

    /// Number of Inf values detected (and handled) during calibration.
    pub inf_count: usize,

    /// Number of near-zero diagonal elements detected.
    pub near_zero_diagonal_count: usize,

    /// Whether AD fallback to finite differences was triggered.
    pub ad_fallback_used: bool,

    /// Variance of AD gradients vs finite differences (if compared).
    ///
    /// High variance (> 1e6) indicates AD instability.
    pub ad_variance: Option<T>,
}

impl<T: Float> Default for NumericalDiagnostics<T> {
    fn default() -> Self {
        Self {
            condition_number: None,
            residual_history: Vec::new(),
            regularisation_applied: RegularisationType::None,
            jacobian_quality: JacobianQuality::Good,
            nan_count: 0,
            inf_count: 0,
            near_zero_diagonal_count: 0,
            ad_fallback_used: false,
            ad_variance: None,
        }
    }
}

impl<T: Float> NumericalDiagnostics<T> {
    /// Check if there were any numerical issues.
    pub fn has_issues(&self) -> bool {
        self.nan_count > 0
            || self.inf_count > 0
            || self.near_zero_diagonal_count > 0
            || !self.jacobian_quality.is_good()
    }

    /// Check if condition number exceeds threshold.
    pub fn is_ill_conditioned(&self, threshold: T) -> bool {
        self.condition_number
            .map(|c| c > threshold)
            .unwrap_or(false)
    }

    /// Check if AD was unstable (high variance compared to finite differences).
    pub fn is_ad_unstable(&self, variance_threshold: T) -> bool {
        self.ad_variance
            .map(|v| v > variance_threshold)
            .unwrap_or(false)
    }

    /// Get the final residual norm (last entry in history).
    pub fn final_residual(&self) -> Option<T> { self.residual_history.last().copied() }

    /// Get the number of iterations (length of residual history).
    pub fn iteration_count(&self) -> usize { self.residual_history.len() }

    /// Check if regularisation was applied.
    pub fn was_regularised(&self) -> bool { self.regularisation_applied.is_regularised() }

    /// Generate a summary string for logging.
    pub fn summary(&self) -> String
    where
        T: std::fmt::Display + std::fmt::LowerExp,
    {
        let cond_str = self
            .condition_number
            .map(|c| format!("{c:.2e}"))
            .unwrap_or_else(|| "N/A".to_string());

        let issues = if self.has_issues() {
            format!(
                " [NaN:{}, Inf:{}, ZeroDiag:{}]",
                self.nan_count, self.inf_count, self.near_zero_diagonal_count
            )
        } else {
            String::new()
        };

        format!(
            "Iterations: {}, Final residual: {}, Condition: {}, Quality: {}, Regularisation: {}{}",
            self.iteration_count(),
            self.final_residual()
                .map(|r| format!("{r:.6e}"))
                .unwrap_or_else(|| "N/A".to_string()),
            cond_str,
            self.jacobian_quality,
            self.regularisation_applied,
            issues
        )
    }
}

/// Validates a Jacobian matrix for NaN, Inf, and near-zero diagonal elements.
pub fn validate_jacobian_matrix<T: Float>(
    jacobian: &[T],
    nrows: usize,
    ncols: usize,
    zero_threshold: T,
) -> (JacobianQuality, NumericalDiagnostics<T>) {
    let mut diagnostics = NumericalDiagnostics::default();

    // Check for NaN and Inf
    for &val in jacobian {
        if val.is_nan() {
            diagnostics.nan_count += 1;
        }
        if val.is_infinite() {
            diagnostics.inf_count += 1;
        }
    }

    // Check diagonal elements (for square matrices)
    if nrows == ncols {
        for i in 0..nrows {
            let idx = i * ncols + i;
            if idx < jacobian.len() {
                let diag_val = jacobian[idx];
                if diag_val.abs() < zero_threshold {
                    diagnostics.near_zero_diagonal_count += 1;
                }
            }
        }
    }

    // Determine quality
    let quality = if diagnostics.nan_count > 0 {
        JacobianQuality::Poor {
            reason: "NaN detected in Jacobian",
        }
    } else if diagnostics.inf_count > 0 {
        JacobianQuality::Poor {
            reason: "Inf detected in Jacobian",
        }
    } else if diagnostics.near_zero_diagonal_count > 0 {
        JacobianQuality::Warning {
            reason: "Near-zero diagonal element detected",
        }
    } else {
        JacobianQuality::Good
    };

    diagnostics.jacobian_quality = quality;

    (quality, diagnostics)
}

/// Validates a Jacobian DMatrix for numerical quality.
pub fn validate_jacobian_dmatrix<T>(
    jacobian: &pricer_core::math::linalg::DMatrix<T>,
    zero_threshold: T,
) -> (JacobianQuality, NumericalDiagnostics<T>)
where
    T: Float + pricer_core::math::linalg::RealField,
{
    let nrows = jacobian.nrows();
    let ncols = jacobian.ncols();

    let mut diagnostics = NumericalDiagnostics::default();

    // Check for NaN and Inf
    for &val in jacobian.iter() {
        if val.is_nan() {
            diagnostics.nan_count += 1;
        }
        if val.is_infinite() {
            diagnostics.inf_count += 1;
        }
    }

    // Check diagonal elements (for square matrices)
    if nrows == ncols {
        for i in 0..nrows {
            let diag_val = jacobian[(i, i)];
            if Float::abs(diag_val) < zero_threshold {
                diagnostics.near_zero_diagonal_count += 1;
            }
        }
    }

    // Determine quality
    let quality = if diagnostics.nan_count > 0 {
        JacobianQuality::Poor {
            reason: "NaN detected in Jacobian",
        }
    } else if diagnostics.inf_count > 0 {
        JacobianQuality::Poor {
            reason: "Inf detected in Jacobian",
        }
    } else if diagnostics.near_zero_diagonal_count > 0 {
        JacobianQuality::Warning {
            reason: "Near-zero diagonal element detected",
        }
    } else {
        JacobianQuality::Good
    };

    diagnostics.jacobian_quality = quality;

    (quality, diagnostics)
}

/// Estimates condition number using row-sum norm heuristic (O(n^2), not SVD).
pub fn estimate_condition_number<T>(jacobian: &pricer_core::math::linalg::DMatrix<T>) -> Option<T>
where
    T: Float + pricer_core::math::linalg::RealField,
{
    let nrows = jacobian.nrows();
    if nrows == 0 {
        return None;
    }

    let mut max_row_sum = T::zero();
    let mut min_row_sum = T::infinity();

    for i in 0..nrows {
        let row_sum = (0..jacobian.ncols())
            .map(|j| Float::abs(jacobian[(i, j)]))
            .fold(T::zero(), |acc, x| acc + x);

        if row_sum > max_row_sum {
            max_row_sum = row_sum;
        }
        if row_sum < min_row_sum && row_sum > T::zero() {
            min_row_sum = row_sum;
        }
    }

    if min_row_sum > T::zero() {
        Some(max_row_sum / min_row_sum)
    } else {
        None
    }
}

/// Applies Tikhonov regularisation (adds λI) to improve conditioning.
pub fn apply_tikhonov_regularisation<T>(
    matrix: &mut pricer_core::math::linalg::DMatrix<T>,
    damping: T,
) where
    T: Float + pricer_core::math::linalg::RealField,
{
    let n = matrix.nrows().min(matrix.ncols());
    for i in 0..n {
        matrix[(i, i)] = matrix[(i, i)] + damping;
    }
}

/// Returns recommended damping factor if condition number exceeds threshold.
#[allow(dead_code)]
pub fn should_apply_regularisation<T: Float>(
    condition_number: T,
    max_condition_number: T,
) -> Option<T> {
    if condition_number > max_condition_number {
        // Use sqrt of excess as damping factor
        let excess = condition_number / max_condition_number;
        let damping = excess.sqrt() * from_f64::<T>(1e-6);
        Some(damping)
    } else {
        None
    }
}

/// Errors during IFT (Implicit Function Theorem) sensitivity computation.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum IftError {
    /// Jacobian inverse is not cached.
    #[error("Jacobian逆行列がキャッシュされていません。store_jacobian_inverse=trueで再キャリブレーションしてください")]
    NoJacobianInverse,

    /// Input vector dimension does not match expected size.
    #[error("次元不整合: 期待値 {expected}、実際値 {got}")]
    DimensionMismatch {
        /// Expected dimension (number of instruments/pillars)
        expected: usize,
        /// Actual dimension provided
        got: usize,
    },

    /// Batch input matrix has wrong dimensions.
    #[error("バッチ入力の次元不整合: 行数 {expected}、実際値 {got}")]
    BatchDimensionMismatch {
        /// Expected number of rows (n_instruments)
        expected: usize,
        /// Actual number of rows provided
        got: usize,
    },

    /// Numerical error during IFT computation.
    #[error("IFT計算中の数値エラー: {message}")]
    NumericalError {
        /// Description of the numerical issue
        message: String,
    },
}

/// Calibration error type.
#[derive(Error, Debug, Clone)]
pub enum CalibrationError {
    /// Convergence failure - optimiser did not converge.
    #[error(
        "キャリブレーションが収束しませんでした (iterations: {iterations}, residual: {residual:.6e})"
    )]
    ConvergenceFailure {
        /// Number of iterations performed
        iterations: usize,
        /// Final residual (sum of squared errors)
        residual: f64,
    },

    /// Parameter bounds violated during calibration.
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

    /// Insufficient market data for calibration.
    #[error("市場データが不足しています (required: {required}, provided: {provided})")]
    InsufficientData {
        /// Minimum required data points
        required: usize,
        /// Actual data points provided
        provided: usize,
    },

    /// Numerical instability (NaN, Inf, etc.).
    #[error("数値不安定性: {message}")]
    NumericalInstability {
        /// Description of the numerical issue
        message: String,
    },

    /// Invalid market data.
    #[error("無効な市場データ: {message}")]
    InvalidMarketData {
        /// Description of the validation failure
        message: String,
    },

    /// Model-specific error.
    #[error("{model_name} モデルエラー: {message}")]
    ModelError {
        /// Name of the model
        model_name: String,
        /// Error description
        message: String,
    },

    /// Arbitrage violation detected.
    #[error("アービトラージ違反: {message}")]
    ArbitrageViolation {
        /// Description of the arbitrage condition
        message: String,
    },

    /// Gradient computation failed.
    #[error("勾配計算失敗: {message}")]
    GradientError {
        /// Description of the gradient computation failure
        message: String,
    },

    /// No instruments provided for calibration.
    #[error("キャリブレーション商品が指定されていません")]
    NoInstruments,

    /// Jacobian matrix is singular.
    #[error("Jacobian行列が特異です (condition number: {condition_number:.2e})")]
    SingularJacobian {
        /// Condition number of the matrix (estimate)
        condition_number: f64,
    },

    /// Solver diverged during iteration.
    #[error("ソルバーが発散しました (iteration: {iteration}, residual: {residual:.6e})")]
    Divergence {
        /// Iteration at which divergence was detected
        iteration: usize,
        /// Residual value at divergence
        residual: f64,
    },

    /// Instrument evaluation failed.
    #[error("商品 {instrument_index} の評価に失敗しました: {message}")]
    InstrumentEvaluationFailed {
        /// Index of the failed instrument (0-based)
        instrument_index: usize,
        /// Description of the failure
        message: String,
    },

    /// Dimension mismatch between instruments and parameters.
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
    #[error("ソルバーエラー: {message}")]
    SolverError {
        /// Error message from the solver
        message: String,
    },

    /// Missing required input.
    #[error("必須入力が不足しています: {field}")]
    MissingInput {
        /// Name of the missing field
        field: String,
    },

    /// Jump calibration failed.
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
    /// Returns true if retrying with different parameters might succeed.
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
                CalibrationError::NumericalInstability {
                    message: format!("Negative rate {rate} at maturity {maturity}"),
                }
            }
            BootstrapError::ArbitrageDetected { maturity } => {
                CalibrationError::ArbitrageViolation {
                    message: format!("Arbitrage detected at maturity {maturity}"),
                }
            }
            BootstrapError::DuplicateMaturity { maturity } => CalibrationError::InvalidMarketData {
                message: format!("Duplicate maturity: {maturity}"),
            },
            BootstrapError::Solver(solver_err) => CalibrationError::NumericalInstability {
                message: solver_err.to_string(),
            },
            BootstrapError::MarketData(mkt_err) => CalibrationError::InvalidMarketData {
                message: mkt_err.to_string(),
            },
            BootstrapError::InvalidInput(msg) => {
                CalibrationError::InvalidMarketData { message: msg }
            }
            BootstrapError::InvalidMaturity {
                maturity,
                max_maturity,
            } => CalibrationError::InvalidMarketData {
                message: format!("Invalid maturity {maturity} (max: {max_maturity})"),
            },
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
    fn test_ift_error_no_jacobian_inverse() {
        let err = IftError::NoJacobianInverse;
        let msg = format!("{err}");
        assert!(msg.contains("Jacobian"));
        assert!(msg.contains("store_jacobian_inverse"));
        assert!(matches!(err, IftError::NoJacobianInverse));
    }

    #[test]
    fn test_ift_error_dimension_mismatch() {
        let err = IftError::DimensionMismatch {
            expected: 10,
            got: 5,
        };
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
        let err = IftError::BatchDimensionMismatch {
            expected: 20,
            got: 15,
        };
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
        let err = IftError::NumericalError {
            message: "NaN detected".to_string(),
        };
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
        let err1 = IftError::NoJacobianInverse;
        let err2 = IftError::NoJacobianInverse;
        assert_eq!(err1, err2);

        let err3 = IftError::DimensionMismatch {
            expected: 10,
            got: 5,
        };
        let err4 = IftError::DimensionMismatch {
            expected: 10,
            got: 5,
        };
        assert_eq!(err3, err4);

        let err5 = IftError::DimensionMismatch {
            expected: 10,
            got: 5,
        };
        let err6 = IftError::DimensionMismatch {
            expected: 10,
            got: 6,
        };
        assert_ne!(err5, err6);
    }

    #[test]
    fn test_convergence_failure_error() {
        let err = CalibrationError::ConvergenceFailure {
            iterations: 1000,
            residual: 1e-4,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("1000"));
        assert!(msg.contains("収束"));
    }

    #[test]
    fn test_bounds_violation_error() {
        let err = CalibrationError::BoundsViolation {
            param_name: "alpha".to_string(),
            value: 1.5,
            lower: 0.0,
            upper: 1.0,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("alpha"));
        assert!(msg.contains("1.5"));
    }

    #[test]
    fn test_insufficient_data_error() {
        let err = CalibrationError::InsufficientData {
            required: 5,
            provided: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("5"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn test_is_recoverable() {
        assert!((CalibrationError::ConvergenceFailure {
            iterations: 100,
            residual: 0.1,
        })
        .is_recoverable());
        assert!((CalibrationError::NumericalInstability {
            message: "NaN".to_string(),
        })
        .is_recoverable());
        assert!(!(CalibrationError::InsufficientData {
            required: 5,
            provided: 3,
        })
        .is_recoverable());
    }

    #[test]
    fn test_from_bootstrap_convergence() {
        use super::super::BootstrapError;
        let bootstrap_err = BootstrapError::ConvergenceFailure {
            maturity: 5.0,
            residual: 0.001,
            iterations: 100,
        };
        let calib_err: CalibrationError = bootstrap_err.into();
        assert!(matches!(
            calib_err,
            CalibrationError::ConvergenceFailure { .. }
        ));
    }

    #[test]
    fn test_from_bootstrap_insufficient_data() {
        use super::super::BootstrapError;
        let bootstrap_err = BootstrapError::InsufficientData {
            required: 10,
            provided: 3,
        };
        let calib_err: CalibrationError = bootstrap_err.into();
        assert!(matches!(
            calib_err,
            CalibrationError::InsufficientData { .. }
        ));
    }

    #[test]
    fn test_calibration_to_pricing_error() {
        use pricer_core::types::PricingError;
        let calib_err = CalibrationError::ConvergenceFailure {
            iterations: 100,
            residual: 0.01,
        };
        let pricing_err: PricingError = calib_err.into();
        assert!(matches!(pricing_err, PricingError::NumericalInstability(_)));
    }

    #[test]
    fn test_calibration_to_pricing_invalid_data() {
        use pricer_core::types::PricingError;
        let calib_err = CalibrationError::InvalidMarketData {
            message: "negative price".to_string(),
        };
        let pricing_err: PricingError = calib_err.into();
        assert!(matches!(pricing_err, PricingError::InvalidInput(_)));
    }

    #[test]
    fn test_no_instruments_error() {
        let err = CalibrationError::NoInstruments;
        let msg = format!("{}", err);
        assert!(msg.contains("商品"));
        assert!(matches!(err, CalibrationError::NoInstruments));
    }

    #[test]
    fn test_singular_jacobian_error() {
        let err = CalibrationError::SingularJacobian {
            condition_number: 1e16,
        };
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
        let err = CalibrationError::Divergence {
            iteration: 50,
            residual: 1.5e3,
        };
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
        let err = CalibrationError::InstrumentEvaluationFailed {
            instrument_index: 3,
            message: "discount factor is NaN".to_string(),
        };
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
        let err = CalibrationError::DimensionMismatch {
            instruments: 5,
            parameters: 3,
        };
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
        assert!(!CalibrationError::NoInstruments.is_recoverable());
    }

    #[test]
    fn test_singular_jacobian_is_recoverable() {
        assert!(!(CalibrationError::SingularJacobian {
            condition_number: 1e16,
        })
        .is_recoverable());
    }

    #[test]
    fn test_divergence_is_recoverable() {
        assert!(!(CalibrationError::Divergence {
            iteration: 10,
            residual: 100.0,
        })
        .is_recoverable());
    }

    #[test]
    fn test_jump_calibration_failed_error() {
        let err = CalibrationError::JumpCalibrationFailed {
            message: "convergence failure".to_string(),
            residual: 1.5e-3,
            iterations: 50,
        };
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
        let err = CalibrationError::InvalidJumpParameter {
            date: "0.5Y".to_string(),
            value: 150.0,
            reason: "exceeds ±100bps limit".to_string(),
        };
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
        assert!((CalibrationError::JumpCalibrationFailed {
            message: "test".to_string(),
            residual: 0.01,
            iterations: 10,
        })
        .is_recoverable());
    }

    #[test]
    fn test_invalid_jump_parameter_is_not_recoverable() {
        assert!(!(CalibrationError::InvalidJumpParameter {
            date: "0.5Y".to_string(),
            value: 150.0,
            reason: "out of range".to_string(),
        })
        .is_recoverable());
    }

    #[test]
    fn test_jump_calibration_to_pricing_error() {
        use pricer_core::types::PricingError;

        let err = CalibrationError::JumpCalibrationFailed {
            message: "test failure".to_string(),
            residual: 0.01,
            iterations: 25,
        };
        let pricing_err: PricingError = err.into();
        assert!(matches!(pricing_err, PricingError::NumericalInstability(_)));
        if let PricingError::NumericalInstability(msg) = pricing_err {
            assert!(msg.contains("Jump"));
        }
    }

    #[test]
    fn test_invalid_jump_to_pricing_error() {
        use pricer_core::types::PricingError;

        let err = CalibrationError::InvalidJumpParameter {
            date: "2024-06-01".to_string(),
            value: -120.0,
            reason: "too large".to_string(),
        };
        let pricing_err: PricingError = err.into();
        assert!(matches!(pricing_err, PricingError::InvalidInput(_)));
        if let PricingError::InvalidInput(msg) = pricing_err {
            assert!(msg.contains("Invalid jump"));
        }
    }

    #[test]
    fn test_jacobian_quality_good() {
        let quality = JacobianQuality::Good;
        assert!(quality.is_good());
        assert!(quality.is_acceptable());
        assert!(quality.reason().is_none());
        assert_eq!(format!("{quality}"), "Good");
    }

    #[test]
    fn test_jacobian_quality_warning() {
        let quality = JacobianQuality::Warning {
            reason: "near-zero diagonal",
        };
        assert!(!quality.is_good());
        assert!(quality.is_acceptable());
        assert_eq!(quality.reason(), Some("near-zero diagonal"));
        assert!(format!("{quality}").contains("Warning"));
    }

    #[test]
    fn test_jacobian_quality_poor() {
        let quality = JacobianQuality::Poor {
            reason: "NaN detected",
        };
        assert!(!quality.is_good());
        assert!(!quality.is_acceptable());
        assert_eq!(quality.reason(), Some("NaN detected"));
        assert!(format!("{quality}").contains("Poor"));
    }

    #[test]
    fn test_jacobian_quality_equality() {
        assert_eq!(JacobianQuality::Good, JacobianQuality::Good);
        assert_eq!(
            JacobianQuality::Warning { reason: "test" },
            JacobianQuality::Warning { reason: "test" }
        );
        assert_ne!(
            JacobianQuality::Good,
            JacobianQuality::Poor { reason: "NaN" }
        );
    }

    #[test]
    fn test_regularisation_none() {
        let reg: RegularisationType<f64> = RegularisationType::None;
        assert!(!reg.is_regularised());
        assert!(reg.damping().is_none());
        assert_eq!(format!("{reg}"), "None");
    }

    #[test]
    fn test_regularisation_tikhonov() {
        let reg: RegularisationType<f64> = RegularisationType::Tikhonov { damping: 1e-6 };
        assert!(reg.is_regularised());
        assert!((reg.damping().unwrap() - 1e-6).abs() < 1e-15);
        assert!(format!("{reg}").contains("Tikhonov"));
    }

    #[test]
    fn test_regularisation_levenberg_marquardt() {
        let reg: RegularisationType<f64> = RegularisationType::LevenbergMarquardt { lambda: 0.01 };
        assert!(reg.is_regularised());
        assert!((reg.damping().unwrap() - 0.01).abs() < 1e-15);
        assert!(format!("{reg}").contains("LM"));
    }

    #[test]
    fn test_regularisation_default() {
        let reg: RegularisationType<f64> = RegularisationType::default();
        assert!(!reg.is_regularised());
        assert!(matches!(reg, RegularisationType::None));
    }

    #[test]
    fn test_numerical_diagnostics_default() {
        let diag: NumericalDiagnostics<f64> = NumericalDiagnostics::default();
        assert!(diag.condition_number.is_none());
        assert!(diag.residual_history.is_empty());
        assert!(!diag.has_issues());
        assert!(!diag.was_regularised());
        assert!(!diag.ad_fallback_used);
    }

    #[test]
    fn test_numerical_diagnostics_with_condition_number() {
        let mut diag: NumericalDiagnostics<f64> = NumericalDiagnostics::default();
        diag.condition_number = Some(1e8);
        assert!(diag.condition_number.is_some());
        assert!((diag.condition_number.unwrap() - 1e8).abs() < 1.0);
    }

    #[test]
    fn test_numerical_diagnostics_residual_history() {
        let mut diag: NumericalDiagnostics<f64> = NumericalDiagnostics::default();
        diag.residual_history.push(1.0);
        diag.residual_history.push(0.1);
        diag.residual_history.push(0.01);

        assert_eq!(diag.iteration_count(), 3);
        assert!((diag.final_residual().unwrap() - 0.01).abs() < 1e-15);
    }

    #[test]
    fn test_numerical_diagnostics_issues() {
        let mut diag: NumericalDiagnostics<f64> = NumericalDiagnostics::default();
        assert!(!diag.has_issues());

        diag.nan_count += 1;
        assert!(diag.has_issues());
        assert_eq!(diag.nan_count, 1);

        diag.inf_count += 1;
        assert_eq!(diag.inf_count, 1);

        diag.near_zero_diagonal_count += 1;
        assert_eq!(diag.near_zero_diagonal_count, 1);
    }

    #[test]
    fn test_numerical_diagnostics_ill_conditioned() {
        let mut diag: NumericalDiagnostics<f64> = NumericalDiagnostics::default();
        diag.condition_number = Some(1e12);
        assert!(diag.is_ill_conditioned(1e10));
        assert!(!diag.is_ill_conditioned(1e14));
    }

    #[test]
    fn test_numerical_diagnostics_ad_fallback() {
        let mut diag: NumericalDiagnostics<f64> = NumericalDiagnostics::default();
        assert!(!diag.ad_fallback_used);
        diag.ad_fallback_used = true;
        assert!(diag.ad_fallback_used);
    }

    #[test]
    fn test_numerical_diagnostics_ad_unstable() {
        let mut diag: NumericalDiagnostics<f64> = NumericalDiagnostics::default();
        diag.ad_variance = Some(1e7);
        assert!(diag.is_ad_unstable(1e6));
        assert!(!diag.is_ad_unstable(1e8));
    }

    #[test]
    fn test_numerical_diagnostics_summary() {
        let mut diag: NumericalDiagnostics<f64> = NumericalDiagnostics::default();
        diag.condition_number = Some(1e8);
        diag.residual_history.push(0.001);
        diag.nan_count += 1;

        let summary = diag.summary();
        assert!(summary.contains("Iterations: 1"));
        assert!(summary.contains("NaN:1"));
    }

    #[test]
    fn test_numerical_diagnostics_with_regularisation() {
        let mut diag: NumericalDiagnostics<f64> = NumericalDiagnostics::default();
        diag.regularisation_applied = RegularisationType::Tikhonov { damping: 1e-6 };
        assert!(diag.was_regularised());
        let summary = diag.summary();
        assert!(summary.contains("Tikhonov"));
    }

    #[test]
    fn test_validate_jacobian_good() {
        let jacobian = vec![1.0, 0.1, 0.1, 1.0];
        let (quality, diag) = validate_jacobian_matrix(&jacobian, 2, 2, 1e-14);

        assert!(quality.is_good());
        assert!(!diag.has_issues());
    }

    #[test]
    fn test_validate_jacobian_with_nan() {
        let jacobian = vec![1.0, f64::NAN, 0.1, 1.0];
        let (quality, diag) = validate_jacobian_matrix(&jacobian, 2, 2, 1e-14);

        assert!(matches!(quality, JacobianQuality::Poor { .. }));
        assert_eq!(diag.nan_count, 1);
    }

    #[test]
    fn test_validate_jacobian_with_inf() {
        let jacobian = vec![1.0, 0.1, f64::INFINITY, 1.0];
        let (quality, diag) = validate_jacobian_matrix(&jacobian, 2, 2, 1e-14);

        assert!(matches!(quality, JacobianQuality::Poor { .. }));
        assert_eq!(diag.inf_count, 1);
    }

    #[test]
    fn test_validate_jacobian_near_zero_diagonal() {
        let jacobian = vec![1e-16, 0.1, 0.1, 1.0];
        let (quality, diag) = validate_jacobian_matrix(&jacobian, 2, 2, 1e-14);

        assert!(matches!(quality, JacobianQuality::Warning { .. }));
        assert_eq!(diag.near_zero_diagonal_count, 1);
    }

    #[test]
    fn test_should_apply_regularisation() {
        // Below threshold - no regularisation
        let damping = should_apply_regularisation(1e8, 1e10);
        assert!(damping.is_none());

        // Above threshold - regularisation needed
        let damping = should_apply_regularisation(1e12, 1e10);
        assert!(damping.is_some());
        assert!(damping.unwrap() > 0.0);
    }
}
