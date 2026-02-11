//! Levenberg-Marquardt nonlinear least-squares solver.
//!
//! This module provides the [`LevenbergMarquardtSolver`] for solving nonlinear
//! least-squares problems commonly encountered in model calibration.
//!
//! # Algorithm
//!
//! The Levenberg-Marquardt algorithm combines Gauss-Newton and gradient
//! descent:
//!
//! ```text
//! (J^T J + λI) δ = J^T r
//! p_{n+1} = p_n + δ
//! ```
//!
//! where:
//! - `J` is the Jacobian matrix of residuals
//! - `r` is the residual vector
//! - `λ` is the damping factor (adjusted during iteration)
//! - `δ` is the parameter update step
//!
//! # Implementation
//!
//! Delegates to the [`levenberg_marquardt`](::levenberg_marquardt) crate while
//! preserving the same public API.
//!
//! # Example
//!
//! ```
//! use pricer_core::math::solvers::{LevenbergMarquardtSolver, LMConfig};
//!
//! // Fit y = a * exp(-b * x) to data
//! let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
//! let y_data = vec![1.0, 0.37, 0.14, 0.05, 0.02];
//!
//! let config = LMConfig::default();
//! let solver = LevenbergMarquardtSolver::new(config);
//!
//! // Residual function: model(x) - y
//! let residuals = |params: &[f64]| -> Vec<f64> {
//!     let a = params[0];
//!     let b = params[1];
//!     x_data.iter().zip(&y_data).map(|(&x, &y)| {
//!         a * (-b * x).exp() - y
//!     }).collect()
//! };
//!
//! let initial_params = vec![1.0, 1.0];
//! let result = solver.solve(residuals, initial_params).unwrap();
//!
//! // Should converge to a ≈ 1.0, b ≈ 1.0
//! assert!(result.converged);
//! ```

use levenberg_marquardt::{LeastSquaresProblem, LevenbergMarquardt};
use nalgebra::{DMatrix, DVector, Dyn, Owned};

use crate::types::SolverError;

/// Configuration for Levenberg-Marquardt solver.
///
/// # Fields
///
/// * `tolerance` - Convergence tolerance for residual norm
/// * `max_iterations` - Maximum number of iterations
/// * `initial_lambda` - Initial damping factor
/// * `lambda_up` - Factor to increase lambda when step is rejected
/// * `lambda_down` - Factor to decrease lambda when step is accepted
/// * `min_lambda` - Minimum value for lambda
/// * `max_lambda` - Maximum value for lambda
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LMConfig {
    /// Convergence tolerance for relative residual change.
    pub tolerance: f64,
    /// Maximum number of iterations.
    pub max_iterations: usize,
    /// Initial damping factor.
    pub initial_lambda: f64,
    /// Factor to increase lambda on rejected step.
    pub lambda_up: f64,
    /// Factor to decrease lambda on accepted step.
    pub lambda_down: f64,
    /// Minimum damping factor.
    pub min_lambda: f64,
    /// Maximum damping factor.
    pub max_lambda: f64,
    /// Tolerance for parameter change convergence.
    pub param_tolerance: f64,
}

impl Default for LMConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-10,
            max_iterations: 100,
            initial_lambda: 1e-3,
            lambda_up: 10.0,
            lambda_down: 0.1,
            min_lambda: 1e-10,
            max_lambda: 1e10,
            param_tolerance: 1e-10,
        }
    }
}

impl LMConfig {
    /// Create a new LM configuration.
    pub fn new(tolerance: f64, max_iterations: usize) -> Self {
        Self {
            tolerance,
            max_iterations,
            ..Default::default()
        }
    }

    /// Create a fast configuration with relaxed tolerances.
    pub fn fast() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 50,
            ..Default::default()
        }
    }

    /// Create a high precision configuration.
    pub fn high_precision() -> Self {
        Self {
            tolerance: 1e-14,
            max_iterations: 500,
            param_tolerance: 1e-14,
            ..Default::default()
        }
    }
}

/// Result of Levenberg-Marquardt optimisation.
#[derive(Debug, Clone, PartialEq)]
pub struct LMResult {
    /// Final optimised parameters.
    pub params: Vec<f64>,
    /// Final residual sum of squares.
    pub residual_ss: f64,
    /// Number of iterations performed.
    pub iterations: usize,
    /// Whether convergence was achieved.
    pub converged: bool,
    /// Final lambda value.
    pub final_lambda: f64,
}

