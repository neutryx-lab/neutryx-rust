//! WebSocket handler for real-time updates.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Serialize;
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

    // Handle incoming messages from client
    let tx = state.tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                // Echo or handle client messages
                if text.as_str() == "ping" {
                    let _ = tx.send(r#"{"type":"pong"}"#.to_string());
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
}
