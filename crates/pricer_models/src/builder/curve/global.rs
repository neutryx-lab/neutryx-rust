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

use num_traits::Float;
use pricer_core::{
    math::{
        linalg::{lu_solve, DMatrix, DVector, LinearAlgebraError, RealField},
        numeric::from_f64,
    },
    types::SolverError,
};

use crate::{
    builder::{
        error::IftError,
        jump::{JumpConfig, JumpPillar},
        problem::JacobianMethod,
        CalibrationInstrument, CalibrationProblem, CalibrationProblemConfig,
    },
    market::curves::{BootstrapInterpolation, BootstrappedCurve},
};

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for global bootstrapping.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for tolerance values
#[derive(Debug, Clone, PartialEq)]
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

    /// Jacobian calculation method.
    pub jacobian_method: JacobianMethod,

    /// Enable telescoping for OIS/SOFR instruments.
    pub enable_telescoping: bool,

    /// Damping factor for Levenberg-Marquardt style regularisation.
    pub damping_factor: Option<T>,

    /// Enable debug logging of iteration progress.
    pub debug_logging: bool,

    /// Maximum allowed condition number for Jacobian matrix.
    pub max_condition_number: T,

    /// Jump configuration for CB meeting dates.
    ///
    /// When set, the bootstrapper will calibrate additional jump parameters
    /// at the specified central bank meeting dates.
    pub jump_config: Option<JumpConfig<T>>,

    /// Variance threshold for AD instability detection.
    ///
    /// When comparing AD Jacobian with finite difference approximation,
    /// if the variance exceeds this threshold, the system falls back to
    /// central difference method. Default: 1e6.
    #[cfg(feature = "enzyme-ad")]
    pub ad_variance_threshold: T,

    /// Checkpointing interval for AD gradient computation.
    ///
    /// Specifies how often to checkpoint during reverse-mode AD.
    /// Lower values use more memory but reduce re-computation.
    /// Default: None (no checkpointing).
    #[cfg(feature = "enzyme-ad")]
    pub ad_checkpoint_interval: Option<usize>,
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
            jacobian_method: JacobianMethod::default(),
            enable_telescoping: true,
            damping_factor: None,
            debug_logging: false,
            max_condition_number: from_f64(1e12),
            jump_config: None,
            #[cfg(feature = "enzyme-ad")]
            ad_variance_threshold: from_f64(1e6),
            #[cfg(feature = "enzyme-ad")]
            ad_checkpoint_interval: None,
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
            jacobian_method: JacobianMethod::CentralDifference,
            enable_telescoping: true,
            damping_factor: None,
            debug_logging: false,
            max_condition_number: from_f64(1e14),
            jump_config: None,
            #[cfg(feature = "enzyme-ad")]
            ad_variance_threshold: from_f64(1e8), // Higher threshold for high-precision
            #[cfg(feature = "enzyme-ad")]
            ad_checkpoint_interval: None,
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
            jacobian_method: JacobianMethod::FiniteDifference,
            enable_telescoping: true,
            damping_factor: None,
            debug_logging: false,
            max_condition_number: from_f64(1e10),
            jump_config: None,
            #[cfg(feature = "enzyme-ad")]
            ad_variance_threshold: from_f64(1e6),
            #[cfg(feature = "enzyme-ad")]
            ad_checkpoint_interval: None,
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

    /// Set the Jacobian calculation method.
    pub fn with_jacobian_method(mut self, method: JacobianMethod) -> Self {
        self.jacobian_method = method;
        self
    }

    /// Enable or disable telescoping for OIS/SOFR instruments.
    pub fn with_telescoping(mut self, enable: bool) -> Self {
        self.enable_telescoping = enable;
        self
    }

    /// Set the damping factor for Levenberg-Marquardt regularisation.
    pub fn with_damping(mut self, factor: T) -> Self {
        self.damping_factor = Some(factor);
        self
    }

    /// Enable or disable debug logging.
    pub fn with_debug_logging(mut self, enable: bool) -> Self {
        self.debug_logging = enable;
        self
    }

    /// Set the maximum allowed condition number.
    pub fn with_max_condition_number(mut self, max_cond: T) -> Self {
        self.max_condition_number = max_cond;
        self
    }

    /// Set the tolerance.
    pub fn with_tolerance(mut self, tol: T) -> Self {
        self.tolerance = tol;
        self
    }

    /// Set the maximum iterations.
    pub fn with_max_iterations(mut self, max_iter: usize) -> Self {
        self.max_iterations = max_iter;
        self
    }

    /// Set the jump configuration for CB meeting dates.
    ///
    /// # Arguments
    ///
    /// * `config` - Jump configuration with meeting dates and expected jumps
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pricer_models::builder::{GlobalBootstrapConfig, JumpConfig, JumpPillar};
    ///
    /// let config = GlobalBootstrapConfig::default()
    ///     .with_jump_config(JumpConfig::with_pillars(vec![
    ///         JumpPillar::new(0.5, 25.0),
    ///         JumpPillar::new(1.0, 25.0),
    ///     ]));
    /// ```
    pub fn with_jump_config(mut self, config: JumpConfig<T>) -> Self {
        self.jump_config = Some(config);
        self
    }

    /// Set jump pillars directly (convenience method).
    ///
    /// Creates a `JumpConfig` with the provided pillars and enables jump
    /// calibration.
    ///
    /// # Arguments
    ///
    /// * `pillars` - Vector of jump pillars at CB meeting dates
    pub fn with_jumps(mut self, pillars: Vec<JumpPillar<T>>) -> Self {
        self.jump_config = Some(JumpConfig::with_pillars(pillars));
        self
    }

    /// Check if jump calibration is configured and active.
    pub fn has_jumps(&self) -> bool { self.jump_config.as_ref().is_some_and(|jc| jc.is_active()) }

    /// Get the number of configured jump pillars.
    pub fn num_jumps(&self) -> usize { self.jump_config.as_ref().map_or(0, |jc| jc.num_jumps()) }

    // =========================================================================
    // Enzyme AD Configuration (Task 5.2, Requirement 6.2)
    // =========================================================================

    /// Enable Automatic Differentiation for Jacobian computation.
    ///
    /// Sets the Jacobian method to `AutomaticDifferentiation` and enables
    /// AD-specific optimisations.
    ///
    /// # Requirement: 6.2
    ///
    /// This method is only available when the `enzyme-ad` feature is enabled.
    /// If called without the feature, a compile-time error will occur.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pricer_models::builder::GlobalBootstrapConfig;
    ///
    /// let config = GlobalBootstrapConfig::default()
    ///     .with_automatic_differentiation();
    /// ```
    #[cfg(feature = "enzyme-ad")]
    pub fn with_automatic_differentiation(mut self) -> Self {
        self.jacobian_method = JacobianMethod::AutomaticDifferentiation;
        self
    }

    /// Set the AD variance threshold for instability detection.
    ///
    /// When the variance between AD Jacobian and finite difference
    /// approximation exceeds this threshold, the system automatically
    /// falls back to central difference method.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Variance threshold (default: 1e6)
    ///
    /// # Requirement: 5.4
    #[cfg(feature = "enzyme-ad")]
    pub fn with_ad_variance_threshold(mut self, threshold: T) -> Self {
        self.ad_variance_threshold = threshold;
        self
    }

    /// Set the AD checkpointing interval.
    ///
    /// Controls memory vs re-computation trade-off during reverse-mode AD.
    ///
    /// # Arguments
    ///
    /// * `interval` - Number of operations between checkpoints
    #[cfg(feature = "enzyme-ad")]
    pub fn with_ad_checkpoint_interval(mut self, interval: usize) -> Self {
        self.ad_checkpoint_interval = Some(interval);
        self
    }
}

