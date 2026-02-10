//! Black-Scholes pricing model for European options.
//!
//! This module provides the Black-Scholes model for pricing European
//! call and put options with analytical Greeks calculations.
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

use super::error::FormulaError;
use crate::math::{
    normal_dist::{norm_cdf, norm_pdf},
    numeric::from_f64,
};

/// Black-Scholes model for European option pricing.
///
/// Provides closed-form pricing and Greeks calculations for European
/// options under lognormal dynamics.
///
/// # Type Parameters
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Examples
/// ```
/// use pricer_core::math::formulas::BlackScholes;
///
/// let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
/// let call_price = bs.price_call(100.0, 1.0);
/// let put_price = bs.price_put(100.0, 1.0);
///
/// // Put-call parity: C - P = S - K*exp(-rT)
/// let parity = call_price - put_price - (100.0 - 100.0 * (-0.05_f64).exp());
/// assert!(parity.abs() < 1e-10);
/// ```
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
    /// Creates a new Black-Scholes model.
    ///
    /// # Arguments
    /// * `spot` - Current spot price (must be positive)
    /// * `rate` - Risk-free interest rate (annualised)
    /// * `volatility` - Volatility (must be positive)
    ///
    /// # Errors
    /// - `FormulaError::InvalidSpot` if spot <= 0
    /// - `FormulaError::InvalidVolatility` if volatility <= 0
    ///
    /// # Examples
    /// ```
    /// use pricer_core::math::formulas::BlackScholes;
    ///
    /// let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
    ///
    /// // Invalid spot
    /// assert!(BlackScholes::new(-100.0_f64, 0.05, 0.2).is_err());
    ///
    /// // Invalid volatility
    /// assert!(BlackScholes::new(100.0_f64, 0.05, 0.0).is_err());
    /// ```
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

    /// Computes the d1 term of the Black-Scholes formula.
    ///
    /// d₁ = (ln(S/K) + (r + σ²/2)T) / (σ√T)
    ///
    /// # Arguments
    /// * `strike` - Strike price (K)
    /// * `expiry` - Time to expiration in years (T)
    ///
    /// # Returns
    /// The d1 term. Returns large positive/negative values for limiting cases.
    #[inline]
    pub fn d1(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();
        let half: T = from_f64(0.5);
        let epsilon: T = from_f64(1e-10);

        // Handle expiry ≈ 0 case
        if expiry <= epsilon {
            // At expiry, if S > K, d1 → +∞, otherwise d1 → -∞
            let large: T = from_f64(100.0);
            if self.spot > strike {
                return large;
            } else if self.spot < strike {
                return -large;
            } else {
                return zero;
            }
        }

        let sqrt_t = expiry.sqrt();
        let vol_sqrt_t = self.volatility * sqrt_t;

        // d1 = (ln(S/K) + (r + σ²/2)T) / (σ√T)
        let log_moneyness = (self.spot / strike).ln();
        let drift = (self.rate + half * self.volatility * self.volatility) * expiry;

        (log_moneyness + drift) / vol_sqrt_t
    }

    /// Computes the d2 term of the Black-Scholes formula.
    ///
    /// d₂ = d₁ - σ√T
    ///
    /// # Arguments
    /// * `strike` - Strike price (K)
    /// * `expiry` - Time to expiration in years (T)
    ///
    /// # Returns
    /// The d2 term.
    #[inline]
    pub fn d2(&self, strike: T, expiry: T) -> T {
        let epsilon: T = from_f64(1e-10);

        if expiry <= epsilon {
            return self.d1(strike, expiry);
        }

        let sqrt_t = expiry.sqrt();
        self.d1(strike, expiry) - self.volatility * sqrt_t
    }

    /// Computes European call option price.
    ///
    /// C = S·N(d₁) - K·e^(-rT)·N(d₂)
    ///
    /// # Arguments
    /// * `strike` - Strike price (K)
    /// * `expiry` - Time to expiration in years (T)
    ///
    /// # Returns
    /// The theoretical call option price.
    ///
    /// # Examples
    /// ```
    /// use pricer_core::math::formulas::BlackScholes;
    ///
    /// let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
    /// let price = bs.price_call(100.0, 1.0);
    ///
    /// // ATM call should have positive value
    /// assert!(price > 0.0);
    /// ```
    #[inline]
    pub fn price_call(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();
        let epsilon: T = from_f64(1e-10);

        // Handle expiry = 0: return intrinsic value
        if expiry <= epsilon {
            let intrinsic = self.spot - strike;
            return if intrinsic > zero { intrinsic } else { zero };
        }

        let d1 = self.d1(strike, expiry);
        let d2 = self.d2(strike, expiry);

        let discount = (-self.rate * expiry).exp();

        // C = S·N(d₁) - K·e^(-rT)·N(d₂)
        self.spot * norm_cdf(d1) - strike * discount * norm_cdf(d2)
    }

    /// Computes European put option price.
    ///
    /// P = K·e^(-rT)·N(-d₂) - S·N(-d₁)
    ///
    /// # Arguments
    /// * `strike` - Strike price (K)
    /// * `expiry` - Time to expiration in years (T)
    ///
    /// # Returns
    /// The theoretical put option price.
    ///
    /// # Examples
    /// ```
    /// use pricer_core::math::formulas::BlackScholes;
    ///
    /// let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
    /// let price = bs.price_put(100.0, 1.0);
    ///
    /// // ATM put should have positive value
    /// assert!(price > 0.0);
    /// ```
    #[inline]
    pub fn price_put(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();
        let epsilon: T = from_f64(1e-10);

        // Handle expiry = 0: return intrinsic value
        if expiry <= epsilon {
            let intrinsic = strike - self.spot;
            return if intrinsic > zero { intrinsic } else { zero };
        }

        let d1 = self.d1(strike, expiry);
        let d2 = self.d2(strike, expiry);

        let discount = (-self.rate * expiry).exp();

        // P = K·e^(-rT)·N(-d₂) - S·N(-d₁)
        strike * discount * norm_cdf(-d2) - self.spot * norm_cdf(-d1)
    }

    /// Computes option price based on call/put flag.
    ///
    /// # Arguments
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiration
    /// * `is_call` - True for call, false for put
    ///
    /// # Returns
    /// The theoretical option price.
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
    ///
    /// # Arguments
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiration
    /// * `is_call` - True for call, false for put
    ///
    /// # Returns
    /// The delta sensitivity.
    #[inline]
    pub fn delta(&self, strike: T, expiry: T, is_call: bool) -> T {
        let epsilon: T = from_f64(1e-10);

        if expiry <= epsilon {
            let one = T::one();
            let zero = T::zero();
            if is_call {
                return if self.spot > strike { one } else { zero };
            } else {
                return if self.spot < strike { -one } else { zero };
            }
        }

        let d1 = self.d1(strike, expiry);
        let n_d1 = norm_cdf(d1);

        if is_call {
            n_d1
        } else {
            n_d1 - T::one()
        }
    }

    /// Computes Gamma (∂²V/∂S²).
    ///
    /// Gamma = φ(d₁) / (S·σ·√T)
    ///
    /// Gamma is the same for both calls and puts.
    ///
    /// # Arguments
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiration
    ///
    /// # Returns
    /// The gamma sensitivity (always non-negative).
    #[inline]
    pub fn gamma(&self, strike: T, expiry: T) -> T {
        let epsilon: T = from_f64(1e-10);

        if expiry <= epsilon {
            return T::zero();
        }

        let d1 = self.d1(strike, expiry);
        let sqrt_t = expiry.sqrt();

        // Gamma = φ(d₁) / (S·σ·√T)
        norm_pdf(d1) / (self.spot * self.volatility * sqrt_t)
    }

    /// Computes Vega (∂V/∂σ).
    ///
    /// Vega = S·√T·φ(d₁)
    ///
    /// Vega is the same for both calls and puts.
    ///
    /// # Arguments
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiration
    ///
    /// # Returns
    /// The vega sensitivity (always non-negative).
    #[inline]
    pub fn vega(&self, strike: T, expiry: T) -> T {
        let epsilon: T = from_f64(1e-10);

        if expiry <= epsilon {
            return T::zero();
        }

        let d1 = self.d1(strike, expiry);
        let sqrt_t = expiry.sqrt();

        // Vega = S·√T·φ(d₁)
        self.spot * sqrt_t * norm_pdf(d1)
    }

    /// Computes Theta (∂V/∂t).
    ///
    /// - Call Theta = -(S·σ·φ(d₁))/(2√T) - r·K·e^(-rT)·N(d₂)
    /// - Put Theta = -(S·σ·φ(d₁))/(2√T) + r·K·e^(-rT)·N(-d₂)
    ///
    /// Note: This returns the rate of change with respect to time,
    /// which is typically negative (time decay).
    ///
    /// # Arguments
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiration
    /// * `is_call` - True for call, false for put
    ///
    /// # Returns
    /// The theta sensitivity (usually negative).
    #[inline]
    pub fn theta(&self, strike: T, expiry: T, is_call: bool) -> T {
        let epsilon: T = from_f64(1e-10);

        if expiry <= epsilon {
            return T::zero();
        }

        let d1 = self.d1(strike, expiry);
        let d2 = self.d2(strike, expiry);
        let sqrt_t = expiry.sqrt();
        let discount = (-self.rate * expiry).exp();
        let two: T = from_f64(2.0);

        // Common term: -(S·σ·φ(d₁))/(2√T)
        let term1 = -(self.spot * self.volatility * norm_pdf(d1)) / (two * sqrt_t);

        if is_call {
            // Call Theta = term1 - r·K·e^(-rT)·N(d₂)
            term1 - self.rate * strike * discount * norm_cdf(d2)
        } else {
            // Put Theta = term1 + r·K·e^(-rT)·N(-d₂)
            term1 + self.rate * strike * discount * norm_cdf(-d2)
        }
    }

    /// Computes Rho (∂V/∂r).
    ///
    /// - Call Rho = K·T·e^(-rT)·N(d₂)
    /// - Put Rho = -K·T·e^(-rT)·N(-d₂)
    ///
    /// # Arguments
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiration
    /// * `is_call` - True for call, false for put
    ///
    /// # Returns
    /// The rho sensitivity.
    #[inline]
    pub fn rho(&self, strike: T, expiry: T, is_call: bool) -> T {
        let epsilon: T = from_f64(1e-10);

        if expiry <= epsilon {
            return T::zero();
        }

        let d2 = self.d2(strike, expiry);
        let discount = (-self.rate * expiry).exp();

        if is_call {
            // Call Rho = K·T·e^(-rT)·N(d₂)
            strike * expiry * discount * norm_cdf(d2)
        } else {
            // Put Rho = -K·T·e^(-rT)·N(-d₂)
            -strike * expiry * discount * norm_cdf(-d2)
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    // ==========================================================
    // Constructor Tests
    // ==========================================================

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
        // Negative rates should be allowed
        let bs = BlackScholes::new(100.0_f64, -0.02, 0.2);
        assert!(bs.is_ok());
    }

    // ==========================================================
    // d1/d2 Tests
    // ==========================================================

    #[test]
    fn test_d1_atm() {
        // ATM with r=0: d1 = σ√T / 2
        let bs = BlackScholes::new(100.0_f64, 0.0, 0.2).unwrap();
        let d1 = bs.d1(100.0, 1.0);
        // d1 = (0 + 0.02/2 * 1) / 0.2 = 0.05
        assert_relative_eq!(d1, 0.1, epsilon = 1e-10);
    }

    #[test]
    fn test_d2_atm() {
        // ATM with r=0: d2 = d1 - σ√T = -σ√T / 2
        let bs = BlackScholes::new(100.0_f64, 0.0, 0.2).unwrap();
        let d2 = bs.d2(100.0, 1.0);
        // d2 = 0.1 - 0.2 = -0.1
        assert_relative_eq!(d2, -0.1, epsilon = 1e-10);
    }

    #[test]
    fn test_d1_d2_relationship() {
        // d2 = d1 - σ√T
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let d1 = bs.d1(105.0, 0.5);
        let d2 = bs.d2(105.0, 0.5);
        let expected_d2 = d1 - 0.2 * 0.5_f64.sqrt();
        assert_relative_eq!(d2, expected_d2, epsilon = 1e-10);
    }

    #[test]
    fn test_d1_expiry_zero() {
        let bs = BlackScholes::new(110.0_f64, 0.05, 0.2).unwrap();
        // ITM call at expiry: d1 → +∞
        let d1_itm = bs.d1(100.0, 0.0);
        assert!(d1_itm > 50.0);

        // OTM call at expiry: d1 → -∞
        let d1_otm = bs.d1(120.0, 0.0);
        assert!(d1_otm < -50.0);
    }

    // ==========================================================
    // Price Tests
    // ==========================================================

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
        // Known reference: S=100, K=100, r=0.05, σ=0.2, T=1
        // Expected call price ≈ 10.4506
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let price = bs.price_call(100.0, 1.0);
        assert_relative_eq!(price, 10.4506, epsilon = 0.001);
    }

    #[test]
    fn test_put_price_reference_value() {
        // Known reference: S=100, K=100, r=0.05, σ=0.2, T=1
        // Expected put price ≈ 5.5735
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

    // ==========================================================
    // Put-Call Parity Tests
    // ==========================================================

    #[test]
    fn test_put_call_parity() {
        // C - P = S - K*exp(-rT)
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

    // ==========================================================
    // Greeks Tests
    // ==========================================================

    #[test]
    fn test_delta_call_bounds() {
        // Call delta ∈ [0, 1]
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let delta = bs.delta(strike, 1.0, true);
            assert!(delta >= 0.0, "Call delta should be >= 0");
            assert!(delta <= 1.0, "Call delta should be <= 1");
        }
    }

    #[test]
    fn test_delta_put_bounds() {
        // Put delta ∈ [-1, 0]
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let delta = bs.delta(strike, 1.0, false);
            assert!(delta >= -1.0, "Put delta should be >= -1");
            assert!(delta <= 0.0, "Put delta should be <= 0");
        }
    }

    #[test]
    fn test_delta_call_put_relationship() {
        // Put delta = Call delta - 1
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let call_delta = bs.delta(100.0, 1.0, true);
        let put_delta = bs.delta(100.0, 1.0, false);
        assert_relative_eq!(put_delta, call_delta - 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_gamma_non_negative() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let gamma = bs.gamma(strike, 1.0);
            assert!(gamma >= 0.0, "Gamma should be non-negative");
        }
    }

    #[test]
    fn test_vega_non_negative() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let vega = bs.vega(strike, 1.0);
            assert!(vega >= 0.0, "Vega should be non-negative");
        }
    }

    #[test]
    fn test_theta_call_typically_negative() {
        // For most cases, theta is negative (time decay)
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let theta = bs.theta(100.0, 1.0, true);
        assert!(theta < 0.0, "Call theta should typically be negative");
    }

    #[test]
    fn test_rho_call_positive() {
        // Call rho is typically positive
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let rho = bs.rho(100.0, 1.0, true);
        assert!(rho > 0.0, "Call rho should be positive");
    }

    #[test]
    fn test_rho_put_negative() {
        // Put rho is typically negative
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let rho = bs.rho(100.0, 1.0, false);
        assert!(rho < 0.0, "Put rho should be negative");
    }

    // ==========================================================
    // Clone and Debug Tests
    // ==========================================================

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

    // ==========================================================
    // f32 Compatibility Tests
    // ==========================================================

    #[test]
    fn test_f32_compatibility() {
        let bs = BlackScholes::new(100.0_f32, 0.05_f32, 0.2_f32).unwrap();
        let call = bs.price_call(100.0_f32, 1.0_f32);
        assert!(call > 0.0_f32);
    }
}
