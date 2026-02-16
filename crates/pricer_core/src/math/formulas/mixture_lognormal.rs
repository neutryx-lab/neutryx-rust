//! Mixture of Lognormals implied volatility formula.
//!
//! The call price under a mixture of `N` lognormals is:
//!
//! ```text
//! C_mix(K, T) = Σ wᵢ · BS_call(Fᵢ, K, σᵢ, T)
//! ```
//!
//! with martingale constraint `Σ wᵢ Fᵢ = F`.
//!
//! Implied volatility is recovered by inverting the mixture price through
//! Black-Scholes via Newton-Raphson.
//!
//! Typical use case: FX bimodal distributions (N = 2).

use num_traits::Float;
use thiserror::Error;

use crate::math::numeric::from_f64;

/// Default smoothing epsilon.
const DEFAULT_EPSILON: f64 = 1e-10;
/// Maximum Newton-Raphson iterations for implied vol inversion.
const MAX_INVERSION_ITERATIONS: usize = 50;

/// Error type for mixture lognormal calculations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MixtureLnError {
    /// Invalid parameters.
    #[error("Invalid mixture parameter: {0}")]
    InvalidParameter(String),
    /// Invalid strike.
    #[error("Invalid strike: K = {0} (must be positive)")]
    InvalidStrike(f64),
    /// Inversion failed.
    #[error("Implied vol inversion failed after {0} iterations")]
    InversionFailed(usize),
    /// Non-finite result.
    #[error("Non-finite result in {0}")]
    NonFinite(String),
}

/// Mixture of lognormals parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct MixtureLognormalParams<T: Float> {
    /// Mixture weights (wᵢ > 0, sum = 1).
    pub weights: Vec<T>,
    /// Component forward rates (Fᵢ > 0).
    pub forwards: Vec<T>,
    /// Component volatilities (σᵢ > 0).
    pub volatilities: Vec<T>,
}

impl<T: Float> MixtureLognormalParams<T> {
    /// Creates new mixture parameters.
    pub fn new(
        weights: Vec<T>,
        forwards: Vec<T>,
        volatilities: Vec<T>,
    ) -> Result<Self, MixtureLnError> {
        let n = weights.len();
        if n == 0 {
            return Err(MixtureLnError::InvalidParameter(
                "need at least one component".to_string(),
            ));
        }
        if forwards.len() != n || volatilities.len() != n {
            return Err(MixtureLnError::InvalidParameter(
                "weights, forwards, and volatilities must have equal length".to_string(),
            ));
        }
        Ok(Self {
            weights,
            forwards,
            volatilities,
        })
    }

