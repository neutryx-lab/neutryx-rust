//! SSVI (Surface SVI) implied volatility formula (Gatheral & Jacquier 2014).
//!
//! Parametrises the full implied variance *surface* with a no-calendar-spread-
//! arbitrage guarantee:
//!
//! ```text
//! w(k, θ) = θ/2 · (1 + ρ·φ(θ)·k + √((φ(θ)·k + ρ)² + (1 − ρ²)))
//! φ(θ)   = η / (θ^γ · (1 + θ)^(1 − γ))
//! ```
//!
//! where `θ = σ²_ATM · T` is the ATM total variance and `k = ln(K/F)`.

use num_traits::Float;
use thiserror::Error;

use crate::math::{
    numeric::from_f64,
    smoothing::{smooth_log, smooth_max, smooth_pow, smooth_sqrt},
};

/// Default smoothing epsilon.
const DEFAULT_EPSILON: f64 = 1e-10;

/// Error type for SSVI calculations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SsviError {
    /// Invalid parameter.
    #[error("Invalid SSVI parameter: {0}")]
    InvalidParameter(String),
    /// Invalid strike.
    #[error("Invalid strike: K = {0} (must be positive)")]
    InvalidStrike(f64),
    /// Non-finite result.
    #[error("Non-finite result in {0}")]
    NonFinite(String),
}

/// SSVI global parameters.
///
/// A single set of `(ρ, η, γ)` governs the entire surface, combined with a
/// term-structure of ATM total variances `θ_T`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SsviParams<T: Float> {
    /// Global skew (−1 < ρ < 1).
    pub rho: T,
    /// Vol-of-vol scaling (η > 0).
    pub eta: T,
    /// Power-law exponent (0 < γ ≤ 1).
    pub gamma: T,
}

impl<T: Float> SsviParams<T> {
    /// Validates the SSVI parameters.
    pub fn validate(&self) -> Result<(), SsviError> {
        if self.rho <= -T::one() || self.rho >= T::one() {
            return Err(SsviError::InvalidParameter(format!(
                "rho = {} must be in (-1, 1)",
                self.rho.to_f64().unwrap_or(0.0)
            )));
        }
        if self.eta <= T::zero() {
            return Err(SsviError::InvalidParameter(format!(
                "eta = {} must be > 0",
                self.eta.to_f64().unwrap_or(0.0)
            )));
        }
        if self.gamma <= T::zero() || self.gamma > T::one() {
            return Err(SsviError::InvalidParameter(format!(
                "gamma = {} must be in (0, 1]",
                self.gamma.to_f64().unwrap_or(0.0)
            )));
        }
        Ok(())
    }
}

/// Computes the SSVI mixing function `φ(θ)`.
///
/// `φ(θ) = η / (θ^γ · (1 + θ)^(1 − γ))`
#[inline]
pub fn ssvi_phi<T: Float>(params: &SsviParams<T>, atm_total_var: T, epsilon: T) -> T {
    let one = T::one();
    let theta_gamma = smooth_pow(atm_total_var, params.gamma, epsilon);
    let one_plus_theta = one + atm_total_var;
    let one_minus_gamma = one - params.gamma;
    let second_factor = smooth_pow(one_plus_theta, one_minus_gamma, epsilon);
    params.eta / (theta_gamma * second_factor)
}

/// Computes SSVI total variance for given log-moneyness and ATM total
/// variance.
///
/// ```text
/// w(k, θ) = θ/2 · (1 + ρ·φ(θ)·k + √((φ(θ)·k + ρ)² + (1 − ρ²)))
/// ```
pub fn ssvi_total_variance<T: Float>(
    params: &SsviParams<T>,
    log_moneyness: T,
    atm_total_var: T,
    epsilon: T,
) -> T {
    let one = T::one();
    let two: T = from_f64(2.0);
    let phi = ssvi_phi(params, atm_total_var, epsilon);

    let phi_k = phi * log_moneyness;
    let inner = (phi_k + params.rho) * (phi_k + params.rho) + (one - params.rho * params.rho);
    let sqrt_term = smooth_sqrt(inner, epsilon);

    (atm_total_var / two) * (one + params.rho * phi_k + sqrt_term)
}

