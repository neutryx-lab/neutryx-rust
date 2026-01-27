//! Garman-Kohlhagen model for FX option pricing.
//!
//! This module provides the Garman-Kohlhagen closed-form solution for pricing
//! European FX options. This is the standard model for FX options and extends
//! Black-Scholes to account for two interest rates (domestic and foreign).
//!
//! # Mathematical Background
//!
//! The Garman-Kohlhagen formula prices FX options with:
//! - S: spot exchange rate (domestic per foreign)
//! - K: strike price
//! - rd: domestic risk-free rate (continuous compounding)
//! - rf: foreign risk-free rate (continuous compounding)
//! - σ: volatility of the exchange rate
//! - T: time to maturity in years
//!
//! ## Call Option Price
//! C = S * e^(-rf*T) * N(d1) - K * e^(-rd*T) * N(d2)
//!
//! ## Put Option Price
//! P = K * e^(-rd*T) * N(-d2) - S * e^(-rf*T) * N(-d1)
//!
//! where:
//! d1 = [ln(S/K) + (rd - rf + σ²/2) * T] / (σ * √T)
//! d2 = d1 - σ * √T
//!
//! # Examples
//!
//! ```
//! use pricer_core::math::formulas::garman_kohlhagen::{
//!     GarmanKohlhagen, GarmanKohlhagenParams,
//! };
//!
//! let params = GarmanKohlhagenParams::new(
//!     1.10,   // spot
//!     1.12,   // strike
//!     0.03,   // domestic rate (3%)
//!     0.01,   // foreign rate (1%)
//!     0.15,   // volatility (15%)
//!     1.0,    // expiry (1 year)
//! ).unwrap();
//!
//! let model = GarmanKohlhagen::new(params);
//! let call_price = model.price(true);
//! let put_price = model.price(false);
//!
//! // Put-call parity check
//! let parity_diff = call_price - put_price
//!     - (1.10 * (-0.01_f64).exp() - 1.12 * (-0.03_f64).exp());
//! assert!(parity_diff.abs() < 1e-10);
//! ```

use num_traits::Float;

use super::error::FormulaError;
use crate::math::{distributions::{norm_cdf, norm_pdf}, numeric::from_f64};

/// Parameters for the Garman-Kohlhagen model.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
#[derive(Debug, Clone, Copy)]
pub struct GarmanKohlhagenParams<T: Float> {
    /// Spot exchange rate (domestic per foreign).
    pub spot: T,
    /// Strike price.
    pub strike: T,
    /// Domestic risk-free rate (continuous compounding).
    pub rate_domestic: T,
    /// Foreign risk-free rate (continuous compounding).
    pub rate_foreign: T,
    /// Volatility of the exchange rate.
    pub volatility: T,
    /// Time to expiry in years.
    pub expiry: T,
}

impl<T: Float> GarmanKohlhagenParams<T> {
    /// Creates new Garman-Kohlhagen parameters.
    ///
    /// # Arguments
    ///
    /// * `spot` - Spot exchange rate (must be positive)
    /// * `strike` - Strike price (must be positive)
    /// * `rate_domestic` - Domestic risk-free rate (can be negative)
    /// * `rate_foreign` - Foreign risk-free rate (can be negative)
    /// * `volatility` - Volatility (must be positive)
    /// * `expiry` - Time to expiry in years (must be positive)
    ///
    /// # Errors
    ///
    /// Returns `FormulaError` if any parameter is invalid.
    pub fn new(
        spot: T,
        strike: T,
        rate_domestic: T,
        rate_foreign: T,
        volatility: T,
        expiry: T,
    ) -> Result<Self, FormulaError> {
        if spot <= T::zero() {
            return Err(FormulaError::InvalidSpot {
                spot: spot.to_f64().unwrap_or(0.0),
            });
        }
        if strike <= T::zero() {
            return Err(FormulaError::InvalidSpot {
                spot: strike.to_f64().unwrap_or(0.0),
            });
        }
        if volatility <= T::zero() {
            return Err(FormulaError::InvalidVolatility {
                volatility: volatility.to_f64().unwrap_or(0.0),
            });
        }
        if expiry <= T::zero() {
            return Err(FormulaError::InvalidExpiry {
                expiry: expiry.to_f64().unwrap_or(0.0),
            });
        }

        Ok(Self {
            spot,
            strike,
            rate_domestic,
            rate_foreign,
            volatility,
            expiry,
        })
    }

