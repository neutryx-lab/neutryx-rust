//! Backtracking Newton-Raphson root-finding solver with Armijo line search.
//!
//! This module implements a Newton-Raphson solver enhanced with backtracking
//! line search using the Armijo condition. This approach improves global
//! convergence behaviour compared to standard Newton-Raphson by ensuring
//! sufficient decrease in the objective function at each step.
//!
//! # Algorithm
//!
//! The backtracking Newton method modifies standard Newton iteration:
//!
//! 1. Compute the Newton direction: `d = -f(x) / f'(x)`
//! 2. Set step size `alpha = 1.0`
//! 3. While Armijo condition is not satisfied:
//!    - Reduce step: `alpha = alpha * 0.5`
//! 4. Update: `x = x + alpha * d`
//!
//! # Armijo Condition
//!
//! The Armijo (sufficient decrease) condition ensures that each step
//! makes adequate progress. For root finding, we minimise `|f(x)|^2`:
//!
//! ```text
//! |f(x + alpha * d)|^2 <= |f(x)|^2 + c1 * alpha * gradient . d
//! ```
//!
//! where `c1` is typically 1e-4 (the default).
//!
//! # Use Cases
//!
//! - When standard Newton-Raphson diverges from poor initial guesses
//! - For functions where the Newton step might overshoot
//! - When robust convergence is needed without bracketing information

use num_traits::Float;

use super::SolverConfig;
use crate::{math::numeric::from_f64, types::SolverError};

/// Backtracking Newton-Raphson root finder with Armijo line search.
///
/// Combines Newton's method with backtracking line search for improved
/// global convergence. The Armijo condition ensures sufficient decrease
/// in the objective function (|f(x)|^2) at each iteration.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`)
///
/// # Algorithm Details
///
/// The solver uses the following parameters:
/// - **Backtracking factor**: 0.5 (step size halved each backtrack)
/// - **Armijo parameter c1**: configurable (default 1e-4)
/// - **Maximum backtracks**: 50 (prevents infinite loops)
///
/// # Example
///
/// ```
/// use pricer_core::math::solvers::{BacktrackingNewtonSolver, SolverConfig};
///
/// // Solve x^2 - 2 = 0 (find sqrt(2))
/// let solver = BacktrackingNewtonSolver::new(SolverConfig::default());
///
/// let f = |x: f64| x * x - 2.0;
/// let f_prime = |x: f64| 2.0 * x;
///
/// let root = solver.find_root(f, f_prime, 1.0).unwrap();
/// assert!((root - std::f64::consts::SQRT_2).abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct BacktrackingNewtonSolver<T: Float> {
    /// Solver configuration
    config: SolverConfig<T>,
    /// Armijo condition parameter (sufficient decrease constant)
    c1: T,
}

impl<T: Float> BacktrackingNewtonSolver<T> {
    /// Create a new backtracking Newton solver with the given configuration.
    ///
    /// Uses the default Armijo parameter `c1 = 1e-4`.
    ///
    /// # Arguments
    ///
    /// * `config` - Solver configuration with tolerance and max iterations
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::solvers::{BacktrackingNewtonSolver, SolverConfig};
    ///
    /// let solver: BacktrackingNewtonSolver<f64> = BacktrackingNewtonSolver::new(SolverConfig::default());
    /// ```
    pub fn new(config: SolverConfig<T>) -> Self {
        Self {
            config,
            c1: from_f64(1e-4),
        }
    }

    /// Create a new backtracking Newton solver with custom Armijo parameter.
    ///
    /// # Arguments
    ///
    /// * `config` - Solver configuration with tolerance and max iterations
    /// * `c1` - Armijo parameter for sufficient decrease condition (typically
    ///   1e-4)
    ///
    /// # Panics
    ///
    /// Panics if `c1` is not in the range (0, 1).
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::solvers::{BacktrackingNewtonSolver, SolverConfig};
    ///
    /// // Use a stricter Armijo condition
    /// let solver: BacktrackingNewtonSolver<f64> =
    ///     BacktrackingNewtonSolver::with_armijo(SolverConfig::default(), 1e-2);
    /// ```
    pub fn with_armijo(config: SolverConfig<T>, c1: T) -> Self {
        assert!(
            c1 > T::zero() && c1 < T::one(),
            "Armijo parameter c1 must be in (0, 1)"
        );
        Self { config, c1 }
    }

