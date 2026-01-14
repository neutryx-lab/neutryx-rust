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
//! ## AAD Support
//!
//! When the `num-dual-mode` feature is enabled, the bootstrapper
//! computes sensitivities using the implicit function theorem,
//! avoiding recording solver iterations in the AD tape.

mod adjoint_solver;
mod config;
mod curve;
mod curve_builder;
mod engine;
mod error;
mod instrument;
mod sensitivity;

pub use adjoint_solver::{
    compute_adjoint_contribution, AdjointSolver, AdjointSolverConfig, SolveResult,
    SolveResultWithSensitivities, SolverType,
};
pub use config::{BootstrapInterpolation, GenericBootstrapConfig, GenericBootstrapConfigBuilder};
pub use curve::{BootstrappedCurve, BootstrappedCurveBuilder};
pub use curve_builder::{BootstrapConfig, CurveBootstrapper, InterpolationMethod};
pub use engine::{GenericBootstrapResult, SequentialBootstrapper};
pub use error::BootstrapError;
pub use instrument::{BootstrapInstrument, Frequency};
pub use sensitivity::{
    BootstrapResultWithSensitivities, SensitivityBootstrapper, SensitivityVerification,
};

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
