//! # pricer_optimiser
//!
//! Calibration, bootstrapping, and numerical solvers for Neutryx.
//!
//! This crate sits between Models (L2) and Pricing (L3) in the architecture,
//! solving inverse problems to construct valid model objects.
//!
//! ## Architecture Position
//!
//! Layer 2.5 in the **P**ricer layer of the A-I-P-S architecture.
//! Depends on `pricer_core` (L1) and `pricer_models` (L2).
//! Can optionally use `pricer_pricing` (L3) for gradient computation via Enzyme.
//!
//! ## Modules
//!
//! - `bootstrapping`: Yield curve stripping from OIS/Swap rates (multi-curve)
//! - `calibration`: Stochastic model calibration (e.g., Hull-White α/σ from swaptions)
//! - `solvers`: Levenberg-Marquardt, BFGS algorithms
//!
//! ## Example
//!
//! ```rust,ignore
//! use pricer_optimiser::{LevenbergMarquardt, CalibrationProblem};
//!
//! let problem = CalibrationProblem::new(market_data);
//! let solver = LevenbergMarquardt::default();
//! let calibrated_params = solver.solve(&problem)?;
//! ```

pub mod bootstrapping;
pub mod calibration;
pub mod provider;
pub mod solvers;

mod error;

pub use error::OptimiserError;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::bootstrapping::*;
    pub use crate::calibration::*;
    pub use crate::solvers::*;
    pub use crate::OptimiserError;
}
