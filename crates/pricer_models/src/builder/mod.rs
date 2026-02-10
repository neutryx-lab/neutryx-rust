/! Builder module for yield curves, volatility surfaces, and market data calibration.

use pricer_core::types::SolverError;
use thiserror::Error;

use crate::market::MarketDataError;

// =============================================================================
// Shared Infrastructure Modules
// =============================================================================

pub mod compile;
mod error;
mod grid;
mod instrument;

#[cfg(feature = "global-bootstrap")]
mod engine;
#[cfg(feature = "global-bootstrap")]
pub mod enzyme_jacobian;
#[cfg(feature = "global-bootstrap")]
mod jump;
#[cfg(feature = "global-bootstrap")]
mod matrix;
#[cfg(feature = "global-bootstrap")]
mod problem;

// =============================================================================
// Calibration Submodules
// =============================================================================

/// Curve calibration (sequential and global bootstrapping).
pub mod curve;

/// Curve construction from definitions and market data.
pub mod construction;

/// Volatility surface and cube calibration.
pub mod vol;

// =============================================================================
// Public Re-exports: Shared Infrastructure
// =============================================================================

// =============================================================================
// Public Re-exports: Curve Calibration
// =============================================================================
// =============================================================================
// Public Re-exports: Instrument Compilation (Requirement 1, 8)
// =============================================================================
pub use compile::{CompileError, CompiledInstrument, InstrumentCompiler, InstrumentType};
// =============================================================================
// Public Re-exports: Curve Construction
// =============================================================================
pub use construction::{
    ConstructionConfig, ConstructionError, ConstructionResult, CurveConstructionEngine,
};
pub use curve::{BootstrapConfig, CurveBootstrapper, JacobianMatrix};
#[cfg(feature = "global-bootstrap")]
pub use curve::{GlobalBootstrapConfig, GlobalBootstrapResult, GlobalBootstrapper};
#[cfg(feature = "global-bootstrap")]
pub use engine::{
    CalibrationEngine, CalibrationEngineConfig, CalibrationResult, GlobalCalibrationEngine,
    SequentialCalibrationEngine,
};
#[cfg(feature = "global-bootstrap")]
pub use enzyme_jacobian::JacobianResult;
#[cfg(feature = "global-bootstrap")]
pub use error::{
    apply_tikhonov_regularisation, estimate_condition_number, should_apply_regularisation,
    validate_jacobian_dmatrix,
};
pub use error::{
    validate_jacobian_matrix, CalibrationError, IftError, JacobianQuality, NumericalDiagnostics,
    RegularisationType,
};
pub use grid::CalibrationGrid;
pub use instrument::CalibrationInstrument;
#[cfg(feature = "global-bootstrap")]
pub use jump::{JumpConfig, JumpPillar};
#[cfg(feature = "global-bootstrap")]
pub use matrix::{CalibrationMatrix, CalibrationMatrixBuilder, InterpolationMatrix};
#[cfg(feature = "global-bootstrap")]
pub use problem::{CalibrationProblem, CalibrationProblemConfig, JacobianMethod};
// =============================================================================
// Public Re-exports: Vol Calibration
// =============================================================================
pub use vol::{
    DeltaVol, DeltaVolSlice, FxVolBuilder, FxVolResult, OrderedFloat, SabrBounds, SabrParams,
    SabrSliceCalibrator, SliceCalibrationConfig, SliceCalibrationDiagnostics,
    SliceCalibrationResult, SliceCalibrator, VolCubeBuilder, VolCubeResult, VolQuote,
};

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
