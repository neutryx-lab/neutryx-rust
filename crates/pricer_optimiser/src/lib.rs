// Clippy configuration for pricer_optimiser
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::similar_names)]
#![allow(clippy::redundant_else)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::unused_self)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::if_not_else)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::too_many_lines)]

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
//! Can optionally use `pricer_pricing` (L3) for gradient computation via
//! Enzyme.
//!
//! ## Modules
//!
//! - `bootstrapping`: Yield curve stripping from OIS/Swap rates (multi-curve)
//! - `calibration`: Stochastic model calibration (e.g., Hull-White α/σ from
//!   swaptions)
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
    pub use crate::{bootstrapping::*, calibration::*, solvers::*, OptimiserError};
}
