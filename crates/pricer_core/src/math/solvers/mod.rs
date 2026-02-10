//! Root-finding and optimisation solvers for numerical computation.
//!
//! This module provides root-finding and optimisation algorithms designed for
//! financial applications such as implied volatility calculation, curve
//! calibration, and model parameter fitting, with support for automatic
//! differentiation.
//!
//! ## Available Solvers
//!
//! - [`NewtonRaphsonSolver`]: Fast quadratic convergence using derivatives.
//!   Supports AD via `find_root_ad` method.
//! - [`LevenbergMarquardtSolver`]: Nonlinear least-squares for model
//!   calibration (internal implementation)
//! - [`MultidimensionalNewtonSolver`]: Multi-dimensional Newton for systems
//!   F(x)=0 (requires `linalg` feature)
//!
//! ## Configuration
//!
//! Root-finding solvers use [`SolverConfig`] for configuring:
//! - `tolerance`: Convergence tolerance (default: 1e-10)
//! - `max_iterations`: Maximum iteration count (default: 100)
//!
//! The LM solver uses [`LMConfig`] with additional parameters for damping
//! control.
//!
//! ## Examples
//!
//! ```
//! use pricer_core::math::solvers::{NewtonRaphsonSolver, SolverConfig};
//!
//! // Solve x² - 2 = 0 (find √2)
//! let config = SolverConfig::default();
//! let solver = NewtonRaphsonSolver::new(config);
//!
//! let f = |x: f64| x * x - 2.0;
//! let f_prime = |x: f64| 2.0 * x;
//!
//! let root = solver.find_root(f, f_prime, 1.0).unwrap();
//! assert!((root - std::f64::consts::SQRT_2).abs() < 1e-10);
//! ```

mod config;
mod levenberg_marquardt;
mod newton_raphson;

// Multi-dimensional Newton-Raphson (requires linalg feature)
#[cfg(feature = "linalg")]
mod multidim_newton;

// Re-export public types at module level
pub use config::SolverConfig;
pub use levenberg_marquardt::{LMConfig, LMResult, LevenbergMarquardtSolver};
// Multi-dimensional solver exports (requires linalg feature)
#[cfg(feature = "linalg")]
pub use multidim_newton::{
    MultidimNewtonConfig, MultidimSolverResult, MultidimensionalNewtonSolver, SystemOfEquations,
};
pub use newton_raphson::NewtonRaphsonSolver;
