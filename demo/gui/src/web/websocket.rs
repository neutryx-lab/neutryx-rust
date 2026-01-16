//! WebSocket handler for real-time updates.
//!
//! This module provides WebSocket functionality for the FrictionalBank Web Dashboard,
//! including real-time updates for risk metrics, exposure, and computation graph changes.
//!
//! ## Message Types
//!
//! - `risk`: Risk metrics update (PV, CVA, DVA, FVA)
//! - `exposure`: Exposure metrics update (EE, EPE, PFE)
//! - `graph_update`: Computation graph node updates (Task 4.1)
//! - `irs_benchmark`: IRS AAD ベンチマーク結果の配信 (Task 6.3)
//!
//! ## Subscription Support (Task 4.3)
//!
//! Clients can subscribe to specific trade graph updates:
//! - Send: `{"type":"subscribe_graph","trade_id":"T001"}`
//! - Send: `{"type":"unsubscribe_graph","trade_id":"T001"}`
//!
//! ## IRS AAD Benchmark Updates (Task 6.3)
//!
//! ベンチマーク結果をリアルタイムで配信:
//! - `irs_benchmark`: AAD vs Bump&Reval の性能比較結果

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use super::AppState;

/// WebSocket upgrade handler
pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    info!("WebSocket client connected");

    // Send initial data
    let initial = serde_json::json!({
        "type": "connected",
        "message": "Welcome to FrictionalBank Dashboard"
    });
    if let Err(e) = sender.send(Message::Text(initial.to_string().into())).await {
        warn!("Failed to send initial message: {}", e);
        return;
    }

    // Spawn task to forward broadcast messages to this client
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if sender.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    // Client is slow, skip lagged messages and continue
                    // This prevents reconnection loops when broadcast traffic is high
                    warn!("WebSocket client lagged by {} messages, continuing", n);
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // Channel closed, exit the task
                    break;
                }
            }
        }
    });

    // Handle incoming messages from client (Task 4.3: subscription support)
    let state_clone = Arc::clone(&state);
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                let text_str = text.as_str();

                // Task 2.3: Handle ping/pong for heartbeat mechanism
                // Support both simple "ping" and JSON format {"type":"ping",...}
                if text_str == "ping" {
                    let _ = state_clone.tx.send(r#"{"type":"pong"}"#.to_string());
                    continue;
                }
                if let Ok(ping_msg) = serde_json::from_str::<serde_json::Value>(text_str) {
                    if ping_msg.get("type").and_then(|v| v.as_str()) == Some("ping") {
                        let now = chrono::Utc::now().timestamp_millis();

                        // Task 6.4: Calculate round-trip latency from client timestamp
                        if let Some(client_ts) = ping_msg.get("timestamp").and_then(|v| v.as_i64())
                        {
                            let latency_ms = (now - client_ts).max(0) as u64;
                            // Record latency in microseconds
                            state_clone
                                .metrics
                                .record_ws_latency(latency_ms * 1000)
                                .await;
                        }

                        let pong = serde_json::json!({
                            "type": "pong",
                            "timestamp": now
                        });
                        let _ = state_clone.tx.send(pong.to_string());
                        continue;
                    }
                }

                // Task 4.3: Handle graph subscription requests
                if let Ok(request) = serde_json::from_str::<GraphSubscriptionRequest>(text_str) {
                    match request.request_type.as_str() {
                        "subscribe_graph" => {
                            state_clone.subscribe_graph(&request.trade_id).await;
                            info!(
                                "Client subscribed to graph updates for trade: {}",
                                request.trade_id
                            );

                            // Send confirmation
                            let confirmation = serde_json::json!({
                                "type": "subscription_confirmed",
                                "trade_id": request.trade_id,
                                "action": "subscribe_graph"
                            });
                            let _ = state_clone.tx.send(confirmation.to_string());
                        }
                        "unsubscribe_graph" => {
                            state_clone.unsubscribe_graph(&request.trade_id).await;
                            info!(
                                "Client unsubscribed from graph updates for trade: {}",
                                request.trade_id
                            );

                            // Send confirmation
                            let confirmation = serde_json::json!({
                                "type": "subscription_confirmed",
                                "trade_id": request.trade_id,
                                "action": "unsubscribe_graph"
                            });
                            let _ = state_clone.tx.send(confirmation.to_string());
                        }
                        _ => {
                            warn!(
                                "Unknown subscription request type: {}",
                                request.request_type
                            );
                        }
                    }
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    info!("WebSocket client disconnected");
}

/// Real-time update message
#[derive(Debug, Serialize)]
pub struct RealTimeUpdate {
    pub update_type: String,
    pub timestamp: i64,
    pub data: serde_json::Value,
}

impl RealTimeUpdate {
    /// Create a risk metrics update
    pub fn risk_update(total_pv: f64, cva: f64, dva: f64, fva: f64) -> Self {
        Self {
            update_type: "risk".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            data: serde_json::json!({
                "total_pv": total_pv,
                "cva": cva,
                "dva": dva,
                "fva": fva
            }),
        }
    }

    /// Create an exposure update
    pub fn exposure_update(ee: f64, epe: f64, pfe: f64) -> Self {
        Self {
            update_type: "exposure".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            data: serde_json::json!({
                "ee": ee,
                "epe": epe,
                "pfe": pfe
            }),
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Broadcast an update to all connected clients
pub fn broadcast_update(state: &AppState, update: RealTimeUpdate) {
    let _ = state.tx.send(update.to_json());
}

// =============================================================================
// Task 4.1: Graph Update Message Types
// =============================================================================

/// Update information for a single graph node.
///
/// Used for WebSocket real-time updates to send only the changed
/// nodes rather than the entire graph.
///
/// # Example
///
/// ```rust
/// use demo_gui::web::websocket::GraphNodeUpdate;
///
/// let update = GraphNodeUpdate {
///     id: "N1".to_string(),
///     value: 101.5,
///     delta: Some(1.5),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeUpdate {
    /// Node ID being updated
    pub id: String,

    /// New computed value
    pub value: f64,

    /// Change from previous value (for animation)
    pub delta: Option<f64>,
}

impl RealTimeUpdate {
    /// Create a graph update event.
    ///
    /// This message type is used to notify clients about changes to
    /// computation graph nodes, typically triggered by market data updates.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - The trade ID whose graph has been updated
    /// * `updated_nodes` - Vector of node updates with new values and deltas
    ///
    /// # Example
    ///
    /// ```rust
    /// use demo_gui::web::websocket::{RealTimeUpdate, GraphNodeUpdate};
    ///
    /// let updated_nodes = vec![
    ///     GraphNodeUpdate {
    ///         id: "N1".to_string(),
    ///         value: 101.5,
    ///         delta: Some(1.5),
    ///     },
    /// ];
    ///
    /// let update = RealTimeUpdate::graph_update("T001", updated_nodes);
    /// ```
    pub fn graph_update(trade_id: &str, updated_nodes: Vec<GraphNodeUpdate>) -> Self {
        Self {
            update_type: "graph_update".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            data: serde_json::json!({
                "trade_id": trade_id,
                "updated_nodes": updated_nodes
            }),
        }
    }

    // =========================================================================
    // Task 3.1: Pricing Complete Message Type
    // =========================================================================

    /// Create a pricing complete event.
    ///
    /// This message type is used to notify clients when a pricing
    /// calculation has completed.
    ///
    /// # Arguments
    ///
    /// * `calculation_id` - Unique ID for the calculation
    /// * `instrument_type` - Type of instrument priced
    /// * `pv` - Present value result
    /// * `greeks` - Optional Greeks data
    pub fn pricing_complete(
        calculation_id: &str,
        instrument_type: &str,
        pv: f64,
        greeks: Option<serde_json::Value>,
    ) -> Self {
        Self {
            update_type: "pricing_complete".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            data: serde_json::json!({
                "calculationId": calculation_id,
                "instrumentType": instrument_type,
                "pv": pv,
                "greeks": greeks
            }),
        }
    }
}

/// Broadcast a pricing complete notification to all connected clients.
///
/// # Arguments
///
/// * `state` - Application state containing the broadcast channel
/// * `calculation_id` - Unique ID for the calculation
/// * `instrument_type` - Type of instrument priced
/// * `pv` - Present value result
/// * `greeks` - Optional Greeks data
pub fn broadcast_pricing_complete(
    state: &AppState,
    calculation_id: &str,
    instrument_type: &str,
    pv: f64,
    greeks: Option<serde_json::Value>,
) {
    let update = RealTimeUpdate::pricing_complete(calculation_id, instrument_type, pv, greeks);
    let _ = state.tx.send(update.to_json());
}

// =============================================================================
// Task 6.3: IRS AAD Benchmark Message Types
// =============================================================================

/// ベンチマーク統計情報を表す構造体。
///
/// AADまたはBump&Revalの実行時間統計を保持する。
///
/// # Example
///
/// ```rust
/// use demo_gui::web::websocket::BenchmarkStats;
///
/// let stats = BenchmarkStats {
///     mean_ns: 15000.0,
///     std_dev_ns: 500.0,
///     min_ns: 14000.0,
///     max_ns: 16000.0,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkStats {
    /// 平均実行時間（ナノ秒）
    pub mean_ns: f64,

    /// 標準偏差（ナノ秒）
    pub std_dev_ns: f64,

    /// 最小実行時間（ナノ秒）
    pub min_ns: f64,

    /// 最大実行時間（ナノ秒）
    pub max_ns: f64,
}

/// IRS AADベンチマーク結果を表す構造体。
///
/// AADとBump&Revalの性能比較結果を保持し、
/// WebSocket経由でリアルタイム配信される。
///
/// # Example
///
/// ```rust
/// use demo_gui::web::websocket::{IrsBenchmarkUpdate, BenchmarkStats};
///
/// let update = IrsBenchmarkUpdate {
///     aad_stats: BenchmarkStats {
///         mean_ns: 15000.0,
///         std_dev_ns: 500.0,
///         min_ns: 14000.0,
///         max_ns: 16000.0,
///     },
///     bump_stats: BenchmarkStats {
///         mean_ns: 300000.0,
///         std_dev_ns: 5000.0,
///         min_ns: 290000.0,
///         max_ns: 310000.0,
///     },
///     speedup_ratio: 20.0,
///     tenor_count: 20,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrsBenchmarkUpdate {
    /// AAD実行時間の統計情報
    pub aad_stats: BenchmarkStats,

    /// Bump&Reval実行時間の統計情報
    pub bump_stats: BenchmarkStats,

    /// 高速化倍率（bump_mean / aad_mean）
    pub speedup_ratio: f64,

    /// テナー数（計算に使用したテナーの数）
    pub tenor_count: usize,
}

impl RealTimeUpdate {
    /// IRS AADベンチマーク結果の更新イベントを生成する。
    ///
    /// このメッセージタイプは、AADとBump&Revalの性能比較結果を
    /// クライアントにリアルタイムで通知するために使用される。
    ///
    /// # Arguments
    ///
    /// * `benchmark` - ベンチマーク結果を含む構造体
    ///
    /// # Example
    ///
    /// ```rust
    /// use demo_gui::web::websocket::{RealTimeUpdate, IrsBenchmarkUpdate, BenchmarkStats};
    ///
    /// let benchmark = IrsBenchmarkUpdate {
    ///     aad_stats: BenchmarkStats {
    ///         mean_ns: 15000.0,
    ///         std_dev_ns: 500.0,
    ///         min_ns: 14000.0,
    ///         max_ns: 16000.0,
    ///     },
    ///     bump_stats: BenchmarkStats {
    ///         mean_ns: 300000.0,
    ///         std_dev_ns: 5000.0,
    ///         min_ns: 290000.0,
    ///         max_ns: 310000.0,
    ///     },
    ///     speedup_ratio: 20.0,
    ///     tenor_count: 20,
    /// };
    ///
    /// let update = RealTimeUpdate::irs_benchmark_update(benchmark);
    /// ```
    pub fn irs_benchmark_update(benchmark: IrsBenchmarkUpdate) -> Self {
        Self {
            update_type: "irs_benchmark".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            data: serde_json::json!({
                "aad_stats": {
                    "mean_ns": benchmark.aad_stats.mean_ns,
                    "std_dev_ns": benchmark.aad_stats.std_dev_ns,
                    "min_ns": benchmark.aad_stats.min_ns,
                    "max_ns": benchmark.aad_stats.max_ns,
                },
                "bump_stats": {
                    "mean_ns": benchmark.bump_stats.mean_ns,
                    "std_dev_ns": benchmark.bump_stats.std_dev_ns,
                    "min_ns": benchmark.bump_stats.min_ns,
                    "max_ns": benchmark.bump_stats.max_ns,
                },
                "speedup_ratio": benchmark.speedup_ratio,
                "tenor_count": benchmark.tenor_count
            }),
        }
    }
}

// =============================================================================
// Task 4.2: Graph Update Broadcast
// =============================================================================

/// Broadcast a graph update to all connected clients.
///
/// This function creates a `graph_update` message and broadcasts it
/// through the existing broadcast channel.
///
/// # Arguments
///
/// * `state` - Application state containing the broadcast channel
/// * `trade_id` - The trade ID whose graph has been updated
/// * `updated_nodes` - Vector of node updates with new values and deltas
///
/// # Example
///
/// ```rust
/// use demo_gui::web::{AppState, websocket::{broadcast_graph_update, GraphNodeUpdate}};
///
/// let state = AppState::new();
/// let updated_nodes = vec![
///     GraphNodeUpdate {
///         id: "N1".to_string(),
///         value: 101.5,
///         delta: Some(1.5),
///     },
/// ];
///
/// broadcast_graph_update(&state, "T001", updated_nodes);
/// ```
pub fn broadcast_graph_update(
    state: &AppState,
    trade_id: &str,
    updated_nodes: Vec<GraphNodeUpdate>,
) {
    let update = RealTimeUpdate::graph_update(trade_id, updated_nodes);
    let _ = state.tx.send(update.to_json());
}

// =============================================================================
// Task 6.3: IRS Benchmark Broadcast
// =============================================================================

/// IRS AADベンチマーク結果を全接続クライアントに配信する。
///
/// この関数は`irs_benchmark`メッセージを生成し、
/// 既存のブロードキャストチャネル経由で配信する。
///
/// # Arguments
///
/// * `state` - ブロードキャストチャネルを含むアプリケーション状態
/// * `benchmark` - ベンチマーク結果を含む構造体
///
/// # Example
///
/// ```rust
/// use demo_gui::web::{AppState, websocket::{broadcast_irs_benchmark, IrsBenchmarkUpdate, BenchmarkStats}};
///
/// let state = AppState::new();
/// let benchmark = IrsBenchmarkUpdate {
///     aad_stats: BenchmarkStats {
///         mean_ns: 15000.0,
///         std_dev_ns: 500.0,
///         min_ns: 14000.0,
///         max_ns: 16000.0,
///     },
///     bump_stats: BenchmarkStats {
///         mean_ns: 300000.0,
///         std_dev_ns: 5000.0,
///         min_ns: 290000.0,
///         max_ns: 310000.0,
///     },
///     speedup_ratio: 20.0,
///     tenor_count: 20,
/// };
///
/// broadcast_irs_benchmark(&state, benchmark);
/// ```
pub fn broadcast_irs_benchmark(state: &AppState, benchmark: IrsBenchmarkUpdate) {
    let update = RealTimeUpdate::irs_benchmark_update(benchmark);
    let _ = state.tx.send(update.to_json());
}

// =============================================================================
// Task 6.2: IRS Bootstrap & Risk WebSocket Events
// =============================================================================

/// Bootstrap complete event data.
///
/// Sent when a yield curve bootstrap operation completes successfully.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapCompleteEvent {
    /// Unique curve identifier (UUID)
    pub curve_id: String,
    /// Number of tenor points in the curve
    pub tenor_count: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
}

