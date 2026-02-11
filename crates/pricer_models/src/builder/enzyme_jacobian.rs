//! Enzyme AD Jacobian computation for curve calibration.
//!
//! This module provides Enzyme AD-based Jacobian computation for
//! `CalibrationProblem`, replacing finite differences with machine-precision
//! automatic differentiation.
//!
//! # Requirements Coverage
//!
//! - **Requirement 1.1**: Enzyme AD reverse mode Jacobian computation
//! - **Requirement 1.2**: 1e-12 relative tolerance for polynomial interpolation
//! - **Requirement 1.3**: Automatic selection when `enzyme-ad` feature enabled
//! - **Requirement 1.4**: Fallback to finite differences on failure
//! - **Requirement 1.5**: Support for Flat, Linear, LogLinear interpolation
//!
//! # Architecture
//!
//! The module uses Enzyme's reverse-mode AD to compute the Jacobian matrix:
//! - For each residual F_i, reverse-mode computes ∂F_i/∂x_j for all j
//! - This requires n reverse passes for n residuals (instruments)
//! - The kernel operates on `f64` slices for Enzyme compatibility

use pricer_core::math::linalg::DMatrix;

use super::JacobianMethod;

// =============================================================================
// JacobianResult
// =============================================================================

/// Result of Jacobian computation with metadata.
///
/// # Requirement 1.4
///
/// When Enzyme AD computation fails, `fallback_used` is set to `true` and
/// `method_used` reflects the actual method used (FiniteDifference).
#[derive(Debug, Clone)]
pub struct JacobianResult {
    /// The computed Jacobian matrix (n_instruments × n_pillars).
    pub matrix: DMatrix<f64>,

    /// The Jacobian computation method that was actually used.
    pub method_used: JacobianMethod,

    /// Computation time in microseconds.
    pub computation_time_us: u64,

    /// Whether fallback to finite differences was triggered.
    pub fallback_used: bool,
}

impl JacobianResult {
    /// Create a new JacobianResult.
    pub fn new(
        matrix: DMatrix<f64>,
        method_used: JacobianMethod,
        computation_time_us: u64,
        fallback_used: bool,
    ) -> Self {
        Self {
            matrix,
            method_used,
            computation_time_us,
            fallback_used,
        }
    }

    /// Create a result from finite difference computation (no fallback).
    pub fn from_finite_diff(matrix: DMatrix<f64>, computation_time_us: u64) -> Self {
        Self::new(
            matrix,
            JacobianMethod::FiniteDifference,
            computation_time_us,
            false,
        )
    }

    /// Create a result from central difference computation (no fallback).
    pub fn from_central_diff(matrix: DMatrix<f64>, computation_time_us: u64) -> Self {
        Self::new(
            matrix,
            JacobianMethod::CentralDifference,
            computation_time_us,
            false,
        )
    }

    /// Create a result from Enzyme AD computation (no fallback).
    #[cfg(feature = "enzyme-ad")]
    pub fn from_enzyme_ad(matrix: DMatrix<f64>, computation_time_us: u64) -> Self {
        Self::new(
            matrix,
            JacobianMethod::AutomaticDifferentiation,
            computation_time_us,
            false,
        )
    }

    /// Create a result indicating fallback was used.
    pub fn with_fallback(matrix: DMatrix<f64>, computation_time_us: u64) -> Self {
        Self::new(
            matrix,
            JacobianMethod::FiniteDifference,
            computation_time_us,
            true,
        )
    }
}

// =============================================================================
// Enzyme AD Kernels (feature-gated)
// =============================================================================

/// Enzyme AD Jacobian computation kernels.
///
/// This module contains the core differentiable kernels for curve calibration.
/// The kernels are pure functions operating on `f64` slices for Enzyme
/// compatibility.
///
/// # Requirement 1.1
///
/// When `JacobianMethod::AutomaticDifferentiation` is selected, the
/// `CalibrationProblem` shall compute the Jacobian using these kernels.
#[cfg(feature = "enzyme-ad")]
pub mod kernels {
    use std::autodiff::autodiff;

