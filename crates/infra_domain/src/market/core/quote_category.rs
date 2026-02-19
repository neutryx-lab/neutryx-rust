//! Market quote category classification.

use std::fmt;

/// Classification of market quote categories grouped by asset class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum QuoteCategory {
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

    /// Fixed-coupon bond (government, corporate, agency).
    Bond,
    /// Credit spread quote (CDS, CDX, iTraxx).
    CreditSpread,
}

impl QuoteCategory {
    /// Returns a short code for this quote category.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            QuoteCategory::Deposit => "DEPO",
            QuoteCategory::Fra => "FRA",
            QuoteCategory::Futures => "FUT",
            QuoteCategory::Swap => "SWAP",
            QuoteCategory::Ois => "OIS",
            QuoteCategory::BasisSwap => "BASIS",
            QuoteCategory::FxSpot => "FXSPOT",
            QuoteCategory::FxForward => "FXFWD",
            QuoteCategory::Vol => "VOL",
            QuoteCategory::Event => "EVENT",
            QuoteCategory::Bond => "BOND",
            QuoteCategory::CreditSpread => "CREDIT",
        }
    }

    /// Returns true if this is an interest rate category.
    #[must_use]
    pub const fn is_interest_rate(&self) -> bool {
        matches!(
            self,
            QuoteCategory::Deposit
                | QuoteCategory::Fra
                | QuoteCategory::Futures
                | QuoteCategory::Swap
                | QuoteCategory::Ois
                | QuoteCategory::BasisSwap
        )
    }

    /// Returns true if this is an FX category.
    #[must_use]
    pub const fn is_fx(&self) -> bool {
        matches!(self, QuoteCategory::FxSpot | QuoteCategory::FxForward)
    }

    /// Returns true if this is a volatility quote.
    #[must_use]
    pub const fn is_volatility(&self) -> bool { matches!(self, QuoteCategory::Vol) }

    /// Returns true if this is an event category (rate jump).
    #[must_use]
    pub const fn is_event(&self) -> bool { matches!(self, QuoteCategory::Event) }

    /// Returns true if this is a bond quote.
    #[must_use]
    pub const fn is_bond(&self) -> bool { matches!(self, QuoteCategory::Bond) }

    /// Returns true if this is a credit spread quote.
    #[must_use]
    pub const fn is_credit(&self) -> bool { matches!(self, QuoteCategory::CreditSpread) }
}

impl fmt::Display for QuoteCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.code()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification() {
        for rt in [
            QuoteCategory::Deposit,
            QuoteCategory::Fra,
            QuoteCategory::Futures,
            QuoteCategory::Swap,
            QuoteCategory::Ois,
            QuoteCategory::BasisSwap,
        ] {
            assert!(rt.is_interest_rate(), "{} should be interest rate", rt);
            assert!(!rt.is_fx());
            assert!(!rt.is_volatility());
            assert!(!rt.is_event());
        }
        for rt in [QuoteCategory::FxSpot, QuoteCategory::FxForward] {
            assert!(rt.is_fx(), "{} should be FX", rt);
            assert!(!rt.is_interest_rate());
        }
        assert!(QuoteCategory::Vol.is_volatility());
        assert!(!QuoteCategory::Vol.is_interest_rate());
        assert!(QuoteCategory::Event.is_event());
        assert!(!QuoteCategory::Event.is_interest_rate());

        assert!(QuoteCategory::Bond.is_bond());
        assert!(!QuoteCategory::Bond.is_interest_rate());
        assert!(!QuoteCategory::Bond.is_fx());
        assert!(QuoteCategory::CreditSpread.is_credit());
        assert!(!QuoteCategory::CreditSpread.is_interest_rate());
    }
}
