//! Web dashboard module for FrictionalBank demo.
//!
//! Provides a browser-based dashboard with:
//! - REST API for portfolio and risk data
//! - WebSocket for real-time updates
//! - Static file serving for HTML/JS/CSS

pub mod handlers;
pub mod websocket;

use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Broadcast channel for real-time updates
    pub tx: broadcast::Sender<String>,
}

impl AppState {
    /// Create new application state
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the web application router
pub fn build_router(state: Arc<AppState>) -> Router {
    // CORS configuration for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // API routes
    let api_routes = Router::new()
        .route("/health", get(handlers::health))
        .route("/portfolio", get(handlers::get_portfolio))
        .route("/portfolio", post(handlers::price_portfolio))
        .route("/exposure", get(handlers::get_exposure))
        .route("/risk", get(handlers::get_risk_metrics))
        .route("/ws", get(websocket::ws_handler));

    // Static file serving for the dashboard
    let static_files = ServeDir::new("demo/gui/static");

    Router::new()
        .nest("/api", api_routes)
        .fallback_service(static_files)
        .layer(cors)
        .with_state(state)
}

/// Run the web server
pub async fn run_server(addr: SocketAddr) -> anyhow::Result<()> {
    let state = Arc::new(AppState::new());
    let app = build_router(state);

    info!("Starting web dashboard at http://{}", addr);
    info!("Open http://{} in your browser", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let state = AppState::new();
        // Should be able to subscribe to the broadcast channel
        let _rx = state.tx.subscribe();
    }

    #[test]
    fn test_router_builds() {
        let state = Arc::new(AppState::new());
        let _router = build_router(state);
    }
}
