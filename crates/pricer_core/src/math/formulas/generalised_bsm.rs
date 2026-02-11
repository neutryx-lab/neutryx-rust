//! Generalised Black-Scholes-Merton model with cost-of-carry parameter.
//!
//! Provides the unified pricing engine for European options under lognormal
//! dynamics. All closed-form models (Black-Scholes, Merton, Garman-Kohlhagen,
//! Black-76) are special cases with different cost-of-carry `b`.
//!
//! ## Special Cases
//!
//! | Model | Cost-of-carry `b` |
//! |---|---|
//! | Black-Scholes (no dividend) | `b = r` |
//! | Merton (continuous dividend yield `q`) | `b = r - q` |
//! | Garman-Kohlhagen (FX) | `b = rd - rf` |
//! | Black-76 (futures) | `b = 0` |
//!
//! ## Formulas
//!
//! - d₁ = \[ln(S/K) + (b + σ²/2)T\] / (σ√T)
//! - d₂ = d₁ - σ√T
//! - Call = S·exp((b-r)T)·N(d₁) - K·exp(-rT)·N(d₂)
//! - Put  = K·exp(-rT)·N(-d₂) - S·exp((b-r)T)·N(-d₁)

use num_traits::Float;

use super::error::FormulaError;
use crate::math::{
    normal_dist::{norm_cdf, norm_pdf},
    numeric::from_f64,
};

/// Generalised Black-Scholes-Merton model.
///
/// Pre-computes d₁, d₂, and discount factors for efficient multi-Greek
/// evaluation.
///
/// # Type Parameters
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Examples
/// ```
/// use pricer_core::math::formulas::GeneralisedBSM;
///
/// // Black-Scholes case: b = r
/// let bsm = GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, 0.2, 1.0).unwrap();
/// let call = bsm.price(true);
/// let put = bsm.price(false);
///
/// // Put-call parity: C - P = S·exp((b-r)T) - K·exp(-rT)
/// let parity = call - put - (100.0 - 100.0 * (-0.05_f64).exp());
/// assert!(parity.abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct GeneralisedBSM<T: Float> {
    spot: T,
    strike: T,
    rate: T,
    cost_of_carry: T,
    volatility: T,
    expiry: T,
    d1: T,
    d2: T,
    sqrt_t: T,
    /// exp(-rT)
    df: T,
    /// exp((b-r)T)
    carry_df: T,
}

impl<T: Float> GeneralisedBSM<T> {
    /// Creates a new GeneralisedBSM model.
    ///
    /// # Arguments
    /// * `spot` - Spot price (must be positive)
    /// * `strike` - Strike price (must be positive)
    /// * `rate` - Risk-free rate (r)
    /// * `cost_of_carry` - Cost-of-carry parameter (b)
    /// * `volatility` - Volatility (must be positive)
    /// * `expiry` - Time to expiry in years (must be positive)
    ///
    /// # Errors
    /// Returns `FormulaError` if spot, strike, volatility, or expiry is
    /// non-positive.
    pub fn new(
        spot: T,
        strike: T,
        rate: T,
        cost_of_carry: T,
        volatility: T,
        expiry: T,
    ) -> Result<Self, FormulaError> {
        let zero = T::zero();

        if spot <= zero {
            return Err(FormulaError::InvalidSpot {
                spot: spot.to_f64().unwrap_or(0.0),
            });
        }
        if strike <= zero {
            return Err(FormulaError::InvalidSpot {
                spot: strike.to_f64().unwrap_or(0.0),
            });
        }
        if volatility <= zero {
            return Err(FormulaError::InvalidVolatility {
                volatility: volatility.to_f64().unwrap_or(0.0),
            });
        }
        if expiry <= zero {
            return Err(FormulaError::InvalidExpiry {
                expiry: expiry.to_f64().unwrap_or(0.0),
            });
        }

        let two: T = from_f64(2.0);
        let sqrt_t = expiry.sqrt();
        let vol_sqrt_t = volatility * sqrt_t;

        let log_sk = (spot / strike).ln();
        let drift = cost_of_carry + volatility * volatility / two;
        let d1 = (log_sk + drift * expiry) / vol_sqrt_t;
        let d2 = d1 - vol_sqrt_t;

        let df = (-rate * expiry).exp();
        let carry_df = ((cost_of_carry - rate) * expiry).exp();

        Ok(Self {
            spot,
            strike,
            rate,
            cost_of_carry,
            volatility,
            expiry,
            d1,
            d2,
            sqrt_t,
            df,
            carry_df,
        })
    }

