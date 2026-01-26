//! Events data types for the Market Data Viewer WebApp.
//!
//! Defines types for market events including:
//! - Central Bank meetings and policy decisions
//! - Economic data releases
//! - Market holidays
//! - News and announcements (future)

use serde::{Deserialize, Serialize};

// =============================================================================
// Event Types
// =============================================================================

/// Type of market event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Central Bank policy meeting
    CentralBankMeeting,
    /// Economic data release (GDP, CPI, NFP, etc.)
    EconomicRelease,
    /// Market holiday
    Holiday,
    /// Important news or announcement
    News,
    /// Options/Futures expiry
    Expiry,
    /// Other market event
    Other,
}

impl EventType {
    /// Get display name for the event type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CentralBankMeeting => "Central Bank Meeting",
            Self::EconomicRelease => "Economic Release",
            Self::Holiday => "Holiday",
            Self::News => "News",
            Self::Expiry => "Expiry",
            Self::Other => "Other",
        }
    }

    /// Get icon name for the event type.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::CentralBankMeeting => "fa-landmark",
            Self::EconomicRelease => "fa-chart-bar",
            Self::Holiday => "fa-calendar-times",
            Self::News => "fa-newspaper",
            Self::Expiry => "fa-hourglass-end",
            Self::Other => "fa-info-circle",
        }
    }
}

/// Importance level of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventImportance {
    /// Low importance
    Low,
    /// Medium importance
    Medium,
    /// High importance (market moving)
    High,
    /// Critical importance (major policy decision)
    Critical,
}

/// Central Bank identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CentralBank {
    /// Bank code (e.g., "FED", "ECB", "BOJ", "BOE")
    pub code: String,
    /// Full name
    pub name: String,
    /// Associated currency
    pub currency: String,
    /// Country or region
    pub region: String,
}

/// A market event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketEvent {
    /// Unique event ID
    pub id: String,
    /// Event type
    pub event_type: EventType,
    /// Event title/name
    pub title: String,
    /// Event description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Event date (ISO 8601 date: YYYY-MM-DD)
    pub date: String,
    /// Event time (optional, HH:MM format in local time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// Timezone (e.g., "America/New_York", "Europe/London")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// Associated currency (e.g., "USD", "EUR")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Associated region/country
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Importance level
    pub importance: EventImportance,
    /// Central bank info (for CB meetings)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub central_bank: Option<CentralBank>,
    /// Previous value (for economic releases)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<String>,
    /// Forecast/consensus value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forecast: Option<String>,
    /// Actual value (if released)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    /// Source of the event data
    pub source: String,
    /// Tags for filtering
    #[serde(default)]
    pub tags: Vec<String>,
}

// =============================================================================
// API Request/Response Types
// =============================================================================

/// Query parameters for events list.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsQuery {
    /// Filter by event type
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    /// Filter by currency
    pub currency: Option<String>,
    /// Filter by region
    pub region: Option<String>,
    /// Filter by importance
    pub importance: Option<String>,
    /// Start date (YYYY-MM-DD)
    pub start_date: Option<String>,
    /// End date (YYYY-MM-DD)
    pub end_date: Option<String>,
    /// Limit number of results
    pub limit: Option<usize>,
}

/// Response for events list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsResponse {
    /// List of events
    pub events: Vec<MarketEvent>,
    /// Total count (before pagination)
    pub total_count: usize,
    /// Last updated timestamp
    pub last_updated: i64,
}

/// Response for single event detail.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDetailResponse {
    /// The event
    pub event: MarketEvent,
    /// Related events (same type/currency)
    pub related_events: Vec<MarketEvent>,
}

/// Response for event types list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTypesResponse {
    /// Available event types
    pub types: Vec<EventTypeInfo>,
}

/// Information about an event type.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTypeInfo {
    /// Event type value
    pub value: EventType,
    /// Display name
    pub display_name: String,
    /// Icon class
    pub icon: String,
    /// Event count of this type
    pub count: usize,
}

/// Response for central banks list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralBanksResponse {
    /// List of central banks
    pub central_banks: Vec<CentralBank>,
}
