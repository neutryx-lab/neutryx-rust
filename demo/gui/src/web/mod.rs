//! Web dashboard module for FrictionalBank demo.
//!
//! Provides a browser-based dashboard with:
//! - REST API for portfolio and risk data
//! - WebSocket for real-time updates
//! - Static file serving for HTML/JS/CSS
//!
//! ## Graph Visualisation Support
//!
//! - REST API: `GET /api/graph` for computation graph data
//! - WebSocket: `graph_update` messages for real-time node updates (Task 4.1)
//! - Subscription: Clients can subscribe to specific trade graph updates (Task
//!   4.3)
//!
//! ## Curve Builder Support (curve-builder-webapp)
//!
//! - REST API: `/api/curves/*` endpoints for curve construction
//! - Index-based instrument management
//! - Builder model selection (interpolation, bootstrap method)

pub mod curve_builder_handlers;
pub mod curve_builder_types;
pub mod error;
pub mod fxcurve_handlers;
pub mod fxcurve_types;
pub mod fxvol_handlers;
pub mod fxvol_types;
pub mod generic_pricer_handlers;
pub mod irvol_handlers;
pub mod irvol_types;
pub mod jobs;
pub mod market_data;
pub mod market_handlers;
pub mod market_types;
pub mod metrics;
pub mod openapi;
pub mod pricer_types;
pub mod pricing_service;
pub mod risk_engine_handlers;
pub mod risk_engine_types;
pub mod scenario_handlers;
pub mod schedule_utils;
pub mod state;
pub mod trade_handlers;
pub mod trade_types;
pub mod volcube_handlers;
pub mod volcube_types;
pub mod websocket;

// Legacy handlers module (being gradually migrated)
#[path = "handlers.rs"]
pub mod handlers;

// New modular handlers (migration in progress)
pub mod handlers_v2;

use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{atomic::AtomicU32, Arc},
    time::Instant,
};

use axum::{
    http::HeaderValue,
    routing::{get, post},
    Router,
};
use handlers::{GraphCache, PortfolioGraphCache};
use jobs::JobManager;
use market_data::MarketDataCache;
use pricer_types::BootstrapCurveCache;
use tokio::sync::{broadcast, RwLock};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    services::ServeDir,
    set_header::SetResponseHeaderLayer,
};
use tracing::info;

// =========================================================================
// Task 6.1: PerformanceMetrics State (Requirement 9.5)
// =========================================================================

/// Performance metrics for API response times and WebSocket connections.
///
/// Tracks response times for each API endpoint and WebSocket statistics.
/// Uses RwLock for thread-safe access and AtomicU32 for connection count.
pub struct PerformanceMetrics {
    /// Portfolio API response times in microseconds (limited to 1000 entries)
    pub portfolio_times: RwLock<Vec<u64>>,
    /// Exposure API response times in microseconds
    pub exposure_times: RwLock<Vec<u64>>,
    /// Risk API response times in microseconds
    pub risk_times: RwLock<Vec<u64>>,
    /// Graph API response times in microseconds
    pub graph_times: RwLock<Vec<u64>>,
    /// Number of active WebSocket connections
    pub ws_connections: AtomicU32,
    /// WebSocket message latencies in microseconds
    pub ws_message_latencies: RwLock<Vec<u64>>,
    /// Server start time for uptime calculation
    pub start_time: Instant,
}

impl PerformanceMetrics {
    /// Maximum number of timing entries to keep (Requirement 9.5: limit to
    /// 1000)
    const MAX_ENTRIES: usize = 1000;

    /// Create new performance metrics instance
    pub fn new() -> Self {
        Self {
            portfolio_times: RwLock::new(Vec::with_capacity(Self::MAX_ENTRIES)),
            exposure_times: RwLock::new(Vec::with_capacity(Self::MAX_ENTRIES)),
            risk_times: RwLock::new(Vec::with_capacity(Self::MAX_ENTRIES)),
            graph_times: RwLock::new(Vec::with_capacity(Self::MAX_ENTRIES)),
            ws_connections: AtomicU32::new(0),
            ws_message_latencies: RwLock::new(Vec::with_capacity(Self::MAX_ENTRIES)),
            start_time: Instant::now(),
        }
    }

