//! FX Volatility Surface Calibration.
//!
//! This module provides configuration types and utilities for calibrating
//! FX volatility surfaces from market quotes.
//!
//! ## Components
//!
//! - [`FxCurve`]: Generic trait for FX forward curve operations
//! - [`CalibratedFxCurve`]: Calibrated FX curve with interpolated forward points
//! - [`SimpleFxCurve`]: Simple FX curve using interest rate parity
//! - [`FxForwardCurveBuilder`]: Builder for constructing FX forward curves
//! - [`FxVolSurfaceConfig`]: Configuration for FX vol surface calibration
//! - [`FxCalibrationError`]: Error types for calibration operations
//! - Newtypes: [`Strike`], [`Vol`], [`ForwardPoints`], [`ExpiryInterpolation`]

mod builder;
mod config;
mod curve;
mod error;
mod types;

pub use builder::{FxForwardCurveBuilder, FxSwapData, XccySwapData};
pub use config::FxVolSurfaceConfig;
pub use curve::{CalibratedFxCurve, ExtrapolationPolicy, FxCurve, FxCurveError, SimpleFxCurve};
pub use error::FxCalibrationError;
pub use types::{ExpiryInterpolation, ForwardPoints, Strike, Vol};
