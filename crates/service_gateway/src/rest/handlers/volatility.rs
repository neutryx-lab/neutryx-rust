//! Volatility handlers.

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
    error::{ServerError, ValidatedJson},
    rest::dto::{
        BuildFxVolSurfaceRequest, BuildFxVolSurfaceResponse, BuildVolCubeRequest,
        BuildVolCubeResponse, GetImpliedVolRequest, GetImpliedVolResponse,
    },
    services::VolatilityService,
    state::AppState,
};

/// POST /api/v1/volatility/fx-surface.
#[cfg(feature = "volatility")]
pub async fn build_fx_vol_surface(
    State(state): State<Arc<AppState>>,
    ValidatedJson(request): ValidatedJson<BuildFxVolSurfaceRequest>,
) -> Result<(StatusCode, Json<BuildFxVolSurfaceResponse>), ServerError> {
    let response = VolatilityService::build_fx_vol_surface(&request, &state)?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// POST /api/v1/volatility/cube.
#[cfg(feature = "volatility")]
pub async fn build_vol_cube(
    State(state): State<Arc<AppState>>,
    ValidatedJson(request): ValidatedJson<BuildVolCubeRequest>,
) -> Result<(StatusCode, Json<BuildVolCubeResponse>), ServerError> {
    let response = VolatilityService::build_vol_cube(&request, &state)?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// POST /api/v1/volatility/{id}/implied-vol.
#[cfg(feature = "volatility")]
pub async fn get_implied_vol(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<GetImpliedVolRequest>,
) -> Result<Json<GetImpliedVolResponse>, ServerError> {
    let response = VolatilityService::get_implied_vol(&id, &request, &state)?;
    Ok(Json(response))
}