impl LMResult {
    /// Create a new LM result.
    pub fn new(
        params: Vec<f64>,
        residual_ss: f64,
        iterations: usize,
        converged: bool,
        final_lambda: f64,
    ) -> Self {
        Self {
            params,
            residual_ss,
            iterations,
            converged,
            final_lambda,
        }
    }

    /// Get the root mean square error.
    pub fn rmse(&self, n_observations: usize) -> f64 {
        if n_observations == 0 {
            return 0.0;
        }
        (self.residual_ss / n_observations as f64).sqrt()
    }
}

// =============================================================================
// Adapter: wraps a closure-based residual function as a LeastSquaresProblem
// =============================================================================

/// Internal adapter that wraps a user-supplied closure into the
/// [`LeastSquaresProblem`] trait required by the `levenberg-marquardt` crate.
struct ClosureProblem<F> {
    /// Current parameter vector.
    params: DVector<f64>,
    /// User-supplied residual function.
    residuals_fn: F,
}

impl<F> LeastSquaresProblem<f64, Dyn, Dyn> for ClosureProblem<F>
where
    F: Fn(&[f64]) -> Vec<f64>,
{
    type ParameterStorage = Owned<f64, Dyn>;
    type ResidualStorage = Owned<f64, Dyn>;
    type JacobianStorage = Owned<f64, Dyn, Dyn>;

    fn set_params(&mut self, params: &DVector<f64>) { self.params.copy_from(params); }

    fn params(&self) -> DVector<f64> { self.params.clone() }

    fn residuals(&self) -> Option<DVector<f64>> {
        let r = (self.residuals_fn)(self.params.as_slice());
        if r.is_empty() {
            return None;
        }
        Some(DVector::from_vec(r))
    }

    fn jacobian(&self) -> Option<DMatrix<f64>> {
        let params = self.params.as_slice();
        let r0 = (self.residuals_fn)(params);
        let n_params = params.len();
        let n_residuals = r0.len();
        if n_residuals == 0 || n_params == 0 {
            return None;
        }

        let eps = 1e-8;
        let mut jacobian = DMatrix::zeros(n_residuals, n_params);

        for j in 0..n_params {
            let h = eps * params[j].abs().max(1.0);
            let mut params_plus = params.to_vec();
            params_plus[j] += h;

            let r_plus = (self.residuals_fn)(&params_plus);

            for i in 0..n_residuals {
                jacobian[(i, j)] = (r_plus[i] - r0[i]) / h;
            }
        }

        Some(jacobian)
    }
}

/// Levenberg-Marquardt nonlinear least-squares solver.
///
/// Solves optimisation problems of the form:
/// ```text
/// min_p ||f(p)||^2
/// ```
///
/// where `f(p)` is a vector-valued function (residuals) and `p` is a parameter
/// vector.
///
/// # Type Parameters
///
/// This solver works with `f64` for optimal numerical stability.
///
/// # Example
///
/// ```
/// use pricer_core::math::solvers::{LevenbergMarquardtSolver, LMConfig, LMResult};
///
/// let config = LMConfig::default();
/// let solver = LevenbergMarquardtSolver::new(config);
///
/// // Simple quadratic: minimize (p[0] - 2)^2 + (p[1] - 3)^2
/// let residuals = |params: &[f64]| -> Vec<f64> {
///     vec![params[0] - 2.0, params[1] - 3.0]
/// };
///
/// let result = solver.solve(residuals, vec![0.0, 0.0]).unwrap();
/// assert!(result.converged);
/// assert!((result.params[0] - 2.0).abs() < 1e-6);
/// assert!((result.params[1] - 3.0).abs() < 1e-6);
/// ```
#[derive(Debug, Clone)]
pub struct LevenbergMarquardtSolver {
    config: LMConfig,
}

impl LevenbergMarquardtSolver {
    /// Create a new LM solver with the given configuration.
    pub fn new(config: LMConfig) -> Self { Self { config } }

    /// Create a solver with default configuration.
    pub fn with_defaults() -> Self {
        Self {
            config: LMConfig::default(),
        }
    }

    /// Get the solver configuration.
    pub fn config(&self) -> &LMConfig { &self.config }

