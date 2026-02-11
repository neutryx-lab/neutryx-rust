//! Risk factor identification for sensitivity analysis.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Unique identifier for a risk factor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskFactorId {
    /// Underlying asset identifier (e.g., "USDJPY", "SPX").
    Underlying(String),

    /// Yield curve identifier (e.g., "USD-OIS", "JPY-LIBOR").
    Curve(String),

    /// Volatility surface identifier (e.g., "SPX-Vol", "USDJPY-Vol").
    VolSurface(String),
}

impl RiskFactorId {
    /// Creates a new underlying risk factor.
    #[inline]
    pub fn underlying(name: impl Into<String>) -> Self { Self::Underlying(name.into()) }

    /// Creates a new curve risk factor.
    #[inline]
    pub fn curve(name: impl Into<String>) -> Self { Self::Curve(name.into()) }

    /// Creates a new volatility surface risk factor.
    #[inline]
    pub fn vol_surface(name: impl Into<String>) -> Self { Self::VolSurface(name.into()) }

    /// Returns the risk factor type as a string.
    #[inline]
    pub fn factor_type(&self) -> &'static str {
        match self {
            Self::Underlying(_) => "Underlying",
            Self::Curve(_) => "Curve",
            Self::VolSurface(_) => "VolSurface",
        }
    }

    /// Returns the risk factor name (identifier string).
    #[inline]
    pub fn name(&self) -> &str {
        match self {
            Self::Underlying(name) | Self::Curve(name) | Self::VolSurface(name) => name,
        }
    }

    /// Returns true if this is an underlying risk factor.
    #[inline]
    pub fn is_underlying(&self) -> bool { matches!(self, Self::Underlying(_)) }

    /// Returns true if this is a curve risk factor.
    #[inline]
    pub fn is_curve(&self) -> bool { matches!(self, Self::Curve(_)) }

    /// Returns true if this is a volatility surface risk factor.
    #[inline]
    pub fn is_vol_surface(&self) -> bool { matches!(self, Self::VolSurface(_)) }
}