    /// Returns the number of mixture components.
    pub fn num_components(&self) -> usize { self.weights.len() }

    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), MixtureLnError> {
        let eps: T = from_f64(1e-6);
        let mut sum = T::zero();
        for &w in &self.weights {
            if w < T::zero() {
                return Err(MixtureLnError::InvalidParameter(
                    "weights must be non-negative".to_string(),
                ));
            }
            sum = sum + w;
        }
        if (sum - T::one()).abs() > eps {
            return Err(MixtureLnError::InvalidParameter(format!(
                "weights must sum to 1, got {}",
                sum.to_f64().unwrap_or(0.0)
            )));
        }
        for &f in &self.forwards {
            if f <= T::zero() {
                return Err(MixtureLnError::InvalidParameter(
                    "forwards must be positive".to_string(),
                ));
            }
        }
        for &v in &self.volatilities {
            if v <= T::zero() {
                return Err(MixtureLnError::InvalidParameter(
                    "volatilities must be positive".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Standard normal CDF approximation (Abramowitz & Stegun).
#[inline]
fn norm_cdf<T: Float>(x: T) -> T {
    let one = T::one();
    let half: T = from_f64(0.5);
    // erfc-based: Φ(x) = erfc(-x/√2) / 2
    // Using the tanh approximation for broad compatibility:
    let sqrt2: T = from_f64(std::f64::consts::SQRT_2);
    half * (one + (x / sqrt2 * from_f64(std::f64::consts::FRAC_2_SQRT_PI) * from_f64(0.5)).tanh()
        .max(-one)
        .min(one))
}

/// More accurate normal CDF via rational approximation (Hart 1968).
#[inline]
fn norm_cdf_precise<T: Float>(x: T) -> T {
    let half: T = from_f64(0.5);
    let one = T::one();

    if x < from_f64(-8.0) {
        return T::zero();
    }
    if x > from_f64(8.0) {
        return one;
    }

    // Horner-style rational approximation
    let a1: T = from_f64(0.254829592);
    let a2: T = from_f64(-0.284496736);
    let a3: T = from_f64(1.421413741);
    let a4: T = from_f64(-1.453152027);
    let a5: T = from_f64(1.061405429);
    let p: T = from_f64(0.3275911);

    let sign = if x < T::zero() { -one } else { one };
    let abs_x = x.abs();
    let t = one / (one + p * abs_x);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    let y = one - (a1 * t + a2 * t2 + a3 * t3 + a4 * t4 + a5 * t5)
        * (-abs_x * abs_x * half).exp();

    half * (one + sign * y)
}

/// Black-Scholes call price for a single lognormal component.
#[inline]
fn bs_call_price<T: Float>(forward: T, strike: T, vol: T, expiry: T, epsilon: T) -> T {
    let half: T = from_f64(0.5);
    let vol_sqrt_t = vol * expiry.sqrt();

    if vol_sqrt_t < epsilon {
        // Intrinsic value
        let diff = forward - strike;
        return if diff > T::zero() { diff } else { T::zero() };
    }

    let d1 = ((forward / strike).ln() + half * vol_sqrt_t * vol_sqrt_t) / vol_sqrt_t;
    let d2 = d1 - vol_sqrt_t;

    forward * norm_cdf_precise(d1) - strike * norm_cdf_precise(d2)
}

/// Black-Scholes vega (dC/dσ).
#[inline]
fn bs_vega<T: Float>(forward: T, strike: T, vol: T, expiry: T, epsilon: T) -> T {
    let half: T = from_f64(0.5);
    let vol_sqrt_t = vol * expiry.sqrt();

    if vol_sqrt_t < epsilon {
        return T::zero();
    }

    let d1 = ((forward / strike).ln() + half * vol_sqrt_t * vol_sqrt_t) / vol_sqrt_t;

    // vega = F √T · n(d1) where n is the standard normal PDF
    let inv_sqrt_2pi: T = from_f64(0.3989422804014327); // 1/√(2π)
    let pdf_d1 = inv_sqrt_2pi * (-half * d1 * d1).exp();

    forward * expiry.sqrt() * pdf_d1
}

/// Computes the mixture-lognormal call price.
///
/// `C_mix = Σ wᵢ · BS_call(Fᵢ, K, σᵢ, T)`
pub fn mixture_lognormal_call_price<T: Float>(
    params: &MixtureLognormalParams<T>,
    strike: T,
    expiry: T,
) -> Result<T, MixtureLnError> {
    let epsilon: T = from_f64(DEFAULT_EPSILON);

    if strike <= T::zero() {
        return Err(MixtureLnError::InvalidStrike(
            strike.to_f64().unwrap_or(f64::NAN),
        ));
    }

    let mut price = T::zero();
    for i in 0..params.num_components() {
        let component = bs_call_price(
            params.forwards[i],
            strike,
            params.volatilities[i],
            expiry,
            epsilon,
        );
        price = price + params.weights[i] * component;
    }

    Ok(price)
}

/// Computes mixture-lognormal implied volatility via Newton-Raphson inversion.
///
/// Inverts the mixture call price through Black-Scholes to extract the
/// corresponding implied volatility.
pub fn mixture_lognormal_implied_vol<T: Float>(
    params: &MixtureLognormalParams<T>,
    strike: T,
    forward: T,
    expiry: T,
) -> Result<T, MixtureLnError> {
    let epsilon: T = from_f64(DEFAULT_EPSILON);
    let tol: T = from_f64(1e-10);

    let target_price = mixture_lognormal_call_price(params, strike, expiry)?;

    // Initial guess: weighted average vol
    let mut vol = T::zero();
    for i in 0..params.num_components() {
        vol = vol + params.weights[i] * params.volatilities[i];
    }

    // Newton-Raphson
    for iter in 0..MAX_INVERSION_ITERATIONS {
        let price = bs_call_price(forward, strike, vol, expiry, epsilon);
        let vega = bs_vega(forward, strike, vol, expiry, epsilon);

        let diff = price - target_price;

        if diff.abs() < tol {
            return Ok(vol);
        }

        if vega < epsilon {
            // Vega too small — try bisection step
            let half: T = from_f64(0.5);
            if diff > T::zero() {
                vol = vol * half;
            } else {
                vol = vol * from_f64(1.5);
            }
            continue;
        }

        let update = diff / vega;
        vol = vol - update;

        // Keep vol positive
        if vol <= T::zero() {
            vol = from_f64(0.001);
        }

        if iter > MAX_INVERSION_ITERATIONS / 2 && update.abs() < tol {
            return Ok(vol);
        }
    }

    // Check if converged enough
    let final_price = bs_call_price(forward, strike, vol, expiry, epsilon);
    if (final_price - target_price).abs() < from_f64(1e-6) {
        return Ok(vol);
    }

    Err(MixtureLnError::InversionFailed(MAX_INVERSION_ITERATIONS))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn bimodal_params() -> MixtureLognormalParams<f64> {
        MixtureLognormalParams::new(
            vec![0.6, 0.4],
            vec![100.0, 100.0],
            vec![0.15, 0.30],
        )
        .unwrap()
    }

    #[test]
    fn test_single_component_matches_bs() {
        let p = MixtureLognormalParams::new(
            vec![1.0],
            vec![100.0],
            vec![0.20],
        )
        .unwrap();
        let price = mixture_lognormal_call_price(&p, 100.0, 1.0).unwrap();
        let bs_price = bs_call_price(100.0, 100.0, 0.20, 1.0, 1e-10);
        assert_relative_eq!(price, bs_price, epsilon = 1e-8);
    }

    #[test]
    fn test_single_component_implied_vol() {
        let p = MixtureLognormalParams::new(
            vec![1.0],
            vec![100.0],
            vec![0.20],
        )
        .unwrap();
        let vol = mixture_lognormal_implied_vol(&p, 100.0, 100.0, 1.0).unwrap();
        assert_relative_eq!(vol, 0.20, epsilon = 1e-4);
    }

    #[test]
    fn test_bimodal_call_price_positive() {
        let p = bimodal_params();
        let price = mixture_lognormal_call_price(&p, 100.0, 1.0).unwrap();
        assert!(price > 0.0);
    }

    #[test]
    fn test_bimodal_implied_vol() {
        let p = bimodal_params();
        let vol = mixture_lognormal_implied_vol(&p, 100.0, 100.0, 1.0).unwrap();
        // Should be between the two component vols
        assert!(vol > 0.10 && vol < 0.35, "Vol {} out of expected range", vol);
    }

    #[test]
    fn test_negative_strike() {
        let p = bimodal_params();
        assert!(mixture_lognormal_call_price(&p, -1.0, 1.0).is_err());
    }

    #[test]
    fn test_validate_bad_weights() {
        let p = MixtureLognormalParams::new(
            vec![0.3, 0.3],
            vec![100.0, 100.0],
            vec![0.2, 0.3],
        )
        .unwrap();
        assert!(p.validate().is_err()); // weights sum to 0.6
    }

    #[test]
    fn test_f32_compatibility() {
        let p = MixtureLognormalParams::new(
            vec![1.0_f32],
            vec![100.0],
            vec![0.20],
        )
        .unwrap();
        let vol = mixture_lognormal_implied_vol(&p, 100.0_f32, 100.0, 1.0).unwrap();
        assert!(vol > 0.0);
    }
}
