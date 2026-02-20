//! FX option quote type definitions for volatility surface construction.

use serde::{Deserialize, Serialize};

/// FX option volatility quote type.
///
/// Standard FX option market quotes follow the delta-space convention.
/// These types represent the standard instruments used to construct
/// FX volatility surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FxOptionQuoteType {
    /// ATM straddle (delta-neutral).
    AtmStraddle,
    /// 35-delta risk reversal.
    RiskReversal35D,
    /// 25-delta risk reversal.
    RiskReversal25D,
    /// 15-delta risk reversal.
    RiskReversal15D,
    /// 10-delta risk reversal.
    RiskReversal10D,
    /// 35-delta butterfly.
    Butterfly35D,
    /// 25-delta butterfly.
    Butterfly25D,
    /// 15-delta butterfly.
    Butterfly15D,
    /// 10-delta butterfly.
    Butterfly10D,
}

impl FxOptionQuoteType {
    /// Returns true if this is an ATM quote.
    #[must_use]
    pub fn is_atm(&self) -> bool { matches!(self, Self::AtmStraddle) }

    /// Returns true if this is a risk reversal.
    #[must_use]
    pub fn is_risk_reversal(&self) -> bool {
        matches!(
            self,
            Self::RiskReversal35D
                | Self::RiskReversal25D
                | Self::RiskReversal15D
                | Self::RiskReversal10D
        )
    }

    /// Returns true if this is a butterfly.
    #[must_use]
    pub fn is_butterfly(&self) -> bool {
        matches!(
            self,
            Self::Butterfly35D | Self::Butterfly25D | Self::Butterfly15D | Self::Butterfly10D
        )
    }

    /// Returns the delta value as a fraction (e.g., 0.25 for 25-delta).
    #[must_use]
    pub fn delta(&self) -> Option<f64> {
        match self {
            Self::AtmStraddle => None,
            Self::RiskReversal35D | Self::Butterfly35D => Some(0.35),
            Self::RiskReversal25D | Self::Butterfly25D => Some(0.25),
            Self::RiskReversal15D | Self::Butterfly15D => Some(0.15),
            Self::RiskReversal10D | Self::Butterfly10D => Some(0.10),
        }
    }

    /// Returns all quote types in standard ordering for surface construction.
    #[must_use]
    pub fn standard_set() -> &'static [Self] {
        &[
            Self::AtmStraddle,
            Self::RiskReversal25D,
            Self::Butterfly25D,
            Self::RiskReversal10D,
            Self::Butterfly10D,
        ]
    }

    /// Returns the full set of quote types including all deltas.
    #[must_use]
    pub fn full_set() -> &'static [Self] {
        &[
            Self::AtmStraddle,
            Self::RiskReversal35D,
            Self::Butterfly35D,
            Self::RiskReversal25D,
            Self::Butterfly25D,
            Self::RiskReversal15D,
            Self::Butterfly15D,
            Self::RiskReversal10D,
            Self::Butterfly10D,
        ]
    }
}

impl std::fmt::Display for FxOptionQuoteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AtmStraddle => write!(f, "ATM"),
            Self::RiskReversal35D => write!(f, "35D RR"),
            Self::RiskReversal25D => write!(f, "25D RR"),
            Self::RiskReversal15D => write!(f, "15D RR"),
            Self::RiskReversal10D => write!(f, "10D RR"),
            Self::Butterfly35D => write!(f, "35D BF"),
            Self::Butterfly25D => write!(f, "25D BF"),
            Self::Butterfly15D => write!(f, "15D BF"),
            Self::Butterfly10D => write!(f, "10D BF"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_atm() {
        assert!(FxOptionQuoteType::AtmStraddle.is_atm());
        assert!(!FxOptionQuoteType::RiskReversal25D.is_atm());
    }

    #[test]
    fn test_is_risk_reversal() {
        assert!(FxOptionQuoteType::RiskReversal25D.is_risk_reversal());
        assert!(FxOptionQuoteType::RiskReversal10D.is_risk_reversal());
        assert!(!FxOptionQuoteType::AtmStraddle.is_risk_reversal());
        assert!(!FxOptionQuoteType::Butterfly25D.is_risk_reversal());
    }

    #[test]
    fn test_is_butterfly() {
        assert!(FxOptionQuoteType::Butterfly25D.is_butterfly());
        assert!(FxOptionQuoteType::Butterfly10D.is_butterfly());
        assert!(!FxOptionQuoteType::AtmStraddle.is_butterfly());
        assert!(!FxOptionQuoteType::RiskReversal25D.is_butterfly());
    }

    #[test]
    fn test_delta() {
        assert_eq!(FxOptionQuoteType::AtmStraddle.delta(), None);
        assert_eq!(FxOptionQuoteType::RiskReversal25D.delta(), Some(0.25));
        assert_eq!(FxOptionQuoteType::Butterfly10D.delta(), Some(0.10));
        assert_eq!(FxOptionQuoteType::RiskReversal35D.delta(), Some(0.35));
    }

    #[test]
    fn test_standard_set() {
        let set = FxOptionQuoteType::standard_set();
        assert_eq!(set.len(), 5);
        assert_eq!(set[0], FxOptionQuoteType::AtmStraddle);
    }

    #[test]
    fn test_full_set() {
        let set = FxOptionQuoteType::full_set();
        assert_eq!(set.len(), 9);
    }

    #[test]
    fn test_display() {
        assert_eq!(FxOptionQuoteType::AtmStraddle.to_string(), "ATM");
        assert_eq!(FxOptionQuoteType::RiskReversal25D.to_string(), "25D RR");
        assert_eq!(FxOptionQuoteType::Butterfly25D.to_string(), "25D BF");
    }
}
