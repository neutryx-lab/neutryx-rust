//! SVI (Stochastic Volatility Inspired) implied volatility formula.
//!
//! Gatheral (2004) parametrisation of the implied variance smile:
//!
//! ```text
//! w(k) = a + b * (ρ(k − m) + √((k − m)² + σ²))
//! ```
//!
//! where `k = ln(K/F)` is the log-moneyness and `w = σ²_BS T` is the total
//! implied variance.

use num_traits::Float;
use thiserror::Error;

use crate::math::{
    numeric::from_f64,
    smoothing::{smooth_log, smooth_max, smooth_sqrt},
};

/// Default smoothing epsilon.
const DEFAULT_EPSILON: f64 = 1e-10;

/// Error type for SVI calculations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SviError {
    /// Invalid parameter value.
    #[error("Invalid SVI parameter: {0}")]
    InvalidParameter(String),
    /// Negative total variance computed.
    #[error("Negative total variance at log-moneyness {0}")]
    NegativeTotalVariance(f64),
    /// Invalid strike.
    #[error("Invalid strike: K = {0} (must be positive)")]
    InvalidStrike(f64),
    /// Non-finite result.
    #[error("Non-finite result in {0}")]
    NonFinite(String),
}

/// SVI raw parametrisation (Gatheral 2004).
///
/// The five parameters map log-moneyness to total implied variance:
/// `w(k) = a + b * (ρ(k − m) + √((k − m)² + σ²))`
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SviParams<T: Float> {
    /// Vertical displacement (overall variance level).
    pub a: T,
    /// Slope magnitude (b ≥ 0).
    pub b: T,
    /// Rotation / skew (−1 < ρ < 1).
    pub rho: T,
    /// Horizontal translation (minimum variance location).
    pub m: T,
    /// ATM curvature (σ > 0).
    pub sigma: T,
}

impl<T: Float> SviParams<T> {
    /// Validates the SVI parameters.
    pub fn validate(&self) -> Result<(), SviError> {
        if self.b < T::zero() {
            return Err(SviError::InvalidParameter(format!(
                "b = {} must be >= 0",
                self.b.to_f64().unwrap_or(0.0)
            )));
        }
        if self.rho <= -T::one() || self.rho >= T::one() {
            return Err(SviError::InvalidParameter(format!(
                "rho = {} must be in (-1, 1)",
                self.rho.to_f64().unwrap_or(0.0)
            )));
        }
        if self.sigma <= T::zero() {
            return Err(SviError::InvalidParameter(format!(
                "sigma = {} must be > 0",
                self.sigma.to_f64().unwrap_or(0.0)
            )));
        }
        Ok(())
    }
}

/// Computes SVI total variance `w(k)` for a given log-moneyness.
///
/// `w(k) = a + b * (ρ(k − m) + √((k − m)² + σ²))`
#[inline]
pub fn svi_total_variance<T: Float>(params: &SviParams<T>, log_moneyness: T, epsilon: T) -> T {
    let k_m = log_moneyness - params.m;
    let inner = k_m * k_m + params.sigma * params.sigma;
    let sqrt_term = smooth_sqrt(inner, epsilon);
    params.a + params.b * (params.rho * k_m + sqrt_term)
}

/// Computes SVI implied volatility from raw parameters.
///
/// Returns `σ_BS = √(w(k) / T)` where `w` is the SVI total variance.
pub fn svi_implied_vol<T: Float>(
    params: &SviParams<T>,
    strike: T,
    forward: T,
    expiry: T,
) -> Result<T, SviError> {
    let epsilon: T = from_f64(DEFAULT_EPSILON);

    if strike <= T::zero() {
        return Err(SviError::InvalidStrike(strike.to_f64().unwrap_or(f64::NAN)));
    }

    let log_moneyness = smooth_log(strike / forward, epsilon);
    let w = svi_total_variance(params, log_moneyness, epsilon);

    // Ensure non-negative total variance
    let w_safe = smooth_max(w, epsilon, epsilon);

    if !w_safe.is_finite() {
        return Err(SviError::NonFinite("total_variance".to_string()));
    }

    let vol = (w_safe / expiry).sqrt();

    if !vol.is_finite() {
        return Err(SviError::NonFinite("implied_vol".to_string()));
    }

    Ok(vol)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn test_params() -> SviParams<f64> {
        SviParams {
            a: 0.04,
            b: 0.4,
            rho: -0.4,
            m: 0.0,
            sigma: 0.1,
        }
    }

    #[test]
    fn test_params_validate_valid() {
        assert!(test_params().validate().is_ok());
    }

    #[test]
    fn test_params_validate_negative_b() {
        let mut p = test_params();
        p.b = -0.1;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_params_validate_invalid_rho() {
        let mut p = test_params();
        p.rho = 1.0;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_params_validate_negative_sigma() {
        let mut p = test_params();
        p.sigma = -0.01;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_total_variance_at_money() {
        let p = test_params();
        let w = svi_total_variance(&p, 0.0, 1e-10);
        // At k=0: w = a + b * (ρ*0 + √(0 + σ²)) = a + b*σ = 0.04 + 0.4*0.1 = 0.08
        assert_relative_eq!(w, 0.08, epsilon = 1e-6);
    }

    #[test]
    fn test_implied_vol_atm() {
        let p = test_params();
        let vol = svi_implied_vol(&p, 100.0, 100.0, 1.0).unwrap();
        // w(0) = 0.08, vol = sqrt(0.08/1.0) ≈ 0.2828
        assert_relative_eq!(vol, 0.08_f64.sqrt(), epsilon = 1e-4);
    }

    #[test]
    fn test_implied_vol_negative_strike() {
        let p = test_params();
        assert!(svi_implied_vol(&p, -1.0, 100.0, 1.0).is_err());
    }

    #[test]
    fn test_smile_shape_negative_rho() {
        let p = test_params();
        let vol_low = svi_implied_vol(&p, 90.0, 100.0, 1.0).unwrap();
        let vol_atm = svi_implied_vol(&p, 100.0, 100.0, 1.0).unwrap();
        let vol_high = svi_implied_vol(&p, 110.0, 100.0, 1.0).unwrap();

        // Negative rho → left skew (low strikes have higher vol)
        assert!(
            vol_low > vol_high,
            "Expected negative skew: {} > {}",
            vol_low,
            vol_high
        );
        assert!(vol_atm > 0.0);
    }

    #[test]
    fn test_f32_compatibility() {
        let p = SviParams::<f32> {
            a: 0.04,
            b: 0.4,
            rho: -0.4,
            m: 0.0,
            sigma: 0.1,
        };
        let vol = svi_implied_vol(&p, 100.0_f32, 100.0, 1.0).unwrap();
        assert!(vol > 0.0);
    }
}