    /// Record a timing value, maintaining the size limit
    async fn record_time(times: &RwLock<Vec<u64>>, duration_us: u64) {
        let mut times = times.write().await;
        if times.len() >= Self::MAX_ENTRIES {
            times.remove(0);
        }
        times.push(duration_us);
    }

    /// Record portfolio API response time
    pub async fn record_portfolio_time(&self, duration_us: u64) {
        Self::record_time(&self.portfolio_times, duration_us).await;
    }

    /// Record exposure API response time
    pub async fn record_exposure_time(&self, duration_us: u64) {
        Self::record_time(&self.exposure_times, duration_us).await;
    }

    /// Record risk API response time
    pub async fn record_risk_time(&self, duration_us: u64) {
        Self::record_time(&self.risk_times, duration_us).await;
    }

    /// Record graph API response time
    pub async fn record_graph_time(&self, duration_us: u64) {
        Self::record_time(&self.graph_times, duration_us).await;
    }

    /// Record WebSocket message latency
    pub async fn record_ws_latency(&self, latency_us: u64) {
        Self::record_time(&self.ws_message_latencies, latency_us).await;
    }

    /// Calculate average from a timing vector
    async fn calculate_avg(times: &RwLock<Vec<u64>>) -> f64 {
        let times = times.read().await;
        if times.is_empty() {
            return 0.0;
        }
        let sum: u64 = times.iter().sum();
        (sum as f64) / (times.len() as f64) / 1000.0 // Convert to milliseconds
    }

    /// Get average portfolio response time in milliseconds
    pub async fn portfolio_avg_ms(&self) -> f64 { Self::calculate_avg(&self.portfolio_times).await }

    /// Get average exposure response time in milliseconds
    pub async fn exposure_avg_ms(&self) -> f64 { Self::calculate_avg(&self.exposure_times).await }

    /// Get average risk response time in milliseconds
    pub async fn risk_avg_ms(&self) -> f64 { Self::calculate_avg(&self.risk_times).await }

    /// Get average graph response time in milliseconds
    pub async fn graph_avg_ms(&self) -> f64 { Self::calculate_avg(&self.graph_times).await }

    /// Get average WebSocket message latency in milliseconds
    pub async fn ws_latency_avg_ms(&self) -> f64 {
        Self::calculate_avg(&self.ws_message_latencies).await
    }

    /// Get server uptime in seconds
    pub fn uptime_seconds(&self) -> u64 { self.start_time.elapsed().as_secs() }

    /// Get current WebSocket connection count
    pub fn ws_connection_count(&self) -> u32 {
        self.ws_connections
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Increment WebSocket connection count
    pub fn increment_ws_connections(&self) {
        self.ws_connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Decrement WebSocket connection count
    pub fn decrement_ws_connections(&self) {
        self.ws_connections
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self { Self::new() }
}

// =========================================================================
// Task 1.1: Debug Configuration
// =========================================================================

/// Debug configuration read from environment variables
pub struct DebugConfig {
    /// Whether debug mode is enabled (FB_DEBUG_MODE)
    pub debug_mode: bool,
    /// Log level (FB_LOG_LEVEL)
    pub log_level: String,
}

impl DebugConfig {
    /// Read configuration from environment variables
    pub fn from_env() -> Self {
        let debug_mode = std::env::var("FB_DEBUG_MODE")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);

        let log_level = std::env::var("FB_LOG_LEVEL")
            .ok()
            .map(|v| v.to_uppercase())
            .filter(|v| ["DEBUG", "INFO", "WARN", "ERROR"].contains(&v.as_str()))
            .unwrap_or_else(|| "INFO".to_string());

        Self {
            debug_mode,
            log_level,
        }
    }
}

/// Application state shared across handlers
pub struct AppState {
    /// Broadcast channel for real-time updates
    pub tx: broadcast::Sender<String>,
    /// Graph cache for performance optimisation (Task 3.3)
    pub graph_cache: RwLock<GraphCache>,
    /// Portfolio graph cache for performance optimisation (Task 4.4)
    pub portfolio_graph_cache: RwLock<PortfolioGraphCache>,
    /// Set of trade IDs that clients have subscribed to for graph updates (Task
    /// 4.3)
    pub graph_subscriptions: RwLock<HashSet<String>>,
    /// Performance metrics (Task 6.1)
    pub metrics: PerformanceMetrics,
    /// Debug configuration (Task 1.1)
    pub debug_config: DebugConfig,
    /// Bootstrapped curve cache for IRS pricing (Task 1.5)
    pub curve_cache: BootstrapCurveCache,
    /// Async job manager (Task 7.1)
    pub job_manager: JobManager,
    /// Market data cache (market-data-viewer-webapp Task 3.1)
    pub market_data_cache: Arc<MarketDataCache>,
    /// VolCube cache for calibrated volatility cubes (volcube-calibration-ui)
    pub volcube_cache: volcube_handlers::VolCubeCache,
    /// FxVol cache for built FX volatility surfaces (volcube-calibration-ui)
    pub fxvol_cache: fxvol_handlers::FxVolCache,
    /// IrVol cache for built IR volatility surfaces (market-data-viewer-webapp)
    pub irvol_cache: irvol_handlers::IrVolState,
}

impl AppState {
    /// Create new application state
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            tx,
            graph_cache: RwLock::new(GraphCache::new()),
            portfolio_graph_cache: RwLock::new(PortfolioGraphCache::new()),
            graph_subscriptions: RwLock::new(HashSet::new()),
            metrics: PerformanceMetrics::new(),
            debug_config: DebugConfig::from_env(),
            curve_cache: BootstrapCurveCache::new(),
            job_manager: JobManager::new(),
            market_data_cache: Arc::new(MarketDataCache::new()),
            volcube_cache: volcube_handlers::VolCubeCache::new(10),
            fxvol_cache: fxvol_handlers::FxVolCache::new(10),
            irvol_cache: irvol_handlers::create_irvol_state(),
        }
    }

