//! Strike type definitions.
//!
//! This module provides the classification of strike conventions
//! used in options and volatility surface specifications.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Strike convention for volatility quotes.
///
/// Defines how strike prices are expressed in volatility surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum StrikeType {
    /// Absolute strike rate (e.g., 2.5% = 0.025).
    #[default]
    Absolute,
    /// Relative to ATM forward (e.g., +50bp, -100bp).
    RelativeToAtm,
    /// Moneyness (K/F ratio).
    Moneyness,
    /// Log-moneyness (ln(K/F)).
    LogMoneyness,
    /// Delta (option delta as strike indicator).
    Delta,
}

impl StrikeType {
    /// Get the display name for this strike type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Absolute => "Absolute Strike",
            Self::RelativeToAtm => "Relative to ATM",
            Self::Moneyness => "Moneyness (K/F)",
            Self::LogMoneyness => "Log-Moneyness",
            Self::Delta => "Delta",
        }
    }

    /// Get a short code for this strike type.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Absolute => "ABS",
            Self::RelativeToAtm => "REL",
            Self::Moneyness => "MON",
            Self::LogMoneyness => "LOG",
            Self::Delta => "DLT",
        }
    }

    /// Check if this strike type requires a forward rate for conversion.
    pub fn requires_forward(&self) -> bool {
        matches!(
            self,
            Self::RelativeToAtm | Self::Moneyness | Self::LogMoneyness | Self::Delta
        )
    }

    /// Returns all strike type variants.
    pub fn all() -> &'static [StrikeType] {
        &[
            StrikeType::Absolute,
            StrikeType::RelativeToAtm,
            StrikeType::Moneyness,
            StrikeType::LogMoneyness,
            StrikeType::Delta,
        ]
    }
}

impl std::fmt::Display for StrikeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        assert_eq!(StrikeType::default(), StrikeType::Absolute);
    }

    #[test]
    fn test_display_name() {
        assert_eq!(StrikeType::Absolute.display_name(), "Absolute Strike");
        assert_eq!(StrikeType::Delta.display_name(), "Delta");
    }

    #[test]
    fn test_code() {
        assert_eq!(StrikeType::Absolute.code(), "ABS");
        assert_eq!(StrikeType::Moneyness.code(), "MON");
    }

    #[test]
    fn test_requires_forward() {
        assert!(!StrikeType::Absolute.requires_forward());
        assert!(StrikeType::RelativeToAtm.requires_forward());
        assert!(StrikeType::Moneyness.requires_forward());
        assert!(StrikeType::LogMoneyness.requires_forward());
        assert!(StrikeType::Delta.requires_forward());
    }

    #[test]
    fn test_all_variants() {
        let all = StrikeType::all();
        assert_eq!(all.len(), 5);
    }
}
