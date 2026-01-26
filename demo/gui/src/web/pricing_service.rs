//! Pricing service adapter module.
//!
//! This module provides a clean interface between HTTP handlers and the
//! underlying pricer_models/pricer_pricing crates. All pricing calculations
//! are delegated to the crates - no calculations are performed in this module.
//!
//! # Design Principles
//!
//! - **No calculations**: All mathematical operations are delegated to crates
//! - **Type conversion**: Converts between HTTP types and crate types
//! - **Error mapping**: Maps crate errors to HTTP-friendly error types
//! - **API stability**: Provides a stable interface for handlers

use pricer_models::{
    analytical::{BlackScholes, GarmanKohlhagen, GarmanKohlhagenParams},
    instruments::FxOptionType,
};

/// Result of Black-Scholes pricing.
#[derive(Debug, Clone)]
pub struct BsPricingResult {
    /// Option price.
    pub price: f64,
    /// Optional Greeks if requested.
    pub greeks: Option<GreeksData>,
}

/// Result of Garman-Kohlhagen (FX option) pricing.
#[derive(Debug, Clone)]
pub struct GkPricingResult {
    /// Option price.
    pub price: f64,
    /// Optional Greeks if requested.
    pub greeks: Option<GkGreeksData>,
}

/// Greeks data structure for equity options.
#[derive(Debug, Clone, Copy)]
pub struct GreeksData {
    /// Delta: ∂V/∂S.
    pub delta: f64,
    /// Gamma: ∂²V/∂S².
    pub gamma: f64,
    /// Vega: ∂V/∂σ (per 1% volatility change).
    pub vega: f64,
    /// Theta: ∂V/∂t (per day).
    pub theta: f64,
    /// Rho: ∂V/∂r (per 1% rate change).
    pub rho: f64,
}

/// Greeks data structure for FX options.
#[derive(Debug, Clone, Copy)]
pub struct GkGreeksData {
    /// Delta: ∂V/∂S (adjusted for foreign rate).
    pub delta: f64,
    /// Gamma: ∂²V/∂S².
    pub gamma: f64,
    /// Vega: ∂V/∂σ (per 1% volatility change).
    pub vega: f64,
    /// Theta: ∂V/∂t (per day).
    pub theta: f64,
    /// Rho domestic: ∂V/∂rd (per 1% rate change).
    pub rho_domestic: f64,
    /// Rho foreign: ∂V/∂rf (per 1% rate change).
    pub rho_foreign: f64,
}

/// Pricing service error type.
#[derive(Debug)]
pub enum PricingServiceError {
    /// Invalid input parameter.
    InvalidInput {
        /// Field name.
        field: String,
        /// Error message.
        message: String,
    },
    /// Error from underlying crate.
    CrateError {
        /// Source crate.
        source: String,
        /// Error message.
        message: String,
    },
}

impl std::fmt::Display for PricingServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PricingServiceError::InvalidInput { field, message } => {
                write!(f, "Invalid input for '{}': {}", field, message)
            }
            PricingServiceError::CrateError { source, message } => {
                write!(f, "Error from {}: {}", source, message)
            }
        }
    }
}

impl std::error::Error for PricingServiceError {}

/// Price an equity option using Black-Scholes from pricer_models.
///
/// # Arguments
///
/// * `spot` - Current spot price
/// * `strike` - Strike price
/// * `expiry` - Time to expiration in years
/// * `rate` - Risk-free interest rate (annualised)
/// * `volatility` - Volatility (annualised)
/// * `is_call` - True for call option, false for put option
/// * `compute_greeks` - Whether to compute Greeks
///
/// # Returns
///
/// Pricing result with price and optional Greeks.
pub fn price_equity_option(
    spot: f64,
    strike: f64,
    expiry: f64,
    rate: f64,
    volatility: f64,
    is_call: bool,
    compute_greeks: bool,
) -> Result<BsPricingResult, PricingServiceError> {
    // Handle expiry <= 0 case: return intrinsic value
    if expiry <= 0.0 {
        let intrinsic = if is_call {
            (spot - strike).max(0.0)
        } else {
            (strike - spot).max(0.0)
        };

        let greeks = if compute_greeks {
            Some(GreeksData {
                delta: if is_call {
                    if spot > strike {
                        1.0
                    } else {
                        0.0
                    }
                } else if spot < strike {
                    -1.0
                } else {
                    0.0
                },
                gamma: 0.0,
                vega: 0.0,
                theta: 0.0,
                rho: 0.0,
            })
        } else {
            None
        };

        return Ok(BsPricingResult {
            price: intrinsic,
            greeks,
        });
    }

    // Create BlackScholes model from crate
    let bs =
        BlackScholes::new(spot, rate, volatility).map_err(|e| PricingServiceError::CrateError {
            source: "pricer_models::analytical::BlackScholes".to_string(),
            message: format!("{:?}", e),
        })?;

    // Calculate price using crate method
    let price = if is_call {
        bs.price_call(strike, expiry)
    } else {
        bs.price_put(strike, expiry)
    };

    // Calculate Greeks if requested using crate methods
    let greeks = if compute_greeks {
        Some(GreeksData {
            delta: bs.delta(strike, expiry, is_call),
            gamma: bs.gamma(strike, expiry),
            // Crate returns raw vega; convert to per-1% format
            vega: bs.vega(strike, expiry) / 100.0,
            // Crate returns annual theta; convert to per-day format
            theta: bs.theta(strike, expiry, is_call) / 365.0,
            // Crate returns raw rho; convert to per-1% format
            rho: bs.rho(strike, expiry, is_call) / 100.0,
        })
    } else {
        None
    };

    Ok(BsPricingResult { price, greeks })
}