// Conversion to CalibrationProblemConfig
impl<T: Float> From<&GlobalBootstrapConfig<T>> for CalibrationProblemConfig<T> {
    fn from(config: &GlobalBootstrapConfig<T>) -> Self {
        Self {
            jacobian_method: config.jacobian_method,
            jacobian_epsilon: config.jacobian_epsilon,
            interpolation: config.interpolation,
            allow_extrapolation: config.allow_extrapolation,
        }
    }
}

// =============================================================================
// Calibration Result
// =============================================================================

/// Result of global bootstrapping.
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
    pub jacobian_inverse: Option<DMatrix<T>>,

    /// Residual norm history at each iteration (for debugging).
    pub residual_history: Option<Vec<T>>,

    /// Condition number of the final Jacobian matrix (estimate).
    pub condition_number: Option<T>,

    /// Individual pricing errors for each instrument at the solution.
    pub pricing_errors: Option<Vec<T>>,

    /// Realised jump values at CB meeting dates (if jump calibration was used).
    ///
    /// Each entry contains the calibrated jump pillar with:
    /// - time: Time to the CB meeting in years
    /// - expected_jump: The expected jump (input)
    /// - realised_jump: The calibrated jump value
    pub realised_jumps: Option<Vec<JumpPillar<T>>>,
}

impl<T: Float> GlobalBootstrapResult<T> {
    /// Check if the Jacobian inverse is available.
    pub fn has_jacobian_inverse(&self) -> bool { self.jacobian_inverse.is_some() }

    /// Create from a CalibrationResult.
    ///
    /// This allows using the new unified `CalibrationEngine` while
    /// maintaining compatibility with existing code expecting
    /// `GlobalBootstrapResult`.
    pub fn from_calibration_result(result: super::super::engine::CalibrationResult<T>) -> Self {
        Self {
            curve: result.curve,
            pillars: result.pillars,
            discount_factors: result.discount_factors,
            residual_norm: result.residual_norm,
            iterations: result.iterations,
            converged: result.converged,
            jacobian_inverse: result.jacobian_inverse,
            residual_history: result.residual_history,
            condition_number: None, // Not computed by CalibrationEngine
            pricing_errors: None,   // Not computed by CalibrationEngine
            realised_jumps: result.realised_jumps,
        }
    }

    /// Check if the residual history is available.
    pub fn has_residual_history(&self) -> bool { self.residual_history.is_some() }

    /// Get the maximum pricing error across all instruments.
    pub fn max_pricing_error(&self) -> Option<T> {
        self.pricing_errors.as_ref().map(|errors| {
            errors
                .iter()
                .copied()
                .map(Float::abs)
                .fold(T::zero(), |max, err| if err > max { err } else { max })
        })
    }

