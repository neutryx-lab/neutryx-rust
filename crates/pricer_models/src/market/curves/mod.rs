//! Curve abstractions for interest rate, credit risk, and FX calculations.
//!
//! This module provides:
//! - [`YieldCurve`]: Generic trait for discount factor and rate calculations
//! - [`FlatCurve`]: Constant rate yield curve implementation
//! - [`InterpolatedCurve`]: Pillar-based interpolated yield curve
//! - [`CurveInterpolation`]: Interpolation method selection
//! - [`CurveName`]: Standard curve name enumeration for multi-curve framework
//! - [`CurveEnum`]: Static dispatch enum wrapping concrete curve
//!   implementations
//! - [`CurveSet`]: Container for managing multiple named yield curves
//! - [`CreditCurve`]: Generic trait for hazard rate and survival probability
//!   calculations
//! - [`HazardRateCurve`]: Interpolated hazard rate curve implementation
//! - [`FlatHazardRateCurve`]: Constant hazard rate curve implementation
//! - [`FxCurve`]: Generic trait for FX forward curve operations
//! - [`SimpleFxCurve`]: Simple FX curve using interest rate parity
//! - [`CalibratedFxCurve`]: Calibrated FX curve with interpolated forward
//!   points
//! - [`FxForwardCurveBuilder`]: Builder for constructing calibrated FX curves

mod credit;
mod curve_enum;
mod curve_set;
mod flat;
mod fx;
mod interpolated;
mod traits;

pub use credit::{CreditCurve, FlatHazardRateCurve, HazardRateCurve};
pub use curve_enum::{CurveEnum, CurveName};
pub use curve_set::CurveSet;
pub use flat::FlatCurve;
pub use fx::{
    CalibratedFxCurve, ExtrapolationPolicy, ForwardPoints, FxCurve, FxCurveError,
    FxForwardCurveBuilder, FxSwapData, SimpleFxCurve, XccySwapData,
};
pub use interpolated::{CurveInterpolation, InterpolatedCurve};
pub use traits::YieldCurve;
