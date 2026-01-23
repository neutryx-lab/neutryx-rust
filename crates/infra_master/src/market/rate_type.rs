//! Market rate type classification.
//!
//! This module provides the [`RateType`] enum for classifying different
//! types of market rates used in curve calibration and pricing.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::RateType;
//!
//! let rate_type = RateType::Swap;
//! assert_eq!(rate_type.code(), "SWAP");
//! ```

use std::fmt;

/// Classification of market rate types grouped by asset class.
///
/// Represents the different types of market rates that can be used
/// for curve calibration and instrument mapping.
///
/// # Ordering Rationale
///
/// Variants are grouped by asset class following standard market convention:
///
/// 1. **Interest Rate instruments** (core curve building inputs):
///    - `Deposit` - Money market, short-dated
///    - `Fra` - Forward rates, short to medium
///    - `Futures` - Exchange-traded, short to medium
///    - `Swap` - Medium to long-dated
///    - `Ois` - Overnight compounding, all tenors
///    - `BasisSwap` - Multi-curve framework
///
/// 2. **FX instruments** (secondary):
///    - `FxSpot` - Spot exchange rates
///    - `FxForward` - Forward exchange rates
///
/// 3. **Volatility** (tertiary):
///    - `Vol` - Implied volatility quotes
///
/// # Adding New Variants
///
/// When adding new rate types, place them within the appropriate asset class
/// group. Within interest rates, order by typical instrument maturity.
///
/// # Examples
///
/// ```
/// use infra_master::market::RateType;
///
/// let swap = RateType::Swap;
/// assert_eq!(swap.code(), "SWAP");
/// assert!(swap.is_interest_rate());
///
/// let fx = RateType::FxSpot;
/// assert!(fx.is_fx());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum RateType {
    // === Interest Rate instruments (ordered by typical maturity) ===
    /// Money market deposit rate (short-dated, O/N to 1Y).
    Deposit,
    /// Forward rate agreement (short to medium, 1M to 2Y).
    Fra,
    /// Interest rate futures (exchange-traded, 3M to 2Y).
    Futures,
    /// Vanilla interest rate swap (medium to long, 1Y to 50Y).
    Swap,
    /// Overnight index swap (all tenors, primary discounting).
    Ois,
    /// Basis swap (two floating legs, multi-curve framework).
    BasisSwap,

    // === FX instruments ===
    /// FX spot rate.
    FxSpot,
    /// FX forward rate.
    FxForward,

    // === Volatility ===
    /// Volatility quote (implied vol for options).
    Vol,
}

impl RateType {
    /// Returns a short code for this rate type.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::RateType;
    ///
    /// assert_eq!(RateType::Deposit.code(), "DEPO");
    /// assert_eq!(RateType::Swap.code(), "SWAP");
    /// assert_eq!(RateType::Ois.code(), "OIS");
    /// ```
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            RateType::Deposit => "DEPO",
            RateType::Fra => "FRA",
            RateType::Futures => "FUT",
            RateType::Swap => "SWAP",
            RateType::Ois => "OIS",
            RateType::BasisSwap => "BASIS",
            RateType::FxSpot => "FXSPOT",
            RateType::FxForward => "FXFWD",
            RateType::Vol => "VOL",
        }
    }

    /// Returns true if this is an interest rate type.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::RateType;
    ///
    /// assert!(RateType::Deposit.is_interest_rate());
    /// assert!(RateType::Swap.is_interest_rate());
    /// assert!(!RateType::FxSpot.is_interest_rate());
    /// ```
    #[must_use]
    pub const fn is_interest_rate(&self) -> bool {
        matches!(
            self,
            RateType::Deposit
                | RateType::Fra
                | RateType::Futures
                | RateType::Swap
                | RateType::Ois
                | RateType::BasisSwap
        )
    }

    /// Returns true if this is an FX rate type.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::RateType;
    ///
    /// assert!(RateType::FxSpot.is_fx());
    /// assert!(RateType::FxForward.is_fx());
    /// assert!(!RateType::Swap.is_fx());
    /// ```
    #[must_use]
    pub const fn is_fx(&self) -> bool { matches!(self, RateType::FxSpot | RateType::FxForward) }

    /// Returns true if this is a volatility quote.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::RateType;
    ///
    /// assert!(RateType::Vol.is_volatility());
    /// assert!(!RateType::Swap.is_volatility());
    /// ```
    #[must_use]
    pub const fn is_volatility(&self) -> bool { matches!(self, RateType::Vol) }
}

