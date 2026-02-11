//! Multi-dimensional Newton-Raphson solver for systems of equations.
//!
//! This module provides a generalised Newton-Raphson solver for solving
//! systems of nonlinear equations F(x) = 0, where F: R^n -> R^n.
//!
//! The solver is designed for curve calibration and other financial
//! applications requiring simultaneous solution of multiple equations.
//!
//! ## Implementation
//!
//! Uses `nalgebra` directly for LU decomposition (linear system solve)
//! and matrix inversion. No custom linear algebra routines.
//!
//! ## Key Features
//!
//! - Generic over `T: RealField + Copy` for AD compatibility
//! - Jacobian inverse storage for implicit function theorem (AAD)
//! - Configurable convergence criteria (residual norm, parameter change)
//! - Numerical Jacobian fallback via finite differences
//!
//! ## Example
//!
//! ```ignore
//! use pricer_core::math::solvers::{
//!     MultidimensionalNewtonSolver, MultidimNewtonConfig, SystemOfEquations,
//! };
//! use nalgebra::{DMatrix, DVector};
//!
//! // Define a simple 2D system: F(x, y) = (x² + y - 3, x + y² - 5)
//! struct QuadraticSystem;
//!
//! impl SystemOfEquations<f64> for QuadraticSystem {
//!     fn dimension(&self) -> usize { 2 }
//!
//!     fn evaluate(&self, x: &DVector<f64>) -> Result<DVector<f64>, SolverError> {
//!         Ok(DVector::from_vec(vec![
//!             x[0] * x[0] + x[1] - 3.0,
//!             x[0] + x[1] * x[1] - 5.0,
//!         ]))
//!     }
//!
//!     fn jacobian(&self, x: &DVector<f64>) -> Result<DMatrix<f64>, SolverError> {
//!         Ok(DMatrix::from_row_slice(2, 2, &[
//!             2.0 * x[0], 1.0,
//!             1.0, 2.0 * x[1],
//!         ]))
//!     }
//! }
//!
//! let config = MultidimNewtonConfig::default();
//! let solver = MultidimensionalNewtonSolver::new(config);
//! let initial = DVector::from_vec(vec![1.0, 1.0]);
//!
//! let result = solver.solve(&QuadraticSystem, initial).unwrap();
//! assert!(result.converged);
//! ```

use nalgebra::{DMatrix, DVector, RealField};
use num_traits::Float;

use crate::{math::numeric::from_f64, types::SolverError};

// =============================================================================
// SystemOfEquations Trait
// =============================================================================

/// Trait for defining a system of nonlinear equations F(x) = 0.
///
/// Implementations must provide methods to evaluate the residual vector F(x)
/// and the Jacobian matrix J(x) = ∂F/∂x. A default numerical Jacobian
/// implementation is provided via finite differences.
///
/// # Type Parameters
///
/// * `T` - Floating-point type satisfying `RealField + Copy` for AD
///   compatibility
///
/// # Example
///
/// ```ignore
/// use nalgebra::{DMatrix, DVector, RealField};
/// use pricer_core::math::solvers::SystemOfEquations;
/// use pricer_core::types::SolverError;
///
/// struct LinearSystem<T: RealField + Copy> {
///     a: DMatrix<T>,
///     b: DVector<T>,
/// }
///
/// impl<T: RealField + Copy> SystemOfEquations<T> for LinearSystem<T> {
///     fn dimension(&self) -> usize {
///         self.b.len()
///     }
///
///     fn evaluate(&self, x: &DVector<T>) -> Result<DVector<T>, SolverError> {
///         Ok(&self.a * x - &self.b)
///     }
///
///     fn jacobian(&self, _x: &DVector<T>) -> Result<DMatrix<T>, SolverError> {
///         Ok(self.a.clone())
///     }
/// }
/// ```
pub trait SystemOfEquations<T: RealField + Copy + Float> {
    /// Returns the dimension of the system (number of equations/unknowns).
    ///
    /// For a square system, this is both the input and output dimension.
    fn dimension(&self) -> usize;

