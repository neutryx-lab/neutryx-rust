//! Bisection method root-finding solver.
//!
//! The bisection method is a simple, robust bracketing algorithm that
//! repeatedly halves the search interval to locate a root. While slower
//! than Newton-type methods, it guarantees convergence for continuous
//! functions with a valid bracket.
//!
//! # Algorithm
//!
//! Given a continuous function `f` and an interval `[a, b]` where
//! `f(a)` and `f(b)` have opposite signs (i.e., a valid bracket):
//!
//! 1. Compute the midpoint `m = (a + b) / 2`
//! 2. Evaluate `f(m)`
//! 3. If `|f(m)| < tolerance`, return `m` as the root
//! 4. Otherwise, replace `a` or `b` with `m` to maintain opposite signs
//! 5. Repeat until convergence or max iterations reached
//!
//! # Convergence
//!
//! The bisection method has linear convergence, reducing the interval
//! size by half each iteration. For an initial interval of width `w`,
//! after `n` iterations the interval width is `w / 2^n`.
//!
//! # Use Cases
//!
//! - When derivative information is unavailable or expensive to compute
//! - When robustness is more important than speed
//! - As a fallback when faster methods fail
//! - For functions with discontinuous derivatives

use num_traits::Float;

use super::SolverConfig;
use crate::{math::numeric::from_f64, types::SolverError};

/// Bisection method root finder.
///
/// A simple, robust bracketing algorithm that repeatedly halves the
/// search interval. Guaranteed to converge for continuous functions
/// with a valid bracket.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`)
///
/// # Convergence
///
/// The bisection method converges linearly, with the error halving each
/// iteration. This makes it slower than Newton-Raphson (quadratic) or
/// Brent's method (superlinear), but it is extremely robust.
///
/// # Example
///
/// ```
/// use pricer_core::math::solvers::{BisectionSolver, SolverConfig};
///
/// // Solve x^2 - 2 = 0 (find sqrt(2))
/// let solver = BisectionSolver::new(SolverConfig::default());
///
/// let f = |x: f64| x * x - 2.0;
///
/// let root = solver.find_root(f, 0.0, 2.0).unwrap();
/// assert!((root - std::f64::consts::SQRT_2).abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct BisectionSolver<T: Float> {
    /// Solver configuration
    config: SolverConfig<T>,
}

impl<T: Float> BisectionSolver<T> {
    /// Create a new bisection solver with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Solver configuration with tolerance and max iterations
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::solvers::{BisectionSolver, SolverConfig};
    ///
    /// let solver: BisectionSolver<f64> = BisectionSolver::new(SolverConfig::default());
    /// ```
    pub fn new(config: SolverConfig<T>) -> Self { Self { config } }

    /// Create a solver with default configuration.
    ///
    /// Default values:
    /// - `tolerance`: 1e-10
    /// - `max_iterations`: 100
    pub fn with_defaults() -> Self {
        Self {
            config: SolverConfig::default(),
        }
    }