/// Risk calculation complete event data.
///
/// Sent when a risk calculation (Bump, AAD, or Compare) completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskCompleteEvent {
    /// Curve identifier used for the calculation
    pub curve_id: String,
    /// Method used: "bump", "aad", or "compare"
    pub method: String,
    /// DV01 result (sum of all deltas)
    pub dv01: f64,
    /// Speedup ratio (Bump time / AAD time), null if AAD unavailable
    pub speedup_ratio: Option<f64>,
}

/// Calculation error event data.
///
/// Sent when a calculation (bootstrap, pricing, or risk) fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculationErrorEvent {
    /// Operation that failed: "bootstrap", "pricing", "risk_bump", "risk_aad", "risk_compare"
    pub operation: String,
    /// Error message
    pub error: String,
    /// Tenor that failed (for bootstrap failures)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_tenor: Option<String>,
}

impl RealTimeUpdate {
    /// Create a bootstrap complete event.
    ///
    /// # Arguments
    ///
    /// * `event` - Bootstrap complete event data
    pub fn bootstrap_complete(event: BootstrapCompleteEvent) -> Self {
        Self {
            update_type: "bootstrap_complete".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            data: serde_json::json!({
                "curveId": event.curve_id,
                "tenorCount": event.tenor_count,
                "processingTimeMs": event.processing_time_ms
            }),
        }
    }

