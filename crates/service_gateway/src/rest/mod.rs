//! REST API routes (Axum)

use std::sync::Arc;

use axum::{
    routing::get,
    routing::post,
    Router,
};
#[cfg(feature = "risk")]
use axum::routing::{delete, put};

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

/// Create the full router with all features (used when demo feature is disabled)
#[cfg(not(feature = "demo"))]
pub fn create_full_router(app_state: Arc<AppState>, ws_state: Arc<WsAppState>) -> Router {
    let graph_state = ws_state.graph_state.clone();
    Router::new()
        .route("/health", get(handlers::health))
        .nest("/api", api_routes(app_state.clone()))
        .nest("/api/v1", api_v1_routes(app_state))
        .nest("/api/portfolio", portfolio_routes(graph_state))
        .merge(ws_routes(ws_state))
}

/// Create demo router with demo_gui integration (feature = "demo")
///
/// This router includes:
/// - Demo API endpoints at /api/*
/// - API v1 endpoints at /api/v1/*
/// - Static file serving from demo/gui/dist/ (Vite build output)
/// - Data file serving from demo/data/input/
#[cfg(feature = "demo")]
pub fn create_demo_router(state: Arc<AppState>) -> Router {
    use tower_http::services::{ServeDir, ServeFile};

    let router = Router::new()
        .route("/health", get(handlers::health))
        .nest("/api", demo_api_routes(state.clone()))
        .nest("/api/v1", api_v1_routes(state));

    // Serve built assets from demo/gui/dist/ directory
    let serve_dir = ServeDir::new("demo/gui/dist")
        .not_found_service(ServeFile::new("demo/gui/dist/index.html"));

    // Serve data files for frontend to fetch
    let data_dir = ServeDir::new("demo/data/input");

    router
        .nest_service("/assets", ServeDir::new("demo/gui/dist/assets"))
        .nest_service("/data/input", data_dir)
        .fallback_service(serve_dir)
}

/// Demo API routes (feature = "demo")
#[cfg(feature = "demo")]
fn demo_api_routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Configuration
        .route("/config", get(handlers::demo::get_config))
        // Instruments
        .route("/instruments", get(handlers::demo::get_instruments))
        // Trade expansion
        .route("/trade/expand", post(handlers::demo::expand_trade))
        // Pricing
        .route("/pricer/price", post(handlers::demo::price_trade))
        .route("/pricer/greeks", post(handlers::demo::calculate_greeks))
        // Curves (demo endpoints + existing)
        .route("/curves", get(handlers::demo::get_available_curves))
        .route("/curves/indices", get(handlers::demo::get_curve_indices))
        .route("/curves/instruments/:index", get(handlers::demo::get_curve_instruments))
        .route("/curves/build", post(handlers::build_curve))
        .route("/curves/discount-factor", post(handlers::get_discount_factor))
        .route("/curves/forward-rate", post(handlers::get_forward_rate))
        // Volcube
        .route("/volcube/indices", get(handlers::demo::get_volcube_indices))
        .route("/volcube/models", get(handlers::demo::get_volcube_models))
        .route("/volcube/instruments/:currency", get(handlers::demo::get_volcube_instruments))
        // Market data
        .route("/market/rates", get(handlers::demo::get_market_rates))
        .route("/market/config", get(handlers::demo::get_market_config))
        .route("/market/rates/refresh", post(handlers::demo::refresh_market_rates))
        .route("/market/rates/:rate_id", get(handlers::demo::get_rate_detail))
        .route("/market/conventions", get(handlers::demo::get_conventions))
        .route("/market/conventions/:id", get(handlers::demo::get_convention_detail))
        .route("/market/events", get(handlers::demo::get_events))
        .route("/market/events/types", get(handlers::demo::get_event_types))
        .route("/market/export/csv", get(handlers::demo::export_market_csv))
        .route("/market/export/json", get(handlers::demo::export_market_json))
        // IR Volatility
        .route("/irvol/currencies", get(handlers::demo::get_ir_vol_currencies))
        .route("/irvol/quotes/:currency", get(handlers::demo::get_ir_vol_quotes))
        // FX Volatility
        .route("/fxvol/pairs", get(handlers::demo::get_fx_vol_pairs))
        .route("/fxvol/quotes/:pair", get(handlers::demo::get_fx_vol_quotes))
        // Pricing endpoints
        .route("/price", post(handlers::price_instrument))
        .route("/price/batch", post(handlers::price_portfolio))
        .with_state(state)
}

#[cfg(not(feature = "demo"))]
fn ws_routes(state: Arc<WsAppState>) -> Router {
    Router::new()
        .route("/ws", get(ws_handlers::ws_handler))
        .with_state(state)
}

fn api_routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Configuration endpoint (no state required)
        .route("/config", get(handlers::get_config))
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
#[allow(unused_mut)]
fn api_v1_routes(state: Arc<AppState>) -> Router {
    let mut router = Router::new()
        // Configuration endpoint (always available, no state required)
        .route("/config", get(handlers::get_config))
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
            .route(
                "/volatility/fx-surface",
                post(handlers::build_fx_vol_surface),
            )
            .route("/volatility/cube", post(handlers::build_vol_cube))
            .route(
                "/volatility/:id/implied-vol",
                post(handlers::get_implied_vol),
            );
    }

    router.with_state(state)
}

#[cfg(not(feature = "demo"))]
fn portfolio_routes(state: Arc<GraphAppState>) -> Router {
    Router::new()
        .route("/graph", get(graph_handlers::get_portfolio_graph))
        .route("/trades", get(graph_handlers::get_portfolio_trades))
        .with_state(state)
}
