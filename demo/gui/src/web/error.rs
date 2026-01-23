//! Unified error handling for the FrictionalBank Web API.
//!
//! This module provides a consistent error type and response format across all
//! API endpoints. It replaces the various domain-specific error types with a
//! single `ApiError` type that implements `IntoResponse` for Axum.
//!
//! # Error Categories
//!
//! - `validation`: Input validation failures (400 Bad Request)
//! - `not_found`: Resource not found (404 Not Found)
//! - `calculation`: Calculation or processing failures (422 Unprocessable
//!   Entity)
//! - `internal`: Internal server errors (500 Internal Server Error)
//!
//! # Example
//!
//! ```rust,ignore
//! use demo_gui::web::error::{ApiError, ApiResult};
//!
//! async fn my_handler() -> ApiResult<MyResponse> {
//!     let data = validate_input(&request)
//!         .map_err(|e| ApiError::validation(e.to_string(), "field_name"))?;
//!
//!     let result = compute(data)
//!         .map_err(|e| ApiError::calculation(e.to_string()))?;
//!
//!     Ok(Json(result))
//! }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

// =============================================================================
// API Error Type
// =============================================================================

/// Unified API error type for consistent error responses.
///
/// This type provides a standardised error format across all API endpoints,
/// making it easier for clients to handle errors consistently.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// Error category identifier.
    ///
    /// One of: `"validation"`, `"not_found"`, `"calculation"`, `"internal"`
    pub error: &'static str,

    /// HTTP status code as a number for client parsing.
    pub status_code: u16,

    /// Human-readable error message describing what went wrong.
    pub message: String,

    /// Field name that caused the error (for validation errors).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,

    /// Additional context information (e.g., failed tenor, invalid value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

impl ApiError {
    // =========================================================================
    // Constructor Methods
    // =========================================================================

    /// Create a validation error (400 Bad Request).
    ///
    /// Use this for input validation failures where the client provided
    /// invalid data.
    ///
    /// # Arguments
    ///
    /// * `message` - Human-readable description of the validation failure
    /// * `field` - The field name that failed validation
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// ApiError::validation("Notional must be positive", "notional")
    /// ```
    pub fn validation(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            error: "validation",
            status_code: 400,
            message: message.into(),
            field: Some(field.into()),
            context: None,
        }
    }

    /// Create a validation error without a specific field.
    ///
    /// Use this for validation errors that span multiple fields or apply
    /// to the request as a whole.
    pub fn validation_general(message: impl Into<String>) -> Self {
        Self {
            error: "validation",
            status_code: 400,
            message: message.into(),
            field: None,
            context: None,
        }
    }

    /// Create a not found error (404 Not Found).
    ///
    /// Use this when a requested resource does not exist.
    ///
    /// # Arguments
    ///
    /// * `resource` - The type of resource (e.g., "Curve", "Trade", "Job")
    /// * `id` - The identifier that was not found
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// ApiError::not_found("Curve", curve_id)
    /// ```
    pub fn not_found(resource: &str, id: impl std::fmt::Display) -> Self {
        Self {
            error: "not_found",
            status_code: 404,
            message: format!("{} '{}' not found", resource, id),
            field: None,
            context: None,
        }
    }

    /// Create a calculation error (422 Unprocessable Entity).
    ///
    /// Use this when input is syntactically valid but cannot be processed
    /// (e.g., bootstrap convergence failure, numerical errors).
    ///
    /// # Arguments
    ///
    /// * `message` - Description of the calculation failure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// ApiError::calculation("Bootstrap failed to converge at tenor 5Y")
    /// ```
    pub fn calculation(message: impl Into<String>) -> Self {
        Self {
            error: "calculation",
            status_code: 422,
            message: message.into(),
            field: None,
            context: None,
        }
    }

    /// Create an internal server error (500 Internal Server Error).
    ///
    /// Use this for unexpected errors that are not the client's fault.
    ///
    /// # Arguments
    ///
    /// * `message` - Description of the internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            error: "internal",
            status_code: 500,
            message: message.into(),
            field: None,
            context: None,
        }
    }

    // =========================================================================
    // Builder Methods
    // =========================================================================

    /// Add context information to the error.
    ///
    /// Context can include additional details like the failed tenor,
    /// the invalid value, or other relevant information.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// ApiError::calculation("Bootstrap failed")
    ///     .with_context(serde_json::json!({
    ///         "failedTenor": "5Y",
    ///         "iteration": 100
    ///     }))
    /// ```
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    /// Add a field name to the error.
    ///
    /// Useful when you need to specify the field after creating the error.
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    // =========================================================================
    // Utility Methods
    // =========================================================================

    /// Get the HTTP status code for this error.
    pub fn status(&self) -> StatusCode {
        StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Check if this is a client error (4xx).
    pub fn is_client_error(&self) -> bool { (400..500).contains(&self.status_code) }

    /// Check if this is a server error (5xx).
    pub fn is_server_error(&self) -> bool { (500..600).contains(&self.status_code) }
}

