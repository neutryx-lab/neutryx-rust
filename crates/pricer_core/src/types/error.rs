//! Error types for structured error handling.
//!
//! This module provides:
//! - `PricingError`: Errors from pricing operations
//! - `SolverError`: Errors from root-finding solvers
//! - `CalibrationError`: Errors from model calibration
//!
//! For `DateError` and `CurrencyError`, import directly from `infra_domain`.

use std::fmt;

use thiserror::Error;

use crate::math::{linalg::LinearAlgebraError, normal_dist::DistributionError};

/// Categorised pricing errors.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PricingError {
    /// Invalid input data or parameters
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Numerical instability during computation
    #[error("Numerical instability: {0}")]
    NumericalInstability(String),

    /// Model failed to produce valid result
    #[error("Model failure: {0}")]
    ModelFailure(String),

    /// Instrument type not supported
    #[error("Unsupported instrument: {0}")]
    UnsupportedInstrument(String),
}

/// Root-finding solver errors.
#[derive(Error, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SolverError {
    /// Solver failed to converge within maximum iterations
    #[error("Failed to converge after {iterations} iterations")]
    MaxIterationsExceeded {
        /// Number of iterations attempted
        iterations: usize,
    },

    /// Derivative near zero (division by zero risk in Newton-Raphson)
    #[error("Derivative near zero at x = {x}")]
    DerivativeNearZero {
        /// The x value where derivative was near zero
        x: f64,
    },

    /// No valid bracket (function values at endpoints have same sign)
    #[error("No bracket: f({a}) and f({b}) have same sign")]
    NoBracket {
        /// Left bracket endpoint
        a: f64,
        /// Right bracket endpoint
        b: f64,
    },

    /// Jacobian matrix is singular or near-singular.
    #[error("Singular Jacobian: min pivot = {min_pivot:.2e}")]
    SingularJacobian {
        /// Smallest pivot value encountered during LU decomposition
        min_pivot: f64,
    },

    /// Dimension mismatch between input and expected values.
    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch {
        /// Expected dimension
        expected: usize,
        /// Actual dimension received
        got: usize,
    },

    /// Numerical instability during computation.
    #[error("Numerical instability: {0}")]
    NumericalInstability(String),

    /// External crate error (argmin, roots, levenberg-marquardt, etc.).
    #[error("External solver error: {0}")]
    External(String),
}

/// Convert linear algebra errors to solver errors.
impl From<LinearAlgebraError> for SolverError {
    fn from(err: LinearAlgebraError) -> Self {
        match err {
            LinearAlgebraError::SingularMatrix => SolverError::SingularJacobian { min_pivot: 0.0 },
            LinearAlgebraError::NotPositiveDefinite => {
                SolverError::NumericalInstability("Matrix is not positive definite".to_string())
            }
            LinearAlgebraError::NotSquare { rows, cols } => {
                SolverError::NumericalInstability(format!("Matrix is not square: {rows}x{cols}"))
            }
            LinearAlgebraError::DimensionMismatch { expected, got } => {
                SolverError::NumericalInstability(format!(
                    "Dimension mismatch: expected {expected}, got {got}"
                ))
            }
            LinearAlgebraError::DecompositionFailed(msg) => {
                SolverError::NumericalInstability(format!("Decomposition failed: {msg}"))
            }
            LinearAlgebraError::InvalidInput(msg) => {
                SolverError::NumericalInstability(format!("Invalid input: {msg}"))
            }
        }
    }
}

/// Calibration error kind.
#[derive(Error, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CalibrationErrorKind {
    /// Calibration did not converge within iteration limit.
    #[error("calibration did not converge")]
    NotConverged,

    /// Parameter constraint was violated.
    #[error("constraint violation: {0}")]
    InvalidConstraint(String),

    /// Numerical instability during calibration.
    #[error("numerical instability")]
    NumericalInstability,

    /// Insufficient market data for calibration.
    #[error("insufficient data: need at least {need} points, got {got}")]
    InsufficientData {
        /// Number of data points provided.
        got: usize,
        /// Minimum required data points.
        need: usize,
    },

    /// Invalid parameter value.
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),
}

/// Calibration error with detailed diagnostics.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CalibrationError {
    /// The type of calibration error.
    pub kind: CalibrationErrorKind,

    /// Final residual sum of squares.
    pub residual_ss: f64,

    /// Number of iterations performed.
    pub iterations: usize,

    /// Detailed error message.
    pub message: Option<String>,

    /// Final parameter values (if available).
    pub parameter_values: Option<Vec<f64>>,
}

