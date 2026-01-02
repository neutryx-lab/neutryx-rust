//! Black-Scholes pricing model for European options.
//!
//! This module provides closed-form solutions using the Black-Scholes-Merton model:
//! - Option prices (Call/Put)
//! - Analytical Greeks (Delta, Gamma, Vega, Theta, Rho)
//!
//! ## Mathematical Background
//!
//! The Black-Scholes model assumes the underlying follows geometric Brownian motion:
//! dS = μS dt + σS dW
//!
//! Under risk-neutral measure, the price of a European call option is:
//! C = S * N(d₁) - K * exp(-r * T) * N(d₂)
//!
//! Where:
//! - d₁ = (ln(S/K) + (r + σ²/2) * T) / (σ * √T)
//! - d₂ = d₁ - σ * √T
//! - N(x) = standard normal CDF

use super::distributions::{norm_cdf, norm_pdf};
use super::error::AnalyticalError;
use crate::instruments::VanillaOption;
use num_traits::{Float, ToPrimitive};

/// Black-Scholes pricing model.
///
/// This struct encapsulates the market parameters needed for Black-Scholes pricing:
/// - Spot price (current underlying price)
/// - Risk-free rate (continuous compounding)
/// - Volatility (annualised, σ > 0)
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
/// use pricer_models::analytical::BlackScholes;
///
/// let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
///
/// // Price a call option with strike 100, expiry 1 year
/// let call_price = bs.price_call(100.0, 1.0);
/// println!("Call price: {:.4}", call_price);
///
/// // Price a put option
/// let put_price = bs.price_put(100.0, 1.0);
/// println!("Put price: {:.4}", put_price);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlackScholes<T: Float> {
    /// Current spot price of the underlying (S > 0).
    spot: T,
    /// Risk-free interest rate (continuous compounding).
    rate: T,
    /// Annualised volatility (σ > 0).
    volatility: T,
}

/// Greeks output structure containing all first-order sensitivities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Greeks<T: Float> {
    /// Price of the option.
    pub price: T,
    /// Sensitivity to spot price (∂V/∂S).
    pub delta: T,
    /// Second derivative with respect to spot (∂²V/∂S²).
    pub gamma: T,
    /// Sensitivity to volatility (∂V/∂σ).
    pub vega: T,
    /// Sensitivity to time decay (∂V/∂T), per year.
    pub theta: T,
    /// Sensitivity to interest rate (∂V/∂r).
    pub rho: T,
}

impl<T: Float + ToPrimitive> BlackScholes<T> {
    /// Creates a new Black-Scholes model instance.
    ///
    /// # Arguments
    ///
    /// * `spot` - Current spot price (must be > 0)
    /// * `rate` - Risk-free interest rate (can be negative)
    /// * `volatility` - Annualised volatility (must be > 0)
    ///
    /// # Returns
    ///
    /// `Ok(BlackScholes)` if parameters are valid, otherwise an error.
    ///
    /// # Errors
    ///
    /// - [`AnalyticalError::InvalidSpot`] if spot <= 0
    /// - [`AnalyticalError::InvalidVolatility`] if volatility <= 0
    pub fn new(spot: T, rate: T, volatility: T) -> Result<Self, AnalyticalError> {
        let zero = T::zero();

        if spot <= zero {
            // Convert to f64 for error message
            let spot_val = spot.to_f64().unwrap_or(0.0);
            return Err(AnalyticalError::InvalidSpot { spot: spot_val });
        }

        if volatility <= zero {
            let vol_val = volatility.to_f64().unwrap_or(0.0);
            return Err(AnalyticalError::InvalidVolatility { volatility: vol_val });
        }

        Ok(Self {
            spot,
            rate,
            volatility,
        })
    }

    /// Returns the spot price.
    #[inline]
    pub fn spot(&self) -> T {
        self.spot
    }

    /// Returns the risk-free rate.
    #[inline]
    pub fn rate(&self) -> T {
        self.rate
    }

    /// Returns the volatility.
    #[inline]
    pub fn volatility(&self) -> T {
        self.volatility
    }

