//! Market data structures and calibration for quantitative finance.
//!
//! This module provides:
//! - [`curves`]: Yield curve trait and implementations
//! - [`surfaces`]: Volatility surface trait and implementations
//! - [`calibration`]: Model and curve calibration (bootstrapping, SABR, Heston,
//!   Hull-White)
//! - [`provider`]: Lazy market data resolution
//!
//! # Architecture
//!
//! All structures are generic over `T: Float` to support both standard
//! floating-point types (f64, f32) and automatic differentiation types
//! (Dual64). This design ensures compatibility with Enzyme AD at LLVM level.
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::market::curves::{YieldCurve, FlatCurve};
//! use pricer_models::market::surfaces::{VolatilitySurface, FlatVol};
//!
//! // Create a flat yield curve with 5% rate
//! let curve = FlatCurve::new(0.05_f64);
//! let df = curve.discount_factor(1.0).unwrap();
//!
//! // Create a flat volatility surface with 20% vol
//! let vol_surface = FlatVol::new(0.20_f64);
//! let sigma = vol_surface.volatility(100.0, 1.0).unwrap();
//! ```

pub mod calibration;
pub mod curves;
pub mod error;
pub mod fx_density;
pub mod provider;
pub mod surfaces;
pub mod volcube;

// Re-export commonly used types
// Re-export calibration types
pub use calibration::{
    calibrate_heston, calibrate_hull_white, calibrate_sabr, calibrate_sabr_fixed_beta,
    CalibrationDiagnostics, CalibrationEngine, CalibrationEngineConfig, CalibrationError,
    CalibrationResult, CalibrationScope, CalibrationTarget, GenericCalibrator,
    HestonCalibrationData, HestonCalibrator, HestonMarketPoint, HullWhiteCalibrationData,
    HullWhiteCalibrator, ModelCalibrator, ModelCalibratorConfig, OptionTarget, SABRCalibrationData,
    SABRCalibrator, SABRSmilePoint, SwaptionCalibrator, SwaptionMarketData, SwaptionMarketPoint,
    SwaptionTarget, VolatilityType,
};
pub use curves::{
    CreditCurve, CurveEnum, CurveInterpolation, CurveName, CurveSet, FlatCurve,
    FlatHazardRateCurve, HazardRateCurve, InterpolatedCurve, YieldCurve,
};
pub use error::MarketDataError;
pub use provider::MarketProvider;
pub use fx_density::{DeltaType, FxDensityCalculator};
pub use surfaces::{
    FlatVol, FxDeltaPoint, FxVolatilitySurface, InterpolatedVolSurface, VolSurfaceEnum,
    VolatilitySurface,
};
pub use volcube::{
    CacheStats, CalibrationDiagnostics as VolCubeCalibrationDiagnostics, ExtrapolationMethod,
    InstrumentId, InterpolationMethod, OptimizerType, SabrParams, SharedVolCubeCache,
    StrikeAxisType, VolCubeCache, VolCubeConfig, VolCubeError, VolCubeKey, VolInstrument,
};
