//! Bachelier (Normal) pricing model for European options.
//!
//! The Bachelier model assumes the underlying follows arithmetic Brownian motion:
//! dS = μ dt + σ dW
//!
//! This model is particularly useful for:
//! - Interest rate derivatives where rates can go negative
//! - Spread options
//! - Situations where lognormal dynamics are inappropriate
//!
//! ## Mathematical Background
//!
//! Under risk-neutral measure, the price of a European call option is:
//! C = (F - K) * N(d) + σ * √T * φ(d)
//!
//! Where:
//! - d = (F - K) / (σ * √T)
//! - F = forward price
//! - N(x) = standard normal CDF
//! - φ(x) = standard normal PDF

use super::distributions::{norm_cdf, norm_pdf};
use super::error::AnalyticalError;
use crate::instruments::VanillaOption;
use num_traits::{Float, ToPrimitive};

/// Bachelier (Normal) pricing model.
///
/// This struct encapsulates the parameters needed for Bachelier pricing:
/// - Forward price (can be negative, e.g., for rates)
/// - Volatility (annualised, σ > 0, in absolute terms)
///
/// ## Generic Parameter
///
/// The type parameter `T: Float` allows use with:
/// - `f64` for standard pricing
/// - `Dual64` for automatic differentiation
///
/// ## Example
///
/// ```
/// use pricer_models::analytical::Bachelier;
///
/// let bach = Bachelier::new(100.0_f64, 10.0).unwrap();
///
/// // Price a call option with strike 100, expiry 1 year
/// let call_price = bach.price_call(100.0, 1.0);
/// println!("Call price: {:.4}", call_price);
///
/// // Price a put option
/// let put_price = bach.price_put(100.0, 1.0);
/// println!("Put price: {:.4}", put_price);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bachelier<T: Float> {
    /// Forward price (can be negative for rate products).
    forward: T,
    /// Annualised volatility (σ > 0, in absolute terms).
    volatility: T,
}

impl<T: Float + ToPrimitive> Bachelier<T> {
    /// Creates a new Bachelier model instance.
    ///
    /// # Arguments
    ///
    /// * `forward` - Forward price (can be negative)
    /// * `volatility` - Annualised volatility in absolute terms (must be > 0)
    ///
    /// # Returns
    ///
    /// `Ok(Bachelier)` if parameters are valid, otherwise an error.
    ///
    /// # Errors
    ///
    /// - [`AnalyticalError::InvalidVolatility`] if volatility <= 0
    pub fn new(forward: T, volatility: T) -> Result<Self, AnalyticalError> {
        let zero = T::zero();

        if volatility <= zero {
            let vol_val = volatility.to_f64().unwrap_or(0.0);
            return Err(AnalyticalError::InvalidVolatility { volatility: vol_val });
        }

        Ok(Self { forward, volatility })
    }

    /// Returns the forward price.
    #[inline]
    pub fn forward(&self) -> T {
        self.forward
    }

    /// Returns the volatility.
    #[inline]
    pub fn volatility(&self) -> T {
        self.volatility
    }

    /// Computes the d term for Bachelier formula.
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price (K)
    /// * `expiry` - Time to expiry in years (T)
    ///
    /// # Returns
    ///
    /// d = (F - K) / (σ * √T)
    #[inline]
    pub fn d(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();

        if expiry <= zero {
            // At expiry, d approaches ±∞ depending on moneyness
            let infinity = T::infinity();
            if self.forward > strike {
                return infinity;
            } else if self.forward < strike {
                return -infinity;
            } else {
                return zero;
            }
        }

        let sqrt_t = expiry.sqrt();
        let vol_sqrt_t = self.volatility * sqrt_t;

        (self.forward - strike) / vol_sqrt_t
    }

