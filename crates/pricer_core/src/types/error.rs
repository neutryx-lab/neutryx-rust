//! Error types for structured error handling.
//!
//! This module provides:
//! - `PricingError`: Errors from pricing operations
//! - `InterpolationError`: Errors from interpolation operations
//! - `SolverError`: Errors from root-finding solvers
//! - `CalibrationError`: Errors from model calibration
//!
//! For `DateError` and `CurrencyError`, import directly from `infra_master`.

use std::fmt;

use thiserror::Error;

// Import math errors for From implementations
use crate::math::distributions::DistributionError;
#[cfg(feature = "linalg")]
use crate::math::linalg::LinearAlgebraError;
use crate::math::{
    fitting::FittingError, integrators::IntegrationError, optimisers::OptimisationError,
};

/// Categorised pricing errors.
///
/// Provides structured error handling for pricing operations with
/// descriptive context for each failure mode.
///
/// # Variants
/// - `InvalidInput`: Invalid market data or parameters
/// - `NumericalInstability`: Computation failed to converge
/// - `ModelFailure`: Model assumptions violated
/// - `UnsupportedInstrument`: Instrument type not supported by model
///
/// # Examples
/// ```
/// use pricer_core::types::PricingError;
///
/// let err = PricingError::InvalidInput("Negative spot price".to_string());
/// assert_eq!(format!("{}", err), "Invalid input: Negative spot price");
/// ```
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

