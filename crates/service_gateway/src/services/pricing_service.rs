//! Pricing service - API layer delegating to the unified `Pricer`.

use std::time::Instant;

use chrono::Datelike;
use infra_domain::{
    market::{instrument::ExerciseStyle, Currency, CurrencyPair},
    time::Date,
    trade::{Direction, Leg, LegType, OptionType, SettlementType, Trade, TradeType},
};
use pricer_core::math::formulas::forward::{Forward, ForwardParams};
use pricer_models::{market::CurveEnum, vol_surface::VolSurfaceEnum};
use pricer_pricing::{CalcSetting, MarketEnvironmentBuilder, Pricer};

use crate::{
    error::ServerError,
    rest::dto::{
        GreeksResponse, InstrumentType, PortfolioInstrumentResult, PortfolioPricingRequest,
        PortfolioPricingResponse, PricingRequest, PricingResponse,
    },
};

/// Service for pricing instruments - delegates to `pricer_core`.
pub struct PricingService;

impl PricingService {
    /// Price a single instrument.
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

    /// Price a portfolio of instruments.
    #[allow(clippy::unnecessary_wraps)]
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

    /// Price a vanilla European option via the unified [`Pricer`].
    ///
    /// Builds a synthetic FX option trade (EUR/USD convention) and delegates
    /// to `Pricer::price_unified()` which dispatches to Garman-Kohlhagen.
    fn price_vanilla_option(
        request: &PricingRequest,
    ) -> Result<(f64, Option<GreeksResponse>), ServerError> {
        let val_date = Date::from_ymd(2025, 1, 1)
            .map_err(|e| ServerError::Internal(format!("{e}")))?;

        let expiry_days = (request.expiry * 365.0).round() as i64;
        let expiry_inner =
            val_date.into_inner() + chrono::Duration::days(expiry_days);
        let expiry_date = Date::from_ymd(
            expiry_inner.year(),
            expiry_inner.month(),
            expiry_inner.day(),
        )
        .map_err(|e| ServerError::Internal(format!("{e}")))?;

        // Synthetic two-leg trade so Pricer can extract base/quote currencies.
        let leg_base = Leg::new(vec![], Direction::Receiver, LegType::Generic, Currency::EUR);
        let leg_quote = Leg::new(vec![], Direction::Payer, LegType::Generic, Currency::USD);

        let option_type = if request.is_call {
            OptionType::Call
        } else {
            OptionType::Put
        };

        let trade = Trade::new(
            "PRICING-SVC",
            vec![leg_base, leg_quote],
            TradeType::FxOption {
                option_type,
                strike: request.strike,
                exercise_type: ExerciseStyle::European,
                settlement_type: SettlementType::Cash,
                expiry_date,
            },
        );

        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let vol_surface = VolSurfaceEnum::<f64>::flat(request.volatility)
            .map_err(|e| ServerError::Internal(format!("{e}")))?;

        let market = MarketEnvironmentBuilder::new(val_date)
            .with_discount_curve(Currency::USD, CurveEnum::flat(request.rate))
            .with_discount_curve(Currency::EUR, CurveEnum::flat(request.dividend_yield))
            .with_fx_spot(pair, request.spot)
            .with_vol_surface("FX:EUR/USD", vol_surface)
            .build();

        let calc = CalcSetting::builder()
            .compute_greeks(request.compute_greeks)
            .reporting_currency(Currency::USD)
            .build();

        let result = Pricer::price_unified(&trade, &market, &calc)
            .map_err(|e| ServerError::Internal(format!("Pricing failed: {e}")))?;

        let greeks = result.greeks.map(|g| GreeksResponse {
            delta: g.delta.unwrap_or(0.0),
            gamma: g.gamma.unwrap_or(0.0),
            vega: g.vega.unwrap_or(0.0),
            theta: g.theta.unwrap_or(0.0),
            rho: g.rho.unwrap_or(0.0),
        });

        Ok((result.pv, greeks))
    }

    /// Price a forward contract.
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

        assert!(greeks.delta > 0.55 && greeks.delta < 0.65);
        assert!(greeks.gamma > 0.0);
        assert!(greeks.vega > 0.0);
    }
}