    /// Evaluates the residual vector F(x).
    ///
    /// # Arguments
    ///
    /// * `x` - Current iterate, must have length equal to `dimension()`
    ///
    /// # Returns
    ///
    /// * `Ok(DVector<T>)` - Residual vector of length `dimension()`
    /// * `Err(SolverError)` - If evaluation fails (e.g., domain error)
    ///
    /// # Errors
    ///
    /// Returns `SolverError::DimensionMismatch` if `x.len() != dimension()`.
    fn evaluate(&self, x: &DVector<T>) -> Result<DVector<T>, SolverError>;

    /// Computes the Jacobian matrix J(x) = ∂F/∂x.
    ///
    /// For a system of n equations in n unknowns, the Jacobian is an n×n matrix
    /// where J\[i,j\] = ∂F_i/∂x_j.
    ///
    /// # Arguments
    ///
    /// * `x` - Current iterate at which to evaluate the Jacobian
    ///
    /// # Returns
    ///
    /// * `Ok(DMatrix<T>)` - Jacobian matrix of shape (n, n)
    /// * `Err(SolverError)` - If computation fails
    ///
    /// # Default Implementation
    ///
    /// If not overridden, falls back to
    /// [`jacobian_numerical`](Self::jacobian_numerical) with epsilon =
    /// 1e-8.
    fn jacobian(&self, x: &DVector<T>) -> Result<DMatrix<T>, SolverError> {
        self.jacobian_numerical(x, from_f64(1e-8))
    }

    /// Computes a numerical approximation to the Jacobian via finite
    /// differences.
    ///
    /// Uses forward differences: J\[i,j\] ≈ (F_i(x + ε*e_j) - F_i(x)) / ε
    ///
    /// # Arguments
    ///
    /// * `x` - Current iterate
    /// * `epsilon` - Finite difference step size
    ///
    /// # Returns
    ///
    /// * `Ok(DMatrix<T>)` - Numerically approximated Jacobian
    /// * `Err(SolverError)` - If function evaluation fails
    ///
    /// # Performance Note
    ///
    /// Requires n+1 function evaluations where n = dimension().
    /// For large systems, consider providing an analytical Jacobian.
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

// =============================================================================
// Solver Configuration
// =============================================================================

/// Configuration for the multi-dimensional Newton-Raphson solver.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for tolerance values
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::solvers::MultidimNewtonConfig;
///
/// // Use default configuration
/// let config: MultidimNewtonConfig<f64> = MultidimNewtonConfig::default();
///
/// // Custom configuration
/// let config = MultidimNewtonConfig {
///     tolerance: 1e-12,
///     param_tolerance: 1e-10,
///     max_iterations: 200,
///     jacobian_epsilon: 1e-6,
///     store_jacobian_inverse: true,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultidimNewtonConfig<T: RealField + Copy> {
    /// Convergence tolerance for residual norm.
    ///
    /// The solver stops when ||F(x)|| < tolerance.
    pub tolerance: T,

    /// Convergence tolerance for parameter change.
    ///
    /// The solver also stops when ||Δx|| < param_tolerance.
    pub param_tolerance: T,

    /// Maximum number of iterations before returning an error.
    pub max_iterations: usize,

    /// Step size for numerical Jacobian approximation.
    ///
    /// Used when `jacobian_numerical` is called.
    pub jacobian_epsilon: T,

    /// Whether to store the Jacobian inverse in the result.
    ///
    /// Set to `true` for AAD applications using the implicit function theorem.
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
    /// Create a new configuration with specified tolerances.
    pub fn new(tolerance: T, max_iterations: usize) -> Self {
        Self {
            tolerance,
            param_tolerance: tolerance,
            max_iterations,
            ..Self::default()
        }
    }

    /// Create a high-precision configuration.
    ///
    /// Uses tighter tolerances (1e-14) and more iterations (500).
    pub fn high_precision() -> Self {
        Self {
            tolerance: from_f64(1e-14),
            param_tolerance: from_f64(1e-14),
            max_iterations: 500,
            jacobian_epsilon: from_f64(1e-10),
            store_jacobian_inverse: true,
        }
    }

