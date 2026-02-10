//! Market rate type classification.
//!
//! This module provides the [`RateType`] enum for classifying different
//! types of market rates used in curve calibration and pricing.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::RateType;
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
/// 4. **Events** (curve jump instruments):
///    - `Event` - Central bank meetings, scheduled events
///
/// # Adding New Variants
///
/// When adding new rate types, place them within the appropriate asset class
/// group. Within interest rates, order by typical instrument maturity.
///
/// # Examples
///
/// ```
/// use infra_domain::market::RateType;
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

    // === Events ===
    /// Central bank meeting or scheduled market event (rate jump).
    Event,
}

impl RateType {
    /// Returns a short code for this rate type.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::RateType;
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
            RateType::Event => "EVENT",
        }
    }

    /// Returns true if this is an interest rate type.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::RateType;
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
    /// use infra_domain::market::RateType;
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
    /// use infra_domain::market::RateType;
    ///
    /// assert!(RateType::Vol.is_volatility());
    /// assert!(!RateType::Swap.is_volatility());
    /// ```
    #[must_use]
    pub const fn is_volatility(&self) -> bool { matches!(self, RateType::Vol) }

    /// Returns true if this is an event type (rate jump).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::RateType;
    ///
    /// assert!(RateType::Event.is_event());
    /// assert!(!RateType::Swap.is_event());
    /// ```
    #[must_use]
    pub const fn is_event(&self) -> bool { matches!(self, RateType::Event) }
}

impl fmt::Display for RateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.code()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification() {
        // Interest rate instruments
        for rt in [
            RateType::Deposit,
            RateType::Fra,
            RateType::Futures,
            RateType::Swap,
            RateType::Ois,
            RateType::BasisSwap,
        ] {
            assert!(rt.is_interest_rate(), "{} should be interest rate", rt);
            assert!(!rt.is_fx());
            assert!(!rt.is_volatility());
            assert!(!rt.is_event());
        }
        // FX instruments
        for rt in [RateType::FxSpot, RateType::FxForward] {
            assert!(rt.is_fx(), "{} should be FX", rt);
            assert!(!rt.is_interest_rate());
        }
        // Vol
        assert!(RateType::Vol.is_volatility());
        assert!(!RateType::Vol.is_interest_rate());
        // Event
        assert!(RateType::Event.is_event());
        assert!(!RateType::Event.is_interest_rate());
    }
}
