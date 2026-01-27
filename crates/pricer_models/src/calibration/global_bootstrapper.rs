//! Global curve bootstrapping with simultaneous discount factor calibration.
//!
//! This module provides `GlobalBootstrapper<T>` which calibrates all discount
//! factors simultaneously by solving a system of nonlinear equations where
//! each equation represents the pricing error for a market instrument.
//!
//! ## Key Features
//!
//! - Simultaneous calibration of all discount factors
//! - Jacobian inverse storage for AAD sensitivity computation
//! - Supports all instrument types via `CalibrationInstrument` trait
//! - Configurable convergence tolerances
//!
//! ## Comparison with Sequential Bootstrapping
//!
//! | Aspect | Sequential | Global |
//! |--------|-----------|--------|
//! | Speed | O(n) solves | O(1) solve, O(n³) per iteration |
//! | AAD | Requires implicit function theorem per pillar | Single J⁻¹ captures all sensitivities |
//! | Flexibility | Forward-starting only | Any instrument structure |
//! | Stability | Stable for well-ordered instruments | May require damping for ill-conditioned |
//!
//! ## Example
//!
//! ```ignore
//! use pricer_models::market::calibration::bootstrapping::{
//!     GlobalBootstrapper, GlobalBootstrapConfig, BootstrapInstrument,
//! };
//!
//! let instruments = vec![
//!     BootstrapInstrument::ois(1.0, 0.03),
//!     BootstrapInstrument::ois(2.0, 0.035),
//!     BootstrapInstrument::ois(5.0, 0.04),
//!     BootstrapInstrument::ois(10.0, 0.045),
//! ];
//!
//! let config = GlobalBootstrapConfig::default();
//! let bootstrapper = GlobalBootstrapper::new(config);
//!
//! let result = bootstrapper.calibrate(&instruments)?;
//! let curve = result.curve;
//! ```

use num_traits::Float;

use pricer_core::math::linalg::{DMatrix, LinearAlgebraError, RealField, lu_solve};
use pricer_core::math::numeric::from_f64;
use pricer_core::types::SolverError;

use crate::market::calibration::bootstrapping::{
    BootstrapInterpolation, BootstrappedCurve, CalibrationInstrument,
};

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for global bootstrapping.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for tolerance values
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalBootstrapConfig<T: Float> {
    /// Convergence tolerance for residual norm (||F(x)||).
    pub tolerance: T,

    /// Convergence tolerance for parameter change (||Δx||).
    pub param_tolerance: T,

    /// Maximum number of Newton iterations.
    pub max_iterations: usize,

    /// Step size for numerical Jacobian approximation.
    pub jacobian_epsilon: T,

    /// Whether to store the Jacobian inverse for AAD.
    pub store_jacobian_inverse: bool,

    /// Interpolation method for the output curve.
    pub interpolation: BootstrapInterpolation,

    /// Whether to allow extrapolation in the output curve.
    pub allow_extrapolation: bool,
}

impl<T: Float> Default for GlobalBootstrapConfig<T> {
    fn default() -> Self {
        Self {
            tolerance: from_f64(1e-10),
            param_tolerance: from_f64(1e-10),
            max_iterations: 100,
            jacobian_epsilon: from_f64(1e-8),
            store_jacobian_inverse: true,
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
        }
    }
}

impl<T: Float> GlobalBootstrapConfig<T> {
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
    pub fn high_precision() -> Self {
        Self {
            tolerance: from_f64(1e-14),
            param_tolerance: from_f64(1e-14),
            max_iterations: 500,
            jacobian_epsilon: from_f64(1e-10),
            store_jacobian_inverse: true,
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
        }
    }

    /// Create a fast configuration with relaxed tolerances.
    pub fn fast() -> Self {
        Self {
            tolerance: from_f64(1e-6),
            param_tolerance: from_f64(1e-6),
            max_iterations: 50,
            jacobian_epsilon: from_f64(1e-6),
            store_jacobian_inverse: false,
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
        }
    }

    /// Set the interpolation method.
    pub fn with_interpolation(mut self, method: BootstrapInterpolation) -> Self {
        self.interpolation = method;
        self
    }

    /// Enable or disable Jacobian inverse storage.
    pub fn with_jacobian_inverse(mut self, store: bool) -> Self {
        self.store_jacobian_inverse = store;
        self
    }
}

// =============================================================================
// Calibration Result
// =============================================================================

/// Result of global bootstrapping.
///
/// Contains the calibrated curve, convergence information, and optionally
/// the Jacobian inverse for AAD sensitivity computation.
#[derive(Debug, Clone)]
pub struct GlobalBootstrapResult<T: Float> {
    /// The calibrated yield curve.
    pub curve: BootstrappedCurve<T>,

