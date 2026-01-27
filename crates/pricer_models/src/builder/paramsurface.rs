//! Slice-wise parameter surface calibration.
//!
//! This module provides builders for calibrating model parameters (e.g., SABR α, β, ρ, ν)
//! across volatility surfaces and cubes. Unlike global solvers that solve all parameters
//! simultaneously, slice-wise calibration:
//!
//! 1. Calibrates each expiry/tenor slice independently
//! 2. Aggregates calibrated slices into a complete surface/cube
//!
//! ## Calibration Patterns Comparison
//!
//! | Pattern | Module | Description |
//! |---------|--------|-------------|
//! | Sequential | `bootstrap` | Solve one at a time, using previous results |
//! | Slice-wise | `paramsurface` | Solve each slice independently, then aggregate |
//! | Global | `globalsolver` | Solve all parameters simultaneously |
//!
//! ## Supported Surface Types
//!
//! - **VolCube**: Swaption volatility cube (3D: expiry × tenor × strike)
//! - **FxVol**: FX volatility surface (2D: expiry × strike)
//!
//! ## Example
//!
//! ```ignore
//! use pricer_models::builder::{VolCubeBuilder, FxVolBuilder};
//!
//! // Build a swaption vol cube
//! let cube = VolCubeBuilder::new()
//!     .add_slice(expiry, tenor, quotes)
//!     .calibrate()?;
//!
//! // Build an FX vol surface
//! let surface = FxVolBuilder::new()
//!     .add_slice(expiry, quotes)
//!     .calibrate()?;
//! ```

use num_traits::Float;
use std::collections::BTreeMap;

use pricer_core::math::numeric::from_f64;

