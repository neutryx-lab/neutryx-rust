//! Route modules for the pricer server
//!
//! This module contains endpoint group-specific routers:
//! - pricing: Option pricing endpoints
//! - greeks: Greeks calculation endpoints
//! - xva: XVA calculation endpoints
//! - health: Health check and monitoring endpoints

pub mod greeks;
pub mod health;
pub mod pricing;
pub mod xva;

use axum::Router;
use std::sync::Arc;

use crate::config::ServerConfig;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// Server configuration
    pub config: Arc<ServerConfig>,
    /// Server start time for uptime calculation
    pub start_time: std::time::Instant,
}

impl AppState {
    /// Create a new AppState
    pub fn new(config: Arc<ServerConfig>) -> Self {
        Self {
            config,
            start_time: std::time::Instant::now(),
        }
    }
}

/// Build the main application router by merging all route modules
pub fn build_router(config: Arc<ServerConfig>) -> Router {
    let state = AppState::new(config);

    Router::new()
        .merge(health::routes())
        .merge(pricing::routes())
        .merge(greeks::routes())
        .merge(xva::routes())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_build_router_creates_valid_router() {
        let config = Arc::new(ServerConfig::default());
        let router = build_router(config);

        // Test that the router is created and can handle requests
        let response = router
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Health endpoint should return 200
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_router_merges_all_route_groups() {
        let config = Arc::new(ServerConfig::default());
        let router = build_router(config);

        // Test health routes
        let response = router
            .clone()
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test ready endpoint
        let response = router
            .clone()
            .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test pricing routes (stub should return 501)
        let response = router
            .clone()
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
        // Stub returns 501 Not Implemented
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        // Test greeks routes (stub should return 501)
        let response = router
            .clone()
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

        // Test xva routes (stub should return 501)
        let response = router
            .clone()
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
    async fn test_unknown_route_returns_404() {
        let config = Arc::new(ServerConfig::default());
        let router = build_router(config);

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/unknown/path")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_app_state_uptime() {
        let config = Arc::new(ServerConfig::default());
        let state = AppState::new(config);

        // Wait a tiny bit
        std::thread::sleep(std::time::Duration::from_millis(10));

        let elapsed = state.start_time.elapsed();
        assert!(elapsed.as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_app_state_config_access() {
        let mut config = ServerConfig::default();
        config.port = 9999;
        let config = Arc::new(config);
        let state = AppState::new(config.clone());

        assert_eq!(state.config.port, 9999);
    }
}
