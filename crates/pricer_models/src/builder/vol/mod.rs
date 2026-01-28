//! Volatility surface and cube calibration module.
//!
//! This module provides slice-wise calibration for volatility surfaces and cubes:
//!
//! - **FX volatility surface** ([`surface`]): 2D (expiry × strike)
//! - **Swaption volatility cube** ([`cube`]): 3D (expiry × tenor × strike)
//!
//! ## Calibration Pattern
//!
//! Unlike global curve calibration, vol surfaces use **slice-wise** calibration:
//!
//! 1. Calibrate each expiry/tenor slice independently
//! 2. Aggregate calibrated slices into a complete surface/cube
//!
//! This approach is efficient because slices are independent.

mod surface;
mod cube;

use num_traits::Float;
use std::cmp::Ordering;

use pricer_core::math::numeric::from_f64;

use super::error::CalibrationError;

// =============================================================================
// Re-exports
// =============================================================================

pub use surface::{FxVolBuilder, FxVolResult};
pub use cube::{VolCubeBuilder, VolCubeResult};

// =============================================================================
// SABR Parameters
// =============================================================================

/// SABR model parameters for a single slice.
///
/// The SABR model is defined by:
/// - `alpha` (α): Initial volatility level
/// - `beta` (β): CEV exponent (typically fixed, e.g., 0.5 for normal-like, 1.0 for log-normal)
/// - `rho` (ρ): Correlation between spot and volatility (-1 < ρ < 1)
/// - `nu` (ν): Volatility of volatility
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
        Self { alpha, beta, rho, nu }
    }

    /// Creates parameters with typical defaults for rates.
    ///
    /// Uses β = 0.5 (normal-like), ρ = -0.3, ν = 0.4.
    pub fn default_rates(alpha: T) -> Self {
        Self {
            alpha,
            beta: from_f64(0.5),
            rho: from_f64(-0.3),
            nu: from_f64(0.4),
        }
    }

    /// Creates parameters with typical defaults for FX.
    ///
    /// Uses β = 1.0 (log-normal), ρ = -0.2, ν = 0.3.
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
    fn default() -> Self {
        Self::default_rates(from_f64(0.03))
    }
}

// =============================================================================
// Volatility Quote
// =============================================================================

/// A single volatility quote for calibration.
#[derive(Debug, Clone, Copy)]
pub struct VolQuote<T: Float> {
    /// Strike (absolute or relative to forward)
    pub strike: T,
    /// Market-observed implied volatility
    pub volatility: T,
    /// Forward rate/price at this expiry
    pub forward: T,
}

impl<T: Float> VolQuote<T> {
    /// Creates a new volatility quote.
    pub fn new(strike: T, volatility: T, forward: T) -> Self {
        Self { strike, volatility, forward }
    }
}

// =============================================================================
// Slice Calibrator Trait
// =============================================================================

/// Trait for calibrating a single parameter slice.
///
/// A slice represents parameters at a fixed expiry (for FX) or
/// expiry-tenor pair (for swaptions).
pub trait SliceCalibrator<T: Float> {
    /// The output parameter type for a single slice.
    type Params;

    /// Calibrates parameters from a set of volatility quotes.
    fn calibrate_slice(
        &self,
        quotes: &[VolQuote<T>],
        config: &SliceCalibrationConfig<T>,
    ) -> Result<Self::Params, CalibrationError>;
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
}

impl<T: Float> Default for SliceCalibrationConfig<T> {
    fn default() -> Self {
        Self {
            fixed_beta: Some(from_f64(0.5)),
            max_iterations: 100,
            tolerance: from_f64(1e-8),
            initial_alpha: from_f64(0.03),
        }
    }
}

impl<T: Float> SliceCalibrationConfig<T> {
    /// Creates a configuration for rates (β = 0.5).
    pub fn rates() -> Self {
        Self {
            fixed_beta: Some(from_f64(0.5)),
            ..Self::default()
        }
    }

    /// Creates a configuration for FX (β = 1.0).
    pub fn fx() -> Self {
        Self {
            fixed_beta: Some(from_f64(1.0)),
            ..Self::default()
        }
    }
}

// =============================================================================
// SABR Slice Calibrator
// =============================================================================

/// SABR model slice calibrator.
///
/// Calibrates SABR parameters (α, ρ, ν) for a single expiry/tenor slice,
/// with β typically fixed.
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
    ) -> Result<Self::Params, CalibrationError> {
        if quotes.is_empty() {
            return Err(CalibrationError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }

        // Find ATM quote for initial guess
        let atm_quote = quotes
            .iter()
            .min_by(|a, b| {
                let diff_a = (a.strike - a.forward).abs();
                let diff_b = (b.strike - b.forward).abs();
                diff_a.partial_cmp(&diff_b).unwrap()
            })
            .unwrap();

        let beta = config.fixed_beta.unwrap_or(from_f64(0.5));
        let alpha = atm_quote.volatility * atm_quote.forward.powf(T::one() - beta);

        // Placeholder: return initial guess
        // TODO: Implement full SABR calibration using pricer_core optimisers
        let params = SabrParams::new(alpha, beta, from_f64(-0.3), from_f64(0.4));

        params.validate()?;
        Ok(params)
    }
}

// =============================================================================
// OrderedFloat (for BTreeMap keys)
// =============================================================================

/// Wrapper for Float that implements Ord (for use in BTreeMap keys).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderedFloat<T: Float>(pub T);

impl<T: Float> Eq for OrderedFloat<T> {}

impl<T: Float> PartialOrd for OrderedFloat<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Float> Ord for OrderedFloat<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}

// =============================================================================
// Tests
// =============================================================================

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
        let quote: VolQuote<f64> = VolQuote::new(0.03, 0.2, 0.03);
        assert!((quote.strike - 0.03).abs() < 1e-10);
        assert!((quote.volatility - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_slice_calibration_config_defaults() {
        let rates: SliceCalibrationConfig<f64> = SliceCalibrationConfig::rates();
        assert!((rates.fixed_beta.unwrap() - 0.5).abs() < 1e-10);

        let fx: SliceCalibrationConfig<f64> = SliceCalibrationConfig::fx();
        assert!((fx.fixed_beta.unwrap() - 1.0).abs() < 1e-10);
    }
}
