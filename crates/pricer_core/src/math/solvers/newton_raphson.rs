//! Newton-Raphson root-finding solver.

use super::SolverConfig;
use crate::types::SolverError;
use num_traits::Float;

/// Newton-Raphson root finder with optional AD support.
///
/// Uses Newton's method: `x_{n+1} = x_n - f(x_n) / f'(x_n)` for fast
/// quadratic convergence on smooth functions.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`)
///
/// # Convergence
///
/// Newton-Raphson converges quadratically near a root, meaning the number
/// of correct digits approximately doubles each iteration. However, it may
/// fail if:
/// - The derivative is near zero
/// - The initial guess is far from the root
/// - The function has discontinuities
///
/// # Example
///
/// ```
/// use pricer_core::math::solvers::{NewtonRaphsonSolver, SolverConfig};
///
/// // Solve x² - 2 = 0 (find √2)
/// let solver = NewtonRaphsonSolver::new(SolverConfig::default());
///
/// let f = |x: f64| x * x - 2.0;
/// let f_prime = |x: f64| 2.0 * x;
///
/// let root = solver.find_root(f, f_prime, 1.0).unwrap();
/// assert!((root - std::f64::consts::SQRT_2).abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct NewtonRaphsonSolver<T: Float> {
    /// Solver configuration
    config: SolverConfig<T>,
}

impl<T: Float> NewtonRaphsonSolver<T> {
    /// Create a new Newton-Raphson solver with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Solver configuration with tolerance and max iterations
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::solvers::{NewtonRaphsonSolver, SolverConfig};
    ///
    /// let solver: NewtonRaphsonSolver<f64> = NewtonRaphsonSolver::new(SolverConfig::default());
    /// ```
    pub fn new(config: SolverConfig<T>) -> Self {
        Self { config }
    }

    /// Create a solver with default configuration.
    pub fn with_defaults() -> Self {
        Self {
            config: SolverConfig::default(),
        }
    }

    /// Find a root of `f` using explicit derivative `f_prime`.
    ///
    /// Uses Newton's iteration: `x_{n+1} = x_n - f(x_n) / f'(x_n)`
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
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::solvers::{NewtonRaphsonSolver, SolverConfig};
    ///
    /// let solver = NewtonRaphsonSolver::new(SolverConfig::default());
    ///
    /// // Solve x³ - x - 2 = 0
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
        let epsilon = T::from(1e-30).unwrap();

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

            // Newton update
            #[allow(clippy::assign_op_pattern)]
            {
                x = x - f_val / f_prime_val;
            }

            // Check for non-finite values
            if !x.is_finite() {
                return Err(SolverError::NumericalInstability(
                    "Newton iteration produced non-finite value".to_string(),
                ));
            }
        }

        Err(SolverError::MaxIterationsExceeded {
            iterations: self.config.max_iterations,
        })
    }

    /// Returns a reference to the solver configuration.
    pub fn config(&self) -> &SolverConfig<T> {
        &self.config
    }
}

