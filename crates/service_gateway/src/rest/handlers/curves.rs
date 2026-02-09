//! Curve handlers
//!
//! Thin handlers delegating to `CurveService`.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::{
    error::ServerError,
    rest::dto::{
        CurveBuildRequest, CurveBuildResponse, DiscountFactorRequest, DiscountFactorResponse,
        ForwardRateRequest, ForwardRateResponse, ForwardSwapRateRequest, ForwardSwapRateResponse,
    },
    services::CurveService,
    state::AppState,
};

/// Build a yield curve from market instruments
///
/// POST /api/curves/build
pub async fn build_curve(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CurveBuildRequest>,
) -> Result<Json<CurveBuildResponse>, ServerError> {
    let response = CurveService::build_curve(&request, &state)?;
    Ok(Json(response))
}

/// Get discount factor from a cached curve
///
/// POST /api/curves/discount-factor
pub async fn get_discount_factor(
    State(state): State<Arc<AppState>>,
    Json(request): Json<DiscountFactorRequest>,
) -> Result<Json<DiscountFactorResponse>, ServerError> {
    let response = CurveService::get_discount_factor(&request, &state)?;
    Ok(Json(response))
}

/// Get forward rate from a cached curve
///
/// POST /api/curves/forward-rate
pub async fn get_forward_rate(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ForwardRateRequest>,
) -> Result<Json<ForwardRateResponse>, ServerError> {
    let response = CurveService::get_forward_rate(&request, &state)?;
    Ok(Json(response))
}

/// Compute forward swap rate matrix from a cached curve
///
/// POST /api/curves/forward-swap-rates
pub async fn get_forward_swap_rates(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ForwardSwapRateRequest>,
) -> Result<Json<ForwardSwapRateResponse>, ServerError> {
    let response = CurveService::compute_forward_swap_rates(&request, &state)?;
    Ok(Json(response))
}
