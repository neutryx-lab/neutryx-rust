//! Pricing service - API layer delegating to `pricer_core`
//!
//! This service acts as a thin API layer, delegating all pricing logic
//! to the `pricer_core` crate. It handles request/response transformation
//! and timing measurement only.

use std::time::Instant;

use pricer_core::math::formulas::{
    forward::{Forward, ForwardParams},
    garman_kohlhagen::{GarmanKohlhagen, GarmanKohlhagenParams},
};

use crate::{
    error::ServerError,
    rest::dto::{
        GreeksResponse, InstrumentType, PortfolioInstrumentResult, PortfolioPricingRequest,
        PortfolioPricingResponse, PricingRequest, PricingResponse,
    },
};

/// Service for pricing instruments - delegates to `pricer_core`
pub struct PricingService;

impl PricingService {
    /// Price a single instrument
    pub fn price_instrument(request: &PricingRequest) -> Result<PricingResponse, ServerError> {
        let start = Instant::now();

        let (price, greeks) = match request.instrument_type {
            InstrumentType::VanillaOption | InstrumentType::EuropeanOption => {
                Self::price_vanilla_option(request)?
            }
            InstrumentType::Forward => (Self::price_forward(request)?, None),
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

        let elapsed = start.elapsed();

        Ok(PricingResponse {
            price,
            greeks,
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Price a portfolio of instruments
    #[allow(clippy::unnecessary_wraps)] // Consistent API signature; may return errors in future
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

    /// Price a vanilla European option using `GarmanKohlhagen` (Merton model
    /// with dividend yield)
    ///
    /// Delegates to `pricer_core::math::formulas::garman_kohlhagen`.
    /// `GarmanKohlhagen` with `rate_domestic=r` and `rate_foreign=q` is
    /// mathematically equivalent to the Merton model for equity options
    /// with continuous dividends.
    fn price_vanilla_option(
        request: &PricingRequest,
    ) -> Result<(f64, Option<GreeksResponse>), ServerError> {
        // Build GarmanKohlhagen params: rate_domestic=r, rate_foreign=dividend_yield
        let params = GarmanKohlhagenParams::new(
            request.spot,
            request.strike,
            request.rate,           // domestic rate = risk-free rate
            request.dividend_yield, // foreign rate = dividend yield (Merton equivalence)
            request.volatility,
            request.expiry,
        )
        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;

        let model = GarmanKohlhagen::new(params);
        let price = model.price(request.is_call);

        let greeks = if request.compute_greeks {
            Some(GreeksResponse {
                delta: model.delta(request.is_call),
                gamma: model.gamma(),
                vega: model.vega(),
                theta: model.theta(request.is_call),
                rho: model.rho_domestic(request.is_call),
            })
        } else {
            None
        };

        Ok((price, greeks))
    }

    /// Price a forward contract
    ///
    /// Delegates to `pricer_core::math::formulas::forward`.
    fn price_forward(request: &PricingRequest) -> Result<f64, ServerError> {
        let params = ForwardParams::new(
            request.spot,
            request.strike,
            request.rate,
            request.dividend_yield,
            request.expiry,
        )
        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;

        let model = Forward::new(params);
        Ok(model.present_value())
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
