//! Model calibration module.
//!
//! This module provides calibration infrastructure for financial models:
//! - [`ModelCalibrator`]: Generic calibrator using Levenberg-Marquardt
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
//! # Example
//!
//! ```ignore
//! use pricer_models::calibration::{ModelCalibrator, SwaptionCalibrator};
//! use pricer_models::calibration::{CalibrationError, CalibrationResult, CalibrationDiagnostics};
//! use pricer_core::math::solvers::LMConfig;
//!
//! // Create a calibrator with LM solver
//! let calibrator = ModelCalibrator::new(LMConfig::default());
//!
//! // Calibrate to market data
//! let result = calibrator.calibrate(&residual_fn, initial_params);
//! ```

mod error;
pub mod heston;
pub mod hull_white;
mod model_calibrator;
mod result;
pub mod sabr;
mod swaption_calibrator;
mod targets;

pub use error::CalibrationError;
pub use heston::{
    calibrate_heston, HestonCalibrationData, HestonCalibrator, HestonMarketPoint, HestonParamIndex,
};
pub use hull_white::{
    calibrate_hull_white, HWParamIndex, HWSwaptionPoint, HullWhiteCalibrationData,
    HullWhiteCalibrator,
};
pub use model_calibrator::{ModelCalibrator, ModelCalibratorConfig};
pub use result::{CalibrationDiagnostics, CalibrationResult};
pub use sabr::{
    calibrate_sabr, calibrate_sabr_fixed_beta, SABRCalibrationData, SABRCalibrator, SABRParamIndex,
    SABRSmilePoint,
};
pub use swaption_calibrator::{
    SwaptionCalibrator, SwaptionMarketData, SwaptionMarketPoint, VolatilityType,
};
pub use targets::{CalibrationTarget, OptionTarget, SwaptionTarget};
