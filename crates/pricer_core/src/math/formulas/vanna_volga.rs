//! Vanna-Volga implied volatility interpolation (Castagna & Mercurio 2007).
//!
//! Constructs the FX volatility smile from three market pillar quotes:
//! 25-delta put, ATM, 25-delta call.
//!
//! ```text
//! σ_VV(K) = σ_ATM + x₁(K)·(σ₁ − σ_ATM) + x₃(K)·(σ₃ − σ_ATM)
//! ```
//!
//! The weights `x₁`, `x₃` are log-moneyness ratios that reproduce
//! the three pillar volatilities exactly.

use num_traits::Float;
use thiserror::Error;

use crate::math::{numeric::from_f64, smoothing::smooth_log};

/// Default smoothing epsilon.
const DEFAULT_EPSILON: f64 = 1e-10;

/// Error type for Vanna-Volga calculations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum VannaVolgaError {
    /// Invalid parameter.
    #[error("Invalid Vanna-Volga parameter: {0}")]
    InvalidParameter(String),
    /// Invalid strike.
    #[error("Invalid strike: K = {0} (must be positive)")]
    InvalidStrike(f64),
    /// Non-finite result.
    #[error("Non-finite result in {0}")]
    NonFinite(String),
}

/// Vanna-Volga parameters for a single expiry slice.
///
/// Defined by three pillar volatilities and their corresponding strikes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VannaVolgaParams<T: Float> {
    /// ATM volatility.
    pub sigma_atm: T,
    /// 25-delta put volatility.
    pub sigma_25d_put: T,
    /// 25-delta call volatility.
    pub sigma_25d_call: T,
    /// ATM strike.
    pub strike_atm: T,
    /// 25-delta put strike.
    pub strike_25d_put: T,
    /// 25-delta call strike.
    pub strike_25d_call: T,
}

impl<T: Float> VannaVolgaParams<T> {
    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), VannaVolgaError> {
        if self.sigma_atm <= T::zero() {
            return Err(VannaVolgaError::InvalidParameter(
                "sigma_atm must be positive".to_string(),
            ));
        }
        if self.sigma_25d_put <= T::zero() || self.sigma_25d_call <= T::zero() {
            return Err(VannaVolgaError::InvalidParameter(
                "pillar volatilities must be positive".to_string(),
            ));
        }
        if self.strike_atm <= T::zero()
            || self.strike_25d_put <= T::zero()
            || self.strike_25d_call <= T::zero()
        {
            return Err(VannaVolgaError::InvalidParameter(
                "pillar strikes must be positive".to_string(),
            ));
        }
        Ok(())
    }
}

/// Computes Vanna-Volga implied volatility at an arbitrary strike.
///
/// Uses the first-generation Vanna-Volga method with log-moneyness weights:
///
/// ```text
/// x₁(K) = ln(K₂/K)·ln(K₃/K) / [ln(K₂/K₁)·ln(K₃/K₁)]
/// x₃(K) = ln(K/K₁)·ln(K/K₂) / [ln(K₃/K₁)·ln(K₃/K₂)]
/// σ(K) = σ_ATM + x₁·(σ₁ − σ_ATM) + x₃·(σ₃ − σ_ATM)
/// ```
pub fn vanna_volga_implied_vol<T: Float>(
    params: &VannaVolgaParams<T>,
    strike: T,
) -> Result<T, VannaVolgaError> {
    let epsilon: T = from_f64(DEFAULT_EPSILON);

    if strike <= T::zero() {
        return Err(VannaVolgaError::InvalidStrike(
            strike.to_f64().unwrap_or(f64::NAN),
        ));
    }

    let k1 = params.strike_25d_put;
    let k2 = params.strike_atm;
    let k3 = params.strike_25d_call;

    // Log ratios
    let ln_k2_k = smooth_log(k2 / strike, epsilon);
    let ln_k3_k = smooth_log(k3 / strike, epsilon);
    let ln_k_k1 = smooth_log(strike / k1, epsilon);
    let ln_k_k2 = smooth_log(strike / k2, epsilon);
    let ln_k2_k1 = smooth_log(k2 / k1, epsilon);
    let ln_k3_k1 = smooth_log(k3 / k1, epsilon);
    let ln_k3_k2 = smooth_log(k3 / k2, epsilon);

    // Denominators (protected against division by zero)
    let denom1 = ln_k2_k1 * ln_k3_k1;
    let denom3 = ln_k3_k1 * ln_k3_k2;

    let safe_denom1 = if denom1.abs() < epsilon {
        epsilon
    } else {
        denom1
    };
    let safe_denom3 = if denom3.abs() < epsilon {
        epsilon
    } else {
        denom3
    };

    // Weights
    let x1 = (ln_k2_k * ln_k3_k) / safe_denom1;
    let x3 = (ln_k_k1 * ln_k_k2) / safe_denom3;

    // Vanna-Volga implied vol
    let vol = params.sigma_atm
        + x1 * (params.sigma_25d_put - params.sigma_atm)
        + x3 * (params.sigma_25d_call - params.sigma_atm);

    if !vol.is_finite() {
        return Err(VannaVolgaError::NonFinite("implied_vol".to_string()));
    }

    Ok(vol)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn test_params() -> VannaVolgaParams<f64> {
        VannaVolgaParams {
            sigma_atm: 0.10,
            sigma_25d_put: 0.12,
            sigma_25d_call: 0.09,
            strike_atm: 1.10,
            strike_25d_put: 1.05,
            strike_25d_call: 1.15,
        }
    }

    #[test]
    fn test_reproduces_atm() {
        let p = test_params();
        let vol = vanna_volga_implied_vol(&p, p.strike_atm).unwrap();
        assert_relative_eq!(vol, p.sigma_atm, epsilon = 1e-6);
    }

    #[test]
    fn test_reproduces_25d_put() {
        let p = test_params();
        let vol = vanna_volga_implied_vol(&p, p.strike_25d_put).unwrap();
        assert_relative_eq!(vol, p.sigma_25d_put, epsilon = 1e-6);
    }

    #[test]
    fn test_reproduces_25d_call() {
        let p = test_params();
        let vol = vanna_volga_implied_vol(&p, p.strike_25d_call).unwrap();
        assert_relative_eq!(vol, p.sigma_25d_call, epsilon = 1e-6);
    }

    #[test]
    fn test_interpolation_between_pillars() {
        let p = test_params();
        let vol = vanna_volga_implied_vol(&p, 1.08).unwrap();
        assert!(vol > 0.0 && vol < 0.20);
    }

    #[test]
    fn test_negative_strike() {
        let p = test_params();
        assert!(vanna_volga_implied_vol(&p, -1.0).is_err());
    }

    #[test]
    fn test_validate_negative_vol() {
        let mut p = test_params();
        p.sigma_atm = -0.01;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_f32_compatibility() {
        let p = VannaVolgaParams::<f32> {
            sigma_atm: 0.10,
            sigma_25d_put: 0.12,
            sigma_25d_call: 0.09,
            strike_atm: 1.10,
            strike_25d_put: 1.05,
            strike_25d_call: 1.15,
        };
        let vol = vanna_volga_implied_vol(&p, 1.10_f32).unwrap();
        assert!(vol > 0.0);
    }
}