impl CalibrationError {
    /// Create a new calibration error with auto-generated message from the
    /// kind.
    pub fn new(kind: CalibrationErrorKind) -> Self {
        let message = Some(format!("{kind}"));
        Self {
            kind,
            residual_ss: f64::NAN,
            iterations: 0,
            message,
            parameter_values: None,
        }
    }

    /// Create a not-converged error with iteration and residual context.
    pub fn not_converged(iterations: usize, residual_ss: f64) -> Self {
        Self {
            kind: CalibrationErrorKind::NotConverged,
            residual_ss,
            iterations,
            message: Some(format!(
                "Failed to converge after {iterations} iterations (residual_ss: {residual_ss:.6e})"
            )),
            parameter_values: None,
        }
    }

    /// Create a constraint violation error.
    pub fn constraint_violation(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        Self::new(CalibrationErrorKind::InvalidConstraint(msg))
    }

    /// Create a numerical instability error.
    pub fn numerical_instability(msg: impl Into<String>) -> Self {
        Self::new(CalibrationErrorKind::NumericalInstability).with_message(msg)
    }

    /// Create an insufficient data error.
    pub fn insufficient_data(got: usize, need: usize) -> Self {
        Self::new(CalibrationErrorKind::InsufficientData { got, need })
    }

    /// Create an invalid parameter error.
    pub fn invalid_parameter(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        Self::new(CalibrationErrorKind::InvalidParameter(msg))
    }

    /// Set the final parameter values.
    pub fn with_parameters(mut self, params: Vec<f64>) -> Self {
        self.parameter_values = Some(params);
        self
    }

    /// Set the residual sum of squares.
    pub fn with_residual(mut self, residual_ss: f64) -> Self {
        self.residual_ss = residual_ss;
        self
    }

    /// Set the iteration count.
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Set a detailed message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Get the root mean square error (if residual count is known).
    pub fn rmse(&self, n_observations: usize) -> f64 {
        if n_observations == 0 || self.residual_ss.is_nan() {
            f64::NAN
        } else {
            (self.residual_ss / n_observations as f64).sqrt()
        }
    }

    /// Check if the error is due to non-convergence.
    pub fn is_not_converged(&self) -> bool {
        matches!(self.kind, CalibrationErrorKind::NotConverged)
    }

    /// Check if the error is due to a constraint violation.
    pub fn is_constraint_violation(&self) -> bool {
        matches!(self.kind, CalibrationErrorKind::InvalidConstraint(_))
    }

    /// Check if the error is due to numerical instability.
    pub fn is_numerical_instability(&self) -> bool {
        matches!(self.kind, CalibrationErrorKind::NumericalInstability)
    }

    /// Check if the error is due to insufficient data.
    pub fn is_insufficient_data(&self) -> bool {
        matches!(self.kind, CalibrationErrorKind::InsufficientData { .. })
    }
}

impl fmt::Display for CalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Calibration error: {}", self.kind)?;
        if let Some(ref msg) = self.message {
            if !matches!(self.kind, CalibrationErrorKind::NotConverged) {
                write!(f, " - {}", msg)?;
            }
        }
        if self.iterations > 0 {
            write!(f, " (after {} iterations)", self.iterations)?;
        }
        if !self.residual_ss.is_nan() {
            write!(f, " [residual_ss: {:.6e}]", self.residual_ss)?;
        }
        Ok(())
    }
}

impl std::error::Error for CalibrationError {}

impl From<SolverError> for CalibrationError {
    fn from(err: SolverError) -> Self {
        match err {
            SolverError::MaxIterationsExceeded { iterations } => {
                CalibrationError::not_converged(iterations, f64::NAN)
            }
            SolverError::NumericalInstability(msg) => CalibrationError::numerical_instability(msg),
            SolverError::DerivativeNearZero { x } => CalibrationError::numerical_instability(
                format!("Derivative near zero at x = {}", x),
            ),
            SolverError::NoBracket { a, b } => CalibrationError::numerical_instability(format!(
                "No bracket found between {} and {}",
                a, b
            )),
            SolverError::SingularJacobian { min_pivot } => CalibrationError::numerical_instability(
                format!("Singular Jacobian matrix: min pivot = {min_pivot:.2e}"),
            ),
            SolverError::DimensionMismatch { expected, got } => {
                CalibrationError::invalid_parameter(format!(
                    "Dimension mismatch: expected {expected}, got {got}"
                ))
            }
            SolverError::External(msg) => {
                CalibrationError::numerical_instability(format!("External solver error: {msg}"))
            }
        }
    }
}