    /// Returns the forward exchange rate.
    ///
    /// F = S * exp((rd - rf) * T)
    #[inline]
    pub fn forward(&self) -> T {
        let drift = (self.rate_domestic - self.rate_foreign) * self.expiry;
        self.spot * drift.exp()
    }
}

/// Garman-Kohlhagen model for FX option pricing.
///
/// Provides closed-form solutions for European FX options including
/// price and Greeks calculations.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
#[derive(Debug, Clone)]
pub struct GarmanKohlhagen<T: Float> {
    params: GarmanKohlhagenParams<T>,
    /// d1 term from the formula.
    d1: T,
    /// d2 term from the formula.
    d2: T,
    /// √T
    sqrt_t: T,
    /// e^(-rd * T)
    df_domestic: T,
    /// e^(-rf * T)
    df_foreign: T,
}

impl<T: Float> GarmanKohlhagen<T> {
    /// Creates a new Garman-Kohlhagen model instance.
    ///
    /// Pre-computes d1, d2, and discount factors for efficiency.
    ///
    /// # Arguments
    ///
    /// * `params` - Model parameters
    pub fn new(params: GarmanKohlhagenParams<T>) -> Self {
        let sqrt_t = params.expiry.sqrt();
        let vol_sqrt_t = params.volatility * sqrt_t;

        // d1 = [ln(S/K) + (rd - rf + σ²/2) * T] / (σ * √T)
        let log_sk = (params.spot / params.strike).ln();
        let two: T = from_f64(2.0);
        let drift = params.rate_domestic - params.rate_foreign
            + params.volatility * params.volatility / two;
        let d1 = (log_sk + drift * params.expiry) / vol_sqrt_t;

        // d2 = d1 - σ * √T
        let d2 = d1 - vol_sqrt_t;

        // Discount factors
        let df_domestic = (-params.rate_domestic * params.expiry).exp();
        let df_foreign = (-params.rate_foreign * params.expiry).exp();

        Self {
            params,
            d1,
            d2,
            sqrt_t,
            df_domestic,
            df_foreign,
        }
    }

    /// Returns a reference to the parameters.
    #[inline]
    pub fn params(&self) -> &GarmanKohlhagenParams<T> { &self.params }

    /// Returns d1.
    #[inline]
    pub fn d1(&self) -> T { self.d1 }

    /// Returns d2.
    #[inline]
    pub fn d2(&self) -> T { self.d2 }

    /// Computes the option price.
    ///
    /// # Arguments
    ///
    /// * `is_call` - True for call, false for put
    ///
    /// # Returns
    ///
    /// Option price in domestic currency.
    pub fn price(&self, is_call: bool) -> T {
        let nd1 = norm_cdf(self.d1);
        let nd2 = norm_cdf(self.d2);

        if is_call {
            // C = S * e^(-rf*T) * N(d1) - K * e^(-rd*T) * N(d2)
            self.params.spot * self.df_foreign * nd1
                - self.params.strike * self.df_domestic * nd2
        } else {
            // P = K * e^(-rd*T) * N(-d2) - S * e^(-rf*T) * N(-d1)
            let nd1_neg = norm_cdf(-self.d1);
            let nd2_neg = norm_cdf(-self.d2);
            self.params.strike * self.df_domestic * nd2_neg
                - self.params.spot * self.df_foreign * nd1_neg
        }
    }

    /// Computes European call option price.
    #[inline]
    pub fn price_call(&self) -> T {
        self.price(true)
    }

    /// Computes European put option price.
    #[inline]
    pub fn price_put(&self) -> T {
        self.price(false)
    }

