//! Market Data API handlers.
//!
//! This module provides REST API handlers for the Market Data Viewer,
//! including rates, conventions, and market events.
//!
//! # Task Coverage
//!
//! - Task 3.2: GET /api/market/rates - List market rates
//! - Task 3.3: GET /api/market/rates/:id - Get rate detail
//! - Task 3.4: GET /api/market/conventions - List conventions GET
//!   /api/market/conventions/:id - Get convention detail
//! - Task 3.5: Router integration
//!
//! # Endpoints
//!
//! ## Rates & Conventions
//!
//! | Method | Path                        | Description                    |
//! |--------|-----------------------------|--------------------------------|
//! | GET    | /api/market/indices         | Get all available indices      |
//! | GET    | /api/market/rates           | List all rates (filterable)    |
//! | GET    | /api/market/rates/:id       | Get rate detail with instrument|
//! | POST   | /api/market/rates/refresh   | Refresh rate data              |
//! | GET    | /api/market/conventions     | List all conventions           |
//! | GET    | /api/market/conventions/:id | Get convention detail          |
//! | GET    | /api/market/export/csv      | Export rates as CSV            |
//! | GET    | /api/market/export/json     | Export rates as JSON           |
//!
//! ## Events
//!
//! | Method | Path                          | Description                    |
//! |--------|-------------------------------|--------------------------------|
//! | GET    | /api/market/events            | List market events             |
//! | GET    | /api/market/events/:id        | Get event detail               |
//! | GET    | /api/market/events/types      | List event types               |
//! | GET    | /api/market/events/central-banks | List central banks          |

use std::{collections::HashMap, fmt::Write as _, path::PathBuf, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
// Re-export core event types from infra_master
pub use infra_master::market::events::{CentralBank, EventImportance, EventType, MarketEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info};

use crate::web::market_data::{get_config, get_convention, get_conventions_list, MarketDataCache};

// =============================================================================
// Market Data API DTO Types
// =============================================================================

/// Market rate API response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketRateResponse {
    pub id: String,
    pub currency: String,
    pub tenor: String,
    pub rate_type: String,
    pub value: f64,
    pub quote_type: String,
    pub timestamp: i64,
    pub source: String,
    pub is_stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
}

/// Market rates list response with metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketRatesListResponse {
    pub rates: Vec<MarketRateResponse>,
    pub last_updated: i64,
    pub total_count: usize,
}

/// Query parameters for market rate filtering.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MarketRateQuery {
    pub currency: Option<String>,
    pub rate_type: Option<String>,
    pub index: Option<String>,
}

/// Instrument response for rate detail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentResponse {
    pub instrument_type: String,
    pub currency: String,
    pub start_date: String,
    pub end_date: String,
    pub rate: f64,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Convention field (key/value pair).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConventionField {
    pub label: String,
    pub value: String,
}

/// Convention response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConventionResponse {
    pub convention_type: String,
    pub fields: Vec<ConventionField>,
}

/// Market rate detail response with instrument and convention.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketRateDetailResponse {
    pub rate: MarketRateResponse,
    pub instrument: Option<InstrumentResponse>,
    pub convention: Option<ConventionResponse>,
}

/// Convention summary for list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConventionSummary {
    pub id: String,
    pub currency: String,
    pub convention_type: String,
    pub is_default: bool,
}

/// Conventions list response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConventionsListResponse {
    pub conventions: Vec<ConventionSummary>,
}

/// API error response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataApiError {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl MarketDataApiError {
    pub fn not_found(resource: &str, id: &str) -> Self {
        Self {
            error: "not_found".to_string(),
            message: format!("{} with ID '{}' not found", resource, id),
            details: None,
        }
    }

    pub fn invalid_parameter(param: &str, reason: &str) -> Self {
        Self {
            error: "invalid_parameter".to_string(),
            message: format!("Invalid parameter '{}': {}", param, reason),
            details: None,
        }
    }

    pub fn internal(message: &str) -> Self {
        Self {
            error: "internal_error".to_string(),
            message: message.to_string(),
            details: None,
        }
    }
}

