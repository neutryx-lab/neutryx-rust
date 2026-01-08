//! Financial instrument definitions.
//!
//! This module provides instrument definitions for pricing with
//! enum dispatch architecture for Enzyme AD compatibility.
//!
//! # Architecture
//!
//! Uses enum dispatch (NOT trait objects) for static dispatch optimisation:
//! - `Instrument<T>` enum wraps all instrument types
//! - All types generic over `T: Float` for AD compatibility
//! - Smooth payoff approximations via `pricer_core::math::smoothing`
//!
//! # Asset Class Modules
//!
//! Instruments are organized by asset class (enabled via feature flags):
//! - [`equity`]: Equity derivatives (VanillaOption, Forward) - default
//! - [`rates`]: Interest rate derivatives (IRS, Swaption, Cap/Floor)
//! - [`credit`]: Credit derivatives (CDS)
//! - [`fx`]: FX derivatives (FxOption, FxForward)
//! - [`commodity`]: Commodity derivatives
//! - [`exotic`]: Exotic derivatives (VarianceSwap, Cliquet, etc.)
//!
//! # Instrument Types
//!
//! - [`VanillaOption`]: European/American/Asian options with Call/Put payoffs
//! - [`Forward`]: Forward contracts with linear payoffs
//! - [`Swap`]: Interest rate swaps with payment schedules
//!
//! # Examples
//!
//! ```
//! use pricer_models::instruments::{
//!     Instrument, VanillaOption, Forward, Swap,
//!     InstrumentParams, PayoffType, ExerciseStyle, Direction, PaymentFrequency,
//! };
//! use pricer_core::types::Currency;
//!
//! // Create a vanilla call option
//! let params = InstrumentParams::new(100.0_f64, 1.0, 1_000_000.0).unwrap();
//! let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
//! let instrument = Instrument::Vanilla(call);
//!
//! // Compute payoff
//! let payoff = instrument.payoff(110.0);
//! assert!(payoff > 0.0);
//! ```

// Core types (always available)
mod error;
mod exercise;
mod params;
mod payoff;
mod traits;

// Instrument implementations (always available for backward compatibility)
mod forward;
mod swap;
mod vanilla;

// Asset class submodules (feature-gated)
#[cfg(feature = "equity")]
pub mod equity;

#[cfg(feature = "rates")]
pub mod rates;

#[cfg(feature = "credit")]
pub mod credit;

#[cfg(feature = "fx")]
pub mod fx;

#[cfg(feature = "commodity")]
pub mod commodity;

#[cfg(feature = "exotic")]
pub mod exotic;

// Re-export all public types
pub use error::InstrumentError;
pub use exercise::ExerciseStyle;
pub use forward::{Direction, Forward};
pub use params::InstrumentParams;
pub use payoff::PayoffType;
pub use swap::{PaymentFrequency, Swap};
pub use traits::{Cashflow, CashflowInstrument, InstrumentTrait};
pub use vanilla::VanillaOption;

// Re-export asset class enums (when features enabled)
#[cfg(feature = "equity")]
pub use equity::EquityInstrument;

#[cfg(feature = "rates")]
pub use rates::RatesInstrument;

#[cfg(feature = "credit")]
pub use credit::CreditInstrument;

#[cfg(feature = "fx")]
pub use fx::FxInstrument;

use num_traits::Float;
use pricer_core::types::Currency;

/// Unified instrument enum for static dispatch.
///
/// Wraps all instrument types for Enzyme-compatible static dispatch
/// without trait objects or dynamic allocation.
///
/// # Type Parameters
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Variants
/// - `Vanilla`: Vanilla options (European, American, Bermudan, Asian)
/// - `Forward`: Forward contracts
/// - `Swap`: Interest rate swaps
///
/// # Examples
/// ```
/// use pricer_models::instruments::{
///     Instrument, VanillaOption, Forward, InstrumentParams, PayoffType,
///     ExerciseStyle, Direction,
/// };
///
/// // Create instruments
/// let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
/// let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
/// let forward = Forward::new(100.0, 1.0, 1.0, Direction::Long).unwrap();
///
/// let vanilla_inst = Instrument::Vanilla(call);
/// let forward_inst = Instrument::Forward(forward);
///
/// // Compute payoffs via enum dispatch
/// let payoff1 = vanilla_inst.payoff(110.0);
/// let payoff2 = forward_inst.payoff(110.0);
/// ```
#[derive(Debug, Clone)]
pub enum Instrument<T: Float> {
    /// Vanilla option (Call, Put, Digital)
    Vanilla(VanillaOption<T>),
    /// Forward contract
    Forward(Forward<T>),
    /// Interest rate swap
    Swap(Swap<T>),
}

