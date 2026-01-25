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
//! - [`CalibratedFxVolSurface`]: Calibrated FX volatility surface
//! - [`FxVolSurfaceBuilder`]: Builder for constructing FX volatility surfaces
//! - [`FxVolSurfaceConfig`]: Configuration for FX vol surface calibration
//! - [`FxCalibrationError`]: Error types for calibration operations
//! - Newtypes: [`Strike`], [`Vol`], [`ForwardPoints`], [`ExpiryInterpolation`]

mod builder;
mod config;
mod curve;
mod error;
mod fx_market_builder;
mod lazy_surface;
mod sensitivity;
mod surface;
mod types;
mod vol_builder;

pub use builder::{FxForwardCurveBuilder, FxSwapData, XccySwapData};
pub use config::FxVolSurfaceConfig;
pub use curve::{CalibratedFxCurve, ExtrapolationPolicy, FxCurve, FxCurveError, SimpleFxCurve};
pub use error::FxCalibrationError;
pub use surface::{
    CalibratedFxVolSurface, CalibratedSmile, SabrParameters, VolSmile, VolSurfaceError,
};
pub use types::{ExpiryInterpolation, ForwardPoints, Strike, Vol};
pub use vol_builder::{
    CalibrationDiagnostics, CalibrationError, ExpiryDiagnostics, FxVolSurfaceBuilder, VolQuote,
    VolQuoteType,
};
pub use lazy_surface::{CacheStats, LazyFxVolSurface};
pub use sensitivity::{
    smooth, ComputationGraph, ComputationGraphEdge, ComputationGraphNode, ExpirySensitivity,
    SensitivityConfig, SensitivityMode, VolSurfaceSensitivity,
};
pub use fx_market_builder::{FxMarket, FxMarketBuilder, FxMarketDiagnostics, FxMarketError};
