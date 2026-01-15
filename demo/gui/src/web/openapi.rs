//! OpenAPI documentation module for FrictionalBank WebApp.
//!
//! Provides Swagger UI at `/api/docs` for interactive API exploration.
//!
//! # Task Coverage
//!
//! - Task 8.1: OpenAPI ドキュメント生成の設定
//!
//! # Requirements Coverage
//!
//! - Requirement 7.5: OpenAPI 3.0 形式の API ドキュメントを `/api/docs` で提供

#[cfg(feature = "openapi")]
use utoipa::OpenApi;

#[cfg(feature = "openapi")]
use utoipa_swagger_ui::SwaggerUi;

#[cfg(feature = "openapi")]
use axum::Router;

use crate::web::pricer_types::{
    BucketDv01Request, BucketDv01Response, DeltaResult, FirstOrderGreeksRequest,
    FirstOrderGreeksResponse, GreeksCalculationMode, GreeksCompareRequest, GreeksCompareResponse,
    GreeksDiff, GreeksMethodResult, GreekValue, PaymentFrequency, SecondOrderGreeksRequest,
    SecondOrderGreeksResponse, TenorDiff, TimingComparison, TimingStats,
};

use crate::web::jobs::{JobEntry, JobStatus};

// =============================================================================
// OpenAPI Schema Implementation
// =============================================================================

/// OpenAPI documentation for the FrictionalBank WebApp API.
///
/// This struct configures the OpenAPI specification with all available endpoints,
/// request/response schemas, and documentation metadata.
#[cfg(feature = "openapi")]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "FrictionalBank WebApp API",
        version = "1.0.0",
        description = "REST API for derivatives pricing, Greeks calculation, and risk analytics.\n\n## Features\n\n- IRS pricing with Bump and AAD methods\n- First and second-order Greeks calculation\n- Bucket DV01 and Key Rate Duration\n- Asynchronous job management\n- Real-time WebSocket updates",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
        contact(name = "Neutryx Lab", url = "https://github.com/neutryx-lab/neutryx-rust")
    ),
    servers(
        (url = "/api", description = "Local API server")
    ),
    tags(
        (name = "greeks", description = "Greeks calculation endpoints"),
        (name = "jobs", description = "Asynchronous job management"),
        (name = "scenarios", description = "Scenario analysis endpoints"),
        (name = "health", description = "Health check endpoints")
    ),
    paths(
        crate::web::handlers::health,
        crate::web::handlers::greeks_compare,
        crate::web::handlers::greeks_first_order,
        crate::web::handlers::greeks_second_order,
        crate::web::handlers::list_jobs,
        crate::web::handlers::get_job_status,
    ),
    components(
        schemas(
            GreeksCompareRequest,
            GreeksCompareResponse,
            GreeksMethodResult,
            GreeksDiff,
            GreekValue,
            TenorDiff,
            TimingComparison,
            TimingStats,
            DeltaResult,
            PaymentFrequency,
            GreeksCalculationMode,
            FirstOrderGreeksRequest,
            FirstOrderGreeksResponse,
            SecondOrderGreeksRequest,
            SecondOrderGreeksResponse,
            BucketDv01Request,
            BucketDv01Response,
            JobStatus,
            JobEntry,
            ApiError,
            ValidationErrorResponse,
        )
    )
)]
pub struct ApiDoc;

/// Standard API error response.
///
/// Used for all error responses in the API.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// Error code (e.g., "INVALID_REQUEST", "COMPUTATION_FAILED")
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    /// Create a new API error.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create an API error with additional details.
    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: Some(details),
        }
    }

    /// Invalid request error (400).
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new("INVALID_REQUEST", message)
    }

    /// Computation failed error (500).
    pub fn computation_failed(message: impl Into<String>) -> Self {
        Self::new("COMPUTATION_FAILED", message)
    }

    /// NaN detected error (422).
    pub fn nan_detected(field: &str, value: f64) -> Self {
        Self::with_details(
            "NAN_DETECTED",
            format!("NaN or Inf detected in field: {}", field),
            serde_json::json!({ "field": field, "value": value.to_string() }),
        )
    }

    /// Job not found error (404).
    pub fn job_not_found(job_id: &str) -> Self {
        Self::with_details(
            "JOB_NOT_FOUND",
            format!("Job not found: {}", job_id),
            serde_json::json!({ "job_id": job_id }),
        )
    }

    /// Curve not found error (404).
    pub fn curve_not_found(curve_id: &str) -> Self {
        Self::with_details(
            "CURVE_NOT_FOUND",
            format!("Curve not found: {}", curve_id),
            serde_json::json!({ "curve_id": curve_id }),
        )
    }
}

