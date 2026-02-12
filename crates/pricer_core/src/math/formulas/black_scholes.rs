//! Black-Scholes pricing model for European options.
//!
//! Delegates to [`GeneralisedBSM`] with cost-of-carry `b = r`.
//!
//! ## Mathematical Formulas
//!
//! **Call Price**: C = S·N(d₁) - K·e^(-rT)·N(d₂)
//! **Put Price**: P = K·e^(-rT)·N(-d₂) - S·N(-d₁)
//!
//! Where:
//! - d₁ = (ln(S/K) + (r + σ²/2)T) / (σ√T)
//! - d₂ = d₁ - σ√T

use num_traits::Float;

use super::{
    error::{require_positive_spot, require_positive_vol, FormulaError},
    generalised_bsm::GeneralisedBSM,
};
use crate::math::numeric::from_f64;

/// Black-Scholes model for European option pricing and Greeks.
#[derive(Debug, Clone)]
pub struct BlackScholes<T: Float> {
    /// Spot price (S)
    spot: T,
    /// Risk-free interest rate (r)
    rate: T,
    /// Volatility (σ)
    volatility: T,
}

impl<T: Float> BlackScholes<T> {
    /// Creates a new Black-Scholes model. Returns error if spot or volatility <= 0.
    pub fn new(spot: T, rate: T, volatility: T) -> Result<Self, FormulaError> {
        require_positive_spot(spot)?;
        require_positive_vol(volatility)?;
        Ok(Self { spot, rate, volatility })
    }

    /// Returns the spot price.
    #[inline]
    pub fn spot(&self) -> T { self.spot }

    /// Returns the risk-free rate.
    #[inline]
    pub fn rate(&self) -> T { self.rate }

    /// Returns the volatility.
    #[inline]
    pub fn volatility(&self) -> T { self.volatility }

    /// Creates a [`GeneralisedBSM`] with `b = r` for the given strike/expiry.
    /// Returns `None` if expiry is near zero or strike is non-positive.
    #[inline]
    fn bsm(&self, strike: T, expiry: T) -> Option<GeneralisedBSM<T>> {
        let epsilon: T = from_f64(1e-10);
        if expiry <= epsilon {
            return None;
        }
        GeneralisedBSM::new(
            self.spot,
            strike,
            self.rate,
            self.rate,
            self.volatility,
            expiry,
        )
        .ok()
    }

    /// Moneyness direction for expired option: +large (ITM), -large (OTM), 0 (ATM).
    #[inline]
    fn expired_sign(&self, strike: T) -> T {
        let large: T = from_f64(100.0);
        if self.spot > strike { large } else if self.spot < strike { -large } else { T::zero() }
    }

    /// Expired intrinsic value: max(φ·(S−K), 0) where φ = +1 call / −1 put.
    #[inline]
    fn expired_intrinsic(&self, strike: T, is_call: bool) -> T {
        let diff = if is_call { self.spot - strike } else { strike - self.spot };
        if diff > T::zero() { diff } else { T::zero() }
    }

    /// Computes d₁ = (ln(S/K) + (r + σ²/2)T) / (σ√T).
    #[inline]
    pub fn d1(&self, strike: T, expiry: T) -> T {
        self.bsm(strike, expiry).map_or_else(|| self.expired_sign(strike), |m| m.d1())
    }

    /// Computes d₂ = d₁ - σ√T.
    #[inline]
    pub fn d2(&self, strike: T, expiry: T) -> T {
        self.bsm(strike, expiry).map_or_else(|| self.expired_sign(strike), |m| m.d2())
    }

    /// Computes European call price: C = S·N(d₁) - K·e^(-rT)·N(d₂).
    #[inline]
    pub fn price_call(&self, strike: T, expiry: T) -> T { self.price(strike, expiry, true) }

    /// Computes European put price: P = K·e^(-rT)·N(-d₂) - S·N(-d₁).
    #[inline]
    pub fn price_put(&self, strike: T, expiry: T) -> T { self.price(strike, expiry, false) }

    /// Computes option price based on call/put flag.
    #[inline]
    pub fn price(&self, strike: T, expiry: T, is_call: bool) -> T {
        self.bsm(strike, expiry)
            .map_or_else(|| self.expired_intrinsic(strike, is_call), |m| m.price(is_call))
    }

    /// Computes Delta (∂V/∂S).
    #[inline]
    pub fn delta(&self, strike: T, expiry: T, is_call: bool) -> T {
        self.bsm(strike, expiry).map_or_else(
            || {
                let (one, zero) = (T::one(), T::zero());
                if is_call {
                    if self.spot > strike { one } else { zero }
                } else if self.spot < strike { -one } else { zero }
            },
            |m| m.delta(is_call),
        )
    }

    /// Computes Gamma (∂²V/∂S²).
    #[inline]
    pub fn gamma(&self, strike: T, expiry: T) -> T {
        self.bsm(strike, expiry).map_or(T::zero(), |m| m.gamma())
    }

    /// Computes Vega (∂V/∂σ).
    #[inline]
    pub fn vega(&self, strike: T, expiry: T) -> T {
        self.bsm(strike, expiry).map_or(T::zero(), |m| m.vega())
    }

