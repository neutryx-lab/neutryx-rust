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
    /// Turn date for curve construction (generic turn).
    Turn,
    /// Turn of Year (TOY) — year-end rate spike.
    TurnOfYear,
    /// Turn of Quarter (TOQ) — quarter-end rate spike.
    TurnOfQuarter,
    /// Turn of Month (TOM) — month-end rate spike.
    TurnOfMonth,
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
            Self::Turn => "Turn",
            Self::TurnOfYear => "Turn of Year",
            Self::TurnOfQuarter => "Turn of Quarter",
            Self::TurnOfMonth => "Turn of Month",
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
            Self::Turn | Self::TurnOfYear | Self::TurnOfQuarter | Self::TurnOfMonth => {
                "fa-chart-line"
            }
            Self::Other => "fa-info-circle",
        }
    }

    /// Returns true if this is any kind of turn event.
    pub fn is_turn(&self) -> bool {
        matches!(
            self,
            Self::Turn | Self::TurnOfYear | Self::TurnOfQuarter | Self::TurnOfMonth
        )
    }

    /// Returns all event type variants.
    pub fn all() -> &'static [EventType] {
        &[
            EventType::CentralBankMeeting,
            EventType::EconomicRelease,
            EventType::Holiday,
            EventType::News,
            EventType::Expiry,
            EventType::Turn,
            EventType::TurnOfYear,
            EventType::TurnOfQuarter,
            EventType::TurnOfMonth,
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
    fn test_event_type() {
        // display_name
        assert_eq!(EventType::CentralBankMeeting.display_name(), "Central Bank Meeting");
        assert_eq!(EventType::EconomicRelease.display_name(), "Economic Release");
        assert_eq!(EventType::Holiday.display_name(), "Holiday");
        assert_eq!(EventType::TurnOfYear.display_name(), "Turn of Year");
        assert_eq!(EventType::TurnOfQuarter.display_name(), "Turn of Quarter");
        assert_eq!(EventType::TurnOfMonth.display_name(), "Turn of Month");

        // icon
        assert_eq!(EventType::CentralBankMeeting.icon(), "fa-landmark");
        assert_eq!(EventType::EconomicRelease.icon(), "fa-chart-bar");

        // all variants
        assert_eq!(EventType::all().len(), 10);

        // Display trait
        assert_eq!(format!("{}", EventType::Holiday), "Holiday");

        // is_turn
        for t in [EventType::Turn, EventType::TurnOfYear, EventType::TurnOfQuarter, EventType::TurnOfMonth] {
            assert!(t.is_turn());
        }
        for t in [EventType::CentralBankMeeting, EventType::EconomicRelease, EventType::Other] {
            assert!(!t.is_turn());
        }
    }
}
