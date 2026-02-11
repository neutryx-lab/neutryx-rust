//! Market rate type classification.

use std::fmt;

/// Classification of market rate types grouped by asset class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum RateType {
    /// Money market deposit rate (short-dated, ON to 1Y).
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

    /// FX spot rate.
    FxSpot,
    /// FX forward rate.
    FxForward,

    /// Volatility quote (implied vol for options).
    Vol,

    /// Central bank meeting or scheduled market event (rate jump).
    Event,
}

impl RateType {
    /// Returns a short code for this rate type.
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
    #[must_use]
    pub const fn is_fx(&self) -> bool { matches!(self, RateType::FxSpot | RateType::FxForward) }

    /// Returns true if this is a volatility quote.
    #[must_use]
    pub const fn is_volatility(&self) -> bool { matches!(self, RateType::Vol) }

    /// Returns true if this is an event type (rate jump).
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
        for rt in [RateType::FxSpot, RateType::FxForward] {
            assert!(rt.is_fx(), "{} should be FX", rt);
            assert!(!rt.is_interest_rate());
        }
        assert!(RateType::Vol.is_volatility());
        assert!(!RateType::Vol.is_interest_rate());
        assert!(RateType::Event.is_event());
        assert!(!RateType::Event.is_interest_rate());
    }
}