/// Error codes for API responses.
pub mod error_codes {
    /// Invalid request parameters
    pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
    /// Computation failed
    pub const COMPUTATION_FAILED: &str = "COMPUTATION_FAILED";
    /// NaN or Inf detected in result
    pub const NAN_DETECTED: &str = "NAN_DETECTED";
    /// Job not found
    pub const JOB_NOT_FOUND: &str = "JOB_NOT_FOUND";
    /// Curve not found
    pub const CURVE_NOT_FOUND: &str = "CURVE_NOT_FOUND";
    /// Validation failed
    pub const VALIDATION_FAILED: &str = "VALIDATION_FAILED";
}

/// Validation error response.
///
/// Contains detailed information about validation failures.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct ValidationErrorResponse {
    /// Error code (always "VALIDATION_FAILED")
    pub code: String,
    /// Summary error message
    pub message: String,
    /// List of field-level validation errors
    pub errors: Vec<FieldError>,
}

/// Field-level validation error.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct FieldError {
    /// Field name that failed validation
    pub field: String,
    /// Validation error message
    pub message: String,
    /// Rejected value (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_value: Option<serde_json::Value>,
}

impl ValidationErrorResponse {
    /// Create a new validation error response.
    pub fn new(errors: Vec<FieldError>) -> Self {
        let message = if errors.len() == 1 {
            format!("Validation failed: {}", errors[0].message)
        } else {
            format!("Validation failed with {} errors", errors.len())
        };

        Self {
            code: error_codes::VALIDATION_FAILED.to_string(),
            message,
            errors,
        }
    }

    /// Create a validation error for a single field.
    pub fn single(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(vec![FieldError {
            field: field.into(),
            message: message.into(),
            rejected_value: None,
        }])
    }

    /// Create a validation error for a single field with rejected value.
    pub fn single_with_value(
        field: impl Into<String>,
        message: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        Self::new(vec![FieldError {
            field: field.into(),
            message: message.into(),
            rejected_value: Some(value),
        }])
    }
}

// =============================================================================
// Swagger UI Router
// =============================================================================

/// Create a router with Swagger UI at `/api/docs`.
///
/// This function configures the Swagger UI to serve the OpenAPI specification
/// generated from the `ApiDoc` struct.
///
/// # Example
///
/// ```ignore
/// use demo_gui::web::openapi::swagger_ui_router;
///
/// let app = Router::new()
///     .merge(swagger_ui_router())
///     .nest("/api", api_routes);
/// ```
#[cfg(feature = "openapi")]
pub fn swagger_ui_router() -> Router {
    SwaggerUi::new("/api/docs")
        .url("/api/openapi.json", ApiDoc::openapi())
        .into()
}

/// Get the OpenAPI JSON specification.
///
/// Returns the raw OpenAPI JSON for programmatic access.
#[cfg(feature = "openapi")]
pub fn openapi_json() -> String {
    ApiDoc::openapi().to_json().expect("Failed to generate OpenAPI JSON")
}

/// Get the OpenAPI YAML specification.
///
/// Returns the raw OpenAPI YAML for programmatic access.
#[cfg(feature = "openapi")]
pub fn openapi_yaml() -> String {
    ApiDoc::openapi().to_yaml().expect("Failed to generate OpenAPI YAML")
}

// =============================================================================
// Stub implementations when openapi feature is disabled
// =============================================================================