    /// Pillar maturities in years.
    pub pillars: Vec<T>,

    /// Calibrated discount factors at each pillar.
    pub discount_factors: Vec<T>,

    /// Final residual norm ||F(x*)||.
    pub residual_norm: T,

    /// Number of Newton iterations performed.
    pub iterations: usize,

    /// Whether the calibration converged within tolerance.
    pub converged: bool,

    /// Jacobian inverse at the solution (for AAD).
    ///
    /// This is J⁻¹ where J[i,j] = ∂pricing_error_i / ∂log_df_j.
    /// Used for computing curve sensitivities via implicit function theorem.
    pub jacobian_inverse: Option<DMatrix<T>>,
}

impl<T: Float> GlobalBootstrapResult<T> {
    /// Check if the Jacobian inverse is available.
    pub fn has_jacobian_inverse(&self) -> bool {
        self.jacobian_inverse.is_some()
    }
}

// =============================================================================
// Global Bootstrapper
// =============================================================================

/// Global curve bootstrapper using multi-dimensional Newton-Raphson.
///
/// Solves the system F(x) = 0 where:
/// - x = log(DF) at each pillar (ensures DF > 0)
/// - F_i = pricing_error(instrument_i, curve)
///
/// The Jacobian J[i,j] = ∂F_i/∂x_j is computed numerically via finite
/// differences.
#[derive(Debug, Clone)]
pub struct GlobalBootstrapper<T: Float> {
    config: GlobalBootstrapConfig<T>,
}

impl<T: RealField + Float + Copy> GlobalBootstrapper<T> {
    /// Create a new global bootstrapper with the given configuration.
    pub fn new(config: GlobalBootstrapConfig<T>) -> Self {
        Self { config }
    }

