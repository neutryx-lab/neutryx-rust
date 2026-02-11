//! Pricing handlers.

use axum::Json;

use crate::{
    error::{AppJson, ServerError},
    rest::dto::{
        PortfolioPricingRequest, PortfolioPricingResponse, PricingRequest, PricingResponse,
    },
    services::PricingService,
};

/// POST /api/price.
pub async fn price_instrument(
    AppJson(request): AppJson<PricingRequest>,
) -> Result<Json<PricingResponse>, ServerError> {
    let response = PricingService::price_instrument(&request)?;
    Ok(Json(response))
}

/// POST /api/price/batch.
pub async fn price_portfolio(
    AppJson(request): AppJson<PortfolioPricingRequest>,
) -> Result<Json<PortfolioPricingResponse>, ServerError> {
    let response = PricingService::price_portfolio(&request)?;
    Ok(Json(response))
}
