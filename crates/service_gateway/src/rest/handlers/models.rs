//! Model handlers.

#[cfg(feature = "models")]
use std::sync::Arc;

#[cfg(feature = "models")]
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

#[cfg(feature = "models")]
use crate::{
    error::{AppJson, ServerError},
    rest::dto::{
        CreateModelRequest, CreateModelResponse, GetModelResponse, ModelPricingRequest,
        ModelPricingResponse,
    },
    services::ModelService,
    state::AppState,
};

/// POST /api/v1/models.
#[cfg(feature = "models")]
pub async fn create_model(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<CreateModelRequest>,
) -> Result<(StatusCode, Json<CreateModelResponse>), ServerError> {
    let response = ModelService::create_model(&request, &state)?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/models/{id}.
#[cfg(feature = "models")]
pub async fn get_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GetModelResponse>, ServerError> {
    let response = ModelService::get_model(&id, &state)?;
    Ok(Json(response))
}

/// POST /api/v1/models/{id}/price.
#[cfg(feature = "models")]
pub async fn price_with_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AppJson(request): AppJson<ModelPricingRequest>,
) -> Result<Json<ModelPricingResponse>, ServerError> {
    let response = ModelService::price_with_model(&id, &request, &state)?;
    Ok(Json(response))
}
