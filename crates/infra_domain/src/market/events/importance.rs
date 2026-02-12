//! Event importance level definitions.

use serde::{Deserialize, Serialize};

/// Importance level of a market event.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    strum::Display,
)]
#[serde(rename_all = "lowercase")]
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
