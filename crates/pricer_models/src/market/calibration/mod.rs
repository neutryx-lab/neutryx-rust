//! Model calibration module.
//!
//! This module provides calibration infrastructure for financial models:
//! - [`CalibrationEngine`]: Core calibration engine using Levenberg-Marquardt
//! - [`CalibrationScope`]: Calibration scope (Global, TermByTerm, Piecewise)
//! - [`HestonCalibrator`]: Heston stochastic volatility model calibration
//! - [`SABRCalibrator`]: SABR stochastic volatility model calibration
//! - [`HullWhiteCalibrator`]: Hull-White short rate model calibration
//! - [`SwaptionCalibrator`]: Swaption volatility surface calibration
//! - [`CalibrationError`]: Comprehensive error types for calibration
//! - [`CalibrationResult`]: Generic calibration result with diagnostics
//! - [`CalibrationTarget`]: Target types for calibration (options, swaptions)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Calibration Flow                         │
//! │                                                             │
//! │  Market Data → CalibrationTarget → Calibrator → Result      │
//! │      │             │                   │           │        │
//! │      ▼             ▼                   ▼           ▼        │
//! │  OptionPrices  ModelParams        Optimizer   ModelParams   │
//! │  SwaptionVols  Objective          L-M/AD     + Diagnostics  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Calibration Scope
//!
//! The engine supports different calibration scopes:
//! - `Global`: Calibrate all parameters simultaneously (default)
//! - `TermByTerm`: Calibrate parameters grouped by term/maturity
//! - `Piecewise`: Calibrate parameters in segments
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::calibration::{CalibrationEngine, CalibrationEngineConfig, CalibrationScope};
//! use pricer_core::math::solvers::LMConfig;
//!
//! // Create a calibration engine
//! let config = CalibrationEngineConfig::new(LMConfig::default())
//!     .with_scope(CalibrationScope::Global);
//! let engine = CalibrationEngine::new(config);
//!
//! // Calibrate to market data
//! let result = engine.calibrate_with_residuals(residual_fn, initial_params);
//! ```

pub mod bootstrapping;
mod engine;
mod error;
pub mod heston;
pub mod hull_white;
// Legacy module kept for internal use - use engine.rs for new code
#[allow(dead_code)]
mod model_calibrator;
mod result;
pub mod sabr;
mod swaption_calibrator;
mod targets;

// Primary exports (new API)
// Re-export bootstrapping types
pub use bootstrapping::{
    AdjointSolver, AdjointSolverConfig, BootstrapCache, BootstrapConfig, BootstrapError,
    BootstrapInterpolation, BootstrapResult, BootstrappedCurve, BootstrappedCurveBuilder,
    BufferPool, CachedBootstrapper, CurveBootstrapper, CurveCache, DateCalculator,
    DateCalculatorBuilder, GenericBootstrapConfig, GenericBootstrapConfigBuilder,
    GenericBootstrapResult, InterpolationIndices, InterpolationMethod, MultiCurveBuilder,
    ParallelCurveSetBuilder, SensitivityBootstrapper, SequentialBootstrapper, SolveResult,
    SolveResultWithSensitivities, SolverType,
};
pub use engine::{
    CalibrationEngine,
    CalibrationEngineConfig,
    CalibrationScope,
    GenericCalibrator,
    // Backward compatibility aliases
    ModelCalibrator,
    ModelCalibratorConfig,
};
pub use error::CalibrationError;
pub use heston::{
    calibrate_heston, HestonCalibrationData, HestonCalibrator, HestonMarketPoint, HestonParamIndex,
};
pub use hull_white::{
    calibrate_hull_white, HWParamIndex, HWSwaptionPoint, HullWhiteCalibrationData,
    HullWhiteCalibrator,
};
pub use result::{CalibrationDiagnostics, CalibrationResult};
pub use sabr::{
    calibrate_sabr, calibrate_sabr_fixed_beta, SABRCalibrationData, SABRCalibrator, SABRParamIndex,
    SABRSmilePoint,
};
pub use swaption_calibrator::{
    SwaptionCalibrator, SwaptionMarketData, SwaptionMarketPoint, VolatilityType,
};
pub use targets::{CalibrationTarget, OptionTarget, SwaptionTarget};
