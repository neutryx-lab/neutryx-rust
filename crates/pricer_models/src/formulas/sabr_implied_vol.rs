//! SABR Hagan implied volatility formula.
//!
//! This module provides the Hagan et al. (2002) approximation formula for
//! computing Black-Scholes implied volatility from SABR model parameters.
//!
//! This is a standalone implementation of the implied volatility formula,
//! separate from the full SABR stochastic model implementation.
//!
//! # Mathematical Background
//!
//! The SABR model is defined by the following SDEs:
//! ```text
//! dF = alpha * F^beta * dW_F
//! d(alpha) = nu * alpha * dW_alpha
//! E[dW_F * dW_alpha] = rho * dt
//! ```
//!
//! The Hagan formula approximates the Black-Scholes implied volatility as:
//! ```text
//! σ_B(K,F) = α / [(FK)^((1-β)/2) * D(F/K)]
//!            × (z/x(z))
//!            × [1 + expansion_terms * T]
//! ```
//!
//! where:
//! - D(F/K) = 1 + ((1-β)²/24)*ln²(F/K) + ((1-β)⁴/1920)*ln⁴(F/K)
//! - z = (ν/α)*(FK)^((1-β)/2)*ln(F/K)
//! - x(z) = ln((√(1-2ρz+z²)+z-ρ)/(1-ρ))
//!
//! # Examples
//!
//! ```
//! use pricer_models::formulas::sabr_implied_vol::{sabr_implied_vol, SabrImpliedVolParams};
//!
//! let params = SabrImpliedVolParams {
//!     forward: 100.0,
//!     alpha: 0.2,
//!     beta: 0.5,
//!     nu: 0.4,
//!     rho: -0.3,
//!     maturity: 1.0,
//! };
//!
//! // Compute implied volatility at ATM
//! let iv_atm = sabr_implied_vol(&params, 100.0).unwrap();
//! assert!(iv_atm > 0.0);
//!
//! // Compute implied volatility OTM
//! let iv_otm = sabr_implied_vol(&params, 110.0).unwrap();
//! assert!(iv_otm > 0.0);
//! ```

use num_traits::Float;
use pricer_core::math::{
    numeric::from_f64,
    smoothing::{smooth_log, smooth_pow},
};
use thiserror::Error;

/// Default ATM threshold for determining when to use the ATM expansion formula.
pub const DEFAULT_ATM_THRESHOLD: f64 = 0.01;

/// Default smoothing epsilon for numerical stability.
pub const DEFAULT_SMOOTHING_EPSILON: f64 = 1e-10;

/// Error type for SABR implied volatility calculations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SabrImpliedVolError {
    /// Invalid forward price (must be positive).
    #[error("Invalid forward price: F = {0} (must be positive)")]
    InvalidForward(f64),

    /// Invalid alpha (must be positive).
    #[error("Invalid alpha: α = {0} (must be positive)")]
    InvalidAlpha(f64),

    /// Invalid nu (must be non-negative).
    #[error("Invalid nu: ν = {0} (must be non-negative)")]
    InvalidNu(f64),

    /// Invalid beta (must be in [0, 1]).
    #[error("Invalid beta: β = {0} (must be in [0, 1])")]
    InvalidBeta(f64),

    /// Invalid rho (must be in (-1, 1)).
    #[error("Invalid rho: ρ = {0} (must be in (-1, 1))")]
    InvalidRho(f64),

    /// Invalid maturity (must be positive).
    #[error("Invalid maturity: T = {0} (must be positive)")]
    InvalidMaturity(f64),

    /// Invalid strike (must be positive).
    #[error("Invalid strike: K = {0} (must be positive)")]
    InvalidStrike(f64),

    /// Negative implied volatility computed.
    #[error("Negative implied volatility computed at strike {0}")]
    NegativeImpliedVol(f64),

    /// Non-finite result (NaN or Infinity).
    #[error("Non-finite result in {0}")]
    NonFinite(String),
}

