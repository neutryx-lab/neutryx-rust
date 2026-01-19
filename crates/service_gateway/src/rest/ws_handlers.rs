//! WebSocket handlers for real-time graph updates.
//!
//! Provides WebSocket endpoint for:
//! - Trade selection events from clients
//! - Subgraph updates broadcast to clients
//!
//! # Requirements Coverage
//!
//! - 5.1: `select_trades` event handler
//! - 5.2: `subgraph_update` broadcast

use std::{collections::HashSet, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::error::ServerError;

use super::graph_handlers::GraphAppState;

// ============================================================================
// WebSocket Message Types
// ============================================================================

/// Client-to-server WebSocket message
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Select trades for subgraph extraction
    SelectTrades { trade_ids: Vec<String> },
    /// Request full portfolio graph
    GetFullGraph,
    /// Ping/heartbeat
    Ping,
}

/// Server-to-client WebSocket message
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "update_type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Subgraph update response
    SubgraphUpdate { data: SubgraphData },
    /// Error response
    Error { message: String, code: u16 },
    /// Pong/heartbeat response
    Pong,
    /// Connection acknowledgement
    Connected { session_id: String },
}

/// Subgraph data in server message
#[derive(Debug, Clone, Serialize)]
pub struct SubgraphData {
    pub nodes: Vec<SubgraphNode>,
    #[serde(rename = "links")]
    pub edges: Vec<SubgraphEdge>,
    pub metadata: SubgraphMetadata,
}

/// Simplified node for WebSocket message
#[derive(Debug, Clone, Serialize)]
pub struct SubgraphNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    pub group: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trade_ids: Vec<String>,
}

/// Simplified edge for WebSocket message
#[derive(Debug, Clone, Serialize)]
pub struct SubgraphEdge {
    pub source: String,
    pub target: String,
}

/// Subgraph metadata
#[derive(Debug, Clone, Serialize)]
pub struct SubgraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub selected_trade_count: usize,
    pub shared_node_count: usize,
}

// ============================================================================
// WebSocket State
// ============================================================================

/// Extended app state with WebSocket broadcast channel
pub struct WsAppState {
    /// Graph application state
    pub graph_state: Arc<GraphAppState>,
    /// Broadcast channel for updates
    pub broadcast_tx: broadcast::Sender<ServerMessage>,
}

impl WsAppState {
    /// Create new WebSocket app state
    pub fn new(graph_state: Arc<GraphAppState>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        Self {
            graph_state,
            broadcast_tx,
        }
    }
}

// ============================================================================
// WebSocket Handlers
// ============================================================================

/// WebSocket upgrade handler
///
/// # Endpoint
///
/// `GET /ws`
///
/// # Protocol
///
/// Client sends JSON messages:
/// ```json
/// {"type": "select_trades", "trade_ids": ["T001", "T002"]}
/// {"type": "get_full_graph"}
/// {"type": "ping"}
/// ```
///
/// Server responds with:
/// ```json
/// {"update_type": "subgraph_update", "data": {...}}
/// {"update_type": "error", "message": "...", "code": 404}
/// {"update_type": "pong"}
/// ```
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WsAppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<WsAppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    // Generate session ID
    let session_id = format!("sess_{}", uuid_simple());

    // Send connection acknowledgement
    let connected_msg = ServerMessage::Connected {
        session_id: session_id.clone(),
    };
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        let _ = sender.send(Message::Text(json)).await;
    }

    // Track selected trades for this session
    let mut selected_trades: HashSet<String> = HashSet::new();

    // Spawn task to forward broadcasts to this client
    let state_clone = state.clone();
    let session_id_clone = session_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
        tracing::debug!("Send task ended for session {}", session_id_clone);
    });

    // Handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(client_msg) => {
                            handle_client_message(
                                client_msg,
                                &state_clone,
                                &mut selected_trades,
                            )
                            .await;
                        }
                        Err(e) => {
                            let error_msg = ServerMessage::Error {
                                message: format!("Invalid message format: {}", e),
                                code: 400,
                            };
                            let _ = state_clone.broadcast_tx.send(error_msg);
                        }
                    }
                }
                Message::Close(_) => {
                    tracing::debug!("Client {} disconnected", session_id);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}

