//! XVA calculation endpoints
//!
//! Provides endpoints for portfolio XVA (CVA, DVA, FVA) and counterparty XVA calculation.
//! These are stub implementations that will be completed in Task 7.

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

/// Build the XVA routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/xva/portfolio", post(portfolio_xva_stub))
        .route("/api/v1/xva/counterparty", post(counterparty_xva_stub))
}

/// POST /api/v1/xva/portfolio - Portfolio XVA calculation (stub)
async fn portfolio_xva_stub() -> impl IntoResponse {
    let response = NotImplementedResponse {
        error: "not_implemented".to_string(),
        message: "Portfolio XVA endpoint not yet implemented (Task 7.1)".to_string(),
    };
    (StatusCode::NOT_IMPLEMENTED, Json(response))
}

/// POST /api/v1/xva/counterparty - Counterparty XVA calculation (stub)
async fn counterparty_xva_stub() -> impl IntoResponse {
    let response = NotImplementedResponse {
        error: "not_implemented".to_string(),
        message: "Counterparty XVA endpoint not yet implemented (Task 7.2)".to_string(),
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
    async fn test_portfolio_xva_stub_returns_501() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/xva/portfolio")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_counterparty_xva_stub_returns_501() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/xva/counterparty")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_xva_routes_are_post_only() {
        let router = routes().with_state(create_test_state());

        // GET should return 405 Method Not Allowed
        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/xva/portfolio")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_xva_stub_response_has_error_message() {
        let router = routes().with_state(create_test_state());

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/xva/portfolio")
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
        assert!(stub.message.contains("XVA"));
    }
}
