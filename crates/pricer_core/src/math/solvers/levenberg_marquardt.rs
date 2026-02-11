//! Levenberg-Marquardt nonlinear least-squares solver.
//!
//! Thin wrapper around the [`levenberg_marquardt`](::levenberg_marquardt) crate,
//! preserving a closure-based API for easy use in calibration routines.

use levenberg_marquardt::{LeastSquaresProblem, LevenbergMarquardt};
use nalgebra::{DMatrix, DVector, Dyn, Owned};

use crate::types::SolverError;

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for Levenberg-Marquardt solver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LMConfig {
    pub tolerance: f64,
    pub max_iterations: usize,
    pub initial_lambda: f64,
    pub lambda_up: f64,
    pub lambda_down: f64,
    pub min_lambda: f64,
    pub max_lambda: f64,
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
    pub fn new(tolerance: f64, max_iterations: usize) -> Self {
        Self {
            tolerance,
            max_iterations,
            ..Default::default()
        }
    }

    pub fn fast() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 50,
            ..Default::default()
        }
    }

    pub fn high_precision() -> Self {
        Self {
            tolerance: 1e-14,
            max_iterations: 500,
            param_tolerance: 1e-14,
            ..Default::default()
        }
    }
}

// =============================================================================
// Result
// =============================================================================

/// Result of Levenberg-Marquardt optimisation.
#[derive(Debug, Clone, PartialEq)]
pub struct LMResult {
    pub params: Vec<f64>,
    pub residual_ss: f64,
    pub iterations: usize,
    pub converged: bool,
    pub final_lambda: f64,
}

impl LMResult {
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

    pub fn rmse(&self, n_observations: usize) -> f64 {
        if n_observations == 0 {
            return 0.0;
        }
        (self.residual_ss / n_observations as f64).sqrt()
    }
}

// =============================================================================
// Closure → LeastSquaresProblem adapter
// =============================================================================

struct ClosureProblem<F> {
    params: DVector<f64>,
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
        if r.is_empty() { None } else { Some(DVector::from_vec(r)) }
    }

    fn jacobian(&self) -> Option<DMatrix<f64>> {
        let params = self.params.as_slice();
        let r0 = (self.residuals_fn)(params);
        if r0.is_empty() || params.is_empty() {
            return None;
        }
        let eps = 1e-8;
        let mut jac = DMatrix::zeros(r0.len(), params.len());
        for j in 0..params.len() {
            let h = eps * params[j].abs().max(1.0);
            let mut pp = params.to_vec();
            pp[j] += h;
            let rp = (self.residuals_fn)(&pp);
            for i in 0..r0.len() {
                jac[(i, j)] = (rp[i] - r0[i]) / h;
            }
        }
        Some(jac)
    }
}

// =============================================================================
// Solver
// =============================================================================

/// Levenberg-Marquardt nonlinear least-squares solver.
#[derive(Debug, Clone)]
pub struct LevenbergMarquardtSolver {
    config: LMConfig,
}

impl LevenbergMarquardtSolver {
    pub fn new(config: LMConfig) -> Self { Self { config } }

    pub fn with_defaults() -> Self {
        Self {
            config: LMConfig::default(),
        }
    }

    pub fn config(&self) -> &LMConfig { &self.config }

    pub fn solve<F>(&self, residuals: F, initial_params: Vec<f64>) -> Result<LMResult, SolverError>
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        if initial_params.is_empty() {
            return Err(SolverError::NumericalInstability(
                "Empty parameter vector".into(),
            ));
        }

        let r0 = residuals(&initial_params);
        if r0.is_empty() {
            return Err(SolverError::NumericalInstability(
                "Empty residual vector".into(),
            ));
        }

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

        let problem = ClosureProblem {
            params: DVector::from_vec(initial_params),
            residuals_fn: residuals,
        };

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
        ) || final_ss.sqrt() < self.config.tolerance;

        Ok(LMResult::new(
            final_params,
            final_ss,
            report.number_of_evaluations,
            converged,
            0.0,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_presets() {
        let d = LMConfig::default();
        assert!((d.tolerance - 1e-10).abs() < 1e-15);
        assert_eq!(d.max_iterations, 100);

        let c = LMConfig::new(1e-8, 50);
        assert!((c.tolerance - 1e-8).abs() < 1e-15);
        assert_eq!(c.max_iterations, 50);

        let f = LMConfig::fast();
        assert!(f.tolerance > 1e-8 && f.max_iterations <= 50);

        let hp = LMConfig::high_precision();
        assert!(hp.tolerance < 1e-12 && hp.max_iterations >= 500);
    }

    #[test]
    fn test_result_rmse() {
        let r = LMResult::new(vec![1.0], 4.0, 10, true, 1e-5);
        assert!((r.rmse(4) - 1.0).abs() < 1e-10);
        assert_eq!(r.rmse(0), 0.0);
    }

    #[test]
    fn test_solve_simple_linear() {
        let solver = LevenbergMarquardtSolver::with_defaults();
        let res = solver
            .solve(|p: &[f64]| vec![p[0] - 2.0, p[1] - 3.0], vec![0.0, 0.0])
            .unwrap();
        assert!(res.converged);
        assert!((res.params[0] - 2.0).abs() < 1e-6);
        assert!((res.params[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_solve_quadratic() {
        let solver = LevenbergMarquardtSolver::with_defaults();
        let res = solver
            .solve(|p: &[f64]| vec![p[0] - 3.0], vec![10.0])
            .unwrap();
        assert!(res.converged);
        assert!((res.params[0] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_solve_rosenbrock() {
        let config = LMConfig { max_iterations: 200, ..Default::default() };
        let solver = LevenbergMarquardtSolver::new(config);
        let res = solver
            .solve(
                |p: &[f64]| vec![10.0 * (p[1] - p[0] * p[0]), 1.0 - p[0]],
                vec![0.0, 0.0],
            )
            .unwrap();
        assert!((res.params[0] - 1.0).abs() < 0.1 || res.residual_ss < 0.01);
    }

    #[test]
    fn test_solve_already_optimal() {
        let solver = LevenbergMarquardtSolver::with_defaults();
        let res = solver.solve(|p: &[f64]| vec![p[0] - 5.0], vec![5.0]).unwrap();
        assert!(res.converged && res.iterations <= 1);
    }

    #[test]
    fn test_solve_empty_params() {
        let solver = LevenbergMarquardtSolver::with_defaults();
        assert!(solver.solve(|_: &[f64]| vec![1.0], vec![]).is_err());
    }

    #[test]
    fn test_solve_multi_dimensional() {
        let solver = LevenbergMarquardtSolver::with_defaults();
        let res = solver
            .solve(
                |p: &[f64]| p.iter().enumerate().map(|(i, &v)| v - i as f64).collect(),
                vec![10.0, 10.0, 10.0, 10.0],
            )
            .unwrap();
        assert!(res.converged);
        for (i, &p) in res.params.iter().enumerate() {
            assert!((p - i as f64).abs() < 1e-6);
        }
    }

    #[test]
    fn test_clone_debug() {
        let s1 = LevenbergMarquardtSolver::with_defaults();
        let s2 = s1.clone();
        assert_eq!(s1.config().max_iterations, s2.config().max_iterations);
        assert!(format!("{:?}", s1).contains("LevenbergMarquardtSolver"));

        let c1 = LMConfig::default();
        assert_eq!(c1, c1.clone());

        let r1 = LMResult::new(vec![1.0], 0.01, 10, true, 1e-5);
        assert_eq!(r1, r1.clone());
    }
}
