//! Volatility surface abstractions for option pricing.
//!
//! This module provides the complete volatility surface stack:
//!
//! ## Core Types
//! - [`VolatilitySurface`]: Generic trait for implied volatility lookup
//! - [`FlatVol`]: Constant volatility surface implementation
//! - [`InterpolatedVolSurface`]: Grid-based interpolated volatility surface
//! - [`VolSurfaceEnum`]: Static dispatch enum for volatility surfaces
//! - [`VolCubeSlice`]: Adapter for using 3D VolCube as 2D surface
//!
//! ## FX Volatility Surfaces
//! - [`FxVolatilitySurface`]: Delta-expiry based volatility surface
//! - [`CalibratedFxVolSurface`]: SABR-calibrated FX volatility surface
//! - [`FxDeltaPoint`]: Standard delta points used in FX markets
//! - [`Strike`]: Newtype for FX option strikes
//! - [`Vol`]: Newtype for implied volatility
//!
//! ## FX Configuration & Calibration
//! - [`FxVolSurfaceConfig`]: Configuration for FX vol surface calibration
//! - [`ExpiryInterpolation`]: Expiry dimension interpolation methods
//! - [`FxVolSurfaceBuilder`]: Builder for constructing calibrated surfaces
//! - [`CalibrationDiagnostics`]: Calibration result diagnostics
//! - [`LazyFxVolSurface`]: Lazy wrapper with deferred calibration
//!
//! ## Probability Density
//! - [`FxDensityCalculator`]: Risk-neutral density calculation
//! - [`DeltaType`]: Delta convention types
//! - [`DensityStatistics`]: Distribution statistics

mod flat;
mod fx;
mod interpolated;
mod traits;
mod vol_surface_enum;
mod volcube_slice;

pub use flat::FlatVol;
pub use fx::{
    // Core types
    ExpiryInterpolation, FxDeltaPoint, FxVolSurfaceConfig, FxVolatilitySurface, Strike, Vol,
    // Calibrated surfaces
    CalibratedFxVolSurface, CalibratedSmile, SabrParameters, VolSmile, VolSurfaceError,
    // Builder and diagnostics
    CalibrationDiagnostics, CalibrationError, ExpiryDiagnostics, FxVolSurfaceBuilder,
    VolQuote, VolQuoteType,
    // Lazy evaluation
    CacheStats, LazyFxVolSurface,
    // Density calculator
    DeltaType, DensityStatistics, FxDensityCalculator,
};
pub use interpolated::InterpolatedVolSurface;
pub use traits::VolatilitySurface;
pub use vol_surface_enum::VolSurfaceEnum;
pub use volcube_slice::VolCubeSlice;
