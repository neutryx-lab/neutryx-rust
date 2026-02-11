//! Garman-Kohlhagen model for FX option pricing.
//!
//! Delegates to [`GeneralisedBSM`] with `b = rd - rf` and `r = rd`.
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

use super::{error::FormulaError, generalised_bsm::GeneralisedBSM};
use crate::math::{normal_dist::norm_cdf, numeric::from_f64};

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
    /// # Errors
    ///
    /// Returns `FormulaError` if spot, strike, volatility, or expiry is
    /// non-positive.
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
/// Delegates to [`GeneralisedBSM`] with `b = rd - rf` and `r = rd`.
/// Provides GK-specific scaling conventions:
/// - Vega: per 1% vol change (raw / 100)
/// - Theta: per day (raw / 365)
/// - Rho: per 1% rate change (raw / 100)
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
#[derive(Debug, Clone)]
pub struct GarmanKohlhagen<T: Float> {
    params: GarmanKohlhagenParams<T>,
    bsm: GeneralisedBSM<T>,
}

impl<T: Float> GarmanKohlhagen<T> {
    /// Creates a new Garman-Kohlhagen model instance.
    ///
    /// Pre-computes d1, d2, and discount factors via [`GeneralisedBSM`].
    pub fn new(params: GarmanKohlhagenParams<T>) -> Self {
        let cost_of_carry = params.rate_domestic - params.rate_foreign;
        let bsm = GeneralisedBSM::new(
            params.spot,
            params.strike,
            params.rate_domestic,
            cost_of_carry,
            params.volatility,
            params.expiry,
        )
        .expect("GarmanKohlhagenParams validates all GeneralisedBSM preconditions");

        Self { params, bsm }
    }

    /// Returns a reference to the parameters.
    #[inline]
    pub fn params(&self) -> &GarmanKohlhagenParams<T> { &self.params }

    /// Returns d1.
    #[inline]
    pub fn d1(&self) -> T { self.bsm.d1() }

    /// Returns d2.
    #[inline]
    pub fn d2(&self) -> T { self.bsm.d2() }

    /// Computes the option price.
    pub fn price(&self, is_call: bool) -> T { self.bsm.price(is_call) }

    /// Computes European call option price.
    #[inline]
    pub fn price_call(&self) -> T { self.bsm.price(true) }

    /// Computes European put option price.
    #[inline]
    pub fn price_put(&self) -> T { self.bsm.price(false) }

    /// Computes Delta.
    pub fn delta(&self, is_call: bool) -> T { self.bsm.delta(is_call) }

    /// Computes Gamma. Same for call and put.
    pub fn gamma(&self) -> T { self.bsm.gamma() }

    /// Computes Vega (per 1% volatility change).
    pub fn vega(&self) -> T {
        let hundred: T = from_f64(100.0);
        self.bsm.vega() / hundred
    }

    /// Computes Theta (per day).
    pub fn theta(&self, is_call: bool) -> T {
        let days_per_year: T = from_f64(365.0);
        self.bsm.theta(is_call) / days_per_year
    }

    /// Computes Rho (domestic) per 1% rate change.
    pub fn rho_domestic(&self, is_call: bool) -> T {
        let hundred: T = from_f64(100.0);
        self.bsm.rho(is_call) / hundred
    }

    /// Computes Rho (foreign) per 1% rate change.
    pub fn rho_foreign(&self, is_call: bool) -> T {
        let hundred: T = from_f64(100.0);
        let nd1 = norm_cdf(self.bsm.d1());
        let carry_df = self.bsm.carry_discount_factor();

        if is_call {
            -self.params.spot * self.params.expiry * carry_df * nd1 / hundred
        } else {
            let nd1_neg = T::one() - nd1;
            self.params.spot * self.params.expiry * carry_df * nd1_neg / hundred
        }
    }
}

/// Convenience function to price an FX call option.
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
        let expected = 1.10 * 0.02_f64.exp();
        assert!((forward - expected).abs() < 1e-10);
    }

    #[test]
    fn test_model_d1_d2() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let vol_sqrt_t = 0.15 * 1.0_f64.sqrt();
        assert!((model.d1() - model.d2() - vol_sqrt_t).abs() < 1e-10);
    }

    #[test]
    fn test_call_price() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);
        let call = model.price(true);

        assert!(call > 0.0);
        assert!(call < params.spot);
    }

    #[test]
    fn test_put_price() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);
        let put = model.price(false);

        assert!(put > 0.0);
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

        let df_foreign = (-params.rate_foreign * params.expiry).exp();
        assert!(call_delta > 0.0);
        assert!(call_delta < df_foreign);

        assert!(put_delta < 0.0);
        assert!(put_delta > -df_foreign);

        // Delta relationship: Δ_put - Δ_call = -e^(-rf*T)
        assert!((put_delta - call_delta + df_foreign).abs() < 1e-10);
    }

    #[test]
    fn test_gamma() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);
        assert!(model.gamma() > 0.0);
    }

    #[test]
    fn test_vega() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);
        assert!(model.vega() > 0.0);
    }

    #[test]
    fn test_theta() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let call_theta = model.theta(true);
        let put_theta = model.theta(false);

        assert!(call_theta.is_finite());
        assert!(put_theta.is_finite());
    }

    #[test]
    fn test_rho_domestic() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let call_rho = model.rho_domestic(true);
        let put_rho = model.rho_domestic(false);

        assert!(call_rho > 0.0);
        assert!(put_rho < 0.0);
    }

    #[test]
    fn test_rho_foreign() {
        let params = create_test_params();
        let model = GarmanKohlhagen::new(params);

        let call_rho = model.rho_foreign(true);
        let put_rho = model.rho_foreign(false);

        assert!(call_rho < 0.0);
        assert!(put_rho > 0.0);
    }

    #[test]
    fn test_convenience_functions() {
        let call = fx_call_price(1.10, 1.12, 0.03, 0.01, 0.15, 1.0).unwrap();
        let put = fx_put_price(1.10, 1.12, 0.03, 0.01, 0.15, 1.0).unwrap();

        assert!(call > 0.0);
        assert!(put > 0.0);

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
