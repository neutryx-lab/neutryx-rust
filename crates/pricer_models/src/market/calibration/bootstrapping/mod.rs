//! Yield curve bootstrapping from OIS/Swap rates.
//!
//! This module provides two bootstrapping approaches:
//!
//! - `CurveBootstrapper`: Simple sequential bootstrapping for basic use cases
//! - `GlobalBootstrapper`: Multi-dimensional Newton solver for global calibration
//!
//! ## Core Types
//!
//! - `BootstrapInstrument<T>`: Market instruments (OIS, IRS, FRA, Futures)
//! - `BootstrappedCurve<T>`: Result curve implementing `YieldCurve<T>`
//! - `CalibrationInstrument<T>`: Trait for global calibration instruments
//!
//! ## Example
//!
//! ```ignore
//! use pricer_models::market::calibration::bootstrapping::{
//!     CurveBootstrapper, BootstrapInstrument,
//! };
//!
//! let bootstrapper = CurveBootstrapper::new();
//! let pillars = vec![1.0, 2.0, 5.0];
//! let rates = vec![0.03, 0.035, 0.04];
//!
//! let result = bootstrapper.bootstrap(&pillars, &rates)?;
//! ```

mod calibration_instrument;
mod config;
mod curve;
mod curve_builder;
mod error;
mod instrument;

// Global bootstrapping with multi-dimensional Newton solver
#[cfg(feature = "global-bootstrap")]
mod global_bootstrapper;

// Core exports
pub use calibration_instrument::CalibrationInstrument;
pub use config::{BootstrapInterpolation, GenericBootstrapConfig, GenericBootstrapConfigBuilder};
pub use curve::{BootstrappedCurve, BootstrappedCurveBuilder};
pub use curve_builder::{BootstrapConfig, CurveBootstrapper, InterpolationMethod};
pub use error::BootstrapError;
pub use instrument::{BootstrapInstrument, Frequency};

// Global bootstrapping exports (requires linalg feature)
#[cfg(feature = "global-bootstrap")]
pub use global_bootstrapper::{GlobalBootstrapConfig, GlobalBootstrapResult, GlobalBootstrapper};

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
