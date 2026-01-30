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

// Re-export handlers for external API consumers
pub use graph_handlers::GraphAppState;
pub use ws_handlers::WsAppState;

use crate::state::AppState;

/// Create the REST API router with full state
pub fn create_router_with_state(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .nest("/api", api_routes(state))
}

/// Create the full router with all features
pub fn create_full_router(app_state: Arc<AppState>, ws_state: Arc<WsAppState>) -> Router {
    let graph_state = ws_state.graph_state.clone();
    Router::new()
        .route("/health", get(handlers::health))
        .nest("/api", api_routes(app_state))
        .nest("/api/portfolio", portfolio_routes(graph_state))
        .merge(ws_routes(ws_state))
}

fn ws_routes(state: Arc<WsAppState>) -> Router {
    Router::new()
        .route("/ws", get(ws_handlers::ws_handler))
        .with_state(state)
}

fn api_routes(state: Arc<AppState>) -> Router {
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
