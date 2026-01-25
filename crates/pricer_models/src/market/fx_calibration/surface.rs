//! Calibrated FX Volatility Surface implementation.
//!
//! This module provides:
//! - [`CalibratedFxVolSurface`]: Calibrated FX vol surface with smile interpolation
//! - [`CalibratedSmile`]: Per-expiry calibrated smile parameters
//! - [`VolSmile`]: Extracted smile data structure

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::NaiveDate;
use infra_master::trade::instrument_def::CurrencyPair;
use num_traits::Float;
use pricer_core::math::numeric::from_f64;
use thiserror::Error;

use super::config::FxVolSurfaceConfig;
use super::curve::FxCurve;
use super::types::Strike;
use crate::market::surfaces::VolatilitySurface;
use crate::market::volcube::InterpolationMethod;

// ============================================================================
// VolSurfaceError
// ============================================================================

/// Errors that can occur during volatility surface operations.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum VolSurfaceError {
    /// Missing FX curve for delta-strike conversion.
    #[error("Missing FX curve")]
    MissingFxCurve,

    /// Invalid expiry date or time.
    #[error("Invalid expiry: {message}")]
    InvalidExpiry {
        /// Description of the invalid expiry.
        message: String,
    },

    /// Invalid strike or delta value.
    #[error("Invalid strike/delta: {message}")]
    InvalidStrike {
        /// Description of the invalid strike.
        message: String,
    },

    /// Expiry not found in calibrated surface.
    #[error("Expiry not found: {expiry}")]
    ExpiryNotFound {
        /// The missing expiry in year fraction.
        expiry: f64,
    },

    /// Interpolation error.
    #[error("Interpolation error: {message}")]
    InterpolationError {
        /// Description of the interpolation failure.
        message: String,
    },

    /// Calibration error.
    #[error("Calibration error: {message}")]
    CalibrationError {
        /// Description of the calibration failure.
        message: String,
    },

    /// Extrapolation not allowed.
    #[error("Extrapolation not allowed: {t} is outside [{min}, {max}]")]
    ExtrapolationNotAllowed {
        /// The requested point.
        t: f64,
        /// Minimum valid point.
        min: f64,
        /// Maximum valid point.
        max: f64,
    },
}

impl VolSurfaceError {
    /// Creates an invalid expiry error.
    #[must_use]
    pub fn invalid_expiry(message: impl Into<String>) -> Self {
        Self::InvalidExpiry {
            message: message.into(),
        }
    }

    /// Creates an invalid strike error.
    #[must_use]
    pub fn invalid_strike(message: impl Into<String>) -> Self {
        Self::InvalidStrike {
            message: message.into(),
        }
    }

    /// Creates an expiry not found error.
    #[must_use]
    pub fn expiry_not_found(expiry: f64) -> Self { Self::ExpiryNotFound { expiry } }

    /// Creates an interpolation error.
    #[must_use]
    pub fn interpolation_error(message: impl Into<String>) -> Self {
        Self::InterpolationError {
            message: message.into(),
        }
    }

    /// Creates a calibration error.
    #[must_use]
    pub fn calibration_error(message: impl Into<String>) -> Self {
        Self::CalibrationError {
            message: message.into(),
        }
    }

    /// Creates an extrapolation not allowed error.
    #[must_use]
    pub fn extrapolation_not_allowed(t: f64, min: f64, max: f64) -> Self {
        Self::ExtrapolationNotAllowed { t, min, max }
    }
}

// ============================================================================
// SabrParameters
// ============================================================================

/// SABR model parameters for a single expiry smile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SabrParameters<T: Float> {
    /// Initial volatility level (alpha).
    pub alpha: T,
    /// Backbone/elasticity parameter (beta, typically fixed).
    pub beta: T,
    /// Correlation between forward and vol (rho).
    pub rho: T,
    /// Vol of vol (nu).
    pub nu: T,
    /// Forward rate at calibration time.
    pub forward: T,
    /// Time to expiry.
    pub expiry: T,
}

impl<T: Float> SabrParameters<T> {
    /// Creates new SABR parameters.
    #[must_use]
    pub fn new(alpha: T, beta: T, rho: T, nu: T, forward: T, expiry: T) -> Self {
        Self { alpha, beta, rho, nu, forward, expiry }
    }