/// AD-enabled Newton-Raphson solver for f64.
#[cfg(feature = "num-dual-mode")]
impl NewtonRaphsonSolver<f64> {
    /// Find a root using automatic differentiation.
    ///
    /// This method automatically computes the derivative using dual numbers,
    /// eliminating the need to provide an explicit derivative function.
    ///
    /// # Arguments
    ///
    /// * `f` - Function to find root of (must accept Dual64)
    /// * `x0` - Initial guess
    ///
    /// # Returns
    ///
    /// * `Ok(x)` - Root where `|f(x)| < tolerance`
    /// * `Err(SolverError::MaxIterationsExceeded)` - Failed to converge
    /// * `Err(SolverError::DerivativeNearZero)` - Derivative too small
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pricer_core::math::solvers::{NewtonRaphsonSolver, SolverConfig};
    /// use num_dual::Dual64;
    ///
    /// let solver = NewtonRaphsonSolver::new(SolverConfig::default());
    ///
    /// // Solve x² - 2 = 0 using AD
    /// let f = |x: Dual64| x * x - Dual64::from(2.0);
    ///
    /// let root = solver.find_root_ad(f, 1.0).unwrap();
    /// assert!((root - std::f64::consts::SQRT_2).abs() < 1e-10);
    /// ```
    pub fn find_root_ad<F>(&self, f: F, x0: f64) -> Result<f64, SolverError>
    where
        F: Fn(num_dual::Dual64) -> num_dual::Dual64,
    {
        use num_dual::Dual64;

        let mut x = x0;
        let epsilon = 1e-30;

        for _iteration in 0..self.config.max_iterations {
            // Create dual number with derivative seed
            let x_dual = Dual64::new(x, 1.0);
            let f_dual = f(x_dual);

            // Extract function value and derivative
            let f_val = f_dual.re;
            let f_prime_val = f_dual.eps;

            // Check for convergence
            if f_val.abs() < self.config.tolerance {
                return Ok(x);
            }

            // Check for near-zero derivative
            if f_prime_val.abs() < epsilon {
                return Err(SolverError::DerivativeNearZero { x });
            }

            // Newton update
            x -= f_val / f_prime_val;

            // Check for non-finite values
            if !x.is_finite() {
                return Err(SolverError::NumericalInstability(
                    "Newton iteration produced non-finite value".to_string(),
                ));
            }
        }

        Err(SolverError::MaxIterationsExceeded {
            iterations: self.config.max_iterations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Basic Functionality Tests (Task 9.1)
    // ========================================

    #[test]
    fn test_find_sqrt_2() {
        let solver = NewtonRaphsonSolver::new(SolverConfig::default());

        // Solve x² - 2 = 0 (find √2)
        let f = |x: f64| x * x - 2.0;
        let f_prime = |x: f64| 2.0 * x;

        let root = solver.find_root(f, f_prime, 1.0).unwrap();
        assert!(
            (root - std::f64::consts::SQRT_2).abs() < 1e-10,
            "Expected √2 ≈ {}, got {}",
            std::f64::consts::SQRT_2,
            root
        );
    }

    #[test]
    fn test_find_cubic_root() {
        let solver = NewtonRaphsonSolver::new(SolverConfig::default());

        // Solve x³ - x - 2 = 0
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
        let solver = NewtonRaphsonSolver::new(SolverConfig::default());

        // Solve sin(x) = 0 near x = 3 (should find π)
        let f = |x: f64| x.sin();
        let f_prime = |x: f64| x.cos();

        let root = solver.find_root(f, f_prime, 3.0).unwrap();
        assert!(
            (root - std::f64::consts::PI).abs() < 1e-10,
            "Expected π ≈ {}, got {}",
            std::f64::consts::PI,
            root
        );
    }

    #[test]
    fn test_find_exp_root() {
        let solver = NewtonRaphsonSolver::new(SolverConfig::default());

        // Solve e^x - 2 = 0 (find ln(2))
        let f = |x: f64| x.exp() - 2.0;
        let f_prime = |x: f64| x.exp();

        let root = solver.find_root(f, f_prime, 0.5).unwrap();
        assert!(
            (root - 2.0_f64.ln()).abs() < 1e-10,
            "Expected ln(2) ≈ {}, got {}",
            2.0_f64.ln(),
            root
        );
    }

    // ========================================
    // Error Handling Tests (Task 9.3)
    // ========================================

    #[test]
    fn test_derivative_near_zero() {
        let solver = NewtonRaphsonSolver::new(SolverConfig::default());

        // Solve x³ = 0 starting at x = 0 (derivative is 0)
        let f = |x: f64| x * x * x;
        let f_prime = |_x: f64| 0.0; // Always zero derivative

        let result = solver.find_root(f, f_prime, 0.5);
        assert!(result.is_err());

        match result.unwrap_err() {
            SolverError::DerivativeNearZero { .. } => {}
            other => panic!("Expected DerivativeNearZero error, got {:?}", other),
        }
    }

    #[test]
    fn test_max_iterations_exceeded() {
        let config = SolverConfig::new(1e-15, 5); // Very tight tolerance, few iterations
        let solver = NewtonRaphsonSolver::new(config);

        // Try to solve something that needs more iterations
        let f = |x: f64| x * x - 2.0;
        let f_prime = |x: f64| 2.0 * x;

        let result = solver.find_root(f, f_prime, 100.0); // Far initial guess

        // May succeed or fail depending on convergence rate
        // Let's use a case that definitely won't converge in 5 iterations
        let config2 = SolverConfig::new(1e-100, 3); // Impossible tolerance
        let solver2 = NewtonRaphsonSolver::new(config2);
        let result2 = solver2.find_root(f, f_prime, 1.0);

        assert!(result2.is_err());
        match result2.unwrap_err() {
            SolverError::MaxIterationsExceeded { iterations } => {
                assert_eq!(iterations, 3);
            }
            other => panic!("Expected MaxIterationsExceeded error, got {:?}", other),
        }
    }

    #[test]
    fn test_with_defaults() {
        let solver: NewtonRaphsonSolver<f64> = NewtonRaphsonSolver::with_defaults();

        let f = |x: f64| x - 1.0;
        let f_prime = |_x: f64| 1.0;

        let root = solver.find_root(f, f_prime, 0.0).unwrap();
        assert!((root - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_config_accessor() {
        let config = SolverConfig::new(1e-8, 50);
        let solver = NewtonRaphsonSolver::new(config);

        assert!((solver.config().tolerance - 1e-8).abs() < 1e-15);
        assert_eq!(solver.config().max_iterations, 50);
    }

    #[test]
    fn test_clone() {
        let solver: NewtonRaphsonSolver<f64> = NewtonRaphsonSolver::with_defaults();
        let cloned = solver.clone();

        assert_eq!(
            solver.config().max_iterations,
            cloned.config().max_iterations
        );
    }

    #[test]
    fn test_with_f32() {
        let solver: NewtonRaphsonSolver<f32> = NewtonRaphsonSolver::with_defaults();

        let f = |x: f32| x * x - 2.0;
        let f_prime = |x: f32| 2.0 * x;

        let root = solver.find_root(f, f_prime, 1.0_f32).unwrap();
        assert!((root - std::f32::consts::SQRT_2).abs() < 1e-5);
    }

    // ========================================
    // AD Mode Tests (Task 9.2)
    // ========================================

    #[cfg(feature = "num-dual-mode")]
    mod ad_tests {
        use super::*;
        use num_dual::Dual64;

        #[test]
        fn test_find_root_ad_sqrt_2() {
            let solver = NewtonRaphsonSolver::new(SolverConfig::default());

            // Solve x² - 2 = 0 using AD
            let f = |x: Dual64| x * x - Dual64::from(2.0);

            let root = solver.find_root_ad(f, 1.0).unwrap();
            assert!(
                (root - std::f64::consts::SQRT_2).abs() < 1e-10,
                "Expected √2, got {}",
                root
            );
        }

        #[test]
        fn test_ad_matches_explicit() {
            let solver = NewtonRaphsonSolver::new(SolverConfig::default());

            // Compare AD mode to explicit derivative mode
            let f = |x: f64| x * x - 2.0;
            let f_prime = |x: f64| 2.0 * x;
            let f_ad = |x: Dual64| x * x - Dual64::from(2.0);

            let root_explicit = solver.find_root(f, f_prime, 1.0).unwrap();
            let root_ad = solver.find_root_ad(f_ad, 1.0).unwrap();

            assert!(
                (root_explicit - root_ad).abs() < 1e-10,
                "Explicit root {} should match AD root {}",
                root_explicit,
                root_ad
            );
        }

        #[test]
        fn test_ad_derivative_near_zero() {
            let solver = NewtonRaphsonSolver::new(SolverConfig::default());

            // Function with zero derivative at x=0
            let f = |x: Dual64| x * x * x;

            // Start at point where derivative is zero
            let result = solver.find_root_ad(f, 0.0);

            // Should either converge (since f(0)=0) or report derivative issue
            // In this case, f(0) = 0 so it should converge immediately
            assert!(result.is_ok());
        }
    }
}
