//! Levenberg-Marquardt algorithm for nonlinear least squares.

use crate::{error::OptimiserError, solvers::OptimisationResult};

/// Configuration for Levenberg-Marquardt solver.
#[derive(Debug, Clone)]
pub struct LevenbergMarquardtConfig {
    /// Maximum iterations
    pub max_iterations: usize,
    /// Convergence tolerance for gradient norm
    pub gradient_tolerance: f64,
    /// Convergence tolerance for parameter change
    pub parameter_tolerance: f64,
    /// Convergence tolerance for residual change
    pub residual_tolerance: f64,
    /// Initial damping parameter (λ)
    pub initial_lambda: f64,
    /// Damping increase factor
    pub lambda_increase: f64,
    /// Damping decrease factor
    pub lambda_decrease: f64,
    /// Finite difference step size
    pub fd_step: f64,
}

impl Default for LevenbergMarquardtConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            gradient_tolerance: 1e-8,
            parameter_tolerance: 1e-8,
            residual_tolerance: 1e-8,
            initial_lambda: 1e-3,
            lambda_increase: 10.0,
            lambda_decrease: 0.1,
            fd_step: 1e-6,
        }
    }
}

/// Levenberg-Marquardt solver for nonlinear least squares problems.
///
/// Minimises the sum of squared residuals: Σ r_i(x)²
pub struct LevenbergMarquardt {
    config: LevenbergMarquardtConfig,
}

impl LevenbergMarquardt {
    /// Create a new solver with default configuration.
    pub fn new() -> Self {
        Self {
            config: LevenbergMarquardtConfig::default(),
        }
    }

    /// Create a new solver with custom configuration.
    pub fn with_config(config: LevenbergMarquardtConfig) -> Self { Self { config } }

    /// Solve a nonlinear least squares problem.
    ///
    /// # Arguments
    ///
    /// * `initial` - Initial parameter guess
    /// * `residuals` - Function that computes residual vector given parameters
    ///
    /// # Returns
    ///
    /// An `OptimisationResult` containing the optimal parameters.
    #[allow(clippy::needless_range_loop)]
    pub fn solve<F>(
        &self,
        initial: &[f64],
        residuals: F,
    ) -> Result<OptimisationResult, OptimiserError>
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        let n = initial.len();
        let mut x = initial.to_vec();
        let mut lambda = self.config.initial_lambda;
        let mut func_evals = 0;

        for iteration in 0..self.config.max_iterations {
            // Compute residuals and Jacobian
            let r = residuals(&x);
            func_evals += 1;
            let m = r.len();

            // Compute sum of squared residuals
            let ssr: f64 = r.iter().map(|ri| ri * ri).sum();

            // Check for convergence
            if ssr < self.config.residual_tolerance {
                return Ok(OptimisationResult {
                    parameters: x,
                    objective: ssr,
                    iterations: iteration,
                    function_evaluations: func_evals,
                    converged: true,
                });
            }

            // Compute Jacobian via finite differences
            let mut j = vec![vec![0.0; n]; m];
            for k in 0..n {
                let mut x_plus = x.clone();
                x_plus[k] += self.config.fd_step;
                let r_plus = residuals(&x_plus);
                func_evals += 1;

                for i in 0..m {
                    j[i][k] = (r_plus[i] - r[i]) / self.config.fd_step;
                }
            }

            // Compute J^T * J and J^T * r
            let mut jtj = vec![vec![0.0; n]; n];
            let mut jtr = vec![0.0; n];

            for i in 0..n {
                for k in 0..n {
                    for l in 0..m {
                        jtj[i][k] += j[l][i] * j[l][k];
                    }
                }
                for l in 0..m {
                    jtr[i] += j[l][i] * r[l];
                }
            }

            // Add damping: (J^T J + λI) δ = -J^T r
            for i in 0..n {
                jtj[i][i] += lambda;
            }

            // Solve for step (simple Gaussian elimination for small systems)
            let step = solve_linear_system(&jtj, &jtr.iter().map(|v| -v).collect::<Vec<_>>())?;

            // Check step size
            let step_norm: f64 = step.iter().map(|s| s * s).sum::<f64>().sqrt();
            if step_norm < self.config.parameter_tolerance {
                return Ok(OptimisationResult {
                    parameters: x,
                    objective: ssr,
                    iterations: iteration,
                    function_evaluations: func_evals,
                    converged: true,
                });
            }

            // Trial step
            let x_new: Vec<f64> = x.iter().zip(step.iter()).map(|(xi, si)| xi + si).collect();
            let r_new = residuals(&x_new);
            func_evals += 1;
            let ssr_new: f64 = r_new.iter().map(|ri| ri * ri).sum();

            // Accept or reject step
            if ssr_new < ssr {
                x = x_new;
                lambda *= self.config.lambda_decrease;
            } else {
                lambda *= self.config.lambda_increase;
            }
        }

