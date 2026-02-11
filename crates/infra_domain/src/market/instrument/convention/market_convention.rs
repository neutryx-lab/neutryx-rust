//! Market convention enum for unified convention handling.

use super::{
    DepositConvention, FraConvention, FuturesConvention, FxConvention, FxSwapConvention,
    SwapConvention, XCcyBasisConvention,
};
use crate::market::{Currency, QuoteId, RateType};

/// Unified market convention enum for all instrument types.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type", rename_all = "snake_case")
)]
pub enum MarketConvention {
    /// Deposit (money market) convention.
    Deposit(DepositConvention),
    /// Interest rate swap convention.
    Swap(SwapConvention),
    /// Overnight index swap convention.
    Ois(SwapConvention),
    /// Forward rate agreement convention.
    Fra(FraConvention),
    /// Interest rate futures convention.
    Futures(FuturesConvention),
    /// Cross-currency basis swap convention.
    XCcyBasis(XCcyBasisConvention),
    /// FX forward convention.
    FxForward(FxConvention),
    /// FX swap convention.
    FxSwap(FxSwapConvention),
}

impl MarketConvention {
    /// Returns the instrument type name for this convention.
    #[must_use]
    pub const fn instrument_type_name(&self) -> &'static str {
        match self {
            MarketConvention::Deposit(_) => "Deposit",
            MarketConvention::Swap(_) => "Swap",
            MarketConvention::Ois(_) => "OIS",
            MarketConvention::Fra(_) => "FRA",
            MarketConvention::Futures(_) => "Futures",
            MarketConvention::XCcyBasis(_) => "XCcyBasis",
            MarketConvention::FxForward(_) => "FxForward",
            MarketConvention::FxSwap(_) => "FxSwap",
        }
    }

    /// Returns the corresponding `RateType` for this convention.
    #[must_use]
    pub const fn rate_type(&self) -> RateType {
        match self {
            MarketConvention::Deposit(_) => RateType::Deposit,
            MarketConvention::Swap(_) => RateType::Swap,
            MarketConvention::Ois(_) => RateType::Ois,
            MarketConvention::Fra(_) => RateType::Fra,
            MarketConvention::Futures(_) => RateType::Futures,
            MarketConvention::XCcyBasis(_) => RateType::BasisSwap,
            MarketConvention::FxForward(_) => RateType::FxForward,
            MarketConvention::FxSwap(_) => RateType::FxForward,
        }
    }

    /// Derives an appropriate `MarketConvention` from a `QuoteId`.
    #[must_use]
    pub fn for_quote_id(quote_id: &QuoteId) -> Option<Self> {
        match quote_id.rate_type {
            RateType::Deposit => Self::deposit_convention_for_currency(quote_id.currency),
            RateType::Swap => Self::swap_convention_for_currency(quote_id.currency),
            RateType::Ois => Self::ois_convention_for_currency(quote_id.currency),
            RateType::Fra => Self::fra_convention_for_currency(quote_id.currency),
            RateType::Futures => Self::futures_convention_for_currency(quote_id.currency),
            RateType::BasisSwap => None,
            RateType::FxSpot | RateType::FxForward => {
                Self::fx_convention_for_currency(quote_id.currency)
            }
            RateType::Vol | RateType::Event => None,
        }
    }

    /// Returns a deposit convention for the given currency.
    #[allow(clippy::unnecessary_wraps)]
    fn deposit_convention_for_currency(currency: Currency) -> Option<Self> {
        let convention = match currency {
            Currency::USD => DepositConvention::usd(),
            Currency::EUR => DepositConvention::eur(),
            Currency::GBP => DepositConvention::gbp(),
            Currency::JPY => DepositConvention::jpy(),
            Currency::CHF => DepositConvention::chf(),
        };
        Some(MarketConvention::Deposit(convention))
    }

    /// Returns a swap convention for the given currency.
    fn swap_convention_for_currency(currency: Currency) -> Option<Self> {
        let convention = match currency {
            Currency::USD => SwapConvention::usd_sofr(),
            Currency::EUR => SwapConvention::eur_euribor_6m(),
            Currency::GBP => SwapConvention::gbp_sonia(),
            Currency::JPY => SwapConvention::jpy_tonar(),
            Currency::CHF => return None,
        };
        Some(MarketConvention::Swap(convention))
    }

    /// Returns an OIS convention for the given currency.
    fn ois_convention_for_currency(currency: Currency) -> Option<Self> {
        let convention = match currency {
            Currency::USD => SwapConvention::usd_sofr(),
            Currency::EUR => SwapConvention::eur_estr(),
            Currency::GBP => SwapConvention::gbp_sonia(),
            Currency::JPY => SwapConvention::jpy_tonar(),
            Currency::CHF => return None,
        };
        Some(MarketConvention::Ois(convention))
    }

    /// Returns a FRA convention for the given currency.
    fn fra_convention_for_currency(currency: Currency) -> Option<Self> {
        let convention = match currency {
            Currency::USD => FraConvention::usd_sofr(),
            Currency::EUR => FraConvention::eur_euribor_3m(),
            Currency::GBP | Currency::JPY | Currency::CHF => return None,
        };
        Some(MarketConvention::Fra(convention))
    }

    /// Returns a futures convention for the given currency.
    fn futures_convention_for_currency(currency: Currency) -> Option<Self> {
        let convention = match currency {
            Currency::USD => FuturesConvention::cme_sofr(),
            Currency::EUR => FuturesConvention::eurex_euribor(),
            Currency::GBP | Currency::JPY | Currency::CHF => return None,
        };
        Some(MarketConvention::Futures(convention))
    }

    /// Returns an FX convention for the given currency.
    fn fx_convention_for_currency(currency: Currency) -> Option<Self> {
        let convention = match currency {
            Currency::USD => FxConvention::usd_default(),
            Currency::EUR => FxConvention::eur_default(),
            Currency::GBP => FxConvention::gbp_default(),
            Currency::JPY => FxConvention::jpy_default(),
            Currency::CHF => return None,
        };
        Some(MarketConvention::FxForward(convention))
    }

    /// Returns whether this is a deposit convention.
    #[must_use]
    pub const fn is_deposit(&self) -> bool { matches!(self, MarketConvention::Deposit(_)) }

    /// Returns whether this is a swap convention.
    #[must_use]
    pub const fn is_swap(&self) -> bool { matches!(self, MarketConvention::Swap(_)) }

    /// Returns whether this is an OIS convention.
    #[must_use]
    pub const fn is_ois(&self) -> bool { matches!(self, MarketConvention::Ois(_)) }

    /// Returns whether this is a FRA convention.
    #[must_use]
    pub const fn is_fra(&self) -> bool { matches!(self, MarketConvention::Fra(_)) }

    /// Returns whether this is a futures convention.
    #[must_use]
    pub const fn is_futures(&self) -> bool { matches!(self, MarketConvention::Futures(_)) }

    /// Returns whether this is a cross-currency basis swap convention.
    #[must_use]
    pub const fn is_xccy_basis(&self) -> bool { matches!(self, MarketConvention::XCcyBasis(_)) }

    /// Returns whether this is an FX forward convention.
    #[must_use]
    pub const fn is_fx_forward(&self) -> bool { matches!(self, MarketConvention::FxForward(_)) }

    /// Returns whether this is an FX swap convention.
    #[must_use]
    pub const fn is_fx_swap(&self) -> bool { matches!(self, MarketConvention::FxSwap(_)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Tenor;

    #[test]
    fn test_instrument_type_name_and_rate_type() {
        let cases: Vec<(MarketConvention, &str, RateType)> = vec![
            (
                MarketConvention::Deposit(DepositConvention::usd()),
                "Deposit",
                RateType::Deposit,
            ),
            (
                MarketConvention::Swap(SwapConvention::usd_sofr()),
                "Swap",
                RateType::Swap,
            ),
            (
                MarketConvention::Ois(SwapConvention::usd_sofr()),
                "OIS",
                RateType::Ois,
            ),
            (
                MarketConvention::Fra(FraConvention::usd_sofr()),
                "FRA",
                RateType::Fra,
            ),
            (
                MarketConvention::Futures(FuturesConvention::cme_sofr()),
                "Futures",
                RateType::Futures,
            ),
            (
                MarketConvention::XCcyBasis(XCcyBasisConvention::usd_jpy()),
                "XCcyBasis",
                RateType::BasisSwap,
            ),
            (
                MarketConvention::FxForward(FxConvention::usd_default()),
                "FxForward",
                RateType::FxForward,
            ),
            (
                MarketConvention::FxSwap(FxSwapConvention::usd_jpy()),
                "FxSwap",
                RateType::FxForward,
            ),
        ];
        for (conv, name, rt) in &cases {
            assert_eq!(conv.instrument_type_name(), *name);
            assert_eq!(conv.rate_type(), *rt);
        }
    }

    #[test]
    fn test_for_quote_id_all_types() {
        for ccy in [
            Currency::USD,
            Currency::EUR,
            Currency::GBP,
            Currency::JPY,
            Currency::CHF,
        ] {
            let qid = QuoteId::new(ccy, Tenor::ThreeMonths, RateType::Deposit);
            assert!(MarketConvention::for_quote_id(&qid).unwrap().is_deposit());
        }
        for ccy in [Currency::USD, Currency::EUR, Currency::GBP, Currency::JPY] {
            let qid = QuoteId::new(ccy, Tenor::FiveYears, RateType::Swap);
            assert!(MarketConvention::for_quote_id(&qid).unwrap().is_swap());
        }
        assert!(MarketConvention::for_quote_id(&QuoteId::new(
            Currency::CHF,
            Tenor::FiveYears,
            RateType::Swap
        ))
        .is_none());
        assert!(MarketConvention::for_quote_id(&QuoteId::new(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Fra
        ))
        .unwrap()
        .is_fra());
        assert!(MarketConvention::for_quote_id(&QuoteId::new(
            Currency::GBP,
            Tenor::ThreeMonths,
            RateType::Fra
        ))
        .is_none());
        assert!(MarketConvention::for_quote_id(&QuoteId::new(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Futures
        ))
        .unwrap()
        .is_futures());
        assert!(MarketConvention::for_quote_id(&QuoteId::new(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::FxForward
        ))
        .unwrap()
        .is_fx_forward());
        assert!(MarketConvention::for_quote_id(&QuoteId::new(
            Currency::USD,
            Tenor::OneYear,
            RateType::Vol
        ))
        .is_none());
        assert!(MarketConvention::for_quote_id(&QuoteId::new(
            Currency::USD,
            Tenor::FiveYears,
            RateType::BasisSwap
        ))
        .is_none());
    }

    #[test]
    fn test_is_methods() {
        assert!(MarketConvention::Deposit(DepositConvention::usd()).is_deposit());
        assert!(MarketConvention::Swap(SwapConvention::usd_sofr()).is_swap());
        assert!(MarketConvention::Ois(SwapConvention::usd_sofr()).is_ois());
        assert!(MarketConvention::Fra(FraConvention::usd_sofr()).is_fra());
        assert!(MarketConvention::Futures(FuturesConvention::cme_sofr()).is_futures());
        assert!(MarketConvention::XCcyBasis(XCcyBasisConvention::usd_jpy()).is_xccy_basis());
        assert!(MarketConvention::FxForward(FxConvention::usd_default()).is_fx_forward());
        assert!(MarketConvention::FxSwap(FxSwapConvention::usd_jpy()).is_fx_swap());
        assert!(!MarketConvention::Deposit(DepositConvention::usd()).is_swap());
        assert!(!MarketConvention::Swap(SwapConvention::usd_sofr()).is_ois());
    }
}