/// Interpolation-related errors.
///
/// Provides structured error handling for interpolation operations
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `OutOfBounds`: Query point outside valid interpolation domain
/// - `InsufficientData`: Not enough data points for interpolation
/// - `NonMonotonicData`: Data violates monotonicity requirement
/// - `InvalidInput`: General invalid input error
///
/// # Examples
/// ```
/// use pricer_core::types::InterpolationError;
///
/// let err = InterpolationError::OutOfBounds { x: 5.0, min: 0.0, max: 3.0 };
/// assert!(format!("{}", err).contains("outside valid domain"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InterpolationError {
    /// Query point outside valid interpolation domain.
    #[error("Query point {x} outside valid domain [{min}, {max}]")]
    OutOfBounds {
        /// The query point that was out of bounds
        x: f64,
        /// Minimum valid value
        min: f64,
        /// Maximum valid value
        max: f64,
    },

    /// Insufficient data points for interpolation.
    #[error("Insufficient data points: got {got}, need at least {need}")]
    InsufficientData {
        /// Number of points provided
        got: usize,
        /// Minimum number of points required
        need: usize,
    },

    /// Data is not monotonic when monotonicity is required.
    #[error("Data is not monotonic at index {index}")]
    NonMonotonicData {
        /// Index where monotonicity violation was detected
        index: usize,
    },

    /// Invalid input data or parameters.
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Root-finding solver errors.
///
/// Provides structured error handling for root-finding solver operations
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `MaxIterationsExceeded`: Solver failed to converge within iteration limit
/// - `DerivativeNearZero`: Derivative too small for Newton-Raphson
/// - `NoBracket`: Function values at bracket endpoints have same sign
/// - `SingularJacobian`: Jacobian matrix is singular (multi-dimensional
///   solvers)
/// - `DimensionMismatch`: Input/output dimensions do not match expected values
/// - `NumericalInstability`: General numerical instability
/// - `External`: Error from external crate (argmin, roots, levenberg-marquardt)
///
/// # Examples
/// ```
/// use pricer_core::types::SolverError;
///
/// let err = SolverError::MaxIterationsExceeded { iterations: 100 };
/// assert!(format!("{}", err).contains("100 iterations"));
///
/// // Multi-dimensional solver errors
/// let err = SolverError::SingularJacobian { min_pivot: 1e-15 };
/// assert!(format!("{}", err).contains("Singular Jacobian"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SolverError {
    /// Solver failed to converge within maximum iterations.
    #[error("Failed to converge after {iterations} iterations")]
    MaxIterationsExceeded {
        /// Number of iterations attempted
        iterations: usize,
    },

    /// Derivative near zero (division by zero risk in Newton-Raphson).
    #[error("Derivative near zero at x = {x}")]
    DerivativeNearZero {
        /// The x value where derivative was near zero
        x: f64,
    },

    /// No valid bracket (function values at endpoints have same sign).
    #[error("No bracket: f({a}) and f({b}) have same sign")]
    NoBracket {
        /// Left bracket endpoint
        a: f64,
        /// Right bracket endpoint
        b: f64,
    },

    /// Jacobian matrix is singular or near-singular (multi-dimensional
    /// solvers).
    ///
    /// This occurs when the Jacobian matrix cannot be inverted, typically
    /// indicating the problem is ill-conditioned or the current iterate
    /// is at a singular point.
    #[error("Singular Jacobian: min pivot = {min_pivot:.2e}")]
    SingularJacobian {
        /// Smallest pivot value encountered during LU decomposition
        min_pivot: f64,
    },

    /// Dimension mismatch between input and expected values.
    ///
    /// This occurs when the residual vector or Jacobian matrix dimensions
    /// do not match the expected problem dimension.
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
///
/// This conversion enables seamless error propagation from linear algebra
/// operations (matrix decomposition, inversion) to solver error handling.
#[cfg(feature = "linalg")]
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
///
/// Categorises the type of calibration failure.
///
/// # Variants
/// - `NotConverged`: Calibration failed to converge within iteration limit
/// - `InvalidConstraint`: Parameter constraints were violated
/// - `NumericalInstability`: Numerical issues during calibration
/// - `InsufficientData`: Not enough market data for calibration
/// - `InvalidParameter`: Invalid parameter value during calibration
#[derive(Error, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
///
/// Provides comprehensive information about calibration failures including
/// residuals, iteration count, and convergence metrics.
///
/// # Fields
/// - `kind`: The type of calibration error
/// - `residual_ss`: Final residual sum of squares
/// - `iterations`: Number of iterations performed
/// - `message`: Optional detailed error message
/// - `parameter_values`: Optional final parameter values
///
/// # Examples
/// ```
/// use pricer_core::types::{CalibrationError, CalibrationErrorKind};
///
/// // Create a not-converged error
/// let err = CalibrationError::not_converged(100, 0.01);
/// assert_eq!(err.iterations, 100);
///
/// // Create a constraint violation error
/// let err = CalibrationError::constraint_violation("alpha must be positive");
/// assert!(format!("{}", err).contains("constraint violation"));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Create a new calibration error.
    pub fn new(kind: CalibrationErrorKind) -> Self {
        Self {
            kind,
            residual_ss: f64::NAN,
            iterations: 0,
            message: None,
            parameter_values: None,
        }
    }

    /// Create a not-converged error.
    ///
    /// # Arguments
    /// * `iterations` - Number of iterations performed
    /// * `residual_ss` - Final residual sum of squares
    pub fn not_converged(iterations: usize, residual_ss: f64) -> Self {
        Self {
            kind: CalibrationErrorKind::NotConverged,
            residual_ss,
            iterations,
            message: Some(format!(
                "Failed to converge after {} iterations (residual_ss: {:.6e})",
                iterations, residual_ss
            )),
            parameter_values: None,
        }
    }

    /// Create a constraint violation error.
    ///
    /// # Arguments
    /// * `constraint` - Description of violated constraint
    pub fn constraint_violation(constraint: impl Into<String>) -> Self {
        let msg = constraint.into();
        Self {
            kind: CalibrationErrorKind::InvalidConstraint(msg.clone()),
            residual_ss: f64::NAN,
            iterations: 0,
            message: Some(msg),
            parameter_values: None,
        }
    }

    /// Create a numerical instability error.
    ///
    /// # Arguments
    /// * `message` - Description of the numerical issue
    pub fn numerical_instability(message: impl Into<String>) -> Self {
        Self {
            kind: CalibrationErrorKind::NumericalInstability,
            residual_ss: f64::NAN,
            iterations: 0,
            message: Some(message.into()),
            parameter_values: None,
        }
    }

    /// Create an insufficient data error.
    ///
    /// # Arguments
    /// * `got` - Number of data points provided
    /// * `need` - Minimum required data points
    pub fn insufficient_data(got: usize, need: usize) -> Self {
        Self {
            kind: CalibrationErrorKind::InsufficientData { got, need },
            residual_ss: f64::NAN,
            iterations: 0,
            message: Some(format!(
                "Insufficient data: got {} points, need at least {}",
                got, need
            )),
            parameter_values: None,
        }
    }

    /// Create an invalid parameter error.
    ///
    /// # Arguments
    /// * `message` - Description of the invalid parameter
    pub fn invalid_parameter(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            kind: CalibrationErrorKind::InvalidParameter(msg.clone()),
            residual_ss: f64::NAN,
            iterations: 0,
            message: Some(msg),
            parameter_values: None,
        }
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

