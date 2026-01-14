//! Adjoint solver for AAD-compatible root finding.
//!
//! This module provides `AdjointSolver<T>`, a root-finding solver that
//! computes sensitivities using the implicit function theorem. It avoids
//! recording solver iterations in the AD tape by computing adjoints
//! analytically at convergence.
//!
//! ## Implicit Function Theorem
//!
//! For a root-finding problem f(x, θ) = 0, the implicit function theorem gives:
//!
//! ```text
//! dx/dθ = -(∂f/∂θ) / (∂f/∂x)
//! ```
//!
//! In reverse-mode AD (adjoint), this translates to:
//!
//! ```text
//! θ̄ += x̄ × (-∂f/∂θ / ∂f/∂x)
//! ```
//!
//! This avoids recording the O(n) iterations, making sensitivity computation O(1).

use num_traits::Float;
use pricer_core::math::solvers::{BrentSolver, NewtonRaphsonSolver, SolverConfig};
use pricer_core::types::SolverError;

/// Type of solver used for root-finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverType {
    /// Newton-Raphson with quadratic convergence
    NewtonRaphson,
    /// Brent method (bracketing, robust)
    Brent,
}

/// Result of a root-finding operation with optional adjoint information.
#[derive(Debug, Clone)]
pub struct SolveResult<T: Float> {
    /// The root x where f(x, θ) ≈ 0
    pub solution: T,
    /// Adjoint factor: -∂f/∂θ / ∂f/∂x at the solution
    /// Used for reverse-mode AD: θ̄ += x̄ × adjoint_factor
    pub adjoint_factor: Option<T>,
    /// Which solver method succeeded
    pub solver_used: SolverType,
    /// Number of iterations to converge
    pub iterations: usize,
    /// Final residual |f(x)|
    pub residual: T,
}

/// Result with full sensitivity matrix for multiple parameters.
#[derive(Debug, Clone)]
pub struct SolveResultWithSensitivities {
    /// The root x
    pub solution: f64,
    /// Sensitivities dx/dθ_i for each parameter θ_i
    pub sensitivities: Vec<f64>,
    /// Which solver succeeded
    pub solver_used: SolverType,
    /// Iterations used
    pub iterations: usize,
}

/// Configuration for the adjoint solver.
#[derive(Debug, Clone)]
pub struct AdjointSolverConfig<T: Float> {
    /// Tolerance for convergence
    pub tolerance: T,
    /// Maximum iterations for Newton-Raphson
    pub max_iterations: usize,
    /// Bracket search range for Brent fallback [lower, upper]
    pub brent_bracket: (T, T),
    /// Whether to compute adjoint factors
    pub compute_adjoints: bool,
}

impl<T: Float> Default for AdjointSolverConfig<T> {
    fn default() -> Self {
        Self {
            tolerance: T::from(1e-12).unwrap(),
            max_iterations: 100,
            brent_bracket: (T::from(0.001).unwrap(), T::from(2.0).unwrap()),
            compute_adjoints: true,
        }
    }
}

/// Adjoint-aware root-finding solver.
///
/// Wraps Newton-Raphson and Brent solvers with implicit function theorem
/// adjoint computation for efficient reverse-mode AD.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for AD compatibility
///
/// # Examples
///
/// ```
/// use pricer_optimiser::bootstrapping::AdjointSolver;
///
/// // Solve x² - 2 = 0 with adjoint
/// let solver: AdjointSolver<f64> = AdjointSolver::with_defaults();
///
/// let f = |x: f64| x * x - 2.0;
/// let f_prime = |x: f64| 2.0 * x;
///
/// let result = solver.solve(f, f_prime, 1.0).unwrap();
/// assert!((result.solution - std::f64::consts::SQRT_2).abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct AdjointSolver<T: Float> {
    /// Newton-Raphson solver
    newton: NewtonRaphsonSolver<T>,
    /// Brent solver for fallback
    brent: BrentSolver<T>,
    /// Configuration
    config: AdjointSolverConfig<T>,
}

impl<T: Float> AdjointSolver<T> {
    /// Create a new adjoint solver with custom configuration.
    pub fn new(config: AdjointSolverConfig<T>) -> Self {
        let solver_config = SolverConfig {
            tolerance: config.tolerance,
            max_iterations: config.max_iterations,
        };

        Self {
            newton: NewtonRaphsonSolver::new(solver_config),
            brent: BrentSolver::new(solver_config),
            config,
        }
    }

