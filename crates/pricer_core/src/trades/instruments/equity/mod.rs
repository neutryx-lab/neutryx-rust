//! Equity derivative instruments.

use num_traits::Float;

use super::InstrumentTrait;
pub use super::{
    forward::{Direction, Forward},
    vanilla::VanillaOption,
};
#[allow(deprecated)]
use crate::types::Currency;

/// Equity derivative instruments.
#[derive(Debug, Clone)]
pub enum EquityInstrument<T: Float> {
    /// Vanilla option (Call, Put, Digital).
    Vanilla(VanillaOption<T>),
    /// Forward contract.
    Forward(Forward<T>),
}

impl<T: Float> EquityInstrument<T> {
    /// Compute the payoff at given spot price.
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        match self {
            EquityInstrument::Vanilla(option) => option.payoff(spot),
            EquityInstrument::Forward(forward) => forward.payoff(spot),
        }
    }

    /// Return time to expiry in years.
    #[inline]
    pub fn expiry(&self) -> T {
        match self {
            EquityInstrument::Vanilla(option) => option.expiry(),
            EquityInstrument::Forward(forward) => forward.expiry(),
        }
    }

    /// Return the underlying instrument's currency.
    #[inline]
    pub fn currency(&self) -> Currency { Currency::USD }

    /// Return whether this is a vanilla option.
    #[inline]
    pub fn is_vanilla(&self) -> bool { matches!(self, EquityInstrument::Vanilla(_)) }

    /// Return whether this is a forward contract.
    #[inline]
    pub fn is_forward(&self) -> bool { matches!(self, EquityInstrument::Forward(_)) }

    /// Return a reference to the vanilla option if this is a Vanilla variant.
    pub fn as_vanilla(&self) -> Option<&VanillaOption<T>> {
        match self {
            EquityInstrument::Vanilla(option) => Some(option),
            EquityInstrument::Forward(_) => None,
        }
    }

    /// Return a reference to the forward if this is a Forward variant.
    pub fn as_forward(&self) -> Option<&Forward<T>> {
        match self {
            EquityInstrument::Forward(forward) => Some(forward),
            EquityInstrument::Vanilla(_) => None,
        }
    }
}

impl<T: Float> InstrumentTrait<T> for EquityInstrument<T> {
    #[inline]
    fn payoff(&self, spot: T) -> T { self.payoff(spot) }

    #[inline]
    fn expiry(&self) -> T { self.expiry() }

    #[inline]
    fn currency(&self) -> Currency { self.currency() }

    fn type_name(&self) -> &'static str {
        match self {
            EquityInstrument::Vanilla(_) => "EquityVanilla",
            EquityInstrument::Forward(_) => "EquityForward",
        }
    }
}

impl<T: Float> From<VanillaOption<T>> for EquityInstrument<T> {
    fn from(option: VanillaOption<T>) -> Self { EquityInstrument::Vanilla(option) }
}

impl<T: Float> From<Forward<T>> for EquityInstrument<T> {
    fn from(forward: Forward<T>) -> Self { EquityInstrument::Forward(forward) }
}