/// Price an FX option using Garman-Kohlhagen from pricer_models.
///
/// # Arguments
///
/// * `spot` - Spot exchange rate (domestic per foreign)
/// * `strike` - Strike price
/// * `expiry` - Time to expiration in years
/// * `domestic_rate` - Domestic risk-free rate (continuous compounding)
/// * `foreign_rate` - Foreign risk-free rate (continuous compounding)
/// * `volatility` - Volatility of the exchange rate
/// * `is_call` - True for call option, false for put option
/// * `compute_greeks` - Whether to compute Greeks
///
/// # Returns
///
/// Pricing result with price and optional Greeks.
pub fn price_fx_option(
    spot: f64,
    strike: f64,
    expiry: f64,
    domestic_rate: f64,
    foreign_rate: f64,
    volatility: f64,
    is_call: bool,
    compute_greeks: bool,
) -> Result<GkPricingResult, PricingServiceError> {
    // Create Garman-Kohlhagen parameters
    let params = GarmanKohlhagenParams::new(
        spot,
        strike,
        domestic_rate,
        foreign_rate,
        volatility,
        expiry,
    )
    .map_err(|e| PricingServiceError::CrateError {
        source: "pricer_models::analytical::GarmanKohlhagenParams".to_string(),
        message: format!("{:?}", e),
    })?;

    // Create Garman-Kohlhagen model from crate
    let model = GarmanKohlhagen::new(params);

    // Determine option type
    let option_type = if is_call {
        FxOptionType::Call
    } else {
        FxOptionType::Put
    };

    // Calculate price using crate method
    let price = model.price(option_type);

    // Calculate Greeks if requested using crate methods
    let greeks = if compute_greeks {
        Some(GkGreeksData {
            delta: model.delta(option_type),
            gamma: model.gamma(),
            // Crate already returns vega in per-1% format
            vega: model.vega(),
            // Crate already returns theta in per-day format
            theta: model.theta(option_type),
            // Crate already returns rho in per-1% format
            rho_domestic: model.rho_domestic(option_type),
            rho_foreign: model.rho_foreign(option_type),
        })
    } else {
        None
    };

    Ok(GkPricingResult { price, greeks })
}

