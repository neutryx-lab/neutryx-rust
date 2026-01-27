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
//!
//! Note: Core volatility surface types have been consolidated into `surfaces/fx.rs`.
//! This module re-exports them for backward compatibility.

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

// Re-export consolidated types from surfaces/fx module (canonical location)
pub use crate::market::surfaces::{
    // Core newtypes
    ExpiryInterpolation, FxVolSurfaceConfig, Strike, Vol,
    // Calibrated surface types
    CalibratedFxVolSurface, CalibratedSmile, SabrParameters, VolSmile, VolSurfaceError,
    // Builder and diagnostics
    CalibrationDiagnostics, CalibrationError, ExpiryDiagnostics, FxVolSurfaceBuilder,
    VolQuote, VolQuoteType,
    // Lazy evaluation
    CacheStats, LazyFxVolSurface,
};

// Local module exports (not consolidated)
pub use error::FxCalibrationError;
pub use fx_market_builder::{FxMarket, FxMarketBuilder, FxMarketDiagnostics, FxMarketError};
pub use sensitivity::{
    smooth, ComputationGraph, ComputationGraphEdge, ComputationGraphNode, ExpirySensitivity,
    SensitivityConfig, SensitivityMode, VolSurfaceSensitivity,
};
