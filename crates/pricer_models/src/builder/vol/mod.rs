//! Volatility surface and cube calibration module.
//!
//! This module provides slice-wise calibration for volatility surfaces and
//! cubes:
//!
//! - **FX volatility surface** ([`FxVolBuilder`]): 2D (expiry × strike)
//! - **Swaption volatility cube** ([`VolCubeBuilder`]): 3D (expiry × tenor ×
//!   strike)
//!
//! ## Calibration Pattern
//!
//! Unlike global curve calibration, vol surfaces use **slice-wise**
//! calibration:
//!
//! 1. Calibrate each expiry/tenor slice independently
//! 2. Aggregate calibrated slices into a complete surface/cube
//!
//! This approach is efficient because slices are independent.

mod cube;
mod surface;

use std::cmp::Ordering;

pub use cube::{VolCubeBuilder, VolCubeResult};
use num_traits::Float;
use pricer_core::math::{
    formulas::sabr::{sabr_implied_vol, SabrImpliedVolParams},
    numeric::from_f64,
    solvers::{LMConfig, LevenbergMarquardtSolver},
};
pub use surface::{FxVolBuilder, FxVolResult};

/// Trait for volatility surface/cube builders (configure -> validate -> calibrate).
pub trait VolBuilder<T: Float> {
    /// The calibration result type.
    type Result;

    /// Returns the calibration configuration.
    fn config(&self) -> &SliceCalibrationConfig<T>;

    /// Returns the number of slices currently loaded.
    fn num_slices(&self) -> usize;

    /// Validates that the builder has sufficient data for calibration.
    fn validate(&self) -> Result<(), CalibrationError> {
        if self.num_slices() == 0 {
            return Err(CalibrationError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }
        Ok(())
    }

    /// Calibrates all loaded slices and returns the aggregated result.
    fn calibrate(&self) -> Result<Self::Result, CalibrationError>;
}

use super::error::CalibrationError;

/// Diagnostics from a single slice calibration.
#[derive(Debug, Clone, PartialEq)]
pub struct SliceCalibrationDiagnostics {
    /// Whether the calibration converged.
    pub converged: bool,
    /// Number of iterations performed.
    pub iterations: usize,
    /// Final residual sum of squares.
    pub final_residual_ss: f64,
    /// Root mean square error of the fit.
    pub rmse: f64,
    /// Number of quotes used for calibration.
    pub num_quotes: usize,
    /// Warnings generated during calibration (if any).
    pub warnings: Vec<String>,
}

impl SliceCalibrationDiagnostics {
    /// Creates new diagnostics from calibration results.
    pub fn new(
        converged: bool,
        iterations: usize,
        final_residual_ss: f64,
        num_quotes: usize,
    ) -> Self {
        let rmse = if num_quotes > 0 {
            (final_residual_ss / num_quotes as f64).sqrt()
        } else {
            0.0
        };

        Self {
            converged,
            iterations,
            final_residual_ss,
            rmse,
            num_quotes,
            warnings: Vec::new(),
        }
    }

    /// Adds a warning message.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Creates diagnostics with a warning.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.add_warning(warning);
        self
    }

    /// Returns true if the fit quality is acceptable.
    ///
    /// Uses RMSE threshold of 0.0001 (1 basis point for volatility).
    pub fn is_acceptable(&self) -> bool { self.converged && self.rmse < 0.0001 }

    /// Returns true if there are any warnings.
    pub fn has_warnings(&self) -> bool { !self.warnings.is_empty() }
}

impl Default for SliceCalibrationDiagnostics {
    fn default() -> Self {
        Self {
            converged: false,
            iterations: 0,
            final_residual_ss: f64::INFINITY,
            rmse: f64::INFINITY,
            num_quotes: 0,
            warnings: Vec::new(),
        }
    }
}

/// Result of slice calibration including parameters and diagnostics.
#[derive(Debug, Clone)]
pub struct SliceCalibrationResult<P> {
    /// Calibrated parameters.
    pub params: P,
    /// Calibration diagnostics.
    pub diagnostics: SliceCalibrationDiagnostics,
}

/// SABR model parameters (α, β, ρ, ν) for a single slice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SabrParams<T: Float> {
    /// Initial volatility (α > 0)
    pub alpha: T,
    /// CEV exponent (0 ≤ β ≤ 1)
    pub beta: T,
    /// Spot-vol correlation (-1 < ρ < 1)
    pub rho: T,
    /// Vol-of-vol (ν > 0)
    pub nu: T,
}

impl<T: Float> SabrParams<T> {
    /// Creates new SABR parameters.
    pub fn new(alpha: T, beta: T, rho: T, nu: T) -> Self {
        Self {
            alpha,
            beta,
            rho,
            nu,
        }
    }

    /// Creates parameters with typical defaults for rates (β=0.5, ρ=-0.3, ν=0.4).
    pub fn default_rates(alpha: T) -> Self {
        Self {
            alpha,
            beta: from_f64(0.5),
            rho: from_f64(-0.3),
            nu: from_f64(0.4),
        }
    }

    /// Creates parameters with typical defaults for FX (β=1.0, ρ=-0.2, ν=0.3).
    pub fn default_fx(alpha: T) -> Self {
        Self {
            alpha,
            beta: from_f64(1.0),
            rho: from_f64(-0.2),
            nu: from_f64(0.3),
        }
    }

    /// Validates the parameters are within acceptable bounds.
    pub fn validate(&self) -> Result<(), CalibrationError> {
        if self.alpha <= T::zero() {
            return Err(CalibrationError::BoundsViolation {
                param_name: "alpha".to_string(),
                value: self.alpha.to_f64().unwrap_or(0.0),
                lower: 0.0,
                upper: f64::INFINITY,
            });
        }
        if self.beta < T::zero() || self.beta > T::one() {
            return Err(CalibrationError::BoundsViolation {
                param_name: "beta".to_string(),
                value: self.beta.to_f64().unwrap_or(0.0),
                lower: 0.0,
                upper: 1.0,
            });
        }
        if self.rho <= -T::one() || self.rho >= T::one() {
            return Err(CalibrationError::BoundsViolation {
                param_name: "rho".to_string(),
                value: self.rho.to_f64().unwrap_or(0.0),
                lower: -1.0,
                upper: 1.0,
            });
        }
        if self.nu <= T::zero() {
            return Err(CalibrationError::BoundsViolation {
                param_name: "nu".to_string(),
                value: self.nu.to_f64().unwrap_or(0.0),
                lower: 0.0,
                upper: f64::INFINITY,
            });
        }
        Ok(())
    }
}