/// Parameters for SABR implied volatility calculation.
///
/// # Fields
///
/// * `forward` - Forward price (F > 0)
/// * `alpha` - Initial volatility (α > 0)
/// * `beta` - CEV exponent (0 ≤ β ≤ 1)
/// * `nu` - Vol-of-vol (ν ≥ 0)
/// * `rho` - Correlation (-1 < ρ < 1)
/// * `maturity` - Time to maturity (T > 0)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SabrImpliedVolParams<T: Float> {
    /// Forward price.
    pub forward: T,
    /// Initial volatility (alpha).
    pub alpha: T,
    /// CEV exponent (beta): 0 = Normal, 1 = Lognormal.
    pub beta: T,
    /// Vol-of-vol (nu).
    pub nu: T,
    /// Correlation (rho).
    pub rho: T,
    /// Time to maturity.
    pub maturity: T,
}

impl<T: Float> SabrImpliedVolParams<T> {
    /// Creates new SABR implied volatility parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if any parameter is invalid.
    pub fn new(
        forward: T,
        alpha: T,
        beta: T,
        nu: T,
        rho: T,
        maturity: T,
    ) -> Result<Self, SabrImpliedVolError> {
        let params = Self {
            forward,
            alpha,
            beta,
            nu,
            rho,
            maturity,
        };
        params.validate()?;
        Ok(params)
    }

    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), SabrImpliedVolError> {
        if self.forward <= T::zero() {
            return Err(SabrImpliedVolError::InvalidForward(
                self.forward.to_f64().unwrap_or(0.0),
            ));
        }
        if self.alpha <= T::zero() {
            return Err(SabrImpliedVolError::InvalidAlpha(
                self.alpha.to_f64().unwrap_or(0.0),
            ));
        }
        if self.beta < T::zero() || self.beta > T::one() {
            return Err(SabrImpliedVolError::InvalidBeta(
                self.beta.to_f64().unwrap_or(0.0),
            ));
        }
        if self.nu < T::zero() {
            return Err(SabrImpliedVolError::InvalidNu(
                self.nu.to_f64().unwrap_or(0.0),
            ));
        }
        if self.rho <= -T::one() || self.rho >= T::one() {
            return Err(SabrImpliedVolError::InvalidRho(
                self.rho.to_f64().unwrap_or(0.0),
            ));
        }
        if self.maturity <= T::zero() {
            return Err(SabrImpliedVolError::InvalidMaturity(
                self.maturity.to_f64().unwrap_or(0.0),
            ));
        }
        Ok(())
    }

    /// Returns true if beta is approximately 0 (Normal SABR).
    #[inline]
    pub fn is_normal(&self) -> bool {
        let eps: T = from_f64(DEFAULT_SMOOTHING_EPSILON);
        self.beta.abs() < eps
    }

    /// Returns true if beta is approximately 1 (Lognormal SABR).
    #[inline]
    pub fn is_lognormal(&self) -> bool {
        let eps: T = from_f64(DEFAULT_SMOOTHING_EPSILON);
        (self.beta - T::one()).abs() < eps
    }
}

/// Computes SABR implied volatility using the Hagan et al. (2002) formula.
///
/// # Arguments
///
/// * `params` - SABR parameters
/// * `strike` - Strike price (must be positive)
///
/// # Returns
///
/// Black-Scholes implied volatility.
///
/// # Errors
///
/// - `InvalidStrike`: Strike is not positive
/// - `NegativeImpliedVol`: Computed volatility is negative
/// - `NonFinite`: NaN or Infinity detected
///
/// # Examples
///
/// ```
/// use pricer_models::formulas::sabr_implied_vol::{sabr_implied_vol, SabrImpliedVolParams};
///
/// let params = SabrImpliedVolParams {
///     forward: 100.0,
///     alpha: 0.2,
///     beta: 0.5,
///     nu: 0.4,
///     rho: -0.3,
///     maturity: 1.0,
/// };
///
/// let iv = sabr_implied_vol(&params, 100.0).unwrap();
/// assert!(iv > 0.0);
/// ```
pub fn sabr_implied_vol<T: Float>(
    params: &SabrImpliedVolParams<T>,
    strike: T,
) -> Result<T, SabrImpliedVolError> {
    sabr_implied_vol_with_options(
        params,
        strike,
        from_f64(DEFAULT_ATM_THRESHOLD),
        from_f64(DEFAULT_SMOOTHING_EPSILON),
    )
}