impl<T: Float> Instrument<T> {
    /// Compute the payoff for the instrument at given spot price.
    ///
    /// Static dispatch to the appropriate payoff method based on variant.
    ///
    /// # Arguments
    /// * `spot` - Current spot price
    ///
    /// # Returns
    /// Payoff value for the instrument.
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::{
    ///     Instrument, VanillaOption, InstrumentParams, PayoffType, ExerciseStyle,
    /// };
    ///
    /// let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
    /// let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
    /// let instrument = Instrument::Vanilla(call);
    ///
    /// let payoff = instrument.payoff(110.0);
    /// assert!((payoff - 10.0).abs() < 0.01);
    /// ```
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        match self {
            Instrument::Vanilla(option) => option.payoff(spot),
            Instrument::Forward(forward) => forward.payoff(spot),
            Instrument::Swap(_swap) => {
                // Swaps don't have a simple payoff function
                // Return zero as placeholder (actual valuation requires curve)
                T::zero()
            }
        }
    }

    /// Returns the expiry time of the instrument.
    ///
    /// # Returns
    /// Time to expiry in years.
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::{
    ///     Instrument, Forward, Direction,
    /// };
    ///
    /// let forward = Forward::new(100.0_f64, 0.5, 1.0, Direction::Long).unwrap();
    /// let instrument = Instrument::Forward(forward);
    ///
    /// assert_eq!(instrument.expiry(), 0.5);
    /// ```
    #[inline]
    pub fn expiry(&self) -> T {
        match self {
            Instrument::Vanilla(option) => option.expiry(),
            Instrument::Forward(forward) => forward.expiry(),
            Instrument::Swap(swap) => swap.maturity(),
        }
    }

    /// Returns whether this is a vanilla option.
    #[inline]
    pub fn is_vanilla(&self) -> bool {
        matches!(self, Instrument::Vanilla(_))
    }

    /// Returns whether this is a forward contract.
    #[inline]
    pub fn is_forward(&self) -> bool {
        matches!(self, Instrument::Forward(_))
    }

    /// Returns whether this is a swap.
    #[inline]
    pub fn is_swap(&self) -> bool {
        matches!(self, Instrument::Swap(_))
    }

    /// Returns a reference to the vanilla option if this is a Vanilla variant.
    pub fn as_vanilla(&self) -> Option<&VanillaOption<T>> {
        match self {
            Instrument::Vanilla(option) => Some(option),
            _ => None,
        }
    }

    /// Returns a reference to the forward if this is a Forward variant.
    pub fn as_forward(&self) -> Option<&Forward<T>> {
        match self {
            Instrument::Forward(forward) => Some(forward),
            _ => None,
        }
    }

    /// Returns a reference to the swap if this is a Swap variant.
    pub fn as_swap(&self) -> Option<&Swap<T>> {
        match self {
            Instrument::Swap(swap) => Some(swap),
            _ => None,
        }
    }
}

// ============================================================================
// Hierarchical Instrument Enum (New Architecture)
// ============================================================================

/// Hierarchical instrument enum for asset-class based organization.
///
/// This is the new architecture that organizes instruments by asset class,
/// with each asset class having its own sub-enum. The design enables:
///
/// - Feature-flag based conditional compilation per asset class
/// - Static dispatch (Enzyme AD compatible) at both top and sub-enum levels
/// - Clean separation between asset classes
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Variants
///
/// - `Equity`: Equity derivatives (vanilla options, forwards)
/// - `Rates`: Interest rate derivatives (IRS, swaptions, caps/floors)
/// - `Credit`: Credit derivatives (CDS)
/// - `Fx`: FX derivatives (FX options, FX forwards)
/// - `Commodity`: Commodity derivatives
/// - `Exotic`: Exotic derivatives (variance swaps, cliquets, etc.)
///
/// # Feature Flags
///
/// Each variant is gated by its corresponding feature flag:
/// - `equity` (default): Enables `Equity` variant
/// - `rates`: Enables `Rates` variant
/// - `credit`: Enables `Credit` variant
/// - `fx`: Enables `Fx` variant
/// - `commodity`: Enables `Commodity` variant
/// - `exotic`: Enables `Exotic` variant
///
/// # Examples
///
/// ```
/// use pricer_models::instruments::{
///     InstrumentEnum, EquityInstrument, VanillaOption,
///     InstrumentParams, PayoffType, ExerciseStyle, InstrumentTrait,
/// };
///
/// // Create an equity instrument via the hierarchical enum
/// let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
/// let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
/// let equity = EquityInstrument::Vanilla(call);
/// let instrument = InstrumentEnum::Equity(equity);
///
/// // Use InstrumentTrait methods
/// let payoff = instrument.payoff(110.0);
/// assert!((payoff - 10.0).abs() < 0.01);
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InstrumentEnum<T: Float> {
    /// Equity derivatives (vanilla options, forwards).
    #[cfg(feature = "equity")]
    Equity(EquityInstrument<T>),

    /// Interest rate derivatives (IRS, swaptions, caps/floors).
    /// Requires `rates` feature.
    #[cfg(feature = "rates")]
    Rates(RatesInstrument<T>),

    /// Credit derivatives (CDS).
    /// Requires `credit` feature.
    #[cfg(feature = "credit")]
    Credit(CreditInstrument<T>),

    /// FX derivatives (options, forwards).
    /// Requires `fx` feature.
    #[cfg(feature = "fx")]
    Fx(FxInstrument<T>),
    // Future variants (commented until implemented):
    // #[cfg(feature = "commodity")]
    // Commodity(CommodityInstrument<T>),
    // #[cfg(feature = "exotic")]
    // Exotic(ExoticInstrument<T>),
}

impl<T: Float> InstrumentEnum<T> {
    /// Compute the payoff at given spot price.
    ///
    /// Delegates to the underlying asset-class sub-enum.
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        match self {
            #[cfg(feature = "equity")]
            InstrumentEnum::Equity(equity) => equity.payoff(spot),
            #[cfg(feature = "rates")]
            InstrumentEnum::Rates(rates) => rates.payoff(spot),
            #[cfg(feature = "credit")]
            InstrumentEnum::Credit(credit) => credit.payoff(spot),
            #[cfg(feature = "fx")]
            InstrumentEnum::Fx(fx) => fx.payoff(spot),
        }
    }