impl<T: Float> Default for SabrParams<T> {
    fn default() -> Self { Self::default_rates(from_f64(0.03)) }
}

/// Bounds for SABR parameter optimisation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SabrBounds<T: Float> {
    /// Bounds for alpha: (lower, upper), e.g. (0.001, 1.0)
    pub alpha_bounds: (T, T),
    /// Bounds for rho: (lower, upper), must be within (-1, 1)
    pub rho_bounds: (T, T),
    /// Bounds for nu: (lower, upper), e.g. (0.05, 3.0)
    pub nu_bounds: (T, T),
}

impl<T: Float> Default for SabrBounds<T> {
    fn default() -> Self {
        Self {
            alpha_bounds: (from_f64(0.001), from_f64(1.0)),
            rho_bounds: (from_f64(-0.95), from_f64(0.95)),
            nu_bounds: (from_f64(0.05), from_f64(3.0)),
        }
    }
}

impl<T: Float> SabrBounds<T> {
    /// Creates new SABR bounds.
    pub fn new(alpha_bounds: (T, T), rho_bounds: (T, T), nu_bounds: (T, T)) -> Self {
        Self {
            alpha_bounds,
            rho_bounds,
            nu_bounds,
        }
    }

    /// Creates bounds suitable for rates (swaptions).
    pub fn rates() -> Self {
        Self {
            alpha_bounds: (from_f64(0.001), from_f64(0.5)),
            rho_bounds: (from_f64(-0.9), from_f64(0.5)),
            nu_bounds: (from_f64(0.05), from_f64(2.0)),
        }
    }

    /// Creates bounds suitable for Normal SABR (β=0).
    pub fn normal() -> Self {
        Self {
            alpha_bounds: (from_f64(0.0001), from_f64(0.05)),
            rho_bounds: (from_f64(-0.9), from_f64(0.5)),
            nu_bounds: (from_f64(0.05), from_f64(2.0)),
        }
    }

    /// Creates bounds suitable for FX.
    pub fn fx() -> Self {
        Self {
            alpha_bounds: (from_f64(0.001), from_f64(1.0)),
            rho_bounds: (from_f64(-0.95), from_f64(0.95)),
            nu_bounds: (from_f64(0.05), from_f64(3.0)),
        }
    }

    /// Clamps (alpha, rho, nu) to be within bounds.
    pub fn clamp(&self, alpha: T, rho: T, nu: T) -> (T, T, T) {
        let clamped_alpha = alpha.max(self.alpha_bounds.0).min(self.alpha_bounds.1);
        let clamped_rho = rho.max(self.rho_bounds.0).min(self.rho_bounds.1);
        let clamped_nu = nu.max(self.nu_bounds.0).min(self.nu_bounds.1);
        (clamped_alpha, clamped_rho, clamped_nu)
    }

    /// Checks if parameters are within bounds.
    pub fn is_valid(&self, alpha: T, rho: T, nu: T) -> bool {
        alpha >= self.alpha_bounds.0
            && alpha <= self.alpha_bounds.1
            && rho >= self.rho_bounds.0
            && rho <= self.rho_bounds.1
            && nu >= self.nu_bounds.0
            && nu <= self.nu_bounds.1
    }
}

/// A single volatility quote for calibration.
#[derive(Debug, Clone, Copy)]
pub struct VolQuote<T: Float> {
    /// Strike (absolute or relative to forward)
    pub strike: T,
    /// Market-observed implied volatility
    pub volatility: T,
    /// Forward rate/price at this expiry
    pub forward: T,
    /// Time to expiry (in years)
    pub expiry: T,
}

impl<T: Float> VolQuote<T> {
    /// Creates a new volatility quote with expiry.
    pub fn new(strike: T, volatility: T, forward: T, expiry: T) -> Self {
        Self {
            strike,
            volatility,
            forward,
            expiry,
        }
    }

    /// Creates a new volatility quote without expiry (defaults to `T::one()`).
    pub fn new_without_expiry(strike: T, volatility: T, forward: T) -> Self {
        Self {
            strike,
            volatility,
            forward,
            expiry: T::one(),
        }
    }
}

/// Trait for calibrating a single parameter slice.
pub trait SliceCalibrator<T: Float> {
    /// The output parameter type for a single slice.
    type Params;

    /// Calibrates parameters from a set of volatility quotes.
    fn calibrate_slice(
        &self,
        quotes: &[VolQuote<T>],
        config: &SliceCalibrationConfig<T>,
    ) -> Result<SliceCalibrationResult<Self::Params>, CalibrationError>;
}

/// Configuration for slice calibration.
#[derive(Debug, Clone, Copy)]
pub struct SliceCalibrationConfig<T: Float> {
    /// Fixed beta parameter (if None, calibrate beta too)
    pub fixed_beta: Option<T>,
    /// Maximum iterations for optimiser
    pub max_iterations: usize,
    /// Convergence tolerance
    pub tolerance: T,
    /// Initial guess for alpha
    pub initial_alpha: T,
    /// Initial guess for rho (correlation)
    pub initial_rho: T,
    /// Initial guess for nu (vol-of-vol)
    pub initial_nu: T,
    /// Parameter bounds for optimisation
    pub bounds: SabrBounds<T>,
}

impl<T: Float> Default for SliceCalibrationConfig<T> {
    fn default() -> Self {
        Self {
            fixed_beta: Some(from_f64(0.5)),
            max_iterations: 100,
            tolerance: from_f64(1e-8),
            initial_alpha: from_f64(0.03),
            initial_rho: from_f64(-0.3),
            initial_nu: from_f64(0.4),
            bounds: SabrBounds::default(),
        }
    }
}

impl<T: Float> SliceCalibrationConfig<T> {
    /// Creates a configuration for rates (β = 0.5).
    pub fn rates() -> Self {
        Self {
            fixed_beta: Some(from_f64(0.5)),
            initial_rho: from_f64(-0.3),
            initial_nu: from_f64(0.4),
            bounds: SabrBounds::rates(),
            ..Self::default()
        }
    }

    /// Creates a configuration for Normal SABR (β=0).
    pub fn normal() -> Self {
        Self {
            fixed_beta: Some(from_f64(0.0)),
            initial_alpha: from_f64(0.005),
            initial_rho: from_f64(-0.3),
            initial_nu: from_f64(0.4),
            bounds: SabrBounds::normal(),
            ..Self::default()
        }
    }

    /// Creates a configuration for FX (β = 1.0).
    pub fn fx() -> Self {
        Self {
            fixed_beta: Some(from_f64(1.0)),
            initial_rho: from_f64(-0.2),
            initial_nu: from_f64(0.3),
            bounds: SabrBounds::fx(),
            ..Self::default()
        }
    }