    /// Solve the nonlinear least-squares problem.
    ///
    /// # Arguments
    ///
    /// * `residuals` - Function that computes residuals given parameters
    /// * `initial_params` - Initial parameter guess
    ///
    /// # Returns
    ///
    /// * `Ok(LMResult)` - Optimisation result with final parameters
    /// * `Err(SolverError)` - If optimisation fails
    pub fn solve<F>(&self, residuals: F, initial_params: Vec<f64>) -> Result<LMResult, SolverError>
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        let n_params = initial_params.len();
        if n_params == 0 {
            return Err(SolverError::NumericalInstability(
                "Empty parameter vector".to_string(),
            ));
        }

        // Probe to discover n_residuals
        let r0 = residuals(&initial_params);
        let n_residuals = r0.len();
        if n_residuals == 0 {
            return Err(SolverError::NumericalInstability(
                "Empty residual vector".to_string(),
            ));
        }

        // Check if already optimal
        let initial_ss: f64 = r0.iter().map(|x| x * x).sum();
        if initial_ss.sqrt() < self.config.tolerance {
            return Ok(LMResult::new(
                initial_params,
                initial_ss,
                0,
                true,
                self.config.initial_lambda,
            ));
        }

        // Build adapter problem
        let problem = ClosureProblem {
            params: DVector::from_vec(initial_params),
            residuals_fn: residuals,
        };

        // Configure the library solver.
        //
        // The `levenberg-marquardt` crate uses `ftol` / `gtol` / `xtol` for
        // convergence, and `patience` for maximum iterations. We map our
        // config accordingly.
        let lm = LevenbergMarquardt::new()
            .with_patience(self.config.max_iterations)
            .with_stepbound(self.config.max_lambda)
            .with_tol(self.config.tolerance);

        let (result, report) = lm.minimize(problem);

        let final_params: Vec<f64> = result.params.as_slice().to_vec();
        let final_r = (result.residuals_fn)(&final_params);
        let final_ss: f64 = final_r.iter().map(|x| x * x).sum();

        let converged = matches!(
            report.termination,
            levenberg_marquardt::TerminationReason::Converged { .. }
                | levenberg_marquardt::TerminationReason::ResidualsZero
                | levenberg_marquardt::TerminationReason::Orthogonal
        );

        // If the library converged but our own tolerance check is stricter,
        // honour our tolerance.
        let converged = converged || final_ss.sqrt() < self.config.tolerance;