    /// Computes Delta.
    ///
    /// Delta measures the sensitivity of the option price to changes in spot.
    ///
    /// # Arguments
    ///
    /// * `is_call` - True for call, false for put
    ///
    /// # Returns
    ///
    /// Delta value.
    pub fn delta(&self, is_call: bool) -> T {
        let nd1 = norm_cdf(self.d1);

        if is_call {
            // Δ_call = e^(-rf*T) * N(d1)
            self.df_foreign * nd1
        } else {
            // Δ_put = -e^(-rf*T) * N(-d1) = e^(-rf*T) * (N(d1) - 1)
            self.df_foreign * (nd1 - T::one())
        }
    }

    /// Computes Gamma.
    ///
    /// Gamma measures the rate of change of Delta with respect to spot.
    /// Same for both call and put options.
    ///
    /// # Returns
    ///
    /// Gamma value.
    pub fn gamma(&self) -> T {
        let pdf_d1 = norm_pdf(self.d1);

        // Γ = e^(-rf*T) * N'(d1) / (S * σ * √T)
        self.df_foreign * pdf_d1 / (self.params.spot * self.params.volatility * self.sqrt_t)
    }

    /// Computes Vega.
    ///
    /// Vega measures the sensitivity to volatility changes.
    /// Same for both call and put options.
    ///
    /// # Returns
    ///
    /// Vega value (per 1% volatility change).
    pub fn vega(&self) -> T {
        let pdf_d1 = norm_pdf(self.d1);

        // ν = S * e^(-rf*T) * N'(d1) * √T
        let hundred: T = from_f64(100.0);
        self.params.spot * self.df_foreign * pdf_d1 * self.sqrt_t / hundred
    }

    /// Computes Theta.
    ///
    /// Theta measures the sensitivity to time decay.
    ///
    /// # Arguments
    ///
    /// * `is_call` - True for call, false for put
    ///
    /// # Returns
    ///
    /// Theta value (per day).
    pub fn theta(&self, is_call: bool) -> T {
        let pdf_d1 = norm_pdf(self.d1);
        let nd1 = norm_cdf(self.d1);
        let nd2 = norm_cdf(self.d2);

        let two: T = from_f64(2.0);
        let days_per_year: T = from_f64(365.0);

        let term1 = -self.params.spot * self.df_foreign * pdf_d1 * self.params.volatility
            / (two * self.sqrt_t);

        if is_call {
            let term2 = self.params.rate_foreign * self.params.spot * self.df_foreign * nd1;
            let term3 = self.params.rate_domestic * self.params.strike * self.df_domestic * nd2;
            (term1 + term2 - term3) / days_per_year
        } else {
            let nd1_neg = T::one() - nd1;
            let nd2_neg = T::one() - nd2;
            let term2 =
                -self.params.rate_foreign * self.params.spot * self.df_foreign * nd1_neg;
            let term3 =
                self.params.rate_domestic * self.params.strike * self.df_domestic * nd2_neg;
            (term1 + term2 + term3) / days_per_year
        }
    }

    /// Computes Rho (domestic).
    ///
    /// Rho measures the sensitivity to domestic interest rate changes.
    ///
    /// # Arguments
    ///
    /// * `is_call` - True for call, false for put
    ///
    /// # Returns
    ///
    /// Rho value (per 1% rate change).
    pub fn rho_domestic(&self, is_call: bool) -> T {
        let nd2 = norm_cdf(self.d2);
        let hundred: T = from_f64(100.0);

        if is_call {
            // ρ_d = K * T * e^(-rd*T) * N(d2)
            self.params.strike * self.params.expiry * self.df_domestic * nd2 / hundred
        } else {
            // ρ_d = -K * T * e^(-rd*T) * N(-d2)
            let nd2_neg = T::one() - nd2;
            -self.params.strike * self.params.expiry * self.df_domestic * nd2_neg / hundred
        }
    }

