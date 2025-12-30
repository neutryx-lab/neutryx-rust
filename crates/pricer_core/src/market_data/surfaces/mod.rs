//! Volatility surface abstractions for option pricing.
//!
//! This module provides:
//! - [`VolatilitySurface`]: Generic trait for implied volatility lookup
//! - [`FlatVol`]: Constant volatility surface implementation
//! - [`InterpolatedVolSurface`]: Grid-based interpolated volatility surface

mod flat;
mod interpolated;
mod traits;

pub use flat::FlatVol;
pub use interpolated::InterpolatedVolSurface;
pub use traits::VolatilitySurface;