    /// Computes the price of a European call option using Bachelier model.
    ///
    /// # Formula
    ///
    /// C = (F - K) * N(d) + σ * √T * φ(d)
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// The call option price.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_models::analytical::Bachelier;
    ///
    /// let bach = Bachelier::new(100.0_f64, 10.0).unwrap();
    /// let price = bach.price_call(100.0, 1.0);
    /// assert!(price > 0.0);
    /// ```
    pub fn price_call(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();

        // Handle expiry ≈ 0: intrinsic value
        if expiry <= zero {
            return (self.forward - strike).max(zero);
        }

        let d = self.d(strike, expiry);
        let sqrt_t = expiry.sqrt();
        let vol_sqrt_t = self.volatility * sqrt_t;

        // C = (F - K) * N(d) + σ * √T * φ(d)
        (self.forward - strike) * norm_cdf(d) + vol_sqrt_t * norm_pdf(d)
    }

    /// Computes the price of a European put option using Bachelier model.
    ///
    /// # Formula
    ///
    /// P = (K - F) * N(-d) + σ * √T * φ(d)
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// The put option price.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_models::analytical::Bachelier;
    ///
    /// let bach = Bachelier::new(100.0_f64, 10.0).unwrap();
    /// let price = bach.price_put(100.0, 1.0);
    /// assert!(price > 0.0);
    /// ```
    pub fn price_put(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();

        // Handle expiry ≈ 0: intrinsic value
        if expiry <= zero {
            return (strike - self.forward).max(zero);
        }

        let d = self.d(strike, expiry);
        let sqrt_t = expiry.sqrt();
        let vol_sqrt_t = self.volatility * sqrt_t;

        // P = (K - F) * N(-d) + σ * √T * φ(d)
        // Note: φ(d) = φ(-d) due to symmetry
        (strike - self.forward) * norm_cdf(-d) + vol_sqrt_t * norm_pdf(d)
    }

    /// Computes the Delta of an option.
    ///
    /// For Bachelier model:
    /// - Call Delta = N(d)
    /// - Put Delta = N(d) - 1 = -N(-d)
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    /// * `is_call` - `true` for call option, `false` for put option
    ///
    /// # Returns
    ///
    /// The delta value.
    pub fn delta(&self, strike: T, expiry: T, is_call: bool) -> T {
        let zero = T::zero();
        let one = T::one();

        if expiry <= zero {
            if is_call {
                if self.forward > strike {
                    one
                } else {
                    zero
                }
            } else if self.forward < strike {
                -one
            } else {
                zero
            }
        } else {
            let d = self.d(strike, expiry);
            let n_d = norm_cdf(d);

            if is_call {
                n_d
            } else {
                n_d - one
            }
        }
    }

    /// Computes the Vega of an option.
    ///
    /// For Bachelier model:
    /// Vega = √T * φ(d)
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// The vega value (always non-negative).
    pub fn vega(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();

        if expiry <= zero {
            return zero;
        }

        let d = self.d(strike, expiry);
        let sqrt_t = expiry.sqrt();

        sqrt_t * norm_pdf(d)
    }

