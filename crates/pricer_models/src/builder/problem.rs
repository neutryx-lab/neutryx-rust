//! Calibration problem abstraction for multi-dimensional solvers.
//!
//! This module provides `CalibrationProblem<T, I>` which implements
//! `SystemOfEquations<T>` for use with Newton-based solvers.
//!
//! # Usage
//!
//! - **Curve calibration**: Solve for log(DF) at each pillar
//! - **Vol surface calibration**: Solve for SABR parameters at each slice

use num_traits::Float;
use pricer_core::{
    math::{
        linalg::{DMatrix, DVector, RealField},
        numeric::from_f64,
        solvers::SystemOfEquations,
    },
    types::SolverError,
};

use super::{CalibrationError, CalibrationInstrument};
use crate::market::curves::{BootstrapInterpolation, BootstrappedCurve};

// =============================================================================
// JacobianMethod
// =============================================================================

/// Method for computing the Jacobian matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JacobianMethod {
    /// Analytical differentiation using matrix products.
    Analytical,

    /// Finite difference approximation (default).
    #[default]
    FiniteDifference,

    /// Central difference approximation.
    CentralDifference,

    /// Automatic differentiation (Enzyme AD) - future extension.
    #[cfg(feature = "enzyme-ad")]
    AutomaticDifferentiation,
}

// =============================================================================
// CalibrationProblemConfig
// =============================================================================

/// Configuration for calibration problems.
#[derive(Debug, Clone, Copy)]
pub struct CalibrationProblemConfig<T: Float> {
    /// Jacobian computation method.
    pub jacobian_method: JacobianMethod,
    /// Epsilon for finite difference Jacobian.
    pub jacobian_epsilon: T,
    /// Interpolation method for curves.
    pub interpolation: BootstrapInterpolation,
    /// Whether to allow extrapolation.
    pub allow_extrapolation: bool,
}

impl<T: Float> Default for CalibrationProblemConfig<T> {
    fn default() -> Self {
        Self {
            jacobian_method: JacobianMethod::FiniteDifference,
            jacobian_epsilon: from_f64(1e-8),
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
        }
    }
}

// =============================================================================
// CalibrationProblem
// =============================================================================

/// Calibration problem as a system of equations F(x) = 0.
///
/// The unknowns x = log(DF) at each pillar maturity, and F_i(x) is the
/// pricing error for instrument i evaluated on the curve implied by x.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for calculations
/// * `I` - Instrument type implementing `CalibrationInstrument<T>`
#[derive(Debug, Clone)]
pub struct CalibrationProblem<T: Float, I: CalibrationInstrument<T>> {
    /// Calibration instruments.
    instruments: Vec<I>,
    /// Pillar maturities (sorted).
    pillars: Vec<T>,
    /// Configuration for the calibration.
    config: CalibrationProblemConfig<T>,
}

