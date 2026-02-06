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
    /// J\[i,j\] = ∂F_i/∂x_j where x_j = log(DF_j)
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

    /// Validate the quality of a Jacobian matrix.
    ///
    /// Checks for numerical issues including:
    /// - NaN values (indicates computation failure)
    /// - Inf values (indicates overflow)
    /// - Near-zero diagonal elements (indicates singularity)
    ///
    /// # Requirement: 5.3
    ///
    /// # Arguments
    ///
    /// * `jacobian` - The Jacobian matrix to validate
    ///
    /// # Returns
    ///
    /// * `JacobianQuality` - Classification of the matrix quality
    pub fn validate_jacobian_quality(
        &self,
        jacobian: &DMatrix<T>,
    ) -> super::error::JacobianQuality {
        use super::error::JacobianQuality;

        let nrows = jacobian.nrows();
        let ncols = jacobian.ncols();
        let zero_threshold = from_f64::<T>(1e-14);

        // Check for NaN
        for &val in jacobian.iter() {
            if val.is_nan() {
                return JacobianQuality::poor("NaN detected in Jacobian");
            }
        }

        // Check for Inf
        for &val in jacobian.iter() {
            if val.is_infinite() {
                return JacobianQuality::poor("Inf detected in Jacobian");
            }
        }

        // Check diagonal elements for near-zero values (square matrices only)
        if nrows == ncols {
            for i in 0..nrows {
                let diag_val = jacobian[(i, i)];
                if Float::abs(diag_val) < zero_threshold {
                    return JacobianQuality::warning("Near-zero diagonal element detected");
                }
            }
        }

        JacobianQuality::good()
    }

    /// Validate Jacobian quality and get full diagnostics.
    ///
    /// # Requirement: 5.3, 5.5
    ///
    /// # Arguments
    ///
    /// * `jacobian` - The Jacobian matrix to validate
    ///
    /// # Returns
    ///
    /// * Tuple of (JacobianQuality, NumericalDiagnostics)
    pub fn validate_jacobian_with_diagnostics(
        &self,
        jacobian: &DMatrix<T>,
    ) -> (
        super::error::JacobianQuality,
        super::error::NumericalDiagnostics<T>,
    ) {
        super::error::validate_jacobian_dmatrix(jacobian, from_f64(1e-14))
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
    /// J\[i,j\] = ∂F_i/∂x_j where x = \[log(DF), jumps\]
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

    // =========================================================================
    // AD Instability Helper Methods (Task 4.4, Requirement 5.4)
    // =========================================================================

    /// Compute variance between two Jacobian matrices.
    ///
    /// Calculates the mean squared difference between corresponding elements.
    /// Used to detect AD instability by comparing AD Jacobian with finite
    /// difference.
    pub fn compute_jacobian_variance(&self, j1: &DMatrix<T>, j2: &DMatrix<T>) -> T {
        let n = j1.nrows();
        let m = j1.ncols();

        if n != j2.nrows() || m != j2.ncols() {
            return from_f64(f64::INFINITY);
        }

        if n == 0 || m == 0 {
            return from_f64(0.0);
        }

        let mut sum_sq_diff: T = from_f64(0.0);
        let mut count: T = from_f64(0.0);

        for i in 0..n {
            for j in 0..m {
                let diff = j1[(i, j)] - j2[(i, j)];
                sum_sq_diff = sum_sq_diff + diff * diff;
                count = count + from_f64(1.0);
            }
        }

        if count > from_f64(0.0) {
            sum_sq_diff / count
        } else {
            from_f64(0.0)
        }
    }

    /// Check if AD fallback should be triggered based on variance.
    ///
    /// # Arguments
    ///
    /// * `ad_jacobian` - Jacobian computed via Enzyme AD
    /// * `fd_jacobian` - Jacobian computed via finite difference
    /// * `threshold` - Variance threshold (default 1e6)
    ///
    /// # Returns
    ///
    /// (should_fallback, variance)
    pub fn should_fallback_from_ad(
        &self,
        ad_jacobian: &DMatrix<T>,
        fd_jacobian: &DMatrix<T>,
        threshold: T,
    ) -> (bool, T) {
        let variance = self.compute_jacobian_variance(ad_jacobian, fd_jacobian);
        (variance > threshold, variance)
    }
}

