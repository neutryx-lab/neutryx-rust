//! Market event definitions.
//!
//! This module provides the core market event structure used for
//! economic calendar tracking and event-driven analysis.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{CentralBank, EventImportance, EventType};

/// A market event.
///
/// Represents a scheduled or historical market event such as
/// central bank meetings, economic data releases, or holidays.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct MarketEvent {
    /// Unique event ID.
    pub id: String,
    /// Event type classification.
    pub event_type: EventType,
    /// Event title/name.
    pub title: String,
    /// Event description (optional).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub description: Option<String>,
    /// Event date (ISO 8601 date: YYYY-MM-DD).
    pub date: String,
    /// Event time (optional, HH:MM format in local time).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub time: Option<String>,
    /// Timezone (e.g., "America/New_York", "Europe/London").
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub timezone: Option<String>,
    /// Associated currency (e.g., "USD", "EUR").
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub currency: Option<String>,
    /// Associated region/country.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub region: Option<String>,
    /// Importance level.
    pub importance: EventImportance,
    /// Central bank info (for CB meetings).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub central_bank: Option<CentralBank>,
    /// Previous value (for economic releases).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub previous: Option<String>,
    /// Forecast/consensus value.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub forecast: Option<String>,
    /// Actual value (if released).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub actual: Option<String>,
    /// Source of the event data.
    pub source: String,
    /// Tags for filtering.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tags: Vec<String>,
    /// Expected jump size in basis points for CB meeting events.
    ///
    /// Positive value indicates rate hike expectation.
    /// Range: -100.0 to +100.0 bps.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub expected_jump_bps: Option<f64>,
}

impl MarketEvent {
    /// Create a new market event with minimal required fields.
    pub fn new(
        id: impl Into<String>,
        event_type: EventType,
        title: impl Into<String>,
        date: impl Into<String>,
        importance: EventImportance,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            event_type,
            title: title.into(),
            description: None,
            date: date.into(),
            time: None,
            timezone: None,
            currency: None,
            region: None,
            importance,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: source.into(),
            tags: Vec::new(),
            expected_jump_bps: None,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the time.
    pub fn with_time(mut self, time: impl Into<String>) -> Self {
        self.time = Some(time.into());
        self
    }

    /// Set the timezone.
    pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = Some(timezone.into());
        self
    }

    /// Set the currency.
    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into());
        self
    }

    /// Set the region.
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the central bank.
    pub fn with_central_bank(mut self, central_bank: CentralBank) -> Self {
        self.central_bank = Some(central_bank);
        self
    }

    /// Set economic data values (previous, forecast, actual).
    pub fn with_economic_data(
        mut self,
        previous: Option<String>,
        forecast: Option<String>,
        actual: Option<String>,
    ) -> Self {
        self.previous = previous;
        self.forecast = forecast;
        self.actual = actual;
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set the expected jump size in basis points for CB meeting events.
    ///
    /// # Arguments
    ///
    /// * `bps` - Expected jump size in basis points (-100 to +100). Positive
    ///   indicates rate hike, negative indicates rate cut.
    pub fn with_expected_jump_bps(mut self, bps: f64) -> Self {
        self.expected_jump_bps = Some(bps);
        self
    }

    /// Get the expected jump size in basis points.
    ///
    /// Returns `None` if not set, or the expected jump in bps.
    pub fn expected_jump_bps(&self) -> Option<f64> { self.expected_jump_bps }

    /// Check if this event has an expected jump defined.
    pub fn has_expected_jump(&self) -> bool { self.expected_jump_bps.is_some() }

    /// Check if this is a central bank event.
    pub fn is_central_bank_event(&self) -> bool { self.event_type == EventType::CentralBankMeeting }

    /// Check if this is an economic release.
    pub fn is_economic_release(&self) -> bool { self.event_type == EventType::EconomicRelease }

    /// Check if this event has high importance or above.
    pub fn is_high_impact(&self) -> bool {
        matches!(
            self.importance,
            EventImportance::High | EventImportance::Critical
        )
    }
}

impl std::fmt::Display for MarketEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {} - {}", self.date, self.event_type, self.title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_event() {
        // new + basic fields
        let e = MarketEvent::new("EVT001", EventType::CentralBankMeeting, "FOMC Meeting", "2024-03-20", EventImportance::Critical, "Bloomberg");
        assert_eq!(e.id, "EVT001");
        assert_eq!(e.event_type, EventType::CentralBankMeeting);
        assert!(e.is_high_impact());

        // builder pattern
        let e2 = MarketEvent::new("EVT002", EventType::EconomicRelease, "US NFP", "2024-04-05", EventImportance::High, "Reuters")
            .with_currency("USD").with_region("United States").with_time("08:30").with_timezone("America/New_York")
            .with_economic_data(Some("200K".to_string()), Some("180K".to_string()), None);
        assert_eq!(e2.currency, Some("USD".to_string()));
        assert!(e2.is_economic_release());
        assert!(!e2.is_central_bank_event());

        // display
        let e3 = MarketEvent::new("EVT003", EventType::Holiday, "Christmas Day", "2024-12-25", EventImportance::Low, "Internal");
        let d = format!("{}", e3);
        assert!(d.contains("2024-12-25") && d.contains("Christmas Day"));

        // expected jump bps
        assert!(e.expected_jump_bps().is_none());
        assert!(!e.has_expected_jump());
        let ej = e.with_expected_jump_bps(25.0);
        assert_eq!(ej.expected_jump_bps(), Some(25.0));
        assert!(ej.has_expected_jump());

        // negative jump (rate cut)
        let cut = MarketEvent::new("ECB", EventType::CentralBankMeeting, "ECB", "2024-04-11", EventImportance::Critical, "Reuters")
            .with_expected_jump_bps(-25.0);
        assert_eq!(cut.expected_jump_bps(), Some(-25.0));

        // zero jump
        let zero = MarketEvent::new("BOJ", EventType::CentralBankMeeting, "BOJ", "2024-01-23", EventImportance::High, "Nikkei")
            .with_expected_jump_bps(0.0);
        assert_eq!(zero.expected_jump_bps(), Some(0.0));
        assert!(zero.has_expected_jump());
    }
}
