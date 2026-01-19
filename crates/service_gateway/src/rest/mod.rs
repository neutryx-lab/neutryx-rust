//! REST API routes (Axum)

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

mod graph_handlers;
mod handlers;
mod ws_handlers;

pub use graph_handlers::GraphAppState;
pub use ws_handlers::WsAppState;

/// Create the REST API router
pub fn create_router() -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // API v1 routes
        .nest("/api/v1", api_v1_routes())
}

/// Create the REST API router with graph state
pub fn create_router_with_graph_state(graph_state: Arc<GraphAppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // API v1 routes
        .nest("/api/v1", api_v1_routes())
        // Portfolio graph routes with state
        .nest("/api/v1/portfolio", portfolio_routes(graph_state))
}

/// Create the REST API router with both graph state and WebSocket support
pub fn create_router_with_ws_state(ws_state: Arc<WsAppState>) -> Router {
    let graph_state = ws_state.graph_state.clone();
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // API v1 routes
        .nest("/api/v1", api_v1_routes())
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

fn api_v1_routes() -> Router {
    Router::new()
        .route("/price", post(handlers::price_instrument))
        .route("/price/batch", post(handlers::price_portfolio))
        .route("/calibrate", post(handlers::calibrate))
        .route("/exposure", post(handlers::calculate_exposure))
}

fn portfolio_routes(state: Arc<GraphAppState>) -> Router {
    Router::new()
        .route("/graph", get(graph_handlers::get_portfolio_graph))
        .route("/trades", get(graph_handlers::get_portfolio_trades))
        .with_state(state)
}
