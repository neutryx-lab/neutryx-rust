//! Health check and monitoring endpoints
//!
//! Provides health, readiness, and metrics endpoints for load balancer integration
//! and service availability monitoring.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Serialize;

use super::AppState;

/// Health check response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    /// Health status ("healthy" or "unhealthy")
    pub status: String,
    /// Server version
    pub version: String,
    /// Server uptime in seconds
    pub uptime_secs: u64,
    /// Dependency status
    pub dependencies: DependencyStatus,
}

/// Dependency status for health check
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    /// pricer_core availability
    pub pricer_core: bool,
    /// pricer_models availability
    pub pricer_models: bool,
    /// pricer_xva availability
    pub pricer_xva: bool,
    /// pricer_kernel availability (feature flag dependent)
    pub pricer_kernel: bool,
}

/// Readiness response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadyResponse {
    /// Ready status
    pub ready: bool,
}

/// Build the health routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
}

/// GET /health - Health check endpoint
///
/// Returns the server health status, version, uptime, and dependency status.
async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: crate::VERSION.to_string(),
        uptime_secs: uptime,
        dependencies: DependencyStatus {
            pricer_core: true,
            pricer_models: true,
            pricer_xva: true,
            pricer_kernel: state.config.kernel_enabled,
        },
    };

    (StatusCode::OK, Json(response))
}

/// GET /ready - Readiness probe endpoint
///
/// Returns 200 OK when the server is ready to accept requests.
async fn ready_handler() -> impl IntoResponse {
    let response = ReadyResponse { ready: true };
    (StatusCode::OK, Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use axum::body::Body;
    use axum::http::Request;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn create_test_state() -> AppState {
        AppState::new(Arc::new(ServerConfig::default()))
    }

    #[tokio::test]
    async fn test_health_endpoint_returns_200() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_endpoint_returns_json() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(health.status, "healthy");
        assert_eq!(health.version, crate::VERSION);
        assert!(health.dependencies.pricer_core);
        assert!(health.dependencies.pricer_models);
        assert!(health.dependencies.pricer_xva);
    }

    #[tokio::test]
    async fn test_health_uptime_increases() {
        let state = create_test_state();
        let router = routes().with_state(state.clone());

        // First request
        let response = router
            .clone()
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let health1: HealthResponse = serde_json::from_slice(&body).unwrap();

        // Wait a bit
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Second request
        let response = router
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let health2: HealthResponse = serde_json::from_slice(&body).unwrap();

        assert!(health2.uptime_secs >= health1.uptime_secs);
    }

    #[tokio::test]
    async fn test_ready_endpoint_returns_200() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ready_endpoint_returns_json() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let ready: ReadyResponse = serde_json::from_slice(&body).unwrap();

        assert!(ready.ready);
    }

    #[tokio::test]
    async fn test_health_response_camel_case() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json_str = std::str::from_utf8(&body).unwrap();

        // Check camelCase field names
        assert!(json_str.contains("uptimeSecs"));
        assert!(json_str.contains("pricerCore"));
        assert!(json_str.contains("pricerModels"));
        assert!(json_str.contains("pricerXva"));
        assert!(json_str.contains("pricerKernel"));
    }
}
