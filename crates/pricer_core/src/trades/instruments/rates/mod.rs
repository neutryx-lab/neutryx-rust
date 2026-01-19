//! Interest rate derivative instruments.

use num_traits::Float;

#[allow(deprecated)]
use crate::types::Currency;

use super::InstrumentTrait;

/// Interest rate derivative instruments.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RatesInstrument<T: Float> {
    /// Interest rate swap (placeholder).
    Swap(super::Swap<T>),
}

impl<T: Float> RatesInstrument<T> {
    /// Compute the payoff (placeholder for rates).
    #[inline]
    pub fn payoff(&self, _spot: T) -> T {
        match self {
            RatesInstrument::Swap(_swap) => T::zero(),
        }
    }

    /// Return time to expiry (maturity) in years.
    #[inline]
    pub fn expiry(&self) -> T {
        match self {
            RatesInstrument::Swap(swap) => swap.maturity(),
        }
    }

    /// Return the currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        match self {
            RatesInstrument::Swap(swap) => swap.currency(),
        }
    }

    /// Return whether this is a swap.
    #[inline]
    pub fn is_swap(&self) -> bool {
        matches!(self, RatesInstrument::Swap(_))
    }
}

impl<T: Float> InstrumentTrait<T> for RatesInstrument<T> {
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
            RatesInstrument::Swap(_) => "RatesSwap",
        }
    }
}

impl<T: Float> From<super::Swap<T>> for RatesInstrument<T> {
    fn from(swap: super::Swap<T>) -> Self {
        RatesInstrument::Swap(swap)
    }
}