impl fmt::Display for RateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.code()) }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[test]
    fn test_rate_type_variants() {
        let all_types = [
            RateType::Deposit,
            RateType::Fra,
            RateType::Futures,
            RateType::Swap,
            RateType::Ois,
            RateType::BasisSwap,
            RateType::FxSpot,
            RateType::FxForward,
            RateType::Vol,
        ];
        assert_eq!(all_types.len(), 9);
    }

    #[test]
    fn test_rate_type_code() {
        assert_eq!(RateType::Deposit.code(), "DEPO");
        assert_eq!(RateType::Fra.code(), "FRA");
        assert_eq!(RateType::Futures.code(), "FUT");
        assert_eq!(RateType::Swap.code(), "SWAP");
        assert_eq!(RateType::Ois.code(), "OIS");
        assert_eq!(RateType::BasisSwap.code(), "BASIS");
        assert_eq!(RateType::FxSpot.code(), "FXSPOT");
        assert_eq!(RateType::FxForward.code(), "FXFWD");
        assert_eq!(RateType::Vol.code(), "VOL");
    }

    #[test]
    fn test_rate_type_display() {
        assert_eq!(format!("{}", RateType::Swap), "SWAP");
        assert_eq!(format!("{}", RateType::Ois), "OIS");
        assert_eq!(format!("{}", RateType::FxSpot), "FXSPOT");
    }

    #[test]
    fn test_is_interest_rate() {
        assert!(RateType::Deposit.is_interest_rate());
        assert!(RateType::Fra.is_interest_rate());
        assert!(RateType::Futures.is_interest_rate());
        assert!(RateType::Swap.is_interest_rate());
        assert!(RateType::Ois.is_interest_rate());
        assert!(RateType::BasisSwap.is_interest_rate());

        assert!(!RateType::FxSpot.is_interest_rate());
        assert!(!RateType::FxForward.is_interest_rate());
        assert!(!RateType::Vol.is_interest_rate());
    }

    #[test]
    fn test_is_fx() {
        assert!(RateType::FxSpot.is_fx());
        assert!(RateType::FxForward.is_fx());

        assert!(!RateType::Deposit.is_fx());
        assert!(!RateType::Swap.is_fx());
        assert!(!RateType::Vol.is_fx());
    }

    #[test]
    fn test_is_volatility() {
        assert!(RateType::Vol.is_volatility());

        assert!(!RateType::Deposit.is_volatility());
        assert!(!RateType::FxSpot.is_volatility());
    }

    #[test]
    fn test_rate_type_copy() {
        let original = RateType::Swap;
        let copied = original;
        assert_eq!(original, copied);
    }

    #[test]
    fn test_rate_type_clone() {
        let original = RateType::Ois;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_rate_type_eq() {
        assert_eq!(RateType::Swap, RateType::Swap);
        assert_ne!(RateType::Swap, RateType::Ois);
    }

    #[test]
    fn test_rate_type_hash() {
        let mut set = HashSet::new();
        set.insert(RateType::Swap);
        set.insert(RateType::Ois);
        set.insert(RateType::Swap); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&RateType::Swap));
        assert!(set.contains(&RateType::Ois));
    }

    #[test]
    fn test_rate_type_as_hashmap_key() {
        let mut map: HashMap<RateType, &str> = HashMap::new();
        map.insert(RateType::Swap, "Interest Rate Swap");
        map.insert(RateType::Ois, "Overnight Index Swap");

        assert_eq!(map.get(&RateType::Swap), Some(&"Interest Rate Swap"));
        assert_eq!(map.get(&RateType::Ois), Some(&"Overnight Index Swap"));
    }

    #[test]
    fn test_rate_type_debug() {
        assert_eq!(format!("{:?}", RateType::Deposit), "Deposit");
        assert_eq!(format!("{:?}", RateType::Swap), "Swap");
        assert_eq!(format!("{:?}", RateType::FxSpot), "FxSpot");
    }
}
