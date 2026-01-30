//! Pricing service wrapping GenericPricer facade
//!
//! Provides high-level pricing operations using pricer_pricing.

use std::time::Instant;

use pricer_core::math::distributions::{norm_cdf, norm_pdf};

use crate::error::ServerError;
use crate::rest::dto::{
    GreeksResponse, InstrumentType, PricingRequest, PricingResponse,
    PortfolioInstrumentResult, PortfolioPricingRequest, PortfolioPricingResponse,
};

/// Service for pricing instruments using pricer_pricing facade
pub struct PricingService;

impl PricingService {
    /// Price a single instrument
    pub fn price_instrument(request: &PricingRequest) -> Result<PricingResponse, ServerError> {
        let start = Instant::now();

        let price = match request.instrument_type {
            InstrumentType::VanillaOption | InstrumentType::EuropeanOption => {
                Self::price_vanilla_option(request)?
            }
            InstrumentType::Forward => Self::price_forward(request)?,
            InstrumentType::Swap => {
                return Err(ServerError::InvalidRequest(
                    "Swap pricing requires curve bootstrap - use /api/v1/curves/build first"
                        .to_string(),
                ));
            }
            InstrumentType::Fra => {
                return Err(ServerError::InvalidRequest(
                    "FRA pricing requires curve bootstrap - use /api/v1/curves/build first"
                        .to_string(),
                ));
            }
        };

        let greeks = if request.compute_greeks {
            Some(Self::compute_greeks(request)?)
        } else {
            None
        };

        let elapsed = start.elapsed();

        Ok(PricingResponse {
            price,
            greeks,
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Price a portfolio of instruments
    pub fn price_portfolio(
        request: &PortfolioPricingRequest,
    ) -> Result<PortfolioPricingResponse, ServerError> {
        let start = Instant::now();
        let mut results = Vec::with_capacity(request.instruments.len());
        let mut total_value = 0.0;
        let mut success_count = 0;
        let mut failure_count = 0;

        for instrument in &request.instruments {
            let mut req = instrument.clone();
            req.compute_greeks = request.compute_greeks;

            match Self::price_instrument(&req) {
                Ok(response) => {
                    total_value += response.price;
                    success_count += 1;
                    results.push(PortfolioInstrumentResult {
                        price: response.price,
                        greeks: response.greeks,
                        error: None,
                    });
                }
                Err(e) => {
                    failure_count += 1;
                    results.push(PortfolioInstrumentResult {
                        price: 0.0,
                        greeks: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        let elapsed = start.elapsed();

        Ok(PortfolioPricingResponse {
            results,
            total_value,
            success_count,
            failure_count,
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Price a vanilla European option using Black-Scholes
    fn price_vanilla_option(request: &PricingRequest) -> Result<f64, ServerError> {
        let s = request.spot;
        let k = request.strike;
        let t = request.expiry;
        let r = request.rate;
        let q = request.dividend_yield;
        let sigma = request.volatility;

        if t <= 0.0 {
            return Err(ServerError::InvalidRequest(
                "Expiry must be positive".to_string(),
            ));
        }
        if sigma <= 0.0 {
            return Err(ServerError::InvalidRequest(
                "Volatility must be positive".to_string(),
            ));
        }
        if s <= 0.0 {
            return Err(ServerError::InvalidRequest(
                "Spot must be positive".to_string(),
            ));
        }
        if k <= 0.0 {
            return Err(ServerError::InvalidRequest(
                "Strike must be positive".to_string(),
            ));
        }

        // Using pricer_core's norm_cdf
        let sqrt_t = t.sqrt();
        let d1 = ((s / k).ln() + (r - q + 0.5 * sigma * sigma) * t) / (sigma * sqrt_t);
        let d2 = d1 - sigma * sqrt_t;

        let price = if request.is_call {
            s * (-q * t).exp() * norm_cdf(d1) - k * (-r * t).exp() * norm_cdf(d2)
        } else {
            k * (-r * t).exp() * norm_cdf(-d2) - s * (-q * t).exp() * norm_cdf(-d1)
        };

        Ok(price)
    }

    /// Price a forward contract
    fn price_forward(request: &PricingRequest) -> Result<f64, ServerError> {
        let s = request.spot;
        let k = request.strike;
        let t = request.expiry;
        let r = request.rate;
        let q = request.dividend_yield;

        if t <= 0.0 {
            return Err(ServerError::InvalidRequest(
                "Expiry must be positive".to_string(),
            ));
        }

        // Forward price: (S * e^((r-q)*T) - K) * e^(-r*T)
        let forward_price = s * ((r - q) * t).exp();
        let pv = (forward_price - k) * (-r * t).exp();

        Ok(pv)
    }

    /// Compute Greeks using analytical formulas
    fn compute_greeks(request: &PricingRequest) -> Result<GreeksResponse, ServerError> {
        let s = request.spot;
        let k = request.strike;
        let t = request.expiry;
        let r = request.rate;
        let q = request.dividend_yield;
        let sigma = request.volatility;

        let sqrt_t = t.sqrt();
        let d1 = ((s / k).ln() + (r - q + 0.5 * sigma * sigma) * t) / (sigma * sqrt_t);
        let d2 = d1 - sigma * sqrt_t;

        let e_qt = (-q * t).exp();
        let e_rt = (-r * t).exp();

        // Delta
        let delta = if request.is_call {
            e_qt * norm_cdf(d1)
        } else {
            -e_qt * norm_cdf(-d1)
        };

        // Gamma (same for call and put)
        let gamma = e_qt * norm_pdf(d1) / (s * sigma * sqrt_t);

        // Vega (same for call and put, per 1% move = 0.01)
        let vega = s * e_qt * norm_pdf(d1) * sqrt_t * 0.01;

        // Theta (per day)
        let theta = if request.is_call {
            (-s * e_qt * norm_pdf(d1) * sigma / (2.0 * sqrt_t)
                - r * k * e_rt * norm_cdf(d2)
                + q * s * e_qt * norm_cdf(d1))
                / 365.0
        } else {
            (-s * e_qt * norm_pdf(d1) * sigma / (2.0 * sqrt_t)
                + r * k * e_rt * norm_cdf(-d2)
                - q * s * e_qt * norm_cdf(-d1))
                / 365.0
        };

        // Rho (per 1% move = 0.01)
        let rho = if request.is_call {
            k * t * e_rt * norm_cdf(d2) * 0.01
        } else {
            -k * t * e_rt * norm_cdf(-d2) * 0.01
        };

        Ok(GreeksResponse {
            delta,
            gamma,
            vega,
            theta,
            rho,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_call_option() {
        let request = PricingRequest {
            instrument_type: InstrumentType::VanillaOption,
            strike: 100.0,
            expiry: 1.0,
            is_call: true,
            spot: 100.0,
            volatility: 0.2,
            rate: 0.05,
            dividend_yield: 0.0,
            compute_greeks: false,
        };

        let response = PricingService::price_instrument(&request).unwrap();
        // ATM call with these params should be around 10.45
        assert!(response.price > 10.0 && response.price < 11.0);
    }

    #[test]
    fn test_price_forward() {
        let request = PricingRequest {
            instrument_type: InstrumentType::Forward,
            strike: 100.0,
            expiry: 1.0,
            is_call: true,
            spot: 100.0,
            volatility: 0.2,
            rate: 0.05,
            dividend_yield: 0.0,
            compute_greeks: false,
        };

        let response = PricingService::price_instrument(&request).unwrap();
        // Forward PV: (100 * e^0.05 - 100) * e^-0.05 = 100 - 100*e^-0.05 ≈ 4.88
        assert!(response.price > 4.5 && response.price < 5.5);
    }

    #[test]
    fn test_compute_greeks() {
        let request = PricingRequest {
            instrument_type: InstrumentType::VanillaOption,
            strike: 100.0,
            expiry: 1.0,
            is_call: true,
            spot: 100.0,
            volatility: 0.2,
            rate: 0.05,
            dividend_yield: 0.0,
            compute_greeks: true,
        };

        let response = PricingService::price_instrument(&request).unwrap();
        let greeks = response.greeks.unwrap();

        // ATM call delta should be around 0.6
        assert!(greeks.delta > 0.55 && greeks.delta < 0.65);
        // Gamma should be positive
        assert!(greeks.gamma > 0.0);
        // Vega should be positive
        assert!(greeks.vega > 0.0);
    }
}