    // =========================================================================
    // Task 4.3: Subscription Management Methods
    // =========================================================================

    /// Subscribe to graph updates for a specific trade.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - The trade ID to subscribe to
    pub async fn subscribe_graph(&self, trade_id: &str) {
        let mut subscriptions = self.graph_subscriptions.write().await;
        subscriptions.insert(trade_id.to_string());
    }

    /// Unsubscribe from graph updates for a specific trade.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - The trade ID to unsubscribe from
    pub async fn unsubscribe_graph(&self, trade_id: &str) {
        let mut subscriptions = self.graph_subscriptions.write().await;
        subscriptions.remove(trade_id);
    }

    /// Check if a trade is currently subscribed for graph updates.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - The trade ID to check
    ///
    /// # Returns
    ///
    /// `true` if the trade is subscribed, `false` otherwise
    pub async fn is_graph_subscribed(&self, trade_id: &str) -> bool {
        let subscriptions = self.graph_subscriptions.read().await;
        subscriptions.contains(trade_id)
    }

    /// Get all currently subscribed trade IDs.
    ///
    /// # Returns
    ///
    /// A vector of all subscribed trade IDs
    pub async fn get_graph_subscriptions(&self) -> Vec<String> {
        let subscriptions = self.graph_subscriptions.read().await;
        subscriptions.iter().cloned().collect()
    }

    /// Clear all graph subscriptions.
    pub async fn clear_graph_subscriptions(&self) {
        let mut subscriptions = self.graph_subscriptions.write().await;
        subscriptions.clear();
    }
}

impl Default for AppState {
    fn default() -> Self { Self::new() }
}

