//! Polynomial total-variance smile parametrisation.
//!
//! Fits total implied variance as a polynomial in log-moneyness:
//!
//! ```text
//! w(k) = c₀ + c₁k + c₂k² + … + cₙkⁿ
//! ```
//!
//! where `k = ln(K/F)` and `w = σ²T`.
//!
//! Simple baseline for IR vol surfaces. No arbitrage guarantee.

use num_traits::Float;
use thiserror::Error;

use crate::math::{
    numeric::from_f64,
    smoothing::{smooth_log, smooth_max},
};

/// Default smoothing epsilon.
const DEFAULT_EPSILON: f64 = 1e-10;

/// Error type for polynomial vol calculations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum PolynomialVolError {
    /// No coefficients provided.
    #[error("Empty coefficients")]
    EmptyCoefficients,
    /// Invalid strike.
    #[error("Invalid strike: K = {0} (must be positive)")]
    InvalidStrike(f64),
    /// Non-finite result.
    #[error("Non-finite result in {0}")]
    NonFinite(String),
}

/// Polynomial volatility parameters.
///
/// Coefficients `[c₀, c₁, …, cₙ]` define the total variance polynomial.
#[derive(Clone, Debug, PartialEq)]
pub struct PolynomialVolParams<T: Float> {
    /// Polynomial coefficients in ascending power order.
    pub coefficients: Vec<T>,
}

impl<T: Float> PolynomialVolParams<T> {
    /// Creates new polynomial parameters.
    pub fn new(coefficients: Vec<T>) -> Result<Self, PolynomialVolError> {
        if coefficients.is_empty() {
            return Err(PolynomialVolError::EmptyCoefficients);
        }
        Ok(Self { coefficients })
    }

    /// Returns the polynomial degree.
    pub fn degree(&self) -> usize { self.coefficients.len().saturating_sub(1) }
}

/// Evaluates the polynomial total variance at log-moneyness `k`.
///
/// Uses Horner's method for numerical stability.
#[inline]
pub fn polynomial_total_variance<T: Float>(params: &PolynomialVolParams<T>, log_moneyness: T) -> T {
    let mut result = T::zero();
    for c in params.coefficients.iter().rev() {
        result = result * log_moneyness + *c;
    }
    result
}

/// Computes polynomial implied volatility.
///
/// Returns `σ_BS = √(max(w(k), ε) / T)`.
pub fn polynomial_implied_vol<T: Float>(
    params: &PolynomialVolParams<T>,
    strike: T,
    forward: T,
    expiry: T,
) -> Result<T, PolynomialVolError> {
    let epsilon: T = from_f64(DEFAULT_EPSILON);

    if strike <= T::zero() {
        return Err(PolynomialVolError::InvalidStrike(
            strike.to_f64().unwrap_or(f64::NAN),
        ));
    }

    let log_moneyness = smooth_log(strike / forward, epsilon);
    let w = polynomial_total_variance(params, log_moneyness);
    let w_safe = smooth_max(w, epsilon, epsilon);

    if !w_safe.is_finite() {
        return Err(PolynomialVolError::NonFinite("total_variance".to_string()));
    }

    let vol = (w_safe / expiry).sqrt();

    if !vol.is_finite() {
        return Err(PolynomialVolError::NonFinite("implied_vol".to_string()));
    }

    Ok(vol)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_constant_polynomial() {
        // w(k) = 0.04 → σ = √(0.04/1) = 0.2
        let p = PolynomialVolParams::new(vec![0.04]).unwrap();
        let vol = polynomial_implied_vol(&p, 100.0, 100.0, 1.0).unwrap();
        assert_relative_eq!(vol, 0.2, epsilon = 1e-6);
    }

    #[test]
    fn test_quadratic_smile() {
        // w(k) = 0.04 + 0.1*k² → smile (minimum at ATM)
        let p = PolynomialVolParams::new(vec![0.04, 0.0, 0.1]).unwrap();
        let vol_atm = polynomial_implied_vol(&p, 100.0, 100.0, 1.0).unwrap();
        let vol_otm = polynomial_implied_vol(&p, 110.0, 100.0, 1.0).unwrap();
        assert!(vol_otm > vol_atm, "Quadratic should increase away from ATM");
    }

    #[test]
    fn test_empty_coefficients() {
        assert!(PolynomialVolParams::<f64>::new(vec![]).is_err());
    }

    #[test]
    fn test_negative_strike() {
        let p = PolynomialVolParams::new(vec![0.04]).unwrap();
        assert!(polynomial_implied_vol(&p, -1.0, 100.0, 1.0).is_err());
    }

    #[test]
    fn test_degree() {
        let p = PolynomialVolParams::new(vec![1.0, 2.0, 3.0]).unwrap();
        assert_eq!(p.degree(), 2);
    }

    #[test]
    fn test_f32_compatibility() {
        let p = PolynomialVolParams::new(vec![0.04_f32]).unwrap();
        let vol = polynomial_implied_vol(&p, 100.0_f32, 100.0, 1.0).unwrap();
        assert!(vol > 0.0);
    }
}