/// Computes SSVI implied volatility.
///
/// Requires the ATM volatility at this expiry to compute `θ = σ²_ATM · T`.
pub fn ssvi_implied_vol<T: Float>(
    params: &SsviParams<T>,
    strike: T,
    forward: T,
    expiry: T,
    atm_vol: T,
) -> Result<T, SsviError> {
    let epsilon: T = from_f64(DEFAULT_EPSILON);

    if strike <= T::zero() {
        return Err(SsviError::InvalidStrike(
            strike.to_f64().unwrap_or(f64::NAN),
        ));
    }

    let theta = atm_vol * atm_vol * expiry;
    let log_moneyness = smooth_log(strike / forward, epsilon);
    let w = ssvi_total_variance(params, log_moneyness, theta, epsilon);
    let w_safe = smooth_max(w, epsilon, epsilon);

    if !w_safe.is_finite() {
        return Err(SsviError::NonFinite("total_variance".to_string()));
    }

    let vol = (w_safe / expiry).sqrt();

    if !vol.is_finite() {
        return Err(SsviError::NonFinite("implied_vol".to_string()));
    }

    Ok(vol)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn test_params() -> SsviParams<f64> {
        SsviParams {
            rho: -0.3,
            eta: 1.0,
            gamma: 0.5,
        }
    }

    #[test]
    fn test_params_validate_valid() {
        assert!(test_params().validate().is_ok());
    }

    #[test]
    fn test_params_validate_invalid_rho() {
        let mut p = test_params();
        p.rho = -1.0;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_params_validate_invalid_eta() {
        let mut p = test_params();
        p.eta = 0.0;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_params_validate_invalid_gamma() {
        let mut p = test_params();
        p.gamma = 1.5;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_phi_positive() {
        let p = test_params();
        let phi = ssvi_phi(&p, 0.04, 1e-10);
        assert!(phi > 0.0, "phi should be positive, got {}", phi);
    }

    #[test]
    fn test_atm_total_variance_equals_theta() {
        let p = test_params();
        let theta = 0.04; // sigma_atm^2 * T
        let w_atm = ssvi_total_variance(&p, 0.0, theta, 1e-10);
        // At k=0: w = θ/2 * (1 + √(ρ² + 1 - ρ²)) = θ/2 * (1 + 1) = θ
        assert_relative_eq!(w_atm, theta, epsilon = 1e-6);
    }

    #[test]
    fn test_implied_vol_atm_recovers_input() {
        let p = test_params();
        let atm_vol = 0.2;
        let vol = ssvi_implied_vol(&p, 100.0, 100.0, 1.0, atm_vol).unwrap();
        assert_relative_eq!(vol, atm_vol, epsilon = 1e-6);
    }

    #[test]
    fn test_negative_strike() {
        let p = test_params();
        assert!(ssvi_implied_vol(&p, -1.0, 100.0, 1.0, 0.2).is_err());
    }

    #[test]
    fn test_smile_shape() {
        let p = test_params();
        let vol_low = ssvi_implied_vol(&p, 90.0, 100.0, 1.0, 0.2).unwrap();
        let vol_atm = ssvi_implied_vol(&p, 100.0, 100.0, 1.0, 0.2).unwrap();
        let vol_high = ssvi_implied_vol(&p, 110.0, 100.0, 1.0, 0.2).unwrap();

        assert!(vol_low > vol_high, "Expected negative skew with rho < 0");
        assert!(vol_atm > 0.0);
    }

    #[test]
    fn test_f32_compatibility() {
        let p = SsviParams::<f32> {
            rho: -0.3,
            eta: 1.0,
            gamma: 0.5,
        };
        let vol = ssvi_implied_vol(&p, 100.0_f32, 100.0, 1.0, 0.2).unwrap();
        assert!(vol > 0.0);
    }
}