/// Handle client message
async fn handle_client_message(
    msg: ClientMessage,
    state: &Arc<WsAppState>,
    selected_trades: &mut HashSet<String>,
) {
    match msg {
        ClientMessage::SelectTrades { trade_ids } => {
            // Update selected trades
            selected_trades.clear();
            selected_trades.extend(trade_ids.iter().cloned());

            // Extract subgraph
            match extract_subgraph_for_ws(&state.graph_state, &trade_ids).await {
                Ok(data) => {
                    let response = ServerMessage::SubgraphUpdate { data };
                    let _ = state.broadcast_tx.send(response);
                }
                Err(e) => {
                    let (code, message) = match e {
                        ServerError::NotFound(msg) => (404, msg),
                        ServerError::Timeout(msg) => (504, msg),
                        _ => (500, e.to_string()),
                    };
                    let response = ServerMessage::Error { message, code };
                    let _ = state.broadcast_tx.send(response);
                }
            }
        }
        ClientMessage::GetFullGraph => {
            // Extract full graph
            match extract_subgraph_for_ws(&state.graph_state, &[]).await {
                Ok(data) => {
                    let response = ServerMessage::SubgraphUpdate { data };
                    let _ = state.broadcast_tx.send(response);
                }
                Err(e) => {
                    let response = ServerMessage::Error {
                        message: e.to_string(),
                        code: 500,
                    };
                    let _ = state.broadcast_tx.send(response);
                }
            }
        }
        ClientMessage::Ping => {
            let _ = state.broadcast_tx.send(ServerMessage::Pong);
        }
    }
}

/// Extract subgraph for WebSocket response
async fn extract_subgraph_for_ws(
    state: &Arc<GraphAppState>,
    trade_ids: &[String],
) -> Result<SubgraphData, ServerError> {
    use pricer_pricing::graph::{PortfolioGraphExtractable, PortfolioGraphExtractor};
    use pricer_risk::portfolio::TradeId;
    use std::collections::HashMap;

    let portfolio = &state.portfolio;
    let extractor = PortfolioGraphExtractor::new()
        .with_timeout(500)
        .with_capacity(5_000, 10_000);

    // Build trade graphs
    let all_trade_ids: Vec<String> = portfolio
        .trade_ids()
        .map(|id| id.to_string())
        .collect();

    let mut trade_graphs: HashMap<String, pricer_pricing::graph::ComputationGraph> =
        HashMap::new();

    for trade_id in &all_trade_ids {
        if let Some(trade) = portfolio.trade(&TradeId::new(trade_id)) {
            let graph = super::graph_handlers::create_trade_graph(trade_id, trade);
            trade_graphs.insert(trade_id.clone(), graph);
        }
    }

    // Extract full graph
    let full_graph = extractor
        .extract_portfolio_graph(&all_trade_ids, &trade_graphs)
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    // Extract subgraph if trade_ids specified
    let graph = if trade_ids.is_empty() {
        full_graph
    } else {
        extractor
            .extract_subgraph(&full_graph, trade_ids)
            .map_err(|e| match e {
                pricer_pricing::graph::GraphError::TradeNotFound(id) => {
                    ServerError::NotFound(format!("Trade not found: {}", id))
                }
                _ => ServerError::Internal(e.to_string()),
            })?
    };

    // Convert to WebSocket format
    let nodes: Vec<SubgraphNode> = graph
        .nodes
        .iter()
        .map(|n| SubgraphNode {
            id: n.id.clone(),
            node_type: format!("{:?}", n.node_type),
            label: n.label.clone(),
            value: n.value,
            group: format!("{:?}", n.group),
            trade_ids: n.trade_ids.clone(),
        })
        .collect();

    let edges: Vec<SubgraphEdge> = graph
        .edges
        .iter()
        .map(|e| SubgraphEdge {
            source: e.source.clone(),
            target: e.target.clone(),
        })
        .collect();

    let shared_node_count = nodes.iter().filter(|n| n.trade_ids.len() > 1).count();

    let metadata = SubgraphMetadata {
        node_count: nodes.len(),
        edge_count: edges.len(),
        selected_trade_count: if trade_ids.is_empty() {
            portfolio.trade_count()
        } else {
            trade_ids.len()
        },
        shared_node_count,
    };

    Ok(SubgraphData {
        nodes,
        edges,
        metadata,
    })
}

/// Simple UUID-like string generator
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:04x}", now.as_nanos(), rand_u16())
}

/// Simple random u16 using timing
fn rand_u16() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 65536) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_deserialize_select_trades() {
        let json = r#"{"type": "select_trades", "trade_ids": ["T001", "T002"]}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::SelectTrades { trade_ids } => {
                assert_eq!(trade_ids, vec!["T001", "T002"]);
            }
            _ => panic!("Expected SelectTrades"),
        }
    }

    #[test]
    fn test_client_message_deserialize_get_full_graph() {
        let json = r#"{"type": "get_full_graph"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::GetFullGraph));
    }

    #[test]
    fn test_client_message_deserialize_ping() {
        let json = r#"{"type": "ping"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Ping));
    }

    #[test]
    fn test_server_message_serialize_subgraph_update() {
        let msg = ServerMessage::SubgraphUpdate {
            data: SubgraphData {
                nodes: vec![],
                edges: vec![],
                metadata: SubgraphMetadata {
                    node_count: 0,
                    edge_count: 0,
                    selected_trade_count: 0,
                    shared_node_count: 0,
                },
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("subgraph_update"));
    }

    #[test]
    fn test_server_message_serialize_error() {
        let msg = ServerMessage::Error {
            message: "Not found".to_string(),
            code: 404,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("404"));
    }
}