// =============================================================================
// CompiledInstrument Integration (Requirement 2, 4)
// =============================================================================

use crate::builder::compile::CompiledInstrument;

impl<T> CalibrationProblem<T, CompiledInstrument<T>>
where
    T: Float + RealField + Copy,
{
    /// Create a new calibration problem from compiled instruments.
    ///
    /// # Requirement 2.1
    ///
    /// When `from_compiled()` is called, the Builder shall construct a
    /// CalibrationProblem from pre-compiled instruments.
    ///
    /// # Requirement 2.2
    ///
    /// The CalibrationProblem shall hold references to compiled instruments
    /// and not re-compile during iteration.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Pre-compiled instruments
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Calibration problem with compiled instruments
    /// * `Err(CalibrationError)` - If validation fails
    pub fn from_compiled(
        instruments: Vec<CompiledInstrument<T>>,
    ) -> Result<Self, CalibrationError> {
        Self::from_compiled_with_config(instruments, CalibrationProblemConfig::default())
    }

    /// Create a new calibration problem from compiled instruments with custom
    /// config.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Pre-compiled instruments
    /// * `config` - Calibration configuration
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Calibration problem with compiled instruments
    /// * `Err(CalibrationError)` - If validation fails
    pub fn from_compiled_with_config(
        instruments: Vec<CompiledInstrument<T>>,
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

        // Log compilation info (Requirement 2.3)
        #[cfg(feature = "tracing")]
        {
            let total_cashflows: usize = instruments.iter().map(|i| i.num_cashflows()).sum();
            tracing::info!(
                instruments = instruments.len(),
                cashflows = total_cashflows,
                pillars = pillars.len(),
                "Calibration problem created from compiled instruments"
            );
        }

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

    /// Get the total number of cashflows across all compiled instruments.
    ///
    /// Useful for logging and diagnostics.
    pub fn total_cashflows(&self) -> usize {
        self.instruments.iter().map(|i| i.num_cashflows()).sum()
    }

    /// Create a calibration problem from market instruments.
    ///
    /// # Requirement 2.1
    ///
    /// When `from_market_instruments()` is called, the Builder shall compile
    /// all MarketInstruments to CompiledInstruments.
    ///
    /// # Requirement 2.3
    ///
    /// When compilation is complete, the System shall log the number of
    /// instruments, total cashflows, and compile time.
    ///
    /// # Requirement 2.4
    ///
    /// If a compile error occurs, the Builder shall propagate the error
    /// without leaving partially compiled state.
    ///
    /// # Arguments
    ///
    /// * `market_instruments` - Resolved market instruments from infra_domain
    /// * `valuation_date` - The valuation date for compilation
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Calibration problem with compiled instruments
    /// * `Err(CompileError)` - If any instrument fails to compile
    pub fn from_market_instruments(
        market_instruments: &[infra_domain::market::MarketInstrument],
        valuation_date: infra_domain::time::Date,
    ) -> Result<Self, crate::builder::compile::CompileError> {
        use crate::builder::compile::InstrumentCompiler;

        // Create compiler
        let compiler: InstrumentCompiler<T> = InstrumentCompiler::new(valuation_date);

        // Compile all instruments (Requirement 2.4: fail-fast, no partial state)
        let start_time = std::time::Instant::now();
        let compiled = compiler.compile_batch(market_instruments)?;
        let compile_duration = start_time.elapsed();

        // Log compilation info (Requirement 2.3)
        #[cfg(feature = "tracing")]
        {
            let total_cashflows: usize = compiled.iter().map(|i| i.num_cashflows()).sum();
            tracing::info!(
                instruments = compiled.len(),
                cashflows = total_cashflows,
                compile_time_ms = compile_duration.as_millis(),
                "Compiled instruments for calibration"
            );
        }
        let _ = compile_duration; // Suppress unused warning when tracing is disabled

        // Create calibration problem
        Self::from_compiled(compiled).map_err(|e| {
            crate::builder::compile::CompileError::InvalidConvention {
                index: 0,
                rate_id: format!("CalibrationError: {}", e),
            }
        })
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
            JacobianMethod::AutomaticDifferentiation => {
                // Try Enzyme AD first, fall back to finite differences on failure
                self.compute_jacobian_enzyme_with_fallback(&log_df)
            }
        };

        jacobian.map_err(|e| {
            SolverError::NumericalInstability(format!("Jacobian computation failed: {e}"))
        })
    }
}