// =============================================================================
// Error Conversions: Math Errors → Domain Errors
// =============================================================================
// These conversions enable seamless error propagation from mathematical
// operations to domain-level error types (PricingError, CalibrationError).

/// Convert optimisation errors to calibration errors.
///
/// Optimisation is commonly used in model calibration, so this conversion
/// provides natural error propagation.
impl From<OptimisationError> for CalibrationError {
    fn from(err: OptimisationError) -> Self {
        match err {
            OptimisationError::NotConverged { iterations } => {
                CalibrationError::not_converged(iterations, f64::NAN)
            }
            OptimisationError::InvalidInput(msg) => CalibrationError::invalid_parameter(msg),
            OptimisationError::NumericalError(msg) => CalibrationError::numerical_instability(msg),
            OptimisationError::DimensionMismatch { expected, got } => {
                CalibrationError::invalid_parameter(format!(
                    "Dimension mismatch: expected {expected}, got {got}"
                ))
            }
            OptimisationError::BoundsError(msg) => CalibrationError::constraint_violation(msg),
            OptimisationError::GradientError(msg) => CalibrationError::numerical_instability(
                format!("Gradient computation failed: {msg}"),
            ),
            OptimisationError::LineSearchError(msg) => {
                CalibrationError::numerical_instability(format!("Line search failed: {msg}"))
            }
            OptimisationError::External(msg) => CalibrationError::numerical_instability(format!(
                "External optimisation error: {msg}"
            )),
        }
    }
}

/// Convert fitting errors to calibration errors.
///
/// Curve fitting is a common calibration task, so this conversion enables
/// natural error propagation from fitting algorithms to calibration workflows.
impl From<FittingError> for CalibrationError {
    fn from(err: FittingError) -> Self {
        match err {
            FittingError::InsufficientData { needed, got } => {
                CalibrationError::insufficient_data(got, needed)
            }
            FittingError::DimensionMismatch(msg) => CalibrationError::invalid_parameter(msg),
            FittingError::FittingFailed(msg) => CalibrationError::numerical_instability(msg),
            FittingError::InvalidData(msg) => CalibrationError::invalid_parameter(msg),
            FittingError::NumericalError(msg) => CalibrationError::numerical_instability(msg),
        }
    }
}

/// Convert integration errors to pricing errors.
///
/// Numerical integration is used in option pricing (e.g., integrating payoffs),
/// so this conversion enables natural error propagation.
impl From<IntegrationError> for PricingError {
    fn from(err: IntegrationError) -> Self {
        match err {
            IntegrationError::NotConverged { max_iterations } => {
                PricingError::NumericalInstability(format!(
                    "Integration did not converge after {max_iterations} iterations"
                ))
            }
            IntegrationError::InvalidBounds { a, b } => {
                PricingError::InvalidInput(format!("Invalid integration bounds: [{a}, {b}]"))
            }
            IntegrationError::NumericalError(msg) => PricingError::NumericalInstability(msg),
        }
    }
}

