//! BFGS algorithm for unconstrained optimisation.

use crate::{error::OptimiserError, solvers::OptimisationResult};

/// Configuration for BFGS solver.
#[derive(Debug, Clone)]
pub struct BfgsConfig {
    /// Maximum iterations
    pub max_iterations: usize,
    /// Convergence tolerance for gradient norm
    pub gradient_tolerance: f64,
    /// Convergence tolerance for objective change
    pub objective_tolerance: f64,
    /// Finite difference step size
    pub fd_step: f64,
    /// Line search parameters (Armijo condition)
    pub c1: f64,
    /// Maximum line search iterations
    pub max_line_search: usize,
}

impl Default for BfgsConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            gradient_tolerance: 1e-6,
            objective_tolerance: 1e-10,
            fd_step: 1e-6,
            c1: 1e-4,
            max_line_search: 20,
        }
    }
}

/// BFGS quasi-Newton solver for unconstrained optimisation.
///
/// Uses the Broyden-Fletcher-Goldfarb-Shanno update for the inverse Hessian
/// approximation.
pub struct Bfgs {
    config: BfgsConfig,
}

impl Bfgs {
    /// Create a new solver with default configuration.
    pub fn new() -> Self {
        Self {
            config: BfgsConfig::default(),
        }
    }

    /// Create a new solver with custom configuration.
    pub fn with_config(config: BfgsConfig) -> Self { Self { config } }

    /// Minimise an objective function.
    ///
    /// # Arguments
    ///
    /// * `initial` - Initial parameter guess
    /// * `objective` - Objective function to minimise
    ///
    /// # Returns
    ///
    /// An `OptimisationResult` containing the optimal parameters.
    #[allow(clippy::needless_range_loop)]
    pub fn minimise<F>(
        &self,
        initial: &[f64],
        objective: F,
    ) -> Result<OptimisationResult, OptimiserError>
    where
        F: Fn(&[f64]) -> f64,
    {
        let n = initial.len();
        let mut x = initial.to_vec();
        let mut func_evals = 0;

        // Initialise inverse Hessian as identity
        let mut h_inv = vec![vec![0.0; n]; n];
        for i in 0..n {
            h_inv[i][i] = 1.0;
        }

        let mut f = objective(&x);
        func_evals += 1;
        let mut g = self.gradient(&x, &objective, &mut func_evals);

        for iteration in 0..self.config.max_iterations {
            // Check gradient convergence
            let g_norm: f64 = g.iter().map(|gi| gi * gi).sum::<f64>().sqrt();
            if g_norm < self.config.gradient_tolerance {
                return Ok(OptimisationResult {
                    parameters: x,
                    objective: f,
                    iterations: iteration,
                    function_evaluations: func_evals,
                    converged: true,
                });
            }

            // Compute search direction: p = -H^{-1} * g
            let mut p = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    p[i] -= h_inv[i][j] * g[j];
                }
            }

            // Line search (backtracking)
            let mut alpha = 1.0;
            let f0 = f;
            let slope: f64 = p.iter().zip(g.iter()).map(|(pi, gi)| pi * gi).sum();

            for _ in 0..self.config.max_line_search {
                let x_new: Vec<f64> = x
                    .iter()
                    .zip(p.iter())
                    .map(|(xi, pi)| xi + alpha * pi)
                    .collect();
                let f_new = objective(&x_new);
                func_evals += 1;

                if f_new <= f0 + self.config.c1 * alpha * slope {
                    // Accept step
                    let s: Vec<f64> = p.iter().map(|pi| alpha * pi).collect();
                    let g_new = self.gradient(&x_new, &objective, &mut func_evals);
                    let y: Vec<f64> = g_new.iter().zip(g.iter()).map(|(gn, go)| gn - go).collect();

                    // Check objective convergence
                    if (f - f_new).abs() < self.config.objective_tolerance {
                        return Ok(OptimisationResult {
                            parameters: x_new,
                            objective: f_new,
                            iterations: iteration,
                            function_evaluations: func_evals,
                            converged: true,
                        });
                    }

                    // BFGS update
                    let sy: f64 = s.iter().zip(y.iter()).map(|(si, yi)| si * yi).sum();
                    if sy > 1e-12 {
                        self.bfgs_update(&mut h_inv, &s, &y, sy);
                    }

                    x = x_new;
                    f = f_new;
                    g = g_new;
                    break;
                }

                alpha *= 0.5;
            }
        }

        Err(OptimiserError::ConvergenceFailure {
            iterations: self.config.max_iterations,
            residual: f,
        })
    }

    /// Compute gradient via finite differences.
    #[allow(clippy::needless_range_loop)]
    fn gradient<F>(&self, x: &[f64], objective: &F, func_evals: &mut usize) -> Vec<f64>
    where
        F: Fn(&[f64]) -> f64,
    {
        let n = x.len();
        let mut grad = vec![0.0; n];
        let f0 = objective(x);
        *func_evals += 1;

        for i in 0..n {
            let mut x_plus = x.to_vec();
            x_plus[i] += self.config.fd_step;
            let f_plus = objective(&x_plus);
            *func_evals += 1;
            grad[i] = (f_plus - f0) / self.config.fd_step;
        }

        grad
    }

    /// BFGS inverse Hessian update.
    #[allow(clippy::needless_range_loop)]
    fn bfgs_update(&self, h_inv: &mut [Vec<f64>], s: &[f64], y: &[f64], sy: f64) {
        let n = s.len();
        let rho = 1.0 / sy;

        // Compute H_new = (I - rho * s * y^T) * H * (I - rho * y * s^T) + rho * s * s^T
        let mut hy = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                hy[i] += h_inv[i][j] * y[j];
            }
        }

        let yhy: f64 = y.iter().zip(hy.iter()).map(|(yi, hyi)| yi * hyi).sum();

        for i in 0..n {
            for j in 0..n {
                h_inv[i][j] +=
                    rho * ((1.0 + rho * yhy) * s[i] * s[j] - hy[i] * s[j] - s[i] * hy[j]);
            }
        }
    }
}

impl Default for Bfgs {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadratic() {
        let bfgs = Bfgs::new();

        // Minimise (x-2)² + (y-3)² starting from (0, 0)
        let objective = |p: &[f64]| (p[0] - 2.0).powi(2) + (p[1] - 3.0).powi(2);

        let result = bfgs.minimise(&[0.0, 0.0], objective);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!((result.parameters[0] - 2.0).abs() < 0.01);
        assert!((result.parameters[1] - 3.0).abs() < 0.01);
    }
}