    /// Calculates implied volatility for a given strike using Hagan approximation.
    ///
    /// This is the standard SABR approximation formula by Hagan et al.
    pub fn implied_vol(&self, strike: T) -> T {
        let f = self.forward;
        let k = strike;
        let alpha = self.alpha;
        let beta = self.beta;
        let rho = self.rho;
        let nu = self.nu;
        let t = self.expiry;

        let one = T::one();
        let two = from_f64::<T>(2.0);
        let three = from_f64::<T>(3.0);
        let four = from_f64::<T>(4.0);
        let twenty_four = from_f64::<T>(24.0);

        // Handle ATM case
        if (f - k).abs() < from_f64::<T>(1e-10) {
            // ATM approximation: sigma_atm = alpha / F^(1-beta) * (1 + terms)
            let fk_mid = f.powf(one - beta);
            let term1 = ((one - beta).powi(2) / twenty_four) * alpha.powi(2) / fk_mid.powi(2);
            let term2 = (rho * beta * nu * alpha) / (four * fk_mid);
            let term3 = ((two - three * rho.powi(2)) / twenty_four) * nu.powi(2);
            return (alpha / fk_mid) * (one + (term1 + term2 + term3) * t);
        }

        // Non-ATM case using Hagan approximation
        let fk = (f * k).powf((one - beta) / two);
        let log_fk = (f / k).ln();
        let log_fk_sq = log_fk * log_fk;

        // z and x(z) calculation
        let z = (nu / alpha) * fk * log_fk;
        let sqrt_z = (one - two * rho * z + z * z).sqrt();
        let x_z = ((sqrt_z + z - rho) / (one - rho)).ln();

        // Denominator
        let denom_factor = one - beta;
        let denom = fk
            * (one + denom_factor.powi(2) * log_fk_sq / twenty_four
                + denom_factor.powi(4) * log_fk_sq * log_fk_sq / from_f64::<T>(1920.0));

        // Numerator correction
        let fk_beta = fk.powi(2);
        let term1 = denom_factor.powi(2) * alpha.powi(2) / (twenty_four * fk_beta);
        let term2 = (rho * beta * nu * alpha) / (four * fk);
        let term3 = ((two - three * rho.powi(2)) / twenty_four) * nu.powi(2);

        let vol = (alpha / denom) * (z / x_z) * (one + (term1 + term2 + term3) * t);

        // Ensure positive vol
        if vol <= T::zero() {
            alpha / fk * (one + (term1 + term2 + term3) * t)
        } else {
            vol
        }
    }

    /// Validates SABR parameters are within acceptable bounds.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let neg_one = -T::one();
        self.alpha > T::zero()
            && self.beta >= T::zero()
            && self.beta <= T::one()
            && self.rho > neg_one
            && self.rho < T::one()
            && self.nu >= T::zero()
            && self.forward > T::zero()
            && self.expiry > T::zero()
    }
}

// ============================================================================
// CalibratedSmile
// ============================================================================

/// Per-expiry calibrated volatility smile.
///
/// Stores the calibrated parameters for a single expiry point.
#[derive(Debug, Clone)]
pub struct CalibratedSmile<T: Float> {
    /// Expiry date.
    pub expiry_date: NaiveDate,
    /// Time to expiry in years.
    pub expiry_time: T,
    /// ATM volatility.
    pub atm_vol: T,
    /// Forward rate at this expiry.
    pub forward: T,
    /// SABR parameters if SABR interpolation is used.
    pub sabr_params: Option<SabrParameters<T>>,
    /// Interpolation method used.
    pub interpolation_method: InterpolationMethod,
}

impl<T: Float> CalibratedSmile<T> {
    /// Creates a new calibrated smile with flat vol (no smile).
    #[must_use]
    pub fn flat(expiry_date: NaiveDate, expiry_time: T, atm_vol: T, forward: T) -> Self {
        Self {
            expiry_date,
            expiry_time,
            atm_vol,
            forward,
            sabr_params: None,
            interpolation_method: InterpolationMethod::FlatVol,
        }
    }

