//! Volatility surface abstractions for option pricing.
//!
//! This module provides:
//! - [`VolatilitySurface`]: Generic trait for implied volatility lookup
//! - [`FlatVol`]: Constant volatility surface implementation
//! - [`InterpolatedVolSurface`]: Grid-based interpolated volatility surface
//! - [`FxVolatilitySurface`]: Delta-expiry based volatility surface for FX
//!   options
//! - [`FxDeltaPoint`]: Standard delta points used in FX markets
//! - [`VolCubeSlice`]: Adapter for using 3D VolCube as 2D surface

mod flat;
mod fx;
mod interpolated;
mod traits;
mod vol_surface_enum;
mod volcube_slice;

pub use flat::FlatVol;
pub use fx::{FxDeltaPoint, FxVolatilitySurface};
pub use interpolated::InterpolatedVolSurface;
pub use traits::VolatilitySurface;
pub use vol_surface_enum::VolSurfaceEnum;
pub use volcube_slice::VolCubeSlice;