impl fmt::Display for RiskFactorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Underlying(name) => write!(f, "Underlying:{}", name),
            Self::Curve(name) => write!(f, "Curve:{}", name),
            Self::VolSurface(name) => write!(f, "VolSurface:{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_risk_factor_id_underlying_construction() {
        let factor = RiskFactorId::underlying("SPX");
        assert_eq!(factor, RiskFactorId::Underlying("SPX".to_string()));
    }

    #[test]
    fn test_risk_factor_id_curve_construction() {
        let factor = RiskFactorId::curve("USD-OIS");
        assert_eq!(factor, RiskFactorId::Curve("USD-OIS".to_string()));
    }

    #[test]
    fn test_risk_factor_id_vol_surface_construction() {
        let factor = RiskFactorId::vol_surface("SPX-Vol");
        assert_eq!(factor, RiskFactorId::VolSurface("SPX-Vol".to_string()));
    }

    #[test]
    fn test_risk_factor_id_display_underlying() {
        let factor = RiskFactorId::underlying("USDJPY");
        assert_eq!(format!("{}", factor), "Underlying:USDJPY");
    }

    #[test]
    fn test_risk_factor_id_display_curve() {
        let factor = RiskFactorId::curve("JPY-LIBOR");
        assert_eq!(format!("{}", factor), "Curve:JPY-LIBOR");
    }

    #[test]
    fn test_risk_factor_id_display_vol_surface() {
        let factor = RiskFactorId::vol_surface("USDJPY-Vol");
        assert_eq!(format!("{}", factor), "VolSurface:USDJPY-Vol");
    }

    #[test]
    fn test_risk_factor_id_hash_no_collision() {
        let mut set = HashSet::new();

        let factors = vec![
            RiskFactorId::underlying("SPX"),
            RiskFactorId::underlying("USDJPY"),
            RiskFactorId::curve("USD-OIS"),
            RiskFactorId::curve("JPY-LIBOR"),
            RiskFactorId::vol_surface("SPX-Vol"),
            RiskFactorId::vol_surface("USDJPY-Vol"),
        ];

        for factor in &factors {
            assert!(set.insert(factor.clone()), "Hash collision detected");
        }

        assert_eq!(set.len(), 6);
    }

    #[test]
    fn test_risk_factor_id_equality() {
        let f1 = RiskFactorId::underlying("SPX");
        let f2 = RiskFactorId::underlying("SPX");
        let f3 = RiskFactorId::underlying("AAPL");
        let f4 = RiskFactorId::curve("SPX");

        assert_eq!(f1, f2);
        assert_ne!(f1, f3);
        assert_ne!(f1, f4);
    }

    #[test]
    fn test_risk_factor_id_clone() {
        let original = RiskFactorId::curve("EUR-EURIBOR");
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_risk_factor_id_factor_type() {
        assert_eq!(RiskFactorId::underlying("SPX").factor_type(), "Underlying");
        assert_eq!(RiskFactorId::curve("USD-OIS").factor_type(), "Curve");
        assert_eq!(
            RiskFactorId::vol_surface("SPX-Vol").factor_type(),
            "VolSurface"
        );
    }

    #[test]
    fn test_risk_factor_id_name() {
        assert_eq!(RiskFactorId::underlying("SPX").name(), "SPX");
        assert_eq!(RiskFactorId::curve("USD-OIS").name(), "USD-OIS");
        assert_eq!(RiskFactorId::vol_surface("SPX-Vol").name(), "SPX-Vol");
    }

    #[test]
    fn test_risk_factor_id_type_checks() {
        let underlying = RiskFactorId::underlying("SPX");
        let curve = RiskFactorId::curve("USD-OIS");
        let vol = RiskFactorId::vol_surface("SPX-Vol");

        assert!(underlying.is_underlying());
        assert!(!underlying.is_curve());
        assert!(!underlying.is_vol_surface());

        assert!(!curve.is_underlying());
        assert!(curve.is_curve());
        assert!(!curve.is_vol_surface());

        assert!(!vol.is_underlying());
        assert!(!vol.is_curve());
        assert!(vol.is_vol_surface());
    }

    #[test]
    fn test_risk_factor_id_hashmap_key() {
        use std::collections::HashMap;

        let mut map: HashMap<RiskFactorId, f64> = HashMap::new();
        map.insert(RiskFactorId::underlying("SPX"), 0.5);
        map.insert(RiskFactorId::curve("USD-OIS"), 0.01);
        map.insert(RiskFactorId::vol_surface("SPX-Vol"), 0.2);

        assert_eq!(map.get(&RiskFactorId::underlying("SPX")), Some(&0.5));
        assert_eq!(map.get(&RiskFactorId::curve("USD-OIS")), Some(&0.01));
        assert_eq!(map.get(&RiskFactorId::vol_surface("SPX-Vol")), Some(&0.2));
        assert_eq!(map.get(&RiskFactorId::underlying("AAPL")), None);
    }

    #[test]
    fn test_risk_factor_id_debug() {
        let factor = RiskFactorId::underlying("SPX");
        let debug_str = format!("{:?}", factor);
        assert!(debug_str.contains("Underlying"));
        assert!(debug_str.contains("SPX"));
    }

    mod serde_tests {
        use super::*;

        #[test]
        fn test_risk_factor_id_serialize_deserialize() {
            let original = RiskFactorId::underlying("SPX");
            let json = serde_json::to_string(&original).unwrap();
            let deserialized: RiskFactorId = serde_json::from_str(&json).unwrap();
            assert_eq!(original, deserialized);
        }

        #[test]
        fn test_risk_factor_id_all_variants_serde() {
            let factors = vec![
                RiskFactorId::underlying("SPX"),
                RiskFactorId::curve("USD-OIS"),
                RiskFactorId::vol_surface("SPX-Vol"),
            ];

            for factor in factors {
                let json = serde_json::to_string(&factor).unwrap();
                let deserialized: RiskFactorId = serde_json::from_str(&json).unwrap();
                assert_eq!(factor, deserialized);
            }
        }
    }
}
