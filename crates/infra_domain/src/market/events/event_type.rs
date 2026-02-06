//! Market event type definitions.
//!
//! This module provides the classification of market events used for
//! calendar awareness and economic data tracking.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Type of market event.
///
/// Categorises events that may impact market conditions or trading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum EventType {
    /// Central Bank policy meeting (e.g., FOMC, ECB, BOJ).
    CentralBankMeeting,
    /// Economic data release (GDP, CPI, NFP, etc.).
    EconomicRelease,
    /// Market holiday.
    Holiday,
    /// Important news or announcement.
    News,
    /// Options/Futures expiry.
    Expiry,
    /// Other market event.
    Other,
}

impl EventType {
    /// Get the display name for this event type.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::events::EventType;
    ///
    /// assert_eq!(EventType::CentralBankMeeting.display_name(), "Central Bank Meeting");
    /// ```
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

    /// Get the icon class name for this event type.
    ///
    /// Returns Font Awesome icon class names suitable for UI rendering.
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

    /// Returns all event type variants.
    pub fn all() -> &'static [EventType] {
        &[
            EventType::CentralBankMeeting,
            EventType::EconomicRelease,
            EventType::Holiday,
            EventType::News,
            EventType::Expiry,
            EventType::Other,
        ]
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_name() {
        assert_eq!(
            EventType::CentralBankMeeting.display_name(),
            "Central Bank Meeting"
        );
        assert_eq!(
            EventType::EconomicRelease.display_name(),
            "Economic Release"
        );
        assert_eq!(EventType::Holiday.display_name(), "Holiday");
    }

    #[test]
    fn test_icon() {
        assert_eq!(EventType::CentralBankMeeting.icon(), "fa-landmark");
        assert_eq!(EventType::EconomicRelease.icon(), "fa-chart-bar");
    }

    #[test]
    fn test_all_variants() {
        let all = EventType::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_display_trait() {
        assert_eq!(format!("{}", EventType::Holiday), "Holiday");
    }
}