    /// Sets custom parameter bounds.
    pub fn with_bounds(mut self, bounds: SabrBounds<T>) -> Self {
        self.bounds = bounds;
        self
    }

    /// Sets initial parameter guesses.
    pub fn with_initial_params(mut self, alpha: T, rho: T, nu: T) -> Self {
        self.initial_alpha = alpha;
        self.initial_rho = rho;
        self.initial_nu = nu;
        self
    }
}

/// SABR model slice calibrator using Levenberg-Marquardt optimisation.
#[derive(Debug, Clone, Default)]
pub struct SabrSliceCalibrator<T: Float> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: Float> SabrSliceCalibrator<T> {
    /// Creates a new SABR slice calibrator.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: Float> SliceCalibrator<T> for SabrSliceCalibrator<T> {
    type Params = SabrParams<T>;

    fn calibrate_slice(
        &self,
        quotes: &[VolQuote<T>],
        config: &SliceCalibrationConfig<T>,
    ) -> Result<SliceCalibrationResult<Self::Params>, CalibrationError> {
        if quotes.is_empty() {
            return Err(CalibrationError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }

        // Need at least 3 quotes for proper SABR calibration (alpha, rho, nu)
        if quotes.len() < 3 {
            return Err(CalibrationError::InsufficientData {
                required: 3,
                provided: quotes.len(),
            });
        }

        // Fixed beta from config
        let beta_t = config.fixed_beta.unwrap_or(from_f64(0.5));
        let beta = beta_t.to_f64().unwrap_or(0.5);

        // Find ATM quote for initial alpha guess
        let atm_quote = quotes
            .iter()
            .min_by(|a, b| {
                let diff_a = (a.strike - a.forward).abs();
                let diff_b = (b.strike - b.forward).abs();
                diff_a.partial_cmp(&diff_b).unwrap()
            })
            .unwrap();

        // Extract bounds
        let alpha_lo = config.bounds.alpha_bounds.0.to_f64().unwrap_or(0.001);
        let alpha_hi = config.bounds.alpha_bounds.1.to_f64().unwrap_or(1.0);

        // Initial alpha estimation:
        // - Normal SABR (β ≈ 0): σ_N(ATM) ≈ α, so α ≈ ATM normal vol
        // - General SABR (β > 0): σ_B(ATM) ≈ α / F^(1-β), so α ≈ σ_ATM × F^(1-β)
        let forward_f64 = atm_quote.forward.to_f64().unwrap_or(0.03);
        let atm_vol_f64 = atm_quote.volatility.to_f64().unwrap_or(0.2);
        let initial_alpha = if beta < 1e-6 {
            atm_vol_f64
        } else {
            atm_vol_f64 * forward_f64.powf(1.0 - beta)
        }
        .clamp(alpha_lo, alpha_hi);

        let initial_rho = config.initial_rho.to_f64().unwrap_or(-0.3);
        let initial_nu = config.initial_nu.to_f64().unwrap_or(0.4);
        let rho_lo = config.bounds.rho_bounds.0.to_f64().unwrap_or(-0.95);
        let rho_hi = config.bounds.rho_bounds.1.to_f64().unwrap_or(0.95);
        let nu_lo = config.bounds.nu_bounds.0.to_f64().unwrap_or(0.05);
        let nu_hi = config.bounds.nu_bounds.1.to_f64().unwrap_or(3.0);

        // Convert quotes to f64 vectors for optimisation
        let quote_data: Vec<(f64, f64, f64, f64)> = quotes
            .iter()
            .map(|q| {
                (
                    q.strike.to_f64().unwrap_or(0.03),
                    q.volatility.to_f64().unwrap_or(0.2),
                    q.forward.to_f64().unwrap_or(0.03),
                    q.expiry.to_f64().unwrap_or(1.0),
                )
            })
            .collect();

        // Build residual closure
        let residuals = |params: &[f64]| -> Vec<f64> {
            let alpha = params[0].clamp(alpha_lo, alpha_hi);
            let rho = params[1].clamp(rho_lo, rho_hi);
            let nu = params[2].clamp(nu_lo, nu_hi);

            quote_data
                .iter()
                .map(|&(strike, market_vol, forward, expiry)| {
                    let sabr_params = SabrImpliedVolParams {
                        forward,
                        alpha,
                        beta,
                        nu,
                        rho,
                        maturity: expiry,
                    };

                    let model_vol = sabr_implied_vol(&sabr_params, strike).unwrap_or(market_vol);

                    model_vol - market_vol
                })
                .collect()
        };

        // Configure LM solver
        let lm_config = LMConfig {
            tolerance: config.tolerance.to_f64().unwrap_or(1e-8),
            max_iterations: config.max_iterations,
            ..LMConfig::default()
        };

        let solver = LevenbergMarquardtSolver::new(lm_config);
        let initial_params = vec![initial_alpha, initial_rho, initial_nu];

        // Solve
        let result =
            solver
                .solve(residuals, initial_params)
                .map_err(|e| CalibrationError::SolverError {
                    message: e.to_string(),
                })?;

        // Extract and clamp final parameters
        let final_alpha = result.params[0].clamp(alpha_lo, alpha_hi);
        let final_rho = result.params[1].clamp(rho_lo, rho_hi);
        let final_nu = result.params[2].clamp(nu_lo, nu_hi);

        // Build diagnostics from LM result
        let mut diagnostics = SliceCalibrationDiagnostics::new(
            result.converged,
            result.iterations,
            result.residual_ss,
            quotes.len(),
        );

        // Add warning if not converged
        if !result.converged {
            diagnostics.add_warning(format!(
                "Calibration did not converge after {} iterations (residual: {:.6e})",
                result.iterations, result.residual_ss
            ));
        }

        // Build final SabrParams
        let params = SabrParams::new(
            from_f64(final_alpha),
            beta_t,
            from_f64(final_rho),
            from_f64(final_nu),
        );

        params.validate()?;

        Ok(SliceCalibrationResult {
            params,
            diagnostics,
        })
    }
}

/// Wrapper for Float that implements Ord (for use in BTreeMap keys).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderedFloat<T: Float>(pub T);

impl<T: Float> Eq for OrderedFloat<T> {}

impl<T: Float> PartialOrd for OrderedFloat<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl<T: Float> Ord for OrderedFloat<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}

