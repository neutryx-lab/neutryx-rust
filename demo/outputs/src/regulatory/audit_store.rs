//! Audit trail storage for regulatory compliance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

/// Audit event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Trade booked
    TradeBooked,
    /// Trade amended
    TradeAmended,
    /// Trade cancelled
    TradeCancelled,
    /// Valuation performed
    Valuation,
    /// Risk calculation
    RiskCalculation,
    /// Report generated
    ReportGenerated,
    /// Report submitted
    ReportSubmitted,
    /// User action
    UserAction,
}

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event ID
    pub event_id: String,
    /// Event type
    pub event_type: AuditEventType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// User or system ID
    pub actor: String,
    /// Entity type (trade, report, etc.)
    pub entity_type: String,
    /// Entity ID
    pub entity_id: String,
    /// Event details
    pub details: serde_json::Value,
    /// Before state (for amendments)
    pub before: Option<serde_json::Value>,
    /// After state (for amendments)
    pub after: Option<serde_json::Value>,
}

/// Audit trail store
pub struct AuditStore {
    /// Events (using VecDeque for efficient append and iteration)
    events: Arc<RwLock<VecDeque<AuditEvent>>>,
    /// Maximum events to retain
    max_events: usize,
    /// Event counter for ID generation
    counter: Arc<RwLock<u64>>,
}

impl AuditStore {
    /// Create a new audit store
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::with_capacity(10000))),
            max_events: 100000,
            counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Set maximum events to retain
    pub fn with_max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }

    /// Record an audit event
    pub fn record(&self, event_type: AuditEventType, actor: &str, entity_type: &str, entity_id: &str, details: serde_json::Value) -> String {
        let event_id = {
            let mut counter = self.counter.write().unwrap();
            *counter += 1;
            format!("AUD-{:012}", *counter)
        };

        let event = AuditEvent {
            event_id: event_id.clone(),
            event_type,
            timestamp: Utc::now(),
            actor: actor.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            details,
            before: None,
            after: None,
        };

        self.store_event(event);
        event_id
    }

    /// Record an amendment with before/after state
    pub fn record_amendment(
        &self,
        actor: &str,
        entity_type: &str,
        entity_id: &str,
        before: serde_json::Value,
        after: serde_json::Value,
    ) -> String {
        let event_id = {
            let mut counter = self.counter.write().unwrap();
            *counter += 1;
            format!("AUD-{:012}", *counter)
        };

        let event = AuditEvent {
            event_id: event_id.clone(),
            event_type: AuditEventType::TradeAmended,
            timestamp: Utc::now(),
            actor: actor.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            details: serde_json::json!({}),
            before: Some(before),
            after: Some(after),
        };

        self.store_event(event);
        event_id
    }

    /// Store an event, evicting old events if necessary
    fn store_event(&self, event: AuditEvent) {
        let mut events = self.events.write().unwrap();
        if events.len() >= self.max_events {
            events.pop_front();
        }
        events.push_back(event);
    }

    /// Get events for an entity
    pub fn get_events_for_entity(&self, entity_type: &str, entity_id: &str) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events
            .iter()
            .filter(|e| e.entity_type == entity_type && e.entity_id == entity_id)
            .cloned()
            .collect()
    }

    /// Get events by type
    pub fn get_events_by_type(&self, event_type: AuditEventType) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events
            .iter()
            .filter(|e| e.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Get events in a time range
    pub fn get_events_in_range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events
            .iter()
            .filter(|e| e.timestamp >= from && e.timestamp <= to)
            .cloned()
            .collect()
    }

    /// Get recent events
    pub fn get_recent_events(&self, count: usize) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events.iter().rev().take(count).cloned().collect()
    }

    /// Get total event count
    pub fn count(&self) -> usize {
        let events = self.events.read().unwrap();
        events.len()
    }

    /// Export all events as JSON
    pub fn export_json(&self) -> String {
        let events = self.events.read().unwrap();
        let events_vec: Vec<_> = events.iter().collect();
        serde_json::to_string_pretty(&events_vec).unwrap_or_default()
    }
}

impl Default for AuditStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_store_record() {
        let store = AuditStore::new();
        let event_id = store.record(
            AuditEventType::TradeBooked,
            "SYSTEM",
            "Trade",
            "T001",
            serde_json::json!({"notional": 1000000}),
        );
        assert!(event_id.starts_with("AUD-"));
        assert_eq!(store.count(), 1);
    }
}
