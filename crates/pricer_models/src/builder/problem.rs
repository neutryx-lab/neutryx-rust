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

use super::{CalibrationError, CalibrationInstrument, JumpPillar};
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
/// When jump calibration is enabled, the parameter vector is extended to:
/// `[log(DF_1), ..., log(DF_n), jump_1, ..., jump_m]`
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
    /// Jump pillars for CB meeting dates (optional).
    jump_pillars: Vec<JumpPillar<T>>,
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
            jump_pillars: Vec::new(),
        })
    }

    /// Create a new calibration problem with jump pillars.
    ///
    /// The parameter vector is extended to include jump parameters:
    /// `[log(DF_1), ..., log(DF_n), jump_1, ..., jump_m]`
    ///
    /// # Arguments
    ///
    /// * `instruments` - Calibration instruments
    /// * `jump_pillars` - Jump pillars for CB meeting dates
    /// * `config` - Configuration options
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Calibration problem with jumps
    /// * `Err(CalibrationError)` - If validation fails
    pub fn with_jumps(
        instruments: Vec<I>,
        mut jump_pillars: Vec<JumpPillar<T>>,
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

        // Sort jump pillars by time
        jump_pillars.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Set parameter indices for jump pillars
        let n_pillars = pillars.len();
        for (i, jp) in jump_pillars.iter_mut().enumerate() {
            jp.set_param_index(n_pillars + i);
        }

        Ok(Self {
            instruments,
            pillars,
            config,
            jump_pillars,
        })
    }

    /// Get the instruments.
    pub fn instruments(&self) -> &[I] { &self.instruments }

    /// Get the pillars.
    pub fn pillars(&self) -> &[T] { &self.pillars }

    /// Get the configuration.
    pub fn config(&self) -> &CalibrationProblemConfig<T> { &self.config }

    /// Get the jump pillars.
    pub fn jump_pillars(&self) -> &[JumpPillar<T>] { &self.jump_pillars }

    /// Check if this problem has jump pillars.
    pub fn has_jumps(&self) -> bool { !self.jump_pillars.is_empty() }

    /// Get the number of jump pillars.
    pub fn num_jumps(&self) -> usize { self.jump_pillars.len() }

    /// Get the total dimension of the parameter vector.
    ///
    /// This is n_pillars + n_jumps when jumps are present.
    pub fn total_dimension(&self) -> usize { self.pillars.len() + self.jump_pillars.len() }

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

    // =========================================================================
    // Jump-aware calibration methods
    // =========================================================================

    /// Create an initial guess for extended parameter vector including jumps.
    ///
    /// Returns `[log(DF_1), ..., log(DF_n), jump_1, ..., jump_m]`
    /// where jumps are initialised to their expected values.
    pub fn initial_guess_with_jumps(&self) -> Vec<T> {
        let mut guess = self.initial_guess();

        // Append expected jump values
        for jp in &self.jump_pillars {
            guess.push(jp.expected_jump);
        }

        guess
    }

    /// Create an initial guess DVector including jump parameters.
    pub fn initial_guess_vector_with_jumps(&self) -> DVector<T> {
        DVector::from_vec(self.initial_guess_with_jumps())
    }

    /// Extract log discount factors from an extended parameter vector.
    ///
    /// # Arguments
    ///
    /// * `params` - Extended parameter vector `[log(DF), ..., jumps, ...]`
    ///
    /// # Returns
    ///
    /// Slice of log discount factors.
    pub fn extract_log_df<'a>(&self, params: &'a [T]) -> &'a [T] { &params[..self.pillars.len()] }

    /// Extract jump values from an extended parameter vector.
    ///
    /// # Arguments
    ///
    /// * `params` - Extended parameter vector `[log(DF), ..., jumps, ...]`
    ///
    /// # Returns
    ///
    /// Slice of jump values (absolute rate).
    pub fn extract_jumps<'a>(&self, params: &'a [T]) -> &'a [T] { &params[self.pillars.len()..] }

    /// Build a yield curve with jump adjustments applied.
    ///
    /// The discount factors are adjusted for jumps by multiplying with
    /// the cumulative jump effect: DF_adjusted = DF * Π(1 + jump_i)
    /// for all jumps i where t_jump <= t_pillar.
    ///
    /// # Arguments
    ///
    /// * `log_df` - Log discount factors at pillars
    /// * `jumps` - Jump values in absolute rate
    ///
    /// # Returns
    ///
    /// A `BootstrappedCurve` with jump-adjusted discount factors.
    pub fn build_curve_with_jumps(
        &self,
        log_df: &[T],
        jumps: &[T],
    ) -> Result<BootstrappedCurve<T>, SolverError> {
        // Compute base discount factors
        let base_dfs: Vec<T> = log_df.iter().map(|&x| Float::exp(x)).collect();

        // Apply jump adjustments to each pillar
        let adjusted_dfs: Vec<T> = self
            .pillars
            .iter()
            .zip(base_dfs.iter())
            .map(|(&pillar_time, &df)| {
                // Calculate cumulative jump effect for this pillar
                let jump_factor = self.calculate_cumulative_jump_factor(pillar_time, jumps);
                df * jump_factor
            })
            .collect();

        BootstrappedCurve::new(
            self.pillars.clone(),
            adjusted_dfs,
            self.config.interpolation,
            self.config.allow_extrapolation,
        )
        .map_err(SolverError::NumericalInstability)
    }

    /// Calculate the cumulative jump factor for a given time.
    ///
    /// Returns Π(1 + jump_i) for all jumps where t_jump <= t.
    fn calculate_cumulative_jump_factor(&self, time: T, jumps: &[T]) -> T {
        let mut factor = T::one();

        for (jp, &jump_value) in self.jump_pillars.iter().zip(jumps.iter()) {
            if jp.time <= time {
                factor = factor * (T::one() + jump_value);
            }
        }

        factor
    }

    /// Compute residuals using jump-adjusted curve.
    ///
    /// # Arguments
    ///
    /// * `params` - Extended parameter vector `[log(DF), ..., jumps, ...]`
    ///
    /// # Returns
    ///
    /// Vector of pricing errors.
    pub fn compute_residuals_with_jumps(&self, params: &[T]) -> Result<Vec<T>, CalibrationError> {
        let log_df = self.extract_log_df(params);
        let jumps = self.extract_jumps(params);

        let curve = self.build_curve_with_jumps(log_df, jumps).map_err(|e| {
            CalibrationError::numerical_instability(format!("Failed to build jump curve: {e}"))
        })?;

        self.compute_residuals(&curve)
    }

    /// Compute the Jacobian matrix including jump parameter derivatives.
    ///
    /// The Jacobian is an n × (m + k) matrix where:
    /// - n = number of instruments
    /// - m = number of pillars
    /// - k = number of jump pillars
    ///
    /// J[i,j] = ∂F_i/∂x_j where x = [log(DF), jumps]
    ///
    /// # Arguments
    ///
    /// * `params` - Extended parameter vector
    ///
    /// # Returns
    ///
    /// Extended Jacobian matrix.
    pub fn compute_jacobian_with_jumps(
        &self,
        params: &[T],
    ) -> Result<DMatrix<T>, CalibrationError> {
        let n = self.instruments.len();
        let m = self.pillars.len();
        let k = self.jump_pillars.len();
        let eps = self.config.jacobian_epsilon;

        // Base residuals
        let f0 = self.compute_residuals_with_jumps(params)?;

        // Compute Jacobian columns via finite differences
        let mut jacobian = DMatrix::zeros(n, m + k);

        // Derivatives with respect to log(DF) parameters
        for j in 0..m {
            let mut params_pert = params.to_vec();
            params_pert[j] = params_pert[j] + eps;

            let f_pert = self.compute_residuals_with_jumps(&params_pert)?;

            for i in 0..n {
                jacobian[(i, j)] = (f_pert[i] - f0[i]) / eps;
            }
        }

        // Derivatives with respect to jump parameters
        for j in 0..k {
            let mut params_pert = params.to_vec();
            params_pert[m + j] = params_pert[m + j] + eps;

            let f_pert = self.compute_residuals_with_jumps(&params_pert)?;

            for i in 0..n {
                jacobian[(i, m + j)] = (f_pert[i] - f0[i]) / eps;
            }
        }

        Ok(jacobian)
    }

    /// Compute the Jacobian matrix with jumps using central differences.
    pub fn compute_jacobian_with_jumps_central(
        &self,
        params: &[T],
    ) -> Result<DMatrix<T>, CalibrationError> {
        let n = self.instruments.len();
        let total = self.total_dimension();
        let eps = self.config.jacobian_epsilon;
        let two_eps = eps + eps;

        let mut jacobian = DMatrix::zeros(n, total);

        for j in 0..total {
            let mut params_plus = params.to_vec();
            params_plus[j] = params_plus[j] + eps;
            let f_plus = self.compute_residuals_with_jumps(&params_plus)?;

            let mut params_minus = params.to_vec();
            params_minus[j] = params_minus[j] - eps;
            let f_minus = self.compute_residuals_with_jumps(&params_minus)?;

            for i in 0..n {
                jacobian[(i, j)] = (f_plus[i] - f_minus[i]) / two_eps;
            }
        }

        Ok(jacobian)
    }

    /// Get the realised jump values from a calibrated parameter vector.
    ///
    /// # Arguments
    ///
    /// * `params` - Calibrated extended parameter vector
    ///
    /// # Returns
    ///
    /// Vector of JumpPillar with realised values set.
    pub fn get_realised_jumps(&self, params: &[T]) -> Vec<JumpPillar<T>> {
        let jumps = self.extract_jumps(params);

        self.jump_pillars
            .iter()
            .zip(jumps.iter())
            .map(|(jp, &realised)| {
                let mut pillar = *jp;
                pillar.set_realised_jump(realised);
                pillar
            })
            .collect()
    }
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

    // =========================================================================
    // Jump-related tests
    // =========================================================================

    fn create_jump_pillars() -> Vec<JumpPillar<f64>> {
        vec![
            JumpPillar::new(0.5, 25.0),  // 25bps at 6 months
            JumpPillar::new(1.5, -15.0), // -15bps at 18 months
        ]
    }

    #[test]
    fn test_problem_with_jumps_creation() {
        let instruments = create_test_instruments();
        let jumps = create_jump_pillars();
        let config = CalibrationProblemConfig::default();

        let problem = CalibrationProblem::with_jumps(instruments, jumps, config).unwrap();

        assert!(problem.has_jumps());
        assert_eq!(problem.num_jumps(), 2);
        assert_eq!(problem.pillars().len(), 3);
        assert_eq!(problem.total_dimension(), 5); // 3 pillars + 2 jumps
    }

    #[test]
    fn test_problem_with_jumps_param_indices() {
        let instruments = create_test_instruments();
        let jumps = create_jump_pillars();
        let config = CalibrationProblemConfig::default();

        let problem = CalibrationProblem::with_jumps(instruments, jumps, config).unwrap();

        // Jump pillars should have param indices set
        let jp = problem.jump_pillars();
        assert_eq!(jp[0].param_index, Some(3)); // After 3 pillars
        assert_eq!(jp[1].param_index, Some(4));
    }

    #[test]
    fn test_problem_initial_guess_with_jumps() {
        let instruments = create_test_instruments();
        let jumps = create_jump_pillars();
        let config = CalibrationProblemConfig::default();

        let problem = CalibrationProblem::with_jumps(instruments, jumps, config).unwrap();
        let guess = problem.initial_guess_with_jumps();

        assert_eq!(guess.len(), 5);
        // First 3 are log(DF) at 1y, 2y, 5y
        assert_relative_eq!(guess[0], -0.03, epsilon = 1e-10);
        assert_relative_eq!(guess[1], -0.06, epsilon = 1e-10);
        assert_relative_eq!(guess[2], -0.15, epsilon = 1e-10);
        // Last 2 are expected jumps (sorted by time)
        assert_relative_eq!(guess[3], 0.0025, epsilon = 1e-10); // 25bps
        assert_relative_eq!(guess[4], -0.0015, epsilon = 1e-10); // -15bps
    }

    #[test]
    fn test_problem_extract_params() {
        let instruments = create_test_instruments();
        let jumps = create_jump_pillars();
        let config = CalibrationProblemConfig::default();

        let problem = CalibrationProblem::with_jumps(instruments, jumps, config).unwrap();
        let params = problem.initial_guess_with_jumps();

        let log_df = problem.extract_log_df(&params);
        let jump_vals = problem.extract_jumps(&params);

        assert_eq!(log_df.len(), 3);
        assert_eq!(jump_vals.len(), 2);
        assert_relative_eq!(log_df[0], -0.03, epsilon = 1e-10);
        assert_relative_eq!(jump_vals[0], 0.0025, epsilon = 1e-10);
    }

    #[test]
    fn test_problem_build_curve_with_jumps() {
        let instruments = create_test_instruments();
        let jumps = vec![JumpPillar::new(0.5, 100.0)]; // 100bps = 1% jump
        let config = CalibrationProblemConfig::default();

        let problem = CalibrationProblem::with_jumps(instruments, jumps, config).unwrap();
        let log_df = problem.initial_guess();
        let jump_vals = vec![0.01]; // 100bps

        let curve = problem.build_curve_with_jumps(&log_df, &jump_vals).unwrap();

        // All pillars are after the 0.5y jump, so all DFs should be multiplied by 1.01
        let base_df_1y = (-0.03f64).exp();
        let adjusted_df_1y = curve.discount_factor(1.0).unwrap();
        assert_relative_eq!(adjusted_df_1y, base_df_1y * 1.01, epsilon = 1e-8);
    }

    #[test]
    fn test_problem_cumulative_jump_factor() {
        let instruments = create_test_instruments();
        let jumps = vec![
            JumpPillar::new(0.5, 100.0), // 1% at 0.5y
            JumpPillar::new(1.5, 50.0),  // 0.5% at 1.5y
        ];
        let config = CalibrationProblemConfig::default();

        let problem = CalibrationProblem::with_jumps(instruments, jumps, config).unwrap();
        let jump_vals = vec![0.01, 0.005];

        // At 1y: only first jump applies
        let factor_1y = problem.calculate_cumulative_jump_factor(1.0, &jump_vals);
        assert_relative_eq!(factor_1y, 1.01, epsilon = 1e-10);

        // At 2y: both jumps apply
        let factor_2y = problem.calculate_cumulative_jump_factor(2.0, &jump_vals);
        assert_relative_eq!(factor_2y, 1.01 * 1.005, epsilon = 1e-10);
    }

    #[test]
    fn test_problem_compute_jacobian_with_jumps() {
        let instruments = create_test_instruments();
        let jumps = create_jump_pillars();
        let config = CalibrationProblemConfig::default();

        let problem = CalibrationProblem::with_jumps(instruments, jumps, config).unwrap();
        let params = problem.initial_guess_with_jumps();

        let jacobian = problem.compute_jacobian_with_jumps(&params).unwrap();

        // Should be 3 instruments × 5 parameters
        assert_eq!(jacobian.nrows(), 3);
        assert_eq!(jacobian.ncols(), 5);
    }

    #[test]
    fn test_problem_get_realised_jumps() {
        let instruments = create_test_instruments();
        let jumps = create_jump_pillars();
        let config = CalibrationProblemConfig::default();

        let problem = CalibrationProblem::with_jumps(instruments, jumps, config).unwrap();

        // Simulate calibrated parameters
        let mut params = problem.initial_guess_with_jumps();
        params[3] = 0.003; // Realised: 30bps
        params[4] = -0.002; // Realised: -20bps

        let realised = problem.get_realised_jumps(&params);

        assert_eq!(realised.len(), 2);
        assert!(realised[0].is_calibrated());
        assert_relative_eq!(
            realised[0].realised_jump_rate().unwrap(),
            0.003,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            realised[0].realised_jump_bps().unwrap(),
            30.0,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            realised[1].realised_jump_rate().unwrap(),
            -0.002,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            realised[1].realised_jump_bps().unwrap(),
            -20.0,
            epsilon = 1e-6
        );
    }

    #[test]
    fn test_problem_no_jumps_has_jumps_false() {
        let instruments = create_test_instruments();
        let problem = CalibrationProblem::new(instruments).unwrap();

        assert!(!problem.has_jumps());
        assert_eq!(problem.num_jumps(), 0);
        assert_eq!(problem.total_dimension(), 3);
    }

    #[test]
    fn test_problem_jump_pillars_sorted() {
        let instruments = create_test_instruments();
        // Unsorted input
        let jumps = vec![
            JumpPillar::new(1.5, 10.0),
            JumpPillar::new(0.5, 20.0),
            JumpPillar::new(1.0, 15.0),
        ];
        let config = CalibrationProblemConfig::default();

        let problem = CalibrationProblem::with_jumps(instruments, jumps, config).unwrap();
        let jp = problem.jump_pillars();

        // Should be sorted by time
        assert_relative_eq!(jp[0].time, 0.5, epsilon = 1e-10);
        assert_relative_eq!(jp[1].time, 1.0, epsilon = 1e-10);
        assert_relative_eq!(jp[2].time, 1.5, epsilon = 1e-10);
    }
}