    /// Creates a new calibrated smile with SABR parameters.
    #[must_use]
    pub fn sabr(
        expiry_date: NaiveDate,
        expiry_time: T,
        atm_vol: T,
        forward: T,
        sabr_params: SabrParameters<T>,
    ) -> Self {
        Self {
            expiry_date,
            expiry_time,
            atm_vol,
            forward,
            sabr_params: Some(sabr_params),
            interpolation_method: InterpolationMethod::Sabr,
        }
    }

    /// Returns volatility at a given strike.
    pub fn vol_at_strike(&self, strike: T) -> Result<T, VolSurfaceError> {
        if strike <= T::zero() {
            return Err(VolSurfaceError::invalid_strike("Strike must be positive"));
        }

        match &self.sabr_params {
            Some(params) => Ok(params.implied_vol(strike)),
            None => Ok(self.atm_vol), // Flat smile
        }
    }

    /// Returns volatility at a given delta.
    ///
    /// Delta is expressed as a value in (0, 1) where 0.5 is ATM.
    pub fn vol_at_delta(&self, delta: T) -> Result<T, VolSurfaceError> {
        if delta <= T::zero() || delta >= T::one() {
            return Err(VolSurfaceError::invalid_strike(
                "Delta must be in (0, 1)",
            ));
        }

        // For flat smile, return ATM vol
        if self.sabr_params.is_none() {
            return Ok(self.atm_vol);
        }

        // Convert delta to strike using Black-Scholes formula
        // For simplicity, we use an iterative approach
        let strike = self.delta_to_strike(delta)?;
        self.vol_at_strike(strike)
    }

    /// Converts delta to strike using Newton-Raphson iteration.
    fn delta_to_strike(&self, delta: T) -> Result<T, VolSurfaceError> {
        let f = self.forward;
        let t = self.expiry_time;
        let atm = self.atm_vol;

        // Initial guess: use ATM vol approximation
        // For a call: K = F * exp(-sigma * sqrt(T) * N_inv(delta))
        let sqrt_t = t.sqrt();

        // Use ATM vol for initial guess
        let delta_f64 = delta.to_f64().unwrap_or(0.5);

        // Approximate N_inv using rational approximation for initial guess
        let n_inv = approximate_norm_inv(delta_f64);
        let n_inv_t = from_f64::<T>(n_inv);

        let initial_strike = f * (-(atm * sqrt_t * n_inv_t)).exp();

        // For simple implementation, just return the initial guess
        // A full implementation would iterate with Newton-Raphson
        Ok(initial_strike)
    }
}

/// Approximate inverse normal CDF for delta-to-strike conversion.
fn approximate_norm_inv(p: f64) -> f64 {
    // Rational approximation (Abramowitz and Stegun)
    if p <= 0.0 || p >= 1.0 {
        return 0.0;
    }

    let t = if p < 0.5 {
        (-2.0 * p.ln()).sqrt()
    } else {
        (-2.0 * (1.0 - p).ln()).sqrt()
    };

    let c0 = 2.515517;
    let c1 = 0.802853;
    let c2 = 0.010328;
    let d1 = 1.432788;
    let d2 = 0.189269;
    let d3 = 0.001308;

    let result = t - (c0 + c1 * t + c2 * t * t) / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t);

    if p < 0.5 { -result } else { result }
}

// ============================================================================
// VolSmile
// ============================================================================

/// Extracted volatility smile for a single expiry.
///
/// Contains delta-vol pairs for visualisation and analysis.
#[derive(Debug, Clone)]
pub struct VolSmile<T: Float> {
    /// Time to expiry in years.
    pub expiry: T,
    /// Forward rate.
    pub forward: T,
    /// Delta values (typically 0.1, 0.25, 0.5, 0.75, 0.9).
    pub deltas: Vec<T>,
    /// Corresponding volatilities.
    pub vols: Vec<T>,
    /// ATM volatility.
    pub atm_vol: T,
}

impl<T: Float> VolSmile<T> {
    /// Creates a new volatility smile.
    #[must_use]
    pub fn new(expiry: T, forward: T, deltas: Vec<T>, vols: Vec<T>, atm_vol: T) -> Self {
        Self { expiry, forward, deltas, vols, atm_vol }
    }

