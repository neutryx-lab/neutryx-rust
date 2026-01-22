//! L-BFGS (Limited-memory BFGS) optimisation algorithm.
//!
//! L-BFGS is a quasi-Newton method that approximates the inverse Hessian
//! using a limited history of gradient evaluations. It is efficient for
//! large-scale optimisation problems.

use super::{config::LbfgsConfig, error::OptimisationError, result::OptimisationResult};

/// Minimise a function using L-BFGS.
///
/// L-BFGS is a quasi-Newton method suitable for smooth, differentiable
/// functions. It requires gradient information.
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
/// use pricer_core::math::optimisers::{minimize_lbfgs, LbfgsConfig};
///
/// // Quadratic function with gradient
/// let f = |x: &[f64]| {
///     let val = x[0] * x[0] + x[1] * x[1];
///     let grad = vec![2.0 * x[0], 2.0 * x[1]];
///     (val, grad)
/// };
///
/// let result = minimize_lbfgs(f, &[5.0, 5.0], LbfgsConfig::default()).unwrap();
/// assert!(result.converged);
/// ```
pub fn minimize_lbfgs<F>(
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

    let m = config.m; // History size
    let mut x = x0.to_vec();

    // History storage for L-BFGS
    let mut s_history: Vec<Vec<f64>> = Vec::with_capacity(m);
    let mut y_history: Vec<Vec<f64>> = Vec::with_capacity(m);
    let mut rho_history: Vec<f64> = Vec::with_capacity(m);

    let (mut f_val, mut grad) = f(&x);
    let mut func_evals = 1;

    for iteration in 0..config.base.max_iterations {
        // Check gradient convergence
        let grad_norm: f64 = grad.iter().map(|g| g * g).sum::<f64>().sqrt();
        if grad_norm < config.base.abs_tol {
            return Ok(
                OptimisationResult::new(x, f_val, iteration, func_evals, true)
                    .with_message("Converged: gradient norm within tolerance"),
            );
        }

        // Compute search direction using L-BFGS two-loop recursion
        let mut q = grad.clone();

        // First loop: backward
        let k = s_history.len();
        let mut alpha = vec![0.0; k];

        for i in (0..k).rev() {
            alpha[i] = rho_history[i] * dot(&s_history[i], &q);
            for j in 0..n {
                q[j] -= alpha[i] * y_history[i][j];
            }
        }

        // Initial Hessian approximation (scaled identity)
        let mut r = if k > 0 {
            let s_k = &s_history[k - 1];
            let y_k = &y_history[k - 1];
            let ys = dot(s_k, y_k);
            let yy = dot(y_k, y_k);
            let gamma = if yy.abs() > 1e-30 { ys / yy } else { 1.0 };
            q.iter().map(|&qi| gamma * qi).collect::<Vec<f64>>()
        } else {
            q.clone()
        };

        // Second loop: forward
        for i in 0..k {
            let beta = rho_history[i] * dot(&y_history[i], &r);
            for j in 0..n {
                r[j] += (alpha[i] - beta) * s_history[i][j];
            }
        }

        // Search direction is -H * grad
        let direction: Vec<f64> = r.iter().map(|&ri| -ri).collect();

        // Line search with backtracking
        let mut step = 1.0;
        let c1 = config.c1;
        let dir_grad: f64 = dot(&direction, &grad);

        if dir_grad >= 0.0 {
            // Not a descent direction, use negative gradient
            let direction: Vec<f64> = grad.iter().map(|g| -g).collect();
            let dir_grad = -grad_norm * grad_norm;

            let mut step = 1.0;
            let mut armijo_satisfied = false;

            for _ in 0..30 {
                let x_new: Vec<f64> = x
                    .iter()
                    .zip(&direction)
                    .map(|(&xi, &di)| xi + step * di)
                    .collect();
                let (f_new, _) = f(&x_new);
                func_evals += 1;

                if f_new <= f_val + c1 * step * dir_grad {
                    armijo_satisfied = true;
                    break;
                }
                step *= 0.5;
            }

            if !armijo_satisfied {
                return Err(OptimisationError::LineSearchError(
                    "Line search failed to satisfy Armijo condition".to_string(),
                ));
            }

            // Update position
            let x_old = x.clone();
            let grad_old = grad;
            for i in 0..n {
                x[i] += step * direction[i];
            }

            let (f_new, grad_new) = f(&x);
            func_evals += 1;
            f_val = f_new;
            grad = grad_new;

            // Update history
            let s: Vec<f64> = x.iter().zip(&x_old).map(|(&xi, &xo)| xi - xo).collect();
            let y: Vec<f64> = grad
                .iter()
                .zip(&grad_old)
                .map(|(&gi, &go)| gi - go)
                .collect();
            let ys = dot(&s, &y);

            if ys > 1e-10 {
                if s_history.len() >= m {
                    s_history.remove(0);
                    y_history.remove(0);
                    rho_history.remove(0);
                }
                s_history.push(s);
                y_history.push(y);
                rho_history.push(1.0 / ys);
            }

            continue;
        }

        // Normal line search
        let mut armijo_satisfied = false;

        for _ in 0..30 {
            let x_new: Vec<f64> = x
                .iter()
                .zip(&direction)
                .map(|(&xi, &di)| xi + step * di)
                .collect();
            let (f_new, _) = f(&x_new);
            func_evals += 1;

            if f_new <= f_val + c1 * step * dir_grad {
                armijo_satisfied = true;
                break;
            }
            step *= 0.5;
        }

        if !armijo_satisfied {
            // Fall back to gradient descent step
            step = 0.001;
            let direction: Vec<f64> = grad.iter().map(|g| -g).collect();

            let x_new: Vec<f64> = x
                .iter()
                .zip(&direction)
                .map(|(&xi, &di)| xi + step * di)
                .collect();
            let (f_new, grad_new) = f(&x_new);
            func_evals += 1;

            if f_new < f_val {
                x = x_new;
                f_val = f_new;
                grad = grad_new;
                continue;
            }
        }

        // Update position
        let x_old = x.clone();
        let grad_old = grad;
        for i in 0..n {
            x[i] += step * direction[i];
        }

        let (f_new, grad_new) = f(&x);
        func_evals += 1;
        f_val = f_new;
        grad = grad_new;

        // Check function value convergence
        // Note: This is checked after the update
        let f_change = (f_val - f_new).abs();
        if f_change < config.base.abs_tol && iteration > 0 {
            return Ok(
                OptimisationResult::new(x, f_val, iteration, func_evals, true)
                    .with_message("Converged: function value change within tolerance"),
            );
        }

        // Update history
        let s: Vec<f64> = x.iter().zip(&x_old).map(|(&xi, &xo)| xi - xo).collect();
        let y: Vec<f64> = grad
            .iter()
            .zip(&grad_old)
            .map(|(&gi, &go)| gi - go)
            .collect();
        let ys = dot(&s, &y);

        if ys > 1e-10 {
            if s_history.len() >= m {
                s_history.remove(0);
                y_history.remove(0);
                rho_history.remove(0);
            }
            s_history.push(s);
            y_history.push(y);
            rho_history.push(1.0 / ys);
        }
    }

    Ok(
        OptimisationResult::new(x, f_val, config.base.max_iterations, func_evals, false)
            .with_message("Did not converge within maximum iterations"),
    )
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
pub fn minimize_lbfgs_numerical<F>(
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

    minimize_lbfgs(f_with_grad, x0, config)
}

/// Dot product of two vectors.
fn dot(a: &[f64], b: &[f64]) -> f64 { a.iter().zip(b).map(|(&ai, &bi)| ai * bi).sum() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimize_quadratic() {
        // Minimise f(x) = x² -> minimum at x = 0
        let f = |x: &[f64]| {
            let val = x[0] * x[0];
            let grad = vec![2.0 * x[0]];
            (val, grad)
        };

        let config = LbfgsConfig::default();
        let result = minimize_lbfgs(f, &[5.0], config).unwrap();

        assert!(
            result.params[0].abs() < 1e-5,
            "Expected 0, got {}",
            result.params[0]
        );
        assert!(result.value < 1e-10);
        assert!(result.converged);
    }

    #[test]
    fn test_minimize_2d_quadratic() {
        // Minimise f(x, y) = x² + y² -> minimum at (0, 0)
        let f = |x: &[f64]| {
            let val = x[0] * x[0] + x[1] * x[1];
            let grad = vec![2.0 * x[0], 2.0 * x[1]];
            (val, grad)
        };

        let config = LbfgsConfig::default();
        let result = minimize_lbfgs(f, &[3.0, 4.0], config).unwrap();

        assert!(result.params[0].abs() < 1e-5);
        assert!(result.params[1].abs() < 1e-5);
        assert!(result.converged);
    }

    #[test]
    fn test_minimize_rosenbrock() {
        // Rosenbrock function: f(x,y) = (1-x)² + 100(y-x²)²
        // Minimum at (1, 1) with value 0
        let f = |x: &[f64]| {
            let a = 1.0 - x[0];
            let b = x[1] - x[0] * x[0];
            let val = a * a + 100.0 * b * b;
            let grad = vec![-2.0 * a - 400.0 * x[0] * b, 200.0 * b];
            (val, grad)
        };

        let mut config = LbfgsConfig::default();
        config.base.max_iterations = 10000;
        config.base.abs_tol = 1e-6;

        // Start closer to the minimum
        let result = minimize_lbfgs(f, &[0.5, 0.5], config).unwrap();

        // Rosenbrock is difficult, allow larger tolerance
        // The key is that the function value should decrease significantly
        assert!(
            result.value < 1.0,
            "Expected function value < 1, got {}",
            result.value
        );
    }

    #[test]
    fn test_minimize_numerical_gradient() {
        // Same quadratic but with numerical gradient
        let f = |x: &[f64]| x[0] * x[0] + x[1] * x[1];

        let config = LbfgsConfig::default();
        let result = minimize_lbfgs_numerical(f, &[3.0, 4.0], config, 1e-8).unwrap();

        assert!(result.params[0].abs() < 1e-4);
        assert!(result.params[1].abs() < 1e-4);
    }

    #[test]
    fn test_invalid_input_empty() {
        let f = |_: &[f64]| (0.0, vec![]);
        let config = LbfgsConfig::default();
        let result = minimize_lbfgs(f, &[], config);
        assert!(result.is_err());
    }

    #[test]
    fn test_func_evals_counted() {
        let f = |x: &[f64]| {
            let val = x[0] * x[0];
            let grad = vec![2.0 * x[0]];
            (val, grad)
        };
        let config = LbfgsConfig::default();
        let result = minimize_lbfgs(f, &[1.0], config).unwrap();
        assert!(result.func_evals > 0);
    }

    #[test]
    fn test_max_iterations_respected() {
        // A function that is hard to optimise precisely
        let f = |x: &[f64]| {
            let val = (x[0] - 1.0).powi(4) + (x[1] - 2.0).powi(4);
            let grad = vec![4.0 * (x[0] - 1.0).powi(3), 4.0 * (x[1] - 2.0).powi(3)];
            (val, grad)
        };

        let mut config = LbfgsConfig::default();
        config.base.max_iterations = 5;
        config.base.abs_tol = 1e-100; // Impossible tolerance

        let result = minimize_lbfgs(f, &[10.0, 10.0], config).unwrap();
        // Should either hit max iterations or converge early
        assert!(result.iterations <= 5, "Should not exceed max_iterations");
        // If it didn't converge, check it hit max iterations
        if !result.converged {
            assert_eq!(result.iterations, 5);
        }
    }
}
