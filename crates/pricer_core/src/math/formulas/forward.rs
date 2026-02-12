//! Forward contract pricing.
//!
//! This module provides closed-form pricing for forward contracts
//! with continuous dividend yield.
//!
//! ## Mathematical Formula
//!
//! **Forward Price**: F = S · e^((r-q)·T)
//! **Present Value**: PV = (F - K) · e^(-r·T) = S · e^(-q·T) - K · e^(-r·T)
//!
//! Where:
//! - S: spot price
//! - K: strike (delivery) price
//! - r: risk-free rate (continuous compounding)
//! - q: dividend yield (continuous)
//! - T: time to maturity in years

use num_traits::Float;

use super::error::{
    require_positive_expiry, require_positive_spot, require_positive_strike, FormulaError,
};

/// Parameters for forward contract pricing.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
#[derive(Debug, Clone, Copy)]
pub struct ForwardParams<T: Float> {
    /// Spot price (S)
    pub spot: T,
    /// Strike/delivery price (K)
    pub strike: T,
    /// Risk-free rate (r), continuous compounding
    pub rate: T,
    /// Dividend yield (q), continuous
    pub dividend_yield: T,
    /// Time to expiry in years (T)
    pub expiry: T,
}

impl<T: Float> ForwardParams<T> {
    /// Creates new forward contract parameters.
    ///
    /// # Arguments
    ///
    /// * `spot` - Spot price (must be positive)
    /// * `strike` - Strike/delivery price (must be positive)
    /// * `rate` - Risk-free rate (can be negative)
    /// * `dividend_yield` - Dividend yield (can be negative)
    /// * `expiry` - Time to expiry in years (must be positive)
    ///
    /// # Errors
    ///
    /// Returns `FormulaError` if spot, strike, or expiry is non-positive.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_core::math::formulas::forward::ForwardParams;
    ///
    /// let params = ForwardParams::new(100.0_f64, 102.0, 0.05, 0.02, 1.0).unwrap();
    /// assert!((params.spot - 100.0).abs() < 1e-10);
    /// ```
    pub fn new(
        spot: T,
        strike: T,
        rate: T,
        dividend_yield: T,
        expiry: T,
    ) -> Result<Self, FormulaError> {
        require_positive_spot(spot)?;
        require_positive_strike(strike)?;
        require_positive_expiry(expiry)?;

        Ok(Self {
            spot,
            strike,
            rate,
            dividend_yield,
            expiry,
        })
    }
}

/// Forward contract pricing model.
///
/// Provides closed-form pricing for forward contracts with continuous
/// dividend yield.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Examples
///
/// ```
/// use pricer_core::math::formulas::forward::{Forward, ForwardParams};
///
/// let params = ForwardParams::new(100.0_f64, 100.0, 0.05, 0.0, 1.0).unwrap();
/// let model = Forward::new(params);
///
/// // Forward price = S * e^(r*T) = 100 * e^0.05 ≈ 105.13
/// let forward_price = model.forward_price();
/// assert!((forward_price - 105.127).abs() < 0.01);
///
/// // PV when strike = 100: (F - K) * e^(-r*T) ≈ 4.88
/// let pv = model.present_value();
/// assert!((pv - 4.88).abs() < 0.1);
/// ```
#[derive(Debug, Clone)]
pub struct Forward<T: Float> {
    params: ForwardParams<T>,
    /// e^(-r·T)
    df_rate: T,
    /// e^(-q·T)
    df_div: T,
}

impl<T: Float> Forward<T> {
    /// Creates a new Forward pricing model.
    ///
    /// Pre-computes discount factors for efficiency.
    ///
    /// # Arguments
    ///
    /// * `params` - Forward contract parameters
    pub fn new(params: ForwardParams<T>) -> Self {
        let df_rate = (-params.rate * params.expiry).exp();
        let df_div = (-params.dividend_yield * params.expiry).exp();

        Self {
            params,
            df_rate,
            df_div,
        }
    }

    /// Returns a reference to the parameters.
    #[inline]
    pub fn params(&self) -> &ForwardParams<T> { &self.params }

