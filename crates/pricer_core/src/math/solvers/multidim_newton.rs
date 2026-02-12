//! Multi-dimensional Newton-Raphson solver for systems of equations.
//!
//! Solves F(x) = 0 where F: R^n -> R^n, with optional Jacobian inverse
//! storage for the implicit function theorem (AAD).
//!
//! For `f64`, an alternative
//! [`solve_argmin`](MultidimensionalNewtonSolver::solve_argmin)
//! method delegates to argmin's Newton solver internally.

use nalgebra::{DMatrix, DVector, RealField};
use num_traits::Float;

use crate::{math::numeric::from_f64, types::SolverError};

/// Trait for defining a system of nonlinear equations F(x) = 0.
///
/// A default numerical Jacobian is provided via finite differences.
pub trait SystemOfEquations<T: RealField + Copy + Float> {
    /// Dimension of the system (number of equations/unknowns).
    fn dimension(&self) -> usize;

    /// Evaluate the residual vector F(x).
    fn evaluate(&self, x: &DVector<T>) -> Result<DVector<T>, SolverError>;

    /// Compute the Jacobian J(x) = dF/dx. Defaults to numerical approximation.
    fn jacobian(&self, x: &DVector<T>) -> Result<DMatrix<T>, SolverError> {
        self.jacobian_numerical(x, from_f64(1e-8))
    }

    /// Numerical Jacobian via forward finite differences.
    fn jacobian_numerical(&self, x: &DVector<T>, epsilon: T) -> Result<DMatrix<T>, SolverError> {
        let n = self.dimension();
        let f0 = self.evaluate(x)?;
        let mut jacobian = DMatrix::zeros(n, n);
        let mut x_perturbed = x.clone();

        for j in 0..n {
            let x_j_original = x_perturbed[j];
            x_perturbed[j] = x_j_original + epsilon;
            let f_perturbed = self.evaluate(&x_perturbed)?;
            for i in 0..n {
                jacobian[(i, j)] = (f_perturbed[i] - f0[i]) / epsilon;
            }
            x_perturbed[j] = x_j_original;
        }
        Ok(jacobian)
    }
}

/// Configuration for the multi-dimensional Newton-Raphson solver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultidimNewtonConfig<T: RealField + Copy> {
    /// Convergence tolerance for residual norm.
    pub tolerance: T,
    /// Convergence tolerance for parameter change.
    pub param_tolerance: T,
    /// Maximum number of iterations.
    pub max_iterations: usize,
    /// Step size for numerical Jacobian approximation.
    pub jacobian_epsilon: T,
    /// Store J^{-1} at solution for implicit function theorem (AAD).
    pub store_jacobian_inverse: bool,
}

impl<T: RealField + Copy + Float> Default for MultidimNewtonConfig<T> {
    fn default() -> Self {
        Self {
            tolerance: from_f64(1e-10),
            param_tolerance: from_f64(1e-10),
            max_iterations: 100,
            jacobian_epsilon: from_f64(1e-8),
            store_jacobian_inverse: true,
        }
    }
}

impl<T: RealField + Copy + Float> MultidimNewtonConfig<T> {
    /// Create configuration with specified tolerances.
    pub fn new(tolerance: T, max_iterations: usize) -> Self {
        Self {
            tolerance,
            param_tolerance: tolerance,
            max_iterations,
            ..Self::default()
        }
    }

    /// High-precision configuration (tol=1e-14, 500 iters).
    pub fn high_precision() -> Self {
        Self {
            tolerance: from_f64(1e-14),
            param_tolerance: from_f64(1e-14),
            max_iterations: 500,
            jacobian_epsilon: from_f64(1e-10),
            store_jacobian_inverse: true,
        }
    }

    /// Fast configuration with relaxed tolerances (tol=1e-6, 50 iters).
    pub fn fast() -> Self {
        Self {
            tolerance: from_f64(1e-6),
            param_tolerance: from_f64(1e-6),
            max_iterations: 50,
            jacobian_epsilon: from_f64(1e-6),
            store_jacobian_inverse: false,
        }
    }