    /// Create a bootstrapper with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(GlobalBootstrapConfig::default())
    }

    /// Get the configuration.
    pub fn config(&self) -> &GlobalBootstrapConfig<T> {
        &self.config
    }

    /// Calibrate a yield curve from the given instruments.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Market instruments to calibrate against
    ///
    /// # Returns
    ///
    /// * `Ok(GlobalBootstrapResult<T>)` - Calibrated curve and diagnostics
    /// * `Err(SolverError)` - If calibration fails
    ///
    /// # Errors
    ///
    /// - `SolverError::MaxIterationsExceeded`: Didn't converge within limit
    /// - `SolverError::SingularJacobian`: Jacobian is singular
    /// - `SolverError::NumericalInstability`: Numerical issues during solve
    pub fn calibrate<I: CalibrationInstrument<T>>(
        &self,
        instruments: &[I],
    ) -> Result<GlobalBootstrapResult<T>, SolverError> {
        let n = instruments.len();
        if n == 0 {
            return Err(SolverError::NumericalInstability(
                "No instruments provided for calibration".to_string(),
            ));
        }

        // Extract pillars (maturities) from instruments
        let mut pillars: Vec<T> = instruments.iter().map(|i| i.maturity()).collect();
        pillars.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Deduplicate pillars (same maturity instruments share pillar)
        pillars.dedup_by(|a, b| Float::abs(*a - *b) < from_f64::<T>(1e-10));

        let n_pillars = pillars.len();

        // Initial guess: log(DF) assuming flat 3% curve
        let mut x: Vec<T> = pillars
            .iter()
            .map(|&t| -(from_f64::<T>(0.03) * t))
            .collect();

        // Newton iteration
        for iter in 0..self.config.max_iterations {
            // Build curve from current x = log(DF)
            let discount_factors: Vec<T> = x.iter().map(|&xi| Float::exp(xi)).collect();
            let curve = self.build_curve(&pillars, &discount_factors)?;

            // Compute residual vector F(x)
            let residuals = self.compute_residuals(instruments, &curve)?;
            let residual_norm = vector_norm(&residuals);

            // Check convergence
            if residual_norm < self.config.tolerance {
                // Compute Jacobian inverse if requested
                let jacobian_inverse = if self.config.store_jacobian_inverse {
                    let j_vecs = self.compute_jacobian(&x, &pillars, instruments)?;
                    let j_matrix =
                        DMatrix::from_row_slice(n, n_pillars, &self.flatten_jacobian(&j_vecs));
                    Some(self.compute_inverse(&j_matrix)?)
                } else {
                    None
                };

                return Ok(GlobalBootstrapResult {
                    curve,
                    pillars: pillars.clone(),
                    discount_factors,
                    residual_norm,
                    iterations: iter,
                    converged: true,
                    jacobian_inverse,
                });
            }

            // Compute Jacobian
            let j = self.compute_jacobian(&x, &pillars, instruments)?;

            // Solve J * delta = -F for delta
            let neg_residuals: Vec<T> = residuals.iter().map(|&r| -r).collect();
            let j_matrix = DMatrix::from_row_slice(n, n_pillars, &self.flatten_jacobian(&j));
            let delta = self.solve_linear_system(&j_matrix, &neg_residuals)?;

            // Check parameter convergence
            let param_change = vector_norm(&delta);
            if param_change < self.config.param_tolerance {
                let jacobian_inverse = if self.config.store_jacobian_inverse {
                    Some(self.compute_inverse(&j_matrix)?)
                } else {
                    None
                };

                return Ok(GlobalBootstrapResult {
                    curve,
                    pillars: pillars.clone(),
                    discount_factors,
                    residual_norm,
                    iterations: iter,
                    converged: true,
                    jacobian_inverse,
                });
            }

            // Update x
            for (i, d) in delta.iter().enumerate() {
                x[i] = x[i] + *d;
            }
        }

        // Max iterations exceeded
        Err(SolverError::MaxIterationsExceeded {
            iterations: self.config.max_iterations,
        })
    }

    /// Build a curve from pillars and discount factors.
    fn build_curve(
        &self,
        pillars: &[T],
        discount_factors: &[T],
    ) -> Result<BootstrappedCurve<T>, SolverError> {
        BootstrappedCurve::new(
            pillars.to_vec(),
            discount_factors.to_vec(),
            self.config.interpolation,
            self.config.allow_extrapolation,
        )
        .map_err(|e| SolverError::NumericalInstability(e))
    }

    /// Compute the residual vector (pricing errors).
    fn compute_residuals<I: CalibrationInstrument<T>>(
        &self,
        instruments: &[I],
        curve: &BootstrappedCurve<T>,
    ) -> Result<Vec<T>, SolverError> {
        instruments
            .iter()
            .map(|instr| {
                instr
                    .pricing_error(curve)
                    .map_err(|e| SolverError::NumericalInstability(format!("{e}")))
            })
            .collect()
    }

    /// Compute the Jacobian matrix via finite differences.
    ///
    /// J[i,j] = ∂F_i/∂x_j where x_j = log(DF_j)
    fn compute_jacobian<I: CalibrationInstrument<T>>(
        &self,
        x: &[T],
        pillars: &[T],
        instruments: &[I],
    ) -> Result<Vec<Vec<T>>, SolverError> {
        let n = instruments.len();
        let m = pillars.len();
        let eps = self.config.jacobian_epsilon;

        // Base residuals
        let discount_factors: Vec<T> = x.iter().map(|&xi| Float::exp(xi)).collect();
        let curve = self.build_curve(pillars, &discount_factors)?;
        let f0 = self.compute_residuals(instruments, &curve)?;

        // Compute Jacobian columns via finite differences
        let mut jacobian = vec![vec![T::zero(); m]; n];

        for j in 0..m {
            // Perturb x[j]
            let mut x_pert = x.to_vec();
            x_pert[j] = x_pert[j] + eps;

            let df_pert: Vec<T> = x_pert.iter().map(|&xi| Float::exp(xi)).collect();
            let curve_pert = self.build_curve(pillars, &df_pert)?;
            let f_pert = self.compute_residuals(instruments, &curve_pert)?;

            for i in 0..n {
                jacobian[i][j] = (f_pert[i] - f0[i]) / eps;
            }
        }

        Ok(jacobian)
    }

    /// Flatten the Jacobian for matrix construction.
    fn flatten_jacobian(&self, j: &[Vec<T>]) -> Vec<T> {
        j.iter().flat_map(|row| row.iter().copied()).collect()
    }

    /// Solve the linear system J * x = b.
    fn solve_linear_system(
        &self,
        j: &DMatrix<T>,
        b: &[T],
    ) -> Result<Vec<T>, SolverError> {
        lu_solve(j, b).map_err(|e: LinearAlgebraError| e.into())
    }

    /// Compute the inverse of the Jacobian matrix.
    fn compute_inverse(&self, j: &DMatrix<T>) -> Result<DMatrix<T>, SolverError> {
        pricer_core::math::linalg::inverse(j).map_err(|e: LinearAlgebraError| e.into())
    }
}