    /// Create a solver with default configuration.
    ///
    /// Default values:
    /// - `tolerance`: 1e-10
    /// - `max_iterations`: 100
    /// - `c1`: 1e-4
    pub fn with_defaults() -> Self {
        Self {
            config: SolverConfig::default(),
            c1: from_f64(1e-4),
        }
    }

    /// Find a root of `f` using explicit derivative `f_prime`.
    ///
    /// Uses Newton's iteration with backtracking line search:
    /// `x_{n+1} = x_n + alpha * d` where `d = -f(x_n) / f'(x_n)`
    /// and `alpha` is chosen to satisfy the Armijo condition.
    ///
    /// # Arguments
    ///
    /// * `f` - Function to find root of
    /// * `f_prime` - Derivative of f
    /// * `x0` - Initial guess
    ///
    /// # Returns
    ///
    /// * `Ok(x)` - Root where `|f(x)| < tolerance`
    /// * `Err(SolverError::MaxIterationsExceeded)` - Failed to converge
    /// * `Err(SolverError::DerivativeNearZero)` - Derivative too small
    /// * `Err(SolverError::NumericalInstability)` - Line search failed or
    ///   non-finite value
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::solvers::{BacktrackingNewtonSolver, SolverConfig};
    ///
    /// let solver = BacktrackingNewtonSolver::new(SolverConfig::default());
    ///
    /// // Solve x^3 - x - 2 = 0
    /// let f = |x: f64| x * x * x - x - 2.0;
    /// let f_prime = |x: f64| 3.0 * x * x - 1.0;
    ///
    /// let root = solver.find_root(f, f_prime, 1.5).unwrap();
    /// assert!((f(root)).abs() < 1e-10);
    /// ```
    pub fn find_root<F, G>(&self, f: F, f_prime: G, x0: T) -> Result<T, SolverError>
    where
        F: Fn(T) -> T,
        G: Fn(T) -> T,
    {
        let mut x = x0;
        let epsilon: T = from_f64(1e-30);
        let backtrack_factor: T = from_f64(0.5);
        let two: T = from_f64(2.0);
        let max_backtracks: usize = 50;

        for _iteration in 0..self.config.max_iterations {
            let f_val = f(x);

            // Check for convergence
            if f_val.abs() < self.config.tolerance {
                return Ok(x);
            }

            let f_prime_val = f_prime(x);

            // Check for near-zero derivative
            if f_prime_val.abs() < epsilon {
                return Err(SolverError::DerivativeNearZero {
                    x: x.to_f64().unwrap_or(f64::NAN),
                });
            }

            // Newton direction: d = -f(x) / f'(x)
            let d = -f_val / f_prime_val;

            // Current objective value: phi(0) = |f(x)|^2
            let phi_0 = f_val * f_val;

            // Gradient of phi at x in direction d
            // phi(x) = f(x)^2, so phi'(x) = 2*f(x)*f'(x)
            // Directional derivative: phi'(0) = 2*f(x)*f'(x)*d
            //                                 = 2*f(x)*f'(x)*(-f(x)/f'(x))
            //                                 = -2*f(x)^2
            let phi_prime_0 = -two * phi_0;

            // Backtracking line search
            let mut alpha = T::one();
            let mut backtrack_count = 0;

            loop {
                let x_new = x + alpha * d;

                // Check for non-finite values
                if !x_new.is_finite() {
                    return Err(SolverError::NumericalInstability(
                        "Backtracking Newton produced non-finite value".to_string(),
                    ));
                }

                let f_new = f(x_new);
                let phi_new = f_new * f_new;

                // Armijo condition: phi(alpha) <= phi(0) + c1 * alpha * phi'(0)
                // Since phi'(0) = -2*f(x)^2 < 0 for non-zero f(x), this is:
                // |f(x_new)|^2 <= |f(x)|^2 - c1 * alpha * 2 * |f(x)|^2
                if phi_new <= phi_0 + self.c1 * alpha * phi_prime_0 {
                    // Armijo condition satisfied
                    x = x_new;
                    break;
                }

                // Backtrack
                alpha = alpha * backtrack_factor;
                backtrack_count += 1;

                if backtrack_count >= max_backtracks {
                    return Err(SolverError::NumericalInstability(
                        "Line search failed to satisfy Armijo condition".to_string(),
                    ));
                }
            }

            // Check for non-finite result after update
            if !x.is_finite() {
                return Err(SolverError::NumericalInstability(
                    "Backtracking Newton iteration produced non-finite value".to_string(),
                ));
            }
        }

        Err(SolverError::MaxIterationsExceeded {
            iterations: self.config.max_iterations,
        })
    }

