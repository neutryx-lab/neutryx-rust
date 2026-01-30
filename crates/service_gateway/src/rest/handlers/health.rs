//! Health check handler

use axum::Json;

use crate::rest::dto::HealthResponse;

/// Health check endpoint
///
/// GET /health
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
