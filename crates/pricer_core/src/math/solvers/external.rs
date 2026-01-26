//! External solver library wrappers.
//!
//! This module provides wrappers around external solver crates (levenberg-marquardt)
//! to provide battle-tested implementations while maintaining API compatibility.
//!
//! # Feature Flag
//!
//! This module requires the `external-numerics` feature flag.

use levenberg_marquardt::{LeastSquaresProblem, LevenbergMarquardt, MinimizationReport};
use nalgebra::{DMatrix, DVector, Dyn, OMatrix, OVector, Owned};

use super::levenberg_marquardt::{LMConfig, LMResult};
use crate::types::SolverError;

// =============================================================================
// Levenberg-Marquardt Wrapper
// =============================================================================

/// Problem wrapper for external LM crate.
///
/// Adapts a closure-based residual function to the LeastSquaresProblem trait.
struct LMProblem<F> {
    residuals_fn: F,
    n_params: usize,
    n_residuals: usize,
    params: DVector<f64>,
}

impl<F> LMProblem<F>
where
    F: Fn(&[f64]) -> Vec<f64>,
{
    fn new(residuals_fn: F, initial_params: Vec<f64>) -> Self {
        let n_params = initial_params.len();
        let params = DVector::from_vec(initial_params.clone());
        let r = (residuals_fn)(&initial_params);
        let n_residuals = r.len();

        Self {
            residuals_fn,
            n_params,
            n_residuals,
            params,
        }
    }
}

impl<F> LeastSquaresProblem<f64, Dyn, Dyn> for LMProblem<F>
where
    F: Fn(&[f64]) -> Vec<f64>,
{
    type ParameterStorage = Owned<f64, Dyn>;
    type ResidualStorage = Owned<f64, Dyn>;
    type JacobianStorage = Owned<f64, Dyn, Dyn>;

    fn set_params(&mut self, params: &OVector<f64, Dyn>) {
        self.params = params.clone();
    }

    fn params(&self) -> OVector<f64, Dyn> {
        self.params.clone()
    }

    fn residuals(&self) -> Option<OVector<f64, Dyn>> {
        let params_slice = self.params.as_slice();
        let r = (self.residuals_fn)(params_slice);
        Some(DVector::from_vec(r))
    }

    fn jacobian(&self) -> Option<OMatrix<f64, Dyn, Dyn>> {
        // Use numerical differentiation for Jacobian
        let eps = 1e-8;
        let params_slice = self.params.as_slice();
        let r0 = (self.residuals_fn)(params_slice);

        let mut jacobian = DMatrix::zeros(self.n_residuals, self.n_params);

        for j in 0..self.n_params {
            let h = eps * self.params[j].abs().max(1.0);
            let mut params_plus = self.params.clone();
            params_plus[j] += h;

            let r_plus = (self.residuals_fn)(params_plus.as_slice());

            for i in 0..self.n_residuals {
                jacobian[(i, j)] = (r_plus[i] - r0[i]) / h;
            }
        }

        Some(jacobian)
    }
}