    /// Create a risk calculation complete event.
    ///
    /// # Arguments
    ///
    /// * `event` - Risk complete event data
    pub fn risk_complete(event: RiskCompleteEvent) -> Self {
        Self {
            update_type: "risk_complete".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            data: serde_json::json!({
                "curveId": event.curve_id,
                "method": event.method,
                "dv01": event.dv01,
                "speedupRatio": event.speedup_ratio
            }),
        }
    }

    /// Create a calculation error event.
    ///
    /// # Arguments
    ///
    /// * `event` - Calculation error event data
    pub fn calculation_error(event: CalculationErrorEvent) -> Self {
        Self {
            update_type: "calculation_error".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            data: serde_json::json!({
                "operation": event.operation,
                "error": event.error,
                "failedTenor": event.failed_tenor
            }),
        }
    }
}

/// Broadcast a bootstrap complete event to all connected clients.
///
/// # Arguments
///
/// * `state` - Application state containing the broadcast channel
/// * `curve_id` - UUID of the constructed curve
/// * `tenor_count` - Number of tenor points
/// * `processing_time_ms` - Processing time in milliseconds
pub fn broadcast_bootstrap_complete(
    state: &AppState,
    curve_id: &str,
    tenor_count: usize,
    processing_time_ms: f64,
) {
    let event = BootstrapCompleteEvent {
        curve_id: curve_id.to_string(),
        tenor_count,
        processing_time_ms,
    };
    let update = RealTimeUpdate::bootstrap_complete(event);
    let _ = state.tx.send(update.to_json());
}

/// Broadcast a risk calculation complete event to all connected clients.
///
/// # Arguments
///
/// * `state` - Application state containing the broadcast channel
/// * `curve_id` - UUID of the curve used
/// * `method` - Method used: "bump", "aad", or "compare"
/// * `dv01` - DV01 result
/// * `speedup_ratio` - Speedup ratio (optional)
pub fn broadcast_risk_complete(
    state: &AppState,
    curve_id: &str,
    method: &str,
    dv01: f64,
    speedup_ratio: Option<f64>,
) {
    let event = RiskCompleteEvent {
        curve_id: curve_id.to_string(),
        method: method.to_string(),
        dv01,
        speedup_ratio,
    };
    let update = RealTimeUpdate::risk_complete(event);
    let _ = state.tx.send(update.to_json());
}

