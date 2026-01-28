//! Volatility quote type definitions.
//!
//! This module provides the classification of volatility quoting conventions
//! used in options and swaption markets.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Volatility quote type for options and swaptions.
///
/// Defines how volatility is expressed in market quotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum VolQuoteType {
    /// Normal (Bachelier) volatility in basis points.
    ///
    /// Used primarily for rates markets, especially when rates can be negative.
    #[default]
    Normal,
    /// Lognormal (Black) volatility in percentage.
    ///
    /// Traditional Black-Scholes implied volatility.
    Lognormal,
    /// Shifted Lognormal volatility.
    ///
    /// Lognormal volatility with a shift parameter to handle negative rates.
    ShiftedLognormal,
}

impl VolQuoteType {
    /// Get the display name for this volatility type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal (bp)",
            Self::Lognormal => "Lognormal (%)",
            Self::ShiftedLognormal => "Shifted Lognormal",
        }
    }

    /// Get the unit string for display.
    pub fn unit(&self) -> &'static str {
        match self {
            Self::Normal => "bp",
            Self::Lognormal => "%",
            Self::ShiftedLognormal => "%",
        }
    }

    /// Check if this type uses percentage units.
    pub fn is_percentage(&self) -> bool {
        matches!(self, Self::Lognormal | Self::ShiftedLognormal)
    }

    /// Check if this type requires a shift parameter.
    pub fn requires_shift(&self) -> bool {
        matches!(self, Self::ShiftedLognormal)
    }

    /// Returns all volatility quote type variants.
    pub fn all() -> &'static [VolQuoteType] {
        &[
            VolQuoteType::Normal,
            VolQuoteType::Lognormal,
            VolQuoteType::ShiftedLognormal,
        ]
    }
}

impl std::fmt::Display for VolQuoteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        assert_eq!(VolQuoteType::default(), VolQuoteType::Normal);
    }

    #[test]
    fn test_display_name() {
        assert_eq!(VolQuoteType::Normal.display_name(), "Normal (bp)");
        assert_eq!(VolQuoteType::Lognormal.display_name(), "Lognormal (%)");
    }

    #[test]
    fn test_unit() {
        assert_eq!(VolQuoteType::Normal.unit(), "bp");
        assert_eq!(VolQuoteType::Lognormal.unit(), "%");
    }

    #[test]
    fn test_is_percentage() {
        assert!(!VolQuoteType::Normal.is_percentage());
        assert!(VolQuoteType::Lognormal.is_percentage());
        assert!(VolQuoteType::ShiftedLognormal.is_percentage());
    }

    #[test]
    fn test_requires_shift() {
        assert!(!VolQuoteType::Normal.requires_shift());
        assert!(!VolQuoteType::Lognormal.requires_shift());
        assert!(VolQuoteType::ShiftedLognormal.requires_shift());
    }
}
