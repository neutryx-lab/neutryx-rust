//! Curve handlers.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::{
    error::{ServerError, ValidatedJson},
    rest::dto::{
        CurveBuildRequest, CurveBuildResponse, DiscountFactorRequest, DiscountFactorResponse,
        ForwardRateRequest, ForwardRateResponse, ForwardSwapRateRequest, ForwardSwapRateResponse,
    },
    services::CurveService,
    state::AppState,
};

/// Build a yield curve from market instruments.
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/api/curves/build",
    tag = "curves",
    request_body = CurveBuildRequest,
    responses(
        (status = 200, description = "Built curve with pillars and forward rates", body = CurveBuildResponse),
        (status = 400, description = "Invalid request"),
    )
))]
pub async fn build_curve(
    State(state): State<Arc<AppState>>,
    ValidatedJson(request): ValidatedJson<CurveBuildRequest>,
) -> Result<Json<CurveBuildResponse>, ServerError> {
    let response = CurveService::build_curve(&request, &state)?;
    Ok(Json(response))
}

/// Get discount factor from a cached curve.
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/api/curves/discount-factor",
    tag = "curves",
    request_body = DiscountFactorRequest,
    responses(
        (status = 200, description = "Discount factor at given time", body = DiscountFactorResponse),
        (status = 400, description = "Invalid request"),
    )
))]
pub async fn get_discount_factor(
    State(state): State<Arc<AppState>>,
    ValidatedJson(request): ValidatedJson<DiscountFactorRequest>,
) -> Result<Json<DiscountFactorResponse>, ServerError> {
    let response = CurveService::get_discount_factor(&request, &state)?;
    Ok(Json(response))
}

/// Get forward rate from a cached curve.
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/api/curves/forward-rate",
    tag = "curves",
    request_body = ForwardRateRequest,
    responses(
        (status = 200, description = "Forward rate for the period", body = ForwardRateResponse),
        (status = 400, description = "Invalid request"),
    )
))]
pub async fn get_forward_rate(
    State(state): State<Arc<AppState>>,
    ValidatedJson(request): ValidatedJson<ForwardRateRequest>,
) -> Result<Json<ForwardRateResponse>, ServerError> {
    let response = CurveService::get_forward_rate(&request, &state)?;
    Ok(Json(response))
}

/// Compute forward swap rate matrix from a cached curve.
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/api/curves/forward-swap-rates",
    tag = "curves",
    request_body = ForwardSwapRateRequest,
    responses(
        (status = 200, description = "Forward swap rate matrix", body = ForwardSwapRateResponse),
        (status = 400, description = "Invalid request"),
    )
))]
pub async fn get_forward_swap_rates(
    State(state): State<Arc<AppState>>,
    ValidatedJson(request): ValidatedJson<ForwardSwapRateRequest>,
) -> Result<Json<ForwardSwapRateResponse>, ServerError> {
    let response = CurveService::compute_forward_swap_rates(&request, &state)?;
    Ok(Json(response))
}
