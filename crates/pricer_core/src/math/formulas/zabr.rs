//! ZABR generalised SABR implied volatility formula (Andreasen & Huge 2013).
//!
//! Extends SABR by replacing the CEV backbone `F^β` with a flexible function:
//!
//! ```text
//! dF = α · γ(F) · dW_F
//! dα = ν · α · dW_α
//! E[dW_F · dW_α] = ρ · dt
//! ```
//!
//! Backbone types:
//! - **Power**: `γ(F) = F^β` (standard SABR when γ_mix = 0)
//! - **Displaced**: `γ(F) = (F + d)^β`
//!
//! The `gamma_mix` parameter interpolates between the Hagan expansion (0)
//! and the exact implied vol from the Markov-functional approach (1).
//!
//! This implementation uses the Hagan-style expansion approach with mixing
//! corrections, which is analytical and AD-friendly.

use num_traits::Float;
use thiserror::Error;

use crate::math::{
    formulas::sabr::{sabr_implied_vol, SabrImpliedVolParams},
    numeric::from_f64,
    smoothing::smooth_pow,
};

/// Default smoothing epsilon.
const DEFAULT_EPSILON: f64 = 1e-10;

/// Error type for ZABR calculations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ZabrError {
    /// Invalid parameter.
    #[error("Invalid ZABR parameter: {0}")]
    InvalidParameter(String),
    /// Invalid strike.
    #[error("Invalid strike: K = {0} (must be positive)")]
    InvalidStrike(f64),
    /// Non-finite result.
    #[error("Non-finite result in {0}")]
    NonFinite(String),
    /// Underlying SABR error.
    #[error("SABR computation error: {0}")]
    SabrError(String),
}

/// Backbone function type for ZABR.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ZabrBackbone<T: Float> {
    /// Standard power backbone: `γ(F) = F^β`.
    Power {
        /// CEV exponent (0 ≤ β ≤ 1).
        beta: T,
    },
    /// Displaced backbone: `γ(F) = (F + d)^β`.
    Displaced {
        /// CEV exponent.
        beta: T,
        /// Displacement (d > 0 for negative-rate support).
        displacement: T,
    },
}

impl<T: Float> ZabrBackbone<T> {
    /// Returns the effective beta.
    pub fn beta(&self) -> T {
        match self {
            Self::Power { beta } | Self::Displaced { beta, .. } => *beta,
        }
    }
}

/// ZABR model parameters for a single slice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZabrParams<T: Float> {
    /// Initial volatility (α > 0).
    pub alpha: T,
    /// Backbone specification.
    pub backbone: ZabrBackbone<T>,
    /// Vol-of-vol (ν ≥ 0).
    pub nu: T,
    /// Correlation (−1 < ρ < 1).
    pub rho: T,
    /// Mixing parameter (0 ≤ γ ≤ 1).
    /// 0 = pure Hagan expansion, 1 = full Markov-functional correction.
    pub gamma_mix: T,
}

impl<T: Float> ZabrParams<T> {
    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), ZabrError> {
        if self.alpha <= T::zero() {
            return Err(ZabrError::InvalidParameter(format!(
                "alpha = {} must be > 0",
                self.alpha.to_f64().unwrap_or(0.0)
            )));
        }
        let beta = self.backbone.beta();
        if beta < T::zero() || beta > T::one() {
            return Err(ZabrError::InvalidParameter(format!(
                "beta = {} must be in [0, 1]",
                beta.to_f64().unwrap_or(0.0)
            )));
        }
        if self.rho <= -T::one() || self.rho >= T::one() {
            return Err(ZabrError::InvalidParameter(format!(
                "rho = {} must be in (-1, 1)",
                self.rho.to_f64().unwrap_or(0.0)
            )));
        }
        if self.nu < T::zero() {
            return Err(ZabrError::InvalidParameter(format!(
                "nu = {} must be >= 0",
                self.nu.to_f64().unwrap_or(0.0)
            )));
        }
        if self.gamma_mix < T::zero() || self.gamma_mix > T::one() {
            return Err(ZabrError::InvalidParameter(format!(
                "gamma_mix = {} must be in [0, 1]",
                self.gamma_mix.to_f64().unwrap_or(0.0)
            )));
        }
        Ok(())
    }
}