// =============================================================================
// Enzyme AD Jacobian Implementation
// =============================================================================

#[cfg(feature = "enzyme-ad")]
impl<T, I> CalibrationProblem<T, I>
where
    T: Float + RealField + Copy,
    I: CalibrationInstrument<T> + Clone,
{
    /// Compute Jacobian using Enzyme AD with automatic fallback.
    ///
    /// # Requirement 1.4
    ///
    /// If Enzyme AD computation fails due to unsupported operations,
    /// the method falls back to finite differences and logs a warning.
    ///
    /// # Arguments
    ///
    /// * `log_df` - Current log discount factors
    ///
    /// # Returns
    ///
    /// Jacobian matrix, either from Enzyme AD or finite differences.
    pub fn compute_jacobian_enzyme_with_fallback(
        &self,
        log_df: &[T],
    ) -> Result<DMatrix<T>, CalibrationError> {
        use super::enzyme_jacobian::JacobianResult;

        let start_time = std::time::Instant::now();

        // Try Enzyme AD computation
        match self.try_compute_jacobian_enzyme(log_df) {
            Ok(jacobian) => {
                let _elapsed = start_time.elapsed().as_micros() as u64;
                Ok(jacobian)
            }
            Err(e) => {
                // Log warning about fallback
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "Enzyme AD Jacobian failed, falling back to finite differences: {}",
                    e
                );

                // Fall back to finite differences
                let jacobian = self.compute_jacobian_finite_diff(log_df)?;
                let _elapsed = start_time.elapsed().as_micros() as u64;
                Ok(jacobian)
            }
        }
    }

    /// Try to compute Jacobian using Enzyme AD.
    ///
    /// This method extracts instrument parameters and calls the Enzyme kernels.
    /// It may fail if the instruments contain unsupported operations.
    fn try_compute_jacobian_enzyme(&self, log_df: &[T]) -> Result<DMatrix<T>, CalibrationError> {
        use super::enzyme_jacobian::kernels;

        // Convert log_df to f64 for Enzyme (Enzyme only works with f64)
        let log_df_f64: Vec<f64> = log_df.iter().map(|&x| x.to_f64().unwrap_or(0.0)).collect();
        let pillar_times_f64: Vec<f64> = self
            .pillars
            .iter()
            .map(|&x| x.to_f64().unwrap_or(0.0))
            .collect();

        // Extract instrument types and parameters
        let mut instrument_types = Vec::with_capacity(self.instruments.len());
        let mut instrument_params = Vec::with_capacity(self.instruments.len());

        for instrument in &self.instruments {
            let (inst_type, params) = self.extract_enzyme_params(instrument)?;
            instrument_types.push(inst_type);
            instrument_params.push(params);
        }

        // Compute Jacobian using Enzyme kernels
        let jacobian_f64 = kernels::compute_jacobian_enzyme(
            &instrument_types,
            &instrument_params,
            &log_df_f64,
            &pillar_times_f64,
        );

        // Convert back to T
        let n = jacobian_f64.nrows();
        let m = jacobian_f64.ncols();
        let mut jacobian = DMatrix::zeros(n, m);
        for i in 0..n {
            for j in 0..m {
                jacobian[(i, j)] = from_f64(jacobian_f64[(i, j)]);
            }
        }

        Ok(jacobian)
    }

    /// Extract Enzyme-compatible parameters from an instrument.
    ///
    /// # Returns
    ///
    /// Tuple of (instrument_type_code, parameters_vector)
    /// - Type 0: Deposit [maturity, market_rate]
    /// - Type 1: FRA [start_time, end_time, tau, market_rate]
    /// - Type 2: Swap/OIS [maturity, market_rate, n_cf, cf_time_1, yf_1, ...]
    fn extract_enzyme_params(&self, instrument: &I) -> Result<(u32, Vec<f64>), CalibrationError> {
        let maturity = instrument.maturity().to_f64().unwrap_or(0.0);
        let market_rate = instrument.market_rate().to_f64().unwrap_or(0.0);
        let inst_type = instrument.instrument_type();

        match inst_type {
            "Deposit" => {
                // Deposit: [maturity, market_rate]
                Ok((0, vec![maturity, market_rate]))
            }
            "FRA" | "Futures" => {
                // FRA/Futures: [start_time, end_time, tau, market_rate]
                // For simplicity, assume start = 0 if not available
                // tau = maturity (year fraction for the period)
                Ok((1, vec![0.0, maturity, maturity, market_rate]))
            }
            "Swap" | "OIS" | "IRS" => {
                // Swap/OIS: [maturity, market_rate, n_cf, cf_time_1, yf_1, ...]
                // Generate annual cashflows for simplicity
                let mut params = vec![maturity, market_rate];
                let n_cf = maturity.ceil() as usize;
                params.push(n_cf as f64);

                let mut t = 1.0;
                for _ in 0..n_cf {
                    let cf_time = t.min(maturity);
                    let yf = 1.0;
                    params.push(cf_time);
                    params.push(yf);
                    t += 1.0;
                }

                Ok((2, params))
            }
            _ => {
                // Unknown instrument type - return error to trigger fallback
                Err(CalibrationError::numerical_instability(format!(
                    "Unsupported instrument type for Enzyme AD: {}",
                    inst_type
                )))
            }
        }
    }

    /// Compute Jacobian with full result metadata.
    ///
    /// # Requirement 1.1, 1.4
    ///
    /// This method returns a JacobianResult with metadata including:
    /// - The Jacobian matrix
    /// - The method actually used
    /// - Computation time
    /// - Whether fallback was triggered
    pub fn compute_jacobian_enzyme_result(
        &self,
        log_df: &[T],
    ) -> Result<super::enzyme_jacobian::JacobianResult, CalibrationError> {
        use super::enzyme_jacobian::JacobianResult;

        let start_time = std::time::Instant::now();

        // Try Enzyme AD computation
        match self.try_compute_jacobian_enzyme(log_df) {
            Ok(jacobian) => {
                let elapsed = start_time.elapsed().as_micros() as u64;
                // Convert to f64 matrix for JacobianResult
                let jacobian_f64 = self.convert_matrix_to_f64(&jacobian);
                Ok(JacobianResult::from_enzyme_ad(jacobian_f64, elapsed))
            }
            Err(_) => {
                // Fall back to finite differences
                let jacobian = self.compute_jacobian_finite_diff(log_df)?;
                let elapsed = start_time.elapsed().as_micros() as u64;
                let jacobian_f64 = self.convert_matrix_to_f64(&jacobian);
                Ok(JacobianResult::with_fallback(jacobian_f64, elapsed))
            }
        }
    }

    /// Convert a DMatrix<T> to DMatrix<f64>.
    fn convert_matrix_to_f64(&self, matrix: &DMatrix<T>) -> DMatrix<f64> {
        let n = matrix.nrows();
        let m = matrix.ncols();
        let mut result = DMatrix::zeros(n, m);
        for i in 0..n {
            for j in 0..m {
                result[(i, j)] = matrix[(i, j)].to_f64().unwrap_or(0.0);
            }
        }
        result
    }

    // =========================================================================
    // AD Instability Auto-Fallback (Task 4.4)
    // =========================================================================

    /// Compute Jacobian with stability check and auto-fallback.
    ///
    /// # Requirement: 5.4
    ///
    /// Compares Enzyme AD Jacobian with finite difference approximation.
    /// If variance exceeds threshold (1e6), automatically falls back to
    /// central difference method for improved numerical stability.
    ///
    /// # Arguments
    ///
    /// * `log_df` - Log discount factors
    ///
    /// # Returns
    ///
    /// Tuple of (Jacobian matrix, NumericalDiagnostics)
    pub fn compute_jacobian_with_stability_check(
        &self,
        log_df: &[T],
    ) -> Result<(DMatrix<T>, super::error::NumericalDiagnostics<T>), CalibrationError> {
        use super::error::NumericalDiagnostics;

        let variance_threshold: T = from_f64(1e6);
        let mut diagnostics = NumericalDiagnostics::new();

        // Try Enzyme AD computation first
        let enzyme_result = self.try_compute_jacobian_enzyme(log_df);

        match enzyme_result {
            Ok(enzyme_jacobian) => {
                // Compute finite difference Jacobian for comparison
                let fd_jacobian = self.compute_jacobian_finite_diff(log_df)?;

                // Calculate variance between AD and FD
                let variance = self.compute_jacobian_variance(&enzyme_jacobian, &fd_jacobian);
                diagnostics.ad_variance = Some(variance);

                if variance > variance_threshold {
                    // AD is unstable, fall back to central difference
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        variance = %variance.to_f64().unwrap_or(0.0),
                        threshold = %variance_threshold.to_f64().unwrap_or(0.0),
                        "AD Jacobian variance exceeds threshold, falling back to central difference"
                    );

                    diagnostics.ad_fallback_used = true;
                    let central_jacobian = self.compute_jacobian_central_diff(log_df)?;
                    Ok((central_jacobian, diagnostics))
                } else {
                    // AD is stable, use it
                    Ok((enzyme_jacobian, diagnostics))
                }
            }
            Err(e) => {
                // AD failed entirely, fall back to central difference
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    error = %e,
                    "Enzyme AD Jacobian failed, falling back to central difference"
                );

                diagnostics.ad_fallback_used = true;
                let central_jacobian = self.compute_jacobian_central_diff(log_df)?;
                Ok((central_jacobian, diagnostics))
            }
        }
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

    // =========================================================================
    // CompiledInstrument Integration Tests (Requirement 2, 4)
    // =========================================================================

    use crate::builder::compile::{CompiledInstrument, InstrumentType};

    fn create_compiled_instruments() -> Vec<CompiledInstrument<f64>> {
        vec![
            CompiledInstrument::deposit(0.03, 1.0).unwrap(),
            CompiledInstrument::deposit(0.035, 2.0).unwrap(),
            CompiledInstrument::deposit(0.04, 5.0).unwrap(),
        ]
    }

    #[test]
    fn test_from_compiled_creation() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments).unwrap();

        assert_eq!(problem.dimension(), 3);
        assert_eq!(problem.instruments().len(), 3);
        assert_eq!(problem.pillars().len(), 3);
    }

    #[test]
    fn test_from_compiled_empty_instruments() {
        let instruments: Vec<CompiledInstrument<f64>> = vec![];
        let result = CalibrationProblem::from_compiled(instruments);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CalibrationError::NoInstruments
        ));
    }

    #[test]
    fn test_from_compiled_total_cashflows() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments).unwrap();

        // Each deposit has 1 cashflow
        assert_eq!(problem.total_cashflows(), 3);
    }

    #[test]
    fn test_from_compiled_with_config() {
        let instruments = create_compiled_instruments();
        let config = CalibrationProblemConfig {
            jacobian_method: JacobianMethod::CentralDifference,
            jacobian_epsilon: 1e-6,
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
        };

        let problem = CalibrationProblem::from_compiled_with_config(instruments, config).unwrap();

        assert_eq!(
            problem.config().jacobian_method,
            JacobianMethod::CentralDifference
        );
        assert_relative_eq!(problem.config().jacobian_epsilon, 1e-6, epsilon = 1e-15);
    }

    #[test]
    fn test_from_compiled_initial_guess() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments).unwrap();

        let guess = problem.initial_guess();
        assert_eq!(guess.len(), 3);

        // Initial guess: log(DF) = -0.03 * t
        assert_relative_eq!(guess[0], -0.03, epsilon = 1e-10);
        assert_relative_eq!(guess[1], -0.06, epsilon = 1e-10);
        assert_relative_eq!(guess[2], -0.15, epsilon = 1e-10);
    }

    #[test]
    fn test_from_compiled_build_curve() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments).unwrap();

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
    fn test_from_compiled_evaluate() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments).unwrap();

        let x = problem.initial_guess_vector();
        let residuals = problem.evaluate(&x).unwrap();

        assert_eq!(residuals.len(), 3);
    }

    #[test]
    fn test_from_compiled_jacobian() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments).unwrap();

        let x = problem.initial_guess_vector();
        let jacobian = problem.jacobian(&x).unwrap();

        assert_eq!(jacobian.nrows(), 3);
        assert_eq!(jacobian.ncols(), 3);

        let max_elem: f64 = jacobian.iter().map(|&x| x.abs()).fold(0.0, f64::max);
        assert!(max_elem > 0.0);
    }

    #[test]
    fn test_from_compiled_with_swap_instruments() {
        // Test with different instrument types
        let instruments: Vec<CompiledInstrument<f64>> = vec![
            CompiledInstrument::deposit(0.03, 0.25).unwrap(),
            CompiledInstrument::fra(0.032, 0.25, 0.5).unwrap(),
            CompiledInstrument::new(
                InstrumentType::Swap,
                0.035,
                2.0,
                vec![1.0, 2.0],
                vec![1.0, 1.0],
                vec![1.0, 1.0],
                Some(0.035),
            )
            .unwrap(),
        ];

        let problem = CalibrationProblem::from_compiled(instruments).unwrap();
        assert_eq!(problem.dimension(), 3);
        // Total cashflows: deposit=1, fra=2, swap=2
        assert_eq!(problem.total_cashflows(), 5);
    }

    // =========================================================================
    // from_market_instruments Tests (Requirement 2)
    // =========================================================================

    use infra_domain::{
        market::{
            convention::{DepositConvention, FraConvention, MarketConvention, SwapConvention},
            Currency, MarketInstrument as InfraMasterInstrument, RateId, RateType,
        },
        time::{Date, Tenor},
    };

    fn create_infra_market_instruments() -> (Vec<InfraMasterInstrument>, Date) {
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let instruments = vec![
            InfraMasterInstrument::new(
                RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit),
                0.05,
                MarketConvention::Deposit(DepositConvention::usd()),
                valuation_date,
                1_000_000.0,
            )
            .unwrap(),
            InfraMasterInstrument::new(
                RateId::new(Currency::USD, Tenor::OneYear, RateType::Ois),
                0.052,
                MarketConvention::Ois(SwapConvention::usd_sofr()),
                valuation_date,
                5_000_000.0,
            )
            .unwrap(),
            InfraMasterInstrument::new(
                RateId::new(Currency::USD, Tenor::FiveYears, RateType::Swap),
                0.055,
                MarketConvention::Swap(SwapConvention::usd_sofr()),
                valuation_date,
                10_000_000.0,
            )
            .unwrap(),
        ];

        (instruments, valuation_date)
    }

    #[test]
    fn test_from_market_instruments_creation() {
        let (instruments, valuation_date) = create_infra_market_instruments();
        let problem: CalibrationProblem<f64, CompiledInstrument<f64>> =
            CalibrationProblem::from_market_instruments(&instruments, valuation_date).unwrap();

        assert_eq!(problem.dimension(), 3);
        assert_eq!(problem.instruments().len(), 3);
    }

    #[test]
    fn test_from_market_instruments_empty() {
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let instruments: Vec<InfraMasterInstrument> = vec![];

        let result: Result<CalibrationProblem<f64, CompiledInstrument<f64>>, _> =
            CalibrationProblem::from_market_instruments(&instruments, valuation_date);

        // Should fail because no instruments
        assert!(result.is_err());
    }

    #[test]
    fn test_from_market_instruments_evaluate() {
        let (instruments, valuation_date) = create_infra_market_instruments();
        let problem: CalibrationProblem<f64, CompiledInstrument<f64>> =
            CalibrationProblem::from_market_instruments(&instruments, valuation_date).unwrap();

        let x = problem.initial_guess_vector();
        let residuals = problem.evaluate(&x).unwrap();

        assert_eq!(residuals.len(), 3);
    }

    #[test]
    fn test_from_market_instruments_jacobian() {
        let (instruments, valuation_date) = create_infra_market_instruments();
        let problem: CalibrationProblem<f64, CompiledInstrument<f64>> =
            CalibrationProblem::from_market_instruments(&instruments, valuation_date).unwrap();

        let x = problem.initial_guess_vector();
        let jacobian = problem.jacobian(&x).unwrap();

        assert_eq!(jacobian.nrows(), 3);
        assert_eq!(jacobian.ncols(), 3);

        let max_elem: f64 = jacobian.iter().map(|&x| x.abs()).fold(0.0, f64::max);
        assert!(max_elem > 0.0);
    }

    #[test]
    fn test_from_market_instruments_cashflows() {
        let (instruments, valuation_date) = create_infra_market_instruments();
        let problem: CalibrationProblem<f64, CompiledInstrument<f64>> =
            CalibrationProblem::from_market_instruments(&instruments, valuation_date).unwrap();

        // Should have more than 3 cashflows (deposit=1, OIS=1, swap=5)
        assert!(problem.total_cashflows() > 3);
    }

    // =========================================================================
    // AD Instability Auto-Fallback Tests (Task 4.4, Requirement 5.4)
    // =========================================================================

    #[test]
    fn test_compute_jacobian_variance_identical() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments).unwrap();

        let x = problem.initial_guess_vector();
        let jacobian1 = problem.jacobian(&x).unwrap();
        let jacobian2 = jacobian1.clone();

        let variance = problem.compute_jacobian_variance(&jacobian1, &jacobian2);
        assert_relative_eq!(variance, 0.0, epsilon = 1e-15);
    }

    #[test]
    fn test_compute_jacobian_variance_different() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments).unwrap();

        let x = problem.initial_guess_vector();
        let jacobian1 = problem.jacobian(&x).unwrap();

        // Create a perturbed Jacobian
        let mut jacobian2 = jacobian1.clone();
        for i in 0..jacobian2.nrows() {
            for j in 0..jacobian2.ncols() {
                jacobian2[(i, j)] += 0.01; // Add 0.01 to each element
            }
        }

        let variance = problem.compute_jacobian_variance(&jacobian1, &jacobian2);
        // Mean squared diff = 0.01^2 = 0.0001
        assert_relative_eq!(variance, 0.0001, epsilon = 1e-10);
    }

    #[test]
    fn test_should_fallback_from_ad_below_threshold() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments).unwrap();

        let x = problem.initial_guess_vector();
        let jacobian1 = problem.jacobian(&x).unwrap();
        let jacobian2 = jacobian1.clone();

        let threshold = 1e6;
        let (should_fallback, variance) =
            problem.should_fallback_from_ad(&jacobian1, &jacobian2, threshold);

        assert!(!should_fallback);
        assert!(variance < threshold);
    }

    #[test]
    fn test_should_fallback_from_ad_above_threshold() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments).unwrap();

        let x = problem.initial_guess_vector();
        let jacobian1 = problem.jacobian(&x).unwrap();

        // Create a very different Jacobian to trigger fallback
        let mut jacobian2 = jacobian1.clone();
        for i in 0..jacobian2.nrows() {
            for j in 0..jacobian2.ncols() {
                jacobian2[(i, j)] += 2000.0; // Add large perturbation (2000^2 =
                                             // 4e6 > 1e6)
            }
        }

        let threshold = 1e6;
        let (should_fallback, variance) =
            problem.should_fallback_from_ad(&jacobian1, &jacobian2, threshold);

        // Variance = mean((2000)^2) = 4e6 > 1e6 threshold
        assert!(should_fallback);
        assert!(variance > threshold);
    }

    #[test]
    #[cfg(feature = "enzyme-ad")]
    fn test_compute_jacobian_with_stability_check() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments).unwrap();

        let x = problem.initial_guess_vector();
        let result = problem.compute_jacobian_with_stability_check(&x);

        // Should succeed (may use fallback depending on whether Enzyme AD is available)
        assert!(result.is_ok());
        let (jacobian, diagnostics) = result.unwrap();

        // Jacobian should be valid
        assert_eq!(jacobian.nrows(), 3);
        assert_eq!(jacobian.ncols(), 3);

        // Diagnostics should be populated
        // If AD fallback was used, the ad_fallback_used flag should be true
        // and possibly ad_variance might be set
        // We don't assert specific values as they depend on runtime
        assert!(!diagnostics.ad_variance.is_some() || diagnostics.ad_variance.unwrap() >= 0.0);
    }
}
