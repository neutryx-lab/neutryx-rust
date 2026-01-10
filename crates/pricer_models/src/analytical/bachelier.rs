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

use super::distributions::{norm_cdf, norm_pdf};
use super::error::AnalyticalError;
use crate::instruments::{ExerciseStyle, PayoffType, VanillaOption};

/// Bachelier (normal) model for European option pricing.
///
/// Provides closed-form pricing for European options under normal dynamics.
/// Unlike Black-Scholes, this model supports negative forward prices,
/// making it suitable for interest rate markets.
///
/// # Type Parameters
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Examples
/// ```
/// use pricer_models::analytical::Bachelier;
///
/// let model = Bachelier::new(0.01_f64, 0.005).unwrap();
/// let call_price = model.price_call(0.01, 1.0);
/// let put_price = model.price_put(0.01, 1.0);
///
/// // Put-call parity: C - P = F - K
/// let parity = call_price - put_price - (0.01 - 0.01);
/// assert!(parity.abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct Bachelier<T: Float> {
    /// Forward price (F) - can be negative
    forward: T,
    /// Volatility (σ) - must be positive
    volatility: T,
}

impl<T: Float> Bachelier<T> {
    /// Creates a new Bachelier model.
    ///
    /// # Arguments
    /// * `forward` - Forward price (can be negative for interest rates)
    /// * `volatility` - Normal volatility (must be positive)
    ///
    /// # Errors
    /// - `AnalyticalError::InvalidVolatility` if volatility <= 0
    ///
    /// # Examples
    /// ```
    /// use pricer_models::analytical::Bachelier;
    ///
    /// // Positive forward
    /// let model = Bachelier::new(0.03_f64, 0.01).unwrap();
    ///
    /// // Negative forward (interest rates)
    /// let model_neg = Bachelier::new(-0.005_f64, 0.01).unwrap();
    ///
    /// // Invalid volatility
    /// assert!(Bachelier::new(0.01_f64, 0.0).is_err());
    /// ```
    pub fn new(forward: T, volatility: T) -> Result<Self, AnalyticalError> {
        let zero = T::zero();

        if volatility <= zero {
            return Err(AnalyticalError::InvalidVolatility {
                volatility: volatility.to_f64().unwrap_or(0.0),
            });
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

    /// Computes the d term of the Bachelier formula.
    ///
    /// d = (F - K) / (σ√T)
    ///
    /// # Arguments
    /// * `strike` - Strike price (K)
    /// * `expiry` - Time to expiration in years (T)
    ///
    /// # Returns
    /// The d term.
    #[inline]
    fn d(&self, strike: T, expiry: T) -> T {
        let epsilon = T::from(1e-10).unwrap();

        if expiry <= epsilon {
            let zero = T::zero();
            let large = T::from(100.0).unwrap();
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

    /// Computes European call option price under normal dynamics.
    ///
    /// C = (F - K)·N(d) + σ√T·φ(d)
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
    /// use pricer_models::analytical::Bachelier;
    ///
    /// let model = Bachelier::new(0.03_f64, 0.01).unwrap();
    /// let price = model.price_call(0.03, 1.0);
    ///
    /// // ATM call should have positive value
    /// assert!(price > 0.0);
    /// ```
    #[inline]
    pub fn price_call(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();
        let epsilon = T::from(1e-10).unwrap();

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

    /// Computes European put option price under normal dynamics.
    ///
    /// P = (K - F)·N(-d) + σ√T·φ(d)
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
    /// use pricer_models::analytical::Bachelier;
    ///
    /// let model = Bachelier::new(0.03_f64, 0.01).unwrap();
    /// let price = model.price_put(0.03, 1.0);
    ///
    /// // ATM put should have positive value
    /// assert!(price > 0.0);
    /// ```
    #[inline]
    pub fn price_put(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();
        let epsilon = T::from(1e-10).unwrap();

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

    /// Prices a VanillaOption using Bachelier model.
    ///
    /// Extracts strike, expiry, and payoff type from the option
    /// and applies notional scaling.
    ///
    /// # Arguments
    /// * `option` - The vanilla option to price
    ///
    /// # Returns
    /// The option price scaled by notional, or an error if the
    /// exercise style is not European.
    ///
    /// # Errors
    /// - `AnalyticalError::UnsupportedExerciseStyle` if not European
    ///
    /// # Examples
    /// ```
    /// use pricer_models::analytical::Bachelier;
    /// use pricer_models::instruments::{VanillaOption, InstrumentParams, PayoffType, ExerciseStyle};
    ///
    /// let model = Bachelier::new(0.03_f64, 0.01).unwrap();
    /// let params = InstrumentParams::new(0.03, 1.0, 10_000_000.0).unwrap();
    /// let option = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
    ///
    /// let price = model.price_option(&option).unwrap();
    /// assert!(price > 0.0);
    /// ```
    pub fn price_option(&self, option: &VanillaOption<T>) -> Result<T, AnalyticalError> {
        // Verify exercise style is European
        if !option.exercise_style().is_european() {
            let style = if option.exercise_style().is_american() {
                "American".to_string()
            } else if option.exercise_style().is_bermudan() {
                "Bermudan".to_string()
            } else if option.exercise_style().is_asian() {
                "Asian".to_string()
            } else {
                "non-European".to_string()
            };
            return Err(AnalyticalError::UnsupportedExerciseStyle { style });
        }

        let strike = option.strike();
        let expiry = option.expiry();
        let notional = option.notional();

        let unit_price = match option.payoff_type() {
            PayoffType::Call => self.price_call(strike, expiry),
            PayoffType::Put => self.price_put(strike, expiry),
            PayoffType::DigitalCall | PayoffType::DigitalPut => {
                return Err(AnalyticalError::UnsupportedExerciseStyle {
                    style: "Digital options require different pricing".to_string(),
                });
            }
        };

        Ok(notional * unit_price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ==========================================================
    // Constructor Tests
    // ==========================================================

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
            AnalyticalError::InvalidVolatility { volatility } => {
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
            AnalyticalError::InvalidVolatility { .. } => {}
            _ => panic!("Expected InvalidVolatility error"),
        }
    }

    // ==========================================================
    // Price Tests
    // ==========================================================

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
        // ATM Bachelier price = σ√T * √(2/π) ≈ σ√T * 0.3989 * √2 = σ√T * 0.5642
        // Actually: ATM call = σ√T * φ(0) = σ√T * 0.3989
        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        let call = model.price_call(0.03, 1.0);
        // ATM: d=0, N(0)=0.5, so C = 0 * 0.5 + σ√T * φ(0) = σ√T * 0.3989...
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

    // ==========================================================
    // Put-Call Parity Tests
    // ==========================================================

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

    // ==========================================================
    // VanillaOption Integration Tests
    // ==========================================================

    #[test]
    fn test_price_option_european_call() {
        use crate::instruments::InstrumentParams;

        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        let params = InstrumentParams::new(0.03, 1.0, 10_000_000.0).unwrap();
        let option = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);

        let price = model.price_option(&option).unwrap();
        let expected = 10_000_000.0 * model.price_call(0.03, 1.0);
        assert_relative_eq!(price, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_price_option_european_put() {
        use crate::instruments::InstrumentParams;

        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        let params = InstrumentParams::new(0.03, 1.0, 10_000_000.0).unwrap();
        let option = VanillaOption::new(params, PayoffType::Put, ExerciseStyle::European, 1e-6);

        let price = model.price_option(&option).unwrap();
        let expected = 10_000_000.0 * model.price_put(0.03, 1.0);
        assert_relative_eq!(price, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_price_option_american_rejected() {
        use crate::instruments::InstrumentParams;

        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        let params = InstrumentParams::new(0.03, 1.0, 10_000_000.0).unwrap();
        let option = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::American, 1e-6);

        let result = model.price_option(&option);
        assert!(result.is_err());
        match result.unwrap_err() {
            AnalyticalError::UnsupportedExerciseStyle { .. } => {}
            _ => panic!("Expected UnsupportedExerciseStyle error"),
        }
    }

    // ==========================================================
    // Clone and Debug Tests
    // ==========================================================

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

    // ==========================================================
    // f32 Compatibility Tests
    // ==========================================================

    #[test]
    fn test_f32_compatibility() {
        let model = Bachelier::new(0.03_f32, 0.01_f32).unwrap();
        let call = model.price_call(0.03_f32, 1.0_f32);
        assert!(call > 0.0_f32);
    }

    // ==========================================================
    // Dual64 AD Compatibility Tests
    // ==========================================================
    //
    // NOTE: Dual64 (num_dual::Dual64) does not implement num_traits::Float,
    // so Bachelier<Dual64> cannot be instantiated with the current trait bounds.
    // To enable AD compatibility, the trait bounds need to be refactored from
    // `T: Float` to a more permissive combination of traits that both f64 and
    // Dual64 satisfy (e.g., DualNum<f64> or a custom Scalar trait).
    //
    // This is tracked as a future enhancement. For now, sensitivity verification
    // is done via finite difference comparisons.
}