impl<T, I> CalibrationProblem<T, I>
where
    T: Float + RealField + Copy,
    I: CalibrationInstrument<T> + Clone,
{
    /// Create a new calibration problem from instruments.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Calibration instruments
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Calibration problem
    /// * `Err(CalibrationError)` - If validation fails
    pub fn new(instruments: Vec<I>) -> Result<Self, CalibrationError> {
        Self::with_config(instruments, CalibrationProblemConfig::default())
    }

    /// Create a new calibration problem with custom configuration.
    pub fn with_config(
        instruments: Vec<I>,
        config: CalibrationProblemConfig<T>,
    ) -> Result<Self, CalibrationError> {
        if instruments.is_empty() {
            return Err(CalibrationError::no_instruments());
        }

        // Extract and sort pillars (maturities)
        let mut pillars: Vec<T> = instruments.iter().map(|i| i.maturity()).collect();
        pillars.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Deduplicate pillars
        pillars.dedup_by(|a, b| Float::abs(*a - *b) < from_f64(1e-10));

        // Dimension check: enforce square system for now
        if instruments.len() != pillars.len() {
            return Err(CalibrationError::dimension_mismatch(
                instruments.len(),
                pillars.len(),
            ));
        }

        Ok(Self {
            instruments,
            pillars,
            config,
        })
    }

    /// Get the instruments.
    pub fn instruments(&self) -> &[I] { &self.instruments }

    /// Get the pillars.
    pub fn pillars(&self) -> &[T] { &self.pillars }

    /// Get the configuration.
    pub fn config(&self) -> &CalibrationProblemConfig<T> { &self.config }

    /// Build a yield curve from log discount factors.
    ///
    /// # Arguments
    ///
    /// * `log_df` - log(DF) at each pillar
    ///
    /// # Returns
    ///
    /// A `BootstrappedCurve` constructed from the pillar discount factors.
    pub fn build_curve(&self, log_df: &[T]) -> Result<BootstrappedCurve<T>, SolverError> {
        let discount_factors: Vec<T> = log_df.iter().map(|&x| Float::exp(x)).collect();

        BootstrappedCurve::new(
            self.pillars.clone(),
            discount_factors,
            self.config.interpolation,
            self.config.allow_extrapolation,
        )
        .map_err(SolverError::NumericalInstability)
    }

    /// Compute residuals (pricing errors) for all instruments.
    ///
    /// # Arguments
    ///
    /// * `curve` - Yield curve to price against
    ///
    /// # Returns
    ///
    /// Vector of pricing errors, one per instrument.
    pub fn compute_residuals(
        &self,
        curve: &BootstrappedCurve<T>,
    ) -> Result<Vec<T>, CalibrationError> {
        let mut residuals = Vec::with_capacity(self.instruments.len());

        for (idx, instrument) in self.instruments.iter().enumerate() {
            let error = instrument
                .pricing_error(curve)
                .map_err(|e| CalibrationError::instrument_evaluation_failed(idx, e.to_string()))?;
            residuals.push(error);
        }

        Ok(residuals)
    }

    /// Compute the Jacobian matrix using finite differences.
    ///
    /// J[i,j] = ∂F_i/∂x_j where x_j = log(DF_j)
    ///
    /// # Arguments
    ///
    /// * `log_df` - Current log discount factors
    ///
    /// # Returns
    ///
    /// Jacobian matrix as DMatrix.
    pub fn compute_jacobian_finite_diff(
        &self,
        log_df: &[T],
    ) -> Result<DMatrix<T>, CalibrationError> {
        let n = self.instruments.len();
        let m = self.pillars.len();
        let eps = self.config.jacobian_epsilon;

        // Base residuals
        let curve = self.build_curve(log_df).map_err(|e| {
            CalibrationError::numerical_instability(format!("Failed to build curve: {e}"))
        })?;
        let f0 = self.compute_residuals(&curve)?;

        // Compute Jacobian columns via forward differences
        let mut jacobian = DMatrix::zeros(n, m);

        for j in 0..m {
            let mut log_df_pert = log_df.to_vec();
            log_df_pert[j] = log_df_pert[j] + eps;

            let curve_pert = self.build_curve(&log_df_pert).map_err(|e| {
                CalibrationError::numerical_instability(format!(
                    "Failed to build perturbed curve: {e}"
                ))
            })?;
            let f_pert = self.compute_residuals(&curve_pert)?;

            for i in 0..n {
                jacobian[(i, j)] = (f_pert[i] - f0[i]) / eps;
            }
        }

        Ok(jacobian)
    }

    /// Compute the Jacobian matrix using central differences.
    pub fn compute_jacobian_central_diff(
        &self,
        log_df: &[T],
    ) -> Result<DMatrix<T>, CalibrationError> {
        let n = self.instruments.len();
        let m = self.pillars.len();
        let eps = self.config.jacobian_epsilon;

        let mut jacobian = DMatrix::zeros(n, m);

        for j in 0..m {
            let mut log_df_plus = log_df.to_vec();
            log_df_plus[j] = log_df_plus[j] + eps;
            let curve_plus = self.build_curve(&log_df_plus).map_err(|e| {
                CalibrationError::numerical_instability(format!("Failed to build curve+: {e}"))
            })?;
            let f_plus = self.compute_residuals(&curve_plus)?;

            let mut log_df_minus = log_df.to_vec();
            log_df_minus[j] = log_df_minus[j] - eps;
            let curve_minus = self.build_curve(&log_df_minus).map_err(|e| {
                CalibrationError::numerical_instability(format!("Failed to build curve-: {e}"))
            })?;
            let f_minus = self.compute_residuals(&curve_minus)?;

            for i in 0..n {
                jacobian[(i, j)] = (f_plus[i] - f_minus[i]) / (eps + eps);
            }
        }

        Ok(jacobian)
    }

    /// Create an initial guess for log discount factors.
    ///
    /// Uses a flat 3% rate assumption: log(DF(t)) = -0.03 * t
    pub fn initial_guess(&self) -> Vec<T> {
        self.pillars
            .iter()
            .map(|&t| -(from_f64::<T>(0.03) * t))
            .collect()
    }

    /// Create an initial guess DVector for the solver.
    pub fn initial_guess_vector(&self) -> DVector<T> { DVector::from_vec(self.initial_guess()) }
}

// =============================================================================
// SystemOfEquations Implementation
// =============================================================================

