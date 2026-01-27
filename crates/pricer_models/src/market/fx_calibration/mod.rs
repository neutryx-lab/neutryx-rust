//! FX Volatility Surface Calibration.
//!
//! This module provides configuration types and utilities for calibrating
//! FX volatility surfaces from market quotes.
//!
//! ## Components
//!
//! - [`FxCurve`]: Generic trait for FX forward curve operations
//! - [`CalibratedFxCurve`]: Calibrated FX curve with interpolated forward
//!   points
//! - [`SimpleFxCurve`]: Simple FX curve using interest rate parity
//! - [`FxForwardCurveBuilder`]: Builder for constructing FX forward curves
//! - [`CalibratedFxVolSurface`]: Calibrated FX volatility surface
//! - [`FxVolSurfaceBuilder`]: Builder for constructing FX volatility surfaces
//! - [`FxVolSurfaceConfig`]: Configuration for FX vol surface calibration
//! - [`FxCalibrationError`]: Error types for calibration operations
//! - Newtypes: [`Strike`], [`Vol`], [`ForwardPoints`], [`ExpiryInterpolation`]

mod config;
mod error;
mod fx_market_builder;
mod lazy_surface;
mod sensitivity;
mod surface;
mod types;
mod vol_builder;

// Re-export curve types from curves/fx module
pub use crate::market::curves::{
    CalibratedFxCurve, ExtrapolationPolicy, ForwardPoints, FxCurve, FxCurveError,
    FxForwardCurveBuilder, FxSwapData, SimpleFxCurve, XccySwapData,
};
pub use config::FxVolSurfaceConfig;
pub use error::FxCalibrationError;
pub use fx_market_builder::{FxMarket, FxMarketBuilder, FxMarketDiagnostics, FxMarketError};
pub use lazy_surface::{CacheStats, LazyFxVolSurface};
pub use sensitivity::{
    smooth, ComputationGraph, ComputationGraphEdge, ComputationGraphNode, ExpirySensitivity,
    SensitivityConfig, SensitivityMode, VolSurfaceSensitivity,
};
pub use surface::{
    CalibratedFxVolSurface, CalibratedSmile, SabrParameters, VolSmile, VolSurfaceError,
};
// ForwardPoints is now exported from curves/fx module (re-exported above)
pub use types::{ExpiryInterpolation, Strike, Vol};
pub use vol_builder::{
    CalibrationDiagnostics, CalibrationError, ExpiryDiagnostics, FxVolSurfaceBuilder, VolQuote,
    VolQuoteType,
};