/// Calculate a specific Greek for a given set of parameters.
///
/// This is used for heatmap and timeseries visualisations.
///
/// # Arguments
///
/// * `greek_type` - The Greek to calculate ("delta", "gamma", "vega", "theta",
///   "rho")
/// * `spot` - Current spot price
/// * `strike` - Strike price
/// * `expiry` - Time to expiration in years
/// * `rate` - Risk-free interest rate
/// * `volatility` - Volatility
/// * `is_call` - True for call option
///
/// # Returns
///
/// The calculated Greek value.
pub fn calculate_greek(
    greek_type: &str,
    spot: f64,
    strike: f64,
    expiry: f64,
    rate: f64,
    volatility: f64,
    is_call: bool,
) -> Result<f64, PricingServiceError> {
    // Handle expiry <= 0 case
    if expiry <= 0.0 {
        return Ok(match greek_type.to_lowercase().as_str() {
            "delta" => {
                if is_call {
                    if spot > strike {
                        1.0
                    } else {
                        0.0
                    }
                } else if spot < strike {
                    -1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        });
    }

    // Create BlackScholes model
    let bs =
        BlackScholes::new(spot, rate, volatility).map_err(|e| PricingServiceError::CrateError {
            source: "pricer_models::analytical::BlackScholes".to_string(),
            message: format!("{:?}", e),
        })?;

    // Calculate the requested Greek
    let greek = match greek_type.to_lowercase().as_str() {
        "delta" => bs.delta(strike, expiry, is_call),
        "gamma" => bs.gamma(strike, expiry),
        "vega" => bs.vega(strike, expiry) / 100.0,
        "theta" => bs.theta(strike, expiry, is_call) / 365.0,
        "rho" => bs.rho(strike, expiry, is_call) / 100.0,
        other => {
            return Err(PricingServiceError::InvalidInput {
                field: "greek_type".to_string(),
                message: format!("Unknown Greek type: {}", other),
            });
        }
    };

    Ok(greek)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_price_equity_option_call() {
        let result = price_equity_option(100.0, 100.0, 1.0, 0.05, 0.2, true, true).unwrap();

        // Reference value from Black-Scholes: ~10.4506
        assert_relative_eq!(result.price, 10.4506, epsilon = 0.01);

        let greeks = result.greeks.unwrap();
        // Call delta should be between 0 and 1
        assert!(greeks.delta > 0.0 && greeks.delta < 1.0);
        // Gamma should be positive
        assert!(greeks.gamma > 0.0);
        // Vega should be positive
        assert!(greeks.vega > 0.0);
        // Theta should typically be negative
        assert!(greeks.theta < 0.0);
        // Call rho should be positive
        assert!(greeks.rho > 0.0);
    }

    #[test]
    fn test_price_equity_option_put() {
        let result = price_equity_option(100.0, 100.0, 1.0, 0.05, 0.2, false, true).unwrap();

        // Reference value from Black-Scholes: ~5.5735
        assert_relative_eq!(result.price, 5.5735, epsilon = 0.01);

        let greeks = result.greeks.unwrap();
        // Put delta should be between -1 and 0
        assert!(greeks.delta < 0.0 && greeks.delta > -1.0);
        // Put rho should be negative
        assert!(greeks.rho < 0.0);
    }

    #[test]
    fn test_price_equity_option_expired() {
        // ITM call at expiry
        let result = price_equity_option(110.0, 100.0, 0.0, 0.05, 0.2, true, true).unwrap();
        assert_relative_eq!(result.price, 10.0, epsilon = 1e-10);
        assert_eq!(result.greeks.unwrap().delta, 1.0);

        // OTM call at expiry
        let result = price_equity_option(90.0, 100.0, 0.0, 0.05, 0.2, true, true).unwrap();
        assert_relative_eq!(result.price, 0.0, epsilon = 1e-10);
        assert_eq!(result.greeks.unwrap().delta, 0.0);
    }

    #[test]
    fn test_price_fx_option_call() {
        let result = price_fx_option(1.10, 1.12, 1.0, 0.03, 0.01, 0.15, true, true).unwrap();

        // Price should be positive
        assert!(result.price > 0.0);

        let greeks = result.greeks.unwrap();
        // Call delta should be positive
        assert!(greeks.delta > 0.0);
        // Gamma should be positive
        assert!(greeks.gamma > 0.0);
        // Vega should be positive
        assert!(greeks.vega > 0.0);
    }

    #[test]
    fn test_price_fx_option_put() {
        let result = price_fx_option(1.10, 1.12, 1.0, 0.03, 0.01, 0.15, false, true).unwrap();

        // Price should be positive
        assert!(result.price > 0.0);

        let greeks = result.greeks.unwrap();
        // Put delta should be negative
        assert!(greeks.delta < 0.0);
    }

    #[test]
    fn test_fx_option_put_call_parity() {
        let call = price_fx_option(1.10, 1.12, 1.0, 0.03, 0.01, 0.15, true, false).unwrap();
        let put = price_fx_option(1.10, 1.12, 1.0, 0.03, 0.01, 0.15, false, false).unwrap();

        // Put-call parity: C - P = S * e^(-rf*T) - K * e^(-rd*T)
        let forward_diff = 1.10 * (-0.01_f64).exp() - 1.12 * (-0.03_f64).exp();
        assert_relative_eq!(call.price - put.price, forward_diff, epsilon = 1e-10);
    }

    #[test]
    fn test_calculate_greek_delta() {
        let delta = calculate_greek("delta", 100.0, 100.0, 1.0, 0.05, 0.2, true).unwrap();
        // ATM call delta should be around 0.5-0.7
        assert!(delta > 0.5 && delta < 0.7);
    }

    #[test]
    fn test_calculate_greek_invalid_type() {
        let result = calculate_greek("invalid", 100.0, 100.0, 1.0, 0.05, 0.2, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_spot() {
        let result = price_equity_option(-100.0, 100.0, 1.0, 0.05, 0.2, true, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_volatility() {
        let result = price_equity_option(100.0, 100.0, 1.0, 0.05, 0.0, true, false);
        assert!(result.is_err());
    }
}
