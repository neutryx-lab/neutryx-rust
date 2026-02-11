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
    error::{AppJson, ServerError},
    rest::dto::{
        BuildFxVolSurfaceRequest, BuildFxVolSurfaceResponse, BuildVolCubeRequest,
        BuildVolCubeResponse, GetImpliedVolRequest, GetImpliedVolResponse,
    },
    services::VolatilityService,
    state::AppState,
};

#[cfg(feature = "volatility")]
json_created_handler! {
    /// POST /api/v1/volatility/fx-surface
    pub async fn build_fx_vol_surface(BuildFxVolSurfaceRequest => BuildFxVolSurfaceResponse) = VolatilityService::build_fx_vol_surface;
}

#[cfg(feature = "volatility")]
json_created_handler! {
    /// POST /api/v1/volatility/cube
    pub async fn build_vol_cube(BuildVolCubeRequest => BuildVolCubeResponse) = VolatilityService::build_vol_cube;
}

#[cfg(feature = "volatility")]
path_json_handler! {
    /// POST /api/v1/volatility/{id}/implied-vol
    pub async fn get_implied_vol(GetImpliedVolRequest => GetImpliedVolResponse) = VolatilityService::get_implied_vol;
}