/// Build the web application router with Cloud Run compatible CORS.
///
/// CORS configuration:
/// - If `FB_CORS_ORIGINS` is set: use the comma-separated list of origins
/// - If `FB_CORS_ALLOW_ANY` is "true": allow any origin (useful for Cloud Run)
/// - Otherwise: default to localhost:3000 for development
///
/// Note: For same-origin requests (frontend served from same server), CORS
/// doesn't apply. This mainly affects development scenarios with separate
/// frontend/backend servers.
fn build_cors() -> CorsLayer {
    // Check for explicit "allow any" mode (useful for Cloud Run public services)
    let allow_any = std::env::var("FB_CORS_ALLOW_ANY")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);

    if allow_any {
        info!("CORS: Allowing any origin (FB_CORS_ALLOW_ANY=true)");
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
    }

    // Check for explicit origins list
    let origins = std::env::var("FB_CORS_ORIGINS").ok().and_then(|value| {
        let origins: Vec<HeaderValue> = value
            .split(',')
            .map(|origin| origin.trim())
            .filter(|origin| !origin.is_empty())
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect();
        if origins.is_empty() {
            None
        } else {
            info!("CORS: Using explicit origins from FB_CORS_ORIGINS");
            Some(origins)
        }
    });

    if let Some(origins) = origins {
        return CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(Any)
            .allow_headers(Any);
    }

    // Detect Cloud Run environment and be more permissive
    let is_cloud_run = std::env::var("K_SERVICE").is_ok() || std::env::var("CLOUD_RUN_JOB").is_ok();
    if is_cloud_run {
        info!("CORS: Detected Cloud Run environment, allowing any origin");
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
    }

    // Default: development mode with localhost origins
    info!("CORS: Using default localhost origins for development");
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(vec![
            HeaderValue::from_static("http://127.0.0.1:3000"),
            HeaderValue::from_static("http://localhost:3000"),
            HeaderValue::from_static("http://127.0.0.1:8080"),
            HeaderValue::from_static("http://localhost:8080"),
        ]))
        .allow_methods(Any)
        .allow_headers(Any)
}

fn build_csp_header() -> SetResponseHeaderLayer<HeaderValue> {
    // CSP policy:
    // - script-src + script-src-elem: 'self', cdn.plot.ly, 'unsafe-eval' for Plotly
    // - style-src: 'unsafe-inline' for inline styles, external fonts
    // - connect-src: ws/wss for WebSocket connections
    // - worker-src: blob for Plotly web workers
    const DEFAULT_CSP: &str = "default-src 'self'; \
        script-src 'self' 'unsafe-eval' https://cdn.plot.ly; \
        script-src-elem 'self' https://cdn.plot.ly; \
        style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdnjs.cloudflare.com; \
        font-src 'self' https://fonts.gstatic.com https://cdnjs.cloudflare.com data:; \
        img-src 'self' data: blob:; \
        connect-src 'self' ws: wss:; \
        worker-src 'self' blob:;";

    let csp_value = std::env::var("FB_CSP").unwrap_or_else(|_| DEFAULT_CSP.to_string());
    let header_value =
        HeaderValue::from_str(&csp_value).unwrap_or_else(|_| HeaderValue::from_static(DEFAULT_CSP));

    SetResponseHeaderLayer::overriding(axum::http::header::CONTENT_SECURITY_POLICY, header_value)
}

fn build_cache_control_header() -> SetResponseHeaderLayer<HeaderValue> {
    // Disable caching for development - forces browser to always fetch fresh files
    // For production, consider using: "public, max-age=3600" with versioned file
    // names
    let cache_control = std::env::var("FB_CACHE_CONTROL")
        .unwrap_or_else(|_| "no-cache, no-store, must-revalidate".to_string());
    let header_value = HeaderValue::from_str(&cache_control)
        .unwrap_or_else(|_| HeaderValue::from_static("no-cache, no-store, must-revalidate"));

    SetResponseHeaderLayer::overriding(axum::http::header::CACHE_CONTROL, header_value)
}

