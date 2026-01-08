//! Volatility surface abstractions for option pricing.
//!
//! This module provides:
//! - [`VolatilitySurface`]: Generic trait for implied volatility lookup
//! - [`FlatVol`]: Constant volatility surface implementation
//! - [`InterpolatedVolSurface`]: Grid-based interpolated volatility surface
//! - [`FxVolatilitySurface`]: Delta-expiry based volatility surface for FX options
//! - [`FxDeltaPoint`]: Standard delta points used in FX markets

mod flat;
mod fx;
mod interpolated;
mod traits;

pub use flat::FlatVol;
pub use fx::{FxDeltaPoint, FxVolatilitySurface};
pub use interpolated::InterpolatedVolSurface;
pub use traits::VolatilitySurface;
