//! Option pricing endpoints
//!
//! Provides endpoints for vanilla, Asian, barrier, and lookback option pricing.
//! These are stub implementations that will be completed in Task 5.

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

/// Build the pricing routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/price/vanilla", post(vanilla_price_stub))
        .route("/api/v1/price/asian", post(asian_price_stub))
        .route("/api/v1/price/barrier", post(barrier_price_stub))
        .route("/api/v1/price/lookback", post(lookback_price_stub))
}

/// POST /api/v1/price/vanilla - Vanilla option pricing (stub)
async fn vanilla_price_stub() -> impl IntoResponse {
    let response = NotImplementedResponse {
        error: "not_implemented".to_string(),
        message: "Vanilla pricing endpoint not yet implemented (Task 5.1)".to_string(),
    };
    (StatusCode::NOT_IMPLEMENTED, Json(response))
}

/// POST /api/v1/price/asian - Asian option pricing (stub)
async fn asian_price_stub() -> impl IntoResponse {
    let response = NotImplementedResponse {
        error: "not_implemented".to_string(),
        message: "Asian pricing endpoint not yet implemented (Task 5.2)".to_string(),
    };
    (StatusCode::NOT_IMPLEMENTED, Json(response))
}

/// POST /api/v1/price/barrier - Barrier option pricing (stub)
async fn barrier_price_stub() -> impl IntoResponse {
    let response = NotImplementedResponse {
        error: "not_implemented".to_string(),
        message: "Barrier pricing endpoint not yet implemented (Task 5.3)".to_string(),
    };
    (StatusCode::NOT_IMPLEMENTED, Json(response))
}

/// POST /api/v1/price/lookback - Lookback option pricing (stub)
async fn lookback_price_stub() -> impl IntoResponse {
    let response = NotImplementedResponse {
        error: "not_implemented".to_string(),
        message: "Lookback pricing endpoint not yet implemented (Task 5.4)".to_string(),
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
    async fn test_vanilla_price_stub_returns_501() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/price/vanilla")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_asian_price_stub_returns_501() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/price/asian")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_barrier_price_stub_returns_501() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/price/barrier")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_lookback_price_stub_returns_501() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/price/lookback")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_pricing_routes_are_post_only() {
        let router = routes().with_state(create_test_state());

        // GET should return 405 Method Not Allowed
        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/price/vanilla")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_stub_response_has_error_message() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/price/vanilla")
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
        assert!(!stub.message.is_empty());
    }
}