    /// Returns d₁.
    #[inline]
    pub fn d1(&self) -> T { self.d1 }

    /// Returns d₂.
    #[inline]
    pub fn d2(&self) -> T { self.d2 }

    /// Returns the spot price.
    #[inline]
    pub fn spot(&self) -> T { self.spot }

    /// Returns the risk-free rate.
    #[inline]
    pub fn rate(&self) -> T { self.rate }

    /// Returns the volatility.
    #[inline]
    pub fn volatility(&self) -> T { self.volatility }

    /// Returns exp(-rT).
    #[inline]
    pub fn discount_factor(&self) -> T { self.df }

    /// Returns exp((b-r)T).
    #[inline]
    pub fn carry_discount_factor(&self) -> T { self.carry_df }

    /// Computes option price.
    ///
    /// - Call = S·exp((b-r)T)·N(d₁) - K·exp(-rT)·N(d₂)
    /// - Put  = K·exp(-rT)·N(-d₂) - S·exp((b-r)T)·N(-d₁)
    #[inline]
    pub fn price(&self, is_call: bool) -> T {
        if is_call {
            self.spot * self.carry_df * norm_cdf(self.d1)
                - self.strike * self.df * norm_cdf(self.d2)
        } else {
            self.strike * self.df * norm_cdf(-self.d2)
                - self.spot * self.carry_df * norm_cdf(-self.d1)
        }
    }

    /// Computes Delta (∂V/∂S).
    ///
    /// - Call Δ = exp((b-r)T)·N(d₁)
    /// - Put Δ  = exp((b-r)T)·(N(d₁) - 1)
    #[inline]
    pub fn delta(&self, is_call: bool) -> T {
        let nd1 = norm_cdf(self.d1);
        if is_call {
            self.carry_df * nd1
        } else {
            self.carry_df * (nd1 - T::one())
        }
    }

    /// Computes Gamma (∂²V/∂S²). Same for call and put.
    ///
    /// Γ = exp((b-r)T)·φ(d₁) / (S·σ·√T)
    #[inline]
    pub fn gamma(&self) -> T {
        norm_pdf(self.d1) * self.carry_df / (self.spot * self.volatility * self.sqrt_t)
    }

    /// Computes Vega (∂V/∂σ), unscaled. Same for call and put.
    ///
    /// ν = S·exp((b-r)T)·φ(d₁)·√T
    #[inline]
    pub fn vega(&self) -> T { self.spot * self.carry_df * norm_pdf(self.d1) * self.sqrt_t }

    /// Computes Theta (-∂V/∂T), unscaled.
    ///
    /// Common term: -S·exp((b-r)T)·φ(d₁)·σ / (2√T)
    #[inline]
    pub fn theta(&self, is_call: bool) -> T {
        let pdf_d1 = norm_pdf(self.d1);
        let nd1 = norm_cdf(self.d1);
        let nd2 = norm_cdf(self.d2);
        let two: T = from_f64(2.0);
        let b_minus_r = self.cost_of_carry - self.rate;

        let term1 = -self.spot * self.carry_df * pdf_d1 * self.volatility / (two * self.sqrt_t);

        if is_call {
            term1
                - b_minus_r * self.spot * self.carry_df * nd1
                - self.rate * self.strike * self.df * nd2
        } else {
            term1
                + b_minus_r * self.spot * self.carry_df * (T::one() - nd1)
                + self.rate * self.strike * self.df * (T::one() - nd2)
        }
    }

