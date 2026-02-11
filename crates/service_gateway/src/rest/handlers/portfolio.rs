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

#[cfg(feature = "risk")]
json_created_handler! {
    /// POST /api/v1/portfolios
    pub async fn create_portfolio(CreatePortfolioRequest => CreatePortfolioResponse) = PortfolioService::create_portfolio;
}

#[cfg(feature = "risk")]
path_handler! {
    /// GET /api/v1/portfolios/{id}
    pub async fn get_portfolio(=> GetPortfolioResponse) = PortfolioService::get_portfolio;
}

#[cfg(feature = "risk")]
path_json_handler! {
    /// PUT /api/v1/portfolios/{id}/trades
    pub async fn add_trades(AddTradesRequest => AddTradesResponse) = PortfolioService::add_trades;
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

#[cfg(feature = "risk")]
path_handler! {
    /// POST /api/v1/portfolios/{id}/price
    pub async fn price_portfolio_id(=> PortfolioPriceResponse) = PortfolioService::price_portfolio;
}

#[cfg(feature = "risk")]
path_json_handler! {
    /// POST /api/v1/portfolios/{id}/greeks
    pub async fn compute_portfolio_greeks(PortfolioGreeksRequest => PortfolioGreeksResponse) = PortfolioService::compute_portfolio_greeks;
}
