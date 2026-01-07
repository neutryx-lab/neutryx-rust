//! Curve abstractions for interest rate and credit risk calculations.
//!
//! This module provides:
//! - [`YieldCurve`]: Generic trait for discount factor and rate calculations
//! - [`FlatCurve`]: Constant rate yield curve implementation
//! - [`InterpolatedCurve`]: Pillar-based interpolated yield curve
//! - [`CurveInterpolation`]: Interpolation method selection
//! - [`CurveName`]: Standard curve name enumeration for multi-curve framework
//! - [`CurveEnum`]: Static dispatch enum wrapping concrete curve implementations
//! - [`CurveSet`]: Container for managing multiple named yield curves
//! - [`CreditCurve`]: Generic trait for hazard rate and survival probability calculations
//! - [`HazardRateCurve`]: Interpolated hazard rate curve implementation
//! - [`FlatHazardRateCurve`]: Constant hazard rate curve implementation

mod credit;
mod curve_enum;
mod curve_set;
mod flat;
mod interpolated;
mod traits;

pub use credit::{CreditCurve, FlatHazardRateCurve, HazardRateCurve};
pub use curve_enum::{CurveEnum, CurveName};
pub use curve_set::CurveSet;
pub use flat::FlatCurve;
pub use interpolated::{CurveInterpolation, InterpolatedCurve};
pub use traits::YieldCurve;