use super::error::CalibrationError;

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
    ///
    /// # Arguments
    ///
    /// * `alpha` - Initial volatility (must be positive)
    /// * `beta` - CEV exponent (typically 0.0, 0.5, or 1.0)
    /// * `rho` - Spot-vol correlation (must be in (-1, 1))
    /// * `nu` - Vol-of-vol (must be positive)
    pub fn new(alpha: T, beta: T, rho: T, nu: T) -> Self {
        Self {
            alpha,
            beta,
            rho,
            nu,
        }
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
                param: "alpha".to_string(),
                value: self.alpha.to_f64().unwrap_or(0.0),
                min: 0.0,
                max: f64::INFINITY,
            });
        }
        if self.beta < T::zero() || self.beta > T::one() {
            return Err(CalibrationError::BoundsViolation {
                param: "beta".to_string(),
                value: self.beta.to_f64().unwrap_or(0.0),
                min: 0.0,
                max: 1.0,
            });
        }
        if self.rho <= -T::one() || self.rho >= T::one() {
            return Err(CalibrationError::BoundsViolation {
                param: "rho".to_string(),
                value: self.rho.to_f64().unwrap_or(0.0),
                min: -1.0,
                max: 1.0,
            });
        }
        if self.nu <= T::zero() {
            return Err(CalibrationError::BoundsViolation {
                param: "nu".to_string(),
                value: self.nu.to_f64().unwrap_or(0.0),
                min: 0.0,
                max: f64::INFINITY,
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
// Volatility Quote Types
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
        Self {
            strike,
            volatility,
            forward,
        }
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
    ///
    /// # Arguments
    ///
    /// * `quotes` - Market volatility quotes at different strikes
    /// * `config` - Calibration configuration
    ///
    /// # Returns
    ///
    /// Calibrated parameters for this slice.
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

        // For now, use a simple initial guess based on ATM vol
        // Full implementation would use Levenberg-Marquardt or similar
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
// VolCube Builder (Swaptions: 3D)
// =============================================================================

/// Builder for swaption volatility cubes.
///
/// Calibrates SABR parameters for each (expiry, tenor) slice independently,
/// then aggregates into a complete parameter cube.
#[derive(Debug, Clone)]
pub struct VolCubeBuilder<T: Float> {
    /// Quotes organised by (expiry, tenor)
    slices: BTreeMap<(OrderedFloat<T>, OrderedFloat<T>), Vec<VolQuote<T>>>,
    /// Calibration configuration
    config: SliceCalibrationConfig<T>,
    /// Slice calibrator
    calibrator: SabrSliceCalibrator<T>,
}

/// Wrapper for Float that implements Ord (for use in BTreeMap keys).
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedFloat<T: Float>(T);

impl<T: Float> Eq for OrderedFloat<T> {}

impl<T: Float> PartialOrd for OrderedFloat<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Float> Ord for OrderedFloat<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl<T: Float> Default for VolCubeBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Float> VolCubeBuilder<T> {
    /// Creates a new VolCube builder with default configuration.
    pub fn new() -> Self {
        Self {
            slices: BTreeMap::new(),
            config: SliceCalibrationConfig::rates(),
            calibrator: SabrSliceCalibrator::new(),
        }
    }

    /// Creates a builder with custom configuration.
    pub fn with_config(config: SliceCalibrationConfig<T>) -> Self {
        Self {
            slices: BTreeMap::new(),
            config,
            calibrator: SabrSliceCalibrator::new(),
        }
    }

    /// Adds quotes for a single (expiry, tenor) slice.
    pub fn add_slice(&mut self, expiry: T, tenor: T, quotes: Vec<VolQuote<T>>) -> &mut Self {
        let key = (OrderedFloat(expiry), OrderedFloat(tenor));
        self.slices.entry(key).or_default().extend(quotes);
        self
    }

    /// Adds a single quote.
    pub fn add_quote(
        &mut self,
        expiry: T,
        tenor: T,
        strike: T,
        volatility: T,
        forward: T,
    ) -> &mut Self {
        let key = (OrderedFloat(expiry), OrderedFloat(tenor));
        self.slices
            .entry(key)
            .or_default()
            .push(VolQuote::new(strike, volatility, forward));
        self
    }

    /// Calibrates all slices and returns the parameter cube.
    pub fn calibrate(&self) -> Result<VolCubeResult<T>, CalibrationError> {
        if self.slices.is_empty() {
            return Err(CalibrationError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }

        let mut expiries = Vec::new();
        let mut tenors = Vec::new();
        let mut params = BTreeMap::new();

        for ((exp, ten), quotes) in &self.slices {
            let calibrated = self.calibrator.calibrate_slice(quotes, &self.config)?;
            params.insert((exp.0, ten.0), calibrated);

            if !expiries.contains(&exp.0) {
                expiries.push(exp.0);
            }
            if !tenors.contains(&ten.0) {
                tenors.push(ten.0);
            }
        }

        // Sort expiries and tenors
        expiries.sort_by(|a, b| a.partial_cmp(b).unwrap());
        tenors.sort_by(|a, b| a.partial_cmp(b).unwrap());

        Ok(VolCubeResult {
            expiries,
            tenors,
            params,
        })
    }
}

/// Result of VolCube calibration.
#[derive(Debug, Clone)]
pub struct VolCubeResult<T: Float> {
    /// Expiry grid points
    pub expiries: Vec<T>,
    /// Tenor grid points
    pub tenors: Vec<T>,
    /// Calibrated SABR parameters indexed by (expiry, tenor)
    pub params: BTreeMap<(T, T), SabrParams<T>>,
}

impl<T: Float> VolCubeResult<T> {
    /// Gets parameters for a specific (expiry, tenor) point.
    pub fn get(&self, expiry: T, tenor: T) -> Option<&SabrParams<T>> {
        self.params.get(&(expiry, tenor))
    }

    /// Returns the number of calibrated slices.
    pub fn num_slices(&self) -> usize {
        self.params.len()
    }
}

// =============================================================================
// FxVol Builder (FX: 2D)
// =============================================================================

/// Builder for FX volatility surfaces.
///
/// Calibrates SABR parameters for each expiry slice independently,
/// then aggregates into a complete parameter surface.
#[derive(Debug, Clone)]
pub struct FxVolBuilder<T: Float> {
    /// Quotes organised by expiry
    slices: BTreeMap<OrderedFloat<T>, Vec<VolQuote<T>>>,
    /// Calibration configuration
    config: SliceCalibrationConfig<T>,
    /// Slice calibrator
    calibrator: SabrSliceCalibrator<T>,
}

impl<T: Float> Default for FxVolBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Float> FxVolBuilder<T> {
    /// Creates a new FxVol builder with default configuration.
    pub fn new() -> Self {
        Self {
            slices: BTreeMap::new(),
            config: SliceCalibrationConfig::fx(),
            calibrator: SabrSliceCalibrator::new(),
        }
    }

    /// Creates a builder with custom configuration.
    pub fn with_config(config: SliceCalibrationConfig<T>) -> Self {
        Self {
            slices: BTreeMap::new(),
            config,
            calibrator: SabrSliceCalibrator::new(),
        }
    }

    /// Adds quotes for a single expiry slice.
    pub fn add_slice(&mut self, expiry: T, quotes: Vec<VolQuote<T>>) -> &mut Self {
        let key = OrderedFloat(expiry);
        self.slices.entry(key).or_default().extend(quotes);
        self
    }

    /// Adds a single quote.
    pub fn add_quote(&mut self, expiry: T, strike: T, volatility: T, forward: T) -> &mut Self {
        let key = OrderedFloat(expiry);
        self.slices
            .entry(key)
            .or_default()
            .push(VolQuote::new(strike, volatility, forward));
        self
    }

    /// Calibrates all slices and returns the parameter surface.
    pub fn calibrate(&self) -> Result<FxVolResult<T>, CalibrationError> {
        if self.slices.is_empty() {
            return Err(CalibrationError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }

        let mut expiries = Vec::new();
        let mut params = BTreeMap::new();

        for (exp, quotes) in &self.slices {
            let calibrated = self.calibrator.calibrate_slice(quotes, &self.config)?;
            params.insert(exp.0, calibrated);
            expiries.push(exp.0);
        }

        // Sort expiries
        expiries.sort_by(|a, b| a.partial_cmp(b).unwrap());

        Ok(FxVolResult { expiries, params })
    }
}

/// Result of FX vol surface calibration.
#[derive(Debug, Clone)]
pub struct FxVolResult<T: Float> {
    /// Expiry grid points
    pub expiries: Vec<T>,
    /// Calibrated SABR parameters indexed by expiry
    pub params: BTreeMap<T, SabrParams<T>>,
}

impl<T: Float> FxVolResult<T> {
    /// Gets parameters for a specific expiry.
    pub fn get(&self, expiry: T) -> Option<&SabrParams<T>> {
        self.params.get(&expiry)
    }

    /// Returns the number of calibrated slices.
    pub fn num_slices(&self) -> usize {
        self.params.len()
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
    fn test_volcube_builder_empty() {
        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new();
        let result = builder.calibrate();
        assert!(result.is_err());
    }

    #[test]
    fn test_volcube_builder_single_slice() {
        let mut builder: VolCubeBuilder<f64> = VolCubeBuilder::new();
        builder.add_quote(1.0, 5.0, 0.03, 0.2, 0.03);
        builder.add_quote(1.0, 5.0, 0.02, 0.22, 0.03);
        builder.add_quote(1.0, 5.0, 0.04, 0.21, 0.03);

        let result = builder.calibrate();
        assert!(result.is_ok());

        let cube = result.unwrap();
        assert_eq!(cube.num_slices(), 1);
        assert!(cube.get(1.0, 5.0).is_some());
    }

    #[test]
    fn test_fxvol_builder_empty() {
        let builder: FxVolBuilder<f64> = FxVolBuilder::new();
        let result = builder.calibrate();
        assert!(result.is_err());
    }

    #[test]
    fn test_fxvol_builder_single_slice() {
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.25, 1.05, 0.085, 1.10);
        builder.add_quote(0.25, 1.15, 0.082, 1.10);

        let result = builder.calibrate();
        assert!(result.is_ok());

        let surface = result.unwrap();
        assert_eq!(surface.num_slices(), 1);
        assert!(surface.get(0.25).is_some());
    }

    #[test]
    fn test_fxvol_builder_multiple_slices() {
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();

        // 3M expiry
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.25, 1.05, 0.085, 1.10);

        // 1Y expiry
        builder.add_quote(1.0, 1.10, 0.10, 1.10);
        builder.add_quote(1.0, 1.05, 0.105, 1.10);

        let result = builder.calibrate();
        assert!(result.is_ok());

        let surface = result.unwrap();
        assert_eq!(surface.num_slices(), 2);
        assert_eq!(surface.expiries.len(), 2);
    }

    #[test]
    fn test_slice_calibration_config_defaults() {
        let rates: SliceCalibrationConfig<f64> = SliceCalibrationConfig::rates();
        assert!((rates.fixed_beta.unwrap() - 0.5).abs() < 1e-10);

        let fx: SliceCalibrationConfig<f64> = SliceCalibrationConfig::fx();
        assert!((fx.fixed_beta.unwrap() - 1.0).abs() < 1e-10);
    }
}