/// Computes SABR implied volatility with custom options.
///
/// # Arguments
///
/// * `params` - SABR parameters
/// * `strike` - Strike price (must be positive)
/// * `atm_threshold` - Threshold for ATM expansion formula
/// * `epsilon` - Smoothing epsilon for numerical stability
///
/// # Returns
///
/// Black-Scholes implied volatility.
pub fn sabr_implied_vol_with_options<T: Float>(
    params: &SabrImpliedVolParams<T>,
    strike: T,
    atm_threshold: T,
    epsilon: T,
) -> Result<T, SabrImpliedVolError> {
    // Validate strike
    if strike <= T::zero() {
        return Err(SabrImpliedVolError::InvalidStrike(
            strike.to_f64().unwrap_or(f64::NAN),
        ));
    }

    let log_fk = smooth_log(params.forward / strike, epsilon);

    // Select appropriate formula based on moneyness and beta
    let vol = if log_fk.abs() < atm_threshold {
        implied_vol_atm_expansion(params, strike, epsilon)
    } else if params.is_normal() {
        implied_vol_normal(params, strike, epsilon)
    } else if params.is_lognormal() {
        implied_vol_lognormal(params, strike, epsilon)
    } else {
        implied_vol_hagan(params, strike, epsilon)
    };

    // Validate result
    if !vol.is_finite() {
        return Err(SabrImpliedVolError::NonFinite("implied_vol".to_string()));
    }

    if vol <= T::zero() {
        return Err(SabrImpliedVolError::NegativeImpliedVol(
            strike.to_f64().unwrap_or(f64::NAN),
        ));
    }

    Ok(vol)
}

/// Computes SABR implied volatility with a floor.
///
/// If the computed volatility is below the floor, returns the floor value instead.
///
/// # Arguments
///
/// * `params` - SABR parameters
/// * `strike` - Strike price
/// * `floor` - Minimum volatility floor
pub fn sabr_implied_vol_with_floor<T: Float>(
    params: &SabrImpliedVolParams<T>,
    strike: T,
    floor: T,
) -> Result<T, SabrImpliedVolError> {
    let epsilon: T = from_f64(DEFAULT_SMOOTHING_EPSILON);
    let atm_threshold: T = from_f64(DEFAULT_ATM_THRESHOLD);

    // Validate strike
    if strike <= T::zero() {
        return Err(SabrImpliedVolError::InvalidStrike(
            strike.to_f64().unwrap_or(f64::NAN),
        ));
    }

    let log_fk = smooth_log(params.forward / strike, epsilon);

    let vol = if log_fk.abs() < atm_threshold {
        implied_vol_atm_expansion(params, strike, epsilon)
    } else if params.is_normal() {
        implied_vol_normal(params, strike, epsilon)
    } else if params.is_lognormal() {
        implied_vol_lognormal(params, strike, epsilon)
    } else {
        implied_vol_hagan(params, strike, epsilon)
    };

    // Validate result
    if !vol.is_finite() {
        return Err(SabrImpliedVolError::NonFinite("implied_vol".to_string()));
    }

    // Apply floor
    let floored_vol = if vol < floor { floor } else { vol };

    Ok(floored_vol)
}

/// Computes ATM implied volatility.
///
/// At ATM, the Hagan formula simplifies and is more numerically stable.
pub fn sabr_atm_vol<T: Float>(params: &SabrImpliedVolParams<T>) -> T {
    let epsilon: T = from_f64(DEFAULT_SMOOTHING_EPSILON);
    implied_vol_atm_expansion(params, params.forward, epsilon)
}

