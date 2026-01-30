//! REST API routes (Axum)

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
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
        .nest("/api", api_routes(state.clone()))
        .nest("/api/v1", api_v1_routes(state))
}

/// Create the full router with all features
pub fn create_full_router(app_state: Arc<AppState>, ws_state: Arc<WsAppState>) -> Router {
    let graph_state = ws_state.graph_state.clone();
    Router::new()
        .route("/health", get(handlers::health))
        .nest("/api", api_routes(app_state.clone()))
        .nest("/api/v1", api_v1_routes(app_state))
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

/// API v1 routes with feature-gated services
fn api_v1_routes(state: Arc<AppState>) -> Router {
    let mut router = Router::new()
        // Pricing endpoints (always available)
        .route("/price", post(handlers::price_instrument))
        .route("/price/batch", post(handlers::price_portfolio))
        // Curve building endpoints (always available)
        .route("/curves/build", post(handlers::build_curve))
        .route("/curves/discount-factor", post(handlers::get_discount_factor))
        .route("/curves/forward-rate", post(handlers::get_forward_rate));

    // Risk endpoints (feature = "risk")
    #[cfg(feature = "risk")]
    {
        router = router
            // Risk calculation endpoints
            .route("/risk/greeks", post(handlers::compute_greeks))
            .route("/risk/scenarios", post(handlers::run_scenarios))
            // Portfolio CRUD endpoints
            .route("/portfolios", post(handlers::create_portfolio))
            .route("/portfolios/:id", get(handlers::get_portfolio))
            .route("/portfolios/:id", delete(handlers::delete_portfolio))
            .route("/portfolios/:id/trades", put(handlers::add_trades))
            .route("/portfolios/:id/price", post(handlers::price_portfolio_id))
            .route("/portfolios/:id/greeks", post(handlers::compute_portfolio_greeks));
    }

    // Model endpoints (feature = "models")
    #[cfg(feature = "models")]
    {
        router = router
            .route("/models", post(handlers::create_model))
            .route("/models/:id", get(handlers::get_model))
            .route("/models/:id/price", post(handlers::price_with_model));
    }

    // Volatility endpoints (feature = "volatility")
    #[cfg(feature = "volatility")]
    {
        router = router
            .route("/volatility/fx-surface", post(handlers::build_fx_vol_surface))
            .route("/volatility/cube", post(handlers::build_vol_cube))
            .route("/volatility/:id/implied-vol", post(handlers::get_implied_vol));
    }

    router.with_state(state)
}

fn portfolio_routes(state: Arc<GraphAppState>) -> Router {
    Router::new()
        .route("/graph", get(graph_handlers::get_portfolio_graph))
        .route("/trades", get(graph_handlers::get_portfolio_trades))
        .with_state(state)
}