// =============================================================================
// Task 3.2: GET /api/market/rates - List Market Rates
// =============================================================================

/// List market rates with optional filtering.
///
/// # Query Parameters
///
/// - `currency`: Filter by currency code (e.g., "USD", "EUR")
/// - `rateType`: Filter by rate type (e.g., "Swap", "Ois", "Deposit")
/// - `index`: Filter by rate index (e.g., "SOFR", "EURIBOR")
///
/// # Returns
///
/// JSON array of market rates with metadata.
///
/// # Example
///
/// ```text
/// GET /api/market/rates?currency=USD&rateType=Swap
/// ```
pub async fn get_market_rates(
    State(cache): State<Arc<MarketDataCache>>,
    Query(query): Query<MarketRateQuery>,
) -> Json<MarketRatesListResponse> {
    let response = cache.get_rates(&query).await;
    Json(response)
}

// =============================================================================
// Task 3.3: GET /api/market/rates/:id - Get Rate Detail
// =============================================================================

/// Get detailed information for a specific rate.
///
/// Returns the rate with linked instrument and convention information.
///
/// # Path Parameters
///
/// - `id`: Rate identifier (e.g., "USD-5Y-SWAP")
///
/// # Returns
///
/// - 200: Rate detail with instrument and convention
/// - 404: Rate not found
///
/// # Example
///
/// ```text
/// GET /api/market/rates/USD-5Y-SWAP
/// ```
pub async fn get_market_rate_detail(
    State(cache): State<Arc<MarketDataCache>>,
    Path(id): Path<String>,
) -> Result<Json<MarketRateDetailResponse>, (StatusCode, Json<MarketDataApiError>)> {
    match cache.get_rate_detail(&id).await {
        Some(detail) => Ok(Json(detail)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(MarketDataApiError::not_found("Rate", &id)),
        )),
    }
}

// =============================================================================
// Task 5.1: POST /api/market/rates/refresh - Refresh Rate Data
// =============================================================================

/// Refresh market rate data.
///
/// Simulates rate refresh by applying small random perturbations
/// to all rates.
///
/// # Returns
///
/// - 200: Refresh successful with new timestamp
pub async fn refresh_market_rates(
    State(cache): State<Arc<MarketDataCache>>,
) -> Json<RefreshResponse> {
    cache.refresh().await;
    Json(RefreshResponse {
        success: true,
        last_updated: cache.last_updated(),
    })
}

/// Response for rate refresh operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    /// Whether refresh was successful.
    pub success: bool,
    /// New last_updated timestamp.
    pub last_updated: i64,
}

// =============================================================================
// Task 3.4: GET /api/market/conventions - List Conventions
// =============================================================================

/// List all available market conventions.
///
/// # Returns
///
/// JSON array of convention summaries.
pub async fn get_market_conventions() -> Json<ConventionsListResponse> {
    Json(get_conventions_list())
}

/// Get detailed information for a specific convention.
///
/// # Path Parameters
///
/// - `id`: Convention identifier (e.g., "USD-SOFR-OIS")
///
/// # Returns
///
/// - 200: Convention detail with all fields
/// - 404: Convention not found
pub async fn get_market_convention_detail(
    Path(id): Path<String>,
) -> Result<Json<ConventionResponse>, (StatusCode, Json<MarketDataApiError>)> {
    match get_convention(&id) {
        Some(convention) => Ok(Json(convention)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(MarketDataApiError::not_found("Convention", &id)),
        )),
    }
}

// =============================================================================
// GET /api/market/config - Market Data Configuration
// =============================================================================

/// Response for market configuration endpoint.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketConfigResponse {
    /// Tenor codes in canonical order (from INFRA_MASTER).
    pub tenor_order: Vec<&'static str>,
}

