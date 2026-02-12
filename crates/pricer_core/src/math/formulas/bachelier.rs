//! Bachelier (normal) pricing model for European options.
//!
//! This module provides the Bachelier model for pricing European
//! options under normal (arithmetic) dynamics. This model is commonly
//! used for interest rate options where negative forward prices are possible.
//!
//! ## Mathematical Formulas
//!
//! **Call Price**: C = (F - K)·N(d) + σ√T·φ(d)
//! **Put Price**: P = (K - F)·N(-d) + σ√T·φ(d)
//!
//! Where:
//! - d = (F - K) / (σ√T)
//! - N(·) is the standard normal CDF
//! - φ(·) is the standard normal PDF

use num_traits::Float;

use super::error::FormulaError;
use crate::math::{
    normal_dist::{norm_cdf, norm_pdf},
    numeric::from_f64,
};

/// Bachelier (normal) model for European option pricing.
///
/// Supports negative forward prices, making it suitable for interest rate markets.
#[derive(Debug, Clone)]
pub struct Bachelier<T: Float> {
    /// Forward price (F) - can be negative
    forward: T,
    /// Volatility (σ) - must be positive
    volatility: T,
}

impl<T: Float> Bachelier<T> {
    /// Creates a new Bachelier model. Returns error if volatility <= 0.
    pub fn new(forward: T, volatility: T) -> Result<Self, FormulaError> {
        let zero = T::zero();

        if volatility <= zero {
            return Err(FormulaError::InvalidVolatility {
                volatility: volatility.to_f64().unwrap_or(0.0),
            });
        }

        Ok(Self {
            forward,
            volatility,
        })
    }

    /// Returns the forward price.
    #[inline]
    pub fn forward(&self) -> T { self.forward }

    /// Returns the volatility.
    #[inline]
    pub fn volatility(&self) -> T { self.volatility }

    /// Computes d = (F - K) / (σ√T).
    #[inline]
    fn d(&self, strike: T, expiry: T) -> T {
        let epsilon: T = from_f64(1e-10);

        if expiry <= epsilon {
            let zero = T::zero();
            let large: T = from_f64(100.0);
            if self.forward > strike {
                return large;
            } else if self.forward < strike {
                return -large;
            } else {
                return zero;
            }
        }

        let sqrt_t = expiry.sqrt();
        let vol_sqrt_t = self.volatility * sqrt_t;

        (self.forward - strike) / vol_sqrt_t
    }

    /// Computes European call price: C = (F - K)·N(d) + σ√T·φ(d).
    #[inline]
    pub fn price_call(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();
        let epsilon: T = from_f64(1e-10);

        // Handle expiry = 0: return intrinsic value
        if expiry <= epsilon {
            let intrinsic = self.forward - strike;
            return if intrinsic > zero { intrinsic } else { zero };
        }

        let d = self.d(strike, expiry);
        let sqrt_t = expiry.sqrt();
        let vol_sqrt_t = self.volatility * sqrt_t;

        // C = (F - K)·N(d) + σ√T·φ(d)
        (self.forward - strike) * norm_cdf(d) + vol_sqrt_t * norm_pdf(d)
    }