/// ATM expansion formula for implied volatility.
///
/// Used when K ≈ F to avoid numerical instability.
fn implied_vol_atm_expansion<T: Float>(
    params: &SabrImpliedVolParams<T>,
    strike: T,
    epsilon: T,
) -> T {
    let alpha = params.alpha;
    let nu = params.nu;
    let rho = params.rho;
    let t = params.maturity;

    let one = T::one();
    let two: T = from_f64(2.0);
    let three: T = from_f64(3.0);
    let four: T = from_f64(4.0);
    let twenty_four: T = from_f64(24.0);

    // Normal SABR (beta=0)
    if params.is_normal() {
        let term3 = (two - three * rho * rho) / twenty_four * nu * nu;
        let expansion = one + term3 * t;
        return alpha * expansion;
    }

    // Lognormal SABR (beta=1)
    if params.is_lognormal() {
        let term2 = rho * nu * alpha / four;
        let term3 = (two - three * rho * rho) / twenty_four * nu * nu;
        let expansion = one + (term2 + term3) * t;
        return alpha * expansion;
    }

    // General case (0 < beta < 1)
    let f = params.forward;
    let beta = params.beta;
    let one_minus_beta = one - beta;

    // Geometric mean (F*K)^((1-β)/2)
    let fk = f * strike;
    let fk_pow = smooth_pow(fk, one_minus_beta / two, epsilon);

    // Base term: α / (FK)^((1-β)/2)
    let base = alpha / fk_pow;

    // Higher order expansion terms
    let fk_pow_full = smooth_pow(fk, one_minus_beta, epsilon);
    let term1 = one_minus_beta * one_minus_beta / twenty_four * alpha * alpha / fk_pow_full;
    let term2 = rho * beta * nu * alpha / (four * fk_pow);
    let term3 = (two - three * rho * rho) / twenty_four * nu * nu;

    let expansion = one + (term1 + term2 + term3) * t;

    base * expansion
}

/// General Hagan formula for implied volatility.
fn implied_vol_hagan<T: Float>(params: &SabrImpliedVolParams<T>, strike: T, epsilon: T) -> T {
    let f = params.forward;
    let k = strike;
    let alpha = params.alpha;
    let beta = params.beta;
    let nu = params.nu;
    let rho = params.rho;
    let t = params.maturity;

    let one = T::one();
    let two: T = from_f64(2.0);
    let three: T = from_f64(3.0);
    let four: T = from_f64(4.0);
    let twenty_four: T = from_f64(24.0);
    let one_thousand_nine_twenty: T = from_f64(1920.0);

    let one_minus_beta = one - beta;

    // ln(F/K)
    let log_fk = smooth_log(f / k, epsilon);

    // (FK)^((1-β)/2)
    let fk = f * k;
    let fk_pow_half = smooth_pow(fk, one_minus_beta / two, epsilon);

    // D(F/K) = 1 + ((1-β)²/24)*ln²(F/K) + ((1-β)⁴/1920)*ln⁴(F/K)
    let log_fk_2 = log_fk * log_fk;
    let log_fk_4 = log_fk_2 * log_fk_2;
    let one_minus_beta_2 = one_minus_beta * one_minus_beta;
    let one_minus_beta_4 = one_minus_beta_2 * one_minus_beta_2;

    let d_term1 = one_minus_beta_2 / twenty_four * log_fk_2;
    let d_term2 = one_minus_beta_4 / one_thousand_nine_twenty * log_fk_4;
    let d = one + d_term1 + d_term2;

    // z = (ν/α) * (FK)^((1-β)/2) * ln(F/K)
    let z = (nu / alpha) * fk_pow_half * log_fk;

    // x(z) = ln((√(1-2ρz+z²) + z - ρ) / (1-ρ))
    let x_z = compute_x_of_z(z, rho, epsilon);

    // z/x(z) coefficient
    let z_over_x = if z.abs() < epsilon { one } else { z / x_z };

    // Base term: α / [(FK)^((1-β)/2) * D]
    let base = alpha / (fk_pow_half * d);

    // Higher order expansion terms
    let fk_pow_full = smooth_pow(fk, one_minus_beta, epsilon);
    let term1 = one_minus_beta_2 / twenty_four * alpha * alpha / fk_pow_full;
    let term2 = rho * beta * nu * alpha / (four * fk_pow_half);
    let term3 = (two - three * rho * rho) / twenty_four * nu * nu;

    let expansion = one + (term1 + term2 + term3) * t;

    base * z_over_x * expansion
}

