//! Builder module for yield curves, volatility surfaces, and market data
//! calibration.

use pricer_core::types::SolverError;
use thiserror::Error;

use crate::market::MarketDataError;

pub mod compile;
mod error;
mod grid;
mod instrument;

mod engine;
pub mod enzyme_jacobian;
mod jump;
mod matrix;
mod problem;

/// Curve calibration (sequential and global bootstrapping).
pub mod curve;

/// Curve construction from definitions and market data.
pub mod construction;

/// Volatility surface and cube calibration.
pub mod vol;

pub use compile::{CompileError, CompiledInstrument, InstrumentCompiler, InstrumentType};
pub use construction::{
    ConstructionConfig, ConstructionError, ConstructionResult, CurveConstructionEngine,
};
pub use curve::{
    BootstrapConfig, CurveBootstrapper, GlobalBootstrapConfig, GlobalBootstrapResult,
    GlobalBootstrapper, JacobianMatrix,
};
pub use engine::{
    CalibrationEngine, CalibrationEngineConfig, CalibrationResult, GlobalCalibrationEngine,
    SequentialCalibrationEngine,
};
pub use enzyme_jacobian::JacobianResult;
pub use error::{
    apply_tikhonov_regularisation, estimate_condition_number, validate_jacobian_dmatrix,
    validate_jacobian_matrix, CalibrationError, IftError, JacobianQuality, NumericalDiagnostics,
    RegularisationType,
};
pub use grid::CalibrationGrid;
pub use instrument::CalibrationInstrument;
pub use jump::{JumpConfig, JumpPillar};
pub use matrix::{CalibrationMatrix, InterpolationMatrix};
pub use problem::{CalibrationProblem, CalibrationProblemConfig, JacobianMethod};
pub use vol::{
    DeltaVol, DeltaVolSlice, FxVolBuilder, FxVolResult, OrderedFloat, SabrBounds, SabrParams,
    SabrSliceCalibrator, SliceCalibrationConfig, SliceCalibrationDiagnostics,
    SliceCalibrationResult, SliceCalibrator, VolBuilder, VolCubeBuilder, VolCubeResult, VolQuote,
};

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