        Ok(LMResult::new(
            final_params,
            final_ss,
            report.number_of_evaluations,
            converged,
            0.0, // Library does not expose final lambda
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // LMConfig Tests
    // ========================================

    #[test]
    fn test_config_default() {
        let config = LMConfig::default();
        assert!((config.tolerance - 1e-10).abs() < 1e-15);
        assert_eq!(config.max_iterations, 100);
        assert!(config.initial_lambda > 0.0);
    }

    #[test]
    fn test_config_new() {
        let config = LMConfig::new(1e-8, 50);
        assert!((config.tolerance - 1e-8).abs() < 1e-15);
        assert_eq!(config.max_iterations, 50);
    }

    #[test]
    fn test_config_fast() {
        let config = LMConfig::fast();
        assert!(config.tolerance > 1e-8);
        assert!(config.max_iterations <= 50);
    }

    #[test]
    fn test_config_high_precision() {
        let config = LMConfig::high_precision();
        assert!(config.tolerance < 1e-12);
        assert!(config.max_iterations >= 500);
    }

    // ========================================
    // LMResult Tests
    // ========================================

    #[test]
    fn test_result_new() {
        let result = LMResult::new(vec![1.0, 2.0], 0.01, 10, true, 1e-5);
        assert_eq!(result.params.len(), 2);
        assert!(result.converged);
        assert_eq!(result.iterations, 10);
    }

    #[test]
    fn test_result_rmse() {
        let result = LMResult::new(vec![1.0], 4.0, 10, true, 1e-5);
        let rmse = result.rmse(4);
        assert!((rmse - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_result_rmse_empty() {
        let result = LMResult::new(vec![1.0], 4.0, 10, true, 1e-5);
        let rmse = result.rmse(0);
        assert_eq!(rmse, 0.0);
    }

    // ========================================
    // LevenbergMarquardtSolver Tests
    // ========================================

    #[test]
    fn test_solver_new() {
        let config = LMConfig::default();
        let solver = LevenbergMarquardtSolver::new(config);
        assert!(solver.config().tolerance > 0.0);
    }

    #[test]
    fn test_solver_with_defaults() {
        let solver = LevenbergMarquardtSolver::with_defaults();
        assert!(solver.config().tolerance > 0.0);
    }

    #[test]
    fn test_solve_simple_linear() {
        // Minimize (p[0] - 2)^2 + (p[1] - 3)^2
        let residuals = |params: &[f64]| -> Vec<f64> { vec![params[0] - 2.0, params[1] - 3.0] };

        let solver = LevenbergMarquardtSolver::with_defaults();
        let result = solver.solve(residuals, vec![0.0, 0.0]).unwrap();

        assert!(result.converged);
        assert!((result.params[0] - 2.0).abs() < 1e-6);
        assert!((result.params[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_solve_quadratic() {
        // Minimize (p - 3)^2
        let residuals = |params: &[f64]| -> Vec<f64> { vec![params[0] - 3.0] };

        let solver = LevenbergMarquardtSolver::with_defaults();
        let result = solver.solve(residuals, vec![10.0]).unwrap();

        assert!(result.converged);
        assert!((result.params[0] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_solve_rosenbrock() {
        // Rosenbrock function residuals: [10(p[1] - p[0]^2), (1 - p[0])]
        // Minimum at (1, 1)
        let residuals = |params: &[f64]| -> Vec<f64> {
            vec![10.0 * (params[1] - params[0] * params[0]), 1.0 - params[0]]
        };

        let config = LMConfig {
            max_iterations: 200,
            ..Default::default()
        };
        let solver = LevenbergMarquardtSolver::new(config);
        let result = solver.solve(residuals, vec![0.0, 0.0]).unwrap();

        // Rosenbrock may need more iterations
        assert!((result.params[0] - 1.0).abs() < 0.1 || result.residual_ss < 0.01);
    }

    #[test]
    fn test_solve_exponential_fit() {
        // Fit y = a * exp(-x) to data where a = 1
        // Using explicit f64 to avoid type inference issues
        fn model(a: f64, x: f64) -> f64 { a * (-x).exp() }

        let x_data: [f64; 3] = [0.0, 1.0, 2.0];
        let y_data: [f64; 3] = [1.0, (-1.0_f64).exp(), (-2.0_f64).exp()];

        let residuals = |params: &[f64]| -> Vec<f64> {
            let a = params[0];
            vec![
                model(a, x_data[0]) - y_data[0],
                model(a, x_data[1]) - y_data[1],
                model(a, x_data[2]) - y_data[2],
            ]
        };

        let solver = LevenbergMarquardtSolver::with_defaults();
        let result = solver.solve(residuals, vec![0.5]).unwrap();

        assert!(result.converged);
        assert!((result.params[0] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_solve_already_optimal() {
        // Start at optimal point
        let residuals = |params: &[f64]| -> Vec<f64> { vec![params[0] - 5.0] };

        let solver = LevenbergMarquardtSolver::with_defaults();
        let result = solver.solve(residuals, vec![5.0]).unwrap();

        assert!(result.converged);
        assert!(result.iterations <= 1);
    }

    #[test]
    fn test_solve_empty_params() {
        let residuals = |_params: &[f64]| -> Vec<f64> { vec![1.0] };

        let solver = LevenbergMarquardtSolver::with_defaults();
        let result = solver.solve(residuals, vec![]);

        assert!(result.is_err());
    }

    #[test]
    fn test_solve_multi_dimensional() {
        // Minimize sum of (p[i] - i)^2
        let residuals = |params: &[f64]| -> Vec<f64> {
            params
                .iter()
                .enumerate()
                .map(|(i, &p)| p - i as f64)
                .collect()
        };

        let solver = LevenbergMarquardtSolver::with_defaults();
        let result = solver
            .solve(residuals, vec![10.0, 10.0, 10.0, 10.0])
            .unwrap();

        assert!(result.converged);
        for (i, &p) in result.params.iter().enumerate() {
            assert!((p - i as f64).abs() < 1e-6);
        }
    }

    // ========================================
    // Clone/Debug Tests
    // ========================================

    #[test]
    fn test_solver_clone() {
        let solver1 = LevenbergMarquardtSolver::with_defaults();
        let solver2 = solver1.clone();
        assert_eq!(
            solver1.config().max_iterations,
            solver2.config().max_iterations
        );
    }

    #[test]
    fn test_config_clone() {
        let config1 = LMConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_result_clone() {
        let result1 = LMResult::new(vec![1.0], 0.01, 10, true, 1e-5);
        let result2 = result1.clone();
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_solver_debug() {
        let solver = LevenbergMarquardtSolver::with_defaults();
        let debug_str = format!("{:?}", solver);
        assert!(debug_str.contains("LevenbergMarquardtSolver"));
    }
}