/// Broadcast a calculation error event to all connected clients.
///
/// # Arguments
///
/// * `state` - Application state containing the broadcast channel
/// * `operation` - Operation that failed
/// * `error` - Error message
/// * `failed_tenor` - Tenor that failed (for bootstrap failures)
pub fn broadcast_calculation_error(
    state: &AppState,
    operation: &str,
    error: &str,
    failed_tenor: Option<&str>,
) {
    let event = CalculationErrorEvent {
        operation: operation.to_string(),
        error: error.to_string(),
        failed_tenor: failed_tenor.map(|s| s.to_string()),
    };
    let update = RealTimeUpdate::calculation_error(event);
    let _ = state.tx.send(update.to_json());
}

// =============================================================================
// Task 5.3: Greeks Update WebSocket Events
// =============================================================================

/// Greeks update event data.
///
/// Sent when Greeks values are updated (e.g., due to market data changes).
///
/// # Requirements Coverage
///
/// - Requirement 5.5: WebSocket でリアルタイム更新を配信
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GreeksUpdateEvent {
    /// Greek type being updated (delta, gamma, vega, theta, rho)
    pub greek_type: String,
    /// Spot price used for calculation
    pub spot: f64,
    /// Strike price (for timeseries updates)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strike: Option<f64>,
    /// Tenor (time to expiry) in years (for heatmap updates)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenor: Option<f64>,
    /// Updated Greek value
    pub value: f64,
    /// Previous value (for delta change calculation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_value: Option<f64>,
    /// Volatility used
    pub volatility: f64,
    /// Update source: "heatmap" or "timeseries"
    pub source: String,
}

impl RealTimeUpdate {
    /// Create a Greeks update event.
    ///
    /// This message type is used to notify clients when Greeks values
    /// are updated in real-time.
    ///
    /// # Arguments
    ///
    /// * `event` - Greeks update event data
    ///
    /// # Example
    ///
    /// ```rust
    /// use demo_gui::web::websocket::{RealTimeUpdate, GreeksUpdateEvent};
    ///
    /// let event = GreeksUpdateEvent {
    ///     greek_type: "delta".to_string(),
    ///     spot: 100.0,
    ///     strike: Some(100.0),
    ///     tenor: None,
    ///     value: 0.55,
    ///     previous_value: Some(0.52),
    ///     volatility: 0.20,
    ///     source: "timeseries".to_string(),
    /// };
    /// let update = RealTimeUpdate::greeks_update(event);
    /// ```
    pub fn greeks_update(event: GreeksUpdateEvent) -> Self {
        Self {
            update_type: "greeks_update".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            data: serde_json::json!({
                "greekType": event.greek_type,
                "spot": event.spot,
                "strike": event.strike,
                "tenor": event.tenor,
                "value": event.value,
                "previousValue": event.previous_value,
                "volatility": event.volatility,
                "source": event.source
            }),
        }
    }
}

/// Broadcast a Greeks update event to all connected clients.
///
/// # Arguments
///
/// * `state` - Application state containing the broadcast channel
/// * `event` - Greeks update event data
///
/// # Requirements Coverage
///
/// - Requirement 5.5: WebSocket でリアルタイム更新を配信
pub fn broadcast_greeks_update(state: &AppState, event: GreeksUpdateEvent) {
    let update = RealTimeUpdate::greeks_update(event);
    let _ = state.tx.send(update.to_json());
}

/// Broadcast a batch of Greeks updates to all connected clients.
///
/// This is useful when updating multiple Greek values at once,
/// such as when market data changes.
///
/// # Arguments
///
/// * `state` - Application state containing the broadcast channel
/// * `greek_type` - Type of Greek being updated
/// * `spot` - Current spot price
/// * `values` - Vector of (strike_or_tenor, value) pairs
/// * `volatility` - Current volatility
/// * `source` - Update source: "heatmap" or "timeseries"
pub fn broadcast_greeks_batch_update(
    state: &AppState,
    greek_type: &str,
    spot: f64,
    values: Vec<(f64, f64)>,
    volatility: f64,
    source: &str,
) {
    let is_heatmap = source == "heatmap";

    for (key, value) in values {
        let event = GreeksUpdateEvent {
            greek_type: greek_type.to_string(),
            spot,
            strike: if is_heatmap { None } else { Some(key) },
            tenor: if is_heatmap { Some(key) } else { None },
            value,
            previous_value: None,
            volatility,
            source: source.to_string(),
        };
        let update = RealTimeUpdate::greeks_update(event);
        let _ = state.tx.send(update.to_json());
    }
}

// =============================================================================
// Task 4.3: Subscription Message Types
// =============================================================================

/// Client request to subscribe/unsubscribe from graph updates.
///
/// Clients send this message type to indicate which trade graphs
/// they want to receive updates for.
#[derive(Debug, Clone, Deserialize)]
pub struct GraphSubscriptionRequest {
    /// Message type: "subscribe_graph" or "unsubscribe_graph"
    #[serde(rename = "type")]
    pub request_type: String,

    /// Trade ID to subscribe/unsubscribe
    pub trade_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_update() {
        let update = RealTimeUpdate::risk_update(100.0, -10.0, 5.0, -3.0);
        assert_eq!(update.update_type, "risk");
        let json = update.to_json();
        assert!(json.contains("total_pv"));
    }

    #[test]
    fn test_exposure_update() {
        let update = RealTimeUpdate::exposure_update(500.0, 450.0, 800.0);
        assert_eq!(update.update_type, "exposure");
        let json = update.to_json();
        assert!(json.contains("ee"));
    }

    // =========================================================================
    // Task 4.1: graph_update Message Type Tests
    // =========================================================================

    mod graph_update_tests {
        use super::*;

        #[test]
        fn test_graph_update_creation() {
            let updated_nodes = vec![
                GraphNodeUpdate {
                    id: "N1".to_string(),
                    value: 101.5,
                    delta: Some(1.5),
                },
                GraphNodeUpdate {
                    id: "N2".to_string(),
                    value: 25.375,
                    delta: Some(0.375),
                },
            ];

            let update = RealTimeUpdate::graph_update("T001", updated_nodes);

            assert_eq!(update.update_type, "graph_update");
            assert!(update.timestamp > 0);
        }

