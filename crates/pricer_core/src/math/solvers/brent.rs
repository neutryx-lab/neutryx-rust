//! Brent's method root-finding solver.

use super::SolverConfig;
use crate::types::SolverError;
use num_traits::Float;

/// Brent's method root finder.
///
/// Combines bisection, secant, and inverse quadratic interpolation for
/// robust root finding without requiring derivatives. Guaranteed to
/// converge for continuous functions with a valid bracket.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`)
///
/// # Algorithm
///
/// Brent's method intelligently switches between:
/// - **Bisection**: Guaranteed progress, slower convergence
/// - **Secant method**: Faster convergence using linear approximation
/// - **Inverse quadratic interpolation**: Even faster when applicable
///
/// The method falls back to bisection when other methods would be unreliable.
///
/// # Example
///
/// ```
/// use pricer_core::math::solvers::{BrentSolver, SolverConfig};
///
/// let solver = BrentSolver::new(SolverConfig::default());
///
/// // Solve x³ - x - 2 = 0 in bracket [1, 2]
/// let f = |x: f64| x * x * x - x - 2.0;
///
/// let root = solver.find_root(f, 1.0, 2.0).unwrap();
/// assert!((f(root)).abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct BrentSolver<T: Float> {
    /// Solver configuration
    config: SolverConfig<T>,
}

impl<T: Float> BrentSolver<T> {
    /// Create a new Brent solver with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Solver configuration with tolerance and max iterations
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::solvers::{BrentSolver, SolverConfig};
    ///
    /// let solver: BrentSolver<f64> = BrentSolver::new(SolverConfig::default());
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

    /// Find a root of `f` in the bracket [a, b].
    ///
    /// Requires that `f(a)` and `f(b)` have opposite signs (a valid bracket).
    ///
    /// # Arguments
    ///
    /// * `f` - Function to find root of
    /// * `a` - Left bracket endpoint
    /// * `b` - Right bracket endpoint
    ///
    /// # Returns
    ///
    /// * `Ok(x)` - Root where `|f(x)| < tolerance`
    /// * `Err(SolverError::NoBracket)` - `f(a)` and `f(b)` have same sign
    /// * `Err(SolverError::MaxIterationsExceeded)` - Failed to converge
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::solvers::{BrentSolver, SolverConfig};
    ///
    /// let solver = BrentSolver::new(SolverConfig::default());
    ///
    /// // Solve x² - 2 = 0 in bracket [0, 2]
    /// let f = |x: f64| x * x - 2.0;
    ///
    /// let root = solver.find_root(f, 0.0, 2.0).unwrap();
    /// assert!((root - std::f64::consts::SQRT_2).abs() < 1e-10);
    /// ```
    pub fn find_root<F>(&self, f: F, a: T, b: T) -> Result<T, SolverError>
    where
        F: Fn(T) -> T,
    {
        let mut a = a;
        let mut b = b;
        let mut fa = f(a);
        let mut fb = f(b);

        // Check for valid bracket
        if fa * fb > T::zero() {
            return Err(SolverError::NoBracket {
                a: a.to_f64().unwrap_or(f64::NAN),
                b: b.to_f64().unwrap_or(f64::NAN),
            });
        }

        // Ensure |f(a)| >= |f(b)| (swap if necessary)
        if fa.abs() < fb.abs() {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut fa, &mut fb);
        }

        let mut c = a;
        let mut fc = fa;
        let mut d = b - a;
        let mut e = d;

        let two = T::from(2.0).unwrap();
        let three = T::from(3.0).unwrap();