/// Convert distribution errors to pricing errors.
impl From<DistributionError> for PricingError {
    fn from(err: DistributionError) -> Self {
        match err {
            DistributionError::InvalidProbability { p } => {
                PricingError::InvalidInput(format!("Invalid probability: {p}"))
            }
            DistributionError::NumericalError(msg) => PricingError::NumericalInstability(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::normal_dist::DistributionError;

    /// Assert Display output equals expected string.
    macro_rules! assert_display {
        ($err:expr, $expected:expr) => {
            assert_eq!(format!("{}", $err), $expected);
        };
    }

    /// Assert Display output contains substring.
    macro_rules! assert_display_contains {
        ($err:expr, $($sub:expr),+ $(,)?) => {{
            let display = format!("{}", $err);
            $( assert!(display.contains($sub), "Expected '{}' in '{}'", $sub, display); )+
        }};
    }

    // ── PricingError ─────────────────────────────────────────────

    #[test]
    fn pricing_error_display() {
        assert_display!(
            PricingError::InvalidInput("Test error".into()),
            "Invalid input: Test error"
        );
        assert_display!(
            PricingError::NumericalInstability("Failed".into()),
            "Numerical instability: Failed"
        );
        assert_display!(
            PricingError::ModelFailure("Vol OOR".into()),
            "Model failure: Vol OOR"
        );
        assert_display!(
            PricingError::UnsupportedInstrument("Asian".into()),
            "Unsupported instrument: Asian"
        );
    }

    #[test]
    fn pricing_error_clone_eq_trait() {
        let err = PricingError::InvalidInput("Test".into());
        assert_eq!(err.clone(), err);
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn distribution_error_to_pricing() {
        let p: PricingError = DistributionError::InvalidProbability { p: 1.5 }.into();
        assert!(matches!(p, PricingError::InvalidInput(_)));
        assert_display_contains!(p, "1.5");

        let p: PricingError = DistributionError::NumericalError("underflow".into()).into();
        assert!(matches!(p, PricingError::NumericalInstability(_)));
    }

    // ── SolverError ──────────────────────────────────────────────

    #[test]
    fn solver_error_display() {
        assert_display!(
            SolverError::MaxIterationsExceeded { iterations: 100 },
            "Failed to converge after 100 iterations"
        );
        assert_display!(
            SolverError::DerivativeNearZero { x: 1.5 },
            "Derivative near zero at x = 1.5"
        );
        assert_display!(
            SolverError::NoBracket { a: 0.0, b: 1.0 },
            "No bracket: f(0) and f(1) have same sign"
        );
        assert_display!(
            SolverError::NumericalInstability("overflow detected".into()),
            "Numerical instability: overflow detected"
        );
        assert_display_contains!(
            SolverError::SingularJacobian { min_pivot: 1e-15 },
            "Singular Jacobian",
            "1.00e-15"
        );
        assert_display_contains!(
            SolverError::DimensionMismatch {
                expected: 10,
                got: 5
            },
            "Dimension mismatch",
            "10",
            "5"
        );
        assert_display_contains!(
            SolverError::External("lm failed".into()),
            "External",
            "lm failed"
        );
    }

    #[test]
    fn solver_error_clone_eq_trait() {
        let cases: Vec<SolverError> = vec![
            SolverError::NoBracket { a: 0.0, b: 1.0 },
            SolverError::SingularJacobian { min_pivot: 1e-12 },
            SolverError::DimensionMismatch {
                expected: 5,
                got: 3,
            },
        ];
        for err in &cases {
            assert_eq!(err.clone(), *err);
        }
        let _: &dyn std::error::Error = &cases[0];
    }

    #[test]
    fn solver_error_from_linear_algebra() {
        use crate::math::linalg::LinearAlgebraError;

        let s: SolverError = LinearAlgebraError::SingularMatrix.into();
        assert!(matches!(s, SolverError::SingularJacobian { .. }));

        let s: SolverError = LinearAlgebraError::DimensionMismatch {
            expected: "3x3".into(),
            got: "2x3".into(),
        }
        .into();
        assert!(matches!(s, SolverError::NumericalInstability(_)));
    }

    #[test]
    fn solver_error_serde_roundtrip() {
        let err = SolverError::MaxIterationsExceeded { iterations: 100 };
        let json = serde_json::to_string(&err).unwrap();
        let de: SolverError = serde_json::from_str(&json).unwrap();
        assert_eq!(err, de);
    }

    // ── CalibrationErrorKind ─────────────────────────────────────

    #[test]
    fn calibration_kind_display() {
        assert_display!(
            CalibrationErrorKind::NotConverged,
            "calibration did not converge"
        );
        assert_display!(
            CalibrationErrorKind::NumericalInstability,
            "numerical instability"
        );
        assert_display_contains!(
            CalibrationErrorKind::InvalidConstraint("test".into()),
            "constraint violation"
        );
        assert_display_contains!(
            CalibrationErrorKind::InsufficientData { got: 3, need: 10 },
            "insufficient data"
        );
    }

    // ── CalibrationError factories ───────────────────────────────

    #[test]
    fn calibration_new_defaults() {
        let err = CalibrationError::new(CalibrationErrorKind::NotConverged);
        assert!(matches!(err.kind, CalibrationErrorKind::NotConverged));
        assert_eq!(err.iterations, 0);
        assert!(err.residual_ss.is_nan());
    }

    #[test]
    fn calibration_not_converged() {
        let err = CalibrationError::not_converged(100, 0.01);
        assert!(err.is_not_converged());
        assert_eq!(err.iterations, 100);
        assert!((err.residual_ss - 0.01).abs() < 1e-15);
        assert!(err.message.is_some());
    }

    #[test]
    fn calibration_constraint_violation() {
        let err = CalibrationError::constraint_violation("alpha must be positive");
        assert!(err.is_constraint_violation());
        assert_display_contains!(err, "constraint violation");
    }

    #[test]
    fn calibration_numerical_instability() {
        let err = CalibrationError::numerical_instability("NaN encountered");
        assert!(err.is_numerical_instability());
        assert!(err.message.as_ref().unwrap().contains("NaN"));
    }

    #[test]
    fn calibration_insufficient_data() {
        let err = CalibrationError::insufficient_data(3, 10);
        assert!(err.is_insufficient_data());
        assert!(matches!(
            err.kind,
            CalibrationErrorKind::InsufficientData { got: 3, need: 10 }
        ));
    }

    #[test]
    fn calibration_invalid_parameter() {
        let err = CalibrationError::invalid_parameter("vol < 0");
        assert!(matches!(
            err.kind,
            CalibrationErrorKind::InvalidParameter(_)
        ));
    }

    // ── CalibrationError builders ────────────────────────────────

    #[test]
    fn calibration_builder_chain() {
        let err = CalibrationError::not_converged(10, 0.1)
            .with_parameters(vec![0.5, 1.0])
            .with_message("custom");
        assert_eq!(err.parameter_values.as_ref().unwrap().len(), 2);
        assert_eq!(err.message, Some("custom".into()));

        let err = CalibrationError::new(CalibrationErrorKind::NotConverged)
            .with_residual(0.005)
            .with_iterations(50);
        assert!((err.residual_ss - 0.005).abs() < 1e-15);
        assert_eq!(err.iterations, 50);
    }

    // ── CalibrationError::rmse ───────────────────────────────────

    #[test]
    fn calibration_rmse() {
        let err = CalibrationError::not_converged(10, 4.0);
        assert!((err.rmse(4) - 1.0).abs() < 1e-10);
        assert!(
            CalibrationError::new(CalibrationErrorKind::NumericalInstability)
                .rmse(10)
                .is_nan()
        );
        assert!(CalibrationError::not_converged(10, 1.0).rmse(0).is_nan());
    }

    // ── CalibrationError Display ─────────────────────────────────

    #[test]
    fn calibration_display() {
        let err = CalibrationError::not_converged(100, 0.01);
        assert_display_contains!(err, "Calibration error", "100 iterations");
    }

    #[test]
    fn calibration_clone_eq_trait() {
        let err = CalibrationError::not_converged(100, 0.01);
        assert_eq!(err.clone(), err);
        let _: &dyn std::error::Error = &err;
    }

    // ── SolverError → CalibrationError conversion ────────────────

    #[test]
    fn calibration_from_solver() {
        let c: CalibrationError = SolverError::MaxIterationsExceeded { iterations: 50 }.into();
        assert!(c.is_not_converged());
        assert_eq!(c.iterations, 50);

        let c: CalibrationError = SolverError::NumericalInstability("overflow".into()).into();
        assert!(c.is_numerical_instability());

        let c: CalibrationError = SolverError::DerivativeNearZero { x: 1.5 }.into();
        assert!(c.is_numerical_instability());

        let c: CalibrationError = SolverError::NoBracket { a: 0.0, b: 1.0 }.into();
        assert!(c.is_numerical_instability());

        let c: CalibrationError = SolverError::SingularJacobian { min_pivot: 1e-14 }.into();
        assert!(c.is_numerical_instability());
        assert!(c.message.as_ref().unwrap().contains("Singular"));

        let c: CalibrationError = SolverError::DimensionMismatch {
            expected: 10,
            got: 5,
        }
        .into();
        assert!(matches!(c.kind, CalibrationErrorKind::InvalidParameter(_)));

        let c: CalibrationError = SolverError::External("roots crate error".into()).into();
        assert!(c.is_numerical_instability());
        assert!(c.message.as_ref().unwrap().contains("External"));
    }
}
