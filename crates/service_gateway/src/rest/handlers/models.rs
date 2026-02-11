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
    error::{AppJson, ServerError},
    rest::dto::{
        CreateModelRequest, CreateModelResponse, GetModelResponse, ModelPricingRequest,
        ModelPricingResponse,
    },
    services::ModelService,
    state::AppState,
};

#[cfg(feature = "models")]
json_created_handler! {
    /// POST /api/v1/models
    pub async fn create_model(CreateModelRequest => CreateModelResponse) = ModelService::create_model;
}

#[cfg(feature = "models")]
path_handler! {
    /// GET /api/v1/models/{id}
    pub async fn get_model(=> GetModelResponse) = ModelService::get_model;
}

#[cfg(feature = "models")]
path_json_handler! {
    /// POST /api/v1/models/{id}/price
    pub async fn price_with_model(ModelPricingRequest => ModelPricingResponse) = ModelService::price_with_model;
}
