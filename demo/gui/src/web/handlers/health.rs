//! Health check and system monitoring endpoints.
//!
//! This module provides endpoints for:
//! - Health checks (`/api/health`)
//! - Performance metrics (`/api/metrics`)
//! - Index page with config injection (`/`)

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    Json,
};
use serde::Serialize;
use tower_http::services::ServeFile;

use crate::web::AppState;

// =============================================================================
// Health Check
// =============================================================================

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Service status (e.g. "ok").
    pub status: String,
    /// Application version.
    pub version: String,
}

/// Health check endpoint.
///
/// GET /api/health
///
/// Returns the service status and version information.
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// =============================================================================
// Performance Metrics
// =============================================================================

/// API response time statistics.
#[derive(Debug, Serialize)]
pub struct ApiResponseTimes {
    /// Average portfolio API response time in milliseconds.
    pub portfolio_avg_ms: f64,
    /// Average exposure API response time in milliseconds.
    pub exposure_avg_ms: f64,
    /// Average risk API response time in milliseconds.
    pub risk_avg_ms: f64,
    /// Average graph API response time in milliseconds.
    pub graph_avg_ms: f64,
}

/// Performance metrics response.
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    /// API endpoint response time statistics.
    pub api_response_times: ApiResponseTimes,
    /// Number of active WebSocket connections.
    pub websocket_connections: u32,
    /// Average WebSocket message latency in milliseconds.
    pub websocket_message_latency_ms: f64,
    /// Server uptime in seconds.
    pub uptime_seconds: u64,
}

/// Get performance metrics endpoint.
///
/// GET /api/metrics
///
/// Returns JSON with API response times, WebSocket statistics, and uptime.
pub async fn get_metrics(State(state): State<Arc<AppState>>) -> Json<MetricsResponse> {
    let metrics = &state.metrics;

    Json(MetricsResponse {
        api_response_times: ApiResponseTimes {
            portfolio_avg_ms: metrics.portfolio_avg_ms().await,
            exposure_avg_ms: metrics.exposure_avg_ms().await,
            risk_avg_ms: metrics.risk_avg_ms().await,
            graph_avg_ms: metrics.graph_avg_ms().await,
        },
        websocket_connections: metrics.ws_connection_count(),
        websocket_message_latency_ms: metrics.ws_latency_avg_ms().await,
        uptime_seconds: metrics.uptime_seconds(),
    })
}

// =============================================================================
// Index Page
// =============================================================================

/// Serve index.html with injected configuration.
///
/// GET /
///
/// Reads the index.html template and replaces the placeholder config
/// with values from environment variables (FB_DEBUG_MODE, FB_LOG_LEVEL).
pub async fn get_index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let index_path = "demo/gui/static/index.html";

    match tokio::fs::read_to_string(index_path).await {
        Ok(content) => {
            // Replace the config placeholder with actual values
            let config_script = format!(
                r#"<script id="fb-config">
        window.__FB_CONFIG__ = {{
            debugMode: {},
            logLevel: '{}'
        }};
    </script>"#,
                state.debug_config.debug_mode, state.debug_config.log_level
            );

            // Replace the placeholder config in the HTML
            let modified = content.replace(
                r#"<script id="fb-config">
        window.__FB_CONFIG__ = {
            debugMode: false,
            logLevel: 'INFO'
        };
    </script>"#,
                &config_script,
            );

            Html(modified).into_response()
        }
        Err(_) => {
            // Fallback if file cannot be read
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load index.html",
            )
                .into_response()
        }
    }
}

/// Create a service that serves index.html for fallback routes.
pub fn serve_index_with_config() -> ServeFile { ServeFile::new("demo/gui/static/index.html") }

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_returns_ok() {
        let response = health().await;
        assert_eq!(response.status, "ok");
        assert!(!response.version.is_empty());
    }

    #[test]
    fn test_health_response_serialisation() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"version\":\"1.0.0\""));
    }
}
