//! Event importance level definitions.
//!
//! This module provides importance classification for market events,
//! indicating their potential impact on market conditions.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Importance level of a market event.
///
/// Indicates the expected market impact of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum EventImportance {
    /// Low importance - minimal market impact expected.
    Low,
    /// Medium importance - moderate market impact possible.
    #[default]
    Medium,
    /// High importance - significant market moving potential.
    High,
    /// Critical importance - major policy decision or key data release.
    Critical,
}

impl EventImportance {
    /// Get the display name for this importance level.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }

    /// Get the CSS class suffix for styling.
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    /// Returns all importance variants in order.
    pub fn all() -> &'static [EventImportance] {
        &[
            EventImportance::Low,
            EventImportance::Medium,
            EventImportance::High,
            EventImportance::Critical,
        ]
    }
}

impl std::fmt::Display for EventImportance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordering() {
        assert!(EventImportance::Low < EventImportance::Medium);
        assert!(EventImportance::Medium < EventImportance::High);
        assert!(EventImportance::High < EventImportance::Critical);
    }
}