    /// Computes Rho (foreign).
    ///
    /// Rho measures the sensitivity to foreign interest rate changes.
    ///
    /// # Arguments
    ///
    /// * `is_call` - True for call, false for put
    ///
    /// # Returns
    ///
    /// Rho value (per 1% rate change).
    pub fn rho_foreign(&self, is_call: bool) -> T {
        let nd1 = norm_cdf(self.d1);
        let hundred: T = from_f64(100.0);

        if is_call {
            // ρ_f = -S * T * e^(-rf*T) * N(d1)
            -self.params.spot * self.params.expiry * self.df_foreign * nd1 / hundred
        } else {
            // ρ_f = S * T * e^(-rf*T) * N(-d1)
            let nd1_neg = T::one() - nd1;
            self.params.spot * self.params.expiry * self.df_foreign * nd1_neg / hundred
        }
    }
}

/// Convenience function to price an FX call option.
///
/// # Arguments
///
/// * `spot` - Spot exchange rate
/// * `strike` - Strike price
/// * `rate_domestic` - Domestic risk-free rate
/// * `rate_foreign` - Foreign risk-free rate
/// * `volatility` - Volatility
/// * `expiry` - Time to expiry in years
///
/// # Returns
///
/// Call option price.
pub fn fx_call_price<T: Float>(
    spot: T,
    strike: T,
    rate_domestic: T,
    rate_foreign: T,
    volatility: T,
    expiry: T,
) -> Result<T, FormulaError> {
    let params = GarmanKohlhagenParams::new(
        spot,
        strike,
        rate_domestic,
        rate_foreign,
        volatility,
        expiry,
    )?;
    let model = GarmanKohlhagen::new(params);
    Ok(model.price(true))
}

