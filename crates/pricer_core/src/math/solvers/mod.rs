//! Root-finding and optimisation solvers for numerical computation.
//!
//! This module provides a collection of root-finding and optimisation
//! algorithms designed for financial applications such as implied volatility
//! calculation, curve calibration, and model parameter fitting, with support
//! for automatic differentiation.
//!
//! ## Available Solvers
//!
//! ### Root-Finding (Internal Implementations)
//!
//! - [`NewtonRaphsonSolver`]: Fast quadratic convergence using derivatives.
//!   Supports AD via `find_root_ad` method.
//! - [`BacktrackingNewtonSolver`]: Newton-Raphson with Armijo line search for
//!   improved global convergence
//! - [`BisectionSolver`]: Simple, robust bracketing method with linear
//!   convergence
//! - [`BrentSolver`]: Robust bracketing method without derivative requirement
//!
//! ### Optimisation
//!
//! - [`LevenbergMarquardtSolver`]: Nonlinear least-squares for model
//!   calibration (internal implementation)
//! - [`solve_lm_external`]: External LM implementation using
//!   `levenberg-marquardt` crate (requires `external-numerics` feature)
//!
//! ## Feature Flags
//!
//! - `external-numerics`: Enables external solver implementations that use
//!   battle-tested crates (`levenberg-marquardt`). Enabled by default.
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
//! ## AD Compatibility
//!
//! The Newton-Raphson solver provides an AD-powered `find_root_ad` method
//! that automatically computes derivatives using `Dual64`, eliminating the
//! need to provide explicit derivative functions. Other root-finding solvers
//! support generic Float types and are compatible with AD frameworks.
//!
//! Note: External solver implementations (`solve_lm_external`) only support
//! `f64` and are not AD-compatible. Use the internal implementations when
//! AD is required.
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
//! ### Bisection Method
//!
//! ```
//! use pricer_core::math::solvers::{BisectionSolver, SolverConfig};
//!
//! // Solve x³ - x - 2 = 0 in bracket [1, 2]
//! let solver = BisectionSolver::new(SolverConfig::default());
//!
//! let f = |x: f64| x * x * x - x - 2.0;
//!
//! let root = solver.find_root(f, 1.0, 2.0).unwrap();
//! assert!((f(root)).abs() < 1e-10);
//! ```
//!
//! ### Backtracking Newton
//!
//! ```
//! use pricer_core::math::solvers::{BacktrackingNewtonSolver, SolverConfig};
//!
//! // Solve x² - 2 = 0 with backtracking for robust convergence
//! let solver = BacktrackingNewtonSolver::new(SolverConfig::default());
//!
//! let f = |x: f64| x * x - 2.0;
//! let f_prime = |x: f64| 2.0 * x;
//!
//! let root = solver.find_root(f, f_prime, 10.0).unwrap(); // Works even with far initial guess
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

mod backtracking_newton;
mod bisection;
mod brent;
mod config;
mod levenberg_marquardt;
mod newton_raphson;

// Multi-dimensional Newton-Raphson (requires linalg feature)
#[cfg(feature = "linalg")]
mod multidim_newton;

// External implementations (levenberg-marquardt crate wrapper)
#[cfg(feature = "external-numerics")]
mod external;

// Re-export public types at module level
pub use backtracking_newton::BacktrackingNewtonSolver;
pub use bisection::BisectionSolver;
pub use brent::BrentSolver;
pub use config::SolverConfig;
// External implementations (available when external-numerics is enabled)
#[cfg(feature = "external-numerics")]
pub use external::solve_lm_external;
pub use levenberg_marquardt::{LMConfig, LMResult, LevenbergMarquardtSolver};
pub use newton_raphson::NewtonRaphsonSolver;

// Multi-dimensional solver exports (requires linalg feature)
#[cfg(feature = "linalg")]
pub use multidim_newton::{
    MultidimNewtonConfig, MultidimSolverResult, MultidimensionalNewtonSolver, SystemOfEquations,
};
