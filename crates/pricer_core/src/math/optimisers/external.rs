//! External optimisation library wrappers.
//!
//! This module provides wrappers around external optimisation crates (argmin)
//! to provide battle-tested implementations while maintaining API compatibility.
//!
//! # Feature Flag
//!
//! This module requires the `external-numerics` feature flag.

use argmin::core::{CostFunction, Executor, Gradient, State};
use argmin::solver::{
    linesearch::MoreThuenteLineSearch, neldermead::NelderMead as ArgminNelderMead,
    quasinewton::LBFGS as ArgminLBFGS,
};

use super::{LbfgsConfig, NelderMeadConfig, OptimisationError, OptimisationResult};

// =============================================================================
// Nelder-Mead Wrapper
// =============================================================================

/// Problem wrapper for argmin Nelder-Mead optimisation.
struct NelderMeadProblem<F> {
    objective: F,
}

impl<F> CostFunction for NelderMeadProblem<F>
where
    F: Fn(&[f64]) -> f64,
{
    type Param = Vec<f64>;
    type Output = f64;

    fn cost(&self, p: &Self::Param) -> Result<Self::Output, argmin::core::Error> {
        Ok((self.objective)(p))
    }
}

/// Minimise a function using argmin's Nelder-Mead implementation.
///
/// This wraps the external argmin crate's Nelder-Mead algorithm, providing
/// a battle-tested implementation while maintaining API compatibility with
/// the internal implementation.
///
/// # Arguments
///
/// * `f` - Objective function to minimise
/// * `x0` - Initial guess (starting point)
/// * `config` - Nelder-Mead configuration
///
/// # Returns
///
/// An `OptimisationResult` containing the optimal parameters and value.
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::optimisers::{minimize_nelder_mead_external, NelderMeadConfig};
///
/// let f = |x: &[f64]| x[0] * x[0] + x[1] * x[1];
/// let result = minimize_nelder_mead_external(f, &[1.0, 2.0], NelderMeadConfig::default()).unwrap();
/// assert!(result.converged);
/// ```
pub fn minimize_nelder_mead_external<F>(
    f: F,
    x0: &[f64],
    config: NelderMeadConfig,
) -> Result<OptimisationResult, OptimisationError>
where
    F: Fn(&[f64]) -> f64,
{
    let n = x0.len();
    if n == 0 {
        return Err(OptimisationError::InvalidInput(
            "Initial point must have at least one dimension".to_string(),
        ));
    }

    let problem = NelderMeadProblem { objective: f };

    // Build initial simplex (n+1 vertices)
    let mut simplex: Vec<Vec<f64>> = Vec::with_capacity(n + 1);
    simplex.push(x0.to_vec());

    for i in 0..n {
        let mut vertex = x0.to_vec();
        if vertex[i].abs() < 1e-10 {
            vertex[i] = config.initial_scale;
        } else {
            vertex[i] *= 1.0 + config.initial_scale;
        }
        simplex.push(vertex);
    }

    // Create argmin Nelder-Mead solver with custom coefficients
    let solver = ArgminNelderMead::new(simplex)
        .with_alpha(config.alpha)
        .map_err(|e| OptimisationError::External(e.to_string()))?
        .with_gamma(config.gamma)
        .map_err(|e| OptimisationError::External(e.to_string()))?
        .with_rho(config.rho)
        .map_err(|e| OptimisationError::External(e.to_string()))?
        .with_sigma(config.sigma)
        .map_err(|e| OptimisationError::External(e.to_string()))?;

    // Create and run executor
    let result = Executor::new(problem, solver)
        .configure(|state| {
            state
                .max_iters(config.base.max_iterations as u64)
                .target_cost(config.base.abs_tol)
        })
        .run()
        .map_err(|e| OptimisationError::External(e.to_string()))?;

    // Extract results
    let state = result.state();
    let best_param = state.get_best_param().ok_or_else(|| {
        OptimisationError::External("No best parameter found".to_string())
    })?;
    let best_cost = state.get_best_cost();
    let iterations = state.get_iter() as usize;

    // Check convergence
    let converged = best_cost < config.base.abs_tol || state.terminated();

    Ok(OptimisationResult::new(
        best_param.clone(),
        best_cost,
        iterations,
        iterations + n + 1, // Approximate function evaluations
        converged,
    ))
}

// =============================================================================
// L-BFGS Wrapper
// =============================================================================

/// Problem wrapper for argmin L-BFGS optimisation.
struct LbfgsProblem<F> {
    objective: F,
}