/// Normal SABR (beta=0) implied volatility formula.
fn implied_vol_normal<T: Float>(params: &SabrImpliedVolParams<T>, strike: T, epsilon: T) -> T {
    let f = params.forward;
    let k = strike;
    let alpha = params.alpha;
    let nu = params.nu;
    let rho = params.rho;
    let t = params.maturity;

    let one = T::one();
    let two: T = from_f64(2.0);
    let three: T = from_f64(3.0);
    let four: T = from_f64(4.0);
    let twenty_four: T = from_f64(24.0);

    // z = ν/α * (F - K) for Normal SABR
    let f_minus_k = f - k;
    let z = if alpha > epsilon {
        (nu / alpha) * f_minus_k
    } else {
        T::zero()
    };

    // x(z) calculation
    let x_z = compute_x_of_z(z, rho, epsilon);

    // z/x(z) coefficient
    let z_over_x = if z.abs() < epsilon { one } else { z / x_z };

    // Base term: α (for Normal SABR)
    let base = alpha;

    // Higher order expansion terms
    let term1 = T::zero();
    let avg_f = (f + k) / two;
    let term2 = if avg_f > epsilon {
        rho * nu * alpha / (four * avg_f)
    } else {
        T::zero()
    };
    let term3 = (two - three * rho * rho) / twenty_four * nu * nu;

    let expansion = one + (term1 + term2 + term3) * t;

    base * z_over_x * expansion
}

/// Lognormal SABR (beta=1) implied volatility formula.
fn implied_vol_lognormal<T: Float>(params: &SabrImpliedVolParams<T>, strike: T, epsilon: T) -> T {
    let f = params.forward;
    let k = strike;
    let alpha = params.alpha;
    let nu = params.nu;
    let rho = params.rho;
    let t = params.maturity;

    let one = T::one();
    let two: T = from_f64(2.0);
    let three: T = from_f64(3.0);
    let four: T = from_f64(4.0);
    let twenty_four: T = from_f64(24.0);

    // ln(F/K)
    let log_fk = smooth_log(f / k, epsilon);

    // z = ν/α * ln(F/K) for Lognormal SABR
    let z = if alpha > epsilon {
        (nu / alpha) * log_fk
    } else {
        T::zero()
    };

    // x(z) calculation
    let x_z = compute_x_of_z(z, rho, epsilon);

    // z/x(z) coefficient
    let z_over_x = if z.abs() < epsilon { one } else { z / x_z };

    // Base term: α (for Lognormal SABR)
    let base = alpha;

    // Higher order expansion terms
    let term2 = rho * nu * alpha / four;
    let term3 = (two - three * rho * rho) / twenty_four * nu * nu;

    let expansion = one + (term2 + term3) * t;

    base * z_over_x * expansion
}