/// Get market data configuration.
///
/// Returns configuration values used by the frontend, including the
/// canonical tenor ordering from `infra_master::time::Tenor`.
///
/// # Returns
///
/// JSON object with configuration values.
pub async fn get_market_config() -> Json<MarketConfigResponse> {
    Json(MarketConfigResponse {
        tenor_order: infra_master::time::Tenor::all_codes().to_vec(),
    })
}

// =============================================================================
// GET /api/market/indices - Get all available indices
// =============================================================================

/// Path to the indices JSON file.
const INDICES_FILE: &str = "demo/data/input/indices.json";

/// Get all available market data indices.
///
/// Returns the master index of all available market data categorised by type
/// (rates, fx, irvol, fxvol, events).
///
/// # Returns
///
/// JSON object with all available indices.
pub async fn get_indices() -> Result<Json<serde_json::Value>, (StatusCode, Json<MarketDataApiError>)>
{
    match std::fs::read_to_string(INDICES_FILE) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(data) => Ok(Json(data)),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(MarketDataApiError::internal(&format!(
                    "Failed to parse indices file: {}",
                    e
                ))),
            )),
        },
        Err(_) => Err((
            StatusCode::NOT_FOUND,
            Json(MarketDataApiError::not_found("indices", "indices.json")),
        )),
    }
}

// =============================================================================
// Task 6.1: GET /api/market/export/csv - Export as CSV
// =============================================================================

/// Export market rates as CSV.
///
/// # Query Parameters
///
/// Same as GET /api/market/rates
///
/// # Returns
///
/// CSV file with Content-Type: text/csv
pub async fn export_rates_csv(
    State(cache): State<Arc<MarketDataCache>>,
    Query(query): Query<MarketRateQuery>,
) -> impl IntoResponse {
    let response = cache.get_rates(&query).await;

    // Build CSV content
    let mut csv = String::from(
        "id,currency,tenor,rateType,value,quoteType,timestamp,source,isStale,rateIndex\n",
    );

    for rate in &response.rates {
        let _ = writeln!(
            csv,
            "{},{},{},{},{:.6},{},{},{},{},{}",
            rate.id,
            rate.currency,
            rate.tenor,
            rate.rate_type,
            rate.value,
            rate.quote_type,
            rate.timestamp,
            rate.source,
            rate.is_stale,
            rate.rate_index.as_deref().unwrap_or("")
        );
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv; charset=utf-8")
        .header(
            "Content-Disposition",
            "attachment; filename=\"market_rates.csv\"",
        )
        .body(csv)
        .unwrap()
}

// =============================================================================
// Task 6.2: GET /api/market/export/json - Export as JSON
// =============================================================================

/// Export market rates as JSON file.
///
/// # Query Parameters
///
/// Same as GET /api/market/rates
///
/// # Returns
///
/// JSON file with Content-Disposition header for download
pub async fn export_rates_json(
    State(cache): State<Arc<MarketDataCache>>,
    Query(query): Query<MarketRateQuery>,
) -> impl IntoResponse {
    let response = cache.get_rates(&query).await;
    let json = serde_json::to_string_pretty(&response).unwrap_or_default();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json; charset=utf-8")
        .header(
            "Content-Disposition",
            "attachment; filename=\"market_rates.json\"",
        )
        .body(json)
        .unwrap()
}

// =============================================================================
// Events API Types and Handlers
// =============================================================================

/// Default path to the events data directory (fallback).
const DEFAULT_EVENTS_DIR: &str = "demo/data/input/events";

/// Query parameters for events list.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsQuery {
    /// Filter by event type.
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    /// Filter by currency.
    pub currency: Option<String>,
    /// Filter by region.
    pub region: Option<String>,
    /// Filter by importance.
    pub importance: Option<String>,
    /// Start date (YYYY-MM-DD).
    pub start_date: Option<String>,
    /// End date (YYYY-MM-DD).
    pub end_date: Option<String>,
    /// Limit number of results.
    pub limit: Option<usize>,
}

/// Response for events list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsResponse {
    /// List of events.
    pub events: Vec<MarketEvent>,
    /// Total count (before pagination).
    pub total_count: usize,
    /// Last updated timestamp.
    pub last_updated: i64,
}

