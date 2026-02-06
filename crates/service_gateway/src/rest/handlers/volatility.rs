//! Volatility handlers
//!
//! Thin handlers delegating to `VolatilityService`.

#[cfg(feature = "volatility")]
use std::sync::Arc;

#[cfg(feature = "volatility")]
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

#[cfg(feature = "volatility")]
use crate::{
    error::ServerError,
    rest::dto::{
        BuildFxVolSurfaceRequest, BuildFxVolSurfaceResponse, BuildVolCubeRequest,
        BuildVolCubeResponse, GetImpliedVolRequest, GetImpliedVolResponse,
    },
    services::VolatilityService,
    state::AppState,
};

/// Build an FX volatility surface
///
/// POST /api/v1/volatility/fx-surface
#[cfg(feature = "volatility")]
pub async fn build_fx_vol_surface(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BuildFxVolSurfaceRequest>,
) -> Result<(StatusCode, Json<BuildFxVolSurfaceResponse>), ServerError> {
    let response = VolatilityService::build_fx_vol_surface(&request, &state)?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Build an IR volatility cube
///
/// POST /api/v1/volatility/cube
#[cfg(feature = "volatility")]
pub async fn build_vol_cube(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BuildVolCubeRequest>,
) -> Result<(StatusCode, Json<BuildVolCubeResponse>), ServerError> {
    let response = VolatilityService::build_vol_cube(&request, &state)?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Get implied volatility from a surface
///
/// POST /api/v1/volatility/{id}/implied-vol
#[cfg(feature = "volatility")]
pub async fn get_implied_vol(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<GetImpliedVolRequest>,
) -> Result<Json<GetImpliedVolResponse>, ServerError> {
    let response = VolatilityService::get_implied_vol(&id, &request, &state)?;
    Ok(Json(response))
}