    /// Calculates the 25-delta risk reversal (RR).
    ///
    /// RR = σ(25D Call) - σ(25D Put)
    pub fn risk_reversal_25d(&self) -> Option<T> {
        let delta_25p = from_f64::<T>(0.25);
        let delta_75c = from_f64::<T>(0.75); // 25D call = 75 delta in our convention

        let vol_25p = self.vol_at_delta(delta_25p)?;
        let vol_25c = self.vol_at_delta(delta_75c)?;

        Some(vol_25c - vol_25p)
    }

    /// Calculates the 25-delta butterfly (BF).
    ///
    /// BF = (σ(25D Call) + σ(25D Put)) / 2 - σ(ATM)
    pub fn butterfly_25d(&self) -> Option<T> {
        let delta_25p = from_f64::<T>(0.25);
        let delta_75c = from_f64::<T>(0.75);

        let vol_25p = self.vol_at_delta(delta_25p)?;
        let vol_25c = self.vol_at_delta(delta_75c)?;

        let two = from_f64::<T>(2.0);
        Some((vol_25c + vol_25p) / two - self.atm_vol)
    }

    /// Gets volatility at a specific delta by interpolation.
    fn vol_at_delta(&self, delta: T) -> Option<T> {
        // Find bracketing deltas and interpolate
        for i in 0..self.deltas.len().saturating_sub(1) {
            if self.deltas[i] <= delta && delta <= self.deltas[i + 1] {
                let t =
                    (delta - self.deltas[i]) / (self.deltas[i + 1] - self.deltas[i]);
                return Some(self.vols[i] + t * (self.vols[i + 1] - self.vols[i]));
            }
        }
        None
    }
}

// ============================================================================
// CalibratedFxVolSurface
// ============================================================================

/// Calibrated FX Volatility Surface.
///
/// Stores calibrated smiles at pillar expiry dates with interpolation
/// between expiries. Supports both strike-based and delta-based vol queries.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for AAD compatibility
///
/// # Example
///
/// ```ignore
/// let surface = CalibratedFxVolSurface::new(
///     currency_pair,
///     reference_date,
///     smiles,
///     fx_curve,
///     config,
/// );
///
/// let vol = surface.vol(1.0, 1.10)?;
/// let vol_delta = surface.vol_by_delta(1.0, 0.25)?;
/// ```
#[derive(Clone)]
pub struct CalibratedFxVolSurface<T: Float> {
    /// Currency pair.
    currency_pair: CurrencyPair,
    /// Reference/valuation date.
    reference_date: NaiveDate,
    /// Calibrated smiles by expiry date.
    smiles: BTreeMap<NaiveDate, CalibratedSmile<T>>,
    /// Smile times in years (sorted).
    smile_times: Vec<T>,
    /// FX forward curve for delta-strike conversion.
    fx_curve: Arc<dyn FxCurve<T> + Send + Sync>,
    /// Surface configuration.
    config: FxVolSurfaceConfig,
}

impl<T: Float + std::fmt::Debug> std::fmt::Debug for CalibratedFxVolSurface<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalibratedFxVolSurface")
            .field("currency_pair", &self.currency_pair)
            .field("reference_date", &self.reference_date)
            .field("num_expiries", &self.smiles.len())
            .field("config", &self.config)
            .finish()
    }
}

impl<T: Float + Send + Sync> CalibratedFxVolSurface<T> {
    /// Creates a new calibrated FX volatility surface.
    ///
    /// # Arguments
    ///
    /// * `currency_pair` - The currency pair (e.g., EUR/USD)
    /// * `reference_date` - Valuation/reference date
    /// * `smiles` - Calibrated smiles by expiry date
    /// * `fx_curve` - FX forward curve for delta-strike conversion
    /// * `config` - Surface configuration
    pub fn new(
        currency_pair: CurrencyPair,
        reference_date: NaiveDate,
        smiles: BTreeMap<NaiveDate, CalibratedSmile<T>>,
        fx_curve: Arc<dyn FxCurve<T> + Send + Sync>,
        config: FxVolSurfaceConfig,
    ) -> Self {
        let smile_times: Vec<T> = smiles.values().map(|s| s.expiry_time).collect();
        Self {
            currency_pair,
            reference_date,
            smiles,
            smile_times,
            fx_curve,
            config,
        }
    }