    /// Create a fast configuration with relaxed tolerances.
    ///
    /// Uses looser tolerances (1e-6) and fewer iterations (50).
    pub fn fast() -> Self {
        Self {
            tolerance: from_f64(1e-6),
            param_tolerance: from_f64(1e-6),
            max_iterations: 50,
            jacobian_epsilon: from_f64(1e-6),
            store_jacobian_inverse: false,
        }
    }

    /// Enable Jacobian inverse storage for AAD.
    pub fn with_jacobian_inverse(mut self, store: bool) -> Self {
        self.store_jacobian_inverse = store;
        self
    }
}

// =============================================================================
// Solver Result
// =============================================================================

/// Result from the multi-dimensional Newton-Raphson solver.
///
/// Contains the solution, convergence information, and optionally the
/// Jacobian inverse for use with the implicit function theorem in AAD.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for solution components
///
/// # AAD Integration
///
/// When `store_jacobian_inverse` is enabled, the result contains J⁻¹ at
/// the solution point. This is used for computing sensitivities via the
/// implicit function theorem:
///
/// ∂x*/∂m = -J⁻¹ · ∂F/∂m
///
/// where x* is the solution and m are market parameters.
#[derive(Debug, Clone)]
pub struct MultidimSolverResult<T: RealField + Copy> {
    /// Solution vector x* satisfying F(x*) ≈ 0.
    pub solution: DVector<T>,

    /// Euclidean norm of the final residual ||F(x*)||.
    pub residual_norm: T,

    /// Number of iterations performed.
    pub iterations: usize,

    /// Whether the solver converged within tolerance.
    pub converged: bool,

    /// Jacobian inverse at the solution point (for AAD).
    ///
    /// This is `Some(J⁻¹)` if `store_jacobian_inverse` was enabled
    /// and the solver converged successfully.
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
            SolverError::NumericalInstability("Jacobian inverse not available".to_string())
        })
    }
}

// =============================================================================
// Multi-dimensional Newton-Raphson Solver
// =============================================================================

