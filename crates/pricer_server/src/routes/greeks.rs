//! Greeks calculation endpoints
//!
//! Provides endpoints for option Greeks (delta, gamma, vega, theta, rho) calculation.
//! This is a stub implementation that will be completed in Task 6.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};

use super::AppState;

/// Stub response for not-yet-implemented endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotImplementedResponse {
    pub error: String,
    pub message: String,
}

/// Build the greeks routes
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/greeks", post(greeks_stub))
}

/// POST /api/v1/greeks - Greeks calculation (stub)
async fn greeks_stub() -> impl IntoResponse {
    let response = NotImplementedResponse {
        error: "not_implemented".to_string(),
        message: "Greeks calculation endpoint not yet implemented (Task 6.1)".to_string(),
    };
    (StatusCode::NOT_IMPLEMENTED, Json(response))
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
    async fn test_greeks_stub_returns_501() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/greeks")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_greeks_route_is_post_only() {
        let router = routes().with_state(create_test_state());

        // GET should return 405 Method Not Allowed
        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/greeks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_greeks_stub_response_has_error_message() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/greeks")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let stub: NotImplementedResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(stub.error, "not_implemented");
        assert!(stub.message.contains("Greeks"));
    }
}