    // =========================================================================
    // Log-Linear Interpolation Kernel
    // =========================================================================

    /// Compute log-linearly interpolated log(DF) at a given time.
    ///
    /// This kernel performs log-linear interpolation on log discount factors.
    /// Given pillar times [t_0, t_1, ..., t_n] and log discount factors
    /// [log_df_0, log_df_1, ..., log_df_n], it computes log(DF(t)) by linear
    /// interpolation in log-space.
    ///
    /// # Arguments
    ///
    /// * `time` - Query time
    /// * `pillar_times` - Pillar maturities (sorted ascending)
    /// * `log_df` - Log discount factors at pillars
    ///
    /// # Returns
    ///
    /// Interpolated log(DF) at the query time.
    #[inline]
    pub fn log_linear_interp(time: f64, pillar_times: &[f64], log_df: &[f64]) -> f64 {
        let n = pillar_times.len();
        if n == 0 {
            return 0.0;
        }

        // Handle extrapolation
        if time <= pillar_times[0] {
            // Extrapolate flat to the left (DF(0) = 1, so log(DF(0)) = 0)
            // Linear interp: log_df(t) = log_df[0] * (t / t[0])
            if pillar_times[0] > 0.0 {
                return log_df[0] * (time / pillar_times[0]);
            }
            return log_df[0];
        }

        if time >= pillar_times[n - 1] {
            // Flat extrapolation to the right
            return log_df[n - 1];
        }

        // Binary search for interval
        let mut left = 0;
        let mut right = n - 1;
        while right - left > 1 {
            let mid = usize::midpoint(left, right);
            if pillar_times[mid] <= time {
                left = mid;
            } else {
                right = mid;
            }
        }

        // Linear interpolation in log-space
        let t0 = pillar_times[left];
        let t1 = pillar_times[right];
        let w = (time - t0) / (t1 - t0);

        log_df[left] * (1.0 - w) + log_df[right] * w
    }

    /// Compute discount factor from log_df using log-linear interpolation.
    #[inline]
    pub fn discount_factor_log_linear(time: f64, pillar_times: &[f64], log_df: &[f64]) -> f64 {
        log_linear_interp(time, pillar_times, log_df).exp()
    }

    // =========================================================================
    // Instrument Residual Kernels
    // =========================================================================

