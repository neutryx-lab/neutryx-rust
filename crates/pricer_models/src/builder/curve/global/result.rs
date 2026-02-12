use num_traits::Float;
use pricer_core::math::{
    linalg::{DMatrix, DVector, RealField},
    numeric::from_f64,
};

use crate::{
    builder::{error::IftError, jump::JumpPillar},
    market::curves::BootstrappedCurve,
};

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
    pub fn from_calibration_result(
        result: super::super::super::engine::CalibrationResult<T>,
    ) -> Self {
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

    /// Check if IFT sensitivity computation is possible.
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
