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

use super::{error::FormulaError, generalised_bsm::GeneralisedBSM};
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
        let zero = T::zero();

        if spot <= zero {
            return Err(FormulaError::InvalidSpot {
                spot: spot.to_f64().unwrap_or(0.0),
            });
        }

        if volatility <= zero {
            return Err(FormulaError::InvalidVolatility {
                volatility: volatility.to_f64().unwrap_or(0.0),
            });
        }

        Ok(Self {
            spot,
            rate,
            volatility,
        })
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

    /// Computes d₁ = (ln(S/K) + (r + σ²/2)T) / (σ√T).
    #[inline]
    pub fn d1(&self, strike: T, expiry: T) -> T {
        match self.bsm(strike, expiry) {
            Some(m) => m.d1(),
            None => {
                let large: T = from_f64(100.0);
                if self.spot > strike {
                    large
                } else if self.spot < strike {
                    -large
                } else {
                    T::zero()
                }
            }
        }
    }

    /// Computes d₂ = d₁ - σ√T.
    #[inline]
    pub fn d2(&self, strike: T, expiry: T) -> T {
        match self.bsm(strike, expiry) {
            Some(m) => m.d2(),
            None => self.d1(strike, expiry),
        }
    }

    /// Computes European call price: C = S·N(d₁) - K·e^(-rT)·N(d₂).
    #[inline]
    pub fn price_call(&self, strike: T, expiry: T) -> T {
        match self.bsm(strike, expiry) {
            Some(m) => m.price(true),
            None => {
                let zero = T::zero();
                let intrinsic = self.spot - strike;
                if intrinsic > zero {
                    intrinsic
                } else {
                    zero
                }
            }
        }
    }

    /// Computes European put price: P = K·e^(-rT)·N(-d₂) - S·N(-d₁).
    #[inline]
    pub fn price_put(&self, strike: T, expiry: T) -> T {
        match self.bsm(strike, expiry) {
            Some(m) => m.price(false),
            None => {
                let zero = T::zero();
                let intrinsic = strike - self.spot;
                if intrinsic > zero {
                    intrinsic
                } else {
                    zero
                }
            }
        }
    }

    /// Computes option price based on call/put flag.
    #[inline]
    pub fn price(&self, strike: T, expiry: T, is_call: bool) -> T {
        if is_call {
            self.price_call(strike, expiry)
        } else {
            self.price_put(strike, expiry)
        }
    }

    /// Computes Delta (∂V/∂S).
    ///
    /// - Call Delta = N(d₁)
    /// - Put Delta = N(d₁) - 1
    #[inline]
    pub fn delta(&self, strike: T, expiry: T, is_call: bool) -> T {
        match self.bsm(strike, expiry) {
            Some(m) => m.delta(is_call),
            None => {
                let one = T::one();
                let zero = T::zero();
                if is_call {
                    if self.spot > strike {
                        one
                    } else {
                        zero
                    }
                } else if self.spot < strike {
                    -one
                } else {
                    zero
                }
            }
        }
    }

    /// Computes Gamma (∂²V/∂S²).
    ///
    /// Gamma = φ(d₁) / (S·σ·√T)
    #[inline]
    pub fn gamma(&self, strike: T, expiry: T) -> T {
        self.bsm(strike, expiry).map_or(T::zero(), |m| m.gamma())
    }

    /// Computes Vega (∂V/∂σ).
    ///
    /// Vega = S·√T·φ(d₁)
    #[inline]
    pub fn vega(&self, strike: T, expiry: T) -> T {
        self.bsm(strike, expiry).map_or(T::zero(), |m| m.vega())
    }

    /// Computes Theta (∂V/∂t).
    ///
    /// - Call Theta = -(S·σ·φ(d₁))/(2√T) - r·K·e^(-rT)·N(d₂)
    /// - Put Theta = -(S·σ·φ(d₁))/(2√T) + r·K·e^(-rT)·N(-d₂)
    #[inline]
    pub fn theta(&self, strike: T, expiry: T, is_call: bool) -> T {
        self.bsm(strike, expiry)
            .map_or(T::zero(), |m| m.theta(is_call))
    }

    /// Computes Rho (∂V/∂r).
    ///
    /// - Call Rho = K·T·e^(-rT)·N(d₂)
    /// - Put Rho = -K·T·e^(-rT)·N(-d₂)
    #[inline]
    pub fn rho(&self, strike: T, expiry: T, is_call: bool) -> T {
        self.bsm(strike, expiry)
            .map_or(T::zero(), |m| m.rho(is_call))
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_new_valid_parameters() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2);
        assert!(bs.is_ok());

        let bs = bs.unwrap();
        assert_eq!(bs.spot(), 100.0);
        assert_eq!(bs.rate(), 0.05);
        assert_eq!(bs.volatility(), 0.2);
    }

    #[test]
    fn test_new_invalid_spot_negative() {
        let result = BlackScholes::new(-100.0_f64, 0.05, 0.2);
        assert!(result.is_err());
        match result.unwrap_err() {
            FormulaError::InvalidSpot { spot } => {
                assert_eq!(spot, -100.0);
            }
            _ => panic!("Expected InvalidSpot error"),
        }
    }

    #[test]
    fn test_new_invalid_spot_zero() {
        let result = BlackScholes::new(0.0_f64, 0.05, 0.2);
        assert!(result.is_err());
        match result.unwrap_err() {
            FormulaError::InvalidSpot { .. } => {}
            _ => panic!("Expected InvalidSpot error"),
        }
    }

    #[test]
    fn test_new_invalid_volatility_negative() {
        let result = BlackScholes::new(100.0_f64, 0.05, -0.2);
        assert!(result.is_err());
        match result.unwrap_err() {
            FormulaError::InvalidVolatility { volatility } => {
                assert_eq!(volatility, -0.2);
            }
            _ => panic!("Expected InvalidVolatility error"),
        }
    }

    #[test]
    fn test_new_invalid_volatility_zero() {
        let result = BlackScholes::new(100.0_f64, 0.05, 0.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            FormulaError::InvalidVolatility { .. } => {}
            _ => panic!("Expected InvalidVolatility error"),
        }
    }

    #[test]
    fn test_new_negative_rate_allowed() {
        let bs = BlackScholes::new(100.0_f64, -0.02, 0.2);
        assert!(bs.is_ok());
    }

    #[test]
    fn test_d1_atm() {
        // ATM with r=0: d1 = σ√T / 2
        let bs = BlackScholes::new(100.0_f64, 0.0, 0.2).unwrap();
        let d1 = bs.d1(100.0, 1.0);
        assert_relative_eq!(d1, 0.1, epsilon = 1e-10);
    }

    #[test]
    fn test_d2_atm() {
        let bs = BlackScholes::new(100.0_f64, 0.0, 0.2).unwrap();
        let d2 = bs.d2(100.0, 1.0);
        assert_relative_eq!(d2, -0.1, epsilon = 1e-10);
    }

    #[test]
    fn test_d1_d2_relationship() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let d1 = bs.d1(105.0, 0.5);
        let d2 = bs.d2(105.0, 0.5);
        let expected_d2 = d1 - 0.2 * 0.5_f64.sqrt();
        assert_relative_eq!(d2, expected_d2, epsilon = 1e-10);
    }

    #[test]
    fn test_d1_expiry_zero() {
        let bs = BlackScholes::new(110.0_f64, 0.05, 0.2).unwrap();
        let d1_itm = bs.d1(100.0, 0.0);
        assert!(d1_itm > 50.0);

        let d1_otm = bs.d1(120.0, 0.0);
        assert!(d1_otm < -50.0);
    }

    #[test]
    fn test_call_price_positive() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let price = bs.price_call(100.0, 1.0);
        assert!(price > 0.0);
    }

    #[test]
    fn test_put_price_positive() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let price = bs.price_put(100.0, 1.0);
        assert!(price > 0.0);
    }

    #[test]
    fn test_call_price_reference_value() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let price = bs.price_call(100.0, 1.0);
        assert_relative_eq!(price, 10.4506, epsilon = 0.001);
    }

    #[test]
    fn test_put_price_reference_value() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let price = bs.price_put(100.0, 1.0);
        assert_relative_eq!(price, 5.5735, epsilon = 0.001);
    }

    #[test]
    fn test_price_method() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        assert_eq!(bs.price(100.0, 1.0, true), bs.price_call(100.0, 1.0));
        assert_eq!(bs.price(100.0, 1.0, false), bs.price_put(100.0, 1.0));
    }

    #[test]
    fn test_put_call_parity() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let call = bs.price_call(100.0, 1.0);
        let put = bs.price_put(100.0, 1.0);
        let forward = 100.0 - 100.0 * (-0.05_f64).exp();
        assert_relative_eq!(call - put, forward, epsilon = 1e-10);
    }

    #[test]
    fn test_put_call_parity_various_strikes() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let call = bs.price_call(strike, 1.0);
            let put = bs.price_put(strike, 1.0);
            let forward = 100.0 - strike * (-0.05_f64).exp();
            assert_relative_eq!(call - put, forward, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_delta_call_bounds() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let delta = bs.delta(strike, 1.0, true);
            assert!(delta >= 0.0, "Call delta should be >= 0");
            assert!(delta <= 1.0, "Call delta should be <= 1");
        }
    }

    #[test]
    fn test_delta_put_bounds() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let delta = bs.delta(strike, 1.0, false);
            assert!(delta >= -1.0, "Put delta should be >= -1");
            assert!(delta <= 0.0, "Put delta should be <= 0");
        }
    }

    #[test]
    fn test_delta_call_put_relationship() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let call_delta = bs.delta(100.0, 1.0, true);
        let put_delta = bs.delta(100.0, 1.0, false);
        assert_relative_eq!(put_delta, call_delta - 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_gamma_non_negative() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            assert!(bs.gamma(strike, 1.0) >= 0.0);
        }
    }

    #[test]
    fn test_vega_non_negative() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            assert!(bs.vega(strike, 1.0) >= 0.0);
        }
    }

    #[test]
    fn test_theta_call_typically_negative() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        assert!(bs.theta(100.0, 1.0, true) < 0.0);
    }

    #[test]
    fn test_rho_call_positive() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        assert!(bs.rho(100.0, 1.0, true) > 0.0);
    }

    #[test]
    fn test_rho_put_negative() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        assert!(bs.rho(100.0, 1.0, false) < 0.0);
    }

    #[test]
    fn test_clone() {
        let bs1 = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let bs2 = bs1.clone();
        assert_eq!(bs1.spot(), bs2.spot());
        assert_eq!(bs1.rate(), bs2.rate());
        assert_eq!(bs1.volatility(), bs2.volatility());
    }

    #[test]
    fn test_debug() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let debug_str = format!("{:?}", bs);
        assert!(debug_str.contains("BlackScholes"));
        assert!(debug_str.contains("spot"));
    }

    #[test]
    fn test_f32_compatibility() {
        let bs = BlackScholes::new(100.0_f32, 0.05_f32, 0.2_f32).unwrap();
        let call = bs.price_call(100.0_f32, 1.0_f32);
        assert!(call > 0.0_f32);
    }
}
