//! Model handlers
//!
//! Thin handlers delegating to `ModelService`.

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
    error::ServerError,
    rest::dto::{
        CreateModelRequest, CreateModelResponse, GetModelResponse, ModelPricingRequest,
        ModelPricingResponse,
    },
    services::ModelService,
    state::AppState,
};

/// Create a new stochastic model
///
/// POST /api/v1/models
#[cfg(feature = "models")]
pub async fn create_model(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateModelRequest>,
) -> Result<(StatusCode, Json<CreateModelResponse>), ServerError> {
    let response = ModelService::create_model(&request, &state)?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Get a model by ID
///
/// GET /api/v1/models/{id}
#[cfg(feature = "models")]
pub async fn get_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GetModelResponse>, ServerError> {
    let response = ModelService::get_model(&id, &state)?;
    Ok(Json(response))
}

/// Price an instrument using a model
///
/// POST /api/v1/models/{id}/price
#[cfg(feature = "models")]
pub async fn price_with_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<ModelPricingRequest>,
) -> Result<Json<ModelPricingResponse>, ServerError> {
    let response = ModelService::price_with_model(&id, &request, &state)?;
    Ok(Json(response))
}
