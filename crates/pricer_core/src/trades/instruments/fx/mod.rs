//! FX derivative instruments.

use num_traits::Float;

use super::InstrumentTrait;
#[allow(deprecated)]
use crate::types::Currency;

/// FX derivative instruments.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum FxInstrument<T: Float> {
    /// FX Forward placeholder.
    Forward {
        /// Base currency.
        base_currency: Currency,
        /// Quote currency.
        quote_currency: Currency,
        /// Forward rate.
        forward_rate: T,
        /// Notional amount.
        notional: T,
        /// Maturity in years.
        maturity: T,
    },
    /// FX Option placeholder.
    Option {
        /// Base currency.
        base_currency: Currency,
        /// Quote currency.
        quote_currency: Currency,
        /// Strike rate.
        strike: T,
        /// Notional amount.
        notional: T,
        /// Maturity in years.
        maturity: T,
        /// Is call option.
        is_call: bool,
    },
}

impl<T: Float> FxInstrument<T> {
    /// Compute the payoff at given spot rate.
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        match self {
            FxInstrument::Forward {
                forward_rate,
                notional,
                ..
            } => *notional * (spot - *forward_rate),
            FxInstrument::Option {
                strike,
                notional,
                is_call,
                ..
            } => {
                if *is_call {
                    *notional * (spot - *strike).max(T::zero())
                } else {
                    *notional * (*strike - spot).max(T::zero())
                }
            }
        }
    }

    /// Return time to expiry in years.
    #[inline]
    pub fn expiry(&self) -> T {
        match self {
            FxInstrument::Forward { maturity, .. } => *maturity,
            FxInstrument::Option { maturity, .. } => *maturity,
        }
    }

    /// Return the base currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        match self {
            FxInstrument::Forward { base_currency, .. } => *base_currency,
            FxInstrument::Option { base_currency, .. } => *base_currency,
        }
    }

    /// Return whether this is an FX forward.
    #[inline]
    pub fn is_forward(&self) -> bool { matches!(self, FxInstrument::Forward { .. }) }

    /// Return whether this is an FX option.
    #[inline]
    pub fn is_option(&self) -> bool { matches!(self, FxInstrument::Option { .. }) }
}

impl<T: Float> InstrumentTrait<T> for FxInstrument<T> {
    #[inline]
    fn payoff(&self, spot: T) -> T { self.payoff(spot) }

    #[inline]
    fn expiry(&self) -> T { self.expiry() }

    #[inline]
    fn currency(&self) -> Currency { self.currency() }

    fn type_name(&self) -> &'static str {
        match self {
            FxInstrument::Forward { .. } => "FxForward",
            FxInstrument::Option { .. } => "FxOption",
        }
    }
}
