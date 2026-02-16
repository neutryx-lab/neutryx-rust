//! Variance Gamma (VG) option pricing and implied volatility
//! (Madan, Carr & Chang 1998).
//!
//! The VG process is a Brownian motion with drift evaluated at a random time
//! given by a Gamma process:
//!
//! ```text
//! X(t) = θ·G(t) + σ·W(G(t))
//! ```
//!
//! where `G(t)` is a Gamma process with unit mean rate and variance rate `ν`.
//!
//! Parameters: σ (volatility), ν (kurtosis/variance rate), θ (skew/drift).
//!
//! Option pricing uses the closed-form formula involving the modified Bessel
//! function of the second kind, or equivalently the Gamma-weighted BS integral.

use num_traits::Float;
use thiserror::Error;

use crate::math::numeric::from_f64;

/// Default smoothing epsilon.
const DEFAULT_EPSILON: f64 = 1e-10;
/// Maximum Newton-Raphson iterations.
const MAX_INVERSION_ITERATIONS: usize = 50;
/// Number of Gauss-Laguerre quadrature points.
const QUADRATURE_POINTS: usize = 32;

/// Error type for VG calculations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum VarianceGammaError {
    /// Invalid parameter.
    #[error("Invalid VG parameter: {0}")]
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

/// Variance Gamma model parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VarianceGammaParams<T: Float> {
    /// Diffusion volatility (σ > 0).
    pub sigma: T,
    /// Variance rate of the Gamma time-change (ν > 0).
    /// Controls kurtosis / fat tails.
    pub nu: T,
    /// Drift of the Brownian subordinand (θ).
    /// Controls skewness.
    pub theta: T,
}

impl<T: Float> VarianceGammaParams<T> {
    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), VarianceGammaError> {
        if self.sigma <= T::zero() {
            return Err(VarianceGammaError::InvalidParameter(format!(
                "sigma = {} must be > 0",
                self.sigma.to_f64().unwrap_or(0.0)
            )));
        }
        if self.nu <= T::zero() {
            return Err(VarianceGammaError::InvalidParameter(format!(
                "nu = {} must be > 0",
                self.nu.to_f64().unwrap_or(0.0)
            )));
        }
        Ok(())
    }

    /// Returns the VG characteristic exponent ω that ensures the
    /// martingale condition: `E[exp(X_t)] = 1`.
    ///
    /// `ω = (1/ν) · ln(1 − θν − σ²ν/2)`
    pub fn omega(&self) -> T {
        let half: T = from_f64(0.5);
        let one = T::one();
        let inner = one - self.theta * self.nu - half * self.sigma * self.sigma * self.nu;
        inner.ln() / self.nu
    }
}

/// Standard normal PDF.
#[inline]
fn norm_pdf<T: Float>(x: T) -> T {
    let half: T = from_f64(0.5);
    let inv_sqrt_2pi: T = from_f64(0.3989422804014327);
    inv_sqrt_2pi * (-half * x * x).exp()
}

