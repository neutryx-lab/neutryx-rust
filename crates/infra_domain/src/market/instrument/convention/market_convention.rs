//! Market convention enum for unified convention handling.
//!
//! This module provides the [`MarketConvention`] enum that wraps all
//! individual convention types, enabling uniform handling of different
//! instrument types.

use super::{
    DepositConvention, FraConvention, FuturesConvention, FxConvention, FxSwapConvention,
    SwapConvention, XCcyBasisConvention,
};
use crate::market::{Currency, QuoteId, RateType};

/// Unified market convention enum for all instrument types.
///
/// This enum wraps all individual convention types, allowing uniform
/// handling of different instrument conventions. It can be used to
/// store and retrieve conventions from a registry, or to derive
/// conventions from market rate identifiers.
///
/// # Example
///
/// ```rust
/// use infra_domain::market::convention::MarketConvention;
/// use infra_domain::market::{Currency, QuoteId, RateType};
/// use infra_domain::time::Tenor;
///
/// let quote_id = QuoteId::new(Currency::USD, Tenor::FiveYears, RateType::Swap);
/// let convention = MarketConvention::for_quote_id(&quote_id);
/// assert!(convention.is_some());
/// assert_eq!(convention.unwrap().instrument_type_name(), "Swap");
/// ```
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use infra_domain::market::convention::{MarketConvention, DepositConvention};
    ///
    /// let conv = MarketConvention::Deposit(DepositConvention::usd());
    /// assert_eq!(conv.instrument_type_name(), "Deposit");
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use infra_domain::market::convention::{MarketConvention, SwapConvention};
    /// use infra_domain::market::RateType;
    ///
    /// let conv = MarketConvention::Swap(SwapConvention::usd_sofr());
    /// assert_eq!(conv.rate_type(), RateType::Swap);
    /// ```
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
            MarketConvention::FxSwap(_) => RateType::FxForward, // FxSwap uses FxForward rate type
        }
    }

    /// Derives an appropriate `MarketConvention` from a `QuoteId`.
    ///
    /// This method uses the currency and rate type from the `QuoteId` to
    /// determine the appropriate convention. Returns `None` if no
    /// convention is available for the given combination.
    ///
    /// # Arguments
    ///
    /// * `quote_id` - The rate identifier to derive convention from
    ///
    /// # Returns
    ///
    /// Some(MarketConvention) if a convention exists for the combination,
    /// None otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use infra_domain::market::convention::MarketConvention;
    /// use infra_domain::market::{Currency, QuoteId, RateType};
    /// use infra_domain::time::Tenor;
    ///
    /// // USD Deposit
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let conv = MarketConvention::for_quote_id(&quote_id);
    /// assert!(conv.is_some());
    /// assert_eq!(conv.unwrap().instrument_type_name(), "Deposit");
    ///
    /// // EUR Swap
    /// let quote_id = QuoteId::new(Currency::EUR, Tenor::FiveYears, RateType::Swap);
    /// let conv = MarketConvention::for_quote_id(&quote_id);
    /// assert!(conv.is_some());
    /// assert_eq!(conv.unwrap().instrument_type_name(), "Swap");
    ///
    /// // Vol (no convention available)
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::OneYear, RateType::Vol);
    /// let conv = MarketConvention::for_quote_id(&quote_id);
    /// assert!(conv.is_none());
    /// ```
    #[must_use]
    pub fn for_quote_id(quote_id: &QuoteId) -> Option<Self> {
        match quote_id.rate_type {
            RateType::Deposit => Self::deposit_convention_for_currency(quote_id.currency),
            RateType::Swap => Self::swap_convention_for_currency(quote_id.currency),
            RateType::Ois => Self::ois_convention_for_currency(quote_id.currency),
            RateType::Fra => Self::fra_convention_for_currency(quote_id.currency),
            RateType::Futures => Self::futures_convention_for_currency(quote_id.currency),
            RateType::BasisSwap => None, // Requires currency pair, not single currency
            RateType::FxSpot | RateType::FxForward => {
                Self::fx_convention_for_currency(quote_id.currency)
            }
            RateType::Vol | RateType::Event => None, // No standard convention
        }
    }

    /// Returns a deposit convention for the given currency.
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
            Currency::CHF => return None, // CHF swap convention not yet implemented
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
            Currency::CHF => return None, // CHF OIS convention not yet implemented
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
    fn test_instrument_type_name() {
        assert_eq!(
            MarketConvention::Deposit(DepositConvention::usd()).instrument_type_name(),
            "Deposit"
        );
        assert_eq!(
            MarketConvention::Swap(SwapConvention::usd_sofr()).instrument_type_name(),
            "Swap"
        );
        assert_eq!(
            MarketConvention::Ois(SwapConvention::usd_sofr()).instrument_type_name(),
            "OIS"
        );
        assert_eq!(
            MarketConvention::Fra(FraConvention::usd_sofr()).instrument_type_name(),
            "FRA"
        );
        assert_eq!(
            MarketConvention::Futures(FuturesConvention::cme_sofr()).instrument_type_name(),
            "Futures"
        );
        assert_eq!(
            MarketConvention::XCcyBasis(XCcyBasisConvention::usd_jpy()).instrument_type_name(),
            "XCcyBasis"
        );
        assert_eq!(
            MarketConvention::FxForward(FxConvention::usd_default()).instrument_type_name(),
            "FxForward"
        );
        assert_eq!(
            MarketConvention::FxSwap(FxSwapConvention::usd_jpy()).instrument_type_name(),
            "FxSwap"
        );
    }

    #[test]
    fn test_rate_type() {
        assert_eq!(
            MarketConvention::Deposit(DepositConvention::usd()).rate_type(),
            RateType::Deposit
        );
        assert_eq!(
            MarketConvention::Swap(SwapConvention::usd_sofr()).rate_type(),
            RateType::Swap
        );
        assert_eq!(
            MarketConvention::Ois(SwapConvention::usd_sofr()).rate_type(),
            RateType::Ois
        );
        assert_eq!(
            MarketConvention::Fra(FraConvention::usd_sofr()).rate_type(),
            RateType::Fra
        );
    }

    #[test]
    fn test_for_quote_id_deposit() {
        for currency in [
            Currency::USD,
            Currency::EUR,
            Currency::GBP,
            Currency::JPY,
            Currency::CHF,
        ] {
            let quote_id = QuoteId::new(currency, Tenor::ThreeMonths, RateType::Deposit);
            let conv = MarketConvention::for_quote_id(&quote_id);
            assert!(conv.is_some(), "Deposit convention should exist for {:?}", currency);
            assert!(conv.unwrap().is_deposit());
        }
    }

    #[test]
    fn test_for_quote_id_swap() {
        for currency in [Currency::USD, Currency::EUR, Currency::GBP, Currency::JPY] {
            let quote_id = QuoteId::new(currency, Tenor::FiveYears, RateType::Swap);
            let conv = MarketConvention::for_quote_id(&quote_id);
            assert!(conv.is_some(), "Swap convention should exist for {:?}", currency);
            assert!(conv.unwrap().is_swap());
        }

        // CHF swap not implemented
        let quote_id = QuoteId::new(Currency::CHF, Tenor::FiveYears, RateType::Swap);
        assert!(MarketConvention::for_quote_id(&quote_id).is_none());
    }

    #[test]
    fn test_for_quote_id_ois() {
        for currency in [Currency::USD, Currency::EUR, Currency::GBP, Currency::JPY] {
            let quote_id = QuoteId::new(currency, Tenor::OneYear, RateType::Ois);
            let conv = MarketConvention::for_quote_id(&quote_id);
            assert!(conv.is_some(), "OIS convention should exist for {:?}", currency);
            assert!(conv.unwrap().is_ois());
        }
    }

    #[test]
    fn test_for_quote_id_fra() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Fra);
        let conv = MarketConvention::for_quote_id(&quote_id);
        assert!(conv.is_some());
        assert!(conv.unwrap().is_fra());

        let quote_id = QuoteId::new(Currency::EUR, Tenor::ThreeMonths, RateType::Fra);
        let conv = MarketConvention::for_quote_id(&quote_id);
        assert!(conv.is_some());
        assert!(conv.unwrap().is_fra());

        // GBP FRA not implemented
        let quote_id = QuoteId::new(Currency::GBP, Tenor::ThreeMonths, RateType::Fra);
        assert!(MarketConvention::for_quote_id(&quote_id).is_none());
    }

    #[test]
    fn test_for_quote_id_futures() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Futures);
        let conv = MarketConvention::for_quote_id(&quote_id);
        assert!(conv.is_some());
        assert!(conv.unwrap().is_futures());

        let quote_id = QuoteId::new(Currency::EUR, Tenor::ThreeMonths, RateType::Futures);
        let conv = MarketConvention::for_quote_id(&quote_id);
        assert!(conv.is_some());
        assert!(conv.unwrap().is_futures());
    }

    #[test]
    fn test_for_quote_id_fx_forward() {
        for currency in [Currency::USD, Currency::EUR, Currency::GBP, Currency::JPY] {
            let quote_id = QuoteId::new(currency, Tenor::ThreeMonths, RateType::FxForward);
            let conv = MarketConvention::for_quote_id(&quote_id);
            assert!(
                conv.is_some(),
                "FX convention should exist for {:?}",
                currency
            );
            assert!(conv.unwrap().is_fx_forward());
        }
    }

    #[test]
    fn test_for_quote_id_vol_returns_none() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::OneYear, RateType::Vol);
        assert!(MarketConvention::for_quote_id(&quote_id).is_none());
    }

    #[test]
    fn test_for_quote_id_basis_swap_returns_none() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::FiveYears, RateType::BasisSwap);
        assert!(MarketConvention::for_quote_id(&quote_id).is_none());
    }

    #[test]
    fn test_is_methods() {
        let deposit = MarketConvention::Deposit(DepositConvention::usd());
        assert!(deposit.is_deposit());
        assert!(!deposit.is_swap());

        let swap = MarketConvention::Swap(SwapConvention::usd_sofr());
        assert!(swap.is_swap());
        assert!(!swap.is_deposit());

        let ois = MarketConvention::Ois(SwapConvention::usd_sofr());
        assert!(ois.is_ois());
        assert!(!ois.is_swap());

        let fra = MarketConvention::Fra(FraConvention::usd_sofr());
        assert!(fra.is_fra());

        let futures = MarketConvention::Futures(FuturesConvention::cme_sofr());
        assert!(futures.is_futures());

        let xccy = MarketConvention::XCcyBasis(XCcyBasisConvention::usd_jpy());
        assert!(xccy.is_xccy_basis());

        let fx_fwd = MarketConvention::FxForward(FxConvention::usd_default());
        assert!(fx_fwd.is_fx_forward());

        let fx_swap = MarketConvention::FxSwap(FxSwapConvention::usd_jpy());
        assert!(fx_swap.is_fx_swap());
    }

    #[test]
    fn test_market_convention_clone() {
        let conv = MarketConvention::Swap(SwapConvention::usd_sofr());
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }

    #[test]
    fn test_market_convention_debug() {
        let conv = MarketConvention::Deposit(DepositConvention::usd());
        let debug_str = format!("{:?}", conv);
        assert!(debug_str.contains("Deposit"));
    }
}