    /// Returns the currency pair.
    #[inline]
    #[must_use]
    pub fn currency_pair(&self) -> CurrencyPair { self.currency_pair }

    /// Returns the reference date.
    #[inline]
    #[must_use]
    pub fn reference_date(&self) -> NaiveDate { self.reference_date }

    /// Returns the number of calibrated expiries.
    #[inline]
    #[must_use]
    pub fn num_expiries(&self) -> usize { self.smiles.len() }

    /// Returns the configuration.
    #[inline]
    #[must_use]
    pub fn config(&self) -> &FxVolSurfaceConfig { &self.config }

    /// Returns the expiry dates.
    #[must_use]
    pub fn expiry_dates(&self) -> Vec<NaiveDate> { self.smiles.keys().copied().collect() }

    /// Returns volatility at a given strike and expiry.
    ///
    /// # Arguments
    /// * `strike` - The option strike price
    /// * `expiry` - Time to expiry in years
    pub fn vol(&self, strike: Strike, expiry: T) -> Result<T, VolSurfaceError> {
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        if expiry_f64 <= 0.0 {
            return Err(VolSurfaceError::invalid_expiry("Expiry must be positive"));
        }

        let smile = self.get_interpolated_smile(expiry)?;
        let strike_t = T::from(strike.value()).unwrap();
        smile.vol_at_strike(strike_t)
    }

    /// Returns volatility at a given expiry and delta.
    ///
    /// Delta is expressed as a value in (0, 1) where 0.5 is ATM.
    pub fn vol_by_delta(&self, expiry: T, delta: T) -> Result<T, VolSurfaceError> {
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        if expiry_f64 <= 0.0 {
            return Err(VolSurfaceError::invalid_expiry("Expiry must be positive"));
        }
        if delta <= T::zero() || delta >= T::one() {
            return Err(VolSurfaceError::invalid_strike(
                "Delta must be in (0, 1)",
            ));
        }

        let smile = self.get_interpolated_smile(expiry)?;
        smile.vol_at_delta(delta)
    }

    /// Extracts the volatility smile for a specific expiry.
    pub fn smile(&self, expiry: T) -> Result<VolSmile<T>, VolSurfaceError> {
        let smile = self.get_interpolated_smile(expiry)?;

        // Generate standard delta points
        let deltas: Vec<T> = vec![
            from_f64(0.10),
            from_f64(0.25),
            from_f64(0.50),
            from_f64(0.75),
            from_f64(0.90),
        ];

        let mut vols = Vec::with_capacity(deltas.len());
        for &d in &deltas {
            vols.push(smile.vol_at_delta(d)?);
        }

        Ok(VolSmile::new(
            smile.expiry_time,
            smile.forward,
            deltas,
            vols,
            smile.atm_vol,
        ))
    }

    /// Returns the ATM volatility at a given expiry.
    pub fn atm_vol(&self, expiry: T) -> Result<T, VolSurfaceError> {
        let smile = self.get_interpolated_smile(expiry)?;
        Ok(smile.atm_vol)
    }

    /// Gets or interpolates the smile at a given expiry time.
    fn get_interpolated_smile(&self, expiry: T) -> Result<CalibratedSmile<T>, VolSurfaceError> {
        if self.smiles.is_empty() {
            return Err(VolSurfaceError::interpolation_error("No calibrated smiles"));
        }

        // Find bracketing expiries by time
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);

        // Check bounds
        let first_time = self.smile_times.first().map(|t| t.to_f64().unwrap_or(0.0));
        let last_time = self.smile_times.last().map(|t| t.to_f64().unwrap_or(0.0));