impl<F> CostFunction for LbfgsProblem<F>
where
    F: Fn(&[f64]) -> (f64, Vec<f64>),
{
    type Param = Vec<f64>;
    type Output = f64;

    fn cost(&self, p: &Self::Param) -> Result<Self::Output, argmin::core::Error> {
        let (val, _) = (self.objective)(p);
        Ok(val)
    }
}

impl<F> Gradient for LbfgsProblem<F>
where
    F: Fn(&[f64]) -> (f64, Vec<f64>),
{
    type Param = Vec<f64>;
    type Gradient = Vec<f64>;

    fn gradient(&self, p: &Self::Param) -> Result<Self::Gradient, argmin::core::Error> {
        let (_, grad) = (self.objective)(p);
        Ok(grad)
    }
}

/// Minimise a function using argmin's L-BFGS implementation.
///
/// This wraps the external argmin crate's L-BFGS algorithm, providing
/// a battle-tested implementation while maintaining API compatibility with
/// the internal implementation.
///
/// # Arguments
///
/// * `f` - Objective function returning (value, gradient)
/// * `x0` - Initial guess (starting point)
/// * `config` - L-BFGS configuration
///
/// # Returns
///
/// An `OptimisationResult` containing the optimal parameters and value.
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::optimisers::{minimize_lbfgs_external, LbfgsConfig};
///
/// let f = |x: &[f64]| {
///     let val = x[0] * x[0] + x[1] * x[1];
///     let grad = vec![2.0 * x[0], 2.0 * x[1]];
///     (val, grad)
/// };
/// let result = minimize_lbfgs_external(f, &[1.0, 2.0], LbfgsConfig::default()).unwrap();
/// assert!(result.converged);
/// ```
pub fn minimize_lbfgs_external<F>(
    f: F,
    x0: &[f64],
    config: LbfgsConfig,
) -> Result<OptimisationResult, OptimisationError>
where
    F: Fn(&[f64]) -> (f64, Vec<f64>),
{
    let n = x0.len();
    if n == 0 {
        return Err(OptimisationError::InvalidInput(
            "Initial point must have at least one dimension".to_string(),
        ));
    }

    let problem = LbfgsProblem { objective: f };
    let init_param = x0.to_vec();

    // Create line search
    let linesearch = MoreThuenteLineSearch::new().with_c(config.c1, 0.9).map_err(|e| {
        OptimisationError::External(format!("Line search setup failed: {}", e))
    })?;

    // Create argmin L-BFGS solver
    let solver = ArgminLBFGS::new(linesearch, config.m);

    // Create and run executor
    let result = Executor::new(problem, solver)
        .configure(|state| {
            state
                .param(init_param)
                .max_iters(config.base.max_iterations as u64)
                .target_cost(config.base.abs_tol)
        })
        .run()
        .map_err(|e| OptimisationError::External(e.to_string()))?;

    // Extract results
    let state = result.state();
    let best_param = state.get_best_param().ok_or_else(|| {
        OptimisationError::External("No best parameter found".to_string())
    })?;
    let best_cost = state.get_best_cost();
    let iterations = state.get_iter() as usize;
    // Get function evaluation count from the counts map
    let func_counts = state.get_func_counts();
    let func_evals = func_counts.values().sum::<u64>() as usize;

    // Check convergence
    let converged = best_cost < config.base.abs_tol || state.terminated();

    Ok(OptimisationResult::new(
        best_param.clone(),
        best_cost,
        iterations,
        func_evals,
        converged,
    ))
}