    /// Return time to expiry in years.
    #[inline]
    pub fn expiry(&self) -> T {
        match self {
            #[cfg(feature = "equity")]
            InstrumentEnum::Equity(equity) => equity.expiry(),
            #[cfg(feature = "rates")]
            InstrumentEnum::Rates(rates) => rates.expiry(),
            #[cfg(feature = "credit")]
            InstrumentEnum::Credit(credit) => credit.expiry(),
            #[cfg(feature = "fx")]
            InstrumentEnum::Fx(fx) => fx.expiry(),
        }
    }

    /// Return the settlement currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        match self {
            #[cfg(feature = "equity")]
            InstrumentEnum::Equity(equity) => equity.currency(),
            #[cfg(feature = "rates")]
            InstrumentEnum::Rates(rates) => rates.currency(),
            #[cfg(feature = "credit")]
            InstrumentEnum::Credit(credit) => credit.currency(),
            #[cfg(feature = "fx")]
            InstrumentEnum::Fx(fx) => fx.currency(),
        }
    }

    /// Return the asset class of this instrument.
    pub fn asset_class(&self) -> AssetClass {
        match self {
            #[cfg(feature = "equity")]
            InstrumentEnum::Equity(_) => AssetClass::Equity,
            #[cfg(feature = "rates")]
            InstrumentEnum::Rates(_) => AssetClass::Rates,
            #[cfg(feature = "credit")]
            InstrumentEnum::Credit(_) => AssetClass::Credit,
            #[cfg(feature = "fx")]
            InstrumentEnum::Fx(_) => AssetClass::Fx,
        }
    }

    /// Return whether this is an equity instrument.
    #[cfg(feature = "equity")]
    #[inline]
    pub fn is_equity(&self) -> bool {
        matches!(self, InstrumentEnum::Equity(_))
    }

    /// Return whether this is a rates instrument.
    #[cfg(feature = "rates")]
    #[inline]
    pub fn is_rates(&self) -> bool {
        matches!(self, InstrumentEnum::Rates(_))
    }

    /// Return whether this is a credit instrument.
    #[cfg(feature = "credit")]
    #[inline]
    pub fn is_credit(&self) -> bool {
        matches!(self, InstrumentEnum::Credit(_))
    }

    /// Return a reference to the equity instrument if this is an Equity variant.
    #[cfg(feature = "equity")]
    pub fn as_equity(&self) -> Option<&EquityInstrument<T>> {
        match self {
            InstrumentEnum::Equity(equity) => Some(equity),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    /// Return a reference to the rates instrument if this is a Rates variant.
    #[cfg(feature = "rates")]
    pub fn as_rates(&self) -> Option<&RatesInstrument<T>> {
        match self {
            InstrumentEnum::Rates(rates) => Some(rates),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    /// Return a reference to the credit instrument if this is a Credit variant.
    #[cfg(feature = "credit")]
    pub fn as_credit(&self) -> Option<&CreditInstrument<T>> {
        match self {
            InstrumentEnum::Credit(credit) => Some(credit),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    /// Return whether this is an FX instrument.
    #[cfg(feature = "fx")]
    #[inline]
    pub fn is_fx(&self) -> bool {
        matches!(self, InstrumentEnum::Fx(_))
    }

    /// Return a reference to the FX instrument if this is an Fx variant.
    #[cfg(feature = "fx")]
    pub fn as_fx(&self) -> Option<&FxInstrument<T>> {
        match self {
            InstrumentEnum::Fx(fx) => Some(fx),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}

impl<T: Float> InstrumentTrait<T> for InstrumentEnum<T> {
    #[inline]
    fn payoff(&self, spot: T) -> T {
        self.payoff(spot)
    }

    #[inline]
    fn expiry(&self) -> T {
        self.expiry()
    }

    #[inline]
    fn currency(&self) -> Currency {
        self.currency()
    }

    fn type_name(&self) -> &'static str {
        match self {
            #[cfg(feature = "equity")]
            InstrumentEnum::Equity(equity) => equity.type_name(),
            #[cfg(feature = "rates")]
            InstrumentEnum::Rates(rates) => rates.type_name(),
            #[cfg(feature = "credit")]
            InstrumentEnum::Credit(credit) => credit.type_name(),
            #[cfg(feature = "fx")]
            InstrumentEnum::Fx(fx) => fx.type_name(),
        }
    }
}

// Conversion from asset class sub-enums to InstrumentEnum
#[cfg(feature = "equity")]
impl<T: Float> From<EquityInstrument<T>> for InstrumentEnum<T> {
    fn from(equity: EquityInstrument<T>) -> Self {
        InstrumentEnum::Equity(equity)
    }
}

#[cfg(feature = "rates")]
impl<T: Float> From<RatesInstrument<T>> for InstrumentEnum<T> {
    fn from(rates: RatesInstrument<T>) -> Self {
        InstrumentEnum::Rates(rates)
    }
}

#[cfg(feature = "credit")]
impl<T: Float> From<CreditInstrument<T>> for InstrumentEnum<T> {
    fn from(credit: CreditInstrument<T>) -> Self {
        InstrumentEnum::Credit(credit)
    }
}

#[cfg(feature = "fx")]
impl<T: Float> From<FxInstrument<T>> for InstrumentEnum<T> {
    fn from(fx: FxInstrument<T>) -> Self {
        InstrumentEnum::Fx(fx)
    }
}

/// Asset class classification for instruments.
///
/// Used to categorize instruments at the top level for risk management
/// and reporting purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetClass {
    /// Equity derivatives (options, forwards on stocks/indices).
    Equity,
    /// Interest rate derivatives (swaps, swaptions, caps/floors).
    Rates,
    /// Credit derivatives (CDS, credit indices).
    Credit,
    /// FX derivatives (currency options, forwards).
    Fx,
    /// Commodity derivatives (energy, metals, agriculture).
    Commodity,
    /// Exotic/hybrid derivatives (variance swaps, autocallables).
    Exotic,
}

impl std::fmt::Display for AssetClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetClass::Equity => write!(f, "Equity"),
            AssetClass::Rates => write!(f, "Rates"),
            AssetClass::Credit => write!(f, "Credit"),
            AssetClass::Fx => write!(f, "FX"),
            AssetClass::Commodity => write!(f, "Commodity"),
            AssetClass::Exotic => write!(f, "Exotic"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use pricer_core::types::Currency;

    fn create_test_call() -> VanillaOption<f64> {
        let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
        VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6)
    }

    fn create_test_forward() -> Forward<f64> {
        Forward::new(100.0, 1.0, 1.0, Direction::Long).unwrap()
    }

    fn create_test_swap() -> Swap<f64> {
        let dates = vec![0.5, 1.0, 1.5, 2.0];
        Swap::new(
            1_000_000.0,
            0.03,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        )
        .unwrap()
    }

    #[test]
    fn test_instrument_vanilla_payoff() {
        let call = create_test_call();
        let instrument = Instrument::Vanilla(call);

        let payoff = instrument.payoff(110.0);
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_instrument_forward_payoff() {
        let forward = create_test_forward();
        let instrument = Instrument::Forward(forward);

        let payoff = instrument.payoff(110.0);
        assert_relative_eq!(payoff, 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_instrument_swap_payoff() {
        let swap = create_test_swap();
        let instrument = Instrument::Swap(swap);

        // Swap payoff returns zero (placeholder)
        let payoff = instrument.payoff(110.0);
        assert_eq!(payoff, 0.0);
    }

    #[test]
    fn test_instrument_expiry() {
        let call = create_test_call();
        let forward = create_test_forward();
        let swap = create_test_swap();

        let vanilla_inst = Instrument::Vanilla(call);
        let forward_inst = Instrument::Forward(forward);
        let swap_inst = Instrument::Swap(swap);

        assert_eq!(vanilla_inst.expiry(), 1.0);
        assert_eq!(forward_inst.expiry(), 1.0);
        assert_eq!(swap_inst.expiry(), 2.0); // maturity
    }

    #[test]
    fn test_is_vanilla() {
        let call = create_test_call();
        let instrument = Instrument::Vanilla(call);

        assert!(instrument.is_vanilla());
        assert!(!instrument.is_forward());
        assert!(!instrument.is_swap());
    }

    #[test]
    fn test_is_forward() {
        let forward = create_test_forward();
        let instrument = Instrument::Forward(forward);

        assert!(!instrument.is_vanilla());
        assert!(instrument.is_forward());
        assert!(!instrument.is_swap());
    }

    #[test]
    fn test_is_swap() {
        let swap = create_test_swap();
        let instrument = Instrument::Swap(swap);

        assert!(!instrument.is_vanilla());
        assert!(!instrument.is_forward());
        assert!(instrument.is_swap());
    }

    #[test]
    fn test_as_vanilla() {
        let call = create_test_call();
        let instrument = Instrument::Vanilla(call);

        assert!(instrument.as_vanilla().is_some());
        assert!(instrument.as_forward().is_none());
        assert!(instrument.as_swap().is_none());
    }

    #[test]
    fn test_as_forward() {
        let forward = create_test_forward();
        let instrument = Instrument::Forward(forward);

        assert!(instrument.as_vanilla().is_none());
        assert!(instrument.as_forward().is_some());
        assert!(instrument.as_swap().is_none());
    }

    #[test]
    fn test_as_swap() {
        let swap = create_test_swap();
        let instrument = Instrument::Swap(swap);

        assert!(instrument.as_vanilla().is_none());
        assert!(instrument.as_forward().is_none());
        assert!(instrument.as_swap().is_some());
    }

    #[test]
    fn test_clone() {
        let call = create_test_call();
        let inst1 = Instrument::Vanilla(call);
        let inst2 = inst1.clone();

        assert_eq!(inst1.expiry(), inst2.expiry());
    }

    #[test]
    fn test_debug() {
        let call = create_test_call();
        let instrument = Instrument::Vanilla(call);
        let debug_str = format!("{:?}", instrument);
        assert!(debug_str.contains("Vanilla"));
    }

    // ========================================
    // InstrumentEnum Tests (Hierarchical)
    // ========================================

    #[cfg(feature = "equity")]
    mod instrument_enum_tests {
        use super::*;

        fn create_equity_call() -> EquityInstrument<f64> {
            let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
            let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
            EquityInstrument::Vanilla(call)
        }

        fn create_equity_forward() -> EquityInstrument<f64> {
            let forward = Forward::new(100.0, 1.0, 1.0, Direction::Long).unwrap();
            EquityInstrument::Forward(forward)
        }

        #[test]
        fn test_instrument_enum_equity_payoff() {
            let equity = create_equity_call();
            let instrument = InstrumentEnum::Equity(equity);

            let payoff = instrument.payoff(110.0);
            assert!((payoff - 10.0).abs() < 0.01);
        }

        #[test]
        fn test_instrument_enum_equity_forward_payoff() {
            let equity = create_equity_forward();
            let instrument = InstrumentEnum::Equity(equity);

            let payoff = instrument.payoff(110.0);
            assert!((payoff - 10.0).abs() < 1e-10);
        }

        #[test]
        fn test_instrument_enum_expiry() {
            let equity = create_equity_call();
            let instrument = InstrumentEnum::Equity(equity);

            assert!((instrument.expiry() - 1.0).abs() < 1e-10);
        }

        #[test]
        fn test_instrument_enum_currency() {
            let equity = create_equity_call();
            let instrument = InstrumentEnum::Equity(equity);

            assert_eq!(instrument.currency(), Currency::USD);
        }

        #[test]
        fn test_instrument_enum_asset_class() {
            let equity = create_equity_call();
            let instrument = InstrumentEnum::Equity(equity);

            assert_eq!(instrument.asset_class(), AssetClass::Equity);
        }

        #[test]
        fn test_instrument_enum_is_equity() {
            let equity = create_equity_call();
            let instrument = InstrumentEnum::Equity(equity);

            assert!(instrument.is_equity());
        }

        #[test]
        fn test_instrument_enum_as_equity() {
            let equity = create_equity_call();
            let instrument = InstrumentEnum::Equity(equity);

            assert!(instrument.as_equity().is_some());
        }

        #[test]
        fn test_instrument_enum_from_equity() {
            let equity = create_equity_call();
            let instrument: InstrumentEnum<f64> = equity.into();

            assert!(instrument.is_equity());
        }

        #[test]
        fn test_instrument_enum_trait_payoff() {
            let equity = create_equity_call();
            let instrument = InstrumentEnum::Equity(equity);

            // Use InstrumentTrait method
            let payoff = <InstrumentEnum<f64> as InstrumentTrait<f64>>::payoff(&instrument, 110.0);
            assert!((payoff - 10.0).abs() < 0.01);
        }

        #[test]
        fn test_instrument_enum_trait_type_name() {
            let equity = create_equity_call();
            let instrument = InstrumentEnum::Equity(equity);

            assert_eq!(instrument.type_name(), "EquityVanilla");
        }

        #[test]
        fn test_instrument_enum_clone() {
            let equity = create_equity_call();
            let inst1 = InstrumentEnum::Equity(equity);
            let inst2 = inst1.clone();

            assert!((inst1.expiry() - inst2.expiry()).abs() < 1e-10);
        }

        #[test]
        fn test_instrument_enum_debug() {
            let equity = create_equity_call();
            let instrument = InstrumentEnum::Equity(equity);
            let debug_str = format!("{:?}", instrument);

            assert!(debug_str.contains("Equity"));
        }
    }

    // ========================================
    // AssetClass Tests
    // ========================================

    #[test]
    fn test_asset_class_display() {
        assert_eq!(format!("{}", AssetClass::Equity), "Equity");
        assert_eq!(format!("{}", AssetClass::Rates), "Rates");
        assert_eq!(format!("{}", AssetClass::Credit), "Credit");
        assert_eq!(format!("{}", AssetClass::Fx), "FX");
        assert_eq!(format!("{}", AssetClass::Commodity), "Commodity");
        assert_eq!(format!("{}", AssetClass::Exotic), "Exotic");
    }

    #[test]
    fn test_asset_class_equality() {
        assert_eq!(AssetClass::Equity, AssetClass::Equity);
        assert_ne!(AssetClass::Equity, AssetClass::Rates);
    }

    #[test]
    fn test_asset_class_clone() {
        let ac1 = AssetClass::Equity;
        let ac2 = ac1;
        assert_eq!(ac1, ac2);
    }

    #[test]
    fn test_asset_class_debug() {
        let debug_str = format!("{:?}", AssetClass::Equity);
        assert!(debug_str.contains("Equity"));
    }

    #[test]
    fn test_asset_class_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(AssetClass::Equity);
        set.insert(AssetClass::Rates);
        set.insert(AssetClass::Equity); // Duplicate

        assert_eq!(set.len(), 2);
    }
}