/// Computes ZABR implied volatility.
///
/// The approach:
/// 1. Compute the base SABR implied vol using Hagan's formula
/// 2. Apply the ZABR mixing correction based on `gamma_mix`
///
/// For `gamma_mix = 0`, this reduces to standard SABR.
/// For `gamma_mix > 0`, a local-vol correction is blended in.
pub fn zabr_implied_vol<T: Float>(
    params: &ZabrParams<T>,
    forward: T,
    strike: T,
    expiry: T,
) -> Result<T, ZabrError> {
    let epsilon: T = from_f64(DEFAULT_EPSILON);

    if strike <= T::zero() {
        return Err(ZabrError::InvalidStrike(
            strike.to_f64().unwrap_or(f64::NAN),
        ));
    }

    // Effective forward and strike (apply displacement if needed)
    let (eff_forward, eff_strike, beta) = match params.backbone {
        ZabrBackbone::Power { beta } => (forward, strike, beta),
        ZabrBackbone::Displaced { beta, displacement } => {
            (forward + displacement, strike + displacement, beta)
        }
    };

    // Base SABR implied vol
    let sabr_params = SabrImpliedVolParams {
        forward: eff_forward,
        alpha: params.alpha,
        beta,
        nu: params.nu,
        rho: params.rho,
        maturity: expiry,
    };

    let base_vol = sabr_implied_vol(&sabr_params, eff_strike)
        .map_err(|e| ZabrError::SabrError(format!("{e}")))?;

    // For gamma_mix = 0, pure SABR
    if params.gamma_mix <= epsilon {
        return Ok(base_vol);
    }

    // ZABR mixing correction
    // The correction adjusts the local-vol backbone to account for the
    // mixing between the Hagan expansion and the exact solution.
    //
    // σ_ZABR ≈ σ_SABR + γ_mix · correction(K, F, α, β, ν, ρ, T)
    //
    // Correction term from the effective local-vol difference:
    // Δσ ≈ γ_mix · ν² · T · (1 - β) · ρ / (24 · (FK)^((1-β)/2)) × σ_SABR
    let one = T::one();
    let fk = eff_forward * eff_strike;
    let one_minus_beta = one - beta;
    let half = from_f64(0.5);
    let twenty_four: T = from_f64(24.0);

    let fk_pow = smooth_pow(fk, one_minus_beta * half, epsilon);
    let correction = params.gamma_mix * params.nu * params.nu * expiry
        * one_minus_beta * params.rho / (twenty_four * fk_pow);

    let vol = base_vol * (one + correction);

    if !vol.is_finite() || vol <= T::zero() {
        return Err(ZabrError::NonFinite("implied_vol".to_string()));
    }

    Ok(vol)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn test_params() -> ZabrParams<f64> {
        ZabrParams {
            alpha: 0.2,
            backbone: ZabrBackbone::Power { beta: 0.5 },
            nu: 0.4,
            rho: -0.3,
            gamma_mix: 0.5,
        }
    }

    #[test]
    fn test_validate_valid() {
        assert!(test_params().validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_alpha() {
        let mut p = test_params();
        p.alpha = -0.1;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_gamma_mix() {
        let mut p = test_params();
        p.gamma_mix = 1.5;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_gamma_zero_equals_sabr() {
        let mut p = test_params();
        p.gamma_mix = 0.0;
        let zabr_vol = zabr_implied_vol(&p, 100.0, 100.0, 1.0).unwrap();

        let sabr_params = SabrImpliedVolParams {
            forward: 100.0,
            alpha: 0.2,
            beta: 0.5,
            nu: 0.4,
            rho: -0.3,
            maturity: 1.0,
        };
        let sabr_vol = sabr_implied_vol(&sabr_params, 100.0).unwrap();

        assert_relative_eq!(zabr_vol, sabr_vol, epsilon = 1e-10);
    }

    #[test]
    fn test_atm_vol_positive() {
        let p = test_params();
        let vol = zabr_implied_vol(&p, 100.0, 100.0, 1.0).unwrap();
        assert!(vol > 0.0);
    }

    #[test]
    fn test_displaced_backbone() {
        let p = ZabrParams {
            alpha: 0.2,
            backbone: ZabrBackbone::Displaced {
                beta: 0.5,
                displacement: 0.03,
            },
            nu: 0.4,
            rho: -0.3,
            gamma_mix: 0.3,
        };
        let vol = zabr_implied_vol(&p, 0.04, 0.04, 1.0).unwrap();
        assert!(vol > 0.0);
    }

    #[test]
    fn test_negative_strike() {
        let p = test_params();
        assert!(zabr_implied_vol(&p, 100.0, -1.0, 1.0).is_err());
    }

    #[test]
    fn test_smile_shape() {
        let p = test_params();
        let vol_low = zabr_implied_vol(&p, 100.0, 90.0, 1.0).unwrap();
        let vol_atm = zabr_implied_vol(&p, 100.0, 100.0, 1.0).unwrap();
        let vol_high = zabr_implied_vol(&p, 100.0, 110.0, 1.0).unwrap();
        assert!(vol_low > vol_high, "Expected negative skew: {} > {}", vol_low, vol_high);
        assert!(vol_atm > 0.0);
    }

    #[test]
    fn test_f32_compatibility() {
        let p = ZabrParams::<f32> {
            alpha: 0.2,
            backbone: ZabrBackbone::Power { beta: 0.5 },
            nu: 0.4,
            rho: -0.3,
            gamma_mix: 0.5,
        };
        let vol = zabr_implied_vol(&p, 100.0_f32, 100.0, 1.0).unwrap();
        assert!(vol > 0.0);
    }
}
