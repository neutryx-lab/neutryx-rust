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
//!
//! ## Subscription Support (Task 4.3)
//!
//! Clients can subscribe to specific trade graph updates:
//! - Send: `{"type":"subscribe_graph","trade_id":"T001"}`
//! - Send: `{"type":"unsubscribe_graph","trade_id":"T001"}`

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
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages from client (Task 4.3: subscription support)
    let state_clone = Arc::clone(&state);
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                let text_str = text.as_str();

                // Handle ping/pong
                if text_str == "ping" {
                    let _ = state_clone.tx.send(r#"{"type":"pong"}"#.to_string());
                    continue;
                }

                // Task 4.3: Handle graph subscription requests
                if let Ok(request) = serde_json::from_str::<GraphSubscriptionRequest>(text_str) {
                    match request.request_type.as_str() {
                        "subscribe_graph" => {
                            state_clone.subscribe_graph(&request.trade_id).await;
                            info!("Client subscribed to graph updates for trade: {}", request.trade_id);

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
                            info!("Client unsubscribed from graph updates for trade: {}", request.trade_id);

                            // Send confirmation
                            let confirmation = serde_json::json!({
                                "type": "subscription_confirmed",
                                "trade_id": request.trade_id,
                                "action": "unsubscribe_graph"
                            });
                            let _ = state_clone.tx.send(confirmation.to_string());
                        }
                        _ => {
                            warn!("Unknown subscription request type: {}", request.request_type);
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
pub fn broadcast_graph_update(state: &AppState, trade_id: &str, updated_nodes: Vec<GraphNodeUpdate>) {
    let update = RealTimeUpdate::graph_update(trade_id, updated_nodes);
    let _ = state.tx.send(update.to_json());
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
            let updated_nodes = vec![
                GraphNodeUpdate {
                    id: "N1".to_string(),
                    value: 101.5,
                    delta: Some(1.5),
                },
            ];

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
}
