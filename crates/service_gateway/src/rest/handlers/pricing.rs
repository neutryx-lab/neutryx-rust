//! Pricing handlers
//!
//! Thin handlers delegating to `PricingService`.

use axum::Json;

use crate::{
    error::ServerError,
    rest::dto::{
        PortfolioPricingRequest, PortfolioPricingResponse, PricingRequest, PricingResponse,
    },
    services::PricingService,
};

/// Price a single instrument
///
/// POST /api/price
pub async fn price_instrument(
    Json(request): Json<PricingRequest>,
) -> Result<Json<PricingResponse>, ServerError> {
    let response = PricingService::price_instrument(&request)?;
    Ok(Json(response))
}

/// Price a portfolio of instruments
///
/// POST /api/price/batch
pub async fn price_portfolio(
    Json(request): Json<PortfolioPricingRequest>,
) -> Result<Json<PortfolioPricingResponse>, ServerError> {
    let response = PricingService::price_portfolio(&request)?;
    Ok(Json(response))
}