/// Builds the main router with all API routes and middleware.
pub fn build_router(state: Arc<AppState>) -> Router {
    // CORS configuration for development
    let cors = build_cors();

    // API routes
    let api_routes = Router::new()
        .route("/health", get(handlers::health))
        .route("/portfolio", get(handlers::get_portfolio))
        .route("/portfolio", post(handlers::price_portfolio))
        .route("/exposure", get(handlers::get_exposure))
        .route("/risk", get(handlers::get_risk_metrics))
        // Task 3.2: Add /api/graph route for computation graph visualisation
        .route("/graph", get(handlers::get_graph))
        // Task 7.2: Add /api/benchmark/speed-comparison route for speed comparison chart
        .route(
            "/benchmark/speed-comparison",
            get(handlers::get_speed_comparison),
        )
        // Task 6.3: Add /api/metrics endpoint for performance statistics
        .route("/metrics", get(handlers::get_metrics))
        // Task 2.2: Add /api/price endpoint for instrument pricing
        .route("/price", post(handlers::price_instrument))
        // Task 2.1: Add /api/bootstrap endpoint for yield curve construction
        .route("/bootstrap", post(handlers::bootstrap_curve))
        // Task 3.1: Add /api/price-irs endpoint for IRS pricing
        .route("/price-irs", post(handlers::price_irs))
        // Task 4.1: Add /api/risk/bump endpoint for Bump-and-Revalue Delta calculation
        .route("/risk/bump", post(handlers::risk_bump))
        // Task 5.1: Add /api/risk/aad endpoint for AAD Delta calculation
        .route("/risk/aad", post(handlers::risk_aad))
        // Task 6.1: Add /api/risk/compare endpoint for comparison
        .route("/risk/compare", post(handlers::risk_compare))
        // Task 4.1: Add /api/greeks/compare endpoint for Greeks comparison (Bump vs AAD)
        .route("/greeks/compare", post(handlers::greeks_compare))
        // Task 4.2: Add /api/greeks/first-order endpoint for first-order Greeks (Delta, Vega, Rho, Theta)
        .route("/greeks/first-order", post(handlers::greeks_first_order))
        // Task 4.2: Add /api/greeks/second-order endpoint for second-order Greeks (Gamma, Vanna, Volga)
        .route("/greeks/second-order", post(handlers::greeks_second_order))
        // Task 4.3: Add /api/greeks/bucket-dv01 endpoint for tenor-specific DV01 sensitivities
        .route("/greeks/bucket-dv01", post(handlers::greeks_bucket_dv01))
        // Task 5.1: Add /api/greeks/heatmap endpoint for Greeks heatmap visualisation
        .route("/greeks/heatmap", get(handlers::get_greeks_heatmap))
        // Task 5.2: Add /api/greeks/timeseries endpoint for Greeks time decay visualisation
        .route("/greeks/timeseries", get(handlers::get_greeks_timeseries))
        // Task 6.1: Add /api/scenarios/presets endpoint for preset scenario list
        .route(
            "/scenarios/presets",
            get(scenario_handlers::get_scenario_presets),
        )
        // Task 6.2: Add /api/scenarios/run endpoint for scenario execution
        .route("/scenarios/run", post(scenario_handlers::run_scenario))
        // Task 6.4: Add /api/scenarios/compare endpoint for scenario comparison
        .route(
            "/scenarios/compare",
            post(scenario_handlers::compare_scenarios),
        )
        // Task 7.2: Add /api/v1/jobs endpoints for async job management
        .route("/v1/jobs", get(handlers::list_jobs))
        .route("/v1/jobs/:id", get(handlers::get_job_status))
        // Scenario analysis endpoint
        .route("/scenario", post(handlers::run_scenario))
        .route("/ws", get(websocket::ws_handler))
        // Task 4.1: Portfolio Graph API (portfolio-graph-optimisation)
        .route("/v1/portfolio/graph", get(handlers::get_portfolio_graph))
        // Task 4.3: Portfolio Trades List API (portfolio-graph-optimisation)
        .route("/v1/portfolio/trades", get(handlers::get_portfolio_trades))
        // Trade expansion API (pricer-trade-expansion-ui)
        .route("/trade/expand", post(trade_handlers::expand_trade))
        .route("/instruments", get(trade_handlers::get_instruments));

    // Market Data API routes (market-data-viewer-webapp Task 3.5)
    let market_routes = Router::new()
        .route("/rates", get(market_handlers::get_market_rates))
        .route(
            "/rates/refresh",
            post(market_handlers::refresh_market_rates),
        )
        .route("/rates/:id", get(market_handlers::get_market_rate_detail))
        .route("/conventions", get(market_handlers::get_market_conventions))
        .route(
            "/conventions/:id",
            get(market_handlers::get_market_convention_detail),
        )
        .route("/export/csv", get(market_handlers::export_rates_csv))
        .route("/export/json", get(market_handlers::export_rates_json))
        .with_state(state.market_data_cache.clone());

    let api_routes = api_routes.nest("/market", market_routes);

    // Curve Builder API routes (curve-builder-webapp)
    let curve_routes = Router::new()
        .route(
            "/instruments/:index",
            get(curve_builder_handlers::get_instruments),
        )
        .route("/builders", get(curve_builder_handlers::get_builders))
        .route("/build", post(curve_builder_handlers::build_curve))
        .route(
            "/:curve_id/parameters",
            get(curve_builder_handlers::get_parameters),
        )
        .route("/indices", get(curve_builder_handlers::get_indices));

    let api_routes = api_routes.nest("/curves", curve_routes);

    // GenericPricer API routes (demo-webapp-pricer Task 2.4)
    // Available in both standalone and l1l2-integration modes
    let pricer_routes = Router::new()
        .route("/price", post(generic_pricer_handlers::price_generic))
        .route("/greeks", post(generic_pricer_handlers::calculate_greeks))
        .route(
            "/instruments",
            get(generic_pricer_handlers::get_pricer_instruments),
        );
    let api_routes = api_routes.nest("/pricer", pricer_routes);

    // VolCube API routes (volcube-calibration-ui Task 7.1)
    let volcube_routes = Router::new()
        .route("/indices", get(volcube_handlers::get_indices))
        .route("/models", get(volcube_handlers::get_models))
        .route(
            "/instruments/:index",
            get(volcube_handlers::get_instruments),
        )
        .route(
            "/instruments/:index",
            axum::routing::put(volcube_handlers::update_instruments),
        )
        .route("/calibrate", post(volcube_handlers::calibrate))
        .route("/smile", get(volcube_handlers::get_smile))
        .route("/density", get(volcube_handlers::get_density))
        .route("/surface", get(volcube_handlers::get_surface));

    let api_routes = api_routes.nest("/volcube", volcube_routes);

    // FxVol API routes (volcube-calibration-ui Task 7.1, fx-vol-surface-calibration
    // Task 13.2)
    let fxvol_routes = Router::new()
        .route("/pairs", get(fxvol_handlers::get_pairs))
        .route("/delta-types", get(fxvol_handlers::get_delta_types))
        .route("/quotes/:pair", get(fxvol_handlers::get_quotes))
        .route(
            "/quotes/:pair",
            axum::routing::put(fxvol_handlers::update_quotes),
        )
        .route("/build", post(fxvol_handlers::build_surface))
        .route("/calibrate", post(fxvol_handlers::calibrate_surface))
        .route("/smile", get(fxvol_handlers::get_smile))
        .route("/surface", get(fxvol_handlers::get_surface))
        .route("/rr-bf", get(fxvol_handlers::get_rr_bf))
        .route("/density", get(fxvol_handlers::get_density))
        .route(
            "/delta-strike",
            post(fxvol_handlers::delta_to_strike_handler),
        );

    let api_routes = api_routes.nest("/fxvol", fxvol_routes);

    // IrVol API routes (market-data-viewer-webapp)
    let irvol_routes = Router::new()
        .route("/currencies", get(irvol_handlers::get_currencies))
        .route("/quotes/:currency", get(irvol_handlers::get_quotes))
        .route(
            "/quotes/:currency",
            axum::routing::put(irvol_handlers::update_quotes),
        )
        .route("/build", post(irvol_handlers::build_surface))
        .route("/smile", get(irvol_handlers::get_smile))
        .route("/atm-term", get(irvol_handlers::get_atm_term))
        .route("/surface", get(irvol_handlers::get_surface));

    let api_routes = api_routes.nest("/irvol", irvol_routes);

    // FxCurve API routes (fx-vol-surface-calibration Task 13.1)
    let fxcurve_routes = Router::new()
        .route("/build", post(fxcurve_handlers::build_fx_curve))
        .route("/market", post(fxcurve_handlers::build_fx_market))
        .route("/forward", get(fxcurve_handlers::get_forward_rate));
    let api_routes = api_routes.nest("/fxcurve", fxcurve_routes);

    // Risk Engine API routes (generic-pricing-risk-engine Task 8.3)
    let risk_engine_routes = Router::new()
        .route("/greeks", post(risk_engine_handlers::compute_greeks))
        .route(
            "/portfolio-greeks",
            post(risk_engine_handlers::compute_portfolio_greeks),
        )
        .route(
            "/scenario-greeks",
            post(risk_engine_handlers::compute_scenario_greeks),
        );
    let api_routes = api_routes.nest("/risk-engine", risk_engine_routes);

    // Static file serving for the dashboard
    let static_files =
        ServeDir::new("demo/gui/static").not_found_service(handlers::serve_index_with_config());

    // Data file serving for external JSON data
    let data_files = ServeDir::new("demo/data/input");

    // CSP header: default policy for local static assets.
    // - Script sources limited to self (vendor assets).
    // - 'unsafe-inline' required for inline style attributes in the demo.
    // - Override via FB_CSP for stricter policies.
    let csp_header = build_csp_header();

    // Cache-Control header: disable caching for development
    // Override via FB_CACHE_CONTROL for production caching
    let cache_control_header = build_cache_control_header();

    // Task 8.1: Build router with optional OpenAPI/Swagger UI support
    let router = Router::new()
        // Task 13.2: Serve index.html with config injection at root
        .route("/", get(handlers::get_index))
        .nest("/api", api_routes)
        .nest_service("/data/input", data_files)
        .fallback_service(static_files)
        .layer(cache_control_header)
        .layer(csp_header)
        .layer(cors)
        .with_state(state);

    // Merge Swagger UI router if openapi feature is enabled
    #[cfg(feature = "openapi")]
    let router = router.merge(openapi::swagger_ui_router());

    router
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

    // =========================================================================
    // Task 4.3: AppState Subscription Tests
    // =========================================================================

    mod app_state_subscription_tests {
        use super::*;

        #[tokio::test]
        async fn test_app_state_has_graph_subscriptions() {
            let state = AppState::new();
            // Verify graph_subscriptions field exists and is empty initially
            let subscriptions = state.graph_subscriptions.read().await;
            assert!(subscriptions.is_empty());
        }

        #[tokio::test]
        async fn test_subscribe_graph_adds_trade_id() {
            let state = AppState::new();
            state.subscribe_graph("T001").await;

            let subscriptions = state.graph_subscriptions.read().await;
            assert!(subscriptions.contains("T001"));
        }

        #[tokio::test]
        async fn test_unsubscribe_graph_removes_trade_id() {
            let state = AppState::new();
            state.subscribe_graph("T001").await;
            state.unsubscribe_graph("T001").await;

            let subscriptions = state.graph_subscriptions.read().await;
            assert!(!subscriptions.contains("T001"));
        }

        #[tokio::test]
        async fn test_is_graph_subscribed() {
            let state = AppState::new();
            state.subscribe_graph("T001").await;

            assert!(state.is_graph_subscribed("T001").await);
            assert!(!state.is_graph_subscribed("T002").await);
        }

        #[tokio::test]
        async fn test_get_graph_subscriptions() {
            let state = AppState::new();
            state.subscribe_graph("T001").await;
            state.subscribe_graph("T002").await;

            let subs = state.get_graph_subscriptions().await;
            assert_eq!(subs.len(), 2);
        }

        #[tokio::test]
        async fn test_clear_graph_subscriptions() {
            let state = AppState::new();
            state.subscribe_graph("T001").await;
            state.subscribe_graph("T002").await;
            state.clear_graph_subscriptions().await;

            let subscriptions = state.graph_subscriptions.read().await;
            assert!(subscriptions.is_empty());
        }

        #[tokio::test]
        async fn test_duplicate_subscription_is_idempotent() {
            let state = AppState::new();
            state.subscribe_graph("T001").await;
            state.subscribe_graph("T001").await;
            state.subscribe_graph("T001").await;

            let subscriptions = state.graph_subscriptions.read().await;
            assert_eq!(subscriptions.len(), 1);
        }

        #[tokio::test]
        async fn test_unsubscribe_nonexistent_is_safe() {
            let state = AppState::new();
            // Should not panic when unsubscribing from non-existent trade
            state.unsubscribe_graph("NONEXISTENT").await;

            let subscriptions = state.graph_subscriptions.read().await;
            assert!(subscriptions.is_empty());
        }
    }

    // =========================================================================
    // Task 1.5: AppState CurveCache Tests
    // =========================================================================

    mod app_state_curve_cache_tests {
        use super::*;

        #[test]
        fn test_app_state_has_curve_cache() {
            let state = AppState::new();
            // Verify curve_cache field exists and is empty initially
            assert!(state.curve_cache.is_empty());
        }

        #[test]
        fn test_curve_cache_is_accessible() {
            let state = AppState::new();
            // Should be able to access curve_cache methods
            assert_eq!(state.curve_cache.len(), 0);
        }
    }
}
