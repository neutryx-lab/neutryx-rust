//! Curve builder module for yield curve bootstrapping and market data calibration.

use pricer_core::types::SolverError;
use thiserror::Error;

use crate::market::MarketDataError;

mod bootstrap;
mod error;
mod instrument;
#[cfg(feature = "global-bootstrap")]
mod globalsolver;

pub use bootstrap::{BootstrapConfig, CurveBootstrapper, InterpolationMethod};
pub use error::CalibrationError;
pub use instrument::CalibrationInstrument;
#[cfg(feature = "global-bootstrap")]
pub use globalsolver::{GlobalBootstrapConfig, GlobalBootstrapResult, GlobalBootstrapper};

// =============================================================================
// Bootstrap Error and Result Types
// =============================================================================

/// Errors that can occur during bootstrapping.
#[derive(Debug, Clone, Error)]
pub enum BootstrapError {
    /// Convergence failure during bootstrapping.
    #[error("Convergence failure at maturity {maturity}: residual {residual} after {iterations} iterations")]
    ConvergenceFailure {
        /// Maturity where convergence failed.
        maturity: f64,
        /// Final residual.
        residual: f64,
        /// Number of iterations performed.
        iterations: usize,
    },

    /// Insufficient data for bootstrapping.
    #[error("Insufficient data: required {required}, provided {provided}")]
    InsufficientData {
        /// Required number of data points.
        required: usize,
        /// Provided number of data points.
        provided: usize,
    },

    /// Negative rate encountered.
    #[error("Negative rate {rate} at maturity {maturity}")]
    NegativeRate {
        /// Maturity where negative rate was found.
        maturity: f64,
        /// Negative rate value.
        rate: f64,
    },

    /// Arbitrage detected in curve.
    #[error("Arbitrage detected at maturity {maturity}")]
    ArbitrageDetected {
        /// Maturity where arbitrage was detected.
        maturity: f64,
    },

    /// Duplicate maturity in instruments.
    #[error("Duplicate maturity: {maturity}")]
    DuplicateMaturity {
        /// Duplicate maturity value.
        maturity: f64,
    },

    /// Solver error during bootstrapping.
    #[error("Solver error: {0}")]
    Solver(#[from] SolverError),

    /// Market data error.
    #[error("Market data error: {0}")]
    MarketData(#[from] MarketDataError),

    /// Invalid input.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Invalid maturity.
    #[error("Invalid maturity {maturity} (max: {max_maturity})")]
    InvalidMaturity {
        /// Invalid maturity value.
        maturity: f64,
        /// Maximum allowed maturity.
        max_maturity: f64,
    },
}

impl BootstrapError {
    /// Creates a convergence failure error.
    pub fn convergence_failure(maturity: f64, residual: f64, iterations: usize) -> Self {
        Self::ConvergenceFailure {
            maturity,
            residual,
            iterations,
        }
    }

    /// Creates an insufficient data error.
    pub fn insufficient_data(required: usize, provided: usize) -> Self {
        Self::InsufficientData { required, provided }
    }
}

/// Result of a successful bootstrap operation.
#[derive(Debug, Clone)]
pub struct BootstrapResult {
    /// Bootstrapped discount factors.
    pub discount_factors: Vec<f64>,
    /// Pillar maturities.
    pub pillars: Vec<f64>,
    /// Final residual (sum of squared errors).
    pub residual: f64,
}
