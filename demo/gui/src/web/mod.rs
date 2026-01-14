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
//! - Subscription: Clients can subscribe to specific trade graph updates (Task 4.3)

pub mod handlers;
pub mod pricer_types;
pub mod websocket;

use axum::{
    http::HeaderValue,
    routing::{get, post},
    Router,
};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::info;

use handlers::GraphCache;

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
    /// Maximum number of timing entries to keep (Requirement 9.5: limit to 1000)
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
    pub async fn portfolio_avg_ms(&self) -> f64 {
        Self::calculate_avg(&self.portfolio_times).await
    }

    /// Get average exposure response time in milliseconds
    pub async fn exposure_avg_ms(&self) -> f64 {
        Self::calculate_avg(&self.exposure_times).await
    }

    /// Get average risk response time in milliseconds
    pub async fn risk_avg_ms(&self) -> f64 {
        Self::calculate_avg(&self.risk_times).await
    }

    /// Get average graph response time in milliseconds
    pub async fn graph_avg_ms(&self) -> f64 {
        Self::calculate_avg(&self.graph_times).await
    }

    /// Get average WebSocket message latency in milliseconds
    pub async fn ws_latency_avg_ms(&self) -> f64 {
        Self::calculate_avg(&self.ws_message_latencies).await
    }

    /// Get server uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Get current WebSocket connection count
    pub fn ws_connection_count(&self) -> u32 {
        self.ws_connections.load(std::sync::atomic::Ordering::Relaxed)
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
    fn default() -> Self {
        Self::new()
    }
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
    /// Set of trade IDs that clients have subscribed to for graph updates (Task 4.3)
    pub graph_subscriptions: RwLock<HashSet<String>>,
    /// Performance metrics (Task 6.1)
    pub metrics: PerformanceMetrics,
    /// Debug configuration (Task 1.1)
    pub debug_config: DebugConfig,
}

impl AppState {
    /// Create new application state
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            tx,
            graph_cache: RwLock::new(GraphCache::new()),
            graph_subscriptions: RwLock::new(HashSet::new()),
            metrics: PerformanceMetrics::new(),
            debug_config: DebugConfig::from_env(),
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
    fn default() -> Self {
        Self::new()
    }
}

/// Build the web application router
fn build_cors() -> CorsLayer {
    let origins = std::env::var("FB_CORS_ORIGINS")
        .ok()
        .and_then(|value| {
            let origins: Vec<HeaderValue> = value
                .split(',')
                .map(|origin| origin.trim())
                .filter(|origin| !origin.is_empty())
                .filter_map(|origin| HeaderValue::from_str(origin).ok())
                .collect();
            if origins.is_empty() {
                None
            } else {
                Some(origins)
            }
        })
        .unwrap_or_else(|| {
            vec![
                HeaderValue::from_static("http://127.0.0.1:3000"),
                HeaderValue::from_static("http://localhost:3000"),
            ]
        });

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(Any)
        .allow_headers(Any)
}

fn build_csp_header() -> SetResponseHeaderLayer<HeaderValue> {
    const DEFAULT_CSP: &str = "default-src 'self'; \
        script-src 'self'; \
        style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdnjs.cloudflare.com; \
        font-src 'self' https://fonts.gstatic.com https://cdnjs.cloudflare.com data:; \
        img-src 'self' data: blob:; \
        connect-src 'self' ws: wss:;";

    let csp_value = std::env::var("FB_CSP").unwrap_or_else(|_| DEFAULT_CSP.to_string());
    let header_value = HeaderValue::from_str(&csp_value)
        .unwrap_or_else(|_| HeaderValue::from_static(DEFAULT_CSP));

    SetResponseHeaderLayer::overriding(
        axum::http::header::CONTENT_SECURITY_POLICY,
        header_value,
    )
}

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
        .route("/ws", get(websocket::ws_handler));

    // Static file serving for the dashboard
    let static_files = ServeDir::new("demo/gui/static")
        .not_found_service(handlers::serve_index_with_config());

    // CSP header: default policy for local static assets.
    // - Script sources limited to self (vendor assets).
    // - 'unsafe-inline' required for inline style attributes in the demo.
    // - Override via FB_CSP for stricter policies.
    let csp_header = build_csp_header();

    Router::new()
        // Task 13.2: Serve index.html with config injection at root
        .route("/", get(handlers::get_index))
        .nest("/api", api_routes)
        .fallback_service(static_files)
        .layer(csp_header)
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
}