/// Multi-dimensional Newton-Raphson solver for systems of equations.
///
/// Solves F(x) = 0 using the Newton-Raphson iteration:
///
/// x_{k+1} = x_k - J(x_k)⁻¹ · F(x_k)
///
/// where J(x) is the Jacobian matrix ∂F/∂x.
///
/// # Features
///
/// - Dual convergence criteria (residual norm and parameter change)
/// - Optional Jacobian inverse storage for AAD
/// - Support for analytical or numerical Jacobians
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::solvers::{
///     MultidimensionalNewtonSolver, MultidimNewtonConfig, SystemOfEquations,
/// };
/// use nalgebra::DVector;
///
/// let config = MultidimNewtonConfig::default();
/// let solver = MultidimensionalNewtonSolver::new(config);
///
/// // Solve a 2D quadratic system
/// let initial = DVector::from_vec(vec![1.0, 1.0]);
/// let result = solver.solve(&MySystem, initial)?;
///
/// if result.converged {
///     println!("Solution: {:?}", result.solution);
/// }
/// ```
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
    ///
    /// # Arguments
    ///
    /// * `system` - The system of equations to solve
    /// * `initial_guess` - Starting point for iteration
    ///
    /// # Returns
    ///
    /// * `Ok(MultidimSolverResult<T>)` - Solution and convergence information
    /// * `Err(SolverError)` - If the solver fails (singular Jacobian, max
    ///   iterations)
    ///
    /// # Errors
    ///
    /// - `SolverError::DimensionMismatch`: Input dimension doesn't match system
    /// - `SolverError::SingularJacobian`: Jacobian is singular at some iterate
    /// - `SolverError::MaxIterationsExceeded`: Didn't converge within limit
    pub fn solve<S: SystemOfEquations<T>>(
        &self,
        system: &S,
        initial_guess: DVector<T>,
    ) -> Result<MultidimSolverResult<T>, SolverError> {
        let n = system.dimension();

        // Validate input dimension
        if initial_guess.len() != n {
            return Err(SolverError::DimensionMismatch {
                expected: n,
                got: initial_guess.len(),
            });
        }

        let mut x = initial_guess;

        for iter in 0..self.config.max_iterations {
            // Evaluate residual
            let f = system.evaluate(&x)?;
            let residual_norm = f.norm();

            // Check residual convergence
            if residual_norm < self.config.tolerance {
                // Compute Jacobian inverse if requested
                let jacobian_inverse = if self.config.store_jacobian_inverse {
                    let j = system.jacobian(&x)?;
                    Some(self.compute_inverse(&j)?)
                } else {
                    None
                };

                return Ok(MultidimSolverResult::new(
                    x,
                    residual_norm,
                    iter,
                    true,
                    jacobian_inverse,
                ));
            }

            // Compute Jacobian
            let j = system.jacobian(&x)?;

            // Solve J · delta = -f for delta
            let neg_f: Vec<T> = f.iter().map(|&v| -v).collect();
            let delta_vec = self.solve_linear_system(&j, &neg_f)?;
            let delta = DVector::from_vec(delta_vec);

            // Check parameter convergence
            let param_change = delta.norm();
            if param_change < self.config.param_tolerance {
                let jacobian_inverse = if self.config.store_jacobian_inverse {
                    Some(self.compute_inverse(&j)?)
                } else {
                    None
                };

                return Ok(MultidimSolverResult::new(
                    x,
                    residual_norm,
                    iter,
                    true,
                    jacobian_inverse,
                ));
            }

            // Update iterate
            x += &delta;
        }

        // Max iterations exceeded
        Err(SolverError::MaxIterationsExceeded {
            iterations: self.config.max_iterations,
        })
    }

    /// Solve the linear system J * x = b using nalgebra's LU decomposition.
    fn solve_linear_system(&self, j: &DMatrix<T>, b: &[T]) -> Result<Vec<T>, SolverError> {
        let lu = j.clone().lu();
        let b_vec = DVector::from_column_slice(b);
        let x = lu.solve(&b_vec).ok_or_else(|| {
            SolverError::NumericalInstability("Singular Jacobian matrix".to_string())
        })?;
        Ok(x.iter().copied().collect())
    }

    /// Compute the inverse of the Jacobian matrix using nalgebra.
    fn compute_inverse(&self, j: &DMatrix<T>) -> Result<DMatrix<T>, SolverError> {
        j.clone().try_inverse().ok_or_else(|| {
            SolverError::NumericalInstability(
                "Singular Jacobian matrix (cannot invert)".to_string(),
            )
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    // Simple linear system: Ax - b = 0
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

    // Quadratic system: (x² + y - 3, x + y² - 5)
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

    // System using numerical Jacobian (no analytical override)
    struct NumericalJacobianSystem;

    impl SystemOfEquations<f64> for NumericalJacobianSystem {
        fn dimension(&self) -> usize { 2 }

        fn evaluate(&self, x: &DVector<f64>) -> Result<DVector<f64>, SolverError> {
            Ok(DVector::from_vec(vec![
                x[0] * x[0] + x[1] - 3.0,
                x[0] + x[1] * x[1] - 5.0,
            ]))
        }

        // Uses default numerical Jacobian
    }

    // =========================================================================
    // SystemOfEquations Trait Tests
    // =========================================================================

    #[test]
    fn test_system_dimension() {
        let system = QuadraticSystem;
        assert_eq!(system.dimension(), 2);
    }

    #[test]
    fn test_system_evaluate() {
        let system = QuadraticSystem;
        let x = DVector::from_vec(vec![1.0, 2.0]);
        let f = system.evaluate(&x).unwrap();

        assert_eq!(f.len(), 2);
        assert_relative_eq!(f[0], 1.0 + 2.0 - 3.0, epsilon = 1e-10); // x² + y - 3 = 0
        assert_relative_eq!(f[1], 1.0 + 4.0 - 5.0, epsilon = 1e-10); // x + y² -
                                                                     // 5 = 0
    }

    #[test]
    fn test_system_jacobian_analytical() {
        let system = QuadraticSystem;
        let x = DVector::from_vec(vec![2.0, 3.0]);
        let j = system.jacobian(&x).unwrap();

        assert_eq!(j.nrows(), 2);
        assert_eq!(j.ncols(), 2);
        assert_relative_eq!(j[(0, 0)], 4.0, epsilon = 1e-10); // ∂f1/∂x = 2x
        assert_relative_eq!(j[(0, 1)], 1.0, epsilon = 1e-10); // ∂f1/∂y = 1
        assert_relative_eq!(j[(1, 0)], 1.0, epsilon = 1e-10); // ∂f2/∂x = 1
        assert_relative_eq!(j[(1, 1)], 6.0, epsilon = 1e-10); // ∂f2/∂y = 2y
    }

    #[test]
    fn test_system_jacobian_numerical() {
        let system = NumericalJacobianSystem;
        let x = DVector::from_vec(vec![2.0, 3.0]);
        let j = system.jacobian(&x).unwrap();

        // Numerical Jacobian should approximate analytical Jacobian
        assert_relative_eq!(j[(0, 0)], 4.0, epsilon = 1e-5);
        assert_relative_eq!(j[(0, 1)], 1.0, epsilon = 1e-5);
        assert_relative_eq!(j[(1, 0)], 1.0, epsilon = 1e-5);
        assert_relative_eq!(j[(1, 1)], 6.0, epsilon = 1e-5);
    }

    // =========================================================================
    // MultidimNewtonConfig Tests
    // =========================================================================

    #[test]
    fn test_config_default() {
        let config: MultidimNewtonConfig<f64> = MultidimNewtonConfig::default();
        assert_relative_eq!(config.tolerance, 1e-10, epsilon = 1e-15);
        assert_eq!(config.max_iterations, 100);
        assert!(config.store_jacobian_inverse);
    }

    #[test]
    fn test_config_high_precision() {
        let config: MultidimNewtonConfig<f64> = MultidimNewtonConfig::high_precision();
        assert!(config.tolerance < 1e-12);
        assert!(config.max_iterations >= 500);
    }

    #[test]
    fn test_config_fast() {
        let config: MultidimNewtonConfig<f64> = MultidimNewtonConfig::fast();
        assert!(config.tolerance > 1e-8);
        assert!(config.max_iterations <= 50);
        assert!(!config.store_jacobian_inverse);
    }

    #[test]
    fn test_config_builder() {
        let config: MultidimNewtonConfig<f64> =
            MultidimNewtonConfig::new(1e-8, 200).with_jacobian_inverse(false);
        assert_relative_eq!(config.tolerance, 1e-8, epsilon = 1e-15);
        assert_eq!(config.max_iterations, 200);
        assert!(!config.store_jacobian_inverse);
    }

    // =========================================================================
    // MultidimSolverResult Tests
    // =========================================================================

    #[test]
    fn test_solver_result_new() {
        let solution = DVector::from_vec(vec![1.0, 2.0]);
        let jacobian_inverse = DMatrix::identity(2, 2);

        let result =
            MultidimSolverResult::new(solution.clone(), 1e-12, 5, true, Some(jacobian_inverse));

        assert_eq!(result.solution, solution);
        assert!(result.converged);
        assert_eq!(result.iterations, 5);
        assert!(result.has_jacobian_inverse());
    }

    #[test]
    fn test_solver_result_without_jacobian() {
        let solution = DVector::from_vec(vec![1.0, 2.0]);

        let result = MultidimSolverResult::new(solution, 1e-12, 5, true, None);

        assert!(!result.has_jacobian_inverse());
        assert!(result.jacobian_inverse_or_err().is_err());
    }

    // =========================================================================
    // MultidimensionalNewtonSolver Tests
    // =========================================================================

    #[test]
    fn test_solve_linear_system() {
        // Solve: [2 1; 1 3] * x = [5; 8]
        // 2x + y = 5, x + 3y = 8
        // x = 1.4, y = 2.2
        let a = DMatrix::from_row_slice(2, 2, &[2.0, 1.0, 1.0, 3.0]);
        let b = DVector::from_vec(vec![5.0, 8.0]);
        let system = LinearSystem { a, b };

        let solver = MultidimensionalNewtonSolver::with_defaults();
        let initial = DVector::from_vec(vec![0.0, 0.0]);

        let result = solver.solve(&system, initial).unwrap();

        assert!(result.converged);
        assert_relative_eq!(result.solution[0], 1.4, epsilon = 1e-8);
        assert_relative_eq!(result.solution[1], 2.2, epsilon = 1e-8);
    }

    #[test]
    fn test_solve_quadratic_system() {
        // Solve: x² + y - 3 = 0, x + y² - 5 = 0
        // Solutions include approximately (1.2016, 1.5567)
        let solver = MultidimensionalNewtonSolver::with_defaults();
        let initial = DVector::from_vec(vec![1.0, 1.0]);

        let result = solver.solve(&QuadraticSystem, initial).unwrap();

        assert!(result.converged);
        assert!(result.iterations < 20);

        // Verify solution satisfies equations
        let f = QuadraticSystem.evaluate(&result.solution).unwrap();
        assert!(f.norm() < 1e-10);
    }

    #[test]
    fn test_solve_with_numerical_jacobian() {
        let solver = MultidimensionalNewtonSolver::with_defaults();
        let initial = DVector::from_vec(vec![1.0, 1.0]);

        let result = solver.solve(&NumericalJacobianSystem, initial).unwrap();

        assert!(result.converged);

        // Verify solution
        let f = NumericalJacobianSystem.evaluate(&result.solution).unwrap();
        assert!(f.norm() < 1e-8);
    }

    #[test]
    fn test_solve_stores_jacobian_inverse() {
        let config = MultidimNewtonConfig::default().with_jacobian_inverse(true);
        let solver = MultidimensionalNewtonSolver::new(config);
        let initial = DVector::from_vec(vec![1.0, 1.0]);

        let result = solver.solve(&QuadraticSystem, initial).unwrap();

        assert!(result.converged);
        assert!(result.has_jacobian_inverse());

        // Verify J * J^-1 ≈ I
        let j = QuadraticSystem.jacobian(&result.solution).unwrap();
        let j_inv = result.jacobian_inverse.as_ref().unwrap();
        let product = &j * j_inv;

        let identity = DMatrix::<f64>::identity(2, 2);
        for i in 0..2 {
            for j_idx in 0..2 {
                assert_relative_eq!(product[(i, j_idx)], identity[(i, j_idx)], epsilon = 1e-8);
            }
        }
    }

    #[test]
    fn test_solve_dimension_mismatch_error() {
        let solver = MultidimensionalNewtonSolver::with_defaults();
        let initial = DVector::from_vec(vec![1.0, 2.0, 3.0]); // Wrong dimension

        let result = solver.solve(&QuadraticSystem, initial);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SolverError::DimensionMismatch {
                expected: 2,
                got: 3
            }
        ));
    }

    #[test]
    fn test_solve_max_iterations_exceeded() {
        // Use very strict tolerance that won't be reached
        let config = MultidimNewtonConfig {
            tolerance: 1e-100,
            param_tolerance: 1e-100,
            max_iterations: 2,
            ..MultidimNewtonConfig::default()
        };
        let solver = MultidimensionalNewtonSolver::new(config);
        let initial = DVector::from_vec(vec![1.0, 1.0]);

        let result = solver.solve(&QuadraticSystem, initial);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SolverError::MaxIterationsExceeded { iterations: 2 }
        ));
    }

    #[test]
    fn test_solve_without_storing_jacobian_inverse() {
        let config = MultidimNewtonConfig::fast();
        let solver = MultidimensionalNewtonSolver::new(config);
        let initial = DVector::from_vec(vec![1.0, 1.0]);

        let result = solver.solve(&QuadraticSystem, initial).unwrap();

        assert!(result.converged);
        assert!(!result.has_jacobian_inverse());
        assert!(result.jacobian_inverse.is_none());
    }

    #[test]
    fn test_solve_identity_system() {
        // Solve x - [1, 2] = 0 (trivial system)
        let a = DMatrix::<f64>::identity(2, 2);
        let b = DVector::from_vec(vec![1.0, 2.0]);
        let system = LinearSystem { a, b };

        let solver = MultidimensionalNewtonSolver::with_defaults();
        let initial = DVector::from_vec(vec![0.0, 0.0]);

        let result = solver.solve(&system, initial).unwrap();

        assert!(result.converged);
        assert_eq!(result.iterations, 1); // Should converge in 1 iteration
        assert_relative_eq!(result.solution[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result.solution[1], 2.0, epsilon = 1e-10);
    }
}
