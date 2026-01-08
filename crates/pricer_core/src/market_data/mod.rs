//! Market data structures for quantitative finance pricing.
//!
//! This module provides yield curve and volatility surface abstractions
//! for discount factor, forward rate, and implied volatility calculations.
//!
//! # Architecture
//!
//! All structures are generic over `T: Float` to support both standard
//! floating-point types (f64, f32) and automatic differentiation types (Dual64).
//! This design ensures compatibility with Enzyme AD at LLVM level.
//!
//! # Components
//!
//! - [`curves`]: Yield curve trait and implementations (FlatCurve, InterpolatedCurve)
//! - [`surfaces`]: Volatility surface trait and implementations (FlatVol, InterpolatedVolSurface)
//! - [`error`]: Market data error types (MarketDataError)
//!
//! # Example
//!
//! ```
//! use pricer_core::market_data::curves::{YieldCurve, FlatCurve};
//! use pricer_core::market_data::surfaces::{VolatilitySurface, FlatVol};
//!
//! // Create a flat yield curve with 5% rate
//! let curve = FlatCurve::new(0.05_f64);
//! let df = curve.discount_factor(1.0).unwrap();
//! assert!((df - 0.951229).abs() < 1e-5);
//!
//! // Create a flat volatility surface with 20% vol
//! let vol_surface = FlatVol::new(0.20_f64);
//! let sigma = vol_surface.volatility(100.0, 1.0).unwrap();
//! assert_eq!(sigma, 0.20);
//! ```

pub mod curves;
pub mod error;
pub mod surfaces;

// Re-export commonly used types
pub use curves::{
    CreditCurve, CurveEnum, CurveInterpolation, CurveName, CurveSet, FlatCurve,
    FlatHazardRateCurve, HazardRateCurve, InterpolatedCurve, YieldCurve,
};
pub use error::MarketDataError;
pub use surfaces::{
    FlatVol, FxDeltaPoint, FxVolatilitySurface, InterpolatedVolSurface, VolatilitySurface,
};