    /// Computes the forward price.
    ///
    /// F = S · e^((r-q)·T)
    ///
    /// # Returns
    ///
    /// The forward price at maturity.
    #[inline]
    pub fn forward_price(&self) -> T {
        // F = S * e^((r-q)*T) = S * e^(-q*T) / e^(-r*T)
        self.params.spot * self.df_div / self.df_rate
    }

    /// Computes the present value of the forward contract.
    ///
    /// PV = (F - K) · e^(-r·T) = S · e^(-q·T) - K · e^(-r·T)
    ///
    /// # Returns
    ///
    /// The present value of the forward contract (can be negative).
    #[inline]
    pub fn present_value(&self) -> T {
        // PV = S * e^(-q*T) - K * e^(-r*T)
        self.params.spot * self.df_div - self.params.strike * self.df_rate
    }

    /// Computes Delta (∂PV/∂S).
    ///
    /// Delta = e^(-q·T)
    ///
    /// # Returns
    ///
    /// The delta of the forward contract.
    #[inline]
    pub fn delta(&self) -> T { self.df_div }

    /// Computes Gamma (∂²PV/∂S²).
    ///
    /// Gamma = 0 for linear instruments.
    ///
    /// # Returns
    ///
    /// Zero (forwards have no gamma).
    #[inline]
    pub fn gamma(&self) -> T { T::zero() }

    /// Computes Theta (∂PV/∂t).
    ///
    /// Theta = q·S·e^(-q·T) - r·K·e^(-r·T)
    ///
    /// # Returns
    ///
    /// The theta per year (multiply by -1/365 for daily theta).
    #[inline]
    pub fn theta(&self) -> T {
        self.params.dividend_yield * self.params.spot * self.df_div
            - self.params.rate * self.params.strike * self.df_rate
    }

    /// Computes Rho (∂PV/∂r).
    ///
    /// Rho = K·T·e^(-r·T)
    ///
    /// # Returns
    ///
    /// The rho (sensitivity to rate changes).
    #[inline]
    pub fn rho(&self) -> T { self.params.strike * self.params.expiry * self.df_rate }
}

/// Convenience function to compute forward price.
///
/// # Arguments
///
/// * `spot` - Spot price
/// * `rate` - Risk-free rate
/// * `dividend_yield` - Dividend yield
/// * `expiry` - Time to expiry
///
/// # Returns
///
/// Forward price F = S · e^((r-q)·T)
#[inline]
pub fn forward_price<T: Float>(spot: T, rate: T, dividend_yield: T, expiry: T) -> T {
    spot * ((rate - dividend_yield) * expiry).exp()
}