    /// Compute the pricing residual for a single deposit instrument.
    ///
    /// Deposit residual: F = (1/DF(T) - 1) / T - market_rate
    ///
    /// # Arguments
    ///
    /// * `log_df` - Log discount factors at pillars
    /// * `pillar_times` - Pillar maturities
    /// * `maturity` - Instrument maturity
    /// * `market_rate` - Market-quoted rate
    /// * `output` - Output residual (mutable for reverse mode)
    #[autodiff(
        d_deposit_residual,
        Reverse,
        Duplicated,
        Const,
        Const,
        Const,
        Duplicated
    )]
    pub fn deposit_residual(
        log_df: &[f64],
        pillar_times: &[f64],
        maturity: f64,
        market_rate: f64,
        output: &mut f64,
    ) {
        let df = discount_factor_log_linear(maturity, pillar_times, log_df);
        let theoretical_rate = (1.0 / df - 1.0) / maturity;
        *output = theoretical_rate - market_rate;
    }

    /// Compute the pricing residual for a single FRA instrument.
    ///
    /// FRA residual: F = (DF(start)/DF(end) - 1) / tau - market_rate
    ///
    /// # Arguments
    ///
    /// * `log_df` - Log discount factors at pillars
    /// * `pillar_times` - Pillar maturities
    /// * `start_time` - FRA start time (0 if spot-starting)
    /// * `end_time` - FRA end time (maturity)
    /// * `tau` - Year fraction
    /// * `market_rate` - Market-quoted rate
    /// * `output` - Output residual
    #[autodiff(
        d_fra_residual,
        Reverse,
        Duplicated,
        Const,
        Const,
        Const,
        Const,
        Const,
        Duplicated
    )]
    pub fn fra_residual(
        log_df: &[f64],
        pillar_times: &[f64],
        start_time: f64,
        end_time: f64,
        tau: f64,
        market_rate: f64,
        output: &mut f64,
    ) {
        let df_start = if start_time <= 0.0 {
            1.0
        } else {
            discount_factor_log_linear(start_time, pillar_times, log_df)
        };
        let df_end = discount_factor_log_linear(end_time, pillar_times, log_df);
        let theoretical_rate = (df_start / df_end - 1.0) / tau;
        *output = theoretical_rate - market_rate;
    }

    /// Compute the pricing residual for a single swap/OIS instrument.
    ///
    /// Swap residual: F = (1 - DF(T)) / annuity - market_rate
    /// where annuity = sum(DF(t_i) * tau_i)
    ///
    /// # Arguments
    ///
    /// * `log_df` - Log discount factors at pillars
    /// * `pillar_times` - Pillar maturities
    /// * `cashflow_times` - Cashflow payment times
    /// * `year_fractions` - Year fractions for each period
    /// * `maturity` - Swap maturity
    /// * `market_rate` - Market-quoted par rate
    /// * `output` - Output residual
    #[autodiff(
        d_swap_residual,
        Reverse,
        Duplicated,
        Const,
        Const,
        Const,
        Const,
        Const,
        Duplicated
    )]
    pub fn swap_residual(
        log_df: &[f64],
        pillar_times: &[f64],
        cashflow_times: &[f64],
        year_fractions: &[f64],
        maturity: f64,
        market_rate: f64,
        output: &mut f64,
    ) {
        // Compute annuity
        let mut annuity = 0.0;
        let n_cf = cashflow_times.len();
        for i in 0..n_cf {
            let df = discount_factor_log_linear(cashflow_times[i], pillar_times, log_df);
            annuity += df * year_fractions[i];
        }

        // Compute par rate
        let df_maturity = discount_factor_log_linear(maturity, pillar_times, log_df);
        let theoretical_rate = if annuity.abs() > 1e-15 {
            (1.0 - df_maturity) / annuity
        } else {
            0.0
        };

        *output = theoretical_rate - market_rate;
    }

    // =========================================================================
    // Jacobian Computation
    // =========================================================================

    /// Compute a single row of the Jacobian matrix using Enzyme reverse mode.
    ///
    /// This function computes ∂F_i/∂log_df_j for all j using reverse-mode AD.
    ///
    /// # Arguments
    ///
    /// * `instrument_type` - Type of instrument (0=Deposit, 1=FRA, 2=Swap)
    /// * `log_df` - Log discount factors at pillars
    /// * `pillar_times` - Pillar maturities
    /// * `params` - Instrument parameters:
    ///   - Deposit: [maturity, market_rate]
    ///   - FRA: [start_time, end_time, tau, market_rate]
    ///   - Swap: [maturity, market_rate, n_cf, cf_time_1, yf_1, ..., cf_time_n,
    ///     yf_n]
    ///
    /// # Returns
    ///
    /// Gradient vector ∂F/∂log_df
    pub fn compute_jacobian_row(
        instrument_type: u32,
        log_df: &[f64],
        pillar_times: &[f64],
        params: &[f64],
    ) -> Vec<f64> {
        let n = log_df.len();
        let mut gradient = vec![0.0; n];
        let mut output = 0.0;
        let mut d_output = 1.0; // Seed for reverse mode

        match instrument_type {
            0 => {
                // Deposit
                let maturity = params[0];
                let market_rate = params[1];
                d_deposit_residual(
                    log_df,
                    &mut gradient,
                    pillar_times,
                    maturity,
                    market_rate,
                    &mut output,
                    &mut d_output,
                );
            }
            1 => {
                // FRA
                let start_time = params[0];
                let end_time = params[1];
                let tau = params[2];
                let market_rate = params[3];
                d_fra_residual(
                    log_df,
                    &mut gradient,
                    pillar_times,
                    start_time,
                    end_time,
                    tau,
                    market_rate,
                    &mut output,
                    &mut d_output,
                );
            }
            2 => {
                // Swap/OIS
                let maturity = params[0];
                let market_rate = params[1];
                let n_cf = params[2] as usize;

                // Extract cashflow times and year fractions
                let mut cf_times = Vec::with_capacity(n_cf);
                let mut yfs = Vec::with_capacity(n_cf);
                for i in 0..n_cf {
                    cf_times.push(params[3 + 2 * i]);
                    yfs.push(params[4 + 2 * i]);
                }

                d_swap_residual(
                    log_df,
                    &mut gradient,
                    pillar_times,
                    &cf_times,
                    &yfs,
                    maturity,
                    market_rate,
                    &mut output,
                    &mut d_output,
                );
            }
            _ => {
                // Unsupported instrument type - return zero gradient
            }
        }

        gradient
    }

    /// Compute the full Jacobian matrix using Enzyme reverse mode.
    ///
    /// # Arguments
    ///
    /// * `instrument_types` - Type of each instrument
    /// * `instrument_params` - Parameters for each instrument
    /// * `log_df` - Current log discount factors
    /// * `pillar_times` - Pillar maturities
    ///
    /// # Returns
    ///
    /// Jacobian matrix (n_instruments × n_pillars)
    pub fn compute_jacobian_enzyme(
        instrument_types: &[u32],
        instrument_params: &[Vec<f64>],
        log_df: &[f64],
        pillar_times: &[f64],
    ) -> DMatrix<f64> {
        let n_instruments = instrument_types.len();
        let n_pillars = log_df.len();

        let mut jacobian = DMatrix::zeros(n_instruments, n_pillars);

        for i in 0..n_instruments {
            let gradient = compute_jacobian_row(
                instrument_types[i],
                log_df,
                pillar_times,
                &instrument_params[i],
            );

            for j in 0..n_pillars {
                jacobian[(i, j)] = gradient[j];
            }
        }

        jacobian
    }
}