    /// Create a solver with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(AdjointSolverConfig::default())
    }

    /// Get the configuration.
    pub fn config(&self) -> &AdjointSolverConfig<T> {
        &self.config
    }

    /// Solve f(x) = 0 using Newton-Raphson with Brent fallback.
    ///
    /// # Arguments
    ///
    /// * `f` - Function to find root of
    /// * `f_prime` - Derivative of f with respect to x
    /// * `x0` - Initial guess
    ///
    /// # Returns
    ///
    /// * `Ok(SolveResult)` - Solution with adjoint factor for ∂f/∂x
    /// * `Err(SolverError)` - If both Newton-Raphson and Brent fail
    pub fn solve<F, G>(&self, f: F, f_prime: G, x0: T) -> Result<SolveResult<T>, SolverError>
    where
        F: Fn(T) -> T,
        G: Fn(T) -> T,
    {
        // Try Newton-Raphson first
        match self.newton.find_root(&f, &f_prime, x0) {
            Ok(solution) => {
                let residual = f(solution).abs();
                let iterations = 0; // Newton-Raphson doesn't report iterations in current API

                // Compute adjoint factor: 1 / f'(x)
                // This is used for the implicit function theorem
                let adjoint_factor = if self.config.compute_adjoints {
                    let df_dx = f_prime(solution);
                    if df_dx.abs() > T::epsilon() {
                        Some(T::one() / df_dx)
                    } else {
                        None
                    }
                } else {
                    None
                };

                Ok(SolveResult {
                    solution,
                    adjoint_factor,
                    solver_used: SolverType::NewtonRaphson,
                    iterations,
                    residual,
                })
            }
            Err(_) => {
                // Fall back to Brent method
                self.solve_with_brent(f, f_prime)
            }
        }
    }

    /// Solve using Brent method within configured brackets.
    fn solve_with_brent<F, G>(&self, f: F, f_prime: G) -> Result<SolveResult<T>, SolverError>
    where
        F: Fn(T) -> T,
        G: Fn(T) -> T,
    {
        let (a, b) = self.config.brent_bracket;

        match self.brent.find_root(&f, a, b) {
            Ok(solution) => {
                let residual = f(solution).abs();

                // Compute adjoint factor
                let adjoint_factor = if self.config.compute_adjoints {
                    let df_dx = f_prime(solution);
                    if df_dx.abs() > T::epsilon() {
                        Some(T::one() / df_dx)
                    } else {
                        None
                    }
                } else {
                    None
                };

                Ok(SolveResult {
                    solution,
                    adjoint_factor,
                    solver_used: SolverType::Brent,
                    iterations: 0,
                    residual,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Solve with parameter sensitivity computation.
    ///
    /// Computes dx/dθ for each parameter using the implicit function theorem:
    /// dx/dθ_i = -(∂f/∂θ_i) / (∂f/∂x)
    ///
    /// # Arguments
    ///
    /// * `f` - Function f(x, θ) to find root of
    /// * `f_prime_x` - Partial derivative ∂f/∂x
    /// * `f_prime_theta` - Partial derivatives ∂f/∂θ_i for each parameter
    /// * `x0` - Initial guess for x
    ///
    /// # Returns
    ///
    /// * `Ok(SolveResultWithSensitivities)` - Solution with sensitivities
    /// * `Err(SolverError)` - If solver fails
    ///
    /// # Example
    ///
    /// For a simple instrument: f(df, rate, T) = implied_rate(df, T) - rate = 0
    /// - ∂f/∂df = -1/(df² × T)  (for simple discounting)
    /// - ∂f/∂rate = -1
    /// - ∂f/∂T = ...
    ///
    /// Then: d(df)/d(rate) = -(-1) / (-1/(df²×T)) = -df²×T
    pub fn solve_with_sensitivities<F, G, H>(
        &self,
        f: F,
        f_prime_x: G,
        f_prime_theta: H,
        x0: T,
        num_params: usize,
    ) -> Result<SolveResultWithSensitivities, SolverError>
    where
        F: Fn(T) -> T,
        G: Fn(T) -> T,
        H: Fn(T, usize) -> T,
    {
        // Solve for x
        let result = self.solve(&f, &f_prime_x, x0)?;
        let x = result.solution;

        // Compute ∂f/∂x at solution
        let df_dx = f_prime_x(x);

        // Compute sensitivities using implicit function theorem
        let mut sensitivities = Vec::with_capacity(num_params);

        if df_dx.abs() > T::epsilon() {
            for i in 0..num_params {
                let df_dtheta = f_prime_theta(x, i);
                // dx/dθ_i = -(∂f/∂θ_i) / (∂f/∂x)
                let sensitivity = -(df_dtheta / df_dx);
                sensitivities.push(sensitivity.to_f64().unwrap_or(0.0));
            }
        } else {
            // Derivative near zero, sensitivities undefined
            sensitivities.resize(num_params, 0.0);
        }

        Ok(SolveResultWithSensitivities {
            solution: x.to_f64().unwrap_or(0.0),
            sensitivities,
            solver_used: result.solver_used,
            iterations: result.iterations,
        })
    }
}

/// Compute adjoint propagation for a solved implicit equation.
///
/// Given f(x, θ) = 0 solved for x, propagate adjoints:
/// θ̄ += x̄ × (-∂f/∂θ / ∂f/∂x)
///
/// This is a utility function for manual adjoint computation.
///
/// # Arguments
///
/// * `x_bar` - Adjoint of x (incoming gradient)
/// * `df_dx` - Partial derivative ∂f/∂x at solution
/// * `df_dtheta` - Partial derivative ∂f/∂θ at solution
///
/// # Returns
///
/// * Contribution to θ̄
pub fn compute_adjoint_contribution<T: Float>(x_bar: T, df_dx: T, df_dtheta: T) -> T {
    if df_dx.abs() > T::epsilon() {
        -x_bar * df_dtheta / df_dx
    } else {
        T::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Basic Solver Tests
    // ========================================

    #[test]
    fn test_solve_simple_root() {
        let solver: AdjointSolver<f64> = AdjointSolver::with_defaults();

        // Solve x² - 2 = 0 (find √2)
        let f = |x: f64| x * x - 2.0;
        let f_prime = |x: f64| 2.0 * x;

        let result = solver.solve(f, f_prime, 1.0).unwrap();

        assert!(
            (result.solution - std::f64::consts::SQRT_2).abs() < 1e-10,
            "Solution should be √2"
        );
        assert_eq!(result.solver_used, SolverType::NewtonRaphson);
    }

    #[test]
    fn test_solve_linear() {
        let solver: AdjointSolver<f64> = AdjointSolver::with_defaults();

        // Solve 2x - 6 = 0 (x = 3)
        let f = |x: f64| 2.0 * x - 6.0;
        let f_prime = |_x: f64| 2.0;

        let result = solver.solve(f, f_prime, 0.0).unwrap();

        assert!(
            (result.solution - 3.0).abs() < 1e-10,
            "Solution should be 3"
        );
    }

    #[test]
    fn test_solve_cubic() {
        let solver: AdjointSolver<f64> = AdjointSolver::with_defaults();

        // Solve x³ - x - 2 = 0 (has one real root near 1.52)
        let f = |x: f64| x * x * x - x - 2.0;
        let f_prime = |x: f64| 3.0 * x * x - 1.0;

        let result = solver.solve(f, f_prime, 1.5).unwrap();

        assert!(
            f(result.solution).abs() < 1e-10,
            "Residual should be near zero"
        );
    }

    // ========================================
    // Adjoint Factor Tests
    // ========================================

    #[test]
    fn test_adjoint_factor_computed() {
        let solver: AdjointSolver<f64> = AdjointSolver::with_defaults();

        let f = |x: f64| x * x - 4.0;
        let f_prime = |x: f64| 2.0 * x;

        let result = solver.solve(f, f_prime, 1.0).unwrap();

        // Solution should be 2, adjoint factor should be 1/(2*2) = 0.25
        assert!(result.adjoint_factor.is_some());
        let adjoint = result.adjoint_factor.unwrap();
        let expected_adjoint = 1.0 / (2.0 * 2.0); // 1/f'(2)
        assert!(
            (adjoint - expected_adjoint).abs() < 1e-10,
            "Adjoint factor should be 1/f'(x)"
        );
    }

    #[test]
    fn test_adjoint_factor_disabled() {
        let config = AdjointSolverConfig {
            compute_adjoints: false,
            ..Default::default()
        };
        let solver: AdjointSolver<f64> = AdjointSolver::new(config);

        let f = |x: f64| x * x - 4.0;
        let f_prime = |x: f64| 2.0 * x;

        let result = solver.solve(f, f_prime, 1.0).unwrap();

        assert!(
            result.adjoint_factor.is_none(),
            "Adjoint factor should not be computed"
        );
    }

    // ========================================
    // Sensitivity Tests
    // ========================================

    #[test]
    fn test_solve_with_sensitivities() {
        let solver: AdjointSolver<f64> = AdjointSolver::with_defaults();

        // f(x, θ) = x - θ = 0, so x = θ and dx/dθ = 1
        let f = |x: f64| x - 2.0; // θ = 2 is baked in
        let f_prime_x = |_x: f64| 1.0;
        let f_prime_theta = |_x: f64, _i: usize| -1.0; // ∂f/∂θ = -1

        let result = solver
            .solve_with_sensitivities(f, f_prime_x, f_prime_theta, 0.0, 1)
            .unwrap();

        assert!((result.solution - 2.0).abs() < 1e-10);
        // dx/dθ = -(-1)/1 = 1
        assert!(
            (result.sensitivities[0] - 1.0).abs() < 1e-10,
            "Sensitivity should be 1"
        );
    }

    #[test]
    fn test_solve_with_multiple_sensitivities() {
        let solver: AdjointSolver<f64> = AdjointSolver::with_defaults();

        // f(x, a, b) = x² - a*x - b = 0
        // For a=3, b=-4: x² - 3x + 4 = 0 has roots, but let's use x² - 2x - 3 = 0
        // which factors as (x-3)(x+1), so x = 3 or x = -1
        // Using a=2, b=3: x² - 2x - 3 = 0

        let a = 2.0;
        let b = 3.0;
        let f = |x: f64| x * x - a * x - b;
        let f_prime_x = |x: f64| 2.0 * x - a;
        let f_prime_theta = |x: f64, i: usize| {
            match i {
                0 => -x,   // ∂f/∂a = -x
                1 => -1.0, // ∂f/∂b = -1
                _ => 0.0,
            }
        };

        let result = solver
            .solve_with_sensitivities(f, f_prime_x, f_prime_theta, 2.0, 2)
            .unwrap();

        // Solution should be 3
        assert!(
            (result.solution - 3.0).abs() < 1e-6,
            "Solution should be 3, got {}",
            result.solution
        );

        // At x=3: ∂f/∂x = 2*3 - 2 = 4
        // dx/da = -(-3)/4 = 0.75
        // dx/db = -(-1)/4 = 0.25
        assert!(
            (result.sensitivities[0] - 0.75).abs() < 1e-6,
            "dx/da should be 0.75, got {}",
            result.sensitivities[0]
        );
        assert!(
            (result.sensitivities[1] - 0.25).abs() < 1e-6,
            "dx/db should be 0.25, got {}",
            result.sensitivities[1]
        );
    }

    // ========================================
    // Implicit Function Theorem Tests
    // ========================================

    #[test]
    fn test_compute_adjoint_contribution() {
        // For f(x, θ) = x - θ² = 0
        // At θ = 2, x = 4
        // ∂f/∂x = 1, ∂f/∂θ = -2θ = -4
        // x̄ = 1 (assuming downstream gradient is 1)
        // θ̄ += 1 × (-(-4)/1) = 4

        let x_bar = 1.0;
        let df_dx = 1.0;
        let df_dtheta = -4.0;

        let theta_bar = compute_adjoint_contribution(x_bar, df_dx, df_dtheta);

        assert!(
            (theta_bar - 4.0).abs() < 1e-10,
            "Adjoint contribution should be 4"
        );
    }

    #[test]
    fn test_compute_adjoint_contribution_zero_derivative() {
        let result = compute_adjoint_contribution(1.0, 0.0, 1.0);
        assert!(
            result.abs() < 1e-10,
            "Should return zero for zero derivative"
        );
    }

    // ========================================
    // Configuration Tests
    // ========================================

    #[test]
    fn test_config_default() {
        let config: AdjointSolverConfig<f64> = AdjointSolverConfig::default();
        assert!(config.tolerance < 1e-10);
        assert_eq!(config.max_iterations, 100);
        assert!(config.compute_adjoints);
    }

    #[test]
    fn test_custom_config() {
        let config = AdjointSolverConfig {
            tolerance: 1e-14,
            max_iterations: 500,
            brent_bracket: (0.0001, 10.0),
            compute_adjoints: true,
        };

        let solver: AdjointSolver<f64> = AdjointSolver::new(config);
        assert!(solver.config().tolerance < 1e-12);
        assert_eq!(solver.config().max_iterations, 500);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_clone() {
        let solver1: AdjointSolver<f64> = AdjointSolver::with_defaults();
        let solver2 = solver1.clone();

        assert_eq!(
            solver1.config().max_iterations,
            solver2.config().max_iterations
        );
    }

    #[test]
    fn test_solve_result_clone() {
        let result = SolveResult {
            solution: 1.5,
            adjoint_factor: Some(0.5),
            solver_used: SolverType::NewtonRaphson,
            iterations: 5,
            residual: 1e-12,
        };

        let cloned = result.clone();
        assert!((cloned.solution - result.solution).abs() < 1e-15);
    }
}