    /// Enable or disable Jacobian inverse storage for AAD.
    pub fn with_jacobian_inverse(mut self, store: bool) -> Self {
        self.store_jacobian_inverse = store;
        self
    }
}

/// Result from the multi-dimensional Newton-Raphson solver.
///
/// When `store_jacobian_inverse` is enabled, contains J^{-1} at the solution
/// for computing sensitivities: dx*/dm = -J^{-1} * dF/dm.
#[derive(Debug, Clone)]
pub struct MultidimSolverResult<T: RealField + Copy> {
    /// Solution vector x* satisfying F(x*) ~ 0.
    pub solution: DVector<T>,
    /// Euclidean norm of the final residual.
    pub residual_norm: T,
    /// Number of iterations performed.
    pub iterations: usize,
    /// Whether the solver converged within tolerance.
    pub converged: bool,
    /// J^{-1} at the solution point (for AAD implicit function theorem).
    pub jacobian_inverse: Option<DMatrix<T>>,
}

impl<T: RealField + Copy> MultidimSolverResult<T> {
    /// Create a new solver result.
    pub fn new(
        solution: DVector<T>,
        residual_norm: T,
        iterations: usize,
        converged: bool,
        jacobian_inverse: Option<DMatrix<T>>,
    ) -> Self {
        Self {
            solution,
            residual_norm,
            iterations,
            converged,
            jacobian_inverse,
        }
    }

    /// Check if the Jacobian inverse is available.
    pub fn has_jacobian_inverse(&self) -> bool { self.jacobian_inverse.is_some() }

    /// Get the Jacobian inverse, returning an error if not available.
    pub fn jacobian_inverse_or_err(&self) -> Result<&DMatrix<T>, SolverError> {
        self.jacobian_inverse.as_ref().ok_or_else(|| {
            SolverError::NumericalInstability("Jacobian inverse not available".into())
        })
    }
}

/// Newtype wrappers for dynamic nalgebra types, allowing us to implement
/// the argmin-math traits required by `argmin::solver::newton::Newton`.
mod argmin_bridge {
    use std::ops::Deref;

    use argmin_math::{ArgminDot, ArgminInv, ArgminScaledSub, ArgminSub};
    use nalgebra::{DMatrix, DVector};

    /// Newtype around `DVector<f64>` for argmin trait bridge.
    #[derive(Debug, Clone)]
    pub struct DVec(pub DVector<f64>);

    impl Deref for DVec {
        type Target = DVector<f64>;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    /// Newtype around `DMatrix<f64>` for argmin trait bridge.
    #[derive(Debug, Clone)]
    pub struct DMat(pub DMatrix<f64>);

    impl ArgminInv<DMat> for DMat {
        fn inv(&self) -> Result<DMat, argmin::core::Error> {
            self.0
                .clone()
                .try_inverse()
                .map(DMat)
                .ok_or_else(|| argmin::core::Error::msg("Singular matrix"))
        }
    }

    impl ArgminDot<DVec, DVec> for DMat {
        fn dot(&self, rhs: &DVec) -> DVec { DVec(&self.0 * &rhs.0) }
    }

    impl ArgminScaledSub<DVec, f64, DVec> for DVec {
        fn scaled_sub(&self, scale: &f64, rhs: &DVec) -> DVec { DVec(&self.0 - *scale * &rhs.0) }
    }

    impl ArgminSub<DVec, DVec> for DVec {
        fn sub(&self, rhs: &DVec) -> DVec { DVec(&self.0 - &rhs.0) }
    }
}

use argmin_bridge::{DMat, DVec};

/// Adapter that presents F(x)=0 for argmin's Newton solver.
/// "Gradient" = F(x), "Hessian" = J(x), giving x -= J^{-1}*F(x).
struct ArgminSystemAdapter<'a, S> {
    system: &'a S,
}

impl<S: SystemOfEquations<f64>> argmin::core::CostFunction for ArgminSystemAdapter<'_, S> {
    type Param = DVec;
    type Output = f64;

    fn cost(&self, x: &Self::Param) -> Result<f64, argmin::core::Error> {
        let f = self
            .system
            .evaluate(&x.0)
            .map_err(|e| argmin::core::Error::msg(e.to_string()))?;
        Ok(f.norm())
    }
}