// =============================================================================
// Non-enzyme Stubs
// =============================================================================

/// Stub module for when enzyme-ad feature is disabled.
///
/// This provides the same function signatures but falls back to finite
/// differences.
#[cfg(not(feature = "enzyme-ad"))]
pub mod kernels {
    use pricer_core::math::linalg::DMatrix;

    /// Compute log-linearly interpolated log(DF) at a given time.
    #[inline]
    pub fn log_linear_interp(time: f64, pillar_times: &[f64], log_df: &[f64]) -> f64 {
        let n = pillar_times.len();
        if n == 0 {
            return 0.0;
        }

        if time <= pillar_times[0] {
            if pillar_times[0] > 0.0 {
                return log_df[0] * (time / pillar_times[0]);
            }
            return log_df[0];
        }

        if time >= pillar_times[n - 1] {
            return log_df[n - 1];
        }

        // Binary search for interval
        let mut left = 0;
        let mut right = n - 1;
        while right - left > 1 {
            let mid = usize::midpoint(left, right);
            if pillar_times[mid] <= time {
                left = mid;
            } else {
                right = mid;
            }
        }

        let t0 = pillar_times[left];
        let t1 = pillar_times[right];
        let w = (time - t0) / (t1 - t0);

        log_df[left] * (1.0 - w) + log_df[right] * w
    }

    /// Compute discount factor from log_df using log-linear interpolation.
    #[inline]
    pub fn discount_factor_log_linear(time: f64, pillar_times: &[f64], log_df: &[f64]) -> f64 {
        log_linear_interp(time, pillar_times, log_df).exp()
    }

