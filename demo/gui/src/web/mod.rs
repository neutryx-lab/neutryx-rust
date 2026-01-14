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
pub mod websocket;

use axum::{
    routing::{get, post},
    Router,
};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;

use handlers::GraphCache;

/// Application state shared across handlers
pub struct AppState {
    /// Broadcast channel for real-time updates
    pub tx: broadcast::Sender<String>,
    /// Graph cache for performance optimisation (Task 3.3)
    pub graph_cache: RwLock<GraphCache>,
    /// Set of trade IDs that clients have subscribed to for graph updates (Task 4.3)
    pub graph_subscriptions: RwLock<HashSet<String>>,
}

impl AppState {
    /// Create new application state
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            tx,
            graph_cache: RwLock::new(GraphCache::new()),
            graph_subscriptions: RwLock::new(HashSet::new()),
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
        // Task 3.2: Add /api/graph route for computation graph visualisation
        .route("/graph", get(handlers::get_graph))
        // Task 7.2: Add /api/benchmark/speed-comparison route for speed comparison chart
        .route(
            "/benchmark/speed-comparison",
            get(handlers::get_speed_comparison),
        )
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