        for _iteration in 0..self.config.max_iterations {
            // Check for convergence
            if fb.abs() < self.config.tolerance {
                return Ok(b);
            }

            // Check if bracket is small enough
            let tol = self.config.tolerance;
            let m = (c - b) / two;

            if m.abs() <= tol {
                return Ok(b);
            }

            // Decide whether to use interpolation or bisection
            let use_bisection;

            if fa != fc && fb != fc {
                // Inverse quadratic interpolation
                let r = fb / fc;
                let s = fb / fa;
                let t = fa / fc;

                let p = s * (t * (r - t) * (c - b) - (T::one() - r) * (b - a));
                let q = (t - T::one()) * (r - T::one()) * (s - T::one());

                // Check if step is acceptable
                if p.abs() < (three * m * q).abs() / two && p.abs() < (e * q).abs() / two {
                    e = d;
                    d = p / q;
                    use_bisection = false;
                } else {
                    use_bisection = true;
                }
            } else if fb != fa {
                // Secant method
                let s = fb / fa;
                let p = two * m * s;
                let q = T::one() - s;

                // Check if step is acceptable
                if p.abs() < (three * m * q).abs() / two && p.abs() < (e * q).abs() / two {
                    e = d;
                    d = p / q;
                    use_bisection = false;
                } else {
                    use_bisection = true;
                }
            } else {
                use_bisection = true;
            }

            // Apply bisection if interpolation was rejected
            if use_bisection {
                d = m;
                e = m;
            }

            // Update a to old b
            a = b;
            fa = fb;

            // Compute new b
            if d.abs() > tol {
                b = b + d;
            } else {
                // Minimum step
                b = b + if m > T::zero() { tol } else { -tol };
            }

            fb = f(b);

            // Keep bracket valid: ensure f(b) and f(c) have opposite signs
            if (fb > T::zero() && fc > T::zero()) || (fb < T::zero() && fc < T::zero()) {
                c = a;
                fc = fa;
                d = b - a;
                e = d;
            }

            // Ensure |f(c)| >= |f(b)|
            if fc.abs() < fb.abs() {
                a = b;
                b = c;
                c = a;
                fa = fb;
                fb = fc;
                fc = fa;
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Basic Functionality Tests (Task 10.1)
    // ========================================

    #[test]
    fn test_find_sqrt_2() {
        let solver = BrentSolver::new(SolverConfig::default());

        // Solve x² - 2 = 0 in bracket [0, 2]
        let f = |x: f64| x * x - 2.0;

        let root = solver.find_root(f, 0.0, 2.0).unwrap();
        assert!(
            (root - std::f64::consts::SQRT_2).abs() < 1e-10,
            "Expected √2 ≈ {}, got {}",
            std::f64::consts::SQRT_2,
            root
        );
    }

    #[test]
    fn test_find_cubic_root() {
        let solver = BrentSolver::new(SolverConfig::default());

        // Solve x³ - x - 2 = 0 (has root near 1.52)
        let f = |x: f64| x * x * x - x - 2.0;

        let root = solver.find_root(f, 1.0, 2.0).unwrap();
        assert!(
            f(root).abs() < 1e-10,
            "f(root) = {} should be near zero",
            f(root)
        );
    }

    #[test]
    fn test_find_sin_root() {
        let solver = BrentSolver::new(SolverConfig::default());

        // Solve sin(x) = 0 in [3, 4] (should find π)
        let f = |x: f64| x.sin();

        let root = solver.find_root(f, 3.0, 4.0).unwrap();
        assert!(
            (root - std::f64::consts::PI).abs() < 1e-10,
            "Expected π ≈ {}, got {}",
            std::f64::consts::PI,
            root
        );
    }

    #[test]
    fn test_find_exp_root() {
        let solver = BrentSolver::new(SolverConfig::default());

        // Solve e^x - 2 = 0 in [0, 1] (find ln(2))
        let f = |x: f64| x.exp() - 2.0;

        let root = solver.find_root(f, 0.0, 1.0).unwrap();
        assert!(
            (root - 2.0_f64.ln()).abs() < 1e-10,
            "Expected ln(2) ≈ {}, got {}",
            2.0_f64.ln(),
            root
        );
    }

    #[test]
    fn test_bracket_reversed() {
        let solver = BrentSolver::new(SolverConfig::default());

        // Bracket with b < a should still work
        let f = |x: f64| x * x - 2.0;

        let root = solver.find_root(f, 2.0, 0.0).unwrap();
        assert!((root - std::f64::consts::SQRT_2).abs() < 1e-10);
    }

    // ========================================
    // Error Handling Tests (Task 10.3)
    // ========================================

    #[test]
    fn test_no_bracket_same_sign_positive() {
        let solver = BrentSolver::new(SolverConfig::default());

        // f(1) = 1 > 0, f(2) = 4 > 0 - no sign change
        let f = |x: f64| x * x;

        let result = solver.find_root(f, 1.0, 2.0);
        assert!(result.is_err());

        match result.unwrap_err() {
            SolverError::NoBracket { a, b } => {
                assert!((a - 1.0).abs() < 1e-10);
                assert!((b - 2.0).abs() < 1e-10);
            }
            other => panic!("Expected NoBracket error, got {:?}", other),
        }
    }

    #[test]
    fn test_no_bracket_same_sign_negative() {
        let solver = BrentSolver::new(SolverConfig::default());

        // f(-2) = 2 > 0, f(-1) = -1 < 0 - wait, that's a valid bracket
        // Let's use: f(x) = x² + 1 which is always positive
        let f = |x: f64| x * x + 1.0;

        let result = solver.find_root(f, -1.0, 1.0);
        assert!(result.is_err());

        match result.unwrap_err() {
            SolverError::NoBracket { .. } => {}
            other => panic!("Expected NoBracket error, got {:?}", other),
        }
    }

    #[test]
    fn test_root_at_bracket_endpoint() {
        let solver = BrentSolver::new(SolverConfig::default());

        // f(x) = x - 1, root at x = 1
        let f = |x: f64| x - 1.0;

        // Bracket includes root at endpoint
        let root = solver.find_root(f, 0.0, 1.0).unwrap();
        assert!((root - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_max_iterations_exceeded() {
        let config = SolverConfig::new(1e-100, 3); // Impossible tolerance
        let solver = BrentSolver::new(config);

        let f = |x: f64| x * x - 2.0;

        let result = solver.find_root(f, 0.0, 2.0);
        assert!(result.is_err());

        match result.unwrap_err() {
            SolverError::MaxIterationsExceeded { iterations } => {
                assert_eq!(iterations, 3);
            }
            other => panic!("Expected MaxIterationsExceeded error, got {:?}", other),
        }
    }

    #[test]
    fn test_tight_bracket() {
        let solver = BrentSolver::new(SolverConfig::default());

        // Very tight bracket around √2
        let f = |x: f64| x * x - 2.0;
        let sqrt2 = std::f64::consts::SQRT_2;

        let root = solver.find_root(f, sqrt2 - 1e-8, sqrt2 + 1e-8).unwrap();
        assert!((root - sqrt2).abs() < 1e-10);
    }

    // ========================================
    // Convergence Tests (Task 10.2)
    // ========================================

    #[test]
    fn test_achieves_tolerance() {
        let tol = 1e-12;
        let config = SolverConfig::new(tol, 100);
        let solver = BrentSolver::new(config);

        let f = |x: f64| x * x - 2.0;

        let root = solver.find_root(f, 0.0, 2.0).unwrap();
        assert!(
            f(root).abs() < tol,
            "f(root) = {} should be less than tolerance {}",
            f(root),
            tol
        );
    }

    #[test]
    fn test_difficult_function() {
        let solver = BrentSolver::new(SolverConfig::default());

        // Function with slow convergence: x - cos(x) = 0
        let f = |x: f64| x - x.cos();

        let root = solver.find_root(f, 0.0, 1.0).unwrap();
        assert!(
            f(root).abs() < 1e-10,
            "f(root) = {} should be near zero",
            f(root)
        );
    }

    #[test]
    fn test_with_defaults() {
        let solver: BrentSolver<f64> = BrentSolver::with_defaults();

        let f = |x: f64| x - 1.0;

        let root = solver.find_root(f, 0.0, 2.0).unwrap();
        assert!((root - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_config_accessor() {
        let config = SolverConfig::new(1e-8, 50);
        let solver = BrentSolver::new(config);

        assert!((solver.config().tolerance - 1e-8).abs() < 1e-15);
        assert_eq!(solver.config().max_iterations, 50);
    }

    #[test]
    fn test_clone() {
        let solver: BrentSolver<f64> = BrentSolver::with_defaults();
        let cloned = solver.clone();

        assert_eq!(
            solver.config().max_iterations,
            cloned.config().max_iterations
        );
    }

    #[test]
    fn test_with_f32() {
        let solver: BrentSolver<f32> = BrentSolver::with_defaults();

        let f = |x: f32| x * x - 2.0;

        let root = solver.find_root(f, 0.0_f32, 2.0_f32).unwrap();
        assert!((root - std::f32::consts::SQRT_2).abs() < 1e-5);
    }
}