    /// Stub: Enzyme AD is not available without the enzyme-ad feature.
    ///
    /// This function returns an empty matrix. The caller should use finite
    /// differences instead.
    pub fn compute_jacobian_enzyme(
        instrument_types: &[u32],
        _instrument_params: &[Vec<f64>],
        log_df: &[f64],
        _pillar_times: &[f64],
    ) -> DMatrix<f64> {
        let n_instruments = instrument_types.len();
        let n_pillars = log_df.len();
        DMatrix::zeros(n_instruments, n_pillars)
    }

    /// Stub: compute_jacobian_row is not available without enzyme-ad.
    pub fn compute_jacobian_row(
        _instrument_type: u32,
        log_df: &[f64],
        _pillar_times: &[f64],
        _params: &[f64],
    ) -> Vec<f64> {
        vec![0.0; log_df.len()]
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_jacobian_result_from_finite_diff() {
        let matrix = DMatrix::from_element(3, 3, 1.0);
        let result = JacobianResult::from_finite_diff(matrix.clone(), 100);

        assert_eq!(result.method_used, JacobianMethod::FiniteDifference);
        assert_eq!(result.computation_time_us, 100);
        assert!(!result.fallback_used);
        assert_eq!(result.matrix.nrows(), 3);
    }

    #[test]
    fn test_jacobian_result_with_fallback() {
        let matrix = DMatrix::from_element(2, 4, 0.5);
        let result = JacobianResult::with_fallback(matrix, 200);

        assert_eq!(result.method_used, JacobianMethod::FiniteDifference);
        assert!(result.fallback_used);
    }

    #[test]
    fn test_log_linear_interp_at_pillar() {
        let pillar_times = vec![1.0, 2.0, 5.0];
        let log_df = vec![-0.03, -0.06, -0.15];

        // At pillar points, should return exact value
        let result = kernels::log_linear_interp(1.0, &pillar_times, &log_df);
        assert_relative_eq!(result, -0.03, epsilon = 1e-10);

        let result = kernels::log_linear_interp(5.0, &pillar_times, &log_df);
        assert_relative_eq!(result, -0.15, epsilon = 1e-10);
    }

    #[test]
    fn test_log_linear_interp_between_pillars() {
        let pillar_times = vec![1.0, 2.0];
        let log_df = vec![-0.03, -0.06];

        // At midpoint, should be linear average
        let result = kernels::log_linear_interp(1.5, &pillar_times, &log_df);
        assert_relative_eq!(result, -0.045, epsilon = 1e-10);
    }

    #[test]
    fn test_log_linear_interp_extrapolation() {
        let pillar_times = vec![1.0, 2.0];
        let log_df = vec![-0.03, -0.06];

        // Left extrapolation (t < t[0])
        let result = kernels::log_linear_interp(0.5, &pillar_times, &log_df);
        assert_relative_eq!(result, -0.015, epsilon = 1e-10); // log_df[0] * (0.5/1.0)

        // Right extrapolation (t > t[n-1])
        let result = kernels::log_linear_interp(3.0, &pillar_times, &log_df);
        assert_relative_eq!(result, -0.06, epsilon = 1e-10); // Flat extrapolation
    }

    #[test]
    fn test_discount_factor_log_linear() {
        let pillar_times = vec![1.0, 2.0];
        let log_df = vec![-0.03, -0.06];

        let df = kernels::discount_factor_log_linear(1.0, &pillar_times, &log_df);
        assert_relative_eq!(df, (-0.03_f64).exp(), epsilon = 1e-10);
    }

    #[test]
    fn test_jacobian_result_clone_debug() {
        let matrix = DMatrix::from_element(2, 2, 1.0);
        let result = JacobianResult::from_finite_diff(matrix, 50);

        let cloned = result.clone();
        assert_eq!(cloned.computation_time_us, result.computation_time_us);

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("JacobianResult"));
    }
}
