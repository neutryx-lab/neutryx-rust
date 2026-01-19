//! Server error types

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Server-specific errors
#[derive(Error, Debug)]
pub enum ServerError {
    /// Pricing error
    #[error("Pricing error: {0}")]
    Pricing(String),

    /// Calibration error
    #[error("Calibration error: {0}")]
    Calibration(String),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Request timeout (504 Gateway Timeout)
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ServerError::Pricing(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            ServerError::Calibration(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            ServerError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ServerError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ServerError::Timeout(msg) => (StatusCode::GATEWAY_TIMEOUT, msg.clone()),
            ServerError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(json!({
            "error": message,
            "code": status.as_u16()
        }));

        (status, body).into_response()
    }
}