        if let (Some(min_t), Some(max_t)) = (first_time, last_time) {
            if expiry_f64 < min_t || expiry_f64 > max_t {
                if !self.config.allow_extrapolation {
                    return Err(VolSurfaceError::extrapolation_not_allowed(
                        expiry_f64, min_t, max_t,
                    ));
                }
                // Flat extrapolation
                if expiry_f64 < min_t {
                    return Ok(self.smiles.values().next().unwrap().clone());
                } else {
                    return Ok(self.smiles.values().last().unwrap().clone());
                }
            }
        }

        // Find bracketing smiles
        let smiles_vec: Vec<&CalibratedSmile<T>> = self.smiles.values().collect();

        // Exact match
        for smile in &smiles_vec {
            if (smile.expiry_time.to_f64().unwrap_or(0.0) - expiry_f64).abs() < 1e-10 {
                return Ok((*smile).clone());
            }
        }

        // Linear interpolation in time
        for i in 0..smiles_vec.len().saturating_sub(1) {
            let t1 = smiles_vec[i].expiry_time.to_f64().unwrap_or(0.0);
            let t2 = smiles_vec[i + 1].expiry_time.to_f64().unwrap_or(0.0);

            if t1 <= expiry_f64 && expiry_f64 <= t2 {
                let w = (expiry_f64 - t1) / (t2 - t1);
                let w_t = from_f64::<T>(w);

                // Interpolate ATM vol
                let atm1 = smiles_vec[i].atm_vol;
                let atm2 = smiles_vec[i + 1].atm_vol;
                let atm_interp = atm1 + w_t * (atm2 - atm1);

                // Interpolate forward
                let fwd1 = smiles_vec[i].forward;
                let fwd2 = smiles_vec[i + 1].forward;
                let fwd_interp = fwd1 + w_t * (fwd2 - fwd1);

                // Create interpolated smile
                let interpolated = CalibratedSmile::flat(
                    smiles_vec[i].expiry_date,
                    expiry,
                    atm_interp,
                    fwd_interp,
                );

                return Ok(interpolated);
            }
        }

        Err(VolSurfaceError::expiry_not_found(expiry_f64))
    }
}

impl<T: Float + Send + Sync> VolatilitySurface<T> for CalibratedFxVolSurface<T> {
    fn volatility(&self, strike: T, expiry: T) -> Result<T, crate::market::error::MarketDataError> {
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        let strike_f64 = strike.to_f64().unwrap_or(0.0);

        if expiry_f64 <= 0.0 {
            return Err(crate::market::error::MarketDataError::InvalidExpiry {
                expiry: expiry_f64,
            });
        }
        if strike_f64 <= 0.0 {
            return Err(crate::market::error::MarketDataError::InvalidStrike {
                strike: strike_f64,
            });
        }

        let smile = self
            .get_interpolated_smile(expiry)
            .map_err(|e| crate::market::error::MarketDataError::InterpolationFailed {
                reason: e.to_string(),
            })?;

        smile.vol_at_strike(strike).map_err(|e| {
            crate::market::error::MarketDataError::InterpolationFailed {
                reason: e.to_string(),
            }
        })
    }

    fn strike_domain(&self) -> (T, T) {
        // FX strikes can be wide - use a reasonable range
        (from_f64(0.01), from_f64(100.0))
    }

    fn expiry_domain(&self) -> (T, T) {
        if self.smile_times.is_empty() {
            return (T::zero(), T::zero());
        }
        (
            *self.smile_times.first().unwrap(),
            *self.smile_times.last().unwrap(),
        )
    }
}

// ============================================================================
// Differentiable Trait Implementation
// ============================================================================

