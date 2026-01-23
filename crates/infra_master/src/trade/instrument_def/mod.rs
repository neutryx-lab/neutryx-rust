//! Standard instrument definitions for financial products.
//!
//! This module provides comprehensive definitions for standard financial
//! instruments across all asset classes (Rates, FX, Equity, Credit, Commodity)
//! as used by Tier-1 bank trading desks.
//!
//! # Architecture
//!
//! The instrument definitions follow a hierarchical structure:
//!
//! ```text
//! InstrumentDefinition (enum)
//! ├── Rates: Swaption, CapFloor, Frn, CmsSwap, InflationSwap
//! ├── FX: FxSpot, FxForward, FxVanillaOption, FxBarrierOption, FxSwap
//! ├── Equity: EquityForward, EquityVanillaOption, AsianOption, ...
//! ├── Credit: Cds, CdsIndex, CdsOption, NtdBasket
//! └── Commodity: CommodityForward, CommoditySwap, ...
//! ```
//!
//! # Example
//!
//! ```rust
//! use infra_master::trade::instrument_def::{
//!     InstrumentDefinition, AssetClass, Swaption, PayerReceiver,
//! };
//! use infra_master::trade::{ExerciseType, SettlementType};
//! use infra_master::{Currency, Date, Tenor};
//!
//! let swaption = Swaption {
//!     underlying_swap_tenor: Tenor::TenYears,
//!     expiry: Date::from_ymd(2026, 1, 15).unwrap(),
//!     exercise_type: ExerciseType::European,
//!     settlement_type: SettlementType::Cash,
//!     strike: 0.03,
//!     notional: 10_000_000.0,
//!     currency: Currency::USD,
//!     payer_receiver: PayerReceiver::Payer,
//! };
//!
//! let instrument = InstrumentDefinition::Swaption(swaption);
//! assert_eq!(instrument.asset_class(), AssetClass::Rates);
//! assert!(instrument.is_option());
//! ```

mod common;
mod error;
mod expander;

// Asset class specific modules
mod commodity;
mod credit;
mod equity;
mod fx;
mod rates;

// Re-exports
// Commodity instruments
pub use commodity::{
    AgricultureType, CommodityAsianOption, CommodityForward, CommoditySwap, CommodityType,
    CommodityVanillaOption, EnergyType, MetalType, QuantityUnit, SpreadOption,
};
pub use common::{
    AssetClass, BarrierDirection, BarrierType, ExerciseStyle, NotionalSchedule, PayerReceiver,
};
// Credit instruments
pub use credit::{Cds, CdsIndex, CdsOption, CreditEvent, NtdBasket};
// Equity instruments
pub use equity::{
    AsianOption, AveragingType, BasketComponent, BasketOption, EquityBarrierOption, EquityForward,
    EquityReturnType, EquitySwap, EquityUnderlying, EquityVanillaOption, LookbackOption,
    LookbackType, MonitoringFrequency,
};
pub use error::InstrumentError;
pub use expander::InstrumentExpander;
// FX instruments
pub use fx::{CurrencyPair, FxBarrierOption, FxForward, FxSpot, FxSwap, FxVanillaOption};
// Rates instruments
pub use rates::{CapFloor, CapFloorType, CmsSwap, Frn, InflationSwap, SwapType, Swaption};

// ============================================================================
// InstrumentDefinition - Unified enum for all standard instruments
// ============================================================================