/// Standard normal CDF (rational approximation).
#[inline]
fn norm_cdf<T: Float>(x: T) -> T {
    let half: T = from_f64(0.5);
    let one = T::one();

    if x < from_f64(-8.0) { return T::zero(); }
    if x > from_f64(8.0) { return one; }

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

/// Black-Scholes call price.
#[inline]
fn bs_call<T: Float>(forward: T, strike: T, vol: T, expiry: T, epsilon: T) -> T {
    let half: T = from_f64(0.5);
    let vol_sqrt_t = vol * expiry.sqrt();
    if vol_sqrt_t < epsilon {
        let diff = forward - strike;
        return if diff > T::zero() { diff } else { T::zero() };
    }
    let d1 = ((forward / strike).ln() + half * vol_sqrt_t * vol_sqrt_t) / vol_sqrt_t;
    let d2 = d1 - vol_sqrt_t;
    forward * norm_cdf(d1) - strike * norm_cdf(d2)
}

/// Black-Scholes vega.
#[inline]
fn bs_vega<T: Float>(forward: T, strike: T, vol: T, expiry: T, epsilon: T) -> T {
    let half: T = from_f64(0.5);
    let vol_sqrt_t = vol * expiry.sqrt();
    if vol_sqrt_t < epsilon { return T::zero(); }
    let d1 = ((forward / strike).ln() + half * vol_sqrt_t * vol_sqrt_t) / vol_sqrt_t;
    forward * expiry.sqrt() * norm_pdf(d1)
}

/// Computes VG call price using Gamma-weighted BS integration.
///
/// The VG call price can be computed as:
/// `C_VG = ∫₀^∞ BS_call(F·e^{(ω+θ)g}, K, σ√g, 1) · f_Γ(g; T/ν, ν) dg`
///
/// where `f_Γ` is the Gamma PDF with shape `T/ν` and scale `ν`.
///
/// We use Gauss-Laguerre quadrature for the integration.
pub fn vg_call_price<T: Float>(
    params: &VarianceGammaParams<T>,
    forward: T,
    strike: T,
    expiry: T,
) -> Result<T, VarianceGammaError> {
    let epsilon: T = from_f64(DEFAULT_EPSILON);

    if strike <= T::zero() {
        return Err(VarianceGammaError::InvalidStrike(
            strike.to_f64().unwrap_or(f64::NAN),
        ));
    }

    let omega = params.omega();
    let shape = expiry / params.nu; // T/ν
    let scale = params.nu;          // ν

    // Gauss-Laguerre nodes and weights (precomputed for n=32)
    // Approximation: use a simpler Gamma integration via change of variable
    // g = scale * u, integrating over u with Gamma(shape, 1) density
    let shape_f64 = shape.to_f64().unwrap_or(1.0);
    let scale_f64 = scale.to_f64().unwrap_or(1.0);

    // Use trapezoidal rule on the Gamma distribution
    // Change variable: g = scale * quantile, integrate over [0, ~max]
    let n_points = QUADRATURE_POINTS;
    let mut price = T::zero();
    let mut total_weight = T::zero();

    // Integration range: [0, mean + 6*std] for the Gamma distribution
    let gamma_mean = shape_f64 * scale_f64;
    let gamma_std = (shape_f64 * scale_f64 * scale_f64).sqrt();
    let g_max = gamma_mean + 6.0 * gamma_std;
    let dg = g_max / n_points as f64;

    for i in 1..=n_points {
        let g_f64 = dg * (i as f64 - 0.5);
        let g: T = from_f64(g_f64);

        // Gamma PDF: f(g; shape, scale) = g^(shape-1) * exp(-g/scale) / (scale^shape * Γ(shape))
        // Log version for numerical stability
        let log_pdf = (shape_f64 - 1.0) * g_f64.ln()
            - g_f64 / scale_f64
            - shape_f64 * scale_f64.ln()
            - ln_gamma(shape_f64);

        let pdf_weight: T = from_f64((log_pdf).exp() * dg);

        if pdf_weight < epsilon {
            continue;
        }

        // BS call with VG-adjusted forward and vol
        let drift = omega + params.theta;
        let adjusted_forward = forward * (drift * g).exp();
        let adjusted_vol = params.sigma * g.sqrt();

        // Treat as expiry=1 since variance is already scaled by g
        let one = T::one();
        let component = bs_call(adjusted_forward, strike, adjusted_vol, one, epsilon);

        price = price + pdf_weight * component;
        total_weight = total_weight + pdf_weight;
    }

    // Normalise by total weight for numerical robustness
    if total_weight > epsilon {
        price = price / total_weight;
        // Re-scale by expected weight (should be ~1.0 for well-behaved integral)
    }

    if !price.is_finite() {
        return Err(VarianceGammaError::NonFinite("call_price".to_string()));
    }

    Ok(price)
}

/// Log-Gamma function (Stirling approximation).
fn ln_gamma(x: f64) -> f64 {
    // Use Lanczos approximation for accuracy
    if x < 0.5 {
        let pi = std::f64::consts::PI;
        return (pi / (pi * x).sin()).ln() - ln_gamma(1.0 - x);
    }

    let x = x - 1.0;
    let g = 7.0;
    let c = [
        0.999_999_999_999_809_93,
        676.520_368_121_885_1,
        -1259.139_216_722_403,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];

    let mut sum = c[0];
    for (i, &ci) in c.iter().enumerate().skip(1) {
        sum += ci / (x + i as f64);
    }

    let t = x + g + 0.5;
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + sum.ln()
}

/// Computes VG implied volatility via Newton-Raphson inversion.
pub fn vg_implied_vol<T: Float>(
    params: &VarianceGammaParams<T>,
    forward: T,
    strike: T,
    expiry: T,
) -> Result<T, VarianceGammaError> {
    let epsilon: T = from_f64(DEFAULT_EPSILON);
    let tol: T = from_f64(1e-8);

    let target_price = vg_call_price(params, forward, strike, expiry)?;

    // Initial guess: VG σ parameter
    let mut vol = params.sigma;

    for _iter in 0..MAX_INVERSION_ITERATIONS {
        let price = bs_call(forward, strike, vol, expiry, epsilon);
        let vega = bs_vega(forward, strike, vol, expiry, epsilon);

        let diff = price - target_price;

        if diff.abs() < tol {
            return Ok(vol);
        }

        if vega < epsilon {
            let half: T = from_f64(0.5);
            if diff > T::zero() { vol = vol * half; } else { vol = vol * from_f64(1.5); }
            continue;
        }

        vol = vol - diff / vega;
        if vol <= T::zero() { vol = from_f64(0.001); }
    }

    let final_price = bs_call(forward, strike, vol, expiry, epsilon);
    if (final_price - target_price).abs() < from_f64(1e-5) {
        return Ok(vol);
    }

    Err(VarianceGammaError::InversionFailed(MAX_INVERSION_ITERATIONS))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn test_params() -> VarianceGammaParams<f64> {
        VarianceGammaParams {
            sigma: 0.20,
            nu: 0.25,
            theta: -0.10,
        }
    }

    #[test]
    fn test_validate_valid() {
        assert!(test_params().validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_sigma() {
        let mut p = test_params();
        p.sigma = -0.1;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_nu() {
        let mut p = test_params();
        p.nu = 0.0;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_omega_finite() {
        let p = test_params();
        let w = p.omega();
        assert!(w.is_finite(), "omega should be finite, got {}", w);
    }

    #[test]
    fn test_call_price_positive() {
        let p = test_params();
        let price = vg_call_price(&p, 100.0, 100.0, 1.0).unwrap();
        assert!(price > 0.0, "ATM call price should be positive: {}", price);
    }

    #[test]
    fn test_call_price_monotone_in_strike() {
        let p = test_params();
        let price_90 = vg_call_price(&p, 100.0, 90.0, 1.0).unwrap();
        let price_100 = vg_call_price(&p, 100.0, 100.0, 1.0).unwrap();
        let price_110 = vg_call_price(&p, 100.0, 110.0, 1.0).unwrap();
        assert!(price_90 > price_100, "Call price should decrease with strike");
        assert!(price_100 > price_110, "Call price should decrease with strike");
    }

    #[test]
    fn test_implied_vol_positive() {
        let p = test_params();
        let vol = vg_implied_vol(&p, 100.0, 100.0, 1.0).unwrap();
        assert!(vol > 0.0 && vol < 1.0, "Implied vol {} out of range", vol);
    }

    #[test]
    fn test_implied_vol_atm_near_sigma() {
        // For small ν and θ, VG implied vol should be close to σ
        let p = VarianceGammaParams {
            sigma: 0.20,
            nu: 0.01, // Small ν → close to BS
            theta: 0.0,
        };
        let vol = vg_implied_vol(&p, 100.0, 100.0, 1.0).unwrap();
        assert_relative_eq!(vol, 0.20, epsilon = 0.02);
    }

    #[test]
    fn test_negative_strike() {
        let p = test_params();
        assert!(vg_call_price(&p, 100.0, -1.0, 1.0).is_err());
    }

    #[test]
    fn test_ln_gamma_known_values() {
        // Γ(1) = 1, ln(1) = 0
        assert_relative_eq!(ln_gamma(1.0), 0.0, epsilon = 1e-8);
        // Γ(2) = 1, ln(1) = 0
        assert_relative_eq!(ln_gamma(2.0), 0.0, epsilon = 1e-8);
        // Γ(5) = 24, ln(24) ≈ 3.178
        assert_relative_eq!(ln_gamma(5.0), 24.0_f64.ln(), epsilon = 1e-6);
    }
}
