//! Yield curve abstractions for interest rate calculations.
//!
//! This module provides:
//! - [`YieldCurve`]: Generic trait for discount factor and rate calculations
//! - [`FlatCurve`]: Constant rate yield curve implementation
//! - [`InterpolatedCurve`]: Pillar-based interpolated yield curve
//! - [`CurveInterpolation`]: Interpolation method selection

mod flat;
mod interpolated;
mod traits;

pub use flat::FlatCurve;
pub use interpolated::{CurveInterpolation, InterpolatedCurve};
pub use traits::YieldCurve;