        Err(OptimiserError::ConvergenceFailure {
            iterations: self.config.max_iterations,
            residual: residuals(&x).iter().map(|r| r * r).sum(),
        })
    }
}

impl Default for LevenbergMarquardt {
    fn default() -> Self { Self::new() }
}

/// Simple linear system solver (Gaussian elimination with partial pivoting).
fn solve_linear_system(a: &[Vec<f64>], b: &[f64]) -> Result<Vec<f64>, OptimiserError> {
    let n = b.len();
    if a.len() != n || a.iter().any(|row| row.len() != n) {
        return Err(OptimiserError::InvalidBounds(
            "Matrix dimension mismatch".to_string(),
        ));
    }

    // Create augmented matrix
    let mut aug: Vec<Vec<f64>> = a
        .iter()
        .zip(b.iter())
        .map(|(row, bi)| {
            let mut new_row = row.clone();
            new_row.push(*bi);
            new_row
        })
        .collect();

    // Forward elimination with partial pivoting
    for i in 0..n {
        // Find pivot
        let mut max_row = i;
        for k in (i + 1)..n {
            if aug[k][i].abs() > aug[max_row][i].abs() {
                max_row = k;
            }
        }
        aug.swap(i, max_row);

        if aug[i][i].abs() < 1e-12 {
            return Err(OptimiserError::SingularMatrix);
        }

        // Eliminate column
        for k in (i + 1)..n {
            let factor = aug[k][i] / aug[i][i];
            let aug_i_slice: Vec<f64> = aug[i][i..=n].to_vec();
            for (aug_k_j, aug_i_j) in aug[k][i..=n].iter_mut().zip(aug_i_slice.iter()) {
                *aug_k_j -= factor * aug_i_j;
            }
        }
    }

    // Back substitution
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        x[i] = aug[i][n];
        for j in (i + 1)..n {
            x[i] -= aug[i][j] * x[j];
        }
        x[i] /= aug[i][i];
    }

    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_system() {
        // Solve 2x + y = 5, x + 3y = 5 => x = 2, y = 1
        let a = vec![vec![2.0, 1.0], vec![1.0, 3.0]];
        let b = vec![5.0, 5.0];
        let x = solve_linear_system(&a, &b).unwrap();
        assert!((x[0] - 2.0).abs() < 1e-10);
        assert!((x[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_rosenbrock() {
        let lm = LevenbergMarquardt::new();

        // Minimise (1-x)² + 100(y-x²)² starting from (-1, 1)
        // Optimal at (1, 1)
        let residuals = |p: &[f64]| vec![1.0 - p[0], 10.0 * (p[1] - p[0] * p[0])];

        let result = lm.solve(&[-1.0, 1.0], residuals);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!((result.parameters[0] - 1.0).abs() < 0.1);
        assert!((result.parameters[1] - 1.0).abs() < 0.1);
    }
}