    /// Returns a reference to the solver configuration.
    pub fn config(&self) -> &SolverConfig<T> { &self.config }

    /// Returns the Armijo parameter c1.
    pub fn armijo_c1(&self) -> T { self.c1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Basic Functionality Tests
    // ========================================

    #[test]
    fn test_find_sqrt_2() {
        let solver = BacktrackingNewtonSolver::new(SolverConfig::default());

        // Solve x^2 - 2 = 0 (find sqrt(2))
        let f = |x: f64| x * x - 2.0;
        let f_prime = |x: f64| 2.0 * x;

        let root = solver.find_root(f, f_prime, 1.0).unwrap();
        assert!(
            (root - std::f64::consts::SQRT_2).abs() < 1e-10,
            "Expected sqrt(2) = {}, got {}",
            std::f64::consts::SQRT_2,
            root
        );
    }

    #[test]
    fn test_find_cubic_root() {
        let solver = BacktrackingNewtonSolver::new(SolverConfig::default());

        // Solve x^3 - x - 2 = 0
        let f = |x: f64| x * x * x - x - 2.0;
        let f_prime = |x: f64| 3.0 * x * x - 1.0;

        let root = solver.find_root(f, f_prime, 1.5).unwrap();
        assert!(
            f(root).abs() < 1e-10,
            "f(root) = {} should be near zero",
            f(root)
        );
    }

    #[test]
    fn test_find_sin_root() {
        let solver = BacktrackingNewtonSolver::new(SolverConfig::default());

        // Solve sin(x) = 0 near x = 3 (should find pi)
        let f = |x: f64| x.sin();
        let f_prime = |x: f64| x.cos();

        let root = solver.find_root(f, f_prime, 3.0).unwrap();
        assert!(
            (root - std::f64::consts::PI).abs() < 1e-10,
            "Expected pi = {}, got {}",
            std::f64::consts::PI,
            root
        );
    }

    #[test]
    fn test_find_exp_root() {
        let solver = BacktrackingNewtonSolver::new(SolverConfig::default());

        // Solve e^x - 2 = 0 (find ln(2))
        let f = |x: f64| x.exp() - 2.0;
        let f_prime = |x: f64| x.exp();

        let root = solver.find_root(f, f_prime, 0.5).unwrap();
        assert!(
            (root - 2.0_f64.ln()).abs() < 1e-10,
            "Expected ln(2) = {}, got {}",
            2.0_f64.ln(),
            root
        );
    }

    // ========================================
    // Backtracking Behaviour Tests
    // ========================================

    #[test]
    fn test_backtracking_from_far_initial_guess() {
        let solver = BacktrackingNewtonSolver::new(SolverConfig::default());

        // Start far from root - backtracking helps converge
        let f = |x: f64| x * x - 2.0;
        let f_prime = |x: f64| 2.0 * x;

        // Far initial guess where standard Newton might overshoot
        let root = solver.find_root(f, f_prime, 10.0).unwrap();
        assert!(
            (root - std::f64::consts::SQRT_2).abs() < 1e-10,
            "Expected sqrt(2), got {}",
            root
        );
    }

    #[test]
    fn test_backtracking_prevents_divergence() {
        let solver = BacktrackingNewtonSolver::new(SolverConfig::default());

        // Function where standard Newton might have trouble
        // f(x) = arctan(x), f'(x) = 1/(1+x^2)
        let f = |x: f64| x.atan();
        let f_prime = |x: f64| 1.0 / (1.0 + x * x);

        // Start at x=1, Newton step would be large: -arctan(1) * (1+1) = -pi/4 * 2
        let root = solver.find_root(f, f_prime, 1.0).unwrap();
        assert!(root.abs() < 1e-10, "Expected 0, got {}", root);
    }

    // ========================================
    // Error Handling Tests
    // ========================================

    #[test]
    fn test_derivative_near_zero() {
        let solver = BacktrackingNewtonSolver::new(SolverConfig::default());

        // Derivative always zero
        let f = |x: f64| x * x * x;
        let f_prime = |_x: f64| 0.0;

        let result = solver.find_root(f, f_prime, 0.5);
        assert!(result.is_err());

        match result.unwrap_err() {
            SolverError::DerivativeNearZero { .. } => {}
            other => panic!("Expected DerivativeNearZero error, got {:?}", other),
        }
    }

    #[test]
    fn test_max_iterations_exceeded() {
        let config = SolverConfig::new(1e-100, 3); // Impossible tolerance
        let solver = BacktrackingNewtonSolver::new(config);

        let f = |x: f64| x * x - 2.0;
        let f_prime = |x: f64| 2.0 * x;

        let result = solver.find_root(f, f_prime, 1.0);
        assert!(result.is_err());

        match result.unwrap_err() {
            SolverError::MaxIterationsExceeded { iterations } => {
                assert_eq!(iterations, 3);
            }
            other => panic!("Expected MaxIterationsExceeded error, got {:?}", other),
        }
    }

    // ========================================
    // Configuration Tests
    // ========================================

    #[test]
    fn test_with_defaults() {
        let solver: BacktrackingNewtonSolver<f64> = BacktrackingNewtonSolver::with_defaults();

        let f = |x: f64| x - 1.0;
        let f_prime = |_x: f64| 1.0;

        let root = solver.find_root(f, f_prime, 0.0).unwrap();
        assert!((root - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_with_custom_armijo() {
        let config = SolverConfig::default();
        let solver: BacktrackingNewtonSolver<f64> =
            BacktrackingNewtonSolver::with_armijo(config, 1e-2);

        assert!((solver.armijo_c1() - 1e-2).abs() < 1e-15);

        let f = |x: f64| x * x - 2.0;
        let f_prime = |x: f64| 2.0 * x;

        let root = solver.find_root(f, f_prime, 1.0).unwrap();
        assert!((root - std::f64::consts::SQRT_2).abs() < 1e-10);
    }

    #[test]
    #[should_panic(expected = "Armijo parameter c1 must be in (0, 1)")]
    fn test_invalid_armijo_zero() {
        let _: BacktrackingNewtonSolver<f64> =
            BacktrackingNewtonSolver::with_armijo(SolverConfig::default(), 0.0);
    }

    #[test]
    #[should_panic(expected = "Armijo parameter c1 must be in (0, 1)")]
    fn test_invalid_armijo_one() {
        let _: BacktrackingNewtonSolver<f64> =
            BacktrackingNewtonSolver::with_armijo(SolverConfig::default(), 1.0);
    }

    #[test]
    #[should_panic(expected = "Armijo parameter c1 must be in (0, 1)")]
    fn test_invalid_armijo_negative() {
        let _: BacktrackingNewtonSolver<f64> =
            BacktrackingNewtonSolver::with_armijo(SolverConfig::default(), -0.1);
    }

    #[test]
    fn test_config_accessor() {
        let config = SolverConfig::new(1e-8, 50);
        let solver = BacktrackingNewtonSolver::new(config);

        assert!((solver.config().tolerance - 1e-8).abs() < 1e-15);
        assert_eq!(solver.config().max_iterations, 50);
    }

    #[test]
    fn test_armijo_accessor() {
        let solver: BacktrackingNewtonSolver<f64> = BacktrackingNewtonSolver::with_defaults();
        assert!((solver.armijo_c1() - 1e-4).abs() < 1e-15);
    }

    #[test]
    fn test_clone() {
        let solver: BacktrackingNewtonSolver<f64> = BacktrackingNewtonSolver::with_defaults();
        let cloned = solver.clone();

        assert_eq!(
            solver.config().max_iterations,
            cloned.config().max_iterations
        );
        assert!((solver.armijo_c1() - cloned.armijo_c1()).abs() < 1e-15);
    }

    #[test]
    fn test_with_f32() {
        // Use relaxed tolerance for f32
        let config = SolverConfig {
            tolerance: 1e-5_f32,
            max_iterations: 100,
        };
        let solver: BacktrackingNewtonSolver<f32> = BacktrackingNewtonSolver::new(config);

        let f = |x: f32| x * x - 2.0;
        let f_prime = |x: f32| 2.0 * x;

        let root = solver.find_root(f, f_prime, 1.0_f32).unwrap();
        assert!((root - std::f32::consts::SQRT_2).abs() < 1e-5);
    }

    // ========================================
    // Convergence Quality Tests
    // ========================================

    #[test]
    fn test_achieves_tolerance() {
        let tol = 1e-12;
        let config = SolverConfig::new(tol, 100);
        let solver = BacktrackingNewtonSolver::new(config);

        let f = |x: f64| x * x - 2.0;
        let f_prime = |x: f64| 2.0 * x;

        let root = solver.find_root(f, f_prime, 1.0).unwrap();
        assert!(
            f(root).abs() < tol,
            "f(root) = {} should be less than tolerance {}",
            f(root),
            tol
        );
    }

    #[test]
    fn test_transcendental_function() {
        let solver = BacktrackingNewtonSolver::new(SolverConfig::default());

        // f(x) = x - cos(x), has root near 0.739
        let f = |x: f64| x - x.cos();
        let f_prime = |x: f64| 1.0 + x.sin();

        let root = solver.find_root(f, f_prime, 0.5).unwrap();
        assert!(
            f(root).abs() < 1e-10,
            "f(root) = {} should be near zero",
            f(root)
        );
    }

    #[test]
    fn test_polynomial_higher_degree() {
        let solver = BacktrackingNewtonSolver::new(SolverConfig::default());

        // f(x) = x^4 - 16 = 0, roots at +/- 2
        let f = |x: f64| x.powi(4) - 16.0;
        let f_prime = |x: f64| 4.0 * x.powi(3);

        let root = solver.find_root(f, f_prime, 1.5).unwrap();
        assert!((root - 2.0).abs() < 1e-10, "Expected 2, got {}", root);
    }

    #[test]
    fn test_converges_to_root_at_zero() {
        // Use high precision config for functions with degenerate derivative at root
        let config = SolverConfig::high_precision();
        let solver = BacktrackingNewtonSolver::new(config);

        // f(x) = x^3, root at x = 0. Note: f'(0) = 0 so convergence is slower
        let f = |x: f64| x * x * x;
        let f_prime = |x: f64| 3.0 * x * x;

        // Start close to zero but not at zero
        let root = solver.find_root(f, f_prime, 0.1).unwrap();
        // Relaxed tolerance due to degenerate derivative at root
        assert!(root.abs() < 1e-4, "Expected near 0, got {}", root);
    }
}
