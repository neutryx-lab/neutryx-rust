//! Market data structures and calibration for quantitative finance.
//!
//! This module provides:
//! - [`curves`]: Yield curve trait and implementations
//! - [`surfaces`]: Volatility surface trait and implementations
//! - [`calibration`]: Model and curve calibration (bootstrapping, SABR, Heston,
//!   Hull-White)
//! - [`context`]: Market data management (provider, indexed market, validation)
//!
//! # Architecture
//!
//! All structures are generic over `T: Float` to support both standard
//! floating-point types (f64, f32) and automatic differentiation types
//! (Dual64). This design ensures compatibility with Enzyme AD at LLVM level.
//!
//! # Module Structure
//!
//! ```text
//! market/
//! ├── context/       # Market data management
//! │   ├── provider   # Lazy market data resolution with Arc caching
//! │   ├── indexed    # Index-keyed market container
//! │   ├── requirements # Trade index requirements trait
//! │   └── validator  # Market completeness validation
//! ├── curves/        # Yield curve implementations
//! ├── surfaces/      # Volatility surface implementations
//! ├── calibration/   # Model calibration
//! ├── fx_calibration/ # FX-specific calibration
//! └── volcube/       # Swaption volatility cube
//! ```
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::market::curves::{YieldCurve, FlatCurve};
//! use pricer_models::market::surfaces::{VolatilitySurface, FlatVol};
//! use pricer_models::market::context::{IndexedMarket, IndexedMarketBuilder};
//!
//! // Create a flat yield curve with 5% rate
//! let curve = FlatCurve::new(0.05_f64);
//! let df = curve.discount_factor(1.0).unwrap();
//!
//! // Create a flat volatility surface with 20% vol
//! let vol_surface = FlatVol::new(0.20_f64);
//! let sigma = vol_surface.volatility(100.0, 1.0).unwrap();
//!
//! // Build an indexed market
//! let market = IndexedMarketBuilder::new()
//!     .valuation_date(Date::from_ymd(2025, 1, 15).unwrap())
//!     .with_curve(RateIndex::Sofr, curve)
//!     .build()?;
//! ```

pub mod calibration;
pub mod context;
pub mod curves;
pub mod error;
pub mod fx_calibration;
pub mod surfaces;
pub mod volcube;

// ============================================================================
// Re-export commonly used types
// ============================================================================

// Context module types
pub use context::{
    DefaultIndexCurveMapper, IndexCurveMapper, IndexedMarket, IndexedMarketBuilder,
    MarketProvider, MarketValidator, TradeIndexRequirements, ValidationReport, VolCubeProviderKey,
};

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

// Re-export curve types
pub use curves::{
    CreditCurve, CurveEnum, CurveInterpolation, CurveName, CurveSet, FlatCurve,
    FlatHazardRateCurve, HazardRateCurve, InterpolatedCurve, YieldCurve,
};

// Re-export error types (unified hierarchy + legacy)
pub use error::{ContextError, CurveError, MarketBuildError, MarketDataError, MarketError, SurfaceError};

// Re-export FX density types
pub use surfaces::{DeltaType, DensityStatistics, FxDensityCalculator};

// Re-export surface types
pub use surfaces::{
    FlatVol, FxDeltaPoint, FxVolatilitySurface, InterpolatedVolSurface, VolCubeSlice,
    VolSurfaceEnum, VolatilitySurface,
};

// Re-export volcube types
pub use volcube::{
    calculate_forward_swap_rate, CacheStats,
    CalibrationDiagnostics as VolCubeCalibrationDiagnostics, CalibrationProgress,
    ClosureForwardRateProvider, EngineOutput, ExtrapolationMethod, ForwardRateError,
    ForwardRateProvider, InstrumentId, InterpolationMethod, OptimizerType, ProgressCallback,
    SabrParams, SharedVolCubeCache, StrikeAxisType, VolCube, VolCubeCache,
    VolCubeCalibrationEngine, VolCubeConfig, VolCubeError, VolCubeKey, VolInstrument,
    VolatilityCube,
};