/// Solve a nonlinear least-squares problem using the external levenberg-marquardt crate.
///
/// This wraps the external levenberg-marquardt crate, providing a battle-tested
/// implementation while maintaining API compatibility with the internal implementation.
///
/// # Arguments
///
/// * `residuals` - Function that computes residuals given parameters
/// * `initial_params` - Initial parameter guess
/// * `config` - LM solver configuration
///
/// # Returns
///
/// * `Ok(LMResult)` - Optimisation result with final parameters
/// * `Err(SolverError)` - If optimisation fails
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::solvers::{solve_lm_external, LMConfig};
///
/// let residuals = |params: &[f64]| -> Vec<f64> {
///     vec![params[0] - 2.0, params[1] - 3.0]
/// };
///
/// let result = solve_lm_external(residuals, vec![0.0, 0.0], LMConfig::default()).unwrap();
/// assert!(result.converged);
/// ```
pub fn solve_lm_external<F>(
    residuals: F,
    initial_params: Vec<f64>,
    config: LMConfig,
) -> Result<LMResult, SolverError>
where
    F: Fn(&[f64]) -> Vec<f64>,
{
    let n_params = initial_params.len();
    if n_params == 0 {
        return Err(SolverError::NumericalInstability(
            "Empty parameter vector".to_string(),
        ));
    }

    // Check that residuals function returns non-empty vector
    let test_residuals = residuals(&initial_params);
    if test_residuals.is_empty() {
        return Err(SolverError::NumericalInstability(
            "Empty residual vector".to_string(),
        ));
    }

    let problem = LMProblem::new(residuals, initial_params);

    // Configure the solver with tolerance and patience
    let solver = LevenbergMarquardt::new()
        .with_patience(config.max_iterations)
        .with_stepbound(config.initial_lambda);

    // Run the solver
    let (result, report): (LMProblem<F>, MinimizationReport<f64>) = solver.minimize(problem);

    // Extract results
    let final_params = result.params.as_slice().to_vec();
    let residuals_vec = result.residuals().unwrap_or_else(|| DVector::zeros(0));
    let residual_ss: f64 = residuals_vec.iter().map(|r| r * r).sum();

    // Determine convergence based on report
    let converged = report.termination.was_successful();
    let iterations = report.number_of_evaluations;
    let final_lambda = 1.0; // External crate doesn't expose this

    Ok(LMResult::new(
        final_params,
        residual_ss,
        iterations,
        converged,
        final_lambda,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // LM External Tests
    // ==========================================================================

    #[test]
    fn test_lm_external_simple_linear() {
        // Minimize (p[0] - 2)^2 + (p[1] - 3)^2
        let residuals = |params: &[f64]| -> Vec<f64> { vec![params[0] - 2.0, params[1] - 3.0] };

        let result = solve_lm_external(residuals, vec![0.0, 0.0], LMConfig::default()).unwrap();

        assert!(result.converged);
        assert!((result.params[0] - 2.0).abs() < 1e-4);
        assert!((result.params[1] - 3.0).abs() < 1e-4);
    }

    #[test]
    fn test_lm_external_quadratic() {
        // Minimize (p - 3)^2
        let residuals = |params: &[f64]| -> Vec<f64> { vec![params[0] - 3.0] };

        let result = solve_lm_external(residuals, vec![10.0], LMConfig::default()).unwrap();

        assert!(result.converged);
        assert!((result.params[0] - 3.0).abs() < 1e-4);
    }

    #[test]
    fn test_lm_external_rosenbrock() {
        // Rosenbrock function residuals: [10(p[1] - p[0]^2), (1 - p[0])]
        // Minimum at (1, 1)
        let residuals = |params: &[f64]| -> Vec<f64> {
            vec![10.0 * (params[1] - params[0] * params[0]), 1.0 - params[0]]
        };

        let config = LMConfig {
            max_iterations: 200,
            ..Default::default()
        };
        let result = solve_lm_external(residuals, vec![0.0, 0.0], config).unwrap();

        // Rosenbrock may need more iterations but should get close
        assert!((result.params[0] - 1.0).abs() < 0.1 || result.residual_ss < 0.01);
    }

    #[test]
    fn test_lm_external_exponential_fit() {
        // Fit y = a * exp(-x) to data where a = 1
        fn model(a: f64, x: f64) -> f64 {
            a * (-x).exp()
        }

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

        let result = solve_lm_external(residuals, vec![0.5], LMConfig::default()).unwrap();

        assert!(result.converged);
        assert!((result.params[0] - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_lm_external_already_optimal() {
        // Start at optimal point
        let residuals = |params: &[f64]| -> Vec<f64> { vec![params[0] - 5.0] };

        let result = solve_lm_external(residuals, vec![5.0], LMConfig::default()).unwrap();

        assert!(result.converged);
        assert!(result.residual_ss < 1e-10);
    }

    #[test]
    fn test_lm_external_empty_params() {
        let residuals = |_params: &[f64]| -> Vec<f64> { vec![1.0] };

        let result = solve_lm_external(residuals, vec![], LMConfig::default());

        assert!(result.is_err());
    }

    #[test]
    fn test_lm_external_multi_dimensional() {
        // Minimize sum of (p[i] - i)^2
        let residuals = |params: &[f64]| -> Vec<f64> {
            params
                .iter()
                .enumerate()
                .map(|(i, &p)| p - i as f64)
                .collect()
        };

        let result =
            solve_lm_external(residuals, vec![10.0, 10.0, 10.0, 10.0], LMConfig::default()).unwrap();

        assert!(result.converged);
        for (i, &p) in result.params.iter().enumerate() {
            assert!((p - i as f64).abs() < 1e-4);
        }
    }

    #[test]
    fn test_lm_external_api_compatibility() {
        // Verify external function has same API as internal one
        let residuals = |params: &[f64]| -> Vec<f64> { vec![params[0] - 1.0] };

        let result = solve_lm_external(residuals, vec![0.0], LMConfig::default());

        // Should succeed
        assert!(result.is_ok());

        // Should return LMResult
        let lm = result.unwrap();
        assert!(!lm.params.is_empty());
    }
}
