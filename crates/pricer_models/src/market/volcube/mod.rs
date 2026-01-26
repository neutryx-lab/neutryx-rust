//! VolatilityCube (3次元ボラティリティ構造) カリブレーションエンジン。
//!
//! # 概要
//!
//! このモジュールは、3次元ボラティリティ構造（Expiry, Tenor, Strike）の
//! カリブレーションと補間を提供する。CurveBuilderパターンに倣い、
//! Instrumentリストとカリブレーション設定を入力として、VolCubeを構築する。
//!
//! # アーキテクチャ
//!
//! - `VolCube<T>`: 3D vol cube本体（補間と密度計算）
//! - `VolCubeBuilder<T>`: VolCube構築とカリブレーション
//! - `VolCubeCache<T>`: LRUキャッシュによる再カリブレーション回避
//! - `VolCubeConfig`: カリブレーション設定
//! - `BreedenLitzenberger`: 確率密度関数計算
//!
//! # 使用例
//!
//! ```ignore
//! use pricer_models::market::volcube::{VolCubeBuilder, VolCubeConfig};
//!
//! let config = VolCubeConfig::default();
//! let cube = VolCubeBuilder::new()
//!     .with_instruments(instruments)
//!     .with_config(config)
//!     .build()?;
//!
//! let vol = cube.volatility(1.0, 5.0, 0.03)?;
//! ```

mod aad_validation;
mod breeden_litzenberger;
mod builder;
mod cache;
mod calibration_graph;
mod calibrator;
mod config;
mod cube;
mod engine;
mod error;
pub mod graph;
mod interpolator;
mod lazy_evaluator;
mod loader_convert;
mod quote;
mod sabr_surface;
mod sensitivity_path;
mod types;
mod vega;

#[cfg(test)]
mod proptest_tests;

pub use aad_validation::{
    AADCrossValidator, DiscontinuityDetector, DiscontinuityKind, DiscontinuityPoint,
    PointValidation, SmoothingConfig, ValidationReport,
};
pub use breeden_litzenberger::BreedenLitzenberger;
pub use builder::VolCubeBuilder;
pub use cache::{CacheStats, SharedVolCubeCache, VolCubeCache, VolCubeCacheEntry, VolCubeKey};
pub use calibration_graph::{
    CalibrationExecutor, CalibrationGraph, CalibrationNode, CalibrationNodeId, CalibrationState,
    GraphCalibrationResult, GraphError, NoOpCalibrationExecutor, NodeKind,
};
#[cfg(feature = "local-vol")]
pub use calibrator::LocalVolCalibrator;
#[cfg(feature = "stochastic-local-vol")]
pub use calibrator::StochasticLocalVolCalibrator;
pub use calibrator::{
    default_calibrator, BoxedCalibrator, CalibrationResult, CalibratorOutput, SabrCalibrator,
    SviCalibrator, VolCubeCalibrator,
};
pub use config::{
    AxisInterpolationMethod, CalibrationOrder, ExtrapolationMethod, InterpolationMethod,
    OptimizerType, SabrParameterInterpolation, StrikeAxisType, VolCubeConfig,
};
pub use cube::{VolCube, VolatilityCube};
pub use engine::{
    calculate_forward_swap_rate, CalibrationProgress, ClosureForwardRateProvider, EngineOutput,
    ForwardRateError, ForwardRateProvider, ProgressCallback, VolCubeCalibrationEngine,
};
pub use error::{
    ArbitrageViolation, ArbitrageViolationType, BoundaryViolation, CalibrationDiagnostics,
    ConvergenceStatus, SabrParameter, SliceDiagnostics, VolCubeError,
};
pub use graph::{
    VolCubeEdgeType, VolCubeGraphData, VolCubeGraphEdge, VolCubeGraphNode, VolCubeNodeType,
    VolCubeSensitivityInfo,
};
pub use interpolator::{
    FlatInterpolator, InterpolationError, Interpolator, LinearInterpolator, VolCubeInterpolator,
};
pub use lazy_evaluator::{
    CalibratedSlice, LazyEvaluatorStats, LazyEvaluatorStatsSnapshot, QuoteUpdateListener,
    SliceCacheState, SliceKey, VolLazyEvaluator,
};
pub use loader_convert::{
    convert_capfloor_csv_rows, convert_json_quotes, convert_swaption_csv_rows, parse_currency,
    parse_expiry_to_date, parse_quote_type, parse_strike, parse_tenor_to_years,
    parse_underlying_index, ConversionError, ConversionResult, ToVolQuote, ToVolQuoteSet,
};
pub use quote::{
    vol_quote_to_instrument, Currency, GridStats, QuoteType, Tenor, UnderlyingIndex, VolQuote,
    VolQuoteSet, VolStrike,
};
pub use sabr_surface::SabrParameterSurface;
pub use sensitivity_path::{
    IndirectSensitivity, IndirectSensitivityCalculator, SensitivityEdge, SensitivityNode,
    SensitivityNodeKind, SensitivityPath, SensitivityPathBuilder,
};
pub use types::{InstrumentId, SabrParams, VolInstrument};
pub use vega::{
    BucketVega, ForwardModeVegaCalculator, PointVega, VegaBumpConfig, VegaComputationMode,
    VegaError, VegaGrid, VegaVerificationResult, VolCubeVegaCalculator,
};