    /// Computes Theta (∂V/∂t).
    #[inline]
    pub fn theta(&self, strike: T, expiry: T, is_call: bool) -> T {
        self.bsm(strike, expiry).map_or(T::zero(), |m| m.theta(is_call))
    }

    /// Computes Rho (∂V/∂r).
    #[inline]
    pub fn rho(&self, strike: T, expiry: T, is_call: bool) -> T {
        self.bsm(strike, expiry).map_or(T::zero(), |m| m.rho(is_call))
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn bs() -> BlackScholes<f64> { BlackScholes::new(100.0, 0.05, 0.2).unwrap() }

    #[test]
    fn valid_params() {
        let m = bs();
        assert_eq!(m.spot(), 100.0);
        assert_eq!(m.rate(), 0.05);
        assert_eq!(m.volatility(), 0.2);
    }

    #[test]
    fn validation() {
        assert!(matches!(BlackScholes::new(-100.0_f64, 0.05, 0.2).unwrap_err(), FormulaError::InvalidSpot { .. }));
        assert!(matches!(BlackScholes::new(0.0_f64, 0.05, 0.2).unwrap_err(), FormulaError::InvalidSpot { .. }));
        assert!(matches!(BlackScholes::new(100.0_f64, 0.05, -0.2).unwrap_err(), FormulaError::InvalidVolatility { .. }));
        assert!(matches!(BlackScholes::new(100.0_f64, 0.05, 0.0).unwrap_err(), FormulaError::InvalidVolatility { .. }));
        assert!(BlackScholes::new(100.0_f64, -0.02, 0.2).is_ok());
    }

    #[test]
    fn d1_d2_atm() {
        let m = BlackScholes::new(100.0_f64, 0.0, 0.2).unwrap();
        assert_relative_eq!(m.d1(100.0, 1.0), 0.1, epsilon = 1e-10);
        assert_relative_eq!(m.d2(100.0, 1.0), -0.1, epsilon = 1e-10);
    }

    #[test]
    fn d1_d2_relationship() {
        let m = bs();
        let (d1, d2) = (m.d1(105.0, 0.5), m.d2(105.0, 0.5));
        assert_relative_eq!(d2, d1 - 0.2 * 0.5_f64.sqrt(), epsilon = 1e-10);
    }

    #[test]
    fn d1_expiry_zero() {
        let m = BlackScholes::new(110.0_f64, 0.05, 0.2).unwrap();
        assert!(m.d1(100.0, 0.0) > 50.0);
        assert!(m.d1(120.0, 0.0) < -50.0);
    }

    #[test]
    fn reference_prices() {
        let m = bs();
        assert_relative_eq!(m.price_call(100.0, 1.0), 10.4506, epsilon = 0.001);
        assert_relative_eq!(m.price_put(100.0, 1.0), 5.5735, epsilon = 0.001);
        assert!(m.price_call(100.0, 1.0) > 0.0);
        assert!(m.price_put(100.0, 1.0) > 0.0);
    }

    #[test]
    fn price_method_consistency() {
        let m = bs();
        assert_eq!(m.price(100.0, 1.0, true), m.price_call(100.0, 1.0));
        assert_eq!(m.price(100.0, 1.0, false), m.price_put(100.0, 1.0));
    }

    #[test]
    fn put_call_parity() {
        let m = bs();
        for k in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let forward = 100.0 - k * (-0.05_f64).exp();
            assert_relative_eq!(m.price_call(k, 1.0) - m.price_put(k, 1.0), forward, epsilon = 1e-10);
        }
    }

    #[test]
    fn delta_bounds_and_relationship() {
        let m = bs();
        for k in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let cd = m.delta(k, 1.0, true);
            let pd = m.delta(k, 1.0, false);
            assert!((0.0..=1.0).contains(&cd), "Call delta OOB at K={k}");
            assert!((-1.0..=0.0).contains(&pd), "Put delta OOB at K={k}");
        }
        assert_relative_eq!(m.delta(100.0, 1.0, false), m.delta(100.0, 1.0, true) - 1.0, epsilon = 1e-10);
    }

    #[test]
    fn greeks_signs() {
        let m = bs();
        for k in [80.0, 90.0, 100.0, 110.0, 120.0] {
            assert!(m.gamma(k, 1.0) >= 0.0);
            assert!(m.vega(k, 1.0) >= 0.0);
        }
        assert!(m.theta(100.0, 1.0, true) < 0.0);
        assert!(m.rho(100.0, 1.0, true) > 0.0);
        assert!(m.rho(100.0, 1.0, false) < 0.0);
    }

    #[test]
    fn clone_and_debug() {
        let m = bs();
        let c = m.clone();
        assert_eq!(m.spot(), c.spot());
        assert!(format!("{:?}", m).contains("BlackScholes"));
    }

    #[test]
    fn f32_compatibility() {
        let m = BlackScholes::new(100.0_f32, 0.05, 0.2).unwrap();
        assert!(m.price_call(100.0, 1.0) > 0.0_f32);
    }
}
