//! Optimisation algorithms for numerical computation.
//!
//! This module provides optimisation algorithms commonly used in quantitative
//! finance for model calibration, curve fitting, and parameter estimation.
//!
//! ## Available Optimisers
//!
//! ### Derivative-Free Methods
//!
//! - [`minimize_nelder_mead`]: Simplex method for functions without gradient
//!
//! ### Gradient-Based Methods
//!
//! - [`minimize_lbfgs`]: Limited-memory BFGS for large-scale problems
//! - [`minimize_lbfgs_numerical`]: L-BFGS with numerical gradient
//!
//! ## Configuration
//!
//! - [`OptimisationConfig`]: Base configuration for all optimisers
//! - [`LbfgsConfig`]: L-BFGS specific settings
//! - [`NelderMeadConfig`]: Nelder-Mead specific settings
//!
//! ## Results
//!
//! - [`OptimisationResult`]: Contains optimal parameters, value, and metadata
//! - [`OptimisationError`]: Error types for optimisation failures
//!
//! ## Feature Flags
//!
//! - `external-numerics`: Uses battle-tested external implementations (argmin)
//!   instead of internal implementations. Enabled by default.
//!
//! ## Example
//!
//! ```ignore
//! use pricer_core::math::optimisers::{
//!     minimize_nelder_mead, minimize_lbfgs, NelderMeadConfig, LbfgsConfig
//! };
//!
//! // Derivative-free optimisation
//! let f = |x: &[f64]| x[0] * x[0] + x[1] * x[1];
//! let result = minimize_nelder_mead(f, &[5.0, 5.0], NelderMeadConfig::default()).unwrap();
//!
//! // Gradient-based optimisation
//! let f_grad = |x: &[f64]| {
//!     let val = x[0] * x[0] + x[1] * x[1];
//!     let grad = vec![2.0 * x[0], 2.0 * x[1]];
//!     (val, grad)
//! };
//! let result = minimize_lbfgs(f_grad, &[5.0, 5.0], LbfgsConfig::default()).unwrap();
//! ```

mod config;
mod error;
mod lbfgs;
mod nelder_mead;
mod result;

// External implementations (argmin wrappers)
#[cfg(feature = "external-numerics")]
mod external;

pub use config::{LbfgsConfig, NelderMeadConfig, OptimisationConfig};
pub use error::OptimisationError;
// External implementations (available when external-numerics is enabled)
#[cfg(feature = "external-numerics")]
pub use external::{
    minimize_lbfgs_external, minimize_lbfgs_numerical_external, minimize_nelder_mead_external,
};
// Internal implementations (fallback when external-numerics is disabled)
pub use lbfgs::{minimize_lbfgs, minimize_lbfgs_numerical};
pub use nelder_mead::minimize_nelder_mead;
pub use result::OptimisationResult;
