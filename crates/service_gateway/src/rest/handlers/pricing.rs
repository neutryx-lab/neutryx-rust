//! Pricing handlers
//!
//! Thin handlers delegating to `PricingService`.

use axum::Json;

use crate::{
    error::{AppJson, ServerError},
    rest::dto::{
        PortfolioPricingRequest, PortfolioPricingResponse, PricingRequest, PricingResponse,
    },
    services::PricingService,
};

stateless_json_handler! {
    /// POST /api/price
    pub async fn price_instrument(PricingRequest => PricingResponse) = PricingService::price_instrument;
}

stateless_json_handler! {
    /// POST /api/price/batch
    pub async fn price_portfolio(PortfolioPricingRequest => PortfolioPricingResponse) = PricingService::price_portfolio;
}
