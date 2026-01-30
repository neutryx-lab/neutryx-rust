//! REST API routes (Axum)

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

pub mod dto;
mod graph_handlers;
mod handlers;
mod ws_handlers;

// Re-export new handlers
pub use handlers::{
    build_curve, get_discount_factor, get_forward_rate, health, price_instrument, price_portfolio,
};

// Re-export existing types for backwards compatibility
pub use graph_handlers::GraphAppState;
pub use ws_handlers::WsAppState;

use crate::state::AppState;

/// Create the REST API router (legacy v1 endpoints)
pub fn create_router() -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // API v1 routes (legacy, kept for compatibility)
        .nest("/api/v1", api_v1_routes())
}

/// Create the REST API router with full state (v2 endpoints + graph/ws)
pub fn create_router_with_state(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // API v1 routes (legacy)
        .nest("/api/v1", api_v1_routes())
        // API v2 routes (new thin handlers)
        .nest("/api/v2", api_v2_routes(state))
}

/// Create the REST API router with both graph state and WebSocket support
pub fn create_router_with_ws_state(ws_state: Arc<WsAppState>) -> Router {
    let graph_state = ws_state.graph_state.clone();
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // API v1 routes (legacy)
        .nest("/api/v1", api_v1_routes())
        // Portfolio graph routes with state
        .nest("/api/v1/portfolio", portfolio_routes(graph_state))
        // WebSocket endpoint
        .merge(ws_routes(ws_state))
}

/// Create the full router with all features
pub fn create_full_router(app_state: Arc<AppState>, ws_state: Arc<WsAppState>) -> Router {
    let graph_state = ws_state.graph_state.clone();
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // API v1 routes (legacy)
        .nest("/api/v1", api_v1_routes())
        // API v2 routes (new thin handlers)
        .nest("/api/v2", api_v2_routes(app_state))
        // Portfolio graph routes with state
        .nest("/api/v1/portfolio", portfolio_routes(graph_state))
        // WebSocket endpoint
        .merge(ws_routes(ws_state))
}

fn ws_routes(state: Arc<WsAppState>) -> Router {
    Router::new()
        .route("/ws", get(ws_handlers::ws_handler))
        .with_state(state)
}

/// Legacy v1 API routes (placeholder implementations)
fn api_v1_routes() -> Router {
    use crate::rest::handlers as legacy;
    Router::new()
        .route("/price", post(legacy::price_instrument))
        .route("/price/batch", post(legacy::price_portfolio))
}

/// New v2 API routes using facade APIs
fn api_v2_routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Pricing endpoints
        .route("/price", post(handlers::price_instrument))
        .route("/price/batch", post(handlers::price_portfolio))
        // Curve building endpoints
        .route("/curves/build", post(handlers::build_curve))
        .route("/curves/discount-factor", post(handlers::get_discount_factor))
        .route("/curves/forward-rate", post(handlers::get_forward_rate))
        .with_state(state)
}

fn portfolio_routes(state: Arc<GraphAppState>) -> Router {
    Router::new()
        .route("/graph", get(graph_handlers::get_portfolio_graph))
        .route("/trades", get(graph_handlers::get_portfolio_trades))
        .with_state(state)
}

// Keep old handlers module for v1 compatibility
mod handlers_legacy {
    pub use super::handlers::*;
}