/// A volatility slice quoted in ATM / Risk-Reversal / Butterfly format.
#[derive(Debug, Clone, Copy)]
pub struct DeltaVolSlice<T: Float> {
    /// ATM (50-delta) volatility
    pub atm: T,
    /// 25-delta Risk Reversal (call vol - put vol)
    pub rr_25d: Option<T>,
    /// 25-delta Butterfly (average of call+put minus ATM)
    pub bf_25d: Option<T>,
    /// 10-delta Risk Reversal
    pub rr_10d: Option<T>,
    /// 10-delta Butterfly
    pub bf_10d: Option<T>,
    /// Time to expiry (in years)
    pub expiry: T,
    /// Forward price
    pub forward: T,
}

/// Volatility at a specific delta point.
#[derive(Debug, Clone, Copy)]
pub struct DeltaVol<T: Float> {
    /// Delta value (as decimal, e.g., 0.25 for 25-delta)
    pub delta: T,
    /// Is this a call option?
    pub is_call: bool,
    /// Implied volatility
    pub volatility: T,
}

impl<T: Float> DeltaVolSlice<T> {
    /// Creates a new DeltaVolSlice with ATM only.
    pub fn new_atm_only(atm: T, expiry: T, forward: T) -> Self {
        Self {
            atm,
            rr_25d: None,
            bf_25d: None,
            rr_10d: None,
            bf_10d: None,
            expiry,
            forward,
        }
    }

    /// Creates a DeltaVolSlice with 25-delta quotes.
    pub fn new_with_25d(atm: T, rr_25d: T, bf_25d: T, expiry: T, forward: T) -> Self {
        Self {
            atm,
            rr_25d: Some(rr_25d),
            bf_25d: Some(bf_25d),
            rr_10d: None,
            bf_10d: None,
            expiry,
            forward,
        }
    }

    /// Creates a DeltaVolSlice with both 25-delta and 10-delta quotes.
    pub fn new_with_10d_25d(
        atm: T,
        rr_25d: T,
        bf_25d: T,
        rr_10d: T,
        bf_10d: T,
        expiry: T,
        forward: T,
    ) -> Self {
        Self {
            atm,
            rr_25d: Some(rr_25d),
            bf_25d: Some(bf_25d),
            rr_10d: Some(rr_10d),
            bf_10d: Some(bf_10d),
            expiry,
            forward,
        }
    }

    /// Converts RR/BF quotes to individual delta volatilities.
    ///
    /// Formulas: `σ_call = ATM + BF + RR/2`, `σ_put = ATM + BF - RR/2`.
    pub fn to_delta_vols(&self) -> Vec<DeltaVol<T>> {
        let half: T = from_f64(0.5);
        let mut result = Vec::new();

        // ATM (50-delta, both call and put have same vol)
        result.push(DeltaVol {
            delta: from_f64(0.5),
            is_call: true,
            volatility: self.atm,
        });

        // 25-delta
        if let (Some(rr), Some(bf)) = (self.rr_25d, self.bf_25d) {
            // σ_25D_call = ATM + BF + RR/2
            let vol_25d_call = self.atm + bf + rr * half;
            // σ_25D_put = ATM + BF - RR/2
            let vol_25d_put = self.atm + bf - rr * half;

            result.push(DeltaVol {
                delta: from_f64(0.25),
                is_call: true,
                volatility: vol_25d_call,
            });
            result.push(DeltaVol {
                delta: from_f64(0.25),
                is_call: false,
                volatility: vol_25d_put,
            });
        }

        // 10-delta
        if let (Some(rr), Some(bf)) = (self.rr_10d, self.bf_10d) {
            let vol_10d_call = self.atm + bf + rr * half;
            let vol_10d_put = self.atm + bf - rr * half;

            result.push(DeltaVol {
                delta: from_f64(0.1),
                is_call: true,
                volatility: vol_10d_call,
            });
            result.push(DeltaVol {
                delta: from_f64(0.1),
                is_call: false,
                volatility: vol_10d_put,
            });
        }

        result
    }