/// Compute the Euclidean norm of a vector.
fn vector_norm<T: Float>(v: &[T]) -> T {
    let sum_sq = v.iter().fold(T::zero(), |acc, &x| acc + x * x);
    sum_sq.sqrt()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::calibration::bootstrapping::BootstrapInstrument;
    use approx::assert_relative_eq;

    fn create_test_instruments() -> Vec<BootstrapInstrument<f64>> {
        vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
            BootstrapInstrument::ois(5.0, 0.035),
            BootstrapInstrument::ois(10.0, 0.04),
        ]
    }

    #[test]
    fn test_config_default() {
        let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::default();
        assert_relative_eq!(config.tolerance, 1e-10, epsilon = 1e-15);
        assert_eq!(config.max_iterations, 100);
        assert!(config.store_jacobian_inverse);
    }

    #[test]
    fn test_config_high_precision() {
        let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::high_precision();
        assert!(config.tolerance < 1e-12);
        assert!(config.max_iterations >= 500);
    }

    #[test]
    fn test_config_fast() {
        let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::fast();
        assert!(config.tolerance > 1e-8);
        assert!(!config.store_jacobian_inverse);
    }

    #[test]
    fn test_calibrate_basic() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default();
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        assert!(result.converged);
        assert_eq!(result.pillars.len(), 4);
        assert_eq!(result.discount_factors.len(), 4);

        // Verify discount factors are positive and decreasing
        for i in 0..result.discount_factors.len() {
            assert!(result.discount_factors[i] > 0.0);
            assert!(result.discount_factors[i] <= 1.0);
        }

        // Verify pricing errors are small
        for (i, instr) in instruments.iter().enumerate() {
            let error = instr.pricing_error(&result.curve).unwrap();
            assert!(
                error.abs() < 1e-8,
                "Instrument {} has pricing error {}",
                i,
                error
            );
        }
    }

    #[test]
    fn test_calibrate_stores_jacobian_inverse() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default().with_jacobian_inverse(true);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        assert!(result.has_jacobian_inverse());
        let j_inv = result.jacobian_inverse.as_ref().unwrap();
        assert_eq!(j_inv.nrows(), 4);
        assert_eq!(j_inv.ncols(), 4);
    }

    #[test]
    fn test_calibrate_without_jacobian_inverse() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::fast();
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        assert!(result.converged);
        assert!(!result.has_jacobian_inverse());
    }

    #[test]
    fn test_calibrate_empty_instruments_error() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![];
        let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

        let result = bootstrapper.calibrate(&instruments);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SolverError::NumericalInstability(_)
        ));
    }

    #[test]
    fn test_calibrate_single_instrument() {
        let instruments = vec![BootstrapInstrument::ois(5.0, 0.03)];
        let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

        let result = bootstrapper.calibrate(&instruments).unwrap();

        assert!(result.converged);
        assert_eq!(result.pillars.len(), 1);

        // Verify pricing error is small
        let error = instruments[0].pricing_error(&result.curve).unwrap();
        assert!(error.abs() < 1e-8);
    }

    #[test]
    fn test_calibrate_upward_sloping_curve() {
        // Upward sloping rate curve
        let instruments = vec![
            BootstrapInstrument::ois(1.0, 0.02),
            BootstrapInstrument::ois(2.0, 0.025),
            BootstrapInstrument::ois(5.0, 0.03),
            BootstrapInstrument::ois(10.0, 0.035),
            BootstrapInstrument::ois(30.0, 0.04),
        ];

        let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();
        let result = bootstrapper.calibrate(&instruments).unwrap();

        assert!(result.converged);

        // Verify all pricing errors are small
        for (i, instr) in instruments.iter().enumerate() {
            let error = instr.pricing_error(&result.curve).unwrap();
            assert!(
                error.abs() < 1e-8,
                "Instrument {} has pricing error {}",
                i,
                error
            );
        }
    }

    #[test]
    fn test_calibrate_inverted_curve() {
        // Inverted rate curve
        let instruments = vec![
            BootstrapInstrument::ois(1.0, 0.05),
            BootstrapInstrument::ois(2.0, 0.045),
            BootstrapInstrument::ois(5.0, 0.04),
            BootstrapInstrument::ois(10.0, 0.035),
        ];

        let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();
        let result = bootstrapper.calibrate(&instruments).unwrap();

        assert!(result.converged);

        // Verify all pricing errors are small
        for instr in &instruments {
            let error = instr.pricing_error(&result.curve).unwrap();
            assert!(error.abs() < 1e-8);
        }
    }

    #[test]
    fn test_vector_norm() {
        let v = vec![3.0, 4.0];
        assert_relative_eq!(vector_norm(&v), 5.0, epsilon = 1e-10);

        let v2 = vec![1.0, 1.0, 1.0, 1.0];
        assert_relative_eq!(vector_norm(&v2), 2.0, epsilon = 1e-10);
    }
}