    /// Computes Rho (∂V/∂r), unscaled.
    ///
    /// - Call ρ = K·T·exp(-rT)·N(d₂)
    /// - Put ρ  = -K·T·exp(-rT)·N(-d₂)
    #[inline]
    pub fn rho(&self, is_call: bool) -> T {
        if is_call {
            self.strike * self.expiry * self.df * norm_cdf(self.d2)
        } else {
            -self.strike * self.expiry * self.df * norm_cdf(-self.d2)
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_bs_case_put_call_parity() {
        // b = r (Black-Scholes)
        let m = GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, 0.2, 1.0).unwrap();
        let call = m.price(true);
        let put = m.price(false);
        // C - P = S - K·exp(-rT)
        let forward = 100.0 - 100.0 * (-0.05_f64).exp();
        assert_relative_eq!(call - put, forward, epsilon = 1e-10);
    }

    #[test]
    fn test_gk_case_put_call_parity() {
        // b = rd - rf (Garman-Kohlhagen)
        let rd = 0.03;
        let rf = 0.01;
        let m = GeneralisedBSM::new(1.10_f64, 1.12, rd, rd - rf, 0.15, 1.0).unwrap();
        let call = m.price(true);
        let put = m.price(false);
        // C - P = S·exp(-rf·T) - K·exp(-rd·T)
        let forward = 1.10 * (-rf).exp() - 1.12 * (-rd).exp();
        assert_relative_eq!(call - put, forward, epsilon = 1e-10);
    }

    #[test]
    fn test_bs_case_various_strikes() {
        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let m = GeneralisedBSM::new(100.0_f64, strike, 0.05, 0.05, 0.2, 1.0).unwrap();
            let call = m.price(true);
            let put = m.price(false);
            let forward = 100.0 - strike * (-0.05_f64).exp();
            assert_relative_eq!(call - put, forward, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_delta_call_put_relationship() {
        let m = GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, 0.2, 1.0).unwrap();
        let call_delta = m.delta(true);
        let put_delta = m.delta(false);
        assert_relative_eq!(
            put_delta,
            call_delta - m.carry_discount_factor(),
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_gamma_positive() {
        let m = GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, 0.2, 1.0).unwrap();
        assert!(m.gamma() > 0.0);
    }

    #[test]
    fn test_vega_positive() {
        let m = GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, 0.2, 1.0).unwrap();
        assert!(m.vega() > 0.0);
    }

    #[test]
    fn test_theta_call_negative() {
        let m = GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, 0.2, 1.0).unwrap();
        assert!(m.theta(true) < 0.0);
    }

    #[test]
    fn test_rho_call_positive() {
        let m = GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, 0.2, 1.0).unwrap();
        assert!(m.rho(true) > 0.0);
    }

    #[test]
    fn test_rho_put_negative() {
        let m = GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, 0.2, 1.0).unwrap();
        assert!(m.rho(false) < 0.0);
    }

    #[test]
    fn test_validation() {
        assert!(GeneralisedBSM::new(-1.0_f64, 100.0, 0.05, 0.05, 0.2, 1.0).is_err());
        assert!(GeneralisedBSM::new(100.0_f64, -1.0, 0.05, 0.05, 0.2, 1.0).is_err());
        assert!(GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, -0.2, 1.0).is_err());
        assert!(GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, 0.2, -1.0).is_err());
    }

    #[test]
    fn test_d1_d2_relationship() {
        let m = GeneralisedBSM::new(100.0_f64, 105.0, 0.05, 0.03, 0.2, 0.5).unwrap();
        let vol_sqrt_t = 0.2 * 0.5_f64.sqrt();
        assert_relative_eq!(m.d1() - m.d2(), vol_sqrt_t, epsilon = 1e-10);
    }

    #[test]
    fn test_bs_reference_values() {
        // S=100, K=100, r=0.05, b=0.05, σ=0.2, T=1
        let m = GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, 0.2, 1.0).unwrap();
        assert_relative_eq!(m.price(true), 10.4506, epsilon = 0.001);
        assert_relative_eq!(m.price(false), 5.5735, epsilon = 0.001);
    }

    #[test]
    fn test_f32_compatibility() {
        let m = GeneralisedBSM::new(100.0_f32, 100.0, 0.05, 0.05, 0.2, 1.0).unwrap();
        assert!(m.price(true) > 0.0_f32);
        assert!(m.price(false) > 0.0_f32);
    }
}
