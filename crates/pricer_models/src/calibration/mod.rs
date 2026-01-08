//! Model calibration module.
//!
//! This module provides calibration infrastructure for financial models:
//! - [`ModelCalibrator`]: Generic calibrator using Levenberg-Marquardt
//! - [`SwaptionCalibrator`]: Swaption volatility surface calibration
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::calibration::{ModelCalibrator, SwaptionCalibrator};
//! use pricer_core::math::solvers::LMConfig;
//!
//! // Create a calibrator with LM solver
//! let calibrator = ModelCalibrator::new(LMConfig::default());
//!
//! // Calibrate to market data
//! let result = calibrator.calibrate(&residual_fn, initial_params);
//! ```

mod model_calibrator;
mod swaption_calibrator;

pub use model_calibrator::{ModelCalibrator, ModelCalibratorConfig};
pub use swaption_calibrator::{
    SwaptionCalibrator, SwaptionMarketData, SwaptionMarketPoint, VolatilityType,
};