/// Computes the x(z) function from the Hagan formula.
///
/// x(z) = ln((√(1-2ρz+z²) + z - ρ) / (1-ρ))
fn compute_x_of_z<T: Float>(z: T, rho: T, epsilon: T) -> T {
    let one = T::one();
    let two: T = from_f64(2.0);

    // √(1 - 2ρz + z²)
    let discriminant = one - two * rho * z + z * z;
    let safe_discriminant = if discriminant < epsilon * epsilon {
        epsilon * epsilon
    } else {
        discriminant
    };
    let sqrt_disc = safe_discriminant.sqrt();

    // numerator = √(1-2ρz+z²) + z - ρ
    let numerator = sqrt_disc + z - rho;

    // denominator = 1 - ρ
    let denominator = one - rho;

    // x(z) = ln(numerator / denominator)
    smooth_log(numerator / denominator, epsilon)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn create_test_params() -> SabrImpliedVolParams<f64> {
        SabrImpliedVolParams {
            forward: 100.0,
            alpha: 0.2,
            beta: 0.5,
            nu: 0.4,
            rho: -0.3,
            maturity: 1.0,
        }
    }

    #[test]
    fn test_params_new_valid() {
        let params = SabrImpliedVolParams::new(100.0, 0.2, 0.5, 0.4, -0.3, 1.0);
        assert!(params.is_ok());
    }

    #[test]
    fn test_params_invalid_forward() {
        let params = SabrImpliedVolParams::new(0.0, 0.2, 0.5, 0.4, -0.3, 1.0);
        assert!(matches!(params, Err(SabrImpliedVolError::InvalidForward(_))));
    }

    #[test]
    fn test_params_invalid_alpha() {
        let params = SabrImpliedVolParams::new(100.0, -0.1, 0.5, 0.4, -0.3, 1.0);
        assert!(matches!(params, Err(SabrImpliedVolError::InvalidAlpha(_))));
    }

    #[test]
    fn test_params_invalid_beta() {
        let params = SabrImpliedVolParams::new(100.0, 0.2, 1.5, 0.4, -0.3, 1.0);
        assert!(matches!(params, Err(SabrImpliedVolError::InvalidBeta(_))));
    }

    #[test]
    fn test_params_invalid_rho() {
        let params = SabrImpliedVolParams::new(100.0, 0.2, 0.5, 0.4, 1.0, 1.0);
        assert!(matches!(params, Err(SabrImpliedVolError::InvalidRho(_))));
    }

    #[test]
    fn test_sabr_implied_vol_atm() {
        let params = create_test_params();
        let iv = sabr_implied_vol(&params, 100.0).unwrap();
        assert!(iv > 0.0);
        assert!(iv < 1.0); // Reasonable range
    }

    #[test]
    fn test_sabr_implied_vol_otm_call() {
        let params = create_test_params();
        let iv = sabr_implied_vol(&params, 110.0).unwrap();
        assert!(iv > 0.0);
    }

    #[test]
    fn test_sabr_implied_vol_otm_put() {
        let params = create_test_params();
        let iv = sabr_implied_vol(&params, 90.0).unwrap();
        assert!(iv > 0.0);
    }

    #[test]
    fn test_sabr_implied_vol_invalid_strike() {
        let params = create_test_params();
        let result = sabr_implied_vol(&params, 0.0);
        assert!(matches!(result, Err(SabrImpliedVolError::InvalidStrike(_))));
    }

    #[test]
    fn test_sabr_atm_vol() {
        let params = create_test_params();
        let atm_vol = sabr_atm_vol(&params);
        let iv_atm = sabr_implied_vol(&params, 100.0).unwrap();
        assert_relative_eq!(atm_vol, iv_atm, epsilon = 1e-6);
    }

    #[test]
    fn test_sabr_implied_vol_with_floor() {
        let params = create_test_params();
        let floor = 0.01;
        let iv = sabr_implied_vol_with_floor(&params, 100.0, floor).unwrap();
        assert!(iv >= floor);
    }

    #[test]
    fn test_normal_sabr() {
        let params = SabrImpliedVolParams {
            forward: 0.03,
            alpha: 0.01,
            beta: 0.0, // Normal SABR
            nu: 0.3,
            rho: -0.2,
            maturity: 1.0,
        };
        let iv = sabr_implied_vol(&params, 0.03).unwrap();
        assert!(iv > 0.0);
    }

    #[test]
    fn test_lognormal_sabr() {
        let params = SabrImpliedVolParams {
            forward: 100.0,
            alpha: 0.2,
            beta: 1.0, // Lognormal SABR
            nu: 0.4,
            rho: -0.3,
            maturity: 1.0,
        };
        let iv = sabr_implied_vol(&params, 100.0).unwrap();
        assert!(iv > 0.0);
    }

    #[test]
    fn test_smile_negative_rho() {
        // With negative rho, OTM puts should have higher IV than OTM calls
        // Use more extreme parameters to demonstrate skew effect
        let params = SabrImpliedVolParams {
            forward: 100.0,
            alpha: 0.2,
            beta: 0.9, // Closer to lognormal for clearer skew
            nu: 0.4,
            rho: -0.5,
            maturity: 1.0,
        };

        let iv_90 = sabr_implied_vol(&params, 90.0).unwrap();
        let iv_100 = sabr_implied_vol(&params, 100.0).unwrap();
        let iv_110 = sabr_implied_vol(&params, 110.0).unwrap();

        // With negative rho, OTM puts (K < F) should have higher vol
        // than OTM calls (K > F), demonstrating negative skew
        assert!(iv_90 > iv_110, "Expected negative skew: iv_90={} > iv_110={}", iv_90, iv_110);

        // All implied vols should be positive and reasonable
        assert!(iv_90 > 0.0);
        assert!(iv_100 > 0.0);
        assert!(iv_110 > 0.0);
    }

    #[test]
    fn test_f32_compatibility() {
        let params = SabrImpliedVolParams {
            forward: 100.0_f32,
            alpha: 0.2_f32,
            beta: 0.5_f32,
            nu: 0.4_f32,
            rho: -0.3_f32,
            maturity: 1.0_f32,
        };
        let iv = sabr_implied_vol(&params, 100.0_f32).unwrap();
        assert!(iv > 0.0_f32);
    }
}