/// Convert distribution errors to pricing errors.
///
/// Probability distributions are fundamental to option pricing, so this
/// conversion enables natural error propagation.
impl From<DistributionError> for PricingError {
    fn from(err: DistributionError) -> Self {
        match err {
            DistributionError::InvalidProbability { p } => {
                PricingError::InvalidInput(format!("Invalid probability: {p}"))
            }
            DistributionError::InvalidCorrelation { rho } => {
                PricingError::InvalidInput(format!("Invalid correlation: {rho}"))
            }
            DistributionError::InvalidDegreesOfFreedom { df } => {
                PricingError::InvalidInput(format!("Invalid degrees of freedom: {df}"))
            }
            DistributionError::InvalidNonCentrality { ncp } => {
                PricingError::InvalidInput(format!("Invalid non-centrality parameter: {ncp}"))
            }
            DistributionError::NotPositiveDefinite => PricingError::NumericalInstability(
                "Correlation matrix is not positive definite".to_string(),
            ),
            DistributionError::NumericalError(msg) => PricingError::NumericalInstability(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{
        distributions::DistributionError, fitting::FittingError, integrators::IntegrationError,
        optimisers::OptimisationError,
    };

    // ==========================================================================
    // Math Error Conversion Tests (Task 2.1)
    // ==========================================================================

    #[test]
    fn test_optimisation_error_to_calibration_not_converged() {
        let opt_err = OptimisationError::NotConverged { iterations: 100 };
        let calib_err: CalibrationError = opt_err.into();
        assert!(calib_err.is_not_converged());
        assert_eq!(calib_err.iterations, 100);
    }

    #[test]
    fn test_optimisation_error_to_calibration_invalid_input() {
        let opt_err = OptimisationError::InvalidInput("negative step size".to_string());
        let calib_err: CalibrationError = opt_err.into();
        assert!(matches!(
            calib_err.kind,
            CalibrationErrorKind::InvalidParameter(_)
        ));
    }

    #[test]
    fn test_optimisation_error_to_calibration_numerical() {
        let opt_err = OptimisationError::NumericalError("overflow".to_string());
        let calib_err: CalibrationError = opt_err.into();
        assert!(calib_err.is_numerical_instability());
    }

    #[test]
    fn test_optimisation_error_to_calibration_bounds() {
        let opt_err = OptimisationError::BoundsError("parameter out of bounds".to_string());
        let calib_err: CalibrationError = opt_err.into();
        assert!(calib_err.is_constraint_violation());
    }

    #[test]
    fn test_optimisation_error_to_calibration_dimension() {
        let opt_err = OptimisationError::DimensionMismatch {
            expected: 3,
            got: 5,
        };
        let calib_err: CalibrationError = opt_err.into();
        assert!(matches!(
            calib_err.kind,
            CalibrationErrorKind::InvalidParameter(_)
        ));
    }

    #[test]
    fn test_optimisation_error_to_calibration_gradient() {
        let opt_err = OptimisationError::GradientError("NaN gradient".to_string());
        let calib_err: CalibrationError = opt_err.into();
        assert!(calib_err.is_numerical_instability());
    }

    #[test]
    fn test_optimisation_error_to_calibration_line_search() {
        let opt_err = OptimisationError::LineSearchError("backtrack failed".to_string());
        let calib_err: CalibrationError = opt_err.into();
        assert!(calib_err.is_numerical_instability());
    }

    #[test]
    fn test_fitting_error_to_calibration_insufficient_data() {
        let fit_err = FittingError::InsufficientData { needed: 10, got: 3 };
        let calib_err: CalibrationError = fit_err.into();
        assert!(calib_err.is_insufficient_data());
        if let CalibrationErrorKind::InsufficientData { got, need } = calib_err.kind {
            assert_eq!(got, 3);
            assert_eq!(need, 10);
        } else {
            panic!("Expected InsufficientData");
        }
    }

    #[test]
    fn test_fitting_error_to_calibration_dimension_mismatch() {
        let fit_err = FittingError::DimensionMismatch("x and y lengths differ".to_string());
        let calib_err: CalibrationError = fit_err.into();
        assert!(matches!(
            calib_err.kind,
            CalibrationErrorKind::InvalidParameter(_)
        ));
    }

    #[test]
    fn test_fitting_error_to_calibration_fitting_failed() {
        let fit_err = FittingError::FittingFailed("singular matrix".to_string());
        let calib_err: CalibrationError = fit_err.into();
        assert!(calib_err.is_numerical_instability());
    }

    #[test]
    fn test_fitting_error_to_calibration_invalid_data() {
        let fit_err = FittingError::InvalidData("negative weights".to_string());
        let calib_err: CalibrationError = fit_err.into();
        assert!(matches!(
            calib_err.kind,
            CalibrationErrorKind::InvalidParameter(_)
        ));
    }

    #[test]
    fn test_fitting_error_to_calibration_numerical() {
        let fit_err = FittingError::NumericalError("overflow".to_string());
        let calib_err: CalibrationError = fit_err.into();
        assert!(calib_err.is_numerical_instability());
    }

    #[test]
    fn test_integration_error_to_pricing_not_converged() {
        let int_err = IntegrationError::NotConverged { max_iterations: 50 };
        let pricing_err: PricingError = int_err.into();
        assert!(matches!(pricing_err, PricingError::NumericalInstability(_)));
        assert!(format!("{pricing_err}").contains("50"));
    }

    #[test]
    fn test_integration_error_to_pricing_invalid_bounds() {
        let int_err = IntegrationError::InvalidBounds { a: 5.0, b: 2.0 };
        let pricing_err: PricingError = int_err.into();
        assert!(matches!(pricing_err, PricingError::InvalidInput(_)));
    }

    #[test]
    fn test_integration_error_to_pricing_numerical() {
        let int_err = IntegrationError::NumericalError("overflow".to_string());
        let pricing_err: PricingError = int_err.into();
        assert!(matches!(pricing_err, PricingError::NumericalInstability(_)));
    }

    #[test]
    fn test_distribution_error_to_pricing_invalid_probability() {
        let dist_err = DistributionError::InvalidProbability { p: 1.5 };
        let pricing_err: PricingError = dist_err.into();
        assert!(matches!(pricing_err, PricingError::InvalidInput(_)));
        assert!(format!("{pricing_err}").contains("1.5"));
    }

    #[test]
    fn test_distribution_error_to_pricing_invalid_correlation() {
        let dist_err = DistributionError::InvalidCorrelation { rho: 1.5 };
        let pricing_err: PricingError = dist_err.into();
        assert!(matches!(pricing_err, PricingError::InvalidInput(_)));
    }

    #[test]
    fn test_distribution_error_to_pricing_invalid_df() {
        let dist_err = DistributionError::InvalidDegreesOfFreedom { df: -1.0 };
        let pricing_err: PricingError = dist_err.into();
        assert!(matches!(pricing_err, PricingError::InvalidInput(_)));
    }

    #[test]
    fn test_distribution_error_to_pricing_invalid_ncp() {
        let dist_err = DistributionError::InvalidNonCentrality { ncp: -0.5 };
        let pricing_err: PricingError = dist_err.into();
        assert!(matches!(pricing_err, PricingError::InvalidInput(_)));
    }

    #[test]
    fn test_distribution_error_to_pricing_not_positive_definite() {
        let dist_err = DistributionError::NotPositiveDefinite;
        let pricing_err: PricingError = dist_err.into();
        assert!(matches!(pricing_err, PricingError::NumericalInstability(_)));
    }

    #[test]
    fn test_distribution_error_to_pricing_numerical() {
        let dist_err = DistributionError::NumericalError("underflow".to_string());
        let pricing_err: PricingError = dist_err.into();
        assert!(matches!(pricing_err, PricingError::NumericalInstability(_)));
    }

    // ==========================================================================
    // Original PricingError Tests
    // ==========================================================================

    #[test]
    fn test_invalid_input_display() {
        let err = PricingError::InvalidInput("Test error".to_string());
        assert_eq!(format!("{}", err), "Invalid input: Test error");
    }

    #[test]
    fn test_numerical_instability_display() {
        let err = PricingError::NumericalInstability("Failed to converge".to_string());
        assert_eq!(
            format!("{}", err),
            "Numerical instability: Failed to converge"
        );
    }

    #[test]
    fn test_model_failure_display() {
        let err = PricingError::ModelFailure("Volatility out of range".to_string());
        assert_eq!(format!("{}", err), "Model failure: Volatility out of range");
    }

    #[test]
    fn test_unsupported_instrument_display() {
        let err = PricingError::UnsupportedInstrument("Asian option".to_string());
        assert_eq!(format!("{}", err), "Unsupported instrument: Asian option");
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = PricingError::InvalidInput("Test".to_string());
        let _: &dyn std::error::Error = &err; // Verify Error trait is
                                              // implemented
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = PricingError::InvalidInput("Test".to_string());
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // Note: DateError and CurrencyError tests are in infra_master

    // InterpolationError tests

    #[test]
    fn test_interpolation_error_out_of_bounds_display() {
        let err = InterpolationError::OutOfBounds {
            x: 5.0,
            min: 0.0,
            max: 3.0,
        };
        assert_eq!(
            format!("{}", err),
            "Query point 5 outside valid domain [0, 3]"
        );
    }

    #[test]
    fn test_interpolation_error_insufficient_data_display() {
        let err = InterpolationError::InsufficientData { got: 1, need: 2 };
        assert_eq!(
            format!("{}", err),
            "Insufficient data points: got 1, need at least 2"
        );
    }

    #[test]
    fn test_interpolation_error_non_monotonic_display() {
        let err = InterpolationError::NonMonotonicData { index: 3 };
        assert_eq!(format!("{}", err), "Data is not monotonic at index 3");
    }

    #[test]
    fn test_interpolation_error_invalid_input_display() {
        let err = InterpolationError::InvalidInput("empty array".to_string());
        assert_eq!(format!("{}", err), "Invalid input: empty array");
    }

    #[test]
    fn test_interpolation_error_trait_implementation() {
        let err = InterpolationError::OutOfBounds {
            x: 5.0,
            min: 0.0,
            max: 3.0,
        };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_interpolation_error_clone_and_equality() {
        let err1 = InterpolationError::InsufficientData { got: 1, need: 2 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // SolverError tests - Multi-dimensional solver extensions (Task 1.1)

    #[test]
    fn test_solver_error_singular_jacobian_display() {
        let err = SolverError::SingularJacobian { min_pivot: 1e-15 };
        let display = format!("{err}");
        assert!(display.contains("Singular Jacobian"));
        // Format is {:.2e} so we check for the scientific notation
        assert!(display.contains("1.00e-15"));
    }

    #[test]
    fn test_solver_error_dimension_mismatch_display() {
        let err = SolverError::DimensionMismatch {
            expected: 10,
            got: 5,
        };
        let display = format!("{err}");
        assert!(display.contains("Dimension mismatch"));
        assert!(display.contains("10"));
        assert!(display.contains("5"));
    }

    #[cfg(feature = "linalg")]
    #[test]
    fn test_solver_error_from_linear_algebra_singular() {
        use crate::math::linalg::LinearAlgebraError;
        let la_err = LinearAlgebraError::SingularMatrix;
        let solver_err: SolverError = la_err.into();
        assert!(matches!(solver_err, SolverError::SingularJacobian { .. }));
    }

    #[cfg(feature = "linalg")]
    #[test]
    fn test_solver_error_from_linear_algebra_dimension_mismatch() {
        use crate::math::linalg::LinearAlgebraError;
        let la_err = LinearAlgebraError::DimensionMismatch {
            expected: "3x3".to_string(),
            got: "2x3".to_string(),
        };
        let solver_err: SolverError = la_err.into();
        assert!(matches!(solver_err, SolverError::NumericalInstability(_)));
    }

    #[test]
    fn test_solver_error_singular_jacobian_clone_and_equality() {
        let err1 = SolverError::SingularJacobian { min_pivot: 1e-12 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_solver_error_dimension_mismatch_clone_and_equality() {
        let err1 = SolverError::DimensionMismatch {
            expected: 5,
            got: 3,
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_calibration_error_from_singular_jacobian() {
        let solver_err = SolverError::SingularJacobian { min_pivot: 1e-14 };
        let calib_err: CalibrationError = solver_err.into();
        assert!(calib_err.is_numerical_instability());
        assert!(calib_err.message.as_ref().unwrap().contains("Singular"));
    }

    #[test]
    fn test_calibration_error_from_dimension_mismatch() {
        let solver_err = SolverError::DimensionMismatch {
            expected: 10,
            got: 5,
        };
        let calib_err: CalibrationError = solver_err.into();
        assert!(matches!(
            calib_err.kind,
            CalibrationErrorKind::InvalidParameter(_)
        ));
    }

    // Original SolverError tests

    #[test]
    fn test_solver_error_max_iterations_display() {
        let err = SolverError::MaxIterationsExceeded { iterations: 100 };
        assert_eq!(
            format!("{}", err),
            "Failed to converge after 100 iterations"
        );
    }

    #[test]
    fn test_solver_error_derivative_near_zero_display() {
        let err = SolverError::DerivativeNearZero { x: 1.5 };
        assert_eq!(format!("{}", err), "Derivative near zero at x = 1.5");
    }

    #[test]
    fn test_solver_error_no_bracket_display() {
        let err = SolverError::NoBracket { a: 0.0, b: 1.0 };
        assert_eq!(
            format!("{}", err),
            "No bracket: f(0) and f(1) have same sign"
        );
    }

    #[test]
    fn test_solver_error_numerical_instability_display() {
        let err = SolverError::NumericalInstability("overflow detected".to_string());
        assert_eq!(
            format!("{}", err),
            "Numerical instability: overflow detected"
        );
    }

    #[test]
    fn test_solver_error_trait_implementation() {
        let err = SolverError::MaxIterationsExceeded { iterations: 100 };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_solver_error_clone_and_equality() {
        let err1 = SolverError::NoBracket { a: 0.0, b: 1.0 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_solver_error_external_display() {
        let err = SolverError::External("levenberg-marquardt failed".to_string());
        assert!(format!("{}", err).contains("External"));
        assert!(format!("{}", err).contains("levenberg-marquardt"));
    }

    // CalibrationError tests

    #[test]
    fn test_calibration_error_new() {
        let err = CalibrationError::new(CalibrationErrorKind::NotConverged);
        assert!(matches!(err.kind, CalibrationErrorKind::NotConverged));
        assert_eq!(err.iterations, 0);
        assert!(err.residual_ss.is_nan());
    }

    #[test]
    fn test_calibration_error_not_converged() {
        let err = CalibrationError::not_converged(100, 0.01);
        assert!(err.is_not_converged());
        assert_eq!(err.iterations, 100);
        assert!((err.residual_ss - 0.01).abs() < 1e-15);
        assert!(err.message.is_some());
    }

    #[test]
    fn test_calibration_error_constraint_violation() {
        let err = CalibrationError::constraint_violation("alpha must be positive");
        assert!(err.is_constraint_violation());
        assert!(format!("{}", err).contains("constraint violation"));
    }

    #[test]
    fn test_calibration_error_numerical_instability() {
        let err = CalibrationError::numerical_instability("NaN encountered");
        assert!(err.is_numerical_instability());
        assert!(err.message.as_ref().unwrap().contains("NaN"));
    }

    #[test]
    fn test_calibration_error_insufficient_data() {
        let err = CalibrationError::insufficient_data(3, 10);
        assert!(err.is_insufficient_data());
        if let CalibrationErrorKind::InsufficientData { got, need } = err.kind {
            assert_eq!(got, 3);
            assert_eq!(need, 10);
        } else {
            panic!("Expected InsufficientData");
        }
    }

    #[test]
    fn test_calibration_error_invalid_parameter() {
        let err = CalibrationError::invalid_parameter("volatility cannot be negative");
        assert!(matches!(
            err.kind,
            CalibrationErrorKind::InvalidParameter(_)
        ));
    }

    #[test]
    fn test_calibration_error_with_parameters() {
        let err = CalibrationError::not_converged(10, 0.1).with_parameters(vec![0.5, 1.0]);
        assert!(err.parameter_values.is_some());
        assert_eq!(err.parameter_values.unwrap().len(), 2);
    }

    #[test]
    fn test_calibration_error_with_residual() {
        let err = CalibrationError::new(CalibrationErrorKind::NotConverged).with_residual(0.005);
        assert!((err.residual_ss - 0.005).abs() < 1e-15);
    }

    #[test]
    fn test_calibration_error_with_iterations() {
        let err = CalibrationError::new(CalibrationErrorKind::NotConverged).with_iterations(50);
        assert_eq!(err.iterations, 50);
    }

    #[test]
    fn test_calibration_error_with_message() {
        let err = CalibrationError::new(CalibrationErrorKind::NotConverged)
            .with_message("Custom message");
        assert_eq!(err.message, Some("Custom message".to_string()));
    }

    #[test]
    fn test_calibration_error_rmse() {
        let err = CalibrationError::not_converged(10, 4.0);
        let rmse = err.rmse(4);
        assert!((rmse - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_calibration_error_rmse_nan() {
        let err = CalibrationError::new(CalibrationErrorKind::NumericalInstability);
        assert!(err.rmse(10).is_nan());
    }

    #[test]
    fn test_calibration_error_rmse_zero_observations() {
        let err = CalibrationError::not_converged(10, 1.0);
        assert!(err.rmse(0).is_nan());
    }

    #[test]
    fn test_calibration_error_display() {
        let err = CalibrationError::not_converged(100, 0.01);
        let display = format!("{}", err);
        assert!(display.contains("Calibration error"));
        assert!(display.contains("100 iterations"));
    }

    #[test]
    fn test_calibration_error_from_solver_error() {
        let solver_err = SolverError::MaxIterationsExceeded { iterations: 50 };
        let calib_err: CalibrationError = solver_err.into();
        assert!(calib_err.is_not_converged());
        assert_eq!(calib_err.iterations, 50);
    }

    #[test]
    fn test_calibration_error_from_numerical_instability() {
        let solver_err = SolverError::NumericalInstability("overflow".to_string());
        let calib_err: CalibrationError = solver_err.into();
        assert!(calib_err.is_numerical_instability());
    }

    #[test]
    fn test_calibration_error_from_derivative_near_zero() {
        let solver_err = SolverError::DerivativeNearZero { x: 1.5 };
        let calib_err: CalibrationError = solver_err.into();
        assert!(calib_err.is_numerical_instability());
    }

    #[test]
    fn test_calibration_error_from_no_bracket() {
        let solver_err = SolverError::NoBracket { a: 0.0, b: 1.0 };
        let calib_err: CalibrationError = solver_err.into();
        assert!(calib_err.is_numerical_instability());
    }

    #[test]
    fn test_calibration_error_from_solver_external() {
        let solver_err = SolverError::External("roots crate error".to_string());
        let calib_err: CalibrationError = solver_err.into();
        assert!(calib_err.is_numerical_instability());
        assert!(calib_err.message.as_ref().unwrap().contains("External"));
    }

    #[test]
    fn test_calibration_error_from_optimisation_external() {
        let opt_err = OptimisationError::External("argmin error".to_string());
        let calib_err: CalibrationError = opt_err.into();
        assert!(calib_err.is_numerical_instability());
        assert!(calib_err.message.as_ref().unwrap().contains("External"));
    }

    #[test]
    fn test_calibration_error_kind_display() {
        let kind = CalibrationErrorKind::NotConverged;
        assert_eq!(format!("{}", kind), "calibration did not converge");

        let kind = CalibrationErrorKind::InvalidConstraint("test".to_string());
        assert!(format!("{}", kind).contains("constraint violation"));

        let kind = CalibrationErrorKind::NumericalInstability;
        assert_eq!(format!("{}", kind), "numerical instability");

        let kind = CalibrationErrorKind::InsufficientData { got: 3, need: 10 };
        assert!(format!("{}", kind).contains("insufficient data"));
    }

    #[test]
    fn test_calibration_error_clone_and_equality() {
        let err1 = CalibrationError::not_converged(100, 0.01);
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_calibration_error_trait_implementation() {
        let err = CalibrationError::not_converged(100, 0.01);
        let _: &dyn std::error::Error = &err;
    }

    // Serde tests (feature-gated)
    #[cfg(feature = "serde")]
    mod serde_tests {
        use super::*;

        #[test]
        fn test_interpolation_error_serde_roundtrip() {
            let err = InterpolationError::OutOfBounds {
                x: 5.0,
                min: 0.0,
                max: 3.0,
            };
            let json = serde_json::to_string(&err).unwrap();
            let deserialized: InterpolationError = serde_json::from_str(&json).unwrap();
            assert_eq!(err, deserialized);
        }

        #[test]
        fn test_solver_error_serde_roundtrip() {
            let err = SolverError::MaxIterationsExceeded { iterations: 100 };
            let json = serde_json::to_string(&err).unwrap();
            let deserialized: SolverError = serde_json::from_str(&json).unwrap();
            assert_eq!(err, deserialized);
        }
    }
}