/// Unified instrument definition enum covering all asset classes.
///
/// This enum provides a single entry point for all standard financial
/// instruments, enabling type-safe handling and static dispatch for
/// Enzyme compatibility.
///
/// # Asset Classes
///
/// - **Rates**: Interest rate derivatives (swaptions, caps/floors, FRNs, etc.)
/// - **FX**: Foreign exchange instruments (spots, forwards, options, swaps)
/// - **Equity**: Equity derivatives (forwards, options, swaps)
/// - **Credit**: Credit derivatives (CDS, CDX, NtD baskets)
/// - **Commodity**: Commodity derivatives (forwards, swaps, options)
///
/// # Example
///
/// ```rust
/// use infra_master::trade::instrument_def::{
///     InstrumentDefinition, AssetClass, FxSpot, CurrencyPair,
/// };
/// use infra_master::{Currency, Date};
///
/// let fx_spot = FxSpot {
///     currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
///     spot_rate: 1.1050,
///     settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
///     notional: 1_000_000.0,
///     notional_currency: Currency::EUR,
/// };
///
/// let instrument = InstrumentDefinition::FxSpot(fx_spot);
/// assert_eq!(instrument.asset_class(), AssetClass::Fx);
/// assert!(!instrument.is_option());
/// assert!(!instrument.is_swap());
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum InstrumentDefinition {
    // === Rates ===
    /// Swaption (option on interest rate swap).
    Swaption(Swaption),
    /// Interest rate cap or floor.
    CapFloor(CapFloor),
    /// Floating rate note.
    Frn(Frn),
    /// Constant maturity swap.
    CmsSwap(CmsSwap),
    /// Inflation-linked swap.
    InflationSwap(InflationSwap),

    // === FX ===
    /// FX spot transaction.
    FxSpot(FxSpot),
    /// FX forward transaction.
    FxForward(FxForward),
    /// FX vanilla option.
    FxVanillaOption(FxVanillaOption),
    /// FX barrier option.
    FxBarrierOption(FxBarrierOption),
    /// FX swap (short-term, near/far legs).
    FxSwap(FxSwap),

    // === Equity ===
    /// Equity forward.
    EquityForward(EquityForward),
    /// Equity vanilla option.
    EquityVanillaOption(EquityVanillaOption),
    /// Equity barrier option.
    EquityBarrierOption(EquityBarrierOption),
    /// Asian option (path-dependent averaging).
    AsianOption(AsianOption),
    /// Lookback option (path-dependent extremum).
    LookbackOption(LookbackOption),
    /// Equity swap (equity return vs funding).
    EquitySwap(EquitySwap),
    /// Basket option on multiple underlyings.
    BasketOption(BasketOption),

    // === Credit ===
    /// Single-name credit default swap.
    Cds(Cds),
    /// CDS index (CDX/iTraxx).
    CdsIndex(CdsIndex),
    /// CDS option (swaption on CDS).
    CdsOption(CdsOption),
    /// Nth-to-default basket.
    NtdBasket(NtdBasket),

    // === Commodity ===
    /// Commodity forward.
    CommodityForward(CommodityForward),
    /// Commodity swap (fixed vs floating).
    CommoditySwap(CommoditySwap),
    /// Commodity vanilla option.
    CommodityVanillaOption(CommodityVanillaOption),
    /// Commodity Asian option.
    CommodityAsianOption(CommodityAsianOption),
    /// Spread option on two commodities.
    SpreadOption(SpreadOption),
}