impl<T, I> SystemOfEquations<T> for CalibrationProblem<T, I>
where
    T: Float + RealField + Copy,
    I: CalibrationInstrument<T> + Clone,
{
    fn dimension(&self) -> usize { self.instruments.len() }

    fn evaluate(&self, x: &DVector<T>) -> Result<DVector<T>, SolverError> {
        let log_df: Vec<T> = x.iter().copied().collect();

        let curve = self.build_curve(&log_df)?;

        let residuals = self.compute_residuals(&curve).map_err(|e| {
            SolverError::NumericalInstability(format!("Residual computation failed: {e}"))
        })?;

        Ok(DVector::from_vec(residuals))
    }

    fn jacobian(&self, x: &DVector<T>) -> Result<DMatrix<T>, SolverError> {
        let log_df: Vec<T> = x.iter().copied().collect();

        let jacobian = match self.config.jacobian_method {
            JacobianMethod::Analytical => self.compute_jacobian_finite_diff(&log_df),
            JacobianMethod::FiniteDifference => self.compute_jacobian_finite_diff(&log_df),
            JacobianMethod::CentralDifference => self.compute_jacobian_central_diff(&log_df),
            #[cfg(feature = "enzyme-ad")]
            JacobianMethod::AutomaticDifferentiation => self.compute_jacobian_finite_diff(&log_df),
        };

        jacobian.map_err(|e| {
            SolverError::NumericalInstability(format!("Jacobian computation failed: {e}"))
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
    use crate::market::curves::{MarketInstrument, YieldCurve};

    fn create_test_instruments() -> Vec<MarketInstrument<f64>> {
        vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
            MarketInstrument::ois(5.0, 0.04),
        ]
    }

    #[test]
    fn test_problem_creation() {
        let instruments = create_test_instruments();
        let problem = CalibrationProblem::new(instruments).unwrap();

        assert_eq!(problem.dimension(), 3);
        assert_eq!(problem.instruments().len(), 3);
        assert_eq!(problem.pillars().len(), 3);
    }

    #[test]
    fn test_problem_empty_instruments() {
        let instruments: Vec<MarketInstrument<f64>> = vec![];
        let result = CalibrationProblem::new(instruments);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CalibrationError::NoInstruments
        ));
    }

    #[test]
    fn test_problem_initial_guess() {
        let instruments = create_test_instruments();
        let problem = CalibrationProblem::new(instruments).unwrap();

        let guess = problem.initial_guess();
        assert_eq!(guess.len(), 3);

        assert_relative_eq!(guess[0], -0.03, epsilon = 1e-10);
        assert_relative_eq!(guess[1], -0.06, epsilon = 1e-10);
        assert_relative_eq!(guess[2], -0.15, epsilon = 1e-10);
    }

    #[test]
    fn test_problem_build_curve() {
        let instruments = create_test_instruments();
        let problem = CalibrationProblem::new(instruments).unwrap();

        let log_df = problem.initial_guess();
        let curve = problem.build_curve(&log_df).unwrap();

        let df_1y = curve.discount_factor(1.0).unwrap();
        let df_2y = curve.discount_factor(2.0).unwrap();
        let df_5y = curve.discount_factor(5.0).unwrap();

        assert_relative_eq!(df_1y, (-0.03f64).exp(), epsilon = 1e-8);
        assert_relative_eq!(df_2y, (-0.06f64).exp(), epsilon = 1e-8);
        assert_relative_eq!(df_5y, (-0.15f64).exp(), epsilon = 1e-8);
    }

    #[test]
    fn test_problem_evaluate() {
        let instruments = create_test_instruments();
        let problem = CalibrationProblem::new(instruments).unwrap();

        let x = problem.initial_guess_vector();
        let residuals = problem.evaluate(&x).unwrap();

        assert_eq!(residuals.len(), 3);
    }

    #[test]
    fn test_problem_jacobian() {
        let instruments = create_test_instruments();
        let problem = CalibrationProblem::new(instruments).unwrap();

        let x = problem.initial_guess_vector();
        let jacobian = problem.jacobian(&x).unwrap();

        assert_eq!(jacobian.nrows(), 3);
        assert_eq!(jacobian.ncols(), 3);

        let max_elem: f64 = jacobian.iter().map(|&x| x.abs()).fold(0.0, f64::max);
        assert!(max_elem > 0.0);
    }

    #[test]
    fn test_problem_with_config() {
        let instruments = create_test_instruments();
        let config = CalibrationProblemConfig {
            jacobian_method: JacobianMethod::CentralDifference,
            jacobian_epsilon: 1e-6,
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
        };

        let problem = CalibrationProblem::with_config(instruments, config).unwrap();

        assert_eq!(
            problem.config().jacobian_method,
            JacobianMethod::CentralDifference
        );
        assert_relative_eq!(problem.config().jacobian_epsilon, 1e-6, epsilon = 1e-15);
    }

    #[test]
    fn test_jacobian_method_default() {
        assert_eq!(JacobianMethod::default(), JacobianMethod::FiniteDifference);
    }
}