/// Convenience function to price an FX put option.
///
/// # Arguments
///
/// * `spot` - Spot exchange rate
/// * `strike` - Strike price
/// * `rate_domestic` - Domestic risk-free rate
/// * `rate_foreign` - Foreign risk-free rate
/// * `volatility` - Volatility
/// * `expiry` - Time to expiry in years
///
/// # Returns
///
/// Put option price.
pub fn fx_put_price<T: Float>(
    spot: T,
    strike: T,
    rate_domestic: T,
    rate_foreign: T,
    volatility: T,
    expiry: T,
) -> Result<T, FormulaError> {
    let params = GarmanKohlhagenParams::new(
        spot,
        strike,
        rate_domestic,
        rate_foreign,
        volatility,
        expiry,
    )?;
    let model = GarmanKohlhagen::new(params);
    Ok(model.price(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_params() -> GarmanKohlhagenParams<f64> {
        GarmanKohlhagenParams::new(
            1.10, // spot
            1.12, // strike
            0.03, // domestic rate
            0.01, // foreign rate
            0.15, // volatility
            1.0,  // expiry
        )
        .unwrap()
    }

    #[test]
    fn test_params_new() {
        let params = create_test_params();
        assert!((params.spot - 1.10).abs() < 1e-10);
        assert!((params.strike - 1.12).abs() < 1e-10);
        assert!((params.rate_domestic - 0.03).abs() < 1e-10);
        assert!((params.rate_foreign - 0.01).abs() < 1e-10);
        assert!((params.volatility - 0.15).abs() < 1e-10);
        assert!((params.expiry - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_params_invalid_spot() {
        let result = GarmanKohlhagenParams::new(0.0, 1.12, 0.03, 0.01, 0.15, 1.0);
        assert!(result.is_err());

        let result = GarmanKohlhagenParams::new(-1.0, 1.12, 0.03, 0.01, 0.15, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_params_invalid_volatility() {
        let result = GarmanKohlhagenParams::new(1.10, 1.12, 0.03, 0.01, 0.0, 1.0);
        assert!(result.is_err());

        let result = GarmanKohlhagenParams::new(1.10, 1.12, 0.03, 0.01, -0.15, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_forward_rate() {
        let params = create_test_params();
        let forward = params.forward();
        // F = 1.10 * exp((0.03 - 0.01) * 1.0) = 1.10 * exp(0.02)
        let expected = 1.10 * 0.02_f64.exp();
        assert!((forward - expected).abs() < 1e-10);
    }

    #[test]
    fn test_model_d1_d2() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        // d2 = d1 - σ√T
        let vol_sqrt_t = 0.15 * 1.0_f64.sqrt();
        assert!((model.d1() - model.d2() - vol_sqrt_t).abs() < 1e-10);
    }

    #[test]
    fn test_call_price() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);
        let call = model.price(true);

        // Call price should be positive
        assert!(call > 0.0);
        // Call price should be less than spot
        assert!(call < params.spot);
    }

    #[test]
    fn test_put_price() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);
        let put = model.price(false);

        // Put price should be positive
        assert!(put > 0.0);
        // Put price should be less than strike discounted
        assert!(put < params.strike * (-params.rate_domestic * params.expiry).exp());
    }

    #[test]
    fn test_put_call_parity() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let call = model.price(true);
        let put = model.price(false);

        // Put-call parity: C - P = S * e^(-rf*T) - K * e^(-rd*T)
        let forward_diff = params.spot * (-params.rate_foreign * params.expiry).exp()
            - params.strike * (-params.rate_domestic * params.expiry).exp();

        let parity_error = (call - put - forward_diff).abs();
        assert!(
            parity_error < 1e-10,
            "Put-call parity violated: error = {}",
            parity_error
        );
    }

    #[test]
    fn test_delta() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let call_delta = model.delta(true);
        let put_delta = model.delta(false);

        // Call delta should be between 0 and 1 (adjusted for foreign rate)
        let df_foreign = (-params.rate_foreign * params.expiry).exp();
        assert!(call_delta > 0.0);
        assert!(call_delta < df_foreign);

        // Put delta should be between -1 and 0 (adjusted for foreign rate)
        assert!(put_delta < 0.0);
        assert!(put_delta > -df_foreign);

        // Delta relationship: Δ_put - Δ_call = -e^(-rf*T)
        assert!((put_delta - call_delta + df_foreign).abs() < 1e-10);
    }

    #[test]
    fn test_gamma() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let gamma = model.gamma();

        // Gamma should be positive
        assert!(gamma > 0.0);
    }

    #[test]
    fn test_vega() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let vega = model.vega();

        // Vega should be positive for both calls and puts
        assert!(vega > 0.0);
    }

    #[test]
    fn test_theta() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let call_theta = model.theta(true);
        let put_theta = model.theta(false);

        // Just check they're finite
        assert!(call_theta.is_finite());
        assert!(put_theta.is_finite());
    }

    #[test]
    fn test_rho_domestic() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let call_rho = model.rho_domestic(true);
        let put_rho = model.rho_domestic(false);

        // Call rho_domestic should be positive
        assert!(call_rho > 0.0);
        // Put rho_domestic should be negative
        assert!(put_rho < 0.0);
    }

    #[test]
    fn test_rho_foreign() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let call_rho = model.rho_foreign(true);
        let put_rho = model.rho_foreign(false);

        // Call rho_foreign should be negative
        assert!(call_rho < 0.0);
        // Put rho_foreign should be positive
        assert!(put_rho > 0.0);
    }

    #[test]
    fn test_convenience_functions() {
        let call = fx_call_price(1.10, 1.12, 0.03, 0.01, 0.15, 1.0).unwrap();
        let put = fx_put_price(1.10, 1.12, 0.03, 0.01, 0.15, 1.0).unwrap();

        assert!(call > 0.0);
        assert!(put > 0.0);

        // Verify against model
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);
        assert!((call - model.price(true)).abs() < 1e-10);
        assert!((put - model.price(false)).abs() < 1e-10);
    }

    #[test]
    fn test_clone() {
        let params = create_test_params();
        let model1 = GarmanKohlhagen::new(params);
        let model2 = model1.clone();

        assert!((model1.d1() - model2.d1()).abs() < 1e-10);
    }

    #[test]
    fn test_debug() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let debug_str = format!("{:?}", model);
        assert!(debug_str.contains("GarmanKohlhagen"));
    }
}