impl<S: SystemOfEquations<f64>> argmin::core::Gradient for ArgminSystemAdapter<'_, S> {
    type Param = DVec;
    type Gradient = DVec;

    fn gradient(&self, x: &Self::Param) -> Result<Self::Gradient, argmin::core::Error> {
        self.system
            .evaluate(&x.0)
            .map(DVec)
            .map_err(|e| argmin::core::Error::msg(e.to_string()))
    }
}

impl<S: SystemOfEquations<f64>> argmin::core::Hessian for ArgminSystemAdapter<'_, S> {
    type Param = DVec;
    type Hessian = DMat;

    fn hessian(&self, x: &Self::Param) -> Result<Self::Hessian, argmin::core::Error> {
        self.system
            .jacobian(&x.0)
            .map(DMat)
            .map_err(|e| argmin::core::Error::msg(e.to_string()))
    }
}

/// Multi-dimensional Newton-Raphson solver for systems of equations.
///
/// For `f64`, provides [`solve_argmin`](Self::solve_argmin) backed by argmin.
/// J^{-1} is computed at the solution point when requested (for AAD).
#[derive(Debug, Clone)]
pub struct MultidimensionalNewtonSolver<T: RealField + Copy> {
    config: MultidimNewtonConfig<T>,
}

impl<T: RealField + Copy + Float> MultidimensionalNewtonSolver<T> {
    /// Create a new solver with the given configuration.
    pub fn new(config: MultidimNewtonConfig<T>) -> Self { Self { config } }

    /// Create a solver with default configuration.
    pub fn with_defaults() -> Self { Self::new(MultidimNewtonConfig::default()) }

    /// Get the solver configuration.
    pub fn config(&self) -> &MultidimNewtonConfig<T> { &self.config }

    /// Solve the system of equations F(x) = 0.
    pub fn solve<S: SystemOfEquations<T>>(
        &self,
        system: &S,
        initial_guess: DVector<T>,
    ) -> Result<MultidimSolverResult<T>, SolverError> {
        let n = system.dimension();
        if initial_guess.len() != n {
            return Err(SolverError::DimensionMismatch {
                expected: n,
                got: initial_guess.len(),
            });
        }

        let mut x = initial_guess;
        for iter in 0..self.config.max_iterations {
            let f = system.evaluate(&x)?;
            let residual_norm = f.norm();

            if residual_norm < self.config.tolerance {
                let ji = if self.config.store_jacobian_inverse {
                    Some(self.compute_inverse(&system.jacobian(&x)?)?)
                } else {
                    None
                };
                return Ok(MultidimSolverResult::new(x, residual_norm, iter, true, ji));
            }

            let j = system.jacobian(&x)?;
            let neg_f: Vec<T> = f.iter().map(|&v| -v).collect();
            let delta = self.solve_linear(&j, &neg_f)?;

            if DVector::from_vec(delta.clone()).norm() < self.config.param_tolerance {
                let ji = if self.config.store_jacobian_inverse {
                    Some(self.compute_inverse(&j)?)
                } else {
                    None
                };
                return Ok(MultidimSolverResult::new(x, residual_norm, iter, true, ji));
            }

            x += &DVector::from_vec(delta);
        }

        Err(SolverError::MaxIterationsExceeded {
            iterations: self.config.max_iterations,
        })
    }

    /// Solve J * x = b via LU decomposition.
    fn solve_linear(&self, j: &DMatrix<T>, b: &[T]) -> Result<Vec<T>, SolverError> {
        j.clone()
            .lu()
            .solve(&DVector::from_column_slice(b))
            .map(|x| x.iter().copied().collect())
            .ok_or_else(|| SolverError::NumericalInstability("Singular Jacobian matrix".into()))
    }

    /// Compute J^{-1} via nalgebra.
    fn compute_inverse(&self, j: &DMatrix<T>) -> Result<DMatrix<T>, SolverError> {
        j.clone().try_inverse().ok_or_else(|| {
            SolverError::NumericalInstability("Singular Jacobian matrix (cannot invert)".into())
        })
    }
}