impl InstrumentDefinition {
    /// Returns the asset class of this instrument.
    ///
    /// # Example
    ///
    /// ```rust
    /// use infra_master::trade::instrument_def::{
    ///     InstrumentDefinition, AssetClass, Swaption, PayerReceiver,
    /// };
    /// use infra_master::trade::{ExerciseType, SettlementType};
    /// use infra_master::{Currency, Date, Tenor};
    ///
    /// let swaption = Swaption {
    ///     underlying_swap_tenor: Tenor::TenYears,
    ///     expiry: Date::from_ymd(2026, 1, 15).unwrap(),
    ///     exercise_type: ExerciseType::European,
    ///     settlement_type: SettlementType::Cash,
    ///     strike: 0.03,
    ///     notional: 10_000_000.0,
    ///     currency: Currency::USD,
    ///     payer_receiver: PayerReceiver::Payer,
    /// };
    ///
    /// let instrument = InstrumentDefinition::Swaption(swaption);
    /// assert_eq!(instrument.asset_class(), AssetClass::Rates);
    /// ```
    #[must_use]
    pub fn asset_class(&self) -> AssetClass {
        match self {
            // Rates
            InstrumentDefinition::Swaption(_)
            | InstrumentDefinition::CapFloor(_)
            | InstrumentDefinition::Frn(_)
            | InstrumentDefinition::CmsSwap(_)
            | InstrumentDefinition::InflationSwap(_) => AssetClass::Rates,

            // FX
            InstrumentDefinition::FxSpot(_)
            | InstrumentDefinition::FxForward(_)
            | InstrumentDefinition::FxVanillaOption(_)
            | InstrumentDefinition::FxBarrierOption(_)
            | InstrumentDefinition::FxSwap(_) => AssetClass::Fx,

            // Equity
            InstrumentDefinition::EquityForward(_)
            | InstrumentDefinition::EquityVanillaOption(_)
            | InstrumentDefinition::EquityBarrierOption(_)
            | InstrumentDefinition::AsianOption(_)
            | InstrumentDefinition::LookbackOption(_)
            | InstrumentDefinition::EquitySwap(_)
            | InstrumentDefinition::BasketOption(_) => AssetClass::Equity,

            // Credit
            InstrumentDefinition::Cds(_)
            | InstrumentDefinition::CdsIndex(_)
            | InstrumentDefinition::CdsOption(_)
            | InstrumentDefinition::NtdBasket(_) => AssetClass::Credit,

            // Commodity
            InstrumentDefinition::CommodityForward(_)
            | InstrumentDefinition::CommoditySwap(_)
            | InstrumentDefinition::CommodityVanillaOption(_)
            | InstrumentDefinition::CommodityAsianOption(_)
            | InstrumentDefinition::SpreadOption(_) => AssetClass::Commodity,
        }
    }

    /// Returns `true` if this is an option instrument.
    ///
    /// Includes vanilla options, barrier options, Asian options,
    /// lookback options, basket options, swaptions, caps/floors,
    /// CDS options, and commodity options.
    #[must_use]
    pub fn is_option(&self) -> bool {
        matches!(
            self,
            // Rates options
            InstrumentDefinition::Swaption(_)
                | InstrumentDefinition::CapFloor(_)
                // FX options
                | InstrumentDefinition::FxVanillaOption(_)
                | InstrumentDefinition::FxBarrierOption(_)
                // Equity options
                | InstrumentDefinition::EquityVanillaOption(_)
                | InstrumentDefinition::EquityBarrierOption(_)
                | InstrumentDefinition::AsianOption(_)
                | InstrumentDefinition::LookbackOption(_)
                | InstrumentDefinition::BasketOption(_)
                // Credit options
                | InstrumentDefinition::CdsOption(_)
                // Commodity options
                | InstrumentDefinition::CommodityVanillaOption(_)
                | InstrumentDefinition::CommodityAsianOption(_)
                | InstrumentDefinition::SpreadOption(_)
        )
    }

    /// Returns `true` if this is a swap instrument.
    ///
    /// Includes interest rate swaps, FX swaps, equity swaps,
    /// CDS, and commodity swaps.
    #[must_use]
    pub fn is_swap(&self) -> bool {
        matches!(
            self,
            // Rates swaps
            InstrumentDefinition::CmsSwap(_)
                | InstrumentDefinition::InflationSwap(_)
                // FX swaps
                | InstrumentDefinition::FxSwap(_)
                // Equity swaps
                | InstrumentDefinition::EquitySwap(_)
                // Credit (CDS is a swap)
                | InstrumentDefinition::Cds(_)
                | InstrumentDefinition::CdsIndex(_)
                | InstrumentDefinition::NtdBasket(_)
                // Commodity swaps
                | InstrumentDefinition::CommoditySwap(_)
        )
    }