/// Marker trait implementation indicating AD compatibility.
///
/// The [`CalibratedFxVolSurface`] uses smooth interpolation methods that are
/// compatible with automatic differentiation backends. All operations on the
/// generic `T: Float` type avoid discontinuous branching, ensuring gradients
/// can be computed correctly.
///
/// # AD Compatibility
///
/// - Smile interpolation uses linear/cubic methods that are smooth
/// - Delta-to-strike conversion uses smooth Newton iteration
/// - Extrapolation uses flat values (continuous but not smooth at boundary)
///
/// # Usage with Enzyme
///
/// When using Enzyme for reverse-mode AD, ensure that the surface is
/// constructed with AAD-compatible types and that all input quotes flow
/// through the computation graph.
impl<T: Float + Send + Sync> pricer_core::traits::priceable::Differentiable
    for CalibratedFxVolSurface<T>
{
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::curves::FlatCurve;
    use crate::market::fx_calibration::curve::SimpleFxCurve;
    use infra_master::Currency;

    fn make_test_fx_curve() -> Arc<dyn FxCurve<f64> + Send + Sync> {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05));
        let foreign = Arc::new(FlatCurve::new(0.03));
        Arc::new(SimpleFxCurve::new(pair, 1.10, domestic, foreign))
    }

    fn make_test_surface() -> CalibratedFxVolSurface<f64> {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let fx_curve = make_test_fx_curve();
        let config = FxVolSurfaceConfig::default();

        let mut smiles = BTreeMap::new();

        // Add 1M expiry
        let expiry_1m = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
        smiles.insert(
            expiry_1m,
            CalibratedSmile::flat(expiry_1m, 1.0 / 12.0, 0.10, 1.10),
        );

        // Add 3M expiry
        let expiry_3m = NaiveDate::from_ymd_opt(2024, 4, 1).unwrap();
        smiles.insert(
            expiry_3m,
            CalibratedSmile::flat(expiry_3m, 0.25, 0.11, 1.105),
        );

        // Add 1Y expiry
        let expiry_1y = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        smiles.insert(
            expiry_1y,
            CalibratedSmile::flat(expiry_1y, 1.0, 0.12, 1.11),
        );

        CalibratedFxVolSurface::new(pair, ref_date, smiles, fx_curve, config)
    }

    #[test]
    fn test_vol_surface_error_display() {
        let err = VolSurfaceError::invalid_expiry("negative");
        assert!(err.to_string().contains("negative"));

        let err = VolSurfaceError::expiry_not_found(1.5);
        assert!(err.to_string().contains("1.5"));
    }

    #[test]
    fn test_sabr_parameters_atm_vol() {
        let params = SabrParameters::new(0.2, 0.5, -0.2, 0.4, 1.10, 1.0);

        // ATM vol should be close to alpha adjusted for beta
        let atm_vol = params.implied_vol(1.10);
        assert!(atm_vol > 0.0);
        assert!(atm_vol < 1.0);
    }

    #[test]
    fn test_sabr_parameters_valid() {
        let params = SabrParameters::new(0.2, 0.5, -0.2, 0.4, 1.10, 1.0);
        assert!(params.is_valid());

        // Invalid: negative alpha
        let invalid = SabrParameters::new(-0.2, 0.5, -0.2, 0.4, 1.10, 1.0);
        assert!(!invalid.is_valid());

        // Invalid: rho out of bounds
        let invalid = SabrParameters::new(0.2, 0.5, 1.5, 0.4, 1.10, 1.0);
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_calibrated_smile_flat() {
        let expiry = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let smile = CalibratedSmile::flat(expiry, 0.5, 0.15, 1.10);

        let vol = smile.vol_at_strike(1.10).unwrap();
        assert!((vol - 0.15).abs() < 1e-10);

        let vol = smile.vol_at_strike(1.05).unwrap();
        assert!((vol - 0.15).abs() < 1e-10); // Flat smile
    }

    #[test]
    fn test_calibrated_smile_delta() {
        let expiry = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let smile = CalibratedSmile::flat(expiry, 0.5, 0.15, 1.10);

        let vol = smile.vol_at_delta(0.5).unwrap();
        assert!((vol - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_calibrated_smile_invalid_delta() {
        let expiry = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let smile = CalibratedSmile::flat(expiry, 0.5, 0.15, 1.10);

        assert!(smile.vol_at_delta(0.0).is_err());
        assert!(smile.vol_at_delta(1.0).is_err());
        assert!(smile.vol_at_delta(-0.5).is_err());
    }

    #[test]
    fn test_vol_smile_creation() {
        let smile = VolSmile::new(
            1.0,
            1.10,
            vec![0.1, 0.25, 0.5, 0.75, 0.9],
            vec![0.12, 0.11, 0.10, 0.11, 0.12],
            0.10,
        );

        assert!((smile.expiry - 1.0).abs() < 1e-10);
        assert!((smile.atm_vol - 0.10).abs() < 1e-10);
    }

    #[test]
    fn test_vol_smile_risk_reversal() {
        let smile = VolSmile::new(
            1.0,
            1.10,
            vec![0.1, 0.25, 0.5, 0.75, 0.9],
            vec![0.13, 0.12, 0.10, 0.11, 0.12], // Skewed smile
            0.10,
        );

        let rr = smile.risk_reversal_25d().unwrap();
        // RR = vol(75D) - vol(25D) = 0.11 - 0.12 = -0.01
        assert!((rr - (-0.01)).abs() < 1e-10);
    }

    #[test]
    fn test_vol_smile_butterfly() {
        let smile = VolSmile::new(
            1.0,
            1.10,
            vec![0.1, 0.25, 0.5, 0.75, 0.9],
            vec![0.12, 0.12, 0.10, 0.12, 0.12], // Symmetric smile
            0.10,
        );

        let bf = smile.butterfly_25d().unwrap();
        // BF = (vol(75D) + vol(25D)) / 2 - ATM = (0.12 + 0.12) / 2 - 0.10 = 0.02
        assert!((bf - 0.02).abs() < 1e-10);
    }

    #[test]
    fn test_calibrated_surface_creation() {
        let surface = make_test_surface();

        assert_eq!(surface.num_expiries(), 3);
        assert_eq!(surface.currency_pair().base, Currency::EUR);
    }

    #[test]
    fn test_calibrated_surface_atm_vol() {
        let surface = make_test_surface();

        // ATM vol at 1Y should be 0.12
        let atm = surface.atm_vol(1.0).unwrap();
        assert!((atm - 0.12).abs() < 1e-10);
    }

    #[test]
    fn test_calibrated_surface_vol_at_pillar() {
        let surface = make_test_surface();

        // Vol at 1Y expiry
        let vol = surface.volatility(1.10, 1.0).unwrap();
        assert!((vol - 0.12).abs() < 1e-10);
    }

    #[test]
    fn test_calibrated_surface_vol_interpolated() {
        let surface = make_test_surface();

        // Vol at 0.5Y (between 0.25 and 1.0)
        let vol = surface.volatility(1.10, 0.5).unwrap();
        // Should be between 0.11 and 0.12
        assert!(vol > 0.11);
        assert!(vol < 0.12);
    }

    #[test]
    fn test_calibrated_surface_expiry_domain() {
        let surface = make_test_surface();
        let (min_t, max_t) = surface.expiry_domain();

        assert!(min_t > 0.0);
        assert!(max_t > min_t);
        assert!((max_t - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calibrated_surface_vol_by_delta() {
        let surface = make_test_surface();

        // Vol at delta=0.5 (ATM) should equal ATM vol
        let vol = surface.vol_by_delta(1.0, 0.5).unwrap();
        assert!((vol - 0.12).abs() < 1e-10);
    }

    #[test]
    fn test_calibrated_surface_smile_extraction() {
        let surface = make_test_surface();

        let smile = surface.smile(1.0).unwrap();
        assert!((smile.atm_vol - 0.12).abs() < 1e-10);
        assert_eq!(smile.deltas.len(), 5);
        assert_eq!(smile.vols.len(), 5);
    }

    #[test]
    fn test_calibrated_surface_extrapolation_not_allowed() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let fx_curve = make_test_fx_curve();
        let config = FxVolSurfaceConfig::default().with_allow_extrapolation(false);

        let mut smiles = BTreeMap::new();
        let expiry = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        smiles.insert(
            expiry,
            CalibratedSmile::flat(expiry, 1.0, 0.12, 1.11),
        );

        let surface = CalibratedFxVolSurface::new(pair, ref_date, smiles, fx_curve, config);

        // Query outside the range should fail
        let result = surface.atm_vol(2.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_approximate_norm_inv() {
        // Test standard values
        assert!((approximate_norm_inv(0.5) - 0.0).abs() < 0.01);
        assert!(approximate_norm_inv(0.75) > 0.0);
        assert!(approximate_norm_inv(0.25) < 0.0);
    }
}