    /// Computes d₁ and d₂ terms for Black-Scholes formula.
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price (K)
    /// * `expiry` - Time to expiry in years (T)
    ///
    /// # Returns
    ///
    /// A tuple `(d1, d2)` where:
    /// - d₁ = (ln(S/K) + (r + σ²/2) * T) / (σ * √T)
    /// - d₂ = d₁ - σ * √T
    ///
    /// # Panics
    ///
    /// This function does not panic but may return NaN/Inf for extreme inputs.
    #[inline]
    pub fn d1_d2(&self, strike: T, expiry: T) -> (T, T) {
        let zero = T::zero();
        let half = T::from(0.5).unwrap();

        // Handle expiry ≈ 0 case
        if expiry <= zero {
            // At expiry, d1 and d2 approach ±∞ depending on moneyness
            let infinity = T::infinity();
            if self.spot > strike {
                return (infinity, infinity);
            } else if self.spot < strike {
                return (-infinity, -infinity);
            } else {
                return (zero, zero);
            }
        }

        let sqrt_t = expiry.sqrt();
        let vol_sqrt_t = self.volatility * sqrt_t;

        // ln(S/K)
        let log_moneyness = (self.spot / strike).ln();

        // (r + σ²/2) * T
        let drift_term = (self.rate + half * self.volatility * self.volatility) * expiry;

        // d₁ = (ln(S/K) + drift_term) / (σ * √T)
        let d1 = (log_moneyness + drift_term) / vol_sqrt_t;

        // d₂ = d₁ - σ * √T
        let d2 = d1 - vol_sqrt_t;

        (d1, d2)
    }

    /// Computes the price of a European call option.
    ///
    /// # Formula
    ///
    /// C = S * N(d₁) - K * exp(-r * T) * N(d₂)
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
    /// use pricer_models::analytical::BlackScholes;
    ///
    /// let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
    /// let price = bs.price_call(100.0, 1.0);
    /// assert!(price > 0.0);
    /// ```
    pub fn price_call(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();

        // Handle expiry ≈ 0: intrinsic value
        if expiry <= zero {
            return (self.spot - strike).max(zero);
        }

        let (d1, d2) = self.d1_d2(strike, expiry);

        // Discount factor: exp(-r * T)
        let df = (-self.rate * expiry).exp();

        // C = S * N(d₁) - K * exp(-r * T) * N(d₂)
        self.spot * norm_cdf(d1) - strike * df * norm_cdf(d2)
    }

    /// Computes the price of a European put option.
    ///
    /// # Formula
    ///
    /// P = K * exp(-r * T) * N(-d₂) - S * N(-d₁)
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
    /// use pricer_models::analytical::BlackScholes;
    ///
    /// let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
    /// let price = bs.price_put(100.0, 1.0);
    /// assert!(price > 0.0);
    /// ```
    pub fn price_put(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();

        // Handle expiry ≈ 0: intrinsic value
        if expiry <= zero {
            return (strike - self.spot).max(zero);
        }

        let (d1, d2) = self.d1_d2(strike, expiry);

        // Discount factor: exp(-r * T)
        let df = (-self.rate * expiry).exp();

        // P = K * exp(-r * T) * N(-d₂) - S * N(-d₁)
        strike * df * norm_cdf(-d2) - self.spot * norm_cdf(-d1)
    }

    /// Computes the Delta of an option.
    ///
    /// Delta measures the rate of change of the option value with respect to
    /// changes in the underlying asset's price.
    ///
    /// # Formula
    ///
    /// - Call Delta = N(d₁)
    /// - Put Delta = N(d₁) - 1
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    /// * `is_call` - `true` for call option, `false` for put option
    ///
    /// # Returns
    ///
    /// The delta value: [0, 1] for calls, [-1, 0] for puts.
    pub fn delta(&self, strike: T, expiry: T, is_call: bool) -> T {
        let zero = T::zero();
        let one = T::one();

        if expiry <= zero {
            // At expiry: delta = 1 if ITM, 0 if OTM
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
        } else {
            let (d1, _) = self.d1_d2(strike, expiry);
            let n_d1 = norm_cdf(d1);

            if is_call {
                n_d1
            } else {
                n_d1 - one
            }
        }
    }

    /// Computes the Gamma of an option.
    ///
    /// Gamma measures the rate of change of delta with respect to changes in
    /// the underlying price. It is the same for both calls and puts.
    ///
    /// # Formula
    ///
    /// Γ = φ(d₁) / (S * σ * √T)
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// The gamma value (always non-negative).
    pub fn gamma(&self, strike: T, expiry: T) -> T {
        let zero = T::zero();

        if expiry <= zero {
            // At expiry, gamma is theoretically infinite at ATM, zero elsewhere
            // We return zero for numerical stability
            return zero;
        }

        let (d1, _) = self.d1_d2(strike, expiry);
        let sqrt_t = expiry.sqrt();

        // Γ = φ(d₁) / (S * σ * √T)
        norm_pdf(d1) / (self.spot * self.volatility * sqrt_t)
    }

