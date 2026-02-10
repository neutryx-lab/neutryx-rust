//! Standard instrument definitions for all asset classes (Rates, FX, Equity, Credit, Commodity).

mod common;
mod error;
mod expander;

// Asset class specific modules
mod commodity;
mod credit;
mod equity;
mod fx;
mod fx_vol;
mod ir_vol;
mod rates;
mod xccy;

/// Market convention definitions for all asset classes.
pub mod convention;

// Re-exports
// Commodity instruments
pub use commodity::{
    AgricultureType, CommodityAsianOption, CommodityForward, CommoditySwap, CommodityType,
    CommodityVanillaOption, EnergyType, MetalType, QuantityUnit, SpreadOption,
};
pub use common::{
    AssetClass, BarrierDirection, BarrierType, ExerciseStyle, NotionalSchedule, PayerReceiver,
    PaymentSchedule,
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
pub use fx::{
    CurrencyPair, FxBarrierOption, FxForward, FxSpot, FxSwap, FxSwapConvention, FxSwapError,
    FxSwapInstrument, FxSwapTenor, FxVanillaOption, SwapPoints,
};
// FX Vol instruments
pub use fx_vol::{
    CutOffTime, Delta, DeltaType, FxVolConvention, FxVolInstrument, FxVolInstrumentBuilder,
    FxVolInstrumentError,
};
// IR Vol instruments
pub use ir_vol::{
    CapFloor, CapFloorBuilder, CapFloorType, IrVolInstrument, IrVolInstrumentError, Swaption,
    SwaptionBuilder,
};
// Rates instruments
pub use rates::{
    BasisSwap, CmsSwap, Deposit, Fra, Frn, Futures, InflationSwap, InterestRateSwap, Ois, SwapType,
};
// XCCY instruments
pub use xccy::{
    BasisSpread, CrossCurrencyBasisSwap, NotionalExchange, SpreadLeg, XccyBasisConvention, XccyLeg,
    XccySwapError, XccyTenor,
};

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
/// use infra_domain::market::instrument::{
///     InstrumentDefinition, AssetClass, FxSpot, CurrencyPair,
/// };
/// use infra_domain::{market::Currency, time::Date};
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
    /// Money market deposit.
    Deposit(Deposit),
    /// Forward Rate Agreement.
    Fra(Fra),
    /// Interest rate futures.
    Futures(Futures),
    /// Interest Rate Swap (fixed-for-floating).
    InterestRateSwap(InterestRateSwap),
    /// Basis Swap (floating-for-floating).
    BasisSwap(BasisSwap),
    /// Overnight Index Swap (OIS) with daily compounding.
    Ois(Ois),
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
    /// Cross-currency basis swap.
    CrossCurrencyBasisSwap(CrossCurrencyBasisSwap),

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
    /// use infra_domain::market::instrument::{
    ///     InstrumentDefinition, AssetClass, Swaption, PayerReceiver,
    /// };
    /// use infra_domain::trade::{ExerciseType, SettlementType};
    /// use infra_domain::{market::Currency, time::{Date, Tenor}};
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
            InstrumentDefinition::Deposit(_)
            | InstrumentDefinition::Fra(_)
            | InstrumentDefinition::Futures(_)
            | InstrumentDefinition::InterestRateSwap(_)
            | InstrumentDefinition::BasisSwap(_)
            | InstrumentDefinition::Ois(_)
            | InstrumentDefinition::Swaption(_)
            | InstrumentDefinition::CapFloor(_)
            | InstrumentDefinition::Frn(_)
            | InstrumentDefinition::CmsSwap(_)
            | InstrumentDefinition::InflationSwap(_) => AssetClass::Rates,

            // FX
            InstrumentDefinition::FxSpot(_)
            | InstrumentDefinition::FxForward(_)
            | InstrumentDefinition::FxVanillaOption(_)
            | InstrumentDefinition::FxBarrierOption(_)
            | InstrumentDefinition::FxSwap(_)
            | InstrumentDefinition::CrossCurrencyBasisSwap(_) => AssetClass::Fx,

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
            InstrumentDefinition::InterestRateSwap(_)
                | InstrumentDefinition::BasisSwap(_)
                | InstrumentDefinition::Ois(_)
                | InstrumentDefinition::CmsSwap(_)
                | InstrumentDefinition::InflationSwap(_)
                // FX swaps
                | InstrumentDefinition::FxSwap(_)
                | InstrumentDefinition::CrossCurrencyBasisSwap(_)
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
    pub fn validate(&self) -> Result<(), InstrumentError> {
        macro_rules! dispatch_validate {
            ($self:expr; $($Variant:ident),+ $(,)?) => {
                match $self {
                    $(InstrumentDefinition::$Variant(inner) => inner.validate(),)+
                    InstrumentDefinition::CrossCurrencyBasisSwap(x) => x
                        .validate()
                        .map_err(|e| InstrumentError::invalid_parameter(e.to_string())),
                }
            };
        }
        dispatch_validate!(self;
            Deposit, Fra, Futures, InterestRateSwap, BasisSwap, Ois,
            Swaption, CapFloor, Frn, CmsSwap, InflationSwap,
            FxSpot, FxForward, FxVanillaOption, FxBarrierOption, FxSwap,
            EquityForward, EquityVanillaOption, EquityBarrierOption, AsianOption, LookbackOption, EquitySwap, BasketOption,
            Cds, CdsIndex, CdsOption, NtdBasket,
            CommodityForward, CommoditySwap, CommodityVanillaOption, CommodityAsianOption, SpreadOption,
        )
    }
}