    /// Returns `true` if this is a forward instrument.
    ///
    /// Includes FX forwards, equity forwards, commodity forwards,
    /// and FX spots (which are effectively T+2 forwards).
    #[must_use]
    pub fn is_forward(&self) -> bool {
        matches!(
            self,
            InstrumentDefinition::FxSpot(_)
                | InstrumentDefinition::FxForward(_)
                | InstrumentDefinition::EquityForward(_)
                | InstrumentDefinition::CommodityForward(_)
        )
    }

    /// Returns `true` if this is a path-dependent instrument.
    ///
    /// Path-dependent instruments require simulation of the entire
    /// price path, not just the terminal value.
    #[must_use]
    pub fn is_path_dependent(&self) -> bool {
        matches!(
            self,
            InstrumentDefinition::AsianOption(_)
                | InstrumentDefinition::LookbackOption(_)
                | InstrumentDefinition::FxBarrierOption(_)
                | InstrumentDefinition::EquityBarrierOption(_)
                | InstrumentDefinition::CommodityAsianOption(_)
        )
    }

    /// Returns `true` if this is an exotic instrument.
    ///
    /// Exotic instruments have non-standard payoffs that typically
    /// require Monte Carlo pricing.
    #[must_use]
    pub fn is_exotic(&self) -> bool {
        matches!(
            self,
            InstrumentDefinition::AsianOption(_)
                | InstrumentDefinition::LookbackOption(_)
                | InstrumentDefinition::FxBarrierOption(_)
                | InstrumentDefinition::EquityBarrierOption(_)
                | InstrumentDefinition::BasketOption(_)
                | InstrumentDefinition::CommodityAsianOption(_)
                | InstrumentDefinition::SpreadOption(_)
                | InstrumentDefinition::NtdBasket(_)
        )
    }

    /// Validates the instrument parameters.
    ///
    /// Delegates to the specific instrument's validation method.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if validation fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use infra_master::trade::instrument_def::{
    ///     InstrumentDefinition, FxSpot, CurrencyPair,
    /// };
    /// use infra_master::{Currency, Date};
    ///
    /// let fx_spot = FxSpot {
    ///     currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
    ///     spot_rate: 1.1050,
    ///     settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
    ///     notional: 1_000_000.0,
    ///     notional_currency: Currency::EUR,
    /// };
    ///
    /// let instrument = InstrumentDefinition::FxSpot(fx_spot);
    /// assert!(instrument.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), InstrumentError> {
        match self {
            // Rates
            InstrumentDefinition::Swaption(s) => s.validate(),
            InstrumentDefinition::CapFloor(c) => c.validate(),
            InstrumentDefinition::Frn(f) => f.validate(),
            InstrumentDefinition::CmsSwap(c) => c.validate(),
            InstrumentDefinition::InflationSwap(i) => i.validate(),

            // FX
            InstrumentDefinition::FxSpot(s) => s.validate(),
            InstrumentDefinition::FxForward(f) => f.validate(),
            InstrumentDefinition::FxVanillaOption(o) => o.validate(),
            InstrumentDefinition::FxBarrierOption(b) => b.validate(),
            InstrumentDefinition::FxSwap(s) => s.validate(),

            // Equity
            InstrumentDefinition::EquityForward(f) => f.validate(),
            InstrumentDefinition::EquityVanillaOption(o) => o.validate(),
            InstrumentDefinition::EquityBarrierOption(b) => b.validate(),
            InstrumentDefinition::AsianOption(a) => a.validate(),
            InstrumentDefinition::LookbackOption(l) => l.validate(),
            InstrumentDefinition::EquitySwap(s) => s.validate(),
            InstrumentDefinition::BasketOption(b) => b.validate(),

            // Credit
            InstrumentDefinition::Cds(c) => c.validate(),
            InstrumentDefinition::CdsIndex(i) => i.validate(),
            InstrumentDefinition::CdsOption(o) => o.validate(),
            InstrumentDefinition::NtdBasket(n) => n.validate(),

            // Commodity
            InstrumentDefinition::CommodityForward(f) => f.validate(),
            InstrumentDefinition::CommoditySwap(s) => s.validate(),
            InstrumentDefinition::CommodityVanillaOption(o) => o.validate(),
            InstrumentDefinition::CommodityAsianOption(a) => a.validate(),
            InstrumentDefinition::SpreadOption(s) => s.validate(),
        }
    }
}

