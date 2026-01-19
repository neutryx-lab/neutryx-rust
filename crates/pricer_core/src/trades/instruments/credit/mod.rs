//! Credit derivative instruments.

use num_traits::Float;

#[allow(deprecated)]
use crate::types::Currency;

use super::InstrumentTrait;

/// Credit derivative instruments.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum CreditInstrument<T: Float> {
    /// Placeholder for CDS implementation.
    Cds {
        /// Notional amount.
        notional: T,
        /// Spread (bps).
        spread: T,
        /// Maturity in years.
        maturity: T,
        /// Currency.
        currency: Currency,
    },
}

impl<T: Float> CreditInstrument<T> {
    /// Compute the payoff (placeholder).
    #[inline]
    pub fn payoff(&self, _spot: T) -> T {
        match self {
            CreditInstrument::Cds { .. } => T::zero(),
        }
    }

    /// Return time to expiry in years.
    #[inline]
    pub fn expiry(&self) -> T {
        match self {
            CreditInstrument::Cds { maturity, .. } => *maturity,
        }
    }

    /// Return the currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        match self {
            CreditInstrument::Cds { currency, .. } => *currency,
        }
    }

    /// Return whether this is a CDS.
    #[inline]
    pub fn is_cds(&self) -> bool {
        matches!(self, CreditInstrument::Cds { .. })
    }
}

impl<T: Float> InstrumentTrait<T> for CreditInstrument<T> {
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
            CreditInstrument::Cds { .. } => "CDS",
        }
    }
}