    /// Computes European put price: P = (K - F)·N(-d) + σ√T·φ(d).
    #[inline]
    pub fn price_put(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();
        let epsilon: T = from_f64(1e-10);

        // Handle expiry = 0: return intrinsic value
        if expiry <= epsilon {
            let intrinsic = strike - self.forward;
            return if intrinsic > zero { intrinsic } else { zero };
        }

        let d = self.d(strike, expiry);
        let sqrt_t = expiry.sqrt();
        let vol_sqrt_t = self.volatility * sqrt_t;

        // P = (K - F)·N(-d) + σ√T·φ(d)
        (strike - self.forward) * norm_cdf(-d) + vol_sqrt_t * norm_pdf(d)
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
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_new_valid_parameters() {
        let model = Bachelier::new(0.03_f64, 0.01);
        assert!(model.is_ok());

        let model = model.unwrap();
        assert_eq!(model.forward(), 0.03);
        assert_eq!(model.volatility(), 0.01);
    }

    #[test]
    fn test_new_negative_forward_allowed() {
        // Negative forwards should be allowed (interest rate markets)
        let model = Bachelier::new(-0.005_f64, 0.01);
        assert!(model.is_ok());
        assert_eq!(model.unwrap().forward(), -0.005);
    }

    #[test]
    fn test_new_zero_forward_allowed() {
        // Zero forward should be allowed
        let model = Bachelier::new(0.0_f64, 0.01);
        assert!(model.is_ok());
    }

    #[test]
    fn test_new_invalid_volatility_negative() {
        let result = Bachelier::new(0.03_f64, -0.01);
        assert!(result.is_err());
        match result.unwrap_err() {
            FormulaError::InvalidVolatility { volatility } => {
                assert_eq!(volatility, -0.01);
            }
            _ => panic!("Expected InvalidVolatility error"),
        }
    }

    #[test]
    fn test_new_invalid_volatility_zero() {
        let result = Bachelier::new(0.03_f64, 0.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            FormulaError::InvalidVolatility { .. } => {}
            _ => panic!("Expected InvalidVolatility error"),
        }
    }

    #[test]
    fn test_call_price_positive() {
        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        let price = model.price_call(0.03, 1.0);
        assert!(price > 0.0);
    }

    #[test]
    fn test_put_price_positive() {
        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        let price = model.price_put(0.03, 1.0);
        assert!(price > 0.0);
    }

    #[test]
    fn test_atm_call_equals_put() {
        // At ATM, call and put prices are equal under Bachelier
        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        let call = model.price_call(0.03, 1.0);
        let put = model.price_put(0.03, 1.0);
        assert_relative_eq!(call, put, epsilon = 1e-10);
    }

    #[test]
    fn test_atm_price_formula() {
        // ATM Bachelier price = σ√T * φ(0) = σ√T * 0.3989...
        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        let call = model.price_call(0.03, 1.0);
        let expected = 0.01 * 1.0_f64.sqrt() * norm_pdf(0.0_f64);
        assert_relative_eq!(call, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_call_price_expiry_zero_itm() {
        let model = Bachelier::new(0.05_f64, 0.01).unwrap();
        let price = model.price_call(0.03, 0.0);
        assert_relative_eq!(price, 0.02, epsilon = 1e-10);
    }

    #[test]
    fn test_call_price_expiry_zero_otm() {
        let model = Bachelier::new(0.01_f64, 0.01).unwrap();
        let price = model.price_call(0.03, 0.0);
        assert_relative_eq!(price, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_put_price_expiry_zero_itm() {
        let model = Bachelier::new(0.01_f64, 0.01).unwrap();
        let price = model.price_put(0.03, 0.0);
        assert_relative_eq!(price, 0.02, epsilon = 1e-10);
    }

    #[test]
    fn test_put_price_expiry_zero_otm() {
        let model = Bachelier::new(0.05_f64, 0.01).unwrap();
        let price = model.price_put(0.03, 0.0);
        assert_relative_eq!(price, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_negative_forward_price() {
        // Test pricing with negative forward (interest rate scenario)
        let model = Bachelier::new(-0.005_f64, 0.01).unwrap();
        let call = model.price_call(-0.005, 1.0);
        let put = model.price_put(-0.005, 1.0);
        assert!(call > 0.0);
        assert!(put > 0.0);
        assert_relative_eq!(call, put, epsilon = 1e-10); // ATM
    }

    #[test]
    fn test_price_method() {
        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        assert_eq!(model.price(0.03, 1.0, true), model.price_call(0.03, 1.0));
        assert_eq!(model.price(0.03, 1.0, false), model.price_put(0.03, 1.0));
    }

    #[test]
    fn test_put_call_parity() {
        // For Bachelier: C - P = F - K
        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        let call = model.price_call(0.025, 1.0);
        let put = model.price_put(0.025, 1.0);
        let forward_minus_strike = 0.03 - 0.025;
        assert_relative_eq!(call - put, forward_minus_strike, epsilon = 1e-10);
    }

    #[test]
    fn test_put_call_parity_various_strikes() {
        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        for strike in [0.01, 0.02, 0.03, 0.04, 0.05] {
            let call = model.price_call(strike, 1.0);
            let put = model.price_put(strike, 1.0);
            let forward_minus_strike = 0.03 - strike;
            assert_relative_eq!(call - put, forward_minus_strike, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_put_call_parity_various_expiries() {
        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        for expiry in [0.25, 0.5, 1.0, 2.0, 5.0] {
            let call = model.price_call(0.03, expiry);
            let put = model.price_put(0.03, expiry);
            // ATM: F - K = 0
            assert_relative_eq!(call - put, 0.0, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_put_call_parity_negative_forward() {
        let model = Bachelier::new(-0.005_f64, 0.01).unwrap();
        let call = model.price_call(-0.01, 1.0);
        let put = model.price_put(-0.01, 1.0);
        let forward_minus_strike = -0.005 - (-0.01);
        assert_relative_eq!(call - put, forward_minus_strike, epsilon = 1e-10);
    }

    #[test]
    fn test_clone() {
        let model1 = Bachelier::new(0.03_f64, 0.01).unwrap();
        let model2 = model1.clone();
        assert_eq!(model1.forward(), model2.forward());
        assert_eq!(model1.volatility(), model2.volatility());
    }

    #[test]
    fn test_debug() {
        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        let debug_str = format!("{:?}", model);
        assert!(debug_str.contains("Bachelier"));
        assert!(debug_str.contains("forward"));
    }

    #[test]
    fn test_f32_compatibility() {
        let model = Bachelier::new(0.03_f32, 0.01_f32).unwrap();
        let call = model.price_call(0.03_f32, 1.0_f32);
        assert!(call > 0.0_f32);
    }
}
