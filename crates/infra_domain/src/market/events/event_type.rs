//! Market event type definitions.

use serde::{Deserialize, Serialize};

/// Type of market event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Central Bank policy meeting (e.g., FOMC, ECB, BOJ).
    #[strum(serialize = "Central Bank Meeting")]
    CentralBankMeeting,
    /// Economic data release (GDP, CPI, NFP, etc.).
    #[strum(serialize = "Economic Release")]
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
    #[strum(serialize = "Turn of Year")]
    TurnOfYear,
    /// Turn of Quarter (TOQ) — quarter-end rate spike.
    #[strum(serialize = "Turn of Quarter")]
    TurnOfQuarter,
    /// Turn of Month (TOM) — month-end rate spike.
    #[strum(serialize = "Turn of Month")]
    TurnOfMonth,
    /// Other market event.
    Other,
}

impl EventType {

    /// Get the icon class name for this event type.
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_turn() {
        for t in [
            EventType::Turn,
            EventType::TurnOfYear,
            EventType::TurnOfQuarter,
            EventType::TurnOfMonth,
        ] {
            assert!(t.is_turn());
        }
        for t in [
            EventType::CentralBankMeeting,
            EventType::EconomicRelease,
            EventType::Holiday,
            EventType::News,
            EventType::Expiry,
            EventType::Other,
        ] {
            assert!(!t.is_turn());
        }
    }
}
