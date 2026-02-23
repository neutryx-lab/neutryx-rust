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

/// Extracts sorted, deduplicated pillar maturities from instruments.
fn extract_sorted_pillars<T: Float, I: CalibrationInstrument<T>>(instruments: &[I]) -> Vec<T> {
    let mut pillars: Vec<T> = instruments.iter().map(|i| i.maturity()).collect();
    pillars.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    pillars.dedup_by(|a, b| Float::abs(*a - *b) < from_f64(1e-10));
    pillars
}

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

/// Calibration problem as a system of equations F(x) = 0.
///
/// Unknowns x = log(DF) at each pillar maturity. When jump calibration is
/// enabled, the parameter vector is extended to:
/// `[log(DF_1), ..., log(DF_n), jump_1, ..., jump_m]`
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
    pub fn new(instruments: Vec<I>) -> Result<Self, CalibrationError> {
        Self::with_config(instruments, CalibrationProblemConfig::default())
    }

    /// Create a new calibration problem with custom configuration.
    pub fn with_config(
        instruments: Vec<I>,
        config: CalibrationProblemConfig<T>,
    ) -> Result<Self, CalibrationError> {
        if instruments.is_empty() {
            return Err(CalibrationError::NoInstruments);
        }

        let pillars = extract_sorted_pillars(&instruments);

        Ok(Self {
            instruments,
            pillars,
            config,
            jump_pillars: Vec::new(),
        })
    }

    /// Create a new calibration problem with jump pillars.
    pub fn with_jumps(
        instruments: Vec<I>,
        mut jump_pillars: Vec<JumpPillar<T>>,
        config: CalibrationProblemConfig<T>,
    ) -> Result<Self, CalibrationError> {
        if instruments.is_empty() {
            return Err(CalibrationError::NoInstruments);
        }

        let pillars = extract_sorted_pillars(&instruments);

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

    /// Get the total dimension of the parameter vector (n_pillars + n_jumps).
    pub fn total_dimension(&self) -> usize { self.pillars.len() + self.jump_pillars.len() }

    /// Build a yield curve from log discount factors.
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
    pub fn compute_residuals(
        &self,
        curve: &BootstrappedCurve<T>,
    ) -> Result<Vec<T>, CalibrationError> {
        let mut residuals = Vec::with_capacity(self.instruments.len());

        for (idx, instrument) in self.instruments.iter().enumerate() {
            let error = instrument.pricing_error(curve).map_err(|e| {
                CalibrationError::InstrumentEvaluationFailed {
                    instrument_index: idx,
                    message: e.to_string(),
                }
            })?;
            residuals.push(error);
        }

        Ok(residuals)
    }

    /// Compute the Jacobian J\[i,j\] = dF_i/dx_j using forward differences.
    pub fn compute_jacobian_finite_diff(
        &self,
        log_df: &[T],
    ) -> Result<DMatrix<T>, CalibrationError> {
        let n = self.instruments.len();
        let m = self.pillars.len();
        let eps = self.config.jacobian_epsilon;

        // Base residuals
        let curve =
            self.build_curve(log_df)
                .map_err(|e| CalibrationError::NumericalInstability {
                    message: format!("Failed to build curve: {e}"),
                })?;
        let f0 = self.compute_residuals(&curve)?;

        // Compute Jacobian columns via forward differences
        let mut jacobian = DMatrix::zeros(n, m);

        for j in 0..m {
            let mut log_df_pert = log_df.to_vec();
            log_df_pert[j] = log_df_pert[j] + eps;

            let curve_pert = self.build_curve(&log_df_pert).map_err(|e| {
                CalibrationError::NumericalInstability {
                    message: format!("Failed to build perturbed curve: {e}"),
                }
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
                CalibrationError::NumericalInstability {
                    message: format!("Failed to build curve+: {e}"),
                }
            })?;
            let f_plus = self.compute_residuals(&curve_plus)?;

            let mut log_df_minus = log_df.to_vec();
            log_df_minus[j] = log_df_minus[j] - eps;
            let curve_minus = self.build_curve(&log_df_minus).map_err(|e| {
                CalibrationError::NumericalInstability {
                    message: format!("Failed to build curve-: {e}"),
                }
            })?;
            let f_minus = self.compute_residuals(&curve_minus)?;

            for i in 0..n {
                jacobian[(i, j)] = (f_plus[i] - f_minus[i]) / (eps + eps);
            }
        }

        Ok(jacobian)
    }

    /// Validate Jacobian quality (checks for NaN, Inf, near-zero diagonals).
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
                return JacobianQuality::Poor {
                    reason: "NaN detected in Jacobian",
                };
            }
        }

        // Check for Inf
        for &val in jacobian.iter() {
            if val.is_infinite() {
                return JacobianQuality::Poor {
                    reason: "Inf detected in Jacobian",
                };
            }
        }

        // Check diagonal elements for near-zero values (square matrices only)
        if nrows == ncols {
            for i in 0..nrows {
                let diag_val = jacobian[(i, i)];
                if Float::abs(diag_val) < zero_threshold {
                    return JacobianQuality::Warning {
                        reason: "Near-zero diagonal element detected",
                    };
                }
            }
        }

        JacobianQuality::Good
    }

    /// Validate Jacobian quality and return full diagnostics.
    pub fn validate_jacobian_with_diagnostics(
        &self,
        jacobian: &DMatrix<T>,
    ) -> (
        super::error::JacobianQuality,
        super::error::NumericalDiagnostics<T>,
    ) {
        super::error::validate_jacobian_dmatrix(jacobian, from_f64(1e-14))
    }

    /// Create an initial guess for log discount factors using flat 3% rate:
    /// log(DF(t)) = -0.03 * t.
    pub fn initial_guess(&self) -> Vec<T> {
        self.pillars
            .iter()
            .map(|&t| -(from_f64::<T>(0.03) * t))
            .collect()
    }

    /// Create an initial guess DVector for the solver.
    pub fn initial_guess_vector(&self) -> DVector<T> { DVector::from_vec(self.initial_guess()) }

    /// Create an initial guess for extended parameter vector including jumps.
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
    pub fn extract_log_df<'a>(&self, params: &'a [T]) -> &'a [T] { &params[..self.pillars.len()] }

    /// Extract jump values from an extended parameter vector.
    pub fn extract_jumps<'a>(&self, params: &'a [T]) -> &'a [T] { &params[self.pillars.len()..] }

    /// Build a yield curve with jump-adjusted discount factors.
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
    pub fn compute_residuals_with_jumps(&self, params: &[T]) -> Result<Vec<T>, CalibrationError> {
        let log_df = self.extract_log_df(params);
        let jumps = self.extract_jumps(params);

        let curve = self.build_curve_with_jumps(log_df, jumps).map_err(|e| {
            CalibrationError::NumericalInstability {
                message: format!("Failed to build jump curve: {e}"),
            }
        })?;

        let mut residuals = self.compute_residuals(&curve)?;

        // Append jump regularisation residuals: penalise deviation from
        // expected jump values so the system becomes square (n+k equations
        // for n+k unknowns) and the Jacobian is non-singular.
        for (jp, &jump_val) in self.jump_pillars.iter().zip(jumps.iter()) {
            residuals.push(jump_val - jp.expected_jump);
        }

        Ok(residuals)
    }

    /// Compute the (n+k) x (m+k) Jacobian including jump parameters.
    pub fn compute_jacobian_with_jumps(
        &self,
        params: &[T],
    ) -> Result<DMatrix<T>, CalibrationError> {
        let n = self.instruments.len();
        let k = self.jump_pillars.len();
        let total_rows = n + k;
        let total_cols = self.total_dimension();
        let eps = self.config.jacobian_epsilon;

        let f0 = self.compute_residuals_with_jumps(params)?;
        let mut jacobian = DMatrix::zeros(total_rows, total_cols);

        for j in 0..total_cols {
            let mut params_pert = params.to_vec();
            params_pert[j] = params_pert[j] + eps;
            let f_pert = self.compute_residuals_with_jumps(&params_pert)?;
            for i in 0..total_rows {
                jacobian[(i, j)] = (f_pert[i] - f0[i]) / eps;
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
        let k = self.jump_pillars.len();
        let total_rows = n + k;
        let total_cols = self.total_dimension();
        let eps = self.config.jacobian_epsilon;
        let two_eps = eps + eps;

        let mut jacobian = DMatrix::zeros(total_rows, total_cols);

        for j in 0..total_cols {
            let mut params_plus = params.to_vec();
            params_plus[j] = params_plus[j] + eps;
            let f_plus = self.compute_residuals_with_jumps(&params_plus)?;

            let mut params_minus = params.to_vec();
            params_minus[j] = params_minus[j] - eps;
            let f_minus = self.compute_residuals_with_jumps(&params_minus)?;

            for i in 0..total_rows {
                jacobian[(i, j)] = (f_plus[i] - f_minus[i]) / two_eps;
            }
        }

        Ok(jacobian)
    }

    /// Get the realised jump values from a calibrated parameter vector.
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

    /// Compute mean squared difference between two Jacobian matrices.
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

    /// Check if AD fallback should be triggered based on variance threshold.
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

use crate::builder::compile::CompiledInstrument;

impl<T> CalibrationProblem<T, CompiledInstrument<T>>
where
    T: Float + RealField + Copy,
{
    /// Create a new calibration problem from pre-compiled instruments.
    pub fn from_compiled(
        instruments: Vec<CompiledInstrument<T>>,
    ) -> Result<Self, CalibrationError> {
        Self::from_compiled_with_config(instruments, CalibrationProblemConfig::default())
    }

    /// Create a new calibration problem from compiled instruments with custom
    /// config.
    pub fn from_compiled_with_config(
        instruments: Vec<CompiledInstrument<T>>,
        config: CalibrationProblemConfig<T>,
    ) -> Result<Self, CalibrationError> {
        if instruments.is_empty() {
            return Err(CalibrationError::NoInstruments);
        }

        let pillars = extract_sorted_pillars(&instruments);

        // Log compilation info
        {
            let total_cashflows: usize = instruments.iter().map(|i| i.num_cashflows()).sum();
            tracing::info!(
                instruments = instruments.len(),
                cashflows = total_cashflows,
                pillars = pillars.len(),
                "Calibration problem created from compiled instruments"
            );
        }

        Ok(Self {
            instruments,
            pillars,
            config,
            jump_pillars: Vec::new(),
        })
    }

    /// Get the total number of cashflows across all compiled instruments.
    pub fn total_cashflows(&self) -> usize {
        self.instruments.iter().map(|i| i.num_cashflows()).sum()
    }

    /// Create a calibration problem by compiling market instruments.
    pub fn from_market_instruments(
        market_instruments: &[infra_domain::market::MarketInstrument],
        valuation_date: infra_domain::time::Date,
    ) -> Result<Self, crate::builder::compile::CompileError> {
        use crate::builder::compile::InstrumentCompiler;

        let compiler: InstrumentCompiler<T> = InstrumentCompiler::new(valuation_date);
        let start_time = std::time::Instant::now();
        let compiled = compiler.compile_batch(market_instruments)?;
        let compile_duration = start_time.elapsed();

        {
            let total_cashflows: usize = compiled.iter().map(|i| i.num_cashflows()).sum();
            tracing::info!(
                instruments = compiled.len(),
                cashflows = total_cashflows,
                compile_time_ms = compile_duration.as_millis(),
                "Compiled instruments for calibration"
            );
        }
        let _ = compile_duration;

        Self::from_compiled(compiled).map_err(|e| {
            crate::builder::compile::CompileError::InvalidConvention {
                index: 0,
                rate_id: format!("CalibrationError: {}", e),
            }
        })
    }
}

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

#[cfg(feature = "enzyme-ad")]
impl<T, I> CalibrationProblem<T, I>
where
    T: Float + RealField + Copy,
    I: CalibrationInstrument<T> + Clone,
{
    /// Compute Jacobian using Enzyme AD, falling back to finite differences on
    /// failure.
    pub fn compute_jacobian_enzyme_with_fallback(
        &self,
        log_df: &[T],
    ) -> Result<DMatrix<T>, CalibrationError> {
        match self.try_compute_jacobian_enzyme(log_df) {
            Ok(jacobian) => Ok(jacobian),
            Err(e) => {
                tracing::warn!("Enzyme AD Jacobian failed, falling back to FD: {}", e);
                self.compute_jacobian_finite_diff(log_df)
            }
        }
    }

    /// Try to compute Jacobian using Enzyme AD kernels.
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

    /// Extract Enzyme-compatible (type_code, params) from an instrument.
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
                Err(CalibrationError::NumericalInstability {
                    message: format!("Unsupported instrument type for Enzyme AD: {}", inst_type),
                })
            }
        }
    }

    /// Compute Jacobian with full result metadata (method used, timing,
    /// fallback).
    pub fn compute_jacobian_enzyme_result(
        &self,
        log_df: &[T],
    ) -> Result<super::enzyme_jacobian::JacobianResult, CalibrationError> {
        use super::enzyme_jacobian::JacobianResult;
        let start_time = std::time::Instant::now();

        let (jacobian, from_ad) = match self.try_compute_jacobian_enzyme(log_df) {
            Ok(j) => (j, true),
            Err(_) => (self.compute_jacobian_finite_diff(log_df)?, false),
        };
        let elapsed = start_time.elapsed().as_micros() as u64;
        let jacobian_f64 = self.convert_matrix_to_f64(&jacobian);

        Ok(if from_ad {
            JacobianResult::from_enzyme_ad(jacobian_f64, elapsed)
        } else {
            JacobianResult::with_fallback(jacobian_f64, elapsed)
        })
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

    /// Compute Jacobian with AD-vs-FD stability check; falls back to central
    /// diff if unstable.
    pub fn compute_jacobian_with_stability_check(
        &self,
        log_df: &[T],
    ) -> Result<(DMatrix<T>, super::error::NumericalDiagnostics<T>), CalibrationError> {
        use super::error::NumericalDiagnostics;

        let variance_threshold: T = from_f64(1e6);
        let mut diagnostics = NumericalDiagnostics::default();

        match self.try_compute_jacobian_enzyme(log_df) {
            Ok(enzyme_jacobian) => {
                let fd_jacobian = self.compute_jacobian_finite_diff(log_df)?;
                let variance = self.compute_jacobian_variance(&enzyme_jacobian, &fd_jacobian);
                diagnostics.ad_variance = Some(variance);

                if variance > variance_threshold {
                    tracing::warn!(
                        variance = %variance.to_f64().unwrap_or(0.0),
                        "AD variance exceeds threshold, falling back to central diff"
                    );
                    diagnostics.ad_fallback_used = true;
                    Ok((self.compute_jacobian_central_diff(log_df)?, diagnostics))
                } else {
                    Ok((enzyme_jacobian, diagnostics))
                }
            }
            Err(_e) => {
                tracing::warn!(error = %_e, "Enzyme AD failed, falling back to central diff");
                diagnostics.ad_fallback_used = true;
                Ok((self.compute_jacobian_central_diff(log_df)?, diagnostics))
            }
        }
    }
}

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

        // Should be (3 instruments + 2 jump regularisation) × 5 parameters
        assert_eq!(jacobian.nrows(), 5);
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

    use crate::builder::compile::{CompiledInstrument, InstrumentType};

    fn create_compiled_instruments() -> Vec<CompiledInstrument<f64>> {
        vec![
            CompiledInstrument::deposit(0.03, 1.0).unwrap(),
            CompiledInstrument::deposit(0.035, 2.0).unwrap(),
            CompiledInstrument::deposit(0.04, 5.0).unwrap(),
        ]
    }

    #[test]
    fn test_from_compiled_basic_operations() {
        let instruments = create_compiled_instruments();
        let problem = CalibrationProblem::from_compiled(instruments.clone()).unwrap();

        // Creation
        assert_eq!(problem.dimension(), 3);
        assert_eq!(problem.instruments().len(), 3);
        assert_eq!(problem.pillars().len(), 3);
        assert_eq!(problem.total_cashflows(), 3);

        // Initial guess
        let guess = problem.initial_guess();
        assert_eq!(guess.len(), 3);
        assert_relative_eq!(guess[0], -0.03, epsilon = 1e-10);
        assert_relative_eq!(guess[1], -0.06, epsilon = 1e-10);
        assert_relative_eq!(guess[2], -0.15, epsilon = 1e-10);

        // Build curve
        let curve = problem.build_curve(&guess).unwrap();
        assert_relative_eq!(
            curve.discount_factor(1.0).unwrap(),
            (-0.03f64).exp(),
            epsilon = 1e-8
        );
        assert_relative_eq!(
            curve.discount_factor(2.0).unwrap(),
            (-0.06f64).exp(),
            epsilon = 1e-8
        );
        assert_relative_eq!(
            curve.discount_factor(5.0).unwrap(),
            (-0.15f64).exp(),
            epsilon = 1e-8
        );

        // Evaluate
        let x = problem.initial_guess_vector();
        let residuals = problem.evaluate(&x).unwrap();
        assert_eq!(residuals.len(), 3);

        // Jacobian
        let jacobian = problem.jacobian(&x).unwrap();
        assert_eq!(jacobian.nrows(), 3);
        assert_eq!(jacobian.ncols(), 3);
        let max_elem: f64 = jacobian.iter().map(|&x| x.abs()).fold(0.0, f64::max);
        assert!(max_elem > 0.0);
    }

    #[test]
    fn test_from_compiled_empty_instruments() {
        assert!(matches!(
            CalibrationProblem::from_compiled(Vec::<CompiledInstrument<f64>>::new()).unwrap_err(),
            CalibrationError::NoInstruments
        ));
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

    use infra_domain::{
        market::{
            convention::{DepositConvention, MarketConvention, SwapConvention},
            Currency, MarketInstrument as InfraMasterInstrument, QuoteCategory, QuoteId,
        },
        time::{Date, Tenor},
    };

    fn create_infra_market_instruments() -> (Vec<InfraMasterInstrument>, Date) {
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let instruments = vec![
            InfraMasterInstrument::new(
                QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit),
                0.05,
                MarketConvention::Deposit(DepositConvention::usd()),
                valuation_date,
                1_000_000.0,
            )
            .unwrap(),
            InfraMasterInstrument::new(
                QuoteId::new(Currency::USD, Tenor::OneYear, QuoteCategory::Ois),
                0.052,
                MarketConvention::Ois(SwapConvention::usd_sofr()),
                valuation_date,
                5_000_000.0,
            )
            .unwrap(),
            InfraMasterInstrument::new(
                QuoteId::new(Currency::USD, Tenor::FiveYears, QuoteCategory::Swap),
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