        #[test]
        fn test_graph_update_contains_trade_id() {
            let updated_nodes = vec![GraphNodeUpdate {
                id: "N1".to_string(),
                value: 100.0,
                delta: None,
            }];

            let update = RealTimeUpdate::graph_update("T001", updated_nodes);
            let json = update.to_json();

            assert!(json.contains("\"trade_id\":\"T001\""));
        }

        #[test]
        fn test_graph_update_contains_updated_nodes() {
            let updated_nodes = vec![GraphNodeUpdate {
                id: "N1".to_string(),
                value: 101.5,
                delta: Some(1.5),
            }];

            let update = RealTimeUpdate::graph_update("T001", updated_nodes);
            let json = update.to_json();

            assert!(json.contains("\"updated_nodes\":"));
            assert!(json.contains("\"id\":\"N1\""));
            assert!(json.contains("\"value\":101.5"));
            assert!(json.contains("\"delta\":1.5"));
        }

        #[test]
        fn test_graph_update_empty_nodes() {
            let updated_nodes: Vec<GraphNodeUpdate> = vec![];
            let update = RealTimeUpdate::graph_update("T001", updated_nodes);
            let json = update.to_json();

            assert!(json.contains("\"updated_nodes\":[]"));
        }

        #[test]
        fn test_graph_update_json_structure() {
            let updated_nodes = vec![GraphNodeUpdate {
                id: "N1".to_string(),
                value: 100.0,
                delta: Some(5.0),
            }];

            let update = RealTimeUpdate::graph_update("T001", updated_nodes);
            let json = update.to_json();

            // Verify JSON structure matches WebSocket message format
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["update_type"], "graph_update");
            assert!(parsed["timestamp"].is_number());
            assert!(parsed["data"]["trade_id"].is_string());
            assert!(parsed["data"]["updated_nodes"].is_array());
        }

        #[test]
        fn test_graph_node_update_serialisation() {
            let node_update = GraphNodeUpdate {
                id: "N1".to_string(),
                value: 101.5,
                delta: Some(1.5),
            };

            let json = serde_json::to_string(&node_update).unwrap();
            assert!(json.contains("\"id\":\"N1\""));
            assert!(json.contains("\"value\":101.5"));
            assert!(json.contains("\"delta\":1.5"));
        }