impl std::fmt::Display for InstrumentDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        macro_rules! display_name {
            ($self:expr; $($Variant:ident => $name:literal),+ $(,)?) => {
                match $self { $(InstrumentDefinition::$Variant(_) => $name,)+ }
            };
        }
        write!(
            f,
            "{}",
            display_name!(self;
                Deposit => "Deposit", Fra => "FRA", Futures => "Futures",
                InterestRateSwap => "IRS", BasisSwap => "BasisSwap", Ois => "OIS",
                Swaption => "Swaption", CapFloor => "CapFloor", Frn => "FRN",
                CmsSwap => "CMSSwap", InflationSwap => "InflationSwap",
                FxSpot => "FXSpot", FxForward => "FXForward",
                FxVanillaOption => "FXVanillaOption", FxBarrierOption => "FXBarrierOption",
                FxSwap => "FXSwap", CrossCurrencyBasisSwap => "XCCY",
                EquityForward => "EquityForward", EquityVanillaOption => "EquityVanillaOption",
                EquityBarrierOption => "EquityBarrierOption", AsianOption => "AsianOption",
                LookbackOption => "LookbackOption", EquitySwap => "EquitySwap",
                BasketOption => "BasketOption",
                Cds => "CDS", CdsIndex => "CDSIndex", CdsOption => "CDSOption", NtdBasket => "NtDBasket",
                CommodityForward => "CommodityForward", CommoditySwap => "CommoditySwap",
                CommodityVanillaOption => "CommodityVanillaOption",
                CommodityAsianOption => "CommodityAsianOption", SpreadOption => "SpreadOption",
            )
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        market::Currency,
        time::{Date, Tenor},
        trade::{ExerciseType, OptionType, SettlementType},
    };

    fn swaption() -> InstrumentDefinition {
        InstrumentDefinition::Swaption(Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        })
    }
    fn fx_spot() -> InstrumentDefinition {
        InstrumentDefinition::FxSpot(FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        })
    }
    fn fx_option() -> InstrumentDefinition {
        InstrumentDefinition::FxVanillaOption(FxVanillaOption {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            strike: 1.1000,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            delivery_date: Date::from_ymd(2025, 6, 17).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        })
    }
    fn eq_fwd() -> InstrumentDefinition {
        InstrumentDefinition::EquityForward(EquityForward {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            forward_price: 5000.0,
            settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
            notional: 100_000.0,
            currency: Currency::USD,
        })
    }
    fn cds() -> InstrumentDefinition {
        InstrumentDefinition::Cds(Cds {
            reference_entity: "ACME".to_string(),
            notional: 10_000_000.0,
            spread: 0.01,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy],
        })
    }
    fn comm_fwd() -> InstrumentDefinition {
        InstrumentDefinition::CommodityForward(CommodityForward {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            delivery_location: "Cushing".to_string(),
            delivery_date: Date::from_ymd(2025, 6, 15).unwrap(),
            quantity: 1000.0,
            unit: QuantityUnit::Barrels,
            forward_price: 75.0,
            currency: Currency::USD,
        })
    }
    fn asian() -> InstrumentDefinition {
        InstrumentDefinition::AsianOption(AsianOption {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            strike: 5000.0,
            expiry: Date::from_ymd(2025, 12, 15).unwrap(),
            option_type: OptionType::Call,
            averaging_type: AveragingType::Arithmetic,
            observation_frequency: crate::time::Frequency::Monthly,
            observed_values: vec![],
            notional: 100_000.0,
            currency: Currency::USD,
        })
    }

    #[test]
    fn test_asset_class() {
        assert_eq!(swaption().asset_class(), AssetClass::Rates);
        assert_eq!(fx_spot().asset_class(), AssetClass::Fx);
        assert_eq!(eq_fwd().asset_class(), AssetClass::Equity);
        assert_eq!(cds().asset_class(), AssetClass::Credit);
        assert_eq!(comm_fwd().asset_class(), AssetClass::Commodity);
    }

    #[test]
    fn test_classification_predicates() {
        // is_option
        assert!(swaption().is_option());
        assert!(fx_option().is_option());
        assert!(asian().is_option());
        assert!(!fx_spot().is_option());
        assert!(!cds().is_option());

        // is_swap
        assert!(cds().is_swap());
        assert!(!swaption().is_swap());
        assert!(!fx_spot().is_swap());

        // is_forward
        assert!(fx_spot().is_forward());
        assert!(eq_fwd().is_forward());
        assert!(comm_fwd().is_forward());
        assert!(!swaption().is_forward());

        // is_path_dependent / is_exotic
        assert!(asian().is_path_dependent());
        assert!(asian().is_exotic());
        assert!(!fx_option().is_path_dependent());
        assert!(!fx_option().is_exotic());
    }

    #[test]
    fn test_validate_and_display() {
        // Valid instruments
        assert!(swaption().validate().is_ok());
        assert!(fx_spot().validate().is_ok());
        assert!(cds().validate().is_ok());

        // Invalid instruments
        let bad_swaption = Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: -100.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };
        assert!(InstrumentDefinition::Swaption(bad_swaption)
            .validate()
            .is_err());

        let bad_fx = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: -1.0,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(InstrumentDefinition::FxSpot(bad_fx).validate().is_err());

        // Display
        assert_eq!(swaption().to_string(), "Swaption");
        assert_eq!(fx_spot().to_string(), "FXSpot");
        assert_eq!(cds().to_string(), "CDS");
    }

    #[test]
    fn test_clone_and_equality() {
        let s = swaption();
        assert_eq!(s, s.clone());
        assert_ne!(swaption(), fx_spot());
    }
}
