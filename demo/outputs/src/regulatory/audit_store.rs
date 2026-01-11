//! Audit trail storage for regulatory compliance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{self, BufReader, BufWriter};
use std::path::Path;
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

    /// Save audit trail to a JSON file
    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        let events = self.events.read().unwrap();
        let events_vec: Vec<_> = events.iter().collect();

        let file = std::fs::File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &events_vec)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(())
    }

    /// Load audit trail from a JSON file
    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);

        let events_vec: Vec<AuditEvent> = serde_json::from_reader(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Find the highest counter value from loaded events
        let max_counter = events_vec
            .iter()
            .filter_map(|e| {
                e.event_id
                    .strip_prefix("AUD-")
                    .and_then(|s| s.parse::<u64>().ok())
            })
            .max()
            .unwrap_or(0);

        let store = Self {
            events: Arc::new(RwLock::new(VecDeque::from(events_vec))),
            max_events: 100000,
            counter: Arc::new(RwLock::new(max_counter)),
        };

        Ok(store)
    }

    /// Append to existing file (incremental save)
    pub fn append_to_file(&self, path: &Path, since_event_id: Option<&str>) -> io::Result<usize> {
        let events = self.events.read().unwrap();

        // Find events after the specified event ID
        let events_to_save: Vec<_> = if let Some(since_id) = since_event_id {
            let mut found = false;
            events
                .iter()
                .filter(|e| {
                    if found {
                        true
                    } else if e.event_id == since_id {
                        found = true;
                        false
                    } else {
                        false
                    }
                })
                .collect()
        } else {
            events.iter().collect()
        };

        let count = events_to_save.len();
        if count == 0 {
            return Ok(0);
        }

        // Load existing events if file exists
        let mut all_events: Vec<AuditEvent> = if path.exists() {
            let file = std::fs::File::open(path)?;
            let reader = BufReader::new(file);
            serde_json::from_reader(reader)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        } else {
            Vec::new()
        };

        // Append new events
        all_events.extend(events_to_save.into_iter().cloned());

        // Write back
        let file = std::fs::File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &all_events)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(count)
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

    #[test]
    fn test_audit_store_save_and_load() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_audit_store.json");

        // Create and populate store
        let store = AuditStore::new();
        store.record(
            AuditEventType::TradeBooked,
            "USER1",
            "Trade",
            "T001",
            serde_json::json!({"notional": 1000000}),
        );
        store.record(
            AuditEventType::Valuation,
            "SYSTEM",
            "Trade",
            "T001",
            serde_json::json!({"pv": 50000}),
        );
        store.record(
            AuditEventType::ReportSubmitted,
            "USER2",
            "Report",
            "R001",
            serde_json::json!({"type": "SaCcr"}),
        );

        // Save to file
        store.save_to_file(&file_path).expect("Failed to save");

        // Load from file
        let loaded_store = AuditStore::load_from_file(&file_path).expect("Failed to load");

        // Verify loaded data
        assert_eq!(loaded_store.count(), 3);

        let trade_events = loaded_store.get_events_for_entity("Trade", "T001");
        assert_eq!(trade_events.len(), 2);

        let report_events = loaded_store.get_events_by_type(AuditEventType::ReportSubmitted);
        assert_eq!(report_events.len(), 1);
        assert_eq!(report_events[0].entity_id, "R001");

        // Cleanup
        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn test_audit_store_counter_continuity_after_load() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_audit_counter.json");

        // Create store and add events
        let store = AuditStore::new();
        for i in 0..5 {
            store.record(
                AuditEventType::TradeBooked,
                "SYSTEM",
                "Trade",
                &format!("T{:03}", i),
                serde_json::json!({}),
            );
        }

        // Save and reload
        store.save_to_file(&file_path).expect("Failed to save");
        let loaded_store = AuditStore::load_from_file(&file_path).expect("Failed to load");

        // New event should continue from last counter
        let new_event_id = loaded_store.record(
            AuditEventType::TradeBooked,
            "SYSTEM",
            "Trade",
            "T005",
            serde_json::json!({}),
        );

        // The new event should have ID "AUD-000000000006" (counter 6)
        assert!(new_event_id.starts_with("AUD-"));
        assert_eq!(loaded_store.count(), 6);

        // Verify counter continuity
        let events = loaded_store.get_recent_events(1);
        assert_eq!(events[0].event_id, "AUD-000000000006");

        // Cleanup
        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn test_audit_store_append() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_audit_append.json");

        // Start with empty file
        let _ = std::fs::remove_file(&file_path);

        // Create store and add initial events
        let store = AuditStore::new();
        let _id1 = store.record(
            AuditEventType::TradeBooked,
            "USER1",
            "Trade",
            "T001",
            serde_json::json!({}),
        );
        let id2 = store.record(
            AuditEventType::Valuation,
            "SYSTEM",
            "Trade",
            "T001",
            serde_json::json!({}),
        );

        // Save initial events
        store.save_to_file(&file_path).expect("Failed to save");

        // Add more events
        let _id3 = store.record(
            AuditEventType::ReportGenerated,
            "SYSTEM",
            "Report",
            "R001",
            serde_json::json!({}),
        );

        // Append only new events (after id2)
        let appended = store
            .append_to_file(&file_path, Some(&id2))
            .expect("Failed to append");
        assert_eq!(appended, 1);

        // Load and verify
        let loaded_store = AuditStore::load_from_file(&file_path).expect("Failed to load");
        assert_eq!(loaded_store.count(), 3);

        // Cleanup
        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn test_audit_store_load_nonexistent_file() {
        let result = AuditStore::load_from_file(Path::new("/nonexistent/path/audit.json"));
        assert!(result.is_err());
    }
}