impl std::fmt::Display for InstrumentDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            InstrumentDefinition::Swaption(_) => "Swaption",
            InstrumentDefinition::CapFloor(_) => "CapFloor",
            InstrumentDefinition::Frn(_) => "FRN",
            InstrumentDefinition::CmsSwap(_) => "CMSSwap",
            InstrumentDefinition::InflationSwap(_) => "InflationSwap",
            InstrumentDefinition::FxSpot(_) => "FXSpot",
            InstrumentDefinition::FxForward(_) => "FXForward",
            InstrumentDefinition::FxVanillaOption(_) => "FXVanillaOption",
            InstrumentDefinition::FxBarrierOption(_) => "FXBarrierOption",
            InstrumentDefinition::FxSwap(_) => "FXSwap",
            InstrumentDefinition::EquityForward(_) => "EquityForward",
            InstrumentDefinition::EquityVanillaOption(_) => "EquityVanillaOption",
            InstrumentDefinition::EquityBarrierOption(_) => "EquityBarrierOption",
            InstrumentDefinition::AsianOption(_) => "AsianOption",
            InstrumentDefinition::LookbackOption(_) => "LookbackOption",
            InstrumentDefinition::EquitySwap(_) => "EquitySwap",
            InstrumentDefinition::BasketOption(_) => "BasketOption",
            InstrumentDefinition::Cds(_) => "CDS",
            InstrumentDefinition::CdsIndex(_) => "CDSIndex",
            InstrumentDefinition::CdsOption(_) => "CDSOption",
            InstrumentDefinition::NtdBasket(_) => "NtDBasket",
            InstrumentDefinition::CommodityForward(_) => "CommodityForward",
            InstrumentDefinition::CommoditySwap(_) => "CommoditySwap",
            InstrumentDefinition::CommodityVanillaOption(_) => "CommodityVanillaOption",
            InstrumentDefinition::CommodityAsianOption(_) => "CommodityAsianOption",
            InstrumentDefinition::SpreadOption(_) => "SpreadOption",
        };
        write!(f, "{}", name)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        trade::{ExerciseType, OptionType, SettlementType},
        Currency, Date, Frequency, RateIndex, Tenor,
    };

    // === Test Helpers ===

    fn make_test_swaption() -> Swaption {
        Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        }
    }

    fn make_test_fx_spot() -> FxSpot {
        FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        }
    }

    fn make_test_fx_vanilla_option() -> FxVanillaOption {
        FxVanillaOption {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            strike: 1.1000,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            delivery_date: Date::from_ymd(2025, 6, 17).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        }
    }

    fn make_test_equity_forward() -> EquityForward {
        EquityForward {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            forward_price: 5000.0,
            settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
            notional: 100_000.0,
            currency: Currency::USD,
        }
    }

    fn make_test_cds() -> Cds {
        Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.01,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy, CreditEvent::FailureToPay],
        }
    }

    fn make_test_commodity_forward() -> CommodityForward {
        CommodityForward {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            delivery_location: "Cushing, OK".to_string(),
            delivery_date: Date::from_ymd(2025, 6, 15).unwrap(),
            quantity: 1000.0,
            unit: QuantityUnit::Barrels,
            forward_price: 75.0,
            currency: Currency::USD,
        }
    }

    fn make_test_asian_option() -> AsianOption {
        AsianOption {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            strike: 5000.0,
            expiry: Date::from_ymd(2025, 12, 15).unwrap(),
            option_type: OptionType::Call,
            averaging_type: AveragingType::Arithmetic,
            observation_frequency: Frequency::Monthly,
            observed_values: vec![],
            notional: 100_000.0,
            currency: Currency::USD,
        }
    }

    // === Asset Class Tests ===

    #[test]
    fn test_asset_class_rates() {
        let instrument = InstrumentDefinition::Swaption(make_test_swaption());
        assert_eq!(instrument.asset_class(), AssetClass::Rates);
    }

    #[test]
    fn test_asset_class_fx() {
        let instrument = InstrumentDefinition::FxSpot(make_test_fx_spot());
        assert_eq!(instrument.asset_class(), AssetClass::Fx);
    }

    #[test]
    fn test_asset_class_equity() {
        let instrument = InstrumentDefinition::EquityForward(make_test_equity_forward());
        assert_eq!(instrument.asset_class(), AssetClass::Equity);
    }

    #[test]
    fn test_asset_class_credit() {
        let instrument = InstrumentDefinition::Cds(make_test_cds());
        assert_eq!(instrument.asset_class(), AssetClass::Credit);
    }

    #[test]
    fn test_asset_class_commodity() {
        let instrument = InstrumentDefinition::CommodityForward(make_test_commodity_forward());
        assert_eq!(instrument.asset_class(), AssetClass::Commodity);
    }

    // === is_option Tests ===

    #[test]
    fn test_is_option_swaption() {
        let instrument = InstrumentDefinition::Swaption(make_test_swaption());
        assert!(instrument.is_option());
    }

    #[test]
    fn test_is_option_fx_vanilla() {
        let instrument = InstrumentDefinition::FxVanillaOption(make_test_fx_vanilla_option());
        assert!(instrument.is_option());
    }

    #[test]
    fn test_is_option_asian() {
        let instrument = InstrumentDefinition::AsianOption(make_test_asian_option());
        assert!(instrument.is_option());
    }

    #[test]
    fn test_is_not_option_fx_spot() {
        let instrument = InstrumentDefinition::FxSpot(make_test_fx_spot());
        assert!(!instrument.is_option());
    }

    #[test]
    fn test_is_not_option_cds() {
        let instrument = InstrumentDefinition::Cds(make_test_cds());
        assert!(!instrument.is_option());
    }

    // === is_swap Tests ===

    #[test]
    fn test_is_swap_cds() {
        let instrument = InstrumentDefinition::Cds(make_test_cds());
        assert!(instrument.is_swap());
    }

    #[test]
    fn test_is_not_swap_swaption() {
        let instrument = InstrumentDefinition::Swaption(make_test_swaption());
        assert!(!instrument.is_swap());
    }

    #[test]
    fn test_is_not_swap_fx_spot() {
        let instrument = InstrumentDefinition::FxSpot(make_test_fx_spot());
        assert!(!instrument.is_swap());
    }

    // === is_forward Tests ===

    #[test]
    fn test_is_forward_fx_spot() {
        let instrument = InstrumentDefinition::FxSpot(make_test_fx_spot());
        assert!(instrument.is_forward());
    }

    #[test]
    fn test_is_forward_equity_forward() {
        let instrument = InstrumentDefinition::EquityForward(make_test_equity_forward());
        assert!(instrument.is_forward());
    }

    #[test]
    fn test_is_forward_commodity_forward() {
        let instrument = InstrumentDefinition::CommodityForward(make_test_commodity_forward());
        assert!(instrument.is_forward());
    }

    #[test]
    fn test_is_not_forward_swaption() {
        let instrument = InstrumentDefinition::Swaption(make_test_swaption());
        assert!(!instrument.is_forward());
    }

    // === is_path_dependent Tests ===

    #[test]
    fn test_is_path_dependent_asian() {
        let instrument = InstrumentDefinition::AsianOption(make_test_asian_option());
        assert!(instrument.is_path_dependent());
    }

    #[test]
    fn test_is_not_path_dependent_fx_vanilla() {
        let instrument = InstrumentDefinition::FxVanillaOption(make_test_fx_vanilla_option());
        assert!(!instrument.is_path_dependent());
    }

    // === is_exotic Tests ===

    #[test]
    fn test_is_exotic_asian() {
        let instrument = InstrumentDefinition::AsianOption(make_test_asian_option());
        assert!(instrument.is_exotic());
    }

    #[test]
    fn test_is_not_exotic_fx_vanilla() {
        let instrument = InstrumentDefinition::FxVanillaOption(make_test_fx_vanilla_option());
        assert!(!instrument.is_exotic());
    }

    // === Validation Tests ===

    #[test]
    fn test_validate_swaption_success() {
        let instrument = InstrumentDefinition::Swaption(make_test_swaption());
        assert!(instrument.validate().is_ok());
    }

    #[test]
    fn test_validate_fx_spot_success() {
        let instrument = InstrumentDefinition::FxSpot(make_test_fx_spot());
        assert!(instrument.validate().is_ok());
    }

    #[test]
    fn test_validate_cds_success() {
        let instrument = InstrumentDefinition::Cds(make_test_cds());
        assert!(instrument.validate().is_ok());
    }

    #[test]
    fn test_validate_swaption_failure() {
        let mut swaption = make_test_swaption();
        swaption.notional = -100.0; // Invalid: negative notional
        let instrument = InstrumentDefinition::Swaption(swaption);
        assert!(instrument.validate().is_err());
    }

    #[test]
    fn test_validate_fx_spot_failure() {
        let mut fx_spot = make_test_fx_spot();
        fx_spot.spot_rate = -1.0; // Invalid: negative rate
        let instrument = InstrumentDefinition::FxSpot(fx_spot);
        assert!(instrument.validate().is_err());
    }

    // === Display Tests ===

    #[test]
    fn test_display_swaption() {
        let instrument = InstrumentDefinition::Swaption(make_test_swaption());
        assert_eq!(instrument.to_string(), "Swaption");
    }

    #[test]
    fn test_display_fx_spot() {
        let instrument = InstrumentDefinition::FxSpot(make_test_fx_spot());
        assert_eq!(instrument.to_string(), "FXSpot");
    }

    #[test]
    fn test_display_cds() {
        let instrument = InstrumentDefinition::Cds(make_test_cds());
        assert_eq!(instrument.to_string(), "CDS");
    }

    // === Clone and PartialEq Tests ===

    #[test]
    fn test_clone_equality() {
        let instrument = InstrumentDefinition::Swaption(make_test_swaption());
        let cloned = instrument.clone();
        assert_eq!(instrument, cloned);
    }

    #[test]
    fn test_inequality() {
        let swaption = InstrumentDefinition::Swaption(make_test_swaption());
        let fx_spot = InstrumentDefinition::FxSpot(make_test_fx_spot());
        assert_ne!(swaption, fx_spot);
    }

    // === Comprehensive Asset Class Coverage ===

    #[test]
    fn test_all_rates_instruments_have_rates_asset_class() {
        let cap_floor = InstrumentDefinition::CapFloor(CapFloor {
            cap_floor_type: CapFloorType::Cap,
            strikes: vec![0.03],
            index: RateIndex::Sofr,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            tenor: Tenor::FiveYears,
            notional_schedule: NotionalSchedule::constant(10_000_000.0),
            payment_frequency: Frequency::Quarterly,
            currency: Currency::USD,
        });
        assert_eq!(cap_floor.asset_class(), AssetClass::Rates);

        let frn = InstrumentDefinition::Frn(Frn {
            coupon_index: RateIndex::Sofr,
            spread: 0.001,
            reset_frequency: Frequency::Quarterly,
            principal_schedule: NotionalSchedule::constant(1_000_000.0),
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            currency: Currency::USD,
        });
        assert_eq!(frn.asset_class(), AssetClass::Rates);
    }
}
