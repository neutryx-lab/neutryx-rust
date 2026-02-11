//! Pricing handlers.

use axum::Json;

use crate::{
    error::{ServerError, ValidatedJson},
    rest::dto::{
        PortfolioPricingRequest, PortfolioPricingResponse, PricingRequest, PricingResponse,
    },
    services::PricingService,
};

/// Price a single instrument.
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/api/price",
    tag = "pricing",
    request_body = PricingRequest,
    responses(
        (status = 200, description = "Pricing result", body = PricingResponse),
        (status = 400, description = "Invalid request"),
    )
))]
pub async fn price_instrument(
    ValidatedJson(request): ValidatedJson<PricingRequest>,
) -> Result<Json<PricingResponse>, ServerError> {
    let response = PricingService::price_instrument(&request)?;
    Ok(Json(response))
}

/// Price a batch of instruments.
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/api/price/batch",
    tag = "pricing",
    request_body = PortfolioPricingRequest,
    responses(
        (status = 200, description = "Portfolio pricing result", body = PortfolioPricingResponse),
        (status = 400, description = "Invalid request"),
    )
))]
pub async fn price_portfolio(
    ValidatedJson(request): ValidatedJson<PortfolioPricingRequest>,
) -> Result<Json<PortfolioPricingResponse>, ServerError> {
    let response = PricingService::price_portfolio(&request)?;
    Ok(Json(response))
}