/// Specialised `f64` implementation that delegates to argmin's Newton solver.
impl MultidimensionalNewtonSolver<f64> {
    /// Solve using argmin's Newton solver internally, then collect J^{-1}.
    ///
    /// This is semantically equivalent to [`solve`](Self::solve) but uses
    /// argmin's `Executor` + `Newton` for the iteration loop. J^{-1} is
    /// still computed at the solution point when requested.
    pub fn solve_argmin<S: SystemOfEquations<f64>>(
        &self,
        system: &S,
        initial_guess: DVector<f64>,
    ) -> Result<MultidimSolverResult<f64>, SolverError> {
        use argmin::core::State;

        let n = system.dimension();
        if initial_guess.len() != n {
            return Err(SolverError::DimensionMismatch {
                expected: n,
                got: initial_guess.len(),
            });
        }

        let adapter = ArgminSystemAdapter { system };
        let newton = argmin::solver::newton::Newton::new();
        let executor = argmin::core::Executor::new(adapter, newton).configure(|state| {
            state
                .param(DVec(initial_guess))
                .max_iters(self.config.max_iterations as u64)
                .target_cost(self.config.tolerance)
        });

        let result = executor
            .run()
            .map_err(|e| SolverError::NumericalInstability(format!("argmin Newton: {e}")))?;

        let state = &result.state;
        let solution = state
            .get_best_param()
            .map(|p| p.0.clone())
            .ok_or_else(|| SolverError::NumericalInstability("No solution found".into()))?;
        let iterations = state.get_iter() as usize;

        // Evaluate residual at solution to determine convergence
        let f = system.evaluate(&solution)?;
        let residual_norm = f.norm();
        let converged = residual_norm < self.config.tolerance;

        // Compute J^{-1} at the solution for AAD implicit function theorem
        let ji = if self.config.store_jacobian_inverse {
            Some(self.compute_inverse(&system.jacobian(&solution)?)?)
        } else {
            None
        };

        Ok(MultidimSolverResult::new(
            solution,
            residual_norm,
            iterations,
            converged,
            ji,
        ))
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    struct LinearSystem {
        a: DMatrix<f64>,
        b: DVector<f64>,
    }

    impl SystemOfEquations<f64> for LinearSystem {
        fn dimension(&self) -> usize { self.b.len() }
        fn evaluate(&self, x: &DVector<f64>) -> Result<DVector<f64>, SolverError> {
            Ok(&self.a * x - &self.b)
        }
        fn jacobian(&self, _x: &DVector<f64>) -> Result<DMatrix<f64>, SolverError> {
            Ok(self.a.clone())
        }
    }

    struct QuadraticSystem;

    impl SystemOfEquations<f64> for QuadraticSystem {
        fn dimension(&self) -> usize { 2 }
        fn evaluate(&self, x: &DVector<f64>) -> Result<DVector<f64>, SolverError> {
            if x.len() != 2 {
                return Err(SolverError::DimensionMismatch {
                    expected: 2,
                    got: x.len(),
                });
            }
            Ok(DVector::from_vec(vec![
                x[0] * x[0] + x[1] - 3.0,
                x[0] + x[1] * x[1] - 5.0,
            ]))
        }
        fn jacobian(&self, x: &DVector<f64>) -> Result<DMatrix<f64>, SolverError> {
            Ok(DMatrix::from_row_slice(
                2,
                2,
                &[2.0 * x[0], 1.0, 1.0, 2.0 * x[1]],
            ))
        }
    }

    struct NumericalJacobianSystem;

    impl SystemOfEquations<f64> for NumericalJacobianSystem {
        fn dimension(&self) -> usize { 2 }
        fn evaluate(&self, x: &DVector<f64>) -> Result<DVector<f64>, SolverError> {
            Ok(DVector::from_vec(vec![
                x[0] * x[0] + x[1] - 3.0,
                x[0] + x[1] * x[1] - 5.0,
            ]))
        }
    }

    #[test]
    fn test_system_evaluate_and_jacobian() {
        let sys = QuadraticSystem;
        assert_eq!(sys.dimension(), 2);

        let x = DVector::from_vec(vec![1.0, 2.0]);
        let f = sys.evaluate(&x).unwrap();
        assert_relative_eq!(f[0], 0.0, epsilon = 1e-10);
        assert_relative_eq!(f[1], -0.0, epsilon = 1e-10);

        let x2 = DVector::from_vec(vec![2.0, 3.0]);
        let j = sys.jacobian(&x2).unwrap();
        assert_relative_eq!(j[(0, 0)], 4.0, epsilon = 1e-10);
        assert_relative_eq!(j[(1, 1)], 6.0, epsilon = 1e-10);
    }

    #[test]
    fn test_numerical_jacobian() {
        let sys = NumericalJacobianSystem;
        let x = DVector::from_vec(vec![2.0, 3.0]);
        let j = sys.jacobian(&x).unwrap();
        assert_relative_eq!(j[(0, 0)], 4.0, epsilon = 1e-5);
        assert_relative_eq!(j[(0, 1)], 1.0, epsilon = 1e-5);
        assert_relative_eq!(j[(1, 0)], 1.0, epsilon = 1e-5);
        assert_relative_eq!(j[(1, 1)], 6.0, epsilon = 1e-5);
    }

    #[test]
    fn test_config_presets() {
        let d: MultidimNewtonConfig<f64> = MultidimNewtonConfig::default();
        assert_relative_eq!(d.tolerance, 1e-10, epsilon = 1e-15);
        assert_eq!(d.max_iterations, 100);
        assert!(d.store_jacobian_inverse);

        let hp: MultidimNewtonConfig<f64> = MultidimNewtonConfig::high_precision();
        assert!(hp.tolerance < 1e-12 && hp.max_iterations >= 500);

        let f: MultidimNewtonConfig<f64> = MultidimNewtonConfig::fast();
        assert!(f.tolerance > 1e-8 && f.max_iterations <= 50 && !f.store_jacobian_inverse);

        let c: MultidimNewtonConfig<f64> =
            MultidimNewtonConfig::new(1e-8, 200).with_jacobian_inverse(false);
        assert_relative_eq!(c.tolerance, 1e-8, epsilon = 1e-15);
        assert!(!c.store_jacobian_inverse);
    }

    #[test]
    fn test_result_jacobian_inverse() {
        let solution = DVector::from_vec(vec![1.0, 2.0]);
        let ji = DMatrix::identity(2, 2);
        let r = MultidimSolverResult::new(solution, 1e-12, 5, true, Some(ji));
        assert!(r.has_jacobian_inverse());
        assert!(r.jacobian_inverse_or_err().is_ok());

        let r2 = MultidimSolverResult::new(DVector::from_vec(vec![1.0]), 1e-12, 5, true, None);
        assert!(!r2.has_jacobian_inverse());
        assert!(r2.jacobian_inverse_or_err().is_err());
    }

    #[test]
    fn test_solve_linear_system() {
        let a = DMatrix::from_row_slice(2, 2, &[2.0, 1.0, 1.0, 3.0]);
        let b = DVector::from_vec(vec![5.0, 8.0]);
        let solver = MultidimensionalNewtonSolver::with_defaults();
        let result = solver
            .solve(&LinearSystem { a, b }, DVector::from_vec(vec![0.0, 0.0]))
            .unwrap();
        assert!(result.converged);
        assert_relative_eq!(result.solution[0], 1.4, epsilon = 1e-8);
        assert_relative_eq!(result.solution[1], 2.2, epsilon = 1e-8);
    }

    #[test]
    fn test_solve_quadratic_system() {
        let solver = MultidimensionalNewtonSolver::with_defaults();
        let result = solver
            .solve(&QuadraticSystem, DVector::from_vec(vec![1.0, 1.0]))
            .unwrap();
        assert!(result.converged && result.iterations < 20);
        assert!(QuadraticSystem.evaluate(&result.solution).unwrap().norm() < 1e-10);
    }

    #[test]
    fn test_solve_with_numerical_jacobian() {
        let solver = MultidimensionalNewtonSolver::with_defaults();
        let result = solver
            .solve(&NumericalJacobianSystem, DVector::from_vec(vec![1.0, 1.0]))
            .unwrap();
        assert!(result.converged);
        assert!(
            NumericalJacobianSystem
                .evaluate(&result.solution)
                .unwrap()
                .norm()
                < 1e-8
        );
    }

    #[test]
    fn test_stores_jacobian_inverse() {
        let config = MultidimNewtonConfig::default().with_jacobian_inverse(true);
        let solver = MultidimensionalNewtonSolver::new(config);
        let result = solver
            .solve(&QuadraticSystem, DVector::from_vec(vec![1.0, 1.0]))
            .unwrap();

        assert!(result.converged && result.has_jacobian_inverse());
        let j = QuadraticSystem.jacobian(&result.solution).unwrap();
        let ji = result.jacobian_inverse.as_ref().unwrap();
        let prod = &j * ji;
        let id = DMatrix::<f64>::identity(2, 2);
        for i in 0..2 {
            for k in 0..2 {
                assert_relative_eq!(prod[(i, k)], id[(i, k)], epsilon = 1e-8);
            }
        }
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let solver = MultidimensionalNewtonSolver::with_defaults();
        assert!(matches!(
            solver
                .solve(&QuadraticSystem, DVector::from_vec(vec![1.0, 2.0, 3.0]))
                .unwrap_err(),
            SolverError::DimensionMismatch {
                expected: 2,
                got: 3
            }
        ));
    }

    #[test]
    fn test_max_iterations_exceeded() {
        let config = MultidimNewtonConfig {
            tolerance: 1e-100,
            param_tolerance: 1e-100,
            max_iterations: 2,
            ..MultidimNewtonConfig::default()
        };
        let solver = MultidimensionalNewtonSolver::new(config);
        assert!(matches!(
            solver
                .solve(&QuadraticSystem, DVector::from_vec(vec![1.0, 1.0]))
                .unwrap_err(),
            SolverError::MaxIterationsExceeded { iterations: 2 }
        ));
    }

    #[test]
    fn test_solve_without_jacobian_inverse() {
        let solver = MultidimensionalNewtonSolver::new(MultidimNewtonConfig::fast());
        let result = solver
            .solve(&QuadraticSystem, DVector::from_vec(vec![1.0, 1.0]))
            .unwrap();
        assert!(result.converged && !result.has_jacobian_inverse());
    }

    #[test]
    fn test_identity_system() {
        let system = LinearSystem {
            a: DMatrix::<f64>::identity(2, 2),
            b: DVector::from_vec(vec![1.0, 2.0]),
        };
        let solver = MultidimensionalNewtonSolver::with_defaults();
        let result = solver
            .solve(&system, DVector::from_vec(vec![0.0, 0.0]))
            .unwrap();
        assert!(result.converged && result.iterations == 1);
        assert_relative_eq!(result.solution[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result.solution[1], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_solve_argmin_linear() {
        let system = LinearSystem {
            a: DMatrix::from_row_slice(2, 2, &[2.0, 1.0, 1.0, 3.0]),
            b: DVector::from_vec(vec![5.0, 8.0]),
        };
        let solver = MultidimensionalNewtonSolver::with_defaults();
        let result = solver
            .solve_argmin(&system, DVector::from_vec(vec![0.0, 0.0]))
            .unwrap();
        assert!(result.converged);
        assert_relative_eq!(result.solution[0], 1.4, epsilon = 1e-6);
        assert_relative_eq!(result.solution[1], 2.2, epsilon = 1e-6);
    }

    #[test]
    fn test_solve_argmin_quadratic() {
        let solver = MultidimensionalNewtonSolver::with_defaults();
        let result = solver
            .solve_argmin(&QuadraticSystem, DVector::from_vec(vec![1.0, 1.0]))
            .unwrap();
        assert!(result.converged);
        assert!(QuadraticSystem.evaluate(&result.solution).unwrap().norm() < 1e-8);
    }

    #[test]
    fn test_solve_argmin_stores_jacobian_inverse() {
        let config = MultidimNewtonConfig::default().with_jacobian_inverse(true);
        let solver = MultidimensionalNewtonSolver::new(config);
        let result = solver
            .solve_argmin(&QuadraticSystem, DVector::from_vec(vec![1.0, 1.0]))
            .unwrap();
        assert!(result.converged && result.has_jacobian_inverse());
    }
}