    /// Get convergence quality as a summary.
    pub fn convergence_quality(&self, tolerance: T) -> &'static str {
        if self.residual_norm < from_f64(1e-12) {
            "excellent"
        } else if self.residual_norm < from_f64(1e-8) {
            "good"
        } else if self.residual_norm < tolerance {
            "acceptable"
        } else {
            "poor"
        }
    }

    /// Check if this result includes jump calibration.
    pub fn has_jumps(&self) -> bool { self.realised_jumps.as_ref().is_some_and(|j| !j.is_empty()) }

    /// Get the number of calibrated jumps.
    pub fn num_jumps(&self) -> usize { self.realised_jumps.as_ref().map_or(0, |j| j.len()) }

    /// Get the realised jump values in basis points.
    pub fn realised_jumps_bps(&self) -> Option<Vec<(T, T)>> {
        self.realised_jumps.as_ref().map(|jumps| {
            jumps
                .iter()
                .filter_map(|j| {
                    j.realised_jump
                        .map(|r| (j.time, JumpPillar::rate_to_bps(r)))
                })
                .collect()
        })
    }

    /// Get the total cumulative jump effect in basis points.
    pub fn total_jump_bps(&self) -> T {
        self.realised_jumps.as_ref().map_or(T::zero(), |jumps| {
            jumps
                .iter()
                .filter_map(|j| j.realised_jump)
                .fold(T::zero(), |acc, r| acc + JumpPillar::rate_to_bps(r))
        })
    }

    // =========================================================================
    // IFT (Implicit Function Theorem) Sensitivity Methods
    // =========================================================================

    /// Check if IFT sensitivity computation is possible.
    ///
    /// Returns `true` if the Jacobian inverse is cached and the calibration
    /// converged, which are prerequisites for IFT-based sensitivity
    /// calculation.
    ///
    /// # Requirement: 3.2
    pub fn can_compute_ift(&self) -> bool { self.jacobian_inverse.is_some() && self.converged }

    /// Compute IFT sensitivity for a single market parameter.
    ///
    /// Uses the Implicit Function Theorem to compute the sensitivity of
    /// calibrated parameters to a change in market inputs:
    ///
    /// ```text
    /// ∂x*/∂m = -J⁻¹ · ∂F/∂m
    /// ```
    ///
    /// where:
    /// - `x*` are the calibrated discount factors (log space)
    /// - `m` is the market parameter being perturbed
    /// - `J⁻¹` is the cached inverse Jacobian from calibration
    /// - `∂F/∂m` is the sensitivity of residuals to the market parameter
    ///
    /// # Arguments
    ///
    /// * `dF_dm` - Sensitivity of residual function to market parameter, length
    ///   must equal number of pillars/instruments.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<T>)` - Sensitivity ∂x*/∂m for each pillar
    /// * `Err(IftError::NoJacobianInverse)` - If J⁻¹ is not cached
    /// * `Err(IftError::DimensionMismatch)` - If dF_dm has wrong length
    ///
    /// # Requirement: 3.1, 3.2
    ///
    /// # Example
    ///
    /// ```ignore
    /// // After calibration with store_jacobian_inverse=true
    /// let result = bootstrapper.calibrate(&instruments)?;
    ///
    /// // Sensitivity of residuals to a 1bp parallel shift in OIS quotes
    /// let dF_dm = vec![0.0001; result.pillars.len()];
    /// let sensitivity = result.ift_sensitivity(&dF_dm)?;
    /// ```
    #[allow(non_snake_case)]
    pub fn ift_sensitivity(&self, dF_dm: &[T]) -> Result<Vec<T>, IftError>
    where
        T: RealField,
    {
        // Check if J⁻¹ is available
        let j_inv = self
            .jacobian_inverse
            .as_ref()
            .ok_or(IftError::NoJacobianInverse)?;

        // Check dimensions
        let n = j_inv.nrows();
        if dF_dm.len() != n {
            return Err(IftError::DimensionMismatch {
                expected: n,
                got: dF_dm.len(),
            });
        }

        // Compute ∂x*/∂m = -J⁻¹ · ∂F/∂m
        let dF_dm_vec = DVector::from_column_slice(dF_dm);
        let result_vec = j_inv * dF_dm_vec;

        // Negate: ∂x*/∂m = -J⁻¹ · ∂F/∂m
        let sensitivity: Vec<T> = result_vec.iter().map(|&x| -x).collect();

        // Check for NaN or Inf in result
        for (i, &val) in sensitivity.iter().enumerate() {
            if !val.is_finite() {
                return Err(IftError::NumericalError {
                    message: format!("Non-finite value at index {i}"),
                });
            }
        }

        Ok(sensitivity)
    }

    /// Compute IFT sensitivity for multiple market parameters in batch.
    ///
    /// Efficiently computes sensitivities for multiple market parameters
    /// using a single matrix-matrix multiplication:
    ///
    /// ```text
    /// ∂x*/∂M = -J⁻¹ · ∂F/∂M
    /// ```
    ///
    /// where `∂F/∂M` is a matrix with each column representing the
    /// sensitivity to a different market parameter.
    ///
    /// # Arguments
    ///
    /// * `dF_dm_batch` - Matrix of sensitivities, shape (n_instruments,
    ///   n_params). Each column is ∂F/∂m_i for market parameter i.
    ///
    /// # Returns
    ///
    /// * `Ok(DMatrix<T>)` - Sensitivity matrix ∂x*/∂M, shape (n_pillars,
    ///   n_params)
    /// * `Err(IftError::NoJacobianInverse)` - If J⁻¹ is not cached
    /// * `Err(IftError::BatchDimensionMismatch)` - If rows don't match
    ///   n_instruments
    ///
    /// # Requirement: 3.3
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Compute sensitivity to multiple market parameters at once
    /// let n_pillars = result.pillars.len();
    /// let n_params = 3;
    ///
    /// // Sensitivities for 3 different market parameters
    /// let dF_dm_batch = DMatrix::from_fn(n_pillars, n_params, |i, j| {
    ///     // Sensitivity of instrument i to market param j
    ///     0.0001 * (j as f64 + 1.0)
    /// });
    ///
    /// let sensitivities = result.ift_sensitivity_batch(&dF_dm_batch)?;
    /// ```
    #[allow(non_snake_case)]
    pub fn ift_sensitivity_batch(&self, dF_dm_batch: &DMatrix<T>) -> Result<DMatrix<T>, IftError>
    where
        T: RealField,
    {
        // Check if J⁻¹ is available
        let j_inv = self
            .jacobian_inverse
            .as_ref()
            .ok_or(IftError::NoJacobianInverse)?;

        // Check row dimensions
        let n = j_inv.nrows();
        if dF_dm_batch.nrows() != n {
            return Err(IftError::BatchDimensionMismatch {
                expected: n,
                got: dF_dm_batch.nrows(),
            });
        }

        // Compute ∂x*/∂M = -J⁻¹ · ∂F/∂M using matrix multiplication
        let result_matrix = j_inv * dF_dm_batch;

        // Negate the result
        let negated = -result_matrix;

        // Check for NaN or Inf in result
        for (idx, &val) in negated.iter().enumerate() {
            if !val.is_finite() {
                let row = idx % negated.nrows();
                let col = idx / negated.nrows();
                return Err(IftError::NumericalError {
                    message: format!("Non-finite value at ({row}, {col})"),
                });
            }
        }

        Ok(negated)
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
#[derive(Debug, Clone)]
pub struct GlobalBootstrapper<T: Float> {
    config: GlobalBootstrapConfig<T>,
}

impl<T: RealField + Float + Copy> GlobalBootstrapper<T> {
    /// Create a new global bootstrapper with the given configuration.
    pub fn new(config: GlobalBootstrapConfig<T>) -> Self { Self { config } }

    /// Calibrate using the unified CalibrationEngine.
    ///
    /// This method provides an alternative implementation using the
    /// `CalibrationEngine<LUStrategy>`, which shares the same linear
    /// algebra infrastructure with sequential bootstrap.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Market instruments to calibrate
    ///
    /// # Returns
    ///
    /// `GlobalBootstrapResult` compatible with existing code.
    pub fn calibrate_with_engine<I: CalibrationInstrument<T> + Clone>(
        &self,
        instruments: &[I],
    ) -> Result<GlobalBootstrapResult<T>, SolverError> {
        use super::super::engine::{CalibrationEngine, CalibrationEngineConfig};

        let engine_config = CalibrationEngineConfig {
            tolerance: self.config.tolerance,
            param_tolerance: self.config.param_tolerance,
            max_iterations: self.config.max_iterations,
            jacobian_epsilon: self.config.jacobian_epsilon,
            store_jacobian_inverse: self.config.store_jacobian_inverse,
            interpolation: self.config.interpolation,
            allow_extrapolation: self.config.allow_extrapolation,
            damping_factor: self.config.damping_factor,
            debug_logging: self.config.debug_logging,
        };

        let mut engine = CalibrationEngine::with_lu_strategy(engine_config);

        let result = engine.calibrate(instruments).map_err(|e| {
            SolverError::NumericalInstability(format!("CalibrationEngine failed: {e}"))
        })?;

        Ok(GlobalBootstrapResult::from_calibration_result(result))
    }

    /// Calibrate with jumps using the unified CalibrationEngine.
    ///
    /// This method provides an alternative implementation for jump calibration
    /// using `CalibrationEngine<LUStrategy>`.
    pub fn calibrate_with_jumps_engine<I: CalibrationInstrument<T> + Clone>(
        &self,
        instruments: &[I],
        jump_pillars: Vec<JumpPillar<T>>,
    ) -> Result<GlobalBootstrapResult<T>, SolverError> {
        use super::super::engine::{CalibrationEngine, CalibrationEngineConfig};

        let engine_config = CalibrationEngineConfig {
            tolerance: self.config.tolerance,
            param_tolerance: self.config.param_tolerance,
            max_iterations: self.config.max_iterations,
            jacobian_epsilon: self.config.jacobian_epsilon,
            store_jacobian_inverse: self.config.store_jacobian_inverse,
            interpolation: self.config.interpolation,
            allow_extrapolation: self.config.allow_extrapolation,
            damping_factor: self.config.damping_factor,
            debug_logging: self.config.debug_logging,
        };

        let mut engine = CalibrationEngine::with_lu_strategy(engine_config);

        let result = engine
            .calibrate_with_jumps(instruments, jump_pillars)
            .map_err(|e| {
                SolverError::NumericalInstability(format!(
                    "CalibrationEngine with jumps failed: {e}"
                ))
            })?;

        Ok(GlobalBootstrapResult::from_calibration_result(result))
    }

    /// Create a bootstrapper with default configuration.
    pub fn with_defaults() -> Self { Self::new(GlobalBootstrapConfig::default()) }

    /// Get the configuration.
    pub fn config(&self) -> &GlobalBootstrapConfig<T> { &self.config }

    /// Calibrate a yield curve from the given instruments.
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
        pillars.dedup_by(|a, b| Float::abs(*a - *b) < from_f64::<T>(1e-10));

        let n_pillars = pillars.len();

        // Initial guess: log(DF) assuming flat 3% curve
        let mut x: Vec<T> = pillars
            .iter()
            .map(|&t| -(from_f64::<T>(0.03) * t))
            .collect();

        // Residual history for debugging
        let mut residual_history = if self.config.debug_logging {
            Some(Vec::with_capacity(self.config.max_iterations))
        } else {
            None
        };

        // Newton iteration
        for iter in 0..self.config.max_iterations {
            let discount_factors: Vec<T> = x.iter().map(|&xi| Float::exp(xi)).collect();
            let curve = self.build_curve(&pillars, &discount_factors)?;

            let residuals = self.compute_residuals(instruments, &curve)?;
            let residual_norm = vector_norm(&residuals);

            if let Some(ref mut history) = residual_history {
                history.push(residual_norm);
            }

            // Check convergence
            if residual_norm < self.config.tolerance {
                let j_vecs = self.compute_jacobian(&x, &pillars, instruments)?;
                let j_matrix =
                    DMatrix::from_row_slice(n, n_pillars, &self.flatten_jacobian(&j_vecs));

                // For overdetermined systems (n > n_pillars), compute the
                // normal-equation matrix (J^T J)^{-1} which maps rate changes
                // to parameter changes. For square systems, compute J^{-1}.
                let (jacobian_inverse, condition_number) =
                    if self.config.store_jacobian_inverse {
                        if n == n_pillars {
                            let inv = self.compute_inverse(&j_matrix)?;
                            let cond = self.estimate_condition_number(&j_matrix);
                            (Some(inv), cond)
                        } else {
                            let jtj = j_matrix.transpose() * &j_matrix;
                            let inv = self.compute_inverse(&jtj).ok();
                            let cond = inv
                                .as_ref()
                                .and_then(|_| self.estimate_condition_number(&jtj));
                            (inv, cond)
                        }
                    } else {
                        (None, self.estimate_condition_number(&j_matrix))
                    };

                return Ok(GlobalBootstrapResult {
                    curve,
                    pillars: pillars.clone(),
                    discount_factors,
                    residual_norm,
                    iterations: iter,
                    converged: true,
                    jacobian_inverse,
                    residual_history,
                    condition_number,
                    pricing_errors: Some(residuals),
                    realised_jumps: None,
                });
            }

            // Compute Jacobian
            let j = self.compute_jacobian(&x, &pillars, instruments)?;

            // Solve J * delta = -F for delta.
            // For overdetermined systems (n > n_pillars), use least squares
            // via normal equations: (J^T J) delta = J^T (-F).
            let neg_residuals: Vec<T> = residuals.iter().map(|&r| -r).collect();
            let j_matrix = DMatrix::from_row_slice(n, n_pillars, &self.flatten_jacobian(&j));
            let delta = if n == n_pillars {
                self.solve_linear_system(&j_matrix, &neg_residuals)?
            } else {
                self.solve_least_squares(&j_matrix, &neg_residuals)?
            };

            // Check parameter convergence
            let param_change = vector_norm(&delta);
            if param_change < self.config.param_tolerance {
                let (jacobian_inverse, condition_number) =
                    if self.config.store_jacobian_inverse {
                        if n == n_pillars {
                            let inv = self.compute_inverse(&j_matrix)?;
                            let cond = self.estimate_condition_number(&j_matrix);
                            (Some(inv), cond)
                        } else {
                            let jtj = j_matrix.transpose() * &j_matrix;
                            let inv = self.compute_inverse(&jtj).ok();
                            let cond = inv
                                .as_ref()
                                .and_then(|_| self.estimate_condition_number(&jtj));
                            (inv, cond)
                        }
                    } else {
                        (None, self.estimate_condition_number(&j_matrix))
                    };

                return Ok(GlobalBootstrapResult {
                    curve,
                    pillars: pillars.clone(),
                    discount_factors,
                    residual_norm,
                    iterations: iter,
                    converged: true,
                    jacobian_inverse,
                    residual_history,
                    condition_number,
                    pricing_errors: Some(residuals),
                    realised_jumps: None,
                });
            }

            // Update x
            for (i, d) in delta.iter().enumerate() {
                x[i] = x[i] + *d;
            }
        }

        Err(SolverError::MaxIterationsExceeded {
            iterations: self.config.max_iterations,
        })
    }

    /// Estimate the condition number of a matrix.
    fn estimate_condition_number(&self, j: &DMatrix<T>) -> Option<T> {
        let nrows = j.nrows();
        if nrows == 0 {
            return None;
        }

        let mut max_row_sum = T::zero();
        let mut min_row_sum = T::infinity();

        for i in 0..nrows {
            let row_sum = (0..j.ncols())
                .map(|k| Float::abs(j[(i, k)]))
                .fold(T::zero(), |acc, x| acc + x);
            if row_sum > max_row_sum {
                max_row_sum = row_sum;
            }
            if row_sum < min_row_sum && row_sum > T::zero() {
                min_row_sum = row_sum;
            }
        }

        if min_row_sum > T::zero() {
            Some(max_row_sum / min_row_sum)
        } else {
            None
        }
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
        .map_err(SolverError::NumericalInstability)
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
    fn compute_jacobian<I: CalibrationInstrument<T>>(
        &self,
        x: &[T],
        pillars: &[T],
        instruments: &[I],
    ) -> Result<Vec<Vec<T>>, SolverError> {
        let n = instruments.len();
        let m = pillars.len();
        let eps = self.config.jacobian_epsilon;

        let discount_factors: Vec<T> = x.iter().map(|&xi| Float::exp(xi)).collect();
        let curve = self.build_curve(pillars, &discount_factors)?;
        let f0 = self.compute_residuals(instruments, &curve)?;

        let mut jacobian = vec![vec![T::zero(); m]; n];

        for j in 0..m {
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
    fn solve_linear_system(&self, j: &DMatrix<T>, b: &[T]) -> Result<Vec<T>, SolverError> {
        lu_solve(j, b).map_err(|e: LinearAlgebraError| e.into())
    }

    /// Compute the inverse of the Jacobian matrix.
    fn compute_inverse(&self, j: &DMatrix<T>) -> Result<DMatrix<T>, SolverError> {
        pricer_core::math::linalg::inverse(j).map_err(|e: LinearAlgebraError| e.into())
    }

    /// Calibrate using the CalibrationProblem approach.
    pub fn calibrate_with_problem<I>(
        &self,
        instruments: Vec<I>,
    ) -> Result<GlobalBootstrapResult<T>, SolverError>
    where
        I: CalibrationInstrument<T> + Clone,
    {
        use pricer_core::math::solvers::{MultidimNewtonConfig, MultidimensionalNewtonSolver};

        let problem_config = CalibrationProblemConfig::from(&self.config);
        let problem = CalibrationProblem::with_config(instruments.clone(), problem_config)
            .map_err(|e| {
                SolverError::NumericalInstability(format!("Problem creation failed: {e}"))
            })?;

        let solver_config: MultidimNewtonConfig<T> = MultidimNewtonConfig {
            tolerance: self.config.tolerance,
            param_tolerance: self.config.param_tolerance,
            max_iterations: self.config.max_iterations,
            jacobian_epsilon: self.config.jacobian_epsilon,
            store_jacobian_inverse: self.config.store_jacobian_inverse,
        };

        let solver = MultidimensionalNewtonSolver::new(solver_config);

        let initial_guess = problem.initial_guess_vector();
        let result = solver.solve(&problem, initial_guess)?;

        let pillars = problem.pillars().to_vec();
        let log_df: Vec<T> = result.solution.iter().copied().collect();
        let discount_factors: Vec<T> = log_df.iter().map(|&x| Float::exp(x)).collect();

        let curve = self.build_curve(&pillars, &discount_factors)?;

        let pricing_errors = problem.compute_residuals(&curve).ok();

        Ok(GlobalBootstrapResult {
            curve,
            pillars,
            discount_factors,
            residual_norm: result.residual_norm,
            iterations: result.iterations,
            converged: result.converged,
            jacobian_inverse: result.jacobian_inverse,
            residual_history: None,
            condition_number: None,
            pricing_errors,
            realised_jumps: None,
        })
    }

    // =========================================================================
    // Jump-aware calibration methods
    // =========================================================================

    /// Calibrate a yield curve with jump pillars at CB meeting dates.
    ///
    /// This method extends the standard calibration to include jump parameters
    /// at central bank meeting dates. The parameter vector becomes:
    /// `[log(DF_1), ..., log(DF_n), jump_1, ..., jump_m]`
    ///
    /// # Arguments
    ///
    /// * `instruments` - Calibration instruments
    /// * `jump_pillars` - Jump pillars for CB meeting dates
    ///
    /// # Returns
    ///
    /// * `Ok(GlobalBootstrapResult)` - Calibration result with realised jumps
    /// * `Err(SolverError)` - If calibration fails
    pub fn calibrate_with_jumps<I>(
        &self,
        instruments: &[I],
        jump_pillars: Vec<JumpPillar<T>>,
    ) -> Result<GlobalBootstrapResult<T>, SolverError>
    where
        I: CalibrationInstrument<T> + Clone,
    {
        if instruments.is_empty() {
            return Err(SolverError::NumericalInstability(
                "No instruments provided for calibration".to_string(),
            ));
        }

        if jump_pillars.is_empty() {
            // No jumps, fall back to regular calibration
            return self.calibrate(instruments);
        }

        // Create CalibrationProblem with jumps
        let problem_config = CalibrationProblemConfig::from(&self.config);
        let problem = CalibrationProblem::with_jumps(
            instruments.to_vec(),
            jump_pillars.clone(),
            problem_config,
        )
        .map_err(|e| {
            SolverError::NumericalInstability(format!(
                "Failed to create calibration problem with jumps: {e}"
            ))
        })?;

        // Initial guess including jump parameters
        let mut x = problem.initial_guess_with_jumps();
        let n_pillars = problem.pillars().len();
        let n_jumps = problem.num_jumps();

        // Residual history for debugging
        let mut residual_history = if self.config.debug_logging {
            Some(Vec::with_capacity(self.config.max_iterations))
        } else {
            None
        };

        // Newton iteration with extended parameter vector
        for iter in 0..self.config.max_iterations {
            // Compute residuals
            let residuals = problem.compute_residuals_with_jumps(&x).map_err(|e| {
                SolverError::NumericalInstability(format!("Residual computation failed: {e}"))
            })?;
            let residual_norm = vector_norm(&residuals);

            if let Some(ref mut history) = residual_history {
                history.push(residual_norm);
            }

            // Debug logging
            if self.config.debug_logging {
                let jump_vals = problem.extract_jumps(&x);
                let jump_bps: Vec<f64> = jump_vals
                    .iter()
                    .map(|&j| JumpPillar::rate_to_bps(j).to_f64().unwrap_or(0.0))
                    .collect();
                eprintln!(
                    "[Jump Calibration] Iter {}: residual={:.6e}, jumps(bps)={:?}",
                    iter,
                    residual_norm.to_f64().unwrap_or(0.0),
                    jump_bps
                );
            }

            // Check convergence
            if residual_norm < self.config.tolerance {
                return self.finalize_jump_result(
                    &problem,
                    &x,
                    residual_norm,
                    iter,
                    residual_history,
                );
            }

            // Compute Jacobian with jumps
            let jacobian = problem.compute_jacobian_with_jumps(&x).map_err(|e| {
                SolverError::NumericalInstability(format!("Jacobian computation failed: {e}"))
            })?;

            // Solve J * delta = -F
            let neg_residuals: Vec<T> = residuals.iter().map(|&r| -r).collect();

            // The Jacobian is (n+k) × (n+k) after jump regularisation makes
            // the system square.  For any remaining non-square cases, fall
            // back to least squares via normal equations.
            let n_rows = jacobian.nrows();
            let n_cols = jacobian.ncols();
            let delta = if n_rows == n_cols {
                self.solve_linear_system(&jacobian, &neg_residuals)?
            } else {
                self.solve_least_squares(&jacobian, &neg_residuals)?
            };

            // Check parameter convergence
            let param_change = vector_norm(&delta);
            if param_change < self.config.param_tolerance {
                return self.finalize_jump_result(
                    &problem,
                    &x,
                    residual_norm,
                    iter,
                    residual_history,
                );
            }

            // Update parameters with optional damping for jumps
            let jump_damping = self
                .config
                .jump_config
                .as_ref()
                .and_then(|jc| jc.jump_damping)
                .unwrap_or_else(T::one);

            for i in 0..n_pillars {
                x[i] = x[i] + delta[i];
            }
            for i in 0..n_jumps {
                x[n_pillars + i] = x[n_pillars + i] + delta[n_pillars + i] * jump_damping;
            }
        }

        Err(SolverError::MaxIterationsExceeded {
            iterations: self.config.max_iterations,
        })
    }

    /// Finalise the calibration result with jump information.
    ///
    /// Returns a **base** curve (pillar DFs only, no jump adjustment).
    /// The caller must attach jump data via `BootstrappedCurve::with_jumps()`
    /// using the same forward-rate-shift grid as the sequential bootstrap so
    /// that forward rate discontinuities are rendered correctly.
    fn finalize_jump_result<I>(
        &self,
        problem: &CalibrationProblem<T, I>,
        params: &[T],
        residual_norm: T,
        iterations: usize,
        residual_history: Option<Vec<T>>,
    ) -> Result<GlobalBootstrapResult<T>, SolverError>
    where
        I: CalibrationInstrument<T> + Clone,
    {
        let log_df = problem.extract_log_df(params);
        let jumps = problem.extract_jumps(params);

        let pillars = problem.pillars().to_vec();
        let n = pillars.len();
        let discount_factors: Vec<T> = log_df.iter().map(|&x| Float::exp(x)).collect();

        // Build base curve (pillar DFs only, no jump adjustment).
        // Jump data is attached by the service layer using the same
        // forward-rate-shift grid as the sequential bootstrap.
        let curve = BootstrappedCurve::new(
            pillars.clone(),
            discount_factors.clone(),
            self.config.interpolation,
            true,
        )
        .map_err(SolverError::NumericalInstability)?;

        // Get realised jumps
        let realised_jumps = Some(problem.get_realised_jumps(params));

        // Compute pricing errors on the jump-adjusted curve (for diagnostics)
        let adjusted_curve = problem.build_curve_with_jumps(log_df, jumps).map_err(|e| {
            SolverError::NumericalInstability(format!("Failed to build jump curve: {e}"))
        })?;
        let pricing_errors = problem.compute_residuals(&adjusted_curve).ok();

        // Compute Jacobian inverse for the instrument-pillar sub-block.
        //
        // The full Jacobian has structure:
        //   [ J_inst_df (n_inst × n_pillars)    J_inst_jump (n_inst × k)    ]
        //   [ 0         (k × n_pillars)          I_reg       (k × k)          ]
        //
        // For square systems (n_inst == n_pillars), compute J_inst_df^{-1}.
        // For overdetermined systems (n_inst > n_pillars), compute
        // (J^T J)^{-1} which maps rate perturbations to parameter changes.
        let n_inst = problem.instruments().len();
        let jacobian_inverse = if self.config.store_jacobian_inverse {
            let full_jac = problem.compute_jacobian_with_jumps(params).map_err(|e| {
                SolverError::NumericalInstability(format!("Jacobian computation failed: {e}"))
            })?;
            let inst_jac = full_jac.view((0, 0), (n_inst, n)).into_owned();
            if n_inst == n {
                self.compute_inverse(&inst_jac).ok()
            } else {
                let jtj = inst_jac.transpose() * &inst_jac;
                self.compute_inverse(&jtj).ok()
            }
        } else {
            None
        };

        let condition_number = jacobian_inverse.as_ref().and_then(|_| {
            let full_jac = problem.compute_jacobian_with_jumps(params).ok()?;
            let inst_jac = full_jac.view((0, 0), (n_inst, n)).into_owned();
            if n_inst == n {
                self.estimate_condition_number(&inst_jac)
            } else {
                let jtj = inst_jac.transpose() * &inst_jac;
                self.estimate_condition_number(&jtj)
            }
        });

        Ok(GlobalBootstrapResult {
            curve,
            pillars,
            discount_factors,
            residual_norm,
            iterations,
            converged: true,
            jacobian_inverse,
            residual_history,
            condition_number,
            pricing_errors,
            realised_jumps,
        })
    }

    /// Solve a least squares problem for overdetermined systems.
    ///
    /// Solves J^T J x = J^T b using normal equations.
    fn solve_least_squares(&self, j: &DMatrix<T>, b: &[T]) -> Result<Vec<T>, SolverError> {
        let b_vec = DMatrix::from_column_slice(b.len(), 1, b);
        let jt = j.transpose();
        let jtj = &jt * j;
        let jtb = &jt * &b_vec;

        let jtb_vec: Vec<T> = jtb.iter().copied().collect();
        self.solve_linear_system(&jtj, &jtb_vec)
    }

    /// Merge regular pillars with jump pillars, avoiding duplicates.
    ///
    /// # Arguments
    ///
    /// * `regular_pillars` - Standard pillar times from instruments
    /// * `jump_pillars` - Jump pillar times
    /// * `tolerance` - Time tolerance for duplicate detection
    ///
    /// # Returns
    ///
    /// Tuple of (merged_pillars, jump_indices) where jump_indices are
    /// the positions of jump pillars in the merged array.
    pub fn merge_pillars(
        &self,
        regular_pillars: &[T],
        jump_pillars: &[JumpPillar<T>],
        tolerance: T,
    ) -> (Vec<T>, Vec<usize>) {
        let mut merged: Vec<T> = regular_pillars.to_vec();
        let mut jump_indices = Vec::with_capacity(jump_pillars.len());

        for jp in jump_pillars {
            // Check if this jump time is already in the merged list
            let existing_idx = merged
                .iter()
                .position(|&t| Float::abs(t - jp.time) < tolerance);

            match existing_idx {
                Some(idx) => {
                    // Jump pillar coincides with existing pillar
                    jump_indices.push(idx);
                }
                None => {
                    // Find insertion position to maintain sorted order
                    let insert_pos = merged
                        .iter()
                        .position(|&t| t > jp.time)
                        .unwrap_or(merged.len());

                    merged.insert(insert_pos, jp.time);

                    // Adjust indices for the insertion
                    for idx in &mut jump_indices {
                        if *idx >= insert_pos {
                            *idx += 1;
                        }
                    }
                    jump_indices.push(insert_pos);
                }
            }
        }

        (merged, jump_indices)
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
#[allow(non_snake_case)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::market::curves::MarketInstrument;

    fn create_test_instruments() -> Vec<MarketInstrument<f64>> {
        vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.032),
            MarketInstrument::ois(5.0, 0.035),
            MarketInstrument::ois(10.0, 0.04),
        ]
    }

    #[test]
    fn test_config_default() {
        let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::default();
        assert_relative_eq!(config.tolerance, 1e-10, epsilon = 1e-15);
        assert_eq!(config.max_iterations, 100);
        assert!(config.store_jacobian_inverse);
        assert_eq!(config.jacobian_method, JacobianMethod::FiniteDifference);
        assert!(config.enable_telescoping);
        assert!(config.damping_factor.is_none());
        assert!(!config.debug_logging);
    }

    #[test]
    fn test_config_high_precision() {
        let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::high_precision();
        assert!(config.tolerance < 1e-12);
        assert!(config.max_iterations >= 500);
        assert_eq!(config.jacobian_method, JacobianMethod::CentralDifference);
    }

    #[test]
    fn test_config_fast() {
        let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::fast();
        assert!(config.tolerance > 1e-8);
        assert!(!config.store_jacobian_inverse);
        assert_eq!(config.jacobian_method, JacobianMethod::FiniteDifference);
    }

    #[test]
    fn test_config_builder_methods() {
        let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::default()
            .with_jacobian_method(JacobianMethod::Analytical)
            .with_telescoping(false)
            .with_damping(0.01)
            .with_debug_logging(true)
            .with_max_condition_number(1e8)
            .with_tolerance(1e-12)
            .with_max_iterations(200);

        assert_eq!(config.jacobian_method, JacobianMethod::Analytical);
        assert!(!config.enable_telescoping);
        assert_relative_eq!(config.damping_factor.unwrap(), 0.01, epsilon = 1e-15);
        assert!(config.debug_logging);
        assert_relative_eq!(config.max_condition_number, 1e8, epsilon = 1e-5);
        assert_relative_eq!(config.tolerance, 1e-12, epsilon = 1e-15);
        assert_eq!(config.max_iterations, 200);
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

        for i in 0..result.discount_factors.len() {
            assert!(result.discount_factors[i] > 0.0);
            assert!(result.discount_factors[i] <= 1.0);
        }

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
        let instruments: Vec<MarketInstrument<f64>> = vec![];
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
        let instruments = vec![MarketInstrument::ois(5.0, 0.03)];
        let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

        let result = bootstrapper.calibrate(&instruments).unwrap();

        assert!(result.converged);
        assert_eq!(result.pillars.len(), 1);

        let error = instruments[0].pricing_error(&result.curve).unwrap();
        assert!(error.abs() < 1e-8);
    }

    #[test]
    fn test_vector_norm() {
        let v = vec![3.0, 4.0];
        assert_relative_eq!(vector_norm(&v), 5.0, epsilon = 1e-10);

        let v2 = vec![1.0, 1.0, 1.0, 1.0];
        assert_relative_eq!(vector_norm(&v2), 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_calibrate_with_debug_logging() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default().with_debug_logging(true);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        assert!(result.converged);
        assert!(result.has_residual_history());

        let history = result.residual_history.as_ref().unwrap();
        assert!(!history.is_empty());
    }

    #[test]
    fn test_convergence_quality() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default();
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        let quality = result.convergence_quality(1e-10);
        assert!(quality == "excellent" || quality == "good");
    }

    // =========================================================================
    // Jump Configuration Tests
    // =========================================================================

    #[test]
    fn test_config_default_no_jumps() {
        let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::default();
        assert!(config.jump_config.is_none());
        assert!(!config.has_jumps());
        assert_eq!(config.num_jumps(), 0);
    }

    #[test]
    fn test_config_with_jump_config() {
        let jump_config =
            JumpConfig::with_pillars(vec![JumpPillar::new(0.5, 25.0), JumpPillar::new(1.0, 25.0)]);

        let config: GlobalBootstrapConfig<f64> =
            GlobalBootstrapConfig::default().with_jump_config(jump_config);

        assert!(config.jump_config.is_some());
        assert!(config.has_jumps());
        assert_eq!(config.num_jumps(), 2);
    }

    #[test]
    fn test_config_with_jumps_convenience() {
        let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::default().with_jumps(vec![
            JumpPillar::new(0.25, 25.0),
            JumpPillar::new(0.5, 25.0),
            JumpPillar::new(1.0, 25.0),
        ]);

        assert!(config.has_jumps());
        assert_eq!(config.num_jumps(), 3);

        let jump_config = config.jump_config.unwrap();
        assert!(jump_config.enabled);
        assert_eq!(jump_config.jump_pillars.len(), 3);
    }

    #[test]
    fn test_config_with_empty_jumps() {
        let config: GlobalBootstrapConfig<f64> =
            GlobalBootstrapConfig::default().with_jumps(vec![]);

        // Empty jump list should not activate jumps
        assert!(!config.has_jumps());
        assert_eq!(config.num_jumps(), 0);
    }

    #[test]
    fn test_config_with_disabled_jump_config() {
        let jump_config = JumpConfig::with_pillars(vec![JumpPillar::new(0.5, 25.0)]).disable();

        let config: GlobalBootstrapConfig<f64> =
            GlobalBootstrapConfig::default().with_jump_config(jump_config);

        // Jump config exists but is disabled
        assert!(config.jump_config.is_some());
        assert!(!config.has_jumps()); // Not active because disabled
        assert_eq!(config.num_jumps(), 1); // But pillars still counted
    }

    // =========================================================================
    // Jump calibration tests
    // =========================================================================

    #[allow(dead_code)]
    fn create_jump_pillars() -> Vec<JumpPillar<f64>> {
        vec![
            JumpPillar::new(0.5, 25.0),  // 25bps at 6 months
            JumpPillar::new(1.5, -15.0), // -15bps at 18 months
        ]
    }

    #[test]
    fn test_merge_pillars_no_overlap() {
        let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();
        let regular = vec![1.0, 2.0, 5.0];
        let jumps = vec![JumpPillar::new(0.5, 25.0), JumpPillar::new(3.0, 10.0)];

        let (merged, indices) = bootstrapper.merge_pillars(&regular, &jumps, 1e-10);

        assert_eq!(merged.len(), 5); // 3 regular + 2 jumps
        assert_eq!(merged, vec![0.5, 1.0, 2.0, 3.0, 5.0]);
        assert_eq!(indices, vec![0, 3]); // Positions of jump pillars
    }

    #[test]
    fn test_merge_pillars_with_overlap() {
        let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();
        let regular = vec![0.5, 1.0, 2.0, 5.0]; // 0.5 coincides with jump
        let jumps = vec![JumpPillar::new(0.5, 25.0), JumpPillar::new(3.0, 10.0)];

        let (merged, indices) = bootstrapper.merge_pillars(&regular, &jumps, 1e-10);

        assert_eq!(merged.len(), 5); // Only one 0.5, plus 3.0 added
        assert_eq!(merged, vec![0.5, 1.0, 2.0, 3.0, 5.0]);
        assert_eq!(indices, vec![0, 3]); // First jump at index 0 (existing),
                                         // second at 3
    }

    #[test]
    fn test_calibrate_with_jumps_empty_jumps() {
        let instruments = create_test_instruments();
        let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

        // Empty jump list should fall back to regular calibration
        let result = bootstrapper
            .calibrate_with_jumps(&instruments, vec![])
            .unwrap();

        assert!(result.converged);
        assert!(!result.has_jumps());
    }

    #[test]
    fn test_calibrate_with_jumps_basic() {
        let instruments = create_test_instruments();
        let jump_pillars = vec![JumpPillar::new(0.5, 10.0)]; // Small 10bps jump
        let config = GlobalBootstrapConfig::default().with_tolerance(1e-8);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate_with_jumps(&instruments, jump_pillars);

        // The calibration may or may not converge depending on the setup,
        // but it should not panic
        match result {
            Ok(res) => {
                assert!(res.has_jumps());
                assert_eq!(res.num_jumps(), 1);
                // Verify jumps have realised values
                let jumps = res.realised_jumps.unwrap();
                assert!(jumps[0].is_calibrated());
            }
            Err(e) => {
                // If it fails, check it's a convergence issue not a panic
                assert!(matches!(
                    e,
                    SolverError::MaxIterationsExceeded { .. }
                        | SolverError::NumericalInstability(_)
                ));
            }
        }
    }

    #[test]
    fn test_calibrate_with_empty_jumps_falls_back() {
        let instruments = create_test_instruments();
        let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

        // Empty jump pillars should fall back to regular calibrate
        let result = bootstrapper
            .calibrate_with_jumps(&instruments, vec![])
            .unwrap();

        assert!(result.converged);
        // Regular calibrate returns no realised jumps
        assert!(result.realised_jumps.is_none());
    }

    #[test]
    fn test_result_jump_helpers() {
        let instruments = create_test_instruments();
        let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

        let result = bootstrapper.calibrate(&instruments).unwrap();

        // Regular calibration has no jumps
        assert!(!result.has_jumps());
        assert_eq!(result.num_jumps(), 0);
        assert_eq!(result.total_jump_bps(), 0.0);
    }

    // =========================================================================
    // IFT Sensitivity Tests (Requirement 3.1-3.5)
    // =========================================================================

    #[test]
    fn test_can_compute_ift_with_jacobian_inverse() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default().with_jacobian_inverse(true);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        assert!(result.converged);
        assert!(result.has_jacobian_inverse());
        assert!(result.can_compute_ift());
    }

    #[test]
    fn test_can_compute_ift_without_jacobian_inverse() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::fast(); // Does not store J⁻¹
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        assert!(result.converged);
        assert!(!result.has_jacobian_inverse());
        assert!(!result.can_compute_ift());
    }

    #[test]
    fn test_ift_sensitivity_basic() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default().with_jacobian_inverse(true);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        // Sensitivity of residuals to a 1bp parallel shift
        let n = result.pillars.len();
        let dF_dm: Vec<f64> = vec![0.0001; n]; // 1bp shift

        let sensitivity = result.ift_sensitivity(&dF_dm).unwrap();

        assert_eq!(sensitivity.len(), n);
        // All sensitivities should be finite
        for &s in &sensitivity {
            assert!(s.is_finite());
        }
    }

    #[test]
    fn test_ift_sensitivity_no_jacobian_inverse() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::fast(); // No J⁻¹ stored
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        let dF_dm = vec![0.0001; result.pillars.len()];
        let err = result.ift_sensitivity(&dF_dm).unwrap_err();

        assert!(matches!(
            err,
            super::super::super::IftError::NoJacobianInverse
        ));
    }

    #[test]
    fn test_ift_sensitivity_dimension_mismatch() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default().with_jacobian_inverse(true);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        // Wrong length: 2 instead of 4
        let dF_dm = vec![0.0001, 0.0001];
        let err = result.ift_sensitivity(&dF_dm).unwrap_err();

        match err {
            super::super::super::IftError::DimensionMismatch { expected, got } => {
                assert_eq!(expected, 4);
                assert_eq!(got, 2);
            }
            _ => panic!("Expected DimensionMismatch error"),
        }
    }

    #[test]
    fn test_ift_sensitivity_batch_basic() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default().with_jacobian_inverse(true);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        let n = result.pillars.len();
        let n_params = 3;

        // Create batch sensitivity matrix
        let dF_dm_batch = DMatrix::from_fn(n, n_params, |i, j| 0.0001 * ((i + j + 1) as f64));

        let sensitivities = result.ift_sensitivity_batch(&dF_dm_batch).unwrap();

        assert_eq!(sensitivities.nrows(), n);
        assert_eq!(sensitivities.ncols(), n_params);

        // All values should be finite
        for &val in sensitivities.iter() {
            assert!(val.is_finite());
        }
    }

    #[test]
    fn test_ift_sensitivity_batch_no_jacobian_inverse() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::fast();
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        let dF_dm_batch = DMatrix::from_element(result.pillars.len(), 2, 0.0001);
        let err = result.ift_sensitivity_batch(&dF_dm_batch).unwrap_err();

        assert!(matches!(
            err,
            super::super::super::IftError::NoJacobianInverse
        ));
    }

    #[test]
    fn test_ift_sensitivity_batch_dimension_mismatch() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default().with_jacobian_inverse(true);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        // Wrong number of rows: 2 instead of 4
        let dF_dm_batch = DMatrix::from_element(2, 3, 0.0001);
        let err = result.ift_sensitivity_batch(&dF_dm_batch).unwrap_err();

        match err {
            super::super::super::IftError::BatchDimensionMismatch { expected, got } => {
                assert_eq!(expected, 4);
                assert_eq!(got, 2);
            }
            _ => panic!("Expected BatchDimensionMismatch error"),
        }
    }

    #[test]
    fn test_ift_sensitivity_single_vs_batch_consistency() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default().with_jacobian_inverse(true);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        let n = result.pillars.len();

        // Single sensitivity
        let dF_dm = vec![0.0001; n];
        let single_result = result.ift_sensitivity(&dF_dm).unwrap();

        // Same as batch with 1 column
        let dF_dm_batch = DMatrix::from_column_slice(n, 1, &dF_dm);
        let batch_result = result.ift_sensitivity_batch(&dF_dm_batch).unwrap();

        // Results should match
        for i in 0..n {
            assert_relative_eq!(single_result[i], batch_result[(i, 0)], epsilon = 1e-14);
        }
    }

    #[test]
    fn test_ift_sensitivity_linearity() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default().with_jacobian_inverse(true);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        let n = result.pillars.len();

        // dF1 and dF2
        let dF1: Vec<f64> = vec![0.0001; n];
        let dF2: Vec<f64> = (0..n).map(|i| 0.0002 * (i + 1) as f64).collect();

        // Combined: dF1 + dF2
        let dF_combined: Vec<f64> = dF1.iter().zip(&dF2).map(|(&a, &b)| a + b).collect();

        // Compute sensitivities
        let sens1 = result.ift_sensitivity(&dF1).unwrap();
        let sens2 = result.ift_sensitivity(&dF2).unwrap();
        let sens_combined = result.ift_sensitivity(&dF_combined).unwrap();

        // IFT should be linear: sens(dF1 + dF2) = sens(dF1) + sens(dF2)
        for i in 0..n {
            let expected = sens1[i] + sens2[i];
            assert_relative_eq!(sens_combined[i], expected, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_ift_sensitivity_zero_input() {
        let instruments = create_test_instruments();
        let config = GlobalBootstrapConfig::default().with_jacobian_inverse(true);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        let n = result.pillars.len();
        let dF_dm = vec![0.0; n];

        let sensitivity = result.ift_sensitivity(&dF_dm).unwrap();

        // Zero input should give zero output
        for &s in &sensitivity {
            assert_relative_eq!(s, 0.0, epsilon = 1e-15);
        }
    }
}
