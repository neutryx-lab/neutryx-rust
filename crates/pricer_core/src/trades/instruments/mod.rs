//! Financial instrument definitions.
//!
//! This module provides instrument definitions for pricing with
//! enum dispatch architecture for Enzyme AD compatibility.

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

// Re-export all public types
#[cfg(feature = "credit")]
pub use credit::CreditInstrument;
#[cfg(feature = "equity")]
pub use equity::EquityInstrument;
pub use error::InstrumentError;
pub use exercise::ExerciseStyle;
pub use forward::{Direction, Forward};
#[cfg(feature = "fx")]
pub use fx::FxInstrument;
use num_traits::Float;
pub use params::InstrumentParams;
pub use payoff::PayoffType;
#[cfg(feature = "rates")]
pub use rates::RatesInstrument;
pub use swap::{PaymentFrequency, Swap};
pub use traits::{Cashflow, CashflowInstrument, InstrumentTrait};
pub use vanilla::VanillaOption;

#[allow(deprecated)]
use crate::types::Currency;

/// Unified instrument enum for static dispatch.
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
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        match self {
            Instrument::Vanilla(option) => option.payoff(spot),
            Instrument::Forward(forward) => forward.payoff(spot),
            Instrument::Swap(_swap) => T::zero(),
        }
    }

    /// Returns the expiry time of the instrument.
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
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InstrumentEnum<T: Float> {
    /// Equity derivatives (vanilla options, forwards).
    #[cfg(feature = "equity")]
    Equity(EquityInstrument<T>),

    /// Interest rate derivatives (IRS, swaptions, caps/floors).
    #[cfg(feature = "rates")]
    Rates(RatesInstrument<T>),

    /// Credit derivatives (CDS).
    #[cfg(feature = "credit")]
    Credit(CreditInstrument<T>),

    /// FX derivatives (options, forwards).
    #[cfg(feature = "fx")]
    Fx(FxInstrument<T>),
}

impl<T: Float> InstrumentEnum<T> {
    /// Compute the payoff at given spot price.
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
    use approx::assert_relative_eq;

    use super::*;

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

        let payoff = instrument.payoff(110.0);
        assert_eq!(payoff, 0.0);
    }

    #[test]
    fn test_asset_class_display() {
        assert_eq!(format!("{}", AssetClass::Equity), "Equity");
        assert_eq!(format!("{}", AssetClass::Rates), "Rates");
        assert_eq!(format!("{}", AssetClass::Credit), "Credit");
        assert_eq!(format!("{}", AssetClass::Fx), "FX");
    }
}