    /// Computes the Vega of an option.
    ///
    /// Vega measures the sensitivity of the option value to changes in volatility.
    /// It is the same for both calls and puts.
    ///
    /// # Formula
    ///
    /// ν = S * √T * φ(d₁)
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

        let (d1, _) = self.d1_d2(strike, expiry);
        let sqrt_t = expiry.sqrt();

        // ν = S * √T * φ(d₁)
        self.spot * sqrt_t * norm_pdf(d1)
    }

    /// Computes the Theta of an option (annualised).
    ///
    /// Theta measures the rate of change of the option value with respect to time.
    /// This returns the annualised theta (per year decay).
    ///
    /// # Formula
    ///
    /// For call:
    /// Θ = -(S * σ * φ(d₁)) / (2 * √T) - r * K * exp(-r * T) * N(d₂)
    ///
    /// For put:
    /// Θ = -(S * σ * φ(d₁)) / (2 * √T) + r * K * exp(-r * T) * N(-d₂)
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    /// * `is_call` - `true` for call option, `false` for put option
    ///
    /// # Returns
    ///
    /// The theta value (typically negative, indicating time decay).
    pub fn theta(&self, strike: T, expiry: T, is_call: bool) -> T {
        let zero = T::zero();
        let two = T::from(2.0).unwrap();

        if expiry <= zero {
            return zero;
        }

        let (d1, d2) = self.d1_d2(strike, expiry);
        let sqrt_t = expiry.sqrt();
        let df = (-self.rate * expiry).exp();

        // Common term: -(S * σ * φ(d₁)) / (2 * √T)
        let common = -(self.spot * self.volatility * norm_pdf(d1)) / (two * sqrt_t);

        if is_call {
            // Θ_call = common - r * K * exp(-r * T) * N(d₂)
            common - self.rate * strike * df * norm_cdf(d2)
        } else {
            // Θ_put = common + r * K * exp(-r * T) * N(-d₂)
            common + self.rate * strike * df * norm_cdf(-d2)
        }
    }

    /// Computes the Rho of an option.
    ///
    /// Rho measures the sensitivity of the option value to changes in the
    /// risk-free interest rate.
    ///
    /// # Formula
    ///
    /// For call: ρ = K * T * exp(-r * T) * N(d₂)
    /// For put:  ρ = -K * T * exp(-r * T) * N(-d₂)
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    /// * `is_call` - `true` for call option, `false` for put option
    ///
    /// # Returns
    ///
    /// The rho value.
    pub fn rho(&self, strike: T, expiry: T, is_call: bool) -> T {
        let zero = T::zero();

        if expiry <= zero {
            return zero;
        }

        let (_, d2) = self.d1_d2(strike, expiry);
        let df = (-self.rate * expiry).exp();

        if is_call {
            // ρ_call = K * T * exp(-r * T) * N(d₂)
            strike * expiry * df * norm_cdf(d2)
        } else {
            // ρ_put = -K * T * exp(-r * T) * N(-d₂)
            -strike * expiry * df * norm_cdf(-d2)
        }
    }

    /// Computes all Greeks at once for efficiency.
    ///
    /// This method computes the price and all first-order Greeks in a single
    /// pass, avoiding redundant calculations of d₁, d₂, and related terms.
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    /// * `is_call` - `true` for call option, `false` for put option
    ///
    /// # Returns
    ///
    /// A [`Greeks`] struct containing price and all sensitivities.
    pub fn greeks(&self, strike: T, expiry: T, is_call: bool) -> Greeks<T> {
        let zero = T::zero();
        let one = T::one();
        let two = T::from(2.0).unwrap();

        if expiry <= zero {
            // At expiry: return intrinsic value and limit Greeks
            let intrinsic = if is_call {
                (self.spot - strike).max(zero)
            } else {
                (strike - self.spot).max(zero)
            };

            let delta = if is_call {
                if self.spot > strike {
                    one
                } else {
                    zero
                }
            } else if self.spot < strike {
                -one
            } else {
                zero
            };

            return Greeks {
                price: intrinsic,
                delta,
                gamma: zero,
                vega: zero,
                theta: zero,
                rho: zero,
            };
        }

        let (d1, d2) = self.d1_d2(strike, expiry);
        let sqrt_t = expiry.sqrt();
        let df = (-self.rate * expiry).exp();

        let n_d1 = norm_cdf(d1);
        let n_d2 = norm_cdf(d2);
        let n_neg_d1 = norm_cdf(-d1);
        let n_neg_d2 = norm_cdf(-d2);
        let phi_d1 = norm_pdf(d1);

        let vol_sqrt_t = self.volatility * sqrt_t;

        // Price
        let price = if is_call {
            self.spot * n_d1 - strike * df * n_d2
        } else {
            strike * df * n_neg_d2 - self.spot * n_neg_d1
        };

        // Delta
        let delta = if is_call { n_d1 } else { n_d1 - one };

        // Gamma (same for call and put)
        let gamma = phi_d1 / (self.spot * vol_sqrt_t);

        // Vega (same for call and put)
        let vega = self.spot * sqrt_t * phi_d1;

        // Theta
        let common_theta = -(self.spot * self.volatility * phi_d1) / (two * sqrt_t);
        let theta = if is_call {
            common_theta - self.rate * strike * df * n_d2
        } else {
            common_theta + self.rate * strike * df * n_neg_d2
        };

        // Rho
        let rho = if is_call {
            strike * expiry * df * n_d2
        } else {
            -strike * expiry * df * n_neg_d2
        };

        Greeks {
            price,
            delta,
            gamma,
            vega,
            theta,
            rho,
        }
    }

    /// Prices a VanillaOption using the Black-Scholes model.
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
                style: "Black-Scholes only supports European options".to_string(),
            });
        }

        let strike = option.strike();
        let expiry = option.expiry();
        let notional = option.notional();

        let price = match option.payoff_type() {
            PayoffType::Call => self.price_call(strike, expiry),
            PayoffType::Put => self.price_put(strike, expiry),
            PayoffType::DigitalCall | PayoffType::DigitalPut => {
                // Digital options are not directly supported by standard BS
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
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2);
        assert!(bs.is_ok());

        let bs = bs.unwrap();
        assert_relative_eq!(bs.spot(), 100.0, epsilon = 1e-10);
        assert_relative_eq!(bs.rate(), 0.05, epsilon = 1e-10);
        assert_relative_eq!(bs.volatility(), 0.2, epsilon = 1e-10);
    }

    #[test]
    fn test_new_negative_rate() {
        // Negative rates should be allowed
        let bs = BlackScholes::new(100.0_f64, -0.01, 0.2);
        assert!(bs.is_ok());
    }

    #[test]
    fn test_new_invalid_spot() {
        let bs = BlackScholes::new(0.0_f64, 0.05, 0.2);
        assert!(matches!(bs, Err(AnalyticalError::InvalidSpot { .. })));

        let bs = BlackScholes::new(-10.0_f64, 0.05, 0.2);
        assert!(matches!(bs, Err(AnalyticalError::InvalidSpot { .. })));
    }

    #[test]
    fn test_new_invalid_volatility() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.0);
        assert!(matches!(bs, Err(AnalyticalError::InvalidVolatility { .. })));

        let bs = BlackScholes::new(100.0_f64, 0.05, -0.2);
        assert!(matches!(bs, Err(AnalyticalError::InvalidVolatility { .. })));
    }

    // ========================================================================
    // d1/d2 Tests
    // ========================================================================

    #[test]
    fn test_d1_d2_atm() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let (d1, d2) = bs.d1_d2(100.0, 1.0);

        // For ATM: ln(S/K) = 0
        // d1 = (0 + (0.05 + 0.02) * 1) / (0.2 * 1) = 0.07 / 0.2 = 0.35
        // d2 = 0.35 - 0.2 = 0.15
        assert_relative_eq!(d1, 0.35, epsilon = 1e-10);
        assert_relative_eq!(d2, 0.15, epsilon = 1e-10);
    }

    #[test]
    fn test_d1_d2_relationship() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let (d1, d2) = bs.d1_d2(110.0, 0.5);

        // d2 = d1 - σ * √T
        let vol_sqrt_t = 0.2 * 0.5_f64.sqrt();
        assert_relative_eq!(d2, d1 - vol_sqrt_t, epsilon = 1e-10);
    }

    #[test]
    fn test_d1_d2_zero_expiry() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();

        // ITM
        let (d1, d2) = bs.d1_d2(90.0, 0.0);
        assert!(d1.is_infinite() && d1 > 0.0);
        assert!(d2.is_infinite() && d2 > 0.0);

        // OTM
        let (d1, d2) = bs.d1_d2(110.0, 0.0);
        assert!(d1.is_infinite() && d1 < 0.0);
        assert!(d2.is_infinite() && d2 < 0.0);

        // ATM
        let (d1, d2) = bs.d1_d2(100.0, 0.0);
        assert_relative_eq!(d1, 0.0, epsilon = 1e-10);
        assert_relative_eq!(d2, 0.0, epsilon = 1e-10);
    }

    // ========================================================================
    // Price Tests
    // ========================================================================

    #[test]
    fn test_price_call_atm() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let price = bs.price_call(100.0, 1.0);

        // Expected price approximately 10.45 (from standard BS tables)
        assert!(price > 10.0 && price < 11.0);
    }

    #[test]
    fn test_price_put_atm() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let price = bs.price_put(100.0, 1.0);

        // Expected price approximately 5.57 (from put-call parity)
        assert!(price > 5.0 && price < 6.5);
    }

    #[test]
    fn test_price_call_itm() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let price = bs.price_call(80.0, 1.0);

        // Deep ITM call should be worth at least intrinsic value
        assert!(price > 20.0);
    }

    #[test]
    fn test_price_put_itm() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let price = bs.price_put(120.0, 1.0);

        // Deep ITM put should be worth at least intrinsic value
        assert!(price > 15.0);
    }

    #[test]
    fn test_price_zero_expiry_call_itm() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let price = bs.price_call(90.0, 0.0);

        // At expiry, call = max(S - K, 0) = 10
        assert_relative_eq!(price, 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_price_zero_expiry_put_itm() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let price = bs.price_put(110.0, 0.0);

        // At expiry, put = max(K - S, 0) = 10
        assert_relative_eq!(price, 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_price_zero_expiry_otm() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();

        let call = bs.price_call(110.0, 0.0);
        assert_relative_eq!(call, 0.0, epsilon = 1e-10);

        let put = bs.price_put(90.0, 0.0);
        assert_relative_eq!(put, 0.0, epsilon = 1e-10);
    }

    // ========================================================================
    // Put-Call Parity Tests
    // ========================================================================

    #[test]
    fn test_put_call_parity() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let strike = 100.0;
        let expiry = 1.0;

        let call = bs.price_call(strike, expiry);
        let put = bs.price_put(strike, expiry);

        // C - P = S - K * exp(-r * T)
        let parity_rhs = bs.spot() - strike * (-bs.rate() * expiry).exp();
        let parity_lhs = call - put;

        assert_relative_eq!(parity_lhs, parity_rhs, epsilon = 1e-10);
    }

    #[test]
    fn test_put_call_parity_various_strikes() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let expiry = 0.5;

        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let call = bs.price_call(strike, expiry);
            let put = bs.price_put(strike, expiry);

            let parity_rhs = bs.spot() - strike * (-bs.rate() * expiry).exp();
            let parity_lhs = call - put;

            assert_relative_eq!(parity_lhs, parity_rhs, epsilon = 1e-9);
        }
    }

    // ========================================================================
    // Greeks Tests
    // ========================================================================

    #[test]
    fn test_delta_call_bounds() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let delta = bs.delta(100.0, 1.0, true);

        // Call delta should be in [0, 1]
        assert!(delta >= 0.0 && delta <= 1.0);
    }

    #[test]
    fn test_delta_put_bounds() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let delta = bs.delta(100.0, 1.0, false);

        // Put delta should be in [-1, 0]
        assert!(delta >= -1.0 && delta <= 0.0);
    }

    #[test]
    fn test_delta_call_put_relationship() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let call_delta = bs.delta(100.0, 1.0, true);
        let put_delta = bs.delta(100.0, 1.0, false);

        // Put delta = Call delta - 1
        assert_relative_eq!(put_delta, call_delta - 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_gamma_non_negative() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();

        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let gamma = bs.gamma(strike, 1.0);
            assert!(gamma >= 0.0);
        }
    }

    #[test]
    fn test_gamma_atm_highest() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();

        let gamma_atm = bs.gamma(100.0, 1.0);
        let gamma_otm = bs.gamma(120.0, 1.0);
        let gamma_itm = bs.gamma(80.0, 1.0);

        // Gamma is highest at ATM
        assert!(gamma_atm > gamma_otm);
        assert!(gamma_atm > gamma_itm);
    }

    #[test]
    fn test_vega_non_negative() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();

        for strike in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let vega = bs.vega(strike, 1.0);
            assert!(vega >= 0.0);
        }
    }

    #[test]
    fn test_theta_call_typically_negative() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let theta = bs.theta(100.0, 1.0, true);

        // ATM call theta is typically negative (time decay)
        assert!(theta < 0.0);
    }

    #[test]
    fn test_greeks_all_at_once() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let greeks = bs.greeks(100.0, 1.0, true);

        // Price should match individual call price
        let call_price = bs.price_call(100.0, 1.0);
        assert_relative_eq!(greeks.price, call_price, epsilon = 1e-10);

        // Delta should match individual delta
        let delta = bs.delta(100.0, 1.0, true);
        assert_relative_eq!(greeks.delta, delta, epsilon = 1e-10);

        // Gamma should match individual gamma
        let gamma = bs.gamma(100.0, 1.0);
        assert_relative_eq!(greeks.gamma, gamma, epsilon = 1e-10);
    }

    // ========================================================================
    // Greeks vs Finite Difference Verification
    // ========================================================================

    #[test]
    fn test_delta_vs_finite_difference() {
        let spot = 100.0_f64;
        let bs = BlackScholes::new(spot, 0.05, 0.2).unwrap();
        let strike = 100.0;
        let expiry = 1.0;

        let analytical_delta = bs.delta(strike, expiry, true);

        // Finite difference approximation
        let h = 0.01;
        let bs_up = BlackScholes::new(spot + h, 0.05, 0.2).unwrap();
        let bs_down = BlackScholes::new(spot - h, 0.05, 0.2).unwrap();

        let fd_delta = (bs_up.price_call(strike, expiry) - bs_down.price_call(strike, expiry))
            / (2.0 * h);

        assert_relative_eq!(analytical_delta, fd_delta, epsilon = 1e-4);
    }

    #[test]
    fn test_gamma_vs_finite_difference() {
        let spot = 100.0_f64;
        let bs = BlackScholes::new(spot, 0.05, 0.2).unwrap();
        let strike = 100.0;
        let expiry = 1.0;

        let analytical_gamma = bs.gamma(strike, expiry);

        // Finite difference approximation for second derivative
        let h = 0.01;
        let bs_up = BlackScholes::new(spot + h, 0.05, 0.2).unwrap();
        let bs_down = BlackScholes::new(spot - h, 0.05, 0.2).unwrap();

        let price_up = bs_up.price_call(strike, expiry);
        let price = bs.price_call(strike, expiry);
        let price_down = bs_down.price_call(strike, expiry);

        let fd_gamma = (price_up - 2.0 * price + price_down) / (h * h);

        assert_relative_eq!(analytical_gamma, fd_gamma, epsilon = 1e-4);
    }

    #[test]
    fn test_vega_vs_finite_difference() {
        let vol = 0.2_f64;
        let bs = BlackScholes::new(100.0, 0.05, vol).unwrap();
        let strike = 100.0;
        let expiry = 1.0;

        let analytical_vega = bs.vega(strike, expiry);

        // Finite difference approximation
        let h = 0.001;
        let bs_up = BlackScholes::new(100.0, 0.05, vol + h).unwrap();
        let bs_down = BlackScholes::new(100.0, 0.05, vol - h).unwrap();

        let fd_vega = (bs_up.price_call(strike, expiry) - bs_down.price_call(strike, expiry))
            / (2.0 * h);

        assert_relative_eq!(analytical_vega, fd_vega, epsilon = 1e-3);
    }

    // ========================================================================
    // f32 Compatibility Tests
    // ========================================================================

    #[test]
    fn test_f32_compatibility() {
        let bs = BlackScholes::new(100.0_f32, 0.05, 0.2).unwrap();

        let call = bs.price_call(100.0, 1.0);
        let put = bs.price_put(100.0, 1.0);

        assert!(call > 0.0);
        assert!(put > 0.0);
    }

    // ========================================================================
    // Clone/Copy Tests
    // ========================================================================

    #[test]
    fn test_clone() {
        let bs1 = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let bs2 = bs1;

        assert_relative_eq!(bs1.spot(), bs2.spot(), epsilon = 1e-10);
    }

    #[test]
    fn test_debug() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let debug_str = format!("{:?}", bs);
        assert!(debug_str.contains("BlackScholes"));
    }
}