    /// Find a root of `f` in the bracket [a, b].
    ///
    /// Requires that `f(a)` and `f(b)` have opposite signs (a valid bracket).
    /// The algorithm repeatedly halves the interval until convergence.
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
    /// * `Err(SolverError::NumericalInstability)` - Non-finite value
    ///   encountered
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::solvers::{BisectionSolver, SolverConfig};
    ///
    /// let solver = BisectionSolver::new(SolverConfig::default());
    ///
    /// // Solve x^3 - x - 2 = 0 in bracket [1, 2]
    /// let f = |x: f64| x * x * x - x - 2.0;
    ///
    /// let root = solver.find_root(f, 1.0, 2.0).unwrap();
    /// assert!((f(root)).abs() < 1e-10);
    /// ```
    pub fn find_root<F>(&self, f: F, a: T, b: T) -> Result<T, SolverError>
    where
        F: Fn(T) -> T,
    {
        let mut a = a;
        let mut b = b;
        let mut fa = f(a);
        let fb = f(b);

        // Check for valid bracket (opposite signs)
        if fa * fb > T::zero() {
            return Err(SolverError::NoBracket {
                a: a.to_f64().unwrap_or(f64::NAN),
                b: b.to_f64().unwrap_or(f64::NAN),
            });
        }

        // Check if either endpoint is already a root
        if fa.abs() < self.config.tolerance {
            return Ok(a);
        }
        if fb.abs() < self.config.tolerance {
            return Ok(b);
        }

        let two: T = from_f64(2.0);

        for _iteration in 0..self.config.max_iterations {
            // Compute midpoint
            let m = (a + b) / two;
            let fm = f(m);

            // Check for non-finite values
            if !m.is_finite() || !fm.is_finite() {
                return Err(SolverError::NumericalInstability(
                    "Bisection produced non-finite value".to_string(),
                ));
            }

            // Check for convergence
            if fm.abs() < self.config.tolerance {
                return Ok(m);
            }

            // Check if interval is too small (machine precision)
            if (b - a).abs() < self.config.tolerance {
                return Ok(m);
            }

            // Update bracket: maintain opposite signs
            if fa * fm < T::zero() {
                // Root is in [a, m]
                b = m;
                // Note: fb value not needed as we check fa * fm
            } else {
                // Root is in [m, b]
                a = m;
                fa = fm;
            }
        }

        Err(SolverError::MaxIterationsExceeded {
            iterations: self.config.max_iterations,
        })
    }

    /// Returns a reference to the solver configuration.
    pub fn config(&self) -> &SolverConfig<T> { &self.config }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Basic Functionality Tests
    // ========================================

    #[test]
    fn test_find_sqrt_2() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // Solve x^2 - 2 = 0 in bracket [0, 2]
        let f = |x: f64| x * x - 2.0;

