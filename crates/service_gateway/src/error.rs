//! Server error types
//!
//! Provides domain-specific error types for the service layer with
//! automatic HTTP status code mapping.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Server-specific errors with domain separation
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

    // Domain-specific error variants (Requirement 9)
    /// Risk calculation error (Greeks, scenarios)
    #[error("Risk error: {0}")]
    Risk(String),

    /// Portfolio management error
    #[error("Portfolio error: {0}")]
    Portfolio(String),

    /// Model configuration error
    #[error("Model error: {0}")]
    Model(String),

    /// Volatility surface/cube error
    #[error("Volatility error: {0}")]
    Volatility(String),
}

impl ServerError {
    /// Get the error code for structured responses
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Pricing(_) => "PRICING_ERROR",
            Self::Calibration(_) => "CALIBRATION_ERROR",
            Self::InvalidRequest(_) => "INVALID_REQUEST",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Timeout(_) => "TIMEOUT",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::Risk(_) => "RISK_ERROR",
            Self::Portfolio(_) => "PORTFOLIO_ERROR",
            Self::Model(_) => "MODEL_ERROR",
            Self::Volatility(_) => "VOLATILITY_ERROR",
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ServerError::Pricing(msg) | ServerError::Calibration(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, msg.clone())
            }
            ServerError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ServerError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ServerError::Timeout(msg) => (StatusCode::GATEWAY_TIMEOUT, msg.clone()),
            ServerError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            // Domain errors map to 422 Unprocessable Entity
            ServerError::Risk(msg)
            | ServerError::Portfolio(msg)
            | ServerError::Model(msg)
            | ServerError::Volatility(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
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
            ServerError::Risk("test".to_string()).error_code(),
            "RISK_ERROR"
        );
        assert_eq!(
            ServerError::Portfolio("test".to_string()).error_code(),
            "PORTFOLIO_ERROR"
        );
        assert_eq!(
            ServerError::Model("test".to_string()).error_code(),
            "MODEL_ERROR"
        );
        assert_eq!(
            ServerError::Volatility("test".to_string()).error_code(),
            "VOLATILITY_ERROR"
        );
    }

    #[test]
    fn test_error_display() {
        let err = ServerError::Risk("calculation failed".to_string());
        assert_eq!(format!("{err}"), "Risk error: calculation failed");

        let err = ServerError::Portfolio("not found".to_string());
        assert_eq!(format!("{err}"), "Portfolio error: not found");
    }
}
