//! FX Volatility Surface Calibration.
//!
//! This module provides configuration types and utilities for calibrating
//! FX volatility surfaces from market quotes.

mod config;
mod error;
mod types;

pub use config::FxVolSurfaceConfig;
pub use error::FxCalibrationError;
pub use types::{ExpiryInterpolation, ForwardPoints, Strike, Vol};
