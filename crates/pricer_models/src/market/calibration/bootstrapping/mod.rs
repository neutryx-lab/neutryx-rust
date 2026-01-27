//! Yield curve bootstrapping from OIS/Swap rates.
//!
//! This module implements multi-curve stripping logic to construct
//! yield curves from market-observed swap rates, with support for
//! AAD (Adjoint Algorithmic Differentiation) sensitivity computation.
//!
//! ## Architecture
//!
//! The bootstrapping module provides:
//! - `BootstrapInstrument<T>`: Market instruments (OIS, IRS, FRA, Futures)
//! - `CurveBootstrapper<T>`: Sequential bootstrapping engine
//! - `BootstrappedCurve<T>`: Result curve implementing `YieldCurve<T>`
//! - `MultiCurveBuilder<T>`: OIS discount + tenor curve construction
//!
//! ## Sensitivity Support
//!
//! The bootstrapper computes sensitivities using bump-and-revalue
//! finite differences for yield curve risk calculations.

mod adapter;
mod adjoint_solver;
mod cache;
mod config;
mod curve;
mod curve_builder;
mod curve_config;
mod curve_engine;
mod date_utils;
mod definition;
mod engine;
mod engine_error;
mod error;
mod instrument;
mod multi_curve;
mod result_cache;
mod sensitivity;

pub use adapter::InstrumentAdapter;
pub use adjoint_solver::{
    compute_adjoint_contribution, AdjointSolver, AdjointSolverConfig, SolveResult,
    SolveResultWithSensitivities, SolverType,
};
pub use cache::{BootstrapCache, BufferPool, CurveCache, InterpolationIndices};
pub use config::{BootstrapInterpolation, GenericBootstrapConfig, GenericBootstrapConfigBuilder};
pub use curve::{BootstrappedCurve, BootstrappedCurveBuilder};
pub use curve_builder::{BootstrapConfig, CurveBootstrapper, InterpolationMethod};
pub use curve_config::{CurveConfig, CurveConfigBuilder};
pub use curve_engine::{CurveConstructionResult, CurveEngine, CurveEngineBuilder};
pub use date_utils::{DateCalculator, DateCalculatorBuilder, SpotDateConvention};
pub use definition::{CurveDefinition, CurveInstrumentType, InstrumentSpec, InstrumentTenor};
pub use engine::{CachedBootstrapper, GenericBootstrapResult, SequentialBootstrapper};
pub use engine_error::{CurveEngineError, CurveParameterRepresentation};
pub use error::BootstrapError;
pub use instrument::{BootstrapInstrument, Frequency};
pub use multi_curve::{
    CurveDependency, CurveSet, MultiCurveBuilder, ParallelCurveSetBuilder, Tenor,
};
pub use result_cache::{CacheStats, CurveKey, CurveResultCache};
pub use sensitivity::{BootstrapResultWithSensitivities, SensitivityBootstrapper};

/// Result of curve bootstrapping.
#[derive(Debug, Clone)]
pub struct BootstrapResult {
    /// Discount factors at each pillar
    pub discount_factors: Vec<f64>,
    /// Pillar dates (in years from today)
    pub pillars: Vec<f64>,
    /// Residual error
    pub residual: f64,
}