/// Convenience function to compute forward contract PV.
///
/// # Arguments
///
/// * `spot` - Spot price
/// * `strike` - Strike/delivery price
/// * `rate` - Risk-free rate
/// * `dividend_yield` - Dividend yield
/// * `expiry` - Time to expiry
///
/// # Returns
///
/// Present value of forward contract.
///
/// # Errors
///
/// Returns `FormulaError` if parameters are invalid.
pub fn forward_pv<T: Float>(
    spot: T,
    strike: T,
    rate: T,
    dividend_yield: T,
    expiry: T,
) -> Result<T, FormulaError> {
    let params = ForwardParams::new(spot, strike, rate, dividend_yield, expiry)?;
    let model = Forward::new(params);
    Ok(model.present_value())
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn create_test_params() -> ForwardParams<f64> {
        ForwardParams::new(
            100.0, // spot
            100.0, // strike
            0.05,  // rate
            0.0,   // dividend yield
            1.0,   // expiry
        )
        .unwrap()
    }

    #[test]
    fn test_params_new() {
        let params = create_test_params();
        assert!((params.spot - 100.0).abs() < 1e-10);
        assert!((params.strike - 100.0).abs() < 1e-10);
        assert!((params.rate - 0.05).abs() < 1e-10);
        assert!((params.dividend_yield - 0.0).abs() < 1e-10);
        assert!((params.expiry - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_params_invalid_spot() {
        let result = ForwardParams::new(0.0, 100.0, 0.05, 0.0, 1.0);
        assert!(result.is_err());

        let result = ForwardParams::new(-100.0, 100.0, 0.05, 0.0, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_params_invalid_expiry() {
        let result = ForwardParams::new(100.0, 100.0, 0.05, 0.0, 0.0);
        assert!(result.is_err());

        let result = ForwardParams::new(100.0, 100.0, 0.05, 0.0, -1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_forward_price_no_dividend() {
        let params = create_test_params();
        let model = Forward::new(params);

        // F = S * e^(r*T) = 100 * e^0.05 ≈ 105.127
        let forward = model.forward_price();
        let expected = 100.0 * 0.05_f64.exp();
        assert_relative_eq!(forward, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_forward_price_with_dividend() {
        let params = ForwardParams::new(100.0, 100.0, 0.05, 0.02, 1.0).unwrap();
        let model = Forward::new(params);

        // F = S * e^((r-q)*T) = 100 * e^0.03 ≈ 103.045
        let forward = model.forward_price();
        let expected = 100.0 * 0.03_f64.exp();
        assert_relative_eq!(forward, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_present_value_atm() {
        let params = create_test_params();
        let model = Forward::new(params);

        // PV = S - K * e^(-r*T) = 100 - 100 * e^(-0.05) ≈ 4.877
        let pv = model.present_value();
        let expected = 100.0 - 100.0 * (-0.05_f64).exp();
        assert_relative_eq!(pv, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_present_value_itm() {
        // Strike below spot
        let params = ForwardParams::new(100.0, 90.0, 0.05, 0.0, 1.0).unwrap();
        let model = Forward::new(params);

        let pv = model.present_value();
        // PV should be positive (we're long a favourable forward)
        assert!(pv > 0.0);
    }

    #[test]
    fn test_present_value_otm() {
        // Strike above forward price
        let params = ForwardParams::new(100.0, 110.0, 0.05, 0.0, 1.0).unwrap();
        let model = Forward::new(params);

        let pv = model.present_value();
        // Forward ≈ 105.13, strike = 110, so PV should be negative
        assert!(pv < 0.0);
    }

    #[test]
    fn test_delta() {
        let params = create_test_params();
        let model = Forward::new(params);

        // Delta = e^(-q*T) = e^0 = 1.0 (no dividend)
        let delta = model.delta();
        assert_relative_eq!(delta, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_delta_with_dividend() {
        let params = ForwardParams::new(100.0, 100.0, 0.05, 0.02, 1.0).unwrap();
        let model = Forward::new(params);

        // Delta = e^(-q*T) = e^(-0.02) ≈ 0.9802
        let delta = model.delta();
        let expected = (-0.02_f64).exp();
        assert_relative_eq!(delta, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_gamma_is_zero() {
        let params = create_test_params();
        let model = Forward::new(params);

        // Gamma = 0 for forwards
        let gamma = model.gamma();
        assert_eq!(gamma, 0.0);
    }

    #[test]
    fn test_rho() {
        let params = create_test_params();
        let model = Forward::new(params);

        // Rho = K * T * e^(-r*T) = 100 * 1 * e^(-0.05) ≈ 95.12
        let rho = model.rho();
        let expected = 100.0 * 1.0 * (-0.05_f64).exp();
        assert_relative_eq!(rho, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_convenience_forward_price() {
        let price = forward_price(100.0_f64, 0.05, 0.02, 1.0);
        let expected = 100.0 * 0.03_f64.exp();
        assert_relative_eq!(price, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_convenience_forward_pv() {
        let pv = forward_pv(100.0_f64, 100.0, 0.05, 0.0, 1.0).unwrap();
        let expected = 100.0 - 100.0 * (-0.05_f64).exp();
        assert_relative_eq!(pv, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_clone_and_debug() {
        let params = create_test_params();
        let model1 = Forward::new(params);
        let model2 = model1.clone();

        assert_relative_eq!(
            model1.forward_price(),
            model2.forward_price(),
            epsilon = 1e-10
        );

        let debug_str = format!("{:?}", model1);
        assert!(debug_str.contains("Forward"));
    }
}