        #[test]
        fn test_graph_node_update_without_delta() {
            let node_update = GraphNodeUpdate {
                id: "N1".to_string(),
                value: 100.0,
                delta: None,
            };

            let json = serde_json::to_string(&node_update).unwrap();
            assert!(json.contains("\"id\":\"N1\""));
            assert!(json.contains("\"value\":100"));
            assert!(json.contains("\"delta\":null"));
        }
    }

    // =========================================================================
    // Task 4.2: Graph Update Broadcast Tests
    // =========================================================================

    mod graph_broadcast_tests {
        use super::*;

        #[tokio::test]
        async fn test_broadcast_graph_update() {
            let state = AppState::new();
            let mut rx = state.tx.subscribe();

            let updated_nodes = vec![GraphNodeUpdate {
                id: "N1".to_string(),
                value: 101.5,
                delta: Some(1.5),
            }];

            broadcast_graph_update(&state, "T001", updated_nodes);

            // Receive the broadcast message
            let received = rx.try_recv();
            assert!(received.is_ok());

            let msg = received.unwrap();
            assert!(msg.contains("graph_update"));
            assert!(msg.contains("T001"));
        }

        #[tokio::test]
        async fn test_broadcast_graph_update_multiple_nodes() {
            let state = AppState::new();
            let mut rx = state.tx.subscribe();

            let updated_nodes = vec![
                GraphNodeUpdate {
                    id: "N1".to_string(),
                    value: 101.5,
                    delta: Some(1.5),
                },
                GraphNodeUpdate {
                    id: "N2".to_string(),
                    value: 25.5,
                    delta: Some(0.5),
                },
                GraphNodeUpdate {
                    id: "N3".to_string(),
                    value: 50.0,
                    delta: None,
                },
            ];

            broadcast_graph_update(&state, "T001", updated_nodes);

            let received = rx.try_recv().unwrap();
            assert!(received.contains("N1"));
            assert!(received.contains("N2"));
            assert!(received.contains("N3"));
        }
    }

    // =========================================================================
    // Task 4.3: Subscription Tests
    // =========================================================================

    mod subscription_tests {
        use super::*;

        #[tokio::test]
        async fn test_subscribe_to_trade() {
            let state = AppState::new();

            // Subscribe to T001
            state.subscribe_graph("T001").await;

            let subscriptions = state.graph_subscriptions.read().await;
            assert!(subscriptions.contains("T001"));
        }

        #[tokio::test]
        async fn test_unsubscribe_from_trade() {
            let state = AppState::new();

            // Subscribe then unsubscribe
            state.subscribe_graph("T001").await;
            state.unsubscribe_graph("T001").await;

            let subscriptions = state.graph_subscriptions.read().await;
            assert!(!subscriptions.contains("T001"));
        }

        #[tokio::test]
        async fn test_multiple_subscriptions() {
            let state = AppState::new();

            state.subscribe_graph("T001").await;
            state.subscribe_graph("T002").await;
            state.subscribe_graph("T003").await;

            let subscriptions = state.graph_subscriptions.read().await;
            assert_eq!(subscriptions.len(), 3);
            assert!(subscriptions.contains("T001"));
            assert!(subscriptions.contains("T002"));
            assert!(subscriptions.contains("T003"));
        }

        #[tokio::test]
        async fn test_is_subscribed() {
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
            assert!(subs.contains(&"T001".to_string()));
            assert!(subs.contains(&"T002".to_string()));
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
        async fn test_subscription_message_handling() {
            // Test that subscription request messages are properly parsed
            let msg = r#"{"type":"subscribe_graph","trade_id":"T001"}"#;
            let parsed: serde_json::Value = serde_json::from_str(msg).unwrap();

            assert_eq!(parsed["type"], "subscribe_graph");
            assert_eq!(parsed["trade_id"], "T001");
        }

        #[tokio::test]
        async fn test_unsubscription_message_handling() {
            let msg = r#"{"type":"unsubscribe_graph","trade_id":"T001"}"#;
            let parsed: serde_json::Value = serde_json::from_str(msg).unwrap();

            assert_eq!(parsed["type"], "unsubscribe_graph");
            assert_eq!(parsed["trade_id"], "T001");
        }
    }

    // =========================================================================
    // Task 6.3: IRS AAD Benchmark Tests
    // =========================================================================

    mod irs_benchmark_tests {
        use super::*;

        /// テスト用のベンチマーク統計情報を生成するヘルパー関数
        fn create_test_stats(mean: f64, std_dev: f64, min: f64, max: f64) -> BenchmarkStats {
            BenchmarkStats {
                mean_ns: mean,
                std_dev_ns: std_dev,
                min_ns: min,
                max_ns: max,
            }
        }

        /// テスト用のベンチマーク結果を生成するヘルパー関数
        fn create_test_benchmark() -> IrsBenchmarkUpdate {
            IrsBenchmarkUpdate {
                aad_stats: create_test_stats(15000.0, 500.0, 14000.0, 16000.0),
                bump_stats: create_test_stats(300000.0, 5000.0, 290000.0, 310000.0),
                speedup_ratio: 20.0,
                tenor_count: 20,
            }
        }

        #[test]
        fn test_benchmark_stats_creation() {
            let stats = create_test_stats(15000.0, 500.0, 14000.0, 16000.0);

            assert_eq!(stats.mean_ns, 15000.0);
            assert_eq!(stats.std_dev_ns, 500.0);
            assert_eq!(stats.min_ns, 14000.0);
            assert_eq!(stats.max_ns, 16000.0);
        }

        #[test]
        fn test_benchmark_stats_serialisation() {
            let stats = create_test_stats(15000.0, 500.0, 14000.0, 16000.0);

            let json = serde_json::to_string(&stats).unwrap();
            assert!(json.contains("\"mean_ns\":15000"));
            assert!(json.contains("\"std_dev_ns\":500"));
            assert!(json.contains("\"min_ns\":14000"));
            assert!(json.contains("\"max_ns\":16000"));
        }

        #[test]
        fn test_irs_benchmark_update_creation() {
            let benchmark = create_test_benchmark();

            assert_eq!(benchmark.aad_stats.mean_ns, 15000.0);
            assert_eq!(benchmark.bump_stats.mean_ns, 300000.0);
            assert_eq!(benchmark.speedup_ratio, 20.0);
            assert_eq!(benchmark.tenor_count, 20);
        }

        #[test]
        fn test_irs_benchmark_update_serialisation() {
            let benchmark = create_test_benchmark();

            let json = serde_json::to_string(&benchmark).unwrap();
            assert!(json.contains("aad_stats"));
            assert!(json.contains("bump_stats"));
            assert!(json.contains("speedup_ratio"));
            assert!(json.contains("tenor_count"));
        }

        #[test]
        fn test_realtime_update_irs_benchmark() {
            let benchmark = create_test_benchmark();
            let update = RealTimeUpdate::irs_benchmark_update(benchmark);

            assert_eq!(update.update_type, "irs_benchmark");
            assert!(update.timestamp > 0);
        }

        #[test]
        fn test_irs_benchmark_update_json_structure() {
            let benchmark = create_test_benchmark();
            let update = RealTimeUpdate::irs_benchmark_update(benchmark);
            let json = update.to_json();

            // JSONとしてパースできることを確認
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

            // 基本構造を確認
            assert_eq!(parsed["update_type"], "irs_benchmark");
            assert!(parsed["timestamp"].is_number());
            assert!(parsed["data"].is_object());
        }

        #[test]
        fn test_irs_benchmark_update_contains_aad_stats() {
            let benchmark = create_test_benchmark();
            let update = RealTimeUpdate::irs_benchmark_update(benchmark);
            let json = update.to_json();

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            let aad_stats = &parsed["data"]["aad_stats"];

            assert_eq!(aad_stats["mean_ns"], 15000.0);
            assert_eq!(aad_stats["std_dev_ns"], 500.0);
            assert_eq!(aad_stats["min_ns"], 14000.0);
            assert_eq!(aad_stats["max_ns"], 16000.0);
        }

        #[test]
        fn test_irs_benchmark_update_contains_bump_stats() {
            let benchmark = create_test_benchmark();
            let update = RealTimeUpdate::irs_benchmark_update(benchmark);
            let json = update.to_json();

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            let bump_stats = &parsed["data"]["bump_stats"];

            assert_eq!(bump_stats["mean_ns"], 300000.0);
            assert_eq!(bump_stats["std_dev_ns"], 5000.0);
            assert_eq!(bump_stats["min_ns"], 290000.0);
            assert_eq!(bump_stats["max_ns"], 310000.0);
        }

        #[test]
        fn test_irs_benchmark_update_contains_speedup_and_tenor() {
            let benchmark = create_test_benchmark();
            let update = RealTimeUpdate::irs_benchmark_update(benchmark);
            let json = update.to_json();

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

            assert_eq!(parsed["data"]["speedup_ratio"], 20.0);
            assert_eq!(parsed["data"]["tenor_count"], 20);
        }

        #[test]
        fn test_irs_benchmark_update_matches_expected_format() {
            // Task 6.3で指定されたJSONフォーマットと一致することを確認
            let benchmark = create_test_benchmark();
            let update = RealTimeUpdate::irs_benchmark_update(benchmark);
            let json = update.to_json();

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

            // 期待されるフィールドがすべて存在することを確認
            assert!(parsed["update_type"].is_string());
            assert!(parsed["timestamp"].is_number());
            assert!(parsed["data"]["aad_stats"]["mean_ns"].is_number());
            assert!(parsed["data"]["aad_stats"]["std_dev_ns"].is_number());
            assert!(parsed["data"]["bump_stats"]["mean_ns"].is_number());
            assert!(parsed["data"]["bump_stats"]["std_dev_ns"].is_number());
            assert!(parsed["data"]["speedup_ratio"].is_number());
            assert!(parsed["data"]["tenor_count"].is_number());
        }

        #[tokio::test]
        async fn test_broadcast_irs_benchmark() {
            let state = AppState::new();
            let mut rx = state.tx.subscribe();

            let benchmark = create_test_benchmark();
            broadcast_irs_benchmark(&state, benchmark);

            // ブロードキャストメッセージを受信
            let received = rx.try_recv();
            assert!(received.is_ok());

            let msg = received.unwrap();
            assert!(msg.contains("irs_benchmark"));
            assert!(msg.contains("aad_stats"));
            assert!(msg.contains("bump_stats"));
        }

        #[tokio::test]
        async fn test_broadcast_irs_benchmark_multiple_times() {
            let state = AppState::new();
            let mut rx = state.tx.subscribe();

            // 複数回ブロードキャスト
            for i in 0..3 {
                let benchmark = IrsBenchmarkUpdate {
                    aad_stats: create_test_stats(
                        15000.0 + (i as f64) * 100.0,
                        500.0,
                        14000.0,
                        16000.0,
                    ),
                    bump_stats: create_test_stats(300000.0, 5000.0, 290000.0, 310000.0),
                    speedup_ratio: 20.0,
                    tenor_count: 20,
                };
                broadcast_irs_benchmark(&state, benchmark);
            }

            // 3つのメッセージが受信できることを確認
            for _ in 0..3 {
                let received = rx.try_recv();
                assert!(received.is_ok());
            }
        }

        #[test]
        fn test_benchmark_with_different_tenor_counts() {
            // 異なるテナー数でのベンチマーク
            for tenor_count in [5, 10, 20, 40] {
                let benchmark = IrsBenchmarkUpdate {
                    aad_stats: create_test_stats(15000.0, 500.0, 14000.0, 16000.0),
                    bump_stats: create_test_stats(300000.0, 5000.0, 290000.0, 310000.0),
                    speedup_ratio: 20.0,
                    tenor_count,
                };

                let update = RealTimeUpdate::irs_benchmark_update(benchmark);
                let json = update.to_json();
                let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

                assert_eq!(parsed["data"]["tenor_count"], tenor_count);
            }
        }

        #[test]
        fn test_benchmark_speedup_calculation_values() {
            // さまざまなスピードアップ値をテスト
            let speedup_values = [1.0, 5.0, 10.0, 20.0, 50.0, 100.0];

            for speedup in speedup_values {
                let benchmark = IrsBenchmarkUpdate {
                    aad_stats: create_test_stats(15000.0, 500.0, 14000.0, 16000.0),
                    bump_stats: create_test_stats(15000.0 * speedup, 5000.0, 290000.0, 310000.0),
                    speedup_ratio: speedup,
                    tenor_count: 20,
                };

                let update = RealTimeUpdate::irs_benchmark_update(benchmark);
                let json = update.to_json();
                let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

                assert_eq!(parsed["data"]["speedup_ratio"], speedup);
            }
        }
    }

    // =========================================================================
    // Task 3.1: Pricing Complete Tests
    // =========================================================================

    mod pricing_complete_tests {
        use super::*;

        #[test]
        fn test_pricing_complete_creation() {
            let update =
                RealTimeUpdate::pricing_complete("calc-123", "equity_vanilla_option", 10.45, None);

            assert_eq!(update.update_type, "pricing_complete");
            assert!(update.timestamp > 0);
        }

        #[test]
        fn test_pricing_complete_contains_calculation_id() {
            let update =
                RealTimeUpdate::pricing_complete("calc-123", "equity_vanilla_option", 10.45, None);
            let json = update.to_json();

            assert!(json.contains("\"calculationId\":\"calc-123\""));
        }

        #[test]
        fn test_pricing_complete_contains_instrument_type() {
            let update =
                RealTimeUpdate::pricing_complete("calc-123", "equity_vanilla_option", 10.45, None);
            let json = update.to_json();

            assert!(json.contains("\"instrumentType\":\"equity_vanilla_option\""));
        }

        #[test]
        fn test_pricing_complete_contains_pv() {
            let update =
                RealTimeUpdate::pricing_complete("calc-123", "equity_vanilla_option", 10.45, None);
            let json = update.to_json();

            assert!(json.contains("\"pv\":10.45"));
        }

        #[test]
        fn test_pricing_complete_with_greeks() {
            let greeks = serde_json::json!({
                "delta": 0.55,
                "gamma": 0.02,
                "vega": 0.38,
                "theta": -0.05,
                "rho": 0.42
            });

            let update = RealTimeUpdate::pricing_complete(
                "calc-123",
                "equity_vanilla_option",
                10.45,
                Some(greeks),
            );
            let json = update.to_json();

            assert!(json.contains("\"delta\":0.55"));
            assert!(json.contains("\"gamma\":0.02"));
        }

        #[test]
        fn test_pricing_complete_json_structure() {
            let update = RealTimeUpdate::pricing_complete("calc-123", "fx_option", 0.045, None);
            let json = update.to_json();

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["update_type"], "pricing_complete");
            assert!(parsed["timestamp"].is_number());
            assert!(parsed["data"]["calculationId"].is_string());
            assert!(parsed["data"]["instrumentType"].is_string());
            assert!(parsed["data"]["pv"].is_number());
        }

        #[tokio::test]
        async fn test_broadcast_pricing_complete() {
            let state = AppState::new();
            let mut rx = state.tx.subscribe();

            broadcast_pricing_complete(&state, "calc-456", "irs", 45000.0, None);

            let received = rx.try_recv();
            assert!(received.is_ok());

            let msg = received.unwrap();
            assert!(msg.contains("pricing_complete"));
            assert!(msg.contains("calc-456"));
        }

        #[tokio::test]
        async fn test_broadcast_pricing_complete_with_greeks() {
            let state = AppState::new();
            let mut rx = state.tx.subscribe();

            let greeks = serde_json::json!({
                "delta": 450.0,
                "rho": 450.0
            });

            broadcast_pricing_complete(&state, "calc-789", "irs", 90000.0, Some(greeks));

            let received = rx.try_recv();
            assert!(received.is_ok());

            let msg = received.unwrap();
            assert!(msg.contains("delta"));
            assert!(msg.contains("rho"));
        }
    }

    // =========================================================================
    // Task 6.2: Bootstrap & Risk Event Tests
    // =========================================================================

    mod bootstrap_risk_event_tests {
        use super::*;

        #[test]
        fn test_bootstrap_complete_event_creation() {
            let event = BootstrapCompleteEvent {
                curve_id: "test-curve-123".to_string(),
                tenor_count: 9,
                processing_time_ms: 45.5,
            };
            let update = RealTimeUpdate::bootstrap_complete(event);

            assert_eq!(update.update_type, "bootstrap_complete");
            assert!(update.timestamp > 0);
        }

        #[test]
        fn test_bootstrap_complete_json_structure() {
            let event = BootstrapCompleteEvent {
                curve_id: "curve-abc-123".to_string(),
                tenor_count: 9,
                processing_time_ms: 42.5,
            };
            let update = RealTimeUpdate::bootstrap_complete(event);
            let json = update.to_json();

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["update_type"], "bootstrap_complete");
            assert_eq!(parsed["data"]["curveId"], "curve-abc-123");
            assert_eq!(parsed["data"]["tenorCount"], 9);
            assert_eq!(parsed["data"]["processingTimeMs"], 42.5);
        }

        #[test]
        fn test_risk_complete_event_creation() {
            let event = RiskCompleteEvent {
                curve_id: "test-curve-123".to_string(),
                method: "compare".to_string(),
                dv01: 1234.56,
                speedup_ratio: Some(15.5),
            };
            let update = RealTimeUpdate::risk_complete(event);

            assert_eq!(update.update_type, "risk_complete");
            assert!(update.timestamp > 0);
        }

        #[test]
        fn test_risk_complete_with_speedup() {
            let event = RiskCompleteEvent {
                curve_id: "curve-xyz".to_string(),
                method: "compare".to_string(),
                dv01: 5000.0,
                speedup_ratio: Some(20.5),
            };
            let update = RealTimeUpdate::risk_complete(event);
            let json = update.to_json();

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["data"]["method"], "compare");
            assert_eq!(parsed["data"]["dv01"], 5000.0);
            assert_eq!(parsed["data"]["speedupRatio"], 20.5);
        }

        #[test]
        fn test_risk_complete_without_speedup() {
            let event = RiskCompleteEvent {
                curve_id: "curve-xyz".to_string(),
                method: "bump".to_string(),
                dv01: 3000.0,
                speedup_ratio: None,
            };
            let update = RealTimeUpdate::risk_complete(event);
            let json = update.to_json();

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["data"]["method"], "bump");
            assert!(parsed["data"]["speedupRatio"].is_null());
        }

        #[test]
        fn test_calculation_error_event_creation() {
            let event = CalculationErrorEvent {
                operation: "bootstrap".to_string(),
                error: "Convergence failure at 10Y".to_string(),
                failed_tenor: Some("10Y".to_string()),
            };
            let update = RealTimeUpdate::calculation_error(event);

            assert_eq!(update.update_type, "calculation_error");
            assert!(update.timestamp > 0);
        }

        #[test]
        fn test_calculation_error_with_tenor() {
            let event = CalculationErrorEvent {
                operation: "bootstrap".to_string(),
                error: "Failed to converge".to_string(),
                failed_tenor: Some("5Y".to_string()),
            };
            let update = RealTimeUpdate::calculation_error(event);
            let json = update.to_json();

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["data"]["operation"], "bootstrap");
            assert_eq!(parsed["data"]["failedTenor"], "5Y");
        }

        #[test]
        fn test_calculation_error_without_tenor() {
            let event = CalculationErrorEvent {
                operation: "pricing".to_string(),
                error: "Invalid curve ID".to_string(),
                failed_tenor: None,
            };
            let update = RealTimeUpdate::calculation_error(event);
            let json = update.to_json();

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["data"]["operation"], "pricing");
            assert!(parsed["data"]["failedTenor"].is_null());
        }

        #[tokio::test]
        async fn test_broadcast_bootstrap_complete() {
            let state = AppState::new();
            let mut rx = state.tx.subscribe();

            broadcast_bootstrap_complete(&state, "curve-123", 9, 50.0);

            let received = rx.try_recv();
            assert!(received.is_ok());

            let msg = received.unwrap();
            assert!(msg.contains("bootstrap_complete"));
            assert!(msg.contains("curve-123"));
            assert!(msg.contains("tenorCount"));
        }

        #[tokio::test]
        async fn test_broadcast_risk_complete() {
            let state = AppState::new();
            let mut rx = state.tx.subscribe();

            broadcast_risk_complete(&state, "curve-456", "compare", 1500.0, Some(18.5));

            let received = rx.try_recv();
            assert!(received.is_ok());

            let msg = received.unwrap();
            assert!(msg.contains("risk_complete"));
            assert!(msg.contains("compare"));
            assert!(msg.contains("dv01"));
        }

        #[tokio::test]
        async fn test_broadcast_calculation_error() {
            let state = AppState::new();
            let mut rx = state.tx.subscribe();

            broadcast_calculation_error(&state, "bootstrap", "Convergence failure", Some("10Y"));

            let received = rx.try_recv();
            assert!(received.is_ok());

            let msg = received.unwrap();
            assert!(msg.contains("calculation_error"));
            assert!(msg.contains("bootstrap"));
            assert!(msg.contains("Convergence failure"));
        }

        // Task 5.3: Greeks Update Tests

        #[test]
        fn test_greeks_update_event_serialisation() {
            let event = GreeksUpdateEvent {
                greek_type: "delta".to_string(),
                spot: 100.0,
                strike: Some(100.0),
                tenor: None,
                value: 0.55,
                previous_value: Some(0.52),
                volatility: 0.20,
                source: "timeseries".to_string(),
            };

            let update = RealTimeUpdate::greeks_update(event);
            let json = update.to_json();

            assert!(json.contains("greeks_update"));
            assert!(json.contains("delta"));
            assert!(json.contains("0.55"));
            assert!(json.contains("previousValue"));
        }

        #[test]
        fn test_greeks_update_event_heatmap_source() {
            let event = GreeksUpdateEvent {
                greek_type: "gamma".to_string(),
                spot: 110.0,
                strike: None,
                tenor: Some(1.0),
                value: 0.025,
                previous_value: None,
                volatility: 0.25,
                source: "heatmap".to_string(),
            };

            let update = RealTimeUpdate::greeks_update(event);
            let json = update.to_json();

            assert!(json.contains("heatmap"));
            assert!(json.contains("gamma"));
            assert!(json.contains("tenor"));
        }

        #[tokio::test]
        async fn test_broadcast_greeks_update() {
            let state = AppState::new();
            let mut rx = state.tx.subscribe();

            let event = GreeksUpdateEvent {
                greek_type: "vega".to_string(),
                spot: 100.0,
                strike: Some(105.0),
                tenor: None,
                value: 0.35,
                previous_value: None,
                volatility: 0.20,
                source: "timeseries".to_string(),
            };

            broadcast_greeks_update(&state, event);

            let received = rx.try_recv();
            assert!(received.is_ok());

            let msg = received.unwrap();
            assert!(msg.contains("greeks_update"));
            assert!(msg.contains("vega"));
            assert!(msg.contains("0.35"));
        }

        #[tokio::test]
        async fn test_broadcast_greeks_batch_update() {
            let state = AppState::new();
            let mut rx = state.tx.subscribe();

            let values = vec![(0.25, 0.52), (0.5, 0.53), (1.0, 0.55)];
            broadcast_greeks_batch_update(&state, "delta", 100.0, values, 0.20, "heatmap");

            // Should receive 3 messages
            for _ in 0..3 {
                let received = rx.try_recv();
                assert!(received.is_ok());
                let msg = received.unwrap();
                assert!(msg.contains("greeks_update"));
                assert!(msg.contains("delta"));
            }
        }
    }
}