    /// Converts to strike-based [`VolQuote`]s using the given delta-to-strike closure.
    pub fn to_strike_vol_quotes<F>(&self, delta_to_strike: F) -> Vec<VolQuote<T>>
    where
        F: Fn(T, bool, T) -> T,
    {
        let delta_vols = self.to_delta_vols();

        delta_vols
            .into_iter()
            .map(|dv| {
                let strike = delta_to_strike(dv.delta, dv.is_call, dv.volatility);
                VolQuote::new(strike, dv.volatility, self.forward, self.expiry)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sabr_params_new() {
        let params: SabrParams<f64> = SabrParams::new(0.03, 0.5, -0.3, 0.4);
        assert!((params.alpha - 0.03).abs() < 1e-10);
        assert!((params.beta - 0.5).abs() < 1e-10);
        assert!((params.rho - (-0.3)).abs() < 1e-10);
        assert!((params.nu - 0.4).abs() < 1e-10);
    }

    #[test]
    fn test_sabr_params_validate() {
        let valid = SabrParams::new(0.03, 0.5, -0.3, 0.4);
        assert!(valid.validate().is_ok());

        let invalid_alpha: SabrParams<f64> = SabrParams::new(-0.03, 0.5, -0.3, 0.4);
        assert!(invalid_alpha.validate().is_err());

        let invalid_rho: SabrParams<f64> = SabrParams::new(0.03, 0.5, -1.5, 0.4);
        assert!(invalid_rho.validate().is_err());
    }

    #[test]
    fn test_sabr_params_defaults() {
        let rates: SabrParams<f64> = SabrParams::default_rates(0.03);
        assert!((rates.beta - 0.5).abs() < 1e-10);

        let fx: SabrParams<f64> = SabrParams::default_fx(0.1);
        assert!((fx.beta - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_vol_quote() {
        // Test new 4-parameter constructor
        let quote: VolQuote<f64> = VolQuote::new(0.03, 0.2, 0.03, 1.0);
        assert!((quote.strike - 0.03).abs() < 1e-10);
        assert!((quote.volatility - 0.2).abs() < 1e-10);
        assert!((quote.forward - 0.03).abs() < 1e-10);
        assert!((quote.expiry - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_vol_quote_without_expiry() {
        // Test backwards-compatible constructor
        let quote: VolQuote<f64> = VolQuote::new_without_expiry(0.03, 0.2, 0.03);
        assert!((quote.strike - 0.03).abs() < 1e-10);
        assert!((quote.volatility - 0.2).abs() < 1e-10);
        assert!((quote.forward - 0.03).abs() < 1e-10);
        assert!((quote.expiry - 1.0).abs() < 1e-10); // Default expiry =
                                                     // T::one()
    }

    #[test]
    fn test_slice_calibration_config_defaults() {
        let rates: SliceCalibrationConfig<f64> = SliceCalibrationConfig::rates();
        assert!((rates.fixed_beta.unwrap() - 0.5).abs() < 1e-10);
        assert!((rates.initial_rho - (-0.3)).abs() < 1e-10);
        assert!((rates.initial_nu - 0.4).abs() < 1e-10);

        let fx: SliceCalibrationConfig<f64> = SliceCalibrationConfig::fx();
        assert!((fx.fixed_beta.unwrap() - 1.0).abs() < 1e-10);
        assert!((fx.initial_rho - (-0.2)).abs() < 1e-10);
        assert!((fx.initial_nu - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_sabr_bounds_default() {
        let bounds: SabrBounds<f64> = SabrBounds::default();
        assert!((bounds.alpha_bounds.0 - 0.001).abs() < 1e-10);
        assert!((bounds.alpha_bounds.1 - 1.0).abs() < 1e-10);
        assert!((bounds.rho_bounds.0 - (-0.95)).abs() < 1e-10);
        assert!((bounds.rho_bounds.1 - 0.95).abs() < 1e-10);
        assert!((bounds.nu_bounds.0 - 0.05).abs() < 1e-10);
        assert!((bounds.nu_bounds.1 - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_sabr_bounds_clamp() {
        let bounds: SabrBounds<f64> = SabrBounds::default();

        // Values within bounds stay unchanged
        let (a, r, n) = bounds.clamp(0.1, -0.3, 0.5);
        assert!((a - 0.1).abs() < 1e-10);
        assert!((r - (-0.3)).abs() < 1e-10);
        assert!((n - 0.5).abs() < 1e-10);

        // Values outside bounds get clamped
        let (a, r, n) = bounds.clamp(0.0001, -0.99, 5.0);
        assert!((a - 0.001).abs() < 1e-10); // Clamped to lower bound
        assert!((r - (-0.95)).abs() < 1e-10); // Clamped to lower bound
        assert!((n - 3.0).abs() < 1e-10); // Clamped to upper bound
    }

    #[test]
    fn test_sabr_bounds_is_valid() {
        let bounds: SabrBounds<f64> = SabrBounds::default();

        assert!(bounds.is_valid(0.1, -0.3, 0.5));
        assert!(!bounds.is_valid(0.0001, -0.3, 0.5)); // alpha too low
        assert!(!bounds.is_valid(0.1, -0.99, 0.5)); // rho too low
        assert!(!bounds.is_valid(0.1, -0.3, 5.0)); // nu too high
    }

    #[test]
    fn test_sabr_bounds_rates() {
        let bounds: SabrBounds<f64> = SabrBounds::rates();
        assert!((bounds.alpha_bounds.1 - 0.5).abs() < 1e-10);
        assert!((bounds.rho_bounds.1 - 0.5).abs() < 1e-10);
        assert!((bounds.nu_bounds.1 - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_slice_config_with_bounds() {
        let custom_bounds = SabrBounds::new((0.01, 0.5), (-0.8, 0.8), (0.1, 1.5));
        let config: SliceCalibrationConfig<f64> =
            SliceCalibrationConfig::rates().with_bounds(custom_bounds);

        assert!((config.bounds.alpha_bounds.0 - 0.01).abs() < 1e-10);
        assert!((config.bounds.rho_bounds.1 - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_slice_config_with_initial_params() {
        let config: SliceCalibrationConfig<f64> =
            SliceCalibrationConfig::rates().with_initial_params(0.05, -0.5, 0.6);

        assert!((config.initial_alpha - 0.05).abs() < 1e-10);
        assert!((config.initial_rho - (-0.5)).abs() < 1e-10);
        assert!((config.initial_nu - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_delta_vol_slice_atm_only() {
        let slice: DeltaVolSlice<f64> = DeltaVolSlice::new_atm_only(0.10, 1.0, 1.10);

        let vols = slice.to_delta_vols();
        assert_eq!(vols.len(), 1);
        assert!((vols[0].delta - 0.5).abs() < 1e-10);
        assert!((vols[0].volatility - 0.10).abs() < 1e-10);
        assert!(vols[0].is_call);
    }

    #[test]
    fn test_delta_vol_slice_with_25d() {
        // ATM = 10%, RR_25D = 1% (call - put), BF_25D = 0.5%
        let slice: DeltaVolSlice<f64> = DeltaVolSlice::new_with_25d(
            0.10,  // atm
            0.01,  // rr_25d (call vol - put vol)
            0.005, // bf_25d
            1.0,   // expiry
            1.10,  // forward
        );

        let vols = slice.to_delta_vols();
        assert_eq!(vols.len(), 3);

        // ATM
        assert!((vols[0].delta - 0.5).abs() < 1e-10);
        assert!((vols[0].volatility - 0.10).abs() < 1e-10);

        // 25D Call: ATM + BF + RR/2 = 0.10 + 0.005 + 0.005 = 0.11
        let call_25d = vols
            .iter()
            .find(|v| v.is_call && (v.delta - 0.25).abs() < 1e-10)
            .unwrap();
        assert!((call_25d.volatility - 0.11).abs() < 1e-10);

        // 25D Put: ATM + BF - RR/2 = 0.10 + 0.005 - 0.005 = 0.10
        let put_25d = vols
            .iter()
            .find(|v| !v.is_call && (v.delta - 0.25).abs() < 1e-10)
            .unwrap();
        assert!((put_25d.volatility - 0.10).abs() < 1e-10);
    }

    #[test]
    fn test_delta_vol_slice_with_10d_25d() {
        let slice: DeltaVolSlice<f64> = DeltaVolSlice::new_with_10d_25d(
            0.10,  // atm
            0.01,  // rr_25d
            0.005, // bf_25d
            0.02,  // rr_10d
            0.01,  // bf_10d
            1.0,   // expiry
            1.10,  // forward
        );

        let vols = slice.to_delta_vols();
        assert_eq!(vols.len(), 5); // ATM + 2×25D + 2×10D

        // 10D Call: ATM + BF_10D + RR_10D/2 = 0.10 + 0.01 + 0.01 = 0.12
        let call_10d = vols
            .iter()
            .find(|v| v.is_call && (v.delta - 0.1).abs() < 1e-10)
            .unwrap();
        assert!((call_10d.volatility - 0.12).abs() < 1e-10);

        // 10D Put: ATM + BF_10D - RR_10D/2 = 0.10 + 0.01 - 0.01 = 0.10
        let put_10d = vols
            .iter()
            .find(|v| !v.is_call && (v.delta - 0.1).abs() < 1e-10)
            .unwrap();
        assert!((put_10d.volatility - 0.10).abs() < 1e-10);
    }

    #[test]
    fn test_delta_vol_slice_to_strike_quotes() {
        let slice: DeltaVolSlice<f64> = DeltaVolSlice::new_with_25d(
            0.10,  // atm
            0.01,  // rr_25d
            0.005, // bf_25d
            1.0,   // expiry
            1.10,  // forward
        );

        // Simple delta-to-strike function for testing (returns forward + delta offset)
        let delta_to_strike = |delta: f64, is_call: bool, _vol: f64| -> f64 {
            let offset = if is_call {
                0.1 * (0.5 - delta)
            } else {
                -0.1 * (0.5 - delta)
            };
            1.10 + offset
        };

        let quotes = slice.to_strike_vol_quotes(delta_to_strike);
        assert_eq!(quotes.len(), 3);

        // Verify all quotes have correct expiry and forward
        for quote in &quotes {
            assert!((quote.expiry - 1.0).abs() < 1e-10);
            assert!((quote.forward - 1.10).abs() < 1e-10);
        }
    }

    #[test]
    fn test_delta_vol_slice_rr_bf_round_trip() {
        // Given call and put vols, compute RR/BF, then verify reverse
        let atm = 0.10_f64;
        let vol_25d_call = 0.115;
        let vol_25d_put = 0.095;

        // RR = call - put
        let rr_25d = vol_25d_call - vol_25d_put; // 0.02
                                                 // BF = (call + put)/2 - atm
        let bf_25d = (vol_25d_call + vol_25d_put) / 2.0 - atm; // 0.005

        let slice: DeltaVolSlice<f64> = DeltaVolSlice::new_with_25d(atm, rr_25d, bf_25d, 1.0, 1.10);
        let vols = slice.to_delta_vols();

        let call_25d = vols
            .iter()
            .find(|v| v.is_call && (v.delta - 0.25).abs() < 1e-10)
            .unwrap();
        let put_25d = vols
            .iter()
            .find(|v| !v.is_call && (v.delta - 0.25).abs() < 1e-10)
            .unwrap();

        assert!((call_25d.volatility - vol_25d_call).abs() < 1e-10);
        assert!((put_25d.volatility - vol_25d_put).abs() < 1e-10);
    }

    #[test]
    fn test_diagnostics_new() {
        let diag = SliceCalibrationDiagnostics::new(true, 10, 1e-12, 5);

        assert!(diag.converged);
        assert_eq!(diag.iterations, 10);
        assert!((diag.final_residual_ss - 1e-12).abs() < 1e-15);
        assert_eq!(diag.num_quotes, 5);
        // RMSE = sqrt(1e-12 / 5) ≈ 4.47e-7
        assert!(diag.rmse > 0.0);
        assert!(diag.warnings.is_empty());
    }

    #[test]
    fn test_diagnostics_default() {
        let diag = SliceCalibrationDiagnostics::default();

        assert!(!diag.converged);
        assert_eq!(diag.iterations, 0);
        assert!(diag.final_residual_ss.is_infinite());
        assert!(diag.rmse.is_infinite());
        assert_eq!(diag.num_quotes, 0);
    }

    #[test]
    fn test_diagnostics_with_warning() {
        let diag = SliceCalibrationDiagnostics::new(true, 10, 1e-8, 3).with_warning("Test warning");

        assert!(diag.has_warnings());
        assert_eq!(diag.warnings.len(), 1);
        assert_eq!(diag.warnings[0], "Test warning");
    }

    #[test]
    fn test_diagnostics_is_acceptable() {
        // Good calibration: converged with very small RMSE
        let good_diag = SliceCalibrationDiagnostics::new(true, 10, 1e-12, 5);
        assert!(good_diag.is_acceptable());

        // Not converged
        let not_converged = SliceCalibrationDiagnostics::new(false, 100, 1e-12, 5);
        assert!(!not_converged.is_acceptable());

        // High RMSE (0.01 = 100 basis points total error)
        let high_rmse = SliceCalibrationDiagnostics::new(true, 10, 0.01, 1);
        assert!(!high_rmse.is_acceptable()); // RMSE = 0.1 > 0.0001
    }

    #[test]
    fn test_diagnostics_add_warning() {
        let mut diag = SliceCalibrationDiagnostics::new(true, 10, 1e-8, 3);
        assert!(!diag.has_warnings());

        diag.add_warning("Warning 1");
        diag.add_warning("Warning 2");

        assert!(diag.has_warnings());
        assert_eq!(diag.warnings.len(), 2);
    }

    #[test]
    fn test_slice_calibration_result() {
        let params: SabrParams<f64> = SabrParams::new(0.1, 1.0, -0.2, 0.3);
        let diagnostics = SliceCalibrationDiagnostics::new(true, 15, 1e-10, 5);

        let result = SliceCalibrationResult {
            params,
            diagnostics,
        };

        assert!((result.params.alpha - 0.1).abs() < 1e-10);
        assert!(result.diagnostics.converged);
        assert_eq!(result.diagnostics.iterations, 15);
    }

    #[test]
    fn test_sabr_calibrator_returns_diagnostics() {
        let calibrator: SabrSliceCalibrator<f64> = SabrSliceCalibrator::new();
        let config = SliceCalibrationConfig::fx();

        let quotes = vec![
            VolQuote::new(1.10, 0.08, 1.10, 0.25),
            VolQuote::new(1.05, 0.085, 1.10, 0.25),
            VolQuote::new(1.15, 0.082, 1.10, 0.25),
        ];

        let result = calibrator.calibrate_slice(&quotes, &config).unwrap();

        // Should have valid params
        assert!(result.params.alpha > 0.0);
        assert!(result.params.validate().is_ok());

        // Should have diagnostics
        assert!(result.diagnostics.converged);
        assert!(result.diagnostics.iterations > 0);
        assert_eq!(result.diagnostics.num_quotes, 3);
        assert!(result.diagnostics.rmse < 0.01); // Good fit
    }

    #[test]
    fn test_atm_alpha_estimation() {
        // Test that alpha is correctly estimated from ATM volatility
        // Formula: α ≈ σ_ATM × F^(1-β)
        let calibrator: SabrSliceCalibrator<f64> = SabrSliceCalibrator::new();

        // FX config with β = 1.0, so α ≈ σ_ATM × F^0 = σ_ATM
        let fx_config = SliceCalibrationConfig::fx();
        let atm_vol = 0.10; // 10%
        let forward = 1.10;

        // Create ATM quote and some smile quotes
        let quotes = vec![
            VolQuote::new(forward, atm_vol, forward, 1.0), // ATM
            VolQuote::new(forward * 0.95, 0.105, forward, 1.0), // OTM put
            VolQuote::new(forward * 1.05, 0.102, forward, 1.0), // OTM call
        ];

        let result = calibrator.calibrate_slice(&quotes, &fx_config).unwrap();

        // For β = 1.0, calibrated alpha should be close to ATM vol
        // Allow 50% deviation due to smile fitting
        assert!(
            (result.params.alpha - atm_vol).abs() < atm_vol * 0.5,
            "Alpha {} should be close to ATM vol {}",
            result.params.alpha,
            atm_vol
        );
    }

    #[test]
    fn test_rates_alpha_estimation() {
        // Test alpha estimation for rates (β = 0.5)
        // Formula: α ≈ σ_ATM × F^(1-β) = σ_ATM × F^0.5
        let calibrator: SabrSliceCalibrator<f64> = SabrSliceCalibrator::new();
        let rates_config = SliceCalibrationConfig::rates();

        let atm_vol = 0.20; // 20%
        let forward = 0.03; // 3% swap rate
        let _expected_alpha = atm_vol * forward.powf(0.5);

        let quotes = vec![
            VolQuote::new(forward, atm_vol, forward, 5.0), // ATM
            VolQuote::new(forward - 0.01, 0.22, forward, 5.0), // OTM put
            VolQuote::new(forward + 0.01, 0.21, forward, 5.0), // OTM call
        ];

        let result = calibrator.calibrate_slice(&quotes, &rates_config).unwrap();

        // Allow reasonable deviation for rates
        assert!(result.params.alpha > 0.0, "Alpha should be positive");
        assert!(result.params.alpha < 1.0, "Alpha should be less than 1.0");
    }

    #[test]
    fn test_calibration_accuracy_within_50bp() {
        use pricer_core::math::formulas::sabr::{sabr_implied_vol, SabrImpliedVolParams};

        let calibrator: SabrSliceCalibrator<f64> = SabrSliceCalibrator::new();
        let config = SliceCalibrationConfig::fx();

        // Realistic FX smile data
        let forward = 1.10;
        let expiry = 0.25;
        let quotes = vec![
            VolQuote::new(1.05, 0.095, forward, expiry), // 25D put
            VolQuote::new(1.10, 0.085, forward, expiry), // ATM
            VolQuote::new(1.15, 0.092, forward, expiry), // 25D call
        ];

        let result = calibrator.calibrate_slice(&quotes, &config).unwrap();
        let params = result.params;

        // Check each quote's model vol vs market vol
        for quote in &quotes {
            let sabr_params = SabrImpliedVolParams {
                forward: quote.forward,
                alpha: params.alpha,
                beta: params.beta,
                nu: params.nu,
                rho: params.rho,
                maturity: quote.expiry,
            };

            let model_vol = sabr_implied_vol(&sabr_params, quote.strike).unwrap();
            let market_vol = quote.volatility;
            let error_bp = (model_vol - market_vol).abs() * 10000.0;

            assert!(
                error_bp < 50.0,
                "Model vol {} vs market vol {}: error {}bp exceeds 50bp limit",
                model_vol,
                market_vol,
                error_bp
            );
        }
    }

    #[test]
    fn test_rates_calibration_accuracy_within_50bp() {
        use pricer_core::math::formulas::sabr::{sabr_implied_vol, SabrImpliedVolParams};

        let calibrator: SabrSliceCalibrator<f64> = SabrSliceCalibrator::new();
        let config = SliceCalibrationConfig::rates();

        // Realistic swaption smile data (1Y x 5Y)
        let forward = 0.03; // 3% swap rate
        let expiry = 1.0;
        let quotes = vec![
            VolQuote::new(0.02, 0.25, forward, expiry), // 100bp OTM put
            VolQuote::new(0.03, 0.20, forward, expiry), // ATM
            VolQuote::new(0.04, 0.22, forward, expiry), // 100bp OTM call
        ];

        let result = calibrator.calibrate_slice(&quotes, &config).unwrap();
        let params = result.params;

        // Check each quote's model vol vs market vol
        for quote in &quotes {
            let sabr_params = SabrImpliedVolParams {
                forward: quote.forward,
                alpha: params.alpha,
                beta: params.beta,
                nu: params.nu,
                rho: params.rho,
                maturity: quote.expiry,
            };

            let model_vol = sabr_implied_vol(&sabr_params, quote.strike).unwrap();
            let market_vol = quote.volatility;
            let error_bp = (model_vol - market_vol).abs() * 10000.0;

            assert!(
                error_bp < 50.0,
                "Model vol {} vs market vol {}: error {}bp exceeds 50bp limit",
                model_vol,
                market_vol,
                error_bp
            );
        }
    }

    #[test]
    fn test_empty_quotes_returns_insufficient_data() {
        let calibrator: SabrSliceCalibrator<f64> = SabrSliceCalibrator::new();
        let config = SliceCalibrationConfig::fx();

        let quotes: Vec<VolQuote<f64>> = vec![];
        let result = calibrator.calibrate_slice(&quotes, &config);

        assert!(result.is_err());
        match result.unwrap_err() {
            CalibrationError::InsufficientData { required, provided } => {
                assert_eq!(required, 1);
                assert_eq!(provided, 0);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_insufficient_quotes_for_sabr() {
        let calibrator: SabrSliceCalibrator<f64> = SabrSliceCalibrator::new();
        let config = SliceCalibrationConfig::fx();

        // Only 2 quotes (need at least 3 for SABR)
        let quotes = vec![
            VolQuote::new(1.10, 0.08, 1.10, 0.25),
            VolQuote::new(1.05, 0.085, 1.10, 0.25),
        ];
        let result = calibrator.calibrate_slice(&quotes, &config);

        assert!(result.is_err());
        match result.unwrap_err() {
            CalibrationError::InsufficientData { required, provided } => {
                assert_eq!(required, 3);
                assert_eq!(provided, 2);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_calibrated_params_satisfy_constraints() {
        let calibrator: SabrSliceCalibrator<f64> = SabrSliceCalibrator::new();
        let config = SliceCalibrationConfig::fx();

        let quotes = vec![
            VolQuote::new(1.05, 0.095, 1.10, 0.25),
            VolQuote::new(1.10, 0.085, 1.10, 0.25),
            VolQuote::new(1.15, 0.092, 1.10, 0.25),
        ];

        let result = calibrator.calibrate_slice(&quotes, &config).unwrap();
        let params = result.params;

        // α > 0
        assert!(params.alpha > 0.0, "alpha should be positive");
        // 0 ≤ β ≤ 1
        assert!(
            params.beta >= 0.0 && params.beta <= 1.0,
            "beta should be in [0, 1]"
        );
        // -1 < ρ < 1
        assert!(
            params.rho > -1.0 && params.rho < 1.0,
            "rho should be in (-1, 1)"
        );
        // ν > 0
        assert!(params.nu > 0.0, "nu should be positive");

        // Validate method should pass
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_default_max_iterations_is_100() {
        let config: SliceCalibrationConfig<f64> = SliceCalibrationConfig::default();
        assert_eq!(config.max_iterations, 100);
    }

    #[test]
    fn test_default_tolerance_is_1e8() {
        let config: SliceCalibrationConfig<f64> = SliceCalibrationConfig::default();
        assert!((config.tolerance - 1e-8).abs() < 1e-15);
    }

    #[test]
    fn test_convergence_within_100_iterations() {
        let calibrator: SabrSliceCalibrator<f64> = SabrSliceCalibrator::new();
        let config = SliceCalibrationConfig::default();

        // Typical swaption smile data
        let quotes = vec![
            VolQuote::new(0.02, 0.22, 0.03, 1.0),
            VolQuote::new(0.03, 0.20, 0.03, 1.0),
            VolQuote::new(0.04, 0.21, 0.03, 1.0),
        ];

        let result = calibrator.calibrate_slice(&quotes, &config).unwrap();

        assert!(
            result.diagnostics.iterations <= 100,
            "Calibration took {} iterations, exceeds 100",
            result.diagnostics.iterations
        );
        assert!(result.diagnostics.converged);
    }

    #[test]
    fn test_typical_swaption_converges_within_50_iterations() {
        let calibrator: SabrSliceCalibrator<f64> = SabrSliceCalibrator::new();
        let config = SliceCalibrationConfig::rates();

        // Typical 5Y x 5Y swaption smile
        let forward = 0.025; // 2.5% swap rate
        let expiry = 5.0;
        let quotes = vec![
            VolQuote::new(0.015, 0.35, forward, expiry), // 100bp OTM put
            VolQuote::new(0.020, 0.30, forward, expiry), // 50bp OTM put
            VolQuote::new(0.025, 0.25, forward, expiry), // ATM
            VolQuote::new(0.030, 0.27, forward, expiry), // 50bp OTM call
            VolQuote::new(0.035, 0.30, forward, expiry), // 100bp OTM call
        ];

        let result = calibrator.calibrate_slice(&quotes, &config).unwrap();

        assert!(
            result.diagnostics.iterations <= 50,
            "Typical swaption calibration took {} iterations, exceeds 50",
            result.diagnostics.iterations
        );
        assert!(result.diagnostics.converged);
    }

    #[test]
    fn test_calibration_reproducibility() {
        let calibrator: SabrSliceCalibrator<f64> = SabrSliceCalibrator::new();
        let config = SliceCalibrationConfig::fx();

        let quotes = vec![
            VolQuote::new(1.05, 0.095, 1.10, 0.25),
            VolQuote::new(1.10, 0.085, 1.10, 0.25),
            VolQuote::new(1.15, 0.092, 1.10, 0.25),
        ];

        // Run calibration multiple times
        let result1 = calibrator.calibrate_slice(&quotes, &config).unwrap();
        let result2 = calibrator.calibrate_slice(&quotes, &config).unwrap();
        let result3 = calibrator.calibrate_slice(&quotes, &config).unwrap();

        // Results should be identical
        assert!(
            (result1.params.alpha - result2.params.alpha).abs() < 1e-12,
            "Alpha not reproducible: {} vs {}",
            result1.params.alpha,
            result2.params.alpha
        );
        assert!(
            (result1.params.rho - result2.params.rho).abs() < 1e-12,
            "Rho not reproducible: {} vs {}",
            result1.params.rho,
            result2.params.rho
        );
        assert!(
            (result1.params.nu - result2.params.nu).abs() < 1e-12,
            "Nu not reproducible: {} vs {}",
            result1.params.nu,
            result2.params.nu
        );

        // Also check third run
        assert!(
            (result1.params.alpha - result3.params.alpha).abs() < 1e-12,
            "Alpha not reproducible across 3 runs"
        );
    }

    #[test]
    fn test_volcube_calibration_reproducibility() {
        use super::cube::VolCubeBuilder;

        let mut builder1: VolCubeBuilder<f64> = VolCubeBuilder::new();
        let mut builder2: VolCubeBuilder<f64> = VolCubeBuilder::new();

        // Same quotes for both builders
        for builder in [&mut builder1, &mut builder2] {
            builder.add_quote(1.0, 5.0, 0.03, 0.2, 0.03);
            builder.add_quote(1.0, 5.0, 0.02, 0.22, 0.03);
            builder.add_quote(1.0, 5.0, 0.04, 0.21, 0.03);
        }

        let result1 = builder1.calibrate().unwrap();
        let result2 = builder2.calibrate().unwrap();

        let params1 = result1.get(1.0, 5.0).unwrap();
        let params2 = result2.get(1.0, 5.0).unwrap();

        assert!(
            (params1.alpha - params2.alpha).abs() < 1e-12,
            "VolCube calibration not reproducible"
        );
    }

    #[test]
    fn test_volcube_multiple_slice_calibration() {
        use super::cube::VolCubeBuilder;

        let mut builder: VolCubeBuilder<f64> = VolCubeBuilder::new();

        // 1Y x 5Y slice
        builder.add_quote(1.0, 5.0, 0.03, 0.20, 0.03);
        builder.add_quote(1.0, 5.0, 0.02, 0.22, 0.03);
        builder.add_quote(1.0, 5.0, 0.04, 0.21, 0.03);

        // 5Y x 5Y slice
        builder.add_quote(5.0, 5.0, 0.03, 0.18, 0.03);
        builder.add_quote(5.0, 5.0, 0.02, 0.20, 0.03);
        builder.add_quote(5.0, 5.0, 0.04, 0.19, 0.03);

        // 1Y x 10Y slice
        builder.add_quote(1.0, 10.0, 0.03, 0.19, 0.03);
        builder.add_quote(1.0, 10.0, 0.02, 0.21, 0.03);
        builder.add_quote(1.0, 10.0, 0.04, 0.20, 0.03);

        let result = builder.calibrate().unwrap();

        // Verify all slices were calibrated
        assert_eq!(result.num_slices(), 3);
        assert!(result.get(1.0, 5.0).is_some());
        assert!(result.get(5.0, 5.0).is_some());
        assert!(result.get(1.0, 10.0).is_some());

        // Verify all slices converged
        assert!(result.all_converged());

        // Verify each slice has valid parameters
        for ((exp, ten), params) in &result.params {
            assert!(
                params.validate().is_ok(),
                "Params at ({}, {}) invalid",
                exp.0,
                ten.0
            );
        }
    }
}