// =============================================================================
// IntoResponse Implementation
// =============================================================================

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        (status, Json(self)).into_response()
    }
}

// =============================================================================
// Display Implementation
// =============================================================================

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.error, self.message)?;
        if let Some(ref field) = self.field {
            write!(f, " (field: {})", field)?;
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}

// =============================================================================
// Type Aliases
// =============================================================================

/// Result type alias for API handlers.
///
/// Use this as the return type for handlers that can fail with an `ApiError`.
///
/// # Example
///
/// ```rust,ignore
/// pub async fn my_handler(
///     State(state): State<Arc<AppState>>,
///     Json(request): Json<MyRequest>,
/// ) -> ApiResult<MyResponse> {
///     // ...
///     Ok(Json(response))
/// }
/// ```
pub type ApiResult<T> = Result<Json<T>, ApiError>;

// =============================================================================
// Conversion Traits
// =============================================================================

/// Trait for converting domain errors into API errors.
///
/// Implement this trait for domain-specific error types to enable
/// automatic conversion using the `?` operator.
pub trait IntoApiError {
    /// Convert this error into an `ApiError`.
    fn into_api_error(self) -> ApiError;
}

impl<T: IntoApiError> From<T> for ApiError {
    fn from(err: T) -> Self { err.into_api_error() }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error() {
        let error = ApiError::validation("Notional must be positive", "notional");
        assert_eq!(error.error, "validation");
        assert_eq!(error.status_code, 400);
        assert_eq!(error.field, Some("notional".to_string()));
        assert!(error.is_client_error());
        assert!(!error.is_server_error());
    }

    #[test]
    fn test_not_found_error() {
        let error = ApiError::not_found("Curve", "abc-123");
        assert_eq!(error.error, "not_found");
        assert_eq!(error.status_code, 404);
        assert_eq!(error.message, "Curve 'abc-123' not found");
        assert!(error.is_client_error());
    }

    #[test]
    fn test_calculation_error() {
        let error = ApiError::calculation("Bootstrap failed to converge");
        assert_eq!(error.error, "calculation");
        assert_eq!(error.status_code, 422);
        assert!(error.is_client_error());
    }

    #[test]
    fn test_internal_error() {
        let error = ApiError::internal("Database connection failed");
        assert_eq!(error.error, "internal");
        assert_eq!(error.status_code, 500);
        assert!(!error.is_client_error());
        assert!(error.is_server_error());
    }

    #[test]
    fn test_with_context() {
        let error = ApiError::calculation("Bootstrap failed")
            .with_context(serde_json::json!({ "failedTenor": "5Y" }));

        assert!(error.context.is_some());
        let ctx = error.context.as_ref().unwrap();
        assert_eq!(ctx["failedTenor"], "5Y");
    }

    #[test]
    fn test_display() {
        let error = ApiError::validation("Invalid value", "field_name");
        let display = format!("{}", error);
        assert!(display.contains("[validation]"));
        assert!(display.contains("Invalid value"));
        assert!(display.contains("field_name"));
    }

    #[test]
    fn test_serialisation() {
        let error = ApiError::validation("Test error", "test_field");
        let json = serde_json::to_string(&error).unwrap();

        // Should use camelCase
        assert!(json.contains("\"statusCode\""));
        assert!(!json.contains("\"status_code\""));

        // Should not include null values
        let error_no_context = ApiError::validation("Test", "field");
        let json_no_context = serde_json::to_string(&error_no_context).unwrap();
        assert!(!json_no_context.contains("context"));
    }

    #[test]
    fn test_status_conversion() {
        assert_eq!(
            ApiError::validation("", "").status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(ApiError::not_found("", "").status(), StatusCode::NOT_FOUND);
        assert_eq!(
            ApiError::calculation("").status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ApiError::internal("").status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