/// Minimise a function using L-BFGS with numerical gradient.
///
/// This convenience function computes gradients numerically using
/// central differences.
///
/// # Arguments
///
/// * `f` - Objective function (value only)
/// * `x0` - Initial guess
/// * `config` - L-BFGS configuration
/// * `h` - Step size for numerical gradient (default: 1e-8)
pub fn minimize_lbfgs_numerical_external<F>(
    f: F,
    x0: &[f64],
    config: LbfgsConfig,
    h: f64,
) -> Result<OptimisationResult, OptimisationError>
where
    F: Fn(&[f64]) -> f64,
{
    let n = x0.len();

    let f_with_grad = move |x: &[f64]| {
        let val = f(x);
        let mut grad = vec![0.0; n];

        for i in 0..n {
            let mut x_plus = x.to_vec();
            let mut x_minus = x.to_vec();
            x_plus[i] += h;
            x_minus[i] -= h;
            grad[i] = (f(&x_plus) - f(&x_minus)) / (2.0 * h);
        }

        (val, grad)
    };

    minimize_lbfgs_external(f_with_grad, x0, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Nelder-Mead External Tests
    // ==========================================================================

    #[test]
    fn test_nelder_mead_external_quadratic_1d() {
        // Minimise f(x) = x² -> minimum at x = 0
        let f = |x: &[f64]| x[0] * x[0];
        let config = NelderMeadConfig::default();
        let result = minimize_nelder_mead_external(f, &[5.0], config).unwrap();

        assert!(
            result.params[0].abs() < 1e-3,
            "Expected 0, got {}",
            result.params[0]
        );
        assert!(result.value < 1e-6);
    }

    #[test]
    fn test_nelder_mead_external_rosenbrock() {
        // Rosenbrock function: f(x,y) = (1-x)² + 100(y-x²)²
        // Minimum at (1, 1) with value 0
        let f = |x: &[f64]| {
            let a = 1.0 - x[0];
            let b = x[1] - x[0] * x[0];
            a * a + 100.0 * b * b
        };

        let mut config = NelderMeadConfig::default();
        config.base.max_iterations = 5000;
        config.base.abs_tol = 1e-8;

        let result = minimize_nelder_mead_external(f, &[0.0, 0.0], config).unwrap();

        // Nelder-Mead should get reasonably close
        assert!(
            (result.params[0] - 1.0).abs() < 0.1,
            "Expected x ≈ 1, got {}",
            result.params[0]
        );
    }

    #[test]
    fn test_nelder_mead_external_invalid_input() {
        let f = |_: &[f64]| 0.0;
        let config = NelderMeadConfig::default();
        let result = minimize_nelder_mead_external(f, &[], config);
        assert!(result.is_err());
    }

    // ==========================================================================
    // L-BFGS External Tests
    // ==========================================================================

    #[test]
    fn test_lbfgs_external_quadratic_1d() {
        let f = |x: &[f64]| {
            let val = x[0] * x[0];
            let grad = vec![2.0 * x[0]];
            (val, grad)
        };

        let config = LbfgsConfig::default();
        let result = minimize_lbfgs_external(f, &[5.0], config).unwrap();

        assert!(
            result.params[0].abs() < 1e-4,
            "Expected 0, got {}",
            result.params[0]
        );
        assert!(result.value < 1e-8);
    }

    #[test]
    fn test_lbfgs_external_quadratic_2d() {
        let f = |x: &[f64]| {
            let val = x[0] * x[0] + x[1] * x[1];
            let grad = vec![2.0 * x[0], 2.0 * x[1]];
            (val, grad)
        };

        let config = LbfgsConfig::default();
        let result = minimize_lbfgs_external(f, &[3.0, 4.0], config).unwrap();

        assert!(result.params[0].abs() < 1e-4);
        assert!(result.params[1].abs() < 1e-4);
    }

    #[test]
    fn test_lbfgs_external_numerical_gradient() {
        let f = |x: &[f64]| x[0] * x[0] + x[1] * x[1];

        let config = LbfgsConfig::default();
        let result = minimize_lbfgs_numerical_external(f, &[3.0, 4.0], config, 1e-8).unwrap();

        assert!(result.params[0].abs() < 1e-3);
        assert!(result.params[1].abs() < 1e-3);
    }

    #[test]
    fn test_lbfgs_external_invalid_input() {
        let f = |_: &[f64]| (0.0, vec![]);
        let config = LbfgsConfig::default();
        let result = minimize_lbfgs_external(f, &[], config);
        assert!(result.is_err());
    }

    // ==========================================================================
    // Comparison Tests
    // ==========================================================================

    #[test]
    fn test_external_matches_internal_api() {
        // Verify external functions have same API as internal ones
        let f_nm = |x: &[f64]| x[0] * x[0];
        let f_lbfgs = |x: &[f64]| (x[0] * x[0], vec![2.0 * x[0]]);

        let nm_result = minimize_nelder_mead_external(f_nm, &[1.0], NelderMeadConfig::default());
        let lbfgs_result = minimize_lbfgs_external(f_lbfgs, &[1.0], LbfgsConfig::default());

        // Both should succeed
        assert!(nm_result.is_ok());
        assert!(lbfgs_result.is_ok());

        // Both should return OptimisationResult
        let nm = nm_result.unwrap();
        let lbfgs = lbfgs_result.unwrap();

        assert!(!nm.params.is_empty());
        assert!(!lbfgs.params.is_empty());
    }
}
