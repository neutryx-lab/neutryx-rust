//! WebSocket sink for real-time risk updates.

use super::MetricUpdate;
use async_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

/// WebSocket sink for receiving real-time updates
pub struct WebSocketSink {
    /// Broadcast sender for distributing updates
    tx: broadcast::Sender<WebSocketMessage>,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Connection count
    connection_count: Arc<AtomicUsize>,
    /// Message count
    message_count: Arc<AtomicUsize>,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketMessage {
    /// Risk metric update
    MetricUpdate(MetricUpdate),
    /// Batch of metric updates
    MetricBatch(Vec<MetricUpdate>),
    /// Heartbeat
    Heartbeat { timestamp: i64 },
    /// Error message
    Error { message: String },
    /// Connection established
    Connected { client_id: String },
    /// Subscription confirmation
    Subscribed { topics: Vec<String> },
}

impl WebSocketSink {
    /// Create a new WebSocket sink
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self {
            tx,
            running: Arc::new(AtomicBool::new(true)),
            connection_count: Arc::new(AtomicUsize::new(0)),
            message_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Subscribe to receive messages
    pub fn subscribe(&self) -> broadcast::Receiver<WebSocketMessage> {
        self.connection_count.fetch_add(1, Ordering::SeqCst);
        info!(
            connections = self.connection_count.load(Ordering::SeqCst),
            "New WebSocket subscription"
        );
        self.tx.subscribe()
    }

    /// Send a metric update to all subscribers
    pub fn send_metric(&self, metric: MetricUpdate) -> Result<(), &'static str> {
        self.send(WebSocketMessage::MetricUpdate(metric))
    }

    /// Send a batch of metric updates
    pub fn send_batch(&self, metrics: Vec<MetricUpdate>) -> Result<(), &'static str> {
        self.send(WebSocketMessage::MetricBatch(metrics))
    }

    /// Send a message to all subscribers
    pub fn send(&self, message: WebSocketMessage) -> Result<(), &'static str> {
        if !self.running.load(Ordering::SeqCst) {
            return Err("WebSocket sink is stopped");
        }

        self.message_count.fetch_add(1, Ordering::SeqCst);
        
        // Ignore send errors (no subscribers)
        let _ = self.tx.send(message);
        Ok(())
    }

    /// Send a heartbeat
    pub fn heartbeat(&self) {
        let _ = self.send(WebSocketMessage::Heartbeat {
            timestamp: chrono::Utc::now().timestamp_millis(),
        });
    }

    /// Get subscriber count
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.message_count.load(Ordering::SeqCst)
    }

    /// Stop the sink
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        info!("WebSocket sink stopped");
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get statistics
    pub fn statistics(&self) -> SinkStatistics {
        SinkStatistics {
            subscribers: self.tx.receiver_count(),
            total_messages: self.message_count.load(Ordering::SeqCst),
            is_running: self.running.load(Ordering::SeqCst),
        }
    }
}

impl Default for WebSocketSink {
    fn default() -> Self {
        Self::new()
    }
}

/// Sink statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkStatistics {
    /// Current subscriber count
    pub subscribers: usize,
    /// Total messages sent
    pub total_messages: usize,
    /// Running status
    pub is_running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::risk_dashboard::MetricType;

    #[tokio::test]
    async fn test_websocket_sink() {
        let sink = WebSocketSink::new();
        let mut rx = sink.subscribe();

        sink.send_metric(MetricUpdate {
            metric_type: MetricType::CVA,
            entity_id: "CP001".to_string(),
            value: 1000.0,
            currency: "USD".to_string(),
            confidence: None,
            horizon_days: None,
            timestamp: Utc::now(),
        }).unwrap();

        let msg = rx.recv().await.unwrap();
        match msg {
            WebSocketMessage::MetricUpdate(m) => {
                assert_eq!(m.entity_id, "CP001");
            }
            _ => panic!("Expected MetricUpdate"),
        }
    }
}