    /// Prices a VanillaOption using the Bachelier model.
    ///
    /// This method integrates with the instrument definitions from `pricer_models`,
    /// extracting strike, expiry, and payoff type from the option struct.
    ///
    /// # Arguments
    ///
    /// * `option` - A reference to a VanillaOption
    ///
    /// # Returns
    ///
    /// `Ok(price)` on success, or an error if the exercise style is not European.
    ///
    /// # Errors
    ///
    /// - [`AnalyticalError::UnsupportedExerciseStyle`] if the option is not European
    pub fn price_option(&self, option: &VanillaOption<T>) -> Result<T, AnalyticalError> {
        use crate::instruments::{ExerciseStyle, PayoffType};

        // Check for European exercise
        if !matches!(option.exercise_style(), ExerciseStyle::European) {
            return Err(AnalyticalError::UnsupportedExerciseStyle {
                style: "Bachelier only supports European options".to_string(),
            });
        }

        let strike = option.strike();
        let expiry = option.expiry();
        let notional = option.notional();

        let price = match option.payoff_type() {
            PayoffType::Call => self.price_call(strike, expiry),
            PayoffType::Put => self.price_put(strike, expiry),
            PayoffType::DigitalCall | PayoffType::DigitalPut => {
                return Err(AnalyticalError::UnsupportedExerciseStyle {
                    style: "Digital options require specialised pricing".to_string(),
                });
            }
        };

        Ok(price * notional)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ========================================================================
    // Constructor Tests
    // ========================================================================

    #[test]
    fn test_new_valid() {
        let bach = Bachelier::new(100.0_f64, 10.0);
        assert!(bach.is_ok());

        let bach = bach.unwrap();
        assert_relative_eq!(bach.forward(), 100.0, epsilon = 1e-10);
        assert_relative_eq!(bach.volatility(), 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_new_negative_forward() {
        // Negative forward should be allowed (useful for rates)
        let bach = Bachelier::new(-0.5_f64, 0.01);
        assert!(bach.is_ok());
    }

    #[test]
    fn test_new_invalid_volatility() {
        let bach = Bachelier::new(100.0_f64, 0.0);
        assert!(matches!(bach, Err(AnalyticalError::InvalidVolatility { .. })));

        let bach = Bachelier::new(100.0_f64, -10.0);
        assert!(matches!(bach, Err(AnalyticalError::InvalidVolatility { .. })));
    }

    // ========================================================================
    // Price Tests
    // ========================================================================

    #[test]
    fn test_price_call_atm() {
        let bach = Bachelier::new(100.0_f64, 10.0).unwrap();
        let price = bach.price_call(100.0, 1.0);

        // ATM Bachelier call price = σ * √T * φ(0) = 10 * 1 * 0.3989... ≈ 3.989
        assert_relative_eq!(price, 3.989_422_804, epsilon = 1e-6);
    }

    #[test]
    fn test_price_put_atm() {
        let bach = Bachelier::new(100.0_f64, 10.0).unwrap();
        let price = bach.price_put(100.0, 1.0);

        // ATM put = ATM call for Bachelier (put-call parity: C - P = F - K = 0)
        assert_relative_eq!(price, 3.989_422_804, epsilon = 1e-6);
    }

    #[test]
    fn test_price_call_itm() {
        let bach = Bachelier::new(110.0_f64, 10.0).unwrap();
        let price = bach.price_call(100.0, 1.0);

        // ITM call should be at least intrinsic value
        assert!(price > 10.0);
    }

    #[test]
    fn test_price_put_itm() {
        let bach = Bachelier::new(90.0_f64, 10.0).unwrap();
        let price = bach.price_put(100.0, 1.0);

        // ITM put should be at least intrinsic value
        assert!(price > 10.0);
    }

    #[test]
    fn test_price_zero_expiry_call_itm() {
        let bach = Bachelier::new(110.0_f64, 10.0).unwrap();
        let price = bach.price_call(100.0, 0.0);

        // At expiry, call = max(F - K, 0) = 10
        assert_relative_eq!(price, 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_price_zero_expiry_put_itm() {
        let bach = Bachelier::new(90.0_f64, 10.0).unwrap();
        let price = bach.price_put(100.0, 0.0);

        // At expiry, put = max(K - F, 0) = 10
        assert_relative_eq!(price, 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_price_zero_expiry_otm() {
        let bach = Bachelier::new(90.0_f64, 10.0).unwrap();
        let call = bach.price_call(100.0, 0.0);
        assert_relative_eq!(call, 0.0, epsilon = 1e-10);

        let bach = Bachelier::new(110.0_f64, 10.0).unwrap();
        let put = bach.price_put(100.0, 0.0);
        assert_relative_eq!(put, 0.0, epsilon = 1e-10);
    }

    // ========================================================================
    // Put-Call Parity Tests
    // ========================================================================

    #[test]
    fn test_put_call_parity() {
        let bach = Bachelier::new(100.0_f64, 10.0).unwrap();
        let strike = 100.0;
        let expiry = 1.0;

        let call = bach.price_call(strike, expiry);
        let put = bach.price_put(strike, expiry);

        // Bachelier put-call parity: C - P = F - K
        let parity_rhs = bach.forward() - strike;
        let parity_lhs = call - put;

        assert_relative_eq!(parity_lhs, parity_rhs, epsilon = 1e-10);
    }

    #[test]
    fn test_put_call_parity_various_strikes() {
        let bach = Bachelier::new(100.0_f64, 10.0).unwrap();
        let expiry = 0.5;

        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let call = bach.price_call(strike, expiry);
            let put = bach.price_put(strike, expiry);

            let parity_rhs = bach.forward() - strike;
            let parity_lhs = call - put;

            assert_relative_eq!(parity_lhs, parity_rhs, epsilon = 1e-9);
        }
    }

    #[test]
    fn test_put_call_parity_negative_forward() {
        // Test with negative forward (e.g., negative interest rates)
        let bach = Bachelier::new(-0.5_f64, 0.01).unwrap();
        let strike = -0.3;
        let expiry = 1.0;

        let call = bach.price_call(strike, expiry);
        let put = bach.price_put(strike, expiry);

        let parity_rhs = bach.forward() - strike;
        let parity_lhs = call - put;

        assert_relative_eq!(parity_lhs, parity_rhs, epsilon = 1e-10);
    }

    // ========================================================================
    // Delta Tests
    // ========================================================================

    #[test]
    fn test_delta_call_atm() {
        let bach = Bachelier::new(100.0_f64, 10.0).unwrap();
        let delta = bach.delta(100.0, 1.0, true);

        // ATM delta = N(0) = 0.5
        assert_relative_eq!(delta, 0.5, epsilon = 1e-8);
    }

    #[test]
    fn test_delta_put_atm() {
        let bach = Bachelier::new(100.0_f64, 10.0).unwrap();
        let delta = bach.delta(100.0, 1.0, false);

        // ATM put delta = N(0) - 1 = -0.5
        assert_relative_eq!(delta, -0.5, epsilon = 1e-8);
    }

    #[test]
    fn test_delta_call_put_relationship() {
        let bach = Bachelier::new(100.0_f64, 10.0).unwrap();
        let call_delta = bach.delta(100.0, 1.0, true);
        let put_delta = bach.delta(100.0, 1.0, false);

        // Put delta = Call delta - 1
        assert_relative_eq!(put_delta, call_delta - 1.0, epsilon = 1e-10);
    }

    // ========================================================================
    // Vega Tests
    // ========================================================================

    #[test]
    fn test_vega_non_negative() {
        let bach = Bachelier::new(100.0_f64, 10.0).unwrap();

        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let vega = bach.vega(strike, 1.0);
            assert!(vega >= 0.0);
        }
    }

    #[test]
    fn test_vega_atm_maximum() {
        let bach = Bachelier::new(100.0_f64, 10.0).unwrap();

        let vega_atm = bach.vega(100.0, 1.0);
        let vega_otm = bach.vega(120.0, 1.0);
        let vega_itm = bach.vega(80.0, 1.0);

        // Vega is maximum at ATM
        assert!(vega_atm > vega_otm);
        assert!(vega_atm > vega_itm);
    }

    // ========================================================================
    // f32 Compatibility Tests
    // ========================================================================

    #[test]
    fn test_f32_compatibility() {
        let bach = Bachelier::new(100.0_f32, 10.0).unwrap();

        let call = bach.price_call(100.0, 1.0);
        let put = bach.price_put(100.0, 1.0);

        assert!(call > 0.0);
        assert!(put > 0.0);
    }

    // ========================================================================
    // Clone/Copy Tests
    // ========================================================================

    #[test]
    fn test_clone() {
        let bach1 = Bachelier::new(100.0_f64, 10.0).unwrap();
        let bach2 = bach1;

        assert_relative_eq!(bach1.forward(), bach2.forward(), epsilon = 1e-10);
    }

    #[test]
    fn test_debug() {
        let bach = Bachelier::new(100.0_f64, 10.0).unwrap();
        let debug_str = format!("{:?}", bach);
        assert!(debug_str.contains("Bachelier"));
    }
}
