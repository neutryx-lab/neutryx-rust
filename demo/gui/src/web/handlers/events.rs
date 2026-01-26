//! Events API handlers for the Market Data Viewer WebApp.
//!
//! Provides REST API endpoints for market events:
//! - Central Bank meetings
//! - Economic data releases
//! - Market holidays
//!
//! All data is loaded from JSON files in `demo/data/input/events/`.

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use tracing::{error, info};

use crate::web::events_types::*;

// =============================================================================
// Data Loading
// =============================================================================

/// Configuration for loading events data from files.
pub struct EventsDataLoader {
    /// Base directory for events data files
    data_dir: PathBuf,
}

impl EventsDataLoader {
    /// Create a new data loader with the default data directory.
    pub fn new() -> Self {
        Self {
            data_dir: PathBuf::from("demo/data/input/events"),
        }
    }

    /// Load central banks list from file.
    pub fn load_central_banks(&self) -> Vec<CentralBank> {
        let path = self.data_dir.join("central_banks.json");
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct CentralBanksFile {
                    central_banks: Vec<CentralBank>,
                }
                match serde_json::from_str::<CentralBanksFile>(&content) {
                    Ok(data) => {
                        info!("Loaded {} central banks from {:?}", data.central_banks.len(), path);
                        data.central_banks
                    }
                    Err(e) => {
                        error!("Failed to parse central_banks.json: {}", e);
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                error!("Failed to read central_banks.json: {}", e);
                Vec::new()
            }
        }
    }

    /// Load events from a specific file.
    fn load_events_file(&self, filename: &str) -> Vec<MarketEvent> {
        let path = self.data_dir.join(filename);
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                #[derive(Deserialize)]
                struct EventsFile {
                    events: Vec<MarketEvent>,
                }
                match serde_json::from_str::<EventsFile>(&content) {
                    Ok(data) => {
                        info!("Loaded {} events from {:?}", data.events.len(), path);
                        data.events
                    }
                    Err(e) => {
                        error!("Failed to parse {}: {}", filename, e);
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                error!("Failed to read {}: {}", filename, e);
                Vec::new()
            }
        }
    }

    /// Load all events from all data files.
    pub fn load_all_events(&self) -> Vec<MarketEvent> {
        let mut all_events = Vec::new();

        // Load events from each category file
        all_events.extend(self.load_events_file("central_bank_meetings.json"));
        all_events.extend(self.load_events_file("economic_releases.json"));
        all_events.extend(self.load_events_file("holidays.json"));

        // Sort by date
        all_events.sort_by(|a, b| a.date.cmp(&b.date));

        info!("Loaded {} total events", all_events.len());
        all_events
    }
}

impl Default for EventsDataLoader {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// API Handlers
// =============================================================================

/// GET /api/events
/// List market events with optional filters.
pub async fn get_events(Query(query): Query<EventsQuery>) -> impl IntoResponse {
    let loader = EventsDataLoader::new();
    let mut events = loader.load_all_events();

    // Apply filters
    if let Some(ref event_type) = query.event_type {
        let type_lower = event_type.to_lowercase();
        events.retain(|e| {
            let type_str = format!("{:?}", e.event_type).to_lowercase();
            type_str.contains(&type_lower) || type_lower.contains(&type_str.replace('_', ""))
        });
    }

    if let Some(ref currency) = query.currency {
        let currency_upper = currency.to_uppercase();
        events.retain(|e| {
            e.currency
                .as_ref()
                .map(|c| c == &currency_upper)
                .unwrap_or(false)
        });
    }

    if let Some(ref region) = query.region {
        let region_lower = region.to_lowercase();
        events.retain(|e| {
            e.region
                .as_ref()
                .map(|r| r.to_lowercase().contains(&region_lower))
                .unwrap_or(false)
        });
    }

    if let Some(ref importance) = query.importance {
        let imp_lower = importance.to_lowercase();
        events.retain(|e| {
            let imp_str = format!("{:?}", e.importance).to_lowercase();
            imp_str == imp_lower
        });
    }

    if let Some(ref start_date) = query.start_date {
        events.retain(|e| e.date >= *start_date);
    }

    if let Some(ref end_date) = query.end_date {
        events.retain(|e| e.date <= *end_date);
    }

    let total_count = events.len();

    // Apply limit
    if let Some(limit) = query.limit {
        events.truncate(limit);
    }

    Json(EventsResponse {
        events,
        total_count,
        last_updated: chrono::Utc::now().timestamp_millis(),
    })
}

/// GET /api/events/{id}
/// Get single event details.
pub async fn get_event_detail(Path(id): Path<String>) -> impl IntoResponse {
    let loader = EventsDataLoader::new();
    let events = loader.load_all_events();

    match events.iter().find(|e| e.id == id) {
        Some(event) => {
            // Find related events (same type and currency)
            let related: Vec<MarketEvent> = events
                .iter()
                .filter(|e| {
                    e.id != id && e.event_type == event.event_type && e.currency == event.currency
                })
                .take(5)
                .cloned()
                .collect();

            Json(EventDetailResponse {
                event: event.clone(),
                related_events: related,
            })
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "not_found",
                "message": format!("Event {} not found", id)
            })),
        )
            .into_response(),
    }
}

/// GET /api/events/types
/// List available event types.
pub async fn get_event_types() -> impl IntoResponse {
    let loader = EventsDataLoader::new();
    let events = loader.load_all_events();

    let types = vec![
        EventType::CentralBankMeeting,
        EventType::EconomicRelease,
        EventType::Holiday,
        EventType::News,
        EventType::Expiry,
        EventType::Other,
    ];

    let type_infos: Vec<EventTypeInfo> = types
        .into_iter()
        .map(|t| {
            let count = events.iter().filter(|e| e.event_type == t).count();
            EventTypeInfo {
                value: t,
                display_name: t.display_name().to_string(),
                icon: t.icon().to_string(),
                count,
            }
        })
        .collect();

    Json(EventTypesResponse { types: type_infos })
}

/// GET /api/events/central-banks
/// List central banks.
pub async fn get_central_banks_list() -> impl IntoResponse {
    let loader = EventsDataLoader::new();
    Json(CentralBanksResponse {
        central_banks: loader.load_central_banks(),
    })
}
