//! Curve handlers.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::{
    error::{AppJson, ServerError},
    rest::dto::{
        CurveBuildRequest, CurveBuildResponse, DiscountFactorRequest, DiscountFactorResponse,
        ForwardRateRequest, ForwardRateResponse, ForwardSwapRateRequest, ForwardSwapRateResponse,
    },
    services::CurveService,
    state::AppState,
};

/// POST /api/curves/build.
pub async fn build_curve(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<CurveBuildRequest>,
) -> Result<Json<CurveBuildResponse>, ServerError> {
    let response = CurveService::build_curve(&request, &state)?;
    Ok(Json(response))
}

/// POST /api/curves/discount-factor.
pub async fn get_discount_factor(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<DiscountFactorRequest>,
) -> Result<Json<DiscountFactorResponse>, ServerError> {
    let response = CurveService::get_discount_factor(&request, &state)?;
    Ok(Json(response))
}

/// POST /api/curves/forward-rate.
pub async fn get_forward_rate(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<ForwardRateRequest>,
) -> Result<Json<ForwardRateResponse>, ServerError> {
    let response = CurveService::get_forward_rate(&request, &state)?;
    Ok(Json(response))
}

/// POST /api/curves/forward-swap-rates.
pub async fn get_forward_swap_rates(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<ForwardSwapRateRequest>,
) -> Result<Json<ForwardSwapRateResponse>, ServerError> {
    let response = CurveService::compute_forward_swap_rates(&request, &state)?;
    Ok(Json(response))
}
