//! REST API routes (Axum).

use std::sync::Arc;

#[cfg(feature = "risk")]
use axum::routing::{delete, put};
use axum::{
    routing::{get, post},
    Router,
};

pub mod dto;
mod graph_handlers;
mod handlers;
#[cfg(feature = "openapi")]
pub mod openapi;
mod ws_handlers;

pub use graph_handlers::GraphAppState;
pub use ws_handlers::WsAppState;

use crate::state::AppState;

/// Create the REST API router with full state.
pub fn create_router_with_state(state: Arc<AppState>) -> Router {
    let router = Router::new()
        .route("/health", get(handlers::health))
        .nest("/api", api_routes(state.clone()))
        .nest("/api/v1", api_v1_routes(state));

    #[cfg(feature = "openapi")]
    let router = mount_swagger_ui(router);

    router
}

/// Create the full router with all features (used when demo feature is.
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

/// Create demo router with demo_gui integration (feature = "demo").
#[cfg(feature = "demo")]
pub fn create_demo_router(state: Arc<AppState>) -> Router {
    use tower_http::services::{ServeDir, ServeFile};

    let graph_state =
        Arc::new(GraphAppState::default_sample().expect("Failed to create graph state"));

    let router = Router::new()
        .route("/health", get(handlers::health))
        .nest("/api", demo_api_routes(state.clone()))
        .nest("/api/v1", api_v1_routes(state))
        .nest("/api/portfolio", demo_portfolio_routes(graph_state.clone()))
        .nest("/api", graph_alias_routes(graph_state));

    let serve_dir = ServeDir::new("demo/gui/dist")
        .not_found_service(ServeFile::new("demo/gui/dist/index.html"));

    let data_input_dir = ServeDir::new("demo/data/input");
    let data_config_dir = ServeDir::new("demo/data/config");

    let doc_dir =
        ServeDir::new("target/doc").not_found_service(ServeFile::new("target/doc/index.html"));

    #[cfg(feature = "openapi")]
    let router = mount_swagger_ui(router);

    router
        .nest_service("/doc", doc_dir)
        .nest_service("/assets", ServeDir::new("demo/gui/dist/assets"))
        .nest_service("/data/input", data_input_dir)
        .nest_service("/data/config", data_config_dir)
        .fallback_service(serve_dir)
}

/// Demo API routes (feature = "demo").
#[cfg(feature = "demo")]
fn demo_api_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/config", get(handlers::demo::get_config))
        .route("/instruments", get(handlers::demo::get_instruments))
        .route("/pricer/instruments", get(handlers::demo::get_instruments))
        .route("/trade/expand", post(handlers::demo::expand_trade))
        .route("/pricer/expand", post(handlers::demo::expand_trade))
        .route("/pricer/price", post(handlers::demo::price_trade))
        .route("/pricer/greeks", post(handlers::demo::calculate_greeks))
        .route("/pricer/graph", post(handlers::demo::get_pricer_graph))
        .route(
            "/pricer/advanced-greeks",
            post(handlers::demo::calculate_advanced_greeks),
        )
        .route("/utils/resolve-tenor", post(handlers::demo::resolve_tenor))
        .route("/curves", get(handlers::demo::get_available_curves))
        .route("/curves/indices", get(handlers::demo::get_curve_indices))
        .route(
            "/curves/instruments/:index",
            get(handlers::demo::get_curve_instruments),
        )
        .route("/curves/build", post(handlers::build_curve))
        .route(
            "/curves/discount-factor",
            post(handlers::get_discount_factor),
        )
        .route("/curves/forward-rate", post(handlers::get_forward_rate))
        .route(
            "/curves/forward-swap-rates",
            post(handlers::get_forward_swap_rates),
        )
        .route("/volcube/indices", get(handlers::demo::get_volcube_indices))
        .route("/volcube/models", get(handlers::demo::get_volcube_models))
        .route(
            "/volcube/instruments/:currency",
            get(handlers::demo::get_volcube_instruments),
        )
        .route(
            "/volcube/calibrate",
            post(handlers::demo::calibrate_volcube),
        )
        .route(
            "/volcube/implied-pdf",
            post(handlers::demo::compute_implied_pdf),
        )
        .route(
            "/volcube/sabr-smile",
            post(handlers::demo::compute_sabr_smile),
        )
        .route("/fxvol/calibrate", post(handlers::demo::calibrate_fxvol))
        .route("/market/rates", get(handlers::demo::get_market_rates))
        .route("/market/config", get(handlers::demo::get_market_config))
        .route(
            "/market/rates/refresh",
            post(handlers::demo::refresh_market_rates),
        )
        .route(
            "/market/rates/:rate_id/instrument",
            get(handlers::demo::get_rate_instrument),
        )
        .route(
            "/market/rates/:rate_id/cashflows",
            get(handlers::demo::get_rate_cashflows),
        )
        .route(
            "/market/rates/:rate_id",
            get(handlers::demo::get_rate_detail),
        )
        .route("/market/indices", get(handlers::demo::get_rate_indices))
        .route(
            "/market/indices/:code/rates",
            get(handlers::demo::get_index_rates),
        )
        .route(
            "/market/indices/:code/conventions",
            get(handlers::demo::get_index_conventions),
        )
        .route(
            "/market/indices/:code",
            get(handlers::demo::get_rate_index_detail),
        )
        .route("/market/conventions", get(handlers::demo::get_conventions))
        .route(
            "/market/conventions/:id",
            get(handlers::demo::get_convention_detail),
        )
        .route("/market/events", get(handlers::demo::get_events))
        .route("/market/events/types", get(handlers::demo::get_event_types))
        .route("/market/holidays", get(handlers::demo::get_holidays))
        .route("/market/bonds", get(handlers::demo::get_bond_quotes))
        .route("/market/credit", get(handlers::demo::get_credit_quotes))
        .route("/market/export/csv", get(handlers::demo::export_market_csv))
        .route(
            "/market/export/json",
            get(handlers::demo::export_market_json),
        )
        .route(
            "/irvol/currencies",
            get(handlers::demo::get_ir_vol_currencies),
        )
        .route(
            "/irvol/quotes/:currency",
            get(handlers::demo::get_ir_vol_quotes),
        )
        .route("/fxvol/pairs", get(handlers::demo::get_fx_vol_pairs))
        .route(
            "/fxvol/quotes/:pair",
            get(handlers::demo::get_fx_vol_quotes),
        )
        .route(
            "/pricer/exotic-products",
            get(handlers::demo::get_exotic_products),
        )
        .route("/pricer/price-exotic", post(handlers::demo::price_exotic))
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
        .route("/config", get(handlers::get_config))
        .route("/price", post(handlers::price_instrument))
        .route("/price/batch", post(handlers::price_portfolio))
        .route("/curves/build", post(handlers::build_curve))
        .route(
            "/curves/discount-factor",
            post(handlers::get_discount_factor),
        )
        .route("/curves/forward-rate", post(handlers::get_forward_rate))
        .route(
            "/curves/forward-swap-rates",
            post(handlers::get_forward_swap_rates),
        )
        .with_state(state)
}