        let root = solver.find_root(f, 0.0, 2.0).unwrap();
        assert!(
            (root - std::f64::consts::SQRT_2).abs() < 1e-10,
            "Expected sqrt(2) = {}, got {}",
            std::f64::consts::SQRT_2,
            root
        );
    }

    #[test]
    fn test_find_cubic_root() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // Solve x^3 - x - 2 = 0 (has root near 1.52)
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
        let solver = BisectionSolver::new(SolverConfig::default());

        // Solve sin(x) = 0 in [3, 4] (should find pi)
        let f = |x: f64| x.sin();

        let root = solver.find_root(f, 3.0, 4.0).unwrap();
        assert!(
            (root - std::f64::consts::PI).abs() < 1e-10,
            "Expected pi = {}, got {}",
            std::f64::consts::PI,
            root
        );
    }

    #[test]
    fn test_find_exp_root() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // Solve e^x - 2 = 0 in [0, 1] (find ln(2))
        let f = |x: f64| x.exp() - 2.0;

        let root = solver.find_root(f, 0.0, 1.0).unwrap();
        assert!(
            (root - 2.0_f64.ln()).abs() < 1e-10,
            "Expected ln(2) = {}, got {}",
            2.0_f64.ln(),
            root
        );
    }

    #[test]
    fn test_bracket_reversed() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // Bracket with b < a should still work
        let f = |x: f64| x * x - 2.0;

        let root = solver.find_root(f, 2.0, 0.0).unwrap();
        assert!((root - std::f64::consts::SQRT_2).abs() < 1e-10);
    }

    // ========================================
    // Error Handling Tests
    // ========================================

    #[test]
    fn test_no_bracket_same_sign_positive() {
        let solver = BisectionSolver::new(SolverConfig::default());

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
    fn test_no_bracket_always_positive() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // f(x) = x^2 + 1 is always positive
        let f = |x: f64| x * x + 1.0;

        let result = solver.find_root(f, -1.0, 1.0);
        assert!(result.is_err());

        match result.unwrap_err() {
            SolverError::NoBracket { .. } => {}
            other => panic!("Expected NoBracket error, got {:?}", other),
        }
    }

    #[test]
    fn test_root_at_bracket_endpoint_left() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // f(x) = x, root at x = 0
        let f = |x: f64| x;

        // Bracket includes root at left endpoint
        let root = solver.find_root(f, 0.0, 1.0).unwrap();
        assert!(root.abs() < 1e-10);
    }

    #[test]
    fn test_root_at_bracket_endpoint_right() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // f(x) = x - 1, root at x = 1
        let f = |x: f64| x - 1.0;

        // Bracket includes root at right endpoint
        let root = solver.find_root(f, 0.0, 1.0).unwrap();
        assert!((root - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_max_iterations_exceeded() {
        let config = SolverConfig::new(1e-100, 5); // Impossible tolerance, few iterations
        let solver = BisectionSolver::new(config);

        let f = |x: f64| x * x - 2.0;

        let result = solver.find_root(f, 0.0, 2.0);
        assert!(result.is_err());

        match result.unwrap_err() {
            SolverError::MaxIterationsExceeded { iterations } => {
                assert_eq!(iterations, 5);
            }
            other => panic!("Expected MaxIterationsExceeded error, got {:?}", other),
        }
    }

    // ========================================
    // Convergence Tests
    // ========================================

    #[test]
    fn test_achieves_tolerance() {
        let tol = 1e-12;
        let config = SolverConfig::new(tol, 100);
        let solver = BisectionSolver::new(config);

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
    fn test_tight_bracket() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // Very tight bracket around sqrt(2)
        let f = |x: f64| x * x - 2.0;
        let sqrt2 = std::f64::consts::SQRT_2;

        let root = solver.find_root(f, sqrt2 - 1e-8, sqrt2 + 1e-8).unwrap();
        assert!((root - sqrt2).abs() < 1e-10);
    }

    #[test]
    fn test_wide_bracket() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // Wide bracket - use a function with sign change in the interval
        // f(x) = x - 5 has a root at x = 5, and f(-100) < 0, f(100) > 0
        let f = |x: f64| x - 5.0;

        let root = solver.find_root(f, -100.0, 100.0).unwrap();
        assert!((root - 5.0).abs() < 1e-10);
    }

    // ========================================
    // Configuration Tests
    // ========================================

    #[test]
    fn test_with_defaults() {
        let solver: BisectionSolver<f64> = BisectionSolver::with_defaults();

        let f = |x: f64| x - 1.0;

        let root = solver.find_root(f, 0.0, 2.0).unwrap();
        assert!((root - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_config_accessor() {
        let config = SolverConfig::new(1e-8, 50);
        let solver = BisectionSolver::new(config);

        assert!((solver.config().tolerance - 1e-8).abs() < 1e-15);
        assert_eq!(solver.config().max_iterations, 50);
    }

    #[test]
    fn test_clone() {
        let solver: BisectionSolver<f64> = BisectionSolver::with_defaults();
        let cloned = solver.clone();

        assert_eq!(
            solver.config().max_iterations,
            cloned.config().max_iterations
        );
    }

    #[test]
    fn test_with_f32() {
        // Use relaxed tolerance for f32
        let config = SolverConfig {
            tolerance: 1e-5_f32,
            max_iterations: 100,
        };
        let solver: BisectionSolver<f32> = BisectionSolver::new(config);

        let f = |x: f32| x * x - 2.0;

        let root = solver.find_root(f, 0.0_f32, 2.0_f32).unwrap();
        assert!((root - std::f32::consts::SQRT_2).abs() < 1e-5);
    }

    // ========================================
    // Behaviour Tests
    // ========================================

    #[test]
    fn test_linear_function() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // f(x) = 2x - 3, root at x = 1.5
        let f = |x: f64| 2.0 * x - 3.0;

        let root = solver.find_root(f, 0.0, 3.0).unwrap();
        assert!((root - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_polynomial_multiple_roots() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // f(x) = x^3 - x = x(x-1)(x+1), roots at -1, 0, 1
        let f = |x: f64| x * x * x - x;

        // Should find root in the given bracket
        let root = solver.find_root(f, 0.5, 2.0).unwrap();
        assert!((root - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_transcendental_function() {
        let solver = BisectionSolver::new(SolverConfig::default());

        // f(x) = x - cos(x), has root near 0.739
        let f = |x: f64| x - x.cos();

        let root = solver.find_root(f, 0.0, 1.0).unwrap();
        assert!(
            f(root).abs() < 1e-10,
            "f(root) = {} should be near zero",
            f(root)
        );
    }
}
