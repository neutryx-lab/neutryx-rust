//! Portfolio handlers
//!
//! Thin handlers delegating to `PortfolioService`.

#[cfg(feature = "risk")]
use std::sync::Arc;

#[cfg(feature = "risk")]
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

#[cfg(feature = "risk")]
use crate::{
    error::{AppJson, ServerError},
    rest::dto::{
        AddTradesRequest, AddTradesResponse, CreatePortfolioRequest, CreatePortfolioResponse,
        GetPortfolioResponse, PortfolioGreeksRequest, PortfolioGreeksResponse,
        PortfolioPriceResponse,
    },
    services::PortfolioService,
    state::AppState,
};

/// POST /api/v1/portfolios
#[cfg(feature = "risk")]
pub async fn create_portfolio(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<CreatePortfolioRequest>,
) -> Result<(StatusCode, Json<CreatePortfolioResponse>), ServerError> {
    let response = PortfolioService::create_portfolio(&request, &state)?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/portfolios/{id}
#[cfg(feature = "risk")]
pub async fn get_portfolio(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GetPortfolioResponse>, ServerError> {
    let response = PortfolioService::get_portfolio(&id, &state)?;
    Ok(Json(response))
}

/// PUT /api/v1/portfolios/{id}/trades
#[cfg(feature = "risk")]
pub async fn add_trades(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AppJson(request): AppJson<AddTradesRequest>,
) -> Result<Json<AddTradesResponse>, ServerError> {
    let response = PortfolioService::add_trades(&id, &request, &state)?;
    Ok(Json(response))
}

/// DELETE /api/v1/portfolios/{id}
#[cfg(feature = "risk")]
pub async fn delete_portfolio(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ServerError> {
    PortfolioService::delete_portfolio(&id, &state)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/portfolios/{id}/price
#[cfg(feature = "risk")]
pub async fn price_portfolio_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PortfolioPriceResponse>, ServerError> {
    let response = PortfolioService::price_portfolio(&id, &state)?;
    Ok(Json(response))
}

/// POST /api/v1/portfolios/{id}/greeks
#[cfg(feature = "risk")]
pub async fn compute_portfolio_greeks(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AppJson(request): AppJson<PortfolioGreeksRequest>,
) -> Result<Json<PortfolioGreeksResponse>, ServerError> {
    let response = PortfolioService::compute_portfolio_greeks(&id, &request, &state)?;
    Ok(Json(response))
}