/// API v1 routes with feature-gated services.
#[allow(unused_mut)]
fn api_v1_routes(state: Arc<AppState>) -> Router {
    let mut router = Router::new()
        .route("/config", get(handlers::get_config))
        .route("/price", post(handlers::price_instrument))
        .route("/price/batch", post(handlers::price_portfolio))
        .route("/curves/build", post(handlers::build_curve))
        .route(
            "/curves/discount-factor",
            post(handlers::get_discount_factor),
        )
        .route("/curves/forward-rate", post(handlers::get_forward_rate))
        .route(
            "/curves/forward-swap-rates",
            post(handlers::get_forward_swap_rates),
        );

    #[cfg(feature = "risk")]
    {
        router = router
            .route("/risk/greeks", post(handlers::compute_greeks))
            .route("/risk/scenarios", post(handlers::run_scenarios))
            .route("/portfolios", post(handlers::create_portfolio))
            .route("/portfolios/:id", get(handlers::get_portfolio))
            .route("/portfolios/:id", delete(handlers::delete_portfolio))
            .route("/portfolios/:id/trades", put(handlers::add_trades))
            .route("/portfolios/:id/price", post(handlers::price_portfolio_id))
            .route(
                "/portfolios/:id/greeks",
                post(handlers::compute_portfolio_greeks),
            );
    }

    #[cfg(feature = "models")]
    {
        router = router
            .route("/models", post(handlers::create_model))
            .route("/models/:id", get(handlers::get_model))
            .route("/models/:id/price", post(handlers::price_with_model));
    }

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

/// Portfolio routes for demo mode (feature = "demo").
#[cfg(feature = "demo")]
fn demo_portfolio_routes(state: Arc<GraphAppState>) -> Router {
    Router::new()
        .route("/graph", get(graph_handlers::get_portfolio_graph))
        .route("/trades", get(graph_handlers::get_portfolio_trades))
        .with_state(state)
}

/// Alias routes for frontend compatibility (feature = "demo").
#[cfg(feature = "demo")]
fn graph_alias_routes(state: Arc<GraphAppState>) -> Router {
    Router::new()
        .route("/graph", get(graph_handlers::get_portfolio_graph))
        .with_state(state)
}

/// Serve the OpenAPI JSON spec at `/api-docs/openapi.json`.
#[cfg(feature = "openapi")]
fn mount_swagger_ui(router: Router) -> Router {
    use axum::Json;
    use utoipa::OpenApi;

    router.route(
        "/api-docs/openapi.json",
        get(|| async { Json(openapi::ApiDoc::openapi()) }),
    )
}
