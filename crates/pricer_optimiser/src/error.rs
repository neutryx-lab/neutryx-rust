//! Optimiser errors.

use thiserror::Error;

/// Errors that can occur during optimisation.
#[derive(Error, Debug)]
pub enum OptimiserError {
    /// Convergence failure
    #[error("Failed to converge after {iterations} iterations (residual: {residual})")]
    ConvergenceFailure {
        /// Number of iterations performed
        iterations: usize,
        /// Final residual value
        residual: f64,
    },

    /// Singular matrix encountered
    #[error("Singular matrix encountered during optimisation")]
    SingularMatrix,

    /// Invalid parameter bounds
    #[error("Invalid parameter bounds: {0}")]
    InvalidBounds(String),

    /// Numerical instability
    #[error("Numerical instability: {0}")]
    NumericalInstability(String),

    /// Invalid market data
    #[error("Invalid market data: {0}")]
    InvalidMarketData(String),

    /// Insufficient data points
    #[error("Insufficient data points: need {required}, got {provided}")]
    InsufficientData {
        /// Minimum required data points
        required: usize,
        /// Actual data points provided
        provided: usize,
    },
}
