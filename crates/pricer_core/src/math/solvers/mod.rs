//! Root-finding and optimization solvers for numerical computation.
//!
//! This module provides a collection of root-finding and optimization algorithms
//! designed for financial applications such as implied volatility calculation,
//! curve calibration, and model parameter fitting, with support for automatic
//! differentiation.
//!
//! ## Available Solvers
//!
//! ### Root-Finding
//!
//! - [`NewtonRaphsonSolver`]: Fast quadratic convergence using derivatives
//! - [`BrentSolver`]: Robust bracketing method without derivative requirement
//!
//! ### Optimization
//!
//! - [`LevenbergMarquardtSolver`]: Nonlinear least-squares for model calibration
//!
//! ## Configuration
//!
//! Root-finding solvers use [`SolverConfig`] for configuring:
//! - `tolerance`: Convergence tolerance (default: 1e-10)
//! - `max_iterations`: Maximum iteration count (default: 100)
//!
//! The LM solver uses [`LMConfig`] with additional parameters for damping control.
//!
//! ## AD Compatibility
//!
//! The Newton-Raphson solver provides an AD-powered `find_root_ad` method
//! that automatically computes derivatives using `Dual64`, eliminating the
//! need to provide explicit derivative functions.
//!
//! ## Examples
//!
//! ### Root-Finding
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
//!
//! ### Nonlinear Least-Squares
//!
//! ```
//! use pricer_core::math::solvers::{LevenbergMarquardtSolver, LMConfig};
//!
//! // Minimize (p[0] - 2)² + (p[1] - 3)²
//! let residuals = |params: &[f64]| -> Vec<f64> {
//!     vec![params[0] - 2.0, params[1] - 3.0]
//! };
//!
//! let solver = LevenbergMarquardtSolver::with_defaults();
//! let result = solver.solve(residuals, vec![0.0, 0.0]).unwrap();
//!
//! assert!(result.converged);
//! assert!((result.params[0] - 2.0).abs() < 1e-6);
//! ```

mod brent;
mod config;
mod levenberg_marquardt;
mod newton_raphson;

// Re-export public types at module level
pub use brent::BrentSolver;
pub use config::SolverConfig;
pub use levenberg_marquardt::{LMConfig, LMResult, LevenbergMarquardtSolver};
pub use newton_raphson::NewtonRaphsonSolver;
