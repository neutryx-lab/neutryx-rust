//! Server error types.

use async_trait::async_trait;
use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use thiserror::Error;
use validator::Validate;

/// Server-specific errors with domain separation.
#[derive(Error, Debug)]
pub enum ServerError {
    /// Computation error (pricing, calibration, risk, model, volatility).
    #[error("Pricing error: {0}")]
    Pricing(String),

    /// Invalid request or argument.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Resource not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Request timeout (504 Gateway Timeout).
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl ServerError {
    /// Get the error code for structured responses.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Pricing(_) => "PRICING_ERROR",
            Self::InvalidRequest(_) => "INVALID_REQUEST",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Timeout(_) => "TIMEOUT",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::Io(_) => "IO_ERROR",
        }
    }
}

impl From<JsonRejection> for ServerError {
    fn from(rejection: JsonRejection) -> Self { Self::InvalidRequest(rejection.body_text()) }
}

impl From<serde_json::Error> for ServerError {
    fn from(err: serde_json::Error) -> Self { Self::Internal(format!("JSON error: {err}")) }
}

/// Custom JSON extractor that converts deserialisation failures into.
pub struct AppJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for AppJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ServerError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(ServerError::from(rejection)),
        }
    }
}

/// JSON extractor with automatic validation via the `validator` crate.
///
/// Deserialises the request body, then calls [`Validate::validate`] before
/// yielding the value.  Validation errors are mapped to
/// [`ServerError::InvalidRequest`] with a human-readable summary.
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ServerError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(ServerError::from)?;
        value
            .validate()
            .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
        Ok(Self(value))
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ServerError::Pricing(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            ServerError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ServerError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ServerError::Timeout(msg) => (StatusCode::GATEWAY_TIMEOUT, msg.clone()),
            ServerError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            ServerError::Io(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        };

        let body = Json(json!({
            "error": message,
            "error_code": self.error_code(),
            "code": status.as_u16()
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_mapping() {
        assert_eq!(
            ServerError::Pricing("test".to_string()).error_code(),
            "PRICING_ERROR"
        );
        assert_eq!(
            ServerError::InvalidRequest("test".to_string()).error_code(),
            "INVALID_REQUEST"
        );
        assert_eq!(
            ServerError::NotFound("test".to_string()).error_code(),
            "NOT_FOUND"
        );
    }

    #[test]
    fn test_error_display() {
        let err = ServerError::Pricing("calculation failed".to_string());
        assert_eq!(format!("{err}"), "Pricing error: calculation failed");

        let err = ServerError::NotFound("resource missing".to_string());
        assert_eq!(format!("{err}"), "Not found: resource missing");
    }
}