/// Stub router when openapi feature is disabled.
#[cfg(not(feature = "openapi"))]
pub fn swagger_ui_router() -> axum::Router {
    axum::Router::new()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ApiError Tests
    // =========================================================================

    #[test]
    fn test_api_error_new() {
        let error = ApiError::new("TEST_ERROR", "Test message");
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test message");
        assert!(error.details.is_none());
    }

    #[test]
    fn test_api_error_with_details() {
        let details = serde_json::json!({"key": "value"});
        let error = ApiError::with_details("TEST_ERROR", "Test message", details.clone());
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test message");
        assert_eq!(error.details.unwrap(), details);
    }

    #[test]
    fn test_api_error_invalid_request() {
        let error = ApiError::invalid_request("Invalid notional");
        assert_eq!(error.code, "INVALID_REQUEST");
        assert_eq!(error.message, "Invalid notional");
    }

    #[test]
    fn test_api_error_computation_failed() {
        let error = ApiError::computation_failed("Calculation error");
        assert_eq!(error.code, "COMPUTATION_FAILED");
        assert_eq!(error.message, "Calculation error");
    }

    #[test]
    fn test_api_error_nan_detected() {
        let error = ApiError::nan_detected("delta", f64::NAN);
        assert_eq!(error.code, "NAN_DETECTED");
        assert!(error.message.contains("delta"));
        assert!(error.details.is_some());
    }

    #[test]
    fn test_api_error_job_not_found() {
        let error = ApiError::job_not_found("test-job-id");
        assert_eq!(error.code, "JOB_NOT_FOUND");
        assert!(error.message.contains("test-job-id"));
    }

    #[test]
    fn test_api_error_curve_not_found() {
        let error = ApiError::curve_not_found("test-curve-id");
        assert_eq!(error.code, "CURVE_NOT_FOUND");
        assert!(error.message.contains("test-curve-id"));
    }

    #[test]
    fn test_api_error_serialization() {
        let error = ApiError::new("TEST", "Test");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("TEST"));
        assert!(json.contains("Test"));
    }

    // =========================================================================
    // ValidationErrorResponse Tests
    // =========================================================================

    #[test]
    fn test_validation_error_single() {
        let error = ValidationErrorResponse::single("notional", "must be positive");
        assert_eq!(error.code, "VALIDATION_FAILED");
        assert_eq!(error.errors.len(), 1);
        assert_eq!(error.errors[0].field, "notional");
        assert_eq!(error.errors[0].message, "must be positive");
    }

    #[test]
    fn test_validation_error_single_with_value() {
        let error = ValidationErrorResponse::single_with_value(
            "notional",
            "must be positive",
            serde_json::json!(-100.0),
        );
        assert_eq!(error.errors[0].rejected_value, Some(serde_json::json!(-100.0)));
    }

    #[test]
    fn test_validation_error_multiple() {
        let errors = vec![
            FieldError {
                field: "notional".to_string(),
                message: "must be positive".to_string(),
                rejected_value: Some(serde_json::json!(-100.0)),
            },
            FieldError {
                field: "fixed_rate".to_string(),
                message: "must be between -1 and 1".to_string(),
                rejected_value: Some(serde_json::json!(2.0)),
            },
        ];
        let response = ValidationErrorResponse::new(errors);
        assert_eq!(response.errors.len(), 2);
        assert!(response.message.contains("2 errors"));
    }

    #[test]
    fn test_validation_error_serialization() {
        let error = ValidationErrorResponse::single("field", "error");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("VALIDATION_FAILED"));
        assert!(json.contains("field"));
    }

    // =========================================================================
    // Error Codes Tests
    // =========================================================================

    #[test]
    fn test_error_codes() {
        assert_eq!(error_codes::INVALID_REQUEST, "INVALID_REQUEST");
        assert_eq!(error_codes::COMPUTATION_FAILED, "COMPUTATION_FAILED");
        assert_eq!(error_codes::NAN_DETECTED, "NAN_DETECTED");
        assert_eq!(error_codes::JOB_NOT_FOUND, "JOB_NOT_FOUND");
        assert_eq!(error_codes::CURVE_NOT_FOUND, "CURVE_NOT_FOUND");
        assert_eq!(error_codes::VALIDATION_FAILED, "VALIDATION_FAILED");
    }

    // =========================================================================
    // OpenAPI Feature Tests (only with feature enabled)
    // =========================================================================

    #[cfg(feature = "openapi")]
    mod openapi_tests {
        use super::*;

        #[test]
        fn test_openapi_json_generation() {
            let json = openapi_json();
            assert!(json.contains("FrictionalBank WebApp API"));
            assert!(json.contains("openapi"));
            assert!(json.contains("3.1")); // OpenAPI version
        }

        #[test]
        fn test_openapi_yaml_generation() {
            let yaml = openapi_yaml();
            assert!(yaml.contains("FrictionalBank WebApp API"));
            assert!(yaml.contains("openapi"));
        }

        #[test]
        fn test_openapi_contains_paths() {
            let json = openapi_json();
            assert!(json.contains("/health"));
            assert!(json.contains("/greeks/compare"));
            assert!(json.contains("/greeks/first-order"));
            assert!(json.contains("/greeks/second-order"));
        }

        #[test]
        fn test_openapi_contains_schemas() {
            let json = openapi_json();
            assert!(json.contains("GreeksCompareRequest"));
            assert!(json.contains("GreeksCompareResponse"));
            assert!(json.contains("ApiError"));
        }

        #[test]
        fn test_swagger_ui_router_builds() {
            let _router = swagger_ui_router();
        }
    }
}