/// Response for single event detail.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDetailResponse {
    /// The event.
    pub event: MarketEvent,
    /// Related events (same type/currency).
    pub related_events: Vec<MarketEvent>,
}

/// Response for event types list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTypesResponse {
    /// Available event types.
    pub types: Vec<EventTypeInfo>,
}

/// Information about an event type.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTypeInfo {
    /// Event type value.
    pub value: EventType,
    /// Display name.
    pub display_name: String,
    /// Icon class.
    pub icon: String,
    /// Event count of this type.
    pub count: usize,
}

/// Response for central banks list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralBanksResponse {
    /// List of central banks.
    pub central_banks: Vec<CentralBank>,
}

// =============================================================================
// Events Data Loading
// =============================================================================

/// Configuration for loading events data from files.
///
/// Uses paths from the shared `market_data_config.json` configuration file.
#[allow(clippy::struct_field_names)]
pub struct EventsDataLoader {
    /// Path to central_banks.json.
    central_banks_path: PathBuf,
    /// Path to central_bank_meetings.json.
    cb_meetings_path: PathBuf,
    /// Path to economic_releases.json.
    economic_releases_path: PathBuf,
    /// Path to holidays.json.
    holidays_path: PathBuf,
}

impl EventsDataLoader {
    /// Create a new data loader using paths from the shared config.
    pub fn new() -> Self {
        let config = get_config();
        let default_dir = PathBuf::from(DEFAULT_EVENTS_DIR);

        // Use config paths if available, otherwise fall back to defaults
        let events = config.paths.events.unwrap_or_default();

        Self {
            central_banks_path: events
                .central_banks
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.join("central_banks.json")),
            cb_meetings_path: events
                .central_bank_meetings
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.join("central_bank_meetings.json")),
            economic_releases_path: events
                .economic_releases
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.join("economic_releases.json")),
            holidays_path: events
                .holidays
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.join("holidays.json")),
        }
    }

    /// Load central banks list from file.
    pub fn load_central_banks(&self) -> Vec<CentralBank> {
        let path = &self.central_banks_path;
        match std::fs::read_to_string(path) {
            Ok(content) => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct CentralBanksFile {
                    central_banks: Vec<CentralBank>,
                }
                match serde_json::from_str::<CentralBanksFile>(&content) {
                    Ok(data) => {
                        info!(
                            "Loaded {} central banks from {:?}",
                            data.central_banks.len(),
                            path
                        );
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

    /// Load events from a specific file path.
    fn load_events_from_path(&self, path: &std::path::Path) -> Vec<MarketEvent> {
        match std::fs::read_to_string(path) {
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
                        error!("Failed to parse {:?}: {}", path, e);
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                error!("Failed to read {:?}: {}", path, e);
                Vec::new()
            }
        }
    }

    /// Load all events from all data files.
    pub fn load_all_events(&self) -> Vec<MarketEvent> {
        let mut all_events = Vec::new();

        // Load events from each category file using config paths
        all_events.extend(self.load_events_from_path(&self.cb_meetings_path));
        all_events.extend(self.load_events_from_path(&self.economic_releases_path));
        all_events.extend(self.load_events_from_path(&self.holidays_path));

        // Sort by date
        all_events.sort_by(|a, b| a.date.cmp(&b.date));

        info!("Loaded {} total events", all_events.len());
        all_events
    }
}

impl Default for EventsDataLoader {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// Events API Handlers
// =============================================================================

/// GET /api/market/events
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

/// GET /api/market/events/{id}
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

/// GET /api/market/events/types
/// List available event types.
pub async fn get_event_types() -> impl IntoResponse {
    let loader = EventsDataLoader::new();
    let events = loader.load_all_events();

    let type_infos: Vec<EventTypeInfo> = EventType::all()
        .iter()
        .map(|&t| {
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

/// GET /api/market/events/central-banks
/// List central banks.
pub async fn get_central_banks_list() -> impl IntoResponse {
    let loader = EventsDataLoader::new();
    Json(CentralBanksResponse {
        central_banks: loader.load_central_banks(),
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod handler_tests {
        use super::*;

        #[tokio::test]
        async fn test_get_market_rates_returns_rates() {
            let cache = Arc::new(MarketDataCache::new());
            let query = MarketRateQuery::default();

            let Json(response) = get_market_rates(State(cache), Query(query)).await;

            assert!(response.total_count > 0);
            assert!(!response.rates.is_empty());
        }

        #[tokio::test]
        async fn test_get_market_rates_with_currency_filter() {
            let cache = Arc::new(MarketDataCache::new());
            let query = MarketRateQuery {
                currency: Some("USD".to_string()),
                rate_type: None,
                index: None,
            };

            let Json(response) = get_market_rates(State(cache), Query(query)).await;

            for rate in &response.rates {
                assert_eq!(rate.currency, "USD");
            }
        }

        #[tokio::test]
        async fn test_get_market_rate_detail_found() {
            let cache = Arc::new(MarketDataCache::new());

            let result =
                get_market_rate_detail(State(cache), Path("USD-5Y-SWAP".to_string())).await;

            assert!(result.is_ok());
            let Json(detail) = result.unwrap();
            assert_eq!(detail.rate.id, "USD-5Y-SWAP");
            assert!(detail.instrument.is_some());
            assert!(detail.convention.is_some());
        }

        #[tokio::test]
        async fn test_get_market_rate_detail_not_found() {
            let cache = Arc::new(MarketDataCache::new());

            let result =
                get_market_rate_detail(State(cache), Path("INVALID-RATE".to_string())).await;

            assert!(result.is_err());
            let (status, Json(error)) = result.unwrap_err();
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert_eq!(error.error, "not_found");
        }

        #[tokio::test]
        async fn test_refresh_market_rates() {
            let cache = Arc::new(MarketDataCache::new());
            let initial_ts = cache.last_updated();

            // Wait a bit
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            let Json(response) = refresh_market_rates(State(cache)).await;

            assert!(response.success);
            assert!(response.last_updated > initial_ts);
        }

        #[tokio::test]
        async fn test_get_market_conventions() {
            let Json(response) = get_market_conventions().await;

            assert!(!response.conventions.is_empty());
        }

        #[tokio::test]
        async fn test_get_market_convention_detail_found() {
            let result = get_market_convention_detail(Path("USD-SOFR-OIS".to_string())).await;

            assert!(result.is_ok());
            let Json(convention) = result.unwrap();
            assert_eq!(convention.convention_type, "OisConvention");
        }

        #[tokio::test]
        async fn test_get_market_convention_detail_not_found() {
            let result = get_market_convention_detail(Path("INVALID-CONVENTION".to_string())).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::NOT_FOUND);
        }

        #[tokio::test]
        async fn test_export_rates_csv() {
            let cache = Arc::new(MarketDataCache::new());
            let query = MarketRateQuery {
                currency: Some("USD".to_string()),
                rate_type: Some("Deposit".to_string()),
                index: None,
            };

            let response = export_rates_csv(State(cache), Query(query))
                .await
                .into_response();

            assert_eq!(response.status(), StatusCode::OK);

            // Check content-type header
            let content_type = response.headers().get("content-type");
            assert!(content_type.is_some());
            assert!(content_type.unwrap().to_str().unwrap().contains("text/csv"));
        }

        #[tokio::test]
        async fn test_export_rates_json() {
            let cache = Arc::new(MarketDataCache::new());
            let query = MarketRateQuery::default();

            let response = export_rates_json(State(cache), Query(query))
                .await
                .into_response();

            assert_eq!(response.status(), StatusCode::OK);

            let content_type = response.headers().get("content-type");
            assert!(content_type.is_some());
            assert!(content_type
                .unwrap()
                .to_str()
                .unwrap()
                .contains("application/json"));
        }
    }
}
